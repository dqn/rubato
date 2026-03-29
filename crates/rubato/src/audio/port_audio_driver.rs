//! PortAudio Driver - cpal output stream backed by Kira.
//!
//! Translated from: PortAudioDriver.java
//! In Rust, Kira (via cpal backend) replaces PortAudio for audio output.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

use rayon::prelude::*;

use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, PlaybackRate, Tween};

use bms::model::bms_model::BMSModel;
use bms::model::note::Note;

use crate::audio::abstract_audio_driver::SliceWav;
use crate::audio::audio_driver::AudioDriver;
use crate::audio::gdx_sound_driver::{
    BackgroundLoadResult, FileCacheEntry, LoadTask, add_note_entry, configure_path_sound_for_play,
    configure_sound_for_play, linear_to_db,
};

pub struct PortAudioDriver {
    manager: AudioManager,
    // Map from path to sound handle
    path_sounds: HashMap<String, StaticSoundHandle>,
    // Map from wav ID to sound data for BMS keysounds
    wav_sounds: HashMap<i32, StaticSoundData>,
    wav_handles: HashMap<i32, Vec<StaticSoundHandle>>,
    global_pitch: f32,
    // Model volume from volwav (0.0-1.0)
    volume: f32,
    song_resource_gen: i32,
    // Sliced sounds by wav ID (for notes with non-zero starttime/duration)
    slicesound: HashMap<i32, Vec<SliceWav<StaticSoundData>>>,
    slice_handles: HashMap<(i32, i64, i64), Vec<StaticSoundHandle>>,
    // Per-note semitone pitch shifts for active handles, so set_global_pitch
    // can compose global_pitch * 2^(shift/12) instead of overwriting.
    wav_pitch_shifts: HashMap<i32, i32>,
    slice_pitch_shifts: HashMap<(i32, i64, i64), i32>,
    // Cache for loaded sounds by path (matches Java soundmap)
    sound_cache: HashMap<String, StaticSoundData>,
    // File-level keysound cache across songs (matches Java AudioCache/ResourcePool)
    file_cache: HashMap<String, FileCacheEntry>,
    // Additional key sounds for judge playback: [6 judges][2: fast=0, late=1]
    additional_key_sounds: [[Option<StaticSoundData>; 2]; 6],
    additional_key_sound_handles: [[Option<StaticSoundHandle>; 2]; 6],
    // Background loading state
    loading_receiver: Option<mpsc::Receiver<BackgroundLoadResult>>,
    pending_load_tasks: Option<Vec<LoadTask>>,
    // JoinHandle for the keysound loader thread so we can join it on abort/dispose.
    loading_thread: Option<std::thread::JoinHandle<()>>,
    // Path sound cache for preloaded sounds (avoids blocking I/O on play_path)
    path_sound_cache: HashMap<String, StaticSoundData>,
    // Paths explicitly preloaded via preload_path() (system/UI sounds).
    // During eviction, only these entries are retained in path_sound_cache;
    // entries from deferred play_path() cache misses are cleared.
    preloaded_paths: HashSet<String>,
    // Deferred path loader for non-blocking cache-miss loads
    deferred_path_loader: crate::audio::deferred_path_loader::DeferredPathLoader,
    // Gradual loading progress counter (incremented per file in background thread)
    loading_progress: Arc<AtomicUsize>,
    // Total number of uncached paths to load (denominator for progress)
    loading_total: usize,
}

impl PortAudioDriver {
    pub fn new(song_resource_gen: i32) -> anyhow::Result<Self> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
        Ok(PortAudioDriver {
            manager,
            path_sounds: HashMap::new(),
            wav_sounds: HashMap::new(),
            wav_handles: HashMap::new(),
            global_pitch: 1.0,
            volume: 1.0,
            song_resource_gen,
            slicesound: HashMap::new(),
            slice_handles: HashMap::new(),
            wav_pitch_shifts: HashMap::new(),
            slice_pitch_shifts: HashMap::new(),
            sound_cache: HashMap::new(),
            file_cache: HashMap::new(),
            additional_key_sounds: Default::default(),
            additional_key_sound_handles: Default::default(),
            loading_receiver: None,
            pending_load_tasks: None,
            loading_thread: None,
            path_sound_cache: HashMap::new(),
            preloaded_paths: HashSet::new(),
            deferred_path_loader: crate::audio::deferred_path_loader::DeferredPathLoader::new(),
            loading_progress: Arc::new(AtomicUsize::new(0)),
            loading_total: 0,
        })
    }
}

impl AudioDriver for PortAudioDriver {
    fn play_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        if path.is_empty() {
            return;
        }

        // Stop any previously playing sound at this path
        if let Some(mut handle) = self.path_sounds.remove(path) {
            handle.stop(Tween::default());
        }

        // Check path sound cache first (populated by preload_path)
        if let Some(sound_data) = self.path_sound_cache.get(path) {
            let sound = configure_path_sound_for_play(sound_data, volume, loop_play);
            match self.manager.play(sound) {
                Ok(handle) => {
                    self.path_sounds.insert(path.to_string(), handle);
                    return;
                }
                Err(e) => {
                    log::warn!("Failed to play sound {}: {}", path, e);
                }
            }
            return;
        }

        // Cache miss: defer loading to background thread to avoid blocking
        // the render thread. The sound will be played on the next poll_loading()
        // cycle after the background load completes.
        self.deferred_path_loader
            .request_load(path, volume, loop_play);
    }

    fn set_volume_path(&mut self, path: &str, volume: f32) {
        if let Some(handle) = self.path_sounds.get_mut(path) {
            handle.set_volume(linear_to_db(volume), Tween::default());
        }
    }

    fn is_playing_path(&self, path: &str) -> bool {
        if let Some(handle) = self.path_sounds.get(path) {
            handle.state() == PlaybackState::Playing
        } else {
            self.deferred_path_loader.has_pending_play(path)
        }
    }

    fn stop_path(&mut self, path: &str) {
        if let Some(mut handle) = self.path_sounds.remove(path) {
            handle.stop(Tween::default());
        }
    }

    fn dispose_path(&mut self, path: &str) {
        self.stop_path(path);
        self.deferred_path_loader.cancel_pending_plays(path);
    }

    fn set_model(&mut self, model: &BMSModel) {
        log::info!("Loading keysound files.");

        // Clear deferred path loader: old path loads become irrelevant when
        // switching to a new model.
        self.deferred_path_loader.clear();

        // Stop all active handles before clearing (matches stop_note(None) pattern)
        for (_, handles) in self.wav_handles.drain() {
            for mut handle in handles {
                handle.stop(Tween::default());
            }
        }
        for (_, handles) in self.slice_handles.drain() {
            for mut handle in handles {
                handle.stop(Tween::default());
            }
        }
        self.wav_sounds.clear();
        self.slicesound.clear();
        self.wav_pitch_shifts.clear();
        self.slice_pitch_shifts.clear();

        // Cancel any in-progress background load: drop the receiver so the
        // loader thread's send() returns Err, then drop the handle instead of
        // joining to avoid blocking while par_iter() finishes all file I/O.
        self.loading_receiver = None;
        self.pending_load_tasks = None;
        drop(self.loading_thread.take());

        // Set volume from model's volwav
        let volwav = model.volwav;
        if volwav > 0 && volwav < 100 {
            self.volume = volwav as f32 / 100.0;
        } else {
            self.volume = 1.0;
        }

        let wav_list = &model.wavmap;
        if wav_list.is_empty() {
            return;
        }

        // Get BMS directory from model path
        let bms_dir = model
            .path()
            .and_then(|p| Path::new(&p).parent().map(|d| d.to_path_buf()));

        // Collect notes by wav ID, deduplicating by (starttime, duration)
        // Translated from AbstractAudioDriver.addNoteList()
        let mut notemap: HashMap<i32, Vec<(i64, i64)>> = HashMap::new();
        let lanes = model.mode().map(|m| m.key()).unwrap_or(0);
        for tl in &model.timelines {
            for i in 0..lanes {
                if let Some(n) = tl.note(i) {
                    add_note_entry(&mut notemap, n);
                    for ln in n.layered_notes() {
                        add_note_entry(&mut notemap, ln);
                    }
                }
                if let Some(hn) = tl.hidden_note(i) {
                    add_note_entry(&mut notemap, hn);
                }
            }
            for n in tl.back_ground_notes() {
                add_note_entry(&mut notemap, n);
            }
        }

        // Prepare loading tasks: (wav_id, resolved_path, note_entries)
        let load_tasks: Vec<LoadTask> = notemap
            .iter()
            .filter_map(|(wav_id, note_entries)| {
                let wav_id_usize = *wav_id as usize;
                if wav_id_usize >= wav_list.len() {
                    return None;
                }
                let wav_path = &wav_list[wav_id_usize];
                if wav_path.is_empty() {
                    return None;
                }
                // Security: reject resource paths with directory traversal
                if !crate::audio::audio_driver::is_bms_resource_path_safe(wav_path) {
                    log::warn!("Audio file path traversal blocked: {}", wav_path);
                    return None;
                }
                let resolved = if let Some(ref dir) = bms_dir {
                    dir.join(wav_path)
                } else {
                    std::path::PathBuf::from(wav_path)
                };
                Some((
                    *wav_id,
                    resolved.to_string_lossy().to_string(),
                    note_entries.clone(),
                ))
            })
            .collect();

        // Check file_cache for each unique path, collect uncached paths
        // Translated from: AudioCache.get() -- cache hit resets gen to 0
        let mut paths_to_load: HashSet<String> = HashSet::new();
        for (_, path, _) in &load_tasks {
            if let Some(entry) = self.file_cache.get_mut(path) {
                entry.generation = 0; // Cache hit: reset generation
            } else {
                paths_to_load.insert(path.clone());
            }
        }

        if paths_to_load.is_empty() {
            // All paths are cached; finalize immediately without spawning a thread
            self.loading_total = 0;
            self.finalize_load(&load_tasks);
        } else {
            // Spawn background thread for parallel loading of uncached paths
            let (tx, rx) = mpsc::channel();
            let paths_vec: Vec<String> = paths_to_load.into_iter().collect();

            self.loading_total = paths_vec.len();
            self.loading_progress = Arc::new(AtomicUsize::new(0));
            let progress_clone = Arc::clone(&self.loading_progress);

            match std::thread::Builder::new()
                .name("keysound-loader".to_string())
                .spawn(move || {
                    let newly_loaded: Vec<(String, StaticSoundData)> = paths_vec
                        .par_iter()
                        .filter_map(|abs_path| {
                            let result = {
                                let candidates = crate::audio::audio_driver::paths(abs_path);
                                let mut loaded = None;
                                for candidate in &candidates {
                                    if let Ok(data) = StaticSoundData::from_file(candidate) {
                                        loaded = Some((abs_path.clone(), data));
                                        break;
                                    }
                                }
                                loaded
                            };
                            progress_clone.fetch_add(1, Ordering::Relaxed);
                            if result.is_none() {
                                log::debug!("Failed to load keysound: {}", abs_path);
                            }
                            result
                        })
                        .collect();

                    let _ = tx.send(BackgroundLoadResult { newly_loaded });
                }) {
                Ok(thread_handle) => {
                    self.loading_receiver = Some(rx);
                    self.pending_load_tasks = Some(load_tasks);
                    self.loading_thread = Some(thread_handle);
                }
                Err(e) => {
                    log::warn!("Failed to spawn keysound-loader thread: {}", e);
                    self.finalize_load(&load_tasks);
                }
            }
        }
    }

    fn set_additional_key_sound(&mut self, judge: i32, fast: bool, path: Option<&str>) {
        if !(0..6).contains(&judge) {
            return;
        }
        let j = judge as usize;
        let idx = if fast { 0 } else { 1 };
        match path {
            Some(p) if !p.is_empty() => {
                self.additional_key_sounds[j][idx] = self.sound(p);
            }
            _ => {
                self.additional_key_sounds[j][idx] = None;
            }
        }
    }
    fn abort(&mut self) {
        // Drop the receiver first so the background thread's send() returns Err
        // and the thread exits promptly, then drop the handle instead of joining
        // to avoid blocking while par_iter() completes remaining file I/O.
        self.loading_receiver = None;
        self.pending_load_tasks = None;
        drop(self.loading_thread.take());
    }

    fn get_progress(&self) -> f32 {
        if self.loading_receiver.is_some() {
            let total = self.loading_total.max(1);
            let done = self.loading_progress.load(Ordering::Relaxed);
            (done as f32) / (total as f32)
        } else {
            1.0
        }
    }

    fn poll_loading(&mut self) -> bool {
        // Sweep stopped handles across all wav_ids and slice keys each frame
        // to prevent unbounded growth for rarely-replayed sounds.
        self.cleanup_stopped_handles();

        // Poll deferred path sound loads (non-blocking).
        for (path, sound_data, plays) in self.deferred_path_loader.poll() {
            self.path_sound_cache
                .insert(path.clone(), sound_data.clone());
            // Intentional: play only the most recent request. Earlier queued requests
            // for the same path are discarded to avoid simultaneous playback.
            if let Some(&(volume, loop_play)) = plays.last() {
                let sound = configure_path_sound_for_play(&sound_data, volume, loop_play);
                match self.manager.play(sound) {
                    Ok(handle) => {
                        self.path_sounds.insert(path, handle);
                    }
                    Err(e) => {
                        log::warn!("Failed to play deferred sound {}: {}", path, e);
                    }
                }
            }
        }

        let Some(rx) = &self.loading_receiver else {
            return true;
        };

        match rx.try_recv() {
            Ok(result) => {
                for (path, sound) in result.newly_loaded {
                    self.file_cache.insert(
                        path,
                        FileCacheEntry {
                            sound,
                            generation: 0,
                        },
                    );
                }

                let load_tasks = self.pending_load_tasks.take().unwrap_or_default();
                self.loading_receiver = None;
                self.finalize_load(&load_tasks);
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                log::warn!("Keysound loader thread disconnected unexpectedly");
                let load_tasks = self.pending_load_tasks.take().unwrap_or_default();
                self.loading_receiver = None;
                self.finalize_load(&load_tasks);
                true
            }
        }
    }

    fn preload_path(&mut self, path: &str) {
        if path.is_empty() || self.path_sound_cache.contains_key(path) {
            return;
        }
        // Track explicitly preloaded paths so evict_old_cache() preserves them.
        self.preloaded_paths.insert(path.to_string());
        // Route through DeferredPathLoader to avoid blocking the main thread.
        // The sound will be cached on the next poll_loading() cycle.
        self.deferred_path_loader.request_preload(path);
    }

    fn play_note(&mut self, n: &Note, volume: f32, pitch: i32) {
        self.play_note_internal(n, self.volume * volume, pitch);
        for ln in n.layered_notes() {
            self.play_note_internal(ln, self.volume * volume, pitch);
        }
    }

    fn play_judge(&mut self, judge: i32, fast: bool) {
        if !(0..6).contains(&judge) {
            return;
        }
        let j = judge as usize;
        let idx = if fast { 0 } else { 1 };
        if let Some(sound_data) = &self.additional_key_sounds[j][idx] {
            // Stop previous handle
            if let Some(mut handle) = self.additional_key_sound_handles[j][idx].take() {
                handle.stop(Tween::default());
            }
            let sound = configure_sound_for_play(sound_data, self.volume);
            match self.manager.play(sound) {
                Ok(handle) => {
                    self.additional_key_sound_handles[j][idx] = Some(handle);
                }
                Err(e) => {
                    log::warn!("Failed to play judge sound {}: {}", judge, e);
                }
            }
        }
    }

    fn stop_note(&mut self, n: Option<&Note>) {
        match n {
            None => {
                // Stop all keysound handles
                for (_, handles) in self.wav_handles.drain() {
                    for mut handle in handles {
                        handle.stop(Tween::default());
                    }
                }
                for (_, handles) in self.slice_handles.drain() {
                    for mut handle in handles {
                        handle.stop(Tween::default());
                    }
                }
                self.wav_pitch_shifts.clear();
                self.slice_pitch_shifts.clear();
            }
            Some(note) => {
                self.stop_note_internal(note);
                for ln in note.layered_notes() {
                    self.stop_note_internal(ln);
                }
            }
        }
    }

    fn set_volume_note(&mut self, n: &Note, volume: f32) {
        self.set_volume_note_internal(n, volume);
        for ln in n.layered_notes() {
            self.set_volume_note_internal(ln, volume);
        }
    }

    fn set_global_pitch(&mut self, pitch: f32) {
        self.global_pitch = pitch;
        let base = pitch as f64;
        for (wav_id, handles) in self.wav_handles.iter_mut() {
            let rate = match self.wav_pitch_shifts.get(wav_id) {
                Some(&shift) if shift != 0 => {
                    PlaybackRate(base * 2.0_f64.powf(shift as f64 / 12.0))
                }
                _ => PlaybackRate(base),
            };
            for handle in handles {
                handle.set_playback_rate(rate, Tween::default());
            }
        }
        for (key, handles) in self.slice_handles.iter_mut() {
            let rate = match self.slice_pitch_shifts.get(key) {
                Some(&shift) if shift != 0 => {
                    PlaybackRate(base * 2.0_f64.powf(shift as f64 / 12.0))
                }
                _ => PlaybackRate(base),
            };
            for handle in handles {
                handle.set_playback_rate(rate, Tween::default());
            }
        }
    }

    fn get_global_pitch(&self) -> f32 {
        self.global_pitch
    }

    fn dispose_old(&mut self) {
        self.evict_old_cache();
    }

    fn dispose(&mut self) {
        // Clear deferred path loader first to drain pending state and join
        // finished background threads before tearing down the rest of the driver.
        self.deferred_path_loader.clear();
        // Drop the receiver first so the background thread's send() returns Err
        // and the thread exits promptly, then drop the handle instead of joining
        // to avoid blocking while par_iter() completes remaining file I/O.
        self.loading_receiver = None;
        self.pending_load_tasks = None;
        drop(self.loading_thread.take());
        // Stop all active handles before clearing (mirrors set_model() pattern).
        // Without this, sounds continue playing after the driver is disposed.
        for (_, mut handle) in self.path_sounds.drain() {
            handle.stop(Tween::default());
        }
        for (_, handles) in self.wav_handles.drain() {
            for mut handle in handles {
                handle.stop(Tween::default());
            }
        }
        for (_, handles) in self.slice_handles.drain() {
            for mut handle in handles {
                handle.stop(Tween::default());
            }
        }
        for row in &mut self.additional_key_sound_handles {
            for handle in row.iter_mut().flatten() {
                handle.stop(Tween::default());
            }
        }
        self.wav_sounds.clear();
        self.slicesound.clear();
        self.wav_pitch_shifts.clear();
        self.slice_pitch_shifts.clear();
        self.sound_cache.clear();
        self.file_cache.clear();
        self.additional_key_sounds = Default::default();
        self.additional_key_sound_handles = Default::default();
        self.path_sound_cache.clear();
        self.preloaded_paths.clear();
    }
}

impl PortAudioDriver {
    /// Finalize keysound loading: build wav_sounds/slicesound from file_cache
    /// and run generational eviction.
    fn finalize_load(&mut self, load_tasks: &[LoadTask]) {
        for (wav_id, path, note_entries) in load_tasks {
            if let Some(entry) = self.file_cache.get(path) {
                let base_sound = &entry.sound;
                for &(starttime, duration) in note_entries {
                    if starttime == 0 && duration == 0 {
                        self.wav_sounds.insert(*wav_id, base_sound.clone());
                    } else {
                        let sample_rate = base_sound.sample_rate as i64;
                        // Clamp negative values to 0 to prevent wrapping to usize::MAX.
                        let start_frame = (starttime.max(0) * sample_rate / 1_000_000) as usize;
                        let duration_frames = (duration.max(0) * sample_rate / 1_000_000) as usize;
                        let total_frames = base_sound.frames.len();
                        let end_frame = if duration_frames == 0 {
                            total_frames
                        } else {
                            (start_frame + duration_frames).min(total_frames)
                        };

                        if start_frame < total_frames {
                            let mut sliced = base_sound.clone();
                            sliced.slice = Some((start_frame, end_frame));

                            self.slicesound
                                .entry(*wav_id)
                                .or_default()
                                .push(SliceWav::new(starttime, duration, sliced));
                        }
                    }
                }
            }
        }

        self.evict_old_cache();

        log::info!(
            "Keysound loading complete. Loaded: {} (sliced: {}) cache: {}",
            self.wav_sounds.len(),
            self.slicesound.values().map(|v| v.len()).sum::<usize>(),
            self.file_cache.len()
        );
    }

    /// Load and cache a sound from path.
    /// Translated from AbstractAudioDriver.getSound()
    fn sound(&mut self, path: &str) -> Option<StaticSoundData> {
        if path.is_empty() {
            return None;
        }
        if let Some(data) = self.sound_cache.get(path) {
            return Some(data.clone());
        }
        let candidates = crate::audio::audio_driver::paths(path);
        for candidate in &candidates {
            if let Ok(sound_data) = StaticSoundData::from_file(candidate) {
                self.sound_cache
                    .insert(path.to_string(), sound_data.clone());
                return Some(sound_data);
            }
        }
        None
    }

    /// Apply pitch shift to a sound handle, composing per-note semitone shift
    /// with the current global pitch: rate = global_pitch * 2^(shift/12).
    fn apply_pitch(&self, handle: &mut StaticSoundHandle, pitch_shift: i32) {
        let base = self.global_pitch as f64;
        if pitch_shift != 0 {
            let rate = base * 2.0_f64.powf(pitch_shift as f64 / 12.0);
            handle.set_playback_rate(PlaybackRate(rate), Tween::default());
        } else if (self.global_pitch - 1.0).abs() > f32::EPSILON {
            handle.set_playback_rate(PlaybackRate(base), Tween::default());
        }
    }

    /// Sweep all wav_handles and slice_handles, removing entries whose playback
    /// has stopped. Without this, handles for rarely-replayed wav_ids accumulate
    /// indefinitely because the per-push retain in play_note_internal only prunes
    /// when the same wav_id is played again.
    fn cleanup_stopped_handles(&mut self) {
        self.wav_handles.retain(|_, handles| {
            handles.retain(|h| h.state() != PlaybackState::Stopped);
            !handles.is_empty()
        });
        self.slice_handles.retain(|_, handles| {
            handles.retain(|h| h.state() != PlaybackState::Stopped);
            !handles.is_empty()
        });
    }

    /// Generational cache eviction.
    /// Translated from: ResourcePool.disposeOld() / AudioCache.disposeOld()
    fn evict_old_cache(&mut self) {
        let prev_size = self.file_cache.len();
        let maxgen = self.song_resource_gen.max(1);
        self.file_cache.retain(|_, entry| {
            if entry.generation >= maxgen {
                false
            } else {
                entry.generation += 1;
                true
            }
        });
        let released = prev_size - self.file_cache.len();
        if released > 0 {
            log::info!(
                "AudioCache capacity: {} released: {}",
                self.file_cache.len(),
                released
            );
        }

        // Clear sound_cache (from set_additional_key_sound / getSound) to prevent
        // unbounded growth across songs.
        self.sound_cache.clear();

        // Evict path_sound_cache entries from deferred play_path() cache misses
        // (e.g. preview sounds for every browsed song). Only retain entries that
        // were explicitly preloaded via preload_path() (system/UI sounds).
        self.path_sound_cache
            .retain(|path, _| self.preloaded_paths.contains(path));
    }

    /// Play a single note's keysound (without layered notes).
    /// Translated from AbstractAudioDriver.play0()
    fn play_note_internal(&mut self, n: &Note, volume: f32, pitch_shift: i32) {
        let wav_id = n.wav();
        if wav_id < 0 {
            return;
        }

        let starttime = n.micro_starttime();
        let duration = n.micro_duration();

        // Check for sliced sound first
        if (starttime != 0 || duration != 0)
            && let Some(slices) = self.slicesound.get(&wav_id)
        {
            for slice in slices {
                if slice.starttime == starttime && slice.duration == duration {
                    let key = (wav_id, starttime, duration);
                    let sound = configure_sound_for_play(&slice.wav, volume);
                    match self.manager.play(sound) {
                        Ok(mut handle) => {
                            self.apply_pitch(&mut handle, pitch_shift);
                            let handles = self.slice_handles.entry(key).or_default();
                            handles.retain(|h| h.state() != PlaybackState::Stopped);
                            // Cap at 256 handles per key, matching Java's ring buffer size.
                            if handles.len() >= 256 {
                                handles.remove(0);
                            }
                            handles.push(handle);
                            if pitch_shift != 0 {
                                self.slice_pitch_shifts.insert(key, pitch_shift);
                            } else {
                                self.slice_pitch_shifts.remove(&key);
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to play sliced keysound wav {}: {}", wav_id, e);
                        }
                    }
                    return;
                }
            }
        }

        // Non-sliced: play full sound.
        // Push new handle into the Vec so that stop_note can stop all instances
        // of the same wav_id (matches Java's 256-slot ring buffer semantics).
        if let Some(sound_data) = self.wav_sounds.get(&wav_id) {
            let sound = configure_sound_for_play(sound_data, volume);
            match self.manager.play(sound) {
                Ok(mut handle) => {
                    self.apply_pitch(&mut handle, pitch_shift);
                    let handles = self.wav_handles.entry(wav_id).or_default();
                    handles.retain(|h| h.state() != PlaybackState::Stopped);
                    // Cap at 256 handles per key, matching Java's ring buffer size.
                    if handles.len() >= 256 {
                        handles.remove(0);
                    }
                    handles.push(handle);
                    if pitch_shift != 0 {
                        self.wav_pitch_shifts.insert(wav_id, pitch_shift);
                    } else {
                        self.wav_pitch_shifts.remove(&wav_id);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to play keysound wav {}: {}", wav_id, e);
                }
            }
        }
    }

    /// Stop a single note's keysound (without layered notes).
    /// Translated from AbstractAudioDriver.stop0()
    /// Drains and stops ALL handles for the wav_id (or slice key),
    /// matching Java's ring-buffer semantics where stop iterates all slots.
    fn stop_note_internal(&mut self, n: &Note) {
        let wav_id = n.wav();
        if wav_id < 0 {
            return;
        }

        let starttime = n.micro_starttime();
        let duration = n.micro_duration();

        if starttime != 0 || duration != 0 {
            let key = (wav_id, starttime, duration);
            if let Some(handles) = self.slice_handles.remove(&key) {
                for mut handle in handles {
                    handle.stop(Tween::default());
                }
                self.slice_pitch_shifts.remove(&key);
                return;
            }
        }

        if let Some(handles) = self.wav_handles.remove(&wav_id) {
            for mut handle in handles {
                handle.stop(Tween::default());
            }
            self.wav_pitch_shifts.remove(&wav_id);
        }
    }

    /// Set volume on a single note's keysound (without layered notes).
    /// Translated from AbstractAudioDriver.setVolume0()
    fn set_volume_note_internal(&mut self, n: &Note, volume: f32) {
        let wav_id = n.wav();
        if wav_id < 0 {
            return;
        }

        let starttime = n.micro_starttime();
        let duration = n.micro_duration();

        if starttime != 0 || duration != 0 {
            let key = (wav_id, starttime, duration);
            if let Some(handles) = self.slice_handles.get_mut(&key) {
                for handle in handles {
                    handle.set_volume(linear_to_db(volume), Tween::default());
                }
                return;
            }
        }

        if let Some(handles) = self.wav_handles.get_mut(&wav_id) {
            for handle in handles {
                handle.set_volume(linear_to_db(volume), Tween::default());
            }
        }
    }
}

impl Drop for PortAudioDriver {
    fn drop(&mut self) {
        self.dispose();
    }
}

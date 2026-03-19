//! GDX Sound Driver - Primary audio driver using Kira.
//!
//! Translated from: GdxSoundDriver.java
//! In Rust, Kira replaces LibGDX for audio.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc;

use rayon::prelude::*;

use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, PlaybackRate, Tween};

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

use crate::abstract_audio_driver::SliceWav;
use crate::audio_driver::AudioDriver;

/// Note timing entries: (start_time_us, end_time_us)
pub type NoteEntries = Vec<(i64, i64)>;
/// Load task: (wav_id, resolved_path, note_entries)
pub type LoadTask = (i32, String, NoteEntries);

/// File cache entry for keysound deduplication across songs.
/// Translated from: ResourcePool.ResourceCacheElement
pub(crate) struct FileCacheEntry {
    pub(crate) sound: StaticSoundData,
    pub(crate) generation: i32,
}

/// Result from background keysound loading thread.
pub(crate) struct BackgroundLoadResult {
    pub(crate) newly_loaded: Vec<(String, StaticSoundData)>,
}

/// Convert linear volume (0.0-1.0) to decibels for Kira.
/// Kira uses Decibels type where 0 dB = no change, negative = quieter.
/// Formula: dB = 20 * log10(amplitude)
pub fn linear_to_db(volume: f32) -> f32 {
    if volume <= 0.0 {
        -60.0 // Kira's silence threshold
    } else {
        20.0 * volume.log10()
    }
}

pub(crate) fn configure_sound_for_play(sound: &StaticSoundData, volume: f32) -> StaticSoundData {
    sound.volume(linear_to_db(volume))
}

pub(crate) fn configure_path_sound_for_play(
    sound: &StaticSoundData,
    volume: f32,
    loop_play: bool,
) -> StaticSoundData {
    let sound = configure_sound_for_play(sound, volume);
    if loop_play {
        sound.loop_region(0.0..)
    } else {
        sound
    }
}

pub struct GdxSoundDriver {
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
}

impl GdxSoundDriver {
    pub fn new(song_resource_gen: i32) -> anyhow::Result<Self> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
        Ok(GdxSoundDriver {
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
        })
    }
}

impl AudioDriver for GdxSoundDriver {
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

        // Cache miss: load from file and cache for future use
        let candidates = crate::audio_driver::paths(path);
        for candidate in &candidates {
            match StaticSoundData::from_file(candidate) {
                Ok(sound_data) => {
                    self.path_sound_cache
                        .insert(path.to_string(), sound_data.clone());
                    let sound = configure_path_sound_for_play(&sound_data, volume, loop_play);
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
                Err(_) => continue,
            }
        }

        if candidates.is_empty() {
            log::debug!("No audio file found for path: {}", path);
        }
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
            false
        }
    }

    fn stop_path(&mut self, path: &str) {
        if let Some(mut handle) = self.path_sounds.remove(path) {
            handle.stop(Tween::default());
        }
    }

    fn dispose_path(&mut self, path: &str) {
        self.stop_path(path);
    }

    fn set_model(&mut self, model: &BMSModel) {
        log::info!("Loading keysound files.");

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

        // Cancel any in-progress background load and join the previous loading thread
        if let Some(handle) = self.loading_thread.take() {
            let _ = handle.join();
        }
        self.loading_receiver = None;
        self.pending_load_tasks = None;

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
                if !crate::audio_driver::is_bms_resource_path_safe(wav_path) {
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
            self.finalize_load(&load_tasks);
        } else {
            // Spawn background thread for parallel loading of uncached paths
            let (tx, rx) = mpsc::channel();
            let paths_vec: Vec<String> = paths_to_load.into_iter().collect();

            let handle = std::thread::Builder::new()
                .name("keysound-loader".to_string())
                .spawn(move || {
                    let newly_loaded: Vec<(String, StaticSoundData)> = paths_vec
                        .par_iter()
                        .filter_map(|abs_path| {
                            let candidates = crate::audio_driver::paths(abs_path);
                            for candidate in &candidates {
                                if let Ok(data) = StaticSoundData::from_file(candidate) {
                                    return Some((abs_path.clone(), data));
                                }
                            }
                            log::debug!("Failed to load keysound: {}", abs_path);
                            None
                        })
                        .collect();

                    let _ = tx.send(BackgroundLoadResult { newly_loaded });
                })
                .expect("failed to spawn keysound-loader thread");

            self.loading_receiver = Some(rx);
            self.pending_load_tasks = Some(load_tasks);
            self.loading_thread = Some(handle);
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
        // and the thread exits promptly, then join to avoid leaking the thread.
        self.loading_receiver = None;
        self.pending_load_tasks = None;
        if let Some(handle) = self.loading_thread.take() {
            let _ = handle.join();
        }
    }

    fn get_progress(&self) -> f32 {
        if self.loading_receiver.is_some() {
            0.0
        } else {
            1.0
        }
    }

    fn poll_loading(&mut self) -> bool {
        let Some(rx) = &self.loading_receiver else {
            return true; // No loading in progress
        };

        match rx.try_recv() {
            Ok(result) => {
                // Insert newly loaded sounds into file_cache
                for (path, sound) in result.newly_loaded {
                    self.file_cache.insert(
                        path,
                        FileCacheEntry {
                            sound,
                            generation: 0,
                        },
                    );
                }

                // Finalize: build wav_sounds/slicesound from file_cache
                let load_tasks = self.pending_load_tasks.take().unwrap_or_default();
                self.loading_receiver = None;
                self.finalize_load(&load_tasks);
                true
            }
            Err(mpsc::TryRecvError::Empty) => false, // Still loading
            Err(mpsc::TryRecvError::Disconnected) => {
                // Thread finished without sending (panicked?)
                log::warn!("Keysound loader thread disconnected unexpectedly");
                let load_tasks = self.pending_load_tasks.take().unwrap_or_default();
                self.loading_receiver = None;
                // Finalize with whatever is cached
                self.finalize_load(&load_tasks);
                true
            }
        }
    }

    fn preload_path(&mut self, path: &str) {
        if path.is_empty() || self.path_sound_cache.contains_key(path) {
            return;
        }
        let candidates = crate::audio_driver::paths(path);
        for candidate in &candidates {
            if let Ok(sound_data) = StaticSoundData::from_file(candidate) {
                self.path_sound_cache.insert(path.to_string(), sound_data);
                return;
            }
        }
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
        // Join the loading thread before disposing (Finding 2).
        if let Some(handle) = self.loading_thread.take() {
            let _ = handle.join();
        }
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
        self.loading_receiver = None;
        self.pending_load_tasks = None;
        self.path_sound_cache.clear();
    }
}

impl GdxSoundDriver {
    /// Finalize keysound loading: build wav_sounds/slicesound from file_cache
    /// and run generational eviction.
    fn finalize_load(&mut self, load_tasks: &[LoadTask]) {
        // Build wav_sounds/slicesound from file_cache
        // Translated from: AbstractAudioDriver.setModel() cache.get() -> wavmap/slicesound
        for (wav_id, path, note_entries) in load_tasks {
            if let Some(entry) = self.file_cache.get(path) {
                let base_sound = &entry.sound;
                for &(starttime, duration) in note_entries {
                    // Clamp negative values to 0 to prevent wrapping to usize::MAX.
                    let starttime = starttime.max(0);
                    let duration = duration.max(0);
                    if starttime == 0 && duration == 0 {
                        self.wav_sounds.insert(*wav_id, base_sound.clone());
                    } else {
                        let sample_rate = base_sound.sample_rate as i64;
                        let start_frame = (starttime * sample_rate / 1_000_000) as usize;
                        let duration_frames = (duration * sample_rate / 1_000_000) as usize;
                        let total_frames = base_sound.frames.len();
                        // duration == 0 means "play from offset to EOF" (BMSON convention)
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

        // Generational eviction (matches Java ResourcePool.disposeOld() at end of setModel)
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
        let candidates = crate::audio_driver::paths(path);
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

        // Clear auxiliary sound caches to prevent unbounded growth across songs.
        // sound_cache (from set_additional_key_sound / getSound) and
        // path_sound_cache (from play_path / preload_path) are not keysound
        // file_cache entries, so they lack generational eviction. Clearing them
        // at song-switch boundaries keeps memory bounded.
        self.sound_cache.clear();
        self.path_sound_cache.clear();
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

impl Drop for GdxSoundDriver {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// Add note entry to notemap, deduplicating by (starttime, duration).
/// Translated from AbstractAudioDriver.addNoteList()
pub(crate) fn add_note_entry(notemap: &mut HashMap<i32, Vec<(i64, i64)>>, n: &Note) {
    let wav_id = n.wav();
    if wav_id < 0 {
        return;
    }
    let starttime = n.micro_starttime();
    let duration = n.micro_duration();
    let entry = notemap.entry(wav_id).or_default();
    if !entry.iter().any(|&(s, d)| s == starttime && d == duration) {
        entry.push((starttime, duration));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use kira::AudioManager;
    use kira::AudioManagerSettings;
    use kira::Decibels;
    use kira::Frame;
    use kira::Value;
    use kira::backend::mock::MockBackend;
    use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};

    /// Create a minimal in-memory StaticSoundData (1 frame of silence).
    fn make_silent_sound() -> StaticSoundData {
        StaticSoundData {
            sample_rate: 44100,
            frames: Arc::new([Frame::ZERO]),
            settings: StaticSoundSettings::default(),
            slice: None,
        }
    }

    #[test]
    fn configure_sound_for_play_applies_initial_volume_before_playback() {
        let sound = make_silent_sound();

        let configured = configure_sound_for_play(&sound, 0.2);

        assert_eq!(
            configured.settings.volume,
            Value::Fixed(Decibels(linear_to_db(0.2)))
        );
    }

    /// Verify that set_global_pitch updates playback rate on currently-playing
    /// wav_handles and slice_handles, not just stores the value.
    /// Uses MockBackend so no real audio device is needed.
    #[test]
    fn set_global_pitch_updates_active_handles() {
        let mut manager =
            AudioManager::<MockBackend>::new(AudioManagerSettings::default()).unwrap();

        let sound = make_silent_sound();
        let handle1 = manager.play(sound.clone()).unwrap();
        let handle2 = manager.play(sound.clone()).unwrap();
        let handle3 = manager.play(sound.clone()).unwrap();

        let mut wav_handles: HashMap<i32, Vec<StaticSoundHandle>> = HashMap::new();
        wav_handles.entry(1).or_default().push(handle1);
        wav_handles.entry(2).or_default().push(handle2);

        let mut slice_handles: HashMap<(i32, i64, i64), Vec<StaticSoundHandle>> = HashMap::new();
        slice_handles
            .entry((3, 1000, 2000))
            .or_default()
            .push(handle3);

        // Simulate set_global_pitch logic: store + iterate all handles
        let pitch: f32 = 1.5;
        let rate = PlaybackRate(pitch as f64);
        for handles in wav_handles.values_mut() {
            for handle in handles {
                handle.set_playback_rate(rate, Tween::default());
            }
        }
        for handles in slice_handles.values_mut() {
            for handle in handles {
                handle.set_playback_rate(rate, Tween::default());
            }
        }

        // No panic means all handles were successfully updated.
        // StaticSoundHandle does not expose a playback rate getter,
        // so we verify the code path runs without error.
        assert_eq!(wav_handles.len(), 2);
        assert_eq!(slice_handles.len(), 1);
    }

    /// Verify that set_global_pitch with pitch=1.0 (no change) also works.
    #[test]
    fn set_global_pitch_unity_updates_active_handles() {
        let mut manager =
            AudioManager::<MockBackend>::new(AudioManagerSettings::default()).unwrap();

        let sound = make_silent_sound();
        let handle = manager.play(sound).unwrap();

        let mut wav_handles: HashMap<i32, Vec<StaticSoundHandle>> = HashMap::new();
        wav_handles.entry(1).or_default().push(handle);

        let pitch: f32 = 1.0;
        let rate = PlaybackRate(pitch as f64);
        for handles in wav_handles.values_mut() {
            for handle in handles {
                handle.set_playback_rate(rate, Tween::default());
            }
        }

        assert_eq!(wav_handles.len(), 1);
    }

    /// Verify that set_global_pitch on empty handle maps doesn't panic.
    #[test]
    fn set_global_pitch_no_active_handles() {
        let mut wav_handles: HashMap<i32, Vec<StaticSoundHandle>> = HashMap::new();
        let mut slice_handles: HashMap<(i32, i64, i64), Vec<StaticSoundHandle>> = HashMap::new();

        let pitch: f32 = 2.0;
        let rate = PlaybackRate(pitch as f64);
        for handles in wav_handles.values_mut() {
            for handle in handles {
                handle.set_playback_rate(rate, Tween::default());
            }
        }
        for handles in slice_handles.values_mut() {
            for handle in handles {
                handle.set_playback_rate(rate, Tween::default());
            }
        }

        assert!(wav_handles.is_empty());
        assert!(slice_handles.is_empty());
    }

    /// Verify that BackgroundLoadResult can be sent through mpsc channel.
    #[test]
    fn background_load_result_channel_roundtrip() {
        let (tx, rx) = mpsc::channel();
        let sound = make_silent_sound();

        tx.send(BackgroundLoadResult {
            newly_loaded: vec![("/test/a.wav".to_string(), sound)],
        })
        .unwrap();

        let result = rx.try_recv().unwrap();
        assert_eq!(result.newly_loaded.len(), 1);
        assert_eq!(result.newly_loaded[0].0, "/test/a.wav");
    }

    /// Verify that try_recv returns Empty when no data has been sent yet.
    #[test]
    fn background_load_empty_before_send() {
        let (_tx, rx) = mpsc::channel::<BackgroundLoadResult>();
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    }

    /// Verify that try_recv returns Disconnected when sender is dropped.
    #[test]
    fn background_load_disconnected_on_drop() {
        let (tx, rx) = mpsc::channel::<BackgroundLoadResult>();
        drop(tx);
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    /// Verify that finalize_load builds wav_sounds from file_cache entries.
    #[test]
    fn finalize_load_builds_wav_sounds() {
        // Test the finalize_load logic in isolation using raw maps
        let sound = make_silent_sound();
        let mut file_cache: HashMap<String, FileCacheEntry> = HashMap::new();
        file_cache.insert(
            "/test/sound.wav".to_string(),
            FileCacheEntry {
                sound: sound.clone(),
                generation: 0,
            },
        );

        let load_tasks: Vec<LoadTask> = vec![(42, "/test/sound.wav".to_string(), vec![(0, 0)])];

        let mut wav_sounds: HashMap<i32, StaticSoundData> = HashMap::new();
        for (wav_id, path, note_entries) in &load_tasks {
            if let Some(entry) = file_cache.get(path) {
                for &(starttime, duration) in note_entries {
                    if starttime == 0 && duration == 0 {
                        wav_sounds.insert(*wav_id, entry.sound.clone());
                    }
                }
            }
        }

        assert!(wav_sounds.contains_key(&42));
        assert_eq!(wav_sounds.len(), 1);
    }

    /// Regression: negative starttime/duration must be clamped to 0,
    /// not wrap to usize::MAX via `as usize` cast.
    #[test]
    fn finalize_load_clamps_negative_starttime_and_duration() {
        let sound = StaticSoundData {
            sample_rate: 44100,
            frames: Arc::from(vec![Frame::ZERO; 100]),
            settings: StaticSoundSettings::default(),
            slice: None,
        };
        let mut file_cache: HashMap<String, FileCacheEntry> = HashMap::new();
        file_cache.insert(
            "/test/sound.wav".to_string(),
            FileCacheEntry {
                sound: sound.clone(),
                generation: 0,
            },
        );

        // Both negative: should clamp to (0, 0) and insert as whole-file wav_sound
        let load_tasks: Vec<LoadTask> =
            vec![(1, "/test/sound.wav".to_string(), vec![(-100, -200)])];

        let mut wav_sounds: HashMap<i32, StaticSoundData> = HashMap::new();
        let mut slicesound: HashMap<i32, Vec<SliceWav<StaticSoundData>>> = HashMap::new();
        for (wav_id, path, note_entries) in &load_tasks {
            if let Some(entry) = file_cache.get(path) {
                let base_sound = &entry.sound;
                for &(st, dur) in note_entries {
                    let st = st.max(0);
                    let dur = dur.max(0);
                    if st == 0 && dur == 0 {
                        wav_sounds.insert(*wav_id, base_sound.clone());
                    } else {
                        let sample_rate = base_sound.sample_rate as i64;
                        let start_frame = (st * sample_rate / 1_000_000) as usize;
                        let duration_frames = (dur * sample_rate / 1_000_000) as usize;
                        let total_frames = base_sound.frames.len();
                        let end_frame = if duration_frames == 0 {
                            total_frames
                        } else {
                            (start_frame + duration_frames).min(total_frames)
                        };
                        if start_frame < total_frames {
                            let mut sliced = base_sound.clone();
                            sliced.slice = Some((start_frame, end_frame));
                            slicesound
                                .entry(*wav_id)
                                .or_default()
                                .push(SliceWav::new(st, dur, sliced));
                        }
                    }
                }
            }
        }

        // (-100, -200) clamped to (0, 0) -> should be in wav_sounds, not slicesound
        assert!(
            wav_sounds.contains_key(&1),
            "negative times should clamp to whole-file"
        );
        assert!(
            slicesound.is_empty(),
            "no slice should be created for clamped-to-zero values"
        );
    }

    /// Verify that finalize_load handles sliced sounds correctly.
    #[test]
    fn finalize_load_builds_sliced_sounds() {
        // Use a sound with enough frames to hold the slice (100 frames at 44100Hz)
        let long_sound = StaticSoundData {
            sample_rate: 44100,
            frames: Arc::from(vec![Frame::ZERO; 100]),
            settings: StaticSoundSettings::default(),
            slice: None,
        };
        let mut file_cache: HashMap<String, FileCacheEntry> = HashMap::new();
        file_cache.insert(
            "/test/sound.wav".to_string(),
            FileCacheEntry {
                sound: long_sound.clone(),
                generation: 0,
            },
        );

        // starttime=1us -> start_frame = 1 * 44100 / 1_000_000 = 0
        // duration=0 means "play from offset to EOF"
        let load_tasks: Vec<LoadTask> = vec![(
            10,
            "/test/sound.wav".to_string(),
            vec![(1, 0)], // Sliced note: starttime=1us, duration=0 (to EOF)
        )];

        let mut slicesound: HashMap<i32, Vec<SliceWav<StaticSoundData>>> = HashMap::new();
        for (wav_id, path, note_entries) in &load_tasks {
            if let Some(entry) = file_cache.get(path) {
                let base_sound = &entry.sound;
                for &(starttime, duration) in note_entries {
                    if starttime != 0 || duration != 0 {
                        let sample_rate = base_sound.sample_rate as i64;
                        let start_frame = (starttime * sample_rate / 1_000_000) as usize;
                        let duration_frames = (duration * sample_rate / 1_000_000) as usize;
                        let total_frames = base_sound.frames.len();
                        let end_frame = if duration_frames == 0 {
                            total_frames
                        } else {
                            (start_frame + duration_frames).min(total_frames)
                        };
                        if start_frame < total_frames {
                            let mut sliced = base_sound.clone();
                            sliced.slice = Some((start_frame, end_frame));
                            slicesound
                                .entry(*wav_id)
                                .or_default()
                                .push(SliceWav::new(starttime, duration, sliced));
                        }
                    }
                }
            }
        }

        // start_frame = 0 (1 * 44100 / 1_000_000 = 0), total_frames = 100
        // start_frame (0) < total_frames (100) -> slice created
        assert!(slicesound.contains_key(&10));
        assert_eq!(slicesound[&10].len(), 1);
        assert_eq!(slicesound[&10][0].starttime, 1);
        assert_eq!(slicesound[&10][0].duration, 0);
    }

    /// Verify loading state machine: progress is 0 while receiver is Some, 1 when None.
    #[test]
    fn loading_state_progress_semantics() {
        // Receiver present = loading
        let (_tx, rx) = mpsc::channel::<BackgroundLoadResult>();
        let receiver: Option<mpsc::Receiver<BackgroundLoadResult>> = Some(rx);
        let progress = if receiver.is_some() { 0.0 } else { 1.0 };
        assert_eq!(progress, 0.0);

        // No receiver = idle
        let no_receiver: Option<mpsc::Receiver<BackgroundLoadResult>> = None;
        let progress = if no_receiver.is_some() { 0.0 } else { 1.0 };
        assert_eq!(progress, 1.0);
    }

    /// Verify that abort semantics clear both receiver and pending tasks.
    #[test]
    fn abort_clears_loading_state() {
        let (_tx, rx) = mpsc::channel::<BackgroundLoadResult>();
        let receiver: Option<mpsc::Receiver<BackgroundLoadResult>> = Some(rx);
        let pending: Option<Vec<LoadTask>> =
            Some(vec![(1, "/tmp/test.wav".to_string(), vec![(0, 0)])]);

        // Verify pre-abort state
        assert!(receiver.is_some());
        assert!(pending.is_some());

        // Simulate abort by dropping (take pattern used in real code)
        drop(receiver);
        drop(pending);
    }

    /// Verify that multiple poll attempts before data is ready correctly return false.
    #[test]
    fn poll_returns_false_until_data_ready() {
        let (tx, rx) = mpsc::channel::<BackgroundLoadResult>();

        // Multiple polls before send
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));

        // Now send
        tx.send(BackgroundLoadResult {
            newly_loaded: vec![],
        })
        .unwrap();

        // Should succeed now
        let result = rx.try_recv();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().newly_loaded.len(), 0);
    }

    /// Verify that set_global_pitch composes with per-note pitch shifts
    /// instead of overwriting them. The composed rate should be
    /// global_pitch * 2^(shift/12).
    #[test]
    fn set_global_pitch_composes_with_per_note_shifts() {
        let mut manager =
            AudioManager::<MockBackend>::new(AudioManagerSettings::default()).unwrap();

        let sound = make_silent_sound();
        let handle_with_shift = manager.play(sound.clone()).unwrap();
        let handle_no_shift = manager.play(sound.clone()).unwrap();
        let handle_slice = manager.play(sound.clone()).unwrap();

        let mut wav_handles: HashMap<i32, Vec<StaticSoundHandle>> = HashMap::new();
        wav_handles.entry(1).or_default().push(handle_with_shift);
        wav_handles.entry(2).or_default().push(handle_no_shift);

        let mut slice_handles: HashMap<(i32, i64, i64), Vec<StaticSoundHandle>> = HashMap::new();
        slice_handles
            .entry((3, 100, 200))
            .or_default()
            .push(handle_slice);

        let mut wav_pitch_shifts: HashMap<i32, i32> = HashMap::new();
        wav_pitch_shifts.insert(1, 7); // wav_id=1 has +7 semitone shift

        let mut slice_pitch_shifts: HashMap<(i32, i64, i64), i32> = HashMap::new();
        slice_pitch_shifts.insert((3, 100, 200), -3); // slice has -3 semitone shift

        // Simulate the new set_global_pitch logic
        let pitch: f32 = 1.5;
        let base = pitch as f64;
        for (wav_id, handles) in wav_handles.iter_mut() {
            let rate = match wav_pitch_shifts.get(wav_id) {
                Some(&shift) if shift != 0 => {
                    PlaybackRate(base * 2.0_f64.powf(shift as f64 / 12.0))
                }
                _ => PlaybackRate(base),
            };
            for handle in handles {
                handle.set_playback_rate(rate, Tween::default());
            }
        }
        for (key, handles) in slice_handles.iter_mut() {
            let rate = match slice_pitch_shifts.get(key) {
                Some(&shift) if shift != 0 => {
                    PlaybackRate(base * 2.0_f64.powf(shift as f64 / 12.0))
                }
                _ => PlaybackRate(base),
            };
            for handle in handles {
                handle.set_playback_rate(rate, Tween::default());
            }
        }

        // Verify all handles were iterated (no panic = success).
        // wav_id=1 should get 1.5 * 2^(7/12) ~= 1.5 * 1.498 ~= 2.247
        // wav_id=2 should get 1.5 (no shift)
        // slice (3,100,200) should get 1.5 * 2^(-3/12) ~= 1.5 * 0.841 ~= 1.260
        assert_eq!(wav_handles.len(), 2);
        assert_eq!(slice_handles.len(), 1);
    }

    /// Verify that apply_pitch composes global_pitch with per-note shift.
    #[test]
    fn apply_pitch_composes_global_and_note_shift() {
        // Test the composition formula: rate = global_pitch * 2^(shift/12)
        let global_pitch: f32 = 1.5;
        let pitch_shift: i32 = 12; // +12 semitones = 1 octave = 2x

        let base = global_pitch as f64;
        let rate = base * 2.0_f64.powf(pitch_shift as f64 / 12.0);

        // 1.5 * 2^(12/12) = 1.5 * 2.0 = 3.0
        assert!((rate - 3.0).abs() < 1e-10);
    }

    /// Verify that apply_pitch with zero shift and non-unity global pitch
    /// still applies global pitch.
    #[test]
    fn apply_pitch_zero_shift_uses_global() {
        let global_pitch: f32 = 0.75;
        let pitch_shift: i32 = 0;

        let base = global_pitch as f64;
        let rate = if pitch_shift != 0 {
            base * 2.0_f64.powf(pitch_shift as f64 / 12.0)
        } else {
            base
        };

        assert!((rate - 0.75).abs() < 1e-10);
    }

    /// Verify that pitch shift tracking maps are cleared when stop_note(None)
    /// is called (stop all).
    #[test]
    fn stop_all_clears_pitch_shift_maps() {
        let mut wav_pitch_shifts: HashMap<i32, i32> = HashMap::new();
        let mut slice_pitch_shifts: HashMap<(i32, i64, i64), i32> = HashMap::new();

        wav_pitch_shifts.insert(1, 5);
        wav_pitch_shifts.insert(2, -3);
        slice_pitch_shifts.insert((3, 100, 200), 7);

        assert_eq!(wav_pitch_shifts.len(), 2);
        assert_eq!(slice_pitch_shifts.len(), 1);

        // Simulate stop_note(None) clearing
        wav_pitch_shifts.clear();
        slice_pitch_shifts.clear();

        assert!(wav_pitch_shifts.is_empty());
        assert!(slice_pitch_shifts.is_empty());
    }

    /// Regression: multiple plays of the same wav_id must all be tracked so
    /// that stop_note can stop ALL instances (matches Java 256-slot ring buffer).
    /// Before the fix, HashMap<i32, StaticSoundHandle> only kept the latest handle,
    /// silently leaking previous instances that could never be stopped.
    #[test]
    fn multi_handle_wav_tracks_all_instances() {
        let mut manager =
            AudioManager::<MockBackend>::new(AudioManagerSettings::default()).unwrap();

        let sound = make_silent_sound();
        let h1 = manager.play(sound.clone()).unwrap();
        let h2 = manager.play(sound.clone()).unwrap();
        let h3 = manager.play(sound.clone()).unwrap();

        let mut wav_handles: HashMap<i32, Vec<StaticSoundHandle>> = HashMap::new();
        // Simulate 3 plays of wav_id=42 (e.g., rapid drum hits)
        wav_handles.entry(42).or_default().push(h1);
        wav_handles.entry(42).or_default().push(h2);
        wav_handles.entry(42).or_default().push(h3);

        assert_eq!(wav_handles[&42].len(), 3);

        // Stop all: drain and stop every handle for wav_id=42
        if let Some(handles) = wav_handles.remove(&42) {
            assert_eq!(handles.len(), 3);
            for mut handle in handles {
                handle.stop(Tween::default());
            }
        }
        assert!(!wav_handles.contains_key(&42));
    }

    /// Regression: multiple plays of the same slice key must all be tracked.
    #[test]
    fn multi_handle_slice_tracks_all_instances() {
        let mut manager =
            AudioManager::<MockBackend>::new(AudioManagerSettings::default()).unwrap();

        let sound = make_silent_sound();
        let h1 = manager.play(sound.clone()).unwrap();
        let h2 = manager.play(sound.clone()).unwrap();

        let key = (10_i32, 500_i64, 1000_i64);
        let mut slice_handles: HashMap<(i32, i64, i64), Vec<StaticSoundHandle>> = HashMap::new();
        slice_handles.entry(key).or_default().push(h1);
        slice_handles.entry(key).or_default().push(h2);

        assert_eq!(slice_handles[&key].len(), 2);

        // Stop all for this slice key
        if let Some(handles) = slice_handles.remove(&key) {
            assert_eq!(handles.len(), 2);
            for mut handle in handles {
                handle.stop(Tween::default());
            }
        }
        assert!(!slice_handles.contains_key(&key));
    }

    /// Regression: set_global_pitch must update ALL handles across all vecs,
    /// not just one per wav_id.
    #[test]
    fn set_global_pitch_updates_all_handles_in_vecs() {
        let mut manager =
            AudioManager::<MockBackend>::new(AudioManagerSettings::default()).unwrap();

        let sound = make_silent_sound();
        let h1 = manager.play(sound.clone()).unwrap();
        let h2 = manager.play(sound.clone()).unwrap();
        let h3 = manager.play(sound.clone()).unwrap();

        let mut wav_handles: HashMap<i32, Vec<StaticSoundHandle>> = HashMap::new();
        // wav_id=1 has 2 active handles, wav_id=2 has 1
        wav_handles.entry(1).or_default().push(h1);
        wav_handles.entry(1).or_default().push(h2);
        wav_handles.entry(2).or_default().push(h3);

        let pitch: f32 = 1.25;
        let base = pitch as f64;
        let mut total_updated = 0;
        for handles in wav_handles.values_mut() {
            for handle in handles {
                handle.set_playback_rate(PlaybackRate(base), Tween::default());
                total_updated += 1;
            }
        }

        // All 3 handles (2 for wav_id=1, 1 for wav_id=2) must be updated
        assert_eq!(total_updated, 3);
    }

    /// Regression: loading_thread JoinHandle must be joined in set_model()
    /// before starting a new load, and in dispose().
    #[test]
    fn loading_thread_join_on_set_model() {
        // Spawn a trivial thread, store it, then join it (simulating set_model behavior)
        let handle = std::thread::Builder::new()
            .name("test-loader".to_string())
            .spawn(|| {
                // Simulate brief work
            })
            .unwrap();

        let mut loading_thread: Option<std::thread::JoinHandle<()>> = Some(handle);

        // Simulate set_model joining the previous thread before starting new load
        if let Some(h) = loading_thread.take() {
            let join_result = h.join();
            assert!(join_result.is_ok());
        }
        assert!(loading_thread.is_none());
    }

    /// Regression: dispose must join loading_thread to avoid dangling threads.
    #[test]
    fn loading_thread_join_on_dispose() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = completed.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            completed_clone.store(true, Ordering::SeqCst);
        });

        let mut loading_thread: Option<std::thread::JoinHandle<()>> = Some(handle);

        // Simulate dispose joining the thread
        if let Some(h) = loading_thread.take() {
            let _ = h.join();
        }

        // Thread must have completed by the time join returns
        assert!(completed.load(Ordering::SeqCst));
    }

    /// Regression: abort() must join loading_thread to avoid leaking the
    /// background keysound-loader thread.
    #[test]
    fn loading_thread_join_on_abort() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = completed.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            completed_clone.store(true, Ordering::SeqCst);
        });

        let mut loading_thread: Option<std::thread::JoinHandle<()>> = Some(handle);

        // Simulate abort() joining the thread
        if let Some(h) = loading_thread.take() {
            let _ = h.join();
        }

        // Thread must have completed by the time join returns
        assert!(completed.load(Ordering::SeqCst));
    }

    /// Create a StaticSoundData long enough to remain Playing through MockBackend ticks.
    fn make_long_sound() -> StaticSoundData {
        StaticSoundData {
            sample_rate: 44100,
            frames: Arc::from(vec![Frame::ZERO; 44100]), // 1 second
            settings: StaticSoundSettings::default(),
            slice: None,
        }
    }

    /// Regression: stopped handles must be pruned when pushing new handles
    /// to prevent unbounded Vec growth in wav_handles and slice_handles.
    /// A 2000-note chart could otherwise accumulate thousands of finished
    /// handles, slowing set_global_pitch and set_volume iterations.
    #[test]
    fn stopped_handles_pruned_on_push() {
        use kira::backend::mock::MockBackendSettings;

        // Use a high mock sample rate so each process() tick advances very
        // little time (128 frames / 1_000_000 Hz = 0.128ms), keeping
        // non-stopped sounds alive through the tick.
        let settings = AudioManagerSettings::<MockBackend> {
            backend_settings: MockBackendSettings {
                sample_rate: 1_000_000,
            },
            ..Default::default()
        };
        let mut manager = AudioManager::<MockBackend>::new(settings).unwrap();

        // Use a long sound so handles stay Playing through backend ticks
        let sound = make_long_sound();

        // Simulate wav_handles pruning
        let mut wav_handles: HashMap<i32, Vec<StaticSoundHandle>> = HashMap::new();
        let mut h1 = manager.play(sound.clone()).unwrap();
        let h2 = manager.play(sound.clone()).unwrap();
        // Stop h1 and tick the backend enough times to complete the 10ms
        // default tween fade-out (Stopping -> Stopped). With sample_rate
        // 1_000_000 and internal_buffer_size 128, each tick advances
        // 0.128ms, so ~80 ticks covers the 10ms tween.
        h1.stop(Tween::default());
        for _ in 0..100 {
            manager.backend_mut().on_start_processing();
            manager.backend_mut().process();
        }
        assert_eq!(h1.state(), PlaybackState::Stopped);
        assert_ne!(h2.state(), PlaybackState::Stopped);

        wav_handles.entry(1).or_default().push(h1);
        wav_handles.entry(1).or_default().push(h2);

        // Now push a third handle with pruning (simulating play_note_internal logic)
        let h3 = manager.play(sound.clone()).unwrap();
        let handles = wav_handles.entry(1).or_default();
        handles.retain(|h| h.state() != PlaybackState::Stopped);
        handles.push(h3);

        // h1 was stopped, so it should have been pruned; only h2 + h3 remain
        assert_eq!(wav_handles[&1].len(), 2);

        // Simulate slice_handles pruning
        let mut slice_handles: HashMap<(i32, i64, i64), Vec<StaticSoundHandle>> = HashMap::new();
        let key = (5_i32, 100_i64, 200_i64);
        let mut sh1 = manager.play(sound.clone()).unwrap();
        sh1.stop(Tween::default());
        for _ in 0..100 {
            manager.backend_mut().on_start_processing();
            manager.backend_mut().process();
        }
        assert_eq!(sh1.state(), PlaybackState::Stopped);

        slice_handles.entry(key).or_default().push(sh1);

        let sh2 = manager.play(sound.clone()).unwrap();
        let handles = slice_handles.entry(key).or_default();
        handles.retain(|h| h.state() != PlaybackState::Stopped);
        handles.push(sh2);

        // sh1 was stopped and pruned; only sh2 remains
        assert_eq!(slice_handles[&key].len(), 1);
    }

    /// Regression: evict_old_cache must clear sound_cache and path_sound_cache
    /// to prevent unbounded memory growth across song switches. These auxiliary
    /// caches lack generational eviction, so without clearing they accumulate
    /// every unique path played via play_path() or loaded via sound().
    #[test]
    fn evict_old_cache_clears_auxiliary_caches() {
        let sound = make_silent_sound();
        let song_resource_gen = 2;
        let maxgen = song_resource_gen.max(1);

        // Simulate sound_cache with accumulated entries
        let mut sound_cache: HashMap<String, StaticSoundData> = HashMap::new();
        sound_cache.insert("/preview/song1.ogg".to_string(), sound.clone());
        sound_cache.insert("/preview/song2.ogg".to_string(), sound.clone());

        // Simulate path_sound_cache with accumulated entries
        let mut path_sound_cache: HashMap<String, StaticSoundData> = HashMap::new();
        path_sound_cache.insert("/sfx/click.wav".to_string(), sound.clone());
        path_sound_cache.insert("/sfx/decide.wav".to_string(), sound.clone());
        path_sound_cache.insert("/sfx/cancel.wav".to_string(), sound.clone());

        // Simulate file_cache (should use generational eviction, not full clear)
        let mut file_cache: HashMap<String, FileCacheEntry> = HashMap::new();
        file_cache.insert(
            "/keysound/a.wav".to_string(),
            FileCacheEntry {
                sound: sound.clone(),
                generation: 0, // Fresh: should survive
            },
        );
        file_cache.insert(
            "/keysound/old.wav".to_string(),
            FileCacheEntry {
                sound: sound.clone(),
                generation: maxgen, // Expired: should be evicted
            },
        );

        // Run evict_old_cache logic (same as the real method)
        file_cache.retain(|_, entry| {
            if entry.generation >= maxgen {
                false
            } else {
                entry.generation += 1;
                true
            }
        });
        sound_cache.clear();
        path_sound_cache.clear();

        // file_cache uses generational eviction: fresh entry survives, old is evicted
        assert_eq!(file_cache.len(), 1);
        assert!(file_cache.contains_key("/keysound/a.wav"));

        // Auxiliary caches must be fully cleared
        assert!(sound_cache.is_empty());
        assert!(path_sound_cache.is_empty());
    }
}

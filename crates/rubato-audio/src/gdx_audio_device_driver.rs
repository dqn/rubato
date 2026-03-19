//! GDX Audio Device Driver - Device-level audio using Kira.
//!
//! Translated from: GdxAudioDeviceDriver.java
//! In Java, this was a stub driver extending AbstractAudioDriver with unimplemented methods.
//! In Rust, we wire it to Kira's AudioManager, following the same pattern as GdxSoundDriver.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc;

use rayon::prelude::*;

use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, PlaybackRate, Semitones, Tween};

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

use crate::abstract_audio_driver::SliceWav;
use crate::audio_driver::AudioDriver;
use crate::gdx_sound_driver::{
    BackgroundLoadResult, FileCacheEntry, LoadTask, add_note_entry, configure_path_sound_for_play,
    configure_sound_for_play, linear_to_db,
};

pub struct GdxAudioDeviceDriver {
    manager: Option<AudioManager>,
    // Map from path to sound handle
    path_sounds: HashMap<String, StaticSoundHandle>,
    // Map from wav ID to sound data for BMS keysounds
    wav_sounds: HashMap<i32, StaticSoundData>,
    wav_handles: HashMap<i32, StaticSoundHandle>,
    global_pitch: f32,
    // Model volume from volwav (0.0-1.0)
    volume: f32,
    song_resource_gen: i32,
    // Sliced sounds by wav ID (for notes with non-zero starttime/duration)
    slicesound: HashMap<i32, Vec<SliceWav<StaticSoundData>>>,
    slice_handles: HashMap<(i32, i64, i64), StaticSoundHandle>,
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

impl GdxAudioDeviceDriver {
    pub fn new(song_resource_gen: i32) -> Self {
        let manager = match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(m) => Some(m),
            Err(e) => {
                log::warn!(
                    "GdxAudioDeviceDriver: Failed to create audio manager: {}",
                    e
                );
                None
            }
        };
        GdxAudioDeviceDriver {
            manager,
            path_sounds: HashMap::new(),
            wav_sounds: HashMap::new(),
            wav_handles: HashMap::new(),
            global_pitch: 1.0,
            volume: 1.0,
            song_resource_gen,
            slicesound: HashMap::new(),
            slice_handles: HashMap::new(),
            sound_cache: HashMap::new(),
            file_cache: HashMap::new(),
            additional_key_sounds: Default::default(),
            additional_key_sound_handles: Default::default(),
            loading_receiver: None,
            pending_load_tasks: None,
            loading_thread: None,
            path_sound_cache: HashMap::new(),
        }
    }
}

impl AudioDriver for GdxAudioDeviceDriver {
    fn play_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        if path.is_empty() {
            return;
        }
        let Some(manager) = self.manager.as_mut() else {
            return;
        };

        // Stop any previously playing sound at this path
        if let Some(mut handle) = self.path_sounds.remove(path) {
            handle.stop(Tween::default());
        }

        // Check path sound cache first (populated by preload_path)
        if let Some(sound_data) = self.path_sound_cache.get(path) {
            let sound = configure_path_sound_for_play(sound_data, volume, loop_play);
            match manager.play(sound) {
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
                    match manager.play(sound) {
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
        if self.manager.is_none() {
            return;
        }

        log::info!("GdxAudioDeviceDriver: Loading keysound files.");

        // Clear previous sounds
        self.wav_sounds.clear();
        self.wav_handles.clear();
        self.slicesound.clear();
        self.slice_handles.clear();

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
            self.finalize_load(&load_tasks);
        } else {
            let (tx, rx) = mpsc::channel();
            let paths_vec: Vec<String> = paths_to_load.into_iter().collect();

            let thread_handle = std::thread::Builder::new()
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
            self.loading_thread = Some(thread_handle);
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
        let Some(manager) = self.manager.as_mut() else {
            return;
        };
        let j = judge as usize;
        let idx = if fast { 0 } else { 1 };
        if let Some(sound_data) = &self.additional_key_sounds[j][idx] {
            // Stop previous handle
            if let Some(mut handle) = self.additional_key_sound_handles[j][idx].take() {
                handle.stop(Tween::default());
            }
            let sound = configure_sound_for_play(sound_data, self.volume);
            match manager.play(sound) {
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
                for (_, mut handle) in self.wav_handles.drain() {
                    handle.stop(Tween::default());
                }
                for (_, mut handle) in self.slice_handles.drain() {
                    handle.stop(Tween::default());
                }
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
        let rate = PlaybackRate(pitch as f64);
        for handle in self.wav_handles.values_mut() {
            handle.set_playback_rate(rate, Tween::default());
        }
        for handle in self.slice_handles.values_mut() {
            handle.set_playback_rate(rate, Tween::default());
        }
    }

    fn get_global_pitch(&self) -> f32 {
        self.global_pitch
    }

    fn dispose_old(&mut self) {
        self.evict_old_cache();
    }

    fn dispose(&mut self) {
        if let Some(handle) = self.loading_thread.take() {
            let _ = handle.join();
        }
        self.path_sounds.clear();
        self.wav_sounds.clear();
        self.wav_handles.clear();
        self.slicesound.clear();
        self.slice_handles.clear();
        self.sound_cache.clear();
        self.file_cache.clear();
        self.additional_key_sounds = Default::default();
        self.additional_key_sound_handles = Default::default();
        self.loading_receiver = None;
        self.pending_load_tasks = None;
        self.path_sound_cache.clear();
    }
}

impl GdxAudioDeviceDriver {
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
            "GdxAudioDeviceDriver: Keysound loading complete. Loaded: {} (sliced: {}) cache: {}",
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

    /// Apply pitch shift to a sound handle.
    fn apply_pitch(&self, handle: &mut StaticSoundHandle, pitch_shift: i32) {
        if pitch_shift != 0 {
            handle.set_playback_rate(Semitones(pitch_shift as f64), Tween::default());
        } else if (self.global_pitch - 1.0).abs() > f32::EPSILON {
            handle.set_playback_rate(PlaybackRate(self.global_pitch as f64), Tween::default());
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
                "GdxAudioDeviceDriver: AudioCache capacity: {} released: {}",
                self.file_cache.len(),
                released
            );
        }

        // Clear auxiliary sound caches to prevent unbounded growth across songs.
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
        let Some(manager) = self.manager.as_mut() else {
            return;
        };

        let starttime = n.micro_starttime();
        let duration = n.micro_duration();

        // Check for sliced sound first
        if (starttime != 0 || duration != 0)
            && let Some(slices) = self.slicesound.get(&wav_id)
        {
            for slice in slices {
                if slice.starttime == starttime && slice.duration == duration {
                    let key = (wav_id, starttime, duration);
                    if let Some(mut old_handle) = self.slice_handles.remove(&key) {
                        old_handle.stop(Tween::default());
                    }
                    let sound = configure_sound_for_play(&slice.wav, volume);
                    match manager.play(sound) {
                        Ok(mut handle) => {
                            self.apply_pitch(&mut handle, pitch_shift);
                            self.slice_handles.insert(key, handle);
                        }
                        Err(e) => {
                            log::warn!("Failed to play sliced keysound wav {}: {}", wav_id, e);
                        }
                    }
                    return;
                }
            }
        }

        // Non-sliced: play full sound
        if let Some(sound_data) = self.wav_sounds.get(&wav_id) {
            if let Some(mut old_handle) = self.wav_handles.remove(&wav_id) {
                old_handle.stop(Tween::default());
            }
            let sound = configure_sound_for_play(sound_data, volume);
            match manager.play(sound) {
                Ok(mut handle) => {
                    self.apply_pitch(&mut handle, pitch_shift);
                    self.wav_handles.insert(wav_id, handle);
                }
                Err(e) => {
                    log::warn!("Failed to play keysound wav {}: {}", wav_id, e);
                }
            }
        }
    }

    /// Stop a single note's keysound (without layered notes).
    /// Translated from AbstractAudioDriver.stop0()
    fn stop_note_internal(&mut self, n: &Note) {
        let wav_id = n.wav();
        if wav_id < 0 {
            return;
        }

        let starttime = n.micro_starttime();
        let duration = n.micro_duration();

        if starttime != 0 || duration != 0 {
            let key = (wav_id, starttime, duration);
            if let Some(mut handle) = self.slice_handles.remove(&key) {
                handle.stop(Tween::default());
                return;
            }
        }

        if let Some(mut handle) = self.wav_handles.remove(&wav_id) {
            handle.stop(Tween::default());
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
            if let Some(handle) = self.slice_handles.get_mut(&key) {
                handle.set_volume(linear_to_db(volume), Tween::default());
                return;
            }
        }

        if let Some(handle) = self.wav_handles.get_mut(&wav_id) {
            handle.set_volume(linear_to_db(volume), Tween::default());
        }
    }
}

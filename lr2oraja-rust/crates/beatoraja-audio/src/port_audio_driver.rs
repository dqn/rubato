//! PortAudio Driver - cpal output stream backed by Kira.
//!
//! Translated from: PortAudioDriver.java
//! In Rust, Kira (via cpal backend) replaces PortAudio for audio output.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rayon::prelude::*;

use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, PlaybackRate, Semitones, Tween};

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

use crate::abstract_audio_driver::SliceWav;
use crate::audio_driver::AudioDriver;
use crate::gdx_sound_driver::{FileCacheEntry, LoadTask, add_note_entry, linear_to_db};

pub struct PortAudioDriver {
    manager: AudioManager,
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
            sound_cache: HashMap::new(),
            file_cache: HashMap::new(),
            additional_key_sounds: Default::default(),
            additional_key_sound_handles: Default::default(),
        })
    }
}

impl AudioDriver for PortAudioDriver {
    fn play_path(&mut self, path: &str, volume: f32, _loop_play: bool) {
        if path.is_empty() {
            return;
        }

        // Stop any previously playing sound at this path
        if let Some(mut handle) = self.path_sounds.remove(path) {
            handle.stop(Tween::default());
        }

        // Try to load and play
        let candidates = crate::audio_driver::get_paths(path);
        for candidate in &candidates {
            match StaticSoundData::from_file(candidate) {
                Ok(sound_data) => match self.manager.play(sound_data) {
                    Ok(mut handle) => {
                        handle.set_volume(linear_to_db(volume), Tween::default());
                        self.path_sounds.insert(path.to_string(), handle);
                        return;
                    }
                    Err(e) => {
                        log::warn!("Failed to play sound {}: {}", path, e);
                    }
                },
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

        // Clear previous sounds
        self.wav_sounds.clear();
        self.wav_handles.clear();
        self.slicesound.clear();
        self.slice_handles.clear();

        // Set volume from model's volwav
        let volwav = model.get_volwav();
        if volwav > 0 && volwav < 100 {
            self.volume = volwav as f32 / 100.0;
        } else {
            self.volume = 1.0;
        }

        let wav_list = model.get_wav_list();
        if wav_list.is_empty() {
            return;
        }

        // Get BMS directory from model path
        let bms_dir = model
            .get_path()
            .and_then(|p| Path::new(&p).parent().map(|d| d.to_path_buf()));

        // Collect notes by wav ID, deduplicating by (starttime, duration)
        // Translated from AbstractAudioDriver.addNoteList()
        let mut notemap: HashMap<i32, Vec<(i64, i64)>> = HashMap::new();
        let lanes = model.get_mode().map(|m| m.key()).unwrap_or(0);
        for tl in model.get_all_time_lines() {
            for i in 0..lanes {
                if let Some(n) = tl.get_note(i) {
                    add_note_entry(&mut notemap, n);
                    for ln in n.get_layered_notes() {
                        add_note_entry(&mut notemap, ln);
                    }
                }
                if let Some(hn) = tl.get_hidden_note(i) {
                    add_note_entry(&mut notemap, hn);
                }
            }
            for n in tl.get_back_ground_notes() {
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
        // Translated from: AudioCache.get() — cache hit resets gen to 0
        let mut paths_to_load: HashSet<String> = HashSet::new();
        for (_, path, _) in &load_tasks {
            if let Some(entry) = self.file_cache.get_mut(path) {
                entry.generation = 0; // Cache hit: reset generation
            } else {
                paths_to_load.insert(path.clone());
            }
        }

        // Parallel load only uncached unique paths via rayon (matches Java parallelStream())
        if !paths_to_load.is_empty() {
            let paths_vec: Vec<String> = paths_to_load.into_iter().collect();
            let newly_loaded: Vec<(String, StaticSoundData)> = paths_vec
                .par_iter()
                .filter_map(|abs_path| {
                    let candidates = crate::audio_driver::get_paths(abs_path);
                    for candidate in &candidates {
                        if let Ok(data) = StaticSoundData::from_file(candidate) {
                            return Some((abs_path.clone(), data));
                        }
                    }
                    log::debug!("Failed to load keysound: {}", abs_path);
                    None
                })
                .collect();

            // Insert newly loaded into file_cache
            for (path, sound) in newly_loaded {
                self.file_cache.insert(
                    path,
                    FileCacheEntry {
                        sound,
                        generation: 0,
                    },
                );
            }
        }

        // Build wav_sounds/slicesound from file_cache
        // Translated from: AbstractAudioDriver.setModel() cache.get() → wavmap/slicesound
        for (wav_id, path, note_entries) in &load_tasks {
            if let Some(entry) = self.file_cache.get(path) {
                let base_sound = &entry.sound;
                for &(starttime, duration) in note_entries {
                    if starttime == 0 && duration == 0 {
                        self.wav_sounds.insert(*wav_id, base_sound.clone());
                    } else {
                        let sample_rate = base_sound.sample_rate as i64;
                        let start_frame = (starttime * sample_rate / 1_000_000) as usize;
                        let duration_frames = (duration * sample_rate / 1_000_000) as usize;
                        let total_frames = base_sound.frames.len();
                        let end_frame = (start_frame + duration_frames).min(total_frames);

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

    fn set_additional_key_sound(&mut self, judge: i32, fast: bool, path: Option<&str>) {
        if !(0..6).contains(&judge) {
            return;
        }
        let j = judge as usize;
        let idx = if fast { 0 } else { 1 };
        match path {
            Some(p) if !p.is_empty() => {
                self.additional_key_sounds[j][idx] = self.get_sound(p);
            }
            _ => {
                self.additional_key_sounds[j][idx] = None;
            }
        }
    }
    fn abort(&mut self) {}
    fn get_progress(&self) -> f32 {
        1.0
    }

    fn play_note(&mut self, n: &Note, volume: f32, pitch: i32) {
        self.play_note_internal(n, self.volume * volume, pitch);
        for ln in n.get_layered_notes() {
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
            let sound = sound_data.clone();
            match self.manager.play(sound) {
                Ok(mut handle) => {
                    handle.set_volume(linear_to_db(self.volume), Tween::default());
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
                for ln in note.get_layered_notes() {
                    self.stop_note_internal(ln);
                }
            }
        }
    }

    fn set_volume_note(&mut self, n: &Note, volume: f32) {
        self.set_volume_note_internal(n, volume);
        for ln in n.get_layered_notes() {
            self.set_volume_note_internal(ln, volume);
        }
    }

    fn set_global_pitch(&mut self, pitch: f32) {
        self.global_pitch = pitch;
    }

    fn get_global_pitch(&self) -> f32 {
        self.global_pitch
    }

    fn dispose_old(&mut self) {
        self.evict_old_cache();
    }

    fn dispose(&mut self) {
        self.path_sounds.clear();
        self.wav_sounds.clear();
        self.wav_handles.clear();
        self.slicesound.clear();
        self.slice_handles.clear();
        self.sound_cache.clear();
        self.file_cache.clear();
        self.additional_key_sounds = Default::default();
        self.additional_key_sound_handles = Default::default();
    }
}

impl PortAudioDriver {
    /// Load and cache a sound from path.
    /// Translated from AbstractAudioDriver.getSound()
    fn get_sound(&mut self, path: &str) -> Option<StaticSoundData> {
        if path.is_empty() {
            return None;
        }
        if let Some(data) = self.sound_cache.get(path) {
            return Some(data.clone());
        }
        let candidates = crate::audio_driver::get_paths(path);
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
                "AudioCache capacity: {} released: {}",
                self.file_cache.len(),
                released
            );
        }
    }

    /// Play a single note's keysound (without layered notes).
    /// Translated from AbstractAudioDriver.play0()
    fn play_note_internal(&mut self, n: &Note, volume: f32, pitch_shift: i32) {
        let wav_id = n.get_wav();
        if wav_id < 0 {
            return;
        }

        let starttime = n.get_micro_starttime();
        let duration = n.get_micro_duration();

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
                    let sound = slice.wav.clone();
                    match self.manager.play(sound) {
                        Ok(mut handle) => {
                            handle.set_volume(linear_to_db(volume), Tween::default());
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
            let sound = sound_data.clone();
            match self.manager.play(sound) {
                Ok(mut handle) => {
                    handle.set_volume(linear_to_db(volume), Tween::default());
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
        let wav_id = n.get_wav();
        if wav_id < 0 {
            return;
        }

        let starttime = n.get_micro_starttime();
        let duration = n.get_micro_duration();

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
        let wav_id = n.get_wav();
        if wav_id < 0 {
            return;
        }

        let starttime = n.get_micro_starttime();
        let duration = n.get_micro_duration();

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

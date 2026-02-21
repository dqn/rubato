//! GDX Sound Driver - Primary audio driver using Kira.
//!
//! Translated from: GdxSoundDriver.java
//! In Rust, Kira replaces LibGDX for audio.

use std::collections::HashMap;
use std::path::Path;

use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, PlaybackRate, Semitones, Tween};

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

use crate::audio_driver::AudioDriver;

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

pub struct GdxSoundDriver {
    manager: AudioManager,
    // Map from path to sound handle
    path_sounds: HashMap<String, StaticSoundHandle>,
    // Map from wav ID to sound data for BMS keysounds
    wav_sounds: HashMap<i32, StaticSoundData>,
    wav_handles: HashMap<i32, StaticSoundHandle>,
    global_pitch: f32,
    // Model volume from volwav (0.0-1.0)
    volume: f32,
    #[allow(dead_code)]
    song_resource_gen: i32,
    // Cache for loaded sounds by path (matches Java soundmap)
    sound_cache: HashMap<String, StaticSoundData>,
    // Additional key sounds for judge playback: [6 judges][2: fast=0, late=1]
    additional_key_sounds: [[Option<StaticSoundData>; 2]; 6],
    additional_key_sound_handles: [[Option<StaticSoundHandle>; 2]; 6],
}

impl GdxSoundDriver {
    pub fn new(song_resource_gen: i32) -> Self {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .expect("Failed to create audio manager");
        GdxSoundDriver {
            manager,
            path_sounds: HashMap::new(),
            wav_sounds: HashMap::new(),
            wav_handles: HashMap::new(),
            global_pitch: 1.0,
            volume: 1.0,
            song_resource_gen,
            sound_cache: HashMap::new(),
            additional_key_sounds: Default::default(),
            additional_key_sound_handles: Default::default(),
        }
    }
}

impl AudioDriver for GdxSoundDriver {
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

        // Collect wav IDs referenced by notes
        let mut referenced_wavs = std::collections::HashSet::new();
        let lanes = model.get_mode().map(|m| m.key()).unwrap_or(0);
        for tl in model.get_all_time_lines() {
            for i in 0..lanes {
                if let Some(n) = tl.get_note(i) {
                    if n.get_wav() >= 0 {
                        referenced_wavs.insert(n.get_wav());
                    }
                    for ln in n.get_layered_notes() {
                        if ln.get_wav() >= 0 {
                            referenced_wavs.insert(ln.get_wav());
                        }
                    }
                }
                if let Some(hn) = tl.get_hidden_note(i)
                    && hn.get_wav() >= 0
                {
                    referenced_wavs.insert(hn.get_wav());
                }
            }
            for n in tl.get_back_ground_notes() {
                if n.get_wav() >= 0 {
                    referenced_wavs.insert(n.get_wav());
                }
            }
        }

        // Load audio files for referenced wav IDs
        for wav_id in &referenced_wavs {
            let wav_id_usize = *wav_id as usize;
            if wav_id_usize >= wav_list.len() {
                continue;
            }
            let wav_path = &wav_list[wav_id_usize];
            if wav_path.is_empty() {
                continue;
            }

            // Resolve path relative to BMS directory
            let resolved = if let Some(ref dir) = bms_dir {
                dir.join(wav_path)
            } else {
                std::path::PathBuf::from(wav_path)
            };

            // Try the resolved path and alternate extensions
            let abs_path = resolved.to_string_lossy().to_string();
            let candidates = crate::audio_driver::get_paths(&abs_path);

            let mut loaded = false;
            for candidate in &candidates {
                match StaticSoundData::from_file(candidate) {
                    Ok(sound_data) => {
                        self.wav_sounds.insert(*wav_id, sound_data);
                        loaded = true;
                        break;
                    }
                    Err(_) => continue,
                }
            }

            if !loaded {
                log::debug!("Failed to load keysound for wav {}: {}", wav_id, abs_path);
            }
        }

        log::info!(
            "Keysound loading complete. Loaded: {}/{}",
            self.wav_sounds.len(),
            referenced_wavs.len()
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

    fn dispose_old(&mut self) {}

    fn dispose(&mut self) {
        self.path_sounds.clear();
        self.wav_sounds.clear();
        self.wav_handles.clear();
        self.sound_cache.clear();
        self.additional_key_sounds = Default::default();
        self.additional_key_sound_handles = Default::default();
    }
}

impl GdxSoundDriver {
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

    /// Play a single note's keysound (without layered notes).
    /// Translated from AbstractAudioDriver.play0()
    fn play_note_internal(&mut self, n: &Note, volume: f32, pitch_shift: i32) {
        let wav_id = n.get_wav();
        if wav_id < 0 {
            return;
        }

        if let Some(sound_data) = self.wav_sounds.get(&wav_id) {
            // Stop any currently playing instance of this keysound
            if let Some(mut old_handle) = self.wav_handles.remove(&wav_id) {
                old_handle.stop(Tween::default());
            }

            // Clone sound data and play
            let sound = sound_data.clone();
            match self.manager.play(sound) {
                Ok(mut handle) => {
                    handle.set_volume(linear_to_db(volume), Tween::default());
                    // Apply pitch: semitone shift if specified, otherwise global pitch
                    if pitch_shift != 0 {
                        handle.set_playback_rate(Semitones(pitch_shift as f64), Tween::default());
                    } else if (self.global_pitch - 1.0).abs() > f32::EPSILON {
                        handle.set_playback_rate(
                            PlaybackRate(self.global_pitch as f64),
                            Tween::default(),
                        );
                    }
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
        if let Some(handle) = self.wav_handles.get_mut(&wav_id) {
            handle.set_volume(linear_to_db(volume), Tween::default());
        }
    }
}

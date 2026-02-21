//! GDX Sound Driver - Primary audio driver using Kira.
//!
//! Translated from: GdxSoundDriver.java
//! In Rust, Kira replaces LibGDX for audio.

use std::collections::HashMap;

use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend};

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

use crate::audio_driver::AudioDriver;

pub struct GdxSoundDriver {
    manager: AudioManager,
    // Map from path to sound handle
    path_sounds: HashMap<String, StaticSoundHandle>,
    // Map from wav ID to sound data for BMS keysounds
    wav_sounds: HashMap<i32, StaticSoundData>,
    wav_handles: HashMap<i32, StaticSoundHandle>,
    global_pitch: f32,
    #[allow(dead_code)]
    song_resource_gen: i32,
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
            song_resource_gen,
        }
    }
}

impl AudioDriver for GdxSoundDriver {
    fn play_path(&mut self, path: &str, _volume: f32, _loop_play: bool) {
        // Full implementation requires loading PCM data and constructing StaticSoundData.
        // Deferred until keysound loading pipeline is complete.
        log::warn!(
            "GdxSoundDriver: play_path not yet fully implemented for {}",
            path
        );
    }

    fn set_volume_path(&mut self, _path: &str, _volume: f32) {
        // Kira handles volume per-sound
    }

    fn is_playing_path(&self, _path: &str) -> bool {
        false // Simplified
    }

    fn stop_path(&mut self, path: &str) {
        if let Some(mut handle) = self.path_sounds.remove(path) {
            handle.stop(Default::default());
        }
    }

    fn dispose_path(&mut self, path: &str) {
        self.stop_path(path);
    }

    fn set_model(&mut self, _model: &BMSModel) {
        // Load all WAV resources from the model
        // Simplified: full implementation requires loading all keysounds
        log::info!("GdxSoundDriver: set_model called (simplified)");
    }

    fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}
    fn abort(&mut self) {}
    fn get_progress(&self) -> f32 {
        1.0
    }

    fn play_note(&mut self, _n: &Note, _volume: f32, _pitch: i32) {
        // Play keysound for the given note
    }

    fn play_judge(&mut self, _judge: i32, _fast: bool) {}

    fn stop_note(&mut self, n: Option<&Note>) {
        if n.is_none() {
            // Stop all
            for (_, mut handle) in self.wav_handles.drain() {
                handle.stop(Default::default());
            }
        }
    }

    fn set_volume_note(&mut self, _n: &Note, _volume: f32) {}

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
    }
}

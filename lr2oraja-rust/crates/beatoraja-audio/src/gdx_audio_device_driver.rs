//! GDX Audio Device Driver - Device-level audio using Kira.
//!
//! Translated from: GdxAudioDeviceDriver.java
//! This is a thin wrapper, similar to GdxSoundDriver but uses AudioDevice API.

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

use crate::audio_driver::AudioDriver;

pub struct GdxAudioDeviceDriver {
    global_pitch: f32,
    song_resource_gen: i32,
}

impl GdxAudioDeviceDriver {
    pub fn new(song_resource_gen: i32) -> Self {
        GdxAudioDeviceDriver {
            global_pitch: 1.0,
            song_resource_gen,
        }
    }
}

impl AudioDriver for GdxAudioDeviceDriver {
    fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {
        log::warn!("GdxAudioDeviceDriver: play_path not yet fully implemented");
    }

    fn set_volume_path(&mut self, _path: &str, _volume: f32) {}

    fn is_playing_path(&self, _path: &str) -> bool {
        false
    }

    fn stop_path(&mut self, _path: &str) {}

    fn dispose_path(&mut self, _path: &str) {}

    fn set_model(&mut self, _model: &BMSModel) {}

    fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}

    fn abort(&mut self) {}

    fn get_progress(&self) -> f32 {
        1.0
    }

    fn play_note(&mut self, _n: &Note, _volume: f32, _pitch: i32) {}

    fn play_judge(&mut self, _judge: i32, _fast: bool) {}

    fn stop_note(&mut self, _n: Option<&Note>) {}

    fn set_volume_note(&mut self, _n: &Note, _volume: f32) {}

    fn set_global_pitch(&mut self, pitch: f32) {
        self.global_pitch = pitch;
    }

    fn get_global_pitch(&self) -> f32 {
        self.global_pitch
    }

    fn dispose_old(&mut self) {}

    fn dispose(&mut self) {}
}

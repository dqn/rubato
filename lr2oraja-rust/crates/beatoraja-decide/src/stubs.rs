// Stubs for external dependencies not yet available as proper imports.

use beatoraja_audio::audio_driver::AudioDriver;
use beatoraja_core::system_sound_manager::SoundType;

// InputProcessorStub: replaced by pub use from beatoraja-input (Phase 18e-11)
pub use beatoraja_input::bms_player_input_processor::BMSPlayerInputProcessor;

// ControlKeysStub: replaced by pub use from beatoraja-input (Phase 18e-11)
pub use beatoraja_input::keyboard_input_processor::ControlKeys;

// MainControllerAccess: real trait from beatoraja-types (Phase 41b)
pub use beatoraja_types::main_controller_access::{MainControllerAccess, NullMainController};

/// Wrapper for MainController reference.
/// Delegates trait methods (change_state) to `Box<dyn MainControllerAccess>`.
/// Stores BMSPlayerInputProcessor locally (type not available on MainControllerAccess trait).
/// AudioDriver is stored directly (Phase 41c) — not on MainControllerAccess trait.
pub struct MainControllerRef {
    inner: Box<dyn MainControllerAccess>,
    audio: Option<Box<dyn AudioDriver>>,
    input_processor: BMSPlayerInputProcessor,
}

impl MainControllerRef {
    pub fn new(inner: Box<dyn MainControllerAccess>) -> Self {
        let config = inner.get_config();
        let player_config = inner.get_player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        Self {
            inner,
            audio: None,
            input_processor,
        }
    }

    pub fn with_audio(inner: Box<dyn MainControllerAccess>, audio: Box<dyn AudioDriver>) -> Self {
        let config = inner.get_config();
        let player_config = inner.get_player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        Self {
            inner,
            audio: Some(audio),
            input_processor,
        }
    }

    pub fn change_state(&mut self, state: beatoraja_types::main_state_type::MainStateType) {
        self.inner.change_state(state);
    }

    pub fn get_input_processor(&mut self) -> &mut BMSPlayerInputProcessor {
        &mut self.input_processor
    }

    pub fn get_audio_processor_mut(&mut self) -> Option<&mut dyn AudioDriver> {
        self.audio
            .as_mut()
            .map(|b| &mut **b as &mut dyn AudioDriver)
    }

    pub fn play_sound(&mut self, sound: &SoundType, loop_sound: bool) {
        self.inner.play_sound(sound, loop_sound);
    }
}

/// PlayerResourceAccess — re-exported from beatoraja-types (Phase 18e-2)
pub use beatoraja_types::player_resource_access::PlayerResourceAccess;

/// NullPlayerResource — re-exported from beatoraja-types for default construction
pub use beatoraja_types::player_resource_access::NullPlayerResource;

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::bms_model::BMSModel;
    use bms_model::note::Note;

    /// Mock AudioDriver for testing.
    struct MockAudioDriver {
        global_pitch: f32,
    }

    impl MockAudioDriver {
        fn new() -> Self {
            Self { global_pitch: 1.0 }
        }
    }

    impl AudioDriver for MockAudioDriver {
        fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {}
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

    #[test]
    fn test_main_controller_ref_new_has_no_audio() {
        let mut mc = MainControllerRef::new(Box::new(NullMainController));
        assert!(mc.get_audio_processor_mut().is_none());
    }

    #[test]
    fn test_main_controller_ref_with_audio_has_audio() {
        let mut mc = MainControllerRef::with_audio(
            Box::new(NullMainController),
            Box::new(MockAudioDriver::new()),
        );
        assert!(mc.get_audio_processor_mut().is_some());
    }

    #[test]
    fn test_main_controller_ref_audio_set_global_pitch() {
        let mut mc = MainControllerRef::with_audio(
            Box::new(NullMainController),
            Box::new(MockAudioDriver::new()),
        );
        if let Some(audio) = mc.get_audio_processor_mut() {
            audio.set_global_pitch(1.0);
            assert_eq!(audio.get_global_pitch(), 1.0);
        } else {
            panic!("expected audio processor to be present");
        }
    }

    #[test]
    fn test_main_controller_ref_audio_stop_note() {
        let mut mc = MainControllerRef::with_audio(
            Box::new(NullMainController),
            Box::new(MockAudioDriver::new()),
        );
        if let Some(audio) = mc.get_audio_processor_mut() {
            audio.stop_note(None);
        }
    }
}

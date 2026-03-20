use rubato_audio::audio_driver::AudioDriver;
use rubato_core::system_sound_manager::SoundType;
use rubato_input::bms_player_input_processor::BMSPlayerInputProcessor;

/// Wrapper for MainController reference.
/// Delegates trait methods (change_state) to `Box<dyn MainControllerAccess>`.
/// Stores BMSPlayerInputProcessor locally (type not available on MainControllerAccess trait).
/// AudioDriver is stored directly (Phase 41c) -- not on MainControllerAccess trait.
pub struct MainControllerRef {
    inner: Box<dyn rubato_types::main_controller_access::MainControllerAccess>,
    audio: Option<Box<dyn AudioDriver>>,
    input_processor: BMSPlayerInputProcessor,
}

impl MainControllerRef {
    pub fn new(inner: Box<dyn rubato_types::main_controller_access::MainControllerAccess>) -> Self {
        let config = inner.config();
        let player_config = inner.player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        Self {
            inner,
            audio: None,
            input_processor,
        }
    }

    pub fn with_audio(
        inner: Box<dyn rubato_types::main_controller_access::MainControllerAccess>,
        audio: Box<dyn AudioDriver>,
    ) -> Self {
        let config = inner.config();
        let player_config = inner.player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        Self {
            inner,
            audio: Some(audio),
            input_processor,
        }
    }

    pub fn config(&self) -> &rubato_types::config::Config {
        self.inner.config()
    }

    pub fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
        self.inner.player_config()
    }

    pub fn change_state(&mut self, state: rubato_types::main_state_type::MainStateType) {
        self.inner.change_state(state);
    }

    pub fn input_processor(&mut self) -> &mut BMSPlayerInputProcessor {
        &mut self.input_processor
    }

    pub fn sync_input_from(&mut self, input: &BMSPlayerInputProcessor) {
        self.input_processor.sync_runtime_state_from(input);
    }

    pub fn sync_input_back_to(&mut self, input: &mut BMSPlayerInputProcessor) {
        input.sync_runtime_state_from(&self.input_processor);
    }

    pub fn audio_processor_mut(&mut self) -> Option<&mut dyn AudioDriver> {
        self.audio
            .as_mut()
            .map(|b| &mut **b as &mut dyn AudioDriver)
    }

    pub fn play_sound(&mut self, sound: &SoundType, loop_sound: bool) {
        self.inner.play_sound(sound, loop_sound);
    }

    pub fn update_audio_config(&self, audio: rubato_types::audio_config::AudioConfig) {
        self.inner.update_audio_config(audio);
    }

    pub fn offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.inner.offset_value(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_audio::recording_audio_driver::RecordingAudioDriver;
    use rubato_types::main_controller_access::NullMainController;

    #[test]
    fn test_main_controller_ref_new_has_no_audio() {
        let mut mc = MainControllerRef::new(Box::new(NullMainController));
        assert!(mc.audio_processor_mut().is_none());
    }

    #[test]
    fn test_main_controller_ref_with_audio_has_audio() {
        let mut mc = MainControllerRef::with_audio(
            Box::new(NullMainController),
            Box::new(RecordingAudioDriver::new()),
        );
        assert!(mc.audio_processor_mut().is_some());
    }

    #[test]
    fn test_main_controller_ref_audio_set_global_pitch() {
        let mut mc = MainControllerRef::with_audio(
            Box::new(NullMainController),
            Box::new(RecordingAudioDriver::new()),
        );
        if let Some(audio) = mc.audio_processor_mut() {
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
            Box::new(RecordingAudioDriver::new()),
        );
        if let Some(audio) = mc.audio_processor_mut() {
            audio.stop_note(None);
        }
    }
}

use crate::core::system_sound_manager::SoundType;

/// Wrapper for MainController reference.
/// Delegates trait methods (change_state) to `Box<dyn MainControllerAccess>`.
pub struct MainControllerRef {
    inner: Box<dyn rubato_types::main_controller_access::MainControllerAccess>,
}

impl MainControllerRef {
    pub fn new(inner: Box<dyn rubato_types::main_controller_access::MainControllerAccess>) -> Self {
        Self { inner }
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

    pub fn play_sound(&mut self, sound: &SoundType, loop_sound: bool) {
        self.inner.play_sound(sound, loop_sound);
    }

    pub fn update_audio_config(&self, audio: rubato_types::audio_config::AudioConfig) {
        self.inner.update_audio_config(audio);
    }

    pub fn offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.inner.offset_value(id)
    }

    pub fn play_audio_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        self.inner.play_audio_path(path, volume, loop_play);
    }

    pub fn stop_audio_path(&mut self, path: &str) {
        self.inner.stop_audio_path(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::main_controller_access::NullMainController;

    #[test]
    fn test_main_controller_ref_new() {
        let mc = MainControllerRef::new(Box::new(NullMainController));
        assert_eq!(
            mc.config().display.window_width,
            rubato_types::config::Config::default().display.window_width,
        );
    }
}

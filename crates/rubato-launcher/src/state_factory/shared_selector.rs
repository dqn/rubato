// SharedMusicSelectorState -- wrapper that delegates MainState methods
// to a shared Arc<Mutex<MusicSelector>> for stream/select screen sharing.

use std::sync::{Arc, Mutex};

use rubato_core::main_state::{MainState, MainStateData, MainStateType};
use rubato_core::timer_manager::TimerManager;
use rubato_state::select::music_selector::MusicSelector;
use rubato_types::sound_type::SoundType;

/// Wrapper that delegates MainState methods to a shared `Arc<Mutex<MusicSelector>>`.
///
/// Java: StreamController and MusicSelect screen share the same MusicSelector instance.
/// In Rust, both hold an `Arc<Mutex<MusicSelector>>` so stream request bars appear in the
/// select screen's bar list.
///
/// The wrapper owns a local `MainStateData` for the `main_state_data()` / `main_state_data_mut()`
/// trait methods (which return references and cannot go through a Mutex). Lifecycle methods
/// (create, render, etc.) delegate through the Arc<Mutex<>> to the shared selector.
pub(super) struct SharedMusicSelectorState {
    selector: Arc<Mutex<MusicSelector>>,
    /// Local state data for skin/score property access.
    /// Synced from the shared selector on create() and after render().
    state_data: MainStateData,
}

impl SharedMusicSelectorState {
    pub(super) fn new(selector: Arc<Mutex<MusicSelector>>) -> Self {
        let state_data = {
            let mut selector_guard = selector.lock().expect("selector lock poisoned");
            std::mem::replace(
                &mut selector_guard.main_state_data,
                MainStateData::new(TimerManager::new()),
            )
        };
        Self {
            selector,
            state_data,
        }
    }

    fn with_selector<R>(&mut self, f: impl FnOnce(&mut MusicSelector) -> R) -> R {
        let mut selector = self.selector.lock().expect("selector lock poisoned");
        std::mem::swap(&mut self.state_data, &mut selector.main_state_data);
        let result = f(&mut selector);
        std::mem::swap(&mut self.state_data, &mut selector.main_state_data);
        result
    }
}

impl MainState for SharedMusicSelectorState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::MusicSelect)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {
        self.with_selector(|selector| selector.create());
    }

    fn prepare(&mut self) {
        self.with_selector(|selector| selector.prepare());
    }

    fn shutdown(&mut self) {
        self.with_selector(|selector| selector.shutdown());
    }

    fn render(&mut self) {
        self.with_selector(|selector| selector.render());
    }

    fn input(&mut self) {
        self.with_selector(|selector| selector.input());
    }

    fn sync_audio(&mut self, audio: &mut dyn rubato_audio::audio_driver::AudioDriver) {
        self.with_selector(|selector| selector.sync_audio(audio));
    }

    fn pause(&mut self) {
        self.with_selector(|selector| selector.pause());
    }

    fn resume(&mut self) {
        self.with_selector(|selector| selector.resume());
    }

    fn resize(&mut self, width: i32, height: i32) {
        self.with_selector(|selector| selector.resize(width, height));
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        self.with_selector(|selector| selector.handle_skin_mouse_pressed(button, x, y));
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        self.with_selector(|selector| selector.handle_skin_mouse_dragged(button, x, y));
    }

    fn dispose(&mut self) {
        self.with_selector(|selector| selector.dispose());
    }

    fn sound(&self, sound: SoundType) -> Option<String> {
        self.selector
            .lock()
            .expect("selector lock poisoned")
            .sound(sound)
    }

    fn play_sound_loop(&mut self, sound: SoundType, loop_sound: bool) {
        self.with_selector(|selector| selector.play_sound_loop(sound, loop_sound));
    }

    fn stop_sound(&mut self, sound: SoundType) {
        self.with_selector(|selector| selector.stop_sound(sound));
    }

    fn load_skin(&mut self, skin_type: i32) {
        self.with_selector(|selector| selector.load_skin(skin_type));
    }

    fn render_skin(&mut self, sprite: &mut rubato_core::sprite_batch_helper::SpriteBatch) {
        self.with_selector(|selector| selector.render_skin(sprite));
    }

    fn take_pending_state_change(&mut self) -> Option<MainStateType> {
        self.with_selector(|selector| selector.take_pending_state_change())
    }
}

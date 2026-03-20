use super::*;
use crate::config::SelectConfig;
use crate::config_pkg::key_configuration::KeyConfiguration;
use crate::config_pkg::skin_configuration::SkinConfiguration;
use crate::main_state::MainStateData;
use rubato_types::test_support::CurrentDirGuard;

/// A minimal test state that implements MainState for testing state dispatch.
struct TestState {
    state_data: MainStateData,
    state_type: MainStateType,
    created: bool,
    prepared: bool,
    shut_down: bool,
    rendered: bool,
    disposed: bool,
}

impl TestState {
    fn new(state_type: MainStateType) -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            state_type,
            created: false,
            prepared: false,
            shut_down: false,
            rendered: false,
            disposed: false,
        }
    }
}

impl MainState for TestState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(self.state_type)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {
        self.created = true;
    }

    fn prepare(&mut self) {
        self.prepared = true;
    }

    fn shutdown(&mut self) {
        self.shut_down = true;
    }

    fn render(&mut self) {
        self.rendered = true;
    }

    fn dispose(&mut self) {
        self.disposed = true;
        self.state_data.skin = None;
    }
}

/// A test factory that creates TestState instances.
struct TestStateFactory;

impl StateFactory for TestStateFactory {
    fn create_state(
        &self,
        state_type: MainStateType,
        _controller: &mut MainController,
    ) -> Option<StateCreateResult> {
        Some(StateCreateResult {
            state: Box::new(TestState::new(state_type)),
            target_score: None,
        })
    }
}

fn make_test_controller() -> MainController {
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(TestStateFactory));
    mc
}

struct AudioSyncTestState {
    state_data: MainStateData,
    state_type: MainStateType,
    render_sync_calls: Arc<Mutex<usize>>,
    shutdown_sync_calls: Arc<Mutex<usize>>,
    was_shutdown: bool,
}

impl AudioSyncTestState {
    fn new(
        state_type: MainStateType,
        render_sync_calls: Arc<Mutex<usize>>,
        shutdown_sync_calls: Arc<Mutex<usize>>,
    ) -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            state_type,
            render_sync_calls,
            shutdown_sync_calls,
            was_shutdown: false,
        }
    }
}

impl MainState for AudioSyncTestState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(self.state_type)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {}

    fn shutdown(&mut self) {
        self.was_shutdown = true;
    }

    fn render(&mut self) {}

    fn sync_audio(&mut self, _audio: &mut dyn AudioDriver) {
        let counter = if self.was_shutdown {
            &self.shutdown_sync_calls
        } else {
            &self.render_sync_calls
        };
        *counter.lock().expect("mutex poisoned") += 1;
    }
}

struct AudioSyncStateFactory {
    render_sync_calls: Arc<Mutex<usize>>,
    shutdown_sync_calls: Arc<Mutex<usize>>,
}

impl AudioSyncStateFactory {
    fn new(render_sync_calls: Arc<Mutex<usize>>, shutdown_sync_calls: Arc<Mutex<usize>>) -> Self {
        Self {
            render_sync_calls,
            shutdown_sync_calls,
        }
    }
}

impl StateFactory for AudioSyncStateFactory {
    fn create_state(
        &self,
        state_type: MainStateType,
        _controller: &mut MainController,
    ) -> Option<StateCreateResult> {
        Some(StateCreateResult {
            state: Box::new(AudioSyncTestState::new(
                state_type,
                Arc::clone(&self.render_sync_calls),
                Arc::clone(&self.shutdown_sync_calls),
            )),
            target_score: None,
        })
    }
}

#[test]
fn test_initial_state_is_none() {
    let mc = make_test_controller();
    assert!(mc.current_state().is_none());
    assert!(mc.current_state_type().is_none());
}

#[test]
fn test_change_state_to_music_select() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::MusicSelect);

    assert!(mc.current_state().is_some());
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn test_change_state_to_play() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::Play);

    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));
}

#[test]
fn test_change_state_to_result() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::Result);

    assert_eq!(mc.current_state_type(), Some(MainStateType::Result));
}

#[test]
fn test_change_state_to_config() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::Config);

    assert_eq!(mc.current_state_type(), Some(MainStateType::Config));
}

#[test]
fn test_process_queued_change_state_command_transitions_current_state() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::MusicSelect);

    let queue = mc.controller_command_queue();
    queue.push(
        rubato_types::main_controller_access::MainControllerCommand::ChangeState(
            MainStateType::Config,
        ),
    );

    mc.process_queued_controller_commands();

    assert_eq!(mc.current_state_type(), Some(MainStateType::Config));
}

#[test]
fn test_change_state_to_skin_config() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::SkinConfig);

    assert_eq!(mc.current_state_type(), Some(MainStateType::SkinConfig));
}

#[test]
fn test_change_state_calls_create_and_prepare() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::MusicSelect);

    // The state should have been created and prepared
    let state = mc.current_state().unwrap();
    assert_eq!(state.state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn test_change_state_shuts_down_previous() {
    let mut mc = make_test_controller();

    // Enter first state
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    // Transition to a different state
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));
}

#[test]
fn test_change_state_same_type_is_noop() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::MusicSelect);

    // Changing to the same state type should be a no-op
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn test_decide_skip_creates_play_state() {
    let config = Config {
        select: SelectConfig {
            skip_decide_screen: true,
            ..SelectConfig::default()
        },
        ..Config::default()
    };
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(TestStateFactory));

    mc.change_state(MainStateType::Decide);

    // With skip_decide_screen, Decide should create Play instead
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));
}

#[test]
fn test_decide_no_skip_creates_decide_state() {
    let config = Config {
        select: SelectConfig {
            skip_decide_screen: false,
            ..SelectConfig::default()
        },
        ..Config::default()
    };
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(TestStateFactory));

    mc.change_state(MainStateType::Decide);

    assert_eq!(mc.current_state_type(), Some(MainStateType::Decide));
}

#[test]
fn test_music_select_with_bmsfile_calls_exit() {
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(
        Some(std::path::PathBuf::from("/test/file.bms")),
        config,
        player,
        None,
        false,
    );
    mc.set_state_factory(Box::new(TestStateFactory));

    // When bmsfile is set and we try to go to MusicSelect, it should call exit()
    // (which just logs a warning) and not create a state
    mc.change_state(MainStateType::MusicSelect);

    // No state should be set since exit() was called
    assert!(mc.current_state().is_none());
}

#[test]
fn test_get_state_type_static() {
    let state = TestState::new(MainStateType::Play);
    assert_eq!(
        MainController::state_type(Some(&state as &dyn MainState)),
        Some(MainStateType::Play)
    );
}

#[test]
fn test_get_state_type_none() {
    assert_eq!(MainController::state_type(None), None);
}

#[test]
fn test_lifecycle_dispatch_render() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::MusicSelect);

    // Render should dispatch to current state
    mc.render();

    // State should still be MusicSelect
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn test_lifecycle_dispatch_pause_resume() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::MusicSelect);

    mc.pause();
    mc.resume();

    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn test_lifecycle_dispatch_resize() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::MusicSelect);

    mc.resize(1920, 1080);

    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn test_dispose_clears_current_state() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::MusicSelect);
    assert!(mc.current_state().is_some());

    mc.dispose();
    assert!(mc.current_state().is_none());
}

#[test]
#[should_panic(expected = "No state factory set; cannot create state MusicSelect")]
fn test_no_factory_panics() {
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    // No factory set — must panic to make wiring bugs immediately visible
    mc.change_state(MainStateType::MusicSelect);
}

#[test]
fn test_multiple_state_transitions() {
    let mut mc = make_test_controller();

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    mc.change_state(MainStateType::Decide);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Decide));

    mc.change_state(MainStateType::Play);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

    mc.change_state(MainStateType::Result);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Result));

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn test_key_configuration_main_state_trait() {
    let config = Config::default();
    let player = PlayerConfig::default();
    let mc = MainController::new(None, config, player, None, false);

    let mut kc = KeyConfiguration::new(&mc);
    let state: &mut dyn MainState = &mut kc;

    assert_eq!(state.state_type(), Some(MainStateType::Config));
    state.create();
    state.render();
    state.input();
    state.dispose();
}

#[test]
fn test_skin_configuration_main_state_trait() {
    let config = Config::default();
    let player = PlayerConfig::default();
    let mc = MainController::new(None, config, player.clone(), None, false);

    let mut sc = SkinConfiguration::new(&mc, &player);
    let state: &mut dyn MainState = &mut sc;

    assert_eq!(state.state_type(), Some(MainStateType::SkinConfig));
    state.create();
    state.render();
    state.input();
    state.dispose();
}

#[test]
fn test_course_result_state_transition() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::CourseResult);
    assert_eq!(mc.current_state_type(), Some(MainStateType::CourseResult));
}

// --- Phase 22c: Render pipeline tests ---

#[test]
fn test_render_creates_sprite_batch_on_create() {
    let mut mc = make_test_controller();
    mc.create();
    // After create(), sprite batch should be initialized
    assert!(mc.sprite_batch().is_some());
}

#[test]
fn test_render_sprite_batch_begin_end_lifecycle() {
    let mut mc = make_test_controller();
    mc.create();

    // Before render, sprite batch should not be drawing
    assert!(mc.sprite_batch().is_some());
    assert!(!mc.sprite_batch().unwrap().drawing);

    // After render, sprite batch should have gone through begin()/end() cycle
    // and should not be drawing anymore
    mc.render();
    assert!(!mc.sprite_batch().unwrap().drawing);
}

#[test]
fn test_render_input_gating_by_time() {
    let mut mc = make_test_controller();
    mc.create();

    // prevtime starts at 0; first render should update it
    assert_eq!(mc.lifecycle.prevtime, 0);

    mc.render();

    // After render, prevtime should be updated to current time
    assert!(mc.lifecycle.prevtime > 0);
}

#[test]
fn test_render_dispatches_to_current_state() {
    let mut mc = make_test_controller();
    mc.set_state_factory(Box::new(TestStateFactory));
    mc.change_state(MainStateType::MusicSelect);

    // render() should dispatch to current state's render()
    mc.render();

    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn test_render_no_state_does_not_panic() {
    let mut mc = make_test_controller();
    // No state set, render should not panic
    mc.render();
    assert!(mc.current_state().is_none());
}

#[test]
fn test_sprite_batch_mut_accessor() {
    let mut mc = make_test_controller();
    mc.create();

    // Should be able to get mutable reference to sprite batch
    let batch = mc.sprite_batch_mut().unwrap();
    batch.begin();
    assert!(batch.drawing);
    batch.end();
    assert!(!batch.drawing);
}

#[test]
fn test_render_timer_updated_each_frame() {
    let mut mc = make_test_controller();
    mc.create();

    let time_before = mc.now_time();
    // Small sleep to ensure timer advances
    std::thread::sleep(std::time::Duration::from_millis(5));
    mc.render();
    let time_after = mc.now_time();

    // Timer should advance (or at least not go backwards)
    assert!(time_after >= time_before);
}

// --- Phase 22d: Skin draw wiring tests ---

use crate::main_state::SkinDrawable;

/// Mock SkinDrawable that tracks method call counts.
struct MockSkinDrawable {
    draw_count: i32,
    update_count: i32,
}

impl MockSkinDrawable {
    fn new() -> Self {
        Self {
            draw_count: 0,
            update_count: 0,
        }
    }
}

impl SkinDrawable for MockSkinDrawable {
    fn draw_all_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
        self.draw_count += 1;
    }

    fn update_custom_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
        self.update_count += 1;
    }

    fn mouse_pressed_at(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
    }
    fn mouse_dragged_at(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
    }
    fn prepare_skin(&mut self) {}
    fn dispose_skin(&mut self) {}
    fn fadeout(&self) -> i32 {
        0
    }
    fn input(&self) -> i32 {
        0
    }
    fn scene(&self) -> i32 {
        0
    }
    fn get_width(&self) -> f32 {
        1280.0
    }
    fn get_height(&self) -> f32 {
        720.0
    }
    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

/// A test state that allows setting a skin for render testing.
struct SkinTestState {
    state_data: MainStateData,
}

impl SkinTestState {
    fn new_with_skin(skin: Box<dyn SkinDrawable>) -> Self {
        let mut data = MainStateData::new(TimerManager::new());
        data.skin = Some(skin);
        Self { state_data: data }
    }
}

impl MainState for SkinTestState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::MusicSelect)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {}
    fn render(&mut self) {}
}

#[test]
fn test_render_calls_skin_draw_methods() {
    let mut mc = make_test_controller();

    // Manually set current state with a mock skin
    let mock_skin = Box::new(MockSkinDrawable::new());
    mc.current = Some(Box::new(SkinTestState::new_with_skin(mock_skin)));

    // Render should call update and draw on the skin
    mc.render();

    // Verify skin methods were called by checking the skin is still present
    // (the take/put-back pattern should preserve it)
    let state = mc.current_state().unwrap();
    assert!(
        state.main_state_data().skin.is_some(),
        "skin should be put back after render"
    );
}

#[test]
fn test_render_without_skin_does_not_panic() {
    let mut mc = make_test_controller();

    // Set a state without a skin
    let mut data = MainStateData::new(TimerManager::new());
    data.skin = None;
    let state = SkinTestState { state_data: data };
    mc.current = Some(Box::new(state));

    // Should not panic when skin is None
    mc.render();
    assert!(mc.current_state().is_some());
}

#[test]
fn test_render_skin_called_once_per_frame() {
    use std::sync::{Arc, Mutex};

    /// A mock that records call counts via shared state.
    struct CountingSkinDrawable {
        counts: Arc<Mutex<(i32, i32)>>, // (update_count, draw_count)
    }

    impl SkinDrawable for CountingSkinDrawable {
        fn draw_all_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
            self.counts.lock().expect("mutex poisoned").1 += 1;
        }

        fn update_custom_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
            self.counts.lock().expect("mutex poisoned").0 += 1;
        }

        fn mouse_pressed_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }
        fn mouse_dragged_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }
        fn prepare_skin(&mut self) {}
        fn dispose_skin(&mut self) {}
        fn fadeout(&self) -> i32 {
            0
        }
        fn input(&self) -> i32 {
            0
        }
        fn scene(&self) -> i32 {
            0
        }
        fn get_width(&self) -> f32 {
            1280.0
        }
        fn get_height(&self) -> f32 {
            720.0
        }
        fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
    }

    let counts = Arc::new(Mutex::new((0, 0)));
    let skin = Box::new(CountingSkinDrawable {
        counts: counts.clone(),
    });

    let mut mc = make_test_controller();
    mc.sprite = Some(SpriteBatch::new());
    mc.current = Some(Box::new(SkinTestState::new_with_skin(skin)));

    // Render 3 frames
    mc.render();
    mc.render();
    mc.render();

    let (update_count, draw_count) = *counts.lock().expect("mutex poisoned");
    assert_eq!(
        update_count, 3,
        "update_custom_objects_timed should be called once per frame"
    );
    assert_eq!(
        draw_count, 3,
        "draw_all_objects_timed should be called once per frame"
    );
}

/// A SkinDrawable that generates vertices via swap_sprite_batch.
/// This catches the original SpriteBatch disconnect bug: if MainController
/// doesn't call swap_sprite_batch, the mock draws to its own internal batch
/// and MainController's batch stays empty.
struct VertexGeneratingSkinDrawable {
    sprite: SpriteBatch,
}

impl VertexGeneratingSkinDrawable {
    fn new() -> Self {
        Self {
            sprite: SpriteBatch::new(),
        }
    }
}

impl SkinDrawable for VertexGeneratingSkinDrawable {
    fn draw_all_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
        // Draw a test quad to our internal sprite batch.
        // After swap_sprite_batch, this is actually MainController's batch.
        use rubato_render::texture::{Texture, TextureRegion};
        use std::sync::Arc;

        let tex = Texture {
            width: 10,
            height: 10,
            disposed: false,
            path: Some(Arc::from("test_vertex_gen")),
            rgba_data: Some(Arc::new(vec![255u8; 400])),
            ..Default::default()
        };
        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            region_x: 0,
            region_y: 0,
            region_width: 10,
            region_height: 10,
            texture: Some(tex),
        };
        self.sprite.draw_region(&region, 0.0, 0.0, 10.0, 10.0);
    }

    fn update_custom_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
    }

    fn mouse_pressed_at(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
    }
    fn mouse_dragged_at(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
    }
    fn prepare_skin(&mut self) {}
    fn dispose_skin(&mut self) {}
    fn fadeout(&self) -> i32 {
        0
    }
    fn input(&self) -> i32 {
        0
    }
    fn scene(&self) -> i32 {
        0
    }
    fn get_width(&self) -> f32 {
        1280.0
    }
    fn get_height(&self) -> f32 {
        720.0
    }

    fn swap_sprite_batch(&mut self, batch: &mut SpriteBatch) {
        std::mem::swap(&mut self.sprite, batch);
    }
}

/// Verifies end-to-end vertex flow: Skin -> swap -> draw -> swap back -> MainController sprite batch.
/// This test directly catches the SpriteBatch disconnect bug where skin drew to an internal
/// batch but MainController flushed a separate empty one.
#[test]
fn test_render_produces_vertices_via_skin() {
    let mut mc = make_test_controller();
    mc.create();

    let skin = Box::new(VertexGeneratingSkinDrawable::new());
    mc.current = Some(Box::new(SkinTestState::new_with_skin(skin)));

    // Before render, sprite batch should be empty
    let batch = mc.sprite_batch().unwrap();
    assert!(
        batch.vertices().is_empty(),
        "sprite batch should start empty"
    );

    // Render one frame
    mc.render();

    // After render, sprite batch should contain vertices from the skin
    let batch = mc.sprite_batch().unwrap();
    assert!(
        !batch.vertices().is_empty(),
        "after render, sprite batch should contain vertices drawn by skin"
    );
    assert_eq!(batch.vertices().len(), 6, "one quad = 6 vertices");
}

// --- triggerLnWarning tests ---

#[test]
fn test_trigger_ln_warning_lnmode_0_is_ln_no_warning() {
    // lnmode=0 → "LN" → no warning (default)
    let mut mc = make_test_controller();
    mc.player.play_settings.lnmode = 0;
    // Should not panic; "LN" mode does not trigger warning
    mc.trigger_ln_warning();
}

#[test]
fn test_trigger_ln_warning_lnmode_1_is_cn() {
    // lnmode=1 → "CN" → warning triggered
    let mut mc = make_test_controller();
    mc.player.play_settings.lnmode = 1;
    mc.trigger_ln_warning();
    // No assertion on log output, but should not panic
}

#[test]
fn test_trigger_ln_warning_lnmode_2_is_hcn() {
    // lnmode=2 → "HCN" → warning triggered
    let mut mc = make_test_controller();
    mc.player.play_settings.lnmode = 2;
    mc.trigger_ln_warning();
}

#[test]
fn test_trigger_ln_warning_lnmode_3_is_ln_no_warning() {
    // lnmode=3 → default "LN" → no warning
    let mut mc = make_test_controller();
    mc.player.play_settings.lnmode = 3;
    mc.trigger_ln_warning();
}

// --- setTargetList tests ---

#[test]
fn test_set_target_list_no_rivals() {
    let mut mc = make_test_controller();
    // With default player config (targetlist contains "MAX") and no rivals
    mc.set_target_list();
    // Should not panic
}

// --- updateStateReferences tests ---

#[test]
fn test_update_state_references_does_not_panic() {
    let mc = make_test_controller();
    mc.update_state_references();
}

// --- Audio driver wiring tests (Phase 24c) ---

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;
use rubato_audio::recording_audio_driver::RecordingAudioDriver;

#[test]
fn test_audio_driver_initially_none() {
    let mc = make_test_controller();
    assert!(mc.audio_processor().is_none());
}

#[test]
fn test_set_audio_driver() {
    let mut mc = make_test_controller();
    mc.set_audio_driver(Box::new(RecordingAudioDriver::new()));
    assert!(mc.audio_processor().is_some());
}

#[test]
fn test_get_audio_processor_returns_trait_ref() {
    let mut mc = make_test_controller();
    mc.set_audio_driver(Box::new(RecordingAudioDriver::new()));

    let audio = mc.audio_processor().unwrap();
    assert_eq!(audio.get_global_pitch(), 1.0);
    assert_eq!(audio.get_progress(), 1.0);
}

#[test]
fn test_get_audio_processor_mut() {
    let mut mc = make_test_controller();
    mc.set_audio_driver(Box::new(RecordingAudioDriver::new()));

    let audio = mc.audio_processor_mut().unwrap();
    audio.set_global_pitch(1.5);
    assert_eq!(audio.get_global_pitch(), 1.5);
}

#[test]
fn test_audio_driver_play_path() {
    let mut mc = make_test_controller();
    mc.set_audio_driver(Box::new(RecordingAudioDriver::new()));

    let audio = mc.audio_processor_mut().unwrap();
    audio.play_path("/test/sound.wav", 0.8, false);
    assert!(audio.is_playing_path("/test/sound.wav"));
}

#[test]
fn render_invokes_state_sync_audio_when_audio_driver_exists() {
    let render_sync_calls = Arc::new(Mutex::new(0));
    let shutdown_sync_calls = Arc::new(Mutex::new(0));
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(AudioSyncStateFactory::new(
        Arc::clone(&render_sync_calls),
        Arc::clone(&shutdown_sync_calls),
    )));
    mc.set_audio_driver(Box::new(RecordingAudioDriver::new()));

    mc.change_state(MainStateType::MusicSelect);
    mc.render();

    assert_eq!(*render_sync_calls.lock().expect("mutex poisoned"), 1);
    assert_eq!(*shutdown_sync_calls.lock().expect("mutex poisoned"), 0);
}

#[test]
fn state_transition_flushes_audio_before_and_after_shutdown() {
    // sync_audio must be called both BEFORE shutdown() (so pending audio
    // commands operate on live state) and AFTER shutdown() (so tick-based
    // processors like PreviewMusicProcessor can see the stop flag and
    // actually halt playback).
    let render_sync_calls = Arc::new(Mutex::new(0));
    let shutdown_sync_calls = Arc::new(Mutex::new(0));
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(AudioSyncStateFactory::new(
        Arc::clone(&render_sync_calls),
        Arc::clone(&shutdown_sync_calls),
    )));
    mc.set_audio_driver(Box::new(RecordingAudioDriver::new()));

    mc.change_state(MainStateType::MusicSelect);
    mc.change_state(MainStateType::Config);

    // First sync_audio fires before shutdown (render_sync_calls).
    // Second sync_audio fires after shutdown (shutdown_sync_calls).
    assert_eq!(*render_sync_calls.lock().expect("mutex poisoned"), 1);
    assert_eq!(*shutdown_sync_calls.lock().expect("mutex poisoned"), 1);
}

// --- Flag-then-tick cross-boundary integration tests ---
// These tests verify observable audio outcomes (not just call ordering)
// when tick-based processors interact with the state machine lifecycle.

/// A test state that simulates the PreviewMusicProcessor pattern:
/// - While running, sync_audio plays a preview track on the audio driver
/// - shutdown() sets a stop flag but does NOT directly stop audio
/// - The next sync_audio tick sees the flag and actually stops audio
///
/// This catches the bug class where shutdown sets a flag but no tick
/// follows to execute the side effect.
struct TickBasedPreviewState {
    state_data: MainStateData,
    preview_path: String,
    preview_started: bool,
    should_stop: bool,
}

impl TickBasedPreviewState {
    fn new(preview_path: &str) -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            preview_path: preview_path.to_string(),
            preview_started: false,
            should_stop: false,
        }
    }
}

impl MainState for TickBasedPreviewState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::MusicSelect)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {}

    fn prepare(&mut self) {}

    fn render(&mut self) {}

    fn shutdown(&mut self) {
        // Like PreviewMusicProcessor::stop(): only sets a flag.
        // Actual audio stop requires a subsequent sync_audio tick.
        self.should_stop = true;
    }

    fn sync_audio(&mut self, audio: &mut dyn AudioDriver) {
        if self.should_stop {
            // Post-shutdown tick: stop the audio.
            if self.preview_started {
                audio.stop_path(&self.preview_path);
                audio.dispose_path(&self.preview_path);
                self.preview_started = false;
            }
            return;
        }
        // Normal tick: start/keep preview playing.
        if !self.preview_started {
            audio.play_path(&self.preview_path, 0.5, true);
            self.preview_started = true;
        }
    }
}

struct TickBasedPreviewStateFactory {
    preview_path: String,
}

impl StateFactory for TickBasedPreviewStateFactory {
    fn create_state(
        &self,
        state_type: MainStateType,
        _controller: &mut MainController,
    ) -> Option<StateCreateResult> {
        if state_type == MainStateType::MusicSelect {
            Some(StateCreateResult {
                state: Box::new(TickBasedPreviewState::new(&self.preview_path)),
                target_score: None,
            })
        } else {
            Some(StateCreateResult {
                state: Box::new(TestState::new(state_type)),
                target_score: None,
            })
        }
    }
}

#[test]
fn state_transition_stops_tick_based_preview_audio() {
    // Integration test: verify the OBSERVABLE OUTCOME (audio is not playing)
    // rather than implementation details (sync_audio call count/order).
    let preview_path = "/preview/song.ogg";

    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(TickBasedPreviewStateFactory {
        preview_path: preview_path.to_string(),
    }));
    mc.set_audio_driver(Box::new(RecordingAudioDriver::new()));

    // Enter MusicSelect: the pre-shutdown sync_audio tick starts preview.
    mc.change_state(MainStateType::MusicSelect);
    // Simulate a render frame so preview starts via sync_audio.
    mc.render();

    // Verify preview is actually playing before transition.
    assert!(
        mc.audio_processor()
            .expect("audio driver should exist")
            .is_playing_path(preview_path),
        "Preview must be playing before transition"
    );

    // Transition to Decide: shutdown sets flag, post-shutdown tick must stop audio.
    mc.change_state(MainStateType::Decide);

    // Verify the observable outcome: audio driver no longer playing the preview.
    assert!(
        !mc.audio_processor()
            .expect("audio driver should exist")
            .is_playing_path(preview_path),
        "Preview audio must not be playing after state transition"
    );
}

/// A shared-state audio driver that records stop/dispose events for
/// cross-boundary assertions without requiring downcast.
struct EventTrackingAudioDriver {
    inner: RecordingAudioDriver,
    stopped: Arc<Mutex<Vec<String>>>,
    disposed: Arc<Mutex<Vec<String>>>,
}

impl EventTrackingAudioDriver {
    fn new(stopped: Arc<Mutex<Vec<String>>>, disposed: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            inner: RecordingAudioDriver::new(),
            stopped,
            disposed,
        }
    }
}

impl AudioDriver for EventTrackingAudioDriver {
    fn play_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        self.inner.play_path(path, volume, loop_play);
    }
    fn set_volume_path(&mut self, path: &str, volume: f32) {
        self.inner.set_volume_path(path, volume);
    }
    fn is_playing_path(&self, path: &str) -> bool {
        self.inner.is_playing_path(path)
    }
    fn stop_path(&mut self, path: &str) {
        self.inner.stop_path(path);
        self.stopped
            .lock()
            .expect("mutex poisoned")
            .push(path.to_string());
    }
    fn dispose_path(&mut self, path: &str) {
        self.inner.dispose_path(path);
        self.disposed
            .lock()
            .expect("mutex poisoned")
            .push(path.to_string());
    }
    fn set_model(&mut self, model: &bms_model::bms_model::BMSModel) {
        self.inner.set_model(model);
    }
    fn set_additional_key_sound(&mut self, judge: i32, fast: bool, path: Option<&str>) {
        self.inner.set_additional_key_sound(judge, fast, path);
    }
    fn abort(&mut self) {
        self.inner.abort();
    }
    fn get_progress(&self) -> f32 {
        self.inner.get_progress()
    }
    fn preload_path(&mut self, path: &str) {
        self.inner.preload_path(path);
    }
    fn play_note(&mut self, n: &bms_model::note::Note, volume: f32, pitch: i32) {
        self.inner.play_note(n, volume, pitch);
    }
    fn play_judge(&mut self, judge: i32, fast: bool) {
        self.inner.play_judge(judge, fast);
    }
    fn stop_note(&mut self, n: Option<&bms_model::note::Note>) {
        self.inner.stop_note(n);
    }
    fn set_volume_note(&mut self, n: &bms_model::note::Note, volume: f32) {
        self.inner.set_volume_note(n, volume);
    }
    fn set_global_pitch(&mut self, pitch: f32) {
        self.inner.set_global_pitch(pitch);
    }
    fn get_global_pitch(&self) -> f32 {
        self.inner.get_global_pitch()
    }
    fn dispose_old(&mut self) {
        self.inner.dispose_old();
    }
    fn dispose(&mut self) {
        self.inner.dispose();
    }
}

#[test]
fn state_transition_emits_stop_and_dispose_events_for_preview() {
    // Verify that both stop_path and dispose_path are called for the preview
    // track during state transition (not just is_playing check).
    let preview_path = "/preview/track.ogg";
    let stopped = Arc::new(Mutex::new(Vec::<String>::new()));
    let disposed = Arc::new(Mutex::new(Vec::<String>::new()));

    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(TickBasedPreviewStateFactory {
        preview_path: preview_path.to_string(),
    }));
    mc.set_audio_driver(Box::new(EventTrackingAudioDriver::new(
        Arc::clone(&stopped),
        Arc::clone(&disposed),
    )));

    mc.change_state(MainStateType::MusicSelect);
    mc.render();
    mc.change_state(MainStateType::Decide);

    let stopped_paths = stopped.lock().expect("mutex poisoned");
    let disposed_paths = disposed.lock().expect("mutex poisoned");
    assert!(
        stopped_paths.contains(&preview_path.to_string()),
        "Preview path must be stopped during transition, got: {stopped_paths:?}"
    );
    assert!(
        disposed_paths.contains(&preview_path.to_string()),
        "Preview path must be disposed during transition, got: {disposed_paths:?}"
    );
}

// --- Phase 24f: update_main_state_listener tests ---

use std::sync::{Arc, Mutex};

/// A mock listener that records calls.
struct MockStateListener {
    calls: Arc<Mutex<Vec<(ScreenType, i32)>>>,
}

impl MockStateListener {
    fn new(calls: Arc<Mutex<Vec<(ScreenType, i32)>>>) -> Self {
        Self { calls }
    }
}

impl MainStateListener for MockStateListener {
    fn update(&mut self, state: &dyn MainStateAccess, status: i32) {
        self.calls
            .lock()
            .unwrap()
            .push((state.screen_type(), status));
    }
}

#[test]
fn test_update_main_state_listener_dispatches_to_listeners() {
    let mut mc = make_test_controller();
    let calls = Arc::new(Mutex::new(Vec::new()));

    mc.add_state_listener(Box::new(MockStateListener::new(calls.clone())));
    mc.change_state(MainStateType::MusicSelect);

    // The transition_to_state calls update_main_state_listener(0) internally,
    // so we should already have one call.
    let recorded = calls.lock().expect("mutex poisoned");
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0], (ScreenType::MusicSelector, 0));
}

#[test]
fn test_update_main_state_listener_multiple_listeners() {
    let mut mc = make_test_controller();
    let calls1 = Arc::new(Mutex::new(Vec::new()));
    let calls2 = Arc::new(Mutex::new(Vec::new()));

    mc.add_state_listener(Box::new(MockStateListener::new(calls1.clone())));
    mc.add_state_listener(Box::new(MockStateListener::new(calls2.clone())));

    mc.change_state(MainStateType::Config);

    assert_eq!(calls1.lock().expect("mutex poisoned").len(), 1);
    assert_eq!(calls2.lock().expect("mutex poisoned").len(), 1);
    assert_eq!(
        calls1.lock().expect("mutex poisoned")[0],
        (ScreenType::KeyConfiguration, 0)
    );
    assert_eq!(
        calls2.lock().expect("mutex poisoned")[0],
        (ScreenType::KeyConfiguration, 0)
    );
}

#[test]
fn test_update_main_state_listener_no_state_no_dispatch() {
    let mut mc = make_test_controller();
    let calls = Arc::new(Mutex::new(Vec::new()));
    mc.add_state_listener(Box::new(MockStateListener::new(calls.clone())));

    // No current state → no dispatch
    mc.update_main_state_listener(0);
    assert!(calls.lock().expect("mutex poisoned").is_empty());
}

#[test]
fn test_update_main_state_listener_preserves_status() {
    let mut mc = make_test_controller();
    let calls = Arc::new(Mutex::new(Vec::new()));
    mc.add_state_listener(Box::new(MockStateListener::new(calls.clone())));

    mc.change_state(MainStateType::Result);
    // Clear the initial call from transition
    calls.lock().expect("mutex poisoned").clear();

    // Manual call with custom status
    mc.update_main_state_listener(42);

    let recorded = calls.lock().expect("mutex poisoned");
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0], (ScreenType::MusicResult, 42));
}

// --- Phase 24f: StateReferencesCallback tests ---

struct MockReferencesCallback {
    called: Arc<Mutex<bool>>,
}

impl StateReferencesCallback for MockReferencesCallback {
    fn update_references(&self, _config: &Config, _player: &PlayerConfig) {
        *self.called.lock().expect("mutex poisoned") = true;
    }
}

#[test]
fn test_update_state_references_calls_callback() {
    let mut mc = make_test_controller();
    let called = Arc::new(Mutex::new(false));
    mc.set_state_references_callback(Box::new(MockReferencesCallback {
        called: called.clone(),
    }));

    mc.update_state_references();
    assert!(*called.lock().expect("mutex poisoned"));
}

#[test]
fn test_update_state_references_without_callback_does_not_panic() {
    let mc = make_test_controller();
    mc.update_state_references();
    // Should not panic
}

// --- Phase 24f: periodic_config_save tests ---

#[test]
fn test_periodic_config_save_skips_during_play() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::Play);
    // Set last_config_save to a long time ago to ensure it would trigger otherwise
    mc.lifecycle.last_config_save = Instant::now() - std::time::Duration::from_secs(300);

    // Should skip because current state is Play
    mc.periodic_config_save();
    // Verify it was NOT reset (still old)
    assert!(mc.lifecycle.last_config_save.elapsed().as_secs() >= 299);
}

#[test]
fn test_periodic_config_save_does_not_trigger_within_interval() {
    let mut mc = make_test_controller();
    mc.change_state(MainStateType::MusicSelect);
    mc.lifecycle.last_config_save = Instant::now();

    // Should not trigger because less than 2 minutes elapsed
    mc.periodic_config_save();
    // last_config_save should not have changed significantly
    assert!(mc.lifecycle.last_config_save.elapsed().as_millis() < 100);
}

// --- Phase 24f: add_state_listener tests ---

#[test]
fn test_add_state_listener() {
    let mut mc = make_test_controller();
    assert!(mc.state_listener.is_empty());

    let calls = Arc::new(Mutex::new(Vec::new()));
    mc.add_state_listener(Box::new(MockStateListener::new(calls)));
    assert_eq!(mc.state_listener.len(), 1);
}

// --- Phase 24f: create() calls update_state_references ---

#[test]
fn test_create_calls_update_state_references() {
    let mut mc = make_test_controller();
    let called = Arc::new(Mutex::new(false));
    mc.set_state_references_callback(Box::new(MockReferencesCallback {
        called: called.clone(),
    }));

    mc.create();
    assert!(*called.lock().expect("mutex poisoned"));
}

// --- Phase 41i: Loudness analyzer tests ---

#[test]
fn test_loudness_analyzer_initialized() {
    let mc = make_test_controller();
    assert!(mc.loudness_analyzer().is_some());
}

#[test]
fn test_loudness_analyzer_is_available() {
    let mc = make_test_controller();
    let analyzer = mc.loudness_analyzer().unwrap();
    assert!(analyzer.is_available());
}

#[test]
fn test_loudness_analyzer_shutdown_no_panic() {
    let mut mc = make_test_controller();
    mc.shutdown_loudness_analyzer();
    // Should not panic
}

#[test]
fn test_get_sound_manager_mut() {
    let mut mc = make_test_controller();
    assert!(mc.sound_manager_mut().is_some());
}

// --- exit() and save_config() tests ---

/// Mutex to serialize tests that change the process-wide CWD.
/// Config::write() writes to CWD-relative "config_sys.json", so tests
/// that verify file I/O must change CWD to a temp dir. This mutex
/// prevents concurrent tests from racing on CWD.
static CWD_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_exit_sets_exit_requested_flag() {
    let _lock = CWD_MUTEX.lock().expect("mutex poisoned");
    let dir = tempfile::tempdir().unwrap();
    let _cwd = CurrentDirGuard::set(dir.path());

    let mc = make_test_controller();
    assert!(!mc.is_exit_requested());

    mc.exit();

    assert!(mc.is_exit_requested());
}

#[test]
fn test_exit_calls_save_config() {
    let _lock = CWD_MUTEX.lock().expect("mutex poisoned");
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config_sys.json");
    let _cwd = CurrentDirGuard::set(dir.path());

    let mc = make_test_controller();
    mc.exit();

    // exit() should have called save_config(), which writes config_sys.json
    assert!(
        config_path.exists(),
        "config_sys.json should be written by exit()"
    );
}

#[test]
fn test_save_config_writes_config_sys_json() {
    let _lock = CWD_MUTEX.lock().expect("mutex poisoned");
    let dir = tempfile::tempdir().unwrap();
    let _cwd = CurrentDirGuard::set(dir.path());

    let mc = make_test_controller();
    mc.save_config();

    let config_path = dir.path().join("config_sys.json");
    assert!(config_path.exists(), "config_sys.json should be created");

    // Verify it's valid JSON that round-trips back to Config
    let contents = std::fs::read_to_string(&config_path).unwrap();
    let deserialized: Config = serde_json::from_str(&contents).unwrap();
    assert_eq!(
        deserialized.display.window_width,
        mc.config.display.window_width
    );
}

#[test]
fn test_save_config_writes_player_config_json() {
    let _lock = CWD_MUTEX.lock().expect("mutex poisoned");
    let dir = tempfile::tempdir().unwrap();
    let _cwd = CurrentDirGuard::set(dir.path());

    let mut config = Config::default();
    config.paths.playerpath = dir.path().join("player").to_string_lossy().to_string();
    let mut player = PlayerConfig::default();
    player.id = Some("test_player".to_string());
    player.name = "TestName".to_string();

    let mc = MainController::new(None, config.clone(), player, None, false);
    // Create the player directory so write succeeds
    std::fs::create_dir_all(format!("{}/test_player", config.paths.playerpath)).unwrap();

    mc.save_config();

    let player_config_path = PathBuf::from(format!(
        "{}/test_player/config_player.json",
        config.paths.playerpath
    ));
    assert!(
        player_config_path.exists(),
        "config_player.json should be created"
    );

    let contents = std::fs::read_to_string(&player_config_path).unwrap();
    let deserialized: PlayerConfig = serde_json::from_str(&contents).unwrap();
    assert_eq!(deserialized.name, "TestName");
}

#[test]
fn test_is_exit_requested_initially_false() {
    let mc = make_test_controller();
    assert!(!mc.is_exit_requested());
}

// --- set_model wiring test ---

use std::sync::atomic::{AtomicI32, Ordering as AtomicOrdering};

/// A test state that owns a BMSModel and exposes it via bms_model().
struct ModelTestState {
    state_data: MainStateData,
    model: BMSModel,
}

impl ModelTestState {
    fn new() -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            model: BMSModel::new(),
        }
    }
}

impl MainState for ModelTestState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Play)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {}
    fn render(&mut self) {}

    fn bms_model(&self) -> Option<&BMSModel> {
        Some(&self.model)
    }
}

/// Mock AudioDriver that uses a shared counter for set_model calls.
struct SetModelTrackingAudioDriver {
    set_model_count: Arc<AtomicI32>,
}

impl AudioDriver for SetModelTrackingAudioDriver {
    fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {}
    fn set_volume_path(&mut self, _path: &str, _volume: f32) {}
    fn is_playing_path(&self, _path: &str) -> bool {
        false
    }
    fn stop_path(&mut self, _path: &str) {}
    fn dispose_path(&mut self, _path: &str) {}
    fn set_model(&mut self, _model: &BMSModel) {
        self.set_model_count.fetch_add(1, AtomicOrdering::SeqCst);
    }
    fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}
    fn abort(&mut self) {}
    fn get_progress(&self) -> f32 {
        1.0
    }
    fn play_note(&mut self, _n: &Note, _volume: f32, _pitch: i32) {}
    fn play_judge(&mut self, _judge: i32, _fast: bool) {}
    fn stop_note(&mut self, _n: Option<&Note>) {}
    fn set_volume_note(&mut self, _n: &Note, _volume: f32) {}
    fn set_global_pitch(&mut self, _pitch: f32) {}
    fn get_global_pitch(&self) -> f32 {
        1.0
    }
    fn dispose_old(&mut self) {}
    fn dispose(&mut self) {}
}

struct ModelTestStateFactory;

impl StateFactory for ModelTestStateFactory {
    fn create_state(
        &self,
        _state_type: MainStateType,
        _controller: &mut MainController,
    ) -> Option<StateCreateResult> {
        Some(StateCreateResult {
            state: Box::new(ModelTestState::new()),
            target_score: None,
        })
    }
}

#[test]
fn test_transition_to_play_calls_audio_set_model() {
    let set_model_count = Arc::new(AtomicI32::new(0));
    let mut mc = make_test_controller();
    mc.set_audio_driver(Box::new(SetModelTrackingAudioDriver {
        set_model_count: Arc::clone(&set_model_count),
    }));
    mc.set_state_factory(Box::new(ModelTestStateFactory));

    mc.change_state(MainStateType::Play);

    assert_eq!(
        set_model_count.load(AtomicOrdering::SeqCst),
        1,
        "audio.set_model() must be called when transitioning to a state that has a BMSModel"
    );
}

// --- ScoreHandoff transfer tests ---

/// A test state that produces a ScoreHandoff on the first render() call.
struct HandoffTestState {
    state_data: MainStateData,
    handoff: Option<rubato_types::score_handoff::ScoreHandoff>,
}

impl HandoffTestState {
    fn new(handoff: rubato_types::score_handoff::ScoreHandoff) -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            handoff: Some(handoff),
        }
    }
}

impl MainState for HandoffTestState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Play)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {}

    fn render(&mut self) {}

    fn take_score_handoff(&mut self) -> Option<rubato_types::score_handoff::ScoreHandoff> {
        self.handoff.take()
    }
}

fn make_handoff(
    assist: i32,
    freq_on: bool,
    force_no_ir_send: bool,
) -> rubato_types::score_handoff::ScoreHandoff {
    rubato_types::score_handoff::ScoreHandoff {
        score_data: None,
        combo: 0,
        maxcombo: 0,
        gauge: vec![],
        groove_gauge: None,
        assist,
        freq_on,
        force_no_ir_send,
        replay_data: None,
        updated_model: None,
    }
}

#[test]
fn test_handoff_update_score_false_when_assist_nonzero() {
    let mut mc = make_test_controller();
    // Set up resource
    mc.restore_player_resource(PlayerResource::new(
        Config::default(),
        PlayerConfig::default(),
    ));
    // Verify default
    assert!(
        mc.resource.as_ref().unwrap().update_score,
        "update_score should default to true"
    );

    // Install a state that produces a handoff with assist=1
    mc.current = Some(Box::new(HandoffTestState::new(make_handoff(
        1, false, false,
    ))));

    mc.render();

    let res = mc.resource.as_ref().unwrap();
    assert_eq!(res.assist, 1);
    assert!(
        !res.update_score,
        "update_score must be false when assist != 0"
    );
}

#[test]
fn test_handoff_update_score_true_when_assist_zero() {
    let mut mc = make_test_controller();
    mc.restore_player_resource(PlayerResource::new(
        Config::default(),
        PlayerConfig::default(),
    ));

    mc.current = Some(Box::new(HandoffTestState::new(make_handoff(
        0, false, false,
    ))));

    mc.render();

    let res = mc.resource.as_ref().unwrap();
    assert_eq!(res.assist, 0);
    assert!(
        res.update_score,
        "update_score must be true when assist == 0"
    );
}

#[test]
fn test_handoff_transfers_freq_on_and_force_no_ir_send() {
    let mut mc = make_test_controller();
    mc.restore_player_resource(PlayerResource::new(
        Config::default(),
        PlayerConfig::default(),
    ));

    // Verify defaults
    let res = mc.resource.as_ref().unwrap();
    assert!(!res.freq_on);
    assert!(!res.force_no_ir_send);

    mc.current = Some(Box::new(HandoffTestState::new(make_handoff(0, true, true))));

    mc.render();

    let res = mc.resource.as_ref().unwrap();
    assert!(res.freq_on, "freq_on must be transferred from handoff");
    assert!(
        res.force_no_ir_send,
        "force_no_ir_send must be transferred from handoff"
    );
}

#[test]
fn test_handoff_freq_flags_false_by_default() {
    let mut mc = make_test_controller();
    mc.restore_player_resource(PlayerResource::new(
        Config::default(),
        PlayerConfig::default(),
    ));

    mc.current = Some(Box::new(HandoffTestState::new(make_handoff(
        0, false, false,
    ))));

    mc.render();

    let res = mc.resource.as_ref().unwrap();
    assert!(!res.freq_on);
    assert!(!res.force_no_ir_send);
}

// --- UpdatePlayConfig forwarding to current state ---

/// Test state that captures play config updates via shared Arc for external inspection.
struct PlayConfigReceiverState {
    state_data: MainStateData,
    received: Arc<Mutex<Vec<(bms_model::mode::Mode, rubato_types::play_config::PlayConfig)>>>,
}

impl PlayConfigReceiverState {
    fn new(
        received: Arc<Mutex<Vec<(bms_model::mode::Mode, rubato_types::play_config::PlayConfig)>>>,
    ) -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            received,
        }
    }
}

impl MainState for PlayConfigReceiverState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Play)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {}
    fn render(&mut self) {}

    fn receive_updated_play_config(
        &mut self,
        mode: bms_model::mode::Mode,
        play_config: rubato_types::play_config::PlayConfig,
    ) {
        self.received.lock().unwrap().push((mode, play_config));
    }
}

#[test]
fn update_play_config_command_forwards_to_current_state() {
    let mut mc = make_test_controller();
    let received = Arc::new(Mutex::new(Vec::new()));
    mc.current = Some(Box::new(PlayConfigReceiverState::new(received.clone())));

    // Set a live hispeed that differs from default (simulating scroll wheel change)
    let live_hispeed = 7.0;
    mc.player
        .play_config(bms_model::mode::Mode::BEAT_7K)
        .playconfig
        .hispeed = live_hispeed;

    // Queue an UpdatePlayConfig command with modmenu-managed fields changed
    // and a stale hispeed that must NOT overwrite the live value
    let mut pc = rubato_types::play_config::PlayConfig::default();
    pc.hispeed = 1.0; // stale -- must NOT overwrite live hispeed
    pc.enablelift = true;
    pc.lanecover = 0.42;
    pc.enablelanecover = true;
    mc.command_queue.push(
        rubato_types::main_controller_access::MainControllerCommand::UpdatePlayConfig(
            bms_model::mode::Mode::BEAT_7K,
            Box::new(pc),
        ),
    );

    mc.process_queued_controller_commands();

    // Verify MainController's authoritative PlayerConfig: non-modmenu fields preserved
    let mc_pc = &mc
        .player
        .play_config_ref(bms_model::mode::Mode::BEAT_7K)
        .playconfig;
    assert!(
        (mc_pc.hispeed - live_hispeed).abs() < f32::EPSILON,
        "hispeed should be preserved (live={}), got {}",
        live_hispeed,
        mc_pc.hispeed
    );
    // Modmenu-managed fields should be updated
    assert!(mc_pc.enablelift);
    assert!(mc_pc.enablelanecover);
    assert!((mc_pc.lanecover - 0.42).abs() < f32::EPSILON);

    // Verify the current state received the forwarded config (raw, unmerged)
    let updates = received.lock().unwrap();
    assert_eq!(updates.len(), 1, "state should receive exactly one update");
    assert_eq!(updates[0].0, bms_model::mode::Mode::BEAT_7K);
    assert!(updates[0].1.enablelift, "forwarded enablelift should match");
}

// --- Skin dispose_skin() called before dropping tests ---

/// A SkinDrawable mock that records dispose_skin() calls via shared counter.
struct DisposableSkinDrawable {
    dispose_count: Arc<Mutex<usize>>,
}

impl DisposableSkinDrawable {
    fn new(dispose_count: Arc<Mutex<usize>>) -> Self {
        Self { dispose_count }
    }
}

impl SkinDrawable for DisposableSkinDrawable {
    fn draw_all_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
    }
    fn update_custom_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
    }
    fn mouse_pressed_at(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
    }
    fn mouse_dragged_at(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
    }
    fn prepare_skin(&mut self) {}
    fn dispose_skin(&mut self) {
        *self.dispose_count.lock().unwrap() += 1;
    }
    fn fadeout(&self) -> i32 {
        0
    }
    fn input(&self) -> i32 {
        0
    }
    fn scene(&self) -> i32 {
        0
    }
    fn get_width(&self) -> f32 {
        1280.0
    }
    fn get_height(&self) -> f32 {
        720.0
    }
    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

/// A state factory that produces states carrying a disposable skin.
struct DisposableSkinStateFactory {
    dispose_count: Arc<Mutex<usize>>,
}

impl StateFactory for DisposableSkinStateFactory {
    fn create_state(
        &self,
        state_type: MainStateType,
        _controller: &mut MainController,
    ) -> Option<StateCreateResult> {
        let skin = Box::new(DisposableSkinDrawable::new(self.dispose_count.clone()));
        let mut data = MainStateData::new(TimerManager::new());
        data.skin = Some(skin);
        let state = DisposableSkinState {
            state_data: data,
            state_type,
        };
        Some(StateCreateResult {
            state: Box::new(state),
            target_score: None,
        })
    }
}

struct DisposableSkinState {
    state_data: MainStateData,
    state_type: MainStateType,
}

impl MainState for DisposableSkinState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(self.state_type)
    }
    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }
    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }
    fn create(&mut self) {}
    fn render(&mut self) {}
}

#[test]
fn transition_to_state_calls_dispose_skin_on_old_state() {
    let dispose_count = Arc::new(Mutex::new(0usize));
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(DisposableSkinStateFactory {
        dispose_count: dispose_count.clone(),
    }));

    // Transition to first state (creates skin via factory)
    mc.change_state(MainStateType::Config);
    assert_eq!(
        *dispose_count.lock().unwrap(),
        0,
        "no dispose on first transition"
    );

    // Transition to second state -- old state's skin.dispose_skin() must be called
    mc.change_state(MainStateType::SkinConfig);
    assert_eq!(
        *dispose_count.lock().unwrap(),
        1,
        "dispose_skin must be called on old state's skin during transition"
    );
}

#[test]
fn default_mainstate_dispose_calls_dispose_skin() {
    let dispose_count = Arc::new(Mutex::new(0usize));
    let skin = Box::new(DisposableSkinDrawable::new(dispose_count.clone()));
    let mut state = DisposableSkinState {
        state_data: MainStateData::new(TimerManager::new()),
        state_type: MainStateType::Config,
    };
    state.state_data.skin = Some(skin);

    // Call the default MainState::dispose()
    state.dispose();

    assert_eq!(
        *dispose_count.lock().unwrap(),
        1,
        "default dispose() must call dispose_skin() before dropping skin"
    );
    assert!(
        state.state_data.skin.is_none(),
        "skin should be None after dispose()"
    );
}

#[test]
fn test_handoff_updated_model_propagates_to_resource() {
    let mut mc = make_test_controller();
    mc.restore_player_resource(PlayerResource::new(
        Config::default(),
        PlayerConfig::default(),
    ));

    // Set up a model on the resource so songdata exists
    {
        let res = mc.resource.as_mut().unwrap();
        let mut model = BMSModel::new();
        model.set_mode(bms_model::mode::Mode::BEAT_7K);
        model.judgerank = 100;
        let mut tl = bms_model::time_line::TimeLine::new(0.0, 1_000_000, 8);
        tl.set_note(0, Some(bms_model::note::Note::new_normal(1)));
        model.timelines = vec![tl];
        let sd = rubato_types::song_data::SongData::new_from_model(model, false);
        res.set_songdata(sd);
    }

    // Verify the resource model note has state=0 initially
    {
        let res = mc.resource.as_ref().unwrap();
        let note = res.bms_model().unwrap().timelines[0].note(0).unwrap();
        assert_eq!(note.state(), 0, "Initial note state should be 0");
    }

    // Create a handoff with an updated model where note has state=1
    let mut updated_model = BMSModel::new();
    updated_model.set_mode(bms_model::mode::Mode::BEAT_7K);
    updated_model.judgerank = 100;
    let mut tl = bms_model::time_line::TimeLine::new(0.0, 1_000_000, 8);
    let mut note = bms_model::note::Note::new_normal(1);
    note.set_state(1);
    note.set_micro_play_time(500);
    tl.set_note(0, Some(note));
    updated_model.timelines = vec![tl];

    let mut handoff = make_handoff(0, false, false);
    handoff.updated_model = Some(updated_model);

    mc.current = Some(Box::new(HandoffTestState::new(handoff)));
    mc.render();

    // After handoff, the resource model should have the updated note states
    let res = mc.resource.as_ref().unwrap();
    let model = res.bms_model().unwrap();
    let note = model.timelines[0].note(0).unwrap();
    assert_eq!(
        note.state(),
        1,
        "Note state should be updated from handoff model"
    );
    assert_eq!(
        note.micro_play_time(),
        500,
        "Note play_time should be updated from handoff model"
    );
}

// ============================================================
// Input gate time override tests
// ============================================================

/// A state that counts how many times `input()` is called.
struct InputCountingState {
    state_data: MainStateData,
    state_type: MainStateType,
    input_count: Arc<Mutex<usize>>,
}

impl InputCountingState {
    fn new(state_type: MainStateType, input_count: Arc<Mutex<usize>>) -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            state_type,
            input_count,
        }
    }
}

impl MainState for InputCountingState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(self.state_type)
    }
    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }
    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }
    fn create(&mut self) {}
    fn render(&mut self) {}
    fn input(&mut self) {
        *self.input_count.lock().unwrap() += 1;
    }
}

struct InputCountingFactory {
    input_count: Arc<Mutex<usize>>,
}

impl StateFactory for InputCountingFactory {
    fn create_state(
        &self,
        state_type: MainStateType,
        _controller: &mut MainController,
    ) -> Option<StateCreateResult> {
        Some(StateCreateResult {
            state: Box::new(InputCountingState::new(
                state_type,
                self.input_count.clone(),
            )),
            target_score: None,
        })
    }
}

#[test]
fn input_gate_override_forces_input_processing() {
    let input_count = Arc::new(Mutex::new(0usize));
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(InputCountingFactory {
        input_count: input_count.clone(),
    }));
    mc.change_state(MainStateType::MusicSelect);

    // Set prevtime to far future so the wall-clock gate would never pass
    mc.lifecycle.prevtime = i64::MAX / 2;

    // Without override, render() should skip input processing
    mc.render();
    assert_eq!(
        *input_count.lock().unwrap(),
        0,
        "input should NOT be called when wall-clock time <= prevtime"
    );

    // With override, render() should process input even though wall-clock < prevtime
    mc.set_input_gate_time_override(mc.lifecycle.prevtime + 1);
    mc.render();
    assert_eq!(
        *input_count.lock().unwrap(),
        1,
        "input SHOULD be called when override_input_gate_time is set"
    );
}

#[test]
fn input_gate_override_is_consumed_after_render() {
    let input_count = Arc::new(Mutex::new(0usize));
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(InputCountingFactory {
        input_count: input_count.clone(),
    }));
    mc.change_state(MainStateType::MusicSelect);

    // Set prevtime to far future
    mc.lifecycle.prevtime = i64::MAX / 2;

    // Set override once
    mc.set_input_gate_time_override(mc.lifecycle.prevtime + 1);
    mc.render();
    assert_eq!(*input_count.lock().unwrap(), 1);

    // Second render without re-setting override should skip input
    // (override was consumed by .take())
    mc.render();
    assert_eq!(
        *input_count.lock().unwrap(),
        1,
        "override should be consumed (taken) after one render call"
    );
}

// ============================================================
// Offset unification and MainControllerAccess delegation
// ============================================================

#[test]
fn offset_value_returns_skin_offset_from_controller() {
    let dir = tempfile::tempdir().unwrap();
    let _cwd = CurrentDirGuard::set(dir.path());

    let mc = MainController::new(
        None,
        Config::default(),
        PlayerConfig::default(),
        None,
        false,
    );

    // Default offset at index 0 should be all zeros
    let offset = mc.offset(0);
    assert!(offset.is_some(), "offset(0) should return Some");
    let o = offset.unwrap();
    assert_eq!(o.x, 0.0);
    assert_eq!(o.y, 0.0);

    // MainControllerAccess trait should delegate to the same data
    let access: &dyn MainControllerAccess = &mc;
    let trait_offset = access.offset_value(0);
    assert!(
        trait_offset.is_some(),
        "offset_value(0) via trait should return Some"
    );

    // Out-of-range should return None
    assert!(access.offset_value(-1).is_none());
    assert!(access.offset_value(999).is_none());
}

#[test]
fn offset_mut_updates_are_visible_through_offset_value() {
    let dir = tempfile::tempdir().unwrap();
    let _cwd = CurrentDirGuard::set(dir.path());

    let mut mc = MainController::new(
        None,
        Config::default(),
        PlayerConfig::default(),
        None,
        false,
    );

    // Write a value via offset_mut
    if let Some(o) = mc.offset_mut(5) {
        o.x = 42.0;
        o.y = -7.5;
    }

    // Read it back via offset_value (trait method)
    let access: &dyn MainControllerAccess = &mc;
    let o = access
        .offset_value(5)
        .expect("offset_value(5) should be Some");
    assert_eq!(o.x, 42.0);
    assert_eq!(o.y, -7.5);
}

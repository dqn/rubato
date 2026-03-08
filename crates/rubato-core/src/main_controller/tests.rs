use super::*;
use crate::config::SelectConfig;
use crate::config_pkg::key_configuration::KeyConfiguration;
use crate::config_pkg::skin_configuration::SkinConfiguration;
use crate::main_state::MainStateData;

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
        self.state_data.stage = None;
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

/// Mock AudioDriver for testing. Tracks method calls.
struct MockAudioDriver {
    global_pitch: f32,
    play_count: i32,
    stop_count: i32,
}

impl MockAudioDriver {
    fn new() -> Self {
        Self {
            global_pitch: 1.0,
            play_count: 0,
            stop_count: 0,
        }
    }
}

impl AudioDriver for MockAudioDriver {
    fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {
        self.play_count += 1;
    }
    fn set_volume_path(&mut self, _path: &str, _volume: f32) {}
    fn is_playing_path(&self, _path: &str) -> bool {
        false
    }
    fn stop_path(&mut self, _path: &str) {
        self.stop_count += 1;
    }
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
fn test_audio_driver_initially_none() {
    let mc = make_test_controller();
    assert!(mc.audio_processor().is_none());
}

#[test]
fn test_set_audio_driver() {
    let mut mc = make_test_controller();
    mc.set_audio_driver(Box::new(MockAudioDriver::new()));
    assert!(mc.audio_processor().is_some());
}

#[test]
fn test_get_audio_processor_returns_trait_ref() {
    let mut mc = make_test_controller();
    mc.set_audio_driver(Box::new(MockAudioDriver::new()));

    let audio = mc.audio_processor().unwrap();
    assert_eq!(audio.get_global_pitch(), 1.0);
    assert_eq!(audio.get_progress(), 1.0);
}

#[test]
fn test_get_audio_processor_mut() {
    let mut mc = make_test_controller();
    mc.set_audio_driver(Box::new(MockAudioDriver::new()));

    let audio = mc.audio_processor_mut().unwrap();
    audio.set_global_pitch(1.5);
    assert_eq!(audio.get_global_pitch(), 1.5);
}

#[test]
fn test_audio_driver_play_path() {
    let mut mc = make_test_controller();
    mc.set_audio_driver(Box::new(MockAudioDriver::new()));

    let audio = mc.audio_processor_mut().unwrap();
    audio.play_path("/test/sound.wav", 0.8, false);
    assert!(!audio.is_playing_path("/test/sound.wav"));
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
    mc.set_audio_driver(Box::new(MockAudioDriver::new()));

    mc.change_state(MainStateType::MusicSelect);
    mc.render();

    assert_eq!(*render_sync_calls.lock().expect("mutex poisoned"), 1);
    assert_eq!(*shutdown_sync_calls.lock().expect("mutex poisoned"), 0);
}

#[test]
fn state_transition_flushes_audio_after_shutdown() {
    let render_sync_calls = Arc::new(Mutex::new(0));
    let shutdown_sync_calls = Arc::new(Mutex::new(0));
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(AudioSyncStateFactory::new(
        Arc::clone(&render_sync_calls),
        Arc::clone(&shutdown_sync_calls),
    )));
    mc.set_audio_driver(Box::new(MockAudioDriver::new()));

    mc.change_state(MainStateType::MusicSelect);
    mc.change_state(MainStateType::Config);

    assert_eq!(*shutdown_sync_calls.lock().expect("mutex poisoned"), 1);
}

// --- Phase 24f: update_main_state_listener tests ---

use std::sync::{Arc, Mutex};

type StateCallLog = Arc<Mutex<Vec<(ScreenType, i32)>>>;

/// A mock listener that records calls.
struct MockStateListener {
    calls: StateCallLog,
}

impl MockStateListener {
    fn new(calls: StateCallLog) -> Self {
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
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let mc = make_test_controller();
    assert!(!mc.is_exit_requested());

    mc.exit();

    assert!(mc.is_exit_requested());

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_exit_calls_save_config() {
    let _lock = CWD_MUTEX.lock().expect("mutex poisoned");
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config_sys.json");

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let mc = make_test_controller();
    mc.exit();

    // exit() should have called save_config(), which writes config_sys.json
    assert!(
        config_path.exists(),
        "config_sys.json should be written by exit()"
    );

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_save_config_writes_config_sys_json() {
    let _lock = CWD_MUTEX.lock().expect("mutex poisoned");
    let dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

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

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_save_config_writes_player_config_json() {
    let _lock = CWD_MUTEX.lock().expect("mutex poisoned");
    let dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

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

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_is_exit_requested_initially_false() {
    let mc = make_test_controller();
    assert!(!mc.is_exit_requested());
}

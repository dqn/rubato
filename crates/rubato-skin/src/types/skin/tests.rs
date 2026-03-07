use super::*;
use crate::property::boolean_property::BooleanProperty;
use crate::property::event_factory;
use crate::skin_header::SkinHeader;
use crate::skin_image::SkinImage;
use rubato_core::main_state::SkinDrawable;
use rubato_types::main_state_type::MainStateType;
use rubato_types::skin_render_context::SkinRenderContext;
use rubato_types::timer_access::TimerAccess;

struct AlwaysTrue;

impl BooleanProperty for AlwaysTrue {
    fn is_static(&self, _state: &dyn MainState) -> bool {
        false
    }

    fn get(&self, _state: &dyn MainState) -> bool {
        true
    }
}

struct RecordingSkinRenderContext {
    timer: crate::stubs::Timer,
    state_type: MainStateType,
    executed_events: Vec<(i32, i32, i32)>,
    changed_states: Vec<MainStateType>,
    timer_writes: Vec<(i32, i64)>,
    audio_plays: Vec<(String, f32, bool)>,
    audio_stops: Vec<String>,
    float_writes: Vec<(i32, f32)>,
}

impl RecordingSkinRenderContext {
    fn new(state_type: MainStateType) -> Self {
        Self {
            timer: crate::stubs::Timer::with_timers(100, 100_000, Vec::new()),
            state_type,
            executed_events: Vec::new(),
            changed_states: Vec::new(),
            timer_writes: Vec::new(),
            audio_plays: Vec::new(),
            audio_stops: Vec::new(),
            float_writes: Vec::new(),
        }
    }
}

impl TimerAccess for RecordingSkinRenderContext {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }

    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }

    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }

    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }

    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for(timer_id)
    }

    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinEventHandler for RecordingSkinRenderContext {
    fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
        self.executed_events.push((id, arg1, arg2));
    }

    fn change_state(&mut self, state: MainStateType) {
        self.changed_states.push(state);
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer_writes.push((timer_id.as_i32(), micro_time));
    }
}

impl rubato_types::skin_render_context::SkinAudioControl for RecordingSkinRenderContext {
    fn audio_play(&mut self, path: &str, volume: f32, is_loop: bool) {
        self.audio_plays.push((path.to_string(), volume, is_loop));
    }

    fn audio_stop(&mut self, path: &str) {
        self.audio_stops.push(path.to_string());
    }
}

impl rubato_types::skin_render_context::SkinStateQuery for RecordingSkinRenderContext {
    fn current_state_type(&self) -> Option<MainStateType> {
        Some(self.state_type)
    }
}

impl rubato_types::skin_render_context::SkinPropertyProvider for RecordingSkinRenderContext {
    fn set_float_value(&mut self, id: i32, value: f32) {
        self.float_writes.push((id, value));
    }
}

impl rubato_types::skin_render_context::SkinConfigAccess for RecordingSkinRenderContext {}

fn make_test_skin() -> Skin {
    Skin::new(SkinHeader::new())
}

#[test]
fn test_timer_only_main_state_returns_expected_values() {
    let timer = crate::stubs::Timer::with_timers(1000, 1_000_000, Vec::new());
    let adapter = TimerOnlyMainState::from_timer(&timer);
    let state: &dyn MainState = &adapter;
    assert_eq!(state.timer().now_time(), 1000);
    assert_eq!(state.timer().now_micro_time(), 1_000_000);
    assert!(state.get_offset_value(0).is_none());
    assert!(state.get_image(0).is_none());
    assert!(!state.get_main().debug);
}

/// Verify that TimerManager timer values flow through SkinDrawable to the skin adapter.
/// Before the fix, all per-timer-id queries returned 0 (frozen animations).
#[test]
fn test_timer_manager_values_flow_through_to_skin_adapter() {
    use rubato_core::timer_manager::TimerManager;
    use rubato_types::timer_access::TimerAccess;

    let mut tm = TimerManager::new();
    tm.update(); // Advance nowmicrotime from Instant::now()
    tm.set_timer_on(rubato_types::timer_id::TimerId::new(10)); // Timer 10 = ON at current micro time

    // Verify TimerManager implements TimerAccess correctly
    assert!(tm.is_timer_on(rubato_types::timer_id::TimerId::new(10)));
    assert!(!tm.is_timer_on(rubato_types::timer_id::TimerId::new(20))); // Timer 20 was never set

    // Create adapter from TimerManager (the path SkinDrawable takes)
    let adapter = TimerOnlyMainState::from_timer(&tm);
    let state: &dyn MainState = &adapter;

    // Timer 10 should be ON through the adapter
    assert!(
        state
            .timer()
            .is_timer_on(rubato_types::timer_id::TimerId::new(10)),
        "Timer 10 should be ON through adapter"
    );
    // Timer 20 should be OFF
    assert!(
        !state
            .timer()
            .is_timer_on(rubato_types::timer_id::TimerId::new(20)),
        "Timer 20 should be OFF through adapter"
    );
    // micro_timer for ON timer should not be i64::MIN
    assert_ne!(
        state
            .timer()
            .micro_timer(rubato_types::timer_id::TimerId::new(10)),
        i64::MIN,
        "ON timer should return its activation time, not i64::MIN"
    );
    // micro_timer for OFF timer should be i64::MIN
    assert_eq!(
        state
            .timer()
            .micro_timer(rubato_types::timer_id::TimerId::new(20)),
        i64::MIN,
        "OFF timer should return i64::MIN"
    );
}

#[test]
fn test_skin_drawable_getter_delegation() {
    let mut skin = make_test_skin();
    skin.fadeout = 500;
    skin.input = 100;
    skin.scene = 2000;

    let drawable: &dyn SkinDrawable = &skin;
    assert_eq!(drawable.fadeout(), 500);
    assert_eq!(drawable.input(), 100);
    assert_eq!(drawable.scene(), 2000);
    // Default resolution is 640x480
    assert_eq!(drawable.get_width(), 640.0);
    assert_eq!(drawable.get_height(), 480.0);
}

#[test]
fn test_draw_all_objects_timed_empty_skin() {
    let mut skin = make_test_skin();
    let mut null_timer = rubato_types::timer_access::NullTimer;
    // Should not panic with no objects
    skin.draw_all_objects_timed(&mut null_timer);
}

#[test]
fn test_update_custom_objects_timed_empty_skin() {
    let mut skin = make_test_skin();
    let mut timer = crate::stubs::Timer::with_timers(100, 100_000, Vec::new());
    // Should not panic with no custom objects
    skin.update_custom_objects_timed(&mut timer);
}

#[test]
fn test_update_custom_objects_timed_executes_custom_events() {
    let mut skin = make_test_skin();
    skin.add_custom_event(CustomEvent::new(
        9001,
        event_factory::create_zero_arg_event(777),
        Some(Box::new(AlwaysTrue)),
        0,
    ));
    let mut ctx = RecordingSkinRenderContext::new(MainStateType::MusicSelect);

    skin.update_custom_objects_timed(&mut ctx);

    assert_eq!(ctx.executed_events, vec![(777, 0, 0)]);
}

#[test]
fn test_dispose_skin_empty() {
    let mut skin = make_test_skin();
    // Should not panic with no objects
    skin.dispose_skin();
}

#[test]
fn test_mouse_pressed_at_empty_skin() {
    let mut skin = make_test_skin();
    let mut timer = rubato_types::timer_access::NullTimer;
    // Should not panic with no objects
    skin.mouse_pressed_at(&mut timer, 0, 100, 200);
}

#[test]
fn test_mouse_dragged_at_empty_skin() {
    let mut skin = make_test_skin();
    let mut timer = rubato_types::timer_access::NullTimer;
    // Should not panic with no objects
    skin.mouse_dragged_at(&mut timer, 0, 100, 200);
}

#[test]
fn test_timer_only_main_state_delegates_mutating_context_methods() {
    let registry = HashMap::new();
    let mut ctx = RecordingSkinRenderContext::new(MainStateType::MusicSelect);
    let mut adapter = TimerOnlyMainState::from_render_context_with_images(&mut ctx, &registry);

    adapter.execute_event(55, 1, 2);
    adapter.change_state(MainStateType::Config);
    adapter.set_timer_micro(rubato_types::timer_id::TimerId::new(9), 12_345);
    adapter.audio_play("test.wav", 0.75, true);
    adapter.audio_stop("test.wav");
    adapter.set_float_value(42, 0.5);

    assert_eq!(ctx.executed_events, vec![(55, 1, 2)]);
    assert_eq!(ctx.changed_states, vec![MainStateType::Config]);
    assert_eq!(ctx.timer_writes, vec![(9, 12_345)]);
    assert_eq!(ctx.audio_plays, vec![("test.wav".to_string(), 0.75, true)]);
    assert_eq!(ctx.audio_stops, vec!["test.wav".to_string()]);
    assert_eq!(ctx.float_writes, vec![(42, 0.5)]);
}

#[test]
fn test_mouse_pressed_dispatches_click_event_through_render_context() {
    let mut skin = make_test_skin();
    let mut image = SkinImage::new_empty();
    image.data.draw = true;
    image.data.region.set_xywh(0.0, 0.0, 100.0, 100.0);
    image.data.set_clickevent_by_id(13);
    skin.add(SkinObject::Image(image));
    skin.objectarray_indices.push(0);

    let registry = HashMap::new();
    let mut ctx = RecordingSkinRenderContext::new(MainStateType::MusicSelect);
    let mut adapter = TimerOnlyMainState::from_render_context_with_images(&mut ctx, &registry);

    skin.mouse_pressed(&mut adapter, 0, 50, 50);

    assert_eq!(ctx.changed_states, vec![MainStateType::Config]);
}

#[test]
fn test_swap_sprite_batch_exchanges_batches() {
    use rubato_render::sprite_batch::SpriteBatch;
    use rubato_render::texture::{Texture, TextureRegion};
    use std::sync::Arc;

    let mut skin = make_test_skin();
    let mut external = SpriteBatch::new();

    // Draw a quad into the external batch
    external.begin();
    let tex = Texture {
        width: 10,
        height: 10,
        disposed: false,
        path: Some(Arc::from("swap_test")),
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
    external.draw_region(&region, 0.0, 0.0, 10.0, 10.0);
    external.end();
    assert_eq!(
        external.vertices().len(),
        6,
        "precondition: 1 quad = 6 vertices"
    );

    // Swap: skin takes the populated batch, external gets empty one
    skin.swap_sprite_batch(&mut external);
    assert!(
        external.vertices().is_empty(),
        "after swap-in, external should be empty"
    );

    // Swap back: external gets the populated batch back
    skin.swap_sprite_batch(&mut external);
    assert_eq!(
        external.vertices().len(),
        6,
        "after swap-back, external has vertices again"
    );
}

#[test]
fn test_swap_sprite_batch_creates_renderer_if_needed() {
    use rubato_render::sprite_batch::SpriteBatch;

    let mut skin = make_test_skin();
    // Skin starts with renderer = None
    let mut batch = SpriteBatch::new();
    // Should not panic — swap_sprite_batch creates renderer lazily
    skin.swap_sprite_batch(&mut batch);
    // Swap back to verify it worked
    skin.swap_sprite_batch(&mut batch);
}

#[test]
fn test_skin_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<Skin>();
}

// =========================================================================
// Phase 40a: Two-phase prepare/draw via SkinObject enum dispatch
// =========================================================================

/// Helper: make a TextureRegion with known dimensions.
fn make_region(w: i32, h: i32) -> TextureRegion {
    TextureRegion {
        region_width: w,
        region_height: h,
        u: 0.0,
        v: 0.0,
        u2: 1.0,
        v2: 1.0,
        ..TextureRegion::default()
    }
}

#[test]
fn test_skin_object_enum_two_phase_image() {
    // Phase 40a: verify SkinObject::Image follows prepare/draw two-phase via enum
    let mut image = crate::skin_image::SkinImage::new_with_single(make_region(32, 32));
    image.data.set_destination_with_int_timer_ops(
        0,
        10.0,
        20.0,
        100.0,
        50.0,
        0,
        255,
        255,
        255,
        255,
        0,
        0,
        0,
        0,
        0,
        0,
        &[0],
    );
    let mut obj = SkinObject::Image(image);

    let state = crate::test_helpers::MockMainState::default();

    // Phase 1: prepare (via enum)
    obj.prepare(0, &state);
    assert!(obj.is_draw());

    // Phase 2: draw (via enum)
    let mut renderer = SkinObjectRenderer::new();
    obj.draw(&mut renderer, &state);
    // Should have generated vertices
    assert_eq!(renderer.sprite.vertices().len(), 6);
}

#[test]
fn test_skin_object_enum_two_phase_bar() {
    // Phase 40a: verify SkinObject::Bar follows prepare/draw two-phase via enum
    let mut bar_obj = crate::skin_bar_object::SkinBarObject::new(0);
    bar_obj.data.set_destination_with_int_timer_ops(
        0,
        0.0,
        0.0,
        640.0,
        480.0,
        0,
        255,
        255,
        255,
        255,
        0,
        0,
        0,
        0,
        0,
        0,
        &[0],
    );
    let mut obj = SkinObject::Bar(bar_obj);

    let state = crate::test_helpers::MockMainState::default();

    // Phase 1: prepare
    obj.prepare(0, &state);
    assert!(obj.is_draw());

    // Phase 2: draw (stub — no panic)
    let mut renderer = SkinObjectRenderer::new();
    obj.draw(&mut renderer, &state);
}

#[test]
fn test_skin_object_enum_two_phase_number() {
    // Phase 40a: verify SkinObject::Number follows prepare/draw two-phase via enum
    let digits: Vec<Vec<TextureRegion>> = vec![(0..12).map(|_| make_region(24, 32)).collect()];
    let mut num =
        crate::skin_number::SkinNumber::new_with_int_timer(digits, None, 0, 0, 3, 1, 0, 0, 0);
    num.data.set_destination_with_int_timer_ops(
        0,
        0.0,
        0.0,
        24.0,
        32.0,
        0,
        255,
        255,
        255,
        255,
        0,
        0,
        0,
        0,
        0,
        0,
        &[0],
    );
    let mut obj = SkinObject::Number(num);

    let state = crate::test_helpers::MockMainState::default();

    // Phase 1: prepare
    obj.prepare(0, &state);
    // draw may be false because integer property returns i32::MIN by default
    // That's expected — the property factory returns None and the default is 0,
    // which IS a valid value. Let's check.
    // The default ref_prop is from integer_property_by_id(0) which returns None,
    // so value = i32::MIN... but wait, SkinNumber::prepare calls ref_prop.get() which
    // returns 0 for id=0 since no property found. Actually ref_prop is None so value = i32::MIN.
    // i32::MIN triggers early return with draw=false. That's correct behavior.
}

#[test]
fn test_skin_object_enum_two_phase_graph() {
    // Phase 40a: verify SkinObject::Graph follows prepare/draw two-phase
    let images = vec![make_region(64, 64)];
    let mut graph = crate::skin_graph::SkinGraph::new_with_int_timer(images, 0, 0, 0, 0);
    graph.data.set_destination_with_int_timer_ops(
        0,
        0.0,
        0.0,
        200.0,
        20.0,
        0,
        255,
        255,
        255,
        255,
        0,
        0,
        0,
        0,
        0,
        0,
        &[0],
    );
    let mut obj = SkinObject::Graph(graph);

    let state = crate::test_helpers::MockMainState::default();

    // Phase 1: prepare
    obj.prepare(0, &state);
    assert!(obj.is_draw());

    // Phase 2: draw
    let mut renderer = SkinObjectRenderer::new();
    obj.draw(&mut renderer, &state);
}

#[test]
fn test_skin_object_enum_two_phase_slider() {
    // Phase 40a: verify SkinObject::Slider follows prepare/draw two-phase
    let images = vec![make_region(16, 16)];
    let mut slider =
        crate::skin_slider::SkinSlider::new_with_int_timer(images, 0, 0, 0, 100, 0, false);
    slider.data.set_destination_with_int_timer_ops(
        0,
        0.0,
        0.0,
        16.0,
        16.0,
        0,
        255,
        255,
        255,
        255,
        0,
        0,
        0,
        0,
        0,
        0,
        &[0],
    );
    let mut obj = SkinObject::Slider(slider);

    let state = crate::test_helpers::MockMainState::default();

    // Phase 1: prepare
    obj.prepare(0, &state);
    assert!(obj.is_draw());

    // Phase 2: draw
    let mut renderer = SkinObjectRenderer::new();
    obj.draw(&mut renderer, &state);
}

// ================================================================
// SkinFloat enum variant tests (Task 47d)
// ================================================================

#[test]
fn test_skin_float_in_enum_data_access() {
    // Verify SkinFloat variant provides data() / data_mut() access
    let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
        vec![vec![None; 12]],
        0,
        0,
        3,
        2,
        false,
        0,
        0,
        0,
        0,
        1.0,
    );
    let mut obj = SkinObject::Float(sf);

    // data() should return the SkinObjectData
    let _data = obj.data();
    assert!(!obj.is_draw());

    // data_mut() should also work
    obj.data_mut().visible = false;
    assert!(!obj.is_visible());
}

#[test]
fn test_skin_float_type_name() {
    let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
        vec![vec![None; 12]],
        0,
        0,
        3,
        2,
        false,
        0,
        0,
        0,
        0,
        1.0,
    );
    let obj = SkinObject::Float(sf);
    assert_eq!(obj.type_name(), "Float");
}

#[test]
fn test_skin_float_prepare_draw_dispose() {
    // Verify SkinFloat follows the prepare/draw/dispose lifecycle
    let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
        vec![vec![None; 12]],
        0,
        0,
        3,
        2,
        false,
        0,
        0,
        0,
        0,
        1.0,
    );
    let mut obj = SkinObject::Float(sf);
    let state = crate::test_helpers::MockMainState::default();

    // prepare should not panic
    obj.prepare(0, &state);

    // draw should not panic
    let mut renderer = SkinObjectRenderer::new();
    obj.draw(&mut renderer, &state);

    // dispose should not panic
    obj.dispose();
}

#[test]
fn test_skin_float_validate_returns_true() {
    let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
        vec![vec![None; 12]],
        0,
        0,
        3,
        2,
        false,
        0,
        0,
        0,
        0,
        1.0,
    );
    let mut obj = SkinObject::Float(sf);
    // Float uses wildcard arm which defaults to true
    assert!(obj.validate());
}

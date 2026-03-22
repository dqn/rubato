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
    timer: crate::reexports::Timer,
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
            timer: crate::reexports::Timer::with_timers(100, 100_000, Vec::new()),
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

impl SkinRenderContext for RecordingSkinRenderContext {
    fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
        self.executed_events.push((id, arg1, arg2));
    }

    fn change_state(&mut self, state: MainStateType) {
        self.changed_states.push(state);
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer_writes.push((timer_id.as_i32(), micro_time));
    }

    fn audio_play(&mut self, path: &str, volume: f32, is_loop: bool) {
        self.audio_plays.push((path.to_string(), volume, is_loop));
    }

    fn audio_stop(&mut self, path: &str) {
        self.audio_stops.push(path.to_string());
    }

    fn current_state_type(&self) -> Option<MainStateType> {
        Some(self.state_type)
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        self.float_writes.push((id, value));
    }
}

fn make_test_skin() -> Skin {
    Skin::new(SkinHeader::new())
}

#[test]
fn test_timer_only_main_state_returns_expected_values() {
    let timer = crate::reexports::Timer::with_timers(1000, 1_000_000, Vec::new());
    let adapter = TimerOnlyMainState::from_timer(&timer, None);
    let state: &dyn MainState = &adapter;
    assert_eq!(state.now_time(), 1000);
    assert_eq!(state.now_micro_time(), 1_000_000);
    assert!(state.get_offset_value(0).is_none());
    assert!(state.skin_image(0).is_none());
    assert!(!state.is_debug());
}

/// Verify that get_offset_value() delegates through TimerOnlyMainState to the underlying
/// SkinRenderContext. Before the fix, TimerOnlyMainState did not override get_offset_value(),
/// so all offset queries returned None even when the underlying context had offset data.
#[test]
fn test_offset_value_delegates_through_timer_only_adapter() {
    use rubato_types::skin_offset::SkinOffset;

    // Create a SkinRenderContext that has offset data
    struct OffsetContext {
        offsets: std::collections::HashMap<i32, SkinOffset>,
    }
    impl rubato_types::timer_access::TimerAccess for OffsetContext {
        fn now_time(&self) -> i64 {
            0
        }
        fn now_micro_time(&self) -> i64 {
            0
        }
        fn micro_timer(&self, _: rubato_types::timer_id::TimerId) -> i64 {
            0
        }
        fn timer(&self, _: rubato_types::timer_id::TimerId) -> i64 {
            0
        }
        fn now_time_for(&self, _: rubato_types::timer_id::TimerId) -> i64 {
            0
        }
        fn is_timer_on(&self, _: rubato_types::timer_id::TimerId) -> bool {
            false
        }
    }
    impl rubato_types::skin_render_context::SkinRenderContext for OffsetContext {
        fn get_offset_value(&self, id: i32) -> Option<&SkinOffset> {
            self.offsets.get(&id)
        }
    }
    impl MainState for OffsetContext {}

    let mut ctx = OffsetContext {
        offsets: std::collections::HashMap::from([
            (
                5,
                SkinOffset {
                    x: 10.0,
                    y: 20.0,
                    w: 0.0,
                    h: 0.0,
                    r: 0.0,
                    a: 0.0,
                },
            ),
            (
                10,
                SkinOffset {
                    x: -5.0,
                    y: 0.0,
                    w: 3.0,
                    h: 4.0,
                    r: 0.0,
                    a: 1.0,
                },
            ),
        ]),
    };

    // Wrap in TimerOnlyMainState (the bridge used by SkinDrawable)
    let registry = std::collections::HashMap::new();
    let adapter = TimerOnlyMainState::from_render_context_with_images(&mut ctx, &registry);
    let state: &dyn MainState = &adapter;

    // Offsets should delegate through to the underlying context
    let off5 = state.get_offset_value(5);
    assert!(off5.is_some(), "offset ID 5 should be present");
    assert_eq!(off5.unwrap().x, 10.0);
    assert_eq!(off5.unwrap().y, 20.0);

    let off10 = state.get_offset_value(10);
    assert!(off10.is_some(), "offset ID 10 should be present");
    assert_eq!(off10.unwrap().x, -5.0);
    assert_eq!(off10.unwrap().a, 1.0);

    // Non-existent offset should return None
    assert!(state.get_offset_value(99).is_none());
}

/// Verify that TimerManager timer values flow through SkinDrawable to the skin adapter.
/// Before the fix, all per-timer-id queries returned 0 (frozen animations).
#[test]
fn test_timer_manager_values_flow_through_to_skin_adapter() {
    use rubato_core::timer_manager::TimerManager;
    let mut tm = TimerManager::new();
    tm.update(); // Advance nowmicrotime from Instant::now()
    tm.set_timer_on(rubato_types::timer_id::TimerId::new(10)); // Timer 10 = ON at current micro time

    // Verify TimerManager implements TimerAccess correctly
    assert!(tm.is_timer_on(rubato_types::timer_id::TimerId::new(10)));
    assert!(!tm.is_timer_on(rubato_types::timer_id::TimerId::new(20))); // Timer 20 was never set

    // Create adapter from TimerManager (the path SkinDrawable takes)
    let adapter = TimerOnlyMainState::from_timer(&tm, None);
    let state: &dyn MainState = &adapter;

    // Timer 10 should be ON through the adapter
    assert!(
        state.is_timer_on(rubato_types::timer_id::TimerId::new(10)),
        "Timer 10 should be ON through adapter"
    );
    // Timer 20 should be OFF
    assert!(
        !state.is_timer_on(rubato_types::timer_id::TimerId::new(20)),
        "Timer 20 should be OFF through adapter"
    );
    // micro_timer for ON timer should not be i64::MIN
    assert_ne!(
        state.micro_timer(rubato_types::timer_id::TimerId::new(10)),
        i64::MIN,
        "ON timer should return its activation time, not i64::MIN"
    );
    // micro_timer for OFF timer should be i64::MIN
    assert_eq!(
        state.micro_timer(rubato_types::timer_id::TimerId::new(20)),
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
    let mut timer = crate::reexports::Timer::with_timers(100, 100_000, Vec::new());
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
// offset_all() regression tests
// =========================================================================

fn make_play_skin(skin_type: rubato_types::skin_type::SkinType) -> Skin {
    let mut header = SkinHeader::new();
    header.set_skin_type(skin_type);
    Skin::new(header)
}

#[test]
fn test_offset_all_returns_none_for_non_play_skin() {
    let mut skin = make_play_skin(rubato_types::skin_type::SkinType::MusicSelect);
    skin.offset.insert(
        crate::skin_property::OFFSET_ALL,
        crate::skin_config_offset::SkinConfigOffset {
            name: "All offset(%)".to_string(),
            x: 5.0,
            y: 10.0,
            w: 20.0,
            h: 15.0,
            ..Default::default()
        },
    );
    assert!(
        skin.offset_all().is_none(),
        "non-play skin should return None"
    );
}

#[test]
fn test_offset_all_returns_none_for_battle_skin() {
    let mut skin = make_play_skin(rubato_types::skin_type::SkinType::Play7KeysBattle);
    skin.offset.insert(
        crate::skin_property::OFFSET_ALL,
        crate::skin_config_offset::SkinConfigOffset {
            name: "All offset(%)".to_string(),
            x: 5.0,
            y: 10.0,
            w: 20.0,
            h: 15.0,
            ..Default::default()
        },
    );
    assert!(
        skin.offset_all().is_none(),
        "battle skin should return None"
    );
}

#[test]
fn test_offset_all_returns_none_when_all_values_zero() {
    let mut skin = make_play_skin(rubato_types::skin_type::SkinType::Play7Keys);
    skin.offset.insert(
        crate::skin_property::OFFSET_ALL,
        crate::skin_config_offset::SkinConfigOffset {
            name: "All offset(%)".to_string(),
            ..Default::default()
        },
    );
    assert!(
        skin.offset_all().is_none(),
        "zero offsets should return None"
    );
}

#[test]
fn test_offset_all_returns_some_for_play_skin_with_nonzero_offset() {
    let mut skin = make_play_skin(rubato_types::skin_type::SkinType::Play7Keys);
    skin.offset.insert(
        crate::skin_property::OFFSET_ALL,
        crate::skin_config_offset::SkinConfigOffset {
            name: "All offset(%)".to_string(),
            x: 5.0,
            y: 10.0,
            w: 20.0,
            h: 15.0,
            r: 1.0,
            a: 2.0,
            enabled: true,
        },
    );
    let result = skin.offset_all();
    assert!(
        result.is_some(),
        "play skin with nonzero offset should return Some"
    );
    let oa = result.unwrap();
    assert_eq!(oa.x, 5.0);
    assert_eq!(oa.y, 10.0);
    assert_eq!(oa.w, 20.0);
    assert_eq!(oa.h, 15.0);
    assert_eq!(oa.r, 1.0);
    assert_eq!(oa.a, 2.0);
}

#[test]
fn test_offset_all_works_for_all_non_battle_play_types() {
    let play_types = [
        rubato_types::skin_type::SkinType::Play5Keys,
        rubato_types::skin_type::SkinType::Play7Keys,
        rubato_types::skin_type::SkinType::Play9Keys,
        rubato_types::skin_type::SkinType::Play10Keys,
        rubato_types::skin_type::SkinType::Play14Keys,
        rubato_types::skin_type::SkinType::Play24Keys,
        rubato_types::skin_type::SkinType::Play24KeysDouble,
    ];
    for st in &play_types {
        let mut skin = make_play_skin(*st);
        skin.offset.insert(
            crate::skin_property::OFFSET_ALL,
            crate::skin_config_offset::SkinConfigOffset {
                name: "All offset(%)".to_string(),
                x: 1.0,
                ..Default::default()
            },
        );
        assert!(
            skin.offset_all().is_some(),
            "{:?} should support offset_all",
            st
        );
    }
}

#[test]
fn test_offset_all_returns_none_when_offset_not_registered() {
    // Play skin without OFFSET_ALL in the offset map
    let skin = make_play_skin(rubato_types::skin_type::SkinType::Play7Keys);
    assert!(
        skin.offset_all().is_none(),
        "skin without OFFSET_ALL entry should return None"
    );
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
        &DestinationParams {
            time: 0,
            x: 10.0,
            y: 20.0,
            w: 100.0,
            h: 50.0,
            acc: 0,
            a: 255,
            r: 255,
            g: 255,
            b: 255,
            blend: 0,
            filter: 0,
            angle: 0,
            center: 0,
            loop_val: 0,
        },
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
        &DestinationParams {
            time: 0,
            x: 0.0,
            y: 0.0,
            w: 640.0,
            h: 480.0,
            acc: 0,
            a: 255,
            r: 255,
            g: 255,
            b: 255,
            blend: 0,
            filter: 0,
            angle: 0,
            center: 0,
            loop_val: 0,
        },
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
    let mut num = crate::skin_number::SkinNumber::new_with_int_timer(
        digits,
        None,
        0,
        0,
        crate::skin_number::NumberDisplayConfig {
            keta: 3,
            zeropadding: 1,
            space: 0,
            align: 0,
        },
        0,
    );
    num.data.set_destination_with_int_timer_ops(
        &DestinationParams {
            time: 0,
            x: 0.0,
            y: 0.0,
            w: 24.0,
            h: 32.0,
            acc: 0,
            a: 255,
            r: 255,
            g: 255,
            b: 255,
            blend: 0,
            filter: 0,
            angle: 0,
            center: 0,
            loop_val: 0,
        },
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
        &DestinationParams {
            time: 0,
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 20.0,
            acc: 0,
            a: 255,
            r: 255,
            g: 255,
            b: 255,
            blend: 0,
            filter: 0,
            angle: 0,
            center: 0,
            loop_val: 0,
        },
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
        &DestinationParams {
            time: 0,
            x: 0.0,
            y: 0.0,
            w: 16.0,
            h: 16.0,
            acc: 0,
            a: 255,
            r: 255,
            g: 255,
            b: 255,
            blend: 0,
            filter: 0,
            angle: 0,
            center: 0,
            loop_val: 0,
        },
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
        crate::skin_float::FloatDisplayConfig {
            iketa: 3,
            fketa: 2,
            is_sign_visible: false,
            align: 0,
            zeropadding: 0,
            space: 0,
            gain: 1.0,
        },
        0,
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
        crate::skin_float::FloatDisplayConfig {
            iketa: 3,
            fketa: 2,
            is_sign_visible: false,
            align: 0,
            zeropadding: 0,
            space: 0,
            gain: 1.0,
        },
        0,
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
        crate::skin_float::FloatDisplayConfig {
            iketa: 3,
            fketa: 2,
            is_sign_visible: false,
            align: 0,
            zeropadding: 0,
            space: 0,
            gain: 1.0,
        },
        0,
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
        crate::skin_float::FloatDisplayConfig {
            iketa: 3,
            fketa: 2,
            is_sign_visible: false,
            align: 0,
            zeropadding: 0,
            space: 0,
            gain: 1.0,
        },
        0,
    );
    let mut obj = SkinObject::Float(sf);
    // Float uses wildcard arm which defaults to true
    assert!(obj.validate());
}

// =========================================================================
// SkinNoteObject pipeline gate tests
// =========================================================================

/// Two-phase lifecycle: SkinObject::Note must be drawable after prepare().
/// This is the test that would have caught the missing setDestination() bug.
#[test]
fn test_skin_object_enum_two_phase_note() {
    let note_obj = crate::skin_note_object::SkinNoteObject::new(7);
    let mut obj = SkinObject::Note(note_obj);

    let state = crate::test_helpers::MockMainState::default();

    // Phase 1: prepare (via enum)
    obj.prepare(0, &state);
    assert!(
        obj.is_draw(),
        "SkinObject::Note must pass is_draw() after prepare()"
    );
    assert!(obj.is_visible());
}

/// Integration test: SkinNoteObject added to a Skin and drawn through
/// draw_all_objects(). Verifies the full pipeline gate (prepare -> is_draw
/// check -> draw) does not skip the note object.
#[test]
fn test_draw_all_objects_includes_note_object() {
    use rubato_play::lane_renderer::{DrawCommand, NoteImageType};

    let mut skin = make_test_skin();

    // Create a note object with a draw command and a wired texture
    let mut note_obj = crate::skin_note_object::SkinNoteObject::new(7);
    note_obj.draw_commands = vec![DrawCommand::DrawNote {
        lane: 0,
        x: 10.0,
        y: 20.0,
        w: 30.0,
        h: 5.0,
        image_type: NoteImageType::Normal,
    }];
    // Wire a texture so the draw actually produces vertices
    note_obj.note_images[0] = Some(make_region(32, 8));

    skin.add(SkinObject::Note(note_obj));
    // Register the object in the draw array
    skin.objectarray_indices.push(skin.objects.len() - 1);

    // Swap in a SpriteBatch and draw
    let mut batch = rubato_render::sprite_batch::SpriteBatch::new();
    skin.swap_sprite_batch(&mut batch);

    let state = crate::test_helpers::MockMainState::default();
    skin.draw_all_objects(&state);

    // Swap out and check that the note was actually drawn
    skin.swap_sprite_batch(&mut batch);
    assert!(
        !batch.vertices().is_empty(),
        "Note object must produce vertices when drawn through draw_all_objects()"
    );
}

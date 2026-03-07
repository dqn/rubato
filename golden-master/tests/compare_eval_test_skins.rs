// Synthetic test skin evaluation tests.
//
// Verifies eval path coverage (interpolation modes, loop variants, draw conditions)
// using programmatically constructed skins. No JSON fixtures needed.
//
// Run: cargo test -p golden-master compare_eval_test_skins -- --nocapture

use golden_master::render_snapshot::{DrawCommand, RenderSnapshot, capture_render_snapshot};
use golden_master::state_provider::StaticStateProvider;

use rubato_skin::skin::{Skin, SkinObject};
use rubato_skin::skin_header::SkinHeader;
use rubato_skin::skin_image::SkinImage;
use rubato_skin::stubs::TextureRegion;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a SkinImage with a single valid TextureRegion source.
fn make_image() -> SkinImage {
    SkinImage::new_with_single(TextureRegion::new())
}

/// Find a command by its name in the snapshot.
fn find_command<'a>(snapshot: &'a RenderSnapshot, name: &str) -> &'a DrawCommand {
    snapshot
        .commands
        .iter()
        .find(|c| c.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("Command with name '{}' not found in snapshot", name))
}

/// Assert x coordinate of a visible command within tolerance.
fn assert_x_approx(cmd: &DrawCommand, expected: f32, tolerance: f32) {
    assert!(
        cmd.visible,
        "Expected '{}' to be visible",
        cmd.name.as_deref().unwrap_or("?")
    );
    let dst = cmd.dst.as_ref().expect("Visible command should have dst");
    assert!(
        (dst.x - expected).abs() <= tolerance,
        "Expected x={:.1} for '{}', got x={:.1} (tolerance={:.1})",
        expected,
        cmd.name.as_deref().unwrap_or("?"),
        dst.x,
        tolerance,
    );
}

// ---------------------------------------------------------------------------
// Skin builders
// ---------------------------------------------------------------------------

/// Build interpolation modes skin: 4 objects with acc=0,1,2,3.
/// Each animates x from 0 to 100 over time 0..100ms, timer=0 (always on).
fn build_interpolation_modes_skin() -> Skin {
    let mut skin = Skin::new(SkinHeader::default());

    let modes = [
        ("linear", 0),
        ("easein", 1),
        ("easeout", 2),
        ("discrete", 3),
    ];

    for (name, acc) in &modes {
        let image = make_image();
        let mut obj = SkinObject::Image(image);
        let data = obj.data_mut();
        data.name = Some(name.to_string());

        // Keyframe 1: time=0, x=0, y=0, w=10, h=10
        data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            10.0,
            10.0,
            *acc,
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
            &[],
        );
        // Keyframe 2: time=100, x=100, y=0, w=10, h=10
        data.set_destination_with_int_timer_ops(
            100,
            100.0,
            0.0,
            10.0,
            10.0,
            *acc,
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
            &[],
        );

        skin.add(obj);
    }

    skin
}

/// Build loop variants skin: 3 objects with loop=0 (wrap), loop=50, loop=-1 (play once).
/// Each animates x from 0 to 100 over time 0..100ms with linear interpolation.
fn build_loop_variants_skin() -> Skin {
    let mut skin = Skin::new(SkinHeader::default());

    // loop_default: loop_time=0 (default wrap)
    {
        let image = make_image();
        let mut obj = SkinObject::Image(image);
        let data = obj.data_mut();
        data.name = Some("loop_default".to_string());
        data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            10.0,
            10.0,
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
            &[],
        );
        data.set_destination_with_int_timer_ops(
            100,
            100.0,
            0.0,
            10.0,
            10.0,
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
            &[],
        );
        // dstloop default is 0 (wrap)
        skin.add(obj);
    }

    // loop_mid: loop_time=50 (wrap from midpoint)
    {
        let image = make_image();
        let mut obj = SkinObject::Image(image);
        let data = obj.data_mut();
        data.name = Some("loop_mid".to_string());
        data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            10.0,
            10.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            50,
            0,
            &[],
        );
        data.set_destination_with_int_timer_ops(
            100,
            100.0,
            0.0,
            10.0,
            10.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            50,
            0,
            &[],
        );
        skin.add(obj);
    }

    // play_once: loop_time=-1 (play once, hide after end)
    {
        let image = make_image();
        let mut obj = SkinObject::Image(image);
        let data = obj.data_mut();
        data.name = Some("play_once".to_string());
        data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            10.0,
            10.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            -1,
            0,
            &[],
        );
        data.set_destination_with_int_timer_ops(
            100,
            100.0,
            0.0,
            10.0,
            10.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            -1,
            0,
            &[],
        );
        skin.add(obj);
    }

    skin
}

/// Build draw conditions matrix skin: 4 objects with draw conditions.
///   pos_true:  op=[1]   → visible when boolean(1) == true
///   pos_false: op=[2]   → visible when boolean(2) == true
///   neg_false: op=[-3]  → visible when boolean(3) == false (negated)
///   neg_true:  op=[-4]  → visible when boolean(4) == false (negated)
fn build_draw_conditions_skin() -> Skin {
    let mut skin = Skin::new(SkinHeader::default());

    let conditions: &[(&str, &[i32])] = &[
        ("pos_true", &[1]),
        ("pos_false", &[2]),
        ("neg_false", &[-3]),
        ("neg_true", &[-4]),
    ];

    for (name, op) in conditions {
        let image = make_image();
        let mut obj = SkinObject::Image(image);
        let data = obj.data_mut();
        data.name = Some(name.to_string());
        // Single keyframe: always at x=10, y=0, w=10, h=10
        // Draw condition set via op on the first call
        data.set_destination_with_int_timer_ops(
            0, 10.0, 0.0, 10.0, 10.0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, op,
        );
        skin.add(obj);
    }

    skin
}

// ===========================================================================
// Test: Interpolation modes (acc=0,1,2,3)
// ===========================================================================

#[test]
fn interpolation_modes_at_midpoint() {
    let skin = build_interpolation_modes_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 50; // midpoint of 0..100

    let snapshot = capture_render_snapshot(&skin, &provider);

    // acc=0 (linear): rate = 0.5, x = 0 + (100-0)*0.5 = 50
    let linear = find_command(&snapshot, "linear");
    assert_x_approx(linear, 50.0, 1.0);

    // acc=1 (ease-in): rate = 0.5^2 = 0.25, x = 0 + 100*0.25 = 25
    let easein = find_command(&snapshot, "easein");
    assert_x_approx(easein, 25.0, 1.0);

    // acc=2 (ease-out): rate = 1 - (0.5-1)^2 = 1 - 0.25 = 0.75, x = 100*0.75 = 75
    let easeout = find_command(&snapshot, "easeout");
    assert_x_approx(easeout, 75.0, 1.0);

    // acc=3 (discrete): stays at first keyframe, x = 0
    let discrete = find_command(&snapshot, "discrete");
    assert_x_approx(discrete, 0.0, 1.0);
}

#[test]
fn interpolation_modes_at_start() {
    let skin = build_interpolation_modes_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 0;

    let snapshot = capture_render_snapshot(&skin, &provider);

    // All modes should be at x=0 at t=0
    for name in &["linear", "easein", "easeout", "discrete"] {
        let cmd = find_command(&snapshot, name);
        assert_x_approx(cmd, 0.0, 1.0);
    }
}

#[test]
fn interpolation_modes_at_end_wraps_due_to_loop() {
    // Default loop_time=0 means animation loops. At t=100 (exactly one full
    // cycle with end=100), time wraps to 0 → all objects return to start.
    let skin = build_interpolation_modes_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 100;

    let snapshot = capture_render_snapshot(&skin, &provider);

    // With loop_time=0, (100-0)%(100-0)+0 = 0, so all wrap to start
    for name in &["linear", "easein", "easeout", "discrete"] {
        let cmd = find_command(&snapshot, name);
        assert_x_approx(cmd, 0.0, 1.0);
    }
}

#[test]
fn interpolation_modes_near_end() {
    // At t=99 (just before loop wrap), all modes should be near x=100.
    let skin = build_interpolation_modes_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 99;

    let snapshot = capture_render_snapshot(&skin, &provider);

    // acc=0 (linear): rate=0.99, x=99
    let linear = find_command(&snapshot, "linear");
    assert_x_approx(linear, 99.0, 1.0);

    // acc=1 (ease-in): rate=0.99^2=0.9801, x=98.01
    let easein = find_command(&snapshot, "easein");
    assert_x_approx(easein, 98.01, 1.0);

    // acc=2 (ease-out): rate=1-(0.99-1)^2=1-0.0001=0.9999, x=99.99
    let easeout = find_command(&snapshot, "easeout");
    assert_x_approx(easeout, 99.99, 1.0);

    // acc=3 (discrete): stays at first keyframe, x=0
    let discrete = find_command(&snapshot, "discrete");
    assert_x_approx(discrete, 0.0, 1.0);
}

// ===========================================================================
// Test: Loop variants (loop=0, loop=50, loop=-1)
// ===========================================================================

#[test]
fn loop_default_wraps_around() {
    // loop_time=0, end=100: at t=150 → (150-0) % (100-0) + 0 = 50
    let skin = build_loop_variants_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 150;

    let snapshot = capture_render_snapshot(&skin, &provider);

    let cmd = find_command(&snapshot, "loop_default");
    assert_x_approx(cmd, 50.0, 1.0);
}

#[test]
fn loop_mid_wraps_from_midpoint() {
    // loop_time=50, end=100: at t=150 → (150-50) % (100-50) + 50 = 100%50+50 = 50
    let skin = build_loop_variants_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 150;

    let snapshot = capture_render_snapshot(&skin, &provider);

    let cmd = find_command(&snapshot, "loop_mid");
    assert_x_approx(cmd, 50.0, 1.0);
}

#[test]
fn loop_mid_at_175() {
    // loop_time=50, end=100: at t=175 → (175-50) % (100-50) + 50 = 125%50+50 = 25+50 = 75
    let skin = build_loop_variants_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 175;

    let snapshot = capture_render_snapshot(&skin, &provider);

    let cmd = find_command(&snapshot, "loop_mid");
    assert_x_approx(cmd, 75.0, 1.0);
}

#[test]
fn play_once_hidden_after_end() {
    // loop_time=-1: at t=150 (past end=100) → returns None → hidden
    let skin = build_loop_variants_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 150;

    let snapshot = capture_render_snapshot(&skin, &provider);

    let cmd = find_command(&snapshot, "play_once");
    assert!(!cmd.visible, "play_once should be hidden after end time");
}

#[test]
fn play_once_visible_before_end() {
    // loop_time=-1: at t=50 (before end=100) → visible, x=50
    let skin = build_loop_variants_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 50;

    let snapshot = capture_render_snapshot(&skin, &provider);

    let cmd = find_command(&snapshot, "play_once");
    assert_x_approx(cmd, 50.0, 1.0);
}

// ===========================================================================
// Test: Draw conditions matrix (BooleanId positive/negated x true/false)
// ===========================================================================

#[test]
fn draw_conditions_visibility() {
    let skin = build_draw_conditions_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 0;

    // Set boolean values:
    //   id 1 = true  → draw=1  (positive, true)  → visible
    //   id 2 = false → draw=2  (positive, false)  → hidden
    //   id 3 = false → draw=-3 (negated, !false=true) → visible
    //   id 4 = true  → draw=-4 (negated, !true=false) → hidden
    provider.booleans.insert(1, true);
    provider.booleans.insert(2, false);
    provider.booleans.insert(3, false);
    provider.booleans.insert(4, true);

    let snapshot = capture_render_snapshot(&skin, &provider);

    let pos_true = find_command(&snapshot, "pos_true");
    assert!(
        pos_true.visible,
        "draw=1 with bool(1)=true should be visible"
    );

    let pos_false = find_command(&snapshot, "pos_false");
    assert!(
        !pos_false.visible,
        "draw=2 with bool(2)=false should be hidden"
    );

    let neg_false = find_command(&snapshot, "neg_false");
    assert!(
        neg_false.visible,
        "draw=-3 with bool(3)=false should be visible (negated)"
    );

    let neg_true = find_command(&snapshot, "neg_true");
    assert!(
        !neg_true.visible,
        "draw=-4 with bool(4)=true should be hidden (negated)"
    );
}

#[test]
fn draw_conditions_all_true() {
    let skin = build_draw_conditions_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 0;

    // All booleans true: positive conditions pass, negated conditions fail
    provider.booleans.insert(1, true);
    provider.booleans.insert(2, true);
    provider.booleans.insert(3, true);
    provider.booleans.insert(4, true);

    let snapshot = capture_render_snapshot(&skin, &provider);

    assert!(
        find_command(&snapshot, "pos_true").visible,
        "draw=1 should be visible"
    );
    assert!(
        find_command(&snapshot, "pos_false").visible,
        "draw=2 should be visible"
    );
    assert!(
        !find_command(&snapshot, "neg_false").visible,
        "draw=-3 should be hidden (negated true)"
    );
    assert!(
        !find_command(&snapshot, "neg_true").visible,
        "draw=-4 should be hidden (negated true)"
    );
}

#[test]
fn draw_conditions_all_false() {
    let skin = build_draw_conditions_skin();
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 0;

    // All booleans false: positive conditions fail, negated conditions pass
    provider.booleans.insert(1, false);
    provider.booleans.insert(2, false);
    provider.booleans.insert(3, false);
    provider.booleans.insert(4, false);

    let snapshot = capture_render_snapshot(&skin, &provider);

    assert!(
        !find_command(&snapshot, "pos_true").visible,
        "draw=1 should be hidden"
    );
    assert!(
        !find_command(&snapshot, "pos_false").visible,
        "draw=2 should be hidden"
    );
    assert!(
        find_command(&snapshot, "neg_false").visible,
        "draw=-3 should be visible (negated false)"
    );
    assert!(
        find_command(&snapshot, "neg_true").visible,
        "draw=-4 should be visible (negated false)"
    );
}

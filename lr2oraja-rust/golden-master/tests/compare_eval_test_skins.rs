// Synthetic test skin evaluation tests.
//
// Verifies eval path coverage (interpolation modes, loop variants, draw conditions)
// using purpose-built JSON skins. No Java fixtures needed.
//
// Run: cargo test -p golden-master compare_eval_test_skins -- --nocapture

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use bms_config::resolution::Resolution;
use bms_render::state_provider::StaticStateProvider;
use bms_skin::loader::json_loader;
use golden_master::render_snapshot::{DrawCommand, RenderSnapshot, capture_render_snapshot};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn test_skin_dir() -> PathBuf {
    project_root().join("test-bms/test-skin")
}

/// Load a JSON skin from test-bms/test-skin/.
fn load_test_skin(filename: &str) -> bms_skin::skin::Skin {
    let path = test_skin_dir().join(filename);
    assert!(path.exists(), "Test skin not found: {}", path.display());
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    let enabled: HashSet<i32> = HashSet::new();
    json_loader::load_skin(&content, &enabled, Resolution::Fullhd, Some(&path))
        .unwrap_or_else(|e| panic!("Failed to load skin {}: {}", path.display(), e))
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

// ===========================================================================
// Test: Interpolation modes (acc=0,1,2,3)
// ===========================================================================

#[test]
fn interpolation_modes_at_midpoint() {
    let skin = load_test_skin("eval_interpolation_modes.json");
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
    let skin = load_test_skin("eval_interpolation_modes.json");
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
    let skin = load_test_skin("eval_interpolation_modes.json");
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
    let skin = load_test_skin("eval_interpolation_modes.json");
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
    let skin = load_test_skin("eval_loop_variants.json");
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 150;

    let snapshot = capture_render_snapshot(&skin, &provider);

    let cmd = find_command(&snapshot, "loop_default");
    assert_x_approx(cmd, 50.0, 1.0);
}

#[test]
fn loop_mid_wraps_from_midpoint() {
    // loop_time=50, end=100: at t=150 → (150-50) % (100-50) + 50 = 100%50+50 = 50
    let skin = load_test_skin("eval_loop_variants.json");
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 150;

    let snapshot = capture_render_snapshot(&skin, &provider);

    let cmd = find_command(&snapshot, "loop_mid");
    assert_x_approx(cmd, 50.0, 1.0);
}

#[test]
fn loop_mid_at_175() {
    // loop_time=50, end=100: at t=175 → (175-50) % (100-50) + 50 = 125%50+50 = 25+50 = 75
    let skin = load_test_skin("eval_loop_variants.json");
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 175;

    let snapshot = capture_render_snapshot(&skin, &provider);

    let cmd = find_command(&snapshot, "loop_mid");
    assert_x_approx(cmd, 75.0, 1.0);
}

#[test]
fn play_once_hidden_after_end() {
    // loop_time=-1: at t=150 (past end=100) → returns None → hidden
    let skin = load_test_skin("eval_loop_variants.json");
    let mut provider = StaticStateProvider::default();
    provider.time_ms = 150;

    let snapshot = capture_render_snapshot(&skin, &provider);

    let cmd = find_command(&snapshot, "play_once");
    assert!(!cmd.visible, "play_once should be hidden after end time");
}

#[test]
fn play_once_visible_before_end() {
    // loop_time=-1: at t=50 (before end=100) → visible, x=50
    let skin = load_test_skin("eval_loop_variants.json");
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
    let skin = load_test_skin("eval_draw_conditions_matrix.json");
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
    let skin = load_test_skin("eval_draw_conditions_matrix.json");
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
    let skin = load_test_skin("eval_draw_conditions_matrix.json");
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

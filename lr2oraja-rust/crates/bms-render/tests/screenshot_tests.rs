// Screenshot regression tests for bms-render.
//
// Uses harness = false (custom main) because Bevy headless rendering
// may need special initialization. Tests are skipped by default; pass
// --ignored to run them (matches cargo test convention for GPU tests).
//
// Run: cargo test -p bms-render --test screenshot_tests -- --ignored --nocapture
// Update fixtures: UPDATE_SCREENSHOTS=1 cargo test -p bms-render --test screenshot_tests -- --ignored --nocapture

mod screenshot_compare;
mod screenshot_harness;
mod test_skin_builder;

use std::path::PathBuf;

use screenshot_compare::compare_or_update;
use screenshot_harness::RenderTestHarness;
use test_skin_builder::TestSkinBuilder;

/// Test resolution: small for fast rendering.
const TEST_W: u32 = 256;
const TEST_H: u32 = 192;

/// SSIM threshold for screenshot comparison.
const SSIM_THRESHOLD: f64 = 0.99;

/// Path to fixture directory.
fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("golden-master")
        .join("fixtures")
        .join("screenshots")
}

fn fixture_path(name: &str) -> PathBuf {
    fixture_dir().join(format!("{}.png", name))
}

/// Helper: build harness, upload images, setup skin, capture frame.
fn run_test(builder: TestSkinBuilder, fixture_name: &str) {
    let (skin, images, provider) = builder.build();
    let mut harness = RenderTestHarness::new(TEST_W, TEST_H);

    for img in &images {
        harness.upload_image(&img.rgba);
    }

    harness.setup_skin(skin, Box::new(provider));

    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = tmp_dir.path().join("screenshot.png");

    harness.capture_frame(&output_path);

    let actual = image::open(&output_path)
        .expect("Failed to read captured screenshot")
        .to_rgba8();

    compare_or_update(&actual, &fixture_path(fixture_name), SSIM_THRESHOLD);
}

// ---------------------------------------------------------------------------
// Test cases
// ---------------------------------------------------------------------------

fn test_render_blank_skin() {
    let builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    run_test(builder, "blank");
}

fn test_render_single_image() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Red 60x40 rectangle at (100, 50)
    builder.add_image(100.0, 50.0, 60.0, 40.0, 255, 0, 0);
    run_test(builder, "single_image");
}

fn test_render_image_alpha() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Semi-transparent blue rectangle
    builder.add_image_with_alpha(50.0, 50.0, 80.0, 60.0, 0, 0, 255, 0.5);
    run_test(builder, "image_alpha");
}

fn test_render_z_order() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Three overlapping rectangles: red (bottom), green (middle), blue (top)
    // Later objects have higher z-order (index * 0.001)
    builder.add_image(60.0, 60.0, 80.0, 80.0, 255, 0, 0);
    builder.add_image(80.0, 70.0, 80.0, 80.0, 0, 255, 0);
    builder.add_image(100.0, 80.0, 80.0, 80.0, 0, 0, 255);
    run_test(builder, "z_order");
}

fn test_render_animation_midpoint() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Green square moving from (0,76) to (200,76) over 1000ms
    // At t=500ms, should be at (100, 76)
    builder.add_animated_image(0.0, 76.0, 200.0, 76.0, 40.0, 40.0, 0, 255, 0, 1000);
    builder.set_time_ms(500);
    run_test(builder, "animation_midpoint");
}

fn test_render_draw_condition_false() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // This image has a draw condition set to false => should not be visible
    builder.add_image_with_condition(50.0, 50.0, 80.0, 60.0, 255, 0, 0, 100, false);
    run_test(builder, "draw_condition_false");
}

fn test_render_timer_inactive() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Image with timer that is inactive (no timer value set) => hidden
    builder.add_image_with_timer(50.0, 50.0, 80.0, 60.0, 255, 255, 0, 200, None);
    run_test(builder, "timer_inactive");
}

fn test_render_four_corners() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    let s = 30.0; // square size
    // Top-left: red
    builder.add_image(0.0, 0.0, s, s, 255, 0, 0);
    // Top-right: green
    builder.add_image(TEST_W as f32 - s, 0.0, s, s, 0, 255, 0);
    // Bottom-left: blue
    builder.add_image(0.0, TEST_H as f32 - s, s, s, 0, 0, 255);
    // Bottom-right: yellow
    builder.add_image(TEST_W as f32 - s, TEST_H as f32 - s, s, s, 255, 255, 0);
    run_test(builder, "four_corners");
}

fn test_render_slider() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Horizontal slider: direction=Right(1), range=100, value=0.5
    // Base at (50, 80), 20x20 thumb
    builder.add_slider(50.0, 80.0, 20.0, 20.0, 255, 128, 0, 1, 100, 50, 0.5);
    run_test(builder, "slider");
}

fn test_render_graph() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Graph: direction=Right(0), value=0.5 => half-width bar
    builder.add_graph(20.0, 80.0, 200.0, 30.0, 0, 200, 100, 0, 60, 0.5);
    run_test(builder, "graph");
}

fn test_render_blend_additive() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Two overlapping colored rectangles with additive blend (blend=2).
    // Red rect at (60, 60) overlaps with blue rect at (90, 60).
    builder.add_image(60.0, 60.0, 80.0, 60.0, 255, 0, 0);
    builder.add_image_with_blend(90.0, 60.0, 80.0, 60.0, 0, 0, 255, 2);
    run_test(builder, "blend_additive");
}

fn test_render_rotation() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Two rotated rectangles: 45 degrees and 90 degrees
    builder.add_image_with_rotation(50.0, 50.0, 60.0, 30.0, 0, 200, 0, 45);
    builder.add_image_with_rotation(150.0, 50.0, 60.0, 30.0, 200, 0, 0, 90);
    run_test(builder, "rotation");
}

fn test_render_fade_midpoint() {
    let mut builder = TestSkinBuilder::new(TEST_W as f32, TEST_H as f32);
    // Image fading from alpha=0 to alpha=1 over 1000ms.
    // At t=500ms, alpha should be ~0.5.
    builder.add_fade_image(60.0, 50.0, 100.0, 80.0, 255, 128, 0, 0.0, 1.0, 1000);
    builder.set_time_ms(500);
    run_test(builder, "fade_midpoint");
}

// ---------------------------------------------------------------------------
// JSON skin file tests
// ---------------------------------------------------------------------------

/// Path to test-skin directory.
fn test_skin_dir() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = lr2oraja-rust/crates/bms-render
    // test-bms is at the project root (3 levels up)
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-bms")
        .join("test-skin")
}

/// Helper: load a JSON skin file, set up state, capture frame.
fn run_json_skin_test(
    skin_json_name: &str,
    provider: bms_render::state_provider::StaticStateProvider,
    fixture_name: &str,
) {
    let skin_path = test_skin_dir().join(skin_json_name);
    let mut harness = RenderTestHarness::new(TEST_W, TEST_H);

    harness.load_json_skin(&skin_path, Box::new(provider));

    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = tmp_dir.path().join("screenshot.png");

    harness.capture_frame(&output_path);

    let actual = image::open(&output_path)
        .expect("Failed to read captured screenshot")
        .to_rgba8();

    screenshot_compare::compare_or_update(&actual, &fixture_path(fixture_name), SSIM_THRESHOLD);
}

fn test_render_json_skin() {
    let mut provider = bms_render::state_provider::StaticStateProvider::default();
    // slider FloatId(17) = 0.5, graph FloatId(100) = 0.6
    provider.floats.insert(17, 0.5);
    provider.floats.insert(100, 0.6);
    run_json_skin_test("skin.json", provider, "json_skin");
}

fn test_render_json_skin_with_condition() {
    let mut provider = bms_render::state_provider::StaticStateProvider::default();
    // BooleanId(900) = false → accent image should be hidden
    provider.booleans.insert(900, false);
    run_json_skin_test(
        "skin_with_condition.json",
        provider,
        "json_skin_with_condition",
    );
}

// ---------------------------------------------------------------------------
// ECFN skin tests (real-world skins, skipped if not present)
// ---------------------------------------------------------------------------

/// Path to ECFN skin directory.
fn ecfn_skin_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("skins")
        .join("ECFN")
}

fn test_render_ecfn_select() {
    run_ecfn_lua_test(
        "select/select.luaskin",
        bms_render::state_provider::StaticStateProvider::default(),
        "ecfn_select",
        1920,
        1080,
    );
}

// ---------------------------------------------------------------------------
// State provider factories for various game states
// ---------------------------------------------------------------------------

/// Play screen: active gameplay.
fn state_play_active() -> bms_render::state_provider::StaticStateProvider {
    let mut p = bms_render::state_provider::StaticStateProvider::default();
    p.time_ms = 30000;
    // Timers
    p.timers.insert(41, 30000); // TIMER_PLAY
    p.timers.insert(46, 500); // TIMER_JUDGE_1P
    p.timers.insert(446, 500); // TIMER_COMBO_1P
    // Gauge
    p.floats.insert(1107, 0.80); // FLOAT_GROOVEGAUGE_1P
    p.integers.insert(107, 80); // NUMBER_GROOVEGAUGE
    // Combo & Score
    p.integers.insert(104, 150); // NUMBER_COMBO
    p.integers.insert(75, 150); // NUMBER_MAXCOMBO
    p.integers.insert(71, 85000); // NUMBER_SCORE
    p.integers.insert(72, 200000); // NUMBER_MAXSCORE
    // Judge counts
    p.integers.insert(110, 120); // NUMBER_PERFECT
    p.integers.insert(111, 25); // NUMBER_GREAT
    p.integers.insert(112, 5); // NUMBER_GOOD
    // BPM
    p.integers.insert(160, 170); // NUMBER_NOWBPM
    p.integers.insert(90, 170); // NUMBER_MAXBPM
    p.integers.insert(91, 170); // NUMBER_MINBPM
    p.integers.insert(92, 170); // NUMBER_MAINBPM
    // Booleans
    p.booleans.insert(173, true); // OPTION_LN
    p.booleans.insert(160, true); // OPTION_7KEYSONG
    p
}

/// Play screen: full combo, max gauge.
fn state_play_fullcombo() -> bms_render::state_provider::StaticStateProvider {
    let mut p = state_play_active();
    p.floats.insert(1107, 1.0);
    p.integers.insert(107, 100);
    p.integers.insert(104, 500); // combo
    p.integers.insert(75, 500); // maxcombo
    p.timers.insert(48, 1000); // TIMER_FULLCOMBO_1P
    p.timers.insert(44, 5000); // TIMER_GAUGE_MAX_1P
    p
}

/// Play screen: danger zone (low gauge).
fn state_play_danger() -> bms_render::state_provider::StaticStateProvider {
    let mut p = state_play_active();
    p.floats.insert(1107, 0.15);
    p.integers.insert(107, 15);
    p.integers.insert(104, 0); // combo broken
    p.integers.insert(113, 20); // NUMBER_BAD
    p.integers.insert(114, 15); // NUMBER_POOR
    p
}

/// Result screen: clear.
fn state_result_clear() -> bms_render::state_provider::StaticStateProvider {
    let mut p = bms_render::state_provider::StaticStateProvider::default();
    p.time_ms = 5000;
    p.timers.insert(1, 5000); // TIMER_STARTINPUT
    p.floats.insert(1107, 0.82);
    p.integers.insert(107, 82);
    p.integers.insert(71, 180000); // score
    p.integers.insert(75, 350); // maxcombo
    p.integers.insert(110, 300); // PERFECT
    p.integers.insert(111, 40); // GREAT
    p.integers.insert(112, 8); // GOOD
    p.integers.insert(113, 2); // BAD
    p.integers.insert(114, 0); // POOR
    p.booleans.insert(90, true); // OPTION_RESULT_CLEAR
    p
}

/// Result screen: fail.
fn state_result_fail() -> bms_render::state_provider::StaticStateProvider {
    let mut p = bms_render::state_provider::StaticStateProvider::default();
    p.time_ms = 5000;
    p.timers.insert(1, 5000);
    p.floats.insert(1107, 0.0);
    p.integers.insert(107, 0);
    p.integers.insert(71, 30000);
    p.integers.insert(75, 45);
    p.integers.insert(110, 50);
    p.integers.insert(111, 20);
    p.integers.insert(112, 15);
    p.integers.insert(113, 30);
    p.integers.insert(114, 40);
    p.booleans.insert(91, true); // OPTION_RESULT_FAIL
    p
}

// ---------------------------------------------------------------------------
// ECFN Lua skin tests
// ---------------------------------------------------------------------------

/// Helper: load a Lua skin from the ECFN directory, capture, and compare.
fn run_ecfn_lua_test(
    relative_path: &str,
    state: bms_render::state_provider::StaticStateProvider,
    fixture_name: &str,
    width: u32,
    height: u32,
) {
    let skin_path = ecfn_skin_dir().join(relative_path);
    if !skin_path.exists() {
        eprintln!("ECFN skin {} not found, skipping", relative_path);
        return;
    }

    let mut harness = RenderTestHarness::new(width, height);
    let resolution = if width >= 1920 {
        bms_config::resolution::Resolution::Fullhd
    } else if width >= 1280 {
        bms_config::resolution::Resolution::Hd
    } else {
        bms_config::resolution::Resolution::Sd
    };
    harness.load_lua_skin_with_resolution(&skin_path, Box::new(state), resolution);

    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = tmp_dir.path().join("screenshot.png");

    harness.capture_frame(&output_path);

    let actual = image::open(&output_path)
        .expect("Failed to read captured screenshot")
        .to_rgba8();

    screenshot_compare::compare_or_update(&actual, &fixture_path(fixture_name), SSIM_THRESHOLD);
}

fn test_render_ecfn_decide() {
    run_ecfn_lua_test(
        "decide/decidemain.lua",
        bms_render::state_provider::StaticStateProvider::default(),
        "ecfn_decide",
        1920,
        1080,
    );
}

fn test_render_ecfn_play7_active() {
    run_ecfn_lua_test(
        "play/play7main.lua",
        state_play_active(),
        "ecfn_play7_active",
        1920,
        1080,
    );
}

fn test_render_ecfn_play7_fullcombo() {
    run_ecfn_lua_test(
        "play/play7main.lua",
        state_play_fullcombo(),
        "ecfn_play7_fullcombo",
        1920,
        1080,
    );
}

fn test_render_ecfn_play7_danger() {
    run_ecfn_lua_test(
        "play/play7main.lua",
        state_play_danger(),
        "ecfn_play7_danger",
        1920,
        1080,
    );
}

fn test_render_ecfn_result_clear() {
    run_ecfn_lua_test(
        "RESULT/result.lua",
        state_result_clear(),
        "ecfn_result_clear",
        1920,
        1080,
    );
}

fn test_render_ecfn_result_fail() {
    run_ecfn_lua_test(
        "RESULT/result.lua",
        state_result_fail(),
        "ecfn_result_fail",
        1920,
        1080,
    );
}

fn test_render_ecfn_play14_active() {
    run_ecfn_lua_test(
        "play/play14main.lua",
        state_play_active(),
        "ecfn_play14_active",
        1920,
        1080,
    );
}

fn test_render_ecfn_play7wide_active() {
    run_ecfn_lua_test(
        "play/play7wide.lua",
        state_play_active(),
        "ecfn_play7wide_active",
        1920,
        1080,
    );
}

fn test_render_ecfn_course_result() {
    run_ecfn_lua_test(
        "RESULT/course_result.lua",
        state_result_clear(),
        "ecfn_course_result",
        1920,
        1080,
    );
}

fn test_render_ecfn_result2_clear() {
    run_ecfn_lua_test(
        "RESULT/result2.luaskin",
        state_result_clear(),
        "ecfn_result2_clear",
        1920,
        1080,
    );
}

// ---------------------------------------------------------------------------
// Custom test runner
// ---------------------------------------------------------------------------

fn get_tests() -> Vec<(&'static str, fn())> {
    vec![
        ("test_render_blank_skin", test_render_blank_skin as fn()),
        ("test_render_single_image", test_render_single_image),
        ("test_render_image_alpha", test_render_image_alpha),
        ("test_render_z_order", test_render_z_order),
        (
            "test_render_animation_midpoint",
            test_render_animation_midpoint,
        ),
        (
            "test_render_draw_condition_false",
            test_render_draw_condition_false,
        ),
        ("test_render_timer_inactive", test_render_timer_inactive),
        ("test_render_four_corners", test_render_four_corners),
        ("test_render_slider", test_render_slider),
        ("test_render_graph", test_render_graph),
        ("test_render_json_skin", test_render_json_skin),
        (
            "test_render_json_skin_with_condition",
            test_render_json_skin_with_condition,
        ),
        ("test_render_ecfn_select", test_render_ecfn_select),
        ("test_render_ecfn_decide", test_render_ecfn_decide),
        (
            "test_render_ecfn_play7_active",
            test_render_ecfn_play7_active,
        ),
        (
            "test_render_ecfn_play7_fullcombo",
            test_render_ecfn_play7_fullcombo,
        ),
        (
            "test_render_ecfn_play7_danger",
            test_render_ecfn_play7_danger,
        ),
        (
            "test_render_ecfn_result_clear",
            test_render_ecfn_result_clear,
        ),
        ("test_render_ecfn_result_fail", test_render_ecfn_result_fail),
        ("test_render_blend_additive", test_render_blend_additive),
        ("test_render_rotation", test_render_rotation),
        ("test_render_fade_midpoint", test_render_fade_midpoint),
        (
            "test_render_ecfn_play14_active",
            test_render_ecfn_play14_active,
        ),
        (
            "test_render_ecfn_play7wide_active",
            test_render_ecfn_play7wide_active,
        ),
        (
            "test_render_ecfn_course_result",
            test_render_ecfn_course_result,
        ),
        (
            "test_render_ecfn_result2_clear",
            test_render_ecfn_result2_clear,
        ),
    ]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Support --list for test discovery
    if args.iter().any(|a| a == "--list") {
        for (name, _) in get_tests() {
            println!("{}: test", name);
        }
        return;
    }

    // Match cargo test convention: skip unless --ignored is passed
    if !args.iter().any(|a| a == "--ignored") {
        eprintln!("Screenshot tests skipped (GPU required). Run with --ignored to execute.");
        return;
    }

    // Optional name filter: first non-flag arg after binary name
    let filter: Option<&str> = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(|s| s.as_str());

    let tests = get_tests();
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for (name, test_fn) in &tests {
        if let Some(f) = filter {
            if !name.contains(f) {
                skipped += 1;
                continue;
            }
        }

        eprint!("test {} ... ", name);
        match std::panic::catch_unwind(test_fn) {
            Ok(_) => {
                eprintln!("ok");
                passed += 1;
            }
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                eprintln!("FAILED\n  {}", msg);
                failed += 1;
            }
        }
    }

    eprintln!(
        "\ntest result: {}. {} passed; {} failed; {} filtered out",
        if failed == 0 { "ok" } else { "FAILED" },
        passed,
        failed,
        skipped
    );

    if failed > 0 {
        std::process::exit(1);
    }
}

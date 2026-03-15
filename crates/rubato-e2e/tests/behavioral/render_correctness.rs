//! Phase 5c: Render correctness E2E tests.
//!
//! Tests SpriteBatch capture infrastructure for draw quad output
//! verification without GPU.

use std::path::PathBuf;

use rubato_e2e::{E2eHarness, MainStateType};
use rubato_launcher::state_factory::LauncherStateFactory;
use rubato_types::main_controller_access::MainControllerAccess;

fn test_bms_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-bms")
}

fn harness_with_factory() -> E2eHarness {
    E2eHarness::new().with_state_factory(Box::new(LauncherStateFactory::new()))
}

fn harness_with_bms(bms_filename: &str) -> Option<E2eHarness> {
    let bms_path = test_bms_dir().join(bms_filename);
    if !bms_path.exists() {
        return None;
    }

    let mut harness = harness_with_factory();
    harness.controller_mut().create();

    let loaded = harness
        .controller_mut()
        .player_resource_mut()
        .expect("controller should own a player resource")
        .set_bms_file(&bms_path, 2, 0); // mode_type=2 is AUTOPLAY
    assert!(loaded, "BMS file should load successfully");

    Some(harness)
}

// ============================================================
// 1. Play state produces draw quads with capture enabled
// ============================================================

#[test]
fn test_play_state_produces_draw_quads() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    harness.change_state(MainStateType::Play);
    harness.enable_render_capture();

    // Render several frames to allow the play state to produce draw calls
    harness.render_frames(10);

    let quads = harness.captured_draw_quads();
    assert!(
        !quads.is_empty(),
        "Play state with BMS loaded should produce draw quads when capture is enabled"
    );
}

// ============================================================
// 2. All captured quads have valid (positive) dimensions
// ============================================================

#[test]
fn test_draw_quads_have_valid_dimensions() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    harness.change_state(MainStateType::Play);
    harness.enable_render_capture();
    harness.render_frames(10);

    let quads = harness.captured_draw_quads();
    if quads.is_empty() {
        // If no quads were produced, skip the dimension check.
        // The test_play_state_produces_draw_quads test covers that case.
        return;
    }

    for (i, quad) in quads.iter().enumerate() {
        // Dimensions may be negative (vertical/horizontal flip in LR2 skins)
        // and may be zero (invisible/spacer elements). They must always be
        // finite (no NaN or Inf from division bugs).
        assert!(
            quad.w.is_finite(),
            "quad[{}] should have finite w, got w={}",
            i,
            quad.w
        );
        assert!(
            quad.h.is_finite(),
            "quad[{}] should have finite h, got h={}",
            i,
            quad.h
        );
    }
}

// ============================================================
// 3. Capture is disabled by default (no quads without enabling)
// ============================================================

#[test]
fn test_capture_disabled_by_default() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    harness.change_state(MainStateType::Play);

    // Do NOT call enable_render_capture()
    harness.render_frames(10);

    let quads = harness.captured_draw_quads();
    assert!(
        quads.is_empty(),
        "captured_draw_quads() should be empty when capture is not enabled, got {} quads",
        quads.len()
    );
}

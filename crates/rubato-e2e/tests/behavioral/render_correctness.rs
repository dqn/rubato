//! Phase 5c: Render correctness E2E tests.
//!
//! Tests SpriteBatch capture infrastructure for draw quad output
//! verification without GPU.

use std::path::PathBuf;

use rubato_e2e::{E2eHarness, MainStateType};
use rubato_game::core::config::Config;
use rubato_game::core::main_loader::MainLoader;
use rubato_game::state_factory::LauncherStateFactory;
use rubato_types::main_controller_access::MainControllerAccess;
use rubato_types::player_config::PlayerConfig;
use rubato_types::skin_config::SkinConfig;
use rubato_types::skin_type::SkinType;
use rubato_types::timer_id::TimerId;

fn test_bms_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-bms")
}

fn harness_with_factory() -> E2eHarness {
    E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator())
}

fn harness_with_player(player: PlayerConfig) -> E2eHarness {
    E2eHarness::new_with_player_config(player)
        .with_state_factory(LauncherStateFactory::new().into_creator())
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

fn ecfn_player_config() -> PlayerConfig {
    let mut player = PlayerConfig::default();
    let play7_idx = SkinType::Play7Keys.id() as usize;
    player.skin[play7_idx] = Some(SkinConfig::new_with_path("skin/ECFN/play/play7.luaskin"));
    player.validate();
    player
}

fn ecfn_harness_with_bms(bms_filename: &str) -> Option<E2eHarness> {
    let bms_path = test_bms_dir().join(bms_filename);
    if !bms_path.exists() {
        return None;
    }

    let mut harness = harness_with_player(ecfn_player_config());
    harness.controller_mut().create();

    let loaded = harness
        .controller_mut()
        .player_resource_mut()
        .expect("controller should own a player resource")
        .set_bms_file(&bms_path, 2, 0); // mode_type=2 is AUTOPLAY
    assert!(loaded, "BMS file should load successfully");

    Some(harness)
}

fn main_loader_harness_with_bms(bms_filename: &str) -> Option<E2eHarness> {
    let bms_path = test_bms_dir().join(bms_filename);
    if !bms_path.exists() {
        return None;
    }

    let mut controller = MainLoader::play(
        Some(bms_path),
        None,
        false,
        Some(Config::default()),
        None,
        false,
    )
    .expect("MainLoader::play should succeed");
    controller.set_state_factory(LauncherStateFactory::new().into_creator());

    let mut harness = E2eHarness::from_controller(controller);
    harness.controller_mut().create();
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

#[test]
#[ignore = "debug helper"]
fn debug_dump_play_quads_after_warmup() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    harness.change_state(MainStateType::Play);
    harness.enable_render_capture();
    harness.render_frames(120);

    let quads = harness.captured_draw_quads();
    println!("captured_quads={}", quads.len());
    for (i, quad) in quads.iter().take(40).enumerate() {
        println!(
            "quad[{i}] tex={:?} pos=({}, {}) size=({}, {}) color={:?}",
            quad.texture_key, quad.x, quad.y, quad.w, quad.h, quad.color
        );
    }
}

#[test]
fn test_ecfn_play_skin_runtime_draws_more_than_lane_only() {
    let Some(mut harness) = ecfn_harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    harness.change_state(MainStateType::Play);
    harness.enable_render_capture();
    harness.render_frames(120);

    let quads = harness.captured_draw_quads();
    let unique_textures = quads
        .iter()
        .filter_map(|quad| quad.texture_key.as_deref())
        .collect::<std::collections::BTreeSet<_>>();

    assert!(
        quads.len() > 100,
        "ECFN play skin should render far more than lane-only quads, got {}",
        quads.len()
    );
    assert!(
        unique_textures.len() >= 3,
        "ECFN play skin should use multiple texture groups, got {:?}",
        unique_textures
    );
}

#[test]
fn test_main_loader_uses_root_ecfn_play_skin_and_renders() {
    let Some(mut harness) = main_loader_harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    let play7_idx = SkinType::Play7Keys.id() as usize;
    let skin_path = harness.controller().player_config().skin[play7_idx]
        .as_ref()
        .and_then(|skin| skin.path());
    assert_eq!(
        skin_path,
        Some("skin/ECFN/play/play7.luaskin"),
        "MainLoader should pick up the root config_player.json ECFN play skin"
    );

    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));

    harness.enable_render_capture();
    harness.render_frames(120);

    let quads = harness.captured_draw_quads();
    assert!(
        quads.len() > 100,
        "MainLoader + root config_player.json should still render more than lane-only quads, got {}",
        quads.len()
    );
}

#[test]
fn test_main_loader_ecfn_play_turns_on_judge_timer() {
    let Some(mut harness) = main_loader_harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    let judge_timer = TimerId::new(46);
    let state_timer_on = |h: &E2eHarness| {
        h.controller()
            .current_state()
            .is_some_and(|state| state.main_state_data().timer.is_timer_on(judge_timer))
    };
    assert!(
        !state_timer_on(&harness),
        "judge timer should start off before autoplay judgments"
    );

    harness.render_until(state_timer_on, 1200);

    assert!(
        state_timer_on(&harness),
        "MainLoader ECFN play path should turn on judge timer 46, frame_state={:?}",
        harness.dump_frame_state()
    );
}

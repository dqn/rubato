// Wiring integration tests.
//
// These tests verify that the **production assembly order** works correctly.
// Unlike e2e_screen_transitions.rs (which tests MainController::new() directly),
// these tests exercise the actual code path from MainLoader::play() through to
// state transitions and rendering — the same path as main.rs play().
//
// This catches "missing wiring" bugs where individual components work but the
// caller fails to assemble all required parts before first use.

#![allow(clippy::field_reassign_with_default)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use beatoraja_core::config::Config;
use beatoraja_core::main_controller::MainController;
use beatoraja_core::main_loader::MainLoader;
use beatoraja_core::main_state::MainStateType;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_launcher::state_factory::LauncherStateFactory;
use beatoraja_types::main_state_access::{MainStateAccess, MainStateListener};

// ---------------------------------------------------------------------------
// Helper: create controller via MainLoader::play() (production path)
// ---------------------------------------------------------------------------

fn play_default() -> MainController {
    MainLoader::play(
        None,
        None,
        false,
        Some(Config::default()),
        Some(PlayerConfig::default()),
        false,
    )
    .expect("MainLoader::play() should succeed with defaults")
}

fn play_with_bmsfile() -> MainController {
    let bms_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-bms")
        .join("minimal_7k.bms");
    MainLoader::play(
        Some(bms_path),
        None,
        false,
        Some(Config::default()),
        Some(PlayerConfig::default()),
        false,
    )
    .expect("MainLoader::play() should succeed with bmsfile")
}

// ---------------------------------------------------------------------------
// A. MainLoader::play() → set_state_factory() → change_state() succeeds
// ---------------------------------------------------------------------------

#[test]
fn play_set_factory_change_state_succeeds() {
    let mut mc = play_default();
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));

    mc.change_state(MainStateType::MusicSelect);

    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect),
    );
}

// ---------------------------------------------------------------------------
// B. MainLoader::play() → set_factory → create() enters MusicSelect
// ---------------------------------------------------------------------------

#[test]
fn play_create_enters_music_select() {
    let mut mc = play_default();
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));

    mc.create();

    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect),
    );
}

// ---------------------------------------------------------------------------
// C. MainLoader::play() with bmsfile → create() enters Play
// ---------------------------------------------------------------------------

#[test]
fn play_with_bmsfile_create_enters_play() {
    let mut mc = play_with_bmsfile();
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));

    mc.create();

    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
}

// ---------------------------------------------------------------------------
// D. create() without factory panics (fail-fast verification)
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "No state factory set")]
fn create_without_factory_panics() {
    let mut mc = play_default();
    // Do NOT set factory
    mc.create();
}

// ---------------------------------------------------------------------------
// E. create() → render() works (first frame after initialization)
// ---------------------------------------------------------------------------

#[test]
fn create_then_render_first_frame() {
    let mut mc = play_default();
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));

    mc.create();
    mc.render();

    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect),
    );
}

// ---------------------------------------------------------------------------
// F. State listeners are dispatched on state change
// ---------------------------------------------------------------------------

struct TestListener {
    called: Arc<AtomicBool>,
}

impl MainStateListener for TestListener {
    fn update(&mut self, _state: &dyn MainStateAccess, _status: i32) {
        self.called.store(true, Ordering::SeqCst);
    }
}

#[test]
fn state_listeners_dispatched_on_change() {
    let mut mc = play_default();
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));

    let called = Arc::new(AtomicBool::new(false));
    mc.add_state_listener(Box::new(TestListener {
        called: called.clone(),
    }));

    mc.change_state(MainStateType::MusicSelect);

    assert!(
        called.load(Ordering::SeqCst),
        "listener should have been called"
    );
}

// ---------------------------------------------------------------------------
// G. create() without audio driver logs but doesn't panic
// ---------------------------------------------------------------------------

#[test]
fn create_without_audio_driver_succeeds() {
    let mut mc = play_default();
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));

    // No audio driver set — create() should succeed (audio is optional)
    mc.create();

    assert!(mc.get_current_state().is_some());
}

// ---------------------------------------------------------------------------
// H. Full production wiring sequence mirrors main.rs play()
// ---------------------------------------------------------------------------

#[test]
fn full_production_wiring_sequence() {
    // 1. MainLoader::play()
    let mut mc = play_default();

    // 2. set_state_factory()
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));

    // 3. Add state listener (mirrors Discord/OBS listener wiring)
    let listener_called = Arc::new(AtomicBool::new(false));
    mc.add_state_listener(Box::new(TestListener {
        called: listener_called.clone(),
    }));

    // 4. create() (called from event loop's resumed())
    mc.create();
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect),
    );
    assert!(mc.get_sprite_batch().is_some());
    assert!(listener_called.load(Ordering::SeqCst));

    // 5. render() multiple frames
    for _ in 0..3 {
        mc.render();
    }
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect),
    );

    // 6. dispose()
    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

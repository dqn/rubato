// Wiring integration tests.
//
// These tests verify that the **production assembly order** works correctly.
// Unlike e2e_screen_transitions.rs (which tests MainController::new() directly),
// these tests exercise the actual code path from MainLoader::play() through to
// state transitions and rendering — the same path as main.rs play().
//
// This catches "missing wiring" bugs where individual components work but the
// caller fails to assemble all required parts before first use.

use rubato_game::core::config::Config;
use rubato_game::core::main_controller::MainController;
use rubato_game::core::main_loader::MainLoader;
use rubato_game::core::main_state::MainStateType;
use rubato_game::core::player_config::PlayerConfig;
use rubato_game::state_factory::LauncherStateFactory;
use rubato_types::app_event::AppEvent;

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
    mc.set_state_factory(LauncherStateFactory::new().into_creator());

    mc.change_state(MainStateType::MusicSelect);

    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect),);
}

// ---------------------------------------------------------------------------
// B. MainLoader::play() → set_factory → create() enters MusicSelect
// ---------------------------------------------------------------------------

#[test]
fn play_create_enters_music_select() {
    let mut mc = play_default();
    mc.set_state_factory(LauncherStateFactory::new().into_creator());

    mc.create();

    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect),);
}

// ---------------------------------------------------------------------------
// C. MainLoader::play() with bmsfile → create() enters Play
// ---------------------------------------------------------------------------

#[test]
fn play_with_bmsfile_create_enters_play() {
    let mut mc = play_with_bmsfile();
    mc.set_state_factory(LauncherStateFactory::new().into_creator());

    mc.create();

    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));
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
    mc.set_state_factory(LauncherStateFactory::new().into_creator());

    mc.create();
    mc.render();

    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect),);
}

// ---------------------------------------------------------------------------
// F. Event senders receive StateChanged on state change
// ---------------------------------------------------------------------------

#[test]
fn event_senders_receive_state_changed_on_transition() {
    let mut mc = play_default();
    mc.set_state_factory(LauncherStateFactory::new().into_creator());

    let (tx, rx) = std::sync::mpsc::sync_channel::<AppEvent>(256);
    mc.add_event_sender(tx);

    mc.change_state(MainStateType::MusicSelect);

    // Drain events and check for a StateChanged event
    let mut found = false;
    while let Ok(event) = rx.try_recv() {
        if matches!(event, AppEvent::StateChanged(_)) {
            found = true;
            break;
        }
    }
    assert!(found, "should have received a StateChanged event");
}

// ---------------------------------------------------------------------------
// G. create() without audio driver logs but doesn't panic
// ---------------------------------------------------------------------------

#[test]
fn create_without_audio_driver_succeeds() {
    let mut mc = play_default();
    mc.set_state_factory(LauncherStateFactory::new().into_creator());

    // No audio driver set — create() should succeed (audio is optional)
    mc.create();

    assert!(mc.current_state().is_some());
}

// ---------------------------------------------------------------------------
// H. Full production wiring sequence mirrors main.rs play()
// ---------------------------------------------------------------------------

#[test]
fn full_production_wiring_sequence() {
    // 1. MainLoader::play()
    let mut mc = play_default();

    // 2. set_state_factory()
    mc.set_state_factory(LauncherStateFactory::new().into_creator());

    // 3. Add event sender (mirrors Discord/OBS listener wiring)
    let (tx, rx) = std::sync::mpsc::sync_channel::<AppEvent>(256);
    mc.add_event_sender(tx);

    // 4. create() (called from event loop's resumed())
    mc.create();
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect),);
    assert!(mc.sprite_batch().is_some());
    // Verify at least one StateChanged event was received
    let mut found_state_changed = false;
    while let Ok(event) = rx.try_recv() {
        if matches!(event, AppEvent::StateChanged(_)) {
            found_state_changed = true;
        }
    }
    assert!(
        found_state_changed,
        "should have received StateChanged event after create()"
    );

    // 5. render() multiple frames
    for _ in 0..3 {
        mc.render();
    }
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect),);

    // 6. dispose()
    mc.dispose();
    assert!(mc.current_state().is_none());
}

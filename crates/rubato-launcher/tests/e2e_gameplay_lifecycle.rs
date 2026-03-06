// E2E gameplay lifecycle tests.
//
// Tests the full BMS load -> play -> result lifecycle with real BMS data.
// Unlike e2e_screen_transitions.rs (which tests transitions with default/empty models),
// these tests load actual BMS files and verify the complete gameplay pipeline works
// end-to-end with real chart data.
//
// Verifies:
// - BMS file loading via PlayerResource.set_bms_file() with real test fixtures
// - State transitions with loaded BMS data: MusicSelect -> Decide -> Play -> Result
// - Direct BMS launch path: create() with bmsfile -> Play -> Result
// - Lifecycle methods (create/render/dispose) work with real chart data in each state
// - PlayerResource correctly propagates BMS model to Play state via factory

#![allow(clippy::field_reassign_with_default)]

use std::path::PathBuf;

use rubato_core::config::Config;
use rubato_core::main_controller::MainController;
use rubato_core::main_loader::MainLoader;
use rubato_core::main_state::MainStateType;
use rubato_core::player_config::PlayerConfig;
use rubato_launcher::state_factory::LauncherStateFactory;
use rubato_types::main_controller_access::MainControllerAccess;

fn test_bms_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-bms")
        .join("minimal_7k.bms")
}

fn make_controller_with_factory() -> MainController {
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));
    mc
}

// ---------------------------------------------------------------------------
// A. Full gameplay lifecycle with BMS file: MusicSelect -> Decide -> Play -> Result
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_select_decide_play_result_with_bms() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    let mut mc = make_controller_with_factory();

    // create() initializes PlayerResource and enters MusicSelect (no bmsfile arg)
    mc.create();
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect),
    );

    // Load BMS file onto PlayerResource (simulates song selection)
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist after create()");
        // mode_type 0 = Play mode
        let loaded = resource.set_bms_file(&bms_path, 0, 0);
        assert!(loaded, "BMS file should load successfully");
        assert!(
            resource.bms_model().is_some(),
            "BMS model should be available after loading"
        );
    }

    // Render a frame in MusicSelect with BMS data loaded
    mc.render();
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect),
    );

    // Transition to Decide
    mc.change_state(MainStateType::Decide);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Decide));
    mc.render();

    // Transition to Play (factory reads BMS model from PlayerResource)
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    mc.render();

    // Transition to Result
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));
    mc.render();

    // Return to MusicSelect (normal game loop)
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect),
    );

    // Clean dispose
    mc.dispose();
    assert!(mc.get_current_state().is_none());
    assert!(mc.get_current_state_type().is_none());
}

// ---------------------------------------------------------------------------
// B. Direct BMS launch: create(bmsfile) -> Play -> Result
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_direct_bms_launch_play_to_result() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    // Use MainLoader::play() with bmsfile (production path for direct launch)
    let mut mc = MainLoader::play(
        Some(bms_path),
        None,
        false,
        Some(Config::default()),
        Some(PlayerConfig::default()),
        false,
    )
    .expect("MainLoader::play() should succeed");
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));

    // create() loads the BMS file and enters Play directly
    mc.create();
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));

    // Verify BMS model was loaded into PlayerResource
    let has_model = mc
        .get_player_resource()
        .and_then(|r| r.get_bms_model())
        .is_some();
    assert!(has_model, "BMS model should be loaded in PlayerResource");

    // Render multiple frames in Play state
    for _ in 0..3 {
        mc.render();
    }
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));

    // Transition to Result (end of song)
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));
    mc.render();

    // Clean dispose
    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

// ---------------------------------------------------------------------------
// C. BMS load -> Play with lifecycle methods (pause/resume/resize)
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_play_lifecycle_with_bms() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // Enter Play state
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));

    // Exercise full lifecycle methods
    mc.render();
    mc.pause();
    mc.resume();
    mc.resize(1920, 1080);
    mc.render();
    mc.resize(1280, 720);
    mc.render();

    // State should remain Play throughout lifecycle
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
}

// ---------------------------------------------------------------------------
// D. BMS data propagation: model reaches Play state via factory
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_bms_model_propagates_to_play_state() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS and capture expected note count
    let expected_notes;
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
        let model = resource
            .bms_model()
            .expect("model should be present after load");
        expected_notes = model.total_notes();
        assert!(
            expected_notes > 0,
            "test BMS file should have at least 1 note"
        );
    }

    // Enter Play state - factory reads model from PlayerResource
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));

    // Verify the BMS model is still accessible via PlayerResource
    let actual_notes = mc
        .get_player_resource()
        .and_then(|r| r.get_bms_model())
        .map(|m| m.total_notes())
        .unwrap_or(0);
    assert_eq!(
        actual_notes, expected_notes,
        "BMS model note count should be preserved through Play state creation"
    );
}

// ---------------------------------------------------------------------------
// E. Course result path: Play -> CourseResult with BMS
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_play_to_course_result_with_bms() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // Play -> CourseResult (course mode end-of-song path)
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    mc.render();

    mc.change_state(MainStateType::CourseResult);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::CourseResult),
    );
    mc.render();

    // Back to MusicSelect
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect),
    );

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

// ---------------------------------------------------------------------------
// F. Multiple play sessions: load BMS, play, result, re-enter play
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_multiple_play_sessions() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    for session in 0..3 {
        // Load BMS (simulates re-selecting the same song)
        {
            let resource = mc
                .player_resource_mut()
                .expect("PlayerResource should exist");
            assert!(
                resource.set_bms_file(&bms_path, 0, 0),
                "session {} BMS load failed",
                session
            );
        }

        // Play
        mc.change_state(MainStateType::Play);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Play),
            "session {} Play state failed",
            session
        );
        mc.render();

        // Result
        mc.change_state(MainStateType::Result);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Result),
            "session {} Result state failed",
            session
        );
        mc.render();

        // Back to MusicSelect
        mc.change_state(MainStateType::MusicSelect);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect),
            "session {} MusicSelect state failed",
            session
        );
    }

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

// ---------------------------------------------------------------------------
// G. Skip decide screen with BMS: MusicSelect -> Decide(skipped) -> Play -> Result
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_skip_decide_with_bms() {
    let bms_path = test_bms_path();

    let mut config = Config::default();
    config.skip_decide_screen = true;
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));
    mc.create();

    // Load BMS
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // When skip_decide_screen is true, requesting Decide creates Play instead
    mc.change_state(MainStateType::Decide);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::Play),
        "Decide should skip to Play when skip_decide_screen is true"
    );
    mc.render();

    // Continue to Result
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));
    mc.render();

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

// ---------------------------------------------------------------------------
// H. Render multiple frames per state with BMS data
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_sustained_rendering_with_bms() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // Render 10 frames in each state to test sustained operation
    let states = [
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
    ];

    for state_type in &states {
        mc.change_state(*state_type);
        for frame in 0..10 {
            mc.render();
            assert_eq!(
                mc.get_current_state_type(),
                Some(*state_type),
                "State should remain {:?} at frame {}",
                state_type,
                frame,
            );
        }
    }

    mc.dispose();
}

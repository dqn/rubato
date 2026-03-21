// E2E screen transition tests.
//
// Tests the full game lifecycle: select -> decide -> play -> result
// using LauncherStateFactory with real state types.
//
// Verifies:
// - LauncherStateFactory creates all 7 state types
// - MainController.change_state() transitions correctly
// - Lifecycle methods (create/prepare/render/shutdown) are dispatched
// - State type is correct after each transition
// - Dispose clears all state

use rubato_core::config::Config;
use rubato_core::main_controller::MainController;
use rubato_core::main_state::MainStateType;
use rubato_core::player_config::PlayerConfig;
use rubato_core::player_resource::PlayerResource;
use rubato_launcher::state_factory::LauncherStateFactory;

fn make_controller_with_factory() -> MainController {
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));
    mc
}

// ---------------------------------------------------------------------------
// Full transition chain: Select -> Decide -> Play -> Result
// ---------------------------------------------------------------------------

#[test]
fn e2e_select_to_decide_to_play_to_result() {
    let mut mc = make_controller_with_factory();
    // Initialize controller to create PlayerResource (required for Decide).
    mc.create();

    // 1. Start at MusicSelect
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    // Render a frame to exercise the render lifecycle
    mc.render();
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    // 2. Transition to Decide
    mc.change_state(MainStateType::Decide);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Decide));

    // 3. Transition to Play
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

    // 4. Transition to Result
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Result));

    // 5. Back to MusicSelect (normal game flow)
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

// ---------------------------------------------------------------------------
// Full transition chain: Select -> Play -> CourseResult
// ---------------------------------------------------------------------------

#[test]
fn e2e_select_to_play_to_course_result() {
    let mut mc = make_controller_with_factory();

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    mc.change_state(MainStateType::Play);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

    mc.restore_player_resource(PlayerResource::new(
        Config::default(),
        PlayerConfig::default(),
    ));
    mc.change_state(MainStateType::CourseResult);
    assert_eq!(mc.current_state_type(), Some(MainStateType::CourseResult));

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

// ---------------------------------------------------------------------------
// Config / SkinConfig screen transitions
// ---------------------------------------------------------------------------

#[test]
fn e2e_select_to_config_and_back() {
    let mut mc = make_controller_with_factory();

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    mc.change_state(MainStateType::Config);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Config));

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn e2e_select_to_skin_config_and_back() {
    let mut mc = make_controller_with_factory();

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    mc.change_state(MainStateType::SkinConfig);
    assert_eq!(mc.current_state_type(), Some(MainStateType::SkinConfig));

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

// ---------------------------------------------------------------------------
// Lifecycle verification: render, pause, resume, resize across transitions
// ---------------------------------------------------------------------------

#[test]
fn e2e_lifecycle_across_transitions() {
    let mut mc = make_controller_with_factory();

    // Start at Select
    mc.change_state(MainStateType::MusicSelect);

    // Exercise lifecycle
    mc.render();
    mc.pause();
    mc.resume();
    mc.resize(1920, 1080);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    // Transition to Play (shutdown old, create+prepare new)
    mc.change_state(MainStateType::Play);

    // Exercise lifecycle on Play state
    mc.render();
    mc.pause();
    mc.resume();
    mc.resize(1280, 720);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

    // Transition to Result (requires a PlayerResource)
    mc.restore_player_resource(PlayerResource::new(
        Config::default(),
        PlayerConfig::default(),
    ));
    mc.change_state(MainStateType::Result);
    mc.render();
    assert_eq!(mc.current_state_type(), Some(MainStateType::Result));
}

// ---------------------------------------------------------------------------
// Dispose lifecycle clears current state
// ---------------------------------------------------------------------------

#[test]
fn e2e_dispose_clears_all_state() {
    let mut mc = make_controller_with_factory();

    mc.change_state(MainStateType::MusicSelect);
    mc.render();
    assert!(mc.current_state().is_some());

    mc.dispose();
    assert!(mc.current_state().is_none());
    assert!(mc.current_state_type().is_none());
}

// ---------------------------------------------------------------------------
// Skip decide screen config: Decide -> Play
// ---------------------------------------------------------------------------

#[test]
fn e2e_skip_decide_screen() {
    let mut config = Config::default();
    config.select.skip_decide_screen = true;
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(Box::new(LauncherStateFactory::new()));
    // Initialize controller to create PlayerResource (required for Decide).
    mc.create();

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    // When skip_decide_screen is true, Decide creates Play instead
    mc.change_state(MainStateType::Decide);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));
}

// ---------------------------------------------------------------------------
// Same state transition is no-op
// ---------------------------------------------------------------------------

#[test]
fn e2e_same_state_transition_noop() {
    let mut mc = make_controller_with_factory();

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    // Transitioning to the same state should be a no-op
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
}

// ---------------------------------------------------------------------------
// All 7 state types can be created and entered
// ---------------------------------------------------------------------------

#[test]
fn e2e_all_state_types_reachable() {
    let mut mc = make_controller_with_factory();
    // Initialize controller to create PlayerResource (required for Decide).
    mc.create();

    let types = [
        MainStateType::MusicSelect,
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::CourseResult,
        MainStateType::Config,
        MainStateType::SkinConfig,
    ];

    for state_type in &types {
        mc.change_state(*state_type);
        let expected =
            if mc.config.select.skip_decide_screen && *state_type == MainStateType::Decide {
                MainStateType::Play
            } else {
                *state_type
            };
        assert_eq!(
            mc.current_state_type(),
            Some(expected),
            "Failed to enter state {:?}",
            state_type
        );
    }
}

// ---------------------------------------------------------------------------
// create() initializes states and enters initial state
// ---------------------------------------------------------------------------

#[test]
fn e2e_create_enters_initial_state() {
    let mut mc = make_controller_with_factory();

    // Before create(), no state is set
    assert!(mc.current_state().is_none());

    // create() should initialize and enter MusicSelect (no bmsfile)
    mc.create();

    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
    assert!(mc.sprite_batch().is_some());
}

// ---------------------------------------------------------------------------
// Rapid transitions don't panic
// ---------------------------------------------------------------------------

#[test]
fn e2e_rapid_transitions_no_panic() {
    let mut mc = make_controller_with_factory();

    // Rapid-fire state transitions to stress test
    for _ in 0..10 {
        mc.change_state(MainStateType::MusicSelect);
        mc.change_state(MainStateType::Decide);
        mc.change_state(MainStateType::Play);
        mc.change_state(MainStateType::Result);
        mc.change_state(MainStateType::CourseResult);
        mc.change_state(MainStateType::Config);
        mc.change_state(MainStateType::SkinConfig);
    }

    // Should end on SkinConfig
    assert_eq!(mc.current_state_type(), Some(MainStateType::SkinConfig));
}

// ---------------------------------------------------------------------------
// Render after transition does not panic
// ---------------------------------------------------------------------------

#[test]
fn e2e_render_after_each_transition() {
    let mut mc = make_controller_with_factory();

    let types = [
        MainStateType::MusicSelect,
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::CourseResult,
        MainStateType::Config,
        MainStateType::SkinConfig,
    ];

    for state_type in &types {
        mc.change_state(*state_type);
        // Rendering should not panic for any state type
        mc.render();
    }
}

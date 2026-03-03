// E2E state transition combination tests.
//
// Tests additional state transition paths not covered by
// e2e_screen_transitions.rs or e2e_gameplay_lifecycle.rs:
//
// 1. Practice mode -> Result state transition
// 2. CoursePlayer multi-song sequences (Play -> Play -> CourseResult)
// 3. State transitions with missing skin/config (graceful handling)
// 4. Rapid state cycling with render (select -> decide -> play -> select loop)
//
// Follows the same patterns as e2e_screen_transitions.rs.

#![allow(clippy::field_reassign_with_default)]

use std::path::PathBuf;

use beatoraja_core::config::Config;
use beatoraja_core::main_controller::MainController;
use beatoraja_core::main_state::MainStateType;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_launcher::state_factory::LauncherStateFactory;
use beatoraja_types::course_data::CourseData;
use beatoraja_types::main_controller_access::MainControllerAccess;

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
// 1. Practice mode -> Result state transition
// ---------------------------------------------------------------------------

#[test]
fn e2e_practice_mode_to_result() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS in practice mode (mode_type 1 = Practice)
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist after create()");
        let loaded = resource.set_bms_file(&bms_path, 1, 0);
        assert!(loaded, "BMS file should load successfully in practice mode");
    }

    // Enter Play state (practice mode is a variant of Play state)
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    mc.render();

    // Transition from Play to Result (practice session ends)
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));
    mc.render();

    // Return to MusicSelect
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect)
    );

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_practice_mode_lifecycle_with_render() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS in practice mode
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 1, 0));
    }

    // Enter Play state from practice
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));

    // Exercise lifecycle in practice mode
    mc.render();
    mc.pause();
    mc.resume();
    mc.resize(1920, 1080);
    mc.render();

    // Practice -> Result
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));
    mc.render();

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_practice_mode_back_to_select_skipping_result() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS in practice mode
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 1, 0));
    }

    // Practice Play -> skip Result -> back to MusicSelect
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    mc.render();

    // Directly return to MusicSelect (practice abort / cancel)
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect)
    );

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

// ---------------------------------------------------------------------------
// 2. CoursePlayer multi-song sequences
// ---------------------------------------------------------------------------

#[test]
fn e2e_course_play_multi_song_sequence() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    let mut mc = make_controller_with_factory();
    mc.create();

    // Set up course data on PlayerResource
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist");
        let course = CourseData::default();
        resource.set_course_data(course);
        // Load the first song
        let loaded = resource.set_bms_file(&bms_path, 0, 0);
        assert!(loaded, "First song of course should load successfully");
    }

    // Select -> Decide -> Play (first song in course)
    mc.change_state(MainStateType::Decide);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Decide));
    mc.render();

    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    mc.render();

    // First song finishes -> transition to Play for second song
    // (In real course play, BMSPlayer requests next_course() and
    // then change_state(Play) again with a new model loaded.)
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist");
        let loaded = resource.set_bms_file(&bms_path, 0, 0);
        assert!(loaded, "Second song of course should load successfully");
    }

    // Force a new Play state (simulates second song transition)
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));
    mc.render();

    // Back to Play for third song
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist");
        let loaded = resource.set_bms_file(&bms_path, 0, 0);
        assert!(loaded, "Third song of course should load successfully");
    }

    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    mc.render();

    // Course ends -> CourseResult
    mc.change_state(MainStateType::CourseResult);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::CourseResult)
    );
    mc.render();

    // Back to MusicSelect
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect)
    );

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_course_play_to_course_result_with_renders() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Set up course data
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist");
        let mut course = CourseData::default();
        course.name = Some("Test Course".to_string());
        resource.set_course_data(course);
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // Play song 1 with sustained rendering
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    for _ in 0..5 {
        mc.render();
    }

    // Play song 2
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }
    mc.change_state(MainStateType::Result);
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    for _ in 0..5 {
        mc.render();
    }

    // CourseResult with sustained rendering
    mc.change_state(MainStateType::CourseResult);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::CourseResult)
    );
    for _ in 0..5 {
        mc.render();
    }

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_course_data_cleared_after_course_result() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Set up course data
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist");
        let mut course = CourseData::default();
        course.name = Some("Cleanup Course".to_string());
        resource.set_course_data(course);
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // Verify course data is set
    {
        let resource = mc
            .get_player_resource()
            .expect("PlayerResource should exist");
        assert!(
            resource.get_course_data().is_some(),
            "Course data should be set before play"
        );
    }

    // Play -> CourseResult
    mc.change_state(MainStateType::Play);
    mc.render();
    mc.change_state(MainStateType::CourseResult);
    mc.render();

    // Clear course data (simulates what MusicSelector does on return)
    {
        let resource = mc
            .get_player_resource_mut()
            .expect("PlayerResource should exist");
        resource.clear_course_data();
    }

    // Back to MusicSelect - course data should be cleared
    mc.change_state(MainStateType::MusicSelect);
    {
        let resource = mc
            .get_player_resource()
            .expect("PlayerResource should exist");
        assert!(
            resource.get_course_data().is_none(),
            "Course data should be cleared after course result"
        );
    }

    mc.dispose();
}

// ---------------------------------------------------------------------------
// 3. State transitions with missing skin/config (graceful handling)
// ---------------------------------------------------------------------------

#[test]
fn e2e_transitions_without_bms_data() {
    // All state transitions should work even without loading BMS data.
    // States will have default/empty models but should not panic.
    let mut mc = make_controller_with_factory();

    // Transition through all gameplay states without loading any BMS file
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect)
    );
    mc.render();

    mc.change_state(MainStateType::Decide);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Decide));
    mc.render();

    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    mc.render();

    mc.change_state(MainStateType::Result);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));
    mc.render();

    mc.change_state(MainStateType::CourseResult);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::CourseResult)
    );
    mc.render();

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_transitions_without_create() {
    // Transitioning without calling create() first.
    // PlayerResource won't exist, but factory should still create states
    // that work without it.
    let mut mc = make_controller_with_factory();

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect)
    );

    mc.change_state(MainStateType::Config);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Config));

    mc.change_state(MainStateType::SkinConfig);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::SkinConfig));

    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_lifecycle_after_dispose_and_reinitialize() {
    // Test dispose then re-create (simulates game restart without process exit)
    let mut mc = make_controller_with_factory();

    // First lifecycle
    mc.create();
    mc.change_state(MainStateType::Play);
    mc.render();
    mc.dispose();
    assert!(mc.get_current_state().is_none());

    // Second lifecycle (re-create after dispose)
    mc.create();
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect)
    );
    mc.render();

    mc.change_state(MainStateType::Play);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    mc.render();

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_render_pause_resume_on_all_states_without_bms() {
    // Exercise full lifecycle methods on every state type, even without BMS data.
    // This tests graceful fallback for missing skin/config resources.
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
        let expected = if mc.config.skip_decide_screen && *state_type == MainStateType::Decide {
            MainStateType::Play
        } else {
            *state_type
        };
        assert_eq!(
            mc.get_current_state_type(),
            Some(expected),
            "Failed to enter state {:?}",
            state_type
        );

        // Exercise full lifecycle
        mc.render();
        mc.pause();
        mc.resume();
        mc.resize(800, 600);
        mc.render();

        assert_eq!(
            mc.get_current_state_type(),
            Some(expected),
            "State should remain {:?} after lifecycle methods",
            expected
        );
    }

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_config_screens_with_default_config() {
    // Config and SkinConfig screens should work without any skin resources loaded
    let mut mc = make_controller_with_factory();

    // MusicSelect -> Config -> SkinConfig -> Config -> MusicSelect
    mc.change_state(MainStateType::MusicSelect);
    mc.render();

    mc.change_state(MainStateType::Config);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Config));
    mc.render();
    mc.render();

    mc.change_state(MainStateType::SkinConfig);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::SkinConfig));
    mc.render();
    mc.render();

    mc.change_state(MainStateType::Config);
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Config));
    mc.render();

    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect)
    );

    mc.dispose();
}

// ---------------------------------------------------------------------------
// 4. Rapid state cycling (select -> decide -> play -> select loop)
// ---------------------------------------------------------------------------

#[test]
fn e2e_rapid_select_decide_play_select_cycle() {
    let mut mc = make_controller_with_factory();

    // Rapid cycle: select -> decide -> play -> select, repeated 10 times
    for iteration in 0..10 {
        mc.change_state(MainStateType::MusicSelect);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect),
            "iteration {} MusicSelect failed",
            iteration
        );

        mc.change_state(MainStateType::Decide);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Decide),
            "iteration {} Decide failed",
            iteration
        );

        mc.change_state(MainStateType::Play);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Play),
            "iteration {} Play failed",
            iteration
        );
    }

    // Should still be functional after rapid cycling
    mc.change_state(MainStateType::MusicSelect);
    mc.render();
    assert_eq!(
        mc.get_current_state_type(),
        Some(MainStateType::MusicSelect)
    );

    mc.dispose();
}

#[test]
fn e2e_rapid_select_decide_play_result_select_cycle_with_render() {
    let mut mc = make_controller_with_factory();

    // Full game loop cycle with render at each step, repeated
    for iteration in 0..5 {
        mc.change_state(MainStateType::MusicSelect);
        mc.render();
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect),
            "iteration {} MusicSelect failed",
            iteration
        );

        mc.change_state(MainStateType::Decide);
        mc.render();
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Decide),
            "iteration {} Decide failed",
            iteration
        );

        mc.change_state(MainStateType::Play);
        mc.render();
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Play),
            "iteration {} Play failed",
            iteration
        );

        mc.change_state(MainStateType::Result);
        mc.render();
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Result),
            "iteration {} Result failed",
            iteration
        );
    }

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_rapid_play_result_alternation() {
    // Stress test: alternate between Play and Result rapidly
    let mut mc = make_controller_with_factory();

    for _ in 0..20 {
        mc.change_state(MainStateType::Play);
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));

        mc.change_state(MainStateType::Result);
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));
    }

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_rapid_full_cycle_with_bms_data() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    for iteration in 0..3 {
        // Load BMS each iteration (simulates re-selecting a song)
        {
            let resource = mc
                .get_player_resource_mut()
                .expect("PlayerResource should exist");
            assert!(
                resource.set_bms_file(&bms_path, 0, 0),
                "iteration {} BMS load failed",
                iteration
            );
        }

        mc.change_state(MainStateType::Decide);
        mc.render();
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Decide),
            "iteration {} Decide failed",
            iteration
        );

        mc.change_state(MainStateType::Play);
        mc.render();
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Play),
            "iteration {} Play failed",
            iteration
        );

        mc.change_state(MainStateType::Result);
        mc.render();
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::Result),
            "iteration {} Result failed",
            iteration
        );

        mc.change_state(MainStateType::MusicSelect);
        mc.render();
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect),
            "iteration {} MusicSelect failed",
            iteration
        );
    }

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_rapid_config_transitions_interleaved_with_gameplay() {
    // Interleave config screens with gameplay transitions
    let mut mc = make_controller_with_factory();

    for _ in 0..5 {
        // Gameplay path
        mc.change_state(MainStateType::MusicSelect);
        mc.render();

        // Detour to Config
        mc.change_state(MainStateType::Config);
        mc.render();

        // Back to select
        mc.change_state(MainStateType::MusicSelect);
        mc.render();

        // Gameplay continues
        mc.change_state(MainStateType::Decide);
        mc.render();

        // Detour to SkinConfig
        mc.change_state(MainStateType::SkinConfig);
        mc.render();

        // Back to Decide
        mc.change_state(MainStateType::Decide);
        mc.render();

        mc.change_state(MainStateType::Play);
        mc.render();

        mc.change_state(MainStateType::Result);
        mc.render();
    }

    // Should end on Result
    assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));

    mc.dispose();
    assert!(mc.get_current_state().is_none());
}

#[test]
fn e2e_dispose_during_mid_cycle() {
    // Dispose at various points during the game cycle
    let types_to_dispose_at = [
        MainStateType::MusicSelect,
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::CourseResult,
        MainStateType::Config,
        MainStateType::SkinConfig,
    ];

    for dispose_at in &types_to_dispose_at {
        let mut mc = make_controller_with_factory();
        mc.change_state(*dispose_at);
        mc.render();

        let expected = if mc.config.skip_decide_screen && *dispose_at == MainStateType::Decide {
            MainStateType::Play
        } else {
            *dispose_at
        };
        assert_eq!(
            mc.get_current_state_type(),
            Some(expected),
            "Failed to enter {:?} before dispose",
            dispose_at
        );

        mc.dispose();
        assert!(
            mc.get_current_state().is_none(),
            "State should be None after dispose at {:?}",
            dispose_at
        );
        assert!(
            mc.get_current_state_type().is_none(),
            "State type should be None after dispose at {:?}",
            dispose_at
        );
    }
}

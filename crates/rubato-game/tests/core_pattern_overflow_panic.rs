// Bug exposure tests for arithmetic overflow and panic issues in beatoraja-pattern.
//
// These tests document latent bugs — they do NOT fix anything.
// Each test either:
//   - Uses #[should_panic] to prove a panic exists
//   - Uses #[ignore] with a BUG comment for silent wrong results
//   - Is a green test documenting edge-case behavior

use bms::model::bms_model::BMSModel;
use bms::model::mode::Mode;
use bms::model::time_line::TimeLine;
use rubato_game::core::pattern::lane_shuffle_modifier::LaneCrossShuffleModifier;

// ---------------------------------------------------------------------------
// LaneCrossShuffleModifier: empty keys causes usize underflow
// ---------------------------------------------------------------------------

/// LaneCrossShuffleModifier::make_random() should return the identity mapping for
/// an empty key slice instead of panicking.
#[test]
fn cross_shuffle_empty_keys_returns_identity() {
    let mut model = BMSModel::new();
    model.timelines = vec![TimeLine::new(0.0, 0, 8)];
    model.set_mode(Mode::BEAT_7K);

    let result = LaneCrossShuffleModifier::make_random(&[], &model, 42);
    assert_eq!(result, (0..Mode::BEAT_7K.key()).collect::<Vec<_>>());
}

/// A single-element key slice should also return the identity mapping.
#[test]
fn cross_shuffle_single_key_returns_identity() {
    let mut model = BMSModel::new();
    model.timelines = vec![TimeLine::new(0.0, 0, 8)];
    model.set_mode(Mode::BEAT_7K);

    let result = LaneCrossShuffleModifier::make_random(&[0], &model, 42);
    assert_eq!(result, (0..Mode::BEAT_7K.key()).collect::<Vec<_>>());
}

// ---------------------------------------------------------------------------
// LaneRotateShuffleModifier: empty keys causes panic in next_int_bounded
// ---------------------------------------------------------------------------

/// LaneRotateShuffleModifier::make_random() should return the identity mapping
/// for an empty key slice instead of calling JavaRandom with an invalid bound.
#[test]
fn rotate_shuffle_empty_keys_returns_identity() {
    use rubato_game::core::pattern::lane_shuffle_modifier::LaneRotateShuffleModifier;

    let mut model = BMSModel::new();
    model.timelines = vec![TimeLine::new(0.0, 0, 8)];
    model.set_mode(Mode::BEAT_7K);

    let result = LaneRotateShuffleModifier::make_random(&[], &model, 42);
    assert_eq!(result, (0..Mode::BEAT_7K.key()).collect::<Vec<_>>());
}

// ---------------------------------------------------------------------------
// ModeModifier: hran_threshold_bpm edge cases (green tests — no bugs)
// ---------------------------------------------------------------------------

/// ModeModifier correctly handles hran_threshold_bpm=0: sets threshold to 0.
/// This is a green test documenting that the guard works.
#[test]
fn mode_modifier_hran_threshold_bpm_zero_handled() {
    use rubato_game::core::pattern::mode_modifier::ModeModifier;
    use rubato_game::core::pattern::pattern_modifier::PatternModifier;
    use rubato_types::player_config::PlayerConfig;

    let config = PlayerConfig {
        play_settings: rubato_types::player_config::PlaySettings {
            hran_threshold_bpm: 0,
            ..Default::default()
        },
        ..Default::default()
    };

    // ModeModifier::modify() checks `if self.config.hran_threshold_bpm <= 0`
    // and sets self.hran_threshold = 0. No division occurs. This should not panic.
    let mut modifier = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.timelines = vec![TimeLine::new(0.0, 0, 9)];

    // Should not panic — the <= 0 branch avoids the division
    modifier.modify(&mut model);
}

/// ModeModifier with negative hran_threshold_bpm also sets threshold to 0 (no panic).
#[test]
fn mode_modifier_hran_threshold_bpm_negative_handled() {
    use rubato_game::core::pattern::mode_modifier::ModeModifier;
    use rubato_game::core::pattern::pattern_modifier::PatternModifier;
    use rubato_types::player_config::PlayerConfig;

    let config = PlayerConfig {
        play_settings: rubato_types::player_config::PlaySettings {
            hran_threshold_bpm: -100,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut modifier = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.timelines = vec![TimeLine::new(0.0, 0, 9)];

    // Should not panic
    modifier.modify(&mut model);
}

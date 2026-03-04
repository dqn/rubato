// Bug exposure tests for arithmetic overflow and panic issues in beatoraja-pattern.
//
// These tests document latent bugs — they do NOT fix anything.
// Each test either:
//   - Uses #[should_panic] to prove a panic exists
//   - Uses #[ignore] with a BUG comment for silent wrong results
//   - Is a green test documenting edge-case behavior

use beatoraja_core::pattern::lane_shuffle_modifier::LaneCrossShuffleModifier;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::time_line::TimeLine;

// ---------------------------------------------------------------------------
// LaneCrossShuffleModifier: empty keys causes usize underflow
// ---------------------------------------------------------------------------

/// BUG: LaneCrossShuffleModifier::make_random() computes `keys.len() / 2 - 1` (line 685).
/// When keys is empty, `keys.len() / 2` = 0, and `0usize - 1` causes arithmetic
/// underflow which panics in debug mode (or wraps to usize::MAX in release mode,
/// leading to an immediate index-out-of-bounds panic on the next line).
///
/// In production, lane_shuffle_modify() calls get_keys_static() which returns empty
/// for invalid player values, and the `if keys.is_empty() { return; }` guard prevents
/// make_random() from being called. However, make_random() is a pub fn that can be
/// called directly with empty keys, and the guard is in the caller, not the function.
#[test]
#[should_panic]
fn cross_shuffle_empty_keys_underflow() {
    let mut model = BMSModel::new();
    model.set_all_time_line(vec![TimeLine::new(0.0, 0, 8)]);
    model.set_mode(Mode::BEAT_7K);

    // Empty keys slice → 0 / 2 - 1 = usize underflow → panic
    let _result = LaneCrossShuffleModifier::make_random(&[], &model, 42);
}

/// Same underflow with a single-element keys slice: len=1, 1/2=0, 0-1 = underflow.
#[test]
#[should_panic]
fn cross_shuffle_single_key_underflow() {
    let mut model = BMSModel::new();
    model.set_all_time_line(vec![TimeLine::new(0.0, 0, 8)]);
    model.set_mode(Mode::BEAT_7K);

    // keys.len()=1 → 1/2=0 → 0-1 = usize underflow → panic
    let _result = LaneCrossShuffleModifier::make_random(&[0], &model, 42);
}

// ---------------------------------------------------------------------------
// LaneRotateShuffleModifier: empty keys causes panic in next_int_bounded
// ---------------------------------------------------------------------------

/// BUG: LaneRotateShuffleModifier::make_random() computes `keys.len() as i32 - 1`
/// and passes it to JavaRandom::next_int_bounded() (line 267).
/// When keys is empty, this becomes next_int_bounded(-1).
///
/// JavaRandom::next_int_bounded() with bound <= 0 will either panic or loop forever
/// depending on the implementation.
///
/// The guard in lane_shuffle_modify() prevents this in normal use, but make_random()
/// is directly callable.
#[test]
#[should_panic]
fn rotate_shuffle_empty_keys_panics() {
    use beatoraja_core::pattern::lane_shuffle_modifier::LaneRotateShuffleModifier;

    let mut model = BMSModel::new();
    model.set_all_time_line(vec![TimeLine::new(0.0, 0, 8)]);
    model.set_mode(Mode::BEAT_7K);

    // keys.len()=0 → next_int_bounded(-1) → panic or infinite loop
    let _result = LaneRotateShuffleModifier::make_random(&[], &model, 42);
}

// ---------------------------------------------------------------------------
// ModeModifier: hran_threshold_bpm edge cases (green tests — no bugs)
// ---------------------------------------------------------------------------

/// ModeModifier correctly handles hran_threshold_bpm=0: sets threshold to 0.
/// This is a green test documenting that the guard works.
#[test]
fn mode_modifier_hran_threshold_bpm_zero_handled() {
    use beatoraja_core::pattern::mode_modifier::ModeModifier;
    use beatoraja_core::pattern::pattern_modifier::PatternModifier;
    use beatoraja_types::player_config::PlayerConfig;

    let config = PlayerConfig {
        hran_threshold_bpm: 0,
        ..Default::default()
    };

    // ModeModifier::modify() checks `if self.config.hran_threshold_bpm <= 0`
    // and sets self.hran_threshold = 0. No division occurs. This should not panic.
    let mut modifier = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.set_all_time_line(vec![TimeLine::new(0.0, 0, 9)]);

    // Should not panic — the <= 0 branch avoids the division
    modifier.modify(&mut model);
}

/// ModeModifier with negative hran_threshold_bpm also sets threshold to 0 (no panic).
#[test]
fn mode_modifier_hran_threshold_bpm_negative_handled() {
    use beatoraja_core::pattern::mode_modifier::ModeModifier;
    use beatoraja_core::pattern::pattern_modifier::PatternModifier;
    use beatoraja_types::player_config::PlayerConfig;

    let config = PlayerConfig {
        hran_threshold_bpm: -100,
        ..Default::default()
    };

    let mut modifier = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.set_all_time_line(vec![TimeLine::new(0.0, 0, 9)]);

    // Should not panic
    modifier.modify(&mut model);
}

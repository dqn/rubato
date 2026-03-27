// Phase 50c: Out-of-bounds access in get_random_pattern_impl via scratch_key[player]
//
// In get_random_pattern_impl (lane_shuffle_modifier.rs line 28):
//   repr[keys as usize - 1] = scratch_key[player as usize];
//
// For BEAT_7K, scratch_key = &[7] (length 1).
// If player=1, scratch_key[1] panics with index out of bounds.
//
// The guard in keys_static prevents player >= mode.player() from reaching modify(),
// but random_pattern() on a manually-constructed modifier with player=1 calls
// get_random_pattern_impl directly, triggering the OOB.

use bms::model::mode::Mode;
use rubato_game::core::pattern::lane_shuffle_modifier::LaneMirrorShuffleModifier;

/// Constructing a LaneMirrorShuffleModifier with player=1 for a single-player
/// mode (BEAT_7K) should not panic even though the player index is invalid.
#[test]
fn scratch_key_oob_with_invalid_player_index_returns_zeroed_pattern() {
    // BEAT_7K: player() == 1, scratch_key() == &[7] (length 1)
    // new(1, false) creates modifier with player=1, is_scratch_lane_modify=false
    let mut modifier = LaneMirrorShuffleModifier::new(1, false);
    // Enable show_shuffle_pattern so get_random_pattern_impl enters the scratch_key branch
    modifier.show_shuffle_pattern = true;

    let pattern = modifier.random_pattern(&Mode::BEAT_7K);
    assert_eq!(pattern, vec![0; Mode::BEAT_7K.key() as usize]);
}

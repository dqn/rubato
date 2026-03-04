// Phase 50c: Out-of-bounds access in get_random_pattern_impl via scratch_key[player]
//
// In get_random_pattern_impl (lane_shuffle_modifier.rs line 28):
//   repr[keys as usize - 1] = scratch_key[player as usize];
//
// For BEAT_7K, scratch_key = &[7] (length 1).
// If player=1, scratch_key[1] panics with index out of bounds.
//
// The guard in get_keys_static prevents player >= mode.player() from reaching modify(),
// but get_random_pattern() on a manually-constructed modifier with player=1 calls
// get_random_pattern_impl directly, triggering the OOB.

use beatoraja_core::pattern::lane_shuffle_modifier::LaneMirrorShuffleModifier;
use bms_model::mode::Mode;

/// Constructing a LaneMirrorShuffleModifier with player=1 for a single-player mode (BEAT_7K)
/// and calling get_random_pattern() causes an index-out-of-bounds panic at
/// scratch_key[player as usize] because scratch_key for BEAT_7K has only 1 element.
#[test]
#[should_panic]
fn scratch_key_oob_with_invalid_player_index() {
    // BEAT_7K: player() == 1, scratch_key() == &[7] (length 1)
    // new(1, false) creates modifier with player=1, is_scratch_lane_modify=false
    let mut modifier = LaneMirrorShuffleModifier::new(1, false);
    // Enable show_shuffle_pattern so get_random_pattern_impl enters the scratch_key branch
    modifier.show_shuffle_pattern = true;

    // get_random_pattern() calls get_random_pattern_impl with player=1, mode=BEAT_7K.
    // Inside: scratch_key = &[7] (len 1), scratch_key[1] -> index out of bounds!
    let _pattern = modifier.get_random_pattern(&Mode::BEAT_7K);
}

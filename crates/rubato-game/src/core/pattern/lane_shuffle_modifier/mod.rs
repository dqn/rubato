use crate::core::pattern::java_random::JavaRandom;
use crate::core::pattern::pattern_modifier::{AssistLevel, PatternModifier, PatternModifierBase};
use bms::model::bms_model::BMSModel;
use bms::model::mode::Mode;
use bms::model::note::Note;
use rubato_types::random_history::{RandomHistoryEntry, add_random_history};

fn get_random_pattern_impl(
    random: &[i32],
    show_shuffle_pattern: bool,
    is_scratch_lane_modify: bool,
    player: i32,
    mode: &Mode,
) -> Vec<i32> {
    let player_count = mode.player();
    if player_count <= 0 {
        return Vec::new();
    }
    let keys = mode.key() / player_count;
    if keys <= 0 {
        return Vec::new();
    }
    let mut repr = vec![0i32; keys as usize];
    if player < 0 || player >= player_count {
        return repr;
    }
    if show_shuffle_pattern {
        let scratch_key = mode.scratch_key();
        if !scratch_key.is_empty() && !is_scratch_lane_modify {
            // BEAT-*K
            let src_start = (keys * player) as usize;
            let copy_len = (keys - 1) as usize;
            if src_start + copy_len <= random.len() {
                repr[..copy_len].copy_from_slice(&random[src_start..src_start + copy_len]);
            }
            if let Some(&scratch_lane) = scratch_key.get(player as usize) {
                repr[keys as usize - 1] = scratch_lane;
            }
        } else {
            let src_start = (keys * player) as usize;
            let copy_len = keys as usize;
            if src_start + copy_len <= random.len() {
                repr[..copy_len].copy_from_slice(&random[src_start..src_start + copy_len]);
            }
        }
    }
    repr
}

fn lane_shuffle_modify(
    base: &mut PatternModifierBase,
    model: &mut BMSModel,
    is_scratch_lane_modify: bool,
    _show_shuffle_pattern: bool,
    make_random: impl FnOnce(&[i32], &BMSModel, i64) -> Vec<i32>,
) -> Vec<i32> {
    let mode = match model.mode() {
        Some(m) => m,
        None => return Vec::new(),
    };
    let keys = PatternModifierBase::keys_static(mode, base.player, is_scratch_lane_modify);
    let lanes = mode.key() as usize;
    if keys.is_empty() {
        return Vec::new();
    }
    let random = make_random(&keys, model, base.seed);

    // Random Trainer History
    if random.len() == 8 {
        let mut random_sb = String::new();
        for &r in &random[..random.len() - 1] {
            random_sb.push_str(&(r + 1).to_string());
        }
        add_random_history(RandomHistoryEntry::new(model.title.clone(), random_sb));
    }

    let timelines = &mut model.timelines;
    for index in 0..timelines.len() {
        let tl = &timelines[index];
        if tl.exist_note() || tl.exist_hidden_note() {
            // Take all notes out of the timeline (move, not clone)
            let mut notes: Vec<Option<Note>> = Vec::with_capacity(lanes);
            let mut hnotes: Vec<Option<Note>> = Vec::with_capacity(lanes);
            for i in 0..lanes {
                notes.push(timelines[index].take_note(i as i32));
                hnotes.push(timelines[index].take_hidden_note(i as i32));
            }
            // Track which source lanes have already been consumed (moved)
            let mut consumed: Vec<bool> = vec![false; lanes];
            for i in 0..lanes {
                let m = if i < random.len() && random[i] >= 0 && (random[i] as usize) < lanes {
                    random[i] as usize
                } else {
                    i
                };
                if consumed[m] {
                    // Source already moved; must clone for duplicate mapping (e.g. Battle)
                    if let Some(ref note) = notes[m] {
                        if note.is_long() && note.is_end() {
                            if let Some(pair_tl_idx) = note.pair()
                                && pair_tl_idx < timelines.len()
                                && let Some(ln_start) =
                                    timelines[pair_tl_idx].note(i as i32).cloned()
                                && ln_start.is_long()
                            {
                                timelines[index].set_note(i as i32, Some(note.clone()));
                            }
                        } else {
                            timelines[index].set_note(i as i32, Some(note.clone()));
                        }
                    } else {
                        timelines[index].set_note(i as i32, None);
                    }
                    if let Some(ref hn) = hnotes[m] {
                        timelines[index].set_hidden_note(i as i32, Some(hn.clone()));
                    } else {
                        timelines[index].set_hidden_note(i as i32, None);
                    }
                } else {
                    // First use of this source lane: move instead of clone
                    timelines[index].set_note(i as i32, notes[m].take());
                    timelines[index].set_hidden_note(i as i32, hnotes[m].take());
                    consumed[m] = true;
                }
            }
        }
    }

    random
}

impl PatternModifierBase {
    pub fn keys_static(mode: &Mode, player: i32, contains_scratch: bool) -> Vec<i32> {
        if player >= mode.player() {
            return Vec::new();
        }
        let startkey = mode.key() * player / mode.player();
        (startkey..startkey + mode.key() / mode.player())
            .filter(|&i| contains_scratch || !mode.is_scratch_key(i))
            .collect()
    }
}

// ---- LaneMirrorShuffleModifier ----

pub struct LaneMirrorShuffleModifier {
    pub base: PatternModifierBase,
    pub is_scratch_lane_modify: bool,
    pub show_shuffle_pattern: bool,
    random: Vec<i32>,
}

impl LaneMirrorShuffleModifier {
    pub fn new(player: i32, is_scratch_lane_modify: bool) -> Self {
        let mut base = PatternModifierBase::with_player(player);
        base.assist = if is_scratch_lane_modify {
            AssistLevel::LightAssist
        } else {
            AssistLevel::None
        };
        LaneMirrorShuffleModifier {
            base,
            is_scratch_lane_modify,
            show_shuffle_pattern: false,
            random: Vec::new(),
        }
    }

    pub fn make_random(keys: &[i32], model: &BMSModel, _seed: i64) -> Vec<i32> {
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);
        let mut result: Vec<i32> = (0..mode_key).collect();
        for (i, &key) in keys.iter().enumerate() {
            result[key as usize] = keys[keys.len() - 1 - i];
        }
        result
    }

    pub fn is_to_display(&self) -> bool {
        self.show_shuffle_pattern
    }

    pub fn random_pattern(&self, mode: &Mode) -> Vec<i32> {
        get_random_pattern_impl(
            &self.random,
            self.show_shuffle_pattern,
            self.is_scratch_lane_modify,
            self.base.player,
            mode,
        )
    }
}

impl PatternModifier for LaneMirrorShuffleModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        self.random = lane_shuffle_modify(
            &mut self.base,
            model,
            self.is_scratch_lane_modify,
            false,
            Self::make_random,
        );
    }

    fn assist_level(&self) -> AssistLevel {
        self.base.assist
    }

    fn set_assist_level(&mut self, assist: AssistLevel) {
        self.base.assist = assist;
    }

    fn get_seed(&self) -> i64 {
        self.base.seed
    }

    fn set_seed(&mut self, seed: i64) {
        if seed >= 0 {
            self.base.seed = seed;
        }
    }

    fn player(&self) -> i32 {
        self.base.player
    }

    fn is_lane_shuffle_to_display(&self) -> bool {
        self.show_shuffle_pattern
    }

    fn get_lane_shuffle_random_pattern(&self, mode: &Mode) -> Option<Vec<i32>> {
        Some(self.random_pattern(mode))
    }
}

// ---- LaneRotateShuffleModifier ----

pub struct LaneRotateShuffleModifier {
    pub base: PatternModifierBase,
    pub is_scratch_lane_modify: bool,
    pub show_shuffle_pattern: bool,
    random: Vec<i32>,
}

impl LaneRotateShuffleModifier {
    pub fn new(player: i32, is_scratch_lane_modify: bool) -> Self {
        let mut base = PatternModifierBase::with_player(player);
        base.assist = if is_scratch_lane_modify {
            AssistLevel::LightAssist
        } else {
            AssistLevel::None
        };
        LaneRotateShuffleModifier {
            base,
            is_scratch_lane_modify,
            show_shuffle_pattern: true,
            random: Vec::new(),
        }
    }

    pub fn make_random(keys: &[i32], model: &BMSModel, seed: i64) -> Vec<i32> {
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);
        let mut result: Vec<i32> = (0..mode_key).collect();
        if keys.len() <= 1 {
            return result;
        }
        let mut rand = JavaRandom::new(seed);
        let inc = rand.next_int_bounded(2) == 1;
        let start = rand.next_int_bounded(keys.len() as i32 - 1) as usize + if inc { 1 } else { 0 };
        let mut rlane = start;
        for &key in keys {
            result[key as usize] = keys[rlane];
            if inc {
                rlane = (rlane + 1) % keys.len();
            } else {
                rlane = (rlane + keys.len() - 1) % keys.len();
            }
        }
        result
    }

    pub fn is_to_display(&self) -> bool {
        self.show_shuffle_pattern
    }

    pub fn random_pattern(&self, mode: &Mode) -> Vec<i32> {
        get_random_pattern_impl(
            &self.random,
            self.show_shuffle_pattern,
            self.is_scratch_lane_modify,
            self.base.player,
            mode,
        )
    }
}

impl PatternModifier for LaneRotateShuffleModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        self.random = lane_shuffle_modify(
            &mut self.base,
            model,
            self.is_scratch_lane_modify,
            true,
            Self::make_random,
        );
    }

    fn assist_level(&self) -> AssistLevel {
        self.base.assist
    }

    fn set_assist_level(&mut self, assist: AssistLevel) {
        self.base.assist = assist;
    }

    fn get_seed(&self) -> i64 {
        self.base.seed
    }

    fn set_seed(&mut self, seed: i64) {
        if seed >= 0 {
            self.base.seed = seed;
        }
    }

    fn player(&self) -> i32 {
        self.base.player
    }

    fn is_lane_shuffle_to_display(&self) -> bool {
        self.show_shuffle_pattern
    }

    fn get_lane_shuffle_random_pattern(&self, mode: &Mode) -> Option<Vec<i32>> {
        Some(self.random_pattern(mode))
    }
}

// ---- LaneRandomShuffleModifier ----

pub struct LaneRandomShuffleModifier {
    pub base: PatternModifierBase,
    pub is_scratch_lane_modify: bool,
    pub show_shuffle_pattern: bool,
    random: Vec<i32>,
}

impl LaneRandomShuffleModifier {
    pub fn new(player: i32, is_scratch_lane_modify: bool) -> Self {
        let mut base = PatternModifierBase::with_player(player);
        base.assist = if is_scratch_lane_modify {
            AssistLevel::LightAssist
        } else {
            AssistLevel::None
        };
        LaneRandomShuffleModifier {
            base,
            is_scratch_lane_modify,
            show_shuffle_pattern: true,
            random: Vec::new(),
        }
    }

    pub fn make_random(keys: &[i32], model: &BMSModel, seed: i64) -> Vec<i32> {
        let mut rand = JavaRandom::new(seed);
        let mut l: Vec<i32> = keys.to_vec();
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);
        let mut result: Vec<i32> = (0..mode_key).collect();
        for &key in keys {
            let r = rand.next_int_bounded(l.len() as i32) as usize;
            result[key as usize] = l[r];
            l.remove(r);
        }
        result
    }

    pub fn is_to_display(&self) -> bool {
        self.show_shuffle_pattern
    }

    pub fn random_pattern(&self, mode: &Mode) -> Vec<i32> {
        get_random_pattern_impl(
            &self.random,
            self.show_shuffle_pattern,
            self.is_scratch_lane_modify,
            self.base.player,
            mode,
        )
    }
}

impl PatternModifier for LaneRandomShuffleModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        self.random = lane_shuffle_modify(
            &mut self.base,
            model,
            self.is_scratch_lane_modify,
            true,
            Self::make_random,
        );
    }

    fn assist_level(&self) -> AssistLevel {
        self.base.assist
    }

    fn set_assist_level(&mut self, assist: AssistLevel) {
        self.base.assist = assist;
    }

    fn get_seed(&self) -> i64 {
        self.base.seed
    }

    fn set_seed(&mut self, seed: i64) {
        if seed >= 0 {
            self.base.seed = seed;
        }
    }

    fn player(&self) -> i32 {
        self.base.player
    }

    fn is_lane_shuffle_to_display(&self) -> bool {
        self.show_shuffle_pattern
    }

    fn get_lane_shuffle_random_pattern(&self, mode: &Mode) -> Option<Vec<i32>> {
        Some(self.random_pattern(mode))
    }
}

mod advanced;
pub use advanced::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pattern::pattern_modifier::make_test_model;
    use bms::model::bms_model::BMSModel;
    use bms::model::mode::Mode;
    use bms::model::note::Note;
    use bms::model::time_line::TimeLine;

    // -- PatternModifierBase::keys_static --

    #[test]
    fn get_keys_static_beat7k_with_scratch() {
        let keys = PatternModifierBase::keys_static(&Mode::BEAT_7K, 0, true);
        assert_eq!(keys, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn get_keys_static_beat7k_without_scratch() {
        let keys = PatternModifierBase::keys_static(&Mode::BEAT_7K, 0, false);
        // Scratch key for BEAT_7K is 7
        assert_eq!(keys, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn get_keys_static_popn9k() {
        let keys = PatternModifierBase::keys_static(&Mode::POPN_9K, 0, false);
        // No scratch keys in POPN_9K
        assert_eq!(keys, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn get_keys_static_invalid_player() {
        let keys = PatternModifierBase::keys_static(&Mode::BEAT_7K, 1, false);
        assert!(keys.is_empty());
    }

    #[test]
    fn get_keys_static_beat14k_player0() {
        let keys = PatternModifierBase::keys_static(&Mode::BEAT_14K, 0, false);
        // Player 0: keys 0..8, scratch at 7
        assert_eq!(keys, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn get_keys_static_beat14k_player1() {
        let keys = PatternModifierBase::keys_static(&Mode::BEAT_14K, 1, false);
        // Player 1: keys 8..16, scratch at 15
        assert_eq!(keys, vec![8, 9, 10, 11, 12, 13, 14]);
    }

    // -- LaneMirrorShuffleModifier --

    #[test]
    fn mirror_modifier_creation() {
        let modifier = LaneMirrorShuffleModifier::new(0, false);
        assert_eq!(modifier.assist_level(), AssistLevel::None);
        assert_eq!(modifier.player(), 0);
    }

    #[test]
    fn mirror_modifier_with_scratch_is_light_assist() {
        let modifier = LaneMirrorShuffleModifier::new(0, true);
        assert_eq!(modifier.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn mirror_make_random_reverses_keys() {
        // For BEAT_7K without scratch: keys = [0,1,2,3,4,5,6]
        // Mirror should reverse: result[0]=6, result[1]=5, ..., result[6]=0
        let mut model = BMSModel::new();
        model.timelines = vec![TimeLine::new(0.0, 0, 8)];
        model.set_mode(Mode::BEAT_7K);

        let keys = PatternModifierBase::keys_static(&Mode::BEAT_7K, 0, false);
        let result = LaneMirrorShuffleModifier::make_random(&keys, &model, 0);

        // result should be [6, 5, 4, 3, 2, 1, 0, 7]
        // (scratch lane 7 stays at position 7)
        assert_eq!(result, vec![6, 5, 4, 3, 2, 1, 0, 7]);
    }

    #[test]
    fn mirror_make_random_with_scratch() {
        let mut model = BMSModel::new();
        model.timelines = vec![TimeLine::new(0.0, 0, 8)];
        model.set_mode(Mode::BEAT_7K);

        let keys = PatternModifierBase::keys_static(&Mode::BEAT_7K, 0, true);
        let result = LaneMirrorShuffleModifier::make_random(&keys, &model, 0);

        // All 8 keys reversed: [7, 6, 5, 4, 3, 2, 1, 0]
        assert_eq!(result, vec![7, 6, 5, 4, 3, 2, 1, 0]);
    }

    #[test]
    fn mirror_modifier_modifies_model() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(10)));
        tl.set_note(6, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = LaneMirrorShuffleModifier::new(0, false);
        modifier.modify(&mut model);

        let tls = model.timelines;
        // Lane 0 mirrored to lane 6, lane 6 mirrored to lane 0
        assert_eq!(tls[0].note(6).unwrap().wav(), 10);
        assert_eq!(tls[0].note(0).unwrap().wav(), 20);
    }

    // -- LaneMirrorShuffleModifier set_seed --

    #[test]
    fn mirror_modifier_set_seed_negative_ignored() {
        let mut modifier = LaneMirrorShuffleModifier::new(0, false);
        let original = modifier.get_seed();
        modifier.set_seed(-5);
        assert_eq!(modifier.get_seed(), original);
    }

    #[test]
    fn mirror_modifier_set_seed_zero() {
        let mut modifier = LaneMirrorShuffleModifier::new(0, false);
        modifier.set_seed(0);
        assert_eq!(modifier.get_seed(), 0);
    }

    // -- LaneRandomShuffleModifier --

    #[test]
    fn random_modifier_creation() {
        let modifier = LaneRandomShuffleModifier::new(0, false);
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn random_modifier_deterministic_with_same_seed() {
        let mode = Mode::BEAT_7K;
        let seed: i64 = 42;

        let make_model = || {
            let tl = TimeLine::new(0.0, 0, 8);
            make_test_model(&mode, vec![tl])
        };

        let keys = PatternModifierBase::keys_static(&mode, 0, false);
        let result1 = LaneRandomShuffleModifier::make_random(&keys, &make_model(), seed);
        let result2 = LaneRandomShuffleModifier::make_random(&keys, &make_model(), seed);
        assert_eq!(result1, result2);
    }

    #[test]
    fn random_modifier_is_valid_permutation() {
        let mode = Mode::BEAT_7K;
        let model = make_test_model(&mode, vec![TimeLine::new(0.0, 0, 8)]);

        let keys = PatternModifierBase::keys_static(&mode, 0, false);
        let result = LaneRandomShuffleModifier::make_random(&keys, &model, 42);

        // result should have 8 elements (mode_key)
        assert_eq!(result.len(), 8);
        // Each key in keys should appear exactly once in result[keys]
        let mut mapped: Vec<i32> = keys.iter().map(|&k| result[k as usize]).collect();
        mapped.sort();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(mapped, sorted_keys);
        // Scratch lane 7 should stay at 7
        assert_eq!(result[7], 7);
    }

    // -- LaneRotateShuffleModifier --

    #[test]
    fn rotate_modifier_creation() {
        let modifier = LaneRotateShuffleModifier::new(0, false);
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn rotate_modifier_deterministic_with_same_seed() {
        let mode = Mode::BEAT_7K;
        let seed: i64 = 123;
        let model = make_test_model(&mode, vec![TimeLine::new(0.0, 0, 8)]);

        let keys = PatternModifierBase::keys_static(&mode, 0, false);
        let result1 = LaneRotateShuffleModifier::make_random(&keys, &model, seed);
        let result2 = LaneRotateShuffleModifier::make_random(&keys, &model, seed);
        assert_eq!(result1, result2);
    }

    #[test]
    fn rotate_modifier_is_valid_permutation() {
        let mode = Mode::BEAT_7K;
        let model = make_test_model(&mode, vec![TimeLine::new(0.0, 0, 8)]);

        let keys = PatternModifierBase::keys_static(&mode, 0, false);
        let result = LaneRotateShuffleModifier::make_random(&keys, &model, 99);

        assert_eq!(result.len(), 8);
        let mut mapped: Vec<i32> = keys.iter().map(|&k| result[k as usize]).collect();
        mapped.sort();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(mapped, sorted_keys);
    }

    // -- LaneCrossShuffleModifier --

    #[test]
    fn cross_modifier_creation() {
        let modifier = LaneCrossShuffleModifier::new(0, false);
        assert_eq!(modifier.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn cross_make_random_swaps_pairs() {
        // For keys [0,1,2,3,4,5,6] (BEAT_7K without scratch):
        // i=0: swap(0,1) and swap(6,5) -> result[0]=1, result[1]=0, result[6]=5, result[5]=6
        // i=2: 2 < 7/2-1=2.5, but while condition is i < keys.len()/2-1 = 2
        //   so i=2 is not < 2, loop ends
        let mode = Mode::BEAT_7K;
        let model = make_test_model(&mode, vec![TimeLine::new(0.0, 0, 8)]);

        let keys = PatternModifierBase::keys_static(&mode, 0, false);
        let result = LaneCrossShuffleModifier::make_random(&keys, &model, 0);

        assert_eq!(result.len(), 8);
        // First pair swapped
        assert_eq!(result[0], 1);
        assert_eq!(result[1], 0);
        // Last pair swapped
        assert_eq!(result[5], 6);
        assert_eq!(result[6], 5);
        // Middle key (3) unchanged
        assert_eq!(result[3], 3);
        // Scratch unchanged
        assert_eq!(result[7], 7);
    }

    // -- PlayerFlipModifier --

    #[test]
    fn flip_modifier_creation() {
        let modifier = PlayerFlipModifier::new();
        assert_eq!(modifier.assist_level(), AssistLevel::None);
        assert_eq!(modifier.player(), 0);
    }

    #[test]
    fn flip_make_random_single_player_no_change() {
        let mode = Mode::BEAT_7K;
        let model = make_test_model(&mode, vec![TimeLine::new(0.0, 0, 8)]);

        let result = PlayerFlipModifier::make_random(&[], &model, 0);
        // Single player mode: no flip, identity
        assert_eq!(result, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn flip_make_random_double_player_swaps_halves() {
        let mode = Mode::BEAT_14K;
        let model = make_test_model(&mode, vec![TimeLine::new(0.0, 0, 16)]);

        let result = PlayerFlipModifier::make_random(&[], &model, 0);
        // Double player: first half <-> second half
        assert_eq!(
            result,
            vec![8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7]
        );
    }

    // -- PlayerBattleModifier --

    #[test]
    fn battle_modifier_creation() {
        let modifier = PlayerBattleModifier::new();
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
        assert_eq!(modifier.player(), 0);
    }

    #[test]
    fn battle_make_random_single_player_returns_empty() {
        let mode = Mode::BEAT_7K;
        let model = make_test_model(&mode, vec![TimeLine::new(0.0, 0, 8)]);

        let keys = PatternModifierBase::keys_static(&mode, 0, true);
        let (result, assist) = PlayerBattleModifier::make_random(&keys, &model, 0);
        // Single player: returns empty
        assert!(result.is_empty());
        assert_eq!(assist, AssistLevel::Assist);
    }

    #[test]
    fn battle_make_random_double_player_duplicates_keys() {
        let mode = Mode::BEAT_14K;
        let model = make_test_model(&mode, vec![TimeLine::new(0.0, 0, 16)]);

        let keys = PatternModifierBase::keys_static(&mode, 0, true);
        let (result, _) = PlayerBattleModifier::make_random(&keys, &model, 0);
        // Should duplicate keys: [keys, keys]
        assert_eq!(result.len(), keys.len() * 2);
        assert_eq!(&result[..keys.len()], &keys[..]);
        assert_eq!(&result[keys.len()..], &keys[..]);
    }

    // -- Bounds safety regression tests --

    #[test]
    fn negative_random_value_falls_back_to_identity() {
        // If a make_random callback somehow returns negative values,
        // lane_shuffle_modify must not panic from wrapping i32 -> usize.
        // We test this by directly calling lane_shuffle_modify with a
        // callback that returns negative values.
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(42)));
        let mut model = make_test_model(&mode, vec![tl]);

        let mut base = PatternModifierBase::with_player(0);
        let _random = lane_shuffle_modify(
            &mut base,
            &mut model,
            false,
            false,
            |_keys, _model, _seed| vec![-1, -2, -3, -4, -5, -6, -7, -8],
        );

        // Should not panic; negative values fall back to identity mapping.
        // Lane 0 should keep its original note (identity fallback).
        assert_eq!(model.timelines[0].note(0).unwrap().wav(), 42);
    }

    #[test]
    fn out_of_range_random_value_falls_back_to_identity() {
        // If random values exceed lane count, they must not cause
        // out-of-bounds panics on notes/hnotes/consumed vectors.
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(99)));
        let mut model = make_test_model(&mode, vec![tl]);

        let mut base = PatternModifierBase::with_player(0);
        let _random = lane_shuffle_modify(
            &mut base,
            &mut model,
            false,
            false,
            |_keys, _model, _seed| vec![100, 200, 300, 400, 500, 600, 700, 800],
        );

        // All values out of range -> all fall back to identity.
        assert_eq!(model.timelines[0].note(0).unwrap().wav(), 99);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::core::pattern::pattern_modifier::make_test_model;
    use bms::model::mode::Mode;
    use bms::model::time_line::TimeLine;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        #[test]
        fn random_shuffle_is_valid_permutation(seed: i64) {
            let mode = Mode::BEAT_7K;
            let model = make_test_model(&mode, vec![TimeLine::new(0.0, 0, 8)]);

            let keys = PatternModifierBase::keys_static(&mode, 0, false);
            let result = LaneRandomShuffleModifier::make_random(&keys, &model, seed);

            // result should have 8 elements (mode_key for BEAT_7K)
            prop_assert_eq!(result.len(), 8);

            // The key positions should be a permutation of the input keys
            let mut mapped: Vec<i32> = keys.iter().map(|&k| result[k as usize]).collect();
            mapped.sort();
            let mut sorted_keys = keys.clone();
            sorted_keys.sort();
            prop_assert_eq!(mapped, sorted_keys);

            // Scratch lane 7 should remain at position 7 (not in keys, so untouched)
            prop_assert_eq!(result[7], 7);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn murioshi_results_are_permutations(
            raw_patterns in proptest::collection::hash_set(0..=511i32, 0..=10)
        ) {
            let keys: Vec<i32> = (0..9).collect();
            let combinations = search_for_no_murioshi_lane_combinations(&raw_patterns, &keys);

            for combo in &combinations {
                prop_assert_eq!(combo.len(), 9, "combination length should be 9");
                let mut sorted = combo.clone();
                sorted.sort();
                prop_assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
                    "combination {:?} is not a valid permutation of 0..9", combo);
            }
        }
    }
}

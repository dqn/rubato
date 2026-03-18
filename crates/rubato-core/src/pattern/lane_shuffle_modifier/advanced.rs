use std::collections::HashSet;

use crate::pattern::java_random::JavaRandom;
use crate::pattern::pattern_modifier::{AssistLevel, PatternModifier, PatternModifierBase};
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;

use super::{get_random_pattern_impl, lane_shuffle_modify};

// ---- PlayerFlipModifier ----

pub struct PlayerFlipModifier {
    pub base: PatternModifierBase,
    pub show_shuffle_pattern: bool,
    random: Vec<i32>,
}

impl Default for PlayerFlipModifier {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerFlipModifier {
    pub fn new() -> Self {
        let mut base = PatternModifierBase::with_player(0);
        base.assist = AssistLevel::None;
        PlayerFlipModifier {
            base,
            show_shuffle_pattern: false,
            random: Vec::new(),
        }
    }

    pub fn make_random(_keys: &[i32], model: &BMSModel, _seed: i64) -> Vec<i32> {
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0) as usize;
        let mut result: Vec<i32> = (0..mode_key as i32).collect();
        if model.mode().map(|m| m.player()).unwrap_or(0) == 2 {
            let len = result.len();
            let half = len / 2;
            for (i, slot) in result.iter_mut().enumerate() {
                *slot = ((i + half) % len) as i32;
            }
        }
        result
    }

    pub fn is_to_display(&self) -> bool {
        self.show_shuffle_pattern
    }

    pub fn random_pattern(&self, mode: &Mode) -> Vec<i32> {
        // Java: super(0, true, false) -> isScratchLaneModify = true
        get_random_pattern_impl(
            &self.random,
            self.show_shuffle_pattern,
            true,
            self.base.player,
            mode,
        )
    }
}

impl PatternModifier for PlayerFlipModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        self.random = lane_shuffle_modify(&mut self.base, model, true, false, Self::make_random);
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

// ---- PlayerBattleModifier ----

pub struct PlayerBattleModifier {
    pub base: PatternModifierBase,
    pub show_shuffle_pattern: bool,
    random: Vec<i32>,
}

impl Default for PlayerBattleModifier {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerBattleModifier {
    pub fn new() -> Self {
        let mut base = PatternModifierBase::with_player(0);
        base.assist = AssistLevel::Assist;
        PlayerBattleModifier {
            base,
            show_shuffle_pattern: false,
            random: Vec::new(),
        }
    }

    pub fn make_random(keys: &[i32], model: &BMSModel, _seed: i64) -> (Vec<i32>, AssistLevel) {
        if model.mode().map(|m| m.player()).unwrap_or(0) == 1 {
            (Vec::new(), AssistLevel::Assist)
        } else {
            let mut result = vec![0i32; keys.len() * 2];
            result[..keys.len()].copy_from_slice(keys);
            result[keys.len()..keys.len() * 2].copy_from_slice(keys);
            (result, AssistLevel::Assist)
        }
    }

    pub fn is_to_display(&self) -> bool {
        self.show_shuffle_pattern
    }

    pub fn random_pattern(&self, mode: &Mode) -> Vec<i32> {
        // Java: super(0, true, false) -> isScratchLaneModify = true
        get_random_pattern_impl(
            &self.random,
            self.show_shuffle_pattern,
            true,
            self.base.player,
            mode,
        )
    }
}

impl PatternModifier for PlayerBattleModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        let mode = match model.mode() {
            Some(m) => m,
            None => return,
        };
        let keys = PatternModifierBase::keys_static(mode, self.base.player, true);
        let lanes = mode.key() as usize;
        if keys.is_empty() {
            return;
        }
        let (random, assist) = Self::make_random(&keys, model, self.base.seed);
        self.base.assist = assist;
        if random.is_empty() {
            return;
        }

        let timelines = &mut model.timelines;
        for tl in timelines.iter_mut() {
            if tl.exist_note() || tl.exist_hidden_note() {
                // Take all notes out of the timeline (move, not clone)
                let mut notes: Vec<Option<Note>> = Vec::with_capacity(lanes);
                let mut hnotes: Vec<Option<Note>> = Vec::with_capacity(lanes);
                for i in 0..lanes {
                    notes.push(tl.take_note(i as i32));
                    hnotes.push(tl.take_hidden_note(i as i32));
                }
                let mut consumed: Vec<bool> = vec![false; lanes];
                for i in 0..lanes {
                    let m = if i < random.len() {
                        random[i] as usize
                    } else {
                        i
                    };
                    if consumed[m] {
                        if let Some(ref note) = notes[m] {
                            tl.set_note(i as i32, Some(note.clone()));
                        } else {
                            tl.set_note(i as i32, None);
                        }
                        if let Some(ref hn) = hnotes[m] {
                            tl.set_hidden_note(i as i32, Some(hn.clone()));
                        } else {
                            tl.set_hidden_note(i as i32, None);
                        }
                    } else {
                        tl.set_note(i as i32, notes[m].take());
                        tl.set_hidden_note(i as i32, hnotes[m].take());
                        consumed[m] = true;
                    }
                }
            }
        }
        self.random = random;
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

// ---- LaneCrossShuffleModifier ----

pub struct LaneCrossShuffleModifier {
    pub base: PatternModifierBase,
    pub is_scratch_lane_modify: bool,
    pub show_shuffle_pattern: bool,
    random: Vec<i32>,
}

impl LaneCrossShuffleModifier {
    pub fn new(player: i32, is_scratch_lane_modify: bool) -> Self {
        let mut base = PatternModifierBase::with_player(player);
        base.assist = AssistLevel::LightAssist;
        LaneCrossShuffleModifier {
            base,
            is_scratch_lane_modify,
            show_shuffle_pattern: true,
            random: Vec::new(),
        }
    }

    pub fn make_random(keys: &[i32], model: &BMSModel, _seed: i64) -> Vec<i32> {
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);
        let mut result: Vec<i32> = (0..mode_key).collect();
        let limit = keys.len() / 2;
        if limit == 0 {
            return result;
        }
        let mut i = 0;
        while i < limit.saturating_sub(1) {
            result[keys[i] as usize] = keys[i + 1];
            result[keys[i + 1] as usize] = keys[i];
            result[keys[keys.len() - i - 1] as usize] = keys[keys.len() - i - 2];
            result[keys[keys.len() - i - 2] as usize] = keys[keys.len() - i - 1];
            i += 2;
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

impl PatternModifier for LaneCrossShuffleModifier {
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

// ---- LanePlayableRandomShuffleModifier ----

pub struct LanePlayableRandomShuffleModifier {
    pub base: PatternModifierBase,
    pub is_scratch_lane_modify: bool,
    pub show_shuffle_pattern: bool,
    random: Vec<i32>,
}

impl LanePlayableRandomShuffleModifier {
    pub fn new(player: i32, is_scratch_lane_modify: bool) -> Self {
        let mut base = PatternModifierBase::with_player(player);
        base.assist = AssistLevel::LightAssist;
        LanePlayableRandomShuffleModifier {
            base,
            is_scratch_lane_modify,
            show_shuffle_pattern: true,
            random: Vec::new(),
        }
    }

    pub fn make_random(keys: &[i32], model: &BMSModel, seed: i64) -> Vec<i32> {
        let mode = match model.mode() {
            Some(m) => m,
            None => return Vec::new(),
        };
        let lanes = mode.key() as usize;
        let mut ln = vec![-1i32; lanes];
        let mut end_ln_note_time = vec![-1i64; lanes];
        let mut max = 0;
        for key in keys {
            max = max.max(*key);
        }
        let mut is_impossible = false;
        let mut original_pattern_list: HashSet<i32> = HashSet::new();

        // Build list of 3+ simultaneous press patterns
        for tl in &model.timelines {
            if tl.exist_note() {
                // LN
                for i in 0..lanes {
                    if let Some(n) = tl.note(i as i32)
                        && n.is_long()
                    {
                        if n.is_end() && tl.time() == end_ln_note_time[i] {
                            ln[i] = -1;
                            end_ln_note_time[i] = -1;
                        } else {
                            ln[i] = i as i32;
                            if !n.is_end() {
                                // Get pair time
                                end_ln_note_time[i] = n.time() as i64;
                            }
                        }
                    }
                }
                // Normal notes
                let mut note_lane: Vec<i32> = Vec::new();
                for (i, &ln_val) in ln.iter().enumerate() {
                    if let Some(n) = tl.note(i as i32) {
                        if n.is_normal() || ln_val != -1 {
                            note_lane.push(i as i32);
                        }
                    } else if ln_val != -1 {
                        note_lane.push(i as i32);
                    }
                }
                if note_lane.len() >= 7 {
                    is_impossible = true;
                    break;
                } else if note_lane.len() >= 3 {
                    let mut pattern = 0i32;
                    for &i in &note_lane {
                        pattern += (2f64).powi(i) as i32;
                    }
                    original_pattern_list.insert(pattern);
                }
            }
        }

        let mut kouho_pattern_list: Vec<Vec<i32>> = Vec::new();
        if !is_impossible {
            kouho_pattern_list =
                search_for_no_murioshi_lane_combinations(&original_pattern_list, keys);
        }

        log::info!("No-murioshi pattern count: {}", kouho_pattern_list.len());

        let mut rng = JavaRandom::new(seed);
        let mut result = vec![0i32; 9];
        if !kouho_pattern_list.is_empty() {
            let r = (rng.next_double() * kouho_pattern_list.len() as f64) as usize;
            for (i, &kouho) in kouho_pattern_list[r].iter().enumerate().take(9) {
                result[kouho as usize] = i as i32;
            }
        } else {
            let mirror = (rng.next_double() * 2.0) as i32;
            for (i, slot) in result.iter_mut().enumerate().take(9) {
                *slot = if mirror == 0 { i as i32 } else { 8 - i as i32 };
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

pub fn search_for_no_murioshi_lane_combinations(
    original_pattern_list: &HashSet<i32>,
    _keys: &[i32],
) -> Vec<Vec<i32>> {
    let mut no_murioshi_lane_combinations: Vec<Vec<i32>> = Vec::new();
    let mut indexes = [0usize; 9];
    let mut lane_numbers: [i32; 9] = std::array::from_fn(|i| i as i32);

    let murioshi_chords: Vec<Vec<i32>> = vec![
        vec![1, 4, 7],
        vec![1, 4, 8],
        vec![1, 4, 9],
        vec![1, 5, 8],
        vec![1, 5, 9],
        vec![1, 6, 9],
        vec![2, 5, 8],
        vec![2, 5, 9],
        vec![2, 6, 9],
        vec![3, 6, 9],
    ];

    let mut i = 0usize;
    while i < 9 {
        if indexes[i] < i {
            let swap_idx = if i.is_multiple_of(2) { 0 } else { indexes[i] };
            lane_numbers.swap(swap_idx, i);

            let mut murioshi_flag = false;
            for pattern in original_pattern_list {
                let mut temp_pattern: Vec<i32> = Vec::new();
                for (j, &lane_num) in lane_numbers.iter().enumerate().take(9) {
                    if ((pattern / (2f64).powi(j as i32) as i32) % 2) == 1 {
                        temp_pattern.push(lane_num + 1);
                    }
                }

                murioshi_flag = murioshi_chords
                    .iter()
                    .any(|chord| chord.iter().all(|c| temp_pattern.contains(c)));
                if murioshi_flag {
                    break;
                }
            }
            if !murioshi_flag {
                let random_combination: Vec<i32> = lane_numbers.to_vec();
                no_murioshi_lane_combinations.push(random_combination);
            }

            indexes[i] += 1;
            i = 0;
        } else {
            indexes[i] = 0;
            i += 1;
        }
    }

    let mirror_pattern: Vec<i32> = vec![8, 7, 6, 5, 4, 3, 2, 1, 0];
    no_murioshi_lane_combinations.retain(|p| *p != mirror_pattern);
    no_murioshi_lane_combinations
}

impl PatternModifier for LanePlayableRandomShuffleModifier {
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

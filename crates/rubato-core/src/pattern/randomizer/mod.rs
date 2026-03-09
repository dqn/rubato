use std::collections::HashMap;

use crate::pattern::java_random::JavaRandom;
use crate::pattern::pattern_modifier::AssistLevel;
use crate::pattern::random::Random;
use crate::player_config::PlayerConfig;
use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;

pub struct RandomizerBase {
    pub mode: Option<Mode>,
    pub modify_lanes: Vec<i32>,
    pub random: JavaRandom,
    ln_active: HashMap<i32, i32>,
    changeable_lane: Vec<i32>,
    assignable_lane: Vec<i32>,
    pub assist: AssistLevel,
}

impl Default for RandomizerBase {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomizerBase {
    pub fn new() -> Self {
        let seed = (rand::random::<f64>() * 65536.0 * 65536.0 * 65536.0) as i64;
        RandomizerBase {
            mode: None,
            modify_lanes: Vec::new(),
            random: JavaRandom::new(seed),
            ln_active: HashMap::new(),
            changeable_lane: Vec::new(),
            assignable_lane: Vec::new(),
            assist: AssistLevel::None,
        }
    }

    pub fn set_modify_lanes(&mut self, lanes: &[i32]) {
        self.changeable_lane.clear();
        self.assignable_lane.clear();
        for &lane in lanes {
            self.changeable_lane.push(lane);
            self.assignable_lane.push(lane);
        }
        self.modify_lanes = lanes.to_vec();
    }

    pub fn set_mode(&mut self, m: Mode) {
        self.mode = Some(m);
    }

    pub fn ln_lane(&self) -> Vec<i32> {
        self.ln_active.values().copied().collect()
    }

    pub fn assist_level(&self) -> AssistLevel {
        self.assist
    }
    pub fn set_random_seed(&mut self, seed: i64) {
        if seed >= 0 {
            self.random = JavaRandom::new(seed);
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn permutate(
        &mut self,
        tl: &mut TimeLine,
        randomize_fn: &mut dyn FnMut(
            &mut TimeLine,
            &mut Vec<i32>,
            &mut Vec<i32>,
            &mut JavaRandom,
        ) -> HashMap<i32, i32>,
    ) -> Vec<i32> {
        let mut changeable = self.changeable_lane.clone();
        let mut assignable = self.assignable_lane.clone();
        let mut permutation_map =
            randomize_fn(tl, &mut changeable, &mut assignable, &mut self.random);

        // LN active lane assignment
        for (&k, &v) in &self.ln_active {
            permutation_map.insert(k, v);
        }

        let mode_key = self.mode.as_ref().map(|m| m.key()).unwrap_or(0) as usize;
        let mut permutation: Vec<i32> = (0..mode_key as i32).collect();

        let mut notes: Vec<Option<Note>> = vec![None; mode_key];
        let mut hnotes: Vec<Option<Note>> = vec![None; mode_key];
        for &lane in &self.modify_lanes {
            notes[lane as usize] = tl.note(lane).cloned();
            hnotes[lane as usize] = tl.hidden_note(lane).cloned();
        }

        for (&x, &y) in &permutation_map {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end() && self.ln_active.contains_key(&x) && tl.time() == note.time() {
                    self.ln_active.remove(&x);
                    self.changeable_lane.push(x);
                    self.assignable_lane.push(y);
                } else if !note.is_end() {
                    self.ln_active.insert(x, y);
                    self.changeable_lane.retain(|&v| v != x);
                    self.assignable_lane.retain(|&v| v != y);
                }
            }
            tl.set_note(y, n);
            tl.set_hidden_note(y, hn);

            permutation[y as usize] = x;
        }
        permutation
    }
}

// ---- TimeBasedRandomizer ----

pub struct TimeBasedRandomizerState {
    pub threshold: i32,
    pub last_note_time: HashMap<i32, i32>,
}

impl TimeBasedRandomizerState {
    pub fn new(threshold: i32) -> Self {
        TimeBasedRandomizerState {
            threshold,
            last_note_time: HashMap::new(),
        }
    }

    pub fn init_lanes(&mut self, lanes: &[i32]) {
        for &lane in lanes {
            self.last_note_time.insert(lane, -10000);
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn time_based_shuffle(
        &self,
        tl: &TimeLine,
        changeable_lane: &mut Vec<i32>,
        assignable_lane: &mut Vec<i32>,
        random: &mut JavaRandom,
        select_lane: &mut dyn FnMut(&[i32], &mut JavaRandom) -> usize,
    ) -> HashMap<i32, i32> {
        let mut random_map: HashMap<i32, i32> = HashMap::new();
        let mut note_lane: Vec<i32> = Vec::with_capacity(changeable_lane.len());
        let mut empty_lane: Vec<i32> = Vec::with_capacity(changeable_lane.len());
        let mut primary_lane: Vec<i32> = Vec::with_capacity(assignable_lane.len());
        let mut inferior_lane: Vec<i32> = Vec::with_capacity(assignable_lane.len());

        for &cl in changeable_lane.iter() {
            let note = tl.note(cl);
            if note.is_none() || note.map(|n| n.is_mine()).unwrap_or(false) {
                empty_lane.push(cl);
            } else {
                note_lane.push(cl);
            }
        }
        for &al in assignable_lane.iter() {
            if tl.time() - *self.last_note_time.get(&al).unwrap_or(&-10000) > self.threshold {
                primary_lane.push(al);
            } else {
                inferior_lane.push(al);
            }
        }

        // Place notes in lanes that won't cause rapid repeats
        while !note_lane.is_empty() && !primary_lane.is_empty() {
            let r = select_lane(&primary_lane, random);
            let note = note_lane.remove(0);
            let assigned = primary_lane.remove(r);
            random_map.insert(note, assigned);
        }

        // If note_lane is not empty, use inferior lanes sorted by last note time
        while !note_lane.is_empty() {
            let min = inferior_lane
                .iter()
                .map(|l| *self.last_note_time.get(l).unwrap_or(&-10000))
                .min()
                .unwrap_or(-10000);
            let min_lane: Vec<i32> = inferior_lane
                .iter()
                .filter(|&&l| *self.last_note_time.get(&l).unwrap_or(&-10000) == min)
                .copied()
                .collect();
            let m = min_lane[random.next_int_bounded(min_lane.len() as i32) as usize];
            let note = note_lane.remove(0);
            random_map.insert(note, m);
            inferior_lane.retain(|&v| v != m);
        }

        // Place remaining lanes randomly
        primary_lane.extend(inferior_lane);
        while !empty_lane.is_empty() {
            let r = random.next_int_bounded(primary_lane.len() as i32) as usize;
            let empty = empty_lane.remove(0);
            let assigned = primary_lane.remove(r);
            random_map.insert(empty, assigned);
        }

        random_map
    }

    pub fn update_note_time(&mut self, tl: &TimeLine, random_map: &HashMap<i32, i32>) {
        for (&key, &val) in random_map {
            let note = tl.note(key);
            if note.is_some() && !note.map(|n| n.is_mine()).unwrap_or(false) {
                self.last_note_time.insert(val, tl.time());
            }
        }
    }
}

// ---- Randomizer enum ----

pub enum Randomizer {
    SRandom(SRandomizer),
    Spiral(SpiralRandomizer),
    AllScratch(AllScratchRandomizer),
    NoMurioshi(NoMurioshiRandomizer),
    Converge(ConvergeRandomizer),
}

impl Randomizer {
    pub fn create(r: Random, mode: &Mode, config: &PlayerConfig) -> Self {
        Self::create_with_side(r, 0, mode, config)
    }

    pub fn create_with_side(r: Random, play_side: i32, mode: &Mode, config: &PlayerConfig) -> Self {
        let threshold_bpm = config.play_settings.hran_threshold_bpm;
        let threshold_millis;
        if threshold_bpm > 0 {
            threshold_millis = (15000.0f32 / threshold_bpm as f32).ceil() as i32;
        } else if threshold_bpm == 0 {
            threshold_millis = 0;
        } else {
            threshold_millis = DEFAULT_HRAN_THRESHOLD;
        };

        let mut randomizer = match r {
            Random::AllScr => Randomizer::AllScratch(AllScratchRandomizer::new(
                SRAN_THRESHOLD,
                threshold_millis,
                play_side,
            )),
            Random::Converge => Randomizer::Converge(ConvergeRandomizer::new(
                threshold_millis,
                threshold_millis * 2,
            )),
            Random::HRandom => {
                Randomizer::SRandom(SRandomizer::new(threshold_millis, AssistLevel::LightAssist))
            }
            Random::Spiral => Randomizer::Spiral(SpiralRandomizer::new()),
            Random::SRandom => {
                Randomizer::SRandom(SRandomizer::new(SRAN_THRESHOLD, AssistLevel::None))
            }
            Random::SRandomNoThreshold => {
                Randomizer::SRandom(SRandomizer::new(0, AssistLevel::None))
            }
            Random::SRandomEx => {
                Randomizer::SRandom(SRandomizer::new(SRAN_THRESHOLD, AssistLevel::LightAssist))
            }
            Random::SRandomPlayable => {
                Randomizer::NoMurioshi(NoMurioshiRandomizer::new(threshold_millis))
            }
            other => {
                log::warn!(
                    "Unhandled Random variant {:?} for Randomizer, using SRandom as fallback",
                    other
                );
                Randomizer::SRandom(SRandomizer::new(SRAN_THRESHOLD, AssistLevel::None))
            }
        };

        randomizer.set_mode(*mode);
        randomizer
    }

    pub fn base(&self) -> &RandomizerBase {
        match self {
            Randomizer::SRandom(r) => &r.base,
            Randomizer::Spiral(r) => &r.base,
            Randomizer::AllScratch(r) => &r.base,
            Randomizer::NoMurioshi(r) => &r.base,
            Randomizer::Converge(r) => &r.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut RandomizerBase {
        match self {
            Randomizer::SRandom(r) => &mut r.base,
            Randomizer::Spiral(r) => &mut r.base,
            Randomizer::AllScratch(r) => &mut r.base,
            Randomizer::NoMurioshi(r) => &mut r.base,
            Randomizer::Converge(r) => &mut r.base,
        }
    }

    /// Returns a reference to the inner `SRandomizer` if this is the `SRandom` variant,
    /// or `None` otherwise.
    pub fn as_srandom(&self) -> Option<&SRandomizer> {
        match self {
            Randomizer::SRandom(r) => Some(r),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner `SRandomizer` if this is the `SRandom` variant,
    /// or `None` otherwise.
    pub fn as_srandom_mut(&mut self) -> Option<&mut SRandomizer> {
        match self {
            Randomizer::SRandom(r) => Some(r),
            _ => None,
        }
    }

    pub fn set_mode(&mut self, m: Mode) {
        match self {
            Randomizer::AllScratch(r) => r.set_mode(m),
            _ => self.base_mut().mode = Some(m),
        }
    }

    pub fn set_modify_lanes(&mut self, lanes: &[i32]) {
        match self {
            Randomizer::SRandom(r) => {
                r.base.set_modify_lanes(lanes);
                r.time_state.init_lanes(lanes);
            }
            Randomizer::Spiral(r) => {
                r.base.set_modify_lanes(lanes);
                r.increment = r
                    .base
                    .random
                    .next_int_bounded((lanes.len().max(1) + 1) as i32)
                    as usize;
                if r.increment == 0 && !lanes.is_empty() {
                    r.increment = 1;
                } else {
                    let upper = lanes.len().max(1);
                    r.increment = if upper > 1 {
                        r.base.random.next_int_bounded((upper - 1) as i32) as usize + 1
                    } else {
                        1
                    };
                }
                r.head = 0;
                r.cycle = lanes.len();
            }
            Randomizer::AllScratch(r) => {
                r.base.set_modify_lanes(lanes);
                r.time_state.init_lanes(lanes);
            }
            Randomizer::NoMurioshi(r) => {
                r.base.set_modify_lanes(lanes);
                r.time_state.init_lanes(lanes);
            }
            Randomizer::Converge(r) => {
                r.base.set_modify_lanes(lanes);
                r.time_state.init_lanes(lanes);
                for &lane in lanes {
                    r.renda_count.insert(lane, 0);
                }
            }
        }
    }

    pub fn set_random_seed(&mut self, seed: i64) {
        self.base_mut().set_random_seed(seed);
    }

    pub fn assist_level(&self) -> AssistLevel {
        self.base().assist_level()
    }

    pub fn permutate(&mut self, tl: &mut TimeLine) -> Vec<i32> {
        match self {
            Randomizer::SRandom(r) => r.permutate(tl),
            Randomizer::Spiral(r) => r.permutate(tl),
            Randomizer::AllScratch(r) => r.permutate(tl),
            Randomizer::NoMurioshi(r) => r.permutate(tl),
            Randomizer::Converge(r) => r.permutate(tl),
        }
    }
}

pub const SRAN_THRESHOLD: i32 = 40;
pub const DEFAULT_HRAN_THRESHOLD: i32 = 100;

mod specialized;
pub use specialized::*;

#[cfg(test)]
mod tests {
    use super::*;

    // -- RandomizerBase --

    #[test]
    fn randomizer_base_default_values() {
        let base = RandomizerBase::new();
        assert!(base.mode.is_none());
        assert!(base.modify_lanes.is_empty());
        assert_eq!(base.assist_level(), AssistLevel::None);
    }

    #[test]
    fn randomizer_base_default_trait() {
        let base = RandomizerBase::default();
        assert!(base.mode.is_none());
    }

    #[test]
    fn randomizer_base_set_mode() {
        let mut base = RandomizerBase::new();
        base.mode = Some(Mode::BEAT_7K);
        assert_eq!(base.mode, Some(Mode::BEAT_7K));
    }

    #[test]
    fn randomizer_base_set_modify_lanes() {
        let mut base = RandomizerBase::new();
        base.set_modify_lanes(&[0, 1, 2, 3]);
        assert_eq!(base.modify_lanes, vec![0, 1, 2, 3]);
    }

    #[test]
    fn randomizer_base_get_ln_lane_initially_empty() {
        let base = RandomizerBase::new();
        assert!(base.ln_lane().is_empty());
    }

    #[test]
    fn randomizer_base_set_assist_level() {
        let mut base = RandomizerBase::new();
        base.assist = AssistLevel::Assist;
        assert_eq!(base.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn randomizer_base_set_random_seed_positive() {
        let mut base1 = RandomizerBase::new();
        let mut base2 = RandomizerBase::new();
        base1.set_random_seed(42);
        base2.set_random_seed(42);
        // After setting same seed, both should produce the same sequence
        let v1 = base1.random.next_int_bounded(1000);
        let v2 = base2.random.next_int_bounded(1000);
        assert_eq!(v1, v2);
    }

    #[test]
    fn randomizer_base_set_random_seed_negative_ignored() {
        let mut base = RandomizerBase::new();
        // Seed with a known value first
        base.set_random_seed(99);
        let val_before = base.random.next_int_bounded(1000);
        // Re-seed to same known value
        base.set_random_seed(99);
        // Negative seed should be ignored
        base.set_random_seed(-1);
        let val_after = base.random.next_int_bounded(1000);
        assert_eq!(val_before, val_after);
    }

    // -- TimeBasedRandomizerState --

    #[test]
    fn time_based_state_creation() {
        let state = TimeBasedRandomizerState::new(100);
        assert_eq!(state.threshold, 100);
        assert!(state.last_note_time.is_empty());
    }

    #[test]
    fn time_based_state_init_lanes() {
        let mut state = TimeBasedRandomizerState::new(100);
        state.init_lanes(&[0, 1, 2]);
        assert_eq!(state.last_note_time.len(), 3);
        assert_eq!(*state.last_note_time.get(&0).unwrap(), -10000);
        assert_eq!(*state.last_note_time.get(&1).unwrap(), -10000);
        assert_eq!(*state.last_note_time.get(&2).unwrap(), -10000);
    }

    // -- Constants --

    #[test]
    fn sran_threshold_value() {
        assert_eq!(SRAN_THRESHOLD, 40);
    }

    #[test]
    fn default_hran_threshold_value() {
        assert_eq!(DEFAULT_HRAN_THRESHOLD, 100);
    }

    // -- Randomizer enum --

    #[test]
    fn randomizer_create_srandom() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::SRandom, &Mode::BEAT_7K, &config);
        assert!(matches!(r, Randomizer::SRandom(_)));
    }

    #[test]
    fn randomizer_create_spiral() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::Spiral, &Mode::BEAT_7K, &config);
        assert!(matches!(r, Randomizer::Spiral(_)));
    }

    #[test]
    fn randomizer_create_allscr() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::AllScr, &Mode::BEAT_7K, &config);
        assert!(matches!(r, Randomizer::AllScratch(_)));
    }

    #[test]
    fn randomizer_create_hrandom() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::HRandom, &Mode::BEAT_7K, &config);
        assert!(matches!(r, Randomizer::SRandom(_)));
    }

    #[test]
    fn randomizer_create_converge() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::Converge, &Mode::BEAT_7K, &config);
        assert!(matches!(r, Randomizer::Converge(_)));
    }

    #[test]
    fn randomizer_create_srandom_playable() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::SRandomPlayable, &Mode::BEAT_7K, &config);
        assert!(matches!(r, Randomizer::NoMurioshi(_)));
    }

    #[test]
    fn randomizer_create_srandom_no_threshold() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::SRandomNoThreshold, &Mode::BEAT_7K, &config);
        assert!(matches!(r, Randomizer::SRandom(_)));
    }

    #[test]
    fn randomizer_create_srandom_ex() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::SRandomEx, &Mode::BEAT_7K, &config);
        assert!(matches!(r, Randomizer::SRandom(_)));
    }

    #[test]
    fn randomizer_set_random_seed() {
        let config = PlayerConfig::default();
        let mut r = Randomizer::create(Random::SRandom, &Mode::BEAT_7K, &config);
        r.set_random_seed(42);
    }

    #[test]
    fn randomizer_set_modify_lanes() {
        let config = PlayerConfig::default();
        let mut r = Randomizer::create(Random::SRandom, &Mode::BEAT_7K, &config);
        r.set_modify_lanes(&[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn randomizer_get_assist_level_srandom() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::SRandom, &Mode::BEAT_7K, &config);
        assert_eq!(r.assist_level(), AssistLevel::None);
    }

    #[test]
    fn randomizer_hrandom_has_light_assist() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::HRandom, &Mode::BEAT_7K, &config);
        assert_eq!(r.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn randomizer_base_accessor() {
        let config = PlayerConfig::default();
        let r = Randomizer::create(Random::SRandom, &Mode::BEAT_7K, &config);
        assert_eq!(r.base().mode, Some(Mode::BEAT_7K));
    }

    #[test]
    fn randomizer_base_mut_accessor() {
        let config = PlayerConfig::default();
        let mut r = Randomizer::create(Random::SRandom, &Mode::BEAT_7K, &config);
        r.base_mut().assist = AssistLevel::Assist;
        assert_eq!(r.assist_level(), AssistLevel::Assist);
    }

    // -- SRandomizer --

    #[test]
    fn srandomizer_creation() {
        let r = SRandomizer::new(40, AssistLevel::None);
        assert_eq!(r.base.assist_level(), AssistLevel::None);
        assert_eq!(r.time_state.threshold, 40);
    }

    // -- SpiralRandomizer --

    #[test]
    fn spiral_randomizer_default() {
        let r = SpiralRandomizer::new();
        assert_eq!(r.base.assist_level(), AssistLevel::LightAssist);
        assert_eq!(r.increment, 0);
        assert_eq!(r.head, 0);
        assert_eq!(r.cycle, 0);
    }

    #[test]
    fn spiral_randomizer_default_trait() {
        let r = SpiralRandomizer::default();
        assert_eq!(r.increment, 0);
    }

    // -- AllScratchRandomizer --

    #[test]
    fn all_scratch_randomizer_creation() {
        let r = AllScratchRandomizer::new(40, 100, 0);
        assert_eq!(r.base.assist_level(), AssistLevel::LightAssist);
        assert_eq!(r.time_state.threshold, 100);
    }

    #[test]
    fn all_scratch_set_mode_single_play() {
        let mut r = AllScratchRandomizer::new(40, 100, 0);
        r.set_mode(Mode::BEAT_7K);
        assert_eq!(r.scratch_lane, vec![7]);
        assert!(!r.is_double_play);
    }

    #[test]
    fn all_scratch_set_mode_double_play_side_0() {
        let mut r = AllScratchRandomizer::new(40, 100, 0);
        r.set_mode(Mode::BEAT_14K);
        // BEAT_14K scratch_key = [7, 15], player=2, side=0 -> half=1, offset=0
        assert_eq!(r.scratch_lane, vec![7]);
        assert!(r.is_double_play);
    }

    #[test]
    fn all_scratch_set_mode_double_play_side_1() {
        let mut r = AllScratchRandomizer::new(40, 100, 1);
        r.set_mode(Mode::BEAT_14K);
        // side=1, half=1, offset=1
        assert_eq!(r.scratch_lane, vec![15]);
        assert!(r.is_double_play);
    }

    // -- ConvergeRandomizer --

    #[test]
    fn converge_randomizer_creation() {
        let r = ConvergeRandomizer::new(100, 200);
        assert_eq!(r.base.assist_level(), AssistLevel::LightAssist);
        assert_eq!(r.time_state.threshold, 100);
    }

    // -- NoMurioshiRandomizer --

    #[test]
    fn no_murioshi_randomizer_creation() {
        let r = NoMurioshiRandomizer::new(100);
        assert_eq!(r.base.assist_level(), AssistLevel::LightAssist);
        assert_eq!(r.time_state.threshold, 100);
    }

    // -- button_combination_table --

    #[test]
    fn button_combination_table_has_10_entries() {
        assert_eq!(button_combination_table().len(), 10);
    }

    #[test]
    fn button_combination_table_entries_have_6_elements() {
        for entry in button_combination_table() {
            assert_eq!(entry.len(), 6);
        }
    }

    #[test]
    fn button_combination_table_values_in_range() {
        for entry in button_combination_table() {
            for &val in entry {
                assert!((0..=8).contains(&val), "Value {} out of range", val);
            }
        }
    }

    #[test]
    fn button_combination_table_is_sorted_per_entry() {
        for entry in button_combination_table() {
            for i in 1..entry.len() {
                assert!(entry[i] > entry[i - 1], "Entry {:?} is not sorted", entry);
            }
        }
    }

    // -- Threshold calculation --

    #[test]
    fn randomizer_with_custom_threshold_bpm() {
        let mut config = PlayerConfig::default();
        config.play_settings.hran_threshold_bpm = 150;
        let r = Randomizer::create(Random::HRandom, &Mode::BEAT_7K, &config);
        // threshold_millis = ceil(15000.0 / 150) = 100
        let sr = r
            .as_srandom()
            .expect("HRandom should produce SRandom variant");
        assert_eq!(sr.time_state.threshold, 100);
    }

    #[test]
    fn randomizer_with_zero_threshold_bpm() {
        let mut config = PlayerConfig::default();
        config.play_settings.hran_threshold_bpm = 0;
        let r = Randomizer::create(Random::HRandom, &Mode::BEAT_7K, &config);
        let sr = r
            .as_srandom()
            .expect("HRandom should produce SRandom variant");
        assert_eq!(sr.time_state.threshold, 0);
    }

    #[test]
    fn randomizer_with_negative_threshold_uses_default() {
        let mut config = PlayerConfig::default();
        config.play_settings.hran_threshold_bpm = -1;
        let r = Randomizer::create(Random::HRandom, &Mode::BEAT_7K, &config);
        let sr = r
            .as_srandom()
            .expect("HRandom should produce SRandom variant");
        assert_eq!(sr.time_state.threshold, DEFAULT_HRAN_THRESHOLD);
    }

    #[test]
    fn as_srandom_returns_none_for_non_srandom_variants() {
        let config = PlayerConfig::default();
        let spiral = Randomizer::create(Random::Spiral, &Mode::BEAT_7K, &config);
        assert!(
            spiral.as_srandom().is_none(),
            "Spiral should not be SRandom"
        );

        let allscr = Randomizer::create(Random::AllScr, &Mode::BEAT_7K, &config);
        assert!(
            allscr.as_srandom().is_none(),
            "AllScr should not be SRandom"
        );

        let converge = Randomizer::create(Random::Converge, &Mode::BEAT_7K, &config);
        assert!(
            converge.as_srandom().is_none(),
            "Converge should not be SRandom"
        );

        let playable = Randomizer::create(Random::SRandomPlayable, &Mode::BEAT_7K, &config);
        assert!(
            playable.as_srandom().is_none(),
            "SRandomPlayable should not be SRandom"
        );
    }

    #[test]
    fn as_srandom_returns_some_for_srandom_variants() {
        let config = PlayerConfig::default();

        let srandom = Randomizer::create(Random::SRandom, &Mode::BEAT_7K, &config);
        assert!(srandom.as_srandom().is_some(), "SRandom should be SRandom");

        let hrandom = Randomizer::create(Random::HRandom, &Mode::BEAT_7K, &config);
        assert!(hrandom.as_srandom().is_some(), "HRandom should be SRandom");

        let srandom_ex = Randomizer::create(Random::SRandomEx, &Mode::BEAT_7K, &config);
        assert!(
            srandom_ex.as_srandom().is_some(),
            "SRandomEx should be SRandom"
        );

        let no_threshold = Randomizer::create(Random::SRandomNoThreshold, &Mode::BEAT_7K, &config);
        assert!(
            no_threshold.as_srandom().is_some(),
            "SRandomNoThreshold should be SRandom"
        );
    }

    #[test]
    fn as_srandom_mut_allows_mutation() {
        let config = PlayerConfig::default();
        let mut r = Randomizer::create(Random::SRandom, &Mode::BEAT_7K, &config);
        if let Some(sr) = r.as_srandom_mut() {
            sr.time_state.threshold = 999;
        }
        let sr = r.as_srandom().expect("should still be SRandom");
        assert_eq!(sr.time_state.threshold, 999);
    }
}

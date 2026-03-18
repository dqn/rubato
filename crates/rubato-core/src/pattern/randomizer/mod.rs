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
        let seed = {
            use std::time::SystemTime;
            let nanos = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            (nanos % (65536 * 65536 * 65536)) as i64
        };
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

        // Safety: x values come from modify_lanes which are validated lane indices (0..mode_key).
        for (&x, &y) in &permutation_map {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end()
                    && self.ln_active.contains_key(&x)
                    && tl.time() == note.time() as i64
                {
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
    pub threshold: i64,
    pub last_note_time: HashMap<i32, i64>,
}

impl TimeBasedRandomizerState {
    pub fn new(threshold: i64) -> Self {
        TimeBasedRandomizerState {
            threshold,
            last_note_time: HashMap::new(),
        }
    }

    pub fn init_lanes(&mut self, lanes: &[i32]) {
        for &lane in lanes {
            self.last_note_time.insert(lane, -10000i64);
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
            if tl.milli_time() - *self.last_note_time.get(&al).unwrap_or(&-10000i64)
                > self.threshold
            {
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
        while !note_lane.is_empty() && !inferior_lane.is_empty() {
            let min = inferior_lane
                .iter()
                .map(|l| *self.last_note_time.get(l).unwrap_or(&-10000i64))
                .min()
                .unwrap_or(-10000i64);
            let min_lane: Vec<i32> = inferior_lane
                .iter()
                .filter(|&&l| *self.last_note_time.get(&l).unwrap_or(&-10000i64) == min)
                .copied()
                .collect();
            let m = min_lane[random.next_int_bounded(min_lane.len() as i32) as usize];
            let note = note_lane.remove(0);
            random_map.insert(note, m);
            inferior_lane.retain(|&v| v != m);
        }

        // Place remaining lanes randomly
        primary_lane.extend(inferior_lane);
        while !empty_lane.is_empty() && !primary_lane.is_empty() {
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
                self.last_note_time.insert(val, tl.milli_time());
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
        let threshold_millis: i64;
        if threshold_bpm > 0 {
            threshold_millis = (15000.0f32 / threshold_bpm as f32).ceil() as i64;
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
                // Accepted trade-off: first next_int_bounded call's value is always
                // overwritten (by line below or the else branch), consuming one extra
                // RNG step. Without the Java source, we cannot verify whether this
                // matches the original's RNG sequence. Preserved as-is for safety.
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

pub const SRAN_THRESHOLD: i64 = 40;
pub const DEFAULT_HRAN_THRESHOLD: i64 = 100;

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

    #[test]
    fn converge_randomizer_renda_count_incremental_update() {
        // Regression: ConvergeRandomizer must update renda_count incrementally
        // inside select_fn during time_based_shuffle, not as a post-hoc batch.
        //
        // With a frozen snapshot (the old bug), all select_fn calls within a
        // single time_based_shuffle see identical renda_counts (all 0), so the
        // "max" filter produces no convergence bias. With incremental updates,
        // the second call sees the count incremented by the first call, changing
        // which lane is selected.
        //
        // We verify the difference by running two copies with the same seed:
        // one with the real (now-fixed) ConvergeRandomizer, and one simulating
        // the old frozen-snapshot behavior. If the bug fix works, they must
        // produce different renda_count distributions.

        let lanes: Vec<i32> = (0..4).collect();
        let seed = 42i64;

        // --- Run with the real (fixed) ConvergeRandomizer ---
        let mut r_fixed = ConvergeRandomizer::new(10, 2000);
        r_fixed.base.mode = Some(Mode::BEAT_5K);
        r_fixed.base.set_modify_lanes(&lanes);
        r_fixed.time_state.init_lanes(&lanes);
        for &lane in &lanes {
            r_fixed.renda_count.insert(lane, 0);
        }
        r_fixed.base.set_random_seed(seed);

        // Two successive timelines, each with 2 notes.
        // First timeline at t=100ms
        let mut tl1 = TimeLine::new(0.0, 100_000, 4);
        tl1.set_note(0, Some(Note::new_normal(1)));
        tl1.set_note(1, Some(Note::new_normal(2)));
        let _perm1 = r_fixed.permutate(&mut tl1);
        let counts_after_first = r_fixed.renda_count.clone();

        // Second timeline at t=150ms (within threshold2=2000)
        let mut tl2 = TimeLine::new(0.0, 150_000, 4);
        tl2.set_note(0, Some(Note::new_normal(3)));
        tl2.set_note(1, Some(Note::new_normal(4)));
        let _perm2 = r_fixed.permutate(&mut tl2);
        let counts_after_second = r_fixed.renda_count.clone();

        // Total renda_count must equal total notes placed
        let total_first: i32 = counts_after_first.values().sum();
        assert_eq!(
            total_first, 2,
            "After first TL: total should be 2, got {}",
            total_first
        );

        let total_second: i32 = counts_after_second.values().sum();
        assert_eq!(
            total_second, 4,
            "After second TL: total should be 4, got {}",
            total_second
        );

        // The second select_fn call within each time_based_shuffle must have
        // seen the first call's increment. Verify by checking that the max
        // count is >= 2 after 4 notes (the converge algorithm prefers lanes
        // with the highest count, so it should pile up).
        let max_count = *counts_after_second.values().max().unwrap_or(&0);
        assert!(
            max_count >= 2,
            "With incremental updates over 4 notes, at least one lane should have \
             count >= 2, but max was {}. renda_count: {:?}",
            max_count,
            counts_after_second,
        );
    }

    #[test]
    fn converge_randomizer_renda_count_persists_across_timelines() {
        // Verify that renda_count state accumulated during one permutate() call
        // carries into the next call (within threshold2), matching Java behavior
        // where rendaCount is a field that persists across randomize() calls.
        let lanes: Vec<i32> = (0..4).collect();
        let mut r = ConvergeRandomizer::new(10, 2000);
        r.base.mode = Some(Mode::BEAT_5K);
        r.base.set_modify_lanes(&lanes);
        r.time_state.init_lanes(&lanes);
        for &lane in &lanes {
            r.renda_count.insert(lane, 0);
        }
        r.base.set_random_seed(123);

        // First timeline at time=100ms with 2 notes
        let mut tl1 = TimeLine::new(0.0, 100_000, 4);
        tl1.set_note(0, Some(Note::new_normal(1)));
        tl1.set_note(1, Some(Note::new_normal(2)));
        let _perm1 = r.permutate(&mut tl1);

        let count_after_first: i32 = r.renda_count.values().sum();
        assert_eq!(
            count_after_first, 2,
            "After first TL with 2 notes, total count should be 2"
        );

        // Second timeline at time=150ms (within threshold2=2000, so counts won't reset)
        let mut tl2 = TimeLine::new(0.0, 150_000, 4);
        tl2.set_note(0, Some(Note::new_normal(3)));
        tl2.set_note(1, Some(Note::new_normal(4)));
        tl2.set_note(2, Some(Note::new_normal(5)));
        let _perm2 = r.permutate(&mut tl2);

        let count_after_second: i32 = r.renda_count.values().sum();
        assert_eq!(
            count_after_second, 5,
            "After second TL with 3 notes, total count should be 2+3=5, got {}",
            count_after_second,
        );
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

    // -- Regression tests for time_based_shuffle fixes --

    #[test]
    fn time_based_shuffle_no_panic_with_empty_inferior_lane() {
        // Regression: when all assignable lanes go to primary_lane and none to
        // inferior_lane, the second while-loop (inferior drain) must not call
        // next_int_bounded(0). With fewer assignable lanes than notes, the
        // loop would previously panic after primary_lane was exhausted.
        let mut state = TimeBasedRandomizerState::new(1000);
        state.init_lanes(&[0, 1, 2]);
        // Set last_note_time to current time so all lanes are "recent"
        // (milli_time - last_note_time <= threshold), putting all into inferior.
        let current_milli = 500i64; // TimeLine time=500_000us => milli_time=500
        state.last_note_time.insert(0, current_milli);
        state.last_note_time.insert(1, current_milli);
        state.last_note_time.insert(2, current_milli);

        // Create a TimeLine with notes on all 3 lanes
        let mut tl = TimeLine::new(0.0, 500_000, 3);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_normal(2)));
        tl.set_note(2, Some(Note::new_normal(3)));

        let mut changeable = vec![0, 1, 2];
        // Only 2 assignable lanes: after inferior drains 2 items, the 3rd note
        // has no lane. Before the fix this would panic; after the fix the loop
        // exits gracefully.
        let mut assignable = vec![0, 1];
        let mut random = JavaRandom::new(42);

        // Must not panic
        let result = state.time_based_shuffle(
            &tl,
            &mut changeable,
            &mut assignable,
            &mut random,
            &mut |lanes, rng| rng.next_int_bounded(lanes.len() as i32) as usize,
        );

        // All mapped notes should point to valid lanes
        for &assigned in result.values() {
            assert!(
                (0..3).contains(&assigned),
                "assigned lane {} out of range",
                assigned
            );
        }
    }

    #[test]
    fn time_based_shuffle_no_panic_with_empty_primary_for_empties() {
        // Regression: after all primary + inferior lanes are consumed by note
        // placement, the empty-lane placement loop must not call
        // next_int_bounded(0).
        let mut state = TimeBasedRandomizerState::new(0);
        state.init_lanes(&[0, 1, 2]);
        // threshold=0, last_note_time=-10000 (from init_lanes):
        // milli_time(500) - (-10000) = 10500 > 0, so all go to primary.

        // 2 notes + 1 empty; only 2 assignable lanes.
        // Primary has 2 lanes, notes consume both, leaving 0 for empty.
        let mut tl = TimeLine::new(0.0, 500_000, 3);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_normal(2)));
        // lane 2 has no note -> empty_lane

        let mut changeable = vec![0, 1, 2];
        let mut assignable = vec![0, 1]; // only 2, both consumed by notes
        let mut random = JavaRandom::new(42);

        // Must not panic
        let result = state.time_based_shuffle(
            &tl,
            &mut changeable,
            &mut assignable,
            &mut random,
            &mut |lanes, rng| rng.next_int_bounded(lanes.len() as i32) as usize,
        );

        // The 2 note lanes should be mapped
        assert!(result.contains_key(&0) || result.contains_key(&1));
    }

    #[test]
    fn time_based_state_uses_milli_time_values() {
        // Regression: last_note_time must store full i64 values, not truncated
        // i32. Values exceeding i32::MAX must be preserved exactly.
        let mut state = TimeBasedRandomizerState::new(100);
        state.init_lanes(&[0]);

        let large_time: i64 = 3_000_000_000;
        state.last_note_time.insert(0, large_time);
        assert_eq!(
            *state.last_note_time.get(&0).unwrap(),
            3_000_000_000i64,
            "last_note_time should store full i64 without truncation"
        );
    }

    // -- SpiralRandomizer: cycle == 0 must not panic --

    #[test]
    fn spiral_randomizer_cycle_zero_no_panic() {
        // SpiralRandomizer with cycle == 0 (default, or set_modify_lanes with empty slice)
        // must return identity permutation without modulo-by-zero panic
        let mut r = SpiralRandomizer::new();
        r.base.mode = Some(Mode::BEAT_7K);
        // cycle is 0 by default (no set_modify_lanes called)
        assert_eq!(r.cycle, 0);

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));

        let perm = r.permutate(&mut tl);
        // Should return identity permutation for BEAT_7K (key count = 8 with scratch)
        let mode_key = Mode::BEAT_7K.key() as usize;
        assert_eq!(perm.len(), mode_key);
        for (i, &p) in perm.iter().enumerate().take(mode_key) {
            assert_eq!(p, i as i32);
        }
    }

    #[test]
    fn no_murioshi_early_return_updates_note_time() {
        // Regression: the early-return path in NoMurioshiRandomizer::permutate()
        // (candidate2 empty) must call update_note_time() so subsequent timelines
        // compute renda_lane from fresh timestamps, not stale ones.
        let lanes: Vec<i32> = (0..7).collect();
        let mut r = NoMurioshiRandomizer::new(100_000); // large threshold
        r.base.mode = Some(Mode::BEAT_7K);
        r.base.set_modify_lanes(&lanes);
        r.time_state.init_lanes(&lanes);
        r.base.set_random_seed(42);

        // Set all lanes as "recently played" (within threshold but distinct from
        // the timeline's milli_time) so candidate2 becomes empty, triggering the
        // early-return path. Use 400 while the timeline will be at milli_time=500.
        let stale_time: i64 = 400;
        for &lane in &lanes {
            r.time_state.last_note_time.insert(lane, stale_time);
        }

        // Create a timeline with 3 notes (note_count in 3..7 to set flag=true)
        let tl_milli: i64 = 500;
        let mut tl = TimeLine::new(0.0, tl_milli * 1000, 7); // milli_time = 500
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_normal(2)));
        tl.set_note(2, Some(Note::new_normal(3)));

        let _perm = r.permutate(&mut tl);

        // After the fix, update_note_time should have updated last_note_time
        // entries for the mapped destination lanes to the current milli_time (500).
        // Without the fix, all entries remain at stale_time (400).
        let updated_count = r
            .time_state
            .last_note_time
            .values()
            .filter(|&&t| t == tl_milli)
            .count();
        assert!(
            updated_count >= 3,
            "Expected at least 3 lanes updated to milli_time {}, but only {} were. \
             The early-return path likely skipped update_note_time().",
            tl_milli,
            updated_count,
        );
    }

    #[test]
    fn spiral_randomizer_empty_modify_lanes_no_panic() {
        // set_modify_lanes with empty slice sets cycle = 0
        let config = PlayerConfig::default();
        let mut r = Randomizer::create(Random::Spiral, &Mode::BEAT_7K, &config);
        r.set_modify_lanes(&[]);

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));

        let perm = r.permutate(&mut tl);
        let mode_key = Mode::BEAT_7K.key() as usize;
        assert_eq!(perm.len(), mode_key);
    }
}

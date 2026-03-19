use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;

use super::{RandomizerBase, TimeBasedRandomizerState};
use crate::pattern::java_random::JavaRandom;
use crate::pattern::pattern_modifier::AssistLevel;

// ---- SRandomizer ----

pub struct SRandomizer {
    pub base: RandomizerBase,
    pub time_state: TimeBasedRandomizerState,
}

impl SRandomizer {
    pub fn new(threshold: i64, assist: AssistLevel) -> Self {
        let mut base = RandomizerBase::new();
        base.assist = assist;
        SRandomizer {
            base,
            time_state: TimeBasedRandomizerState::new(threshold),
        }
    }

    pub fn permutate(&mut self, tl: &mut TimeLine) -> Vec<i32> {
        let mut changeable = self.base.changeable_lane.clone();
        let mut assignable = self.base.assignable_lane.clone();
        let random_map = {
            let mut select_fn = |lane: &[i32], rng: &mut JavaRandom| -> usize {
                rng.next_int_bounded(lane.len() as i32) as usize
            };
            self.time_state.time_based_shuffle(
                tl,
                &mut changeable,
                &mut assignable,
                &mut self.base.random,
                &mut select_fn,
            )
        };

        self.time_state.update_note_time(tl, &random_map);

        // Now do the permutation using the random_map
        self.apply_permutation(tl, random_map)
    }

    fn apply_permutation(
        &mut self,
        tl: &mut TimeLine,
        permutation_map: HashMap<i32, i32>,
    ) -> Vec<i32> {
        let mut full_map = permutation_map;
        for (&k, &v) in &self.base.ln_active {
            full_map.insert(k, v);
        }

        let mode_key = self.base.mode.as_ref().map(|m| m.key()).unwrap_or(0) as usize;
        let mut permutation: Vec<i32> = (0..mode_key as i32).collect();

        let mut notes: Vec<Option<Note>> = vec![None; mode_key];
        let mut hnotes: Vec<Option<Note>> = vec![None; mode_key];
        for &lane in &self.base.modify_lanes {
            notes[lane as usize] = tl.note(lane).cloned();
            hnotes[lane as usize] = tl.hidden_note(lane).cloned();
        }

        for (&x, &y) in &full_map {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end()
                    && self.base.ln_active.contains_key(&x)
                    && tl.time() == note.time()
                {
                    self.base.ln_active.remove(&x);
                    self.base.changeable_lane.push(x);
                    self.base.assignable_lane.push(y);
                } else if !note.is_end() {
                    self.base.ln_active.insert(x, y);
                    self.base.changeable_lane.retain(|&v| v != x);
                    self.base.assignable_lane.retain(|&v| v != y);
                }
            }
            tl.set_note(y, n);
            tl.set_hidden_note(y, hn);
            permutation[y as usize] = x;
        }
        permutation
    }
}

// ---- SpiralRandomizer ----

pub struct SpiralRandomizer {
    pub base: RandomizerBase,
    pub increment: usize,
    pub head: usize,
    pub cycle: usize,
}

impl Default for SpiralRandomizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SpiralRandomizer {
    pub fn new() -> Self {
        let mut base = RandomizerBase::new();
        base.assist = AssistLevel::LightAssist;
        SpiralRandomizer {
            base,
            increment: 0,
            head: 0,
            cycle: 0,
        }
    }

    pub fn permutate(&mut self, tl: &mut TimeLine) -> Vec<i32> {
        if self.cycle == 0 {
            let mode_key = self.base.mode.as_ref().map(|m| m.key()).unwrap_or(0) as usize;
            return (0..mode_key as i32).collect();
        }

        let changeable = &self.base.changeable_lane;
        let mut rotate_map: HashMap<i32, i32> = HashMap::new();

        if changeable.len() == self.cycle {
            self.head = (self.head + self.increment) % self.cycle;
            for (i, &lane) in self.base.modify_lanes.iter().enumerate() {
                rotate_map.insert(lane, self.base.modify_lanes[(i + self.head) % self.cycle]);
            }
        } else {
            for (i, &lane) in self.base.modify_lanes.iter().enumerate() {
                if changeable.contains(&lane) {
                    rotate_map.insert(lane, self.base.modify_lanes[(i + self.head) % self.cycle]);
                }
            }
        }

        // Apply permutation
        let mut full_map = rotate_map;
        for (&k, &v) in &self.base.ln_active {
            full_map.insert(k, v);
        }

        let mode_key = self.base.mode.as_ref().map(|m| m.key()).unwrap_or(0) as usize;
        let mut permutation: Vec<i32> = (0..mode_key as i32).collect();

        let mut notes: Vec<Option<Note>> = vec![None; mode_key];
        let mut hnotes: Vec<Option<Note>> = vec![None; mode_key];
        for &lane in &self.base.modify_lanes {
            notes[lane as usize] = tl.note(lane).cloned();
            hnotes[lane as usize] = tl.hidden_note(lane).cloned();
        }

        for (&x, &y) in &full_map {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end()
                    && self.base.ln_active.contains_key(&x)
                    && tl.time() == note.time()
                {
                    self.base.ln_active.remove(&x);
                    self.base.changeable_lane.push(x);
                    self.base.assignable_lane.push(y);
                } else if !note.is_end() {
                    self.base.ln_active.insert(x, y);
                    self.base.changeable_lane.retain(|&v| v != x);
                    self.base.assignable_lane.retain(|&v| v != y);
                }
            }
            tl.set_note(y, n);
            tl.set_hidden_note(y, hn);
            permutation[y as usize] = x;
        }
        permutation
    }
}

// ---- AllScratchRandomizer ----

pub struct AllScratchRandomizer {
    pub base: RandomizerBase,
    pub time_state: TimeBasedRandomizerState,
    scratch_threshold: i64,
    pub(super) scratch_lane: Vec<i32>,
    scratch_index: usize,
    modify_side: i32,
    pub(super) is_double_play: bool,
}

const SIDE_1P: i32 = 0;
const SIDE_2P: i32 = 1;

impl AllScratchRandomizer {
    pub fn new(s: i64, k: i64, modify_side: i32) -> Self {
        let mut base = RandomizerBase::new();
        base.assist = AssistLevel::LightAssist;
        AllScratchRandomizer {
            base,
            time_state: TimeBasedRandomizerState::new(k),
            scratch_threshold: s,
            scratch_lane: Vec::new(),
            scratch_index: 0,
            modify_side,
            is_double_play: false,
        }
    }

    pub fn set_mode(&mut self, m: Mode) {
        self.is_double_play = m.player() == 2;
        if self.is_double_play {
            let sk = m.scratch_key();
            let half = sk.len() / 2;
            let offset = (self.modify_side as usize) * half;
            self.scratch_lane = sk[offset..offset + half].to_vec();
        } else {
            self.scratch_lane = m.scratch_key().to_vec();
        }
        self.base.mode = Some(m);
    }

    pub fn permutate(&mut self, tl: &mut TimeLine) -> Vec<i32> {
        let mut changeable = self.base.changeable_lane.clone();
        let mut assignable = self.base.assignable_lane.clone();
        let mut random_map: HashMap<i32, i32> = HashMap::new();

        // Try to assign to scratch lane first
        if !self.scratch_lane.is_empty()
            && assignable.contains(&self.scratch_lane[self.scratch_index])
            && tl.milli_time()
                - *self
                    .time_state
                    .last_note_time
                    .get(&self.scratch_lane[self.scratch_index])
                    .unwrap_or(&-10000i64)
                > self.scratch_threshold
        {
            let mut l: i32 = -1;
            for &cl in &changeable {
                let note = tl.note(cl);
                if note.is_some() && !note.map(|n| n.is_mine()).unwrap_or(false) {
                    l = cl;
                    break;
                }
            }
            if l != -1 {
                random_map.insert(l, self.scratch_lane[self.scratch_index]);
                changeable.retain(|&v| v != l);
                assignable.retain(|&v| v != self.scratch_lane[self.scratch_index]);
                self.scratch_index += 1;
                if self.scratch_index == self.scratch_lane.len() {
                    self.scratch_index = 0;
                }
            }
        }

        // Assign remaining
        let is_dp = self.is_double_play;
        let modify_side = self.modify_side;
        let mut select_fn = move |lane: &[i32], rng: &mut JavaRandom| -> usize {
            if is_dp {
                let mut index = 0;
                match modify_side {
                    SIDE_1P => {
                        let mut min = i32::MAX;
                        for (i, &val) in lane.iter().enumerate() {
                            if val < min {
                                min = val;
                                index = i;
                            }
                        }
                    }
                    SIDE_2P => {
                        let mut max = i32::MIN;
                        for (i, &val) in lane.iter().enumerate() {
                            if val > max {
                                max = val;
                                index = i;
                            }
                        }
                    }
                    _ => {}
                }
                index
            } else {
                rng.next_int_bounded(lane.len() as i32) as usize
            }
        };

        let remaining = self.time_state.time_based_shuffle(
            tl,
            &mut changeable,
            &mut assignable,
            &mut self.base.random,
            &mut select_fn,
        );
        random_map.extend(remaining);

        self.time_state.update_note_time(tl, &random_map);

        // Apply permutation
        let mut full_map = random_map;
        for (&k, &v) in &self.base.ln_active {
            full_map.insert(k, v);
        }

        let mode_key = self.base.mode.as_ref().map(|m| m.key()).unwrap_or(0) as usize;
        let mut permutation: Vec<i32> = (0..mode_key as i32).collect();

        let mut notes: Vec<Option<Note>> = vec![None; mode_key];
        let mut hnotes: Vec<Option<Note>> = vec![None; mode_key];
        for &lane in &self.base.modify_lanes {
            notes[lane as usize] = tl.note(lane).cloned();
            hnotes[lane as usize] = tl.hidden_note(lane).cloned();
        }

        for (&x, &y) in &full_map {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end()
                    && self.base.ln_active.contains_key(&x)
                    && tl.time() == note.time()
                {
                    self.base.ln_active.remove(&x);
                    self.base.changeable_lane.push(x);
                    self.base.assignable_lane.push(y);
                } else if !note.is_end() {
                    self.base.ln_active.insert(x, y);
                    self.base.changeable_lane.retain(|&v| v != x);
                    self.base.assignable_lane.retain(|&v| v != y);
                }
            }
            tl.set_note(y, n);
            tl.set_hidden_note(y, hn);
            permutation[y as usize] = x;
        }
        permutation
    }
}

// ---- NoMurioshiRandomizer ----

pub struct NoMurioshiRandomizer {
    pub base: RandomizerBase,
    pub time_state: TimeBasedRandomizerState,
    button_combination: Vec<i32>,
    flag: bool,
}

pub(super) fn button_combination_table() -> &'static Vec<Vec<i32>> {
    use std::sync::OnceLock;
    static TABLE: OnceLock<Vec<Vec<i32>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        vec![
            vec![0, 1, 2, 3, 4, 5],
            vec![0, 1, 2, 4, 5, 6],
            vec![0, 1, 2, 5, 6, 7],
            vec![0, 1, 2, 6, 7, 8],
            vec![1, 2, 3, 4, 5, 6],
            vec![1, 2, 3, 5, 6, 7],
            vec![1, 2, 3, 6, 7, 8],
            vec![2, 3, 4, 5, 6, 7],
            vec![2, 3, 4, 6, 7, 8],
            vec![3, 4, 5, 6, 7, 8],
        ]
    })
}

impl NoMurioshiRandomizer {
    pub fn new(threshold: i64) -> Self {
        let mut base = RandomizerBase::new();
        base.assist = AssistLevel::LightAssist;
        NoMurioshiRandomizer {
            base,
            time_state: TimeBasedRandomizerState::new(threshold),
            button_combination: Vec::new(),
            flag: false,
        }
    }

    fn note_count(&self, tl: &TimeLine) -> usize {
        self.get_note_exist_lane(tl).len() + self.base.ln_lane().len()
    }

    fn get_note_exist_lane(&self, tl: &TimeLine) -> Vec<i32> {
        let mut l = Vec::new();
        for &lane in &self.base.modify_lanes {
            let note = tl.note(lane);
            if note.is_some() && !note.map(|n| n.is_mine()).unwrap_or(false) {
                l.push(lane);
            }
        }
        l
    }

    pub fn permutate(&mut self, tl: &mut TimeLine) -> Vec<i32> {
        let note_count = self.note_count(tl);
        let mut changeable = self.base.changeable_lane.clone();
        let mut assignable = self.base.assignable_lane.clone();

        self.flag = 2 < note_count && note_count < 7;
        if self.flag {
            let ln_lane = self.base.ln_lane();
            let candidate: Vec<&Vec<i32>> = if ln_lane.is_empty() {
                button_combination_table().iter().collect()
            } else {
                button_combination_table()
                    .iter()
                    .filter(|l| ln_lane.iter().all(|lnl| l.contains(lnl)))
                    .collect()
            };

            if !candidate.is_empty() {
                let threshold = self.time_state.threshold;
                let renda_lane: Vec<i32> = self
                    .time_state
                    .last_note_time
                    .iter()
                    .filter(|(_lane, time)| tl.milli_time() - **time < threshold)
                    .map(|(&lane, _)| lane)
                    .collect();

                let candidate2: Vec<Vec<i32>> = candidate
                    .iter()
                    .map(|lanes| {
                        lanes
                            .iter()
                            .filter(|&&lane| !renda_lane.contains(&lane))
                            .copied()
                            .collect::<Vec<i32>>()
                    })
                    .filter(|lanes| lanes.len() >= note_count)
                    .collect();

                if !candidate2.is_empty() {
                    self.button_combination = candidate2
                        [self.base.random.next_int_bounded(candidate2.len() as i32) as usize]
                        .clone();
                } else {
                    let mut random_map: HashMap<i32, i32> = HashMap::new();
                    let cand_idx =
                        self.base.random.next_int_bounded(candidate.len() as i32) as usize;
                    self.button_combination = candidate[cand_idx]
                        .iter()
                        .filter(|&&lane| assignable.contains(&lane))
                        .copied()
                        .collect();
                    let note_exist_lane: Vec<i32> = self
                        .get_note_exist_lane(tl)
                        .into_iter()
                        .filter(|lane| changeable.contains(lane))
                        .collect();
                    for lane in &note_exist_lane {
                        if !self.button_combination.is_empty() {
                            let i = self
                                .base
                                .random
                                .next_int_bounded(self.button_combination.len() as i32)
                                as usize;
                            let assigned = self.button_combination[i];
                            random_map.insert(*lane, assigned);
                            changeable.retain(|&v| v != *lane);
                            assignable.retain(|&v| v != assigned);
                            self.button_combination.remove(i);
                        }
                    }
                    self.flag = false;
                    let bc = self.button_combination.clone();
                    let flag = self.flag;
                    let mut select_fn = |lane: &[i32], rng: &mut JavaRandom| -> usize {
                        if flag {
                            let l: Vec<i32> = lane
                                .iter()
                                .filter(|&&la| bc.contains(&la))
                                .copied()
                                .collect();
                            if !l.is_empty() {
                                let chosen = l[rng.next_int_bounded(l.len() as i32) as usize];
                                return lane
                                    .iter()
                                    .position(|&x| x == chosen)
                                    .expect("position found");
                            }
                        }
                        rng.next_int_bounded(lane.len() as i32) as usize
                    };
                    let remaining = self.time_state.time_based_shuffle(
                        tl,
                        &mut changeable,
                        &mut assignable,
                        &mut self.base.random,
                        &mut select_fn,
                    );
                    random_map.extend(remaining);

                    // Update note timestamps before applying permutation
                    self.time_state.update_note_time(tl, &random_map);
                    return self.apply_permutation(tl, random_map);
                }
            } else {
                self.flag = false;
            }
        }

        let bc = self.button_combination.clone();
        let flag = self.flag;
        let mut select_fn = |lane: &[i32], rng: &mut JavaRandom| -> usize {
            if flag {
                let l: Vec<i32> = lane
                    .iter()
                    .filter(|&&la| bc.contains(&la))
                    .copied()
                    .collect();
                if !l.is_empty() {
                    let chosen = l[rng.next_int_bounded(l.len() as i32) as usize];
                    return lane
                        .iter()
                        .position(|&x| x == chosen)
                        .expect("position found");
                }
            }
            rng.next_int_bounded(lane.len() as i32) as usize
        };
        let random_map = self.time_state.time_based_shuffle(
            tl,
            &mut changeable,
            &mut assignable,
            &mut self.base.random,
            &mut select_fn,
        );
        self.time_state.update_note_time(tl, &random_map);

        self.apply_permutation(tl, random_map)
    }

    fn apply_permutation(
        &mut self,
        tl: &mut TimeLine,
        permutation_map: HashMap<i32, i32>,
    ) -> Vec<i32> {
        let mut full_map = permutation_map;
        for (&k, &v) in &self.base.ln_active {
            full_map.insert(k, v);
        }

        let mode_key = self.base.mode.as_ref().map(|m| m.key()).unwrap_or(0) as usize;
        let mut permutation: Vec<i32> = (0..mode_key as i32).collect();

        let mut notes: Vec<Option<Note>> = vec![None; mode_key];
        let mut hnotes: Vec<Option<Note>> = vec![None; mode_key];
        for &lane in &self.base.modify_lanes {
            notes[lane as usize] = tl.note(lane).cloned();
            hnotes[lane as usize] = tl.hidden_note(lane).cloned();
        }

        for (&x, &y) in &full_map {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end()
                    && self.base.ln_active.contains_key(&x)
                    && tl.time() == note.time()
                {
                    self.base.ln_active.remove(&x);
                    self.base.changeable_lane.push(x);
                    self.base.assignable_lane.push(y);
                } else if !note.is_end() {
                    self.base.ln_active.insert(x, y);
                    self.base.changeable_lane.retain(|&v| v != x);
                    self.base.assignable_lane.retain(|&v| v != y);
                }
            }
            tl.set_note(y, n);
            tl.set_hidden_note(y, hn);
            permutation[y as usize] = x;
        }
        permutation
    }
}

// ---- ConvergeRandomizer ----

pub struct ConvergeRandomizer {
    pub base: RandomizerBase,
    pub time_state: TimeBasedRandomizerState,
    threshold2: i64,
    pub(super) renda_count: HashMap<i32, i32>,
}

impl ConvergeRandomizer {
    pub fn new(threshold1: i64, threshold2: i64) -> Self {
        let mut base = RandomizerBase::new();
        base.assist = AssistLevel::LightAssist;
        ConvergeRandomizer {
            base,
            time_state: TimeBasedRandomizerState::new(threshold1),
            threshold2,
            renda_count: HashMap::new(),
        }
    }

    pub fn permutate(&mut self, tl: &mut TimeLine) -> Vec<i32> {
        // Reset renda count for non-renda lanes
        let threshold2 = self.threshold2;
        let time = tl.milli_time();
        for (&key, count) in self.renda_count.iter_mut() {
            if time
                - *self
                    .time_state
                    .last_note_time
                    .get(&key)
                    .unwrap_or(&-10000i64)
                > threshold2
            {
                *count = 0;
            }
        }

        let mut changeable = self.base.changeable_lane.clone();
        let mut assignable = self.base.assignable_lane.clone();

        // Use Rc<RefCell<...>> so the closure can incrementally update renda_count
        // during time_based_shuffle, matching Java's selectLane() behavior where
        // rendaCount.put(l, rendaCount.get(l) + 1) is called inside the method.
        let renda_count_shared = Rc::new(RefCell::new(self.renda_count.clone()));
        let renda_rc = Rc::clone(&renda_count_shared);
        let mut select_fn = move |lane: &[i32], rng: &mut JavaRandom| -> usize {
            let gya = {
                let rc = renda_rc.borrow();
                let max = lane
                    .iter()
                    .map(|l| *rc.get(l).unwrap_or(&0))
                    .max()
                    .unwrap_or(0);
                lane.iter()
                    .filter(|&&l| *rc.get(&l).unwrap_or(&0) == max)
                    .copied()
                    .collect::<Vec<i32>>()
            };
            let l = gya[rng.next_int_bounded(gya.len() as i32) as usize];
            // Increment renda_count for the chosen lane (Java: rendaCount.put(l, rendaCount.get(l) + 1))
            *renda_rc.borrow_mut().entry(l).or_insert(0) += 1;
            lane.iter().position(|&x| x == l).expect("position found")
        };

        let random_map = self.time_state.time_based_shuffle(
            tl,
            &mut changeable,
            &mut assignable,
            &mut self.base.random,
            &mut select_fn,
        );
        self.time_state.update_note_time(tl, &random_map);

        // Copy the incrementally updated renda_count back from the shared state
        drop(select_fn);
        self.renda_count = Rc::try_unwrap(renda_count_shared)
            .expect("sole owner")
            .into_inner();

        // Apply permutation
        let mut full_map = random_map;
        for (&k, &v) in &self.base.ln_active {
            full_map.insert(k, v);
        }

        let mode_key = self.base.mode.as_ref().map(|m| m.key()).unwrap_or(0) as usize;
        let mut permutation: Vec<i32> = (0..mode_key as i32).collect();

        let mut notes: Vec<Option<Note>> = vec![None; mode_key];
        let mut hnotes: Vec<Option<Note>> = vec![None; mode_key];
        for &lane in &self.base.modify_lanes {
            notes[lane as usize] = tl.note(lane).cloned();
            hnotes[lane as usize] = tl.hidden_note(lane).cloned();
        }

        for (&x, &y) in &full_map {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end()
                    && self.base.ln_active.contains_key(&x)
                    && tl.time() == note.time()
                {
                    self.base.ln_active.remove(&x);
                    self.base.changeable_lane.push(x);
                    self.base.assignable_lane.push(y);
                } else if !note.is_end() {
                    self.base.ln_active.insert(x, y);
                    self.base.changeable_lane.retain(|&v| v != x);
                    self.base.assignable_lane.retain(|&v| v != y);
                }
            }
            tl.set_note(y, n);
            tl.set_hidden_note(y, hn);
            permutation[y as usize] = x;
        }
        permutation
    }
}

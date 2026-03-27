use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use bms::model::mode::Mode;
use bms::model::note::Note;
use bms::model::time_line::TimeLine;

use super::{RandomizerBase, TimeBasedRandomizerState};
use crate::core::pattern::java_random::JavaRandom;
use crate::core::pattern::pattern_modifier::AssistLevel;

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

        // Sort by source lane index for deterministic LN tracking state across iterations.
        let mut sorted_entries: Vec<_> = full_map.iter().collect();
        sorted_entries.sort_by_key(|(k, _)| **k);
        for &(&x, &y) in &sorted_entries {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end() && self.base.ln_active.contains_key(&x) && tl.time() == note.time()
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

        // Sort by source lane index for deterministic LN tracking state across iterations.
        let mut sorted_entries: Vec<_> = full_map.iter().collect();
        sorted_entries.sort_by_key(|(k, _)| **k);
        for &(&x, &y) in &sorted_entries {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end() && self.base.ln_active.contains_key(&x) && tl.time() == note.time()
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

        // Sort by source lane index for deterministic LN tracking state across iterations.
        let mut sorted_entries: Vec<_> = full_map.iter().collect();
        sorted_entries.sort_by_key(|(k, _)| **k);
        for &(&x, &y) in &sorted_entries {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end() && self.base.ln_active.contains_key(&x) && tl.time() == note.time()
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

        // Sort by source lane index for deterministic LN tracking state across iterations.
        let mut sorted_entries: Vec<_> = full_map.iter().collect();
        sorted_entries.sort_by_key(|(k, _)| **k);
        for &(&x, &y) in &sorted_entries {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end() && self.base.ln_active.contains_key(&x) && tl.time() == note.time()
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

        // Sort by source lane index for deterministic LN tracking state across iterations.
        let mut sorted_entries: Vec<_> = full_map.iter().collect();
        sorted_entries.sort_by_key(|(k, _)| **k);
        for &(&x, &y) in &sorted_entries {
            let n = notes[x as usize].take();
            let hn = hnotes[x as usize].take();
            if let Some(ref note) = n
                && note.is_long()
            {
                if note.is_end() && self.base.ln_active.contains_key(&x) && tl.time() == note.time()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // ---- Helpers ----

    /// Create a BEAT_7K SRandomizer with deterministic seed and standard key lanes (0..7).
    fn setup_srandomizer(seed: i64, threshold: i64) -> SRandomizer {
        let mut r = SRandomizer::new(threshold, AssistLevel::None);
        r.base.set_mode(Mode::BEAT_7K);
        r.base.set_modify_lanes(&[0, 1, 2, 3, 4, 5, 6]);
        r.time_state.init_lanes(&[0, 1, 2, 3, 4, 5, 6]);
        r.base.set_random_seed(seed);
        r
    }

    /// Create a BEAT_7K SpiralRandomizer with deterministic seed.
    /// Returns the randomizer after set_modify_lanes (which consumes one RNG call for increment).
    fn setup_spiral(seed: i64) -> SpiralRandomizer {
        let lanes: Vec<i32> = (0..7).collect();
        let mut r = SpiralRandomizer::new();
        r.base.set_mode(Mode::BEAT_7K);
        r.base.set_random_seed(seed);
        // set_modify_lanes for Spiral also computes increment from RNG
        r.base.set_modify_lanes(&lanes);
        r.increment = if lanes.len() > 1 {
            r.base.random.next_int_bounded((lanes.len() - 1) as i32) as usize + 1
        } else {
            1
        };
        r.head = 0;
        r.cycle = lanes.len();
        r
    }

    /// Create a BEAT_7K AllScratchRandomizer with deterministic seed.
    fn setup_all_scratch(
        seed: i64,
        scratch_threshold: i64,
        key_threshold: i64,
    ) -> AllScratchRandomizer {
        let mut r = AllScratchRandomizer::new(scratch_threshold, key_threshold, 0);
        r.set_mode(Mode::BEAT_7K);
        r.base.set_modify_lanes(&[0, 1, 2, 3, 4, 5, 6]);
        r.time_state.init_lanes(&[0, 1, 2, 3, 4, 5, 6]);
        r.base.set_random_seed(seed);
        r
    }

    /// Create a BEAT_7K NoMurioshiRandomizer with deterministic seed.
    fn setup_no_murioshi(seed: i64, threshold: i64) -> NoMurioshiRandomizer {
        let mut r = NoMurioshiRandomizer::new(threshold);
        r.base.set_mode(Mode::BEAT_7K);
        r.base.set_modify_lanes(&[0, 1, 2, 3, 4, 5, 6]);
        r.time_state.init_lanes(&[0, 1, 2, 3, 4, 5, 6]);
        r.base.set_random_seed(seed);
        r
    }

    /// Create a ConvergeRandomizer with deterministic seed.
    fn setup_converge(seed: i64, threshold1: i64, threshold2: i64) -> ConvergeRandomizer {
        let lanes: Vec<i32> = (0..7).collect();
        let mut r = ConvergeRandomizer::new(threshold1, threshold2);
        r.base.set_mode(Mode::BEAT_7K);
        r.base.set_modify_lanes(&lanes);
        r.time_state.init_lanes(&lanes);
        for &lane in &lanes {
            r.renda_count.insert(lane, 0);
        }
        r.base.set_random_seed(seed);
        r
    }

    /// Verify a permutation vector is valid: length matches mode key count,
    /// each value is in [0, mode_key), and the set of values mapped from
    /// modify_lanes covers those lanes (bijection on the modify lane set).
    fn assert_valid_permutation(perm: &[i32], mode_key: usize, modify_lanes: &[i32]) {
        assert_eq!(perm.len(), mode_key, "permutation length mismatch");
        for (i, &v) in perm.iter().enumerate() {
            assert!(
                (0..mode_key as i32).contains(&v),
                "permutation[{}] = {} out of range [0, {})",
                i,
                v,
                mode_key
            );
        }
        // The modify lanes should form a bijection: the set of perm[lane] for lane in modify_lanes
        // should be exactly modify_lanes (as a set).
        let mapped: HashSet<i32> = modify_lanes.iter().map(|&l| perm[l as usize]).collect();
        let original: HashSet<i32> = modify_lanes.iter().copied().collect();
        assert_eq!(
            mapped, original,
            "modify lanes should map bijectively: mapped={:?} vs original={:?}",
            mapped, original
        );
    }

    /// Create an LN-start note whose `time` field (in the Note's NoteData) matches the
    /// given microsecond time (for LN end matching: tl.time() == note.time(), both /1000).
    fn new_ln_start(wav: i32, time_us: i64) -> Note {
        let mut n = Note::new_long(wav);
        n.data_mut().time = time_us;
        n
    }

    /// Create an LN-end note at the given microsecond time.
    fn new_ln_end(wav: i32, time_us: i64) -> Note {
        let mut n = Note::new_long(wav);
        n.set_end(true);
        n.data_mut().time = time_us;
        n
    }

    // ================================================================
    // SRandomizer tests
    // ================================================================

    #[test]
    fn srandomizer_produces_valid_permutation_single_timeline() {
        let mut r = setup_srandomizer(42, 40);
        let mut tl = TimeLine::new(0.0, 100_000, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(2, Some(Note::new_normal(2)));
        tl.set_note(4, Some(Note::new_normal(3)));

        let perm = r.permutate(&mut tl);
        assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn srandomizer_deterministic_with_same_seed() {
        let mut r1 = setup_srandomizer(77, 40);
        let mut r2 = setup_srandomizer(77, 40);

        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(1, Some(Note::new_normal(10)));
        tl1.set_note(3, Some(Note::new_normal(20)));

        let mut tl2 = TimeLine::new(0.0, 100_000, 8);
        tl2.set_note(1, Some(Note::new_normal(10)));
        tl2.set_note(3, Some(Note::new_normal(20)));

        let perm1 = r1.permutate(&mut tl1);
        let perm2 = r2.permutate(&mut tl2);
        assert_eq!(perm1, perm2);
    }

    #[test]
    fn srandomizer_different_seeds_differ() {
        let mut r1 = setup_srandomizer(1, 40);
        let mut r2 = setup_srandomizer(999, 40);

        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        for lane in 0..7 {
            tl1.set_note(lane, Some(Note::new_normal(lane + 1)));
        }
        let mut tl2 = TimeLine::new(0.0, 100_000, 8);
        for lane in 0..7 {
            tl2.set_note(lane, Some(Note::new_normal(lane + 1)));
        }

        let perm1 = r1.permutate(&mut tl1);
        let perm2 = r2.permutate(&mut tl2);
        assert_valid_permutation(&perm1, 8, &[0, 1, 2, 3, 4, 5, 6]);
        assert_valid_permutation(&perm2, 8, &[0, 1, 2, 3, 4, 5, 6]);
        // With 7! = 5040 permutations, collision at different seeds is extremely unlikely.
        assert_ne!(
            perm1, perm2,
            "different seeds should produce different permutations"
        );
    }

    #[test]
    fn srandomizer_valid_across_multiple_timelines() {
        let mut r = setup_srandomizer(42, 40);
        for t in 0..5 {
            let time_us = (t + 1) * 100_000;
            let mut tl = TimeLine::new(0.0, time_us, 8);
            tl.set_note(t as i32 % 7, Some(Note::new_normal(t as i32 + 1)));
            let perm = r.permutate(&mut tl);
            assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn srandomizer_with_active_ln_preserves_mapping() {
        let mut r = setup_srandomizer(42, 40);

        // First timeline: LN start on lane 2
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(2, Some(new_ln_start(10, 100_000)));
        tl1.set_note(4, Some(Note::new_normal(20)));
        let perm1 = r.permutate(&mut tl1);
        assert_valid_permutation(&perm1, 8, &[0, 1, 2, 3, 4, 5, 6]);

        assert!(
            r.base.ln_active.contains_key(&2),
            "lane 2 should be in ln_active after LN start"
        );
        let mapped_dest = r.base.ln_active[&2];

        // Second timeline: more notes, but lane 2's LN is still held
        let mut tl2 = TimeLine::new(0.0, 200_000, 8);
        tl2.set_note(0, Some(Note::new_normal(30)));
        tl2.set_note(5, Some(Note::new_normal(40)));
        let perm2 = r.permutate(&mut tl2);
        assert_valid_permutation(&perm2, 8, &[0, 1, 2, 3, 4, 5, 6]);

        // The LN mapping (lane 2 -> mapped_dest) must be preserved
        assert_eq!(
            perm2[mapped_dest as usize], 2,
            "LN mapping must be preserved: perm[{}] should be 2, got {}",
            mapped_dest, perm2[mapped_dest as usize]
        );
    }

    #[test]
    fn srandomizer_ln_end_releases_lane() {
        let mut r = setup_srandomizer(42, 40);

        // LN start on lane 3
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(3, Some(new_ln_start(10, 100_000)));
        let _perm1 = r.permutate(&mut tl1);
        assert!(r.base.ln_active.contains_key(&3), "lane 3 should be active");

        // LN end on lane 3 (note.time() must match tl.time() for release)
        let mut tl2 = TimeLine::new(0.0, 200_000, 8);
        tl2.set_note(3, Some(new_ln_end(10, 200_000)));
        let _perm2 = r.permutate(&mut tl2);
        assert!(
            !r.base.ln_active.contains_key(&3),
            "lane 3 should be released after LN end"
        );
    }

    #[test]
    fn srandomizer_no_notes_produces_valid_permutation() {
        let mut r = setup_srandomizer(42, 40);
        let mut tl = TimeLine::new(0.0, 100_000, 8);
        let perm = r.permutate(&mut tl);
        assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
    }

    // ================================================================
    // SpiralRandomizer tests
    // ================================================================

    #[test]
    fn spiral_produces_valid_permutation() {
        let mut r = setup_spiral(42);
        let mut tl = TimeLine::new(0.0, 100_000, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(3, Some(Note::new_normal(2)));

        let perm = r.permutate(&mut tl);
        assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn spiral_rotation_advances_each_call() {
        let mut r = setup_spiral(42);
        let increment = r.increment;
        assert!(increment > 0, "increment must be > 0");

        let mut heads: Vec<usize> = Vec::new();
        for t in 0..5 {
            let time_us = (t + 1) * 100_000;
            let mut tl = TimeLine::new(0.0, time_us, 8);
            tl.set_note(0, Some(Note::new_normal(t as i32 + 1)));
            let _perm = r.permutate(&mut tl);
            heads.push(r.head);
        }

        // head should advance by `increment` (mod cycle) each call
        for i in 1..heads.len() {
            let expected = (heads[i - 1] + increment) % r.cycle;
            assert_eq!(
                heads[i],
                expected,
                "head[{}] = {} but expected {} (prev={}, increment={}, cycle={})",
                i,
                heads[i],
                expected,
                heads[i - 1],
                increment,
                r.cycle
            );
        }
    }

    #[test]
    fn spiral_deterministic_with_same_seed() {
        let mut r1 = setup_spiral(55);
        let mut r2 = setup_spiral(55);

        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(1, Some(Note::new_normal(10)));
        let mut tl2 = TimeLine::new(0.0, 100_000, 8);
        tl2.set_note(1, Some(Note::new_normal(10)));

        let perm1 = r1.permutate(&mut tl1);
        let perm2 = r2.permutate(&mut tl2);
        assert_eq!(perm1, perm2);
    }

    #[test]
    fn spiral_cycle_zero_returns_identity() {
        let mut r = SpiralRandomizer::new();
        r.base.set_mode(Mode::BEAT_7K);
        r.cycle = 0;
        let mut tl = TimeLine::new(0.0, 100_000, 8);
        tl.set_note(0, Some(Note::new_normal(1)));

        let perm = r.permutate(&mut tl);
        let expected: Vec<i32> = (0..8).collect();
        assert_eq!(perm, expected, "cycle=0 should produce identity");
    }

    #[test]
    fn spiral_with_active_ln_preserves_mapping() {
        let mut r = setup_spiral(42);

        // LN start on lane 1
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(1, Some(new_ln_start(10, 100_000)));
        let perm1 = r.permutate(&mut tl1);
        assert_valid_permutation(&perm1, 8, &[0, 1, 2, 3, 4, 5, 6]);

        assert!(
            r.base.ln_active.contains_key(&1),
            "lane 1 should be in ln_active"
        );
        let mapped_dest = r.base.ln_active[&1];

        // Second timeline: LN still held
        let mut tl2 = TimeLine::new(0.0, 200_000, 8);
        tl2.set_note(3, Some(Note::new_normal(20)));
        let perm2 = r.permutate(&mut tl2);
        assert_valid_permutation(&perm2, 8, &[0, 1, 2, 3, 4, 5, 6]);

        assert_eq!(
            perm2[mapped_dest as usize], 1,
            "LN mapping must be preserved across spiral rotations"
        );
    }

    #[test]
    fn spiral_produces_periodic_rotation_pattern() {
        let mut r = setup_spiral(100);
        let cycle = r.cycle;
        let increment = r.increment;

        let mut perms: Vec<Vec<i32>> = Vec::new();
        for t in 0..cycle * 2 {
            let time_us = (t as i64 + 1) * 100_000;
            let mut tl = TimeLine::new(0.0, time_us, 8);
            tl.set_note(0, Some(Note::new_normal(t as i32 + 1)));
            let perm = r.permutate(&mut tl);
            assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
            perms.push(perm);
        }

        assert_eq!(perms.len(), cycle * 2);

        fn gcd(a: usize, b: usize) -> usize {
            if b == 0 { a } else { gcd(b, a % b) }
        }
        let period = cycle / gcd(increment, cycle);
        if perms.len() > period {
            for i in 0..perms.len() - period {
                assert_eq!(
                    perms[i],
                    perms[i + period],
                    "spiral should repeat with period {} but perms[{}] != perms[{}]",
                    period,
                    i,
                    i + period
                );
            }
        }
    }

    // ================================================================
    // AllScratchRandomizer tests
    // ================================================================

    #[test]
    fn all_scratch_produces_valid_permutation() {
        let mut r = setup_all_scratch(42, 40, 100);
        let mut tl = TimeLine::new(0.0, 100_000, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(2, Some(Note::new_normal(2)));

        let perm = r.permutate(&mut tl);
        assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn all_scratch_with_scratch_in_assignable_maps_to_scratch() {
        // Include scratch lane (7) in modify_lanes so it's in the assignable pool
        let mut r = AllScratchRandomizer::new(0, 100, 0);
        r.set_mode(Mode::BEAT_7K);
        r.base.set_modify_lanes(&[0, 1, 2, 3, 4, 5, 6, 7]);
        r.time_state.init_lanes(&[0, 1, 2, 3, 4, 5, 6, 7]);
        r.base.set_random_seed(42);

        let mut tl = TimeLine::new(0.0, 100_000, 8);
        tl.set_note(0, Some(Note::new_normal(1)));

        let perm = r.permutate(&mut tl);
        assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6, 7]);

        // With scratch_threshold=0, scratch is always eligible.
        // The note from lane 0 should be assigned to scratch lane 7.
        let dest_for_lane_0 = perm.iter().position(|&src| src == 0);
        assert_eq!(
            dest_for_lane_0,
            Some(7),
            "note from lane 0 should be mapped to scratch lane 7, perm={:?}",
            perm
        );
    }

    #[test]
    fn all_scratch_deterministic_with_same_seed() {
        let mut r1 = setup_all_scratch(77, 40, 100);
        let mut r2 = setup_all_scratch(77, 40, 100);

        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        tl1.set_note(3, Some(Note::new_normal(2)));
        let mut tl2 = TimeLine::new(0.0, 100_000, 8);
        tl2.set_note(0, Some(Note::new_normal(1)));
        tl2.set_note(3, Some(Note::new_normal(2)));

        let perm1 = r1.permutate(&mut tl1);
        let perm2 = r2.permutate(&mut tl2);
        assert_eq!(perm1, perm2);
    }

    #[test]
    fn all_scratch_valid_across_multiple_timelines() {
        let mut r = setup_all_scratch(42, 40, 100);
        for t in 0..5 {
            let time_us = (t + 1) * 100_000;
            let mut tl = TimeLine::new(0.0, time_us, 8);
            tl.set_note(t as i32 % 7, Some(Note::new_normal(t as i32 + 1)));
            let perm = r.permutate(&mut tl);
            assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn all_scratch_with_active_ln_preserves_mapping() {
        let mut r = setup_all_scratch(42, 40, 100);

        // LN start on lane 4
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(4, Some(new_ln_start(10, 100_000)));
        let perm1 = r.permutate(&mut tl1);
        assert_valid_permutation(&perm1, 8, &[0, 1, 2, 3, 4, 5, 6]);
        assert!(r.base.ln_active.contains_key(&4));
        let mapped_dest = r.base.ln_active[&4];

        // Second timeline: LN still held
        let mut tl2 = TimeLine::new(0.0, 200_000, 8);
        tl2.set_note(1, Some(Note::new_normal(20)));
        let perm2 = r.permutate(&mut tl2);
        assert_valid_permutation(&perm2, 8, &[0, 1, 2, 3, 4, 5, 6]);
        assert_eq!(perm2[mapped_dest as usize], 4);
    }

    #[test]
    fn all_scratch_double_play_scratch_lane_assignment() {
        let mut r_1p = AllScratchRandomizer::new(40, 100, SIDE_1P);
        r_1p.set_mode(Mode::BEAT_14K);
        assert!(r_1p.is_double_play);
        assert_eq!(r_1p.scratch_lane, vec![7]);

        let mut r_2p = AllScratchRandomizer::new(40, 100, SIDE_2P);
        r_2p.set_mode(Mode::BEAT_14K);
        assert!(r_2p.is_double_play);
        assert_eq!(r_2p.scratch_lane, vec![15]);
    }

    #[test]
    fn all_scratch_scratch_index_wraps_around() {
        let mut r = AllScratchRandomizer::new(0, 100, 0);
        r.set_mode(Mode::BEAT_7K);
        r.base.set_modify_lanes(&[0, 1, 2, 3, 4, 5, 6, 7]);
        r.time_state.init_lanes(&[0, 1, 2, 3, 4, 5, 6, 7]);
        r.base.set_random_seed(42);

        // BEAT_7K has scratch_lane = [7], so scratch_index wraps at 1 -> 0.
        assert_eq!(r.scratch_index, 0);

        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        let _perm1 = r.permutate(&mut tl1);
        // After first scratch assignment, index wraps back to 0 (only 1 scratch lane)
        assert_eq!(r.scratch_index, 0);
    }

    // ================================================================
    // NoMurioshiRandomizer tests
    // ================================================================

    #[test]
    fn no_murioshi_produces_valid_permutation() {
        let mut r = setup_no_murioshi(42, 100);
        let mut tl = TimeLine::new(0.0, 100_000, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(2, Some(Note::new_normal(2)));
        tl.set_note(4, Some(Note::new_normal(3)));

        let perm = r.permutate(&mut tl);
        assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn no_murioshi_deterministic_with_same_seed() {
        let mut r1 = setup_no_murioshi(42, 100);
        let mut r2 = setup_no_murioshi(42, 100);

        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        tl1.set_note(2, Some(Note::new_normal(2)));
        tl1.set_note(4, Some(Note::new_normal(3)));
        let mut tl2 = TimeLine::new(0.0, 100_000, 8);
        tl2.set_note(0, Some(Note::new_normal(1)));
        tl2.set_note(2, Some(Note::new_normal(2)));
        tl2.set_note(4, Some(Note::new_normal(3)));

        let perm1 = r1.permutate(&mut tl1);
        let perm2 = r2.permutate(&mut tl2);
        assert_eq!(perm1, perm2);
    }

    #[test]
    fn no_murioshi_valid_for_3_to_6_notes() {
        // The flag (button_combination constraint) activates when 2 < note_count < 7.
        for note_count in 3..=6 {
            let mut r = setup_no_murioshi(42, 100);
            let mut tl = TimeLine::new(0.0, 100_000, 8);
            for lane in 0..note_count {
                tl.set_note(lane as i32, Some(Note::new_normal(lane as i32 + 1)));
            }
            let perm = r.permutate(&mut tl);
            assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn no_murioshi_valid_for_2_or_fewer_notes() {
        for note_count in 0..=2 {
            let mut r = setup_no_murioshi(42, 100);
            let mut tl = TimeLine::new(0.0, 100_000, 8);
            for lane in 0..note_count {
                tl.set_note(lane as i32, Some(Note::new_normal(lane as i32 + 1)));
            }
            let perm = r.permutate(&mut tl);
            assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn no_murioshi_valid_for_7_notes() {
        let mut r = setup_no_murioshi(42, 100);
        let mut tl = TimeLine::new(0.0, 100_000, 8);
        for lane in 0..7 {
            tl.set_note(lane, Some(Note::new_normal(lane + 1)));
        }
        let perm = r.permutate(&mut tl);
        assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn no_murioshi_valid_across_varying_note_counts() {
        let mut r = setup_no_murioshi(42, 100);
        for t in 0..10 {
            let time_us = (t + 1) * 100_000;
            let mut tl = TimeLine::new(0.0, time_us, 8);
            let note_count = (t % 7) + 1;
            for lane in 0..note_count {
                tl.set_note(
                    lane as i32,
                    Some(Note::new_normal(t as i32 * 10 + lane as i32)),
                );
            }
            let perm = r.permutate(&mut tl);
            assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn no_murioshi_avoids_same_lane_rapid_repeat() {
        let threshold = 200; // 200ms
        let mut r = setup_no_murioshi(42, threshold);

        // First timeline at t=100ms
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        let _perm1 = r.permutate(&mut tl1);

        let dest1: Vec<i32> = (0..8)
            .filter(|&lane| tl1.note(lane).is_some() && !tl1.note(lane).unwrap().is_mine())
            .collect();
        assert_eq!(dest1.len(), 1, "exactly one note should exist");

        // Second timeline at t=150ms (50ms apart, within threshold=200ms)
        let mut tl2 = TimeLine::new(0.0, 150_000, 8);
        tl2.set_note(0, Some(Note::new_normal(2)));
        let _perm2 = r.permutate(&mut tl2);

        let dest2: Vec<i32> = (0..8)
            .filter(|&lane| tl2.note(lane).is_some() && !tl2.note(lane).unwrap().is_mine())
            .collect();
        assert_eq!(dest2.len(), 1, "exactly one note should exist");

        // With threshold=200 and only 50ms gap, the time-based shuffle should
        // place the second note on a different lane (the first lane goes to inferior).
        assert_ne!(
            dest1[0], dest2[0],
            "within threshold, notes should avoid the same destination lane: \
             first={}, second={}",
            dest1[0], dest2[0]
        );
    }

    #[test]
    fn no_murioshi_with_active_ln_preserves_mapping() {
        let mut r = setup_no_murioshi(42, 100);

        // LN start on lane 5
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(5, Some(new_ln_start(10, 100_000)));
        tl1.set_note(0, Some(Note::new_normal(20)));
        tl1.set_note(1, Some(Note::new_normal(30)));
        let perm1 = r.permutate(&mut tl1);
        assert_valid_permutation(&perm1, 8, &[0, 1, 2, 3, 4, 5, 6]);
        assert!(r.base.ln_active.contains_key(&5));
        let mapped_dest = r.base.ln_active[&5];

        // Second timeline: LN held, more notes
        let mut tl2 = TimeLine::new(0.0, 200_000, 8);
        tl2.set_note(0, Some(Note::new_normal(40)));
        tl2.set_note(2, Some(Note::new_normal(50)));
        tl2.set_note(3, Some(Note::new_normal(60)));
        let perm2 = r.permutate(&mut tl2);
        assert_valid_permutation(&perm2, 8, &[0, 1, 2, 3, 4, 5, 6]);
        assert_eq!(perm2[mapped_dest as usize], 5);
    }

    #[test]
    fn no_murioshi_ln_counted_in_note_count() {
        // Active LNs count toward note_count, affecting button_combination activation.
        let mut r = setup_no_murioshi(42, 100);

        // LN start on lane 0
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(0, Some(new_ln_start(10, 100_000)));
        let _perm1 = r.permutate(&mut tl1);
        assert!(r.base.ln_active.contains_key(&0));

        // 1 active LN + 2 new notes = 3 total, flag activates
        let mut tl2 = TimeLine::new(0.0, 200_000, 8);
        tl2.set_note(1, Some(Note::new_normal(20)));
        tl2.set_note(2, Some(Note::new_normal(30)));
        let perm2 = r.permutate(&mut tl2);
        assert_valid_permutation(&perm2, 8, &[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn no_murioshi_button_combination_destinations_within_valid_range() {
        // When the flag is active (3-6 notes), all note destinations
        // must be within the button_combination_table range (0..=8).
        let mut r = setup_no_murioshi(42, 100);
        let mut tl = TimeLine::new(0.0, 100_000, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_normal(2)));
        tl.set_note(2, Some(Note::new_normal(3)));
        tl.set_note(3, Some(Note::new_normal(4)));

        let perm = r.permutate(&mut tl);
        assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);

        let note_dests: Vec<i32> = (0..8)
            .filter(|&lane| tl.note(lane).is_some() && !tl.note(lane).unwrap().is_mine())
            .collect();
        for &d in &note_dests {
            assert!(
                (0..=8).contains(&d),
                "destination lane {} out of button combination range",
                d
            );
        }
    }

    // ================================================================
    // ConvergeRandomizer tests
    // ================================================================

    #[test]
    fn converge_produces_valid_permutation() {
        let mut r = setup_converge(42, 100, 200);
        let mut tl = TimeLine::new(0.0, 100_000, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(2, Some(Note::new_normal(2)));
        tl.set_note(4, Some(Note::new_normal(3)));

        let perm = r.permutate(&mut tl);
        assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn converge_deterministic_with_same_seed() {
        let mut r1 = setup_converge(42, 100, 200);
        let mut r2 = setup_converge(42, 100, 200);

        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        tl1.set_note(2, Some(Note::new_normal(2)));
        let mut tl2 = TimeLine::new(0.0, 100_000, 8);
        tl2.set_note(0, Some(Note::new_normal(1)));
        tl2.set_note(2, Some(Note::new_normal(2)));

        let perm1 = r1.permutate(&mut tl1);
        let perm2 = r2.permutate(&mut tl2);
        assert_eq!(perm1, perm2);
    }

    #[test]
    fn converge_renda_count_resets_after_threshold2() {
        let mut r = setup_converge(42, 100, 200);

        // First timeline at t=100ms
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        tl1.set_note(1, Some(Note::new_normal(2)));
        let _perm1 = r.permutate(&mut tl1);

        let total_after_first: i32 = r.renda_count.values().sum();
        assert_eq!(total_after_first, 2);

        // Second timeline at t=500ms (400ms apart > threshold2=200ms)
        let mut tl2 = TimeLine::new(0.0, 500_000, 8);
        tl2.set_note(0, Some(Note::new_normal(3)));
        let _perm2 = r.permutate(&mut tl2);

        let total_after_second: i32 = r.renda_count.values().sum();
        assert!(
            total_after_second >= 1,
            "at least 1 note placed in second timeline, got total={}",
            total_after_second
        );
    }

    #[test]
    fn converge_valid_across_multiple_timelines() {
        let mut r = setup_converge(42, 50, 100);
        for t in 0..10 {
            let time_us = (t + 1) * 50_000;
            let mut tl = TimeLine::new(0.0, time_us, 8);
            let note_count = (t % 5) + 1;
            for lane in 0..note_count {
                tl.set_note(
                    lane as i32,
                    Some(Note::new_normal(t as i32 * 10 + lane as i32)),
                );
            }
            let perm = r.permutate(&mut tl);
            assert_valid_permutation(&perm, 8, &[0, 1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn converge_with_active_ln_preserves_mapping() {
        let mut r = setup_converge(42, 100, 200);

        // LN start on lane 3
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        tl1.set_note(3, Some(new_ln_start(10, 100_000)));
        tl1.set_note(0, Some(Note::new_normal(20)));
        let perm1 = r.permutate(&mut tl1);
        assert_valid_permutation(&perm1, 8, &[0, 1, 2, 3, 4, 5, 6]);
        assert!(r.base.ln_active.contains_key(&3));
        let mapped_dest = r.base.ln_active[&3];

        // Second timeline: LN held
        let mut tl2 = TimeLine::new(0.0, 200_000, 8);
        tl2.set_note(1, Some(Note::new_normal(30)));
        let perm2 = r.permutate(&mut tl2);
        assert_valid_permutation(&perm2, 8, &[0, 1, 2, 3, 4, 5, 6]);
        assert_eq!(perm2[mapped_dest as usize], 3);
    }

    #[test]
    fn converge_prefers_lanes_with_higher_renda_count() {
        // The converge algorithm selects lanes with the maximum renda_count.
        // After several rounds, notes should cluster onto fewer lanes.
        let mut r = setup_converge(42, 10, 5000);

        let mut lane_usage: HashMap<i32, usize> = HashMap::new();
        for t in 0..20 {
            let time_us = (t + 1) * 20_000; // 20ms apart, within threshold
            let mut tl = TimeLine::new(0.0, time_us, 8);
            tl.set_note(0, Some(Note::new_normal(t as i32 + 1)));
            let _perm = r.permutate(&mut tl);

            for lane in 0..7 {
                if tl.note(lane).is_some() && !tl.note(lane).unwrap().is_mine() {
                    *lane_usage.entry(lane).or_insert(0) += 1;
                }
            }
        }

        let max_usage = lane_usage.values().max().copied().unwrap_or(0);
        // Uniform would be ~20/7 ~= 2.8, converge should push some lane higher.
        assert!(
            max_usage >= 4,
            "converge should cluster notes: max lane usage={}, distribution={:?}",
            max_usage,
            lane_usage
        );
    }
}

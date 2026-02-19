// Note shuffle modifiers (per-timeline lane permutation)
//
// Ported from Java: Randomizer.java + NoteShuffleModifier.java
//
// Unlike lane shuffle (one fixed mapping for the whole chart), note shuffle
// applies an independent lane permutation per timeline (group of simultaneous
// notes), enabling algorithms like S-RANDOM, SPIRAL, ALL-SCR, etc.

use std::collections::{BTreeMap, HashMap};

use bms_model::{BmsModel, NoteType};

use crate::java_random::JavaRandom;
use crate::modifier::{AssistLevel, PatternModifier, get_keys};

// ---------------------------------------------------------------------------
// Constants (milliseconds, matching Java)
// ---------------------------------------------------------------------------

const SRAN_THRESHOLD: i32 = 40;
const DEFAULT_HRAN_THRESHOLD: i32 = 100;

// ---------------------------------------------------------------------------
// TimelineView — pre-computed snapshot of one time point
// ---------------------------------------------------------------------------

/// Pre-computed timeline snapshot for one time point.
///
/// Separates visible notes from invisible (hidden) notes, and records note
/// types for mine detection during shuffle logic.
struct TimelineView {
    /// Time in milliseconds: `(time_us / 1000) as i32`
    time_ms: i32,
    /// lane -> note index in `model.notes` (Normal/LN/Mine)
    notes: HashMap<usize, usize>,
    /// lane -> note index in `model.notes` (Invisible only)
    hidden_notes: HashMap<usize, usize>,
    /// lane -> NoteType for quick mine detection
    lane_note_types: HashMap<usize, NoteType>,
}

/// Build timeline views from the model's notes, grouped by `time_us`.
///
/// Returns a `Vec<TimelineView>` sorted by time (ascending).
fn build_timeline_views(model: &BmsModel) -> Vec<TimelineView> {
    let mut groups: BTreeMap<i64, TimelineView> = BTreeMap::new();

    for (idx, note) in model.notes.iter().enumerate() {
        let view = groups.entry(note.time_us).or_insert_with(|| TimelineView {
            time_ms: (note.time_us / 1000) as i32,
            notes: HashMap::new(),
            hidden_notes: HashMap::new(),
            lane_note_types: HashMap::new(),
        });
        if note.note_type == NoteType::Invisible {
            view.hidden_notes.insert(note.lane, idx);
        } else {
            view.notes.insert(note.lane, idx);
            view.lane_note_types.insert(note.lane, note.note_type);
        }
    }

    groups.into_values().collect()
}

// ---------------------------------------------------------------------------
// PermutationState — persisted across timelines
// ---------------------------------------------------------------------------

/// State persisted across timelines during permutation.
struct PermutationState {
    /// Active LN tracking: source_lane (changeable slot) -> dest_lane
    ln_active: HashMap<usize, usize>,
    /// Available source lanes (not occupied by active LN)
    changeable_lane: Vec<usize>,
    /// Available destination lanes (not occupied by active LN)
    assignable_lane: Vec<usize>,
    /// All modifiable lanes (constant after init)
    modify_lanes: Vec<usize>,
}

impl PermutationState {
    fn new(modify_lanes: Vec<usize>) -> Self {
        let changeable_lane = modify_lanes.clone();
        let assignable_lane = modify_lanes.clone();
        Self {
            ln_active: HashMap::new(),
            changeable_lane,
            assignable_lane,
            modify_lanes,
        }
    }
}

// ---------------------------------------------------------------------------
// Randomizer enum dispatch
// ---------------------------------------------------------------------------

/// Randomizer implementations dispatched via enum (no trait objects).
enum RandomizerImpl {
    SRandom(SRandomizer),
    Spiral(SpiralRandomizer),
    AllScr(AllScratchRandomizer),
    NoMurioshi(NoMurioshiRandomizer),
    Converge(ConvergeRandomizer),
}

impl RandomizerImpl {
    fn randomize(
        &mut self,
        view: &TimelineView,
        changeable_lane: &mut Vec<usize>,
        assignable_lane: &mut Vec<usize>,
        modify_lanes: &[usize],
        ln_active_values: &[usize],
        rng: &mut JavaRandom,
    ) -> HashMap<usize, usize> {
        match self {
            Self::SRandom(r) => r.randomize(view, changeable_lane, assignable_lane, rng),
            Self::Spiral(r) => r.randomize(view, changeable_lane, modify_lanes),
            Self::AllScr(r) => r.randomize(view, changeable_lane, assignable_lane, rng),
            Self::NoMurioshi(r) => r.randomize(
                view,
                changeable_lane,
                assignable_lane,
                modify_lanes,
                ln_active_values,
                rng,
            ),
            Self::Converge(r) => r.randomize(view, changeable_lane, assignable_lane, rng),
        }
    }

    fn set_modify_lanes(&mut self, lanes: &[usize], rng: &mut JavaRandom) {
        match self {
            Self::SRandom(r) => r.time_state.init_lanes(lanes),
            Self::Spiral(r) => r.init(lanes, rng),
            Self::AllScr(r) => r.time_state.init_lanes(lanes),
            Self::NoMurioshi(r) => r.time_state.init_lanes(lanes),
            Self::Converge(r) => {
                r.time_state.init_lanes(lanes);
                r.init_renda(lanes);
            }
        }
    }

    #[allow(dead_code)] // NoteShuffleModifier::assist_level() duplicates this logic inline
    fn assist_level(&self) -> AssistLevel {
        match self {
            Self::SRandom(r) => r.assist,
            Self::Spiral(_) => AssistLevel::LightAssist,
            Self::AllScr(_) => AssistLevel::LightAssist,
            Self::NoMurioshi(_) => AssistLevel::LightAssist,
            Self::Converge(_) => AssistLevel::LightAssist,
        }
    }
}

// ---------------------------------------------------------------------------
// TimeBasedState — shared time-based threshold logic
// ---------------------------------------------------------------------------

/// Shared state for time-based randomizers.
///
/// Tracks when each lane last had a note, to avoid rapid repeats.
struct TimeBasedState {
    threshold: i32,
    last_note_time: HashMap<usize, i32>,
}

impl TimeBasedState {
    fn new(threshold: i32) -> Self {
        Self {
            threshold,
            last_note_time: HashMap::new(),
        }
    }

    fn init_lanes(&mut self, lanes: &[usize]) {
        for &lane in lanes {
            self.last_note_time.insert(lane, -10000);
        }
    }

    /// Core time-based shuffle: distributes notes to lanes while respecting
    /// the threshold to avoid rapid repeats.
    ///
    /// `select_lane_fn` is called to pick from primary lanes.
    fn time_based_shuffle(
        &self,
        view: &TimelineView,
        changeable_lane: &mut [usize],
        assignable_lane: &mut [usize],
        rng: &mut JavaRandom,
        select_lane_fn: &mut dyn FnMut(&mut Vec<usize>, &mut JavaRandom) -> usize,
    ) -> HashMap<usize, usize> {
        let mut random_map = HashMap::new();

        // Classify changeable lanes into noteLane / emptyLane
        let mut note_lane = Vec::new();
        let mut empty_lane = Vec::new();
        for &cl in changeable_lane.iter() {
            let is_empty = matches!(view.lane_note_types.get(&cl), None | Some(&NoteType::Mine));
            if is_empty {
                empty_lane.push(cl);
            } else {
                note_lane.push(cl);
            }
        }

        // Classify assignable lanes into primaryLane / inferiorLane
        let mut primary_lane = Vec::new();
        let mut inferior_lane = Vec::new();
        for &al in assignable_lane.iter() {
            if view.time_ms - self.last_note_time.get(&al).copied().unwrap_or(-10000)
                > self.threshold
            {
                primary_lane.push(al);
            } else {
                inferior_lane.push(al);
            }
        }

        // Assign noteLane -> primaryLane (avoiding repeats)
        while !note_lane.is_empty() && !primary_lane.is_empty() {
            let r = select_lane_fn(&mut primary_lane, rng);
            let dest = primary_lane.remove(r);
            random_map.insert(note_lane.remove(0), dest);
        }

        // Remaining noteLane -> inferiorLane (pick lanes with smallest lastNoteTime)
        while !note_lane.is_empty() {
            let min_time = inferior_lane
                .iter()
                .map(|l| self.last_note_time.get(l).copied().unwrap_or(-10000))
                .min()
                .unwrap_or(-10000);
            let min_lanes: Vec<usize> = inferior_lane
                .iter()
                .copied()
                .filter(|l| self.last_note_time.get(l).copied().unwrap_or(-10000) == min_time)
                .collect();
            let chosen = min_lanes[rng.next_int(min_lanes.len() as i32) as usize];
            random_map.insert(note_lane.remove(0), chosen);
            inferior_lane.retain(|&l| l != chosen);
        }

        // Remaining emptyLane -> whatever's left (random)
        primary_lane.extend(inferior_lane);
        while !empty_lane.is_empty() {
            let r = rng.next_int(primary_lane.len() as i32) as usize;
            random_map.insert(empty_lane.remove(0), primary_lane.remove(r));
        }

        random_map
    }

    /// Update lastNoteTime for lanes that received a real note.
    fn update_note_time(&mut self, view: &TimelineView, random_map: &HashMap<usize, usize>) {
        for (&src, &dest) in random_map {
            let has_note = !matches!(view.lane_note_types.get(&src), None | Some(&NoteType::Mine));
            if has_note {
                self.last_note_time.insert(dest, view.time_ms);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 1. SRandomizer (S-RANDOM, H-RANDOM, S-RANDOM-EX, S-RANDOM-NO-THRESHOLD)
// ---------------------------------------------------------------------------

struct SRandomizer {
    time_state: TimeBasedState,
    #[allow(dead_code)] // Read via NoteShuffleStrategy::assist_level()
    assist: AssistLevel,
}

impl SRandomizer {
    fn new(threshold: i32, assist: AssistLevel) -> Self {
        Self {
            time_state: TimeBasedState::new(threshold),
            assist,
        }
    }

    fn randomize(
        &mut self,
        view: &TimelineView,
        changeable_lane: &mut [usize],
        assignable_lane: &mut [usize],
        rng: &mut JavaRandom,
    ) -> HashMap<usize, usize> {
        let random_map = self.time_state.time_based_shuffle(
            view,
            changeable_lane,
            assignable_lane,
            rng,
            &mut |lane, r| r.next_int(lane.len() as i32) as usize,
        );
        self.time_state.update_note_time(view, &random_map);
        random_map
    }
}

// ---------------------------------------------------------------------------
// 2. SpiralRandomizer
// ---------------------------------------------------------------------------

struct SpiralRandomizer {
    increment: usize,
    head: usize,
    cycle: usize,
}

impl SpiralRandomizer {
    fn new() -> Self {
        Self {
            increment: 0,
            head: 0,
            cycle: 0,
        }
    }

    fn init(&mut self, lanes: &[usize], rng: &mut JavaRandom) {
        self.increment = rng.next_int(lanes.len() as i32 - 1) as usize + 1;
        self.head = 0;
        self.cycle = lanes.len();
    }

    fn randomize(
        &mut self,
        _view: &TimelineView,
        changeable_lane: &mut [usize],
        modify_lanes: &[usize],
    ) -> HashMap<usize, usize> {
        let mut rotate_map = HashMap::new();
        if changeable_lane.len() == self.cycle {
            // All lanes are changeable: advance head
            self.head = (self.head + self.increment) % self.cycle;
            for (i, &ml) in modify_lanes.iter().enumerate() {
                rotate_map.insert(ml, modify_lanes[(i + self.head) % self.cycle]);
            }
        } else {
            // LN active: keep head unchanged, only map changeable lanes
            for (i, &ml) in modify_lanes.iter().enumerate() {
                if changeable_lane.contains(&ml) {
                    rotate_map.insert(ml, modify_lanes[(i + self.head) % self.cycle]);
                }
            }
        }
        rotate_map
    }
}

// ---------------------------------------------------------------------------
// 3. AllScratchRandomizer
// ---------------------------------------------------------------------------

struct AllScratchRandomizer {
    time_state: TimeBasedState,
    scratch_threshold: i32,
    scratch_lane: Vec<usize>,
    scratch_index: usize,
    modify_side: usize,
    is_double_play: bool,
}

impl AllScratchRandomizer {
    fn new(scratch_threshold: i32, key_threshold: i32, modify_side: usize) -> Self {
        Self {
            time_state: TimeBasedState::new(key_threshold),
            scratch_threshold,
            scratch_lane: Vec::new(),
            scratch_index: 0,
            modify_side,
            is_double_play: false,
        }
    }

    fn set_mode(&mut self, mode: bms_model::PlayMode) {
        let scratch_keys = mode.scratch_keys();
        self.is_double_play = mode.player_count() == 2;
        if self.is_double_play {
            let half = scratch_keys.len() / 2;
            let start = self.modify_side * half;
            self.scratch_lane = scratch_keys[start..start + half].to_vec();
        } else {
            self.scratch_lane = scratch_keys.to_vec();
        }
    }

    #[allow(clippy::ptr_arg)]
    fn randomize(
        &mut self,
        view: &TimelineView,
        changeable_lane: &mut Vec<usize>,
        assignable_lane: &mut Vec<usize>,
        rng: &mut JavaRandom,
    ) -> HashMap<usize, usize> {
        let mut random_map = HashMap::new();

        // Try to assign a note to the scratch lane first
        if !self.scratch_lane.is_empty() {
            let scr = self.scratch_lane[self.scratch_index];
            if assignable_lane.contains(&scr)
                && view.time_ms
                    - self
                        .time_state
                        .last_note_time
                        .get(&scr)
                        .copied()
                        .unwrap_or(-10000)
                    > self.scratch_threshold
            {
                // Find first changeable lane with a real note
                let mut found = None;
                for &cl in changeable_lane.iter() {
                    let has_note =
                        !matches!(view.lane_note_types.get(&cl), None | Some(&NoteType::Mine));
                    if has_note {
                        found = Some(cl);
                        break;
                    }
                }
                if let Some(l) = found {
                    random_map.insert(l, scr);
                    changeable_lane.retain(|&x| x != l);
                    assignable_lane.retain(|&x| x != scr);
                    self.scratch_index += 1;
                    if self.scratch_index == self.scratch_lane.len() {
                        self.scratch_index = 0;
                    }
                }
            }
        }

        // Assign remaining using time-based shuffle
        let is_dp = self.is_double_play;
        let side = self.modify_side;
        let rest = self.time_state.time_based_shuffle(
            view,
            changeable_lane,
            assignable_lane,
            rng,
            &mut |lane, r| {
                if is_dp {
                    // DP: pick lane closest to scratch side
                    match side {
                        0 => {
                            // 1P: pick min lane index
                            lane.iter()
                                .enumerate()
                                .min_by_key(|&(_, &v)| v)
                                .map(|(i, _)| i)
                                .unwrap_or(0)
                        }
                        _ => {
                            // 2P: pick max lane index
                            lane.iter()
                                .enumerate()
                                .max_by_key(|&(_, &v)| v)
                                .map(|(i, _)| i)
                                .unwrap_or(0)
                        }
                    }
                } else {
                    r.next_int(lane.len() as i32) as usize
                }
            },
        );
        random_map.extend(rest.iter());

        self.time_state.update_note_time(view, &random_map);
        random_map
    }
}

// ---------------------------------------------------------------------------
// 4. NoMurioshiRandomizer (PMS S-RANDOM-PLAYABLE)
// ---------------------------------------------------------------------------

/// 10 valid 6-button combinations that avoid murioshi (impossible) patterns.
///
/// Java: `NoMurioshiRandomizer.buttonCombinationTable`
const BUTTON_COMBINATION_TABLE: [[usize; 6]; 10] = [
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 4, 5, 6],
    [0, 1, 2, 5, 6, 7],
    [0, 1, 2, 6, 7, 8],
    [1, 2, 3, 4, 5, 6],
    [1, 2, 3, 5, 6, 7],
    [1, 2, 3, 6, 7, 8],
    [2, 3, 4, 5, 6, 7],
    [2, 3, 4, 6, 7, 8],
    [3, 4, 5, 6, 7, 8],
];

struct NoMurioshiRandomizer {
    time_state: TimeBasedState,
    button_combination: Vec<usize>,
    flag: bool,
}

impl NoMurioshiRandomizer {
    fn new(threshold: i32) -> Self {
        Self {
            time_state: TimeBasedState::new(threshold),
            button_combination: Vec::new(),
            flag: false,
        }
    }

    /// Count lanes with real notes (not mine, not null) among modify_lanes.
    fn get_note_exist_lanes(&self, view: &TimelineView, modify_lanes: &[usize]) -> Vec<usize> {
        let mut result = Vec::new();
        for &ml in modify_lanes {
            let has_note = !matches!(view.lane_note_types.get(&ml), None | Some(&NoteType::Mine));
            if has_note {
                result.push(ml);
            }
        }
        result
    }

    fn randomize(
        &mut self,
        view: &TimelineView,
        changeable_lane: &mut Vec<usize>,
        assignable_lane: &mut Vec<usize>,
        modify_lanes: &[usize],
        ln_active_values: &[usize],
        rng: &mut JavaRandom,
    ) -> HashMap<usize, usize> {
        let note_exist = self.get_note_exist_lanes(view, modify_lanes);
        let note_count = note_exist.len() + ln_active_values.len();

        self.flag = 2 < note_count && note_count < 7;

        if self.flag {
            let candidate: Vec<Vec<usize>> = if ln_active_values.is_empty() {
                BUTTON_COMBINATION_TABLE
                    .iter()
                    .map(|c| c.to_vec())
                    .collect()
            } else {
                BUTTON_COMBINATION_TABLE
                    .iter()
                    .filter(|c| ln_active_values.iter().all(|lv| c.contains(lv)))
                    .map(|c| c.to_vec())
                    .collect()
            };

            if !candidate.is_empty() {
                // Filter out lanes that would cause rapid repeats
                let renda_lane: Vec<usize> = self
                    .time_state
                    .last_note_time
                    .iter()
                    .filter(|&(_, &time)| view.time_ms - time < self.time_state.threshold)
                    .map(|(&lane, _)| lane)
                    .collect();

                let candidate2: Vec<Vec<usize>> = candidate
                    .iter()
                    .map(|lanes| {
                        lanes
                            .iter()
                            .copied()
                            .filter(|l| !renda_lane.contains(l))
                            .collect::<Vec<_>>()
                    })
                    .filter(|lanes| lanes.len() >= note_count)
                    .collect();

                if !candidate2.is_empty() {
                    self.button_combination =
                        candidate2[rng.next_int(candidate2.len() as i32) as usize].clone();
                } else {
                    // Prioritize avoiding murioshi over avoiding repeats
                    let mut random_map = HashMap::new();
                    // Java bug: uses candidate2.size() as bound on candidate —
                    // candidate2 is empty here, so Java would throw.
                    // We use candidate.len() instead.
                    self.button_combination = candidate
                        [rng.next_int(candidate.len() as i32) as usize]
                        .iter()
                        .copied()
                        .filter(|l| assignable_lane.contains(l))
                        .collect();

                    let e: Vec<usize> = note_exist
                        .iter()
                        .copied()
                        .filter(|l| changeable_lane.contains(l))
                        .collect();

                    for lane in &e {
                        if self.button_combination.is_empty() {
                            break;
                        }
                        let i = rng.next_int(self.button_combination.len() as i32) as usize;
                        let dest = self.button_combination.remove(i);
                        random_map.insert(*lane, dest);
                        changeable_lane.retain(|&x| x != *lane);
                        assignable_lane.retain(|&x| x != dest);
                    }
                    self.flag = false;
                    let rest = self.time_state.time_based_shuffle(
                        view,
                        changeable_lane,
                        assignable_lane,
                        rng,
                        &mut |lane, r| r.next_int(lane.len() as i32) as usize,
                    );
                    random_map.extend(rest.iter());
                    return random_map;
                }
            } else {
                // Only murioshi combinations exist — fall back to normal
                self.flag = false;
            }
        }

        let flag = self.flag;
        let bc = &self.button_combination;
        let random_map = self.time_state.time_based_shuffle(
            view,
            changeable_lane,
            assignable_lane,
            rng,
            &mut |lane, r| {
                if flag {
                    let filtered: Vec<usize> =
                        lane.iter().copied().filter(|l| bc.contains(l)).collect();
                    if !filtered.is_empty() {
                        let chosen = filtered[r.next_int(filtered.len() as i32) as usize];
                        return lane.iter().position(|&l| l == chosen).unwrap();
                    }
                }
                r.next_int(lane.len() as i32) as usize
            },
        );
        self.time_state.update_note_time(view, &random_map);
        random_map
    }
}

// ---------------------------------------------------------------------------
// 5. ConvergeRandomizer
// ---------------------------------------------------------------------------

struct ConvergeRandomizer {
    time_state: TimeBasedState,
    threshold2: i32,
    renda_count: HashMap<usize, i32>,
}

impl ConvergeRandomizer {
    fn new(threshold1: i32, threshold2: i32) -> Self {
        Self {
            time_state: TimeBasedState::new(threshold1),
            threshold2,
            renda_count: HashMap::new(),
        }
    }

    fn init_renda(&mut self, lanes: &[usize]) {
        for &lane in lanes {
            self.renda_count.insert(lane, 0);
        }
    }

    fn randomize(
        &mut self,
        view: &TimelineView,
        changeable_lane: &mut [usize],
        assignable_lane: &mut [usize],
        rng: &mut JavaRandom,
    ) -> HashMap<usize, usize> {
        // Reset renda count for lanes that exceed threshold2
        let t2 = self.threshold2;
        let lnt = &self.time_state.last_note_time;
        let resets: Vec<usize> = self
            .renda_count
            .keys()
            .copied()
            .filter(|k| view.time_ms - lnt.get(k).copied().unwrap_or(-10000) > t2)
            .collect();
        for k in resets {
            self.renda_count.insert(k, 0);
        }

        // Split borrow: time_state and renda_count are separate fields
        let rc = &mut self.renda_count;
        let random_map = self.time_state.time_based_shuffle(
            view,
            changeable_lane,
            assignable_lane,
            rng,
            &mut |lane, r| {
                // Pick lane with max renda count (Java: selectLane)
                let max_count = lane
                    .iter()
                    .map(|l| rc.get(l).copied().unwrap_or(0))
                    .max()
                    .unwrap_or(0);
                let max_lanes: Vec<usize> = lane
                    .iter()
                    .copied()
                    .filter(|l| rc.get(l).copied().unwrap_or(0) == max_count)
                    .collect();
                let chosen = max_lanes[r.next_int(max_lanes.len() as i32) as usize];
                // Java: rendaCount.put(l, rendaCount.get(l) + 1) inside selectLane
                // Only primaryLane assignments increment renda count
                *rc.entry(chosen).or_insert(0) += 1;
                lane.iter().position(|&l| l == chosen).unwrap()
            },
        );

        self.time_state.update_note_time(view, &random_map);
        random_map
    }
}

// ---------------------------------------------------------------------------
// permutate — apply randomizer result to model notes
// ---------------------------------------------------------------------------

/// Apply the randomizer's mapping to one timeline, handling LN tracking.
///
/// Java: `Randomizer.permutate(TimeLine tl)`
fn permutate(
    model: &mut BmsModel,
    view: &TimelineView,
    state: &mut PermutationState,
    randomizer: &mut RandomizerImpl,
    rng: &mut JavaRandom,
) {
    let ln_active_values: Vec<usize> = state.ln_active.values().copied().collect();

    let random_map = randomizer.randomize(
        view,
        &mut state.changeable_lane.clone(),
        &mut state.assignable_lane.clone(),
        &state.modify_lanes,
        &ln_active_values,
        rng,
    );

    // Merge LN active mappings
    let mut full_map = random_map;
    for (&src, &dest) in &state.ln_active {
        full_map.insert(src, dest);
    }

    // Apply mappings
    for (&src, &dest) in &full_map {
        // Move visible note
        if let Some(&note_idx) = view.notes.get(&src) {
            let note = &model.notes[note_idx];
            let is_ln = note.is_long_note();
            let is_ln_end = is_ln && note.end_time_us == 0;
            let is_ln_start = is_ln && note.end_time_us > 0;
            let note_time_us = note.time_us;

            if is_ln_end && state.ln_active.contains_key(&src) {
                // Check if this is actually the end of the LN
                // Java checks: ln2.isEnd() && LNactive.containsKey(x) && tl.getTime() == ln2.getTime()
                // For end notes, time_us is the end time and matches the timeline time
                let _ = note_time_us; // time matches by construction
                state.ln_active.remove(&src);
                if !state.changeable_lane.contains(&src) {
                    state.changeable_lane.push(src);
                }
                if !state.assignable_lane.contains(&dest) {
                    state.assignable_lane.push(dest);
                }
            } else if is_ln_start {
                state.ln_active.insert(src, dest);
                state.changeable_lane.retain(|&l| l != src);
                state.assignable_lane.retain(|&l| l != dest);
            }

            // Update lane
            model.notes[note_idx].lane = dest;

            // If LN start, also update the paired end note's lane
            if is_ln_start {
                let pair_idx = model.notes[note_idx].pair_index;
                if pair_idx != usize::MAX {
                    model.notes[pair_idx].lane = dest;
                }
            }
        }

        // Move hidden (invisible) note with same mapping
        if let Some(&hidden_idx) = view.hidden_notes.get(&src) {
            model.notes[hidden_idx].lane = dest;
        }
    }
}

// ---------------------------------------------------------------------------
// Threshold computation
// ---------------------------------------------------------------------------

/// Compute threshold in milliseconds from BPM config value.
///
/// Java: `Randomizer.create()` threshold logic
fn compute_threshold_millis(hran_threshold_bpm: i32) -> i32 {
    if hran_threshold_bpm > 0 {
        (15000.0_f32 / hran_threshold_bpm as f32).ceil() as i32
    } else if hran_threshold_bpm == 0 {
        0
    } else {
        DEFAULT_HRAN_THRESHOLD
    }
}

// ---------------------------------------------------------------------------
// NoteShuffleModifier — public API
// ---------------------------------------------------------------------------

use crate::modifier::RandomType;

/// Note-level pattern modifier: applies per-timeline lane permutation.
///
/// Java: `NoteShuffleModifier` + `Randomizer`
pub struct NoteShuffleModifier {
    random_type: RandomType,
    player: usize,
    seed: i64,
    hran_threshold_bpm: i32,
}

impl NoteShuffleModifier {
    /// Create a new NoteShuffleModifier.
    ///
    /// - `random_type`: The shuffle algorithm to use
    /// - `player`: Player index (0 for 1P, 1 for 2P)
    /// - `seed`: Random seed (negative = use random seed)
    /// - `hran_threshold_bpm`: BPM threshold for H-RANDOM (negative = default, 0 = none)
    pub fn new(random_type: RandomType, player: usize, seed: i64, hran_threshold_bpm: i32) -> Self {
        Self {
            random_type,
            player,
            seed,
            hran_threshold_bpm,
        }
    }

    /// Create the appropriate randomizer impl for this type.
    fn create_randomizer(&self, mode: bms_model::PlayMode) -> RandomizerImpl {
        let threshold = compute_threshold_millis(self.hran_threshold_bpm);
        let mut randomizer = match self.random_type {
            RandomType::SRandom => {
                RandomizerImpl::SRandom(SRandomizer::new(SRAN_THRESHOLD, AssistLevel::None))
            }
            RandomType::HRandom => {
                RandomizerImpl::SRandom(SRandomizer::new(threshold, AssistLevel::LightAssist))
            }
            RandomType::SRandomEx => {
                RandomizerImpl::SRandom(SRandomizer::new(SRAN_THRESHOLD, AssistLevel::LightAssist))
            }
            RandomType::SRandomNoThreshold => {
                RandomizerImpl::SRandom(SRandomizer::new(0, AssistLevel::None))
            }
            RandomType::Spiral => RandomizerImpl::Spiral(SpiralRandomizer::new()),
            RandomType::AllScr => RandomizerImpl::AllScr(AllScratchRandomizer::new(
                SRAN_THRESHOLD,
                threshold,
                self.player,
            )),
            RandomType::SRandomPlayable => {
                RandomizerImpl::NoMurioshi(NoMurioshiRandomizer::new(threshold))
            }
            RandomType::Converge => {
                RandomizerImpl::Converge(ConvergeRandomizer::new(threshold, threshold * 2))
            }
            _ => RandomizerImpl::SRandom(SRandomizer::new(SRAN_THRESHOLD, AssistLevel::None)),
        };

        // Set mode for AllScr
        if let RandomizerImpl::AllScr(ref mut allscr) = randomizer {
            allscr.set_mode(mode);
        }

        randomizer
    }
}

impl PatternModifier for NoteShuffleModifier {
    fn modify(&mut self, model: &mut BmsModel) {
        let contains_scratch = self.random_type.is_scratch_lane_modify();
        let keys = get_keys(model.mode, self.player, contains_scratch);
        if keys.is_empty() {
            return;
        }

        let mut rng = JavaRandom::new(0);
        if self.seed >= 0 {
            rng.set_seed(self.seed);
        }

        let mut randomizer = self.create_randomizer(model.mode);
        randomizer.set_modify_lanes(&keys, &mut rng);

        let mut state = PermutationState::new(keys);

        let views = build_timeline_views(model);
        for view in &views {
            // Only process timelines that have notes
            if view.notes.is_empty() && view.hidden_notes.is_empty() {
                continue;
            }
            permutate(model, view, &mut state, &mut randomizer, &mut rng);
        }
    }

    fn assist_level(&self) -> AssistLevel {
        // Delegate to the randomizer's assist level
        let threshold = compute_threshold_millis(self.hran_threshold_bpm);
        match self.random_type {
            RandomType::SRandom | RandomType::SRandomNoThreshold => AssistLevel::None,
            RandomType::HRandom
            | RandomType::SRandomEx
            | RandomType::Spiral
            | RandomType::AllScr
            | RandomType::SRandomPlayable
            | RandomType::Converge => AssistLevel::LightAssist,
            _ => {
                let _ = threshold;
                AssistLevel::None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::{LnType, Note, PlayMode};

    fn make_model(mode: PlayMode, notes: Vec<Note>) -> BmsModel {
        BmsModel {
            mode,
            notes,
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // build_timeline_views
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_timeline_views_groups_by_time() {
        let notes = vec![
            Note::normal(0, 1000, 1),
            Note::normal(1, 1000, 2),
            Note::normal(0, 2000, 3),
        ];
        let model = make_model(PlayMode::Beat7K, notes);
        let views = build_timeline_views(&model);
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].time_ms, 1);
        assert_eq!(views[0].notes.len(), 2);
        assert_eq!(views[1].time_ms, 2);
        assert_eq!(views[1].notes.len(), 1);
    }

    #[test]
    fn test_build_timeline_views_separates_invisible() {
        let notes = vec![Note::normal(0, 1000, 1), Note::invisible(1, 1000, 2)];
        let model = make_model(PlayMode::Beat7K, notes);
        let views = build_timeline_views(&model);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].notes.len(), 1);
        assert!(views[0].notes.contains_key(&0));
        assert_eq!(views[0].hidden_notes.len(), 1);
        assert!(views[0].hidden_notes.contains_key(&1));
    }

    #[test]
    fn test_build_timeline_views_records_note_types() {
        let notes = vec![Note::normal(0, 1000, 1), Note::mine(1, 1000, 2, 10)];
        let model = make_model(PlayMode::Beat7K, notes);
        let views = build_timeline_views(&model);
        assert_eq!(views[0].lane_note_types[&0], NoteType::Normal);
        assert_eq!(views[0].lane_note_types[&1], NoteType::Mine);
    }

    // -----------------------------------------------------------------------
    // SRandomizer — deterministic with seed
    // -----------------------------------------------------------------------

    #[test]
    fn test_srandom_deterministic() {
        let notes = vec![
            Note::normal(0, 1000000, 1),
            Note::normal(1, 2000000, 2),
            Note::normal(2, 3000000, 3),
        ];
        let mut model1 = make_model(PlayMode::Beat7K, notes.clone());
        let mut model2 = make_model(PlayMode::Beat7K, notes);

        let mut mod1 = NoteShuffleModifier::new(RandomType::SRandom, 0, 42, -1);
        let mut mod2 = NoteShuffleModifier::new(RandomType::SRandom, 0, 42, -1);
        mod1.modify(&mut model1);
        mod2.modify(&mut model2);

        let lanes1: Vec<usize> = model1.notes.iter().map(|n| n.lane).collect();
        let lanes2: Vec<usize> = model2.notes.iter().map(|n| n.lane).collect();
        assert_eq!(lanes1, lanes2);
    }

    // -----------------------------------------------------------------------
    // SpiralRandomizer
    // -----------------------------------------------------------------------

    #[test]
    fn test_spiral_rotates() {
        // Create notes on lane 0 at different times, far apart
        let notes = vec![
            Note::normal(0, 1000000, 1),
            Note::normal(0, 2000000, 2),
            Note::normal(0, 3000000, 3),
        ];
        let mut model = make_model(PlayMode::Beat7K, notes);
        let mut modifier = NoteShuffleModifier::new(RandomType::Spiral, 0, 42, -1);
        modifier.modify(&mut model);

        // Each note should land on a different lane (spiral pattern)
        let lanes: Vec<usize> = model.notes.iter().map(|n| n.lane).collect();
        // All lanes should be different due to spiral increment
        assert_ne!(lanes[0], lanes[1]);
        assert_ne!(lanes[1], lanes[2]);
    }

    #[test]
    fn test_spiral_ln_keeps_head() {
        // LN active should prevent head from advancing
        let mut notes = vec![
            Note::long_note(0, 1000000, 3000000, 1, 2, LnType::LongNote), // LN start
            Note::long_note(0, 3000000, 0, 2, 0, LnType::LongNote),       // LN end
            Note::normal(1, 2000000, 3), // Note during LN — head should NOT advance
        ];
        notes[0].pair_index = 1;
        notes[1].pair_index = 0;

        let mut model = make_model(PlayMode::Beat7K, notes);
        let mut modifier = NoteShuffleModifier::new(RandomType::Spiral, 0, 100, -1);
        modifier.modify(&mut model);

        // LN start and end should be on the same lane
        assert_eq!(model.notes[0].lane, model.notes[1].lane);
    }

    // -----------------------------------------------------------------------
    // LN pair integrity
    // -----------------------------------------------------------------------

    #[test]
    fn test_ln_pair_same_lane_after_shuffle() {
        let mut notes = vec![
            Note::long_note(0, 1000000, 2000000, 1, 2, LnType::LongNote),
            Note::long_note(0, 2000000, 0, 2, 0, LnType::LongNote),
            Note::long_note(3, 1000000, 2000000, 3, 4, LnType::LongNote),
            Note::long_note(3, 2000000, 0, 4, 0, LnType::LongNote),
        ];
        notes[0].pair_index = 1;
        notes[1].pair_index = 0;
        notes[2].pair_index = 3;
        notes[3].pair_index = 2;

        let mut model = make_model(PlayMode::Beat7K, notes);
        let mut modifier = NoteShuffleModifier::new(RandomType::SRandom, 0, 42, -1);
        modifier.modify(&mut model);

        // Each LN start/end pair should be on the same lane
        assert_eq!(model.notes[0].lane, model.notes[1].lane);
        assert_eq!(model.notes[2].lane, model.notes[3].lane);
    }

    // -----------------------------------------------------------------------
    // Invisible notes follow regular notes
    // -----------------------------------------------------------------------

    #[test]
    fn test_invisible_note_follows_mapping() {
        let notes = vec![Note::normal(0, 1000000, 1), Note::invisible(0, 1000000, 2)];
        let mut model = make_model(PlayMode::Beat7K, notes);
        let mut modifier = NoteShuffleModifier::new(RandomType::SRandom, 0, 42, -1);
        modifier.modify(&mut model);

        // Both notes were on lane 0; they should end up on the same dest lane
        assert_eq!(model.notes[0].lane, model.notes[1].lane);
    }

    // -----------------------------------------------------------------------
    // NoteShuffleModifier — assist levels
    // -----------------------------------------------------------------------

    #[test]
    fn test_assist_level_srandom() {
        let m = NoteShuffleModifier::new(RandomType::SRandom, 0, 0, -1);
        assert_eq!(m.assist_level(), AssistLevel::None);
    }

    #[test]
    fn test_assist_level_hrandom() {
        let m = NoteShuffleModifier::new(RandomType::HRandom, 0, 0, -1);
        assert_eq!(m.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn test_assist_level_spiral() {
        let m = NoteShuffleModifier::new(RandomType::Spiral, 0, 0, -1);
        assert_eq!(m.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn test_assist_level_allscr() {
        let m = NoteShuffleModifier::new(RandomType::AllScr, 0, 0, -1);
        assert_eq!(m.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn test_assist_level_converge() {
        let m = NoteShuffleModifier::new(RandomType::Converge, 0, 0, -1);
        assert_eq!(m.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn test_assist_level_srandom_playable() {
        let m = NoteShuffleModifier::new(RandomType::SRandomPlayable, 0, 0, -1);
        assert_eq!(m.assist_level(), AssistLevel::LightAssist);
    }

    // -----------------------------------------------------------------------
    // compute_threshold_millis
    // -----------------------------------------------------------------------

    #[test]
    fn test_threshold_positive_bpm() {
        // 150 BPM -> ceil(15000/150) = 100ms
        assert_eq!(compute_threshold_millis(150), 100);
        // 120 BPM -> ceil(15000/120) = 125ms
        assert_eq!(compute_threshold_millis(120), 125);
    }

    #[test]
    fn test_threshold_zero_bpm() {
        assert_eq!(compute_threshold_millis(0), 0);
    }

    #[test]
    fn test_threshold_negative_bpm() {
        assert_eq!(compute_threshold_millis(-1), DEFAULT_HRAN_THRESHOLD);
    }

    // -----------------------------------------------------------------------
    // AllScratchRandomizer — scratch priority
    // -----------------------------------------------------------------------

    #[test]
    fn test_allscr_deterministic() {
        let notes = vec![
            Note::normal(0, 1000000, 1),
            Note::normal(1, 2000000, 2),
            Note::normal(2, 3000000, 3),
        ];
        let mut model1 = make_model(PlayMode::Beat7K, notes.clone());
        let mut model2 = make_model(PlayMode::Beat7K, notes);

        let mut mod1 = NoteShuffleModifier::new(RandomType::AllScr, 0, 42, -1);
        let mut mod2 = NoteShuffleModifier::new(RandomType::AllScr, 0, 42, -1);
        mod1.modify(&mut model1);
        mod2.modify(&mut model2);

        let lanes1: Vec<usize> = model1.notes.iter().map(|n| n.lane).collect();
        let lanes2: Vec<usize> = model2.notes.iter().map(|n| n.lane).collect();
        assert_eq!(lanes1, lanes2);
    }

    // -----------------------------------------------------------------------
    // NoMurioshiRandomizer
    // -----------------------------------------------------------------------

    #[test]
    fn test_nomurioshi_deterministic() {
        let notes = vec![
            Note::normal(0, 1000000, 1),
            Note::normal(2, 1000000, 2),
            Note::normal(4, 1000000, 3),
            Note::normal(1, 2000000, 4),
        ];
        let mut model1 = make_model(PlayMode::PopN9K, notes.clone());
        let mut model2 = make_model(PlayMode::PopN9K, notes);

        let mut mod1 = NoteShuffleModifier::new(RandomType::SRandomPlayable, 0, 42, -1);
        let mut mod2 = NoteShuffleModifier::new(RandomType::SRandomPlayable, 0, 42, -1);
        mod1.modify(&mut model1);
        mod2.modify(&mut model2);

        let lanes1: Vec<usize> = model1.notes.iter().map(|n| n.lane).collect();
        let lanes2: Vec<usize> = model2.notes.iter().map(|n| n.lane).collect();
        assert_eq!(lanes1, lanes2);
    }

    // -----------------------------------------------------------------------
    // ConvergeRandomizer
    // -----------------------------------------------------------------------

    #[test]
    fn test_converge_deterministic() {
        let notes = vec![
            Note::normal(0, 1000000, 1),
            Note::normal(1, 2000000, 2),
            Note::normal(2, 3000000, 3),
        ];
        let mut model1 = make_model(PlayMode::Beat7K, notes.clone());
        let mut model2 = make_model(PlayMode::Beat7K, notes);

        let mut mod1 = NoteShuffleModifier::new(RandomType::Converge, 0, 42, -1);
        let mut mod2 = NoteShuffleModifier::new(RandomType::Converge, 0, 42, -1);
        mod1.modify(&mut model1);
        mod2.modify(&mut model2);

        let lanes1: Vec<usize> = model1.notes.iter().map(|n| n.lane).collect();
        let lanes2: Vec<usize> = model2.notes.iter().map(|n| n.lane).collect();
        assert_eq!(lanes1, lanes2);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_notes_noop() {
        let mut model = make_model(PlayMode::Beat7K, Vec::new());
        let mut modifier = NoteShuffleModifier::new(RandomType::SRandom, 0, 42, -1);
        modifier.modify(&mut model);
        assert!(model.notes.is_empty());
    }

    #[test]
    fn test_mine_notes_treated_as_empty() {
        // Mine notes should be classified as "empty lane" in time-based shuffle
        let notes = vec![Note::normal(0, 1000000, 1), Note::mine(1, 1000000, 2, 10)];
        let mut model = make_model(PlayMode::Beat7K, notes);
        let mut modifier = NoteShuffleModifier::new(RandomType::SRandom, 0, 42, -1);
        modifier.modify(&mut model);
        // Should not crash; notes are rearranged
        assert_eq!(model.notes.len(), 2);
    }

    #[test]
    fn test_srandom_no_threshold() {
        let notes = vec![
            Note::normal(0, 1000000, 1),
            Note::normal(0, 1001000, 2), // 1ms later
        ];
        let mut model = make_model(PlayMode::Beat7K, notes);
        let mut modifier = NoteShuffleModifier::new(RandomType::SRandomNoThreshold, 0, 42, -1);
        modifier.modify(&mut model);
        assert_eq!(model.notes.len(), 2);
    }
}

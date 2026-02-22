use crate::bms_player_rule::BMSPlayerRule;
use crate::judge_algorithm::JudgeAlgorithm;
use crate::judge_property::{JudgeProperty, MissCondition, NoteType};
use crate::lane_property::LaneProperty;
use beatoraja_core::score_data::ScoreData;
use beatoraja_types::groove_gauge::GrooveGauge;
use bms_model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use bms_model::judge_note::{JUDGE_PR, JudgeNote};
use bms_model::mode::Mode;
use bms_model::note::{TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_LONGNOTE, TYPE_UNDEFINED};

/// HCN gauge change interval (microseconds)
const HCN_MDURATION: i64 = 200000;

/// Configuration for creating a testable JudgeManager.
pub struct JudgeConfig<'a> {
    pub notes: &'a [JudgeNote],
    pub mode: &'a Mode,
    pub ln_type: i32,
    pub judge_rank: i32,
    pub judge_window_rate: [i32; 3],
    pub scratch_judge_window_rate: [i32; 3],
    pub algorithm: JudgeAlgorithm,
    pub autoplay: bool,
    pub judge_property: &'a JudgeProperty,
    pub lane_property: Option<&'a LaneProperty>,
}

/// Internal per-note judge state (parallel to the external notes array).
#[derive(Clone, Debug)]
struct NoteJudgeState {
    state: i32,     // 0=unjudged, 1=PG+1, 2=GR+1, ..., 6=MS+1
    play_time: i64, // Timing difference in microseconds
}

/// Internal per-lane state for judge iteration.
struct LaneIterState {
    lane: usize,
    player: usize,
    offset: usize,
    sckey: i32,
    laneassign: Vec<usize>,
    note_indices: Vec<usize>,
    base_pos: usize,
    seek_pos: usize,
    processing: Option<usize>,
    passing: Option<usize>,
    inclease: bool,
    mpassingcount: i64,
    lnstart_judge: i32,
    lnstart_duration: i64,
    releasetime: i64,
    lnend_judge: i32,
}

impl LaneIterState {
    fn mark(&mut self, time_ms: i32, notes: &[JudgeNote]) {
        while self.base_pos < self.note_indices.len().saturating_sub(1)
            && (notes[self.note_indices[self.base_pos + 1]].time_us / 1000) < time_ms as i64
        {
            self.base_pos += 1;
        }
        while self.base_pos > 0
            && (notes[self.note_indices[self.base_pos]].time_us / 1000) > time_ms as i64
        {
            self.base_pos -= 1;
        }
        self.seek_pos = self.base_pos;
    }

    fn reset(&mut self) {
        self.seek_pos = self.base_pos;
    }

    fn get_note(&mut self) -> Option<usize> {
        if self.seek_pos < self.note_indices.len() {
            let idx = self.note_indices[self.seek_pos];
            self.seek_pos += 1;
            Some(idx)
        } else {
            None
        }
    }
}

/// Collector for simultaneous bad judgments.
struct MultiBadCollector {
    mjudge: Vec<[i64; 2]>,
    enabled: bool,
    note_list: Vec<usize>,
    time_list: Vec<i64>,
    size: usize,
    pub array_start: usize,
}

impl MultiBadCollector {
    fn new() -> Self {
        MultiBadCollector {
            mjudge: Vec::new(),
            enabled: true,
            note_list: Vec::with_capacity(256),
            time_list: Vec::with_capacity(256),
            size: 0,
            array_start: 0,
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !self.enabled {
            self.clear();
        }
    }

    fn clear(&mut self) {
        self.size = 0;
        self.array_start = 0;
        self.note_list.clear();
        self.time_list.clear();
    }

    fn set_judge(&mut self, mjudge: &[[i64; 2]]) {
        self.mjudge = mjudge.to_vec();
    }

    fn add(&mut self, note_idx: usize, dmtime: i64) {
        if !self.enabled {
            return;
        }
        self.note_list.push(note_idx);
        self.time_list.push(dmtime);
        self.size += 1;
    }

    fn filter(&mut self, tnote: Option<usize>, notes: &[JudgeNote]) {
        if !self.enabled || tnote.is_none() {
            return;
        }
        let tnote_idx = tnote.unwrap();

        // Find tnote's dmtime in the collector
        let mut tdmtime: i64 = -1;
        for i in 0..self.size {
            if self.note_list[i] == tnote_idx {
                tdmtime = self.time_list[i];
            }
        }
        if tdmtime == -1 {
            // tnote not in collector - should not happen
            return;
        }

        let good_start = self.mjudge[2][0];
        let good_end = self.mjudge[2][1];
        let bad_start = self.mjudge[3][0];
        let bad_end = self.mjudge[3][1];

        // Filter: keep only notes in bad range but not good range, excluding tnote
        let mut new_notes = Vec::new();
        let mut new_times = Vec::new();
        for i in 0..self.size {
            let dt = self.time_list[i];
            if dt < bad_start || dt > bad_end {
                continue;
            }
            if dt >= good_start && dt <= good_end {
                continue;
            }
            if self.note_list[i] == tnote_idx {
                continue;
            }
            new_notes.push(self.note_list[i]);
            new_times.push(self.time_list[i]);
        }
        self.note_list = new_notes;
        self.time_list = new_times;
        self.size = self.note_list.len();

        // Insertion sort by dmtime
        for i in 1..self.size {
            let mut j = i;
            while j > 0 && self.time_list[j - 1] > self.time_list[j] {
                self.note_list.swap(j - 1, j);
                self.time_list.swap(j - 1, j);
                j -= 1;
            }
        }

        // If tnote is not a bad or is a LN, remove all notes after tnote in time
        let tnote_is_bad = (bad_start <= tdmtime && tdmtime < good_start)
            || (good_end < tdmtime && tdmtime <= bad_end);
        if !tnote_is_bad || notes[tnote_idx].is_long() {
            for i in 0..self.size {
                if self.time_list[i] >= tdmtime {
                    self.size = i;
                    self.note_list.truncate(self.size);
                    self.time_list.truncate(self.size);
                    break;
                }
            }
        }

        // Remove preceding LNs before tnote
        self.array_start = self.size;
        for i in 0..self.size {
            if self.time_list[i] >= tdmtime || !notes[self.note_list[i]].is_long() {
                self.array_start = i;
                break;
            }
        }
    }
}

/// Note judge manager
pub struct JudgeManager {
    lntype: i32,
    score: ScoreData,
    combo: i32,
    coursecombo: i32,
    coursemaxcombo: i32,
    /// Judge laser color per player per lane
    judge: Vec<Vec<i32>>,
    /// Current judge display
    judgenow: Vec<i32>,
    judgecombo: Vec<i32>,
    /// Ghost record
    ghost: Vec<i32>,
    /// Judge timing difference (ms, + is early)
    judgefast: Vec<i64>,
    mjudgefast: Vec<i64>,
    keyassign: Vec<i32>,
    sckey: Vec<i32>,
    /// Note judge table
    nmjudge: Vec<[i64; 2]>,
    mjudgestart: i64,
    mjudgeend: i64,
    /// CN end judge table
    cnendmjudge: Vec<[i64; 2]>,
    nreleasemargin: i64,
    /// Scratch judge table
    smjudge: Vec<[i64; 2]>,
    scnendmjudge: Vec<[i64; 2]>,
    sreleasemargin: i64,
    /// PMS combo condition
    combocond: Vec<bool>,
    miss: MissCondition,
    /// Judge vanish flags
    judge_vanish: Vec<bool>,
    prevmtime: i64,
    autoplay: bool,
    auto_presstime: Vec<i64>,
    auto_minduration: i64,
    algorithm: JudgeAlgorithm,
    /// Recent 100 note judge timings
    recent_judges: Vec<i64>,
    micro_recent_judges: Vec<i64>,
    recent_judges_index: usize,
    presses_since_last_autoadjust: i32,
    /// Per-lane iteration state (only used with testable API)
    lane_states: Vec<LaneIterState>,
    /// Per-note internal judge state
    note_states: Vec<NoteJudgeState>,
    /// MultiBad collector
    multi_bad: MultiBadCollector,
    /// Total lane count
    lane_count: usize,
}

impl Default for JudgeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl JudgeManager {
    pub fn new() -> Self {
        JudgeManager {
            lntype: 0,
            score: ScoreData::default(),
            combo: 0,
            coursecombo: 0,
            coursemaxcombo: 0,
            judge: Vec::new(),
            judgenow: Vec::new(),
            judgecombo: Vec::new(),
            ghost: Vec::new(),
            judgefast: Vec::new(),
            mjudgefast: Vec::new(),
            keyassign: Vec::new(),
            sckey: Vec::new(),
            nmjudge: Vec::new(),
            mjudgestart: 0,
            mjudgeend: 0,
            cnendmjudge: Vec::new(),
            nreleasemargin: 0,
            smjudge: Vec::new(),
            scnendmjudge: Vec::new(),
            sreleasemargin: 0,
            combocond: Vec::new(),
            miss: MissCondition::One,
            judge_vanish: Vec::new(),
            prevmtime: 0,
            autoplay: false,
            auto_presstime: Vec::new(),
            auto_minduration: 80,
            algorithm: JudgeAlgorithm::Combo,
            recent_judges: vec![i64::MIN; 100],
            micro_recent_judges: vec![i64::MIN; 100],
            recent_judges_index: 0,
            presses_since_last_autoadjust: 0,
            lane_states: Vec::new(),
            note_states: Vec::new(),
            multi_bad: MultiBadCollector::new(),
            lane_count: 0,
        }
    }

    /// Create a JudgeManager from a JudgeConfig (testable API).
    pub fn from_config(config: &JudgeConfig) -> Self {
        let lp = config
            .lane_property
            .cloned()
            .unwrap_or_else(|| LaneProperty::new(config.mode));

        let lane_count = config.mode.key() as usize;
        let player_count = config.mode.player() as usize;
        let keys_per_player = lane_count / player_count;

        // Build judge windows
        let nmjudge = config.judge_property.get_judge(
            NoteType::Note,
            config.judge_rank,
            &config.judge_window_rate,
        );
        let cnendmjudge = config.judge_property.get_judge(
            NoteType::LongnoteEnd,
            config.judge_rank,
            &config.judge_window_rate,
        );
        let smjudge = config.judge_property.get_judge(
            NoteType::Scratch,
            config.judge_rank,
            &config.scratch_judge_window_rate,
        );
        let scnendmjudge = config.judge_property.get_judge(
            NoteType::LongscratchEnd,
            config.judge_rank,
            &config.scratch_judge_window_rate,
        );

        let mut mjudgestart: i64 = 0;
        let mut mjudgeend: i64 = 0;
        for l in &nmjudge {
            mjudgestart = mjudgestart.min(l[0]);
            mjudgeend = mjudgeend.max(l[1]);
        }
        for l in &smjudge {
            mjudgestart = mjudgestart.min(l[0]);
            mjudgeend = mjudgeend.max(l[1]);
        }

        // Build per-lane note index lists
        let mut lane_note_indices: Vec<Vec<usize>> = vec![Vec::new(); lane_count];
        for (i, note) in config.notes.iter().enumerate() {
            if note.lane < lane_count {
                lane_note_indices[note.lane].push(i);
            }
        }

        // Build LaneIterState for each lane
        let lane_key_assign = lp.get_lane_key_assign();
        let lane_scratch = lp.get_lane_scratch_assign();
        let lane_skin_offset = lp.get_lane_skin_offset();
        let lane_player = lp.get_lane_player();
        let mut lane_states = Vec::with_capacity(lane_count);
        for lane in 0..lane_count {
            let laneassign = if lane < lane_key_assign.len() {
                lane_key_assign[lane].iter().map(|&k| k as usize).collect()
            } else {
                vec![lane]
            };
            lane_states.push(LaneIterState {
                lane,
                player: if lane < lane_player.len() {
                    lane_player[lane] as usize
                } else {
                    0
                },
                offset: if lane < lane_skin_offset.len() {
                    lane_skin_offset[lane] as usize
                } else {
                    lane
                },
                sckey: if lane < lane_scratch.len() {
                    lane_scratch[lane]
                } else {
                    -1
                },
                laneassign,
                note_indices: lane_note_indices[lane].clone(),
                base_pos: 0,
                seek_pos: 0,
                processing: None,
                passing: None,
                inclease: false,
                mpassingcount: 0,
                lnstart_judge: 0,
                lnstart_duration: 0,
                releasetime: i64::MIN,
                lnend_judge: i32::MIN,
            });
        }

        // Count total playable notes for ghost array.
        // Mirrors Java TimeLine.getTotalNotes(lntype): for LNTYPE_LONGNOTE, LN end notes
        // with TYPE_UNDEFINED are not independently counted (only the LN start counts).
        let total_notes = config
            .notes
            .iter()
            .filter(|n| {
                if n.is_long_end()
                    && n.ln_type == TYPE_UNDEFINED
                    && config.ln_type == LNTYPE_LONGNOTE
                {
                    return false;
                }
                n.is_playable()
            })
            .count();

        let keyassign_vec: Vec<i32> = lp.get_key_lane_assign().to_vec();
        let num_keys = keyassign_vec.len();

        // Scratch key count
        let scratch_count = lp.get_scratch_key_assign().len();

        let mut jm = JudgeManager {
            lntype: config.ln_type,
            score: ScoreData::default(),
            combo: 0,
            coursecombo: 0,
            coursemaxcombo: 0,
            judge: vec![vec![0; keys_per_player + 1]; player_count],
            judgenow: vec![0; 1], // Default judgeregion=1
            judgecombo: vec![0; 1],
            ghost: vec![JUDGE_PR; total_notes],
            judgefast: vec![0; 1],
            mjudgefast: vec![0; 1],
            keyassign: keyassign_vec,
            sckey: vec![0; scratch_count],
            nmjudge,
            mjudgestart,
            mjudgeend,
            cnendmjudge,
            nreleasemargin: config.judge_property.longnote_margin,
            smjudge,
            scnendmjudge,
            sreleasemargin: config.judge_property.longscratch_margin,
            combocond: config.judge_property.combo.clone(),
            miss: config.judge_property.miss,
            judge_vanish: config.judge_property.judge_vanish.clone(),
            prevmtime: 0,
            autoplay: config.autoplay,
            auto_presstime: vec![i64::MIN; num_keys],
            auto_minduration: 80,
            algorithm: config.algorithm,
            recent_judges: vec![i64::MIN; 100],
            micro_recent_judges: vec![i64::MIN; 100],
            recent_judges_index: 0,
            presses_since_last_autoadjust: 0,
            lane_states,
            note_states: vec![
                NoteJudgeState {
                    state: 0,
                    play_time: 0,
                };
                config.notes.len()
            ],
            multi_bad: MultiBadCollector::new(),
            lane_count,
        };
        jm.score.notes = total_notes as i32;
        jm
    }

    /// Main judge update loop (testable API).
    ///
    /// Translates the Java JudgeManager.update() method verbatim.
    /// Called once per frame with current music time, notes, key states, and gauge.
    pub fn update(
        &mut self,
        mtime: i64,
        notes: &[JudgeNote],
        key_states: &[bool],
        key_changed_times: &[i64],
        gauge: &mut GrooveGauge,
    ) {
        let lane_count = self.lane_count;

        // --- Pass-through loop ---
        for lane_idx in 0..lane_count {
            self.lane_states[lane_idx].mark(
                ((self.prevmtime + self.mjudgestart - 100000) / 1000) as i32,
                notes,
            );
            let mut next_inclease = false;

            // Check if any key assigned to this lane is pressed
            let mut pressed = false;
            for &key in &self.lane_states[lane_idx].laneassign {
                if key < key_states.len() && key_states[key] {
                    pressed = true;
                    break;
                }
            }

            // Iterate notes from prevmtime to mtime
            #[allow(clippy::while_let_loop)]
            loop {
                let note_idx = match self.lane_states[lane_idx].get_note() {
                    Some(idx) => idx,
                    None => break,
                };
                if notes[note_idx].time_us > mtime {
                    break;
                }
                if notes[note_idx].time_us <= self.prevmtime {
                    continue;
                }

                // HCN handling
                if notes[note_idx].is_long() {
                    let ln_type = notes[note_idx].ln_type;
                    let is_end = notes[note_idx].is_long_end();
                    if (ln_type == TYPE_UNDEFINED && self.lntype == LNTYPE_LONGNOTE + 2)
                        || ln_type == TYPE_HELLCHARGENOTE
                    {
                        if is_end {
                            self.lane_states[lane_idx].passing = None;
                            self.lane_states[lane_idx].mpassingcount = 0;
                        } else {
                            self.lane_states[lane_idx].passing = Some(note_idx);
                        }
                    }
                } else if notes[note_idx].is_mine() && pressed {
                    // Mine note damage
                    gauge.add_value(-(notes[note_idx].damage as f32));
                }

                // Autoplay processing
                if self.autoplay {
                    if notes[note_idx].is_normal() && self.note_states[note_idx].state == 0 {
                        let first_key = self.lane_states[lane_idx].laneassign[0];
                        self.auto_presstime[first_key] = mtime;
                        self.update_micro(
                            lane_idx, note_idx, notes, mtime, 0, 0, true, false, gauge,
                        );
                    }
                    if notes[note_idx].is_long() {
                        let ln_type = notes[note_idx].ln_type;
                        if notes[note_idx].is_long_start()
                            && self.note_states[note_idx].state == 0
                            && self.lane_states[lane_idx].processing.is_none()
                        {
                            let first_key = self.lane_states[lane_idx].laneassign[0];
                            self.auto_presstime[first_key] = mtime;
                            if (self.lntype == LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                                || ln_type == TYPE_LONGNOTE
                            {
                                self.lane_states[lane_idx].mpassingcount = 0;
                                let player = self.lane_states[lane_idx].player;
                                let offset = self.lane_states[lane_idx].offset;
                                if player < self.judge.len() && offset < self.judge[player].len() {
                                    self.judge[player][offset] = 8;
                                }
                            } else {
                                self.update_micro(
                                    lane_idx, note_idx, notes, mtime, 0, 0, true, false, gauge,
                                );
                            }
                            let pair_idx = notes[note_idx].pair_index;
                            self.lane_states[lane_idx].processing = pair_idx;
                        }
                        if notes[note_idx].is_long_end()
                            && self.note_states[note_idx].state == 0
                            && ((self.lntype != LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                                || ln_type == TYPE_CHARGENOTE
                                || ln_type == TYPE_HELLCHARGENOTE)
                        {
                            let sc = self.lane_states[lane_idx].sckey;
                            if sc >= 0 && self.lane_states[lane_idx].laneassign.len() >= 2 {
                                let first_key = self.lane_states[lane_idx].laneassign[0];
                                let second_key = self.lane_states[lane_idx].laneassign[1];
                                self.auto_presstime[first_key] = i64::MIN;
                                self.auto_presstime[second_key] = mtime;
                            }
                            self.update_micro(
                                lane_idx, note_idx, notes, mtime, 0, 0, true, false, gauge,
                            );
                            self.lane_states[lane_idx].processing = None;
                        }
                    }
                }
            }

            // HCN gauge increase/decrease check
            if let Some(passing_idx) = self.lane_states[lane_idx].passing {
                let pair_idx = notes[passing_idx].pair_index;
                let pair_state = pair_idx
                    .filter(|&pi| pi < self.note_states.len())
                    .map(|pi| self.note_states[pi].state)
                    .unwrap_or(0);
                if pressed || (pair_state > 0 && pair_state <= 3) || self.autoplay {
                    next_inclease = true;
                }
            }

            // Autoplay key release timing
            if self.autoplay {
                for &key in &self.lane_states[lane_idx].laneassign {
                    if key < self.auto_presstime.len()
                        && self.auto_presstime[key] != i64::MIN
                        && mtime - self.auto_presstime[key] > self.auto_minduration
                        && self.lane_states[lane_idx].processing.is_none()
                    {
                        self.auto_presstime[key] = i64::MIN;
                    }
                }
            }
            self.lane_states[lane_idx].inclease = next_inclease;
        }

        // --- HCN gauge change loop ---
        for lane_idx in 0..lane_count {
            let passing = self.lane_states[lane_idx].passing;
            if passing.is_none() {
                continue;
            }
            let passing_idx = passing.unwrap();
            if self.note_states[passing_idx].state == 0 {
                continue;
            }

            if self.lane_states[lane_idx].inclease {
                self.lane_states[lane_idx].mpassingcount += mtime - self.prevmtime;
                if self.lane_states[lane_idx].mpassingcount > HCN_MDURATION {
                    gauge.update_with_rate(1, 0.5);
                    self.lane_states[lane_idx].mpassingcount -= HCN_MDURATION;
                }
            } else {
                self.lane_states[lane_idx].mpassingcount -= mtime - self.prevmtime;
                if self.lane_states[lane_idx].mpassingcount < -HCN_MDURATION {
                    gauge.update_with_rate(3, 0.5);
                    self.lane_states[lane_idx].mpassingcount += HCN_MDURATION;
                }
            }
        }
        self.prevmtime = mtime;

        // --- Key press/release processing ---
        for key in 0..self.keyassign.len() {
            let lane = self.keyassign[key];
            if lane == -1 {
                continue;
            }
            let lane_idx = lane as usize;
            if lane_idx >= lane_count {
                continue;
            }
            if key >= key_changed_times.len() {
                continue;
            }
            let pmtime = key_changed_times[key];
            if pmtime == i64::MIN {
                continue;
            }
            self.lane_states[lane_idx].reset();
            let sc = self.lane_states[lane_idx].sckey;

            if key < key_states.len() && key_states[key] {
                // Key pressed
                if let Some(proc_idx) = self.lane_states[lane_idx].processing {
                    let proc_ln_type = notes[proc_idx].ln_type;
                    if (self.lntype != LNTYPE_LONGNOTE && proc_ln_type == TYPE_UNDEFINED)
                        || proc_ln_type == TYPE_CHARGENOTE
                        || proc_ln_type == TYPE_HELLCHARGENOTE
                    {
                        if sc >= 0
                            && (sc as usize) < self.sckey.len()
                            && key as i32 != self.sckey[sc as usize]
                        {
                            // BSS end processing
                            let mjudge = &self.scnendmjudge;
                            let dmtime = notes[proc_idx].time_us - pmtime;
                            let mut j = 0;
                            while j < mjudge.len()
                                && !(dmtime >= mjudge[j][0] && dmtime <= mjudge[j][1])
                            {
                                j += 1;
                            }
                            self.update_micro(
                                lane_idx, proc_idx, notes, mtime, j as i32, dmtime, true, false,
                                gauge,
                            );
                            self.lane_states[lane_idx].processing = None;
                            self.lane_states[lane_idx].releasetime = i64::MIN;
                            self.lane_states[lane_idx].lnend_judge = i32::MIN;
                            self.sckey[sc as usize] = 0;
                        } else {
                            // Re-press: cancel pending release
                            self.lane_states[lane_idx].releasetime = i64::MIN;
                        }
                    } else {
                        // Re-press for LN
                        self.lane_states[lane_idx].releasetime = i64::MIN;
                    }
                } else {
                    // No LN processing - find target note
                    let mjudge = if sc >= 0 {
                        self.smjudge.clone()
                    } else {
                        self.nmjudge.clone()
                    };
                    self.lane_states[lane_idx].reset();
                    let mut tnote: Option<usize> = None;
                    let mut best_judge: i32 = 0;
                    self.multi_bad.clear();
                    self.multi_bad.set_judge(&mjudge);

                    // Scan notes for best match
                    #[allow(clippy::while_let_loop)]
                    #[allow(clippy::nonminimal_bool)]
                    loop {
                        let note_idx = match self.lane_states[lane_idx].get_note() {
                            Some(idx) => idx,
                            None => break,
                        };
                        let dmtime = notes[note_idx].time_us - pmtime;
                        if dmtime >= self.mjudgeend {
                            break;
                        }
                        if dmtime < self.mjudgestart {
                            continue;
                        }
                        // Skip mine notes and LN end notes
                        if notes[note_idx].is_mine() || notes[note_idx].is_long_end() {
                            continue;
                        }
                        if self.note_states[note_idx].state == 0 {
                            self.multi_bad.add(note_idx, dmtime);
                        }

                        let tnote_state = tnote.map(|ti| self.note_states[ti].state).unwrap_or(-1);
                        if tnote.is_none()
                            || tnote_state != 0
                            || self.algorithm.compare_times(
                                tnote.map(|ti| notes[ti].time_us).unwrap_or(0),
                                notes[note_idx].time_us,
                                self.note_states[note_idx].state,
                                pmtime,
                                &mjudge,
                            )
                        {
                            let note_state = self.note_states[note_idx].state;
                            let note_play_time = self.note_states[note_idx].play_time;
                            // MissCondition::One check
                            if self.miss == MissCondition::One
                                && (note_state != 0
                                    || (note_state == 0
                                        && note_play_time != 0
                                        && (dmtime > mjudge[2][1] || dmtime < mjudge[2][0])))
                            {
                                continue;
                            }

                            let judge;
                            if note_state != 0 {
                                judge = if dmtime >= mjudge[4][0] && dmtime <= mjudge[4][1] {
                                    5
                                } else {
                                    6
                                };
                            } else if notes[note_idx].is_long_start() && dmtime < mjudge[2][0] {
                                // LR2oraja: Remove late bad for LN
                                judge = 6;
                            } else {
                                let mut j = 0;
                                while j < mjudge.len()
                                    && !(dmtime >= mjudge[j][0] && dmtime <= mjudge[j][1])
                                {
                                    j += 1;
                                }
                                judge = if j >= 4 { j as i32 + 1 } else { j as i32 };
                            }

                            if judge < 6 {
                                if judge < 4
                                    || tnote.is_none()
                                    || (tnote.map(|ti| notes[ti].time_us).unwrap_or(0) - pmtime)
                                        .abs()
                                        > (notes[note_idx].time_us - pmtime).abs()
                                {
                                    tnote = Some(note_idx);
                                    best_judge = judge;
                                }
                            } else {
                                tnote = None;
                            }
                        }
                    }
                    self.multi_bad.filter(tnote, notes);

                    if let Some(tnote_idx) = tnote {
                        // Process multi-bad notes
                        for i in self.multi_bad.array_start..self.multi_bad.size {
                            let bad_idx = self.multi_bad.note_list[i];
                            let bad_time = self.multi_bad.time_list[i];
                            let vanish = self.judge_vanish[3];
                            self.update_micro(
                                lane_idx, bad_idx, notes, mtime, 3, bad_time, vanish, true, gauge,
                            );
                        }

                        if notes[tnote_idx].is_long_start() {
                            // Long note press processing
                            let dmtime = notes[tnote_idx].time_us - pmtime;
                            let ln_type = notes[tnote_idx].ln_type;
                            if (self.lntype == LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                                || ln_type == TYPE_LONGNOTE
                            {
                                // LN processing
                                if self.judge_vanish[best_judge as usize] {
                                    self.lane_states[lane_idx].lnstart_judge = best_judge;
                                    self.lane_states[lane_idx].lnstart_duration = dmtime;
                                    self.lane_states[lane_idx].processing =
                                        notes[tnote_idx].pair_index;
                                    self.lane_states[lane_idx].releasetime = i64::MIN;
                                    self.lane_states[lane_idx].lnend_judge = i32::MIN;
                                    if sc >= 0 && (sc as usize) < self.sckey.len() {
                                        self.sckey[sc as usize] = key as i32;
                                    }
                                    let player = self.lane_states[lane_idx].player;
                                    let offset = self.lane_states[lane_idx].offset;
                                    if player < self.judge.len()
                                        && offset < self.judge[player].len()
                                    {
                                        self.judge[player][offset] = 8;
                                    }
                                } else {
                                    self.update_micro(
                                        lane_idx, tnote_idx, notes, mtime, best_judge, dmtime,
                                        false, false, gauge,
                                    );
                                }
                            } else {
                                // CN, HCN press processing
                                if self.judge_vanish[best_judge as usize] {
                                    self.lane_states[lane_idx].processing =
                                        notes[tnote_idx].pair_index;
                                    self.lane_states[lane_idx].releasetime = i64::MIN;
                                    self.lane_states[lane_idx].lnend_judge = i32::MIN;
                                    if sc >= 0 && (sc as usize) < self.sckey.len() {
                                        self.sckey[sc as usize] = key as i32;
                                    }
                                }
                                let vanish = self.judge_vanish[best_judge as usize];
                                self.update_micro(
                                    lane_idx, tnote_idx, notes, mtime, best_judge, dmtime, vanish,
                                    false, gauge,
                                );
                            }
                        } else {
                            // Normal note processing
                            let dmtime = notes[tnote_idx].time_us - pmtime;
                            let vanish = self.judge_vanish[best_judge as usize];
                            self.update_micro(
                                lane_idx, tnote_idx, notes, mtime, best_judge, dmtime, vanish,
                                false, gauge,
                            );
                        }
                    } else {
                        // Empty POOR - no matching note
                        let player = self.lane_states[lane_idx].player;
                        let offset = self.lane_states[lane_idx].offset;
                        if player < self.judge.len() && offset < self.judge[player].len() {
                            self.judge[player][offset] = 0;
                        }
                    }
                }
            } else {
                // Key released
                if let Some(proc_idx) = self.lane_states[lane_idx].processing {
                    let proc_ln_type = notes[proc_idx].ln_type;
                    let mjudge = if sc >= 0 {
                        &self.scnendmjudge
                    } else {
                        &self.cnendmjudge
                    };
                    let dmtime = notes[proc_idx].time_us - pmtime;
                    let mut judge = 0;
                    while judge < mjudge.len() as i32
                        && !(dmtime >= mjudge[judge as usize][0]
                            && dmtime <= mjudge[judge as usize][1])
                    {
                        judge += 1;
                    }

                    if (self.lntype != LNTYPE_LONGNOTE && proc_ln_type == TYPE_UNDEFINED)
                        || proc_ln_type == TYPE_CHARGENOTE
                        || proc_ln_type == TYPE_HELLCHARGENOTE
                    {
                        // CN, HCN release
                        let mut release = true;
                        if sc >= 0 && (sc as usize) < self.sckey.len() {
                            if judge != 4 || key as i32 != self.sckey[sc as usize] {
                                release = false;
                            } else {
                                self.sckey[sc as usize] = 0;
                            }
                        }
                        if release {
                            if judge >= 3 && dmtime > 0 {
                                self.lane_states[lane_idx].releasetime = mtime;
                                self.lane_states[lane_idx].lnend_judge = judge;
                            } else {
                                self.update_micro(
                                    lane_idx, proc_idx, notes, mtime, judge, dmtime, true, false,
                                    gauge,
                                );
                                self.lane_states[lane_idx].processing = None;
                                self.lane_states[lane_idx].releasetime = i64::MIN;
                                self.lane_states[lane_idx].lnend_judge = i32::MIN;
                            }
                        }
                    } else {
                        // LN release
                        let mut release = true;
                        if sc >= 0 && (sc as usize) < self.sckey.len() {
                            if key as i32 != self.sckey[sc as usize] {
                                release = false;
                            } else {
                                self.sckey[sc as usize] = 0;
                            }
                        }
                        if release {
                            judge = judge.max(self.lane_states[lane_idx].lnstart_judge);
                            let mut dmtime = dmtime;
                            if self.lane_states[lane_idx].lnstart_duration.abs() > dmtime.abs() {
                                dmtime = self.lane_states[lane_idx].lnstart_duration;
                            }
                            if judge >= 3 && dmtime > 0 {
                                self.lane_states[lane_idx].releasetime = mtime;
                                self.lane_states[lane_idx].lnend_judge = 3;
                            } else {
                                // Get pair of processing note for LN
                                let pair_of_proc = notes[proc_idx].pair_index;
                                let judge_note = pair_of_proc.unwrap_or(proc_idx);
                                self.update_micro(
                                    lane_idx,
                                    judge_note,
                                    notes,
                                    mtime,
                                    judge.min(3),
                                    dmtime,
                                    true,
                                    false,
                                    gauge,
                                );
                                self.lane_states[lane_idx].processing = None;
                                self.lane_states[lane_idx].releasetime = i64::MIN;
                                self.lane_states[lane_idx].lnend_judge = i32::MIN;
                            }
                        }
                    }
                }
            }
        }

        // --- Miss POOR and LN end processing ---
        for lane_idx in 0..lane_count {
            let sc = self.lane_states[lane_idx].sckey;
            let mjudge = if sc >= 0 {
                self.smjudge.clone()
            } else {
                self.nmjudge.clone()
            };
            let releasemargin = if sc >= 0 {
                self.sreleasemargin
            } else {
                self.nreleasemargin
            };

            // LN end processing
            if let Some(proc_idx) = self.lane_states[lane_idx].processing {
                let proc_ln_type = notes[proc_idx].ln_type;
                if (self.lntype == LNTYPE_LONGNOTE && proc_ln_type == TYPE_UNDEFINED)
                    || proc_ln_type == TYPE_LONGNOTE
                {
                    if self.lane_states[lane_idx].releasetime != i64::MIN
                        && self.lane_states[lane_idx].releasetime + releasemargin <= mtime
                    {
                        let pair_of_proc = notes[proc_idx].pair_index.unwrap_or(proc_idx);
                        let lnend_judge = self.lane_states[lane_idx].lnend_judge;
                        let release_dmtime =
                            notes[proc_idx].time_us - self.lane_states[lane_idx].releasetime;
                        self.update_micro(
                            lane_idx,
                            pair_of_proc,
                            notes,
                            mtime,
                            lnend_judge,
                            release_dmtime,
                            true,
                            false,
                            gauge,
                        );
                        self.lane_states[lane_idx].processing = None;
                        self.lane_states[lane_idx].releasetime = i64::MIN;
                        self.lane_states[lane_idx].lnend_judge = i32::MIN;
                    } else if notes[proc_idx].time_us < mtime {
                        let pair_of_proc = notes[proc_idx].pair_index.unwrap_or(proc_idx);
                        let lnstart_judge = self.lane_states[lane_idx].lnstart_judge;
                        let lnstart_duration = self.lane_states[lane_idx].lnstart_duration;
                        self.update_micro(
                            lane_idx,
                            pair_of_proc,
                            notes,
                            mtime,
                            lnstart_judge,
                            lnstart_duration,
                            true,
                            false,
                            gauge,
                        );
                        self.lane_states[lane_idx].processing = None;
                        self.lane_states[lane_idx].releasetime = i64::MIN;
                        self.lane_states[lane_idx].lnend_judge = i32::MIN;
                    }
                } else if self.lane_states[lane_idx].releasetime != i64::MIN
                    && self.lane_states[lane_idx].releasetime + releasemargin <= mtime
                {
                    let lnend_judge = self.lane_states[lane_idx].lnend_judge;
                    let release_dmtime =
                        notes[proc_idx].time_us - self.lane_states[lane_idx].releasetime;
                    self.update_micro(
                        lane_idx,
                        proc_idx,
                        notes,
                        mtime,
                        lnend_judge,
                        release_dmtime,
                        true,
                        false,
                        gauge,
                    );
                    self.lane_states[lane_idx].processing = None;
                    self.lane_states[lane_idx].releasetime = i64::MIN;
                    self.lane_states[lane_idx].lnend_judge = i32::MIN;
                }
            }

            // Miss POOR detection
            self.lane_states[lane_idx].reset();
            #[allow(clippy::while_let_loop)]
            loop {
                let note_idx = match self.lane_states[lane_idx].get_note() {
                    Some(idx) => idx,
                    None => break,
                };
                if notes[note_idx].time_us >= mtime + mjudge[3][0] {
                    break;
                }
                let mjud = notes[note_idx].time_us - mtime;

                if notes[note_idx].is_normal() && self.note_states[note_idx].state == 0 {
                    self.update_micro(
                        lane_idx, note_idx, notes, mtime, 4, mjud, true, false, gauge,
                    );
                } else if notes[note_idx].is_long() {
                    let ln_type = notes[note_idx].ln_type;
                    if notes[note_idx].is_long_start() && self.note_states[note_idx].state == 0 {
                        if (self.lntype != LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                            || ln_type == TYPE_CHARGENOTE
                            || ln_type == TYPE_HELLCHARGENOTE
                        {
                            self.update_micro(
                                lane_idx, note_idx, notes, mtime, 4, mjud, true, false, gauge,
                            );
                            if let Some(pair_idx) = notes[note_idx].pair_index {
                                self.update_micro(
                                    lane_idx, pair_idx, notes, mtime, 4, mjud, true, false, gauge,
                                );
                            }
                        }
                        if ((self.lntype == LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                            || ln_type == TYPE_LONGNOTE)
                            && self.lane_states[lane_idx].processing != notes[note_idx].pair_index
                        {
                            self.update_micro(
                                lane_idx, note_idx, notes, mtime, 4, mjud, true, false, gauge,
                            );
                        }
                    }
                    if ((self.lntype != LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                        || ln_type == TYPE_CHARGENOTE
                        || ln_type == TYPE_HELLCHARGENOTE)
                        && notes[note_idx].is_long_end()
                        && self.note_states[note_idx].state == 0
                    {
                        self.update_micro(
                            lane_idx, note_idx, notes, mtime, 4, mjud, true, false, gauge,
                        );
                        self.lane_states[lane_idx].processing = None;
                        self.lane_states[lane_idx].releasetime = i64::MIN;
                        self.lane_states[lane_idx].lnend_judge = i32::MIN;
                        let sc = self.lane_states[lane_idx].sckey;
                        if sc >= 0 && (sc as usize) < self.sckey.len() {
                            self.sckey[sc as usize] = 0;
                        }
                    }
                }
            }
        }
    }

    /// Internal judge update: records score, combo, ghost, and gauge changes.
    #[allow(clippy::too_many_arguments)]
    fn update_micro(
        &mut self,
        lane_idx: usize,
        note_idx: usize,
        notes: &[JudgeNote],
        _mtime: i64,
        judge: i32,
        mfast: i64,
        judge_vanish: bool,
        multi_bad: bool,
        gauge: &mut GrooveGauge,
    ) {
        let _ = notes; // used for type info if needed in future
        if note_idx >= self.note_states.len() {
            return;
        }
        if judge_vanish {
            if (self.score.passnotes as usize) < self.ghost.len() {
                self.ghost[self.score.passnotes as usize] = judge;
            }
            self.note_states[note_idx].state = judge + 1;
            self.score.passnotes += 1;
        }
        if self.miss == MissCondition::One
            && judge == 4
            && self.note_states[note_idx].play_time != 0
        {
            return;
        }
        self.note_states[note_idx].play_time = mfast;
        self.score.add_judge_count(judge, mfast >= 0, 1);

        if judge < 4 {
            self.recent_judges_index = (self.recent_judges_index + 1) % self.recent_judges.len();
            self.recent_judges[self.recent_judges_index] = mfast / 1000;
            self.micro_recent_judges[self.recent_judges_index] = mfast;
        }

        if (judge as usize) < self.combocond.len() && self.combocond[judge as usize] && judge < 5 {
            self.combo += 1;
            self.score.combo = self.score.combo.max(self.combo);
            self.coursecombo += 1;
            self.coursemaxcombo = self.coursemaxcombo.max(self.coursecombo);
        }
        if (judge as usize) < self.combocond.len() && !self.combocond[judge as usize] {
            self.combo = 0;
            self.coursecombo = 0;
        }

        if judge != 4 {
            let player = self.lane_states[lane_idx].player;
            let offset = self.lane_states[lane_idx].offset;
            if player < self.judge.len() && offset < self.judge[player].len() {
                self.judge[player][offset] = if judge == 0 {
                    1
                } else {
                    judge * 2 + if mfast > 0 { 0 } else { 1 }
                };
            }
        }

        if !multi_bad {
            gauge.update(judge);
        }
    }

    // --- Legacy API (backward compat) ---

    pub fn init(&mut self, model: &BMSModel, judgeregion: i32) {
        self.prevmtime = 0;
        self.judgenow = vec![0; judgeregion as usize];
        self.judgecombo = vec![0; judgeregion as usize];
        self.judgefast = vec![0; judgeregion as usize];
        self.mjudgefast = vec![0; judgeregion as usize];

        let orgmode = model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
        self.score = ScoreData::default();
        self.score.notes = model.get_total_notes();

        self.ghost = vec![4; model.get_total_notes() as usize];
        self.lntype = model.get_lntype();

        let rule = BMSPlayerRule::get_bms_player_rule(&orgmode);
        let judgerank = model.get_judgerank();
        let key_judge_window_rate = [100, 100, 100];
        let scratch_judge_window_rate = [100, 100, 100];

        self.combocond = rule.judge.combo.clone();
        self.miss = rule.judge.miss;
        self.judge_vanish = rule.judge.judge_vanish.clone();

        self.nmjudge = rule
            .judge
            .get_judge(NoteType::Note, judgerank, &key_judge_window_rate);
        self.cnendmjudge =
            rule.judge
                .get_judge(NoteType::LongnoteEnd, judgerank, &key_judge_window_rate);
        self.nreleasemargin = rule.judge.longnote_margin;
        self.smjudge =
            rule.judge
                .get_judge(NoteType::Scratch, judgerank, &scratch_judge_window_rate);
        self.scnendmjudge = rule.judge.get_judge(
            NoteType::LongscratchEnd,
            judgerank,
            &scratch_judge_window_rate,
        );
        self.sreleasemargin = rule.judge.longscratch_margin;

        self.mjudgestart = 0;
        self.mjudgeend = 0;
        for l in &self.nmjudge {
            self.mjudgestart = self.mjudgestart.min(l[0]);
            self.mjudgeend = self.mjudgeend.max(l[1]);
        }
        for l in &self.smjudge {
            self.mjudgestart = self.mjudgestart.min(l[0]);
            self.mjudgeend = self.mjudgeend.max(l[1]);
        }

        let player_count = orgmode.player();
        let keys_per_player = orgmode.key() / player_count;
        self.judge = vec![vec![0; keys_per_player as usize + 1]; player_count as usize];

        self.recent_judges = vec![i64::MIN; 100];
        self.micro_recent_judges = vec![i64::MIN; 100];
        self.recent_judges_index = 0;
        self.presses_since_last_autoadjust = 0;
    }

    // --- Getters ---

    pub fn score(&self) -> &ScoreData {
        &self.score
    }

    pub fn max_combo(&self) -> i32 {
        self.score.combo
    }

    pub fn ghost(&self) -> Vec<usize> {
        self.ghost.iter().map(|&g| g as usize).collect()
    }

    pub fn past_notes(&self) -> i32 {
        self.score.passnotes
    }

    pub fn get_recent_judges(&self) -> &[i64] {
        &self.recent_judges
    }

    pub fn get_micro_recent_judges(&self) -> &[i64] {
        &self.micro_recent_judges
    }

    pub fn get_recent_judges_index(&self) -> usize {
        self.recent_judges_index
    }

    pub fn get_recent_judge_timing(&self, player: usize) -> i64 {
        if player < self.judgefast.len() {
            self.judgefast[player]
        } else {
            0
        }
    }

    pub fn get_recent_judge_micro_timing(&self, player: usize) -> i64 {
        if player < self.mjudgefast.len() {
            self.mjudgefast[player]
        } else {
            0
        }
    }

    pub fn get_processing_long_note(&self, lane: usize) -> Option<usize> {
        if lane < self.lane_states.len() {
            self.lane_states[lane].processing
        } else {
            None
        }
    }

    pub fn get_passing_long_note(&self, lane: usize) -> Option<usize> {
        if lane < self.lane_states.len() {
            self.lane_states[lane].passing
        } else {
            None
        }
    }

    pub fn get_hell_charge_judge(&self, lane: usize) -> bool {
        if lane < self.lane_states.len() {
            self.lane_states[lane].inclease
        } else {
            false
        }
    }

    pub fn get_auto_presstime(&self) -> &[i64] {
        &self.auto_presstime
    }

    pub fn get_combo(&self) -> i32 {
        self.combo
    }

    pub fn get_course_combo(&self) -> i32 {
        self.coursecombo
    }

    pub fn set_course_combo(&mut self, combo: i32) {
        self.coursecombo = combo;
    }

    pub fn get_course_maxcombo(&self) -> i32 {
        self.coursemaxcombo
    }

    pub fn set_course_maxcombo(&mut self, combo: i32) {
        self.coursemaxcombo = combo;
    }

    pub fn get_judge_time_region(&self, lane: usize) -> &[[i64; 2]] {
        if lane < self.lane_states.len() && self.lane_states[lane].sckey >= 0 {
            &self.smjudge
        } else {
            &self.nmjudge
        }
    }

    pub fn get_score_data(&self) -> &ScoreData {
        &self.score
    }

    pub fn get_judge_count(&self, judge: i32) -> i32 {
        self.score.get_judge_count_total(judge)
    }

    pub fn get_judge_count_fast(&self, judge: i32, fast: bool) -> i32 {
        self.score.get_judge_count(judge, fast)
    }

    pub fn get_now_judge(&self, player: usize) -> i32 {
        if player < self.judgenow.len() {
            self.judgenow[player]
        } else {
            0
        }
    }

    pub fn get_now_combo(&self, player: usize) -> i32 {
        if player < self.judgecombo.len() {
            self.judgecombo[player]
        } else {
            0
        }
    }

    pub fn get_judge_table(&self, sc: bool) -> &[[i64; 2]] {
        if sc { &self.smjudge } else { &self.nmjudge }
    }

    pub fn get_past_notes(&self) -> i32 {
        self.score.passnotes
    }

    pub fn get_ghost(&self) -> &[i32] {
        &self.ghost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::judge_note::{JUDGE_PG, build_judge_notes};
    use bms_model::note::Note;
    use bms_model::time_line::TimeLine;

    #[test]
    fn new_creates_default_state() {
        let jm = JudgeManager::new();
        assert_eq!(jm.get_combo(), 0);
        assert_eq!(jm.get_course_combo(), 0);
        assert_eq!(jm.get_course_maxcombo(), 0);
    }

    #[test]
    fn default_is_same_as_new() {
        let jm1 = JudgeManager::new();
        let jm2 = JudgeManager::default();
        assert_eq!(jm1.get_combo(), jm2.get_combo());
        assert_eq!(jm1.get_course_combo(), jm2.get_course_combo());
        assert_eq!(jm1.get_course_maxcombo(), jm2.get_course_maxcombo());
    }

    #[test]
    fn recent_judges_initialized_to_min() {
        let jm = JudgeManager::new();
        let judges = jm.get_recent_judges();
        assert_eq!(judges.len(), 100);
        for &j in judges {
            assert_eq!(j, i64::MIN);
        }
    }

    #[test]
    fn micro_recent_judges_initialized_to_min() {
        let jm = JudgeManager::new();
        let judges = jm.get_micro_recent_judges();
        assert_eq!(judges.len(), 100);
        for &j in judges {
            assert_eq!(j, i64::MIN);
        }
    }

    #[test]
    fn recent_judges_index_starts_at_zero() {
        let jm = JudgeManager::new();
        assert_eq!(jm.get_recent_judges_index(), 0);
    }

    #[test]
    fn set_course_combo() {
        let mut jm = JudgeManager::new();
        jm.set_course_combo(42);
        assert_eq!(jm.get_course_combo(), 42);
    }

    #[test]
    fn set_course_maxcombo() {
        let mut jm = JudgeManager::new();
        jm.set_course_maxcombo(100);
        assert_eq!(jm.get_course_maxcombo(), 100);
    }

    #[test]
    fn get_now_judge_out_of_bounds_returns_zero() {
        let jm = JudgeManager::new();
        assert_eq!(jm.get_now_judge(0), 0);
        assert_eq!(jm.get_now_judge(100), 0);
    }

    #[test]
    fn get_now_combo_out_of_bounds_returns_zero() {
        let jm = JudgeManager::new();
        assert_eq!(jm.get_now_combo(0), 0);
        assert_eq!(jm.get_now_combo(100), 0);
    }

    #[test]
    fn get_recent_judge_timing_out_of_bounds_returns_zero() {
        let jm = JudgeManager::new();
        assert_eq!(jm.get_recent_judge_timing(0), 0);
        assert_eq!(jm.get_recent_judge_timing(100), 0);
    }

    #[test]
    fn get_recent_judge_micro_timing_out_of_bounds_returns_zero() {
        let jm = JudgeManager::new();
        assert_eq!(jm.get_recent_judge_micro_timing(0), 0);
        assert_eq!(jm.get_recent_judge_micro_timing(100), 0);
    }

    #[test]
    fn init_sets_up_judge_tables() {
        let mut jm = JudgeManager::new();
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        jm.init(&model, 1);

        assert_eq!(jm.get_now_judge(0), 0);
        let table = jm.get_judge_table(false);
        assert!(!table.is_empty());
        let sc_table = jm.get_judge_table(true);
        assert!(!sc_table.is_empty());
    }

    #[test]
    fn init_sets_up_ghost_array() {
        let mut jm = JudgeManager::new();
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        jm.init(&model, 1);

        let ghost = jm.get_ghost();
        let total = model.get_total_notes() as usize;
        assert_eq!(ghost.len(), total);
        for &g in ghost {
            assert_eq!(g, 4);
        }
    }

    #[test]
    fn init_resets_recent_judges() {
        let mut jm = JudgeManager::new();
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        jm.init(&model, 1);

        assert_eq!(jm.get_recent_judges_index(), 0);
        for &j in jm.get_recent_judges() {
            assert_eq!(j, i64::MIN);
        }
    }

    #[test]
    fn get_judge_count_initially_zero() {
        let jm = JudgeManager::new();
        for i in 0..6 {
            assert_eq!(jm.get_judge_count(i), 0);
        }
    }

    #[test]
    fn get_judge_count_fast_initially_zero() {
        let jm = JudgeManager::new();
        for i in 0..6 {
            assert_eq!(jm.get_judge_count_fast(i, true), 0);
            assert_eq!(jm.get_judge_count_fast(i, false), 0);
        }
    }

    #[test]
    fn get_past_notes_initially_zero() {
        let jm = JudgeManager::new();
        assert_eq!(jm.get_past_notes(), 0);
    }

    #[test]
    fn get_auto_presstime_initially_empty() {
        let jm = JudgeManager::new();
        assert!(jm.get_auto_presstime().is_empty());
    }

    #[test]
    fn get_score_data_returns_default() {
        let jm = JudgeManager::new();
        let score = jm.get_score_data();
        assert_eq!(score.combo, 0);
        assert_eq!(score.epg, 0);
        assert_eq!(score.egr, 0);
    }

    #[test]
    fn init_with_judgeregion_2() {
        let mut jm = JudgeManager::new();
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K);
        model.set_judgerank(100);
        jm.init(&model, 2);

        assert_eq!(jm.get_now_judge(0), 0);
        assert_eq!(jm.get_now_judge(1), 0);
    }

    #[test]
    fn judge_time_region_returns_note_judge() {
        let mut jm = JudgeManager::new();
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        jm.init(&model, 1);

        let region = jm.get_judge_time_region(0);
        assert!(!region.is_empty());
        assert!(region[0][0] < 0);
        assert!(region[0][1] > 0);
    }

    // --- New testable API tests ---

    fn make_model_with_notes(note_times_us: &[i64]) -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        let mut timelines = Vec::new();
        for &time_us in note_times_us {
            let mut tl = TimeLine::new(0.0, time_us, 8);
            let mut note = Note::new_normal(1);
            note.set_micro_time(time_us);
            tl.set_note(0, Some(note));
            timelines.push(tl);
        }
        model.set_all_time_line(timelines);
        model
    }

    #[test]
    fn from_config_creates_valid_state() {
        let model = make_model_with_notes(&[1_000_000, 2_000_000]);
        let notes = build_judge_notes(&model);
        let jp = crate::judge_property::lr2();

        let config = JudgeConfig {
            notes: &notes,
            mode: &Mode::BEAT_7K,
            ln_type: 0,
            judge_rank: 100,
            judge_window_rate: [100, 100, 100],
            scratch_judge_window_rate: [100, 100, 100],
            algorithm: JudgeAlgorithm::Combo,
            autoplay: true,
            judge_property: &jp,
            lane_property: None,
        };
        let jm = JudgeManager::from_config(&config);

        assert_eq!(jm.score().notes, 2);
        assert_eq!(jm.ghost().len(), 2);
        assert_eq!(jm.get_combo(), 0);
        assert_eq!(jm.past_notes(), 0);
    }

    #[test]
    fn autoplay_judges_all_notes_as_pgreat() {
        let model = make_model_with_notes(&[500_000, 1_000_000, 1_500_000]);
        let notes = build_judge_notes(&model);
        let jp = crate::judge_property::lr2();

        let config = JudgeConfig {
            notes: &notes,
            mode: &Mode::BEAT_7K,
            ln_type: 0,
            judge_rank: 100,
            judge_window_rate: [100, 100, 100],
            scratch_judge_window_rate: [100, 100, 100],
            algorithm: JudgeAlgorithm::Combo,
            autoplay: true,
            judge_property: &jp,
            lane_property: None,
        };
        let mut jm = JudgeManager::from_config(&config);

        // Create a minimal gauge (use BMSModel directly)
        let gp = crate::gauge_property::GaugeProperty::Lr2;
        let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

        let lp = LaneProperty::new(&Mode::BEAT_7K);
        let key_count = lp.get_key_lane_assign().len();
        let key_states = vec![false; key_count];
        let key_times = vec![i64::MIN; key_count];

        // Prime
        jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

        // Run simulation
        let mut time = 0i64;
        while time <= 2_500_000 {
            jm.update(time, &notes, &key_states, &key_times, &mut gauge);
            time += 1000;
        }

        // All 3 notes should be PGREAT
        assert_eq!(jm.score().epg + jm.score().lpg, 3);
        assert_eq!(jm.max_combo(), 3);
        assert_eq!(jm.past_notes(), 3);
        for &g in &jm.ghost() {
            assert_eq!(g, JUDGE_PG as usize);
        }
    }

    #[test]
    fn miss_all_notes_without_input() {
        let model = make_model_with_notes(&[500_000]);
        let notes = build_judge_notes(&model);
        let jp = crate::judge_property::lr2();

        let config = JudgeConfig {
            notes: &notes,
            mode: &Mode::BEAT_7K,
            ln_type: 0,
            judge_rank: 100,
            judge_window_rate: [100, 100, 100],
            scratch_judge_window_rate: [100, 100, 100],
            algorithm: JudgeAlgorithm::Combo,
            autoplay: false,
            judge_property: &jp,
            lane_property: None,
        };
        let mut jm = JudgeManager::from_config(&config);

        let gp = crate::gauge_property::GaugeProperty::Lr2;
        let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

        let lp = LaneProperty::new(&Mode::BEAT_7K);
        let key_count = lp.get_key_lane_assign().len();
        let key_states = vec![false; key_count];
        let key_times = vec![i64::MIN; key_count];

        jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

        let mut time = 0i64;
        while time <= 1_500_000 {
            jm.update(time, &notes, &key_states, &key_times, &mut gauge);
            time += 1000;
        }

        // Note should be miss-POOR (judge=4)
        assert_eq!(jm.past_notes(), 1);
        assert_eq!(jm.ghost()[0], JUDGE_PR as usize);
    }
}

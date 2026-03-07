pub(crate) use crate::bms_player_rule::BMSPlayerRule;
pub(crate) use crate::judge::algorithm::JudgeAlgorithm;
pub(crate) use crate::judge::property::{JudgeProperty, MissCondition, NoteType};
pub(crate) use crate::lane_property::LaneProperty;
pub(crate) use bms_model::bms_model::{BMSModel, LNTYPE_HELLCHARGENOTE, LNTYPE_LONGNOTE, LnType};
pub(crate) use bms_model::judge_note::{JUDGE_PR, JudgeNote};
pub(crate) use bms_model::mode::Mode;
pub(crate) use bms_model::note::{
    TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_LONGNOTE, TYPE_UNDEFINED,
};
pub(crate) use rubato_core::score_data::ScoreData;
pub(crate) use rubato_types::course_data::CourseDataConstraint;
pub(crate) use rubato_types::groove_gauge::GrooveGauge;
pub(crate) use rubato_types::player_config::PlayerConfig;

/// HCN gauge change interval (microseconds)
const HCN_MDURATION: i64 = 200000;

/// Configuration for creating a testable JudgeManager.
pub struct JudgeConfig<'a> {
    pub notes: &'a [JudgeNote],
    pub mode: &'a Mode,
    pub ln_type: LnType,
    pub judge_rank: i32,
    pub judge_window_rate: [i32; 3],
    pub scratch_judge_window_rate: [i32; 3],
    pub algorithm: JudgeAlgorithm,
    pub autoplay: bool,
    pub judge_property: &'a JudgeProperty,
    pub lane_property: Option<&'a LaneProperty>,
    /// Whether notes display timing auto-adjust is enabled (PlayerConfig flag).
    pub auto_adjust_enabled: bool,
    /// Whether the play mode is PLAY or PRACTICE (auto-adjust only works in these modes).
    pub is_play_or_practice: bool,
}

/// Internal per-note judge state (parallel to the external notes array).
#[derive(Clone, Debug)]
struct NoteJudgeState {
    state: i32,     // 0=unjudged, 1=PG+1, 2=GR+1, ..., 6=MS+1
    play_time: i64, // Timing difference in microseconds
}

/// Internal per-lane state for judge iteration.
struct LaneIterState {
    _lane: usize,
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

    fn note(&mut self) -> Option<usize> {
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

    fn _set_enabled(&mut self, enabled: bool) {
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
        let tnote_idx = tnote.expect("tnote");

        // Find tnote's dmtime in the collector
        let mut tdmtime: i64 = -1;
        for (&note, &time) in self
            .note_list
            .iter()
            .zip(self.time_list.iter())
            .take(self.size)
        {
            if note == tnote_idx {
                tdmtime = time;
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
        for (&note, &dt) in self
            .note_list
            .iter()
            .zip(self.time_list.iter())
            .take(self.size)
        {
            if dt < bad_start || dt > bad_end {
                continue;
            }
            if dt >= good_start && dt <= good_end {
                continue;
            }
            if note == tnote_idx {
                continue;
            }
            new_notes.push(note);
            new_times.push(dt);
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
        if (!tnote_is_bad || notes[tnote_idx].is_long())
            && let Some(pos) = self.time_list[..self.size]
                .iter()
                .position(|&t| t >= tdmtime)
        {
            self.size = pos;
            self.note_list.truncate(self.size);
            self.time_list.truncate(self.size);
        }

        // Remove preceding LNs before tnote
        self.array_start = self.note_list[..self.size]
            .iter()
            .zip(self.time_list[..self.size].iter())
            .position(|(&note, &time)| time >= tdmtime || !notes[note].is_long())
            .unwrap_or(self.size);
    }
}

/// Judge timing windows and thresholds for notes and scratches.
pub struct JudgeWindows {
    /// Note judge table
    pub nmjudge: Vec<[i64; 2]>,
    pub mjudgestart: i64,
    pub mjudgeend: i64,
    /// CN end judge table
    pub cnendmjudge: Vec<[i64; 2]>,
    pub nreleasemargin: i64,
    /// Scratch judge table
    pub smjudge: Vec<[i64; 2]>,
    pub scnendmjudge: Vec<[i64; 2]>,
    pub sreleasemargin: i64,
}

impl Default for JudgeWindows {
    fn default() -> Self {
        Self {
            nmjudge: Vec::new(),
            mjudgestart: 0,
            mjudgeend: 0,
            cnendmjudge: Vec::new(),
            nreleasemargin: 0,
            smjudge: Vec::new(),
            scnendmjudge: Vec::new(),
            sreleasemargin: 0,
        }
    }
}

/// Score, combo, and judge display state.
pub struct ScoreAccumulator {
    pub score: ScoreData,
    pub combo: i32,
    pub coursecombo: i32,
    pub coursemaxcombo: i32,
    /// Ghost record
    pub ghost: Vec<i32>,
    /// Judge laser color per player per lane
    pub judge: Vec<Vec<i32>>,
    /// Current judge display
    pub judgenow: Vec<i32>,
    pub judgecombo: Vec<i32>,
    /// Judge timing difference (ms, + is early)
    pub judgefast: Vec<i64>,
    pub mjudgefast: Vec<i64>,
}

impl Default for ScoreAccumulator {
    fn default() -> Self {
        Self {
            score: ScoreData::default(),
            combo: 0,
            coursecombo: 0,
            coursemaxcombo: 0,
            ghost: Vec::new(),
            judge: Vec::new(),
            judgenow: Vec::new(),
            judgecombo: Vec::new(),
            judgefast: Vec::new(),
            mjudgefast: Vec::new(),
        }
    }
}

/// Timing auto-adjust state (Java JudgeManager lines 754-768).
pub struct AutoAdjustState {
    /// Recent 100 note judge timings
    pub recent_judges: Vec<i64>,
    pub micro_recent_judges: Vec<i64>,
    pub recent_judges_index: usize,
    pub presses_since_last_autoadjust: i32,
    /// Whether timing auto-adjust is enabled
    pub auto_adjust_enabled: bool,
    /// Whether play mode is PLAY or PRACTICE
    pub is_play_or_practice: bool,
    /// Accumulated judge timing delta from auto-adjust (caller applies to PlayerConfig)
    pub judgetiming_delta: i32,
}

impl Default for AutoAdjustState {
    fn default() -> Self {
        Self {
            recent_judges: vec![i64::MIN; 100],
            micro_recent_judges: vec![i64::MIN; 100],
            recent_judges_index: 0,
            presses_since_last_autoadjust: 0,
            auto_adjust_enabled: false,
            is_play_or_practice: false,
            judgetiming_delta: 0,
        }
    }
}

/// Note judge manager
pub struct JudgeManager {
    lntype: LnType,
    /// Score, combo, and display state.
    pub(crate) scoring: ScoreAccumulator,
    /// Judge timing windows and thresholds.
    pub(crate) windows: JudgeWindows,
    /// Timing auto-adjust state.
    pub(crate) auto_adjust: AutoAdjustState,
    keyassign: Vec<i32>,
    sckey: Vec<i32>,
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

mod accessors;
mod construction;
mod update;

#[cfg(test)]
mod tests;

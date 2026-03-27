pub(crate) use crate::core::score_data::ScoreData;
pub(crate) use crate::play::bms_player_rule::BMSPlayerRule;
pub(crate) use crate::play::judge::algorithm::JudgeAlgorithm;
pub(crate) use crate::play::judge::property::{JudgeProperty, MissCondition, NoteType};
pub(crate) use crate::play::lane_property::LaneProperty;
pub(crate) use bms::model::bms_model::{BMSModel, LNTYPE_HELLCHARGENOTE, LNTYPE_LONGNOTE, LnType};
pub(crate) use bms::model::judge_note::{JUDGE_PR, JudgeNote};
pub(crate) use bms::model::mode::Mode;
pub(crate) use bms::model::note::{
    TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_LONGNOTE, TYPE_UNDEFINED,
};
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
    /// Number of judge display regions (1 for SP, 2 for DP). Controls the size of
    /// judgenow/judgecombo/judgefast/mjudgefast arrays.
    pub judgeregion: i32,
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
    fn mark(&mut self, time_ms: i64, notes: &[JudgeNote]) {
        while self.base_pos < self.note_indices.len().saturating_sub(1)
            && (notes[self.note_indices[self.base_pos + 1]].time_us / 1000) < time_ms
        {
            self.base_pos += 1;
        }
        while self.base_pos > 0
            && (notes[self.note_indices[self.base_pos]].time_us / 1000) > time_ms
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
        if !self.enabled || self.size >= 256 {
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
        if self.mjudge.len() < 4 {
            self.clear();
            return;
        }
        let tnote_idx = tnote.expect("tnote");

        // Find tnote's dmtime in the collector
        let mut tdmtime: Option<i64> = None;
        for (&note, &time) in self
            .note_list
            .iter()
            .zip(self.time_list.iter())
            .take(self.size)
        {
            if note == tnote_idx {
                tdmtime = Some(time);
            }
        }
        let Some(tdmtime) = tdmtime else {
            // tnote not in collector - should not happen
            return;
        };

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

/// Note judge manager
pub struct JudgeManager {
    lntype: LnType,
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
    /// Whether timing auto-adjust is enabled
    auto_adjust_enabled: bool,
    /// Whether play mode is PLAY or PRACTICE
    is_play_or_practice: bool,
    /// Accumulated judge timing delta from auto-adjust (caller applies to PlayerConfig)
    judgetiming_delta: i32,
    /// Per-lane iteration state (only used with testable API)
    lane_states: Vec<LaneIterState>,
    /// Per-note internal judge state
    note_states: Vec<NoteJudgeState>,
    /// MultiBad collector
    multi_bad: MultiBadCollector,
    /// Total lane count
    lane_count: usize,
    /// Lanes that received a new judgment during the last update() call.
    /// Drained by the caller after update() to trigger key beam timers.
    judged_lanes: Vec<usize>,
    /// Judge events produced during update(). Each entry is (judge, mtime) where
    /// judge is the judgment type (0=PG, 1=GR, ...) and mtime is the music time
    /// at which the judgment occurred. Drained by the caller to trigger
    /// update_judge() side effects (BGA miss layer, score timers, pomyu, etc.).
    judged_events: Vec<(i32, i64)>,
    /// Visual side effects produced during update(). Each entry records the
    /// player region, key offset, and judge so the caller can trigger skin
    /// timers (judge text, combo timer, bomb animation) from the main thread.
    judged_visual_events: Vec<JudgeVisualEvent>,
    /// Keysound play events produced during update(). Each entry is a JudgeNote
    /// index that the caller should map to a model Note and play via
    /// `AudioDriver::play_note(note, key_volume, 0)`.
    ///
    /// Corresponds to Java `keysound.play(note, keyvolume, 0)` calls in
    /// JudgeManager.update(). The index refers to the JudgeNote slice passed
    /// to update(); the caller uses `judge_note_to_model` to resolve the
    /// actual model Note for the audio driver.
    keysound_play_indices: Vec<usize>,
    /// Keysound volume-set events produced during update(). Each entry is
    /// (JudgeNote index, volume) that the caller should map to a model Note
    /// and apply via `AudioDriver::set_volume_note(note, volume)`.
    ///
    /// Corresponds to Java `keysound.setVolume(note, vol)` calls in
    /// JudgeManager.update() for HCN processing.
    keysound_volume_set_indices: Vec<(usize, f32)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JudgeVisualEvent {
    pub player: usize,
    pub offset: usize,
    pub judge: i32,
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

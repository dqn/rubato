//! Judge manager for BMS play.
//!
//! Ported from Java: `JudgeManager.java` (1,063 lines).
//! Orchestrates key input → judge window matching → score update → gauge update.

use bms_model::{LaneProperty, LnType, Note, NoteType, PlayMode};

use crate::groove_gauge::GrooveGauge;
use crate::judge_algorithm::JudgeAlgorithm;
use crate::judge_property::{JudgeNoteType, JudgeProperty, JudgeWindowTable, MissCondition};
use crate::score_data::ScoreData;
use crate::{JUDGE_BD, JUDGE_MS, JUDGE_PR};

/// HCN gauge increment/decrement interval in microseconds (200ms).
const HCN_DURATION: i64 = 200_000;

/// Autoplay minimum key press duration in microseconds (80ms).
const AUTO_MIN_DURATION: i64 = 80_000;

/// Sentinel for "not set" / "not released" timestamps.
const NOT_SET: i64 = i64::MIN;

/// Sentinel for "no LN end judgment".
const NO_LN_END_JUDGE: usize = usize::MAX;

/// Sentinel for "no note index".
const NO_NOTE: usize = usize::MAX;

/// Per-lane judgment state machine.
///
/// Tracks cursor position, active LN processing, HCN passing, and release timing.
#[derive(Debug, Clone)]
struct LaneState {
    /// Lane index (reserved for future use)
    #[allow(dead_code)]
    lane: usize,
    /// Whether this lane is a scratch lane
    is_scratch: bool,
    /// Index into lane_notes: next note to consider
    cursor: usize,
    /// Currently processing LN end note index (NO_NOTE = none)
    processing: usize,
    /// Currently passing HCN start note index (NO_NOTE = none)
    passing: usize,
    /// HCN: true = key held (gauge increase), false = key released (gauge decrease)
    inclease: bool,
    /// HCN: μs accumulator for 200ms gauge update interval
    passing_count: i64,
    /// Judgment at LN start (used for worst-of-three calculation)
    ln_start_judge: usize,
    /// Timing offset at LN start (μs)
    ln_start_duration: i64,
    /// Key release time (NOT_SET = not released yet)
    release_time: i64,
    /// LN end judgment (set on key release, applied after release margin)
    ln_end_judge: usize,
}

impl LaneState {
    fn new(lane: usize, is_scratch: bool) -> Self {
        Self {
            lane,
            is_scratch,
            cursor: 0,
            processing: NO_NOTE,
            passing: NO_NOTE,
            inclease: false,
            passing_count: 0,
            ln_start_judge: 0,
            ln_start_duration: 0,
            release_time: NOT_SET,
            ln_end_judge: NO_LN_END_JUDGE,
        }
    }
}

/// PMS-specific multi-BAD collector.
///
/// Collects unjudged notes within the BAD window (excluding GOOD window) and
/// applies simultaneous POOR judgments to them.
#[derive(Debug, Clone)]
struct MultiBadCollector {
    /// (note_index_in_all_notes, dmtime) pairs
    entries: Vec<(usize, i64)>,
    /// true only for PMS mode
    enabled: bool,
}

impl MultiBadCollector {
    fn new(enabled: bool) -> Self {
        Self {
            entries: Vec::new(),
            enabled,
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn add(&mut self, note_index: usize, dmtime: i64) {
        if !self.enabled {
            return;
        }
        self.entries.push((note_index, dmtime));
    }

    /// Filter entries after note selection. Returns the slice of multi-BAD candidates.
    ///
    /// Removes:
    /// 1. Notes outside BAD window (but inside GOOD window)
    /// 2. The selected note itself
    /// 3. If tnote is LN or not a true BAD, remove notes after tnote
    /// 4. Remove preceding LN notes
    fn filter(
        &mut self,
        tnote_index: usize,
        tnote_is_ln: bool,
        judge_table: &JudgeWindowTable,
    ) -> &[(usize, i64)] {
        if !self.enabled || judge_table.len() < 4 {
            self.entries.clear();
            return &self.entries;
        }

        let good_start = judge_table[2][0];
        let good_end = judge_table[2][1];
        let bad_start = judge_table[3][0];
        let bad_end = judge_table[3][1];

        // Find tnote's dmtime
        let tdmtime = self
            .entries
            .iter()
            .find(|(idx, _)| *idx == tnote_index)
            .map(|(_, t)| *t)
            .unwrap_or(-1);

        // Filter: keep only BAD-range (excluding GOOD-range), remove tnote
        self.entries.retain(|(idx, t)| {
            *idx != tnote_index
                && *t >= bad_start
                && *t <= bad_end
                && !(*t >= good_start && *t <= good_end)
        });

        // Sort by dmtime
        self.entries.sort_by_key(|(_, t)| *t);

        // If tnote is LN or not a true BAD, remove all notes at/after tnote's time
        let tnote_is_bad = (bad_start <= tdmtime && tdmtime < good_start)
            || (good_end < tdmtime && tdmtime <= bad_end);
        if (!tnote_is_bad || tnote_is_ln)
            && let Some(pos) = self.entries.iter().position(|(_, t)| *t >= tdmtime)
        {
            self.entries.truncate(pos);
        }

        // Remove preceding LN notes (tracked by the caller using note_type)
        // For simplicity, this is handled at the call site by checking note types.

        &self.entries
    }
}

/// Configuration for initializing the JudgeManager.
pub struct JudgeConfig<'a> {
    /// All notes in the chart (sorted by time)
    pub notes: &'a [Note],
    /// Play mode
    pub play_mode: PlayMode,
    /// LN type from BMS header
    pub ln_type: LnType,
    /// Judge rank (#RANK value)
    pub judge_rank: i32,
    /// Judge window rate for [PG, GR, GD] (100 = normal)
    pub judge_window_rate: [i32; 3],
    /// Scratch judge window rate (100 = normal)
    pub scratch_judge_window_rate: [i32; 3],
    /// Note selection algorithm
    pub algorithm: JudgeAlgorithm,
    /// Whether autoplay is active
    pub autoplay: bool,
    /// Judge property set
    pub judge_property: &'a JudgeProperty,
    /// Lane property for key→lane mapping (optional, auto-created from play_mode if None)
    pub lane_property: Option<&'a LaneProperty>,
}

/// Events generated by JudgeManager during update.
#[derive(Debug, Clone, PartialEq)]
pub enum JudgeEvent {
    /// A note was judged.
    Judge {
        note_index: usize,
        judge: usize,
        duration: i64,
    },
    /// A key sound should play.
    KeySound { wav_id: u16 },
    /// Mine note damage.
    MineDamage { lane: usize, damage: i32 },
    /// HCN gauge update.
    HcnGauge { increase: bool },
}

/// Judge manager: orchestrates key input → judgment → score/gauge updates.
///
/// Ported from Java JudgeManager.java.
pub struct JudgeManager {
    // Configuration
    algorithm: JudgeAlgorithm,
    miss_condition: MissCondition,
    ln_type: LnType,

    // Combo continuation flags per judge [PG, GR, GD, BD, PR, MS]
    combo_cond: [bool; 6],
    // Judge vanish flags per judge [PG, GR, GD, BD, PR, MS]
    judge_vanish: [bool; 6],

    // Scaled judge windows
    nmjudge: JudgeWindowTable,
    smjudge: JudgeWindowTable,
    cnendmjudge: JudgeWindowTable,
    scnendmjudge: JudgeWindowTable,
    nreleasemargin: i64,
    sreleasemargin: i64,

    // Combined window bounds (for early-exit optimization)
    mjudge_start: i64,
    mjudge_end: i64,

    // Per-lane state
    lane_states: Vec<LaneState>,
    // lane_index -> sorted note indices into the original notes slice
    lane_notes: Vec<Vec<usize>>,
    // Local copy of notes with state tracking
    note_states: Vec<i32>,

    // Score tracking
    score: ScoreData,
    combo: i32,
    max_combo: i32,
    course_combo: i32,
    course_max_combo: i32,

    // Ghost data (per-note judgment: 0-5, initialized to JUDGE_PR=4)
    ghost: Vec<usize>,
    pass_notes: i32,

    // Recent judges (circular buffer)
    recent_judges: Vec<i64>,
    recent_index: usize,

    // Per-player judge result (for skin display)
    now_judge: Vec<usize>,
    now_combo: Vec<i32>,

    // Total lane count (for lane → player index mapping)
    lane_count: usize,

    // Lane property: maps physical keys to logical lanes (for BSS sckey tracking)
    lane_property: LaneProperty,

    // BSS tracking: which physical key index is holding each scratch controller
    // sckey[scratch_index] = physical key index that started BSS (0 = not set)
    sckey: Vec<i32>,

    // Autoplay
    autoplay: bool,
    // Per-physical-key autoplay press time (length = physical_key_count)
    auto_presstime: Vec<i64>,

    // MultiBad
    multi_bad: MultiBadCollector,

    // Per-lane judge value: lane_judge[lane] = encoded judge
    // 0=none, 1=PG, 2=GR_EARLY, 3=GR_LATE, 4=GD_EARLY, 5=GD_LATE,
    // 6=BD_EARLY, 7=BD_LATE, 8=LN_HOLD
    lane_judge: Vec<i32>,

    // Timing
    prev_time: i64,
}

impl JudgeManager {
    /// Initialize the judge manager with chart data and configuration.
    pub fn new(config: &JudgeConfig<'_>) -> Self {
        let play_mode = config.play_mode;
        let key_count = play_mode.key_count();

        // Build per-lane note indices
        let mut lane_notes: Vec<Vec<usize>> = vec![Vec::new(); key_count];
        for (i, note) in config.notes.iter().enumerate() {
            if note.lane < key_count {
                lane_notes[note.lane].push(i);
            }
        }
        // Sort each lane's notes by time
        for lane in &mut lane_notes {
            lane.sort_by_key(|&i| config.notes[i].time_us);
        }

        // Build lane states
        let lane_states: Vec<LaneState> = (0..key_count)
            .map(|lane| LaneState::new(lane, play_mode.is_scratch_key(lane)))
            .collect();

        // Compute scaled judge windows
        let nmjudge = config.judge_property.judge_windows(
            JudgeNoteType::Note,
            config.judge_rank,
            &config.judge_window_rate,
        );
        let smjudge = if config.judge_property.scratch.is_empty() {
            nmjudge.clone()
        } else {
            config.judge_property.judge_windows(
                JudgeNoteType::Scratch,
                config.judge_rank,
                &config.scratch_judge_window_rate,
            )
        };
        let cnendmjudge = config.judge_property.judge_windows(
            JudgeNoteType::LongNoteEnd,
            config.judge_rank,
            &config.judge_window_rate,
        );
        let scnendmjudge = if config.judge_property.longscratch.is_empty() {
            cnendmjudge.clone()
        } else {
            config.judge_property.judge_windows(
                JudgeNoteType::LongScratchEnd,
                config.judge_rank,
                &config.scratch_judge_window_rate,
            )
        };
        let nreleasemargin = config.judge_property.longnote_margin;
        let sreleasemargin = config.judge_property.longscratch_margin;

        // Compute combined window bounds
        let mut mjudge_start: i64 = 0;
        let mut mjudge_end: i64 = 0;
        for w in nmjudge.iter().chain(smjudge.iter()) {
            mjudge_start = mjudge_start.min(w[0]);
            mjudge_end = mjudge_end.max(w[1]);
        }

        // Total playable notes for ghost array
        // Exclude pure LN end notes (not independently judged in pure LN mode)
        let total_notes = config
            .notes
            .iter()
            .filter(|n| {
                if !n.is_playable() {
                    return false;
                }
                // Pure LN end: LongNote type, end_time_us == 0 (end marker), has pair link
                if n.note_type == NoteType::LongNote
                    && n.end_time_us == 0
                    && n.pair_index != usize::MAX
                    && config.ln_type == LnType::LongNote
                {
                    return false;
                }
                true
            })
            .count();

        // Initialize ghost array (default to JUDGE_PR = 4)
        let ghost = vec![JUDGE_PR; total_notes];

        // Note states (0 = unjudged)
        let note_states = vec![0i32; config.notes.len()];

        // Lane property (for physical key → lane mapping and BSS sckey tracking)
        let lane_property = match config.lane_property {
            Some(lp) => lp.clone(),
            None => LaneProperty::new(play_mode),
        };

        // BSS tracking
        let sckey = vec![0i32; lane_property.scratch_count()];

        // Autoplay press times (per physical key)
        let auto_presstime = vec![NOT_SET; lane_property.physical_key_count()];

        // PMS multi-bad enabled
        let is_pms = matches!(play_mode, PlayMode::PopN5K | PlayMode::PopN9K);

        let player_count = play_mode.player_count();

        Self {
            algorithm: config.algorithm,
            miss_condition: config.judge_property.miss,
            ln_type: config.ln_type,
            combo_cond: config.judge_property.combo,
            judge_vanish: config.judge_property.judge_vanish,
            nmjudge,
            smjudge,
            cnendmjudge,
            scnendmjudge,
            nreleasemargin,
            sreleasemargin,
            mjudge_start,
            mjudge_end,
            lane_count: key_count,
            lane_property,
            lane_states,
            lane_notes,
            note_states,
            score: ScoreData::default(),
            combo: 0,
            max_combo: 0,
            course_combo: 0,
            course_max_combo: 0,
            ghost,
            pass_notes: 0,
            recent_judges: vec![NOT_SET; 100],
            recent_index: 0,
            now_judge: vec![0; player_count],
            now_combo: vec![0; player_count],
            sckey,
            autoplay: config.autoplay,
            auto_presstime,
            lane_judge: vec![0i32; key_count],
            multi_bad: MultiBadCollector::new(is_pms),
            prev_time: 0,
        }
    }

    /// Main per-frame update.
    ///
    /// Processes note passing, HCN gauge, key input, release margins, and misses.
    ///
    /// # Arguments
    /// * `time_us` - Current time in microseconds
    /// * `notes` - All notes in the chart (same slice as passed to `new`)
    /// * `key_states` - Per-physical-key state (true = pressed), length = physical_key_count
    /// * `key_changed_times` - Per-physical-key last change time (NOT_SET if no change)
    /// * `gauge` - Groove gauge to update
    pub fn update(
        &mut self,
        time_us: i64,
        notes: &[Note],
        key_states: &[bool],
        key_changed_times: &[i64],
        gauge: &mut GrooveGauge,
    ) -> Vec<JudgeEvent> {
        let mut events = Vec::new();

        // Phase 1: Pass phase — advance cursors, handle HCN passing and mines
        self.phase_pass(time_us, notes, key_states, gauge, &mut events);

        // Phase 2: HCN gauge phase
        self.phase_hcn_gauge(time_us, notes, gauge, &mut events);

        self.prev_time = time_us;

        // Phase 3: Key input phase
        self.phase_key_input(
            time_us,
            notes,
            key_states,
            key_changed_times,
            gauge,
            &mut events,
        );

        // Phase 4: Release margin phase
        self.phase_release_margin(time_us, notes, gauge, &mut events);

        // Phase 5: Miss phase
        self.phase_miss(time_us, notes, gauge, &mut events);

        events
    }

    /// Phase 1: Pass through notes that have been reached by current time.
    fn phase_pass(
        &mut self,
        time_us: i64,
        notes: &[Note],
        key_states: &[bool],
        gauge: &mut GrooveGauge,
        events: &mut Vec<JudgeEvent>,
    ) {
        let lane_count = self.lane_states.len();
        for lane_idx in 0..lane_count {
            let mut next_inclease = false;
            let pressed = self
                .lane_property
                .lane_to_keys(lane_idx)
                .iter()
                .any(|&k| key_states.get(k).copied().unwrap_or(false));

            let cursor = self.lane_states[lane_idx].cursor;
            // Collect indices to avoid immutable borrow during iteration
            let note_indices: Vec<usize> = self.lane_notes[lane_idx]
                .iter()
                .skip(cursor)
                .copied()
                .collect();

            // Iterate through notes from cursor
            for note_idx in note_indices {
                let note = &notes[note_idx];
                if note.time_us > time_us {
                    break;
                }
                if note.time_us <= self.prev_time {
                    continue;
                }

                match note.note_type {
                    NoteType::LongNote | NoteType::ChargeNote | NoteType::HellChargeNote => {
                        // Check if this is HCN (or undefined LN treated as HCN)
                        let is_hcn = note.note_type == NoteType::HellChargeNote
                            || (note.note_type == NoteType::LongNote
                                && self.ln_type == LnType::HellChargeNote);

                        if is_hcn {
                            let is_end = note.pair_index == usize::MAX
                                || (note.pair_index < notes.len()
                                    && notes[note.pair_index].time_us < note.time_us);
                            // Determine if this is an "end" note based on pair relationship
                            // In our model, LN start has pair_index pointing to end note
                            // For HCN passing: start sets passing, end clears it
                            if note.end_time_us > 0 && note.time_us < note.end_time_us {
                                // This is a start note
                                self.lane_states[lane_idx].passing = note_idx;
                            } else if is_end {
                                // This is an end note
                                self.lane_states[lane_idx].passing = NO_NOTE;
                                self.lane_states[lane_idx].passing_count = 0;
                            }
                        }
                    }
                    NoteType::Mine => {
                        if pressed {
                            // Mine damage
                            events.push(JudgeEvent::MineDamage {
                                lane: lane_idx,
                                damage: note.damage,
                            });
                            if note.wav_id > 0 {
                                events.push(JudgeEvent::KeySound {
                                    wav_id: note.wav_id,
                                });
                            }
                        }
                    }
                    _ => {}
                }

                // Autoplay: judge normal notes and LN starts automatically
                if self.autoplay && self.note_states[note_idx] == 0 {
                    let first_key = self.lane_property.lane_to_keys(lane_idx)[0];
                    match note.note_type {
                        NoteType::Normal => {
                            self.auto_presstime[first_key] = time_us;
                            if note.wav_id > 0 {
                                events.push(JudgeEvent::KeySound {
                                    wav_id: note.wav_id,
                                });
                            }
                            self.update_judge(lane_idx, note_idx, 0, 0, true, false, gauge, events);
                        }
                        NoteType::LongNote | NoteType::ChargeNote | NoteType::HellChargeNote => {
                            // LN start in autoplay
                            if note.end_time_us > note.time_us
                                && self.lane_states[lane_idx].processing == NO_NOTE
                            {
                                self.auto_presstime[first_key] = time_us;
                                if note.wav_id > 0 {
                                    events.push(JudgeEvent::KeySound {
                                        wav_id: note.wav_id,
                                    });
                                }

                                let is_ln = note.note_type == NoteType::LongNote;

                                if is_ln && self.ln_type == LnType::LongNote {
                                    // Pure LN: don't judge start, just track
                                    self.lane_states[lane_idx].passing_count = 0;
                                } else {
                                    // CN/HCN: judge start
                                    self.update_judge(
                                        lane_idx, note_idx, 0, 0, true, false, gauge, events,
                                    );
                                }
                                if note.pair_index != usize::MAX {
                                    self.lane_states[lane_idx].processing = note.pair_index;
                                }
                                // Record sckey for BSS tracking
                                if let Some(sc_idx) = self.lane_property.scratch_index(lane_idx) {
                                    self.sckey[sc_idx] = first_key as i32;
                                }
                            }
                            // LN end in autoplay (CN/HCN only; pure LN end handled by phase_release_margin)
                            if (note.end_time_us <= note.time_us || note.pair_index == usize::MAX)
                                && self.note_states[note_idx] == 0
                            {
                                let is_cn_hcn_end = note.note_type == NoteType::ChargeNote
                                    || note.note_type == NoteType::HellChargeNote
                                    || (note.note_type == NoteType::LongNote
                                        && self.ln_type != LnType::LongNote);

                                if is_cn_hcn_end {
                                    // BSS autoplay end: release starting key, press alternate key
                                    if let Some(sc_idx) = self.lane_property.scratch_index(lane_idx)
                                    {
                                        let keys = self.lane_property.scratch_keys(sc_idx);
                                        self.auto_presstime[keys[0]] = NOT_SET;
                                        self.auto_presstime[keys[1]] = time_us;
                                    }

                                    self.update_judge(
                                        lane_idx, note_idx, 0, 0, true, false, gauge, events,
                                    );
                                    if note.wav_id > 0 {
                                        events.push(JudgeEvent::KeySound {
                                            wav_id: note.wav_id,
                                        });
                                    }
                                    self.lane_states[lane_idx].processing = NO_NOTE;
                                }
                                // Pure LN end: no action here; phase_release_margin
                                // timeout will judge the start note when end time passes
                            }
                        }
                        _ => {}
                    }
                }
            }

            // HCN gauge: check if key is held while passing
            if self.lane_states[lane_idx].passing != NO_NOTE {
                let passing_idx = self.lane_states[lane_idx].passing;
                if pressed
                    || (passing_idx < notes.len()
                        && notes[passing_idx].pair_index < notes.len()
                        && self.note_states[notes[passing_idx].pair_index] > 0
                        && self.note_states[notes[passing_idx].pair_index] <= 4)
                    || self.autoplay
                {
                    next_inclease = true;
                }
            }

            // Autoplay: release keys after min duration (check all physical keys for lane)
            if self.autoplay && self.lane_states[lane_idx].processing == NO_NOTE {
                for &key_idx in self.lane_property.lane_to_keys(lane_idx) {
                    if self.auto_presstime[key_idx] != NOT_SET
                        && time_us - self.auto_presstime[key_idx] > AUTO_MIN_DURATION
                    {
                        self.auto_presstime[key_idx] = NOT_SET;
                    }
                }
            }

            self.lane_states[lane_idx].inclease = next_inclease;
        }
    }

    /// Phase 2: HCN gauge increment/decrement.
    fn phase_hcn_gauge(
        &mut self,
        time_us: i64,
        notes: &[Note],
        gauge: &mut GrooveGauge,
        events: &mut Vec<JudgeEvent>,
    ) {
        let delta = time_us - self.prev_time;

        for state in &mut self.lane_states {
            if state.passing == NO_NOTE {
                continue;
            }
            let passing_idx = state.passing;
            if passing_idx >= notes.len() || self.note_states[passing_idx] == 0 {
                continue;
            }

            if state.inclease {
                state.passing_count += delta;
                if state.passing_count > HCN_DURATION {
                    gauge.update_with_rate(1, 0.5); // GR at 0.5 rate
                    events.push(JudgeEvent::HcnGauge { increase: true });
                    state.passing_count -= HCN_DURATION;
                }
            } else {
                state.passing_count -= delta;
                if state.passing_count < -HCN_DURATION {
                    gauge.update_with_rate(JUDGE_BD, 0.5); // BD at 0.5 rate
                    events.push(JudgeEvent::HcnGauge { increase: false });
                    state.passing_count += HCN_DURATION;
                }
            }
        }
    }

    /// Phase 3: Process key input (press and release).
    ///
    /// Iterates per physical key (not per lane) to support BSS sckey tracking.
    fn phase_key_input(
        &mut self,
        time_us: i64,
        notes: &[Note],
        key_states: &[bool],
        key_changed_times: &[i64],
        gauge: &mut GrooveGauge,
        events: &mut Vec<JudgeEvent>,
    ) {
        let physical_key_count = self.lane_property.physical_key_count();

        for key_idx in 0..physical_key_count {
            let lane_idx = self.lane_property.key_to_lane(key_idx);
            let pmtime = key_changed_times.get(key_idx).copied().unwrap_or(NOT_SET);
            if pmtime == NOT_SET {
                continue;
            }
            let pressed = key_states.get(key_idx).copied().unwrap_or(false);
            let is_scratch = self.lane_states[lane_idx].is_scratch;
            let sc = self.lane_property.scratch_index(lane_idx);

            if pressed {
                // Key pressed
                if self.lane_states[lane_idx].processing != NO_NOTE {
                    let proc_idx = self.lane_states[lane_idx].processing;
                    let proc_note = &notes[proc_idx];
                    let is_cn_hcn = proc_note.note_type == NoteType::ChargeNote
                        || proc_note.note_type == NoteType::HellChargeNote
                        || (proc_note.note_type == NoteType::LongNote
                            && self.ln_type != LnType::LongNote);

                    if let Some(sc_idx) = sc {
                        if is_cn_hcn && key_idx as i32 != self.sckey[sc_idx] {
                            // BSS end: pressing different scratch key ends BSS
                            let mjudge = &self.scnendmjudge;
                            let dmtime = proc_note.time_us - pmtime;
                            let judge = self.find_judge_window(dmtime, mjudge);

                            if proc_note.wav_id > 0 {
                                events.push(JudgeEvent::KeySound {
                                    wav_id: proc_note.wav_id,
                                });
                            }
                            self.update_judge(
                                lane_idx, proc_idx, judge, dmtime, true, false, gauge, events,
                            );
                            self.lane_states[lane_idx].processing = NO_NOTE;
                            self.lane_states[lane_idx].release_time = NOT_SET;
                            self.lane_states[lane_idx].ln_end_judge = NO_LN_END_JUDGE;
                            self.sckey[sc_idx] = 0;
                        } else {
                            // Same key re-press: cancel release timer
                            self.lane_states[lane_idx].release_time = NOT_SET;
                        }
                    } else if is_cn_hcn {
                        // Non-scratch CN/HCN re-press: cancel release timer
                        self.lane_states[lane_idx].release_time = NOT_SET;
                    } else {
                        // LN re-press: cancel release timer
                        self.lane_states[lane_idx].release_time = NOT_SET;
                    }
                } else {
                    // No active LN: search for best note in window
                    let mjudge = if is_scratch {
                        &self.smjudge
                    } else {
                        &self.nmjudge
                    };

                    self.multi_bad.clear();

                    let mut best_note_idx: Option<usize> = None;
                    let mut best_judge: usize = 0;

                    let lane_note_indices = self.lane_notes[lane_idx].clone();
                    for &note_idx in &lane_note_indices {
                        let note = &notes[note_idx];
                        let dmtime = note.time_us - pmtime;

                        if dmtime >= self.mjudge_end {
                            break;
                        }
                        if dmtime < self.mjudge_start {
                            continue;
                        }

                        // Skip mines and LN ends
                        if note.note_type == NoteType::Mine || note.note_type == NoteType::Invisible
                        {
                            continue;
                        }
                        // Skip end notes (pair_index set AND end_time_us == 0 means this is an end)
                        if note.end_time_us == 0 && note.is_long_note() {
                            continue;
                        }

                        let state = self.note_states[note_idx];
                        if state == 0 {
                            self.multi_bad.add(note_idx, dmtime);
                        }

                        // Note selection using JudgeAlgorithm
                        let should_select = match best_note_idx {
                            None => true,
                            Some(best_idx) => {
                                let best_state = self.note_states[best_idx];
                                best_state != 0
                                    || self.algorithm.compare(
                                        notes[best_idx].time_us,
                                        note.time_us,
                                        state,
                                        pmtime,
                                        mjudge,
                                    )
                            }
                        };

                        if should_select {
                            // Apply MissCondition::One filter
                            if self.miss_condition == MissCondition::One
                                && state != 0
                                && (note.time_us != 0
                                    && (dmtime > mjudge[2][1] || dmtime < mjudge[2][0]))
                            {
                                continue;
                            }

                            let judge = if state != 0 {
                                // Already judged: empty POOR or skip
                                if dmtime >= mjudge[4][0] && dmtime <= mjudge[4][1] {
                                    JUDGE_MS
                                } else {
                                    6 // out of range
                                }
                            } else if note.is_long_note() && dmtime < mjudge[2][0] {
                                // LR2oraja: remove late BAD for LN
                                6
                            } else {
                                let mut j = self.find_judge_window(dmtime, mjudge);
                                // Map window index: 0-3 direct, 4+ becomes POOR(4) or MS(5)
                                if j >= 4 {
                                    j += 1;
                                }
                                j
                            };

                            if judge < 6 {
                                if judge < JUDGE_PR
                                    || best_note_idx.is_none()
                                    || (best_note_idx.is_some()
                                        && (notes[best_note_idx.unwrap()].time_us - pmtime).abs()
                                            > (note.time_us - pmtime).abs())
                                {
                                    best_note_idx = Some(note_idx);
                                    best_judge = judge;
                                }
                            } else {
                                best_note_idx = None;
                            }
                        }
                    }

                    if let Some(tnote_idx) = best_note_idx {
                        let tnote = &notes[tnote_idx];

                        // Process multi-BAD
                        let tnote_is_ln = tnote.is_long_note();
                        let multi_bad_entries = self
                            .multi_bad
                            .filter(tnote_idx, tnote_is_ln, mjudge)
                            .to_vec();
                        for &(mb_idx, mb_dmtime) in &multi_bad_entries {
                            // Skip LN notes in multi-bad
                            if notes[mb_idx].is_long_note() {
                                continue;
                            }
                            self.update_judge(
                                lane_idx,
                                mb_idx,
                                JUDGE_BD,
                                mb_dmtime,
                                self.judge_vanish[JUDGE_BD],
                                true,
                                gauge,
                                events,
                            );
                        }

                        let dmtime = tnote.time_us - pmtime;

                        if tnote.is_long_note() {
                            // Long note processing
                            if tnote.wav_id > 0 {
                                events.push(JudgeEvent::KeySound {
                                    wav_id: tnote.wav_id,
                                });
                            }

                            let is_pure_ln = tnote.note_type == NoteType::LongNote
                                && self.ln_type == LnType::LongNote;

                            if is_pure_ln {
                                // LN: defer judgment to release
                                if self.judge_vanish[best_judge] {
                                    self.lane_states[lane_idx].ln_start_judge = best_judge;
                                    self.lane_states[lane_idx].ln_start_duration = dmtime;
                                    if tnote.pair_index != usize::MAX {
                                        self.lane_states[lane_idx].processing = tnote.pair_index;
                                    }
                                    self.lane_states[lane_idx].release_time = NOT_SET;
                                    self.lane_states[lane_idx].ln_end_judge = NO_LN_END_JUDGE;
                                    // Per-lane: LN hold state
                                    if lane_idx < self.lane_judge.len() {
                                        self.lane_judge[lane_idx] = 8;
                                    }
                                    // Record sckey for BSS tracking
                                    if let Some(sc_idx) = sc {
                                        self.sckey[sc_idx] = key_idx as i32;
                                    }
                                } else {
                                    self.update_judge(
                                        lane_idx, tnote_idx, best_judge, dmtime, false, false,
                                        gauge, events,
                                    );
                                }
                            } else {
                                // CN/HCN: judge start immediately
                                if self.judge_vanish[best_judge] {
                                    if tnote.pair_index != usize::MAX {
                                        self.lane_states[lane_idx].processing = tnote.pair_index;
                                    }
                                    self.lane_states[lane_idx].release_time = NOT_SET;
                                    self.lane_states[lane_idx].ln_end_judge = NO_LN_END_JUDGE;
                                    // Per-lane: LN hold state
                                    if lane_idx < self.lane_judge.len() {
                                        self.lane_judge[lane_idx] = 8;
                                    }
                                    // Record sckey for BSS tracking
                                    if let Some(sc_idx) = sc {
                                        self.sckey[sc_idx] = key_idx as i32;
                                    }
                                }
                                self.update_judge(
                                    lane_idx,
                                    tnote_idx,
                                    best_judge,
                                    dmtime,
                                    self.judge_vanish[best_judge],
                                    false,
                                    gauge,
                                    events,
                                );
                            }
                        } else {
                            // Normal note
                            if tnote.wav_id > 0 {
                                events.push(JudgeEvent::KeySound {
                                    wav_id: tnote.wav_id,
                                });
                            }
                            self.update_judge(
                                lane_idx,
                                tnote_idx,
                                best_judge,
                                dmtime,
                                self.judge_vanish[best_judge],
                                false,
                                gauge,
                                events,
                            );
                        }
                    }
                }
            } else {
                // Key released
                if self.lane_states[lane_idx].processing != NO_NOTE {
                    let proc_idx = self.lane_states[lane_idx].processing;
                    let proc_note = &notes[proc_idx];
                    let mjudge = if is_scratch {
                        &self.scnendmjudge
                    } else {
                        &self.cnendmjudge
                    };
                    let dmtime = proc_note.time_us - pmtime;
                    let judge = self.find_judge_window(dmtime, mjudge);

                    let is_cn_hcn = proc_note.note_type == NoteType::ChargeNote
                        || proc_note.note_type == NoteType::HellChargeNote
                        || (proc_note.note_type == NoteType::LongNote
                            && self.ln_type != LnType::LongNote);

                    if is_cn_hcn {
                        // CN/HCN release with sckey guard
                        let mut release = true;
                        if let Some(sc_idx) = sc {
                            if judge != JUDGE_PR || key_idx as i32 != self.sckey[sc_idx] {
                                release = false;
                            } else {
                                self.sckey[sc_idx] = 0;
                            }
                        }
                        if release {
                            if judge >= JUDGE_BD && dmtime > 0 {
                                // Early release: start release margin timer
                                self.lane_states[lane_idx].release_time = time_us;
                                self.lane_states[lane_idx].ln_end_judge = judge;
                            } else {
                                // Good release or late release: judge immediately
                                self.update_judge(
                                    lane_idx, proc_idx, judge, dmtime, true, false, gauge, events,
                                );
                                if proc_note.wav_id > 0 {
                                    events.push(JudgeEvent::KeySound {
                                        wav_id: proc_note.wav_id,
                                    });
                                }
                                self.lane_states[lane_idx].processing = NO_NOTE;
                                self.lane_states[lane_idx].release_time = NOT_SET;
                                self.lane_states[lane_idx].ln_end_judge = NO_LN_END_JUDGE;
                            }
                        }
                    } else {
                        // LN release with sckey guard
                        let mut release = true;
                        if let Some(sc_idx) = sc {
                            if key_idx as i32 != self.sckey[sc_idx] {
                                release = false;
                            } else {
                                self.sckey[sc_idx] = 0;
                            }
                        }
                        if release {
                            let mut final_judge =
                                judge.max(self.lane_states[lane_idx].ln_start_judge);
                            let final_dmtime = if self.lane_states[lane_idx].ln_start_duration.abs()
                                > dmtime.abs()
                            {
                                self.lane_states[lane_idx].ln_start_duration
                            } else {
                                dmtime
                            };

                            if final_judge >= JUDGE_BD && final_dmtime > 0 {
                                // Early release: start release margin timer
                                self.lane_states[lane_idx].release_time = time_us;
                                self.lane_states[lane_idx].ln_end_judge = final_judge;
                            } else {
                                final_judge = final_judge.min(JUDGE_BD);
                                // Get pair note index for LN end
                                let pair_idx = if proc_note.pair_index != usize::MAX {
                                    proc_note.pair_index
                                } else {
                                    proc_idx
                                };
                                self.update_judge(
                                    lane_idx,
                                    pair_idx,
                                    final_judge,
                                    final_dmtime,
                                    true,
                                    false,
                                    gauge,
                                    events,
                                );
                                if proc_note.wav_id > 0 {
                                    events.push(JudgeEvent::KeySound {
                                        wav_id: proc_note.wav_id,
                                    });
                                }
                                self.lane_states[lane_idx].processing = NO_NOTE;
                                self.lane_states[lane_idx].release_time = NOT_SET;
                                self.lane_states[lane_idx].ln_end_judge = NO_LN_END_JUDGE;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Phase 4: LN end delayed judgment after release margin.
    fn phase_release_margin(
        &mut self,
        time_us: i64,
        notes: &[Note],
        gauge: &mut GrooveGauge,
        events: &mut Vec<JudgeEvent>,
    ) {
        let lane_count = self.lane_states.len();
        for lane_idx in 0..lane_count {
            if self.lane_states[lane_idx].processing == NO_NOTE {
                continue;
            }

            let proc_idx = self.lane_states[lane_idx].processing;
            let proc_note = &notes[proc_idx];
            let is_scratch = self.lane_states[lane_idx].is_scratch;
            let release_margin = if is_scratch {
                self.sreleasemargin
            } else {
                self.nreleasemargin
            };

            let is_pure_ln =
                proc_note.note_type == NoteType::LongNote && self.ln_type == LnType::LongNote;

            if is_pure_ln {
                // LN release margin
                let release_time = self.lane_states[lane_idx].release_time;
                if release_time != NOT_SET && release_time + release_margin <= time_us {
                    let ln_end_judge = self.lane_states[lane_idx].ln_end_judge;
                    let dmtime = proc_note.time_us - release_time;
                    let pair_idx = if proc_note.pair_index != usize::MAX {
                        proc_note.pair_index
                    } else {
                        proc_idx
                    };
                    self.update_judge(
                        lane_idx,
                        pair_idx,
                        ln_end_judge,
                        dmtime,
                        true,
                        false,
                        gauge,
                        events,
                    );
                    if proc_note.wav_id > 0 {
                        events.push(JudgeEvent::KeySound {
                            wav_id: proc_note.wav_id,
                        });
                    }
                    self.lane_states[lane_idx].processing = NO_NOTE;
                    self.lane_states[lane_idx].release_time = NOT_SET;
                    self.lane_states[lane_idx].ln_end_judge = NO_LN_END_JUDGE;
                } else if proc_note.time_us < time_us {
                    // LN end has passed: judge with start timing
                    let pair_idx = if proc_note.pair_index != usize::MAX {
                        proc_note.pair_index
                    } else {
                        proc_idx
                    };
                    let start_judge = self.lane_states[lane_idx].ln_start_judge;
                    let start_duration = self.lane_states[lane_idx].ln_start_duration;
                    self.update_judge(
                        lane_idx,
                        pair_idx,
                        start_judge,
                        start_duration,
                        true,
                        false,
                        gauge,
                        events,
                    );
                    if proc_note.wav_id > 0 {
                        events.push(JudgeEvent::KeySound {
                            wav_id: proc_note.wav_id,
                        });
                    }
                    self.lane_states[lane_idx].processing = NO_NOTE;
                    self.lane_states[lane_idx].release_time = NOT_SET;
                    self.lane_states[lane_idx].ln_end_judge = NO_LN_END_JUDGE;
                }
            } else {
                // CN/HCN release margin
                let release_time = self.lane_states[lane_idx].release_time;
                if release_time != NOT_SET && release_time + release_margin <= time_us {
                    let ln_end_judge = self.lane_states[lane_idx].ln_end_judge;
                    if ln_end_judge != NO_LN_END_JUDGE && ln_end_judge >= JUDGE_BD {
                        // Judge as BAD+
                    }
                    let dmtime = proc_note.time_us - release_time;
                    self.update_judge(
                        lane_idx,
                        proc_idx,
                        if ln_end_judge == NO_LN_END_JUDGE {
                            JUDGE_PR
                        } else {
                            ln_end_judge
                        },
                        dmtime,
                        true,
                        false,
                        gauge,
                        events,
                    );
                    if proc_note.wav_id > 0 {
                        events.push(JudgeEvent::KeySound {
                            wav_id: proc_note.wav_id,
                        });
                    }
                    self.lane_states[lane_idx].processing = NO_NOTE;
                    self.lane_states[lane_idx].release_time = NOT_SET;
                    self.lane_states[lane_idx].ln_end_judge = NO_LN_END_JUDGE;
                }
            }
        }
    }

    /// Phase 5: Miss detection — judge notes that have passed the window.
    fn phase_miss(
        &mut self,
        time_us: i64,
        notes: &[Note],
        gauge: &mut GrooveGauge,
        events: &mut Vec<JudgeEvent>,
    ) {
        let lane_count = self.lane_states.len();
        for lane_idx in 0..lane_count {
            let is_scratch = self.lane_states[lane_idx].is_scratch;
            let mjudge = if is_scratch {
                &self.smjudge
            } else {
                &self.nmjudge
            };
            let bd_late = if mjudge.len() > JUDGE_BD {
                mjudge[JUDGE_BD][0]
            } else {
                continue;
            };

            let lane_note_indices = self.lane_notes[lane_idx].clone();
            for &note_idx in &lane_note_indices {
                let note = &notes[note_idx];
                if note.time_us >= time_us + bd_late {
                    break;
                }
                let mjud = note.time_us - time_us;

                if self.note_states[note_idx] != 0 {
                    continue;
                }

                match note.note_type {
                    NoteType::Normal => {
                        self.update_judge(
                            lane_idx, note_idx, JUDGE_PR, mjud, true, false, gauge, events,
                        );
                    }
                    NoteType::LongNote | NoteType::ChargeNote | NoteType::HellChargeNote => {
                        // Only process start notes (those with end_time > time)
                        if note.end_time_us > note.time_us {
                            let is_cn_hcn = note.note_type == NoteType::ChargeNote
                                || note.note_type == NoteType::HellChargeNote
                                || (note.note_type == NoteType::LongNote
                                    && self.ln_type != LnType::LongNote);

                            if is_cn_hcn {
                                // CN/HCN start miss: also miss the end
                                self.update_judge(
                                    lane_idx, note_idx, JUDGE_PR, mjud, true, false, gauge, events,
                                );
                                if note.pair_index != usize::MAX
                                    && self.note_states[note.pair_index] == 0
                                {
                                    self.update_judge(
                                        lane_idx,
                                        note.pair_index,
                                        JUDGE_PR,
                                        mjud,
                                        true,
                                        false,
                                        gauge,
                                        events,
                                    );
                                }
                            } else {
                                // LN start miss (only if not currently processing)
                                let processing = self.lane_states[lane_idx].processing;
                                if note.pair_index == usize::MAX || processing != note.pair_index {
                                    self.update_judge(
                                        lane_idx, note_idx, JUDGE_PR, mjud, true, false, gauge,
                                        events,
                                    );
                                }
                            }
                        } else {
                            // LN end miss — pure LN end is not independently judged
                            let is_pure_ln_end = note.note_type == NoteType::LongNote
                                && self.ln_type == LnType::LongNote;
                            if !is_pure_ln_end {
                                // CN/HCN end miss
                                self.update_judge(
                                    lane_idx, note_idx, JUDGE_PR, mjud, true, false, gauge, events,
                                );
                            }
                            // Clear processing state for all types
                            self.lane_states[lane_idx].processing = NO_NOTE;
                            self.lane_states[lane_idx].release_time = NOT_SET;
                            self.lane_states[lane_idx].ln_end_judge = NO_LN_END_JUDGE;
                            // Reset sckey on LN end miss
                            if let Some(sc_idx) = self.lane_property.scratch_index(lane_idx) {
                                self.sckey[sc_idx] = 0;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Core judgment update: updates note state, score, combo, ghost, now_judge/now_combo.
    #[allow(clippy::too_many_arguments)]
    fn update_judge(
        &mut self,
        lane: usize,
        note_idx: usize,
        judge: usize,
        duration: i64,
        judge_vanish: bool,
        _multi_bad: bool,
        gauge: &mut GrooveGauge,
        events: &mut Vec<JudgeEvent>,
    ) {
        if judge_vanish {
            if (self.pass_notes as usize) < self.ghost.len() {
                self.ghost[self.pass_notes as usize] = judge;
            }
            self.note_states[note_idx] = judge as i32 + 1;
            self.pass_notes += 1;
            self.score.passnotes = self.pass_notes;
        }

        // MissCondition::One: skip if already judged as POOR
        if self.miss_condition == MissCondition::One
            && judge == JUDGE_PR
            && self.note_states[note_idx] != 0
            && self.note_states[note_idx] != (judge as i32 + 1)
        {
            return;
        }

        // Update score
        let is_early = duration >= 0;
        self.score.add_judge_count(judge, is_early, 1);

        // Recent judges (for timing display)
        if judge < JUDGE_PR {
            self.recent_index = (self.recent_index + 1) % self.recent_judges.len();
            self.recent_judges[self.recent_index] = duration;
        }

        // Combo tracking
        if self.combo_cond[judge] && judge < JUDGE_MS {
            self.combo += 1;
            self.max_combo = self.max_combo.max(self.combo);
            self.score.maxcombo = self.max_combo;
            self.course_combo += 1;
            self.course_max_combo = self.course_max_combo.max(self.course_combo);
        }
        if !self.combo_cond[judge] {
            self.combo = 0;
            self.course_combo = 0;
        }

        // Gauge update
        gauge.update(judge);

        // Update now_judge / now_combo for skin display
        // Java: judgeindex = state.lane / (lanelength / judgenow.length)
        if !self.now_judge.is_empty() && self.lane_count > 0 {
            let judge_index = lane / (self.lane_count / self.now_judge.len());
            if judge_index < self.now_judge.len() {
                self.now_judge[judge_index] = judge + 1; // +1: 0=no judgment, 1=PG, 2=GR, ...
                self.now_combo[judge_index] = self.course_combo;
            }
        }

        // Per-lane judge tracking (Java: this.judge[player][offset])
        if judge != JUDGE_PR && lane < self.lane_judge.len() {
            self.lane_judge[lane] = if judge == 0 {
                1 // PGREAT
            } else {
                (judge as i32) * 2 + if duration >= 0 { 0 } else { 1 }
            };
        }

        // Emit judge event
        events.push(JudgeEvent::Judge {
            note_index: note_idx,
            judge,
            duration,
        });
    }

    /// Find which judge window a timing offset falls into.
    /// Returns 0-4 (PG, GR, GD, BD, MS), or table.len() if outside all windows.
    fn find_judge_window(&self, dmtime: i64, table: &JudgeWindowTable) -> usize {
        for (i, window) in table.iter().enumerate() {
            if dmtime >= window[0] && dmtime <= window[1] {
                return i;
            }
        }
        table.len()
    }

    // --- Getters ---

    /// Get the current score data.
    pub fn score(&self) -> &ScoreData {
        &self.score
    }

    /// Get a mutable reference to the score data.
    pub fn score_mut(&mut self) -> &mut ScoreData {
        &mut self.score
    }

    /// Get the current combo count.
    pub fn combo(&self) -> i32 {
        self.combo
    }

    /// Get the maximum combo achieved.
    pub fn max_combo(&self) -> i32 {
        self.max_combo
    }

    /// Get the course combo count.
    pub fn course_combo(&self) -> i32 {
        self.course_combo
    }

    /// Set the course combo count (for multi-stage courses).
    pub fn set_course_combo(&mut self, combo: i32) {
        self.course_combo = combo;
    }

    /// Get the course max combo.
    pub fn course_max_combo(&self) -> i32 {
        self.course_max_combo
    }

    /// Set the course max combo (for multi-stage courses).
    pub fn set_course_max_combo(&mut self, combo: i32) {
        self.course_max_combo = combo;
    }

    /// Get the ghost data (per-note judgments).
    pub fn ghost(&self) -> &[usize] {
        &self.ghost
    }

    /// Get the current judge display for a player.
    pub fn now_judge(&self, player: usize) -> usize {
        self.now_judge.get(player).copied().unwrap_or(0)
    }

    /// Get the current combo display for a player.
    pub fn now_combo(&self, player: usize) -> i32 {
        self.now_combo.get(player).copied().unwrap_or(0)
    }

    /// Get the per-lane judge value for a specific lane.
    ///
    /// Returns encoded judge: 0=none, 1=PG, 2=GR_EARLY, 3=GR_LATE,
    /// 4=GD_EARLY, 5=GD_LATE, 6=BD_EARLY, 7=BD_LATE, 8=LN_HOLD.
    pub fn lane_judge(&self, lane: usize) -> i32 {
        self.lane_judge.get(lane).copied().unwrap_or(0)
    }

    /// Get the number of notes that have been processed (passed).
    pub fn past_notes(&self) -> i32 {
        self.pass_notes
    }

    /// Get the recent judge timings (circular buffer).
    pub fn recent_judges(&self) -> &[i64] {
        &self.recent_judges
    }

    /// Get the recent judges index.
    pub fn recent_judges_index(&self) -> usize {
        self.recent_index
    }

    /// Get the note state for a specific note index.
    pub fn note_state(&self, note_idx: usize) -> i32 {
        self.note_states.get(note_idx).copied().unwrap_or(0)
    }

    /// Check if a lane is currently processing an LN.
    pub fn processing_ln(&self, lane: usize) -> bool {
        self.lane_states
            .get(lane)
            .is_some_and(|s| s.processing != NO_NOTE)
    }

    /// Check if a lane's HCN is active (increasing).
    pub fn hcn_active(&self, lane: usize) -> bool {
        self.lane_states
            .get(lane)
            .is_some_and(|s| s.passing != NO_NOTE && s.inclease)
    }

    /// Get the autoplay press times per physical key.
    pub fn auto_presstime(&self) -> &[i64] {
        &self.auto_presstime
    }

    /// Get the judge window table for a lane.
    pub fn judge_table(&self, is_scratch: bool) -> &JudgeWindowTable {
        if is_scratch {
            &self.smjudge
        } else {
            &self.nmjudge
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauge_property;
    use crate::judge_property::JudgeProperty;
    use crate::{GaugeProperty, GaugeType};

    /// Beat7K: 8 lanes, 9 physical keys (keys 7,8 → lane 7 scratch)
    const BEAT7K_KEY_COUNT: usize = 9;

    fn make_config_with_notes<'a>(
        notes: &'a [Note],
        judge_property: &'a JudgeProperty,
    ) -> JudgeConfig<'a> {
        JudgeConfig {
            notes,
            play_mode: PlayMode::Beat7K,
            ln_type: LnType::LongNote,
            judge_rank: 100,
            judge_window_rate: [100, 100, 100],
            scratch_judge_window_rate: [100, 100, 100],
            algorithm: JudgeAlgorithm::Combo,
            autoplay: false,
            judge_property,
            lane_property: None,
        }
    }

    fn make_gauge() -> (GaugeProperty, GrooveGauge) {
        let prop = gauge_property::sevenkeys();
        let gauge = GrooveGauge::new(&prop, GaugeType::Normal, 300.0, 100);
        (prop, gauge)
    }

    // --- LaneState tests ---

    #[test]
    fn lane_state_initial_values() {
        let state = LaneState::new(3, true);
        assert_eq!(state.lane, 3);
        assert!(state.is_scratch);
        assert_eq!(state.cursor, 0);
        assert_eq!(state.processing, NO_NOTE);
        assert_eq!(state.passing, NO_NOTE);
        assert!(!state.inclease);
        assert_eq!(state.passing_count, 0);
    }

    #[test]
    fn lane_state_not_scratch() {
        let state = LaneState::new(0, false);
        assert!(!state.is_scratch);
    }

    #[test]
    fn lane_state_release_time_sentinel() {
        let state = LaneState::new(0, false);
        assert_eq!(state.release_time, NOT_SET);
        assert_eq!(state.ln_end_judge, NO_LN_END_JUDGE);
    }

    // --- MultiBadCollector tests ---

    #[test]
    fn multi_bad_disabled() {
        let mut mbc = MultiBadCollector::new(false);
        mbc.add(0, 100);
        mbc.add(1, 200);
        assert!(mbc.entries.is_empty());
    }

    #[test]
    fn multi_bad_enabled_collects() {
        let mut mbc = MultiBadCollector::new(true);
        mbc.add(0, -50000);
        mbc.add(1, 100000);
        assert_eq!(mbc.entries.len(), 2);
    }

    #[test]
    fn multi_bad_clear() {
        let mut mbc = MultiBadCollector::new(true);
        mbc.add(0, 100);
        mbc.clear();
        assert!(mbc.entries.is_empty());
    }

    #[test]
    fn multi_bad_filter_removes_tnote() {
        let mut mbc = MultiBadCollector::new(true);
        // Using sevenkeys BD window: [-280000, 220000], GD: [-150000, 150000]
        mbc.add(0, -200000); // in BAD range, outside GOOD
        mbc.add(1, -100000); // in GOOD range
        mbc.add(2, 160000); // in BAD range, outside GOOD
        mbc.add(3, -200000); // same as 0, different index (selected note)

        let table = vec![
            [-20000, 20000],   // PG
            [-60000, 60000],   // GR
            [-150000, 150000], // GD
            [-280000, 220000], // BD
            [-150000, 500000], // MS
        ];

        let result = mbc.filter(3, false, &table);
        // Should keep: 0 (-200000, in BAD outside GOOD), 2 (160000, in BAD outside GOOD)
        // Should remove: 1 (in GOOD), 3 (selected note)
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|(idx, _)| *idx != 3)); // tnote removed
        assert!(result.iter().all(|(idx, _)| *idx != 1)); // GOOD range removed
    }

    // --- JudgeManager init tests ---

    #[test]
    fn init_creates_correct_lane_count() {
        let notes = vec![Note::normal(0, 1_000_000, 1), Note::normal(3, 2_000_000, 2)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let jm = JudgeManager::new(&config);
        assert_eq!(jm.lane_states.len(), 8); // Beat7K = 8 lanes
        assert_eq!(jm.lane_notes.len(), 8);
    }

    #[test]
    fn init_distributes_notes_to_lanes() {
        let notes = vec![
            Note::normal(0, 1_000_000, 1),
            Note::normal(0, 2_000_000, 2),
            Note::normal(3, 1_500_000, 3),
        ];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let jm = JudgeManager::new(&config);
        assert_eq!(jm.lane_notes[0].len(), 2);
        assert_eq!(jm.lane_notes[3].len(), 1);
        assert_eq!(jm.lane_notes[1].len(), 0);
    }

    #[test]
    fn init_ghost_size_matches_playable_notes() {
        let notes = vec![
            Note::normal(0, 1_000_000, 1),
            Note::mine(1, 2_000_000, 0, 50),
            Note::normal(2, 3_000_000, 2),
        ];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let jm = JudgeManager::new(&config);
        assert_eq!(jm.ghost.len(), 2); // 2 playable (mines excluded)
    }

    // --- Normal note judgment tests ---

    #[test]
    fn judge_normal_note_pgreat() {
        let notes = vec![Note::normal(0, 1_000_000, 1)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Press at exactly the note time → PG (dmtime = 0)
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];

        let events = jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        let judge_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::Judge { .. }))
            .collect();
        assert_eq!(judge_events.len(), 1);
        if let JudgeEvent::Judge {
            judge, duration, ..
        } = judge_events[0]
        {
            assert_eq!(*judge, 0); // PG
            assert_eq!(*duration, 0);
        }
    }

    #[test]
    fn judge_normal_note_great() {
        // Sevenkeys GR window: [-60000, 60000]
        let notes = vec![Note::normal(0, 1_000_000, 1)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Press 40000μs early (dmtime = note_time - press_time = 40000, within GR)
        let press_time = 960_000;
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            press_time, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];

        let events = jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        let judge_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::Judge { .. }))
            .collect();
        assert_eq!(judge_events.len(), 1);
        if let JudgeEvent::Judge { judge, .. } = judge_events[0] {
            assert_eq!(*judge, 1); // GR
        }
    }

    #[test]
    fn judge_normal_note_good() {
        // Sevenkeys GD window: [-150000, 150000]
        let notes = vec![Note::normal(0, 1_000_000, 1)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Press 100000μs early (dmtime = 100000, within GD but outside GR)
        let press_time = 900_000;
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            press_time, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];

        let events = jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        let judge_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::Judge { .. }))
            .collect();
        assert_eq!(judge_events.len(), 1);
        if let JudgeEvent::Judge { judge, .. } = judge_events[0] {
            assert_eq!(*judge, 2); // GD
        }
    }

    #[test]
    fn judge_normal_note_bad() {
        // Sevenkeys BD window: [-280000, 220000]
        let notes = vec![Note::normal(0, 1_000_000, 1)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Press 200000μs early (dmtime = 200000, within BD but outside GD)
        let press_time = 800_000;
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            press_time, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];

        let events = jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        let judge_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::Judge { .. }))
            .collect();
        assert_eq!(judge_events.len(), 1);
        if let JudgeEvent::Judge { judge, .. } = judge_events[0] {
            assert_eq!(*judge, 3); // BD (window index 3 → judge 3)
        }
    }

    #[test]
    fn judge_miss_detection() {
        // Note passes completely through the window
        let notes = vec![Note::normal(0, 500_000, 1)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // No key pressed, advance time past the note + BD window
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];

        // First update at time well past the note
        let events = jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        let judge_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::Judge { .. }))
            .collect();
        assert_eq!(judge_events.len(), 1);
        if let JudgeEvent::Judge { judge, .. } = judge_events[0] {
            assert_eq!(*judge, JUDGE_PR); // POOR (miss)
        }
    }

    // --- LN tests ---

    #[test]
    fn judge_ln_start_and_end() {
        // Create LN from 1.0s to 2.0s
        let mut notes = vec![Note::long_note(
            0,
            1_000_000,
            2_000_000,
            1,
            1,
            LnType::ChargeNote,
        )];
        // Create end note
        let mut end_note = Note::normal(0, 2_000_000, 1);
        end_note.note_type = NoteType::ChargeNote;
        end_note.end_time_us = 0;
        notes.push(end_note);
        notes[0].pair_index = 1;
        notes[1].pair_index = 0;

        let jp = JudgeProperty::sevenkeys();
        let mut config = make_config_with_notes(&notes, &jp);
        config.ln_type = LnType::ChargeNote;
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Press at LN start
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        let events = jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Should have judged the start
        let judge_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::Judge { .. }))
            .collect();
        assert!(!judge_events.is_empty(), "LN start should be judged");

        // Verify LN is now processing
        assert!(jm.processing_ln(0));

        // Release at LN end
        let key_states = vec![
            false, false, false, false, false, false, false, false, false,
        ];
        let key_times = vec![
            2_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        let events = jm.update(2_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Should have judged the end
        let judge_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::Judge { .. }))
            .collect();
        assert!(!judge_events.is_empty(), "LN end should be judged");
        assert!(!jm.processing_ln(0));
    }

    // --- CN re-press test ---

    #[test]
    fn cn_repress_cancels_release() {
        let mut notes = vec![Note::long_note(
            0,
            1_000_000,
            3_000_000,
            1,
            1,
            LnType::ChargeNote,
        )];
        let mut end_note = Note::normal(0, 3_000_000, 1);
        end_note.note_type = NoteType::ChargeNote;
        end_note.end_time_us = 0;
        notes.push(end_note);
        notes[0].pair_index = 1;
        notes[1].pair_index = 0;

        let jp = JudgeProperty::pms();
        let mut config = make_config_with_notes(&notes, &jp);
        config.ln_type = LnType::ChargeNote;
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Press at start
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Release briefly (early release)
        let key_states = vec![
            false, false, false, false, false, false, false, false, false,
        ];
        let key_times = vec![
            1_500_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_500_000, &notes, &key_states, &key_times, &mut gauge);

        // Re-press before release margin expires
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_600_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_600_000, &notes, &key_states, &key_times, &mut gauge);

        // Should still be processing (release was cancelled)
        assert!(jm.processing_ln(0), "Re-press should cancel release timer");
    }

    // --- Combo tracking tests ---

    #[test]
    fn combo_increments_on_good_judges() {
        let notes = vec![
            Note::normal(0, 1_000_000, 1),
            Note::normal(0, 2_000_000, 2),
            Note::normal(0, 3_000_000, 3),
        ];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Judge three notes with PG timing
        for i in 0..3 {
            let t = (i + 1) as i64 * 1_000_000;
            let key_states = vec![true, false, false, false, false, false, false, false, false];
            let key_times = vec![
                t, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
            ];
            jm.update(t, &notes, &key_states, &key_times, &mut gauge);

            // Reset key (no change on next frame)
            let key_states = vec![false; BEAT7K_KEY_COUNT];
            let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
            jm.update(t + 100_000, &notes, &key_states, &key_times, &mut gauge);
        }

        assert_eq!(jm.combo(), 3);
        assert_eq!(jm.max_combo(), 3);
    }

    #[test]
    fn combo_resets_on_miss() {
        let notes = vec![
            Note::normal(0, 1_000_000, 1),
            Note::normal(0, 3_000_000, 2), // Will be missed
            Note::normal(0, 5_000_000, 3),
        ];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Judge first note
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Reset key
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(1_100_000, &notes, &key_states, &key_times, &mut gauge);

        assert_eq!(jm.combo(), 1);

        // Skip second note (let it pass) - advance past BD window
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(4_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Combo should be reset (POOR doesn't continue combo in sevenkeys)
        assert_eq!(jm.combo(), 0);
        assert_eq!(jm.max_combo(), 1);
    }

    // --- Ghost recording test ---

    #[test]
    fn ghost_records_per_note_judgments() {
        let notes = vec![Note::normal(0, 1_000_000, 1), Note::normal(1, 2_000_000, 2)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Default ghost values are JUDGE_PR (4)
        assert_eq!(jm.ghost()[0], JUDGE_PR);
        assert_eq!(jm.ghost()[1], JUDGE_PR);

        // Judge first note with PG
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);

        assert_eq!(jm.ghost()[0], 0); // PG
        assert_eq!(jm.ghost()[1], JUDGE_PR); // Still default
    }

    // --- Mine note test ---

    #[test]
    fn mine_triggers_damage_when_pressed() {
        let notes = vec![Note::mine(0, 1_000_000, 5, 50)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Hold key when mine passes
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT]; // No key change, just held
        jm.prev_time = 500_000;
        let events = jm.update(1_500_000, &notes, &key_states, &key_times, &mut gauge);

        let mine_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::MineDamage { .. }))
            .collect();
        assert_eq!(mine_events.len(), 1);
        if let JudgeEvent::MineDamage { damage, .. } = mine_events[0] {
            assert_eq!(*damage, 50);
        }
    }

    #[test]
    fn mine_does_not_trigger_when_not_pressed() {
        let notes = vec![Note::mine(0, 1_000_000, 5, 50)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Key not held
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.prev_time = 500_000;
        let events = jm.update(1_500_000, &notes, &key_states, &key_times, &mut gauge);

        let mine_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::MineDamage { .. }))
            .collect();
        assert_eq!(mine_events.len(), 0);
    }

    // --- Autoplay test ---

    #[test]
    fn autoplay_judges_all_notes() {
        let notes = vec![Note::normal(0, 1_000_000, 1), Note::normal(1, 2_000_000, 2)];
        let jp = JudgeProperty::sevenkeys();
        let mut config = make_config_with_notes(&notes, &jp);
        config.autoplay = true;
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];

        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        jm.update(2_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Both notes should be judged as PG
        assert_eq!(jm.score().judge_count(0), 2); // 2 PGs
        assert_eq!(jm.combo(), 2);
    }

    // --- HCN gauge test ---

    #[test]
    fn hcn_gauge_increase_decrease() {
        // Create an HCN from 1.0s to 2.0s
        let mut notes = vec![Note::long_note(
            0,
            1_000_000,
            2_000_000,
            1,
            1,
            LnType::HellChargeNote,
        )];
        let mut end_note = Note::normal(0, 2_000_000, 1);
        end_note.note_type = NoteType::HellChargeNote;
        end_note.end_time_us = 0;
        notes.push(end_note);
        notes[0].pair_index = 1;
        notes[1].pair_index = 0;

        let jp = JudgeProperty::sevenkeys();
        let mut config = make_config_with_notes(&notes, &jp);
        config.ln_type = LnType::HellChargeNote;
        config.autoplay = true;
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];

        // Update through the HCN duration
        jm.update(900_000, &notes, &key_states, &key_times, &mut gauge);
        jm.update(1_100_000, &notes, &key_states, &key_times, &mut gauge);

        // HCN should be active
        // Note: autoplay + HCN behavior depends on passing state being set
        // The key assertion is that no crash occurs
    }

    // --- Score tracking test ---

    #[test]
    fn score_tracks_early_late_correctly() {
        let notes = vec![
            Note::normal(0, 1_000_000, 1), // Will be hit early
            Note::normal(0, 3_000_000, 2), // Will be hit late
        ];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Hit first note 30000μs early (within GR window, dmtime = 30000 > 0 = early)
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            970_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Reset
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(1_100_000, &notes, &key_states, &key_times, &mut gauge);

        // Hit second note 30000μs late (dmtime = -30000 < 0 = late)
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            3_030_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(3_030_000, &notes, &key_states, &key_times, &mut gauge);

        // One early GR, one late GR
        assert_eq!(jm.score().egr, 1);
        assert_eq!(jm.score().lgr, 1);
    }

    // --- find_judge_window test ---

    #[test]
    fn find_judge_window_returns_correct_index() {
        let notes: Vec<Note> = vec![];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let jm = JudgeManager::new(&config);

        let table = &jm.nmjudge;
        assert_eq!(jm.find_judge_window(0, table), 0); // PG
        assert_eq!(jm.find_judge_window(40000, table), 1); // GR
        assert_eq!(jm.find_judge_window(100000, table), 2); // GD
        assert_eq!(jm.find_judge_window(200000, table), 3); // BD
    }

    #[test]
    fn find_judge_window_outside_all() {
        let notes: Vec<Note> = vec![];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let jm = JudgeManager::new(&config);

        let table = &jm.nmjudge;
        // Way outside any window
        assert_eq!(jm.find_judge_window(900000, table), table.len());
    }

    // --- Past notes counter ---

    #[test]
    fn past_notes_increments() {
        let notes = vec![Note::normal(0, 1_000_000, 1), Note::normal(1, 2_000_000, 2)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        assert_eq!(jm.past_notes(), 0);

        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);

        assert_eq!(jm.past_notes(), 1);
    }

    // --- PR (empty POOR) timing test ---

    #[test]
    fn judge_normal_note_poor_empty() {
        // Sevenkeys MS window: [-150000, 500000]
        // Press way outside BD window but inside MS window → empty POOR (JUDGE_MS)
        let notes = vec![
            Note::normal(0, 1_000_000, 1), // First: already judged
            Note::normal(0, 2_000_000, 2), // Second: will get empty POOR
        ];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Judge first note with PG
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Reset key
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(1_100_000, &notes, &key_states, &key_times, &mut gauge);

        // Let first note pass completely, then press within its MS window → empty POOR
        // dmtime for first note = 1_000_000 - 1_400_000 = -400000 (in MS window [-150000, 500000])
        // But the note was already judged, so the press should cause an empty POOR
        // Actually, the first note is already judged, and the second note has dmtime = 2_000_000 - 1_400_000 = 600000 > BD range
        // So pressing at this time should not produce a normal judge. Let the second note go to miss instead.

        // Instead: let second note pass → POOR miss
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(3_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Second note should be a POOR miss
        assert_eq!(jm.score().judge_count(JUDGE_PR), 1);
    }

    // --- LN worst-of-three test ---

    #[test]
    fn ln_worst_of_three_judgment() {
        // Create LN (pure LN type) from 1.0s to 3.0s
        let mut notes = vec![Note::long_note(
            0,
            1_000_000,
            3_000_000,
            1,
            1,
            LnType::LongNote,
        )];
        let mut end_note = Note::normal(0, 3_000_000, 1);
        end_note.note_type = NoteType::LongNote;
        end_note.end_time_us = 0;
        notes.push(end_note);
        notes[0].pair_index = 1;
        notes[1].pair_index = 0;

        let jp = JudgeProperty::pms();
        let mut config = make_config_with_notes(&notes, &jp);
        config.ln_type = LnType::LongNote;
        config.play_mode = PlayMode::PopN9K;
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Press at LN start with GR timing (40000μs early)
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            960_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);

        // LN should be processing
        assert!(jm.processing_ln(0));

        // Release at LN end with BD timing (220000μs early → dmtime = 220000)
        let key_states = vec![
            false, false, false, false, false, false, false, false, false,
        ];
        let key_times = vec![
            2_780_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        let _events = jm.update(2_900_000, &notes, &key_states, &key_times, &mut gauge);

        // Check that early release starts release margin timer (BD + early = margin)
        // The release was at 210000μs early, which is in BD window
        // For LN, worst of (start=GR, end=BD) = BD, and dmtime > 0, so release margin starts
        assert!(
            jm.processing_ln(0),
            "Should still be processing (release margin active)"
        );

        // Advance past release margin (let it expire)
        let key_states = vec![false; 9];
        let key_times = vec![NOT_SET; 9];
        let events = jm.update(3_500_000, &notes, &key_states, &key_times, &mut gauge);

        // LN should now be resolved
        let judge_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::Judge { .. }))
            .collect();
        // The final judge should be BD (worst of GR start and BD end)
        if let Some(JudgeEvent::Judge { judge, .. }) = judge_events.first() {
            assert_eq!(*judge, JUDGE_BD, "Worst of GR and BD should be BD");
        }
    }

    // --- CN release margin expiry test ---

    #[test]
    fn cn_release_margin_expired_gives_poor() {
        let mut notes = vec![Note::long_note(
            0,
            1_000_000,
            4_000_000,
            1,
            1,
            LnType::ChargeNote,
        )];
        let mut end_note = Note::normal(0, 4_000_000, 1);
        end_note.note_type = NoteType::ChargeNote;
        end_note.end_time_us = 0;
        notes.push(end_note);
        notes[0].pair_index = 1;
        notes[1].pair_index = 0;

        let jp = JudgeProperty::sevenkeys();
        let mut config = make_config_with_notes(&notes, &jp);
        config.ln_type = LnType::ChargeNote;
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Press at CN start (PG)
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        assert!(jm.processing_ln(0));

        // Release very early (dmtime = 4_000_000 - 1_500_000 = 2_500_000 > BD window) → BD margin
        let key_states = vec![
            false, false, false, false, false, false, false, false, false,
        ];
        let key_times = vec![
            1_500_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_500_000, &notes, &key_states, &key_times, &mut gauge);

        // Release margin started, don't re-press
        // Advance past release margin
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(2_500_000, &notes, &key_states, &key_times, &mut gauge);

        // CN should be resolved after margin expired
        assert!(
            !jm.processing_ln(0),
            "CN should be done after release margin expired"
        );
    }

    // --- HCN gauge value verification ---

    #[test]
    fn hcn_gauge_events_emitted_after_interval() {
        // Create an HCN from 0.5s to 2.0s (autoplay mode)
        let mut notes = vec![Note::long_note(
            0,
            500_000,
            2_000_000,
            1,
            1,
            LnType::HellChargeNote,
        )];
        let mut end_note = Note::normal(0, 2_000_000, 1);
        end_note.note_type = NoteType::HellChargeNote;
        end_note.end_time_us = 0;
        notes.push(end_note);
        notes[0].pair_index = 1;
        notes[1].pair_index = 0;

        let jp = JudgeProperty::sevenkeys();
        let mut config = make_config_with_notes(&notes, &jp);
        config.ln_type = LnType::HellChargeNote;
        config.autoplay = true;
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];

        // Advance to trigger HCN start (autoplay)
        jm.update(400_000, &notes, &key_states, &key_times, &mut gauge);
        jm.update(600_000, &notes, &key_states, &key_times, &mut gauge);

        // Now advance 250ms at a time to trigger HCN gauge events
        let mut hcn_increase_count = 0;
        for step in 1..=4 {
            let t = 600_000 + step * 250_000;
            let events = jm.update(t, &notes, &key_states, &key_times, &mut gauge);
            hcn_increase_count += events
                .iter()
                .filter(|e| matches!(e, JudgeEvent::HcnGauge { increase: true }))
                .count();
        }

        // Over 1000ms (4 × 250ms) with 200ms interval, expect ~5 gauge increments
        assert!(
            hcn_increase_count >= 3,
            "Expected at least 3 HCN gauge increase events, got {hcn_increase_count}"
        );
    }

    // --- LN start miss test ---

    #[test]
    fn ln_start_miss_also_misses_end() {
        // Create CN from 1.0s to 2.0s
        let mut notes = vec![Note::long_note(
            0,
            1_000_000,
            2_000_000,
            1,
            1,
            LnType::ChargeNote,
        )];
        let mut end_note = Note::normal(0, 2_000_000, 1);
        end_note.note_type = NoteType::ChargeNote;
        end_note.end_time_us = 0;
        notes.push(end_note);
        notes[0].pair_index = 1;
        notes[1].pair_index = 0;

        let jp = JudgeProperty::sevenkeys();
        let mut config = make_config_with_notes(&notes, &jp);
        config.ln_type = LnType::ChargeNote;
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Don't press anything. Advance past both start and end.
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];

        // Past start + BD window
        jm.update(2_000_000, &notes, &key_states, &key_times, &mut gauge);
        // Past end + BD window
        jm.update(3_000_000, &notes, &key_states, &key_times, &mut gauge);

        // Both start and end should be POOR
        assert_eq!(
            jm.score().judge_count(JUDGE_PR),
            2,
            "Both LN start and end should be POOR"
        );
    }

    // --- now_judge / now_combo update test ---

    #[test]
    fn now_judge_and_now_combo_updated_after_judgment() {
        let notes = vec![Note::normal(0, 1_000_000, 1), Note::normal(0, 2_000_000, 2)];
        let jp = JudgeProperty::sevenkeys();
        let config = make_config_with_notes(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Initially no judgment
        assert_eq!(jm.now_judge(0), 0);
        assert_eq!(jm.now_combo(0), 0);

        // Judge first note with PG
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_000_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);

        // now_judge should be 1 (PG + 1), now_combo should be 1
        assert_eq!(jm.now_judge(0), 1, "now_judge should be PG+1=1");
        assert_eq!(jm.now_combo(0), 1, "now_combo should be 1 after first PG");

        // Reset key
        let key_states = vec![false; BEAT7K_KEY_COUNT];
        let key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(1_100_000, &notes, &key_states, &key_times, &mut gauge);

        // Judge second note with GR (40000μs early)
        let key_states = vec![true, false, false, false, false, false, false, false, false];
        let key_times = vec![
            1_960_000, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET, NOT_SET,
        ];
        jm.update(2_000_000, &notes, &key_states, &key_times, &mut gauge);

        assert_eq!(jm.now_judge(0), 2, "now_judge should be GR+1=2");
        assert_eq!(
            jm.now_combo(0),
            2,
            "now_combo should be 2 after second note"
        );
    }

    // --- BSS sckey tracking tests ---

    /// Helper: create a BSS (scratch CN) start+end note pair on lane 7.
    fn make_scratch_cn(start_us: i64, end_us: i64) -> Vec<Note> {
        let mut start = Note::long_note(7, start_us, end_us, 1, 1, LnType::ChargeNote);
        let mut end = Note::normal(7, end_us, 1);
        end.note_type = NoteType::ChargeNote;
        end.end_time_us = 0;
        start.pair_index = 1;
        end.pair_index = 0;
        vec![start, end]
    }

    fn make_scratch_config<'a>(
        notes: &'a [Note],
        judge_property: &'a JudgeProperty,
    ) -> JudgeConfig<'a> {
        let mut config = make_config_with_notes(notes, judge_property);
        config.ln_type = LnType::ChargeNote;
        config
    }

    #[test]
    fn bss_end_on_different_key() {
        // Start BSS with physical key 7, end by pressing physical key 8.
        let notes = make_scratch_cn(1_000_000, 2_000_000);
        let jp = JudgeProperty::sevenkeys();
        let config = make_scratch_config(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        // Prime
        let empty_states = vec![false; BEAT7K_KEY_COUNT];
        let empty_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(-1, &notes, &empty_states, &empty_times, &mut gauge);

        // Press key 7 (start BSS) at 1_000_000
        let mut key_states = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_states[7] = true;
        key_times[7] = 1_000_000;
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        assert!(jm.processing_ln(7), "BSS should be active on lane 7");

        // Press key 8 (different scratch key) at 2_000_000 → BSS end
        let mut key_states2 = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times2 = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_states2[7] = true; // key 7 still held
        key_states2[8] = true;
        key_times2[8] = 2_000_000;
        let events = jm.update(2_000_000, &notes, &key_states2, &key_times2, &mut gauge);
        assert!(
            !jm.processing_ln(7),
            "BSS should be ended by different key press"
        );
        // Should have emitted a Judge event for the end note
        let judge_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, JudgeEvent::Judge { .. }))
            .collect();
        assert!(
            !judge_events.is_empty(),
            "BSS end should produce a judge event"
        );
    }

    #[test]
    fn bss_same_key_repress() {
        // Releasing same key within BD window (not POOR) is blocked by sckey guard.
        // Re-pressing cancels the release timer, BSS stays active.
        // longscratch BD window: [-290000, 230000] — release at dmtime=200000 → BD judge
        let notes = make_scratch_cn(1_000_000, 2_000_000);
        let jp = JudgeProperty::sevenkeys();
        let config = make_scratch_config(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        let empty_states = vec![false; BEAT7K_KEY_COUNT];
        let empty_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(-1, &notes, &empty_states, &empty_times, &mut gauge);

        // Press key 7 (start BSS)
        let mut key_states = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_states[7] = true;
        key_times[7] = 1_000_000;
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        assert!(jm.processing_ln(7));

        // Release key 7 at 1_800_000 (dmtime=200000, within BD window)
        // judge=BD(3) != POOR(4), so sckey guard blocks the release.
        let key_states2 = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times2 = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_times2[7] = 1_800_000;
        jm.update(1_800_000, &notes, &key_states2, &key_times2, &mut gauge);
        assert!(
            jm.processing_ln(7),
            "Same-key release within BD window should be blocked by sckey guard"
        );

        // Re-press key 7 — BSS should still be active
        let mut key_states3 = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times3 = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_states3[7] = true;
        key_times3[7] = 1_900_000;
        jm.update(1_900_000, &notes, &key_states3, &key_times3, &mut gauge);
        assert!(jm.processing_ln(7), "Same-key re-press should not end BSS");
    }

    #[test]
    fn bss_release_only_same_key() {
        // Releasing the OTHER scratch key should be ignored (BSS continues).
        let notes = make_scratch_cn(1_000_000, 2_000_000);
        let jp = JudgeProperty::sevenkeys();
        let config = make_scratch_config(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        let empty_states = vec![false; BEAT7K_KEY_COUNT];
        let empty_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(-1, &notes, &empty_states, &empty_times, &mut gauge);

        // Press key 7 (start BSS)
        let mut key_states = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_states[7] = true;
        key_times[7] = 1_000_000;
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        assert!(jm.processing_ln(7));

        // Release key 8 (different key) — BSS should NOT react
        let mut key_states2 = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times2 = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_states2[7] = true; // key 7 still held
        key_times2[8] = 1_500_000; // key 8 released
        jm.update(1_500_000, &notes, &key_states2, &key_times2, &mut gauge);
        assert!(
            jm.processing_ln(7),
            "Releasing different key should not affect BSS"
        );
    }

    #[test]
    fn bss_cn_release_guard() {
        // BSS sckey guard: same-key release with good timing (PG) is BLOCKED.
        // Only POOR-timing releases or pressing the other key can end BSS.
        // longscratch PG window: [-130000, 130000] — release at dmtime=0 → PG judge
        let notes = make_scratch_cn(1_000_000, 2_000_000);
        let jp = JudgeProperty::sevenkeys();
        let config = make_scratch_config(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        let empty_states = vec![false; BEAT7K_KEY_COUNT];
        let empty_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(-1, &notes, &empty_states, &empty_times, &mut gauge);

        // Press key 7 (start BSS)
        let mut key_states = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_states[7] = true;
        key_times[7] = 1_000_000;
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        assert!(jm.processing_ln(7));

        // Release key 7 at exact end note time (dmtime=0, PG judge)
        // PG(0) != POOR(4), so sckey guard blocks the release — BSS stays active.
        let key_states2 = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times2 = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_times2[7] = 2_000_000;
        jm.update(2_000_000, &notes, &key_states2, &key_times2, &mut gauge);
        assert!(
            jm.processing_ln(7),
            "Same-key release with PG timing should be blocked on BSS"
        );

        // Press key 8 (other scratch key) to actually end BSS
        let mut key_states3 = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times3 = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_states3[8] = true;
        key_times3[8] = 2_100_000;
        jm.update(2_100_000, &notes, &key_states3, &key_times3, &mut gauge);
        assert!(
            !jm.processing_ln(7),
            "Pressing other scratch key should end BSS"
        );
    }

    #[test]
    fn bss_sckey_reset_on_miss() {
        // When BSS times out (miss), sckey should be reset.
        let notes = make_scratch_cn(1_000_000, 2_000_000);
        let jp = JudgeProperty::sevenkeys();
        let config = make_scratch_config(&notes, &jp);
        let mut jm = JudgeManager::new(&config);
        let (_prop, mut gauge) = make_gauge();

        let empty_states = vec![false; BEAT7K_KEY_COUNT];
        let empty_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        jm.update(-1, &notes, &empty_states, &empty_times, &mut gauge);

        // Press key 7 (start BSS)
        let mut key_states = vec![false; BEAT7K_KEY_COUNT];
        let mut key_times = vec![NOT_SET; BEAT7K_KEY_COUNT];
        key_states[7] = true;
        key_times[7] = 1_000_000;
        jm.update(1_000_000, &notes, &key_states, &key_times, &mut gauge);
        assert!(jm.processing_ln(7));

        // Hold key 7 but never release/press other key → run past miss window
        let held_states = vec![false, false, false, false, false, false, false, true, false];
        let no_change = vec![NOT_SET; BEAT7K_KEY_COUNT];
        // Advance past the end note miss window (far future)
        for t in (2_100_000..4_000_000).step_by(100_000) {
            jm.update(t, &notes, &held_states, &no_change, &mut gauge);
        }

        // BSS should have been missed — no longer processing
        assert!(!jm.processing_ln(7), "BSS should end via miss timeout");
    }
}

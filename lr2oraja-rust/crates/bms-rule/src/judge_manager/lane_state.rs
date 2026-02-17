//! Per-lane judgment state and PMS multi-BAD collection.

use crate::judge_property::JudgeWindowTable;

/// Sentinel for "no note index".
pub(super) const NO_NOTE: usize = usize::MAX;

/// Sentinel for "not set" / "not released" timestamps.
pub(super) const NOT_SET: i64 = i64::MIN;

/// Sentinel for "no LN end judgment".
pub(super) const NO_LN_END_JUDGE: usize = usize::MAX;

/// Per-lane judgment state machine.
///
/// Tracks cursor position, active LN processing, HCN passing, and release timing.
#[derive(Debug, Clone)]
pub(crate) struct LaneState {
    /// Lane index (reserved for future use)
    #[allow(dead_code)] // Parsed for completeness (lane index for debug/future use)
    pub(super) lane: usize,
    /// Whether this lane is a scratch lane
    pub(super) is_scratch: bool,
    /// Index into lane_notes: next note to consider
    pub(super) cursor: usize,
    /// Currently processing LN end note index (NO_NOTE = none)
    pub(super) processing: usize,
    /// Currently passing HCN start note index (NO_NOTE = none)
    pub(super) passing: usize,
    /// HCN: true = key held (gauge increase), false = key released (gauge decrease)
    pub(super) inclease: bool,
    /// HCN: μs accumulator for 200ms gauge update interval
    pub(super) passing_count: i64,
    /// Judgment at LN start (used for worst-of-three calculation)
    pub(super) ln_start_judge: usize,
    /// Timing offset at LN start (μs)
    pub(super) ln_start_duration: i64,
    /// Key release time (NOT_SET = not released yet)
    pub(super) release_time: i64,
    /// LN end judgment (set on key release, applied after release margin)
    pub(super) ln_end_judge: usize,
}

impl LaneState {
    pub(super) fn new(lane: usize, is_scratch: bool) -> Self {
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
pub(crate) struct MultiBadCollector {
    /// (note_index_in_all_notes, dmtime) pairs
    pub(super) entries: Vec<(usize, i64)>,
    /// true only for PMS mode
    enabled: bool,
}

impl MultiBadCollector {
    pub(super) fn new(enabled: bool) -> Self {
        Self {
            entries: Vec::new(),
            enabled,
        }
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(super) fn add(&mut self, note_index: usize, dmtime: i64) {
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
    pub(super) fn filter(
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

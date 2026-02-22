use crate::bms_model::BMSModel;
use crate::lane::Lane;
use crate::note::Note;

// Judge result constants: PG=0, GR=1, GD=2, BD=3, PR=4, MS=5
pub const JUDGE_PG: i32 = 0;
pub const JUDGE_GR: i32 = 1;
pub const JUDGE_GD: i32 = 2;
pub const JUDGE_BD: i32 = 3;
pub const JUDGE_PR: i32 = 4;
pub const JUDGE_MS: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JudgeNoteKind {
    Normal,
    LongStart,
    LongEnd,
    Mine,
}

#[derive(Clone, Debug)]
pub struct JudgeNote {
    pub time_us: i64,
    pub end_time_us: i64,
    pub lane: usize,
    pub wav: i32,
    pub kind: JudgeNoteKind,
    pub ln_type: i32,
    pub damage: f64,
    pub pair_index: Option<usize>,
}

impl JudgeNote {
    pub fn is_playable(&self) -> bool {
        matches!(
            self.kind,
            JudgeNoteKind::Normal | JudgeNoteKind::LongStart | JudgeNoteKind::LongEnd
        )
    }

    pub fn is_normal(&self) -> bool {
        self.kind == JudgeNoteKind::Normal
    }

    pub fn is_long_start(&self) -> bool {
        self.kind == JudgeNoteKind::LongStart
    }

    pub fn is_long_end(&self) -> bool {
        self.kind == JudgeNoteKind::LongEnd
    }

    pub fn is_mine(&self) -> bool {
        self.kind == JudgeNoteKind::Mine
    }

    pub fn is_long(&self) -> bool {
        matches!(self.kind, JudgeNoteKind::LongStart | JudgeNoteKind::LongEnd)
    }
}

/// Build a flat array of judge-ready notes from a BMSModel.
///
/// Notes are sorted by time (ascending). For notes at the same time, they are sorted
/// by lane index to ensure deterministic ordering.
/// LN start/end pairs are cross-linked via `pair_index` into the flat array.
/// `end_time_us` for LN start notes is set to the paired end note's time.
///
/// Pairing: the BMS decoder does not set `Note::pair` for LNTYPE_LONGNOTE channel notes
/// (51-59). We compute pairing here by matching each LongStart with the next LongEnd
/// in the same lane (stack-based, LIFO for nested LNs).
pub fn build_judge_notes(model: &BMSModel) -> Vec<JudgeNote> {
    let keys = model.get_mode().map(|m| m.key()).unwrap_or(0);
    let mut all_notes = Vec::new();

    for lane_idx in 0..keys {
        let lane = Lane::new(model, lane_idx);
        let lane_notes = lane.get_notes();
        let base_idx = all_notes.len();

        for note in lane_notes {
            let jn = match note {
                Note::Normal(data) => JudgeNote {
                    time_us: data.time,
                    end_time_us: data.time,
                    lane: lane_idx as usize,
                    wav: data.wav,
                    kind: JudgeNoteKind::Normal,
                    ln_type: 0,
                    damage: 0.0,
                    pair_index: None,
                },
                Note::Long {
                    data,
                    end,
                    pair,
                    note_type,
                } => JudgeNote {
                    time_us: data.time,
                    end_time_us: data.time,
                    lane: lane_idx as usize,
                    wav: data.wav,
                    kind: if *end {
                        JudgeNoteKind::LongEnd
                    } else {
                        JudgeNoteKind::LongStart
                    },
                    ln_type: *note_type,
                    damage: 0.0,
                    // Use Note::pair if set; otherwise pair_index will be fixed below.
                    pair_index: pair.map(|p| p + base_idx),
                },
                Note::Mine { data, damage } => JudgeNote {
                    time_us: data.time,
                    end_time_us: data.time,
                    lane: lane_idx as usize,
                    wav: data.wav,
                    kind: JudgeNoteKind::Mine,
                    ln_type: 0,
                    damage: *damage,
                    pair_index: None,
                },
            };
            all_notes.push(jn);
        }

        // For LN notes with pair_index=None (BMS decoder doesn't set Note::pair),
        // compute pairing by matching LongStart with the next LongEnd in this lane.
        // Uses a stack for potential nested LNs.
        let lane_end = all_notes.len();
        let mut start_stack: Vec<usize> = Vec::new();
        for i in base_idx..lane_end {
            if all_notes[i].pair_index.is_some() {
                // pair already set (e.g. from Note::pair), skip auto-pairing
                continue;
            }
            match all_notes[i].kind {
                JudgeNoteKind::LongStart => {
                    start_stack.push(i);
                }
                JudgeNoteKind::LongEnd => {
                    if let Some(start_idx) = start_stack.pop() {
                        all_notes[start_idx].pair_index = Some(i);
                        all_notes[i].pair_index = Some(start_idx);
                    }
                }
                _ => {}
            }
        }
    }

    // Fix end_time_us for LN start notes
    for i in 0..all_notes.len() {
        if all_notes[i].kind == JudgeNoteKind::LongStart
            && let Some(pair_idx) = all_notes[i].pair_index
            && pair_idx < all_notes.len()
        {
            all_notes[i].end_time_us = all_notes[pair_idx].time_us;
        }
    }

    // Create index array to track old positions
    let mut indices: Vec<usize> = (0..all_notes.len()).collect();

    // Sort indices by the note properties (time, lane)
    indices.sort_by_key(|&i| (all_notes[i].time_us, all_notes[i].lane));

    // Create a mapping from old index to new index
    let mut old_to_new = vec![0; all_notes.len()];
    for new_idx in 0..indices.len() {
        old_to_new[indices[new_idx]] = new_idx;
    }

    // Reorder notes according to sorted indices
    let sorted_notes: Vec<JudgeNote> = indices.iter().map(|&i| all_notes[i].clone()).collect();

    // Update pair_index values to reflect new indices
    let mut result = sorted_notes;
    for note in &mut result {
        if let Some(old_pair_idx) = note.pair_index {
            note.pair_index = Some(old_to_new[old_pair_idx]);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms_model::BMSModel;
    use crate::mode::Mode;
    use crate::note;
    use crate::time_line::TimeLine;

    #[test]
    fn judge_constants() {
        assert_eq!(JUDGE_PG, 0);
        assert_eq!(JUDGE_GR, 1);
        assert_eq!(JUDGE_GD, 2);
        assert_eq!(JUDGE_BD, 3);
        assert_eq!(JUDGE_PR, 4);
        assert_eq!(JUDGE_MS, 5);
    }

    #[test]
    fn judge_note_kind_is_playable() {
        let normal = JudgeNote {
            time_us: 0,
            end_time_us: 0,
            lane: 0,
            wav: 1,
            kind: JudgeNoteKind::Normal,
            ln_type: 0,
            damage: 0.0,
            pair_index: None,
        };
        assert!(normal.is_playable());

        let mine = JudgeNote {
            kind: JudgeNoteKind::Mine,
            damage: 0.5,
            ..normal.clone()
        };
        assert!(!mine.is_playable());

        let ln_start = JudgeNote {
            kind: JudgeNoteKind::LongStart,
            ..normal.clone()
        };
        assert!(ln_start.is_playable());

        let ln_end = JudgeNote {
            kind: JudgeNoteKind::LongEnd,
            ..normal
        };
        assert!(ln_end.is_playable());
    }

    #[test]
    fn build_judge_notes_empty_model() {
        let model = BMSModel::new();
        let notes = build_judge_notes(&model);
        assert!(notes.is_empty());
    }

    #[test]
    fn build_judge_notes_normal_notes() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut tl = TimeLine::new(0.0, 1_000_000, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(2, Some(Note::new_normal(2)));

        let mut tl2 = TimeLine::new(1.0, 2_000_000, 8);
        tl2.set_note(0, Some(Note::new_normal(3)));

        model.set_all_time_line(vec![tl, tl2]);

        let notes = build_judge_notes(&model);
        assert_eq!(notes.len(), 3);

        // Notes should be time-ordered, then lane-ordered
        // Expected order: lane0@1s, lane2@1s, lane0@2s
        assert_eq!(notes[0].time_us, 1_000_000);
        assert_eq!(notes[0].lane, 0);
        assert!(notes[0].is_normal());

        assert_eq!(notes[1].time_us, 1_000_000);
        assert_eq!(notes[1].lane, 2);
        assert!(notes[1].is_normal());

        assert_eq!(notes[2].time_us, 2_000_000);
        assert_eq!(notes[2].lane, 0);
        assert!(notes[2].is_normal());
    }

    #[test]
    fn build_judge_notes_mine() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut tl = TimeLine::new(0.0, 500_000, 8);
        tl.set_note(1, Some(Note::new_mine(10, 0.75)));

        model.set_all_time_line(vec![tl]);

        let notes = build_judge_notes(&model);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].is_mine());
        assert!(!notes[0].is_playable());
        assert!((notes[0].damage - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn build_judge_notes_longnote_pairing() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        // Create LN start at t=1s, end at t=2s
        let mut ln_start = Note::new_long(5);
        ln_start.set_micro_time(1_000_000);
        ln_start.set_pair_index(Some(1));

        let mut ln_end = Note::new_long(5);
        ln_end.set_micro_time(2_000_000);
        ln_end.set_end(true);
        ln_end.set_pair_index(Some(0));

        let mut tl1 = TimeLine::new(0.0, 1_000_000, 8);
        tl1.set_note(0, Some(ln_start));

        let mut tl2 = TimeLine::new(1.0, 2_000_000, 8);
        tl2.set_note(0, Some(ln_end));

        model.set_all_time_line(vec![tl1, tl2]);

        let notes = build_judge_notes(&model);
        assert_eq!(notes.len(), 2);

        // LN start
        assert!(notes[0].is_long_start());
        assert_eq!(notes[0].time_us, 1_000_000);
        assert_eq!(notes[0].end_time_us, 2_000_000);
        assert_eq!(notes[0].pair_index, Some(1));

        // LN end
        assert!(notes[1].is_long_end());
        assert_eq!(notes[1].time_us, 2_000_000);
        assert_eq!(notes[1].pair_index, Some(0));
    }

    #[test]
    fn build_judge_notes_sets_ln_type() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut ln = Note::new_long(1);
        ln.set_long_note_type(note::TYPE_CHARGENOTE);
        ln.set_micro_time(500_000);

        let mut tl = TimeLine::new(0.0, 500_000, 8);
        tl.set_note(0, Some(ln));

        model.set_all_time_line(vec![tl]);

        let notes = build_judge_notes(&model);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].ln_type, note::TYPE_CHARGENOTE);
    }

    #[test]
    fn build_judge_notes_time_ordered() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        // Create notes in different lanes at different times
        let mut tl1 = TimeLine::new(0.0, 1_000_000, 8);
        tl1.set_note(0, Some(Note::new_normal(1))); // lane 0, t=1s
        tl1.set_note(2, Some(Note::new_normal(2))); // lane 2, t=1s

        let mut tl2 = TimeLine::new(1.0, 2_000_000, 8);
        tl2.set_note(0, Some(Note::new_normal(3))); // lane 0, t=2s

        model.set_all_time_line(vec![tl1, tl2]);

        let notes = build_judge_notes(&model);
        assert_eq!(notes.len(), 3);

        // Verify ordering: notes should be time-ordered
        for window in notes.windows(2) {
            assert!(
                window[1].time_us >= window[0].time_us,
                "Notes should be time-ordered"
            );
        }
    }
}

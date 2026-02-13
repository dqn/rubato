use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::mode::PlayMode;
use crate::note::{BgNote, LnType, Note};
use crate::timeline::{BgaEvent, BpmChange, StopEvent, TimeLine};

/// Type of judge rank value stored in the BMS model.
///
/// Java's `BMSPlayerRule.validate()` converts the raw value differently
/// depending on this type, using the window rule's judgerank table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum JudgeRankType {
    /// `#RANK N` (0-4) — needs conversion via window rule's judgerank table.
    /// Index into `[VERYHARD, HARD, NORMAL, EASY, VERYEASY]`.
    #[default]
    BmsRank,
    /// `#DEFEXRANK N` — percentage scaled by window rule's default.
    /// `judgerank = raw * windowrule.judgerank[2] / 100`
    BmsDefExRank,
    /// bmson `judge_rank` — direct judgerank value (100 = standard).
    BmsonJudgeRank,
}

/// Complete BMS chart model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BmsModel {
    // Metadata
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub sub_artist: String,
    pub genre: String,
    pub banner: String,
    pub stage_file: String,
    pub back_bmp: String,
    pub preview: String,

    // Difficulty
    pub play_level: i32,
    /// Raw judge rank value as specified in BMS file.
    /// For `#RANK`: 0-4 (index). For `#DEFEXRANK`: percentage. For bmson: direct value.
    /// Use `JudgeWindowRule::resolve_judge_rank()` with the appropriate rule to
    /// convert to the effective judgerank for judge window calculation.
    pub judge_rank: i32,
    /// Raw judge rank value for database storage (same as judge_rank).
    pub judge_rank_raw: i32,
    /// Type of judge rank value, determines how to convert to effective judgerank.
    pub judge_rank_type: JudgeRankType,
    pub total: f64,
    pub difficulty: i32,

    // Mode
    pub mode: PlayMode,
    pub ln_type: LnType,
    pub player: i32,

    // BPM / Timing
    pub initial_bpm: f64,
    pub bpm_changes: Vec<BpmChange>,
    pub stop_events: Vec<StopEvent>,
    pub timelines: Vec<TimeLine>,

    // Notes
    pub notes: Vec<Note>,

    // Background notes (BGM channel 0x01 / bmson BGM)
    pub bg_notes: Vec<BgNote>,

    // BGA events (channels 04/06/07)
    pub bga_events: Vec<BgaEvent>,

    // WAV/BMP definitions
    #[serde(skip)]
    pub wav_defs: HashMap<u16, PathBuf>,
    #[serde(skip)]
    pub bmp_defs: HashMap<u16, PathBuf>,

    // Hashes (computed after parsing)
    pub md5: String,
    pub sha256: String,

    // Total measure count
    pub total_measures: u32,

    // Total play time in microseconds
    pub total_time_us: i64,

    // Whether the chart contains #RANDOM commands
    pub has_random: bool,
}

impl Default for BmsModel {
    fn default() -> Self {
        Self {
            title: String::new(),
            subtitle: String::new(),
            artist: String::new(),
            sub_artist: String::new(),
            genre: String::new(),
            banner: String::new(),
            stage_file: String::new(),
            back_bmp: String::new(),
            preview: String::new(),
            play_level: 0,
            judge_rank: 2,
            judge_rank_raw: 2,
            judge_rank_type: JudgeRankType::BmsRank,
            total: 300.0,
            difficulty: 0,
            mode: PlayMode::Beat7K,
            ln_type: LnType::LongNote,
            player: 1,
            initial_bpm: 130.0,
            bpm_changes: Vec::new(),
            stop_events: Vec::new(),
            timelines: Vec::new(),
            notes: Vec::new(),
            bg_notes: Vec::new(),
            bga_events: Vec::new(),
            wav_defs: HashMap::new(),
            bmp_defs: HashMap::new(),
            md5: String::new(),
            sha256: String::new(),
            total_measures: 0,
            total_time_us: 0,
            has_random: false,
        }
    }
}

impl BmsModel {
    /// Number of playable notes (excludes mines and invisible)
    pub fn total_notes(&self) -> usize {
        self.notes.iter().filter(|n| n.is_playable()).count()
    }

    /// Number of long notes
    pub fn total_long_notes(&self) -> usize {
        self.notes.iter().filter(|n| n.is_long_note()).count()
    }

    /// Get notes for a specific lane, sorted by time
    pub fn lane_notes(&self, lane: usize) -> Vec<&Note> {
        let mut notes: Vec<&Note> = self.notes.iter().filter(|n| n.lane == lane).collect();
        notes.sort_by_key(|n| n.time_us);
        notes
    }

    /// Get all playable notes sorted by time
    pub fn playable_notes(&self) -> Vec<&Note> {
        let mut notes: Vec<&Note> = self.notes.iter().filter(|n| n.is_playable()).collect();
        notes.sort_by_key(|n| n.time_us);
        notes
    }

    /// Minimum BPM in the chart
    pub fn min_bpm(&self) -> f64 {
        self.bpm_changes
            .iter()
            .map(|c| c.bpm)
            .fold(self.initial_bpm, f64::min)
    }

    /// Maximum BPM in the chart
    pub fn max_bpm(&self) -> f64 {
        self.bpm_changes
            .iter()
            .map(|c| c.bpm)
            .fold(self.initial_bpm, f64::max)
    }

    /// Main (most frequent) BPM in the chart, weighted by time duration.
    ///
    /// Each BPM segment's duration is computed from `bpm_changes` and `total_time_us`.
    /// Returns `initial_bpm` if there are no BPM changes.
    pub fn main_bpm(&self) -> f64 {
        if self.bpm_changes.is_empty() {
            return self.initial_bpm;
        }

        // Accumulate duration per distinct BPM value.
        // Key: BPM bits (f64::to_bits) to group identical BPMs exactly.
        let mut durations: HashMap<u64, (f64, i64)> = HashMap::new();

        // Initial BPM segment: from 0 to first change
        let first_time = self.bpm_changes[0].time_us;
        if first_time > 0 {
            let key = self.initial_bpm.to_bits();
            durations.entry(key).or_insert((self.initial_bpm, 0)).1 += first_time;
        }

        // BPM change segments
        for i in 0..self.bpm_changes.len() {
            let bpm = self.bpm_changes[i].bpm;
            let start = self.bpm_changes[i].time_us;
            let end = if i + 1 < self.bpm_changes.len() {
                self.bpm_changes[i + 1].time_us
            } else {
                self.total_time_us
            };
            let duration = end - start;
            if duration > 0 {
                let key = bpm.to_bits();
                durations.entry(key).or_insert((bpm, 0)).1 += duration;
            }
        }

        durations
            .into_values()
            .max_by_key(|&(_, d)| d)
            .map(|(bpm, _)| bpm)
            .unwrap_or(self.initial_bpm)
    }

    /// BPM at a given time in microseconds.
    ///
    /// Walks `bpm_changes` (assumed sorted by time) and returns the
    /// active BPM at `time_us`. Returns `initial_bpm` if before any change.
    pub fn bpm_at(&self, time_us: i64) -> f64 {
        let mut bpm = self.initial_bpm;
        for change in &self.bpm_changes {
            if change.time_us <= time_us {
                bpm = change.bpm;
            } else {
                break;
            }
        }
        bpm
    }

    /// Time of the last note/event in milliseconds.
    /// Equivalent to Java `BMSModel.getLastTime()` — returns the time of the last
    /// timeline that contains any note (playable, invisible, or mine), including
    /// LN end positions.
    pub fn last_event_time_ms(&self) -> i32 {
        self.notes
            .iter()
            .map(|n| {
                if n.is_long_note() && n.end_time_us > n.time_us {
                    n.end_time_us
                } else {
                    n.time_us
                }
            })
            .max()
            .map(|us| (us / 1000) as i32)
            .unwrap_or(0)
    }

    /// Build note list for JudgeManager by splitting LN into start+end pairs.
    ///
    /// The parser stores LN as a single note with `time_us` (start) and
    /// `end_time_us` (end). JudgeManager expects two separate notes linked
    /// by `pair_index`. This method performs that conversion.
    ///
    /// Non-LN notes are cloned as-is. `self.notes` is not modified.
    pub fn build_judge_notes(&self) -> Vec<Note> {
        use crate::note::NoteType;

        // Collect original notes + generated end notes with origin tracking.
        // pairs: Vec<(note, Option<origin_index>)> where origin_index links
        // an end note back to its start note's position in this vec.
        let mut notes: Vec<Note> = Vec::with_capacity(self.notes.len() * 2);
        // (start_index_in_notes_vec, end_index_in_notes_vec) pairs to link after sorting
        let mut pending_pairs: Vec<(usize, usize)> = Vec::new();

        for note in &self.notes {
            let start_idx = notes.len();
            notes.push(note.clone());

            if note.is_long_note() && note.end_time_us > note.time_us {
                let end_idx = notes.len();
                notes.push(Note {
                    lane: note.lane,
                    note_type: note.note_type,
                    time_us: note.end_time_us,
                    end_time_us: 0,
                    wav_id: note.end_wav_id,
                    end_wav_id: 0,
                    damage: 0,
                    pair_index: usize::MAX,
                    micro_starttime: 0,
                    micro_duration: 0,
                });
                pending_pairs.push((start_idx, end_idx));
            }
        }

        // Build old→new index mapping after stable sort by (time_us, lane)
        let mut indices: Vec<usize> = (0..notes.len()).collect();
        indices.sort_by(|&a, &b| {
            notes[a]
                .time_us
                .cmp(&notes[b].time_us)
                .then(notes[a].lane.cmp(&notes[b].lane))
        });

        // inverse mapping: old_index → new_index
        let mut old_to_new = vec![0usize; notes.len()];
        for (new_idx, &old_idx) in indices.iter().enumerate() {
            old_to_new[old_idx] = new_idx;
        }

        // Reorder notes
        let mut sorted = vec![
            Note {
                lane: 0,
                note_type: NoteType::Normal,
                time_us: 0,
                end_time_us: 0,
                wav_id: 0,
                end_wav_id: 0,
                damage: 0,
                pair_index: usize::MAX,
                micro_starttime: 0,
                micro_duration: 0,
            };
            notes.len()
        ];
        for (old_idx, note) in notes.into_iter().enumerate() {
            sorted[old_to_new[old_idx]] = note;
        }

        // Set pair_index bidirectionally
        for (start_old, end_old) in pending_pairs {
            let start_new = old_to_new[start_old];
            let end_new = old_to_new[end_old];
            sorted[start_new].pair_index = end_new;
            sorted[end_new].pair_index = start_new;
        }

        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::{LnType, NoteType};

    fn make_model(notes: Vec<Note>) -> BmsModel {
        BmsModel {
            notes,
            ..Default::default()
        }
    }

    #[test]
    fn normal_notes_unchanged() {
        let model = make_model(vec![
            Note::normal(0, 1_000_000, 1),
            Note::normal(1, 2_000_000, 2),
            Note::normal(2, 3_000_000, 3),
        ]);
        let judge = model.build_judge_notes();
        assert_eq!(judge.len(), 3);
        for n in &judge {
            assert_eq!(n.pair_index, usize::MAX);
            assert!(!n.is_long_note());
        }
    }

    #[test]
    fn single_ln_split() {
        let model = make_model(vec![Note::long_note(
            0,
            1_000_000,
            2_000_000,
            10,
            11,
            LnType::LongNote,
        )]);
        let judge = model.build_judge_notes();
        assert_eq!(judge.len(), 2);

        let start = &judge[0];
        let end = &judge[1];

        assert_eq!(start.time_us, 1_000_000);
        assert_eq!(start.end_time_us, 2_000_000);
        assert_eq!(start.wav_id, 10);
        assert_eq!(start.note_type, NoteType::LongNote);

        assert_eq!(end.time_us, 2_000_000);
        assert_eq!(end.end_time_us, 0);
        assert_eq!(end.wav_id, 11);
        assert_eq!(end.note_type, NoteType::LongNote);

        assert_eq!(start.pair_index, 1);
        assert_eq!(end.pair_index, 0);
    }

    #[test]
    fn multiple_ln_different_lanes() {
        let model = make_model(vec![
            Note::long_note(0, 1_000_000, 3_000_000, 1, 2, LnType::LongNote),
            Note::long_note(1, 2_000_000, 4_000_000, 3, 4, LnType::LongNote),
        ]);
        let judge = model.build_judge_notes();
        assert_eq!(judge.len(), 4);

        // Verify all pairs are bidirectional
        for i in 0..judge.len() {
            if judge[i].pair_index != usize::MAX {
                let pair = judge[i].pair_index;
                assert_eq!(
                    judge[pair].pair_index, i,
                    "pair_index not bidirectional at {i}"
                );
            }
        }

        // Count LN starts (end_time_us > 0) and ends (end_time_us == 0)
        let starts: Vec<_> = judge
            .iter()
            .filter(|n| n.is_long_note() && n.end_time_us > 0)
            .collect();
        let ends: Vec<_> = judge
            .iter()
            .filter(|n| n.is_long_note() && n.end_time_us == 0)
            .collect();
        assert_eq!(starts.len(), 2);
        assert_eq!(ends.len(), 2);
    }

    #[test]
    fn mixed_notes_sorted() {
        let model = make_model(vec![
            Note::normal(0, 3_000_000, 1),
            Note::long_note(1, 1_000_000, 4_000_000, 2, 3, LnType::LongNote),
            Note::mine(2, 2_000_000, 4, 10),
        ]);
        let judge = model.build_judge_notes();
        // 3 original + 1 LN end = 4
        assert_eq!(judge.len(), 4);

        // Verify time ordering
        for w in judge.windows(2) {
            assert!(
                w[0].time_us <= w[1].time_us,
                "not sorted: {} > {}",
                w[0].time_us,
                w[1].time_us
            );
        }
    }

    #[test]
    fn end_note_properties() {
        let model = make_model(vec![Note::long_note(
            3,
            1_000_000,
            5_000_000,
            20,
            25,
            LnType::ChargeNote,
        )]);
        let judge = model.build_judge_notes();
        let end = judge.iter().find(|n| n.end_time_us == 0).unwrap();

        assert_eq!(end.lane, 3);
        assert_eq!(end.note_type, NoteType::ChargeNote);
        assert_eq!(end.time_us, 5_000_000);
        assert_eq!(end.wav_id, 25);
        assert_eq!(end.end_wav_id, 0);
        assert_eq!(end.damage, 0);
        assert_eq!(end.micro_starttime, 0);
        assert_eq!(end.micro_duration, 0);
    }

    #[test]
    fn pair_index_bidirectional() {
        let model = make_model(vec![
            Note::long_note(0, 1_000_000, 2_000_000, 1, 2, LnType::LongNote),
            Note::long_note(1, 3_000_000, 4_000_000, 3, 4, LnType::ChargeNote),
            Note::long_note(2, 5_000_000, 6_000_000, 5, 6, LnType::HellChargeNote),
        ]);
        let judge = model.build_judge_notes();
        assert_eq!(judge.len(), 6);

        for i in 0..judge.len() {
            let pi = judge[i].pair_index;
            assert_ne!(pi, usize::MAX, "note {i} should have pair_index set");
            assert_eq!(
                judge[pi].pair_index, i,
                "notes[notes[{i}].pair_index].pair_index != {i}"
            );
        }
    }

    #[test]
    fn playable_count_includes_ends() {
        let model = make_model(vec![
            Note::long_note(0, 1_000_000, 2_000_000, 1, 2, LnType::LongNote),
            Note::normal(1, 3_000_000, 3),
        ]);
        let judge = model.build_judge_notes();
        let playable = judge.iter().filter(|n| n.is_playable()).count();
        // 1 LN start + 1 LN end + 1 normal = 3
        assert_eq!(playable, 3);
    }

    #[test]
    fn main_bpm_no_changes() {
        let model = BmsModel {
            initial_bpm: 150.0,
            ..Default::default()
        };
        assert!((model.main_bpm() - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn main_bpm_single_bpm() {
        let model = BmsModel {
            initial_bpm: 120.0,
            bpm_changes: vec![BpmChange {
                time_us: 0,
                bpm: 120.0,
            }],
            total_time_us: 10_000_000,
            ..Default::default()
        };
        assert!((model.main_bpm() - 120.0).abs() < f64::EPSILON);
    }

    #[test]
    fn main_bpm_multiple_bpms() {
        // 120 BPM for 8s, 180 BPM for 2s -> main = 120
        let model = BmsModel {
            initial_bpm: 120.0,
            bpm_changes: vec![BpmChange {
                time_us: 8_000_000,
                bpm: 180.0,
            }],
            total_time_us: 10_000_000,
            ..Default::default()
        };
        assert!((model.main_bpm() - 120.0).abs() < f64::EPSILON);
    }

    #[test]
    fn main_bpm_dominant_later_bpm() {
        // 120 BPM for 2s, 180 BPM for 8s -> main = 180
        let model = BmsModel {
            initial_bpm: 120.0,
            bpm_changes: vec![BpmChange {
                time_us: 2_000_000,
                bpm: 180.0,
            }],
            total_time_us: 10_000_000,
            ..Default::default()
        };
        assert!((model.main_bpm() - 180.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bpm_at_before_any_change() {
        let model = BmsModel {
            initial_bpm: 130.0,
            bpm_changes: vec![BpmChange {
                time_us: 5_000_000,
                bpm: 200.0,
            }],
            ..Default::default()
        };
        assert!((model.bpm_at(1_000_000) - 130.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bpm_at_after_change() {
        let model = BmsModel {
            initial_bpm: 130.0,
            bpm_changes: vec![BpmChange {
                time_us: 5_000_000,
                bpm: 200.0,
            }],
            ..Default::default()
        };
        assert!((model.bpm_at(6_000_000) - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bpm_at_exact_change_time() {
        let model = BmsModel {
            initial_bpm: 130.0,
            bpm_changes: vec![BpmChange {
                time_us: 5_000_000,
                bpm: 200.0,
            }],
            ..Default::default()
        };
        assert!((model.bpm_at(5_000_000) - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn charge_note_and_hell_charge() {
        for ln_type in [LnType::ChargeNote, LnType::HellChargeNote] {
            let model = make_model(vec![Note::long_note(
                0, 1_000_000, 2_000_000, 1, 2, ln_type,
            )]);
            let judge = model.build_judge_notes();
            assert_eq!(judge.len(), 2, "failed for {ln_type:?}");

            let start = &judge[0];
            let end = &judge[1];
            assert_eq!(start.pair_index, 1);
            assert_eq!(end.pair_index, 0);

            let expected_type = match ln_type {
                LnType::ChargeNote => NoteType::ChargeNote,
                LnType::HellChargeNote => NoteType::HellChargeNote,
                LnType::LongNote => NoteType::LongNote,
            };
            assert_eq!(start.note_type, expected_type);
            assert_eq!(end.note_type, expected_type);
        }
    }
}

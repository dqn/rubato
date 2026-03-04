use bms_model::bms_model::BMSModel;
use bms_model::note::{Note, TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_LONGNOTE, TYPE_UNDEFINED};

use crate::pattern::java_random::JavaRandom;
use crate::pattern::pattern_modifier::{AssistLevel, PatternModifier, PatternModifierBase};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Remove,
    AddLn,
    AddCn,
    AddHcn,
    AddAll,
}

impl Mode {
    pub fn values() -> &'static [Mode] {
        &[
            Mode::Remove,
            Mode::AddLn,
            Mode::AddCn,
            Mode::AddHcn,
            Mode::AddAll,
        ]
    }

    pub fn from_index(index: i32) -> Mode {
        let values = Self::values();
        if index >= 0 && (index as usize) < values.len() {
            values[index as usize]
        } else {
            Mode::Remove
        }
    }
}

pub struct LongNoteModifier {
    pub base: PatternModifierBase,
    mode: Mode,
    rate: f64,
}

impl Default for LongNoteModifier {
    fn default() -> Self {
        Self::new()
    }
}

impl LongNoteModifier {
    pub fn new() -> Self {
        LongNoteModifier {
            base: PatternModifierBase::new(),
            mode: Mode::Remove,
            rate: 1.0,
        }
    }

    pub fn with_params(mode: i32, rate: f64) -> Self {
        LongNoteModifier {
            base: PatternModifierBase::new(),
            mode: Mode::from_index(mode),
            rate,
        }
    }
}

impl PatternModifier for LongNoteModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        let mode_key = model.get_mode().map(|m| m.key()).unwrap_or(0);
        let mut rng = JavaRandom::new(self.base.seed);

        if self.mode == Mode::Remove {
            let mut assist = AssistLevel::None;
            let timelines = model.get_all_time_lines_mut();
            for tl in timelines.iter_mut() {
                for lane in 0..mode_key {
                    if let Some(note) = tl.get_note(lane)
                        && note.is_long()
                        && rng.next_double() < self.rate
                    {
                        let replacement = if note.is_end() {
                            None
                        } else {
                            Some(Note::new_normal(note.get_wav()))
                        };
                        tl.set_note(lane, replacement);
                        assist = AssistLevel::Assist;
                    }
                }
            }
            self.base.assist = assist;
        } else {
            let mut assist = AssistLevel::None;

            let timelines = model.get_all_time_lines_mut();
            let tl_len = timelines.len();
            for i in 0..tl_len - 1 {
                for lane in 0..mode_key {
                    let is_normal = timelines[i]
                        .get_note(lane)
                        .map(|n| n.is_normal())
                        .unwrap_or(false);
                    let next_empty = !timelines[i + 1].exist_note_at(lane);
                    if is_normal && next_empty && rng.next_double() < self.rate {
                        let lntype = match self.mode {
                            Mode::AddLn => TYPE_LONGNOTE,
                            Mode::AddCn => TYPE_CHARGENOTE,
                            Mode::AddHcn => TYPE_HELLCHARGENOTE,
                            Mode::AddAll => (rng.next_double() * 3.0 + 1.0) as i32,
                            _ => TYPE_UNDEFINED,
                        };

                        if lntype != TYPE_LONGNOTE {
                            assist = AssistLevel::Assist;
                        }

                        let wav = timelines[i].get_note(lane).unwrap().get_wav();
                        let start = timelines[i].get_note(lane).unwrap().get_micro_starttime();
                        let duration = timelines[i].get_note(lane).unwrap().get_micro_duration();

                        let mut lnstart = Note::new_long_with_start_duration(wav, start, duration);
                        lnstart.set_long_note_type(lntype);
                        let lnend = Note::new_long(-2);

                        timelines[i].set_note(lane, Some(lnstart));
                        timelines[i + 1].set_note(lane, Some(lnend));
                        // Note: pair setting would need timeline index tracking
                    }
                }
            }
            self.base.assist = assist;
        }
    }

    fn get_assist_level(&self) -> AssistLevel {
        self.base.assist
    }

    fn set_assist_level(&mut self, assist: AssistLevel) {
        self.base.assist = assist;
    }

    fn get_seed(&self) -> i64 {
        self.base.seed
    }

    fn set_seed(&mut self, seed: i64) {
        if seed >= 0 {
            self.base.seed = seed;
        }
    }

    fn get_player(&self) -> i32 {
        self.base.player
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::pattern_modifier::{PatternModifier, make_test_model};
    use bms_model::mode::Mode as BmsMode;
    use bms_model::note::{Note, TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_LONGNOTE};
    use bms_model::time_line::TimeLine;

    // -- Mode::from_index boundary tests --

    #[test]
    fn from_index_valid_values() {
        assert_eq!(Mode::from_index(0), Mode::Remove);
        assert_eq!(Mode::from_index(1), Mode::AddLn);
        assert_eq!(Mode::from_index(2), Mode::AddCn);
        assert_eq!(Mode::from_index(3), Mode::AddHcn);
        assert_eq!(Mode::from_index(4), Mode::AddAll);
    }

    #[test]
    fn from_index_negative_returns_remove() {
        assert_eq!(Mode::from_index(-1), Mode::Remove);
    }

    #[test]
    fn from_index_out_of_bounds_returns_remove() {
        assert_eq!(Mode::from_index(5), Mode::Remove);
        assert_eq!(Mode::from_index(100), Mode::Remove);
    }

    #[test]
    fn mode_values_returns_all_variants() {
        let values = Mode::values();
        assert_eq!(values.len(), 5);
        assert_eq!(values[0], Mode::Remove);
        assert_eq!(values[4], Mode::AddAll);
    }

    // -- LongNoteModifier construction --

    #[test]
    fn new_defaults() {
        let m = LongNoteModifier::new();
        assert_eq!(m.mode, Mode::Remove);
        assert!((m.rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(m.base.assist, AssistLevel::None);
    }

    #[test]
    fn with_params_sets_mode_and_rate() {
        let m = LongNoteModifier::with_params(1, 0.5);
        assert_eq!(m.mode, Mode::AddLn);
        assert!((m.rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn default_matches_new() {
        let from_new = LongNoteModifier::new();
        let from_default = LongNoteModifier::default();
        assert_eq!(from_new.mode, from_default.mode);
        assert!((from_new.rate - from_default.rate).abs() < f64::EPSILON);
    }

    // -- PatternModifier trait methods --

    #[test]
    fn set_seed_positive() {
        let mut m = LongNoteModifier::new();
        m.set_seed(42);
        assert_eq!(m.get_seed(), 42);
    }

    #[test]
    fn set_seed_negative_ignored() {
        let mut m = LongNoteModifier::new();
        let original = m.get_seed();
        m.set_seed(-1);
        assert_eq!(m.get_seed(), original);
    }

    #[test]
    fn set_assist_level() {
        let mut m = LongNoteModifier::new();
        m.set_assist_level(AssistLevel::Assist);
        assert_eq!(m.get_assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn get_player_default() {
        let m = LongNoteModifier::new();
        assert_eq!(m.get_player(), 0);
    }

    // -- Mode::Remove with rate=1.0 (remove all LNs) --

    #[test]
    fn remove_mode_rate_1_converts_ln_start_to_normal() {
        // Setup: model with BEAT_7K, 2 timelines
        // tl[0]: lane 0 has LN start (wav=1)
        // tl[1]: lane 0 has LN end
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        let mut ln_start = Note::new_long(1);
        ln_start.set_long_note_type(TYPE_LONGNOTE);
        tl0.set_note(0, Some(ln_start));

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        let mut ln_end = Note::new_long(-2);
        ln_end.set_end(true);
        tl1.set_note(0, Some(ln_end));

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = LongNoteModifier::with_params(0, 1.0); // Remove, rate=1.0
        modifier.set_seed(42);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // tl[0] lane 0 should have a normal note with wav=1
        let note0 = tls[0].get_note(0).expect("should have note at lane 0");
        assert!(note0.is_normal());
        assert_eq!(note0.get_wav(), 1);

        // tl[1] lane 0 should be None (LN end removed)
        assert!(tls[1].get_note(0).is_none());

        // assist level should be Assist
        assert_eq!(modifier.get_assist_level(), AssistLevel::Assist);
    }

    // -- Mode::Remove with rate=0.0 (remove no LNs) --

    #[test]
    fn remove_mode_rate_0_leaves_lns_unchanged() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        let mut ln_start = Note::new_long(1);
        ln_start.set_long_note_type(TYPE_LONGNOTE);
        tl0.set_note(0, Some(ln_start));

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        let mut ln_end = Note::new_long(-2);
        ln_end.set_end(true);
        tl1.set_note(0, Some(ln_end));

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = LongNoteModifier::with_params(0, 0.0); // Remove, rate=0.0
        modifier.set_seed(0);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // Both notes should remain as LN (next_double() is always >= 0.0, never < 0.0)
        assert!(tls[0].get_note(0).unwrap().is_long());
        assert!(tls[1].get_note(0).unwrap().is_long());

        // No changes made, so assist should remain None
        assert_eq!(modifier.get_assist_level(), AssistLevel::None);
    }

    // -- Mode::AddLn with rate=1.0 --

    #[test]
    fn add_ln_converts_normal_to_longnote() {
        // Setup: 3 timelines, tl[0] has normal note, tl[1] is empty, tl[2] is sentinel
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let tl1 = TimeLine::new(1.0, 1_000_000, 8);
        let tl2 = TimeLine::new(2.0, 2_000_000, 8);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = LongNoteModifier::with_params(1, 1.0); // AddLn, rate=1.0
        modifier.set_seed(42);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // tl[0] lane 0 should have LN start with TYPE_LONGNOTE
        let note0 = tls[0].get_note(0).expect("should have LN start");
        assert!(note0.is_long());
        assert_eq!(note0.get_long_note_type(), TYPE_LONGNOTE);

        // tl[1] lane 0 should have LN end
        let note1 = tls[1].get_note(0).expect("should have LN end");
        assert!(note1.is_long());

        // AddLn with TYPE_LONGNOTE does not set assist (only non-LONGNOTE types set Assist)
        assert_eq!(modifier.get_assist_level(), AssistLevel::None);
    }

    // -- Mode::AddCn --

    #[test]
    fn add_cn_converts_normal_to_chargenote() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let tl1 = TimeLine::new(1.0, 1_000_000, 8);
        let tl2 = TimeLine::new(2.0, 2_000_000, 8);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = LongNoteModifier::with_params(2, 1.0); // AddCn
        modifier.set_seed(42);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        let note0 = tls[0].get_note(0).expect("should have LN start");
        assert!(note0.is_long());
        assert_eq!(note0.get_long_note_type(), TYPE_CHARGENOTE);

        let note1 = tls[1].get_note(0).expect("should have LN end");
        assert!(note1.is_long());

        // Non-LONGNOTE type sets Assist
        assert_eq!(modifier.get_assist_level(), AssistLevel::Assist);
    }

    // -- Mode::AddHcn --

    #[test]
    fn add_hcn_converts_normal_to_hellchargenote() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let tl1 = TimeLine::new(1.0, 1_000_000, 8);
        let tl2 = TimeLine::new(2.0, 2_000_000, 8);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = LongNoteModifier::with_params(3, 1.0); // AddHcn
        modifier.set_seed(42);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        let note0 = tls[0].get_note(0).expect("should have LN start");
        assert!(note0.is_long());
        assert_eq!(note0.get_long_note_type(), TYPE_HELLCHARGENOTE);

        assert_eq!(modifier.get_assist_level(), AssistLevel::Assist);
    }

    // -- Normal note followed by non-empty lane: no conversion --

    #[test]
    fn add_ln_skips_when_next_lane_occupied() {
        // tl[0] lane 0 = normal, tl[1] lane 0 = normal (occupied)
        // Only 2 timelines: loop goes i=0 only, next=tl[1] has a note -> no conversion
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_note(0, Some(Note::new_normal(2)));

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = LongNoteModifier::with_params(1, 1.0); // AddLn
        modifier.set_seed(42);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // tl[0] lane 0 should remain normal (next_empty is false for tl[1] lane 0)
        assert!(tls[0].get_note(0).unwrap().is_normal());
        assert_eq!(tls[0].get_note(0).unwrap().get_wav(), 1);
    }

    // -- AddAll mode assigns a random LN type --

    #[test]
    fn add_all_assigns_random_ln_type() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let tl1 = TimeLine::new(1.0, 1_000_000, 8);
        let tl2 = TimeLine::new(2.0, 2_000_000, 8);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = LongNoteModifier::with_params(4, 1.0); // AddAll
        modifier.set_seed(42);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        let note0 = tls[0].get_note(0).expect("should have LN start");
        assert!(note0.is_long());
        // LN type should be one of 1, 2, or 3
        let lntype = note0.get_long_note_type();
        assert!(
            lntype == TYPE_LONGNOTE || lntype == TYPE_CHARGENOTE || lntype == TYPE_HELLCHARGENOTE,
            "unexpected LN type: {}",
            lntype
        );
    }

    // -- Multiple lanes --

    #[test]
    fn remove_mode_handles_multiple_lanes() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        let mut ln_start0 = Note::new_long(1);
        ln_start0.set_long_note_type(TYPE_LONGNOTE);
        tl0.set_note(0, Some(ln_start0));
        let mut ln_start1 = Note::new_long(2);
        ln_start1.set_long_note_type(TYPE_LONGNOTE);
        tl0.set_note(1, Some(ln_start1));

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        let mut ln_end0 = Note::new_long(-2);
        ln_end0.set_end(true);
        tl1.set_note(0, Some(ln_end0));
        let mut ln_end1 = Note::new_long(-2);
        ln_end1.set_end(true);
        tl1.set_note(1, Some(ln_end1));

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = LongNoteModifier::with_params(0, 1.0); // Remove, rate=1.0
        modifier.set_seed(42);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // Both lanes should be converted to normal
        assert!(tls[0].get_note(0).unwrap().is_normal());
        assert!(tls[0].get_note(1).unwrap().is_normal());
        // Both LN ends should be removed
        assert!(tls[1].get_note(0).is_none());
        assert!(tls[1].get_note(1).is_none());
    }

    // -- No notes in model: no panic --

    #[test]
    fn modify_empty_model_no_panic() {
        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![]);

        let mut modifier = LongNoteModifier::with_params(0, 1.0);
        modifier.set_seed(42);
        modifier.modify(&mut model);
        // Should not panic
    }

    // -- Single timeline in add mode: no conversion (need at least 2 timelines) --

    #[test]
    fn add_mode_single_timeline_no_conversion() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0]);

        // With only 1 timeline, the loop `for i in 0..tl_len - 1` runs 0 times
        // (tl_len - 1 = 0)
        let mut modifier = LongNoteModifier::with_params(1, 1.0);
        modifier.set_seed(42);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        assert!(tls[0].get_note(0).unwrap().is_normal());
    }
}

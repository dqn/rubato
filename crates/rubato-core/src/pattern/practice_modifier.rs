use bms_model::bms_model::BMSModel;

use crate::pattern::pattern_modifier::{
    AssistLevel, PatternModifier, PatternModifierBase, move_to_background,
};

pub struct PracticeModifier {
    pub base: PatternModifierBase,
    start: i64,
    end: i64,
}

impl PracticeModifier {
    pub fn new(start: i64, end: i64) -> Self {
        PracticeModifier {
            base: PatternModifierBase::with_assist(AssistLevel::Assist),
            start,
            end,
        }
    }
}

impl PatternModifier for PracticeModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        let totalnotes = model.total_notes();
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);

        let timelines = model.all_time_lines_mut();
        let tl_len = timelines.len();
        for tl_idx in 0..tl_len {
            let time = timelines[tl_idx].time();
            for i in 0..mode_key {
                if (time as i64) < self.start || (time as i64) >= self.end {
                    move_to_background(timelines, tl_idx, i);
                }
            }
        }

        let new_total_notes = model.total_notes();
        if totalnotes > 0 {
            let total = model.total;
            model.total = total * new_total_notes as f64 / totalnotes as f64;
        }
    }

    fn assist_level(&self) -> AssistLevel {
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

    fn player(&self) -> i32 {
        self.base.player
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::pattern_modifier::{PatternModifier, make_test_model};
    use bms_model::mode::Mode;
    use bms_model::note::Note;
    use bms_model::time_line::TimeLine;

    // -- Construction --

    #[test]
    fn new_sets_start_and_end() {
        let m = PracticeModifier::new(1000, 3000);
        assert_eq!(m.start, 1000);
        assert_eq!(m.end, 3000);
    }

    #[test]
    fn new_defaults_to_assist_level() {
        let m = PracticeModifier::new(0, 0);
        assert_eq!(m.assist_level(), AssistLevel::Assist);
    }

    // -- PatternModifier trait methods --

    #[test]
    fn set_seed_positive() {
        let mut m = PracticeModifier::new(0, 0);
        m.set_seed(42);
        assert_eq!(m.get_seed(), 42);
    }

    #[test]
    fn set_seed_negative_ignored() {
        let mut m = PracticeModifier::new(0, 0);
        let original = m.get_seed();
        m.set_seed(-1);
        assert_eq!(m.get_seed(), original);
    }

    #[test]
    fn set_assist_level() {
        let mut m = PracticeModifier::new(0, 0);
        m.set_assist_level(AssistLevel::None);
        assert_eq!(m.assist_level(), AssistLevel::None);
    }

    #[test]
    fn get_player_default() {
        let m = PracticeModifier::new(0, 0);
        assert_eq!(m.player(), 0);
    }

    // -- Notes before start are moved to background --

    #[test]
    fn notes_before_start_moved_to_background() {
        // 3 timelines at time=500, 1500, 2500 (micro_time = time*1000)
        // TimeLine::get_time() returns time/1000
        let mut tl0 = TimeLine::new(0.0, 500_000, 8); // get_time() = 500
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut tl1 = TimeLine::new(1.0, 1_500_000, 8); // get_time() = 1500
        tl1.set_note(0, Some(Note::new_normal(2)));

        let mut tl2 = TimeLine::new(2.0, 2_500_000, 8); // get_time() = 2500
        tl2.set_note(0, Some(Note::new_normal(3)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = PracticeModifier::new(1000, 3000);
        modifier.modify(&mut model);

        let tls = model.timelines;
        // tl[0] (time=500) is before start=1000, note should be moved to background
        assert!(tls[0].note(0).is_none());
        assert_eq!(tls[0].back_ground_notes().len(), 1);
        assert_eq!(tls[0].back_ground_notes()[0].wav(), 1);

        // tl[1] (time=1500) is within range, note should remain
        assert!(tls[1].note(0).is_some());
        assert_eq!(tls[1].note(0).unwrap().wav(), 2);

        // tl[2] (time=2500) is within range, note should remain
        assert!(tls[2].note(0).is_some());
        assert_eq!(tls[2].note(0).unwrap().wav(), 3);
    }

    // -- Notes after end are moved to background --

    #[test]
    fn notes_after_end_moved_to_background() {
        let mut tl0 = TimeLine::new(0.0, 500_000, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut tl1 = TimeLine::new(1.0, 1_500_000, 8);
        tl1.set_note(0, Some(Note::new_normal(2)));

        let mut tl2 = TimeLine::new(2.0, 2_500_000, 8);
        tl2.set_note(0, Some(Note::new_normal(3)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = PracticeModifier::new(0, 2000);
        modifier.modify(&mut model);

        let tls = model.timelines;
        // tl[0] and tl[1] are within range, notes remain
        assert!(tls[0].note(0).is_some());
        assert!(tls[1].note(0).is_some());

        // tl[2] (time=2500) is >= end=2000, note moved to background
        assert!(tls[2].note(0).is_none());
        assert_eq!(tls[2].back_ground_notes().len(), 1);
        assert_eq!(tls[2].back_ground_notes()[0].wav(), 3);
    }

    // -- All notes within range: no changes --

    #[test]
    fn all_notes_within_range_no_changes() {
        let mut tl0 = TimeLine::new(0.0, 500_000, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut tl1 = TimeLine::new(1.0, 1_500_000, 8);
        tl1.set_note(0, Some(Note::new_normal(2)));

        let mut tl2 = TimeLine::new(2.0, 2_500_000, 8);
        tl2.set_note(0, Some(Note::new_normal(3)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = PracticeModifier::new(0, 5000);
        modifier.modify(&mut model);

        let tls = model.timelines;
        assert!(tls[0].note(0).is_some());
        assert!(tls[1].note(0).is_some());
        assert!(tls[2].note(0).is_some());
    }

    // -- start==end: all notes moved to background --

    #[test]
    fn start_equals_end_all_notes_moved() {
        let mut tl0 = TimeLine::new(0.0, 500_000, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut tl1 = TimeLine::new(1.0, 1_500_000, 8);
        tl1.set_note(0, Some(Note::new_normal(2)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl0, tl1]);

        // start==end means time < start || time >= end is always true
        let mut modifier = PracticeModifier::new(1000, 1000);
        modifier.modify(&mut model);

        let tls = model.timelines;
        assert!(tls[0].note(0).is_none());
        assert!(tls[1].note(0).is_none());
        assert_eq!(tls[0].back_ground_notes().len(), 1);
        assert_eq!(tls[1].back_ground_notes().len(), 1);
    }

    // -- Total scaling --

    #[test]
    fn total_is_scaled_proportionally() {
        // 4 notes across 4 timelines, total=300.0
        let mut tl0 = TimeLine::new(0.0, 500_000, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut tl1 = TimeLine::new(1.0, 1_500_000, 8);
        tl1.set_note(0, Some(Note::new_normal(2)));

        let mut tl2 = TimeLine::new(2.0, 2_500_000, 8);
        tl2.set_note(0, Some(Note::new_normal(3)));

        let mut tl3 = TimeLine::new(3.0, 3_500_000, 8);
        tl3.set_note(0, Some(Note::new_normal(4)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl0, tl1, tl2, tl3]);
        model.total = 300.0;

        // Remove notes outside [1000, 3000) - removes tl[0](time=500) and tl[3](time=3500)
        // 2 notes removed, 2 remain
        let mut modifier = PracticeModifier::new(1000, 3000);
        modifier.modify(&mut model);

        // new_total = 300.0 * 2/4 = 150.0
        assert!((model.total - 150.0).abs() < f64::EPSILON);
    }

    // -- Total scaling: all notes removed -> total becomes 0.0 --

    #[test]
    fn total_zero_when_all_notes_removed() {
        let mut tl0 = TimeLine::new(0.0, 500_000, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl0]);
        model.total = 200.0;

        // Range excludes all notes
        let mut modifier = PracticeModifier::new(1000, 1000);
        modifier.modify(&mut model);

        // new_total_notes = 0, so 200.0 * 0/1 = 0.0
        assert!((model.total).abs() < f64::EPSILON);
    }

    // -- Empty model: no panic --

    #[test]
    fn modify_empty_model_no_panic() {
        let mut model = make_test_model(&Mode::BEAT_7K, vec![]);

        let mut modifier = PracticeModifier::new(0, 1000);
        modifier.modify(&mut model);
        // Should not panic
    }

    // -- Multiple lanes --

    #[test]
    fn multiple_lanes_outside_range_all_moved() {
        let mut tl0 = TimeLine::new(0.0, 500_000, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));
        tl0.set_note(1, Some(Note::new_normal(2)));
        tl0.set_note(2, Some(Note::new_normal(3)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl0]);

        let mut modifier = PracticeModifier::new(1000, 2000);
        modifier.modify(&mut model);

        let tls = model.timelines;
        // All 3 notes on tl[0] (time=500) should be moved to background
        assert!(tls[0].note(0).is_none());
        assert!(tls[0].note(1).is_none());
        assert!(tls[0].note(2).is_none());
        assert_eq!(tls[0].back_ground_notes().len(), 3);
    }

    // -- Boundary: time exactly at start is included --

    #[test]
    fn note_at_exact_start_time_is_included() {
        // time=1000 exactly, start=1000 -> time < start is false, time >= end with end=2000 is false
        let mut tl0 = TimeLine::new(0.0, 1_000_000, 8); // get_time() = 1000
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl0]);

        let mut modifier = PracticeModifier::new(1000, 2000);
        modifier.modify(&mut model);

        let tls = model.timelines;
        // Note at exactly start should remain
        assert!(tls[0].note(0).is_some());
    }

    // -- Boundary: time exactly at end is excluded --

    #[test]
    fn note_at_exact_end_time_is_excluded() {
        // time=2000 exactly, end=2000 -> time >= end is true, so it gets moved
        let mut tl0 = TimeLine::new(0.0, 2_000_000, 8); // get_time() = 2000
        tl0.set_note(0, Some(Note::new_normal(1)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl0]);

        let mut modifier = PracticeModifier::new(1000, 2000);
        modifier.modify(&mut model);

        let tls = model.timelines;
        // Note at exactly end should be moved to background
        assert!(tls[0].note(0).is_none());
        assert_eq!(tls[0].back_ground_notes().len(), 1);
    }
}

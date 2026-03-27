use bms::model::bms_model::BMSModel;

use crate::core::pattern::pattern_modifier::{
    AssistLevel, PatternModifier, PatternModifierBase, move_to_background,
};

pub struct AutoplayModifier {
    pub base: PatternModifierBase,
    lanes: Vec<i32>,
    margin: i32,
}

impl AutoplayModifier {
    pub fn new(lanes: Vec<i32>) -> Self {
        Self::with_margin(lanes, 0)
    }

    pub fn with_margin(lanes: Vec<i32>, margin: i32) -> Self {
        AutoplayModifier {
            base: PatternModifierBase::new(),
            lanes,
            margin,
        }
    }
}

impl PatternModifier for AutoplayModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        let mut assist = AssistLevel::None;
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0) as usize;

        let timelines = &mut model.timelines;
        let tl_len = timelines.len();
        let mut pos = 0usize;
        let mut lns = vec![false; mode_key];

        for i in 0..tl_len {
            let mut remove = false;

            if self.margin > 0 {
                while pos < tl_len
                    && timelines[pos].time() < timelines[i].time() - self.margin as i64
                {
                    for (lane, ln_active) in lns.iter_mut().enumerate() {
                        if let Some(note) = timelines[pos].note(lane as i32)
                            && note.is_long()
                        {
                            *ln_active = !note.is_end();
                        }
                    }
                    pos += 1;
                }
                let mut endtime = timelines[i].time() + self.margin as i64;
                for &lane in &self.lanes {
                    if let Some(note) = timelines[i].note(lane)
                        && note.is_long()
                        && !note.is_end()
                    {
                        endtime = endtime.max(note.time() + self.margin as i64);
                    }
                }

                for tl in &timelines[pos..tl_len] {
                    if tl.time() >= endtime {
                        break;
                    }
                    for (lane, &ln_active) in lns.iter().enumerate() {
                        let mut b = true;
                        for &rlane in &self.lanes {
                            if lane as i32 == rlane {
                                b = false;
                                break;
                            }
                        }
                        if b && (tl.note(lane as i32).is_some() || ln_active) {
                            remove = true;
                            break;
                        }
                    }
                    if remove {
                        break;
                    }
                }
            } else {
                remove = true;
            }

            if remove {
                for &lane in &self.lanes {
                    if timelines[i].exist_note_at(lane) {
                        assist = AssistLevel::Assist;
                    }
                    move_to_background(timelines, i, lane);
                }
            }
        }
        self.base.assist = assist;
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
    use crate::core::pattern::pattern_modifier::make_test_model;
    use bms::model::mode::Mode;
    use bms::model::note::Note;
    use bms::model::time_line::TimeLine;

    #[test]
    fn autoplay_modifier_creation() {
        let modifier = AutoplayModifier::new(vec![0, 1]);
        assert_eq!(modifier.assist_level(), AssistLevel::None);
        assert_eq!(modifier.player(), 0);
    }

    #[test]
    fn autoplay_modifier_with_margin() {
        let modifier = AutoplayModifier::with_margin(vec![0], 100);
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn autoplay_modifier_set_seed_positive() {
        let mut modifier = AutoplayModifier::new(vec![0]);
        modifier.set_seed(42);
        assert_eq!(modifier.get_seed(), 42);
    }

    #[test]
    fn autoplay_modifier_set_seed_negative_ignored() {
        let mut modifier = AutoplayModifier::new(vec![0]);
        let original = modifier.get_seed();
        modifier.set_seed(-1);
        assert_eq!(modifier.get_seed(), original);
    }

    #[test]
    fn autoplay_modifier_set_assist_level() {
        let mut modifier = AutoplayModifier::new(vec![0]);
        modifier.set_assist_level(AssistLevel::LightAssist);
        assert_eq!(modifier.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn autoplay_moves_note_to_background_no_margin() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(10)));
        tl.set_note(1, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl]);

        // Autoplay lane 0 (no margin)
        let mut modifier = AutoplayModifier::new(vec![0]);
        modifier.modify(&mut model);

        let tls = model.timelines;
        // Lane 0 note should be moved to background
        assert!(tls[0].note(0).is_none());
        // Lane 1 note should also be moved (margin=0 means remove=true for all)
        // Wait, re-read: margin=0 means else branch: remove = true
        // So ALL specified lanes are moved to background
        // Lane 1 is NOT in the lanes list, so it stays
        assert!(tls[0].note(1).is_some());
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn autoplay_sets_assist_when_notes_exist() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(10)));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = AutoplayModifier::new(vec![0]);
        modifier.modify(&mut model);

        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn autoplay_no_notes_keeps_none_assist() {
        let mode = Mode::BEAT_7K;
        let tl = TimeLine::new(0.0, 0, 8);

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = AutoplayModifier::new(vec![0]);
        modifier.modify(&mut model);

        // No notes exist in lane 0, so assist stays None
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn autoplay_multiple_lanes() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(10)));
        tl.set_note(1, Some(Note::new_normal(20)));
        tl.set_note(2, Some(Note::new_normal(30)));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = AutoplayModifier::new(vec![0, 1]);
        modifier.modify(&mut model);

        let tls = model.timelines;
        assert!(tls[0].note(0).is_none());
        assert!(tls[0].note(1).is_none());
        // Lane 2 is not in autoplay lanes
        assert!(tls[0].note(2).is_some());
    }

    #[test]
    fn autoplay_mine_note_removed_not_backgrounded() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_mine(-1, 10.0)));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = AutoplayModifier::new(vec![0]);
        modifier.modify(&mut model);

        let tls = model.timelines;
        assert!(tls[0].note(0).is_none());
        // Mine notes are removed entirely, not added to background
        assert!(tls[0].back_ground_notes().is_empty());
    }

    // --- margin > 0 tests ---

    #[test]
    fn margin_no_conflict_keeps_note_playable() {
        // Autoplay lane 0 with margin=100. Only lane 0 has notes, no
        // non-autoplay lane notes within the margin window, so no conflict
        // and the note should NOT be moved to background.
        let mode = Mode::BEAT_7K;
        // Two timelines far apart: time()=0 and time()=500000
        // (raw times; time() divides by 1000, so time()=0 and time()=500)
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(10)));

        let mut tl1 = TimeLine::new(1.0, 500_000, 8);
        tl1.set_note(0, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl0, tl1]);

        // margin=100 means window is [time-100, time+100].
        // No non-autoplay lane notes exist at all, so no conflict.
        let mut modifier = AutoplayModifier::with_margin(vec![0], 100);
        modifier.modify(&mut model);

        // Notes should remain in their lanes (not moved to background)
        assert!(model.timelines[0].note(0).is_some());
        assert!(model.timelines[1].note(0).is_some());
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn margin_conflict_moves_note_to_background() {
        // Autoplay lane 0 with margin=200. Lane 1 (non-autoplay) has a note
        // at time()=50 which is within margin window of lane 0's note at
        // time()=0 (endtime=0+200=200, and 50 < 200), so conflict detected.
        let mode = Mode::BEAT_7K;
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(10))); // autoplay lane

        let mut tl1 = TimeLine::new(0.0, 50_000, 8);
        tl1.set_note(1, Some(Note::new_normal(20))); // non-autoplay lane, within margin

        let mut model = make_test_model(&mode, vec![tl0, tl1]);

        let mut modifier = AutoplayModifier::with_margin(vec![0], 200);
        modifier.modify(&mut model);

        // Lane 0 note at tl0 should be moved to background (conflict detected)
        assert!(model.timelines[0].note(0).is_none());
        assert!(!model.timelines[0].back_ground_notes().is_empty());
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn margin_conflict_outside_window_keeps_note() {
        // Autoplay lane 0 with margin=50. Lane 1 has a note at time()=200,
        // which is outside the margin window of lane 0's note at time()=0
        // (endtime=0+50=50, and 200 >= 50), so no conflict.
        let mode = Mode::BEAT_7K;
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(10)));

        let mut tl1 = TimeLine::new(1.0, 200_000, 8);
        tl1.set_note(1, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl0, tl1]);

        let mut modifier = AutoplayModifier::with_margin(vec![0], 50);
        modifier.modify(&mut model);

        // No conflict: lane 0 note stays
        assert!(model.timelines[0].note(0).is_some());
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn margin_exact_boundary_no_conflict() {
        // Note in non-autoplay lane at exactly endtime should NOT trigger
        // conflict because the scan breaks on `tl.time() >= endtime`.
        let mode = Mode::BEAT_7K;
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(10)));

        // endtime = 0 + 100 = 100. Non-autoplay note at time()=100 exactly.
        let mut tl1 = TimeLine::new(1.0, 100_000, 8);
        tl1.set_note(1, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl0, tl1]);

        let mut modifier = AutoplayModifier::with_margin(vec![0], 100);
        modifier.modify(&mut model);

        // time()=100 >= endtime=100, so the scan breaks before checking.
        // No conflict; note stays.
        assert!(model.timelines[0].note(0).is_some());
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn margin_just_inside_boundary_triggers_conflict() {
        // Non-autoplay note at time()=99, endtime=100. 99 < 100 so conflict.
        let mode = Mode::BEAT_7K;
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(10)));

        let mut tl1 = TimeLine::new(1.0, 99_000, 8);
        tl1.set_note(1, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl0, tl1]);

        let mut modifier = AutoplayModifier::with_margin(vec![0], 100);
        modifier.modify(&mut model);

        // 99 < 100, conflict detected; lane 0 note moved to background
        assert!(model.timelines[0].note(0).is_none());
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn margin_multi_lane_conflict_detection() {
        // Autoplay lanes [0, 1]. Non-autoplay lane 2 has a note within
        // margin of lane 0's note. Both autoplay lane notes at the same
        // timeline should be moved to background.
        let mode = Mode::BEAT_7K;
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(10)));
        tl0.set_note(1, Some(Note::new_normal(20)));

        let mut tl1 = TimeLine::new(0.0, 50_000, 8);
        tl1.set_note(2, Some(Note::new_normal(30))); // non-autoplay, within margin

        let mut model = make_test_model(&mode, vec![tl0, tl1]);

        let mut modifier = AutoplayModifier::with_margin(vec![0, 1], 200);
        modifier.modify(&mut model);

        // Both autoplay lanes should be moved to background
        assert!(model.timelines[0].note(0).is_none());
        assert!(model.timelines[0].note(1).is_none());
        // Lane 2 in tl1 is not an autoplay lane; stays
        assert!(model.timelines[1].note(2).is_some());
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn margin_only_autoplay_lane_notes_in_window_no_conflict() {
        // Two autoplay lanes with notes in same window. Non-autoplay lanes
        // are empty. Even though notes from autoplay lanes are within
        // the margin, there's no non-autoplay conflict.
        let mode = Mode::BEAT_7K;
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(10)));

        let mut tl1 = TimeLine::new(0.0, 50_000, 8);
        tl1.set_note(0, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl0, tl1]);

        let mut modifier = AutoplayModifier::with_margin(vec![0], 200);
        modifier.modify(&mut model);

        // No non-autoplay notes anywhere, so no conflicts
        assert!(model.timelines[0].note(0).is_some());
        assert!(model.timelines[1].note(0).is_some());
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn margin_ln_active_in_non_autoplay_lane_triggers_conflict() {
        // An LN starts in non-autoplay lane 1 at tl0 (time=0) and ends
        // at tl2 (time=500). An autoplay note appears at tl1 (time=300).
        // The LN is active during tl1's window, so conflict is detected
        // via the lns[] active state.
        let mode = Mode::BEAT_7K;

        // tl0: LN start in lane 1 (non-autoplay)
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        let mut ln_start = Note::new_long(10);
        ln_start.set_pair_index(Some(2)); // pairs with tl2
        tl0.set_note(1, Some(ln_start));

        // tl1: autoplay note in lane 0 at time()=300
        // margin=100, so window scans [200, 400].
        // pos advances past tl0 (time()=0 < 300-100=200), recording lns[1]=true.
        // Then scan from pos to endtime checks if lns[1] is active -> conflict.
        let mut tl1 = TimeLine::new(0.0, 300_000, 8);
        tl1.set_note(0, Some(Note::new_normal(20)));

        // tl2: LN end in lane 1 (non-autoplay)
        let mut tl2 = TimeLine::new(1.0, 500_000, 8);
        let mut ln_end = Note::new_long(10);
        ln_end.set_end(true);
        ln_end.set_pair_index(Some(0)); // pairs back with tl0
        tl2.set_note(1, Some(ln_end));

        let mut model = make_test_model(&mode, vec![tl0, tl1, tl2]);

        let mut modifier = AutoplayModifier::with_margin(vec![0], 100);
        modifier.modify(&mut model);

        // Lane 0 note at tl1 should be moved to background due to
        // active LN in lane 1
        assert!(model.timelines[1].note(0).is_none());
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn margin_ln_ended_before_window_no_conflict() {
        // An LN in non-autoplay lane 1 starts at tl0 (time=0) and ends at
        // tl1 (time=100). An autoplay note appears at tl2 (time=500).
        // The LN ended well before the window, so lns[1] should be false
        // and no conflict.
        let mode = Mode::BEAT_7K;

        // tl0: LN start in lane 1
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        let mut ln_start = Note::new_long(10);
        ln_start.set_pair_index(Some(1));
        tl0.set_note(1, Some(ln_start));

        // tl1: LN end in lane 1
        let mut tl1 = TimeLine::new(0.0, 100_000, 8);
        let mut ln_end = Note::new_long(10);
        ln_end.set_end(true);
        ln_end.set_pair_index(Some(0));
        tl1.set_note(1, Some(ln_end));

        // tl2: autoplay note in lane 0, far from the LN
        let mut tl2 = TimeLine::new(1.0, 500_000, 8);
        tl2.set_note(0, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl0, tl1, tl2]);

        let mut modifier = AutoplayModifier::with_margin(vec![0], 100);
        modifier.modify(&mut model);

        // LN ended before the window; no conflict; lane 0 note stays
        assert!(model.timelines[2].note(0).is_some());
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn margin_multiple_timelines_mixed_conflicts() {
        // Multiple autoplay notes: one has a conflict, one does not.
        // Only the conflicting one should be moved.
        let mode = Mode::BEAT_7K;

        // tl0: autoplay note in lane 0 at time()=0
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_note(0, Some(Note::new_normal(10)));

        // tl1: non-autoplay note in lane 1 at time()=50 (within margin of tl0)
        let mut tl1 = TimeLine::new(0.0, 50_000, 8);
        tl1.set_note(1, Some(Note::new_normal(20)));

        // tl2: autoplay note in lane 0 at time()=1000 (far from any non-autoplay notes)
        let mut tl2 = TimeLine::new(1.0, 1_000_000, 8);
        tl2.set_note(0, Some(Note::new_normal(30)));

        let mut model = make_test_model(&mode, vec![tl0, tl1, tl2]);

        let mut modifier = AutoplayModifier::with_margin(vec![0], 100);
        modifier.modify(&mut model);

        // tl0 has conflict (tl1's lane 1 note within window) -> moved
        assert!(model.timelines[0].note(0).is_none());
        // tl2 has no conflict -> stays
        assert!(model.timelines[2].note(0).is_some());
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }
}

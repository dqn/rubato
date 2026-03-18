use bms_model::bms_model::BMSModel;

use crate::pattern::pattern_modifier::{
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
                while timelines[pos].time() < timelines[i].time() - self.margin as i64 {
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
                        endtime = endtime.max(note.time() as i64 + self.margin as i64);
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
    use crate::pattern::pattern_modifier::make_test_model;
    use bms_model::mode::Mode;
    use bms_model::note::Note;
    use bms_model::time_line::TimeLine;

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
}

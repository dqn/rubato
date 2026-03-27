use bms::model::bms_model::BMSModel;
use bms::model::note::Note;

use crate::core::pattern::pattern_modifier::{AssistLevel, PatternModifier, PatternModifierBase};

pub struct ExtraNoteModifier {
    pub base: PatternModifierBase,
    _note_type: i32,
    depth: i32,
    scratch: bool,
}

impl ExtraNoteModifier {
    pub fn new(note_type: i32, depth: i32, scratch: bool) -> Self {
        ExtraNoteModifier {
            base: PatternModifierBase::new(),
            _note_type: note_type,
            depth,
            scratch,
        }
    }
}

impl PatternModifier for ExtraNoteModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        let mut assist = AssistLevel::None;
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);
        if mode_key == 0 {
            return;
        }
        let scratch = self.scratch;

        let mode = model.mode().copied();
        let timelines = &mut model.timelines;
        let mut lns = vec![false; mode_key as usize];
        let mut blank = vec![false; mode_key as usize];
        let mut lastnote: Vec<Option<Note>> = vec![None; mode_key as usize];
        let mut lastoffset = 0usize;

        for tl in timelines.iter_mut() {
            for key in 0..mode_key as usize {
                let note = tl.note(key as i32);
                if let Some(n) = note
                    && n.is_long()
                {
                    lns[key] = !n.is_end();
                }
                let is_scratch = mode
                    .as_ref()
                    .map(|m| m.is_scratch_key(key as i32))
                    .unwrap_or(false);
                blank[key] = !lns[key] && tl.note(key as i32).is_none() && (scratch || !is_scratch);
            }

            for _d in 0..self.depth {
                if !tl.back_ground_notes().is_empty() {
                    let note = tl.back_ground_notes()[0].clone();

                    let mut offset = lastoffset;
                    for _j in 1..mode_key as usize {
                        if let Some(ref ln) = lastnote[offset]
                            && ln.wav() == note.wav()
                        {
                            break;
                        }
                        offset = (offset + 1) % mode_key as usize;
                    }
                    lastoffset = offset;

                    let mut placed = false;
                    let mut key = offset % mode_key as usize;
                    for _j in 0..mode_key as usize {
                        if blank[key] {
                            lastnote[key] = Some(note.clone());
                            tl.set_note(key as i32, Some(note.clone()));
                            tl.remove_back_ground_note(0);
                            assist = AssistLevel::Assist;
                            placed = true;
                            break;
                        }
                        key = (key + 1) % mode_key as usize;
                    }
                    if !placed {
                        break;
                    }
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
    use bms::model::time_line::TimeLine;

    #[test]
    fn extra_note_modifier_creation() {
        let modifier = ExtraNoteModifier::new(0, 1, false);
        assert_eq!(modifier.assist_level(), AssistLevel::None);
        assert_eq!(modifier.player(), 0);
    }

    #[test]
    fn extra_note_modifier_set_seed() {
        let mut modifier = ExtraNoteModifier::new(0, 1, false);
        modifier.set_seed(42);
        assert_eq!(modifier.get_seed(), 42);
    }

    #[test]
    fn extra_note_modifier_set_seed_negative_ignored() {
        let mut modifier = ExtraNoteModifier::new(0, 1, false);
        let original = modifier.get_seed();
        modifier.set_seed(-1);
        assert_eq!(modifier.get_seed(), original);
    }

    #[test]
    fn extra_note_modifier_set_assist_level() {
        let mut modifier = ExtraNoteModifier::new(0, 1, false);
        modifier.set_assist_level(AssistLevel::Assist);
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn extra_note_modifier_no_background_notes_is_noop() {
        let mode = Mode::BEAT_7K;
        let tl = TimeLine::new(0.0, 0, 8);
        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = ExtraNoteModifier::new(0, 1, false);
        modifier.modify(&mut model);

        // No background notes -> no extra notes placed
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn extra_note_modifier_places_background_note() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        // Add a background note
        tl.add_back_ground_note(Note::new_normal(5));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = ExtraNoteModifier::new(0, 1, false);
        modifier.modify(&mut model);

        // Background note should be placed somewhere on the lanes
        let tls = model.timelines;
        let mut found = false;
        for lane in 0..8 {
            if let Some(note) = tls[0].note(lane)
                && note.wav() == 5
            {
                found = true;
                break;
            }
        }
        assert!(found, "Background note should be placed on a lane");
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn extra_note_modifier_depth_limits_placement() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        // Add multiple background notes
        tl.add_back_ground_note(Note::new_normal(1));
        tl.add_back_ground_note(Note::new_normal(2));
        tl.add_back_ground_note(Note::new_normal(3));

        let mut model = make_test_model(&mode, vec![tl]);

        // depth=1 should place at most 1 note per timeline
        let mut modifier = ExtraNoteModifier::new(0, 1, false);
        modifier.modify(&mut model);

        let tls = model.timelines;
        let mut placed_count = 0;
        for lane in 0..8 {
            if tls[0].note(lane).is_some() {
                placed_count += 1;
            }
        }
        assert_eq!(placed_count, 1, "depth=1 should place exactly 1 note");
    }

    // -- Bounds safety regression tests --

    #[test]
    fn extra_note_modifier_no_mode_no_panic() {
        // When model has no mode (mode_key == 0), modify must not panic
        // from modulo-by-zero in `% mode_key as usize`.
        let mut model = bms::model::bms_model::BMSModel::new();
        // model.mode() returns None -> mode_key = 0
        let mut tl = TimeLine::new(0.0, 0, 0);
        tl.add_back_ground_note(bms::model::note::Note::new_normal(1));
        model.timelines = vec![tl];

        let mut modifier = ExtraNoteModifier::new(0, 1, false);
        // Before fix: panics with division by zero (% 0).
        // After fix: early return when mode_key == 0.
        modifier.modify(&mut model);

        // Should remain None assist since nothing was placed
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn extra_note_modifier_depth_2() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.add_back_ground_note(Note::new_normal(1));
        tl.add_back_ground_note(Note::new_normal(2));
        tl.add_back_ground_note(Note::new_normal(3));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = ExtraNoteModifier::new(0, 2, false);
        modifier.modify(&mut model);

        let tls = model.timelines;
        let mut placed_count = 0;
        for lane in 0..8 {
            if tls[0].note(lane).is_some() {
                placed_count += 1;
            }
        }
        assert_eq!(placed_count, 2, "depth=2 should place exactly 2 notes");
    }
}

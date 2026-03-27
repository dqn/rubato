use bms::model::bms_model::BMSModel;
use bms::model::note::Note;

use crate::core::pattern::pattern_modifier::{AssistLevel, PatternModifier, PatternModifierBase};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Remove,
    AddRandom,
    AddNear,
    AddBlank,
}

impl Mode {
    pub fn values() -> &'static [Mode] {
        &[Mode::Remove, Mode::AddRandom, Mode::AddNear, Mode::AddBlank]
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

pub struct MineNoteModifier {
    pub base: PatternModifierBase,
    exists: bool,
    mode: Mode,
    damage: i32,
}

impl Default for MineNoteModifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MineNoteModifier {
    pub fn new() -> Self {
        MineNoteModifier {
            base: PatternModifierBase::new(),
            exists: false,
            mode: Mode::Remove,
            damage: 10,
        }
    }

    pub fn with_mode(mode: i32) -> Self {
        MineNoteModifier {
            base: PatternModifierBase::new(),
            exists: false,
            mode: Mode::from_index(mode),
            damage: 10,
        }
    }

    pub fn mine_note_exists(&self) -> bool {
        self.exists
    }
}

impl PatternModifier for MineNoteModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);

        if self.mode == Mode::Remove {
            let mut assist = AssistLevel::None;
            let timelines = &mut model.timelines;
            for tl in timelines.iter_mut() {
                for lane in 0..mode_key {
                    if let Some(note) = tl.note(lane)
                        && note.is_mine()
                    {
                        assist = AssistLevel::LightAssist;
                        self.exists = true;
                        tl.set_note(lane, None);
                    }
                }
            }
            self.base.assist = assist;
        } else {
            let timelines = &mut model.timelines;
            let mut ln = vec![false; mode_key as usize];
            let mut blank = vec![false; mode_key as usize];

            for tl in timelines.iter_mut() {
                for key in 0..mode_key as usize {
                    let note = tl.note(key as i32);
                    if let Some(n) = note
                        && n.is_long()
                    {
                        ln[key] = !n.is_end();
                    }
                    blank[key] = !ln[key] && tl.note(key as i32).is_none();
                }

                for key in 0..mode_key as usize {
                    if blank[key] {
                        match self.mode {
                            Mode::AddRandom => {
                                // Java parity: Math.random() (not seeded JavaRandom) -- same as long_note_modifier.rs
                                if rand::random::<f64>() > 0.9 {
                                    tl.set_note(
                                        key as i32,
                                        Some(Note::new_mine(-1, self.damage as f64)),
                                    );
                                }
                            }
                            Mode::AddNear => {
                                if (key > 0 && !blank[key - 1])
                                    || (key < mode_key as usize - 1 && !blank[key + 1])
                                {
                                    tl.set_note(
                                        key as i32,
                                        Some(Note::new_mine(-1, self.damage as f64)),
                                    );
                                }
                            }
                            Mode::AddBlank => {
                                tl.set_note(
                                    key as i32,
                                    Some(Note::new_mine(-1, self.damage as f64)),
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
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
    use crate::core::pattern::pattern_modifier::{PatternModifier, make_test_model};
    use bms::model::time_line::TimeLine;

    // -- Mode enum (mine_note_modifier::Mode) --

    #[test]
    fn mode_values_has_4_elements() {
        assert_eq!(Mode::values().len(), 4);
    }

    #[test]
    fn mode_from_index_valid() {
        assert_eq!(Mode::from_index(0), Mode::Remove);
        assert_eq!(Mode::from_index(1), Mode::AddRandom);
        assert_eq!(Mode::from_index(2), Mode::AddNear);
        assert_eq!(Mode::from_index(3), Mode::AddBlank);
    }

    #[test]
    fn mode_from_index_negative_returns_remove() {
        assert_eq!(Mode::from_index(-1), Mode::Remove);
    }

    #[test]
    fn mode_from_index_out_of_range_returns_remove() {
        assert_eq!(Mode::from_index(4), Mode::Remove);
        assert_eq!(Mode::from_index(100), Mode::Remove);
    }

    // -- MineNoteModifier creation --

    #[test]
    fn mine_note_modifier_default() {
        let modifier = MineNoteModifier::new();
        assert_eq!(modifier.assist_level(), AssistLevel::None);
        assert!(!modifier.mine_note_exists());
    }

    #[test]
    fn mine_note_modifier_with_mode() {
        let modifier = MineNoteModifier::with_mode(1);
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn mine_note_modifier_set_seed() {
        let mut modifier = MineNoteModifier::new();
        modifier.set_seed(42);
        assert_eq!(modifier.get_seed(), 42);
    }

    #[test]
    fn mine_note_modifier_set_seed_negative_ignored() {
        let mut modifier = MineNoteModifier::new();
        let original = modifier.get_seed();
        modifier.set_seed(-1);
        assert_eq!(modifier.get_seed(), original);
    }

    // -- Remove mode --

    #[test]
    fn remove_mode_removes_mine_notes() {
        let mode = bms::model::mode::Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_mine(-1, 10.0)));
        tl.set_note(1, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = MineNoteModifier::new(); // default is Remove
        modifier.modify(&mut model);

        let tls = model.timelines;
        // Mine note in lane 0 should be removed
        assert!(tls[0].note(0).is_none());
        // Normal note in lane 1 should remain
        assert!(tls[0].note(1).is_some());
        assert_eq!(modifier.assist_level(), AssistLevel::LightAssist);
        assert!(modifier.mine_note_exists());
    }

    #[test]
    fn remove_mode_no_mines_keeps_none_assist() {
        let mode = bms::model::mode::Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(10)));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = MineNoteModifier::new();
        modifier.modify(&mut model);

        assert_eq!(modifier.assist_level(), AssistLevel::None);
        assert!(!modifier.mine_note_exists());
    }

    // -- AddBlank mode --

    #[test]
    fn add_blank_mode_fills_empty_lanes_with_mines() {
        let mode = bms::model::mode::Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        // Only lane 0 has a note
        tl.set_note(0, Some(Note::new_normal(10)));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = MineNoteModifier::with_mode(3); // AddBlank
        modifier.modify(&mut model);

        let tls = model.timelines;
        // Lane 0 should still have the normal note
        assert!(tls[0].note(0).unwrap().is_normal());
        // All other lanes should have mine notes
        for lane in 1..8 {
            let note = tls[0].note(lane);
            assert!(note.is_some(), "Lane {} should have a mine note", lane);
            assert!(
                note.unwrap().is_mine(),
                "Lane {} should be a mine note",
                lane
            );
        }
    }

    // -- AddNear mode --

    #[test]
    fn add_near_mode_adds_mines_adjacent_to_notes() {
        let mode = bms::model::mode::Mode::BEAT_7K;
        let mut tl = TimeLine::new(0.0, 0, 8);
        // Place note in lane 3 (middle)
        tl.set_note(3, Some(Note::new_normal(10)));

        let mut model = make_test_model(&mode, vec![tl]);

        let mut modifier = MineNoteModifier::with_mode(2); // AddNear
        modifier.modify(&mut model);

        let tls = model.timelines;
        // Lane 3 should still have normal note
        assert!(tls[0].note(3).unwrap().is_normal());
        // Lane 2 (adjacent) should have a mine
        assert!(tls[0].note(2).unwrap().is_mine());
        // Lane 4 (adjacent) should have a mine
        assert!(tls[0].note(4).unwrap().is_mine());
    }

    // -- Default trait --

    #[test]
    fn mine_note_modifier_default_trait() {
        let modifier = MineNoteModifier::default();
        assert!(!modifier.mine_note_exists());
    }
}

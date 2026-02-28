use bms_model::bms_model::BMSModel;
use bms_model::note::{Note, TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_LONGNOTE, TYPE_UNDEFINED};

use crate::java_random::JavaRandom;
use crate::pattern_modifier::{AssistLevel, PatternModifier, PatternModifierBase};

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

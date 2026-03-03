use beatoraja_core::player_config::PlayerConfig;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;

use crate::pattern_modifier::{AssistLevel, PatternModifier, PatternModifierBase};

pub struct ModeModifier {
    pub base: PatternModifierBase,
    config: PlayerConfig,
    hran_threshold: i32,
    before_mode: Mode,
    after_mode: Mode,
}

impl ModeModifier {
    pub fn new(before_mode: Mode, after_mode: Mode, config: PlayerConfig) -> Self {
        ModeModifier {
            base: PatternModifierBase::with_assist(AssistLevel::LightAssist),
            config,
            hran_threshold: 125,
            before_mode,
            after_mode,
        }
    }
}

impl PatternModifier for ModeModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        model.set_mode(self.after_mode.clone());
        let algorithm = Algorithm::get(&self.before_mode, &self.after_mode);
        let lanes = self.after_mode.key() as usize;
        let mut ln = vec![-1i32; lanes];
        let mut last_note_time = vec![-100i32; lanes];
        let mut end_ln_note_time = vec![-1i32; lanes];

        if self.config.hran_threshold_bpm <= 0 {
            self.hran_threshold = 0;
        } else {
            self.hran_threshold =
                (15000.0f32 / self.config.hran_threshold_bpm as f32).ceil() as i32;
        }

        let after_mode = self.after_mode.clone();
        let hran_threshold = self.hran_threshold;
        let seven_to_nine_pattern = self.config.seven_to_nine_pattern;
        let seven_to_nine_type = self.config.seven_to_nine_type;

        let timelines = model.get_all_time_lines_mut();
        // Pre-compute timeline index → time for LN end note pair lookup
        let tl_times: Vec<i32> = timelines.iter().map(|tl| tl.get_time()).collect();
        for tl in timelines.iter_mut() {
            if tl.exist_note() || tl.exist_hidden_note() {
                let mut notes: Vec<Option<Note>> = Vec::with_capacity(lanes);
                let mut hnotes: Vec<Option<Note>> = Vec::with_capacity(lanes);
                for i in 0..lanes {
                    notes.push(tl.get_note(i as i32).cloned());
                    hnotes.push(tl.get_hidden_note(i as i32).cloned());
                }

                let keys = PatternModifierBase::get_keys_static(&after_mode, 0, true);
                let random = if let Some(alg) = algorithm {
                    if !keys.is_empty() {
                        alg.modify(
                            &keys,
                            &ln,
                            &notes,
                            &last_note_time,
                            tl.get_time(),
                            hran_threshold,
                            seven_to_nine_pattern,
                            seven_to_nine_type,
                        )
                    } else {
                        keys
                    }
                } else {
                    keys
                };

                for i in 0..lanes {
                    let m = if i < random.len() {
                        random[i] as usize
                    } else {
                        i
                    };
                    let n = notes[m].take();
                    let hn = hnotes[m].take();
                    if let Some(ref note) = n {
                        let is_long = note.is_long();
                        let is_end = note.is_end();
                        let _note_time = note.get_time();
                        if is_long {
                            if is_end && tl.get_time() == end_ln_note_time[i] {
                                tl.set_note(i as i32, n);
                                ln[i] = -1;
                                end_ln_note_time[i] = -1;
                            } else {
                                ln[i] = m as i32;
                                if !is_end {
                                    // Java: endLnNoteTime[i] = ln2.getPair().getTime()
                                    // Store the END note's timeline time (not the start note's)
                                    end_ln_note_time[i] =
                                        note.get_pair().map(|idx| tl_times[idx]).unwrap_or(-1);
                                }
                                last_note_time[i] = tl.get_time();
                                tl.set_note(i as i32, n);
                            }
                        } else {
                            last_note_time[i] = tl.get_time();
                            tl.set_note(i as i32, n);
                        }
                    } else {
                        tl.set_note(i as i32, None);
                    }
                    tl.set_hidden_note(i as i32, hn);
                }
            }
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

// get_keys_static is defined in lane_shuffle_modifier.rs

#[derive(Clone, Copy)]
enum Algorithm {
    SevenToNine,
}

impl Algorithm {
    fn get(before_mode: &Mode, after_mode: &Mode) -> Option<Algorithm> {
        if *before_mode == Mode::BEAT_7K && *after_mode == Mode::POPN_9K {
            Some(Algorithm::SevenToNine)
        } else {
            None
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn modify(
        &self,
        _keys: &[i32],
        activeln: &[i32],
        _notes: &[Option<Note>],
        last_note_time: &[i32],
        now: i32,
        duration: i32,
        seven_to_nine_pattern: i32,
        seven_to_nine_type: i32,
    ) -> Vec<i32> {
        match self {
            Algorithm::SevenToNine => {
                #[allow(clippy::eq_op)]
                let (key_lane, sc_lane, rest_lane) = match seven_to_nine_pattern {
                    1 => (2 - 1, 1 - 1, 9 - 1),
                    2 => (3 - 1, 1 - 1, 2 - 1),
                    4 => (1 - 1, 8 - 1, 9 - 1),
                    5 => (1 - 1, 9 - 1, 8 - 1),
                    6 => (2 - 1, 9 - 1, 1 - 1),
                    3 => (3 - 1, 2 - 1, 1 - 1),
                    _ => (3 - 1, 2 - 1, 1 - 1),
                };

                let mut result = vec![0i32; 9];
                for i in 0..7 {
                    result[i + key_lane as usize] = i as i32;
                }

                if activeln[sc_lane as usize] != -1 || activeln[rest_lane as usize] != -1 {
                    if activeln[sc_lane as usize] == 7 {
                        result[sc_lane as usize] = 7;
                        result[rest_lane as usize] = 8;
                    } else {
                        result[sc_lane as usize] = 8;
                        result[rest_lane as usize] = 7;
                    }
                } else {
                    match seven_to_nine_type {
                        1 => {
                            if now - last_note_time[sc_lane as usize] > duration
                                || now - last_note_time[sc_lane as usize]
                                    >= now - last_note_time[rest_lane as usize]
                            {
                                result[sc_lane as usize] = 7;
                                result[rest_lane as usize] = 8;
                            } else {
                                result[sc_lane as usize] = 8;
                                result[rest_lane as usize] = 7;
                            }
                        }
                        2 => {
                            if now - last_note_time[sc_lane as usize]
                                >= now - last_note_time[rest_lane as usize]
                            {
                                result[sc_lane as usize] = 7;
                                result[rest_lane as usize] = 8;
                            } else {
                                result[sc_lane as usize] = 8;
                                result[rest_lane as usize] = 7;
                            }
                        }
                        0 => {
                            result[sc_lane as usize] = 7;
                            result[rest_lane as usize] = 8;
                        }
                        _ => {
                            result[sc_lane as usize] = 7;
                            result[rest_lane as usize] = 8;
                        }
                    }
                }

                result
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_modifier::{PatternModifier, make_test_model};
    use bms_model::mode::Mode;
    use bms_model::note::Note;
    use bms_model::time_line::TimeLine;

    // -- Algorithm::get dispatch tests --

    #[test]
    fn algorithm_get_7k_to_9k_returns_seven_to_nine() {
        let alg = Algorithm::get(&Mode::BEAT_7K, &Mode::POPN_9K);
        assert!(alg.is_some());
    }

    #[test]
    fn algorithm_get_same_mode_returns_none() {
        assert!(Algorithm::get(&Mode::BEAT_7K, &Mode::BEAT_7K).is_none());
        assert!(Algorithm::get(&Mode::POPN_9K, &Mode::POPN_9K).is_none());
    }

    #[test]
    fn algorithm_get_9k_to_7k_returns_none() {
        assert!(Algorithm::get(&Mode::POPN_9K, &Mode::BEAT_7K).is_none());
    }

    #[test]
    fn algorithm_get_other_modes_return_none() {
        assert!(Algorithm::get(&Mode::BEAT_5K, &Mode::POPN_9K).is_none());
        assert!(Algorithm::get(&Mode::BEAT_14K, &Mode::POPN_9K).is_none());
        assert!(Algorithm::get(&Mode::BEAT_7K, &Mode::BEAT_14K).is_none());
    }

    // -- Algorithm::SevenToNine.modify with various patterns --

    #[test]
    fn seven_to_nine_pattern_0_default_mapping() {
        // pattern=0 (default, same as pattern=3)
        // key_lane=2, sc_lane=1, rest_lane=0
        let alg = Algorithm::SevenToNine;
        let activeln = vec![-1i32; 9];
        let notes: Vec<Option<Note>> = vec![None; 9];
        let last_note_time = vec![-100i32; 9];

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            0,   // now
            125, // duration
            0,   // seven_to_nine_pattern=0 (default)
            0,   // seven_to_nine_type=0
        );

        assert_eq!(result.len(), 9);
        // key_lane=2, so result[2..9] = 0..7
        assert_eq!(result[2], 0);
        assert_eq!(result[3], 1);
        assert_eq!(result[4], 2);
        assert_eq!(result[5], 3);
        assert_eq!(result[6], 4);
        assert_eq!(result[7], 5);
        assert_eq!(result[8], 6);
        // sc_lane=1 gets 7, rest_lane=0 gets 8
        assert_eq!(result[1], 7);
        assert_eq!(result[0], 8);
    }

    #[test]
    fn seven_to_nine_pattern_1_mapping() {
        // pattern=1: key_lane=1, sc_lane=0, rest_lane=8
        let alg = Algorithm::SevenToNine;
        let activeln = vec![-1i32; 9];
        let notes: Vec<Option<Note>> = vec![None; 9];
        let last_note_time = vec![-100i32; 9];

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            0,
            125,
            1, // pattern=1
            0, // type=0
        );

        assert_eq!(result.len(), 9);
        // key_lane=1, so result[1..8] = 0..7
        assert_eq!(result[1], 0);
        assert_eq!(result[2], 1);
        assert_eq!(result[3], 2);
        assert_eq!(result[4], 3);
        assert_eq!(result[5], 4);
        assert_eq!(result[6], 5);
        assert_eq!(result[7], 6);
        // sc_lane=0 gets 7, rest_lane=8 gets 8
        assert_eq!(result[0], 7);
        assert_eq!(result[8], 8);
    }

    #[test]
    fn seven_to_nine_pattern_4_mapping() {
        // pattern=4: key_lane=0, sc_lane=7, rest_lane=8
        let alg = Algorithm::SevenToNine;
        let activeln = vec![-1i32; 9];
        let notes: Vec<Option<Note>> = vec![None; 9];
        let last_note_time = vec![-100i32; 9];

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            0,
            125,
            4, // pattern=4
            0, // type=0
        );

        assert_eq!(result.len(), 9);
        // key_lane=0, so result[0..7] = 0..7
        assert_eq!(result[0], 0);
        assert_eq!(result[1], 1);
        assert_eq!(result[2], 2);
        assert_eq!(result[3], 3);
        assert_eq!(result[4], 4);
        assert_eq!(result[5], 5);
        assert_eq!(result[6], 6);
        // sc_lane=7 gets 7, rest_lane=8 gets 8
        assert_eq!(result[7], 7);
        assert_eq!(result[8], 8);
    }

    #[test]
    fn seven_to_nine_pattern_5_mapping() {
        // pattern=5: key_lane=0, sc_lane=8, rest_lane=7
        let alg = Algorithm::SevenToNine;
        let activeln = vec![-1i32; 9];
        let notes: Vec<Option<Note>> = vec![None; 9];
        let last_note_time = vec![-100i32; 9];

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            0,
            125,
            5, // pattern=5
            0,
        );

        assert_eq!(result.len(), 9);
        // key_lane=0, so result[0..7] = 0..7
        for i in 0..7 {
            assert_eq!(result[i], i as i32);
        }
        // sc_lane=8 gets 7, rest_lane=7 gets 8
        assert_eq!(result[8], 7);
        assert_eq!(result[7], 8);
    }

    // -- seven_to_nine_type=1: assign based on duration threshold --

    #[test]
    fn seven_to_nine_type_1_sc_exceeds_duration() {
        // type=1: if (now - last_note_time[sc_lane]) > duration, sc gets 7
        // pattern=0: sc_lane=1, rest_lane=0
        let alg = Algorithm::SevenToNine;
        let activeln = vec![-1i32; 9];
        let notes: Vec<Option<Note>> = vec![None; 9];
        let mut last_note_time = vec![-100i32; 9];
        // sc_lane=1, rest_lane=0
        last_note_time[1] = 0; // now - 0 = 1000 > duration(125) -> sc gets 7
        last_note_time[0] = 900; // now - 900 = 100

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            1000, // now
            125,  // duration
            0,    // pattern=0 (sc_lane=1, rest_lane=0)
            1,    // type=1
        );

        assert_eq!(result[1], 7); // sc_lane gets 7
        assert_eq!(result[0], 8); // rest_lane gets 8
    }

    #[test]
    fn seven_to_nine_type_1_sc_within_duration_but_older() {
        // type=1: now - last_note_time[sc_lane] >= now - last_note_time[rest_lane]
        // i.e., sc has older last note -> sc gets 7
        let alg = Algorithm::SevenToNine;
        let activeln = vec![-1i32; 9];
        let notes: Vec<Option<Note>> = vec![None; 9];
        let mut last_note_time = vec![-100i32; 9];
        // sc_lane=1, rest_lane=0
        last_note_time[1] = 800; // now - 800 = 200 (older)
        last_note_time[0] = 900; // now - 900 = 100 (more recent)

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            1000, // now
            125,  // duration (now - sc = 200 > 125)
            0,
            1, // type=1
        );

        // sc exceeds duration threshold -> sc gets 7
        assert_eq!(result[1], 7);
        assert_eq!(result[0], 8);
    }

    #[test]
    fn seven_to_nine_type_1_rest_gets_7_when_sc_more_recent() {
        // type=1: now - sc < duration AND now - sc < now - rest
        // -> rest gets 7, sc gets 8
        let alg = Algorithm::SevenToNine;
        let activeln = vec![-1i32; 9];
        let notes: Vec<Option<Note>> = vec![None; 9];
        let mut last_note_time = vec![-100i32; 9];
        last_note_time[1] = 950; // sc: now - 950 = 50 <= duration(125) and 50 < 100
        last_note_time[0] = 900; // rest: now - 900 = 100

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            1000,
            125,
            0,
            1, // type=1
        );

        // sc is more recent and within duration -> rest gets swapped
        assert_eq!(result[1], 8); // sc gets 8
        assert_eq!(result[0], 7); // rest gets 7
    }

    // -- seven_to_nine_type=2: assign based on which lane has older last note --

    #[test]
    fn seven_to_nine_type_2_sc_older_gets_7() {
        // type=2: if now - sc >= now - rest, sc gets 7
        let alg = Algorithm::SevenToNine;
        let activeln = vec![-1i32; 9];
        let notes: Vec<Option<Note>> = vec![None; 9];
        let mut last_note_time = vec![-100i32; 9];
        last_note_time[1] = 500; // sc: now - 500 = 500
        last_note_time[0] = 800; // rest: now - 800 = 200

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            1000,
            125,
            0,
            2, // type=2
        );

        // sc is older (500 >= 200) -> sc gets 7
        assert_eq!(result[1], 7);
        assert_eq!(result[0], 8);
    }

    #[test]
    fn seven_to_nine_type_2_rest_older_gets_7() {
        // type=2: now - sc < now - rest -> rest gets 7, sc gets 8
        let alg = Algorithm::SevenToNine;
        let activeln = vec![-1i32; 9];
        let notes: Vec<Option<Note>> = vec![None; 9];
        let mut last_note_time = vec![-100i32; 9];
        last_note_time[1] = 900; // sc: now - 900 = 100
        last_note_time[0] = 500; // rest: now - 500 = 500

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            1000,
            125,
            0,
            2, // type=2
        );

        // rest is older (100 < 500) -> rest gets 7, sc gets 8
        assert_eq!(result[1], 8);
        assert_eq!(result[0], 7);
    }

    // -- Active LN: overrides type-based assignment --

    #[test]
    fn active_ln_sc_lane_equals_7_sc_gets_7() {
        // When activeln[sc_lane] == 7, sc gets 7 and rest gets 8
        let alg = Algorithm::SevenToNine;
        let mut activeln = vec![-1i32; 9];
        activeln[1] = 7; // sc_lane=1 (for pattern=0) has active LN on lane 7
        let notes: Vec<Option<Note>> = vec![None; 9];
        let last_note_time = vec![-100i32; 9];

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            0,
            125,
            0, // pattern=0
            0,
        );

        assert_eq!(result[1], 7); // sc gets 7
        assert_eq!(result[0], 8); // rest gets 8
    }

    #[test]
    fn active_ln_sc_lane_not_7_sc_gets_8() {
        // When activeln[sc_lane] != -1 && != 7, sc gets 8 and rest gets 7
        let alg = Algorithm::SevenToNine;
        let mut activeln = vec![-1i32; 9];
        activeln[1] = 8; // sc_lane=1 has active LN on lane 8 (not 7)
        let notes: Vec<Option<Note>> = vec![None; 9];
        let last_note_time = vec![-100i32; 9];

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            0,
            125,
            0,
            0,
        );

        assert_eq!(result[1], 8); // sc gets 8
        assert_eq!(result[0], 7); // rest gets 7
    }

    #[test]
    fn active_ln_rest_lane_triggers_active_branch() {
        // When activeln[rest_lane] != -1 (even if sc is -1), active LN branch triggers
        let alg = Algorithm::SevenToNine;
        let mut activeln = vec![-1i32; 9];
        activeln[0] = 8; // rest_lane=0 has active LN
        let notes: Vec<Option<Note>> = vec![None; 9];
        let last_note_time = vec![-100i32; 9];

        let result = alg.modify(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            &activeln,
            &notes,
            &last_note_time,
            0,
            125,
            0,
            0,
        );

        // activeln[sc_lane=1] == -1 (not 7), so sc gets 8, rest gets 7
        assert_eq!(result[1], 8);
        assert_eq!(result[0], 7);
    }

    // -- ModeModifier construction and PatternModifier trait --

    #[test]
    fn mode_modifier_new_sets_assist_light() {
        let config = PlayerConfig::default();
        let m = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
        assert_eq!(m.get_assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn mode_modifier_set_seed() {
        let config = PlayerConfig::default();
        let mut m = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
        m.set_seed(100);
        assert_eq!(m.get_seed(), 100);
    }

    #[test]
    fn mode_modifier_set_seed_negative_ignored() {
        let config = PlayerConfig::default();
        let mut m = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
        let original = m.get_seed();
        m.set_seed(-1);
        assert_eq!(m.get_seed(), original);
    }

    #[test]
    fn mode_modifier_get_player() {
        let config = PlayerConfig::default();
        let m = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
        assert_eq!(m.get_player(), 0);
    }

    // -- ModeModifier::modify changes model mode --

    #[test]
    fn modify_changes_model_mode_to_after() {
        let config = PlayerConfig::default();
        let tl = TimeLine::new(0.0, 0, 8);
        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl]);

        let mut modifier = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
        modifier.modify(&mut model);

        assert_eq!(model.get_mode(), Some(&Mode::POPN_9K));
    }

    // -- ModeModifier::modify remaps notes for 7K -> 9K --

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn modify_7k_to_9k_remaps_notes() {
        let mut config = PlayerConfig::default();
        config.seven_to_nine_pattern = 0; // default pattern
        config.seven_to_nine_type = 0;

        // Create a BEAT_7K model with notes on lanes 0..7
        let mut tl = TimeLine::new(0.0, 0, 8);
        for i in 0..8 {
            tl.set_note(i, Some(Note::new_normal(i + 1)));
        }
        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl]);

        let mut modifier = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // After conversion, model is POPN_9K (9 lanes)
        // With pattern=0: key_lane=2, sc_lane=1, rest_lane=0
        // result[2..9] = 0..7 (input lanes), result[1]=7 (scratch), result[0]=8 (rest)
        // Lanes 0..7 of input are mapped to output lanes 2..9
        // Lane 8 of output gets input lane 8 (which doesn't exist in 7K, so None)

        // Verify the key lanes got the original notes
        // result[2]=0 -> output lane 2 gets input lane 0 (wav=1)
        assert_eq!(tls[0].get_note(2).unwrap().get_wav(), 1);
        // result[3]=1 -> output lane 3 gets input lane 1 (wav=2)
        assert_eq!(tls[0].get_note(3).unwrap().get_wav(), 2);
        // result[8]=6 -> output lane 8 gets input lane 6 (wav=7)
        assert_eq!(tls[0].get_note(8).unwrap().get_wav(), 7);
        // result[1]=7 -> output lane 1 gets input lane 7 (wav=8, scratch)
        assert_eq!(tls[0].get_note(1).unwrap().get_wav(), 8);
        // result[0]=8 -> output lane 0 gets input lane 8 (None in 8-lane model)
        assert!(tls[0].get_note(0).is_none());
    }

    // -- hran_threshold_bpm <= 0 sets threshold to 0 --

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn hran_threshold_bpm_zero_sets_threshold_zero() {
        let mut config = PlayerConfig::default();
        config.hran_threshold_bpm = 0;
        config.seven_to_nine_pattern = 0;
        config.seven_to_nine_type = 0;

        let tl = TimeLine::new(0.0, 0, 8);
        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl]);

        let mut modifier = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
        modifier.modify(&mut model);

        // When hran_threshold_bpm <= 0, hran_threshold is set to 0
        assert_eq!(modifier.hran_threshold, 0);
    }

    // -- Same mode: no algorithm, identity mapping --

    #[test]
    fn modify_same_mode_no_algorithm_identity() {
        let config = PlayerConfig::default();

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(3, Some(Note::new_normal(4)));

        let mut model = make_test_model(&Mode::BEAT_7K, vec![tl]);

        // BEAT_7K -> BEAT_7K: Algorithm::get returns None, keys are used as identity
        let mut modifier = ModeModifier::new(Mode::BEAT_7K, Mode::BEAT_7K, config);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // Notes should be in the same positions (identity mapping)
        assert_eq!(tls[0].get_note(0).unwrap().get_wav(), 1);
        assert_eq!(tls[0].get_note(3).unwrap().get_wav(), 4);
    }

    // -- Empty model: no panic --

    #[test]
    fn modify_empty_model_no_panic() {
        let config = PlayerConfig::default();
        let mut model = make_test_model(&Mode::BEAT_7K, vec![]);

        let mut modifier = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config);
        modifier.modify(&mut model);
    }
}

use crate::player_config::PlayerConfig;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;

use crate::pattern::lane_shuffle_modifier::*;
use crate::pattern::note_shuffle_modifier::NoteShuffleModifier;
use crate::pattern::pattern_modify_log::PatternModifyLog;
use crate::pattern::random::{Random, RandomUnit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssistLevel {
    None,
    LightAssist,
    Assist,
}

pub trait PatternModifier {
    fn modify(&mut self, model: &mut BMSModel);

    fn assist_level(&self) -> AssistLevel;
    fn set_assist_level(&mut self, assist: AssistLevel);

    fn get_seed(&self) -> i64;
    fn set_seed(&mut self, seed: i64);

    fn player(&self) -> i32;

    /// Whether this modifier has a displayable lane shuffle pattern.
    /// LaneShuffleModifier subclasses override this to return true.
    fn is_lane_shuffle_to_display(&self) -> bool {
        false
    }

    /// Get the random lane pattern for display (e.g., for playinfo.laneShufflePattern).
    /// LaneShuffleModifier subclasses override this to return their pattern.
    fn get_lane_shuffle_random_pattern(&self, _mode: &Mode) -> Option<Vec<i32>> {
        None
    }

    fn keys(&self, mode: &Mode, player: i32, contains_scratch: bool) -> Vec<i32> {
        if player >= mode.player() {
            return Vec::new();
        }
        let startkey = mode.key() * player / mode.player();
        (startkey..startkey + mode.key() / mode.player())
            .filter(|&i| contains_scratch || !mode.is_scratch_key(i))
            .collect()
    }
}

pub struct PatternModifierBase {
    pub assist: AssistLevel,
    pub seed: i64,
    pub player: i32,
}

impl Default for PatternModifierBase {
    fn default() -> Self {
        PatternModifierBase {
            assist: AssistLevel::None,
            seed: (rand::random::<f64>() * 65536.0 * 256.0) as i64,
            player: 0,
        }
    }
}

impl PatternModifierBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_player(player: i32) -> Self {
        PatternModifierBase {
            player,
            ..Self::default()
        }
    }

    pub fn with_assist(assist: AssistLevel) -> Self {
        PatternModifierBase {
            assist,
            ..Self::default()
        }
    }
}

/// Identity modifier (does nothing)
pub struct IdentityModifier {
    pub base: PatternModifierBase,
}

impl Default for IdentityModifier {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentityModifier {
    pub fn new() -> Self {
        IdentityModifier {
            base: PatternModifierBase::new(),
        }
    }
}

impl PatternModifier for IdentityModifier {
    fn modify(&mut self, _model: &mut BMSModel) {}

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

/// Apply pattern modify log to a model
pub fn apply_modify_log(model: &mut BMSModel, log: &[PatternModifyLog]) {
    let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);
    let lanes = mode_key as usize;

    let timelines = model.all_time_lines_mut();
    for tl in timelines.iter_mut() {
        let mut pm: Option<&PatternModifyLog> = None;
        for pms in log {
            if pms.section == tl.get_section() {
                pm = Some(pms);
                break;
            }
        }
        if let Some(pm) = pm
            && let Some(ref modify) = pm.modify
        {
            let mut notes: Vec<Option<Note>> = Vec::with_capacity(lanes);
            let mut hnotes: Vec<Option<Note>> = Vec::with_capacity(lanes);
            for i in 0..lanes {
                let m = if i < modify.len() {
                    modify[i]
                } else {
                    i as i32
                };
                notes.push(tl.take_note(m));
                hnotes.push(tl.hidden_note(m).cloned());
            }
            for i in 0..lanes {
                tl.set_note(i as i32, notes[i].take());
                tl.set_hidden_note(i as i32, hnotes[i].take());
            }
        }
    }
}

/// Create a pattern modifier from an option ID
pub fn create_pattern_modifier(
    id: i32,
    player: i32,
    mode: &Mode,
    config: &PlayerConfig,
) -> Box<dyn PatternModifier> {
    let chart_option = Random::from_id(id, mode);
    match chart_option {
        Random::Identity => Box::new(IdentityModifier::new()),
        Random::Mirror => Box::new(LaneMirrorShuffleModifier::new(player, false)),
        Random::MirrorEx => Box::new(LaneMirrorShuffleModifier::new(player, true)),
        Random::Rotate => Box::new(LaneRotateShuffleModifier::new(player, false)),
        Random::RotateEx => Box::new(LaneRotateShuffleModifier::new(player, true)),
        Random::Random => Box::new(LaneRandomShuffleModifier::new(player, false)),
        Random::RandomEx => Box::new(LaneRandomShuffleModifier::new(player, true)),
        Random::Cross => Box::new(LaneCrossShuffleModifier::new(player, false)),
        Random::RandomPlayable => Box::new(LanePlayableRandomShuffleModifier::new(player, false)),
        Random::Flip => Box::new(PlayerFlipModifier::new()),
        Random::Battle => Box::new(PlayerBattleModifier::new()),
        _ => match chart_option.unit() {
            RandomUnit::Note => {
                Box::new(NoteShuffleModifier::new(chart_option, player, mode, config))
            }
            _ => panic!("Unexpected value: {:?}", chart_option.unit()),
        },
    }
}

/// Create a BMSModel with the given mode and timelines, for testing purposes.
#[cfg(test)]
pub(crate) fn make_test_model(mode: &Mode, timelines: Vec<TimeLine>) -> BMSModel {
    let mut model = BMSModel::new();
    model.timelines = timelines;
    model.set_mode(mode.clone());
    model
}

pub fn move_to_background(tls: &mut [TimeLine], tl_index: usize, lane: i32) {
    let note = tls[tl_index].note(lane).cloned();
    if let Some(ref n) = note {
        if n.is_long() {
            // Find the pair timeline
            if let Some(_pair_idx) = n.pair() {
                // In the Java code, pair is a direct reference. Here pair_idx is the timeline index
                // where the pair note lives. We need to find the timeline where the pair note is.
                // Actually, in our model, get_pair() returns an Option<usize> which is the pair index
                // in the timelines array, but in the pattern context, we need to search by section.
                // Let's search for the pair note by iterating over timelines.
                for (i, tl_item) in tls.iter_mut().enumerate() {
                    if i == tl_index {
                        continue;
                    }
                    if let Some(pair_note) = tl_item.note(lane)
                        && pair_note.is_long()
                        && pair_note.is_end()
                    {
                        // Check if this is the matching pair
                        let pair_note_clone = tl_item.take_note(lane);
                        if let Some(pn) = pair_note_clone {
                            tl_item.add_back_ground_note(pn);
                        }
                        break;
                    }
                }
            }
        }

        if !n.is_mine() {
            let note_to_bg = tls[tl_index].take_note(lane);
            if let Some(nbg) = note_to_bg {
                tls[tl_index].add_back_ground_note(nbg);
            }
        } else {
            tls[tl_index].set_note(lane, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::note::Note;
    use bms_model::time_line::TimeLine;

    // -- AssistLevel --

    #[test]
    fn assist_level_eq() {
        assert_eq!(AssistLevel::None, AssistLevel::None);
        assert_eq!(AssistLevel::LightAssist, AssistLevel::LightAssist);
        assert_eq!(AssistLevel::Assist, AssistLevel::Assist);
        assert_ne!(AssistLevel::None, AssistLevel::Assist);
    }

    #[test]
    fn assist_level_clone_and_copy() {
        let a = AssistLevel::LightAssist;
        let b = a;
        assert_eq!(a, b);
    }

    // -- PatternModifierBase --

    #[test]
    fn base_default_assist_is_none() {
        let base = PatternModifierBase::new();
        assert_eq!(base.assist, AssistLevel::None);
    }

    #[test]
    fn base_default_player_is_zero() {
        let base = PatternModifierBase::new();
        assert_eq!(base.player, 0);
    }

    #[test]
    fn base_default_seed_is_non_negative() {
        let base = PatternModifierBase::new();
        assert!(base.seed >= 0);
    }

    #[test]
    fn base_with_player() {
        let base = PatternModifierBase::with_player(2);
        assert_eq!(base.player, 2);
        assert_eq!(base.assist, AssistLevel::None);
    }

    #[test]
    fn base_with_assist() {
        let base = PatternModifierBase::with_assist(AssistLevel::Assist);
        assert_eq!(base.assist, AssistLevel::Assist);
        assert_eq!(base.player, 0);
    }

    // -- IdentityModifier --

    #[test]
    fn identity_modifier_default_values() {
        let modifier = IdentityModifier::new();
        assert_eq!(modifier.assist_level(), AssistLevel::None);
        assert_eq!(modifier.player(), 0);
    }

    #[test]
    fn identity_modifier_set_seed_positive() {
        let mut modifier = IdentityModifier::new();
        modifier.set_seed(42);
        assert_eq!(modifier.get_seed(), 42);
    }

    #[test]
    fn identity_modifier_set_seed_negative_is_ignored() {
        let mut modifier = IdentityModifier::new();
        let original_seed = modifier.get_seed();
        modifier.set_seed(-1);
        assert_eq!(modifier.get_seed(), original_seed);
    }

    #[test]
    fn identity_modifier_set_seed_zero() {
        let mut modifier = IdentityModifier::new();
        modifier.set_seed(0);
        assert_eq!(modifier.get_seed(), 0);
    }

    #[test]
    fn identity_modifier_set_assist_level() {
        let mut modifier = IdentityModifier::new();
        modifier.set_assist_level(AssistLevel::Assist);
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn identity_modifier_does_not_change_model() {
        let mut model = BMSModel::new();
        let tl = TimeLine::new(0.0, 0, 8);
        model.timelines = vec![tl];
        model.set_mode(Mode::BEAT_7K);
        let note = Note::new_normal(1);
        model.all_time_lines_mut()[0].set_note(0, Some(note));

        let mut modifier = IdentityModifier::new();
        modifier.modify(&mut model);

        // Note should still be in lane 0
        assert!(model.timelines[0].note(0).is_some());
        assert_eq!(model.timelines[0].note(0).unwrap().wav(), 1);
    }

    // -- keys --

    #[test]
    fn get_keys_beat_7k_player0_no_scratch() {
        let modifier = IdentityModifier::new();
        let mode = Mode::BEAT_7K;
        let keys = modifier.keys(&mode, 0, false);
        // BEAT_7K: key=8, player=1, scratch=7
        // keys 0..8 excluding scratch key 7
        assert_eq!(keys, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn get_keys_beat_7k_player0_with_scratch() {
        let modifier = IdentityModifier::new();
        let mode = Mode::BEAT_7K;
        let keys = modifier.keys(&mode, 0, true);
        // All keys 0..8 including scratch
        assert_eq!(keys, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn get_keys_popn_9k_no_scratch() {
        let modifier = IdentityModifier::new();
        let mode = Mode::POPN_9K;
        let keys = modifier.keys(&mode, 0, false);
        // POPN_9K: key=9, player=1, no scratch keys
        assert_eq!(keys, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn get_keys_beat_14k_player0_no_scratch() {
        let modifier = IdentityModifier::new();
        let mode = Mode::BEAT_14K;
        // BEAT_14K: key=16, player=2
        // Player 0: startkey = 16*0/2 = 0, range = 0..8
        // Scratch keys: 7, 15 -> only 7 is in range
        let keys = modifier.keys(&mode, 0, false);
        assert_eq!(keys, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn get_keys_beat_14k_player1_no_scratch() {
        let modifier = IdentityModifier::new();
        let mode = Mode::BEAT_14K;
        // Player 1: startkey = 16*1/2 = 8, range = 8..16
        // Scratch keys: 7, 15 -> only 15 is in range
        let keys = modifier.keys(&mode, 1, false);
        assert_eq!(keys, vec![8, 9, 10, 11, 12, 13, 14]);
    }

    #[test]
    fn get_keys_invalid_player_returns_empty() {
        let modifier = IdentityModifier::new();
        let mode = Mode::BEAT_7K;
        // player=1 but mode.player()=1, so player >= mode.player() -> empty
        let keys = modifier.keys(&mode, 1, false);
        assert!(keys.is_empty());
    }

    // -- move_to_background --

    #[test]
    fn move_normal_note_to_background() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));

        let mut tls = vec![tl];
        move_to_background(&mut tls, 0, 0);

        // Note should be removed from lane 0
        assert!(tls[0].note(0).is_none());
        // Note should be in background
        assert_eq!(tls[0].back_ground_notes().len(), 1);
        assert_eq!(tls[0].back_ground_notes()[0].wav(), 1);
    }

    #[test]
    fn move_mine_note_removes_without_background() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_mine(-1, 10.0)));

        let mut tls = vec![tl];
        move_to_background(&mut tls, 0, 0);

        // Mine note should be removed
        assert!(tls[0].note(0).is_none());
        // Should NOT be placed in background
        assert!(tls[0].back_ground_notes().is_empty());
    }

    #[test]
    fn move_to_background_no_note_is_noop() {
        let tl = TimeLine::new(0.0, 0, 8);
        let mut tls = vec![tl];
        move_to_background(&mut tls, 0, 0);
        assert!(tls[0].note(0).is_none());
        assert!(tls[0].back_ground_notes().is_empty());
    }

    // -- apply_modify_log --

    #[test]
    fn apply_modify_log_swaps_notes() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(1.0, 1000, 8);
        tl.set_note(0, Some(Note::new_normal(10)));
        tl.set_note(1, Some(Note::new_normal(20)));

        let mut model = make_test_model(&mode, vec![tl]);

        // Swap lanes 0 and 1 (modify = [1, 0, 2, 3, 4, 5, 6, 7])
        let log = vec![PatternModifyLog::new(1.0, vec![1, 0, 2, 3, 4, 5, 6, 7])];
        apply_modify_log(&mut model, &log);

        let tls = model.timelines;
        // Lane 0 should now have wav=20 (originally from lane 1)
        assert_eq!(tls[0].note(0).unwrap().wav(), 20);
        // Lane 1 should now have wav=10 (originally from lane 0)
        assert_eq!(tls[0].note(1).unwrap().wav(), 10);
    }

    #[test]
    fn apply_modify_log_no_matching_section_is_noop() {
        let mode = Mode::BEAT_7K;
        let mut tl = TimeLine::new(1.0, 1000, 8);
        tl.set_note(0, Some(Note::new_normal(10)));

        let mut model = make_test_model(&mode, vec![tl]);

        // Log has section 2.0, but timeline has section 1.0 -> no match
        let log = vec![PatternModifyLog::new(2.0, vec![1, 0, 2, 3, 4, 5, 6, 7])];
        apply_modify_log(&mut model, &log);

        let tls = model.timelines;
        assert_eq!(tls[0].note(0).unwrap().wav(), 10);
    }
}

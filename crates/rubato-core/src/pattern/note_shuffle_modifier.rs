use crate::player_config::PlayerConfig;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;

use crate::pattern::pattern_modifier::{AssistLevel, PatternModifier, PatternModifierBase};
use crate::pattern::random::Random;
use crate::pattern::randomizer::Randomizer;

pub struct NoteShuffleModifier {
    pub base: PatternModifierBase,
    randomizer: Randomizer,
    is_scratch_lane_modify: bool,
}

impl NoteShuffleModifier {
    pub fn new(r: Random, player: i32, mode: &Mode, config: &PlayerConfig) -> Self {
        let randomizer = Randomizer::create_with_side(r, player, mode, config);
        NoteShuffleModifier {
            base: PatternModifierBase::with_player(player),
            randomizer,
            is_scratch_lane_modify: r.is_scratch_lane_modify(),
        }
    }
}

impl PatternModifier for NoteShuffleModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        self.randomizer.set_random_seed(self.base.seed);
        let keys = match model.mode() {
            Some(m) => self.keys(m, self.base.player, self.is_scratch_lane_modify),
            None => return,
        };
        self.randomizer.set_modify_lanes(&keys);
        let timelines = &mut model.timelines;
        for tl in timelines.iter_mut() {
            if tl.exist_note() || tl.exist_hidden_note() {
                self.randomizer.permutate(tl);
            }
        }
        self.base.assist = self.randomizer.assist_level();
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
    use bms_model::note::Note;
    use bms_model::time_line::TimeLine;
    use std::collections::HashSet;

    fn default_config() -> PlayerConfig {
        PlayerConfig::default()
    }

    /// Build a model with the given number of timelines, each having notes on
    /// the specified lanes. Each note gets a unique wav id derived from the
    /// timeline index and lane so we can track where it ends up.
    fn make_model_with_notes(mode: &Mode, timeline_count: usize, note_lanes: &[i32]) -> BMSModel {
        let key_count = mode.key() as usize;
        let mut timelines = Vec::with_capacity(timeline_count);
        for i in 0..timeline_count {
            let mut tl = TimeLine::new(i as f64, (i * 1000) as i64, key_count as i32);
            for &lane in note_lanes {
                // wav = timeline_index * 100 + lane, so each note is unique
                let wav = (i as i32) * 100 + lane;
                tl.set_note(lane, Some(Note::new_normal(wav)));
            }
            timelines.push(tl);
        }
        make_test_model(mode, timelines)
    }

    /// Collect the (lane -> wav) mapping from all timelines.
    fn collect_note_positions(model: &BMSModel) -> Vec<Vec<(i32, i32)>> {
        let key_count = model.mode().map(|m| m.key()).unwrap_or(0);
        model
            .timelines
            .iter()
            .map(|tl| {
                (0..key_count)
                    .filter_map(|lane| tl.note(lane).map(|n| (lane, n.wav())))
                    .collect()
            })
            .collect()
    }

    // -- Construction --

    #[test]
    fn new_srandom_sets_player() {
        let config = default_config();
        let modifier = NoteShuffleModifier::new(Random::SRandom, 0, &Mode::BEAT_7K, &config);
        assert_eq!(modifier.player(), 0);
    }

    #[test]
    fn new_hrandom_sets_player() {
        let config = default_config();
        let modifier = NoteShuffleModifier::new(Random::HRandom, 1, &Mode::BEAT_14K, &config);
        assert_eq!(modifier.player(), 1);
    }

    #[test]
    fn new_srandom_is_not_scratch_modify() {
        let config = default_config();
        let modifier = NoteShuffleModifier::new(Random::SRandom, 0, &Mode::BEAT_7K, &config);
        assert!(!modifier.is_scratch_lane_modify);
    }

    #[test]
    fn new_allscr_is_scratch_modify() {
        let config = default_config();
        let modifier = NoteShuffleModifier::new(Random::AllScr, 0, &Mode::BEAT_7K, &config);
        assert!(modifier.is_scratch_lane_modify);
    }

    // -- set_seed / get_seed --

    #[test]
    fn set_seed_positive_updates() {
        let config = default_config();
        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &Mode::BEAT_7K, &config);
        modifier.set_seed(42);
        assert_eq!(modifier.get_seed(), 42);
    }

    #[test]
    fn set_seed_zero_updates() {
        let config = default_config();
        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &Mode::BEAT_7K, &config);
        modifier.set_seed(0);
        assert_eq!(modifier.get_seed(), 0);
    }

    #[test]
    fn set_seed_negative_is_ignored() {
        let config = default_config();
        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &Mode::BEAT_7K, &config);
        let original = modifier.get_seed();
        modifier.set_seed(-1);
        assert_eq!(modifier.get_seed(), original);
    }

    // -- assist level --

    #[test]
    fn initial_assist_level_is_none() {
        let config = default_config();
        let modifier = NoteShuffleModifier::new(Random::SRandom, 0, &Mode::BEAT_7K, &config);
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn set_assist_level_roundtrips() {
        let config = default_config();
        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &Mode::BEAT_7K, &config);
        modifier.set_assist_level(AssistLevel::Assist);
        assert_eq!(modifier.assist_level(), AssistLevel::Assist);
    }

    // -- Basic modification: notes are shuffled --

    #[test]
    fn srandom_modify_shuffles_notes() {
        let mode = Mode::BEAT_7K;
        let config = default_config();
        // Put notes on all 7 non-scratch lanes across 5 timelines.
        // With SRandom, at least one timeline should have a different permutation
        // from identity (the probability of all 5 being identity with 7 lanes is negligible).
        let note_lanes: Vec<i32> = (0..7).collect();
        let mut model = make_model_with_notes(&mode, 5, &note_lanes);

        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
        modifier.set_seed(12345);
        modifier.modify(&mut model);

        let positions = collect_note_positions(&model);
        // Verify all original wav values still exist (notes are permuted, not lost)
        let mut all_wavs: HashSet<i32> = HashSet::new();
        for tl_notes in &positions {
            for &(_lane, wav) in tl_notes {
                all_wavs.insert(wav);
            }
        }
        // We created 5 timelines * 7 notes = 35 unique wav values
        assert_eq!(all_wavs.len(), 35);

        // Check that at least one timeline has notes in different lanes than original
        let mut any_changed = false;
        for (tl_idx, tl_notes) in positions.iter().enumerate() {
            for &(lane, wav) in tl_notes {
                let original_lane = wav - (tl_idx as i32) * 100;
                if lane != original_lane {
                    any_changed = true;
                    break;
                }
            }
            if any_changed {
                break;
            }
        }
        assert!(any_changed, "S-Random should shuffle at least one note");
    }

    // -- S-Random: each timeline is independently permuted --

    #[test]
    fn srandom_timelines_have_independent_permutations() {
        let mode = Mode::BEAT_7K;
        let config = default_config();
        let note_lanes: Vec<i32> = (0..7).collect();
        // Use many timelines to make it highly unlikely all get the same permutation
        let mut model = make_model_with_notes(&mode, 20, &note_lanes);

        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
        modifier.set_seed(99999);
        modifier.modify(&mut model);

        let positions = collect_note_positions(&model);

        // Extract the permutation for each timeline: for each note, find which
        // original lane it came from (via wav = tl_idx * 100 + original_lane).
        let mut permutations: Vec<Vec<i32>> = Vec::new();
        for (tl_idx, tl_notes) in positions.iter().enumerate() {
            let mut perm = vec![-1i32; 7];
            for &(lane, wav) in tl_notes {
                let original_lane = wav - (tl_idx as i32) * 100;
                perm[lane as usize] = original_lane;
            }
            permutations.push(perm);
        }

        // Verify that not all permutations are identical (would indicate lane shuffle, not note shuffle)
        let distinct_count = permutations.iter().collect::<HashSet<_>>().len();
        assert!(
            distinct_count > 1,
            "S-Random should produce different permutations across timelines, got {} distinct out of {}",
            distinct_count,
            permutations.len()
        );
    }

    // -- H-Random: permutation constrained by hran_threshold --

    #[test]
    fn hrandom_avoids_rapid_same_lane_repeats() {
        let mode = Mode::BEAT_7K;
        let mut config = default_config();
        // Set a high threshold BPM so threshold_millis is large,
        // making it very unlikely to place consecutive notes on the same lane.
        config.play_settings.hran_threshold_bpm = 30; // threshold = ceil(15000/30) = 500ms
        // Create timelines with small time gaps (100ms apart) and single notes.
        // With a 500ms threshold, each note should end up on a different lane
        // than the previous one.
        let key_count = mode.key() as usize;
        let mut timelines = Vec::new();
        for i in 0..7 {
            let mut tl = TimeLine::new(i as f64, (i * 100) as i64, key_count as i32);
            // Put a single note on lane 0 in each timeline
            tl.set_note(0, Some(Note::new_normal(i)));
            timelines.push(tl);
        }
        let mut model = make_test_model(&mode, timelines);

        let mut modifier = NoteShuffleModifier::new(Random::HRandom, 0, &mode, &config);
        modifier.set_seed(42);
        modifier.modify(&mut model);

        // Collect which lane each note ended up on
        let mut assigned_lanes: Vec<i32> = Vec::new();
        for tl in model.timelines {
            for lane in 0..key_count as i32 {
                if tl.note(lane).is_some() {
                    assigned_lanes.push(lane);
                    break;
                }
            }
        }

        // With 500ms threshold and 100ms gaps, consecutive notes should not
        // be on the same lane (the threshold prevents it).
        for i in 1..assigned_lanes.len() {
            assert_ne!(
                assigned_lanes[i],
                assigned_lanes[i - 1],
                "H-Random should avoid placing consecutive notes on the same lane (timelines {} and {})",
                i - 1,
                i
            );
        }
    }

    #[test]
    fn hrandom_sets_light_assist_level() {
        let mode = Mode::BEAT_7K;
        let config = default_config();
        let note_lanes: Vec<i32> = (0..7).collect();
        let mut model = make_model_with_notes(&mode, 3, &note_lanes);

        let mut modifier = NoteShuffleModifier::new(Random::HRandom, 0, &mode, &config);
        modifier.set_seed(42);
        modifier.modify(&mut model);

        assert_eq!(modifier.assist_level(), AssistLevel::LightAssist);
    }

    // -- Deterministic with same seed --

    #[test]
    fn srandom_same_seed_produces_same_result() {
        let mode = Mode::BEAT_7K;
        let config = default_config();
        let note_lanes: Vec<i32> = (0..7).collect();
        let seed: i64 = 777;

        let run = || {
            let mut model = make_model_with_notes(&mode, 10, &note_lanes);
            let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
            modifier.set_seed(seed);
            modifier.modify(&mut model);
            collect_note_positions(&model)
        };

        let result1 = run();
        let result2 = run();
        assert_eq!(
            result1, result2,
            "Same seed should produce identical permutations"
        );
    }

    #[test]
    fn hrandom_same_seed_produces_same_result() {
        let mode = Mode::BEAT_7K;
        let config = default_config();
        let note_lanes: Vec<i32> = (0..7).collect();
        let seed: i64 = 555;

        let run = || {
            let mut model = make_model_with_notes(&mode, 10, &note_lanes);
            let mut modifier = NoteShuffleModifier::new(Random::HRandom, 0, &mode, &config);
            modifier.set_seed(seed);
            modifier.modify(&mut model);
            collect_note_positions(&model)
        };

        let result1 = run();
        let result2 = run();
        assert_eq!(
            result1, result2,
            "Same seed should produce identical permutations"
        );
    }

    #[test]
    fn different_seeds_produce_different_results() {
        let mode = Mode::BEAT_7K;
        let config = default_config();
        let note_lanes: Vec<i32> = (0..7).collect();

        let run = |seed: i64| {
            let mut model = make_model_with_notes(&mode, 10, &note_lanes);
            let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
            modifier.set_seed(seed);
            modifier.modify(&mut model);
            collect_note_positions(&model)
        };

        let result1 = run(100);
        let result2 = run(200);
        assert_ne!(
            result1, result2,
            "Different seeds should produce different permutations"
        );
    }

    // -- Edge cases --

    #[test]
    fn modify_with_no_mode_is_noop() {
        // BMSModel without a mode set
        let mut model = BMSModel::new();
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        model.timelines = vec![tl];
        // Do NOT call model.set_mode()

        let config = default_config();
        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &Mode::BEAT_7K, &config);
        modifier.set_seed(42);
        modifier.modify(&mut model);

        // Note should remain unchanged
        assert!(model.timelines[0].note(0).is_some());
        assert_eq!(model.timelines[0].note(0).unwrap().wav(), 1);
    }

    #[test]
    fn modify_with_empty_timelines_is_noop() {
        let mode = Mode::BEAT_7K;
        let mut model = make_test_model(&mode, vec![]);

        let config = default_config();
        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
        modifier.set_seed(42);
        modifier.modify(&mut model);

        assert!(model.timelines.is_empty());
    }

    #[test]
    fn modify_skips_timelines_without_notes() {
        let mode = Mode::BEAT_7K;
        let key_count = mode.key() as usize;
        // First timeline: has notes. Second: empty.
        let mut tl0 = TimeLine::new(0.0, 0, key_count as i32);
        tl0.set_note(0, Some(Note::new_normal(10)));
        tl0.set_note(1, Some(Note::new_normal(11)));
        let tl1 = TimeLine::new(1.0, 1000, key_count as i32);
        // tl1 has no notes

        let mut model = make_test_model(&mode, vec![tl0, tl1]);

        let config = default_config();
        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
        modifier.set_seed(42);
        modifier.modify(&mut model);

        // Second timeline should still have no notes
        let tls = model.timelines;
        let mut has_note = false;
        for lane in 0..key_count as i32 {
            if tls[1].note(lane).is_some() {
                has_note = true;
                break;
            }
        }
        assert!(!has_note, "Empty timeline should remain empty after modify");
    }

    #[test]
    fn note_count_preserved_after_modify() {
        let mode = Mode::BEAT_7K;
        let config = default_config();
        let note_lanes = vec![0, 2, 4];
        let mut model = make_model_with_notes(&mode, 5, &note_lanes);

        let before_count: usize = model
            .timelines
            .iter()
            .map(|tl| {
                (0..mode.key())
                    .filter(|&lane| tl.note(lane).is_some())
                    .count()
            })
            .sum();

        let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
        modifier.set_seed(42);
        modifier.modify(&mut model);

        let after_count: usize = model
            .timelines
            .iter()
            .map(|tl| {
                (0..mode.key())
                    .filter(|&lane| tl.note(lane).is_some())
                    .count()
            })
            .sum();

        assert_eq!(before_count, after_count, "Note count should be preserved");
    }
}

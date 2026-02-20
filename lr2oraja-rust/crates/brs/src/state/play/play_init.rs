// Play state initialization — constructor and judge/gauge setup.

use tracing::{info, warn};

use bms_database::RivalDataAccessor;
use bms_database::score_data_property::ScoreDataProperty;
use bms_model::{LaneProperty, PlayMode};
use bms_pattern::{
    ModeModifier, PatternModifier, RandomType, SevenToNinePattern, SevenToNineType, get_random,
};
use bms_rule::judge_algorithm::DEFAULT_ALGORITHMS;
use bms_rule::judge_manager::{JudgeConfig, JudgeManager};
use bms_rule::{GrooveGauge, PlayerRule};

use crate::app_state::AppStateType;
use crate::state::StateContext;
use crate::target_property::{RivalScore, TargetContext, TargetProperty};

use super::play_skin_state::ScratchAngleState;
use super::{
    FINISH_MARGIN_US, GaugeAutoShift, NOT_SET, PlayPhase, PlayState, apply_double_option,
    apply_double_option_with_autoplay, apply_pattern_modifier, apply_pre_shuffle_modifiers,
    assist_to_i32, gauge_type_from_i32,
};

impl PlayState {
    pub fn new() -> Self {
        Self {
            phase: PlayPhase::Preload,
            judge_notes: Vec::new(),
            lane_property: LaneProperty::new(PlayMode::Beat7K),
            judge_manager: None,
            gauge: None,
            gauge_auto_shift: GaugeAutoShift::None,
            bottom_gauge: bms_rule::gauge_property::GaugeType::Normal,
            playtime_us: 0,
            last_note_time_us: 0,
            last_gauge_log_time_us: 0,
            gauge_log: Vec::new(),
            replay_log: Vec::new(),
            replay_cursor: 0,
            is_autoplay: true,
            is_replay: false,
            key_states: Vec::new(),
            key_changed_times: Vec::new(),
            audio_driver: None,
            key_sound_processor: None,
            bga_processor: None,
            input_processor: None,
            play_speed: 100,
            key_beam_stop: false,
            assist: 0,
            is_judge_started: false,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            now_bpm: 0.0,
            score_data_property: ScoreDataProperty::new(),
            scratch_angle: ScratchAngleState::new(0),
            start_pressed: false,
            select_pressed: false,
            is_practice: false,
            practice_config: None,
            rhythm_timer: None,
            last_render_time_us: 0,
        }
    }

    /// Build the rival score array for target comparison.
    ///
    /// Iterates all loaded rivals, queries each rival's score for the given
    /// song (sha256 + mode), and collects them alongside the player's own
    /// score. The result is sorted by exscore descending, matching Java's
    /// `RivalTargetProperty.createScoreArray()`.
    pub(super) fn build_rival_scores(ctx: &StateContext) -> Vec<RivalScore> {
        let db = match ctx.database {
            Some(db) => db,
            None => return Vec::new(),
        };
        let model = match &ctx.resource.bms_model {
            Some(m) => m,
            None => return Vec::new(),
        };

        let sha256 = &model.sha256;
        let mode = model.mode.mode_id();
        let mut scores = Vec::new();

        for rival in db.rival.rivals() {
            match RivalDataAccessor::get_rival_score(&rival.db_path, sha256, mode) {
                Ok(Some(sd)) => {
                    scores.push(RivalScore {
                        name: rival.info.name.clone(),
                        exscore: sd.exscore(),
                    });
                }
                Ok(None) => {} // Rival has no score for this song
                Err(e) => {
                    warn!(rival = rival.info.name, "Failed to load rival score: {e}");
                }
            }
        }

        // Add player's own score (name = "" to match Java convention)
        let oldscore = &ctx.resource.oldscore;
        scores.push(RivalScore {
            name: String::new(),
            exscore: oldscore.exscore(),
        });

        // Sort by exscore descending
        scores.sort_by(|a, b| b.exscore.cmp(&a.exscore));
        scores
    }

    /// Initialize judge and gauge from the loaded model.
    pub(super) fn init_judge_and_gauge(&mut self, ctx: &mut StateContext) {
        let model = match &ctx.resource.bms_model {
            Some(m) => m,
            None => {
                info!("Play: no BMS model loaded, skipping to Result");
                *ctx.transition = Some(AppStateType::Result);
                return;
            }
        };

        // Clone model for pattern modification
        let mut model = model.clone();
        self.lane_property = LaneProperty::new(model.mode);

        // Initialize BGA processor from model (before pattern modifiers alter it)
        self.bga_processor = Some(bms_render::bga::bga_processor::BgaProcessor::new(&model));

        // Consume ghost battle settings (if present, overrides pattern seed and
        // skips pre-shuffle modifiers to match the opponent's exact pattern).
        // Java parity: BMSPlayer lines ~190-350, GhostBattlePlay.consume()
        let ghost_battle = ctx.resource.ghost_battle.take();

        if let Some(ref gb) = ghost_battle {
            info!(
                seed = gb.random_seed,
                lane_sequence = gb.lane_sequence,
                "Play: ghost battle active"
            );
        }

        // Apply pre-shuffle modifiers (scroll, longnote, mine, extranote)
        // Java: applied before lane shuffle, config value > 0 means active
        // Java offsets config values by -1 (e.g., ScrollMode 1 -> enum index 0)
        // Ghost battle: skip pre-shuffle modifiers (Java clears mods array)
        if ghost_battle.is_none() {
            self.assist += apply_pre_shuffle_modifiers(&mut model, ctx.player_config);
        }

        // M2: Random Trainer — override 1P pattern with fixed lane order.
        // When active, skip normal random and apply a fixed lane mapping instead.
        let random_trainer_active = ctx.resource.random_trainer_enabled
            && !self.is_autoplay
            && ghost_battle.is_none()
            && !ctx.resource.is_course();

        // Apply 1P pattern shuffle
        // Ghost battle: use the opponent's seed for deterministic pattern sharing
        let random_type = get_random(ctx.player_config.random as usize, model.mode);
        let seed: i64 = ghost_battle
            .as_ref()
            .map_or_else(rand::random, |gb| gb.random_seed);

        // Ghost battle lane_sequence: when the opponent's lane ordering is known,
        // adjust for MIRROR selection (reverse digits so mirror is applied on
        // top of the ghost's order).
        // Java parity: GhostBattlePlay reverses laneOrder digits for mirror.
        let ghost_lane_seq = ghost_battle
            .as_ref()
            .map(|gb| {
                if gb.lane_sequence != 0 && random_type == RandomType::Mirror {
                    reverse_lane_sequence(gb.lane_sequence)
                } else {
                    gb.lane_sequence
                }
            })
            .unwrap_or(0);

        if ghost_lane_seq != 0 {
            info!(
                lane_sequence = ghost_lane_seq,
                "Play: ghost battle using lane sequence"
            );
        }

        if random_trainer_active {
            // M2: Apply fixed lane order from random trainer
            apply_fixed_lane_order(&mut model, &ctx.resource.random_trainer_lane_order, 0);
            info!(
                lane_order = ?ctx.resource.random_trainer_lane_order,
                "Play: random trainer active, fixed lane order applied"
            );
        } else {
            self.assist += apply_pattern_modifier(
                &mut model,
                random_type,
                0,
                seed,
                ctx.player_config.hran_threshold_bpm,
            );
        }

        // DP: Apply 2P pattern + doubleoption (flip)
        if model.mode.player_count() > 1 {
            let random_type_2p = get_random(ctx.player_config.random2 as usize, model.mode);
            self.assist += apply_pattern_modifier(
                &mut model,
                random_type_2p,
                1,
                seed,
                ctx.player_config.hran_threshold_bpm,
            );
            apply_double_option(&mut model, ctx.player_config.doubleoption);
        }

        // SP Battle mode: doubleoption >= 2 converts SP to DP with battle shuffle
        // and optionally autoplay scratch (doubleoption == 3).
        // Java: BMSPlayer lines 331-351
        if model.mode.player_count() == 1 && ctx.player_config.doubleoption >= 2 {
            self.assist +=
                apply_double_option_with_autoplay(&mut model, ctx.player_config.doubleoption);
            self.lane_property = LaneProperty::new(model.mode);
        }

        // Apply 7-to-9 mode modifier (after lane shuffle, matching Java order)
        if ctx.player_config.seven_to_nine_pattern >= 1 && model.mode == PlayMode::Beat7K {
            let pattern = SevenToNinePattern::from_id(ctx.player_config.seven_to_nine_pattern);
            let seven_type = SevenToNineType::from_id(ctx.player_config.seven_to_nine_type);
            let mut mode_mod = ModeModifier::new(PlayMode::Beat7K, PlayMode::PopN9K)
                .with_pattern(pattern)
                .with_type(seven_type)
                .with_hran_threshold_bpm(ctx.player_config.hran_threshold_bpm as f64);
            mode_mod.modify(&mut model);
            self.assist += assist_to_i32(mode_mod.assist_level());
            self.lane_property = LaneProperty::new(model.mode);
            info!(
                pattern = ctx.player_config.seven_to_nine_pattern,
                "Play: applied 7-to-9 mode modifier"
            );
        }

        // M1: Frequency Trainer — scale chart tempo and timing.
        // Java: BMSPlayer.java lines 248-267
        let freq = ctx.resource.freq_trainer_freq;
        if freq > 0 && freq != 100 && !self.is_autoplay && !ctx.resource.is_course() {
            let freq_ratio = freq as f64 / 100.0;
            model.change_frequency(freq_ratio);
            ctx.resource.force_no_ir_send = true;
            info!(freq, "Play: frequency trainer active");
        }

        // M3: Judge Trainer — override chart's judge rank.
        // Java: BMSPlayer.java lines 283-295
        if ctx.resource.judge_trainer_active && !self.is_autoplay {
            // Transform UI rank (EASY=0, NORMAL=1, HARD=2, VERY_HARD=3) to
            // windowrule index (VERY_HARD=0, HARD=1, NORMAL=2, EASY=3)
            let window_rule_index = 3_i32.saturating_sub(ctx.resource.judge_trainer_rank);
            model.judge_rank = window_rule_index;
            self.assist = self.assist.max(2); // Judge trainer counts as assist >= 2
            info!(judge_rank = model.judge_rank, "Play: judge trainer active");
        }

        let rule = PlayerRule::lr2();
        self.judge_notes = model.build_judge_notes();

        let total_notes = self.judge_notes.iter().filter(|n| n.is_playable()).count();
        let total = if model.total > 0.0 {
            model.total
        } else {
            PlayerRule::default_total(total_notes)
        };

        // Determine gauge type from player config
        let gauge_type = gauge_type_from_i32(ctx.player_config.gauge);
        self.gauge_auto_shift = GaugeAutoShift::from_i32(ctx.player_config.gauge_auto_shift);
        self.bottom_gauge = gauge_type_from_i32(ctx.player_config.bottom_shiftable_gauge);

        let judge_rank = rule
            .judge
            .window_rule
            .resolve_judge_rank(model.judge_rank, model.judge_rank_type);

        // Judge window rates
        let (jwr, sjwr) = if ctx.player_config.custom_judge {
            (
                [
                    ctx.player_config.key_judge_window_rate_perfect_great,
                    ctx.player_config.key_judge_window_rate_great,
                    ctx.player_config.key_judge_window_rate_good,
                ],
                [
                    ctx.player_config.scratch_judge_window_rate_perfect_great,
                    ctx.player_config.scratch_judge_window_rate_great,
                    ctx.player_config.scratch_judge_window_rate_good,
                ],
            )
        } else {
            ([100, 100, 100], [100, 100, 100])
        };

        let config = JudgeConfig {
            notes: &self.judge_notes,
            play_mode: model.mode,
            ln_type: model.ln_type,
            judge_rank,
            judge_window_rate: jwr,
            scratch_judge_window_rate: sjwr,
            algorithm: DEFAULT_ALGORITHMS[0],
            autoplay: self.is_autoplay,
            judge_property: &rule.judge,
            lane_property: Some(&self.lane_property),
        };

        let mut jm = JudgeManager::new(&config);
        let mut gauge = GrooveGauge::new(&rule.gauge, gauge_type, total, total_notes);

        // Allocate key state arrays
        let phys_count = self.lane_property.physical_key_count();
        self.key_states = vec![false; phys_count];
        self.key_changed_times = vec![NOT_SET; phys_count];

        // Prime JudgeManager: set prev_time to -1 so notes at time_us=0 are not skipped.
        jm.update(
            -1,
            &self.judge_notes,
            &self.key_states,
            &self.key_changed_times,
            &mut gauge,
        );

        // Calculate playtime
        self.last_note_time_us = self
            .judge_notes
            .iter()
            .map(|n| n.time_us.max(n.end_time_us))
            .max()
            .unwrap_or(0);
        self.playtime_us = self.last_note_time_us + FINISH_MARGIN_US;

        // M4: Set initial judge timing offset from config
        jm.set_timing_offset(ctx.player_config.judgetiming as i64 * 1000);

        self.judge_manager = Some(jm);
        self.gauge = Some(gauge);

        // Initialize scratch angle state
        self.scratch_angle = ScratchAngleState::new(self.lane_property.scratch_count());

        // Store total notes in resource
        ctx.resource.score_data.notes = total_notes as i32;

        // Initialize BPM tracking
        self.min_bpm = model.min_bpm();
        self.max_bpm = model.max_bpm();
        self.main_bpm = model.main_bpm();
        self.now_bpm = model.initial_bpm;

        // L5: Initialize RhythmTimerProcessor for PMS note expansion
        let is_pms = matches!(model.mode, PlayMode::PopN5K | PlayMode::PopN9K);
        self.rhythm_timer = Some(super::rhythm_timer::RhythmTimerProcessor::new(
            &model, is_pms,
        ));

        // Initialize ScoreDataProperty for real-time score comparison
        let mut sdp = ScoreDataProperty::new();
        let oldscore = &ctx.resource.oldscore;
        let best_ghost = oldscore.decode_ghost();
        let target = TargetProperty::resolve(&ctx.player_config.targetid);
        let rival_scores = Self::build_rival_scores(ctx);
        let rival_scores_ref = if rival_scores.is_empty() && ctx.database.is_none() {
            None
        } else {
            Some(rival_scores.as_slice())
        };
        let target_ctx = TargetContext {
            total_notes: total_notes as i32,
            current_exscore: oldscore.exscore(),
            rival_scores: rival_scores_ref,
            ranking_data: ctx.resource.ranking_data.as_ref(),
        };
        let (target_exscore, _target_name) = target.compute_target(&target_ctx);
        sdp.set_target_score(
            oldscore.exscore(),
            best_ghost,
            target_exscore,
            None, // target ghost (static targets don't have ghost)
            total_notes as i32,
        );
        self.score_data_property = sdp;
    }
}

/// Apply a fixed lane order from the random trainer.
///
/// `lane_order` is 1-indexed (values 1-7 representing key lanes).
/// Scratch lane (0) is not remapped.
/// `player` selects which half of the key range to remap (0 = 1P, 1 = 2P).
fn apply_fixed_lane_order(model: &mut bms_model::BmsModel, lane_order: &[u8; 7], player: usize) {
    let mode = model.mode;
    let key_count = mode.key_count();
    let keys_per_player = key_count / mode.player_count().max(1);
    let base = player * keys_per_player;

    // Build mapping: mapping[old_lane] = new_lane
    let mut mapping: Vec<usize> = (0..key_count).collect();
    for (i, &src_lane) in lane_order.iter().enumerate() {
        if i >= keys_per_player || (src_lane as usize) < 1 || (src_lane as usize) > keys_per_player
        {
            continue;
        }
        // lane_order[i] = src (1-indexed) -> position i+1 (1-indexed)
        // In the model, lane 0 = scratch, lanes 1-7 = keys
        mapping[base + src_lane as usize] = base + i + 1;
    }

    for note in &mut model.notes {
        if note.lane < mapping.len() {
            note.lane = mapping[note.lane];
        }
    }
}

/// Reverse the digits of a lane_sequence value.
///
/// Lane sequence is encoded as a decimal integer where each digit (1-7)
/// represents a lane. Reversing produces the mirror permutation.
/// e.g., 1234567 → 7654321, 3142567 → 7652413
fn reverse_lane_sequence(seq: i32) -> i32 {
    let mut digits = Vec::new();
    let mut n = seq;
    while n > 0 {
        digits.push(n % 10);
        n /= 10;
    }
    let mut result = 0;
    for &d in &digits {
        result = result * 10 + d;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_lane_sequence_identity() {
        assert_eq!(reverse_lane_sequence(1234567), 7654321);
    }

    #[test]
    fn reverse_lane_sequence_mirror() {
        assert_eq!(reverse_lane_sequence(7654321), 1234567);
    }

    #[test]
    fn reverse_lane_sequence_arbitrary() {
        assert_eq!(reverse_lane_sequence(3142567), 7652413);
    }

    #[test]
    fn reverse_lane_sequence_zero() {
        assert_eq!(reverse_lane_sequence(0), 0);
    }
}

// Play state initialization — constructor and judge/gauge setup.

use tracing::{info, warn};

use bms_database::RivalDataAccessor;
use bms_database::score_data_property::ScoreDataProperty;
use bms_model::{LaneProperty, PlayMode};
use bms_pattern::{ModeModifier, PatternModifier, SevenToNinePattern, SevenToNineType, get_random};
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

        // Apply pre-shuffle modifiers (scroll, longnote, mine, extranote)
        // Java: applied before lane shuffle, config value > 0 means active
        // Java offsets config values by -1 (e.g., ScrollMode 1 -> enum index 0)
        self.assist += apply_pre_shuffle_modifiers(&mut model, ctx.player_config);

        // Apply 1P pattern shuffle
        let random_type = get_random(ctx.player_config.random as usize, model.mode);
        let seed: i64 = rand::random();
        self.assist += apply_pattern_modifier(
            &mut model,
            random_type,
            0,
            seed,
            ctx.player_config.hran_threshold_bpm,
        );

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

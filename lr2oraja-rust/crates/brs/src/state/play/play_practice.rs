// Practice mode — apply practice settings and reinitialize judge/gauge.

use tracing::{info, warn};

use std::path::Path;

use bms_audio::driver::AudioDriver;
use bms_audio::key_sound::KeySoundProcessor;
use bms_audio::kira_driver::KiraAudioDriver;
use bms_database::score_data_property::ScoreDataProperty;
use bms_model::{JudgeRankType, LaneProperty};
use bms_pattern::{PatternModifier, PracticeModifier, get_random};
use bms_render::bga::bga_processor::BgaProcessor;
use bms_rule::judge_algorithm::DEFAULT_ALGORITHMS;
use bms_rule::judge_manager::{JudgeConfig, JudgeManager};
use bms_rule::{GrooveGauge, PlayerRule};

use crate::state::StateContext;

use super::play_skin_state::ScratchAngleState;
use super::{
    FINISH_MARGIN_US, GaugeAutoShift, NOT_SET, PlayState, apply_double_option,
    apply_pattern_modifier, gauge_type_from_i32,
};

impl PlayState {
    /// Apply practice settings to the BMS model and reinitialize judge/gauge.
    ///
    /// Ported from Java BMSPlayer.java lines 684-722.
    /// Called when the user presses the play key in the practice menu.
    /// Unlike `init_judge_and_gauge`, this applies practice-specific modifiers
    /// (freq, time range, practice random) instead of the normal config modifiers.
    pub(super) fn apply_practice_settings(&mut self, ctx: &mut StateContext) {
        let property = match &self.practice_config {
            Some(pc) => pc.property.clone(),
            None => return,
        };

        // Reload BMS model to get a fresh copy
        if let Err(e) = ctx.resource.reload_bms() {
            warn!("Play: practice apply_settings reload_bms failed: {e}");
            return;
        }

        // Clone the fresh model for modification
        let mut model = match &ctx.resource.bms_model {
            Some(m) => m.clone(),
            None => return,
        };

        // Apply frequency change
        if property.freq != 100 {
            let freq_ratio = property.freq as f64 / 100.0;
            model.change_frequency(freq_ratio);
        }

        // Override total and judge_rank from practice settings
        model.total = property.total;
        model.judge_rank = property.judgerank;
        model.judge_rank_type = JudgeRankType::BmsonJudgeRank;

        // Apply PracticeModifier: move notes outside the selected time range
        // Times are scaled by freq (Java: starttime * 100 / freq)
        let start_ms = (property.starttime as i64) * 100 / (property.freq as i64);
        let end_ms = (property.endtime as i64) * 100 / (property.freq as i64);
        let mut practice_mod = PracticeModifier::new(start_ms, end_ms);
        practice_mod.modify(&mut model);

        self.lane_property = LaneProperty::new(model.mode);

        // Apply pattern modifiers from practice settings (not player config)
        let seed: i64 = rand::random();
        self.assist = 0;
        let random_type = get_random(property.random as usize, model.mode);
        self.assist += apply_pattern_modifier(
            &mut model,
            random_type,
            0,
            seed,
            ctx.player_config.hran_threshold_bpm,
        );

        if model.mode.player_count() > 1 {
            let random_type_2p = get_random(property.random2 as usize, model.mode);
            self.assist += apply_pattern_modifier(
                &mut model,
                random_type_2p,
                1,
                seed,
                ctx.player_config.hran_threshold_bpm,
            );
            apply_double_option(&mut model, property.doubleop);
        }

        // Build judge notes and initialize gauge from modified model
        let rule = PlayerRule::lr2();
        self.judge_notes = model.build_judge_notes();

        let total_notes = self.judge_notes.iter().filter(|n| n.is_playable()).count();
        let total = if model.total > 0.0 {
            model.total
        } else {
            PlayerRule::default_total(total_notes)
        };

        let judge_rank = rule
            .judge
            .window_rule
            .resolve_judge_rank(model.judge_rank, model.judge_rank_type);

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

        // Practice gauge: use practice settings instead of player config
        let gauge_type = gauge_type_from_i32(property.gaugetype);
        let mut gauge = GrooveGauge::new(&rule.gauge, gauge_type, total, total_notes);
        gauge.set_value(property.startgauge as f32);
        // Practice: no auto-shift
        self.gauge_auto_shift = GaugeAutoShift::Continue;

        // Allocate key state arrays
        let phys_count = self.lane_property.physical_key_count();
        self.key_states = vec![false; phys_count];
        self.key_changed_times = vec![NOT_SET; phys_count];

        // Prime JudgeManager
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

        ctx.resource.score_data.notes = total_notes as i32;

        // BPM tracking
        self.min_bpm = model.min_bpm();
        self.max_bpm = model.max_bpm();
        self.main_bpm = model.main_bpm();
        self.now_bpm = model.initial_bpm;

        // Practice mode: no target score comparison
        self.score_data_property = ScoreDataProperty::new();
        ctx.resource.update_score = false;

        // Reset gauge log for new practice loop
        self.gauge_log.clear();
        self.last_gauge_log_time_us = 0;
        self.key_beam_stop = false;
        self.is_judge_started = false;

        // Reinitialize audio for the modified model
        if let Some(driver) = &mut self.audio_driver {
            driver.stop_all();
        }
        let base_path = ctx.resource.bms_dir.as_deref().unwrap_or(Path::new("."));
        match KiraAudioDriver::new() {
            Ok(mut driver) => {
                if let Err(e) = driver.set_model(&model, base_path) {
                    warn!("Play: practice audio reload failed: {e}");
                }
                self.key_sound_processor = Some(KeySoundProcessor::new(&model, 1.0));
                self.audio_driver = Some(Box::new(driver));
            }
            Err(e) => {
                warn!("Play: practice audio driver creation failed: {e}");
            }
        }

        // Initialize BGA processor
        self.bga_processor = Some(BgaProcessor::new(&model));

        info!(
            freq = property.freq,
            start_ms, end_ms, "Play: practice settings applied"
        );
    }
}

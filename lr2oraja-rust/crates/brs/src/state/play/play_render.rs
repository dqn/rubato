// Play render — Playing phase rendering and score data building.

use tracing::{info, warn};

use bms_rule::{ClearType, JUDGE_BD, JUDGE_MS, JUDGE_PR};
use bms_skin::property_id::{
    TIMER_ENDOFNOTE_1P, TIMER_FULLCOMBO_1P, TIMER_GAUGE_MAX_1P, TIMER_MUSIC_END, TIMER_PLAY,
};

use crate::state::StateContext;

use super::{PlayPhase, PlayState};

impl PlayState {
    /// Handle the Playing phase render logic (timer-driven state checks).
    pub(super) fn render_playing(&mut self, ctx: &mut StateContext) {
        let ptime_us = ctx.timer.now_time_of(TIMER_PLAY) * 1000;

        // Update current BPM at this time position
        if let Some(model) = &ctx.resource.bms_model {
            self.now_bpm = model.bpm_at(ptime_us);
        }

        // Update ScoreDataProperty
        if let Some(jm) = &self.judge_manager {
            self.score_data_property.update(jm.score(), jm.past_notes());
        }

        // Update BGA timeline and movie frames
        if let Some(bga) = &mut self.bga_processor {
            bga.update(ptime_us);
            if let Some(images) = &mut ctx.bevy_images {
                bga.update_movie_frames(images);
            }

            // Sync BGA image handles to shared game state for skin rendering
            if let Some(shared) = &mut ctx.shared_state {
                shared.bga_image = bga.get_bga_image().cloned();
                shared.layer_image = bga.get_layer_image().cloned();
                shared.poor_image = bga.get_poor_image().cloned();
                shared.poor_active = bga.is_poor_active();
            }
        }

        // BGM autoplay via KeySoundProcessor
        if let (Some(ksp), Some(driver)) = (&mut self.key_sound_processor, &mut self.audio_driver) {
            ksp.update(ptime_us, driver.as_mut());
        }

        // Audio driver recovery: recreate AudioManager after consecutive failures
        if let Some(driver) = &mut self.audio_driver
            && driver.needs_recovery()
            && let Err(e) = driver.try_recover()
        {
            warn!("Audio recovery failed: {e}");
        }

        // Record gauge log every 500ms
        self.record_gauge_log(ptime_us);

        // Check gauge death
        if let Some(gauge) = &self.gauge
            && gauge.value() <= 0.0
            && !self.handle_gauge_death(ctx)
        {
            return; // Transitioned to Failed
        }

        // Update gauge-related timers
        if let Some(gauge) = &self.gauge {
            ctx.timer
                .switch_timer(TIMER_GAUGE_MAX_1P, gauge.active_gauge().is_max());
        }

        // Check end-of-notes
        ctx.timer
            .switch_timer(TIMER_ENDOFNOTE_1P, ptime_us > self.last_note_time_us);

        // Check fullcombo (no combo-breaking judgments)
        if let Some(jm) = &self.judge_manager {
            let score = jm.score();
            let judged = score.total_judge_count();
            if judged >= score.notes
                && score.judge_count(JUDGE_BD) == 0
                && score.judge_count(JUDGE_PR) == 0
                && score.judge_count(JUDGE_MS) == 0
            {
                ctx.timer.switch_timer(TIMER_FULLCOMBO_1P, true);
            }
        }

        // Check if play is finished
        if ptime_us >= self.playtime_us {
            self.phase = PlayPhase::Finished;
            ctx.timer.set_timer_on(TIMER_MUSIC_END);
            info!("Play: finished (all notes processed)");
        }
    }

    /// Build score data and save to resource for Result state.
    pub(super) fn build_score_data(&self, ctx: &mut StateContext) {
        if let Some(jm) = &self.judge_manager {
            let mut score = jm.score().clone();
            score.maxcombo = jm.max_combo();

            if let Some(gauge) = &self.gauge {
                score.clear = if gauge.is_qualified() {
                    ClearType::from_gauge_type(gauge.active_type() as usize)
                        .unwrap_or(ClearType::Normal)
                } else {
                    ClearType::Failed
                };
                score.gauge = gauge.active_type() as i32;
            }

            if let Some(model) = &ctx.resource.bms_model {
                score.sha256 = model.sha256.clone();
                score.mode = model.mode.mode_id();
            }

            score.minbp = score.judge_count(JUDGE_BD)
                + score.judge_count(JUDGE_PR)
                + score.judge_count(JUDGE_MS);
            score.assist = self.assist;

            // H6: Assist level overrides clear type
            // Java: MusicResult — assist >= 2 → AssistEasy, assist >= 1 → LightAssistEasy
            if score.clear != ClearType::Failed {
                if self.assist >= 2 {
                    score.clear = ClearType::AssistEasy;
                } else if self.assist >= 1 {
                    score.clear = ClearType::LightAssistEasy;
                }
            }

            ctx.resource.score_data = score;
        }

        ctx.resource.gauge_log = self.gauge_log.clone();
        ctx.resource.maxcombo = self.judge_manager.as_ref().map_or(0, |jm| jm.max_combo());
        ctx.resource.update_score = !self.is_autoplay && !self.is_replay;

        // Save target/rival EX score for result comparison (H8)
        ctx.resource.target_exscore = Some(self.score_data_property.rival_score());
    }
}

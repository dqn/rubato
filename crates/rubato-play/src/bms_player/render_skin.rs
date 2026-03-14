use super::skin_context::PlayRenderContext;
use super::*;

impl BMSPlayer {
    pub(super) fn render_skin_impl(
        &mut self,
        sprite: &mut rubato_render::sprite_batch::SpriteBatch,
    ) {
        let mut skin = match self.main_state_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.main_state_data.timer);

        // Compute note draw commands via the type-erased SkinDrawable bridge.
        // This calls LaneRenderer::draw_lane() inside the skin to populate SkinNoteObject.draw_commands.
        if let Some(ref mut lr) = self.lanerender {
            let lane_count = self.model.mode().map_or(8, |m| m.key() as usize);
            // Safety: DrawLaneContext is consumed synchronously within compute_note_draw_commands.
            // self.model.timelines outlives the context because the function returns before
            // self is accessed again.
            let all_timelines: &'static [bms_model::time_line::TimeLine] =
                unsafe { std::mem::transmute(self.model.timelines.as_slice()) };
            let judge_table = self.judge.judge_table(false);
            let bad_judge_time = judge_table.get(3).map_or(0, |jt| jt[1]);
            let draw_ctx = crate::lane_renderer::DrawLaneContext {
                time: timer.now_time(),
                timer_play: if timer.is_timer_on(TIMER_PLAY) {
                    Some(timer.now_time_for_id(TIMER_PLAY))
                } else {
                    None
                },
                timer_141: if timer.is_timer_on(TimerId::new(141)) {
                    Some(timer.now_time_for_id(TimerId::new(141)))
                } else {
                    None
                },
                judge_timing: self.player_config.judge_settings.judgetiming as i64,
                is_practice: self.state == PlayState::Practice
                    || self.state == PlayState::PracticeFinished,
                practice_start_time: self.practice.practice_property().starttime as i64,
                now_time: timer.now_time(),
                now_quarter_note_time: self
                    .rhythm
                    .as_ref()
                    .map_or(0, |r| r.now_quarter_note_time()),
                note_expansion_rate: self.play_skin.note_expansion_rate,
                lane_group_regions: Vec::new(),
                show_bpmguide: self.player_config.display_settings.bpmguide,
                show_pastnote: self.player_config.display_settings.showpastnote,
                mark_processednote: self.player_config.display_settings.markprocessednote,
                show_hiddennote: self.player_config.display_settings.showhiddennote,
                show_judgearea: self.player_config.display_settings.showjudgearea,
                lntype: self.model.lntype(),
                judge_time_regions: (0..lane_count)
                    .map(|i| self.judge.judge_time_region(i).to_vec())
                    .collect(),
                processing_long_notes: (0..lane_count)
                    .map(|i| self.judge.processing_long_note(i))
                    .collect(),
                passing_long_notes: (0..lane_count)
                    .map(|i| self.judge.passing_long_note(i))
                    .collect(),
                hell_charge_judges: (0..lane_count)
                    .map(|i| self.judge.hell_charge_judge(i))
                    .collect(),
                bad_judge_time,
                model_bpm: self.model.bpm,
                all_timelines,
                forced_cn_endings: false,
            };
            skin.compute_note_draw_commands(lr, Box::new(draw_ctx));
        }

        {
            let lr_ref = self.lanerender.as_ref();
            let mut ctx = PlayRenderContext {
                timer: &mut timer,
                judge: &self.judge,
                gauge: self.gauge.as_ref(),
                player_config: &self.player_config,
                option_info: &self.score.playinfo,
                play_config: &self
                    .player_config
                    .play_config_ref(
                        self.model
                            .mode()
                            .cloned()
                            .unwrap_or(bms_model::mode::Mode::BEAT_7K),
                    )
                    .playconfig,
                target_score: self.score.target_score.as_ref(),
                playtime: self.playtime,
                total_notes: self.total_notes,
                play_mode: self.play_mode,
                state: self.state,
                media_load_finished: self.media_load_finished,
                now_bpm: lr_ref.map_or(0.0, |lr| lr.now_bpm()),
                min_bpm: lr_ref.map_or(0.0, |lr| lr.min_bpm()),
                max_bpm: lr_ref.map_or(0.0, |lr| lr.max_bpm()),
                main_bpm: lr_ref.map_or(0.0, |lr| lr.main_bpm()),
                system_volume: self.system_volume,
                key_volume: self.key_volume,
                bg_volume: self.bg_volume,
                is_mode_changed: self.orgmode.is_some_and(|org| {
                    self.model
                        .mode()
                        .copied()
                        .unwrap_or(bms_model::mode::Mode::BEAT_7K)
                        != org
                }),
                lnmode_override: self.lnmode_override,
            };
            skin.update_custom_objects_timed(&mut ctx);
            skin.swap_sprite_batch(sprite);
            skin.draw_all_objects_timed(&mut ctx);
            skin.swap_sprite_batch(sprite);
        }

        self.main_state_data.timer = timer;
        self.main_state_data.skin = Some(skin);
    }
}

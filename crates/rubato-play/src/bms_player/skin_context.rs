use super::*;

/// Render context adapter for BMSPlayer skin rendering.
/// Provides real gameplay data (judge, gauge, combo) through SkinRenderContext
/// so skin objects can query live state during the draw cycle.
pub(super) struct PlayRenderContext<'a> {
    pub(super) timer: &'a mut TimerManager,
    pub(super) judge: &'a JudgeManager,
    pub(super) gauge: Option<&'a GrooveGauge>,
    pub(super) player_config: &'a PlayerConfig,
    pub(super) option_info: &'a ReplayData,
    pub(super) play_config: &'a PlayConfig,
    pub(super) target_score: Option<&'a ScoreData>,
    pub(super) playtime: i32,
    pub(super) total_notes: i32,
    pub(super) play_mode: BMSPlayerMode,
    pub(super) state: PlayState,
    pub(super) media_load_finished: bool,
}

impl rubato_types::timer_access::TimerAccess for PlayRenderContext<'_> {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }
    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for_id(timer_id)
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinEventHandler for PlayRenderContext<'_> {
    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }
}

impl rubato_types::skin_render_context::SkinAudioControl for PlayRenderContext<'_> {}

impl rubato_types::skin_render_context::SkinStateQuery for PlayRenderContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Play)
    }

    fn now_judge(&self, player: i32) -> i32 {
        self.judge.now_judge(player as usize)
    }

    fn now_combo(&self, player: i32) -> i32 {
        self.judge.now_combo(player as usize)
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        self.judge.judge_count_fast(judge, fast)
    }

    fn gauge_value(&self) -> f32 {
        self.gauge.map_or(0.0, |g| g.value())
    }

    fn gauge_type(&self) -> i32 {
        self.gauge.map_or(0, |g| g.gauge_type())
    }

    fn recent_judges(&self) -> &[i64] {
        rubato_types::skin_render_context::SkinStateQuery::recent_judges(self.timer)
    }

    fn recent_judges_index(&self) -> usize {
        rubato_types::skin_render_context::SkinStateQuery::recent_judges_index(self.timer)
    }
}

impl rubato_types::skin_render_context::SkinConfigAccess for PlayRenderContext<'_> {
    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(self.player_config)
    }
}

impl rubato_types::skin_render_context::SkinPropertyProvider for PlayRenderContext<'_> {
    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        Some(self.option_info)
    }

    fn target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.target_score
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        Some(self.play_config)
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            // Total notes
            350 => self.total_notes,
            // Playtime (hours/minutes/seconds from boot)
            17 => (self.timer.now_time() / 3_600_000) as i32,
            18 => ((self.timer.now_time() % 3_600_000) / 60_000) as i32,
            19 => ((self.timer.now_time() % 60_000) / 1_000) as i32,
            // Song duration
            312 => self.playtime,
            1163 => self.playtime / 60,
            1164 => self.playtime % 60,
            // Loading progress: 100 if media loaded, else 0
            165 => {
                if self.media_load_finished {
                    100
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn float_value(&self, id: i32) -> f32 {
        match id {
            // Gauge value (0.0-1.0)
            1107 => self.gauge.map_or(0.0, |g| g.value()),
            // Hi-speed
            310 => self.player_config.mode7.playconfig.hispeed,
            _ => 0.0,
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        match id {
            // Autoplay mode
            200 => {
                self.play_mode.mode == rubato_core::bms_player_mode::Mode::Autoplay
                    || self.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay
            }
            // Practice mode
            201 => self.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice,
            // Loading state (PlayState::Preload = 0)
            80 => self.state == PlayState::Preload,
            _ => false,
        }
    }
}

pub(super) struct PlayMouseContext<'a> {
    pub(super) timer: &'a mut TimerManager,
    pub(super) player: &'a mut BMSPlayer,
}

impl rubato_types::timer_access::TimerAccess for PlayMouseContext<'_> {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }

    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }

    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }

    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }

    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for_id(timer_id)
    }

    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinEventHandler for PlayMouseContext<'_> {
    fn change_state(&mut self, state: rubato_types::main_state_type::MainStateType) {
        self.player.pending.pending_state_change = Some(state);
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }
}

impl rubato_types::skin_render_context::SkinAudioControl for PlayMouseContext<'_> {}

impl rubato_types::skin_render_context::SkinStateQuery for PlayMouseContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Play)
    }
}

impl rubato_types::skin_render_context::SkinPropertyProvider for PlayMouseContext<'_> {}

impl rubato_types::skin_render_context::SkinConfigAccess for PlayMouseContext<'_> {
    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        Some(&mut self.player.player_config)
    }
}

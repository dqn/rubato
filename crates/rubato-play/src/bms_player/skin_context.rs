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
    /// BPM values from LaneRenderer for skin property display.
    pub(super) now_bpm: f64,
    pub(super) min_bpm: f64,
    pub(super) max_bpm: f64,
    pub(super) main_bpm: f64,
    /// Volume values from AudioConfig for skin property display.
    pub(super) system_volume: f32,
    pub(super) key_volume: f32,
    pub(super) bg_volume: f32,
    /// Whether the chart's original mode differs from the current mode
    /// (e.g. 7-key chart converted to 9-key via chart options).
    pub(super) is_mode_changed: bool,
    /// Pre-computed lnmode override from chart data (SongData).
    /// When the chart explicitly defines LN types, this overrides the config setting
    /// for image_index_value ID 308.
    pub(super) lnmode_override: Option<i32>,
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

impl rubato_types::skin_render_context::SkinRenderContext for PlayRenderContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Play)
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(self.player_config)
    }

    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        Some(self.option_info)
    }

    fn target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.target_score
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        Some(self.play_config)
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn now_judge(&self, player: i32) -> i32 {
        self.judge.now_judge(player.max(0) as usize)
    }

    fn now_combo(&self, player: i32) -> i32 {
        self.judge.now_combo(player.max(0) as usize)
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

    fn is_mode_changed(&self) -> bool {
        self.is_mode_changed
    }

    fn gauge_element_borders(&self) -> Vec<(f32, f32)> {
        match self.gauge {
            Some(g) => (0..g.gauge_type_length())
                .map(|i| {
                    let prop = g.gauge_by_type(i as i32).property();
                    (prop.border, prop.max)
                })
                .collect(),
            None => Vec::new(),
        }
    }

    fn recent_judges(&self) -> &[i64] {
        rubato_types::skin_render_context::SkinRenderContext::recent_judges(self.timer)
    }

    fn recent_judges_index(&self) -> usize {
        rubato_types::skin_render_context::SkinRenderContext::recent_judges_index(self.timer)
    }

    fn lane_shuffle_pattern_value(&self, player: usize, lane: usize) -> i32 {
        self.option_info
            .lane_shuffle_pattern
            .as_ref()
            .and_then(|patterns| patterns.get(player))
            .and_then(|lanes| lanes.get(lane))
            .copied()
            .unwrap_or(-1)
    }

    fn image_index_value(&self, id: i32) -> i32 {
        match id {
            // Java IntegerPropertyFactory ID 308 (lnmode): on BMSPlayer, override
            // from chart data when the chart explicitly defines LN types.
            308 => {
                if let Some(override_val) = self.lnmode_override {
                    return override_val;
                }
                self.default_image_index_value(id)
            }
            _ => self.default_image_index_value(id),
        }
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            // Total notes
            350 => self.total_notes,
            // Playtime (hours/minutes/seconds from boot)
            17 => (self.timer.now_time() / 3_600_000) as i32,
            18 => ((self.timer.now_time() % 3_600_000) / 60_000) as i32,
            19 => ((self.timer.now_time() % 60_000) / 1_000) as i32,
            // Volume (0-100 scale)
            57 => (self.system_volume * 100.0) as i32,
            58 => (self.key_volume * 100.0) as i32,
            59 => (self.bg_volume * 100.0) as i32,
            // BPM
            90 => self.max_bpm as i32,
            91 => self.min_bpm as i32,
            92 => self.main_bpm as i32,
            160 => self.now_bpm as i32,
            // Song duration
            312 => self.playtime,
            1163 => self.playtime / 60000,
            1164 => (self.playtime % 60000) / 1000,
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
            // Volume (0.0-1.0)
            17 => self.system_volume,
            18 => self.key_volume,
            19 => self.bg_volume,
            // Loading progress (0.0-1.0)
            165 => {
                if self.media_load_finished {
                    1.0
                } else {
                    0.0
                }
            }
            // Gauge value (0.0-100.0)
            1107 => self.gauge.map_or(0.0, |g| g.value()),
            // Hi-speed (from active play config, not always mode7)
            310 => self.play_config.hispeed,
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

impl rubato_types::skin_render_context::SkinRenderContext for PlayMouseContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Play)
    }

    fn change_state(&mut self, state: rubato_types::main_state_type::MainStateType) {
        self.player.pending.pending_state_change = Some(state);
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(&self.player.player_config)
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        Some(&mut self.player.player_config)
    }

    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        Some(&self.player.score.playinfo)
    }

    fn target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.player.score.target_score.as_ref()
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        Some(
            &self
                .player
                .player_config
                .play_config_ref(
                    self.player
                        .model
                        .mode()
                        .cloned()
                        .unwrap_or(bms_model::mode::Mode::BEAT_7K),
                )
                .playconfig,
        )
    }

    fn now_judge(&self, player: i32) -> i32 {
        self.player.judge.now_judge(player.max(0) as usize)
    }

    fn now_combo(&self, player: i32) -> i32 {
        self.player.judge.now_combo(player.max(0) as usize)
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        self.player.judge.judge_count_fast(judge, fast)
    }

    fn gauge_value(&self) -> f32 {
        self.player.gauge.as_ref().map_or(0.0, |g| g.value())
    }

    fn gauge_type(&self) -> i32 {
        self.player.gauge.as_ref().map_or(0, |g| g.gauge_type())
    }

    fn recent_judges(&self) -> &[i64] {
        rubato_types::skin_render_context::SkinRenderContext::recent_judges(self.timer)
    }

    fn recent_judges_index(&self) -> usize {
        rubato_types::skin_render_context::SkinRenderContext::recent_judges_index(self.timer)
    }

    fn image_index_value(&self, id: i32) -> i32 {
        match id {
            // Java IntegerPropertyFactory ID 308 (lnmode): on BMSPlayer, override
            // from chart data when the chart explicitly defines LN types.
            308 => {
                if let Some(override_val) = self.player.lnmode_override {
                    return override_val;
                }
                self.default_image_index_value(id)
            }
            _ => self.default_image_index_value(id),
        }
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            350 => self.player.total_notes,
            17 => (self.timer.now_time() / 3_600_000) as i32,
            18 => ((self.timer.now_time() % 3_600_000) / 60_000) as i32,
            19 => ((self.timer.now_time() % 60_000) / 1_000) as i32,
            // Volume (0-100 scale)
            57 => (self.player.system_volume * 100.0) as i32,
            58 => (self.player.key_volume * 100.0) as i32,
            59 => (self.player.bg_volume * 100.0) as i32,
            // BPM
            90 => self
                .player
                .lanerender
                .as_ref()
                .map_or(0, |lr| lr.max_bpm() as i32),
            91 => self
                .player
                .lanerender
                .as_ref()
                .map_or(0, |lr| lr.min_bpm() as i32),
            92 => self
                .player
                .lanerender
                .as_ref()
                .map_or(0, |lr| lr.main_bpm() as i32),
            160 => self
                .player
                .lanerender
                .as_ref()
                .map_or(0, |lr| lr.now_bpm() as i32),
            312 => self.player.playtime,
            1163 => self.player.playtime / 60000,
            1164 => (self.player.playtime % 60000) / 1000,
            165 => {
                if self.player.media_load_finished {
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
            // Volume (0.0-1.0)
            17 => self.player.system_volume,
            18 => self.player.key_volume,
            19 => self.player.bg_volume,
            // Loading progress (0.0-1.0)
            165 => {
                if self.player.media_load_finished {
                    1.0
                } else {
                    0.0
                }
            }
            1107 => self.player.gauge.as_ref().map_or(0.0, |g| g.value()),
            310 => {
                self.player
                    .player_config
                    .play_config_ref(
                        self.player
                            .model
                            .mode()
                            .cloned()
                            .unwrap_or(bms_model::mode::Mode::BEAT_7K),
                    )
                    .playconfig
                    .hispeed
            }
            _ => 0.0,
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        match id {
            200 => {
                self.player.play_mode.mode == rubato_core::bms_player_mode::Mode::Autoplay
                    || self.player.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay
            }
            201 => self.player.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice,
            80 => self.player.state == PlayState::Preload,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::skin_render_context::SkinRenderContext;

    /// Build a minimal PlayRenderContext with the given playtime (in ms).
    fn make_render_ctx(playtime: i32) -> PlayRenderContext<'static> {
        // Use Box::leak for test-only references so we get 'static lifetimes.
        let timer = Box::leak(Box::new(TimerManager::new()));
        let judge = Box::leak(Box::new(JudgeManager::default()));
        let player_config = Box::leak(Box::new(PlayerConfig::default()));
        let option_info = Box::leak(Box::new(ReplayData::default()));
        let play_config = Box::leak(Box::new(PlayConfig::default()));
        PlayRenderContext {
            timer,
            judge,
            gauge: None,
            player_config,
            option_info,
            play_config,
            target_score: None,
            playtime,
            total_notes: 0,
            play_mode: BMSPlayerMode::new(rubato_core::bms_player_mode::Mode::Play),
            state: PlayState::Play,
            media_load_finished: false,
            now_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            system_volume: 0.0,
            key_volume: 0.0,
            bg_volume: 0.0,
            is_mode_changed: false,
            lnmode_override: None,
        }
    }

    #[test]
    fn playtime_minutes_and_seconds_from_milliseconds() {
        // 150_000 ms = 2 minutes 30 seconds
        let ctx = make_render_ctx(150_000);
        assert_eq!(ctx.integer_value(1163), 2, "ID 1163 should return minutes");
        assert_eq!(ctx.integer_value(1164), 30, "ID 1164 should return seconds");
    }

    #[test]
    fn playtime_exactly_one_minute() {
        let ctx = make_render_ctx(60_000);
        assert_eq!(ctx.integer_value(1163), 1);
        assert_eq!(ctx.integer_value(1164), 0);
    }

    #[test]
    fn playtime_zero() {
        let ctx = make_render_ctx(0);
        assert_eq!(ctx.integer_value(1163), 0);
        assert_eq!(ctx.integer_value(1164), 0);
    }

    #[test]
    fn playtime_sub_second_truncated() {
        // 61_999 ms = 1 min 1.999 sec -> minutes=1, seconds=1 (truncated)
        let ctx = make_render_ctx(61_999);
        assert_eq!(ctx.integer_value(1163), 1);
        assert_eq!(ctx.integer_value(1164), 1);
    }

    #[test]
    fn playtime_raw_ms_unchanged() {
        let ctx = make_render_ctx(123_456);
        assert_eq!(
            ctx.integer_value(312),
            123_456,
            "ID 312 should return raw ms"
        );
    }

    #[test]
    fn playtime_large_value() {
        // 7_200_000 ms = 120 minutes
        let ctx = make_render_ctx(7_200_000);
        assert_eq!(ctx.integer_value(1163), 120);
        assert_eq!(ctx.integer_value(1164), 0);
    }

    fn make_render_ctx_with_pattern(pattern: Option<Vec<Vec<i32>>>) -> PlayRenderContext<'static> {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let judge = Box::leak(Box::new(JudgeManager::default()));
        let player_config = Box::leak(Box::new(PlayerConfig::default()));
        let option_info = Box::leak(Box::new(ReplayData {
            lane_shuffle_pattern: pattern,
            ..ReplayData::default()
        }));
        let play_config = Box::leak(Box::new(PlayConfig::default()));
        PlayRenderContext {
            timer,
            judge,
            gauge: None,
            player_config,
            option_info,
            play_config,
            target_score: None,
            playtime: 0,
            total_notes: 0,
            play_mode: BMSPlayerMode::new(rubato_core::bms_player_mode::Mode::Play),
            state: PlayState::Play,
            media_load_finished: false,
            now_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            system_volume: 0.0,
            key_volume: 0.0,
            bg_volume: 0.0,
            is_mode_changed: false,
            lnmode_override: None,
        }
    }

    #[test]
    fn lane_shuffle_pattern_1p_returns_source_lane() {
        let ctx = make_render_ctx_with_pattern(Some(vec![vec![2, 0, 1, 3, 4, 5, 6, 7, 8, 9]]));
        // ID 450 = 1P lane 0 -> source lane 2
        assert_eq!(ctx.image_index_value(450), 2);
        // ID 451 = 1P lane 1 -> source lane 0
        assert_eq!(ctx.image_index_value(451), 0);
        // ID 452 = 1P lane 2 -> source lane 1
        assert_eq!(ctx.image_index_value(452), 1);
    }

    #[test]
    fn lane_shuffle_pattern_2p_returns_source_lane() {
        let ctx = make_render_ctx_with_pattern(Some(vec![
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            vec![5, 4, 3, 2, 1, 0, 6, 7, 8, 9],
        ]));
        // ID 460 = 2P lane 0 -> source lane 5
        assert_eq!(ctx.image_index_value(460), 5);
        // ID 461 = 2P lane 1 -> source lane 4
        assert_eq!(ctx.image_index_value(461), 4);
    }

    #[test]
    fn lane_shuffle_pattern_none_returns_minus_one() {
        let ctx = make_render_ctx_with_pattern(None);
        assert_eq!(ctx.image_index_value(450), -1);
        assert_eq!(ctx.image_index_value(460), -1);
    }

    #[test]
    fn lane_shuffle_pattern_out_of_range_returns_minus_one() {
        let ctx = make_render_ctx_with_pattern(Some(vec![vec![0, 1, 2]]));
        // Lane index 3 is out of range for a 3-element pattern
        assert_eq!(ctx.image_index_value(453), -1);
        // 2P not provided
        assert_eq!(ctx.image_index_value(460), -1);
    }

    #[test]
    fn lane_shuffle_pattern_scratch_1p() {
        let ctx = make_render_ctx_with_pattern(Some(vec![vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 42]]));
        // ID 459 = 1P scratch (lane index 9) -> source lane 42
        assert_eq!(ctx.image_index_value(459), 42);
    }

    // ============================================================
    // lnmode (ID 308) image_index_value override tests
    // ============================================================

    fn make_render_ctx_with_lnmode(lnmode_override: Option<i32>) -> PlayRenderContext<'static> {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let judge = Box::leak(Box::new(JudgeManager::default()));
        let player_config = Box::leak(Box::new(PlayerConfig {
            play_settings: rubato_types::player_config::PlaySettings {
                lnmode: 99, // sentinel value to detect fallback
                ..Default::default()
            },
            ..PlayerConfig::default()
        }));
        let option_info = Box::leak(Box::new(ReplayData::default()));
        let play_config = Box::leak(Box::new(PlayConfig::default()));
        PlayRenderContext {
            timer,
            judge,
            gauge: None,
            player_config,
            option_info,
            play_config,
            target_score: None,
            playtime: 0,
            total_notes: 0,
            play_mode: BMSPlayerMode::new(rubato_core::bms_player_mode::Mode::Play),
            state: PlayState::Play,
            media_load_finished: false,
            now_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            system_volume: 0.0,
            key_volume: 0.0,
            bg_volume: 0.0,
            is_mode_changed: false,
            lnmode_override,
        }
    }

    #[test]
    fn lnmode_308_override_longnote() {
        // Chart defines LN -> override returns 0
        let ctx = make_render_ctx_with_lnmode(Some(0));
        assert_eq!(ctx.image_index_value(308), 0);
    }

    #[test]
    fn lnmode_308_override_chargenote() {
        // Chart defines CN -> override returns 1
        let ctx = make_render_ctx_with_lnmode(Some(1));
        assert_eq!(ctx.image_index_value(308), 1);
    }

    #[test]
    fn lnmode_308_override_hellchargenote() {
        // Chart defines HCN -> override returns 2
        let ctx = make_render_ctx_with_lnmode(Some(2));
        assert_eq!(ctx.image_index_value(308), 2);
    }

    #[test]
    fn lnmode_308_no_override_falls_through_to_config() {
        // No chart override -> falls through to player_config.play_settings.lnmode (99)
        let ctx = make_render_ctx_with_lnmode(None);
        assert_eq!(ctx.image_index_value(308), 99);
    }
}

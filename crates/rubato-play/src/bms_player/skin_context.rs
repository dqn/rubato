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
    pub(super) playtime: i64,
    pub(super) total_notes: i32,
    pub(super) play_mode: BMSPlayerMode,
    pub(super) state: PlayState,
    pub(super) media_load_finished: bool,
    /// Live hi-speed value from LaneRenderer (mutated by START/SELECT during play).
    pub(super) live_hispeed: f32,
    /// Live lanecover value from LaneRenderer.
    pub(super) live_lanecover: f32,
    /// Live lift value from LaneRenderer.
    pub(super) live_lift: f32,
    /// Live hidden value from LaneRenderer.
    pub(super) live_hidden: f32,
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
    /// Global config reference for BGA mode and other skin property queries.
    pub(super) config: &'a rubato_types::config::Config,
    /// Score data property for Lua skin accessors (rate, exscore, etc.).
    pub(super) score_data_property: &'a rubato_types::score_data_property::ScoreDataProperty,
    /// Song metadata for string property queries (title, artist, genre, etc.).
    pub(super) song_metadata: &'a rubato_types::song_data::SongMetadata,
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

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(self.config)
    }

    fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
        self.score_data_property
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

    fn is_gauge_max(&self) -> bool {
        self.gauge.is_some_and(|g| g.gauge().is_max())
    }

    fn gauge_border_max(&self) -> Option<(f32, f32)> {
        let g = self.gauge?;
        let prop = g.gauge_by_type(g.gauge_type()).property();
        Some((prop.border, prop.max))
    }

    fn gauge_min(&self) -> f32 {
        self.gauge
            .map_or(0.0, |g| g.gauge_by_type(g.gauge_type()).property().min)
    }

    fn is_mode_changed(&self) -> bool {
        self.is_mode_changed
    }

    fn is_media_load_finished(&self) -> bool {
        self.media_load_finished
    }

    fn is_practice_mode(&self) -> bool {
        self.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice
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
            // Hi-speed (LR2 format: hispeed * 100, e.g. 3.5 -> 350)
            // Uses live LaneRenderer value, not saved player_config.
            10 => (self.live_hispeed * 100.0) as i32,
            // Hi-speed fractional part (e.g. 3.52 -> 2)
            311 => ((self.live_hispeed * 100.0) as i32) % 10,
            // Lanecover (0-1000 scale from live LaneRenderer)
            14 => (self.live_lanecover * 1000.0) as i32,
            // Lift (0-1000 scale from live LaneRenderer)
            314 => (self.live_lift * 1000.0) as i32,
            // Hidden (0-1000 scale from live LaneRenderer)
            315 => (self.live_hidden * 1000.0) as i32,
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
            312 => self.playtime.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            1163 => (self.playtime / 60000) as i32,
            1164 => ((self.playtime % 60000) / 1000) as i32,
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
            // Hi-speed (from live LaneRenderer, not saved play config)
            310 => self.live_hispeed,
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

    fn string_value(&self, id: i32) -> String {
        match id {
            // title
            10 => self.song_metadata.title.clone(),
            // subtitle
            11 => self.song_metadata.subtitle.clone(),
            // fulltitle
            12 => self.song_metadata.full_title(),
            // genre
            13 => self.song_metadata.genre.clone(),
            // artist
            14 => self.song_metadata.artist.clone(),
            // subartist
            15 => self.song_metadata.subartist.clone(),
            // fullartist
            16 => {
                if self.song_metadata.subartist.is_empty() {
                    self.song_metadata.artist.clone()
                } else {
                    format!(
                        "{} {}",
                        self.song_metadata.artist, self.song_metadata.subartist
                    )
                }
            }
            _ => String::new(),
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

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(&self.player.config)
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

    fn is_gauge_max(&self) -> bool {
        self.player
            .gauge
            .as_ref()
            .is_some_and(|g| g.gauge().is_max())
    }

    fn gauge_border_max(&self) -> Option<(f32, f32)> {
        let g = self.player.gauge.as_ref()?;
        let prop = g.gauge_by_type(g.gauge_type()).property();
        Some((prop.border, prop.max))
    }

    fn gauge_min(&self) -> f32 {
        self.player
            .gauge
            .as_ref()
            .map_or(0.0, |g| g.gauge_by_type(g.gauge_type()).property().min)
    }

    fn gauge_element_borders(&self) -> Vec<(f32, f32)> {
        match self.player.gauge.as_ref() {
            Some(g) => (0..g.gauge_type_length())
                .map(|i| {
                    let prop = g.gauge_by_type(i as i32).property();
                    (prop.border, prop.max)
                })
                .collect(),
            None => Vec::new(),
        }
    }

    fn is_mode_changed(&self) -> bool {
        self.player.orgmode.is_some_and(|org| {
            self.player
                .model
                .mode()
                .copied()
                .unwrap_or(bms_model::mode::Mode::BEAT_7K)
                != org
        })
    }

    fn is_media_load_finished(&self) -> bool {
        self.player.media_load_finished
    }

    fn is_practice_mode(&self) -> bool {
        self.player.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice
    }

    fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
        &self.player.main_state_data.score
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        match id {
            // Volume (0.0-1.0): write back to BMSPlayer's cached volume fields
            // so that skin property reads (float_value/integer_value) reflect the new value.
            17 => self.player.system_volume = value.clamp(0.0, 1.0),
            18 => self.player.key_volume = value.clamp(0.0, 1.0),
            19 => self.player.bg_volume = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn lane_shuffle_pattern_value(&self, player: usize, lane: usize) -> i32 {
        self.player
            .score
            .playinfo
            .lane_shuffle_pattern
            .as_ref()
            .and_then(|patterns| patterns.get(player))
            .and_then(|lanes| lanes.get(lane))
            .copied()
            .unwrap_or(-1)
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
            312 => self.player.playtime.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            1163 => (self.player.playtime / 60000) as i32,
            1164 => ((self.player.playtime % 60000) / 1000) as i32,
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

    fn string_value(&self, id: i32) -> String {
        match id {
            10 => self.player.song_metadata.title.clone(),
            11 => self.player.song_metadata.subtitle.clone(),
            12 => self.player.song_metadata.full_title(),
            13 => self.player.song_metadata.genre.clone(),
            14 => self.player.song_metadata.artist.clone(),
            15 => self.player.song_metadata.subartist.clone(),
            16 => {
                if self.player.song_metadata.subartist.is_empty() {
                    self.player.song_metadata.artist.clone()
                } else {
                    format!(
                        "{} {}",
                        self.player.song_metadata.artist, self.player.song_metadata.subartist
                    )
                }
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::skin_render_context::SkinRenderContext;

    /// Build a minimal PlayRenderContext with the given playtime (in ms).
    fn make_render_ctx(playtime: i64) -> PlayRenderContext<'static> {
        // Use Box::leak for test-only references so we get 'static lifetimes.
        let timer = Box::leak(Box::new(TimerManager::new()));
        let judge = Box::leak(Box::new(JudgeManager::default()));
        let player_config = Box::leak(Box::new(PlayerConfig::default()));
        let option_info = Box::leak(Box::new(ReplayData::default()));
        let play_config = Box::leak(Box::new(PlayConfig::default()));
        let config = Box::leak(Box::new(rubato_types::config::Config::default()));
        let score_data_property = Box::leak(Box::new(
            rubato_types::score_data_property::ScoreDataProperty::default(),
        ));
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
            live_hispeed: 0.0,
            live_lanecover: 0.0,
            live_lift: 0.0,
            live_hidden: 0.0,
            now_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            system_volume: 0.0,
            key_volume: 0.0,
            bg_volume: 0.0,
            is_mode_changed: false,
            lnmode_override: None,
            config,
            score_data_property,
            song_metadata: Box::leak(Box::new(rubato_types::song_data::SongMetadata::default())),
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
        let config = Box::leak(Box::new(rubato_types::config::Config::default()));
        let score_data_property = Box::leak(Box::new(
            rubato_types::score_data_property::ScoreDataProperty::default(),
        ));
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
            live_hispeed: 0.0,
            live_lanecover: 0.0,
            live_lift: 0.0,
            live_hidden: 0.0,
            now_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            system_volume: 0.0,
            key_volume: 0.0,
            bg_volume: 0.0,
            is_mode_changed: false,
            lnmode_override: None,
            config,
            score_data_property,
            song_metadata: Box::leak(Box::new(rubato_types::song_data::SongMetadata::default())),
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
        let config = Box::leak(Box::new(rubato_types::config::Config::default()));
        let score_data_property = Box::leak(Box::new(
            rubato_types::score_data_property::ScoreDataProperty::default(),
        ));
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
            live_hispeed: 0.0,
            live_lanecover: 0.0,
            live_lift: 0.0,
            live_hidden: 0.0,
            now_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            system_volume: 0.0,
            key_volume: 0.0,
            bg_volume: 0.0,
            is_mode_changed: false,
            lnmode_override,
            config,
            score_data_property,
            song_metadata: Box::leak(Box::new(rubato_types::song_data::SongMetadata::default())),
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

    // ============================================================
    // config_ref and score_data_property tests
    // ============================================================

    #[test]
    fn config_ref_returns_bga_mode() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let judge = Box::leak(Box::new(JudgeManager::default()));
        let player_config = Box::leak(Box::new(PlayerConfig::default()));
        let option_info = Box::leak(Box::new(ReplayData::default()));
        let play_config = Box::leak(Box::new(PlayConfig::default()));
        let config = Box::leak(Box::new(rubato_types::config::Config::default()));
        let score_data_property = Box::leak(Box::new(
            rubato_types::score_data_property::ScoreDataProperty::default(),
        ));
        let ctx = PlayRenderContext {
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
            live_hispeed: 0.0,
            live_lanecover: 0.0,
            live_lift: 0.0,
            live_hidden: 0.0,
            now_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            system_volume: 0.0,
            key_volume: 0.0,
            bg_volume: 0.0,
            is_mode_changed: false,
            lnmode_override: None,
            config,
            score_data_property,
            song_metadata: Box::leak(Box::new(rubato_types::song_data::SongMetadata::default())),
        };
        // config_ref should return Some
        assert!(ctx.config_ref().is_some());
        // image_index_value(72) reads BGA mode from config -- default is 0 (ON)
        let bga_index = ctx.image_index_value(72);
        assert_eq!(bga_index, 0, "default BGA mode should be 0 (ON)");
    }

    #[test]
    fn score_data_property_returns_live_data() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let judge = Box::leak(Box::new(JudgeManager::default()));
        let player_config = Box::leak(Box::new(PlayerConfig::default()));
        let option_info = Box::leak(Box::new(ReplayData::default()));
        let play_config = Box::leak(Box::new(PlayConfig::default()));
        let config = Box::leak(Box::new(rubato_types::config::Config::default()));
        let score_data_property = Box::leak(Box::new(
            rubato_types::score_data_property::ScoreDataProperty {
                nowrate: 0.85,
                nowscore: 999,
                ..rubato_types::score_data_property::ScoreDataProperty::default()
            },
        ));
        let ctx = PlayRenderContext {
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
            live_hispeed: 0.0,
            live_lanecover: 0.0,
            live_lift: 0.0,
            live_hidden: 0.0,
            now_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            system_volume: 0.0,
            key_volume: 0.0,
            bg_volume: 0.0,
            is_mode_changed: false,
            lnmode_override: None,
            config,
            score_data_property,
            song_metadata: Box::leak(Box::new(rubato_types::song_data::SongMetadata::default())),
        };
        let prop = ctx.score_data_property();
        assert!((prop.now_rate() - 0.85).abs() < f32::EPSILON);
        assert_eq!(prop.now_ex_score(), 999);
    }

    // ============================================================
    // is_gauge_max() tests
    // ============================================================

    #[test]
    fn is_gauge_max_returns_true_when_gauge_at_max() {
        let model = {
            let mut m = bms_model::bms_model::BMSModel::new();
            m.total = 300.0;
            m
        };
        let gauge = Box::leak(Box::new(rubato_types::groove_gauge::GrooveGauge::new(
            &model,
            rubato_types::groove_gauge::NORMAL,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        )));
        // Push gauge to max (init=20, max=100, add_value clamps)
        gauge.add_value(200.0);
        assert!(
            gauge.gauge().is_max(),
            "gauge should be at max after add_value"
        );

        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            ctx.is_gauge_max(),
            "PlayRenderContext::is_gauge_max() should return true when gauge is at max"
        );
    }

    #[test]
    fn is_gauge_max_returns_false_when_gauge_not_at_max() {
        let model = {
            let mut m = bms_model::bms_model::BMSModel::new();
            m.total = 300.0;
            m
        };
        let gauge = Box::leak(Box::new(rubato_types::groove_gauge::GrooveGauge::new(
            &model,
            rubato_types::groove_gauge::NORMAL,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        )));
        // Gauge starts at init=20, not at max=100
        assert!(!gauge.gauge().is_max());

        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            !ctx.is_gauge_max(),
            "PlayRenderContext::is_gauge_max() should return false when gauge is not at max"
        );
    }

    #[test]
    fn is_gauge_max_returns_false_when_no_gauge() {
        let ctx = make_render_ctx(0);
        assert!(ctx.gauge.is_none());
        assert!(
            !ctx.is_gauge_max(),
            "PlayRenderContext::is_gauge_max() should return false when gauge is None"
        );
    }

    // ============================================================
    // string_value() delegation tests
    // ============================================================

    fn make_render_ctx_with_metadata(
        metadata: rubato_types::song_data::SongMetadata,
    ) -> PlayRenderContext<'static> {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let judge = Box::leak(Box::new(JudgeManager::default()));
        let player_config = Box::leak(Box::new(PlayerConfig::default()));
        let option_info = Box::leak(Box::new(ReplayData::default()));
        let play_config = Box::leak(Box::new(PlayConfig::default()));
        let config = Box::leak(Box::new(rubato_types::config::Config::default()));
        let score_data_property = Box::leak(Box::new(
            rubato_types::score_data_property::ScoreDataProperty::default(),
        ));
        let song_metadata = Box::leak(Box::new(metadata));
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
            live_hispeed: 0.0,
            live_lanecover: 0.0,
            live_lift: 0.0,
            live_hidden: 0.0,
            now_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            system_volume: 0.0,
            key_volume: 0.0,
            bg_volume: 0.0,
            is_mode_changed: false,
            lnmode_override: None,
            config,
            score_data_property,
            song_metadata,
        }
    }

    /// Build a SongMetadata with the given public fields set.
    /// Uses Default::default() to handle private cached fields.
    fn make_metadata(
        title: &str,
        subtitle: &str,
        genre: &str,
        artist: &str,
        subartist: &str,
    ) -> rubato_types::song_data::SongMetadata {
        let mut m = rubato_types::song_data::SongMetadata::default();
        m.title = title.to_string();
        m.subtitle = subtitle.to_string();
        m.genre = genre.to_string();
        m.artist = artist.to_string();
        m.subartist = subartist.to_string();
        m
    }

    #[test]
    fn string_value_returns_song_metadata() {
        let metadata = make_metadata(
            "Test Title",
            "Test Subtitle",
            "Test Genre",
            "Test Artist",
            "Test SubArtist",
        );
        let ctx = make_render_ctx_with_metadata(metadata);
        assert_eq!(ctx.string_value(10), "Test Title");
        assert_eq!(ctx.string_value(11), "Test Subtitle");
        assert_eq!(ctx.string_value(12), "Test Title Test Subtitle");
        assert_eq!(ctx.string_value(13), "Test Genre");
        assert_eq!(ctx.string_value(14), "Test Artist");
        assert_eq!(ctx.string_value(15), "Test SubArtist");
        assert_eq!(ctx.string_value(16), "Test Artist Test SubArtist");
    }

    #[test]
    fn string_value_fulltitle_without_subtitle() {
        let metadata = make_metadata("Only Title", "", "", "", "");
        let ctx = make_render_ctx_with_metadata(metadata);
        assert_eq!(ctx.string_value(12), "Only Title");
    }

    #[test]
    fn string_value_fullartist_without_subartist() {
        let metadata = make_metadata("", "", "", "Only Artist", "");
        let ctx = make_render_ctx_with_metadata(metadata);
        assert_eq!(ctx.string_value(16), "Only Artist");
    }

    #[test]
    fn string_value_unknown_id_returns_empty() {
        let metadata = make_metadata("Test", "", "", "", "");
        let ctx = make_render_ctx_with_metadata(metadata);
        assert_eq!(ctx.string_value(999), "");
    }

    // ============================================================
    // PlayMouseContext config_ref() delegation test
    // ============================================================

    #[test]
    fn play_mouse_context_config_ref_returns_some() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let ctx = PlayMouseContext { timer, player };
        assert!(
            ctx.config_ref().is_some(),
            "PlayMouseContext::config_ref() must delegate to player.config"
        );
    }

    #[test]
    fn play_mouse_context_config_ref_reads_bga_mode() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        // BGA mode default is 0 (ON); image_index_value(72) reads it
        let ctx = PlayMouseContext { timer, player };
        assert_eq!(
            ctx.image_index_value(72),
            0,
            "PlayMouseContext should read BGA mode from config via config_ref()"
        );
    }

    // ============================================================
    // PlayMouseContext missing delegation regression tests
    // ============================================================

    #[test]
    fn play_mouse_context_score_data_property_returns_live_data() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        player.main_state_data.score.nowrate = 0.75;
        player.main_state_data.score.nowscore = 1234;
        let ctx = PlayMouseContext { timer, player };
        let prop = ctx.score_data_property();
        assert!(
            (prop.now_rate() - 0.75).abs() < f32::EPSILON,
            "PlayMouseContext::score_data_property() must return live rate"
        );
        assert_eq!(
            prop.now_ex_score(),
            1234,
            "PlayMouseContext::score_data_property() must return live exscore"
        );
    }

    #[test]
    fn play_mouse_context_gauge_border_max_returns_some_with_gauge() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new({
            let mut m = bms_model::bms_model::BMSModel::new();
            m.total = 300.0;
            m
        })));
        player.gauge = Some(rubato_types::groove_gauge::GrooveGauge::new(
            &player.model,
            rubato_types::groove_gauge::NORMAL,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        ));
        let ctx = PlayMouseContext { timer, player };
        let result = ctx.gauge_border_max();
        assert!(
            result.is_some(),
            "PlayMouseContext::gauge_border_max() must return Some when gauge exists"
        );
        let (border, max) = result.unwrap();
        assert!(border > 0.0, "border should be positive for NORMAL gauge");
        assert!(
            (max - 100.0).abs() < f32::EPSILON,
            "max should be 100.0 for NORMAL gauge"
        );
    }

    #[test]
    fn play_mouse_context_gauge_border_max_returns_none_without_gauge() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let ctx = PlayMouseContext { timer, player };
        assert!(
            ctx.gauge_border_max().is_none(),
            "PlayMouseContext::gauge_border_max() must return None without gauge"
        );
    }

    #[test]
    fn play_mouse_context_gauge_min_returns_value_with_gauge() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new({
            let mut m = bms_model::bms_model::BMSModel::new();
            m.total = 300.0;
            m
        })));
        player.gauge = Some(rubato_types::groove_gauge::GrooveGauge::new(
            &player.model,
            rubato_types::groove_gauge::NORMAL,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        ));
        let ctx = PlayMouseContext { timer, player };
        // NORMAL gauge min is 2.0 (from GaugeProperty::SevenKeys NORMAL)
        let min = ctx.gauge_min();
        assert!(
            min >= 0.0,
            "PlayMouseContext::gauge_min() must return non-negative value"
        );
    }

    #[test]
    fn play_mouse_context_gauge_element_borders_non_empty_with_gauge() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new({
            let mut m = bms_model::bms_model::BMSModel::new();
            m.total = 300.0;
            m
        })));
        player.gauge = Some(rubato_types::groove_gauge::GrooveGauge::new(
            &player.model,
            rubato_types::groove_gauge::NORMAL,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        ));
        let ctx = PlayMouseContext { timer, player };
        let borders = ctx.gauge_element_borders();
        assert!(
            !borders.is_empty(),
            "PlayMouseContext::gauge_element_borders() must be non-empty with gauge"
        );
    }

    #[test]
    fn play_mouse_context_gauge_element_borders_empty_without_gauge() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let ctx = PlayMouseContext { timer, player };
        assert!(
            ctx.gauge_element_borders().is_empty(),
            "PlayMouseContext::gauge_element_borders() must be empty without gauge"
        );
    }

    #[test]
    fn play_mouse_context_is_gauge_max_delegates() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new({
            let mut m = bms_model::bms_model::BMSModel::new();
            m.total = 300.0;
            m
        })));
        player.gauge = Some(rubato_types::groove_gauge::GrooveGauge::new(
            &player.model,
            rubato_types::groove_gauge::NORMAL,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        ));
        // Gauge starts at init=20, not at max
        let ctx = PlayMouseContext { timer, player };
        assert!(
            !ctx.is_gauge_max(),
            "PlayMouseContext::is_gauge_max() must return false when gauge is not at max"
        );
    }

    #[test]
    fn play_mouse_context_is_gauge_max_true_when_maxed() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new({
            let mut m = bms_model::bms_model::BMSModel::new();
            m.total = 300.0;
            m
        })));
        let mut gauge = rubato_types::groove_gauge::GrooveGauge::new(
            &player.model,
            rubato_types::groove_gauge::NORMAL,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        );
        gauge.add_value(200.0);
        player.gauge = Some(gauge);
        let ctx = PlayMouseContext { timer, player };
        assert!(
            ctx.is_gauge_max(),
            "PlayMouseContext::is_gauge_max() must return true when gauge is at max"
        );
    }

    #[test]
    fn play_mouse_context_is_mode_changed_false_by_default() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let ctx = PlayMouseContext { timer, player };
        assert!(
            !ctx.is_mode_changed(),
            "PlayMouseContext::is_mode_changed() must be false when orgmode is None"
        );
    }

    #[test]
    fn play_mouse_context_lane_shuffle_pattern_delegates() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        player.score.playinfo.lane_shuffle_pattern = Some(vec![vec![2, 0, 1, 3, 4, 5, 6, 7, 8, 9]]);
        let ctx = PlayMouseContext { timer, player };
        // ID 450 = 1P lane 0 -> source lane 2
        assert_eq!(
            ctx.image_index_value(450),
            2,
            "PlayMouseContext must delegate lane_shuffle_pattern_value for image_index 450"
        );
        // ID 451 = 1P lane 1 -> source lane 0
        assert_eq!(ctx.image_index_value(451), 0);
    }

    #[test]
    fn play_mouse_context_lane_shuffle_pattern_none_returns_minus_one() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let ctx = PlayMouseContext { timer, player };
        assert_eq!(
            ctx.image_index_value(450),
            -1,
            "PlayMouseContext must return -1 for lane shuffle when no pattern"
        );
    }

    // ============================================================
    // PlayMouseContext set_float_value volume delegation tests
    // ============================================================

    #[test]
    fn play_mouse_set_float_value_system_volume() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        player.system_volume = 0.5;
        let mut ctx = PlayMouseContext { timer, player };
        ctx.set_float_value(17, 0.8);
        assert!(
            (ctx.player.system_volume - 0.8).abs() < f32::EPSILON,
            "set_float_value(17) must update system_volume on BMSPlayer"
        );
    }

    #[test]
    fn play_mouse_set_float_value_key_volume() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        player.key_volume = 0.5;
        let mut ctx = PlayMouseContext { timer, player };
        ctx.set_float_value(18, 0.3);
        assert!(
            (ctx.player.key_volume - 0.3).abs() < f32::EPSILON,
            "set_float_value(18) must update key_volume on BMSPlayer"
        );
    }

    #[test]
    fn play_mouse_set_float_value_bg_volume() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        player.bg_volume = 0.5;
        let mut ctx = PlayMouseContext { timer, player };
        ctx.set_float_value(19, 0.1);
        assert!(
            (ctx.player.bg_volume - 0.1).abs() < f32::EPSILON,
            "set_float_value(19) must update bg_volume on BMSPlayer"
        );
    }

    #[test]
    fn play_mouse_set_float_value_clamps_volume() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let mut ctx = PlayMouseContext { timer, player };
        // Over 1.0 should clamp to 1.0
        ctx.set_float_value(17, 1.5);
        assert!(
            (ctx.player.system_volume - 1.0).abs() < f32::EPSILON,
            "set_float_value must clamp values above 1.0"
        );
        // Below 0.0 should clamp to 0.0
        ctx.set_float_value(17, -0.5);
        assert!(
            ctx.player.system_volume.abs() < f32::EPSILON,
            "set_float_value must clamp values below 0.0"
        );
    }

    // ============================================================
    // is_media_load_finished() delegation tests
    // ============================================================

    #[test]
    fn play_render_context_is_media_load_finished_true() {
        let mut ctx = make_render_ctx(0);
        ctx.media_load_finished = true;
        assert!(
            ctx.is_media_load_finished(),
            "PlayRenderContext::is_media_load_finished() must return true when media_load_finished is true"
        );
    }

    #[test]
    fn play_render_context_is_media_load_finished_false() {
        let ctx = make_render_ctx(0);
        assert!(
            !ctx.is_media_load_finished(),
            "PlayRenderContext::is_media_load_finished() must return false when media_load_finished is false"
        );
    }

    #[test]
    fn play_mouse_context_is_media_load_finished_true() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        player.media_load_finished = true;
        let ctx = PlayMouseContext { timer, player };
        assert!(
            ctx.is_media_load_finished(),
            "PlayMouseContext::is_media_load_finished() must return true when player.media_load_finished is true"
        );
    }

    #[test]
    fn play_mouse_context_is_media_load_finished_false() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let ctx = PlayMouseContext { timer, player };
        assert!(
            !ctx.is_media_load_finished(),
            "PlayMouseContext::is_media_load_finished() must return false by default"
        );
    }

    // ============================================================
    // is_practice_mode() delegation tests
    // ============================================================

    #[test]
    fn play_render_context_is_practice_mode_true() {
        let mut ctx = make_render_ctx(0);
        ctx.play_mode = BMSPlayerMode::new(rubato_core::bms_player_mode::Mode::Practice);
        assert!(
            ctx.is_practice_mode(),
            "PlayRenderContext::is_practice_mode() must return true in Practice mode"
        );
    }

    #[test]
    fn play_render_context_is_practice_mode_false() {
        let ctx = make_render_ctx(0);
        assert!(
            !ctx.is_practice_mode(),
            "PlayRenderContext::is_practice_mode() must return false in Play mode"
        );
    }

    #[test]
    fn play_mouse_context_is_practice_mode_true() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        player.play_mode = BMSPlayerMode::new(rubato_core::bms_player_mode::Mode::Practice);
        let ctx = PlayMouseContext { timer, player };
        assert!(
            ctx.is_practice_mode(),
            "PlayMouseContext::is_practice_mode() must return true in Practice mode"
        );
    }

    #[test]
    fn play_mouse_context_is_practice_mode_false() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let ctx = PlayMouseContext { timer, player };
        assert!(
            !ctx.is_practice_mode(),
            "PlayMouseContext::is_practice_mode() must return false in Play mode"
        );
    }

    #[test]
    fn play_mouse_set_float_value_unknown_id_no_op() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        player.system_volume = 0.5;
        player.key_volume = 0.5;
        player.bg_volume = 0.5;
        let mut ctx = PlayMouseContext { timer, player };
        ctx.set_float_value(999, 0.0);
        assert!(
            (ctx.player.system_volume - 0.5).abs() < f32::EPSILON,
            "set_float_value with unknown ID must not change system_volume"
        );
        assert!(
            (ctx.player.key_volume - 0.5).abs() < f32::EPSILON,
            "set_float_value with unknown ID must not change key_volume"
        );
        assert!(
            (ctx.player.bg_volume - 0.5).abs() < f32::EPSILON,
            "set_float_value with unknown ID must not change bg_volume"
        );
    }
}

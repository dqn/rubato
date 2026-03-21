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
    /// Song data for boolean property queries (chart mode, LN, BGA, difficulty, etc.).
    pub(super) song_data: Option<&'a rubato_types::song_data::SongData>,
    /// Skin offset values for positional adjustments during prepare().
    pub(super) offsets: &'a std::collections::HashMap<i32, rubato_types::skin_offset::SkinOffset>,
    /// Cumulative playtime in seconds from PlayerData.
    /// Java: PlayerData.getPlaytime() -- total play time across all sessions.
    pub(super) cumulative_playtime_seconds: i64,
    /// Current scroll duration from LaneRenderer (240000/bpm/hispeed * (1-lanecover)).
    /// Java: BMSPlayer.getLanerender().getCurrentDuration()
    pub(super) current_duration: i32,
    /// Pending actions outbox for side effects (audio play/stop) that cannot be
    /// executed directly during rendering.
    #[allow(dead_code)]
    pub(super) pending: &'a mut super::PendingActions,
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

    fn boot_time_millis(&self) -> i64 {
        self.timer.boot_time_millis()
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

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.song_data
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
            // Hi-speed fractional part (e.g. 3.52 -> 52)
            311 => ((self.live_hispeed * 100.0) as i32) % 100,
            // Lanecover (0-1000 scale from live LaneRenderer)
            14 => (self.live_lanecover * 1000.0) as i32,
            // Lift (0-1000 scale from live LaneRenderer)
            314 => (self.live_lift * 1000.0) as i32,
            // Hidden (0-1000 scale from live LaneRenderer)
            315 => (self.live_hidden * 1000.0) as i32,
            // Total notes
            350 => self.total_notes,
            // Cumulative playtime (hours/minutes/seconds from PlayerData, in seconds)
            // Java: PlayerData.getPlaytime() / 3600, / 60 % 60, % 60
            17 => (self.cumulative_playtime_seconds / 3600) as i32,
            18 => ((self.cumulative_playtime_seconds / 60) % 60) as i32,
            19 => (self.cumulative_playtime_seconds % 60) as i32,
            // Volume (0-100 scale)
            57 => (self.system_volume * 100.0) as i32,
            58 => (self.key_volume * 100.0) as i32,
            59 => (self.bg_volume * 100.0) as i32,
            // BPM
            90 => self.max_bpm as i32,
            91 => self.min_bpm as i32,
            92 => self.main_bpm as i32,
            160 => self.now_bpm as i32,
            // Elapsed playtime from TIMER_PLAY (Java: timer.getNowTime(TIMER_PLAY))
            // Division in i64 before narrowing to i32 to avoid overflow for songs >35.8 min.
            161 => (self.timer.now_time_for_id(TIMER_PLAY) / 60000) as i32,
            162 => ((self.timer.now_time_for_id(TIMER_PLAY) / 1000) % 60) as i32,
            // Remaining playtime (Java: max(playtime - elapsed + 1000, 0))
            163 => {
                let remaining =
                    (self.playtime - self.timer.now_time_for_id(TIMER_PLAY) + 1000).max(0);
                (remaining / 60000) as i32
            }
            164 => {
                let remaining =
                    (self.playtime - self.timer.now_time_for_id(TIMER_PLAY) + 1000).max(0);
                ((remaining / 1000) % 60) as i32
            }
            // Scroll duration from LaneRenderer (Java: getCurrentDuration())
            312 => self.current_duration,
            // Lanecover2: (1 - lift) * lanecover * 1000
            316 => ((1.0 - self.live_lift) * self.live_lanecover * 1000.0) as i32,
            // Chart length (minutes/seconds)
            1163 => ((self.playtime.max(0) / 60000) % 60) as i32,
            1164 => ((self.playtime.max(0) / 1000) % 60) as i32,
            // Scroll duration variants (IDs 1312-1327)
            // Java: IntegerPropertyFactory NUMBER_DURATION_LANECOVER_ON..NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_OFF
            1312..=1327 => {
                let offset = id - 1312;
                let green = offset % 2 == 1;
                let cover = offset % 4 < 2;
                let mode = offset / 4;
                let bpm = match mode {
                    0 => self.now_bpm,
                    1 => self.main_bpm,
                    2 => self.min_bpm,
                    3 => self.max_bpm,
                    _ => 0.0,
                };
                if bpm == 0.0 || self.live_hispeed == 0.0 {
                    return 0;
                }
                (240000.0 / bpm / self.live_hispeed as f64
                    * if cover {
                        1.0 - self.live_lanecover as f64
                    } else {
                        1.0
                    }
                    * if green { 0.6 } else { 1.0 })
                .round() as i32
            }
            // Loading progress: 100 if media loaded, else 0
            165 => {
                if self.media_load_finished {
                    100
                } else {
                    0
                }
            }
            // IDs 20-26 (FPS, system date/time) handled by default_integer_value
            _ => self.default_integer_value(id),
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
            _ => self.default_float_value(id),
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        match id {
            // OPTION_AUTOPLAYOFF (Java: SkinProperty.OPTION_AUTOPLAYOFF = 32)
            32 => {
                self.play_mode.mode != rubato_core::bms_player_mode::Mode::Autoplay
                    && self.play_mode.mode != rubato_core::bms_player_mode::Mode::Replay
            }
            // OPTION_AUTOPLAYON (Java: SkinProperty.OPTION_AUTOPLAYON = 33)
            33 => {
                self.play_mode.mode == rubato_core::bms_player_mode::Mode::Autoplay
                    || self.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay
            }
            // Loading state (OPTION_LOADING1 = 80)
            80 => self.state == PlayState::Preload,
            // OPTION_LOADED (Java: 81)
            81 => self.state != PlayState::Preload,
            // OPTION_REPLAY_OFF (Java: 82)
            82 => self.play_mode.mode != rubato_core::bms_player_mode::Mode::Replay,
            // OPTION_REPLAY_PLAYING (Java: 84)
            84 => self.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay,
            // OPTION_LANECOVER1_ON (Java: 271)
            271 => self.live_lanecover > 0.0,
            // OPTION_LIFT1_ON (Java: 272)
            272 => self.live_lift > 0.0,
            // OPTION_HIDDEN1_ON (Java: 273)
            273 => self.live_hidden > 0.0,
            // OPTION_STATE_PRACTICE (Java: 1080)
            1080 => self.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice,
            // OPTION_1P_0_9 through OPTION_1P_100 (Java: 230-240)
            // Gauge value falls within the corresponding 10-unit range (max=100)
            230..=240 => self.gauge.is_some_and(|g| {
                let bucket = id - 230;
                let low = bucket as f32 * 10.0;
                let high = (bucket + 1) as f32 * 10.0;
                g.value() >= low && g.value() < high
            }),
            // OPTION_1P_BORDER_OR_MORE (Java: 1240) -- gauge >= clear threshold
            1240 => self.gauge.is_some_and(|g| g.is_qualified()),
            _ => self.default_boolean_value(id),
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

    fn get_offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.offsets.get(&id)
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

    fn boot_time_millis(&self) -> i64 {
        self.timer.boot_time_millis()
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

    fn score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.player.score.db_score.as_ref()
    }

    fn rival_score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.player.score.rival_score.as_ref()
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

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.player.song_data.as_ref()
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

    fn config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
        Some(&mut self.player.config)
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        // Volume (0.0-1.0): write back to BMSPlayer's cached volume fields
        // so that skin property reads (float_value/integer_value) reflect the new value,
        // and propagate to audio driver via pending_audio_config outbox.
        if (17..=19).contains(&id) {
            let clamped = value.clamp(0.0, 1.0);
            match id {
                17 => self.player.system_volume = clamped,
                18 => self.player.key_volume = clamped,
                19 => self.player.bg_volume = clamped,
                _ => unreachable!(),
            }
            if let Some(mut audio) = self.player.config.audio.clone() {
                match id {
                    17 => audio.systemvolume = clamped,
                    18 => audio.keyvolume = clamped,
                    19 => audio.bgvolume = clamped,
                    _ => unreachable!(),
                }
                self.player.config.audio = Some(audio.clone());
                self.player.pending.pending_audio_config = Some(audio);
            }
        }
    }

    fn notify_audio_config_changed(&mut self) {
        if let Some(audio) = self.player.config.audio.clone() {
            self.player.pending.pending_audio_config = Some(audio);
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
            // Hi-speed (LR2 format: hispeed * 100, e.g. 3.5 -> 350)
            // Uses live LaneRenderer value, not saved player_config.
            10 => {
                let hs = self
                    .player
                    .lanerender
                    .as_ref()
                    .map_or(0.0, |lr| lr.hispeed());
                (hs * 100.0) as i32
            }
            // Hi-speed fractional part (e.g. 3.52 -> 52)
            311 => {
                let hs = self
                    .player
                    .lanerender
                    .as_ref()
                    .map_or(0.0, |lr| lr.hispeed());
                ((hs * 100.0) as i32) % 100
            }
            // Lanecover (0-1000 scale from live LaneRenderer)
            14 => {
                let lc = self
                    .player
                    .lanerender
                    .as_ref()
                    .map_or(0.0, |lr| lr.lanecover());
                (lc * 1000.0) as i32
            }
            // Lift (0-1000 scale from live LaneRenderer)
            314 => {
                let lift = self
                    .player
                    .lanerender
                    .as_ref()
                    .map_or(0.0, |lr| lr.lift_region());
                (lift * 1000.0) as i32
            }
            // Hidden (0-1000 scale from live LaneRenderer)
            315 => {
                let hidden = self
                    .player
                    .lanerender
                    .as_ref()
                    .map_or(0.0, |lr| lr.hidden_cover());
                (hidden * 1000.0) as i32
            }
            // Total notes
            350 => self.player.total_notes(),
            // Cumulative playtime (hours/minutes/seconds from PlayerData, in seconds)
            17 => (self.player.cumulative_playtime_seconds / 3600) as i32,
            18 => ((self.player.cumulative_playtime_seconds / 60) % 60) as i32,
            19 => (self.player.cumulative_playtime_seconds % 60) as i32,
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
            // Elapsed playtime from TIMER_PLAY
            // Division in i64 before narrowing to i32 to avoid overflow for songs >35.8 min.
            161 => (self.timer.now_time_for_id(TIMER_PLAY) / 60000) as i32,
            162 => ((self.timer.now_time_for_id(TIMER_PLAY) / 1000) % 60) as i32,
            // Remaining playtime
            163 => {
                let remaining =
                    (self.player.playtime - self.timer.now_time_for_id(TIMER_PLAY) + 1000).max(0);
                (remaining / 60000) as i32
            }
            164 => {
                let remaining =
                    (self.player.playtime - self.timer.now_time_for_id(TIMER_PLAY) + 1000).max(0);
                ((remaining / 1000) % 60) as i32
            }
            // Scroll duration from LaneRenderer (Java: getCurrentDuration())
            312 => self
                .player
                .lanerender
                .as_ref()
                .map_or(0, |lr| lr.current_duration()),
            // Lanecover2: (1 - lift) * lanecover * 1000
            316 => {
                let lr = self.player.lanerender.as_ref();
                let lc = lr.map_or(0.0, |lr| lr.lanecover());
                let lift = lr.map_or(0.0, |lr| lr.lift_region());
                ((1.0 - lift) * lc * 1000.0) as i32
            }
            // Chart length (minutes/seconds)
            1163 => ((self.player.playtime.max(0) / 60000) % 60) as i32,
            1164 => ((self.player.playtime.max(0) / 1000) % 60) as i32,
            // Scroll duration variants (IDs 1312-1327)
            1312..=1327 => {
                let lr = self.player.lanerender.as_ref();
                let offset = id - 1312;
                let green = offset % 2 == 1;
                let cover = offset % 4 < 2;
                let mode = offset / 4;
                let bpm = match mode {
                    0 => lr.map_or(0.0, |lr| lr.now_bpm()),
                    1 => lr.map_or(0.0, |lr| lr.main_bpm()),
                    2 => lr.map_or(0.0, |lr| lr.min_bpm()),
                    3 => lr.map_or(0.0, |lr| lr.max_bpm()),
                    _ => 0.0,
                };
                let hispeed = lr.map_or(1.0, |lr| lr.hispeed()) as f64;
                if bpm == 0.0 || hispeed == 0.0 {
                    return 0;
                }
                let lanecover = lr.map_or(0.0, |lr| lr.lanecover()) as f64;
                (240000.0 / bpm / hispeed
                    * if cover { 1.0 - lanecover } else { 1.0 }
                    * if green { 0.6 } else { 1.0 })
                .round() as i32
            }
            // Loading progress: 100 if media loaded, else 0
            165 => {
                if self.player.media_load_finished {
                    100
                } else {
                    0
                }
            }
            // IDs 20-29 (FPS, system date/time, boot time) handled by default_integer_value
            _ => self.default_integer_value(id),
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
            // Hi-speed (from live LaneRenderer, not saved play config)
            310 => self
                .player
                .lanerender
                .as_ref()
                .map_or(0.0, |lr| lr.hispeed()),
            _ => self.default_float_value(id),
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        match id {
            32 => {
                self.player.play_mode.mode != rubato_core::bms_player_mode::Mode::Autoplay
                    && self.player.play_mode.mode != rubato_core::bms_player_mode::Mode::Replay
            }
            33 => {
                self.player.play_mode.mode == rubato_core::bms_player_mode::Mode::Autoplay
                    || self.player.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay
            }
            80 => self.player.state == PlayState::Preload,
            81 => self.player.state != PlayState::Preload,
            82 => self.player.play_mode.mode != rubato_core::bms_player_mode::Mode::Replay,
            84 => self.player.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay,
            271 => self
                .player
                .lanerender
                .as_ref()
                .is_some_and(|lr| lr.lanecover() > 0.0),
            272 => self
                .player
                .lanerender
                .as_ref()
                .is_some_and(|lr| lr.lift_region() > 0.0),
            273 => self
                .player
                .lanerender
                .as_ref()
                .is_some_and(|lr| lr.hidden_cover() > 0.0),
            1080 => self.player.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice,
            // OPTION_1P_0_9 through OPTION_1P_100 (Java: 230-240)
            230..=240 => self.player.gauge.as_ref().is_some_and(|g| {
                let bucket = id - 230;
                let low = bucket as f32 * 10.0;
                let high = (bucket + 1) as f32 * 10.0;
                g.value() >= low && g.value() < high
            }),
            // OPTION_1P_BORDER_OR_MORE (Java: 1240) -- gauge >= clear threshold
            1240 => self.player.gauge.as_ref().is_some_and(|g| g.is_qualified()),
            _ => self.default_boolean_value(id),
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

    fn get_offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.player.main_state_data.offsets.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::skin_render_context::SkinRenderContext;

    static EMPTY_OFFSETS: std::sync::LazyLock<
        std::collections::HashMap<i32, rubato_types::skin_offset::SkinOffset>,
    > = std::sync::LazyLock::new(std::collections::HashMap::new);

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
            song_data: None,
            offsets: &EMPTY_OFFSETS,
            cumulative_playtime_seconds: 0,
            current_duration: 0,
            pending: Box::leak(Box::new(PendingActions::new())),
        }
    }

    #[test]
    fn play_render_context_get_offset_value_returns_populated_offsets() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let judge = Box::leak(Box::new(JudgeManager::default()));
        let player_config = Box::leak(Box::new(PlayerConfig::default()));
        let option_info = Box::leak(Box::new(ReplayData::default()));
        let play_config = Box::leak(Box::new(PlayConfig::default()));
        let config = Box::leak(Box::new(rubato_types::config::Config::default()));
        let score_data_property = Box::leak(Box::new(
            rubato_types::score_data_property::ScoreDataProperty::default(),
        ));
        let offsets = Box::leak(Box::new(std::collections::HashMap::from([(
            3,
            rubato_types::skin_offset::SkinOffset {
                x: 10.0,
                y: 20.0,
                w: 0.0,
                h: 0.0,
                r: 0.0,
                a: 0.0,
            },
        )])));
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
            song_data: None,
            offsets,
            cumulative_playtime_seconds: 0,
            current_duration: 0,
            pending: Box::leak(Box::new(PendingActions::new())),
        };

        // Populated offset should be returned
        let off = ctx.get_offset_value(3);
        assert!(off.is_some(), "offset ID 3 should be present");
        assert_eq!(off.unwrap().x, 10.0);
        assert_eq!(off.unwrap().y, 20.0);

        // Non-existent offset should return None
        assert!(ctx.get_offset_value(999).is_none());
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
    fn id_312_returns_current_duration() {
        // ID 312 returns LaneRenderer.getCurrentDuration() (scroll duration),
        // not raw playtime. In test context, current_duration defaults to 0.
        let ctx = make_render_ctx(123_456);
        assert_eq!(
            ctx.integer_value(312),
            0,
            "ID 312 should return current_duration (default 0)"
        );

        // With current_duration set explicitly
        let mut ctx2 = make_render_ctx(123_456);
        ctx2.current_duration = 1500;
        assert_eq!(
            ctx2.integer_value(312),
            1500,
            "ID 312 should return current_duration from LaneRenderer"
        );
    }

    #[test]
    fn playtime_large_value() {
        // 7_200_000 ms = 120 minutes = 2 hours 0 minutes
        // ID 1163: (120) % 60 = 0 (minutes within hour)
        // ID 1164: (7200) % 60 = 0
        let ctx = make_render_ctx(7_200_000);
        assert_eq!(ctx.integer_value(1163), 0);
        assert_eq!(ctx.integer_value(1164), 0);

        // 3_900_000 ms = 65 minutes = 1 hour 5 minutes
        // ID 1163: (65) % 60 = 5
        // ID 1164: (3900) % 60 = 0
        let ctx = make_render_ctx(3_900_000);
        assert_eq!(ctx.integer_value(1163), 5);
        assert_eq!(ctx.integer_value(1164), 0);
    }

    #[test]
    fn playtime_negative_clamped_to_zero() {
        // Negative playtime (corrupted data) should be clamped to 0, not produce
        // negative minutes/seconds. Matches select/decide/result screen behavior.
        let ctx = make_render_ctx(-5000);
        assert_eq!(
            ctx.integer_value(1163),
            0,
            "negative playtime minutes must be 0"
        );
        assert_eq!(
            ctx.integer_value(1164),
            0,
            "negative playtime seconds must be 0"
        );
    }

    #[test]
    fn cumulative_playtime_ids_17_18_19() {
        // 3661 seconds = 1 hour 1 minute 1 second
        let mut ctx = make_render_ctx(0);
        ctx.cumulative_playtime_seconds = 3661;
        assert_eq!(ctx.integer_value(17), 1, "ID 17: hours");
        assert_eq!(ctx.integer_value(18), 1, "ID 18: minutes");
        assert_eq!(ctx.integer_value(19), 1, "ID 19: seconds");
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
            song_data: None,
            offsets: &EMPTY_OFFSETS,
            cumulative_playtime_seconds: 0,
            current_duration: 0,
            pending: Box::leak(Box::new(PendingActions::new())),
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
            song_data: None,
            offsets: &EMPTY_OFFSETS,
            cumulative_playtime_seconds: 0,
            current_duration: 0,
            pending: Box::leak(Box::new(PendingActions::new())),
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
            song_data: None,
            offsets: &EMPTY_OFFSETS,
            cumulative_playtime_seconds: 0,
            current_duration: 0,
            pending: Box::leak(Box::new(PendingActions::new())),
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
            song_data: None,
            offsets: &EMPTY_OFFSETS,
            cumulative_playtime_seconds: 0,
            current_duration: 0,
            pending: Box::leak(Box::new(PendingActions::new())),
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
            song_data: None,
            offsets: &EMPTY_OFFSETS,
            cumulative_playtime_seconds: 0,
            current_duration: 0,
            pending: Box::leak(Box::new(PendingActions::new())),
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

    // ============================================================
    // float_value fallthrough to default_float_value regression tests
    // ============================================================

    #[test]
    fn play_render_context_float_value_falls_through_to_default() {
        let mut song_data = rubato_types::song_data::SongData::default();
        song_data.info = Some(rubato_types::song_information::SongInformation {
            peakdensity: 12.5,
            ..Default::default()
        });
        let song_data = Box::leak(Box::new(song_data));
        let mut ctx = make_render_ctx(0);
        ctx.song_data = Some(song_data);
        // ID 360 = chart_peakdensity, handled by default_float_value
        let val = ctx.float_value(360);
        assert!(
            (val - 12.5).abs() < f32::EPSILON,
            "PlayRenderContext::float_value(360) must fall through to default_float_value, got {val}"
        );
    }

    #[test]
    fn play_mouse_context_float_value_falls_through_to_default() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let mut song_data = rubato_types::song_data::SongData::default();
        song_data.info = Some(rubato_types::song_information::SongInformation {
            peakdensity: 7.25,
            ..Default::default()
        });
        player.song_data = Some(song_data);
        let ctx = PlayMouseContext { timer, player };
        // ID 360 = chart_peakdensity, handled by default_float_value
        let val = ctx.float_value(360);
        assert!(
            (val - 7.25).abs() < f32::EPSILON,
            "PlayMouseContext::float_value(360) must fall through to default_float_value, got {val}"
        );
    }

    // ============================================================
    // Gauge range boolean IDs 230-240 and 1240 tests
    // ============================================================

    /// Helper: create a GrooveGauge with NORMAL type and set gauge value.
    fn make_gauge_with_value(value: f32) -> rubato_types::groove_gauge::GrooveGauge {
        let model = {
            let mut m = bms_model::bms_model::BMSModel::new();
            m.total = 300.0;
            m
        };
        let mut gauge = rubato_types::groove_gauge::GrooveGauge::new(
            &model,
            rubato_types::groove_gauge::NORMAL,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        );
        gauge.set_value(value);
        gauge
    }

    #[test]
    fn play_render_context_gauge_range_0_9_true_when_value_in_range() {
        let gauge = Box::leak(Box::new(make_gauge_with_value(5.0)));
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            ctx.boolean_value(230),
            "ID 230 (0-9%) should be true when gauge value is 5.0 (max=100)"
        );
    }

    #[test]
    fn play_render_context_gauge_range_0_9_false_when_value_out_of_range() {
        let gauge = Box::leak(Box::new(make_gauge_with_value(15.0)));
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            !ctx.boolean_value(230),
            "ID 230 (0-9%) should be false when gauge value is 15.0 (max=100)"
        );
    }

    #[test]
    fn play_render_context_gauge_range_50_59_true() {
        let gauge = Box::leak(Box::new(make_gauge_with_value(55.0)));
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            ctx.boolean_value(235),
            "ID 235 (50-59%) should be true when gauge value is 55.0 (max=100)"
        );
    }

    #[test]
    fn play_render_context_gauge_range_100_true_when_at_max() {
        let gauge = Box::leak(Box::new(make_gauge_with_value(100.0)));
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            ctx.boolean_value(240),
            "ID 240 (100%) should be true when gauge value is 100.0 (max=100)"
        );
    }

    #[test]
    fn play_render_context_gauge_range_100_false_when_not_at_max() {
        let gauge = Box::leak(Box::new(make_gauge_with_value(99.0)));
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            !ctx.boolean_value(240),
            "ID 240 (100%) should be false when gauge value is 99.0 (max=100)"
        );
    }

    #[test]
    fn play_render_context_gauge_range_boundary_exclusive_high() {
        let gauge = Box::leak(Box::new(make_gauge_with_value(10.0)));
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            !ctx.boolean_value(230),
            "ID 230 should be false at exact boundary 10.0 (exclusive high)"
        );
        assert!(
            ctx.boolean_value(231),
            "ID 231 should be true at exact boundary 10.0 (inclusive low)"
        );
    }

    #[test]
    fn play_render_context_gauge_range_false_without_gauge() {
        let ctx = make_render_ctx(0);
        assert!(
            !ctx.boolean_value(230),
            "ID 230 should be false when gauge is None"
        );
        assert!(
            !ctx.boolean_value(1240),
            "ID 1240 should be false when gauge is None"
        );
    }

    #[test]
    fn play_render_context_border_or_more_true_when_qualified() {
        let gauge = Box::leak(Box::new(make_gauge_with_value(85.0)));
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            ctx.boolean_value(1240),
            "ID 1240 (BORDER_OR_MORE) should be true when gauge is qualified (value >= border)"
        );
    }

    #[test]
    fn play_render_context_border_or_more_false_when_not_qualified() {
        let gauge = Box::leak(Box::new(make_gauge_with_value(50.0)));
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(
            !ctx.boolean_value(1240),
            "ID 1240 (BORDER_OR_MORE) should be false when gauge is not qualified (value < border)"
        );
    }

    #[test]
    fn play_mouse_context_gauge_range_50_59_true() {
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
        gauge.set_value(55.0);
        player.gauge = Some(gauge);
        let ctx = PlayMouseContext { timer, player };
        assert!(
            ctx.boolean_value(235),
            "PlayMouseContext ID 235 (50-59%) should be true when gauge value is 55.0"
        );
    }

    #[test]
    fn play_mouse_context_border_or_more_true_when_qualified() {
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
        gauge.set_value(85.0);
        player.gauge = Some(gauge);
        let ctx = PlayMouseContext { timer, player };
        assert!(
            ctx.boolean_value(1240),
            "PlayMouseContext ID 1240 (BORDER_OR_MORE) should be true when gauge is qualified"
        );
    }

    #[test]
    fn play_mouse_context_gauge_range_false_without_gauge() {
        let timer = Box::leak(Box::new(TimerManager::new()));
        let player = Box::leak(Box::new(BMSPlayer::new(
            bms_model::bms_model::BMSModel::new(),
        )));
        let ctx = PlayMouseContext { timer, player };
        assert!(
            !ctx.boolean_value(235),
            "PlayMouseContext ID 235 should be false when gauge is None"
        );
        assert!(
            !ctx.boolean_value(1240),
            "PlayMouseContext ID 1240 should be false when gauge is None"
        );
    }

    // ============================================================
    // IDs 161-164: elapsed/remaining playtime from TIMER_PLAY
    // ============================================================

    #[test]
    fn elapsed_playtime_minutes_seconds_timer_off() {
        // When TIMER_PLAY is off, now_time_for_id returns 0
        let ctx = make_render_ctx(120_000);
        assert_eq!(ctx.integer_value(161), 0, "elapsed minutes when timer off");
        assert_eq!(ctx.integer_value(162), 0, "elapsed seconds when timer off");
    }

    #[test]
    fn elapsed_playtime_minutes_seconds_timer_on() {
        let ctx = make_render_ctx(120_000);
        // Simulate TIMER_PLAY being on for 65 seconds (65_000 ms)
        // Set nowmicrotime to 100_000_000 us and TIMER_PLAY to 35_000_000 us
        // so elapsed = (100_000_000 - 35_000_000) / 1000 = 65_000 ms
        ctx.timer.set_now_micro_time(100_000_000);
        ctx.timer.set_micro_timer(TIMER_PLAY, 35_000_000);
        assert_eq!(
            ctx.integer_value(161),
            1,
            "elapsed minutes = 65000/60000 = 1"
        );
        assert_eq!(
            ctx.integer_value(162),
            5,
            "elapsed seconds = (65000/1000)%60 = 5"
        );
    }

    #[test]
    fn remaining_playtime_timer_off() {
        // playtime=120000, elapsed=0 -> remaining = 120000 + 1000 = 121000
        let ctx = make_render_ctx(120_000);
        assert_eq!(
            ctx.integer_value(163),
            2,
            "remaining minutes = 121000/60000 = 2"
        );
        assert_eq!(
            ctx.integer_value(164),
            1,
            "remaining seconds = (121000/1000)%60 = 1"
        );
    }

    #[test]
    fn remaining_playtime_timer_on() {
        let ctx = make_render_ctx(120_000);
        // elapsed = 65_000 ms, remaining = max(120000 - 65000 + 1000, 0) = 56000
        ctx.timer.set_now_micro_time(100_000_000);
        ctx.timer.set_micro_timer(TIMER_PLAY, 35_000_000);
        assert_eq!(
            ctx.integer_value(163),
            0,
            "remaining minutes = 56000/60000 = 0"
        );
        assert_eq!(
            ctx.integer_value(164),
            56,
            "remaining seconds = (56000/1000)%60 = 56"
        );
    }

    #[test]
    fn remaining_playtime_past_end_clamped_to_zero() {
        let ctx = make_render_ctx(60_000);
        // elapsed = 120_000 ms (past end), remaining = max(60000 - 120000 + 1000, 0) = 0
        ctx.timer.set_now_micro_time(150_000_000);
        ctx.timer.set_micro_timer(TIMER_PLAY, 30_000_000);
        assert_eq!(ctx.integer_value(163), 0, "remaining minutes clamped to 0");
        assert_eq!(ctx.integer_value(164), 0, "remaining seconds clamped to 0");
    }

    // ============================================================
    // ID 316: NUMBER_LANECOVER2
    // ============================================================

    #[test]
    fn lanecover2_id_316() {
        let mut ctx = make_render_ctx(0);
        ctx.live_lanecover = 0.5;
        ctx.live_lift = 0.2;
        // (1.0 - 0.2) * 0.5 * 1000.0 = 400.0
        assert_eq!(ctx.integer_value(316), 400);
    }

    #[test]
    fn lanecover2_no_lift() {
        let mut ctx = make_render_ctx(0);
        ctx.live_lanecover = 0.3;
        ctx.live_lift = 0.0;
        // (1.0 - 0.0) * 0.3 * 1000.0 = 300.0
        assert_eq!(ctx.integer_value(316), 300);
    }

    #[test]
    fn lanecover2_full_lift() {
        let mut ctx = make_render_ctx(0);
        ctx.live_lanecover = 0.5;
        ctx.live_lift = 1.0;
        // (1.0 - 1.0) * 0.5 * 1000.0 = 0.0
        assert_eq!(ctx.integer_value(316), 0);
    }

    // ============================================================
    // IDs 1312-1327: DURATION_LANECOVER scroll duration variants
    // ============================================================

    #[test]
    fn duration_lanecover_now_bpm_cover_on() {
        let mut ctx = make_render_ctx(0);
        ctx.now_bpm = 120.0;
        ctx.live_hispeed = 1.0;
        ctx.live_lanecover = 0.5;
        // ID 1312: mode=0 (now), green=false, cover=true
        // 240000/120/1.0 * (1 - 0.5) * 1.0 = 1000
        assert_eq!(ctx.integer_value(1312), 1000);
    }

    #[test]
    fn duration_lanecover_now_bpm_green_cover_on() {
        let mut ctx = make_render_ctx(0);
        ctx.now_bpm = 120.0;
        ctx.live_hispeed = 1.0;
        ctx.live_lanecover = 0.5;
        // ID 1313: mode=0 (now), green=true, cover=true
        // 240000/120/1.0 * (1 - 0.5) * 0.6 = 600
        assert_eq!(ctx.integer_value(1313), 600);
    }

    #[test]
    fn duration_lanecover_now_bpm_cover_off() {
        let mut ctx = make_render_ctx(0);
        ctx.now_bpm = 120.0;
        ctx.live_hispeed = 2.0;
        ctx.live_lanecover = 0.5;
        // ID 1314: mode=0 (now), green=false, cover=false
        // 240000/120/2.0 * 1.0 * 1.0 = 1000
        assert_eq!(ctx.integer_value(1314), 1000);
    }

    #[test]
    fn duration_lanecover_now_bpm_green_cover_off() {
        let mut ctx = make_render_ctx(0);
        ctx.now_bpm = 120.0;
        ctx.live_hispeed = 2.0;
        ctx.live_lanecover = 0.5;
        // ID 1315: mode=0 (now), green=true, cover=false
        // 240000/120/2.0 * 1.0 * 0.6 = 600
        assert_eq!(ctx.integer_value(1315), 600);
    }

    #[test]
    fn duration_lanecover_main_bpm() {
        let mut ctx = make_render_ctx(0);
        ctx.main_bpm = 150.0;
        ctx.live_hispeed = 1.0;
        ctx.live_lanecover = 0.0;
        // ID 1316: mode=1 (main), green=false, cover=true
        // 240000/150/1.0 * (1 - 0) * 1.0 = 1600
        assert_eq!(ctx.integer_value(1316), 1600);
    }

    #[test]
    fn duration_lanecover_min_bpm() {
        let mut ctx = make_render_ctx(0);
        ctx.min_bpm = 100.0;
        ctx.live_hispeed = 1.0;
        ctx.live_lanecover = 0.0;
        // ID 1320: mode=2 (min), green=false, cover=true
        // 240000/100/1.0 * 1.0 * 1.0 = 2400
        assert_eq!(ctx.integer_value(1320), 2400);
    }

    #[test]
    fn duration_lanecover_max_bpm() {
        let mut ctx = make_render_ctx(0);
        ctx.max_bpm = 200.0;
        ctx.live_hispeed = 1.0;
        ctx.live_lanecover = 0.0;
        // ID 1324: mode=3 (max), green=false, cover=true
        // 240000/200/1.0 * 1.0 * 1.0 = 1200
        assert_eq!(ctx.integer_value(1324), 1200);
    }

    #[test]
    fn duration_lanecover_zero_bpm_returns_zero() {
        let mut ctx = make_render_ctx(0);
        ctx.now_bpm = 0.0;
        ctx.live_hispeed = 1.0;
        ctx.live_lanecover = 0.5;
        assert_eq!(ctx.integer_value(1312), 0);
    }

    #[test]
    fn duration_lanecover_last_id_1327() {
        let mut ctx = make_render_ctx(0);
        ctx.max_bpm = 120.0;
        ctx.live_hispeed = 1.0;
        ctx.live_lanecover = 0.0;
        // ID 1327: offset=15, mode=3 (max), green=true, cover=false
        // 240000/120/1.0 * 1.0 * 0.6 = 1200
        assert_eq!(ctx.integer_value(1327), 1200);
    }

    // ============================================================
    // ID 1240: OPTION_1P_BORDER_OR_MORE (gauge qualified)
    // ============================================================

    #[test]
    fn border_or_more_false_when_no_gauge() {
        let ctx = make_render_ctx(0);
        assert!(!ctx.boolean_value(1240));
    }

    #[test]
    fn border_or_more_false_when_gauge_below_border() {
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
        // NORMAL gauge: init=20%, border=80%. Value 20% < 80%, not qualified.
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(!ctx.boolean_value(1240));
    }

    #[test]
    fn border_or_more_true_when_gauge_at_border() {
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
        // Push gauge to >= border (80%). add_value(60) -> 20 + 60 = 80%
        gauge.add_value(60.0);
        let mut ctx = make_render_ctx(0);
        ctx.gauge = Some(gauge);
        assert!(ctx.boolean_value(1240));
    }
}

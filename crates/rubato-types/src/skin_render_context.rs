use crate::main_state_type::MainStateType;
use crate::song_data::ChartInfo;
use crate::timer_access::TimerAccess;

/// Compute the lnmode image-index override from chart data.
///
/// Java IntegerPropertyFactory (ID 308 / lnmode): when on BMSPlayer or MusicResult,
/// if the chart explicitly defines LN types (has_any_long_note && !has_undefined_long_note),
/// return 0 (LN), 1 (CN), or 2 (HCN) from the chart instead of the config setting.
///
/// Returns `Some(value)` when the chart defines an explicit LN type, `None` otherwise.
pub fn compute_lnmode_from_chart(chart: &ChartInfo) -> Option<i32> {
    if chart.has_any_long_note() && !chart.has_undefined_long_note() {
        if chart.has_long_note() {
            Some(0)
        } else if chart.has_charge_note() {
            Some(1)
        } else {
            // HCN (hell charge note)
            Some(2)
        }
    } else {
        None
    }
}

/// Extended context for skin rendering that provides timer access plus
/// additional capabilities (event execution, state changes, audio, timers).
///
/// Replaces the 5 no-op methods that were on skin's MainState trait, enabling
/// proper delegation when MainController context is available during rendering.
///
/// All methods have default no-op implementations for adapters that only carry
/// timer data (e.g., TimerOnlyMainState).
pub trait SkinRenderContext: TimerAccess {
    /// Execute a custom skin event by ID with arguments.
    fn execute_event(&mut self, _id: i32, _arg1: i32, _arg2: i32) {
        // default no-op
    }

    /// Change the application state (e.g., to CONFIG, SKINCONFIG).
    fn change_state(&mut self, _state: MainStateType) {
        // default no-op
    }

    /// Set a timer value by ID (micro-seconds).
    fn set_timer_micro(&mut self, _timer_id: crate::timer_id::TimerId, _micro_time: i64) {
        // default no-op
    }

    /// Play an audio file at the given path with volume and loop flag.
    fn audio_play(&mut self, _path: &str, _volume: f32, _is_loop: bool) {
        // default no-op
    }

    /// Stop an audio file at the given path.
    fn audio_stop(&mut self, _path: &str) {
        // default no-op
    }

    /// Returns the current main state type (e.g., Play, MusicSelect, Result).
    /// Used by skin adapters to answer state-specific queries like `is_bms_player()`.
    fn current_state_type(&self) -> Option<MainStateType> {
        None
    }

    /// Returns true when the current skin context is the music select screen.
    fn is_music_selector(&self) -> bool {
        self.current_state_type() == Some(MainStateType::MusicSelect)
    }

    /// Returns true when the current skin context is a result screen.
    fn is_result_state(&self) -> bool {
        matches!(
            self.current_state_type(),
            Some(MainStateType::Result | MainStateType::CourseResult)
        )
    }

    /// Returns the recent judge timing offsets (milliseconds).
    /// 100-element circular buffer. Used by SkinTimingVisualizer and SkinHitErrorVisualizer.
    fn recent_judges(&self) -> &[i64] {
        &[]
    }

    /// Returns the current write index into the recent judges circular buffer.
    fn recent_judges_index(&self) -> usize {
        0
    }

    // ============================================================
    // Property value delegation (skin property factories)
    // ============================================================

    /// Returns the integer property value for the given ID.
    /// Delegate properties call this via MainState::integer_value().
    fn integer_value(&self, id: i32) -> i32 {
        self.default_integer_value(id)
    }

    /// Default implementation for global integer property IDs.
    ///
    /// Handles IDs that Java IntegerPropertyFactory defines as global lambdas
    /// (work on ALL screens):
    /// - 20: current FPS
    /// - 21-26: system date/time (year/month/day/hour/minute/second)
    ///
    /// Callers that override `integer_value()` should fall through to this
    /// for unmatched IDs instead of returning `0`.
    fn default_integer_value(&self, id: i32) -> i32 {
        match id {
            // Current FPS
            20 => crate::fps_counter::current_fps(),
            // System date/time
            21 => {
                let now = chrono::Local::now();
                chrono::Datelike::year(&now)
            }
            22 => {
                let now = chrono::Local::now();
                chrono::Datelike::month(&now) as i32
            }
            23 => {
                let now = chrono::Local::now();
                chrono::Datelike::day(&now) as i32
            }
            24 => {
                let now = chrono::Local::now();
                chrono::Timelike::hour(&now) as i32
            }
            25 => {
                let now = chrono::Local::now();
                chrono::Timelike::minute(&now) as i32
            }
            26 => {
                let now = chrono::Local::now();
                chrono::Timelike::second(&now) as i32
            }
            _ => 0,
        }
    }

    /// Returns the image-index property value for the given ID.
    /// This is separate from `integer_value()` because Java distinguishes
    /// numeric refs and image-index refs even when they share the same ID.
    fn image_index_value(&self, id: i32) -> i32 {
        self.default_image_index_value(id)
    }

    /// Shared default implementation for image-index refs.
    fn default_image_index_value(&self, id: i32) -> i32 {
        let bool_to_i32 = |value: bool| if value { 1 } else { 0 };
        let player_config = self.player_config_ref();
        let target_image_index = player_config.map_or(-1, |config| {
            config
                .select_settings
                .targetlist
                .iter()
                .position(|target| target == &config.select_settings.targetid)
                .map(|index| index.min(10) as i32)
                .unwrap_or(0)
        });

        match id {
            11 => self.mode_image_index().unwrap_or(-1),
            12 => self.sort_image_index().unwrap_or(-1),
            40 => {
                if matches!(
                    self.current_state_type(),
                    Some(MainStateType::Play | MainStateType::Result | MainStateType::CourseResult)
                ) {
                    self.gauge_type()
                } else {
                    player_config.map_or(-1, |config| config.play_settings.gauge)
                }
            }
            42 => self.replay_option_data().map_or_else(
                || player_config.map_or(-1, |config| config.play_settings.random),
                |replay| replay.randomoption,
            ),
            43 => self.replay_option_data().map_or_else(
                || player_config.map_or(-1, |config| config.play_settings.random2),
                |replay| replay.randomoption2,
            ),
            54 => self.replay_option_data().map_or_else(
                || player_config.map_or(-1, |config| config.play_settings.doubleoption),
                |replay| replay.doubleoption,
            ),
            55 => self
                .current_play_config_ref()
                .map_or(-1, |config| config.fixhispeed),
            61 => self.target_score_data().map_or(-1, |score| {
                if score.play_option.option >= 0 {
                    score.play_option.option % 10
                } else {
                    -1
                }
            }),
            62 => self.target_score_data().map_or(-1, |score| {
                if score.play_option.option >= 0 {
                    (score.play_option.option / 10) % 10
                } else {
                    -1
                }
            }),
            63 => self.target_score_data().map_or(-1, |score| {
                if score.play_option.option >= 0 {
                    (score.play_option.option / 100) % 10
                } else {
                    -1
                }
            }),
            72 => self
                .config_ref()
                .map_or(-1, |config| config.render.bga as i32),
            75 => player_config.map_or(-1, |config| {
                bool_to_i32(config.judge_settings.notes_display_timing_auto_adjust)
            }),
            77 => target_image_index,
            78 => player_config.map_or(-1, |config| config.play_settings.gauge_auto_shift),
            89 => self.song_data_ref().map_or(-1, |song| {
                let favorite = song.favorite;
                if favorite & crate::song_data::INVISIBLE_SONG != 0 {
                    2
                } else if favorite & crate::song_data::FAVORITE_SONG != 0 {
                    1
                } else {
                    0
                }
            }),
            90 => self.song_data_ref().map_or(-1, |song| {
                let favorite = song.favorite;
                if favorite & crate::song_data::INVISIBLE_CHART != 0 {
                    2
                } else if favorite & crate::song_data::FAVORITE_CHART != 0 {
                    1
                } else {
                    0
                }
            }),
            301 => {
                player_config.map_or(-1, |config| bool_to_i32(config.judge_settings.custom_judge))
            }
            303 => player_config.map_or(-1, |config| {
                bool_to_i32(config.display_settings.showjudgearea)
            }),
            305 => player_config.map_or(-1, |config| {
                bool_to_i32(config.display_settings.markprocessednote)
            }),
            306 => player_config.map_or(-1, |config| bool_to_i32(config.display_settings.bpmguide)),
            308 => player_config.map_or(-1, |config| config.play_settings.lnmode),
            330 => self
                .current_play_config_ref()
                .map_or(-1, |config| bool_to_i32(config.enablelanecover)),
            331 => self
                .current_play_config_ref()
                .map_or(-1, |config| bool_to_i32(config.enablelift)),
            332 => self
                .current_play_config_ref()
                .map_or(-1, |config| bool_to_i32(config.enablehidden)),
            340 => self.current_play_config_ref().map_or(-1, |config| {
                match config.judgetype.as_str() {
                    "Combo" => 0,
                    "Duration" => 1,
                    "Lowest" => 2,
                    "Score" => 3,
                    _ => -1,
                }
            }),
            321..=324 => player_config
                .and_then(|config| {
                    config
                        .misc_settings
                        .autosavereplay
                        .get((id - 321) as usize)
                        .copied()
                })
                .unwrap_or(-1),
            341 => player_config.map_or(-1, |config| config.play_settings.bottom_shiftable_gauge),
            342 => self
                .current_play_config_ref()
                .map_or(-1, |config| bool_to_i32(config.hispeedautoadjust)),
            343 => player_config.map_or(-1, |config| {
                bool_to_i32(config.display_settings.is_guide_se)
            }),
            350 => player_config.map_or(-1, |config| config.display_settings.extranote_depth),
            351 => player_config.map_or(-1, |config| config.play_settings.mine_mode),
            352 => player_config.map_or(-1, |config| config.display_settings.scroll_mode),
            353 => player_config.map_or(-1, |config| config.note_modifier_settings.longnote_mode),
            360 => player_config.map_or(-1, |config| {
                config.note_modifier_settings.seven_to_nine_pattern
            }),
            361 => player_config.map_or(-1, |config| {
                config.note_modifier_settings.seven_to_nine_type
            }),
            370 => self.score_data_ref().map_or(-1, |score| score.clear),
            371 => self.rival_score_data_ref().map_or(-1, |score| score.clear),
            390..=399 => self.ranking_score_clear_type(id - 390),
            400 => self
                .current_play_config_ref()
                .map_or(-1, |config| if config.enable_constant { 1 } else { 0 }),
            450..=459 => self.lane_shuffle_pattern_value(0, (id - 450) as usize),
            460..=469 => self.lane_shuffle_pattern_value(1, (id - 460) as usize),
            _ => self.integer_value(id),
        }
    }

    /// Returns the boolean property value for the given ID.
    fn boolean_value(&self, _id: i32) -> bool {
        false
    }

    /// Default implementation for song-data-derived boolean property IDs.
    ///
    /// Computes boolean values for IDs that depend on `song_data_ref()`:
    /// - 150-155: chart difficulty
    /// - 160-164, 1160-1161: chart mode (key type)
    /// - 170-171: BGA presence
    /// - 172-173: long note presence
    /// - 174-175: text/document presence
    /// - 176-177: BPM change
    /// - 1177: BPM stop
    /// - 178-179: random sequence
    /// - 180-184: judge difficulty
    ///
    /// Callers that override `boolean_value()` should fall through to this
    /// for unmatched IDs instead of returning `false`, so that song-data
    /// booleans are correctly evaluated via `song_data_ref()`.
    fn default_boolean_value(&self, id: i32) -> bool {
        let Some(song) = self.song_data_ref() else {
            return false;
        };
        let chart = &song.chart;
        match id {
            // Difficulty
            150 => chart.difficulty <= 0 || chart.difficulty > 5, // OPTION_DIFFICULTY0
            151 => chart.difficulty == 1,                         // OPTION_DIFFICULTY1
            152 => chart.difficulty == 2,                         // OPTION_DIFFICULTY2
            153 => chart.difficulty == 3,                         // OPTION_DIFFICULTY3
            154 => chart.difficulty == 4,                         // OPTION_DIFFICULTY4
            155 => chart.difficulty == 5,                         // OPTION_DIFFICULTY5
            // Chart mode (key type)
            160 => chart.mode == 7,   // OPTION_7KEYSONG (BEAT_7K)
            161 => chart.mode == 5,   // OPTION_5KEYSONG (BEAT_5K)
            162 => chart.mode == 14,  // OPTION_14KEYSONG (BEAT_14K)
            163 => chart.mode == 10,  // OPTION_10KEYSONG (BEAT_10K)
            164 => chart.mode == 9,   // OPTION_9KEYSONG (POPN_9K)
            1160 => chart.mode == 25, // OPTION_24KEYSONG (KEYBOARD_24K)
            1161 => chart.mode == 50, // OPTION_24KEYDPSONG (KEYBOARD_24K_DOUBLE)
            // BGA presence
            170 => !chart.has_bga(), // OPTION_NO_BGA
            171 => chart.has_bga(),  // OPTION_BGA
            // Long note presence
            172 => !chart.has_any_long_note(), // OPTION_NO_LN
            173 => chart.has_any_long_note(),  // OPTION_LN
            // Text/document presence
            174 => !chart.has_document(), // OPTION_NO_TEXT
            175 => chart.has_document(),  // OPTION_TEXT
            // BPM change
            176 => chart.minbpm == chart.maxbpm, // OPTION_NO_BPMCHANGE
            177 => chart.minbpm < chart.maxbpm,  // OPTION_BPMCHANGE
            1177 => chart.is_bpmstop(),          // OPTION_BPMSTOP
            // Random sequence
            178 => !chart.has_random_sequence(), // OPTION_NO_RANDOMSEQUENCE
            179 => chart.has_random_sequence(),  // OPTION_RANDOMSEQUENCE
            // Judge difficulty
            180 => chart.judge == 0 || (chart.judge >= 10 && chart.judge < 35), // OPTION_JUDGE_VERYHARD
            181 => chart.judge == 1 || (chart.judge >= 35 && chart.judge < 60), // OPTION_JUDGE_HARD
            182 => chart.judge == 2 || (chart.judge >= 60 && chart.judge < 85), // OPTION_JUDGE_NORMAL
            183 => chart.judge == 3 || (chart.judge >= 85 && chart.judge < 110), // OPTION_JUDGE_EASY
            184 => chart.judge == 4 || chart.judge >= 110, // OPTION_JUDGE_VERYEASY
            _ => false,
        }
    }

    /// Returns the float property value for the given ID.
    fn float_value(&self, _id: i32) -> f32 {
        0.0
    }

    /// Shared default implementation for float property values.
    ///
    /// Handles float properties that can be computed from `song_data_ref()`,
    /// `player_config_ref()`, `config_ref()`, and `current_play_config_ref()`.
    /// Contexts should call this as a fallback from `float_value()` when no
    /// state-specific override applies.
    fn default_float_value(&self, id: i32) -> f32 {
        match id {
            // hispeed (310): Java FloatPropertyFactory reads from PlayConfig when not BMSPlayer
            310 => self
                .current_play_config_ref()
                .map_or(f32::MIN, |pc| pc.hispeed),
            // chart_peakdensity (360)
            360 => self
                .song_data_ref()
                .and_then(|s| s.info.as_ref())
                .map_or(f32::MIN, |i| i.peakdensity as f32),
            // chart_enddensity (362)
            362 => self
                .song_data_ref()
                .and_then(|s| s.info.as_ref())
                .map_or(f32::MIN, |i| i.enddensity as f32),
            // chart_averagedensity (367)
            367 => self
                .song_data_ref()
                .and_then(|s| s.info.as_ref())
                .map_or(f32::MIN, |i| i.density as f32),
            // chart_totalgauge (368)
            368 => self
                .song_data_ref()
                .and_then(|s| s.info.as_ref())
                .map_or(f32::MIN, |i| i.total as f32),
            _ => 0.0,
        }
    }

    /// Returns the string property value for the given ID.
    fn string_value(&self, _id: i32) -> String {
        String::new()
    }

    /// Returns replay option data when the current state exposes it.
    fn replay_option_data(&self) -> Option<&crate::replay_data::ReplayData> {
        None
    }

    /// Returns target score data when the current state exposes it.
    fn target_score_data(&self) -> Option<&crate::score_data::ScoreData> {
        None
    }

    /// Returns the current score data when the current state exposes it.
    fn score_data_ref(&self) -> Option<&crate::score_data::ScoreData> {
        None
    }

    /// Returns the comparison score data when the current state exposes it.
    fn rival_score_data_ref(&self) -> Option<&crate::score_data::ScoreData> {
        None
    }

    /// Returns the clear type ID for the ranking score at the given slot
    /// (0-based index relative to the current ranking offset).
    /// Used by image_index IDs 390-399 (cleartype_ranking1-10).
    /// Returns -1 when ranking data is unavailable or the slot is out of range.
    fn ranking_score_clear_type(&self, _slot: i32) -> i32 {
        -1
    }

    /// Returns the current ranking display offset.
    /// Used together with `ranking_score_clear_type` to compute absolute indices.
    fn ranking_offset(&self) -> i32 {
        0
    }

    /// Returns the play config currently associated with the state.
    fn current_play_config_ref(&self) -> Option<&crate::play_config::PlayConfig> {
        None
    }

    /// Returns the active song data when the current state exposes it.
    fn song_data_ref(&self) -> Option<&crate::song_data::SongData> {
        None
    }

    /// Returns the lane shuffle pattern value for the given player (0=1P, 1=2P) and lane index.
    /// Used by image-index IDs 450-459 (1P lanes) and 460-469 (2P lanes).
    /// Returns -1 when lane shuffle data is unavailable or the indices are out of range.
    fn lane_shuffle_pattern_value(&self, _player: usize, _lane: usize) -> i32 {
        -1
    }

    /// Returns the LR2 image index for the mode selector when available.
    fn mode_image_index(&self) -> Option<i32> {
        None
    }

    /// Returns the image index for the current sort mode when available.
    fn sort_image_index(&self) -> Option<i32> {
        None
    }

    /// Sets the float property value for the given ID.
    fn set_float_value(&mut self, _id: i32, _value: f32) {
        // default no-op
    }

    // ============================================================
    // Gameplay state queries
    // ============================================================

    /// Returns the judge count for the given judge index.
    fn judge_count(&self, _judge: i32, _fast: bool) -> i32 {
        0
    }

    /// Returns the gauge value (0.0-1.0).
    fn gauge_value(&self) -> f32 {
        0.0
    }

    /// Returns the gauge type ID.
    fn gauge_type(&self) -> i32 {
        0
    }

    /// Returns whether the chart's original mode differs from the current mode
    /// (e.g. 7-key chart converted to 9-key via chart options).
    /// Used by SkinGauge to adjust parts count for border alignment.
    fn is_mode_changed(&self) -> bool {
        false
    }

    /// Returns (border, max) for each gauge type.
    /// Used by SkinGauge to adjust parts count so borders divide evenly.
    fn gauge_element_borders(&self) -> Vec<(f32, f32)> {
        Vec::new()
    }

    /// Returns the current judge type for the given player.
    fn now_judge(&self, _player: i32) -> i32 {
        0
    }

    /// Returns the current combo count for the given player.
    fn now_combo(&self, _player: i32) -> i32 {
        0
    }

    // ============================================================
    // Config access
    // ============================================================

    /// Returns immutable reference to the player config.
    fn player_config_ref(&self) -> Option<&crate::player_config::PlayerConfig> {
        None
    }

    /// Returns mutable reference to the player config when the current state allows editing it.
    fn player_config_mut(&mut self) -> Option<&mut crate::player_config::PlayerConfig> {
        None
    }

    /// Returns immutable reference to the global config.
    fn config_ref(&self) -> Option<&crate::config::Config> {
        None
    }

    /// Returns mutable reference to the global config when the current state allows editing it.
    fn config_mut(&mut self) -> Option<&mut crate::config::Config> {
        None
    }

    /// Returns mutable reference to the selected play config when available.
    fn selected_play_config_mut(&mut self) -> Option<&mut crate::play_config::PlayConfig> {
        None
    }

    /// Propagate the current audio config to the audio driver.
    /// Called after Lua `set_volume_*` functions modify `config_mut().audio` so
    /// the changes reach MainController (same pattern as the UI volume slider fix).
    fn notify_audio_config_changed(&mut self) {
        // default no-op
    }

    /// Plays the option change sound for click/slider-driven config changes.
    fn play_option_change_sound(&mut self) {
        // default no-op
    }

    /// Refreshes bar UI after a config change on music select.
    fn update_bar_after_change(&mut self) {
        // default no-op
    }

    /// Starts song selection for a built-in click event.
    /// Uses the skin event ID to avoid introducing a core dependency here.
    fn select_song_mode(&mut self, _event_id: i32) {
        // default no-op
    }

    // ============================================================
    // Offset access
    // ============================================================

    /// Returns the skin offset for the given ID.
    /// Replaces `get_offset_value` from MainState.
    fn get_offset_value(&self, _id: i32) -> Option<&crate::skin_offset::SkinOffset> {
        None
    }

    // ============================================================
    // Mouse position
    // ============================================================

    /// Returns the current mouse X position.
    fn mouse_x(&self) -> f32 {
        0.0
    }

    /// Returns the current mouse Y position.
    fn mouse_y(&self) -> f32 {
        0.0
    }

    // ============================================================
    // Display config
    // ============================================================

    /// Returns the prepare frame-per-second value from config.
    ///
    /// Java Skin.java:241 reads `state.main.getConfig().getPrepareFramePerSecond()`.
    /// When 0, `prepareduration` becomes 1 (every frame).
    fn prepare_fps(&self) -> i32 {
        self.config_ref()
            .map_or(0, |c| c.display.prepare_frame_per_second)
    }

    /// Returns whether debug mode is active.
    fn is_debug(&self) -> bool {
        false
    }

    // ============================================================
    // Timing distribution (for result screens)
    // ============================================================

    /// Returns the timing distribution data when available.
    fn get_timing_distribution(&self) -> Option<&crate::timing_distribution::TimingDistribution> {
        None
    }

    /// Returns the judge area (timing windows per judge level) computed from
    /// the current BMS model and player resource.
    /// Used by SkinTimingDistributionGraph and SkinHitErrorVisualizer.
    fn judge_area(&self) -> Option<Vec<Vec<i32>>> {
        None
    }

    // ============================================================
    // Score data property (for Lua rate/exscore accessors)
    // ============================================================

    /// Returns the ScoreDataProperty for the current state.
    fn score_data_property(&self) -> &crate::score_data_property::ScoreDataProperty {
        static DEFAULT: std::sync::OnceLock<crate::score_data_property::ScoreDataProperty> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(crate::score_data_property::ScoreDataProperty::default)
    }

    // ============================================================
    // Gauge history (for result screens)
    // ============================================================

    /// Returns the gauge history (per-frame gauge values per gauge type).
    fn gauge_history(&self) -> Option<&Vec<Vec<f32>>> {
        None
    }

    /// Returns the course gauge history (one entry per course stage).
    fn course_gauge_history(&self) -> &[Vec<Vec<f32>>] {
        &[]
    }

    /// Returns (border, max) for the current gauge type's properties.
    fn gauge_border_max(&self) -> Option<(f32, f32)> {
        None
    }

    /// Returns the minimum gauge value for the current gauge type.
    /// Used by SkinGauge for the result-screen fill animation (Java: getProperty().min).
    fn gauge_min(&self) -> f32 {
        0.0
    }

    /// Returns the gauge type for result screen rendering.
    fn result_gauge_type(&self) -> i32 {
        self.gauge_type()
    }

    /// Returns whether the gauge reached max value.
    fn is_gauge_max(&self) -> bool {
        false
    }

    // ============================================================
    // Media/practice state
    // ============================================================

    /// Returns whether media loading has finished.
    fn is_media_load_finished(&self) -> bool {
        false
    }

    /// Returns whether the current mode is practice mode.
    fn is_practice_mode(&self) -> bool {
        false
    }

    // ============================================================
    // Distribution data (for folder lamp/rank graphs)
    // ============================================================

    /// Returns distribution data for the current folder selection.
    fn get_distribution_data(&self) -> Option<crate::distribution_data::DistributionData> {
        None
    }

    // ============================================================
    // BMSPlayer state check
    // ============================================================

    /// Returns true when the current state is BMSPlayer (Play state).
    fn is_bms_player(&self) -> bool {
        matches!(
            self.current_state_type(),
            Some(crate::main_state_type::MainStateType::Play)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer_access::TimerAccess;
    use crate::timer_id::TimerId;

    /// Minimal stub implementing SkinRenderContext for testing default_image_index_value.
    struct TestContext {
        ranking_clear_types: Vec<i32>,
        ranking_offset: i32,
    }

    impl TestContext {
        fn new() -> Self {
            Self {
                ranking_clear_types: Vec::new(),
                ranking_offset: 0,
            }
        }

        fn with_ranking(clear_types: Vec<i32>, offset: i32) -> Self {
            Self {
                ranking_clear_types: clear_types,
                ranking_offset: offset,
            }
        }
    }

    impl TimerAccess for TestContext {
        fn now_time(&self) -> i64 {
            0
        }
        fn now_micro_time(&self) -> i64 {
            0
        }
        fn micro_timer(&self, _: TimerId) -> i64 {
            i64::MIN
        }
        fn timer(&self, _: TimerId) -> i64 {
            i64::MIN
        }
        fn now_time_for(&self, _: TimerId) -> i64 {
            i64::MIN
        }
        fn is_timer_on(&self, _: TimerId) -> bool {
            false
        }
    }

    impl SkinRenderContext for TestContext {
        fn ranking_score_clear_type(&self, slot: i32) -> i32 {
            let index = (self.ranking_offset + slot) as usize;
            self.ranking_clear_types.get(index).copied().unwrap_or(-1)
        }

        fn ranking_offset(&self) -> i32 {
            self.ranking_offset
        }

        fn current_play_config_ref(&self) -> Option<&crate::play_config::PlayConfig> {
            // Not easily constructible in a unit test without a static.
            // Use image_index_value override for the constant test instead.
            None
        }
    }

    #[test]
    fn default_image_index_390_to_399_delegate_to_ranking_score_clear_type() {
        let ctx = TestContext::with_ranking(vec![8, 6, 5, 4, 3, 2, 1, 0, 9, 10], 0);

        for (slot, expected) in [8, 6, 5, 4, 3, 2, 1, 0, 9, 10].iter().enumerate() {
            let id = 390 + slot as i32;
            assert_eq!(
                ctx.default_image_index_value(id),
                *expected,
                "ID {} (slot {}) should return {}",
                id,
                slot,
                expected
            );
        }
    }

    #[test]
    fn default_image_index_390_with_offset() {
        // 3 scores, offset=1 -> slot 0 reads index 1, slot 1 reads index 2, slot 2 -> -1
        let ctx = TestContext::with_ranking(vec![8, 6, 5], 1);

        assert_eq!(ctx.default_image_index_value(390), 6);
        assert_eq!(ctx.default_image_index_value(391), 5);
        assert_eq!(ctx.default_image_index_value(392), -1);
    }

    #[test]
    fn default_image_index_390_returns_minus_one_when_no_ranking() {
        let ctx = TestContext::new();
        for id in 390..=399 {
            assert_eq!(
                ctx.default_image_index_value(id),
                -1,
                "ID {} should return -1",
                id
            );
        }
    }

    #[test]
    fn default_image_index_400_returns_constant_flag() {
        // To test ID 400 properly, we need a PlayConfig. Let's test via a context
        // that has one. We'll use a static PlayConfig.
        use std::sync::OnceLock;
        static PLAY_CONFIG_ENABLED: OnceLock<crate::play_config::PlayConfig> = OnceLock::new();
        static PLAY_CONFIG_DISABLED: OnceLock<crate::play_config::PlayConfig> = OnceLock::new();

        struct ConstantTestContext {
            config: &'static crate::play_config::PlayConfig,
        }

        impl TimerAccess for ConstantTestContext {
            fn now_time(&self) -> i64 {
                0
            }
            fn now_micro_time(&self) -> i64 {
                0
            }
            fn micro_timer(&self, _: TimerId) -> i64 {
                i64::MIN
            }
            fn timer(&self, _: TimerId) -> i64 {
                i64::MIN
            }
            fn now_time_for(&self, _: TimerId) -> i64 {
                i64::MIN
            }
            fn is_timer_on(&self, _: TimerId) -> bool {
                false
            }
        }

        impl SkinRenderContext for ConstantTestContext {
            fn current_play_config_ref(&self) -> Option<&crate::play_config::PlayConfig> {
                Some(self.config)
            }
        }

        let enabled = PLAY_CONFIG_ENABLED.get_or_init(|| crate::play_config::PlayConfig {
            enable_constant: true,
            ..crate::play_config::PlayConfig::default()
        });
        let disabled = PLAY_CONFIG_DISABLED.get_or_init(|| crate::play_config::PlayConfig {
            enable_constant: false,
            ..crate::play_config::PlayConfig::default()
        });

        let ctx_on = ConstantTestContext { config: enabled };
        assert_eq!(ctx_on.default_image_index_value(400), 1);

        let ctx_off = ConstantTestContext { config: disabled };
        assert_eq!(ctx_off.default_image_index_value(400), 0);
    }

    #[test]
    fn default_image_index_400_returns_minus_one_when_no_play_config() {
        let ctx = TestContext::new();
        assert_eq!(ctx.default_image_index_value(400), -1);
    }

    #[test]
    fn default_image_index_450_to_469_delegate_to_lane_shuffle_pattern() {
        struct PatternTestContext {
            patterns: Vec<Vec<i32>>,
        }

        impl TimerAccess for PatternTestContext {
            fn now_time(&self) -> i64 {
                0
            }
            fn now_micro_time(&self) -> i64 {
                0
            }
            fn micro_timer(&self, _: TimerId) -> i64 {
                i64::MIN
            }
            fn timer(&self, _: TimerId) -> i64 {
                i64::MIN
            }
            fn now_time_for(&self, _: TimerId) -> i64 {
                i64::MIN
            }
            fn is_timer_on(&self, _: TimerId) -> bool {
                false
            }
        }

        impl SkinRenderContext for PatternTestContext {
            fn lane_shuffle_pattern_value(&self, player: usize, lane: usize) -> i32 {
                self.patterns
                    .get(player)
                    .and_then(|lanes| lanes.get(lane))
                    .copied()
                    .unwrap_or(-1)
            }
        }

        let ctx = PatternTestContext {
            patterns: vec![
                vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 7],
                vec![8, 6, 7, 5, 3, 0, 9, 4, 2, 1],
            ],
        };

        // 1P lanes (IDs 450-459)
        assert_eq!(ctx.default_image_index_value(450), 3);
        assert_eq!(ctx.default_image_index_value(451), 1);
        assert_eq!(ctx.default_image_index_value(459), 7);

        // 2P lanes (IDs 460-469)
        assert_eq!(ctx.default_image_index_value(460), 8);
        assert_eq!(ctx.default_image_index_value(461), 6);
        assert_eq!(ctx.default_image_index_value(469), 1);
    }

    #[test]
    fn default_image_index_450_returns_minus_one_when_no_pattern() {
        let ctx = TestContext::new();
        assert_eq!(ctx.default_image_index_value(450), -1);
        assert_eq!(ctx.default_image_index_value(460), -1);
    }

    // ============================================================
    // compute_lnmode_from_chart tests
    // ============================================================

    #[test]
    fn lnmode_from_chart_longnote_returns_0() {
        use crate::song_data::{ChartInfo, FEATURE_LONGNOTE};
        let chart = ChartInfo {
            feature: FEATURE_LONGNOTE,
            ..ChartInfo::default()
        };
        assert_eq!(compute_lnmode_from_chart(&chart), Some(0));
    }

    #[test]
    fn lnmode_from_chart_chargenote_returns_1() {
        use crate::song_data::{ChartInfo, FEATURE_CHARGENOTE};
        let chart = ChartInfo {
            feature: FEATURE_CHARGENOTE,
            ..ChartInfo::default()
        };
        assert_eq!(compute_lnmode_from_chart(&chart), Some(1));
    }

    #[test]
    fn lnmode_from_chart_hellchargenote_returns_2() {
        use crate::song_data::{ChartInfo, FEATURE_HELLCHARGENOTE};
        let chart = ChartInfo {
            feature: FEATURE_HELLCHARGENOTE,
            ..ChartInfo::default()
        };
        assert_eq!(compute_lnmode_from_chart(&chart), Some(2));
    }

    #[test]
    fn lnmode_from_chart_undefined_ln_returns_none() {
        use crate::song_data::{ChartInfo, FEATURE_UNDEFINEDLN};
        let chart = ChartInfo {
            feature: FEATURE_UNDEFINEDLN,
            ..ChartInfo::default()
        };
        assert_eq!(compute_lnmode_from_chart(&chart), None);
    }

    #[test]
    fn lnmode_from_chart_no_ln_returns_none() {
        use crate::song_data::ChartInfo;
        let chart = ChartInfo::default();
        assert_eq!(compute_lnmode_from_chart(&chart), None);
    }

    #[test]
    fn lnmode_from_chart_longnote_plus_undefined_returns_none() {
        use crate::song_data::{ChartInfo, FEATURE_LONGNOTE, FEATURE_UNDEFINEDLN};
        // Both LN and undefined set: has_any_long_note is true but
        // has_undefined_long_note is also true, so no override.
        let chart = ChartInfo {
            feature: FEATURE_LONGNOTE | FEATURE_UNDEFINEDLN,
            ..ChartInfo::default()
        };
        assert_eq!(compute_lnmode_from_chart(&chart), None);
    }

    #[test]
    fn lnmode_from_chart_longnote_plus_chargenote_returns_0() {
        use crate::song_data::{ChartInfo, FEATURE_CHARGENOTE, FEATURE_LONGNOTE};
        // Both LN and CN set: has_long_note() is checked first, returns 0.
        let chart = ChartInfo {
            feature: FEATURE_LONGNOTE | FEATURE_CHARGENOTE,
            ..ChartInfo::default()
        };
        assert_eq!(compute_lnmode_from_chart(&chart), Some(0));
    }
}

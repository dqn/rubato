use crate::main_state_type::MainStateType;
use crate::timer_access::TimerAccess;

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
    fn set_timer_micro(&mut self, _timer_id: i32, _micro_time: i64) {
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
    fn get_recent_judges(&self) -> &[i64] {
        &[]
    }

    /// Returns the current write index into the recent judges circular buffer.
    fn get_recent_judges_index(&self) -> usize {
        0
    }

    // ============================================================
    // Property value delegation (skin property factories)
    // ============================================================

    /// Returns the integer property value for the given ID.
    /// Delegate properties call this via MainState::integer_value().
    fn integer_value(&self, _id: i32) -> i32 {
        0
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
        let player_config = self.get_player_config_ref();
        let target_image_index = player_config.map_or(-1, |config| {
            config
                .targetlist
                .iter()
                .position(|target| target == &config.targetid)
                .map(|index| index.min(10) as i32)
                .unwrap_or(0)
        });

        match id {
            11 => self.get_mode_image_index().unwrap_or(-1),
            12 => self.get_sort_image_index().unwrap_or(-1),
            40 => {
                if matches!(
                    self.current_state_type(),
                    Some(MainStateType::Play | MainStateType::Result | MainStateType::CourseResult)
                ) {
                    self.get_gauge_type()
                } else {
                    player_config.map_or(-1, |config| config.gauge)
                }
            }
            42 => self.get_replay_option_data().map_or_else(
                || player_config.map_or(-1, |config| config.random),
                |replay| replay.randomoption,
            ),
            43 => self.get_replay_option_data().map_or_else(
                || player_config.map_or(-1, |config| config.random2),
                |replay| replay.randomoption2,
            ),
            54 => self.get_replay_option_data().map_or_else(
                || player_config.map_or(-1, |config| config.doubleoption),
                |replay| replay.doubleoption,
            ),
            55 => self
                .get_current_play_config_ref()
                .map_or(-1, |config| config.fixhispeed),
            61 => self.get_target_score_data().map_or(-1, |score| {
                if score.option >= 0 {
                    score.option % 10
                } else {
                    -1
                }
            }),
            62 => self.get_target_score_data().map_or(-1, |score| {
                if score.option >= 0 {
                    (score.option / 10) % 10
                } else {
                    -1
                }
            }),
            63 => self.get_target_score_data().map_or(-1, |score| {
                if score.option >= 0 {
                    (score.option / 100) % 10
                } else {
                    -1
                }
            }),
            72 => self.get_config_ref().map_or(-1, |config| config.bga),
            75 => player_config.map_or(-1, |config| {
                bool_to_i32(config.notes_display_timing_auto_adjust)
            }),
            77 => target_image_index,
            78 => player_config.map_or(-1, |config| config.gauge_auto_shift),
            89 => self.get_song_data_ref().map_or(-1, |song| {
                let favorite = song.favorite;
                if favorite & crate::song_data::INVISIBLE_SONG != 0 {
                    2
                } else if favorite & crate::song_data::FAVORITE_SONG != 0 {
                    1
                } else {
                    0
                }
            }),
            90 => self.get_song_data_ref().map_or(-1, |song| {
                let favorite = song.favorite;
                if favorite & crate::song_data::INVISIBLE_CHART != 0 {
                    2
                } else if favorite & crate::song_data::FAVORITE_CHART != 0 {
                    1
                } else {
                    0
                }
            }),
            301 => player_config.map_or(-1, |config| bool_to_i32(config.custom_judge)),
            303 => player_config.map_or(-1, |config| bool_to_i32(config.showjudgearea)),
            305 => player_config.map_or(-1, |config| bool_to_i32(config.markprocessednote)),
            306 => player_config.map_or(-1, |config| bool_to_i32(config.bpmguide)),
            308 => player_config.map_or(-1, |config| config.lnmode),
            330 => self
                .get_current_play_config_ref()
                .map_or(-1, |config| bool_to_i32(config.enablelanecover)),
            331 => self
                .get_current_play_config_ref()
                .map_or(-1, |config| bool_to_i32(config.enablelift)),
            332 => self
                .get_current_play_config_ref()
                .map_or(-1, |config| bool_to_i32(config.enablehidden)),
            340 => self.get_current_play_config_ref().map_or(-1, |config| {
                match config.judgetype.as_str() {
                    "Combo" => 0,
                    "Duration" => 1,
                    "Lowest" => 2,
                    _ => -1,
                }
            }),
            321..=324 => player_config
                .and_then(|config| config.autosavereplay.get((id - 321) as usize).copied())
                .unwrap_or(-1),
            341 => player_config.map_or(-1, |config| config.bottom_shiftable_gauge),
            342 => self
                .get_current_play_config_ref()
                .map_or(-1, |config| bool_to_i32(config.hispeedautoadjust)),
            343 => player_config.map_or(-1, |config| bool_to_i32(config.is_guide_se)),
            350 => player_config.map_or(-1, |config| config.extranote_depth),
            351 => player_config.map_or(-1, |config| config.mine_mode),
            352 => player_config.map_or(-1, |config| config.scroll_mode),
            353 => player_config.map_or(-1, |config| config.longnote_mode),
            360 => player_config.map_or(-1, |config| config.seven_to_nine_pattern),
            361 => player_config.map_or(-1, |config| config.seven_to_nine_type),
            370 => self.get_score_data_ref().map_or(-1, |score| score.clear),
            371 => self
                .get_rival_score_data_ref()
                .map_or(-1, |score| score.clear),
            _ => self.integer_value(id),
        }
    }

    /// Returns the boolean property value for the given ID.
    fn boolean_value(&self, _id: i32) -> bool {
        false
    }

    /// Returns the float property value for the given ID.
    fn float_value(&self, _id: i32) -> f32 {
        0.0
    }

    /// Returns the string property value for the given ID.
    fn string_value(&self, _id: i32) -> String {
        String::new()
    }

    /// Returns replay option data when the current state exposes it.
    fn get_replay_option_data(&self) -> Option<&crate::replay_data::ReplayData> {
        None
    }

    /// Returns target score data when the current state exposes it.
    fn get_target_score_data(&self) -> Option<&crate::score_data::ScoreData> {
        None
    }

    /// Returns the current score data when the current state exposes it.
    fn get_score_data_ref(&self) -> Option<&crate::score_data::ScoreData> {
        None
    }

    /// Returns the comparison score data when the current state exposes it.
    fn get_rival_score_data_ref(&self) -> Option<&crate::score_data::ScoreData> {
        None
    }

    /// Returns the play config currently associated with the state.
    fn get_current_play_config_ref(&self) -> Option<&crate::play_config::PlayConfig> {
        None
    }

    /// Returns the active song data when the current state exposes it.
    fn get_song_data_ref(&self) -> Option<&crate::song_data::SongData> {
        None
    }

    /// Returns the LR2 image index for the mode selector when available.
    fn get_mode_image_index(&self) -> Option<i32> {
        None
    }

    /// Returns the image index for the current sort mode when available.
    fn get_sort_image_index(&self) -> Option<i32> {
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
    fn get_judge_count(&self, _judge: i32, _fast: bool) -> i32 {
        0
    }

    /// Returns the gauge value (0.0-1.0).
    fn get_gauge_value(&self) -> f32 {
        0.0
    }

    /// Returns the gauge type ID.
    fn get_gauge_type(&self) -> i32 {
        0
    }

    /// Returns the current judge type for the given player.
    fn get_now_judge(&self, _player: i32) -> i32 {
        0
    }

    /// Returns the current combo count for the given player.
    fn get_now_combo(&self, _player: i32) -> i32 {
        0
    }

    // ============================================================
    // Config access
    // ============================================================

    /// Returns immutable reference to the player config.
    fn get_player_config_ref(&self) -> Option<&crate::player_config::PlayerConfig> {
        None
    }

    /// Returns mutable reference to the player config when the current state allows editing it.
    fn get_player_config_mut(&mut self) -> Option<&mut crate::player_config::PlayerConfig> {
        None
    }

    /// Returns immutable reference to the global config.
    fn get_config_ref(&self) -> Option<&crate::config::Config> {
        None
    }

    /// Returns mutable reference to the global config when the current state allows editing it.
    fn get_config_mut(&mut self) -> Option<&mut crate::config::Config> {
        None
    }

    /// Returns mutable reference to the selected play config when available.
    fn get_selected_play_config_mut(&mut self) -> Option<&mut crate::play_config::PlayConfig> {
        None
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
}

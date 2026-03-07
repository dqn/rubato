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
    ///
    /// Arms are grouped into helper methods by functional domain.
    fn default_image_index_value(&self, id: i32) -> i32 {
        match id {
            11 => self.mode_image_index().unwrap_or(-1),
            12 => self.sort_image_index().unwrap_or(-1),
            40 | 42 | 43 | 54 | 55 => self.play_option_image_index(id),
            61..=63 => self.target_option_image_index(id),
            72 | 75 | 77 | 78 => self.config_image_index(id),
            89 | 90 => self.favorite_image_index(id),
            301 | 303 | 305 | 306 | 308 => self.display_option_image_index(id),
            321..=324 => self.autosave_replay_image_index(id),
            330..=332 | 340..=343 => self.play_config_image_index(id),
            350..=353 | 360 | 361 => self.note_option_image_index(id),
            370 | 371 => self.clear_image_index(id),
            _ => self.integer_value(id),
        }
    }

    /// Gauge type, random options, double option, hispeed fix.
    fn play_option_image_index(&self, id: i32) -> i32 {
        let player_config = self.player_config_ref();
        match id {
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
            _ => self.integer_value(id),
        }
    }

    /// Target score option digit extraction (ones, tens, hundreds).
    fn target_option_image_index(&self, id: i32) -> i32 {
        let divisor = match id {
            61 => 1,
            62 => 10,
            63 => 100,
            _ => return self.integer_value(id),
        };
        self.target_score_data().map_or(-1, |score| {
            if score.play_option.option >= 0 {
                (score.play_option.option / divisor) % 10
            } else {
                -1
            }
        })
    }

    /// BGA, timing auto-adjust, target index, gauge auto-shift.
    fn config_image_index(&self, id: i32) -> i32 {
        let player_config = self.player_config_ref();
        match id {
            72 => self.config_ref().map_or(-1, |config| config.render.bga),
            75 => player_config.map_or(-1, |config| {
                i32::from(config.judge_settings.notes_display_timing_auto_adjust)
            }),
            77 => player_config.map_or(-1, |config| {
                config
                    .select_settings
                    .targetlist
                    .iter()
                    .position(|target| target == &config.select_settings.targetid)
                    .map(|index| index.min(10) as i32)
                    .unwrap_or(0)
            }),
            78 => player_config.map_or(-1, |config| config.play_settings.gauge_auto_shift),
            _ => self.integer_value(id),
        }
    }

    /// Song/chart favorite status.
    fn favorite_image_index(&self, id: i32) -> i32 {
        self.song_data_ref().map_or(-1, |song| {
            let favorite = song.favorite;
            let (invisible_mask, favorite_mask) = match id {
                89 => (
                    crate::song_data::INVISIBLE_SONG,
                    crate::song_data::FAVORITE_SONG,
                ),
                90 => (
                    crate::song_data::INVISIBLE_CHART,
                    crate::song_data::FAVORITE_CHART,
                ),
                _ => return self.integer_value(id),
            };
            if favorite & invisible_mask != 0 {
                2
            } else if favorite & favorite_mask != 0 {
                1
            } else {
                0
            }
        })
    }

    /// Custom judge, judge area, mark processed note, BPM guide, LN mode.
    fn display_option_image_index(&self, id: i32) -> i32 {
        let player_config = self.player_config_ref();
        match id {
            301 => player_config.map_or(-1, |c| i32::from(c.judge_settings.custom_judge)),
            303 => player_config.map_or(-1, |c| i32::from(c.display_settings.showjudgearea)),
            305 => player_config.map_or(-1, |c| i32::from(c.display_settings.markprocessednote)),
            306 => player_config.map_or(-1, |c| i32::from(c.display_settings.bpmguide)),
            308 => player_config.map_or(-1, |c| c.play_settings.lnmode),
            _ => self.integer_value(id),
        }
    }

    /// Autosave replay array (IDs 321..=324).
    fn autosave_replay_image_index(&self, id: i32) -> i32 {
        self.player_config_ref()
            .and_then(|config| {
                config
                    .misc_settings
                    .autosavereplay
                    .get((id - 321) as usize)
                    .copied()
            })
            .unwrap_or(-1)
    }

    /// Lane cover, lift, hidden, judge type, bottom-shiftable gauge, hispeed auto-adjust, guide SE.
    fn play_config_image_index(&self, id: i32) -> i32 {
        let player_config = self.player_config_ref();
        match id {
            330 => self
                .current_play_config_ref()
                .map_or(-1, |pc| i32::from(pc.enablelanecover)),
            331 => self
                .current_play_config_ref()
                .map_or(-1, |pc| i32::from(pc.enablelift)),
            332 => self
                .current_play_config_ref()
                .map_or(-1, |pc| i32::from(pc.enablehidden)),
            340 => self.current_play_config_ref().map_or(-1, |pc| {
                match pc.judgetype.as_str() {
                    "Combo" => 0,
                    "Duration" => 1,
                    "Lowest" => 2,
                    _ => -1,
                }
            }),
            341 => player_config.map_or(-1, |c| c.play_settings.bottom_shiftable_gauge),
            342 => self
                .current_play_config_ref()
                .map_or(-1, |pc| i32::from(pc.hispeedautoadjust)),
            343 => player_config.map_or(-1, |c| i32::from(c.display_settings.is_guide_se)),
            _ => self.integer_value(id),
        }
    }

    /// Extra note depth, mine mode, scroll mode, long note mode, 7-to-9 pattern/type.
    fn note_option_image_index(&self, id: i32) -> i32 {
        let player_config = self.player_config_ref();
        match id {
            350 => player_config.map_or(-1, |c| c.display_settings.extranote_depth),
            351 => player_config.map_or(-1, |c| c.play_settings.mine_mode),
            352 => player_config.map_or(-1, |c| c.display_settings.scroll_mode),
            353 => player_config.map_or(-1, |c| c.note_modifier_settings.longnote_mode),
            360 => player_config.map_or(-1, |c| c.note_modifier_settings.seven_to_nine_pattern),
            361 => player_config.map_or(-1, |c| c.note_modifier_settings.seven_to_nine_type),
            _ => self.integer_value(id),
        }
    }

    /// Score/rival clear status.
    fn clear_image_index(&self, id: i32) -> i32 {
        match id {
            370 => self.score_data_ref().map_or(-1, |score| score.clear),
            371 => self.rival_score_data_ref().map_or(-1, |score| score.clear),
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

    /// Returns the play config currently associated with the state.
    fn current_play_config_ref(&self) -> Option<&crate::play_config::PlayConfig> {
        None
    }

    /// Returns the active song data when the current state exposes it.
    fn song_data_ref(&self) -> Option<&crate::song_data::SongData> {
        None
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

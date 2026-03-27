// Translated from MusicDecide.java
// Music decide screen state.

use crate::core::app_context::GameContext;
use crate::core::main_state::{MainState, MainStateData, MainStateType, StateTransition};
use crate::core::system_sound_manager::SoundType;
use crate::core::timer_manager::TimerManager;
use rubato_skin::skin_property::{TIMER_FADEOUT, TIMER_STARTINPUT};
use rubato_skin::skin_type::SkinType;
use rubato_types::property_snapshot::PropertySnapshot;
use rubato_types::skin_action_queue::SkinActionQueue;
use rubato_types::timer_id::TimerId;

use super::main_controller_ref::MainControllerRef;
use super::{ControlKeys, NullPlayerResource, PlayerResourceAccess};

/// Render context adapter for decide screen skin rendering.
/// Provides config access through SkinRenderContext.
/// Production code uses PropertySnapshot; this adapter is retained for tests.
#[cfg_attr(not(test), allow(dead_code))]
struct DecideRenderContext<'a> {
    timer: &'a mut TimerManager,
    resource: &'a mut dyn PlayerResourceAccess,
    main: &'a mut MainControllerRef,
    score_data_property: &'a rubato_types::score_data_property::ScoreDataProperty,
    offsets: &'a std::collections::HashMap<i32, rubato_types::skin_offset::SkinOffset>,
    /// Events collected during rendering for deferred dispatch.
    /// Skin timer callbacks and Lua code may call `execute_event()` during
    /// `draw_all_objects_timed`/`update_custom_objects_timed`, but the skin
    /// is `take()`-ed so `execute_custom_event` cannot be called directly.
    /// Events are replayed after the render block completes.
    pending_events: Vec<(i32, i32, i32)>,
}

impl rubato_types::timer_access::TimerAccess for DecideRenderContext<'_> {
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

impl rubato_types::skin_render_context::SkinRenderContext for DecideRenderContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Decide)
    }

    fn boot_time_millis(&self) -> i64 {
        self.timer.boot_time_millis()
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(self.main.player_config())
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(self.main.config())
    }

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.resource.songdata()
    }

    fn score_data_ref(&self) -> Option<&crate::core::score_data::ScoreData> {
        self.resource.score_data()
    }

    fn target_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
        self.resource.target_score_data()
    }

    fn rival_score_data_ref(&self) -> Option<&crate::core::score_data::ScoreData> {
        self.resource.rival_score_data()
    }

    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        self.resource.replay_data()
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        let mode = self
            .resource
            .songdata()
            .and_then(|song| match song.chart.mode {
                5 => Some(bms::model::mode::Mode::BEAT_5K),
                7 => Some(bms::model::mode::Mode::BEAT_7K),
                9 => Some(bms::model::mode::Mode::POPN_9K),
                10 => Some(bms::model::mode::Mode::BEAT_10K),
                14 => Some(bms::model::mode::Mode::BEAT_14K),
                25 => Some(bms::model::mode::Mode::KEYBOARD_24K),
                50 => Some(bms::model::mode::Mode::KEYBOARD_24K_DOUBLE),
                _ => None,
            })?;
        Some(
            &self
                .resource
                .player_config()
                .play_config_ref(mode)
                .playconfig,
        )
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn audio_play(&mut self, path: &str, volume: f32, is_loop: bool) {
        self.main.play_audio_path(path, volume, is_loop);
    }

    fn audio_stop(&mut self, path: &str) {
        self.main.stop_audio_path(path);
    }

    fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
        // Queue events for replay after the render block completes.
        // During rendering the skin is `take()`-ed, so we cannot call
        // `skin.execute_custom_event()` directly here.
        self.pending_events.push((id, arg1, arg2));
    }

    fn change_state(&mut self, state: rubato_types::main_state_type::MainStateType) {
        self.main.change_state(state);
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        self.resource.player_config_mut()
    }

    fn string_value(&self, id: i32) -> String {
        match id {
            10 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.title.clone()),
            11 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.subtitle.clone()),
            12 => self.resource.songdata().map_or_else(String::new, |s| {
                if s.metadata.subtitle.is_empty() {
                    s.metadata.title.clone()
                } else {
                    format!("{} {}", s.metadata.title, s.metadata.subtitle)
                }
            }),
            13 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.genre.clone()),
            14 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.artist.clone()),
            15 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.subartist.clone()),
            16 => self.resource.songdata().map_or_else(String::new, |s| {
                if s.metadata.subartist.is_empty() {
                    s.metadata.artist.clone()
                } else {
                    format!("{} {}", s.metadata.artist, s.metadata.subartist)
                }
            }),
            _ => String::new(),
        }
    }

    fn image_index_value(&self, id: i32) -> i32 {
        match id {
            // Java IntegerPropertyFactory ID 308 (lnmode): on Decide screen, override
            // from chart data when the chart explicitly defines LN types.
            308 => {
                if let Some(song) = self.resource.songdata()
                    && let Some(override_val) =
                        rubato_types::skin_render_context::compute_lnmode_from_chart(&song.chart)
                {
                    return override_val;
                }
                self.default_image_index_value(id)
            }
            _ => self.default_image_index_value(id),
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        match id {
            // ---- BGA on/off (OPTION_BGAOFF: 40 / OPTION_BGAON: 41) ----
            // Java: main.getConfig().getBga() == 2 (Off)
            40 => self.main.config().render.bga == rubato_types::config::BgaMode::Off,
            41 => self.main.config().render.bga != rubato_types::config::BgaMode::Off,
            // ---- Save score (OPTION_DISABLE_SAVE_SCORE: 60 / OPTION_ENABLE_SAVE_SCORE: 61) ----
            // Java: !resource.isUpdateScore() / resource.isUpdateScore()
            60 => !self.resource.is_update_score(),
            61 => self.resource.is_update_score(),
            // ---- Stagefile/banner/backbmp existence (190-195) ----
            // Java: songdata.getStagefile().length() == 0, etc.
            190 => self
                .resource
                .songdata()
                .is_none_or(|s| s.file.stagefile.is_empty()),
            191 => self
                .resource
                .songdata()
                .is_some_and(|s| !s.file.stagefile.is_empty()),
            192 => self
                .resource
                .songdata()
                .is_none_or(|s| s.file.banner.is_empty()),
            193 => self
                .resource
                .songdata()
                .is_some_and(|s| !s.file.banner.is_empty()),
            194 => self
                .resource
                .songdata()
                .is_none_or(|s| s.file.backbmp.is_empty()),
            195 => self
                .resource
                .songdata()
                .is_some_and(|s| !s.file.backbmp.is_empty()),
            // ---- Course stage (OPTION_COURSE_STAGE1-4: 280-283) ----
            // Java: resource.getCourseIndex() == stage
            280 => self.resource.course_data().is_some() && self.resource.course_index() == 0,
            281 => self.resource.course_data().is_some() && self.resource.course_index() == 1,
            282 => self.resource.course_data().is_some() && self.resource.course_index() == 2,
            283 => self.resource.course_data().is_some() && self.resource.course_index() == 3,
            // ---- Course stage final (OPTION_COURSE_STAGE_FINAL: 289) ----
            // Java: resource.getCourseData() != null &&
            //       resource.getCourseIndex() == resource.getCourseData().getSong().length - 1
            289 => {
                if let Some(cd) = self.resource.course_data() {
                    let song_count = cd.hash.len();
                    song_count > 0 && self.resource.course_index() == song_count - 1
                } else {
                    false
                }
            }
            // ---- Course mode (OPTION_MODE_COURSE: 290) ----
            // Java: resource.getCourseData() != null
            290 => self.resource.course_data().is_some(),
            _ => self.default_boolean_value(id),
        }
    }

    fn float_value(&self, id: i32) -> f32 {
        match id {
            // Volume (0.0-1.0) from audio config
            // Java: FloatPropertyFactory mastervolume/keyvolume/bgmvolume
            17 => self
                .main
                .config()
                .audio_config()
                .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                    a.systemvolume
                }),
            18 => self
                .main
                .config()
                .audio_config()
                .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                    a.keyvolume
                }),
            19 => self
                .main
                .config()
                .audio_config()
                .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                    a.bgvolume
                }),
            _ => self.default_float_value(id),
        }
    }

    fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
        self.score_data_property
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            // ---- Hi-speed (NUMBER_HISPEED_LR2: 10) ----
            // Java: (hispeed * 100) as int
            10 => self
                .current_play_config_ref()
                .map_or(i32::MIN, |pc| (pc.hispeed * 100.0) as i32),
            // ---- Judge timing (NUMBER_JUDGETIMING: 12) ----
            // Java: player.getJudgeConfig().getJudgetiming()
            12 => self.resource.player_config().judge_settings.judgetiming,
            // Volume (0-100 scale) from audio config
            // Java: IntegerPropertyFactory volume_system/volume_key/volume_background
            57 => {
                (self
                    .main
                    .config()
                    .audio_config()
                    .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                        a.systemvolume
                    })
                    * 100.0) as i32
            }
            58 => {
                (self
                    .main
                    .config()
                    .audio_config()
                    .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                        a.keyvolume
                    })
                    * 100.0) as i32
            }
            59 => {
                (self
                    .main
                    .config()
                    .audio_config()
                    .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                        a.bgvolume
                    })
                    * 100.0) as i32
            }
            // ---- EX score (NUMBER_SCORE / SCORE2 / SCORE3: 71/101/171) ----
            // Java: AbstractResult -> getNewScore().getExscore()
            71 | 101 | 171 => self.resource.score_data().map_or(i32::MIN, |s| s.exscore()),
            // ---- Max score (NUMBER_MAXSCORE: 72) ----
            // Java: score.getNotes() * 2
            72 => self.resource.score_data().map_or(i32::MIN, |s| s.notes * 2),
            // ---- Max combo (NUMBER_MAXCOMBO: 75) ----
            75 => self.resource.score_data().map_or(i32::MIN, |s| s.maxcombo),
            // ---- Miss count / minbp (NUMBER_MISSCOUNT: 76) ----
            76 => self.resource.score_data().map_or(i32::MIN, |s| s.minbp),
            // ---- Judge counts (NUMBER_PERFECT2..NUMBER_POOR2: 80-84) ----
            // Java: score != null ? score.getJudgeCount(index) : Integer.MIN_VALUE
            80..=84 => {
                let index = id - 80;
                self.resource
                    .score_data()
                    .map_or(i32::MIN, |s| s.judge_count_total(index))
            }
            // ---- Judge count rates (NUMBER_PERFECT_RATE..NUMBER_POOR_RATE: 85-89) ----
            // Java: score != null && notes > 0 ? count * 100 / notes : Integer.MIN_VALUE
            85..=89 => {
                let index = id - 85;
                self.resource.score_data().map_or(i32::MIN, |s| {
                    if s.notes > 0 {
                        s.judge_count_total(index) * 100 / s.notes
                    } else {
                        i32::MIN
                    }
                })
            }
            // Song BPM from songdata
            90 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| s.chart.maxbpm),
            91 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| s.chart.minbpm),
            // mainbpm: prefer SongInformation.mainbpm when available.
            // Java returns Integer.MIN_VALUE when SongInformation is absent,
            // signaling "no data" so skin renderers hide the value.
            92 => self.resource.songdata().map_or(i32::MIN, |s| {
                s.info
                    .as_ref()
                    .map(|i| i.mainbpm as i32)
                    .unwrap_or(i32::MIN)
            }),
            // Chart level
            96 => self.resource.songdata().map_or(i32::MIN, |s| s.chart.level),
            // ---- Point / score (NUMBER_POINT: 100) ----
            // Java: getScoreDataProperty().getNowScore()
            100 => self.score_data_property.now_score(),
            // ---- Score rate (NUMBER_SCORE_RATE: 102) ----
            // Java: score != null ? getNowRateInt() : Integer.MIN_VALUE
            102 => {
                if self.resource.score_data().is_some() {
                    self.score_data_property.nowrate_int
                } else {
                    i32::MIN
                }
            }
            // ---- Score rate afterdot (NUMBER_SCORE_RATE_AFTERDOT: 103) ----
            103 => {
                if self.resource.score_data().is_some() {
                    self.score_data_property.nowrate_after_dot
                } else {
                    i32::MIN
                }
            }
            // ---- Diff vs target (NUMBER_DIFF_EXSCORE / DIFF_EXSCORE2 / DIFF_TARGETSCORE: 108/128/153) ----
            // Java: nowEXScore - nowRivalScore
            108 | 128 | 153 => {
                self.score_data_property.nowscore - self.score_data_property.nowrivalscore
            }
            // ---- Total rate (NUMBER_TOTAL_RATE / NUMBER_SCORE_RATE2: 115/155) ----
            // Java: score != null ? getRateInt() : Integer.MIN_VALUE
            115 | 155 => {
                if self.resource.score_data().is_some() {
                    self.score_data_property.rate_int
                } else {
                    i32::MIN
                }
            }
            // ---- Total rate afterdot (NUMBER_TOTAL_RATE_AFTERDOT / NUMBER_SCORE_RATE_AFTERDOT2: 116/156) ----
            116 | 156 => {
                if self.resource.score_data().is_some() {
                    self.score_data_property.rate_after_dot
                } else {
                    i32::MIN
                }
            }
            // ---- Target / rival score (NUMBER_TARGET_SCORE / TARGET_SCORE2: 121/151) ----
            // Java: getScoreDataProperty().getRivalScore()
            121 | 151 => self.score_data_property.rivalscore,
            // ---- Target / rival score rate (NUMBER_TARGET_SCORE_RATE / TARGET_TOTAL_RATE: 122/157) ----
            122 | 157 => self.score_data_property.rivalrate_int,
            // ---- Target / rival score rate afterdot (123/158) ----
            123 | 158 => self.score_data_property.rivalrate_after_dot,
            // ---- Diff vs high score (NUMBER_DIFF_HIGHSCORE / DIFF_HIGHSCORE2: 152/172) ----
            // Java: nowEXScore - nowBestScore
            152 | 172 => self.score_data_property.nowscore - self.score_data_property.nowbestscore,
            // ---- Diff next rank (NUMBER_DIFF_NEXTRANK: 154) ----
            154 => self.score_data_property.nextrank,
            // ---- Best rate (NUMBER_BEST_RATE: 183) ----
            183 => self.score_data_property.bestrate_int,
            // ---- Best rate afterdot (NUMBER_BEST_RATE_AFTERDOT: 184) ----
            184 => self.score_data_property.bestrate_after_dot,
            // ---- Hi-speed integer part (NUMBER_HISPEED: 310) ----
            310 => self
                .current_play_config_ref()
                .map_or(i32::MIN, |pc| pc.hispeed as i32),
            // ---- Hi-speed afterdot (NUMBER_HISPEED_AFTERDOT: 311) ----
            311 => self
                .current_play_config_ref()
                .map_or(i32::MIN, |pc| ((pc.hispeed * 100.0) as i32) % 100),
            // Song duration
            312 => self.resource.songdata().map_or(0, |s| s.chart.length),
            // ---- Duration green number (NUMBER_DURATION_GREEN: 313) ----
            // Java: (int)(PlayConfig.duration_green)
            313 => self
                .current_play_config_ref()
                .map_or(i32::MIN, |pc| pc.duration),
            // Total notes
            350 => self.resource.songdata().map_or(0, |s| s.chart.notes),
            // ---- Chart note breakdown from SongInformation (351-353) ----
            // Java: SongInformation.getN() / .getLn() / .getS()
            351 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.n),
            352 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.ln),
            353 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.s),
            // ---- Density integers + afterdot (360-365) ----
            // Java: (int) peakdensity, (int)((peakdensity*100)%100), etc.
            360 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.peakdensity as i32),
            361 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| ((i.peakdensity * 100.0) as i32) % 100),
            362 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.enddensity as i32),
            363 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| ((i.enddensity * 100.0) as i32) % 100),
            364 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.density as i32),
            365 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| ((i.density * 100.0) as i32) % 100),
            // ---- Chart total gauge integer (368) ----
            368 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.total as i32),
            // ---- Judge rank (NUMBER_JUDGERANK: 400) ----
            // Java: state.resource.getSongdata().getJudge() -- chart judge rank
            400 => self.resource.songdata().map_or(i32::MIN, |s| s.chart.judge),
            // Song duration minutes/seconds
            1163 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| (s.chart.length.max(0) / 60000) % 60),
            1164 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| (s.chart.length.max(0) / 1000) % 60),
            // Cumulative playtime (hours/minutes/seconds from PlayerData, in seconds)
            // Java: PlayerData.getPlaytime() / 3600, / 60 % 60, % 60
            17 => self
                .resource
                .player_data()
                .map_or(0, |data| (data.playtime / 3600) as i32),
            18 => self
                .resource
                .player_data()
                .map_or(0, |data| ((data.playtime / 60) % 60) as i32),
            19 => self
                .resource
                .player_data()
                .map_or(0, |data| (data.playtime % 60) as i32),
            // ---- Player profile stats (IDs 30-37, 333) ----
            // Java: state.resource.getPlayerData().getPlaycount() etc.
            // Available on all screens (global IntegerPropertyFactory).
            30 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.playcount as i32),
            31 => self.resource.player_data().map_or(0, |pd| pd.clear as i32),
            32 => self
                .resource
                .player_data()
                .map_or(0, |pd| (pd.playcount - pd.clear) as i32),
            33 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(0) as i32),
            34 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(1) as i32),
            35 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(2) as i32),
            36 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(3) as i32),
            37 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(4) as i32),
            333 => self.resource.player_data().map_or(0, |pd| {
                let total: i64 = (0..=3).map(|judge| pd.judge_count(judge)).sum();
                total.min(i32::MAX as i64) as i32
            }),
            // IDs 20-29 (FPS, system date/time, boot time) handled by default_integer_value
            _ => self.default_integer_value(id),
        }
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        if (17..=19).contains(&id)
            && let Some(mut audio) = self.main.config().audio.clone()
        {
            let clamped = value.clamp(0.0, 1.0);
            match id {
                17 => audio.systemvolume = clamped,
                18 => audio.keyvolume = clamped,
                19 => audio.bgvolume = clamped,
                _ => unreachable!(),
            }
            self.main.update_audio_config(audio);
        }
    }

    fn notify_audio_config_changed(&mut self) {
        if let Some(audio) = self.main.config().audio.clone() {
            self.main.update_audio_config(audio);
        }
    }

    fn get_offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.offsets.get(&id)
    }
}

impl rubato_skin::reexports::MainState for DecideRenderContext<'_> {}

#[allow(dead_code)] // Only used in tests after PropertySnapshot migration
struct DecideMouseContext<'a> {
    timer: &'a mut TimerManager,
    main: &'a mut MainControllerRef,
    resource: &'a mut dyn PlayerResourceAccess,
    score_data_property: &'a rubato_types::score_data_property::ScoreDataProperty,
    offsets: &'a std::collections::HashMap<i32, rubato_types::skin_offset::SkinOffset>,
    /// Events collected during mouse handling for deferred dispatch.
    /// Skin click events that route through `DelegateEvent` call `execute_event()`,
    /// but most decide-screen interactions use direct trait methods (`change_state`,
    /// `set_timer_micro`, `player_config_mut`) which bypass `execute_event` entirely.
    /// Events collected here are replayed after the skin is restored.
    pending_events: Vec<(i32, i32, i32)>,
}

impl rubato_types::timer_access::TimerAccess for DecideMouseContext<'_> {
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

impl rubato_types::skin_render_context::SkinRenderContext for DecideMouseContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Decide)
    }

    fn boot_time_millis(&self) -> i64 {
        self.timer.boot_time_millis()
    }

    fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
        // Queue events for replay after the skin is restored.
        // During mouse handling the skin is `take()`-ed, so we cannot call
        // `skin.execute_custom_event()` directly here.
        self.pending_events.push((id, arg1, arg2));
    }

    fn change_state(&mut self, state: rubato_types::main_state_type::MainStateType) {
        self.main.change_state(state);
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn audio_play(&mut self, path: &str, volume: f32, is_loop: bool) {
        self.main.play_audio_path(path, volume, is_loop);
    }

    fn audio_stop(&mut self, path: &str) {
        self.main.stop_audio_path(path);
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(self.resource.player_config())
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(self.main.config())
    }

    fn score_data_ref(&self) -> Option<&crate::core::score_data::ScoreData> {
        self.resource.score_data()
    }

    fn target_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
        self.resource.target_score_data()
    }

    fn rival_score_data_ref(&self) -> Option<&crate::core::score_data::ScoreData> {
        self.resource.rival_score_data()
    }

    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        self.resource.replay_data()
    }

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.resource.songdata()
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        let mode = self
            .resource
            .songdata()
            .and_then(|song| match song.chart.mode {
                5 => Some(bms::model::mode::Mode::BEAT_5K),
                7 => Some(bms::model::mode::Mode::BEAT_7K),
                9 => Some(bms::model::mode::Mode::POPN_9K),
                10 => Some(bms::model::mode::Mode::BEAT_10K),
                14 => Some(bms::model::mode::Mode::BEAT_14K),
                25 => Some(bms::model::mode::Mode::KEYBOARD_24K),
                50 => Some(bms::model::mode::Mode::KEYBOARD_24K_DOUBLE),
                _ => None,
            })?;
        Some(
            &self
                .resource
                .player_config()
                .play_config_ref(mode)
                .playconfig,
        )
    }

    fn boolean_value(&self, id: i32) -> bool {
        match id {
            // ---- BGA on/off (OPTION_BGAOFF: 40 / OPTION_BGAON: 41) ----
            40 => self.main.config().render.bga == rubato_types::config::BgaMode::Off,
            41 => self.main.config().render.bga != rubato_types::config::BgaMode::Off,
            // ---- Save score (OPTION_DISABLE_SAVE_SCORE: 60 / OPTION_ENABLE_SAVE_SCORE: 61) ----
            60 => !self.resource.is_update_score(),
            61 => self.resource.is_update_score(),
            // ---- Stagefile/banner/backbmp existence (190-195) ----
            190 => self
                .resource
                .songdata()
                .is_none_or(|s| s.file.stagefile.is_empty()),
            191 => self
                .resource
                .songdata()
                .is_some_and(|s| !s.file.stagefile.is_empty()),
            192 => self
                .resource
                .songdata()
                .is_none_or(|s| s.file.banner.is_empty()),
            193 => self
                .resource
                .songdata()
                .is_some_and(|s| !s.file.banner.is_empty()),
            194 => self
                .resource
                .songdata()
                .is_none_or(|s| s.file.backbmp.is_empty()),
            195 => self
                .resource
                .songdata()
                .is_some_and(|s| !s.file.backbmp.is_empty()),
            // ---- Course stage (280-283, 289, 290) ----
            280 => self.resource.course_data().is_some() && self.resource.course_index() == 0,
            281 => self.resource.course_data().is_some() && self.resource.course_index() == 1,
            282 => self.resource.course_data().is_some() && self.resource.course_index() == 2,
            283 => self.resource.course_data().is_some() && self.resource.course_index() == 3,
            289 => {
                if let Some(cd) = self.resource.course_data() {
                    let song_count = cd.hash.len();
                    song_count > 0 && self.resource.course_index() == song_count - 1
                } else {
                    false
                }
            }
            290 => self.resource.course_data().is_some(),
            _ => self.default_boolean_value(id),
        }
    }

    fn float_value(&self, id: i32) -> f32 {
        match id {
            // Volume (0.0-1.0) from audio config
            17 => self
                .main
                .config()
                .audio_config()
                .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                    a.systemvolume
                }),
            18 => self
                .main
                .config()
                .audio_config()
                .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                    a.keyvolume
                }),
            19 => self
                .main
                .config()
                .audio_config()
                .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                    a.bgvolume
                }),
            _ => self.default_float_value(id),
        }
    }

    fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
        self.score_data_property
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            // ---- Hi-speed (NUMBER_HISPEED_LR2: 10) ----
            10 => self
                .current_play_config_ref()
                .map_or(i32::MIN, |pc| (pc.hispeed * 100.0) as i32),
            // ---- Judge timing (NUMBER_JUDGETIMING: 12) ----
            12 => self.resource.player_config().judge_settings.judgetiming,
            // Volume (0-100 scale) from audio config
            57 => {
                (self
                    .main
                    .config()
                    .audio_config()
                    .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                        a.systemvolume
                    })
                    * 100.0) as i32
            }
            58 => {
                (self
                    .main
                    .config()
                    .audio_config()
                    .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                        a.keyvolume
                    })
                    * 100.0) as i32
            }
            59 => {
                (self
                    .main
                    .config()
                    .audio_config()
                    .map_or(rubato_types::audio_config::DEFAULT_AUDIO_VOLUME, |a| {
                        a.bgvolume
                    })
                    * 100.0) as i32
            }
            // ---- EX score (71/101/171) ----
            71 | 101 | 171 => self.resource.score_data().map_or(i32::MIN, |s| s.exscore()),
            // ---- Max score (72) ----
            72 => self.resource.score_data().map_or(i32::MIN, |s| s.notes * 2),
            // ---- Max combo (75) ----
            75 => self.resource.score_data().map_or(i32::MIN, |s| s.maxcombo),
            // ---- Miss count / minbp (76) ----
            76 => self.resource.score_data().map_or(i32::MIN, |s| s.minbp),
            // ---- Judge counts (80-84) ----
            80..=84 => {
                let index = id - 80;
                self.resource
                    .score_data()
                    .map_or(i32::MIN, |s| s.judge_count_total(index))
            }
            // ---- Judge count rates (85-89) ----
            85..=89 => {
                let index = id - 85;
                self.resource.score_data().map_or(i32::MIN, |s| {
                    if s.notes > 0 {
                        s.judge_count_total(index) * 100 / s.notes
                    } else {
                        i32::MIN
                    }
                })
            }
            90 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| s.chart.maxbpm),
            91 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| s.chart.minbpm),
            92 => self.resource.songdata().map_or(i32::MIN, |s| {
                s.info
                    .as_ref()
                    .map(|i| i.mainbpm as i32)
                    .unwrap_or(i32::MIN)
            }),
            // ---- Point / score (100) ----
            100 => self.score_data_property.now_score(),
            // ---- Score rate (102) ----
            102 => {
                if self.resource.score_data().is_some() {
                    self.score_data_property.nowrate_int
                } else {
                    i32::MIN
                }
            }
            // ---- Score rate afterdot (103) ----
            103 => {
                if self.resource.score_data().is_some() {
                    self.score_data_property.nowrate_after_dot
                } else {
                    i32::MIN
                }
            }
            // ---- Diff vs target (108/128/153) ----
            108 | 128 | 153 => {
                self.score_data_property.nowscore - self.score_data_property.nowrivalscore
            }
            // ---- Total rate (115/155) ----
            115 | 155 => {
                if self.resource.score_data().is_some() {
                    self.score_data_property.rate_int
                } else {
                    i32::MIN
                }
            }
            // ---- Total rate afterdot (116/156) ----
            116 | 156 => {
                if self.resource.score_data().is_some() {
                    self.score_data_property.rate_after_dot
                } else {
                    i32::MIN
                }
            }
            // ---- Target / rival score (121/151) ----
            121 | 151 => self.score_data_property.rivalscore,
            // ---- Target / rival score rate (122/157) ----
            122 | 157 => self.score_data_property.rivalrate_int,
            // ---- Target / rival score rate afterdot (123/158) ----
            123 | 158 => self.score_data_property.rivalrate_after_dot,
            // ---- Diff vs high score (152/172) ----
            152 | 172 => self.score_data_property.nowscore - self.score_data_property.nowbestscore,
            // ---- Diff next rank (154) ----
            154 => self.score_data_property.nextrank,
            // ---- Best rate (183/184) ----
            183 => self.score_data_property.bestrate_int,
            184 => self.score_data_property.bestrate_after_dot,
            // ---- Hi-speed integer/afterdot (310/311) ----
            310 => self
                .current_play_config_ref()
                .map_or(i32::MIN, |pc| pc.hispeed as i32),
            311 => self
                .current_play_config_ref()
                .map_or(i32::MIN, |pc| ((pc.hispeed * 100.0) as i32) % 100),
            312 => self.resource.songdata().map_or(0, |s| s.chart.length),
            // ---- Duration green number (313) ----
            313 => self
                .current_play_config_ref()
                .map_or(i32::MIN, |pc| pc.duration),
            350 => self.resource.songdata().map_or(0, |s| s.chart.notes),
            // ---- Chart note breakdown from SongInformation (351-353) ----
            351 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.n),
            352 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.ln),
            353 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.s),
            // ---- Density integers + afterdot (360-365) ----
            360 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.peakdensity as i32),
            361 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| ((i.peakdensity * 100.0) as i32) % 100),
            362 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.enddensity as i32),
            363 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| ((i.enddensity * 100.0) as i32) % 100),
            364 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.density as i32),
            365 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| ((i.density * 100.0) as i32) % 100),
            // ---- Chart total gauge integer (368) ----
            368 => self
                .resource
                .songdata()
                .and_then(|s| s.info.as_ref())
                .map_or(i32::MIN, |i| i.total as i32),
            // ---- Judge rank (NUMBER_JUDGERANK: 400) ----
            // Java: state.resource.getSongdata().getJudge() -- chart judge rank
            400 => self.resource.songdata().map_or(i32::MIN, |s| s.chart.judge),
            1163 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| (s.chart.length.max(0) / 60000) % 60),
            1164 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| (s.chart.length.max(0) / 1000) % 60),
            // Cumulative playtime (hours/minutes/seconds from PlayerData, in seconds)
            17 => self
                .resource
                .player_data()
                .map_or(0, |data| (data.playtime / 3600) as i32),
            18 => self
                .resource
                .player_data()
                .map_or(0, |data| ((data.playtime / 60) % 60) as i32),
            19 => self
                .resource
                .player_data()
                .map_or(0, |data| (data.playtime % 60) as i32),
            // Chart level
            96 => self.resource.songdata().map_or(i32::MIN, |s| s.chart.level),
            // ---- Player profile stats (IDs 30-37, 333) ----
            // Java: state.resource.getPlayerData().getPlaycount() etc.
            // Available on all screens (global IntegerPropertyFactory).
            30 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.playcount as i32),
            31 => self.resource.player_data().map_or(0, |pd| pd.clear as i32),
            32 => self
                .resource
                .player_data()
                .map_or(0, |pd| (pd.playcount - pd.clear) as i32),
            33 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(0) as i32),
            34 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(1) as i32),
            35 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(2) as i32),
            36 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(3) as i32),
            37 => self
                .resource
                .player_data()
                .map_or(0, |pd| pd.judge_count(4) as i32),
            333 => self.resource.player_data().map_or(0, |pd| {
                let total: i64 = (0..=3).map(|judge| pd.judge_count(judge)).sum();
                total.min(i32::MAX as i64) as i32
            }),
            // IDs 20-29 (FPS, system date/time, boot time) handled by default_integer_value
            _ => self.default_integer_value(id),
        }
    }

    fn image_index_value(&self, id: i32) -> i32 {
        match id {
            308 => {
                if let Some(song) = self.resource.songdata()
                    && let Some(override_val) =
                        rubato_types::skin_render_context::compute_lnmode_from_chart(&song.chart)
                {
                    return override_val;
                }
                self.default_image_index_value(id)
            }
            _ => self.default_image_index_value(id),
        }
    }

    fn string_value(&self, id: i32) -> String {
        match id {
            10 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.title.clone()),
            11 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.subtitle.clone()),
            12 => self.resource.songdata().map_or_else(String::new, |s| {
                if s.metadata.subtitle.is_empty() {
                    s.metadata.title.clone()
                } else {
                    format!("{} {}", s.metadata.title, s.metadata.subtitle)
                }
            }),
            13 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.genre.clone()),
            14 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.artist.clone()),
            15 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.subartist.clone()),
            16 => self.resource.songdata().map_or_else(String::new, |s| {
                if s.metadata.subartist.is_empty() {
                    s.metadata.artist.clone()
                } else {
                    format!("{} {}", s.metadata.artist, s.metadata.subartist)
                }
            }),
            _ => String::new(),
        }
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        self.resource.player_config_mut()
    }

    fn play_option_change_sound(&mut self) {
        self.main.play_sound(&SoundType::OptionChange, false);
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        if (17..=19).contains(&id)
            && let Some(mut audio) = self.main.config().audio.clone()
        {
            let clamped = value.clamp(0.0, 1.0);
            match id {
                17 => audio.systemvolume = clamped,
                18 => audio.keyvolume = clamped,
                19 => audio.bgvolume = clamped,
                _ => unreachable!(),
            }
            self.main.update_audio_config(audio);
        }
    }

    fn notify_audio_config_changed(&mut self) {
        if let Some(audio) = self.main.config().audio.clone() {
            self.main.update_audio_config(audio);
        }
    }

    fn get_offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.offsets.get(&id)
    }
}

/// MusicDecide - music decide screen state
///
/// Translated from MusicDecide.java
/// In Java, MusicDecide extends MainState. In Rust, we use composition
/// with MainStateData and hold references to MainController and PlayerResource.
pub struct MusicDecide {
    pub data: MainStateData,
    pub main: MainControllerRef,
    pub resource: Box<dyn PlayerResourceAccess>,
    cancel: bool,
    /// Cached ScoreDataProperty for skin property delegation.
    cached_score_data_property: rubato_types::score_data_property::ScoreDataProperty,
    /// Read-only input snapshot for the current frame.
    input_snapshot: Option<rubato_input::input_snapshot::InputSnapshot>,
    /// Outbox: pending system sound plays.
    pending_sounds: Vec<(SoundType, bool)>,
    /// Outbox: pending audio path plays.
    pending_audio_path_plays: Vec<(String, f32, bool)>,
    /// Outbox: pending audio path stops.
    pending_audio_path_stops: Vec<String>,
    /// Outbox: pending audio config update.
    pending_audio_config: Option<rubato_types::audio_config::AudioConfig>,
}

impl MusicDecide {
    pub fn new(
        main: MainControllerRef,
        resource: Box<dyn PlayerResourceAccess>,
        timer: TimerManager,
    ) -> Self {
        let mut cached_score_data_property =
            rubato_types::score_data_property::ScoreDataProperty::new();
        cached_score_data_property
            .update_score_and_rival(resource.score_data(), resource.rival_score_data());
        Self {
            data: MainStateData::new(timer),
            main,
            resource,
            cancel: false,
            cached_score_data_property,
            input_snapshot: None,
            pending_sounds: Vec::new(),
            pending_audio_path_plays: Vec::new(),
            pending_audio_path_stops: Vec::new(),
            pending_audio_config: None,
        }
    }
}

impl MusicDecide {
    /// Build a PropertySnapshot capturing all raw data needed for skin rendering.
    fn build_snapshot(&self, timer: &TimerManager) -> PropertySnapshot {
        let mut s = PropertySnapshot::new();

        // Timing
        s.now_time = timer.now_time();
        s.now_micro_time = timer.now_micro_time();
        s.boot_time_millis = timer.boot_time_millis();
        for (i, &val) in timer.timer_values().iter().enumerate() {
            if val != i64::MIN {
                s.timers.insert(TimerId::new(i as i32), val);
            }
        }

        // State identity
        s.state_type = Some(rubato_types::main_state_type::MainStateType::Decide);

        // Config
        s.config = Some(Box::new(self.main.config().clone()));
        s.player_config = Some(Box::new(self.main.player_config().clone()));

        // Play config (resolve mode from song data)
        s.play_config = self
            .resource
            .songdata()
            .and_then(|song| match song.chart.mode {
                5 => Some(bms::model::mode::Mode::BEAT_5K),
                7 => Some(bms::model::mode::Mode::BEAT_7K),
                9 => Some(bms::model::mode::Mode::POPN_9K),
                10 => Some(bms::model::mode::Mode::BEAT_10K),
                14 => Some(bms::model::mode::Mode::BEAT_14K),
                25 => Some(bms::model::mode::Mode::KEYBOARD_24K),
                50 => Some(bms::model::mode::Mode::KEYBOARD_24K_DOUBLE),
                _ => None,
            })
            .map(|mode| {
                Box::new(
                    self.resource
                        .player_config()
                        .play_config_ref(mode)
                        .playconfig
                        .clone(),
                )
            });

        // Song / score data
        s.song_data = self.resource.songdata().map(|d| Box::new(d.clone()));
        s.score_data = self.resource.score_data().map(|d| Box::new(d.clone()));
        s.rival_score_data = self
            .resource
            .rival_score_data()
            .map(|d| Box::new(d.clone()));
        s.target_score_data = self
            .resource
            .target_score_data()
            .map(|d| Box::new(d.clone()));
        s.replay_option_data = self.resource.replay_data().map(|d| Box::new(d.clone()));
        s.score_data_property = self.cached_score_data_property.clone();

        // Player / course data
        s.player_data = self.resource.player_data().copied();
        s.is_course_mode = self.resource.course_data().is_some();
        s.course_index = self.resource.course_index();
        s.course_song_count = self.resource.course_data().map_or(0, |cd| cd.hash.len());
        s.is_update_score = self.resource.is_update_score();

        // Offsets
        s.offsets = self.data.offsets.clone();

        s
    }

    /// Apply queued actions from the snapshot back to live game state.
    /// Audio actions are stored in pending lists for lifecycle outbox consumption.
    fn drain_actions(&mut self, actions: &mut SkinActionQueue, timer: &mut TimerManager) {
        // Timer sets
        for (timer_id, micro_time) in actions.timer_sets.drain(..) {
            timer.set_micro_timer(timer_id, micro_time);
        }

        // State changes (must stay on command queue)
        for state in actions.state_changes.drain(..) {
            self.main.change_state(state);
        }

        // Audio: store in pending lists for outbox drain
        for (path, volume, is_loop) in actions.audio_plays.drain(..) {
            self.pending_audio_path_plays.push((path, volume, is_loop));
        }
        for path in actions.audio_stops.drain(..) {
            self.pending_audio_path_stops.push(path);
        }

        // Float writes (volume sliders) -- apply to pending audio config
        for (id, value) in actions.float_writes.drain(..) {
            if (17..=19).contains(&id) {
                let mut audio = self
                    .pending_audio_config
                    .clone()
                    .or_else(|| self.main.config().audio.clone())
                    .unwrap_or_default();
                let clamped = value.clamp(0.0, 1.0);
                match id {
                    17 => audio.systemvolume = clamped,
                    18 => audio.keyvolume = clamped,
                    19 => audio.bgvolume = clamped,
                    _ => {}
                }
                self.pending_audio_config = Some(audio);
            }
        }

        // Config propagation
        if actions.audio_config_changed {
            if self.pending_audio_config.is_none() {
                self.pending_audio_config = self.main.config().audio.clone();
            }
            actions.audio_config_changed = false;
        }

        // Player config mutations: copy back from snapshot if modified
        // (handled at call site since we need snapshot access)
    }
}

impl MainState for MusicDecide {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Decide)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.data
    }

    fn create(&mut self) {
        self.cancel = false;

        // loadSkin(SkinType.DECIDE)
        self.load_skin(SkinType::Decide.id());

        // resource.setOrgGaugeOption(resource.getPlayerConfig().getGauge())
        let gauge = self.resource.player_config().play_settings.gauge;
        self.resource.set_org_gauge_option(gauge);
    }

    fn prepare(&mut self) {
        // super.prepare() - default empty in MainState
        // play(DECIDE) -- via outbox, drained by lifecycle
        self.pending_sounds.push((SoundType::Decide, false));
    }

    fn render_skin(&mut self, sprite: &mut rubato_render::sprite_batch::SpriteBatch) {
        let mut skin = match self.data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.data.timer);

        let mut snapshot = self.build_snapshot(&timer);
        skin.update_custom_objects_timed(&mut snapshot);
        skin.swap_sprite_batch(sprite);
        skin.draw_all_objects_timed(&mut snapshot);
        skin.swap_sprite_batch(sprite);

        // Drain non-event actions (timers, audio, state changes)
        self.drain_actions(&mut snapshot.actions, &mut timer);

        // Replay queued custom events now that the skin is available again.
        let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
        let mut depth = 0;
        while !pending_events.is_empty() && depth < 8 {
            let mut replay_snapshot = self.build_snapshot(&timer);
            for (id, arg1, arg2) in pending_events {
                skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
            }
            self.drain_actions(&mut replay_snapshot.actions, &mut timer);
            pending_events = replay_snapshot.actions.custom_events;
            depth += 1;
        }
        if depth >= 8 {
            log::warn!("Decide render_skin event replay exceeded depth limit");
        }

        self.data.timer = timer;
        self.data.skin = Some(skin);
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.data.timer);

        let mut snapshot = self.build_snapshot(&timer);
        skin.mouse_pressed_at(&mut snapshot, button, x, y);
        self.drain_actions(&mut snapshot.actions, &mut timer);

        // Replay queued custom events.
        let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
        let mut depth = 0;
        while !pending_events.is_empty() && depth < 8 {
            let mut replay_snapshot = self.build_snapshot(&timer);
            for (id, arg1, arg2) in pending_events {
                skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
            }
            self.drain_actions(&mut replay_snapshot.actions, &mut timer);
            pending_events = replay_snapshot.actions.custom_events;
            depth += 1;
        }
        if depth >= 8 {
            log::warn!("Decide mouse_pressed event replay exceeded depth limit");
        }

        self.data.timer = timer;
        self.data.skin = Some(skin);
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.data.timer);

        let mut snapshot = self.build_snapshot(&timer);
        skin.mouse_dragged_at(&mut snapshot, button, x, y);
        self.drain_actions(&mut snapshot.actions, &mut timer);

        // Replay queued custom events.
        let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
        let mut depth = 0;
        while !pending_events.is_empty() && depth < 8 {
            let mut replay_snapshot = self.build_snapshot(&timer);
            for (id, arg1, arg2) in pending_events {
                skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
            }
            self.drain_actions(&mut replay_snapshot.actions, &mut timer);
            pending_events = replay_snapshot.actions.custom_events;
            depth += 1;
        }
        if depth >= 8 {
            log::warn!("Decide mouse_dragged event replay exceeded depth limit");
        }

        self.data.timer = timer;
        self.data.skin = Some(skin);
    }

    fn render(&mut self) {
        let nowtime = self.data.timer.now_time();
        // Skin timing values; fall back to 0 when no skin is loaded so the
        // decide screen still transitions to Play instead of stalling forever.
        let input_time = self.data.skin.as_ref().map_or(0, |s| s.input() as i64);
        let fadeout_time = self.data.skin.as_ref().map_or(0, |s| s.fadeout() as i64);
        let scene_time = self.data.skin.as_ref().map_or(0, |s| s.scene() as i64);

        if nowtime > input_time {
            self.data.timer.switch_timer(TIMER_STARTINPUT, true);
        }
        if self.data.timer.is_timer_on(TIMER_FADEOUT) {
            if self.data.timer.now_time_for_id(TIMER_FADEOUT) > fadeout_time {
                self.main.change_state(if self.cancel {
                    MainStateType::MusicSelect
                } else {
                    MainStateType::Play
                });
            }
        } else if nowtime > scene_time {
            self.data.timer.set_timer_on(TIMER_FADEOUT);
        }
    }

    fn render_with_game_context(
        &mut self,
        _ctx: &mut GameContext,
    ) -> Option<StateTransition> {
        let nowtime = self.data.timer.now_time();
        // Skin timing values; fall back to 0 when no skin is loaded so the
        // decide screen still transitions to Play instead of stalling forever.
        let input_time = self.data.skin.as_ref().map_or(0, |s| s.input() as i64);
        let fadeout_time = self.data.skin.as_ref().map_or(0, |s| s.fadeout() as i64);
        let scene_time = self.data.skin.as_ref().map_or(0, |s| s.scene() as i64);

        if nowtime > input_time {
            self.data.timer.switch_timer(TIMER_STARTINPUT, true);
        }
        if self.data.timer.is_timer_on(TIMER_FADEOUT) {
            if self.data.timer.now_time_for_id(TIMER_FADEOUT) > fadeout_time {
                return Some(StateTransition::ChangeTo(if self.cancel {
                    MainStateType::MusicSelect
                } else {
                    MainStateType::Play
                }));
            }
        } else if nowtime > scene_time {
            self.data.timer.set_timer_on(TIMER_FADEOUT);
        }

        Some(StateTransition::Continue)
    }

    fn input_with_game_context(&mut self, ctx: &mut GameContext) -> Option<()> {
        if let Some(ref snapshot) = self.input_snapshot
            && !self.data.timer.is_timer_on(TIMER_FADEOUT)
            && self.data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            let decide = snapshot.key_state[0]
                || snapshot.key_state[2]
                || snapshot.key_state[4]
                || snapshot.key_state[6]
                || snapshot
                    .control_key_states
                    .get(&ControlKeys::Enter)
                    .copied()
                    .unwrap_or(false);
            let cancel = snapshot
                .control_key_states
                .get(&ControlKeys::Escape)
                .copied()
                .unwrap_or(false)
                || (snapshot.start_pressed && snapshot.select_pressed);
            if decide {
                self.data.timer.set_timer_on(TIMER_FADEOUT);
            }
            if cancel {
                self.cancel = true;
                ctx.set_global_pitch(1f32);
                self.data.timer.set_timer_on(TIMER_FADEOUT);
            }
        }
        Some(())
    }

    fn sync_input_snapshot(&mut self, snapshot: &rubato_input::input_snapshot::InputSnapshot) {
        self.input_snapshot = Some(snapshot.clone());
    }

    fn input_with_ctx(&mut self, ctx: &mut crate::core::app_context::GameContext) {
        if let Some(ref snapshot) = self.input_snapshot
            && !self.data.timer.is_timer_on(TIMER_FADEOUT)
            && self.data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            let decide = snapshot.key_state[0]
                || snapshot.key_state[2]
                || snapshot.key_state[4]
                || snapshot.key_state[6]
                || snapshot
                    .control_key_states
                    .get(&ControlKeys::Enter)
                    .copied()
                    .unwrap_or(false);
            let cancel = snapshot
                .control_key_states
                .get(&ControlKeys::Escape)
                .copied()
                .unwrap_or(false)
                || (snapshot.start_pressed && snapshot.select_pressed);
            if decide {
                self.data.timer.set_timer_on(TIMER_FADEOUT);
            }
            if cancel {
                self.cancel = true;
                ctx.set_global_pitch(1f32);
                self.data.timer.set_timer_on(TIMER_FADEOUT);
            }
        }
    }

    fn load_skin(&mut self, skin_type: i32) {
        let skin_path = rubato_skin::skin_loader::skin_path_from_player_config(
            self.main.player_config(),
            skin_type,
        );
        let skin = {
            let mut snapshot = self.build_snapshot(&self.data.timer);
            let registry = std::collections::HashMap::new();
            let mut state =
                rubato_skin::snapshot_main_state::SnapshotMainState::new(&mut snapshot, &registry);
            skin_path.as_deref().and_then(|path| {
                rubato_skin::skin_loader::load_skin_from_path_with_state(
                    &mut state, skin_type, path,
                )
            })
        };
        self.data.skin =
            skin.map(|skin| Box::new(skin) as Box<dyn crate::core::main_state::SkinDrawable>);
    }

    fn dispose(&mut self) {
        // super.dispose()
        if let Some(ref mut skin) = self.data.skin {
            skin.dispose_skin();
        }
        self.data.skin = None;
    }

    fn take_player_resource_box(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        let null: Box<dyn PlayerResourceAccess> = Box::new(NullPlayerResource::new());
        let old = std::mem::replace(&mut self.resource, null);
        Some(old.into_any_send())
    }

    fn drain_pending_sounds(&mut self) -> Vec<(SoundType, bool)> {
        std::mem::take(&mut self.pending_sounds)
    }

    fn drain_pending_audio_path_plays(&mut self) -> Vec<(String, f32, bool)> {
        std::mem::take(&mut self.pending_audio_path_plays)
    }

    fn drain_pending_audio_path_stops(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_audio_path_stops)
    }

    fn take_pending_audio_config(&mut self) -> Option<rubato_types::audio_config::AudioConfig> {
        self.pending_audio_config.take()
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::core::main_state::SkinDrawable;
    use crate::core::sprite_batch_helper::SpriteBatch;
    use crate::state::decide::{NullMainController, NullPlayerResource};
    use rubato_types::main_controller_access::MainControllerAccess;
    use std::sync::{Arc, Mutex};

    static EMPTY_OFFSETS: std::sync::LazyLock<
        std::collections::HashMap<i32, rubato_types::skin_offset::SkinOffset>,
    > = std::sync::LazyLock::new(std::collections::HashMap::new);

    /// Mock SkinDrawable for testing render logic with configurable timing values.
    struct MockSkin {
        input: i32,
        scene: i32,
        fadeout: i32,
    }

    impl MockSkin {
        fn new() -> Self {
            Self {
                input: 0,
                scene: 0,
                fadeout: 0,
            }
        }

        fn with_values(input: i32, scene: i32, fadeout: i32) -> Self {
            Self {
                input,
                scene,
                fadeout,
            }
        }
    }

    impl SkinDrawable for MockSkin {
        fn draw_all_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }
        fn update_custom_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }
        fn mouse_pressed_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }
        fn mouse_dragged_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }
        fn prepare_skin(
            &mut self,
            _state_type: Option<rubato_types::main_state_type::MainStateType>,
        ) {
        }
        fn dispose_skin(&mut self) {}
        fn fadeout(&self) -> i32 {
            self.fadeout
        }
        fn input(&self) -> i32 {
            self.input
        }
        fn scene(&self) -> i32 {
            self.scene
        }
        fn get_width(&self) -> f32 {
            0.0
        }
        fn get_height(&self) -> f32 {
            0.0
        }
        fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
    }

    struct ChangeStateSkin {
        state: MainStateType,
    }

    impl SkinDrawable for ChangeStateSkin {
        fn draw_all_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }

        fn update_custom_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }

        fn mouse_pressed_at(
            &mut self,
            ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
            ctx.change_state(self.state);
        }

        fn mouse_dragged_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }

        fn prepare_skin(
            &mut self,
            _state_type: Option<rubato_types::main_state_type::MainStateType>,
        ) {
        }

        fn dispose_skin(&mut self) {}

        fn fadeout(&self) -> i32 {
            0
        }

        fn input(&self) -> i32 {
            0
        }

        fn scene(&self) -> i32 {
            0
        }

        fn get_width(&self) -> f32 {
            0.0
        }

        fn get_height(&self) -> f32 {
            0.0
        }

        fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
    }

    struct RecordingMainController {
        changed_states: Arc<Mutex<Vec<MainStateType>>>,
        audio_configs: Arc<Mutex<Vec<rubato_types::audio_config::AudioConfig>>>,
        config: rubato_types::config::Config,
        player_config: rubato_types::player_config::PlayerConfig,
    }

    impl RecordingMainController {
        fn new(changed_states: Arc<Mutex<Vec<MainStateType>>>) -> Self {
            Self {
                changed_states,
                audio_configs: Arc::new(Mutex::new(Vec::new())),
                config: rubato_types::config::Config::default(),
                player_config: rubato_types::player_config::PlayerConfig::default(),
            }
        }

        fn with_audio_recording(
            changed_states: Arc<Mutex<Vec<MainStateType>>>,
            audio_configs: Arc<Mutex<Vec<rubato_types::audio_config::AudioConfig>>>,
            config: rubato_types::config::Config,
        ) -> Self {
            Self {
                changed_states,
                audio_configs,
                config,
                player_config: rubato_types::player_config::PlayerConfig::default(),
            }
        }
    }

    impl MainControllerAccess for RecordingMainController {
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }

        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }

        fn change_state(&mut self, state: MainStateType) {
            self.changed_states
                .lock()
                .expect("mutex poisoned")
                .push(state);
        }

        fn save_config(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn exit(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn save_last_recording(&self, _reason: &str) {}

        fn update_song(&mut self, _path: Option<&str>) {}

        fn player_resource(
            &self,
        ) -> Option<&dyn rubato_types::player_resource_access::PlayerResourceAccess> {
            None
        }

        fn player_resource_mut(
            &mut self,
        ) -> Option<&mut dyn rubato_types::player_resource_access::PlayerResourceAccess> {
            None
        }

        fn update_audio_config(&self, audio: rubato_types::audio_config::AudioConfig) {
            self.audio_configs
                .lock()
                .expect("mutex poisoned")
                .push(audio);
        }
    }

    fn make_decide() -> MusicDecide {
        MusicDecide::new(
            MainControllerRef::new(Box::new(NullMainController)),
            Box::new(NullPlayerResource::new()),
            TimerManager::new(),
        )
    }

    #[test]
    fn test_state_type() {
        let decide = make_decide();
        assert_eq!(decide.state_type(), Some(MainStateType::Decide));
    }

    #[test]
    fn test_create_resets_cancel() {
        let mut decide = make_decide();
        decide.cancel = true;
        decide.create();
        assert!(!decide.cancel);
    }

    #[test]
    fn test_create_calls_load_skin_with_decide_type() {
        let mut decide = make_decide();
        decide.create();
        assert_eq!(SkinType::Decide.id(), 6);
        assert!(
            decide.data.skin.is_some(),
            "decide create() should load the configured decide skin"
        );
    }

    #[test]
    fn test_create_sets_org_gauge_option() {
        let mut decide = make_decide();
        decide.create();
        // NullPlayerResource returns default gauge (0), verify no panic
    }

    #[test]
    fn test_prepare_plays_decide_sound() {
        let mut decide = make_decide();
        // Should not panic — stub logs warning
        decide.prepare();
    }

    #[test]
    fn test_render_no_skin_no_panic() {
        let mut decide = make_decide();
        // data.skin is None — render should not panic
        decide.render();
    }

    #[test]
    fn test_render_with_skin_nowtime_zero_no_startinput() {
        let mut decide = make_decide();
        decide.data.skin = Some(Box::new(MockSkin::new()));
        // nowmicrotime=0 from fresh TimerManager, now_time()=0
        // skin.input()=0, condition is nowtime > input i.e. 0 > 0 = false
        decide.render();
        assert!(!decide.data.timer.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn test_render_with_skin_sets_startinput_when_past_input_time() {
        let mut decide = make_decide();
        // input=-1 so that nowtime(0) > input(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(-1, i32::MAX, 0)));
        decide.render();
        assert!(decide.data.timer.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn test_render_scene_timeout_triggers_fadeout() {
        let mut decide = make_decide();
        // scene=-1 so that nowtime(0) > scene(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, -1, 0)));
        decide.render();
        assert!(decide.data.timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn test_render_fadeout_with_cancel_transitions_to_select() {
        let mut decide = make_decide();
        // fadeout=-1 so that now_time_for_id(TIMER_FADEOUT)(=0) > fadeout(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, i32::MAX, -1)));
        decide.cancel = true;
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        decide.render();
        // change_state(MusicSelect) is a stub that logs — verify no panic
    }

    #[test]
    fn test_render_fadeout_without_cancel_transitions_to_play() {
        let mut decide = make_decide();
        // fadeout=-1 so that now_time_for_id(TIMER_FADEOUT)(=0) > fadeout(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, i32::MAX, -1)));
        decide.cancel = false;
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        decide.render();
        // change_state(Play) is a stub that logs — verify no panic
    }

    /// Build a minimal GameContext for testing.
    fn make_game_context() -> GameContext {
        use crate::core::main_controller::{DatabaseState, IntegrationState, LifecycleState};
        use std::sync::atomic::AtomicBool;
        GameContext {
            config: rubato_types::config::Config::default(),
            player: rubato_types::player_config::PlayerConfig::default(),
            audio: None,
            sound: None,
            loudness_analyzer: None,
            timer: TimerManager::new(),
            input: None,
            input_poll_quit: std::sync::Arc::new(AtomicBool::new(false)),
            db: DatabaseState::default(),
            offset: Vec::new(),
            showfps: false,
            debug: false,
            integration: IntegrationState::default(),
            lifecycle: LifecycleState::new(),
            exit_requested: AtomicBool::new(false),
            resource: None,
        }
    }

    // ============================================================
    // render_with_game_context tests
    // ============================================================

    #[test]
    fn test_render_with_game_context_no_skin_returns_continue() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        // data.skin is None -- should return Continue (no transition)
        let result = decide.render_with_game_context(&mut ctx);
        assert_eq!(result, Some(StateTransition::Continue));
    }

    #[test]
    fn test_render_with_game_context_nowtime_zero_no_startinput() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        decide.data.skin = Some(Box::new(MockSkin::new()));
        // nowmicrotime=0, skin.input()=0, 0 > 0 = false
        let result = decide.render_with_game_context(&mut ctx);
        assert_eq!(result, Some(StateTransition::Continue));
        assert!(!decide.data.timer.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn test_render_with_game_context_sets_startinput_when_past_input_time() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        // input=-1 so that nowtime(0) > input(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(-1, i32::MAX, 0)));
        let result = decide.render_with_game_context(&mut ctx);
        assert_eq!(result, Some(StateTransition::Continue));
        assert!(decide.data.timer.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn test_render_with_game_context_scene_timeout_triggers_fadeout() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        // scene=-1 so that nowtime(0) > scene(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, -1, 0)));
        let result = decide.render_with_game_context(&mut ctx);
        assert_eq!(result, Some(StateTransition::Continue));
        assert!(decide.data.timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn test_render_with_game_context_fadeout_cancel_returns_change_to_select() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        // fadeout=-1 so that now_time_for_id(TIMER_FADEOUT)(=0) > fadeout(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, i32::MAX, -1)));
        decide.cancel = true;
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        let result = decide.render_with_game_context(&mut ctx);
        assert_eq!(
            result,
            Some(StateTransition::ChangeTo(MainStateType::MusicSelect))
        );
    }

    #[test]
    fn test_render_with_game_context_fadeout_no_cancel_returns_change_to_play() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        // fadeout=-1 so that now_time_for_id(TIMER_FADEOUT)(=0) > fadeout(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, i32::MAX, -1)));
        decide.cancel = false;
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        let result = decide.render_with_game_context(&mut ctx);
        assert_eq!(
            result,
            Some(StateTransition::ChangeTo(MainStateType::Play))
        );
    }

    // ============================================================
    // input_with_game_context tests
    // ============================================================

    #[test]
    fn test_input_with_game_context_returns_some() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        // Even with no timers/input, input_with_game_context returns Some(())
        let result = decide.input_with_game_context(&mut ctx);
        assert_eq!(result, Some(()));
    }

    #[test]
    fn test_input_with_game_context_no_timers_no_action() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        // Neither TIMER_FADEOUT nor TIMER_STARTINPUT is on
        decide.input_with_game_context(&mut ctx);
        assert!(!decide.cancel);
        assert!(!decide.data.timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn test_input_with_game_context_decide_key_triggers_fadeout() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        decide.data.timer.set_timer_on(TIMER_STARTINPUT);
        // Set up snapshot with key_state[0] = true (decide key)
        let mut snapshot = rubato_input::input_snapshot::InputSnapshot::default();
        snapshot.key_state[0] = true;
        decide.input_snapshot = Some(snapshot);
        decide.input_with_game_context(&mut ctx);
        assert!(decide.data.timer.is_timer_on(TIMER_FADEOUT));
        assert!(!decide.cancel);
    }

    #[test]
    fn test_input_with_game_context_cancel_key_triggers_fadeout_and_cancel() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        decide.data.timer.set_timer_on(TIMER_STARTINPUT);
        // Set up snapshot with Escape pressed
        let mut snapshot = rubato_input::input_snapshot::InputSnapshot::default();
        snapshot
            .control_key_states
            .insert(ControlKeys::Escape, true);
        decide.input_snapshot = Some(snapshot);
        decide.input_with_game_context(&mut ctx);
        assert!(decide.data.timer.is_timer_on(TIMER_FADEOUT));
        assert!(decide.cancel);
    }

    #[test]
    fn test_input_with_game_context_during_fadeout_no_action() {
        let mut decide = make_decide();
        let mut ctx = make_game_context();
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        decide.data.timer.set_timer_on(TIMER_STARTINPUT);
        // Set up snapshot with decide key -- should be blocked by fadeout
        let mut snapshot = rubato_input::input_snapshot::InputSnapshot::default();
        snapshot.key_state[0] = true;
        decide.input_snapshot = Some(snapshot);
        let result = decide.input_with_game_context(&mut ctx);
        assert_eq!(result, Some(()));
        // cancel should not be changed
        assert!(!decide.cancel);
    }

    #[test]
    fn test_input_no_timer_no_action() {
        let mut decide = make_decide();
        // Neither TIMER_FADEOUT nor TIMER_STARTINPUT is on — input does nothing
        decide.input();
        assert!(!decide.cancel);
    }

    #[test]
    fn test_input_during_fadeout_no_action() {
        let mut decide = make_decide();
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        decide.data.timer.set_timer_on(TIMER_STARTINPUT);
        // TIMER_FADEOUT is on — input is blocked
        decide.input();
    }

    #[test]
    fn test_input_startinput_only_no_keys() {
        let mut decide = make_decide();
        decide.data.timer.set_timer_on(TIMER_STARTINPUT);
        // TIMER_STARTINPUT on, TIMER_FADEOUT off — input block entered
        // But no keys pressed (stub returns false for all), so nothing happens
        decide.input();
        assert!(!decide.cancel);
        assert!(!decide.data.timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn test_handle_skin_mouse_pressed_uses_decide_context() {
        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let mut decide = MusicDecide::new(
            MainControllerRef::new(Box::new(RecordingMainController::new(Arc::clone(
                &changed_states,
            )))),
            Box::new(NullPlayerResource::new()),
            TimerManager::new(),
        );
        decide.data.skin = Some(Box::new(ChangeStateSkin {
            state: MainStateType::MusicSelect,
        }));

        <MusicDecide as MainState>::handle_skin_mouse_pressed(&mut decide, 0, 10, 10);

        assert_eq!(
            *changed_states.lock().expect("mutex poisoned"),
            vec![MainStateType::MusicSelect]
        );
    }

    #[test]
    fn test_dispose_clears_skin() {
        let mut decide = make_decide();
        decide.dispose();
        assert!(decide.data.skin.is_none());
    }

    #[test]
    fn test_main_state_data_accessors() {
        let mut decide = make_decide();
        let _ = decide.main_state_data();
        let _ = decide.main_state_data_mut();
    }

    /// Mock PlayerResourceAccess that returns a SongData with a given chart.length.
    struct SongLengthResource {
        song: rubato_types::song_data::SongData,
        config: rubato_types::config::Config,
        player_config: rubato_types::player_config::PlayerConfig,
        score: Option<crate::core::score_data::ScoreData>,
        rival_score: Option<crate::core::score_data::ScoreData>,
        target_score: Option<crate::core::score_data::ScoreData>,
        replay_data: Option<rubato_types::replay_data::ReplayData>,
        player_data: Option<rubato_types::player_data::PlayerData>,
    }

    impl SongLengthResource {
        fn with_length_ms(length: i32) -> Self {
            let mut song = rubato_types::song_data::SongData::default();
            song.chart.length = length;
            Self {
                song,
                config: rubato_types::config::Config::default(),
                player_config: rubato_types::player_config::PlayerConfig::default(),
                score: None,
                rival_score: None,
                target_score: None,
                replay_data: None,
                player_data: None,
            }
        }
    }

    impl rubato_types::player_resource_access::ConfigAccess for SongLengthResource {
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }
        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }
    }

    impl rubato_types::player_resource_access::ScoreAccess for SongLengthResource {
        fn score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
            self.score.as_ref()
        }
        fn rival_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
            self.rival_score.as_ref()
        }
        fn target_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
            self.target_score.as_ref()
        }
        fn course_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
            None
        }
        fn set_course_score_data(&mut self, _score: crate::core::score_data::ScoreData) {}
        fn score_data_mut(&mut self) -> Option<&mut crate::core::score_data::ScoreData> {
            None
        }
    }

    impl rubato_types::player_resource_access::SongAccess for SongLengthResource {
        fn songdata(&self) -> Option<&rubato_types::song_data::SongData> {
            Some(&self.song)
        }
        fn songdata_mut(&mut self) -> Option<&mut rubato_types::song_data::SongData> {
            Some(&mut self.song)
        }
        fn set_songdata(&mut self, _data: Option<rubato_types::song_data::SongData>) {}
        fn course_song_data(&self) -> Vec<rubato_types::song_data::SongData> {
            vec![]
        }
    }

    impl rubato_types::player_resource_access::ReplayAccess for SongLengthResource {
        fn replay_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
            self.replay_data.as_ref()
        }
        fn replay_data_mut(&mut self) -> Option<&mut rubato_types::replay_data::ReplayData> {
            self.replay_data.as_mut()
        }
        fn course_replay(&self) -> &[rubato_types::replay_data::ReplayData] {
            &[]
        }
        fn add_course_replay(&mut self, _rd: rubato_types::replay_data::ReplayData) {}
        fn course_replay_mut(&mut self) -> &mut Vec<rubato_types::replay_data::ReplayData> {
            static mut EMPTY: Vec<rubato_types::replay_data::ReplayData> = Vec::new();
            // SAFETY: only used in tests, never concurrently
            unsafe { &mut *std::ptr::addr_of_mut!(EMPTY) }
        }
    }

    impl rubato_types::player_resource_access::CourseAccess for SongLengthResource {
        fn course_data(&self) -> Option<&rubato_types::course_data::CourseData> {
            None
        }
        fn course_index(&self) -> usize {
            0
        }
        fn next_course(&mut self) -> bool {
            false
        }
        fn constraint(&self) -> Vec<rubato_types::course_data::CourseDataConstraint> {
            vec![]
        }
        fn set_course_data(&mut self, _data: rubato_types::course_data::CourseData) {}
        fn clear_course_data(&mut self) {}
    }

    impl rubato_types::player_resource_access::GaugeAccess for SongLengthResource {
        fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
            None
        }
        fn groove_gauge(&self) -> Option<&rubato_types::groove_gauge::GrooveGauge> {
            None
        }
        fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
            static EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
            &EMPTY
        }
        fn add_course_gauge(&mut self, _gauge: Vec<Vec<f32>>) {}
        fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
            static mut EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
            // SAFETY: only used in tests, never concurrently
            unsafe { &mut *std::ptr::addr_of_mut!(EMPTY) }
        }
    }

    impl rubato_types::player_resource_access::PlayerStateAccess for SongLengthResource {
        fn maxcombo(&self) -> i32 {
            0
        }
        fn org_gauge_option(&self) -> i32 {
            0
        }
        fn set_org_gauge_option(&mut self, _val: i32) {}
        fn assist(&self) -> i32 {
            0
        }
        fn is_update_score(&self) -> bool {
            false
        }
        fn is_update_course_score(&self) -> bool {
            false
        }
        fn is_force_no_ir_send(&self) -> bool {
            false
        }
        fn is_freq_on(&self) -> bool {
            false
        }
    }

    impl rubato_types::player_resource_access::SessionMutation for SongLengthResource {
        fn clear(&mut self) {}
        fn set_bms_file(
            &mut self,
            _path: &std::path::Path,
            _mode_type: i32,
            _mode_id: i32,
        ) -> bool {
            false
        }
        fn set_course_bms_files(&mut self, _files: &[std::path::PathBuf]) -> bool {
            false
        }
        fn set_tablename(&mut self, _name: &str) {}
        fn set_tablelevel(&mut self, _level: &str) {}
        fn set_rival_score_data_option(
            &mut self,
            _score: Option<crate::core::score_data::ScoreData>,
        ) {
        }
        fn set_chart_option_data(
            &mut self,
            _option: Option<rubato_types::replay_data::ReplayData>,
        ) {
        }
    }

    impl rubato_types::player_resource_access::MediaAccess for SongLengthResource {
        fn reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }
        fn reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }
        fn player_data(&self) -> Option<&rubato_types::player_data::PlayerData> {
            self.player_data.as_ref()
        }
    }

    impl PlayerResourceAccess for SongLengthResource {
        fn into_any_send(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }
    }

    #[test]
    fn decide_render_context_song_duration_minutes_seconds() {
        // 150_000 ms = 2 minutes 30 seconds
        let mut resource = SongLengthResource::with_length_ms(150_000);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(312), 150_000, "ID 312: raw ms");
        assert_eq!(ctx.integer_value(1163), 2, "ID 1163: minutes");
        assert_eq!(ctx.integer_value(1164), 30, "ID 1164: seconds");
    }

    #[test]
    fn decide_render_context_song_duration_no_songdata() {
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let mut resource = NullPlayerResource::new();
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(1163), i32::MIN);
        assert_eq!(ctx.integer_value(1164), i32::MIN);
    }

    #[test]
    fn decide_render_context_song_data_ref_returns_songdata() {
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.song_data_ref().is_some());
        assert_eq!(ctx.song_data_ref().unwrap().chart.length, 100_000);
    }

    #[test]
    fn decide_render_context_song_data_ref_none_when_no_song() {
        let mut resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.song_data_ref().is_none());
    }

    #[test]
    fn decide_render_context_current_play_config_ref_for_7k() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.mode = 7;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.current_play_config_ref().is_some());
    }

    #[test]
    fn decide_render_context_current_play_config_ref_none_for_unknown_mode() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.mode = 999;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.current_play_config_ref().is_none());
    }

    #[test]
    fn decide_render_context_current_play_config_ref_none_when_no_songdata() {
        let mut resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.current_play_config_ref().is_none());
    }

    #[test]
    fn decide_render_context_favorite_image_index_uses_song_data_ref() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.favorite = rubato_types::song_data::FAVORITE_SONG;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // ID 89 (favorite_song) should now return 1 instead of -1
        assert_eq!(ctx.image_index_value(89), 1);
    }

    #[test]
    fn decide_render_context_mainbpm_from_song_information() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.maxbpm = 200;
        resource.song.chart.minbpm = 100;
        // Set SongInformation with mainbpm = 160
        let mut info = rubato_types::song_information::SongInformation::default();
        info.mainbpm = 160.0;
        resource.song.info = Some(info);

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // ID 92 should return mainbpm from SongInformation
        assert_eq!(ctx.integer_value(92), 160);
    }

    #[test]
    fn decide_render_context_mainbpm_no_info_returns_min_value() {
        // When SongInformation is absent, Java returns Integer.MIN_VALUE
        // so skin renderers hide the value.
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.maxbpm = 180;
        // No SongInformation set -> should return i32::MIN, not maxbpm

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(92), i32::MIN);
    }

    #[test]
    fn decide_render_context_mainbpm_no_songdata_returns_min_value() {
        // When songdata is absent, Java returns Integer.MIN_VALUE.
        let mut resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(92), i32::MIN);
    }

    #[test]
    fn decide_render_context_maxbpm_no_songdata_returns_min_value() {
        // When songdata is absent, ID 90 (maxbpm) should return i32::MIN
        // so skin renderers hide the value, matching select screen behavior.
        let mut resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(90), i32::MIN);
    }

    #[test]
    fn decide_render_context_minbpm_no_songdata_returns_min_value() {
        // When songdata is absent, ID 91 (minbpm) should return i32::MIN
        // so skin renderers hide the value, matching select screen behavior.
        let mut resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(91), i32::MIN);
    }

    #[test]
    fn decide_render_context_maxbpm_with_songdata_returns_value() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.maxbpm = 200;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(90), 200);
    }

    #[test]
    fn decide_render_context_minbpm_with_songdata_returns_value() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.minbpm = 120;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(91), 120);
    }

    #[test]
    fn decide_render_context_negative_length_clamped_to_zero() {
        // Negative chart.length should be clamped to 0, not produce
        // negative minutes/seconds.
        let mut resource = SongLengthResource::with_length_ms(-120_000);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(1163),
            0,
            "negative length minutes should be 0"
        );
        assert_eq!(
            ctx.integer_value(1164),
            0,
            "negative length seconds should be 0"
        );
    }

    // ============================================================
    // DecideRenderContext image_index_value ID 308 (lnmode) tests
    // ============================================================

    #[test]
    fn decide_render_context_lnmode_308_override_longnote() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_LONGNOTE;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(308),
            0,
            "ID 308 should return 0 (LN) when chart has FEATURE_LONGNOTE"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_override_chargenote() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_CHARGENOTE;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(308),
            1,
            "ID 308 should return 1 (CN) when chart has FEATURE_CHARGENOTE"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_override_hellchargenote() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_HELLCHARGENOTE;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(308),
            2,
            "ID 308 should return 2 (HCN) when chart has FEATURE_HELLCHARGENOTE"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_no_override_falls_through() {
        // No LN features -> falls through to config-based default
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // default_image_index_value uses player_config.play_settings.lnmode (default 0)
        let default_lnmode = ctx.player_config_ref().unwrap().play_settings.lnmode;
        assert_eq!(
            ctx.image_index_value(308),
            default_lnmode,
            "ID 308 should fall through to config lnmode when chart has no LN features"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_undefined_ln_falls_through() {
        // UNDEFINEDLN set -> no override (has_undefined_long_note is true)
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_UNDEFINEDLN;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let default_lnmode = ctx.player_config_ref().unwrap().play_settings.lnmode;
        assert_eq!(
            ctx.image_index_value(308),
            default_lnmode,
            "ID 308 should fall through when chart has FEATURE_UNDEFINEDLN"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_no_songdata_falls_through() {
        let mut resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // No songdata -> falls through to config-based default
        let default_lnmode = ctx
            .player_config_ref()
            .map(|pc| pc.play_settings.lnmode)
            .unwrap_or(0);
        assert_eq!(
            ctx.image_index_value(308),
            default_lnmode,
            "ID 308 should fall through when no songdata available"
        );
    }

    // ============================================================
    // DecideRenderContext score_data_ref / image_index 370/371 tests
    // ============================================================

    #[test]
    fn decide_render_context_image_index_370_returns_clear_type() {
        // Regression: image_index_value(370) must return the clear type from
        // score_data_ref, not -1. Without score_data_ref delegation, the
        // default trait method returns None and 370 maps to -1.
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut score = crate::core::score_data::ScoreData::default();
        score.clear = 5; // e.g. ClearType::FullCombo
        resource.score = Some(score);

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(370),
            5,
            "ID 370 (cleartype) should return score_data.clear, not -1"
        );
    }

    #[test]
    fn decide_render_context_image_index_370_no_score_returns_minus_one() {
        // When no score data is available, 370 should still return -1.
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(370),
            -1,
            "ID 370 should return -1 when no score data is available"
        );
    }

    // ============================================================
    // DecideMouseContext missing delegation tests (Finding 2)
    // ============================================================

    #[test]
    fn decide_mouse_context_score_data_ref_delegates_to_resource() {
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut score = crate::core::score_data::ScoreData::default();
        score.clear = 4;
        resource.score = Some(score);

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let sd = ctx.score_data_ref();
        assert!(
            sd.is_some(),
            "DecideMouseContext::score_data_ref() must delegate, not return None"
        );
        assert_eq!(sd.unwrap().clear, 4);
    }

    #[test]
    fn decide_mouse_context_song_data_ref_delegates_to_resource() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.metadata.title = "DecideTest".to_string();

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let song = ctx.song_data_ref();
        assert!(
            song.is_some(),
            "DecideMouseContext::song_data_ref() must delegate, not return None"
        );
        assert_eq!(song.unwrap().metadata.title, "DecideTest");
    }

    #[test]
    fn decide_mouse_context_current_play_config_ref_delegates_for_7k() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.mode = 7;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(
            ctx.current_play_config_ref().is_some(),
            "DecideMouseContext::current_play_config_ref() must delegate, not return None"
        );
    }

    #[test]
    fn decide_mouse_context_integer_value_delegates_bpm_ids() {
        let mut resource = SongLengthResource::with_length_ms(150_000);
        resource.song.chart.maxbpm = 200;
        resource.song.chart.minbpm = 100;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(90),
            200,
            "DecideMouseContext::integer_value(90) must delegate maxbpm, not return 0"
        );
        assert_eq!(
            ctx.integer_value(91),
            100,
            "DecideMouseContext::integer_value(91) must delegate minbpm, not return 0"
        );
    }

    #[test]
    fn decide_mouse_context_image_index_value_delegates_lnmode() {
        // Set lnmode config to a non-zero sentinel so we can distinguish
        // the chart-based override (CHARGENOTE -> 1) from the config fallback.
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_CHARGENOTE;
        resource.player_config.play_settings.lnmode = 99;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(308),
            1,
            "DecideMouseContext::image_index_value(308) must return 1 (CN) from chart override, not config lnmode (99)"
        );
    }

    #[test]
    fn decide_mouse_context_string_value_delegates_title() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.metadata.title = "DecideTitle".to_string();

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.string_value(10),
            "DecideTitle",
            "DecideMouseContext::string_value(10) must delegate title, not return empty"
        );
    }

    // DecideRenderContext / DecideMouseContext integer_value ID 96 (chart level) tests

    #[test]
    fn decide_render_context_integer_value_chart_level() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.level = 12;
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(96),
            12,
            "DecideRenderContext::integer_value(96) must return chart level"
        );
    }

    #[test]
    fn decide_render_context_integer_value_chart_level_no_songdata() {
        let mut resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(96),
            i32::MIN,
            "DecideRenderContext::integer_value(96) must return i32::MIN when songdata is absent"
        );
    }

    // ============================================================
    // DecideMouseContext set_float_value / notify_audio_config_changed tests
    // ============================================================

    #[test]
    fn decide_mouse_context_set_float_value_updates_system_volume() {
        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let audio_configs = Arc::new(Mutex::new(Vec::new()));
        let mut config = rubato_types::config::Config::default();
        config.audio = Some(rubato_types::audio_config::AudioConfig::default());

        let mut timer = TimerManager::new();
        let mut main =
            MainControllerRef::new(Box::new(RecordingMainController::with_audio_recording(
                Arc::clone(&changed_states),
                Arc::clone(&audio_configs),
                config,
            )));
        let mut resource = NullPlayerResource::new();
        {
            let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
            let mut ctx = DecideMouseContext {
                timer: &mut timer,
                main: &mut main,
                resource: &mut resource,
                score_data_property: &sdp,
                offsets: &EMPTY_OFFSETS,
                pending_events: Vec::new(),
            };
            use rubato_types::skin_render_context::SkinRenderContext;
            ctx.set_float_value(17, 0.75);
        }
        let configs = audio_configs.lock().expect("mutex poisoned");
        assert_eq!(
            configs.len(),
            1,
            "set_float_value(17) must call update_audio_config"
        );
        assert!(
            (configs[0].systemvolume - 0.75).abs() < f32::EPSILON,
            "systemvolume should be 0.75, got {}",
            configs[0].systemvolume
        );
    }

    #[test]
    fn decide_mouse_context_set_float_value_updates_key_volume() {
        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let audio_configs = Arc::new(Mutex::new(Vec::new()));
        let mut config = rubato_types::config::Config::default();
        config.audio = Some(rubato_types::audio_config::AudioConfig::default());

        let mut timer = TimerManager::new();
        let mut main =
            MainControllerRef::new(Box::new(RecordingMainController::with_audio_recording(
                Arc::clone(&changed_states),
                Arc::clone(&audio_configs),
                config,
            )));
        let mut resource = NullPlayerResource::new();
        {
            let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
            let mut ctx = DecideMouseContext {
                timer: &mut timer,
                main: &mut main,
                resource: &mut resource,
                score_data_property: &sdp,
                offsets: &EMPTY_OFFSETS,
                pending_events: Vec::new(),
            };
            use rubato_types::skin_render_context::SkinRenderContext;
            ctx.set_float_value(18, 0.5);
        }
        let configs = audio_configs.lock().expect("mutex poisoned");
        assert_eq!(
            configs.len(),
            1,
            "set_float_value(18) must call update_audio_config"
        );
        assert!(
            (configs[0].keyvolume - 0.5).abs() < f32::EPSILON,
            "keyvolume should be 0.5, got {}",
            configs[0].keyvolume
        );
    }

    #[test]
    fn decide_mouse_context_set_float_value_updates_bg_volume() {
        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let audio_configs = Arc::new(Mutex::new(Vec::new()));
        let mut config = rubato_types::config::Config::default();
        config.audio = Some(rubato_types::audio_config::AudioConfig::default());

        let mut timer = TimerManager::new();
        let mut main =
            MainControllerRef::new(Box::new(RecordingMainController::with_audio_recording(
                Arc::clone(&changed_states),
                Arc::clone(&audio_configs),
                config,
            )));
        let mut resource = NullPlayerResource::new();
        {
            let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
            let mut ctx = DecideMouseContext {
                timer: &mut timer,
                main: &mut main,
                resource: &mut resource,
                score_data_property: &sdp,
                offsets: &EMPTY_OFFSETS,
                pending_events: Vec::new(),
            };
            use rubato_types::skin_render_context::SkinRenderContext;
            ctx.set_float_value(19, 0.25);
        }
        let configs = audio_configs.lock().expect("mutex poisoned");
        assert_eq!(
            configs.len(),
            1,
            "set_float_value(19) must call update_audio_config"
        );
        assert!(
            (configs[0].bgvolume - 0.25).abs() < f32::EPSILON,
            "bgvolume should be 0.25, got {}",
            configs[0].bgvolume
        );
    }

    #[test]
    fn decide_mouse_context_set_float_value_clamps_to_0_1() {
        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let audio_configs = Arc::new(Mutex::new(Vec::new()));
        let mut config = rubato_types::config::Config::default();
        config.audio = Some(rubato_types::audio_config::AudioConfig::default());

        let mut timer = TimerManager::new();
        let mut main =
            MainControllerRef::new(Box::new(RecordingMainController::with_audio_recording(
                Arc::clone(&changed_states),
                Arc::clone(&audio_configs),
                config,
            )));
        let mut resource = NullPlayerResource::new();
        {
            let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
            let mut ctx = DecideMouseContext {
                timer: &mut timer,
                main: &mut main,
                resource: &mut resource,
                score_data_property: &sdp,
                offsets: &EMPTY_OFFSETS,
                pending_events: Vec::new(),
            };
            use rubato_types::skin_render_context::SkinRenderContext;
            ctx.set_float_value(17, 1.5); // over 1.0
        }
        let configs = audio_configs.lock().expect("mutex poisoned");
        assert_eq!(configs.len(), 1);
        assert!(
            (configs[0].systemvolume - 1.0).abs() < f32::EPSILON,
            "systemvolume should be clamped to 1.0, got {}",
            configs[0].systemvolume
        );
    }

    #[test]
    fn decide_mouse_context_set_float_value_ignores_non_volume_ids() {
        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let audio_configs = Arc::new(Mutex::new(Vec::new()));
        let mut config = rubato_types::config::Config::default();
        config.audio = Some(rubato_types::audio_config::AudioConfig::default());

        let mut timer = TimerManager::new();
        let mut main =
            MainControllerRef::new(Box::new(RecordingMainController::with_audio_recording(
                Arc::clone(&changed_states),
                Arc::clone(&audio_configs),
                config,
            )));
        let mut resource = NullPlayerResource::new();
        {
            let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
            let mut ctx = DecideMouseContext {
                timer: &mut timer,
                main: &mut main,
                resource: &mut resource,
                score_data_property: &sdp,
                offsets: &EMPTY_OFFSETS,
                pending_events: Vec::new(),
            };
            use rubato_types::skin_render_context::SkinRenderContext;
            ctx.set_float_value(99, 0.5); // not a volume ID
        }
        let configs = audio_configs.lock().expect("mutex poisoned");
        assert!(
            configs.is_empty(),
            "set_float_value with non-volume ID should not call update_audio_config"
        );
    }

    #[test]
    fn decide_mouse_context_notify_audio_config_changed_propagates() {
        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let audio_configs = Arc::new(Mutex::new(Vec::new()));
        let mut config = rubato_types::config::Config::default();
        let mut audio = rubato_types::audio_config::AudioConfig::default();
        audio.systemvolume = 0.42;
        config.audio = Some(audio);

        let mut timer = TimerManager::new();
        let mut main =
            MainControllerRef::new(Box::new(RecordingMainController::with_audio_recording(
                Arc::clone(&changed_states),
                Arc::clone(&audio_configs),
                config,
            )));
        let mut resource = NullPlayerResource::new();
        {
            let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
            let mut ctx = DecideMouseContext {
                timer: &mut timer,
                main: &mut main,
                resource: &mut resource,
                score_data_property: &sdp,
                offsets: &EMPTY_OFFSETS,
                pending_events: Vec::new(),
            };
            use rubato_types::skin_render_context::SkinRenderContext;
            ctx.notify_audio_config_changed();
        }
        let configs = audio_configs.lock().expect("mutex poisoned");
        assert_eq!(
            configs.len(),
            1,
            "notify_audio_config_changed must call update_audio_config"
        );
        assert!(
            (configs[0].systemvolume - 0.42).abs() < f32::EPSILON,
            "propagated audio config should preserve systemvolume=0.42, got {}",
            configs[0].systemvolume
        );
    }

    #[test]
    fn decide_mouse_context_set_float_value_noop_without_audio_config() {
        // When config.audio is None, set_float_value should be a no-op
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let mut resource = NullPlayerResource::new();
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let mut ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // Should not panic
        ctx.set_float_value(17, 0.5);
    }

    #[test]
    fn decide_mouse_context_notify_audio_config_changed_noop_without_audio_config() {
        // When config.audio is None, notify_audio_config_changed should be a no-op
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let mut resource = NullPlayerResource::new();
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let mut ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // Should not panic
        ctx.notify_audio_config_changed();
    }

    // ============================================================
    // replay_option_data delegation tests
    // ============================================================

    #[test]
    fn decide_render_context_replay_option_data_returns_none_without_replay() {
        // Regression: DecideRenderContext must delegate replay_option_data to resource.
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(
            ctx.replay_option_data().is_none(),
            "DecideRenderContext::replay_option_data() must return None when resource has no replay"
        );
    }

    #[test]
    fn decide_render_context_replay_option_data_returns_some_with_replay() {
        // Regression: DecideRenderContext must delegate replay_option_data to resource.
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut rd = rubato_types::replay_data::ReplayData::default();
        rd.randomoption = 3; // RANDOM option
        resource.replay_data = Some(rd);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let replay = ctx
            .replay_option_data()
            .expect("must return Some when resource has replay data");
        assert_eq!(replay.randomoption, 3);
    }

    #[test]
    fn decide_mouse_context_replay_option_data_returns_none_without_replay() {
        // Regression: DecideMouseContext must delegate replay_option_data to resource.
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let mut resource = NullPlayerResource::new();
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(
            ctx.replay_option_data().is_none(),
            "DecideMouseContext::replay_option_data() must return None when resource has no replay"
        );
    }

    #[test]
    fn decide_mouse_context_replay_option_data_returns_some_with_replay() {
        // Regression: DecideMouseContext must delegate replay_option_data to resource.
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut rd = rubato_types::replay_data::ReplayData::default();
        rd.doubleoption = 2; // DP option
        resource.replay_data = Some(rd);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let replay = ctx
            .replay_option_data()
            .expect("must return Some when resource has replay data");
        assert_eq!(replay.doubleoption, 2);
    }

    // ============================================================
    // Player profile stats (IDs 30-37, 333) tests
    // ============================================================

    fn make_player_data_resource() -> SongLengthResource {
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut pd = rubato_types::player_data::PlayerData::new();
        pd.playcount = 100;
        pd.clear = 75;
        // PG: epg=20, lpg=10 => judge_count(0)=30
        pd.epg = 20;
        pd.lpg = 10;
        // GR: egr=15, lgr=5 => judge_count(1)=20
        pd.egr = 15;
        pd.lgr = 5;
        // GD: egd=7, lgd=3 => judge_count(2)=10
        pd.egd = 7;
        pd.lgd = 3;
        // BD: ebd=2, lbd=1 => judge_count(3)=3
        pd.ebd = 2;
        pd.lbd = 1;
        // PR: epr=8, lpr=2 => judge_count(4)=10
        pd.epr = 8;
        pd.lpr = 2;
        resource.player_data = Some(pd);
        resource
    }

    #[test]
    fn decide_render_context_player_profile_stats() {
        let mut resource = make_player_data_resource();
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(30), 100); // playcount
        assert_eq!(ctx.integer_value(31), 75); // clear
        assert_eq!(ctx.integer_value(32), 25); // playcount - clear
        assert_eq!(ctx.integer_value(33), 30); // PG
        assert_eq!(ctx.integer_value(34), 20); // GR
        assert_eq!(ctx.integer_value(35), 10); // GD
        assert_eq!(ctx.integer_value(36), 3); // BD
        assert_eq!(ctx.integer_value(37), 10); // PR
        // 333 = total of judges 0-3: 30+20+10+3 = 63
        assert_eq!(ctx.integer_value(333), 63);
    }

    #[test]
    fn decide_render_context_player_profile_stats_no_player_data() {
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        for id in 30..=37 {
            assert_eq!(
                ctx.integer_value(id),
                0,
                "ID {id} should be 0 without player data"
            );
        }
        assert_eq!(
            ctx.integer_value(333),
            0,
            "ID 333 should be 0 without player data"
        );
    }

    #[test]
    fn decide_mouse_context_player_profile_stats() {
        let mut resource = make_player_data_resource();
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(30), 100); // playcount
        assert_eq!(ctx.integer_value(31), 75); // clear
        assert_eq!(ctx.integer_value(32), 25); // playcount - clear
        assert_eq!(ctx.integer_value(33), 30); // PG
        assert_eq!(ctx.integer_value(34), 20); // GR
        assert_eq!(ctx.integer_value(35), 10); // GD
        assert_eq!(ctx.integer_value(36), 3); // BD
        assert_eq!(ctx.integer_value(37), 10); // PR
        assert_eq!(ctx.integer_value(333), 63); // total judges 0-3
    }

    // ============================================================
    // DecideRenderContext target_score_data / rival_score_data_ref
    // ============================================================

    #[test]
    fn decide_render_context_target_score_data_delegates_to_resource() {
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut target = crate::core::score_data::ScoreData::default();
        target.notes = 999;
        target.judge_counts.epg = 500;
        resource.target_score = Some(target);

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let target_data = ctx.target_score_data();
        assert!(
            target_data.is_some(),
            "target_score_data must delegate to resource"
        );
        assert_eq!(target_data.unwrap().notes, 999);
    }

    #[test]
    fn decide_render_context_rival_score_data_ref_delegates_to_resource() {
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut rival = crate::core::score_data::ScoreData::default();
        rival.notes = 777;
        rival.judge_counts.egr = 200;
        resource.rival_score = Some(rival);

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let rival_data = ctx.rival_score_data_ref();
        assert!(
            rival_data.is_some(),
            "rival_score_data_ref must delegate to resource"
        );
        assert_eq!(rival_data.unwrap().notes, 777);
    }

    #[test]
    fn decide_render_context_target_and_rival_none_when_absent() {
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.target_score_data().is_none());
        assert!(ctx.rival_score_data_ref().is_none());
    }

    #[test]
    fn decide_mouse_context_target_score_data_delegates_to_resource() {
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut target = crate::core::score_data::ScoreData::default();
        target.notes = 888;
        resource.target_score = Some(target);

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let target_data = ctx.target_score_data();
        assert!(
            target_data.is_some(),
            "DecideMouseContext::target_score_data must delegate"
        );
        assert_eq!(target_data.unwrap().notes, 888);
    }

    #[test]
    fn decide_mouse_context_rival_score_data_ref_delegates_to_resource() {
        let mut resource = SongLengthResource::with_length_ms(100_000);
        let mut rival = crate::core::score_data::ScoreData::default();
        rival.notes = 666;
        resource.rival_score = Some(rival);

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let rival_data = ctx.rival_score_data_ref();
        assert!(
            rival_data.is_some(),
            "DecideMouseContext::rival_score_data_ref must delegate"
        );
        assert_eq!(rival_data.unwrap().notes, 666);
    }

    // ============================================================
    // integer_value: ScoreDataProperty-backed IDs (71, 80-84, 100, 102, 103)
    // ============================================================

    #[test]
    fn decide_render_context_integer_value_exscore_71() {
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut score = crate::core::score_data::ScoreData::new(bms::model::mode::Mode::BEAT_7K);
        score.judge_counts.epg = 50;
        score.notes = 100;
        resource.score = Some(score.clone());

        let mut sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        sdp.update_score(Some(&score));

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // exscore = epg*2 = 100
        assert_eq!(
            ctx.integer_value(71),
            100,
            "ID 71 should return exscore from score_data"
        );
    }

    #[test]
    fn decide_render_context_integer_value_judge_counts_80_84() {
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut score = crate::core::score_data::ScoreData::new(bms::model::mode::Mode::BEAT_7K);
        score.judge_counts.epg = 10;
        score.judge_counts.lpg = 5;
        score.judge_counts.egr = 3;
        score.judge_counts.lgr = 2;
        score.notes = 100;
        resource.score = Some(score.clone());

        let mut sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        sdp.update_score(Some(&score));

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // ID 80 = PG total = epg+lpg = 15
        assert_eq!(
            ctx.integer_value(80),
            15,
            "ID 80 should return PG judge_count_total"
        );
        // ID 81 = GR total = egr+lgr = 5
        assert_eq!(
            ctx.integer_value(81),
            5,
            "ID 81 should return GR judge_count_total"
        );
    }

    #[test]
    fn decide_render_context_integer_value_score_rate_102_103() {
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut score = crate::core::score_data::ScoreData::new(bms::model::mode::Mode::BEAT_7K);
        score.judge_counts.epg = 50;
        score.notes = 100;
        resource.score = Some(score.clone());

        let mut sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        sdp.update_score(Some(&score));

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // rate = 100/200 = 0.5, rate_int = 50, afterdot = 0
        // But these are "now" rate based on current notes, not total rate.
        // nowrate_int corresponds to ID 102, nowrate_after_dot to ID 103.
        assert_eq!(
            ctx.integer_value(102),
            sdp.nowrate_int,
            "ID 102 should return nowrate_int"
        );
        assert_eq!(
            ctx.integer_value(103),
            sdp.nowrate_after_dot,
            "ID 103 should return nowrate_after_dot"
        );
    }

    #[test]
    fn decide_render_context_integer_value_hispeed_10() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.mode = 7;
        resource.player_config.mode7.playconfig.hispeed = 2.5;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // ID 10 = NUMBER_HISPEED_LR2 = (hispeed * 100) as i32 = 250
        assert_eq!(
            ctx.integer_value(10),
            250,
            "ID 10 should return hispeed * 100"
        );
    }

    #[test]
    fn decide_render_context_integer_value_judgetiming_12() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.player_config.judge_settings.judgetiming = 5;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(12), 5, "ID 12 should return judgetiming");
    }

    // ============================================================
    // boolean_value: BGA on/off (40/41), stagefile/banner/backbmp (190-195),
    // course stage (280-283, 289, 290), save score (60/61)
    // ============================================================

    #[test]
    fn decide_render_context_boolean_value_bga_off_on() {
        use rubato_types::config::BgaMode;
        let mut resource = SongLengthResource::with_length_ms(0);

        let mut timer = TimerManager::new();
        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let mut config = rubato_types::config::Config::default();
        config.render.bga = BgaMode::Off;
        let audio_configs = Arc::new(Mutex::new(Vec::new()));
        let mut main =
            MainControllerRef::new(Box::new(RecordingMainController::with_audio_recording(
                Arc::clone(&changed_states),
                Arc::clone(&audio_configs),
                config,
            )));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(
            ctx.boolean_value(40),
            "ID 40 (BGAOFF) should be true when BGA is Off"
        );
        assert!(
            !ctx.boolean_value(41),
            "ID 41 (BGAON) should be false when BGA is Off"
        );
    }

    #[test]
    fn decide_render_context_boolean_value_stagefile_exists() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.file.stagefile = "stage.png".to_string();

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(
            ctx.boolean_value(191),
            "ID 191 (STAGEFILE) should be true when stagefile is set"
        );
        assert!(
            !ctx.boolean_value(190),
            "ID 190 (NO_STAGEFILE) should be false when stagefile is set"
        );
    }

    #[test]
    fn decide_render_context_boolean_value_course_mode() {
        let mut resource = SongLengthResource::with_length_ms(0);

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // No course data -> course mode is false
        assert!(
            !ctx.boolean_value(290),
            "ID 290 (MODE_COURSE) should be false when not in course mode"
        );
    }

    // ============================================================
    // DecideMouseContext: mirror tests for the same IDs
    // ============================================================

    #[test]
    fn decide_mouse_context_integer_value_exscore_71() {
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut score = crate::core::score_data::ScoreData::new(bms::model::mode::Mode::BEAT_7K);
        score.judge_counts.epg = 50;
        score.notes = 100;
        resource.score = Some(score.clone());

        let mut sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        sdp.update_score(Some(&score));

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(71),
            100,
            "DecideMouseContext ID 71 should return exscore"
        );
    }

    #[test]
    fn decide_mouse_context_boolean_value_bga_off() {
        use rubato_types::config::BgaMode;
        let mut resource = SongLengthResource::with_length_ms(0);

        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let mut config = rubato_types::config::Config::default();
        config.render.bga = BgaMode::Off;
        let audio_configs = Arc::new(Mutex::new(Vec::new()));
        let mut timer = TimerManager::new();
        let mut main =
            MainControllerRef::new(Box::new(RecordingMainController::with_audio_recording(
                Arc::clone(&changed_states),
                Arc::clone(&audio_configs),
                config,
            )));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(
            ctx.boolean_value(40),
            "DecideMouseContext ID 40 (BGAOFF) should be true when BGA is Off"
        );
    }

    #[test]
    fn decide_render_context_integer_value_400_returns_chart_judge() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.judge = 42;
        // Set judgetiming to a different value to ensure we are NOT returning it
        resource.player_config.judge_settings.judgetiming = 999;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(400),
            42,
            "DecideRenderContext::integer_value(400) must return chart judge rank, not judgetiming"
        );
    }

    #[test]
    fn decide_render_context_integer_value_400_no_songdata() {
        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let mut resource = NullPlayerResource::new();
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &mut resource,
            main: &mut main,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(400),
            i32::MIN,
            "DecideRenderContext::integer_value(400) must return i32::MIN when songdata is absent"
        );
    }

    #[test]
    fn decide_mouse_context_integer_value_400_returns_chart_judge() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.judge = 77;
        // Set judgetiming to a different value to ensure we are NOT returning it
        resource.player_config.judge_settings.judgetiming = 888;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let sdp = rubato_types::score_data_property::ScoreDataProperty::new();
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
            score_data_property: &sdp,
            offsets: &EMPTY_OFFSETS,
            pending_events: Vec::new(),
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(400),
            77,
            "DecideMouseContext::integer_value(400) must return chart judge rank, not judgetiming"
        );
    }
}

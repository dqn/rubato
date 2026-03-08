use rubato_core::clear_type::ClearType;
use rubato_core::timer_manager::TimerManager;

use super::MusicResult;
use crate::result::abstract_result::AbstractResultData;
use crate::result::stubs::{MainController, PlayerResource};

/// Render context adapter for result screen skin rendering.
/// Provides score data, gauge, config through SkinRenderContext.
pub(super) struct ResultRenderContext<'a> {
    pub(super) timer: &'a mut TimerManager,
    pub(super) data: &'a AbstractResultData,
    pub(super) resource: &'a PlayerResource,
    pub(super) main: &'a MainController,
}

impl rubato_types::timer_access::TimerAccess for ResultRenderContext<'_> {
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

impl rubato_types::skin_render_context::SkinRenderContext for ResultRenderContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Result)
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(self.resource.player_config())
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(self.main.config())
    }

    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        self.resource.replay_data()
    }

    fn target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.resource.target_score_data()
    }

    fn score_data_ref(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.data.score.score.as_ref()
    }

    fn rival_score_data_ref(&self) -> Option<&rubato_core::score_data::ScoreData> {
        Some(&self.data.oldscore)
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        let mode = self
            .resource
            .songdata()
            .and_then(|song| match song.chart.mode {
                5 => Some(bms_model::mode::Mode::BEAT_5K),
                7 => Some(bms_model::mode::Mode::BEAT_7K),
                9 => Some(bms_model::mode::Mode::POPN_9K),
                10 => Some(bms_model::mode::Mode::BEAT_10K),
                14 => Some(bms_model::mode::Mode::BEAT_14K),
                25 => Some(bms_model::mode::Mode::KEYBOARD_24K),
                50 => Some(bms_model::mode::Mode::KEYBOARD_24K_DOUBLE),
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

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.resource.songdata()
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn gauge_value(&self) -> f32 {
        // Return final gauge value from score data
        self.data.oldscore.play_option.gauge as f32 / 100.0
    }

    fn gauge_type(&self) -> i32 {
        self.data.gauge_type
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        self.data
            .score
            .score
            .as_ref()
            .map_or(0, |s| s.judge_count(judge, fast))
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            // EX score
            71 => self.data.score.nowscore,
            // Max combo
            75 => self.data.score.score.as_ref().map_or(0, |s| s.maxcombo),
            // Miss count
            76 => self.data.score.score.as_ref().map_or(0, |s| s.minbp),
            // Total notes
            350 => self.data.score.totalnotes,
            // Playtime (hours/minutes/seconds from boot)
            17 => (self.timer.now_time() / 3_600_000) as i32,
            18 => ((self.timer.now_time() % 3_600_000) / 60_000) as i32,
            19 => ((self.timer.now_time() % 60_000) / 1_000) as i32,
            _ => 0,
        }
    }

    fn float_value(&self, id: i32) -> f32 {
        match id {
            // Score rate
            1102 => self.data.score.rate,
            _ => 0.0,
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        match id {
            // Clear result
            90 => self.data.oldscore.clear >= ClearType::AssistEasy as i32,
            // Fail result
            91 => self.data.oldscore.clear < ClearType::AssistEasy as i32,
            _ => false,
        }
    }

    fn string_value(&self, id: i32) -> String {
        match id {
            // Song metadata from resource
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
}

pub(super) fn replay_index_from_event_id(event_id: i32) -> Option<usize> {
    match event_id {
        19 => Some(0),
        316 => Some(1),
        317 => Some(2),
        318 => Some(3),
        _ => None,
    }
}

pub(super) struct ResultMouseContext<'a> {
    pub(super) timer: &'a mut TimerManager,
    pub(super) result: &'a mut MusicResult,
}

impl rubato_types::timer_access::TimerAccess for ResultMouseContext<'_> {
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

impl rubato_types::skin_render_context::SkinRenderContext for ResultMouseContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Result)
    }

    fn execute_event(&mut self, id: i32, _arg1: i32, _arg2: i32) {
        if let Some(index) = replay_index_from_event_id(id) {
            self.result.save_replay_data(index);
        }
    }

    fn change_state(&mut self, state: rubato_types::main_state_type::MainStateType) {
        self.result.main.change_state(state);
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        self.result.resource.player_config_mut()
    }
}

use super::CourseResult;
use crate::result::abstract_result::AbstractResultData;
use crate::result::shared_render_context;
use crate::result::{MainController, PlayerResource};

pub(super) struct CourseResultRenderContext<'a> {
    pub(super) timer: &'a mut rubato_core::timer_manager::TimerManager,
    pub(super) data: &'a AbstractResultData,
    pub(super) resource: &'a PlayerResource,
    pub(super) main: &'a MainController,
}

impl rubato_types::timer_access::TimerAccess for CourseResultRenderContext<'_> {
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

impl rubato_types::skin_render_context::SkinRenderContext for CourseResultRenderContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::CourseResult)
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        shared_render_context::player_config_ref(self.resource)
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        shared_render_context::config_ref(self.main)
    }

    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        shared_render_context::replay_option_data(self.resource)
    }

    fn target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
        shared_render_context::target_score_data(self.resource)
    }

    fn score_data_ref(&self) -> Option<&rubato_core::score_data::ScoreData> {
        shared_render_context::score_data_ref(self.data)
    }

    fn rival_score_data_ref(&self) -> Option<&rubato_core::score_data::ScoreData> {
        shared_render_context::rival_score_data_ref(self.data)
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        let course = self.resource.course_data()?;
        let mut current_mode: Option<bms_model::mode::Mode> = None;
        for song in &course.hash {
            let song_mode = match song.chart.mode {
                5 => Some(bms_model::mode::Mode::BEAT_5K),
                7 => Some(bms_model::mode::Mode::BEAT_7K),
                9 => Some(bms_model::mode::Mode::POPN_9K),
                10 => Some(bms_model::mode::Mode::BEAT_10K),
                14 => Some(bms_model::mode::Mode::BEAT_14K),
                25 => Some(bms_model::mode::Mode::KEYBOARD_24K),
                50 => Some(bms_model::mode::Mode::KEYBOARD_24K_DOUBLE),
                _ => None,
            }?;
            if let Some(mode) = current_mode.as_ref() {
                if *mode != song_mode {
                    return None;
                }
            } else {
                current_mode = Some(song_mode);
            }
        }
        Some(
            &self
                .resource
                .player_config()
                .play_config_ref(current_mode?)
                .playconfig,
        )
    }

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        shared_render_context::song_data_ref(self.resource)
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn gauge_value(&self) -> f32 {
        shared_render_context::gauge_value(self.resource)
    }

    fn gauge_type(&self) -> i32 {
        shared_render_context::gauge_type(self.data)
    }

    fn is_gauge_max(&self) -> bool {
        shared_render_context::is_gauge_max(self.resource)
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        shared_render_context::judge_count(self.data, judge, fast)
    }

    fn integer_value(&self, id: i32) -> i32 {
        shared_render_context::integer_value(self.data, self.timer.now_time(), id)
    }

    fn ranking_score_clear_type(&self, slot: i32) -> i32 {
        shared_render_context::ranking_score_clear_type(self.data, slot)
    }

    fn ranking_offset(&self) -> i32 {
        shared_render_context::ranking_offset(self.data)
    }

    fn float_value(&self, id: i32) -> f32 {
        match id {
            // FLOAT_GROOVEGAUGE_1P (1107): needs PlayerResource for gauge data.
            // Java: AbstractResult -> gauge[gaugeType].last()
            1107 => shared_render_context::gauge_value(self.resource),
            _ => shared_render_context::float_value(self.data, id),
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        shared_render_context::boolean_value(self.data, self.resource.course_score_data(), id)
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
            14 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.artist.clone()),
            120..=129 => shared_render_context::ranking_name(self.data, id - 120),
            _ => String::new(),
        }
    }

    fn gauge_history(&self) -> Option<&Vec<Vec<f32>>> {
        shared_render_context::gauge_history(self.resource)
    }

    fn course_gauge_history(&self) -> &[Vec<Vec<f32>>] {
        shared_render_context::course_gauge_history(self.resource)
    }

    fn gauge_border_max(&self) -> Option<(f32, f32)> {
        shared_render_context::gauge_border_max(self.resource, self.data.gauge_type)
    }

    fn get_timing_distribution(
        &self,
    ) -> Option<&rubato_types::timing_distribution::TimingDistribution> {
        shared_render_context::get_timing_distribution(self.data)
    }

    fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
        shared_render_context::score_data_property(self.data)
    }

    fn judge_area(&self) -> Option<Vec<Vec<i32>>> {
        shared_render_context::judge_area(self.resource)
    }
}

impl rubato_skin::main_state::MainState for CourseResultRenderContext<'_> {}

pub(super) struct CourseResultMouseContext<'a> {
    pub(super) timer: &'a mut rubato_core::timer_manager::TimerManager,
    pub(super) result: &'a mut CourseResult,
}

impl rubato_types::timer_access::TimerAccess for CourseResultMouseContext<'_> {
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

impl rubato_types::skin_render_context::SkinRenderContext for CourseResultMouseContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::CourseResult)
    }

    fn execute_event(&mut self, id: i32, _arg1: i32, _arg2: i32) {
        if let Some(index) = shared_render_context::replay_index_from_event_id(id) {
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

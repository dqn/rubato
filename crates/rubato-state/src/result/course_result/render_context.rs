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
            };
            let song_mode = match song_mode {
                Some(m) => m,
                None => continue,
            };
            if let Some(mode) = current_mode.as_ref() {
                if *mode != song_mode {
                    return None;
                }
            } else {
                current_mode = Some(song_mode);
            }
        }
        let resolved_mode = current_mode.unwrap_or(bms_model::mode::Mode::BEAT_7K);
        Some(
            &self
                .resource
                .player_config()
                .play_config_ref(resolved_mode)
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

    fn gauge_min(&self) -> f32 {
        shared_render_context::gauge_min(self.resource, self.data.gauge_type)
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        shared_render_context::judge_count(self.data, judge, fast)
    }

    fn image_index_value(&self, id: i32) -> i32 {
        match id {
            // Java IntegerPropertyFactory ID 308 (lnmode): on CourseResult, override
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
            1107 => shared_render_context::gauge_value(self.resource),
            // Explicit ID dispatch for shared float IDs to avoid 0.0 sentinel bug.
            85..=89 | 110..=115 | 122 | 135 | 155 | 157 | 183 | 285..=289 | 1102 | 1115 => {
                shared_render_context::float_value(self.data, id)
            }
            _ => self.default_float_value(id),
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        match id {
            90 | 91 => shared_render_context::boolean_value(
                self.data,
                self.resource.course_score_data(),
                id,
            ),
            _ => self.default_boolean_value(id),
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

    fn gauge_element_borders(&self) -> Vec<(f32, f32)> {
        shared_render_context::gauge_element_borders(self.resource)
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

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        shared_render_context::player_config_ref(&self.result.resource)
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        shared_render_context::config_ref(&self.result.main)
    }

    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        shared_render_context::replay_option_data(&self.result.resource)
    }

    fn target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
        shared_render_context::target_score_data(&self.result.resource)
    }

    fn score_data_ref(&self) -> Option<&rubato_core::score_data::ScoreData> {
        shared_render_context::score_data_ref(&self.result.data)
    }

    fn rival_score_data_ref(&self) -> Option<&rubato_core::score_data::ScoreData> {
        shared_render_context::rival_score_data_ref(&self.result.data)
    }

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        shared_render_context::song_data_ref(&self.result.resource)
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        let course = self.result.resource.course_data()?;
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
            };
            let song_mode = match song_mode {
                Some(m) => m,
                None => continue,
            };
            if let Some(mode) = current_mode.as_ref() {
                if *mode != song_mode {
                    return None;
                }
            } else {
                current_mode = Some(song_mode);
            }
        }
        let resolved_mode = current_mode.unwrap_or(bms_model::mode::Mode::BEAT_7K);
        Some(
            &self
                .result
                .resource
                .player_config()
                .play_config_ref(resolved_mode)
                .playconfig,
        )
    }

    fn gauge_value(&self) -> f32 {
        shared_render_context::gauge_value(&self.result.resource)
    }

    fn gauge_type(&self) -> i32 {
        shared_render_context::gauge_type(&self.result.data)
    }

    fn is_gauge_max(&self) -> bool {
        shared_render_context::is_gauge_max(&self.result.resource)
    }

    fn gauge_min(&self) -> f32 {
        shared_render_context::gauge_min(&self.result.resource, self.result.data.gauge_type)
    }

    fn gauge_border_max(&self) -> Option<(f32, f32)> {
        shared_render_context::gauge_border_max(&self.result.resource, self.result.data.gauge_type)
    }

    fn gauge_history(&self) -> Option<&Vec<Vec<f32>>> {
        shared_render_context::gauge_history(&self.result.resource)
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        shared_render_context::judge_count(&self.result.data, judge, fast)
    }

    fn judge_area(&self) -> Option<Vec<Vec<i32>>> {
        shared_render_context::judge_area(&self.result.resource)
    }

    fn gauge_element_borders(&self) -> Vec<(f32, f32)> {
        shared_render_context::gauge_element_borders(&self.result.resource)
    }

    fn get_timing_distribution(
        &self,
    ) -> Option<&rubato_types::timing_distribution::TimingDistribution> {
        shared_render_context::get_timing_distribution(&self.result.data)
    }

    fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
        shared_render_context::score_data_property(&self.result.data)
    }

    fn image_index_value(&self, id: i32) -> i32 {
        match id {
            308 => {
                if let Some(song) = self.result.resource.songdata()
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

    fn integer_value(&self, id: i32) -> i32 {
        shared_render_context::integer_value(&self.result.data, self.timer.now_time(), id)
    }

    fn ranking_score_clear_type(&self, slot: i32) -> i32 {
        shared_render_context::ranking_score_clear_type(&self.result.data, slot)
    }

    fn ranking_offset(&self) -> i32 {
        shared_render_context::ranking_offset(&self.result.data)
    }

    fn float_value(&self, id: i32) -> f32 {
        match id {
            1107 => shared_render_context::gauge_value(&self.result.resource),
            // Explicit ID dispatch for shared float IDs to avoid 0.0 sentinel bug.
            85..=89 | 110..=115 | 122 | 135 | 155 | 157 | 183 | 285..=289 | 1102 | 1115 => {
                shared_render_context::float_value(&self.result.data, id)
            }
            _ => self.default_float_value(id),
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        match id {
            90 | 91 => shared_render_context::boolean_value(
                &self.result.data,
                self.result.resource.course_score_data(),
                id,
            ),
            _ => self.default_boolean_value(id),
        }
    }

    fn string_value(&self, id: i32) -> String {
        match id {
            10 => self
                .result
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.title.clone()),
            11 => self
                .result
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.subtitle.clone()),
            12 => self
                .result
                .resource
                .songdata()
                .map_or_else(String::new, |s| {
                    if s.metadata.subtitle.is_empty() {
                        s.metadata.title.clone()
                    } else {
                        format!("{} {}", s.metadata.title, s.metadata.subtitle)
                    }
                }),
            13 => self
                .result
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.genre.clone()),
            14 => self
                .result
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.artist.clone()),
            15 => self
                .result
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.subartist.clone()),
            16 => self
                .result
                .resource
                .songdata()
                .map_or_else(String::new, |s| {
                    if s.metadata.subartist.is_empty() {
                        s.metadata.artist.clone()
                    } else {
                        format!("{} {}", s.metadata.artist, s.metadata.subartist)
                    }
                }),
            120..=129 => shared_render_context::ranking_name(&self.result.data, id - 120),
            _ => String::new(),
        }
    }

    fn course_gauge_history(&self) -> &[Vec<Vec<f32>>] {
        shared_render_context::course_gauge_history(&self.result.resource)
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

    fn set_float_value(&mut self, id: i32, value: f32) {
        if (17..=19).contains(&id)
            && let Some(mut audio) = self.result.main.config().audio.clone()
        {
            let clamped = value.clamp(0.0, 1.0);
            match id {
                17 => audio.systemvolume = clamped,
                18 => audio.keyvolume = clamped,
                19 => audio.bgvolume = clamped,
                _ => unreachable!(),
            }
            self.result.main.update_audio_config(audio);
        }
    }

    fn notify_audio_config_changed(&mut self) {
        if let Some(audio) = self.result.main.config().audio.clone() {
            self.result.main.update_audio_config(audio);
        }
    }

    fn play_option_change_sound(&mut self) {
        self.result.main.play_sound(
            &rubato_core::system_sound_manager::SoundType::OptionChange,
            false,
        );
    }
}

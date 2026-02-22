// CourseResult.java -> course_result.rs
// Mechanical line-by-line translation.

use log::{info, warn};

use beatoraja_core::clear_type::ClearType;
use beatoraja_core::score_data::ScoreData;
use beatoraja_core::system_sound_manager::SoundType;
use beatoraja_skin::skin_property::*;

use crate::abstract_result::{
    AbstractResultData, REPLAY_SIZE, ReplayAutoSaveConstraint, ReplayStatus, STATE_IR_FINISHED,
    STATE_IR_PROCESSING, STATE_OFFLINE,
};
use crate::course_result_skin::CourseResultSkin;
use crate::result_key_property::{ResultKey, ResultKeyProperty};
use std::sync::Arc;

use crate::stubs::{
    BMSPlayerModeType, ControlKeys, EventType, IRConfig, IRConnection, IRCourseData, IRScoreData,
    IRStatus, KeyCommand, MainController, PlayerResource, RankingData,
};
use beatoraja_core::ir_config::{IR_SEND_ALWAYS, IR_SEND_COMPLETE_SONG, IR_SEND_UPDATE_SCORE};

/// IR send status for course result
struct CourseIRSendStatus {
    pub ir: Arc<dyn IRConnection>,
    pub course: beatoraja_core::course_data::CourseData,
    pub lnmode: i32,
    pub score: ScoreData,
    pub retry: i32,
}

impl CourseIRSendStatus {
    pub fn new(
        ir: Arc<dyn IRConnection>,
        course: &beatoraja_core::course_data::CourseData,
        lnmode: i32,
        score: &ScoreData,
    ) -> Self {
        Self {
            ir,
            course: course.clone(),
            lnmode,
            score: score.clone(),
            retry: 0,
        }
    }

    pub fn send(&mut self) -> bool {
        info!("IR score sending: {:?}", self.course.name);
        let ir_course_data = IRCourseData::new_with_lntype(&self.course, self.lnmode);
        let ir_score_data = IRScoreData::new(&self.score);
        let send_result = self
            .ir
            .send_course_play_data(&ir_course_data, &ir_score_data);
        if send_result.is_succeeded() {
            info!("IR score send complete: {:?}", self.course.name);
            self.retry = -255;
            true
        } else {
            warn!("IR score send failed: {}", send_result.get_message());
            self.retry += 1;
            false
        }
    }
}

/// Course result screen
pub struct CourseResult {
    pub data: AbstractResultData,
    ir_send_status: Vec<CourseIRSendStatus>,
    property: ResultKeyProperty,
}

impl CourseResult {
    pub fn new() -> Self {
        Self {
            data: AbstractResultData::new(),
            ir_send_status: Vec::new(),
            property: ResultKeyProperty::beat_7k(),
        }
    }

    pub fn create(&mut self, main: &mut MainController, resource: &mut PlayerResource) {
        for i in 0..REPLAY_SIZE {
            let models = resource.get_course_bms_models();
            if let Some(models) = models {
                self.data.save_replay[i] =
                    if main.get_play_data_accessor().exists_replay_data_course(
                        models,
                        resource.get_player_config().lnmode,
                        i as i32,
                        &resource.get_constraint(),
                    ) {
                        ReplayStatus::Exist
                    } else {
                        ReplayStatus::NotExist
                    };
            }
        }

        // Fill missing course gauge data
        // Collect data first to avoid borrow conflicts
        let mut gauge_fill_data: Vec<Vec<Vec<f32>>> = Vec::new();
        if let Some(models) = resource.get_course_bms_models() {
            let course_gauge_size = resource.get_course_gauge().len();
            let gauge_type_length = resource
                .get_groove_gauge()
                .map(|g| g.get_gauge_type_length())
                .unwrap_or(9);
            for i in course_gauge_size..models.len() {
                let mut list: Vec<Vec<f32>> = Vec::with_capacity(gauge_type_length);
                for _type_idx in 0..gauge_type_length {
                    let last_note_time = models[i].get_last_note_time();
                    let fa = vec![0.0f32; ((last_note_time + 500) / 500) as usize];
                    list.push(fa);
                }
                gauge_fill_data.push(list);
            }
        }
        for list in gauge_fill_data {
            resource.get_course_gauge_mut().push(list);
        }

        if let Some(mode) = resource.get_bms_model().get_mode() {
            self.property = ResultKeyProperty::get(mode).unwrap_or_else(ResultKeyProperty::beat_7k);
        } else {
            self.property = ResultKeyProperty::beat_7k();
        }

        self.update_score_database(main, resource);

        // Replay auto save
        if resource.get_play_mode().mode == BMSPlayerModeType::Play {
            for i in 0..REPLAY_SIZE {
                let auto_save = &resource.get_player_config().autosavereplay;
                if i < auto_save.len()
                    && let Some(new_score) = self.get_new_score(resource)
                    && ReplayAutoSaveConstraint::get(auto_save[i])
                        .is_qualified(&self.data.oldscore, new_score)
                {
                    self.save_replay_data(i, main, resource);
                }
            }
        }

        self.data.gauge_type = resource
            .get_groove_gauge()
            .map(|g| g.get_type())
            .unwrap_or(0);

        // loadSkin(SkinType.COURSE_RESULT);
        log::warn!("not yet implemented: loadSkin(SkinType.COURSE_RESULT)");
    }

    pub fn prepare(&mut self, main: &mut MainController, resource: &mut PlayerResource) {
        self.data.state = STATE_OFFLINE;
        let newscore = self.get_new_score(resource).cloned();

        self.data.ranking = if resource.get_ranking_data().is_some()
            && resource.get_course_bms_models().is_some()
        {
            resource.get_ranking_data().cloned()
        } else {
            Some(RankingData::new())
        };
        self.data.ranking_offset = 0;

        let ir = main.get_ir_status();
        if !ir.is_empty() && resource.get_play_mode().mode == BMSPlayerModeType::Play {
            self.data.state = STATE_IR_PROCESSING;

            let mut uln = false;
            if let Some(models) = resource.get_course_bms_models() {
                for model in models {
                    if model.contains_undefined_long_note() {
                        uln = true;
                        break;
                    }
                }
            }
            let lnmode = if uln {
                resource.get_player_config().lnmode
            } else {
                0
            };

            for irc in ir {
                let send = resource.is_update_course_score()
                    && !resource.is_force_no_ir_send()
                    && resource
                        .get_course_data()
                        .map(|cd| cd.release)
                        .unwrap_or(false);
                match irc.config.get_irsend() {
                    IR_SEND_ALWAYS => {}
                    IR_SEND_COMPLETE_SONG => {
                        // commented out in Java
                    }
                    IR_SEND_UPDATE_SCORE => {
                        // commented out in Java
                    }
                    _ => {}
                }

                if send
                    && let Some(ref ns) = newscore
                    && let Some(course_data) = resource.get_course_data()
                {
                    self.ir_send_status.push(CourseIRSendStatus::new(
                        irc.connection.clone(),
                        course_data,
                        lnmode,
                        ns,
                    ));
                }
            }

            // IR processing thread
            // In Java this spawns a Thread. In Rust we'd use tokio::spawn or std::thread::spawn.
            log::warn!("not yet implemented: IR processing thread for course result");
        }

        // Play result sound
        if let Some(ref ns) = newscore {
            let _is_clear = ns.clear != ClearType::Failed.id();
            let _loop_sound = resource
                .get_config()
                .audio
                .as_ref()
                .map(|ac| ac.is_loop_course_result_sound)
                .unwrap_or(false);
            // play(clear ? COURSE_CLEAR/RESULT_CLEAR : COURSE_FAIL/RESULT_FAIL, loop)
            log::warn!("not yet implemented: play course result sound");
        }
    }

    pub fn shutdown(&mut self) {
        // stop(getSound(COURSE_CLEAR) != null ? COURSE_CLEAR : RESULT_CLEAR);
        // stop(getSound(COURSE_FAIL) != null ? COURSE_FAIL : RESULT_FAIL);
        // stop(getSound(COURSE_CLOSE) != null ? COURSE_CLOSE : RESULT_CLOSE);
        log::warn!("not yet implemented: stop course result sounds");
    }

    pub fn render(&mut self, _resource: &PlayerResource) {
        let time = self.data.timer.get_now_time();
        self.data.timer.switch_timer(TIMER_RESULTGRAPH_BEGIN, true);
        self.data.timer.switch_timer(TIMER_RESULTGRAPH_END, true);
        self.data.timer.switch_timer(TIMER_RESULT_UPDATESCORE, true);

        // if(time > getSkin().getInput()) { timer.switchTimer(TIMER_STARTINPUT, true); }
        // Skin access requires integration
        log::warn!("not yet implemented: render with skin");
    }

    pub fn input(&mut self, main: &mut MainController, resource: &mut PlayerResource) {
        self.data.input(main);

        if !self.data.timer.is_timer_on(TIMER_FADEOUT)
            && self.data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            let mut ok = false;
            for i in 0..self.property.get_assign_length() {
                let input_processor = main.get_input_processor();
                if self.property.get_assign(i) == Some(ResultKey::ChangeGraph)
                    && input_processor.get_key_state(i)
                    && input_processor.reset_key_changed_time(i)
                {
                    self.data.gauge_type = (self.data.gauge_type - 5) % 3 + 6;
                } else if self.property.get_assign(i).is_some()
                    && input_processor.get_key_state(i)
                    && input_processor.reset_key_changed_time(i)
                {
                    ok = true;
                }
            }

            {
                let input_processor = main.get_input_processor();
                if input_processor.is_control_key_pressed(ControlKeys::Escape)
                    || input_processor.is_control_key_pressed(ControlKeys::Enter)
                {
                    ok = true;
                }
            }

            if (resource.get_score_data().is_none() || ok)
                && (self.data.state == STATE_OFFLINE || self.data.state == STATE_IR_FINISHED)
            {
                self.data.timer.switch_timer(TIMER_FADEOUT, true);
                // play close sound
            }

            let replay_index = {
                let input_processor = main.get_input_processor();
                if input_processor.is_control_key_pressed(ControlKeys::Num1) {
                    Some(0)
                } else if input_processor.is_control_key_pressed(ControlKeys::Num2) {
                    Some(1)
                } else if input_processor.is_control_key_pressed(ControlKeys::Num3) {
                    Some(2)
                } else if input_processor.is_control_key_pressed(ControlKeys::Num4) {
                    Some(3)
                } else {
                    None
                }
            };
            if let Some(idx) = replay_index {
                self.save_replay_data(idx, main, resource);
            }

            {
                let input_processor = main.get_input_processor();
                if input_processor.is_activated(KeyCommand::OpenIr) {
                    // self.execute_event(EventType::open_ir);
                    log::warn!("not yet implemented: execute open_ir event");
                }
            }
        }
    }

    pub fn update_score_database(
        &mut self,
        main: &mut MainController,
        resource: &mut PlayerResource,
    ) {
        let lnmode = resource.get_player_config().lnmode;
        let random_cfg = resource.get_player_config().random;
        let random2_cfg = resource.get_player_config().random2;
        let doubleoption_cfg = resource.get_player_config().doubleoption;
        let newscore = self.get_new_score(resource).cloned();
        if newscore.is_none() {
            return;
        }
        let mut newscore = newscore.unwrap();

        let mut dp = false;
        if let Some(models) = resource.get_course_bms_models() {
            for model in models {
                dp |= model.get_mode().map(|m| m.player()).unwrap_or(1) == 2;
            }
        }

        newscore.combo = resource.get_maxcombo();
        if newscore.notes != 0 {
            newscore.avgjudge = newscore.total_duration / newscore.notes as i64;
        }

        let mut random = 0;
        if random_cfg > 0 || (dp && (random2_cfg > 0 || doubleoption_cfg > 0)) {
            random = 2;
        }
        if random_cfg == 1 && (!dp || (random2_cfg == 1 && doubleoption_cfg == 1)) {
            random = 1;
        }

        if let Some(models) = resource.get_course_bms_models() {
            let score = main.get_play_data_accessor().read_score_data_course(
                models,
                lnmode,
                random,
                &resource.get_constraint(),
            );
            self.data.oldscore = score.unwrap_or_default();
        }

        let target_exscore = resource
            .get_target_score_data()
            .map(|s| s.get_exscore())
            .unwrap_or(0);
        let total_notes: i32 = resource
            .get_course_bms_models()
            .map(|models| models.iter().map(|m| m.get_total_notes()).sum())
            .unwrap_or(0);
        self.data.score.set_target_score(
            self.data.oldscore.get_exscore(),
            target_exscore,
            total_notes,
        );
        self.data.score.update_score(Some(&newscore));

        if let Some(models) = resource.get_course_bms_models() {
            main.get_play_data_accessor().write_score_data_course(
                &newscore,
                models,
                lnmode,
                random,
                &resource.get_constraint(),
                resource.is_update_course_score(),
            );
        }

        info!("Score database update complete");
    }

    pub fn get_judge_count(&self, judge: i32, fast: bool, resource: &PlayerResource) -> i32 {
        if let Some(score) = resource.get_course_score_data() {
            match judge {
                0 => {
                    if fast {
                        score.epg
                    } else {
                        score.lpg
                    }
                }
                1 => {
                    if fast {
                        score.egr
                    } else {
                        score.lgr
                    }
                }
                2 => {
                    if fast {
                        score.egd
                    } else {
                        score.lgd
                    }
                }
                3 => {
                    if fast {
                        score.ebd
                    } else {
                        score.lbd
                    }
                }
                4 => {
                    if fast {
                        score.epr
                    } else {
                        score.lpr
                    }
                }
                5 => {
                    if fast {
                        score.ems
                    } else {
                        score.lms
                    }
                }
                _ => 0,
            }
        } else {
            0
        }
    }

    pub fn save_replay_data(
        &mut self,
        index: usize,
        main: &mut MainController,
        resource: &mut PlayerResource,
    ) {
        if resource.get_play_mode().mode == BMSPlayerModeType::Play
            && resource.get_course_score_data().is_some()
            && self.data.save_replay[index] != ReplayStatus::Saved
            && resource.is_update_course_score()
        {
            // Extract gauge value first to avoid borrow conflict
            let gauge = resource.get_player_config().gauge;
            let rd = resource.get_course_replay_mut();
            for replay in rd.iter_mut() {
                replay.gauge = gauge;
            }
            let lnmode = resource.get_player_config().lnmode;
            let constraint = resource.get_constraint();
            if let Some(models) = resource.get_course_bms_models() {
                main.get_play_data_accessor().write_replay_data_course(
                    resource.get_course_replay(),
                    models,
                    lnmode,
                    index as i32,
                    &constraint,
                );
            }
            self.data.save_replay[index] = ReplayStatus::Saved;
            main.save_last_recording("ON_REPLAY");
        }
    }

    pub fn get_new_score<'a>(&self, resource: &'a PlayerResource) -> Option<&'a ScoreData> {
        resource.get_course_score_data()
    }

    pub fn dispose(&mut self) {
        // super.dispose() equivalent
    }
}

impl Default for CourseResult {
    fn default() -> Self {
        Self::new()
    }
}

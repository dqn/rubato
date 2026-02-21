// MusicResult.java -> music_result.rs
// Mechanical line-by-line translation.

use log::{info, warn};

use beatoraja_core::clear_type::ClearType;
use beatoraja_core::main_state::MainStateType;
use beatoraja_core::score_data::ScoreData;
use beatoraja_core::system_sound_manager::SoundType;
use beatoraja_play::groove_gauge;
use beatoraja_skin::skin_property::*;
use bms_model::bms_model::BMSModel;

use crate::abstract_result::{
    AbstractResultData, REPLAY_SIZE, ReplayAutoSaveConstraint, ReplayStatus, STATE_IR_FINISHED,
    STATE_IR_PROCESSING, STATE_OFFLINE,
};
use crate::music_result_skin::MusicResultSkin;
use crate::result_key_property::{ResultKey, ResultKeyProperty};
use crate::stubs::{
    BMSPlayerModeType, ControlKeys, EventType, FloatArray, IRConfig, IRSendStatusMain, IRStatus,
    KeyCommand, MainController, PlayerResource, RankingData, IR_SEND_ALWAYS, IR_SEND_COMPLETE_SONG,
    IR_SEND_UPDATE_SCORE, is_freq_negative, is_freq_trainer_enabled,
};

/// Music result screen
pub struct MusicResult {
    pub data: AbstractResultData,
    property: ResultKeyProperty,
}

impl MusicResult {
    pub fn new() -> Self {
        Self {
            data: AbstractResultData::new(),
            property: ResultKeyProperty::beat_7k(),
        }
    }

    pub fn create(&mut self, main: &MainController, resource: &mut PlayerResource) {
        for i in 0..REPLAY_SIZE {
            self.data.save_replay[i] = if main.get_play_data_accessor().exists_replay_data_model(
                resource.get_bms_model(),
                resource.get_player_config().lnmode,
                i as i32,
            ) {
                ReplayStatus::Exist
            } else {
                ReplayStatus::NotExist
            };
        }

        if let Some(mode) = resource.get_bms_model().get_mode() {
            self.property = ResultKeyProperty::get(mode).unwrap_or_else(ResultKeyProperty::beat_7k);
        } else {
            self.property = ResultKeyProperty::beat_7k();
        }

        self.update_score_database(main, resource);

        // Replay auto save
        if resource.get_play_mode().mode == BMSPlayerModeType::Play && !resource.is_freq_on() {
            for i in 0..REPLAY_SIZE {
                let auto_save = &resource.get_player_config().autosavereplay;
                if i < auto_save.len()
                    && let Some(score_data) = resource.get_score_data()
                    && ReplayAutoSaveConstraint::get(auto_save[i])
                        .is_qualified(&self.data.oldscore, score_data)
                {
                    self.save_replay_data(i, main, resource);
                }
            }
        }

        // Stock replay data for course mode
        if resource.get_course_bms_models().is_some() {
            let replay_clone = resource.get_replay_data().clone();
            resource.add_course_replay(&replay_clone);
            let gauge_clone: Vec<_> = resource.get_gauge().to_vec();
            resource.add_course_gauge(&gauge_clone);
        }

        self.data.gauge_type = resource.get_groove_gauge().get_type();

        // loadSkin(SkinType.RESULT);
        todo!("Phase 8+ dependency: loadSkin(SkinType.RESULT)")
    }

    pub fn prepare(&mut self, main: &mut MainController, resource: &mut PlayerResource) {
        self.data.state = STATE_OFFLINE;
        let newscore_clone = self.get_new_score(resource).cloned();

        self.data.ranking = if resource.get_ranking_data().is_some()
            && resource.get_course_bms_models().is_none()
        {
            resource.get_ranking_data().cloned()
        } else {
            Some(RankingData::new())
        };
        self.data.ranking_offset = 0;

        let ir = main.get_ir_status();
        if !ir.is_empty()
            && resource.get_play_mode().mode == BMSPlayerModeType::Play
            && !resource.is_freq_on()
        {
            self.data.state = STATE_IR_PROCESSING;

            let mut pending_ir_sends: Vec<IRSendStatusMain> = Vec::new();
            for irc in ir {
                let mut send = resource.is_update_score() && !resource.is_force_no_ir_send();
                match irc.config.get_irsend() {
                    IR_SEND_ALWAYS => {}
                    IR_SEND_COMPLETE_SONG => {
                        let gauge =
                            &resource.get_gauge()[resource.get_groove_gauge().get_type() as usize];
                        send &= gauge.get(gauge.size - 1) > 0.0;
                    }
                    IR_SEND_UPDATE_SCORE => {
                        if let Some(ref ns) = newscore_clone {
                            send &= ns.get_exscore() > self.data.oldscore.get_exscore()
                                || ns.clear > self.data.oldscore.clear
                                || ns.combo > self.data.oldscore.combo
                                || ns.minbp < self.data.oldscore.minbp;
                        }
                    }
                    _ => {}
                }

                if send && let Some(ref ns) = newscore_clone {
                    pending_ir_sends.push(IRSendStatusMain::new(
                        irc.connection.clone(),
                        resource.get_songdata(),
                        ns,
                    ));
                }
            }
            for status in pending_ir_sends {
                main.ir_send_status_mut().push(status);
            }

            // IR processing thread
            // In Java this spawns a Thread. In Rust we'd use tokio::spawn or std::thread::spawn.
            todo!("Phase 8+ dependency: IR processing thread for music result")
        }

        // Play result sound
        if let Some(ref ns) = newscore_clone {
            let cscore = resource.get_course_score_data();
            let is_clear = ns.clear != ClearType::Failed.id()
                && (cscore.is_none() || cscore.unwrap().clear != ClearType::Failed.id());
            let _loop_sound = resource
                .get_config()
                .audio
                .as_ref()
                .map(|ac| ac.is_loop_result_sound)
                .unwrap_or(false);
            // play(is_clear ? RESULT_CLEAR : RESULT_FAIL, loop_sound);
            todo!("Phase 8+ dependency: play result sound")
        }
    }

    pub fn shutdown(&mut self) {
        // stop(RESULT_CLEAR);
        // stop(RESULT_FAIL);
        // stop(RESULT_CLOSE);
        todo!("Phase 8+ dependency: stop sounds")
    }

    pub fn render(&mut self, main: &mut MainController, resource: &mut PlayerResource) {
        let time = self.data.timer.get_now_time();
        self.data.timer.switch_timer(TIMER_RESULTGRAPH_BEGIN, true);
        self.data.timer.switch_timer(TIMER_RESULTGRAPH_END, true);

        // MusicResultSkin rank time check
        // if (((MusicResultSkin) getSkin()).getRankTime() == 0) {
        //     timer.switchTimer(TIMER_RESULT_UPDATESCORE, true);
        // }

        // if (time > getSkin().getInput()) { timer.switchTimer(TIMER_STARTINPUT, true); }

        if self.data.timer.is_timer_on(TIMER_FADEOUT) {
            // if (timer.getNowTime(TIMER_FADEOUT) > getSkin().getFadeout())
            let fadeout_time = self.data.timer.get_now_time_for_id(TIMER_FADEOUT);
            // Skin access required for fadeout threshold
            // Full render logic with course mode transitions
            todo!("Phase 8+ dependency: render with skin and state transitions")
        } else {
            // if (time > getSkin().getScene()) { ... }
            todo!("Phase 8+ dependency: render scene timeout")
        }
    }

    pub fn input(&mut self, main: &mut MainController, resource: &mut PlayerResource) {
        self.data.input(main);
        let time = self.data.timer.get_now_time();

        if !self.data.timer.is_timer_on(TIMER_FADEOUT)
            && self.data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            // if (time > getSkin().getInput())
            let mut ok = false;
            let mut replay_index: Option<usize> = None;
            let mut open_ir = false;
            {
                let input_processor = main.get_input_processor();
                for i in 0..self.property.get_assign_length() {
                    if self.property.get_assign(i) == Some(ResultKey::ChangeGraph)
                        && input_processor.get_key_state(i)
                        && input_processor.reset_key_changed_time(i)
                    {
                        if self.data.gauge_type >= groove_gauge::ASSISTEASY
                            && self.data.gauge_type <= groove_gauge::HAZARD
                        {
                            self.data.gauge_type = (self.data.gauge_type + 1) % 6;
                        } else {
                            self.data.gauge_type = (self.data.gauge_type - 5) % 3 + 6;
                        }
                    } else if self.property.get_assign(i).is_some()
                        && input_processor.get_key_state(i)
                        && input_processor.reset_key_changed_time(i)
                    {
                        ok = true;
                    }
                }

                if input_processor.is_control_key_pressed(ControlKeys::Escape)
                    || input_processor.is_control_key_pressed(ControlKeys::Enter)
                {
                    ok = true;
                }

                if input_processor.is_control_key_pressed(ControlKeys::Num1) {
                    replay_index = Some(0);
                } else if input_processor.is_control_key_pressed(ControlKeys::Num2) {
                    replay_index = Some(1);
                } else if input_processor.is_control_key_pressed(ControlKeys::Num3) {
                    replay_index = Some(2);
                } else if input_processor.is_control_key_pressed(ControlKeys::Num4) {
                    replay_index = Some(3);
                }

                if input_processor.is_activated(KeyCommand::OpenIr) {
                    open_ir = true;
                }
            }

            if resource.get_score_data().is_none() || ok {
                // MusicResultSkin rank time check
                // if (((MusicResultSkin) getSkin()).getRankTime() != 0 && !timer.isTimerOn(TIMER_RESULT_UPDATESCORE))
                if self.data.state == STATE_OFFLINE
                    || self.data.state == STATE_IR_FINISHED
                    || time - self.data.timer.get_timer(TIMER_IR_CONNECT_BEGIN) >= 1000
                {
                    self.data.timer.switch_timer(TIMER_FADEOUT, true);
                    // if (getSound(RESULT_CLOSE) != null) { stop/play }
                }
            }

            if let Some(idx) = replay_index {
                self.save_replay_data(idx, main, resource);
            }

            if open_ir {
                // self.execute_event(EventType::open_ir);
                todo!("Phase 8+ dependency: execute open_ir event")
            }
        }
    }

    pub fn save_replay_data(
        &mut self,
        index: usize,
        main: &MainController,
        resource: &mut PlayerResource,
    ) {
        if resource.get_play_mode().mode == BMSPlayerModeType::Play
            && resource.get_course_bms_models().is_none()
            && resource.get_score_data().is_some()
            && self.data.save_replay[index] != ReplayStatus::Saved
            && resource.is_update_score()
        {
            let rd = resource.get_replay_data();
            main.get_play_data_accessor().write_replay_data(
                rd,
                resource.get_bms_model(),
                resource.get_player_config().lnmode,
                index as i32,
            );
            self.data.save_replay[index] = ReplayStatus::Saved;
            main.save_last_recording("ON_REPLAY");
        }
    }

    fn update_score_database(&mut self, main: &MainController, resource: &mut PlayerResource) {
        let newscore = resource.get_score_data().cloned();
        if newscore.is_none() {
            let total_notes = resource.get_bms_model().get_total_notes();
            if let Some(cscore) = resource.get_course_score_data_mut() {
                cscore.minbp += total_notes;
                cscore.clear = ClearType::Failed.id();
            }
            return;
        }
        let newscore = newscore.unwrap();

        let oldsc = main.get_play_data_accessor().read_score_data(
            resource.get_bms_model(),
            resource.get_player_config().lnmode,
        );
        self.data.oldscore = oldsc.unwrap_or_default();

        let target_exscore = resource
            .get_target_score_data()
            .map(|s| s.get_exscore())
            .unwrap_or(0);
        self.data.score.set_target_score(
            self.data.oldscore.get_exscore(),
            target_exscore,
            resource.get_bms_model().get_total_notes(),
        );
        self.data.score.update_score(Some(&newscore));

        // duration average
        self.data.avgduration = newscore.avgjudge;
        self.data.avg = newscore.avg;
        self.data.stddev = newscore.stddev;
        self.data.timing_distribution.init();

        let model = resource.get_bms_model();
        let lanes = model.get_mode().map(|m| m.key()).unwrap_or(8);
        for tl in model.get_all_time_lines() {
            for i in 0..lanes {
                let n = tl.get_note(i);
                if let Some(note) = n {
                    // Check if this is not an end LN in LN mode
                    let is_end_ln = (model.get_lnmode() == 1
                        || (model.get_lnmode() == 0
                            && model.get_lntype() == bms_model::bms_model::LNTYPE_LONGNOTE))
                        && note.is_long()
                        && note.is_end();
                    if !is_end_ln {
                        let state = note.get_state();
                        let play_time = note.get_play_time();
                        if state >= 1 {
                            self.data.timing_distribution.add(play_time);
                        }
                    }
                }
            }
        }
        self.data.timing_distribution.statistic_value_calculate();

        // Course mode score accumulation
        if resource.get_course_bms_models().is_some() {
            if newscore.clear == ClearType::Failed.id()
                && let Some(sd) = resource.get_score_data_mut()
            {
                sd.clear = ClearType::NoPlay.id();
            }
            let mut cscore = resource.get_course_score_data().cloned();
            if cscore.is_none() {
                let mut new_cscore = ScoreData {
                    minbp: 0,
                    ..Default::default()
                };
                let mut notes = 0;
                if let Some(models) = resource.get_course_bms_models() {
                    for mo in models {
                        notes += mo.get_total_notes();
                    }
                }
                new_cscore.notes = notes;
                new_cscore.device_type = newscore.device_type.clone();
                new_cscore.option = newscore.option;
                new_cscore.judge_algorithm = newscore.judge_algorithm.clone();
                new_cscore.rule = newscore.rule.clone();
                resource.set_course_score_data(new_cscore.clone());
                cscore = Some(new_cscore);
            }

            if let Some(ref mut cs) = cscore {
                cs.passnotes += newscore.passnotes;
                cs.epg += newscore.epg;
                cs.lpg += newscore.lpg;
                cs.egr += newscore.egr;
                cs.lgr += newscore.lgr;
                cs.egd += newscore.egd;
                cs.lgd += newscore.lgd;
                cs.ebd += newscore.ebd;
                cs.lbd += newscore.lbd;
                cs.epr += newscore.epr;
                cs.lpr += newscore.lpr;
                cs.ems += newscore.ems;
                cs.lms += newscore.lms;
                cs.minbp += newscore.minbp;
                cs.total_duration += newscore.total_duration;

                let gauge_type = resource.get_groove_gauge().get_type() as usize;
                let gauge = &resource.get_gauge()[gauge_type];
                if gauge.get(gauge.size - 1) > 0.0 {
                    if resource.get_assist() > 0 {
                        if resource.get_assist() == 1 && cs.clear != ClearType::AssistEasy.id() {
                            cs.clear = ClearType::LightAssistEasy.id();
                        } else {
                            cs.clear = ClearType::AssistEasy.id();
                        }
                    } else if !(cs.clear == ClearType::LightAssistEasy.id()
                        || cs.clear == ClearType::AssistEasy.id())
                        && let Some(models) = resource.get_course_bms_models()
                        && resource.get_course_index() == models.len() - 1
                    {
                        let mut course_total_notes = 0;
                        for m in models {
                            course_total_notes += m.get_total_notes();
                        }
                        if course_total_notes == resource.get_maxcombo() {
                            if cs.get_judge_count(2, true) + cs.get_judge_count(2, false) == 0 {
                                if cs.get_judge_count(1, true) + cs.get_judge_count(1, false) == 0 {
                                    cs.clear = ClearType::Max.id();
                                } else {
                                    cs.clear = ClearType::Perfect.id();
                                }
                            } else {
                                cs.clear = ClearType::FullCombo.id();
                            }
                        } else {
                            cs.clear = resource.get_groove_gauge().get_clear_type().id();
                        }
                    }
                } else {
                    cs.clear = ClearType::Failed.id();

                    let mut b = false;
                    if let Some(models) = resource.get_course_bms_models() {
                        for m in models {
                            if b {
                                cs.minbp += m.get_total_notes();
                            }
                            if std::ptr::eq(m, resource.get_bms_model()) {
                                b = true;
                            }
                        }
                    }
                }

                resource.set_course_score_data(cs.clone());
            }
        }

        if is_freq_trainer_enabled()
            && let Some(sd) = resource.get_score_data_mut()
        {
            sd.clear = ClearType::NoPlay.id();
        }

        if resource.get_play_mode().mode == BMSPlayerModeType::Play
            && !(is_freq_trainer_enabled() && is_freq_negative())
        {
            if let Some(sd) = resource.get_score_data() {
                main.get_play_data_accessor().write_score_data(
                    sd,
                    resource.get_bms_model(),
                    resource.get_player_config().lnmode,
                    resource.is_update_score(),
                );
            }
        } else {
            info!(
                "Play mode is {:?}, score not registered",
                resource.get_play_mode().mode
            );
        }
    }

    pub fn get_judge_count(&self, judge: i32, fast: bool, resource: &PlayerResource) -> i32 {
        if let Some(score) = resource.get_score_data() {
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

    pub fn dispose(&mut self) {
        // super.dispose() equivalent
    }

    pub fn get_total_notes(&self, resource: &PlayerResource) -> i32 {
        resource.get_bms_model().get_total_notes()
    }

    pub fn get_new_score<'a>(&self, resource: &'a PlayerResource) -> Option<&'a ScoreData> {
        resource.get_score_data()
    }
}

impl Default for MusicResult {
    fn default() -> Self {
        Self::new()
    }
}

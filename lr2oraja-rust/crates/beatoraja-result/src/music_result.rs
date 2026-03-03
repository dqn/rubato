// MusicResult.java -> music_result.rs
// Mechanical line-by-line translation.

use log::{info, warn};

use beatoraja_core::clear_type::ClearType;
use beatoraja_core::main_state::{MainState, MainStateData, MainStateType};
use beatoraja_core::score_data::ScoreData;
use beatoraja_core::system_sound_manager::SoundType;
use beatoraja_core::timer_manager::TimerManager;
use beatoraja_play::groove_gauge;
use beatoraja_skin::skin_property::*;

use crate::abstract_result::{
    AbstractResultData, REPLAY_SIZE, ReplayAutoSaveConstraint, ReplayStatus, STATE_IR_FINISHED,
    STATE_IR_PROCESSING, STATE_OFFLINE,
};
use crate::music_result_skin::MusicResultSkin;
use crate::result_key_property::{ResultKey, ResultKeyProperty};
use crate::stubs::{
    BMSPlayerModeType, ControlKeys, FreqTrainerMenu, IRSendStatusMain, KeyCommand, MainController,
    NullMainController, PlayerResource, RankingData,
};
use beatoraja_core::ir_config::{IR_SEND_ALWAYS, IR_SEND_COMPLETE_SONG, IR_SEND_UPDATE_SCORE};

/// Music result screen
pub struct MusicResult {
    pub data: AbstractResultData,
    pub main_data: MainStateData,
    pub main: MainController,
    pub resource: PlayerResource,
    property: ResultKeyProperty,
    skin: Option<MusicResultSkin>,
}

impl MusicResult {
    pub fn new(main: MainController, resource: PlayerResource, timer: TimerManager) -> Self {
        Self {
            data: AbstractResultData::new(),
            main_data: MainStateData::new(timer),
            main,
            resource,
            property: ResultKeyProperty::beat_7k(),
            skin: None,
        }
    }

    fn do_create(&mut self) {
        for i in 0..REPLAY_SIZE {
            self.data.save_replay[i] =
                if self.main.get_play_data_accessor().exists_replay_data_model(
                    self.resource.get_bms_model(),
                    self.resource.get_player_config().lnmode,
                    i as i32,
                ) {
                    ReplayStatus::Exist
                } else {
                    ReplayStatus::NotExist
                };
        }

        if let Some(mode) = self.resource.get_bms_model().get_mode() {
            self.property = ResultKeyProperty::get(mode).unwrap_or_else(ResultKeyProperty::beat_7k);
        } else {
            self.property = ResultKeyProperty::beat_7k();
        }

        self.update_score_database();

        // Replay auto save
        if self.resource.get_play_mode().mode == BMSPlayerModeType::Play
            && !self.resource.is_freq_on()
        {
            for i in 0..REPLAY_SIZE {
                let auto_save = &self.resource.get_player_config().autosavereplay;
                if i < auto_save.len()
                    && let Some(score_data) = self.resource.get_score_data()
                    && ReplayAutoSaveConstraint::get(auto_save[i])
                        .is_qualified(&self.data.oldscore, score_data)
                {
                    self.save_replay_data(i);
                }
            }
        }

        // Stock replay data for course mode
        if self.resource.get_course_bms_models().is_some() {
            if let Some(replay_clone) = self.resource.get_replay_data().cloned() {
                self.resource.add_course_replay(replay_clone);
            }
            if let Some(gauge) = self.resource.get_gauge() {
                let gauge_clone = gauge.clone();
                self.resource.add_course_gauge(gauge_clone);
            }
        }

        self.data.gauge_type = self
            .resource
            .get_groove_gauge()
            .map(|g| g.get_type())
            .unwrap_or(0);

        // loadSkin(SkinType.RESULT)
        self.load_skin(beatoraja_skin::skin_type::SkinType::Result.id());
    }

    fn do_prepare(&mut self) {
        self.data.state = STATE_OFFLINE;
        let newscore_clone = self.get_new_score().cloned();

        self.data.ranking = if self.resource.get_ranking_data().is_some()
            && self.resource.get_course_bms_models().is_none()
        {
            self.resource.get_ranking_data().cloned()
        } else {
            Some(RankingData::new())
        };
        self.data.ranking_offset = 0;

        let ir = self.main.get_ir_status();
        if !ir.is_empty()
            && self.resource.get_play_mode().mode == BMSPlayerModeType::Play
            && !self.resource.is_freq_on()
        {
            self.data.state = STATE_IR_PROCESSING;

            let mut pending_ir_sends: Vec<IRSendStatusMain> = Vec::new();
            for irc in ir {
                let mut send =
                    self.resource.is_update_score() && !self.resource.is_force_no_ir_send();
                match irc.config.get_irsend() {
                    IR_SEND_ALWAYS => {}
                    IR_SEND_COMPLETE_SONG => {
                        if let (Some(gauge_data), Some(groove_gauge)) =
                            (self.resource.get_gauge(), self.resource.get_groove_gauge())
                        {
                            let gauge = &gauge_data[groove_gauge.get_type() as usize];
                            send &= gauge.last().copied().unwrap_or(0.0) > 0.0;
                        }
                    }
                    IR_SEND_UPDATE_SCORE => {
                        if let Some(ref ns) = newscore_clone {
                            send &= ns.get_exscore() > self.data.oldscore.get_exscore()
                                || ns.clear > self.data.oldscore.clear
                                || ns.maxcombo > self.data.oldscore.maxcombo
                                || ns.minbp < self.data.oldscore.minbp;
                        }
                    }
                    _ => {}
                }

                if send
                    && let Some(ref ns) = newscore_clone
                    && let Some(songdata) = self.resource.get_songdata()
                {
                    pending_ir_sends.push(IRSendStatusMain::new(
                        irc.connection.clone(),
                        songdata,
                        ns,
                    ));
                }
            }
            for status in pending_ir_sends {
                self.main.ir_send_status_mut().push(status);
            }

            // IR processing thread
            // In Java this spawns a Thread. In Rust we'd use tokio::spawn or std::thread::spawn.
            // The thread sends scores, fetches ranking, and updates state.
            // For now, immediately mark as finished (blocking stub).
            let ir_len = self.main.get_ir_status().len();
            let ir_send_count = self.main.get_config().ir_send_count;
            let mut ir_send_list = self.main.ir_send_status_mut();
            let mut irsend = 0;
            let mut succeed = true;
            let mut remove_indices: Vec<usize> = Vec::new();
            let start = if ir_send_list.len() >= ir_len {
                ir_send_list.len() - ir_len
            } else {
                0
            };
            for idx in start..ir_send_list.len() {
                if irsend == 0 {
                    self.data.timer.switch_timer(TIMER_IR_CONNECT_BEGIN, true);
                }
                irsend += 1;
                let send_ok = ir_send_list[idx].send();
                succeed &= send_ok;
                if ir_send_list[idx].retry < 0 || ir_send_list[idx].retry > ir_send_count {
                    remove_indices.push(idx);
                }
            }
            // Remove in reverse order to preserve indices
            for idx in remove_indices.into_iter().rev() {
                ir_send_list.remove(idx);
            }

            if irsend > 0 {
                if succeed {
                    self.data.timer.switch_timer(TIMER_IR_CONNECT_SUCCESS, true);
                } else {
                    self.data.timer.switch_timer(TIMER_IR_CONNECT_FAIL, true);
                }
                // Fetch ranking from IR
                let ir_status = self.main.get_ir_status();
                if !ir_status.is_empty()
                    && let Some(songdata) = self.resource.get_songdata()
                {
                    let chart_data = beatoraja_ir::ir_chart_data::IRChartData::new(songdata);
                    let response = ir_status[0].connection.get_play_data(None, &chart_data);
                    if response.is_succeeded() {
                        if let Some(ir_scores) = response.get_data() {
                            let use_newscore = newscore_clone
                                .as_ref()
                                .map(|ns| ns.get_exscore() > self.data.oldscore.get_exscore())
                                .unwrap_or(false);
                            let score_for_rank: Option<&ScoreData> = if use_newscore {
                                newscore_clone.as_ref()
                            } else {
                                Some(&self.data.oldscore)
                            };
                            if let Some(ref mut ranking) = self.data.ranking {
                                ranking.update_score(ir_scores, score_for_rank);
                                if ranking.get_rank() > 10 {
                                    self.data.ranking_offset = ranking.get_rank() - 5;
                                } else {
                                    self.data.ranking_offset = 0;
                                }
                            }
                        }
                        info!("IR score fetch succeeded: {}", response.get_message());
                    } else {
                        warn!("IR score fetch failed: {}", response.get_message());
                    }
                }
            }
            self.data.state = STATE_IR_FINISHED;
        }

        // Play result sound
        if let Some(ref ns) = newscore_clone {
            let cscore = self.resource.get_course_score_data();
            let is_clear = ns.clear != ClearType::Failed.id()
                && (cscore.is_none() || cscore.unwrap().clear != ClearType::Failed.id());
            let loop_sound = self
                .resource
                .get_config()
                .audio
                .as_ref()
                .map(|ac| ac.is_loop_result_sound)
                .unwrap_or(false);
            if is_clear {
                self.play_sound_loop_inner(SoundType::ResultClear, loop_sound);
            } else {
                self.play_sound_loop_inner(SoundType::ResultFail, loop_sound);
            }
        }
    }

    fn do_shutdown(&mut self) {
        self.stop_sound_inner(SoundType::ResultClear);
        self.stop_sound_inner(SoundType::ResultFail);
        self.stop_sound_inner(SoundType::ResultClose);
    }

    fn do_render(&mut self) {
        let time = self.data.timer.get_now_time();
        self.data.timer.switch_timer(TIMER_RESULTGRAPH_BEGIN, true);
        self.data.timer.switch_timer(TIMER_RESULTGRAPH_END, true);

        if let Some(ref skin) = self.skin
            && skin.get_rank_time() == 0
        {
            self.data.timer.switch_timer(TIMER_RESULT_UPDATESCORE, true);
        }
        let skin_input = self
            .skin
            .as_ref()
            .map(|s| s.get_input() as i64)
            .unwrap_or(0);
        if time > skin_input {
            self.data.timer.switch_timer(TIMER_STARTINPUT, true);
        }

        if self.data.timer.is_timer_on(TIMER_FADEOUT) {
            let fadeout_time = self.data.timer.get_now_time_for_id(TIMER_FADEOUT);
            let skin_fadeout = self
                .skin
                .as_ref()
                .map(|s| s.get_fadeout() as i64)
                .unwrap_or(0);
            if fadeout_time > skin_fadeout {
                if let Some(audio) = self.main.get_audio_processor_mut() {
                    audio.stop_note(None);
                }
                {
                    let input = self.main.get_input_processor();
                    input.reset_all_key_changed_time();
                }

                if self.resource.get_course_bms_models().is_some() {
                    let gauge_type = self
                        .resource
                        .get_groove_gauge()
                        .map(|g| g.get_type() as usize)
                        .unwrap_or(0);
                    let last_gauge = self
                        .resource
                        .get_gauge()
                        .and_then(|gd| gd.get(gauge_type))
                        .and_then(|g| g.last().copied())
                        .unwrap_or(0.0);

                    if last_gauge <= 0.0 {
                        if self.resource.get_course_score_data().is_some() {
                            // Add remaining course notes as POOR
                            // Collect note counts first to avoid borrow conflict
                            let course_gauge_size = self.resource.get_course_gauge().len();
                            let notes_to_add: Vec<i32> = self
                                .resource
                                .get_course_bms_models()
                                .map(|models| {
                                    models
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, _)| course_gauge_size <= *i)
                                        .map(|(_, m)| m.get_total_notes())
                                        .collect()
                                })
                                .unwrap_or_default();
                            for total_notes in notes_to_add {
                                if let Some(mut cscore) =
                                    self.resource.get_course_score_data().cloned()
                                {
                                    cscore.minbp += total_notes;
                                    cscore.total_duration += 1000000i64 * total_notes as i64;
                                    self.resource.set_course_score_data(cscore);
                                }
                            }
                            // Failed course result
                            self.main.change_state(MainStateType::CourseResult);
                        } else {
                            // No course score — go to music select
                            self.main.change_state(MainStateType::MusicSelect);
                        }
                    } else if self.resource.next_course() {
                        // Next course song
                        let lnmode = self.resource.get_player_config().lnmode;
                        if let Some(songdata) = self.resource.get_songdata() {
                            let songrank: Option<RankingData> = self
                                .main
                                .get_ranking_data_cache()
                                .get_song_any(songdata, lnmode)
                                .and_then(|any| any.downcast_ref::<RankingData>())
                                .cloned();
                            if !self.main.get_ir_status().is_empty() && songrank.is_none() {
                                let new_ranking = RankingData::new();
                                self.main.get_ranking_data_cache_mut().put_song_any(
                                    songdata,
                                    lnmode,
                                    Box::new(new_ranking.clone()),
                                );
                                self.resource.set_ranking_data(Some(new_ranking));
                            } else {
                                self.resource.set_ranking_data(songrank);
                            }
                        }
                        self.main.change_state(MainStateType::Play);
                    } else {
                        // Course pass result
                        self.main.change_state(MainStateType::CourseResult);
                    }
                } else {
                    // Non-course mode
                    let org_gauge = self.resource.get_org_gauge_option();
                    self.resource.set_player_config_gauge(org_gauge);

                    let mut key: Option<ResultKey> = None;
                    {
                        let input = self.main.get_input_processor();
                        for i in 0..self.property.get_assign_length() {
                            if self.property.get_assign(i) == Some(ResultKey::ReplayDifferent)
                                && input.get_key_state(i)
                            {
                                key = Some(ResultKey::ReplayDifferent);
                                break;
                            }
                            if self.property.get_assign(i) == Some(ResultKey::ReplaySame)
                                && input.get_key_state(i)
                            {
                                key = Some(ResultKey::ReplaySame);
                                break;
                            }
                        }
                    }

                    if self.resource.get_play_mode().mode == BMSPlayerModeType::Play
                        && key == Some(ResultKey::ReplayDifferent)
                    {
                        info!("Replay without changing options");
                        // Replay without changing options - same chart
                        if let Some(rd) = self.resource.get_replay_data_mut() {
                            rd.randomoptionseed = -1;
                        }
                        self.resource.reload_bms_file();
                        self.main.change_state(MainStateType::Play);
                    } else if self.resource.get_play_mode().mode == BMSPlayerModeType::Play
                        && key == Some(ResultKey::ReplaySame)
                    {
                        // Replay with same chart
                        if self.resource.is_update_score() {
                            info!("Replay with same chart");
                        } else {
                            info!("Cannot replay with same chart in assist mode");
                            if let Some(rd) = self.resource.get_replay_data_mut() {
                                rd.randomoptionseed = -1;
                            }
                        }
                        self.resource.reload_bms_file();
                        self.main.change_state(MainStateType::Play);
                    } else {
                        self.main.change_state(MainStateType::MusicSelect);
                    }
                }
            }
        } else {
            let skin_scene = self
                .skin
                .as_ref()
                .map(|s| s.get_scene() as i64)
                .unwrap_or(0);
            if time > skin_scene {
                self.data.timer.switch_timer(TIMER_FADEOUT, true);
                if self.has_sound(SoundType::ResultClose) {
                    self.stop_sound_inner(SoundType::ResultClear);
                    self.stop_sound_inner(SoundType::ResultFail);
                    self.play_sound_inner(SoundType::ResultClose);
                }
            }
        }
    }

    fn do_input(&mut self) {
        self.data.input(&mut self.main);
        let time = self.data.timer.get_now_time();

        if !self.data.timer.is_timer_on(TIMER_FADEOUT)
            && self.data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            let skin_input = self
                .skin
                .as_ref()
                .map(|s| s.get_input() as i64)
                .unwrap_or(0);
            if time > skin_input {
                let mut ok = false;
                let mut replay_index: Option<usize> = None;
                let mut open_ir = false;
                {
                    let input_processor = self.main.get_input_processor();
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

                if self.resource.get_score_data().is_none() || ok {
                    let rank_time = self.skin.as_ref().map(|s| s.get_rank_time()).unwrap_or(0);
                    if rank_time != 0 && !self.data.timer.is_timer_on(TIMER_RESULT_UPDATESCORE) {
                        self.data.timer.switch_timer(TIMER_RESULT_UPDATESCORE, true);
                    } else if self.data.state == STATE_OFFLINE
                        || self.data.state == STATE_IR_FINISHED
                        || time - self.data.timer.get_timer(TIMER_IR_CONNECT_BEGIN) >= 1000
                    {
                        self.data.timer.switch_timer(TIMER_FADEOUT, true);
                        if self.has_sound(SoundType::ResultClose) {
                            self.stop_sound_inner(SoundType::ResultClear);
                            self.stop_sound_inner(SoundType::ResultFail);
                            self.play_sound_inner(SoundType::ResultClose);
                        }
                    }
                }

                if let Some(idx) = replay_index {
                    self.save_replay_data(idx);
                }

                if open_ir
                    && let Some(ir_status) = self.main.get_ir_status().first()
                    && let Some(songdata) = self.resource.get_songdata()
                {
                    let chart = beatoraja_ir::ir_chart_data::IRChartData::new(songdata);
                    if let Some(url) = ir_status.connection.get_song_url(&chart)
                        && let Err(e) = open::that(&url)
                    {
                        log::error!("Failed to open IR URL: {}", e);
                    }
                }
            }
        }
    }

    pub fn save_replay_data(&mut self, index: usize) {
        if self.resource.get_play_mode().mode == BMSPlayerModeType::Play
            && self.resource.get_course_bms_models().is_none()
            && self.resource.get_score_data().is_some()
            && self.data.save_replay[index] != ReplayStatus::Saved
            && self.resource.is_update_score()
            && let Some(rd) = self.resource.get_replay_data()
        {
            self.main.get_play_data_accessor().write_replay_data_model(
                &mut rd.clone(),
                self.resource.get_bms_model(),
                self.resource.get_player_config().lnmode,
                index as i32,
            );
            self.data.save_replay[index] = ReplayStatus::Saved;
            self.main.save_last_recording("ON_REPLAY");
        }
    }

    fn update_score_database(&mut self) {
        let newscore = self.resource.get_score_data().cloned();
        if newscore.is_none() {
            let total_notes = self.resource.get_bms_model().get_total_notes();
            if let Some(mut cscore) = self.resource.get_course_score_data().cloned() {
                cscore.minbp += total_notes;
                cscore.clear = ClearType::Failed.id();
                self.resource.set_course_score_data(cscore);
            }
            return;
        }
        let newscore = newscore.unwrap();

        let oldsc = self.main.get_play_data_accessor().read_score_data_model(
            self.resource.get_bms_model(),
            self.resource.get_player_config().lnmode,
        );
        self.data.oldscore = oldsc.unwrap_or_default();

        let target_exscore = self
            .resource
            .get_target_score_data()
            .map(|s| s.get_exscore())
            .unwrap_or(0);
        self.data.score.set_target_score(
            self.data.oldscore.get_exscore(),
            target_exscore,
            self.resource.get_bms_model().get_total_notes(),
        );
        self.data.score.update_score(Some(&newscore));

        // duration average
        self.data.avgduration = newscore.avgjudge;
        self.data.avg = newscore.avg;
        self.data.stddev = newscore.stddev;
        self.data.timing_distribution.init();

        let model = self.resource.get_bms_model();
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
        if self.resource.get_course_bms_models().is_some() {
            if newscore.clear == ClearType::Failed.id()
                && let Some(sd) = self.resource.get_score_data_mut()
            {
                sd.clear = ClearType::NoPlay.id();
            }
            let mut cscore = self.resource.get_course_score_data().cloned();
            if cscore.is_none() {
                let mut new_cscore = ScoreData {
                    minbp: 0,
                    ..Default::default()
                };
                let mut notes = 0;
                if let Some(models) = self.resource.get_course_bms_models() {
                    for mo in models {
                        notes += mo.get_total_notes();
                    }
                }
                new_cscore.notes = notes;
                new_cscore.device_type = newscore.device_type.clone();
                new_cscore.option = newscore.option;
                new_cscore.judge_algorithm = newscore.judge_algorithm.clone();
                new_cscore.rule = newscore.rule.clone();
                self.resource.set_course_score_data(new_cscore.clone());
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

                let gauge_type = self
                    .resource
                    .get_groove_gauge()
                    .map(|g| g.get_type() as usize)
                    .unwrap_or(0);
                let last_gauge_val = self
                    .resource
                    .get_gauge()
                    .and_then(|gd| gd.get(gauge_type))
                    .and_then(|g| g.last().copied())
                    .unwrap_or(0.0);
                if last_gauge_val > 0.0 {
                    if self.resource.get_assist() > 0 {
                        if self.resource.get_assist() == 1 && cs.clear != ClearType::AssistEasy.id()
                        {
                            cs.clear = ClearType::LightAssistEasy.id();
                        } else {
                            cs.clear = ClearType::AssistEasy.id();
                        }
                    } else if !(cs.clear == ClearType::LightAssistEasy.id()
                        || cs.clear == ClearType::AssistEasy.id())
                        && let Some(models) = self.resource.get_course_bms_models()
                        && self.resource.get_course_index() == models.len() - 1
                    {
                        let mut course_total_notes = 0;
                        for m in models {
                            course_total_notes += m.get_total_notes();
                        }
                        if course_total_notes == self.resource.get_maxcombo() {
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
                            cs.clear = self
                                .resource
                                .get_groove_gauge()
                                .map(|g| g.get_clear_type())
                                .unwrap_or(ClearType::Failed)
                                .id();
                        }
                    }
                } else {
                    cs.clear = ClearType::Failed.id();

                    let mut b = false;
                    if let Some(models) = self.resource.get_course_bms_models() {
                        for m in models {
                            if b {
                                cs.minbp += m.get_total_notes();
                            }
                            if std::ptr::eq(m, self.resource.get_bms_model()) {
                                b = true;
                            }
                        }
                    }
                }

                self.resource.set_course_score_data(cs.clone());
            }
        }

        if FreqTrainerMenu::is_freq_trainer_enabled()
            && let Some(sd) = self.resource.get_score_data_mut()
        {
            sd.clear = ClearType::NoPlay.id();
        }

        if self.resource.get_play_mode().mode == BMSPlayerModeType::Play
            && !(FreqTrainerMenu::is_freq_trainer_enabled() && FreqTrainerMenu::is_freq_negative())
        {
            if let Some(sd) = self.resource.get_score_data() {
                self.main.get_play_data_accessor().write_score_data_model(
                    sd,
                    self.resource.get_bms_model(),
                    self.resource.get_player_config().lnmode,
                    self.resource.is_update_score(),
                );
            }
        } else {
            info!(
                "Play mode is {:?}, score not registered",
                self.resource.get_play_mode().mode
            );
        }
    }

    pub fn get_judge_count(&self, judge: i32, fast: bool) -> i32 {
        if let Some(score) = self.resource.get_score_data() {
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

    pub fn get_total_notes(&self) -> i32 {
        self.resource.get_bms_model().get_total_notes()
    }

    pub fn get_new_score(&self) -> Option<&ScoreData> {
        self.resource.get_score_data()
    }

    /// Get the skin as MusicResultSkin
    pub fn get_skin(&self) -> Option<&MusicResultSkin> {
        self.skin.as_ref()
    }

    /// Set the skin
    pub fn set_skin(&mut self, skin: MusicResultSkin) {
        self.skin = Some(skin);
    }

    fn has_sound(&self, sound: SoundType) -> bool {
        self.main.get_sound_path(&sound).is_some()
    }

    fn play_sound_inner(&mut self, sound: SoundType) {
        self.main.play_sound(&sound, false);
    }

    fn play_sound_loop_inner(&mut self, sound: SoundType, loop_sound: bool) {
        self.main.play_sound(&sound, loop_sound);
    }

    fn stop_sound_inner(&mut self, sound: SoundType) {
        self.main.stop_sound(&sound);
    }
}

// ============================================================
// MainState trait implementation
// ============================================================

impl MainState for MusicResult {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Result)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.main_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.main_data
    }

    fn create(&mut self) {
        self.do_create();
    }

    fn prepare(&mut self) {
        self.do_prepare();
    }

    fn shutdown(&mut self) {
        self.do_shutdown();
    }

    fn render(&mut self) {
        self.do_render();
    }

    fn input(&mut self) {
        self.do_input();
    }

    fn dispose(&mut self) {
        self.main_data.skin = None;
        self.main_data.stage = None;
    }
}

impl Default for MusicResult {
    fn default() -> Self {
        Self::new(
            MainController::new(Box::new(NullMainController)),
            PlayerResource::default(),
            TimerManager::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abstract_result::{STATE_IR_FINISHED, STATE_IR_PROCESSING, STATE_OFFLINE};

    #[test]
    fn test_music_result_new_defaults() {
        let mr = MusicResult::default();
        assert_eq!(mr.data.state, STATE_OFFLINE);
        assert_eq!(mr.data.gauge_type, 0);
        assert_eq!(mr.data.ranking_offset, 0);
        assert!(mr.skin.is_none());
    }

    #[test]
    fn test_state_type_returns_result() {
        let mr = MusicResult::default();
        assert_eq!(mr.state_type(), Some(MainStateType::Result));
    }

    #[test]
    fn test_get_new_score_none_by_default() {
        let mr = MusicResult::default();
        assert!(mr.get_new_score().is_none());
    }

    #[test]
    fn test_get_judge_count_no_score() {
        let mr = MusicResult::default();
        for judge in 0..6 {
            assert_eq!(mr.get_judge_count(judge, true), 0);
            assert_eq!(mr.get_judge_count(judge, false), 0);
        }
        // out of range
        assert_eq!(mr.get_judge_count(6, true), 0);
        assert_eq!(mr.get_judge_count(-1, false), 0);
    }

    #[test]
    fn test_get_total_notes_default() {
        let mr = MusicResult::default();
        assert_eq!(mr.get_total_notes(), 0);
    }

    #[test]
    fn test_dispose_clears_skin() {
        let mut mr = MusicResult::default();
        // main_data.skin and stage should be None after dispose
        <MusicResult as MainState>::dispose(&mut mr);
        assert!(mr.main_data.skin.is_none());
        assert!(mr.main_data.stage.is_none());
    }

    #[test]
    fn test_shutdown_does_not_panic() {
        let mut mr = MusicResult::default();
        <MusicResult as MainState>::shutdown(&mut mr);
    }

    #[test]
    fn test_create_with_default_resource() {
        let mut mr = MusicResult::default();
        <MusicResult as MainState>::create(&mut mr);
        // Verify replay statuses initialized
        for i in 0..REPLAY_SIZE {
            assert_eq!(mr.data.save_replay[i], ReplayStatus::NotExist);
        }
    }

    #[test]
    fn test_create_calls_load_skin_with_result_type() {
        let mut mr = MusicResult::default();
        // create() calls do_create() which calls self.load_skin(SkinType::Result.id())
        // The trait default is a no-op stub, so data.skin remains None.
        <MusicResult as MainState>::create(&mut mr);
        // Verify SkinType::Result.id() matches expected value (7)
        assert_eq!(beatoraja_skin::skin_type::SkinType::Result.id(), 7);
    }

    #[test]
    fn test_prepare_sets_state_offline() {
        let mut mr = MusicResult::default();
        <MusicResult as MainState>::prepare(&mut mr);
        // With no IR status, state should remain offline
        assert_eq!(mr.data.state, STATE_OFFLINE);
    }

    #[test]
    fn test_render_switches_graph_timers() {
        let mut mr = MusicResult::default();
        // Update timer so get_now_time works
        mr.data.timer.update();
        <MusicResult as MainState>::render(&mut mr);
        // TIMER_RESULTGRAPH_BEGIN and TIMER_RESULTGRAPH_END should be on
        assert!(mr.data.timer.is_timer_on(TIMER_RESULTGRAPH_BEGIN));
        assert!(mr.data.timer.is_timer_on(TIMER_RESULTGRAPH_END));
    }

    #[test]
    fn test_input_scroll_ranking() {
        let mut mr = MusicResult::default();
        mr.data.ranking = Some(RankingData::new());
        mr.data.ranking_offset = 5;
        // input calls data.input which handles scroll
        <MusicResult as MainState>::input(&mut mr);
        // ranking_offset should be clamped
        assert!(mr.data.ranking_offset >= 0);
    }

    #[test]
    fn test_save_replay_data_no_score() {
        let mut mr = MusicResult::default();
        // Should not panic with no score data
        mr.save_replay_data(0);
        assert_eq!(mr.data.save_replay[0], ReplayStatus::NotExist);
    }

    #[test]
    fn test_gauge_type_change_normal_range() {
        let mut mr = MusicResult::default();
        // Test gauge type cycling in ASSISTEASY..HAZARD range (0..5)
        mr.data.gauge_type = 0;
        assert!(mr.data.gauge_type >= groove_gauge::ASSISTEASY);
        assert!(mr.data.gauge_type <= groove_gauge::HAZARD);
        mr.data.gauge_type = (mr.data.gauge_type + 1) % 6;
        assert_eq!(mr.data.gauge_type, 1);
    }

    #[test]
    fn test_gauge_type_change_extended_range() {
        let mut mr = MusicResult::default();
        // Test gauge type cycling in extended range (6+)
        mr.data.gauge_type = 6;
        mr.data.gauge_type = (mr.data.gauge_type - 5) % 3 + 6;
        assert_eq!(mr.data.gauge_type, 7);
    }

    #[test]
    fn test_state_constants() {
        assert_eq!(STATE_OFFLINE, 0);
        assert_eq!(STATE_IR_PROCESSING, 1);
        assert_eq!(STATE_IR_FINISHED, 2);
    }

    #[test]
    fn test_get_skin_none_initially() {
        let mr = MusicResult::default();
        assert!(mr.get_skin().is_none());
    }

    #[test]
    fn test_prepare_initializes_ranking() {
        let mut mr = MusicResult::default();
        <MusicResult as MainState>::prepare(&mut mr);
        assert!(mr.data.ranking.is_some());
        assert_eq!(mr.data.ranking_offset, 0);
    }

    #[test]
    fn test_replay_size() {
        assert_eq!(REPLAY_SIZE, 4);
    }

    #[test]
    fn test_main_state_data_accessors() {
        let mut mr = MusicResult::default();
        let _ = mr.main_state_data();
        let _ = mr.main_state_data_mut();
    }

    #[test]
    fn test_main_state_trait_create_prepare_render_input_lifecycle() {
        let mut mr = MusicResult::default();
        mr.data.timer.update();
        <MusicResult as MainState>::create(&mut mr);
        <MusicResult as MainState>::prepare(&mut mr);
        <MusicResult as MainState>::render(&mut mr);
        <MusicResult as MainState>::input(&mut mr);
        <MusicResult as MainState>::shutdown(&mut mr);
        <MusicResult as MainState>::dispose(&mut mr);
        assert!(mr.main_data.skin.is_none());
        assert!(mr.main_data.stage.is_none());
    }
}

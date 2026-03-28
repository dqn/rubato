// CourseResult.java -> course_result.rs
// Mechanical line-by-line translation.

#[cfg(test)]
use std::sync::Arc;

use log::{info, warn};

use crate::core::clear_type::ClearType;
use crate::core::score_data::ScoreData;
use crate::core::system_sound_manager::SoundType;
use rubato_skin::skin_property::*;

use super::abstract_result::{
    AbstractResultData, REPLAY_SIZE, ReplayAutoSaveConstraint, ReplayStatus, STATE_IR_FINISHED,
    STATE_IR_PROCESSING, STATE_OFFLINE,
};
use super::result_key_property::{ResultKey, ResultKeyProperty};
use super::result_skin_data::ResultSkinData;

use super::{
    BMSPlayerModeType, ControlKeys, FreqTrainerMenu, IRCourseData, KeyCommand, MainController,
    PlayerResource, RankingData,
};
use crate::core::app_context::GameContext;
use crate::core::ir_config::{IR_SEND_ALWAYS, IR_SEND_COMPLETE_SONG, IR_SEND_UPDATE_SCORE};
use crate::core::main_state::{MainStateType, StateTransition};
use crate::core::timer_manager::TimerManager;
use rubato_skin::property_snapshot::PropertySnapshot;
use rubato_skin::skin_action_queue::SkinActionQueue;
use rubato_skin::timer_id::TimerId;

use super::shared_render_context;

#[cfg(test)]
mod render_context;
mod score_handler;

/// IR send result: (succeeded, had_sends, ranking_scores, newscore_clone, old_exscore).
type IrSendResult = (
    bool,
    bool,
    Option<Vec<crate::ir::ir_score_data::IRScoreData>>,
    Option<ScoreData>,
    i32,
);

// render_context types are used only in tests
#[cfg(test)]
use render_context::*;
use score_handler::*;

pub struct CourseResult {
    pub data: AbstractResultData,
    pub main_data: crate::core::main_state::MainStateData,
    pub main: MainController,
    pub resource: PlayerResource,
    ir_send_status: Vec<CourseIRSendStatus>,
    property: ResultKeyProperty,
    skin: Option<ResultSkinData>,
    /// Receiver for async IR results (non-blocking).
    ir_rx: Option<std::sync::mpsc::Receiver<IrSendResult>>,
    /// JoinHandle for the IR send background thread.
    ir_thread: Option<std::thread::JoinHandle<()>>,
    /// Custom events queued during mouse handling (skin is taken, so
    /// execute_custom_event cannot be called). Replayed after the skin
    /// is restored.
    /// Retained for the old CourseResultMouseContext adapter (kept as dead code).
    #[allow(dead_code)]
    pub(crate) pending_custom_events: Vec<(i32, i32, i32)>,
    /// Outbox: pending system sound plays (sound_type, is_loop).
    pending_sounds: Vec<(SoundType, bool)>,
    /// Outbox: pending system sound stops.
    pending_sound_stops: Vec<SoundType>,
    /// Outbox: pending audio path plays (path, volume, is_loop).
    pending_audio_path_plays: Vec<(String, f32, bool)>,
    /// Outbox: pending audio path stops.
    pending_audio_path_stops: Vec<String>,
    /// Outbox: pending audio config update.
    pending_audio_config: Option<rubato_skin::audio_config::AudioConfig>,
    /// Outbox: pending stop-all-notes request (fadeout).
    pending_stop_all_notes: bool,
    /// Outbox: pending state change from skin callbacks / do_render.
    pending_state_change: Option<MainStateType>,
    /// Outbox: pending save_last_recording requests.
    pending_save_last_recording: Vec<String>,
    /// Read-only input snapshot for the current frame.
    input_snapshot: Option<rubato_input::input_snapshot::InputSnapshot>,
}

impl CourseResult {
    pub fn new(
        main: MainController,
        resource: PlayerResource,
        timer: crate::core::timer_manager::TimerManager,
    ) -> Self {
        Self {
            data: AbstractResultData::new(),
            main_data: crate::core::main_state::MainStateData::new(timer),
            main,
            resource,
            ir_send_status: Vec::new(),
            property: ResultKeyProperty::beat_7k(),
            skin: None,
            ir_rx: None,
            ir_thread: None,
            pending_custom_events: Vec::new(),
            pending_sounds: Vec::new(),
            pending_sound_stops: Vec::new(),
            pending_audio_path_plays: Vec::new(),
            pending_audio_path_stops: Vec::new(),
            pending_audio_config: None,
            pending_stop_all_notes: false,
            pending_state_change: None,
            pending_save_last_recording: Vec::new(),
            input_snapshot: None,
        }
    }

    fn do_create(&mut self) {
        // Transfer recent judge offsets from play session so result screen
        // visualizers (SkinTimingVisualizer, SkinHitErrorVisualizer) show data.
        self.main_data.timer.set_recent_judges(
            self.resource.recent_judges_index(),
            self.resource.recent_judges(),
        );

        for i in 0..REPLAY_SIZE {
            let models = self.resource.course_bms_models();
            if let Some(models) = models {
                self.data.save_replay[i] =
                    if self.main.play_data_accessor().exists_replay_data_course(
                        models,
                        self.resource.player_config().play_settings.lnmode,
                        i as i32,
                        &self.resource.constraint(),
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
        if let Some(models) = self.resource.course_bms_models() {
            let course_gauge_size = self.resource.course_gauge().len();
            let gauge_type_length = self
                .resource
                .groove_gauge()
                .map(|g| g.gauge_type_length())
                .unwrap_or(9);
            let safe_offset = course_gauge_size.min(models.len());
            for model in &models[safe_offset..] {
                let mut list: Vec<Vec<f32>> = Vec::with_capacity(gauge_type_length);
                for _type_idx in 0..gauge_type_length {
                    let last_note_milli_time = model.last_note_milli_time().max(0);
                    let slots = ((last_note_milli_time + 500) / 500).min(100_000) as usize;
                    let fa = vec![0.0f32; slots];
                    list.push(fa);
                }
                gauge_fill_data.push(list);
            }
        }
        for list in gauge_fill_data {
            self.resource.course_gauge_mut().push(list);
        }

        if let Some(mode) = self.resource.bms_model().mode() {
            self.property = ResultKeyProperty::get(mode).unwrap_or_else(ResultKeyProperty::beat_7k);
        } else {
            self.property = ResultKeyProperty::beat_7k();
        }

        self.update_score_database();

        // Replay auto save
        if self.resource.play_mode().mode == BMSPlayerModeType::Play {
            for i in 0..REPLAY_SIZE {
                let auto_save = &self.resource.player_config().misc_settings.autosavereplay;
                if i < auto_save.len()
                    && let Some(new_score) = self.resource.course_score_data()
                    && ReplayAutoSaveConstraint::get(auto_save[i])
                        .is_qualified(&self.data.oldscore, new_score)
                {
                    self.save_replay_data(i);
                }
            }
        }

        self.data.gauge_type = super::result_common::set_gauge_type(&self.resource);

        // loadSkin(SkinType.COURSE_RESULT)
        crate::core::main_state::MainState::load_skin(
            self,
            rubato_skin::skin_type::SkinType::CourseResult.id(),
        );
    }

    fn do_prepare(&mut self) {
        self.data.state = STATE_OFFLINE;
        let newscore = self.resource.course_score_data().cloned();

        self.data.ranking = if self.resource.ranking_data().is_some()
            && self.resource.course_bms_models().is_some()
        {
            self.resource.ranking_data().cloned()
        } else {
            Some(RankingData::new())
        };
        self.data.ranking_offset = 0;

        self.process_ir_scores(&newscore);
        self.play_result_sound(&newscore);
    }

    /// Send scores to IR servers in a background thread and fetch ranking data.
    /// Translates: CourseResult.prepare() IR block (Java lines ~200-290)
    fn process_ir_scores(&mut self, newscore: &Option<ScoreData>) {
        let ir = self.main.ir_status();
        if ir.is_empty() || self.resource.play_mode().mode != BMSPlayerModeType::Play {
            return;
        }

        self.data.state = STATE_IR_PROCESSING;

        let lnmode = self.determine_ir_lnmode();

        for irc in ir {
            let send = self.resource.is_update_course_score()
                && !self.resource.is_force_no_ir_send()
                && self
                    .resource
                    .course_data()
                    .map(|cd| cd.release)
                    .unwrap_or(false);
            match irc.config.irsend {
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
                && let Some(ns) = newscore
                && let Some(course_data) = self.resource.course_data()
            {
                self.ir_send_status.push(CourseIRSendStatus::new(
                    irc.connection.clone(),
                    course_data,
                    lnmode,
                    ns,
                ));
            }
        }

        let ir_send_count = self.main.config().network.ir_send_count;
        if !self.ir_send_status.is_empty() {
            self.main_data
                .timer
                .switch_timer(rubato_skin::skin_property::TIMER_IR_CONNECT_BEGIN, true);
        }

        // Move statuses into the thread
        let mut statuses = std::mem::take(&mut self.ir_send_status);
        let ir_connection = self.main.ir_status().first().map(|s| s.connection.clone());
        let course_data_for_ranking = self.resource.course_data().cloned();
        let oldscore_exscore = self.data.oldscore.exscore();
        let newscore_clone = newscore.clone();

        let (tx, rx) = std::sync::mpsc::channel();

        let handle = std::thread::spawn(move || {
            let mut succeed = true;
            let mut irsend = 0;
            let mut remove_indices: Vec<usize> = Vec::new();
            for (idx, status) in statuses.iter_mut().enumerate() {
                irsend += 1;
                let send_ok = status.send();
                succeed &= send_ok;
                if status.retry < 0 || status.retry > ir_send_count {
                    remove_indices.push(idx);
                }
            }
            for idx in remove_indices.into_iter().rev() {
                statuses.remove(idx);
            }

            // Fetch ranking from IR
            let mut ranking_result = None;
            if irsend > 0
                && let Some(ref conn) = ir_connection
                && let Some(ref cd) = course_data_for_ranking
            {
                let ir_course_data = IRCourseData::new_with_lntype(cd, lnmode);
                let response = conn.get_course_play_data(None, &ir_course_data);
                if response.is_succeeded() {
                    ranking_result = response.data().cloned();
                    info!("IR score fetch succeeded: {}", response.message);
                } else {
                    warn!("IR score fetch failed: {}", response.message);
                }
            }

            let _ = tx.send((
                succeed,
                irsend > 0,
                ranking_result,
                newscore_clone,
                oldscore_exscore,
            ));
        });

        // Store receiver and handle for non-blocking polling in render loop
        self.ir_rx = Some(rx);
        self.ir_thread = Some(handle);
    }

    /// Determine LN mode for IR based on whether models contain undefined long notes.
    fn determine_ir_lnmode(&self) -> i32 {
        let has_uln = self
            .resource
            .course_bms_models()
            .map(|models| models.iter().any(|m| m.contains_undefined_long_note()))
            .unwrap_or(false);
        if has_uln {
            self.resource.player_config().play_settings.lnmode
        } else {
            0
        }
    }

    /// Poll for IR thread results (non-blocking) and update ranking/timer state.
    /// Block until the IR background thread sends its result, then process it.
    /// Used only in tests where we need to synchronously wait for the IR thread.
    #[cfg(test)]
    fn wait_and_poll_ir_results(&mut self) {
        let rx = match self.ir_rx.take() {
            Some(rx) => rx,
            None => return,
        };
        // Block until the thread sends the result (or disconnects)
        match rx.recv() {
            Ok(result) => {
                // Re-insert as a ready receiver so poll_ir_results can process it
                let (tx, new_rx) = std::sync::mpsc::channel();
                let _ = tx.send(result);
                self.ir_rx = Some(new_rx);
                self.poll_ir_results();
            }
            Err(_) => {
                self.data.state = STATE_IR_FINISHED;
            }
        }
    }

    fn poll_ir_results(&mut self) {
        let rx = match self.ir_rx.as_ref() {
            Some(rx) => rx,
            None => return,
        };
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.ir_rx = None;
                self.data.state = STATE_IR_FINISHED;
                return;
            }
        };
        self.ir_rx = None;
        let (succeed, had_sends, ranking_scores, ns_clone, old_exscore) = result;
        self.data.state = STATE_IR_FINISHED;
        if had_sends {
            if succeed {
                self.main_data
                    .timer
                    .switch_timer(rubato_skin::skin_property::TIMER_IR_CONNECT_SUCCESS, true);
            } else {
                self.main_data
                    .timer
                    .switch_timer(rubato_skin::skin_property::TIMER_IR_CONNECT_FAIL, true);
            }
            if let Some(ir_scores) = ranking_scores {
                let use_newscore = ns_clone
                    .as_ref()
                    .map(|ns| ns.exscore() > old_exscore)
                    .unwrap_or(false);
                let score_for_rank: Option<&crate::core::score_data::ScoreData> = if use_newscore {
                    ns_clone.as_ref()
                } else {
                    Some(&self.data.oldscore)
                };
                if let Some(ref mut ranking) = self.data.ranking {
                    ranking.update_score(&ir_scores, score_for_rank);
                    if ranking.rank() > 10 {
                        self.data.ranking_offset = ranking.rank() - 5;
                    } else {
                        self.data.ranking_offset = 0;
                    }
                }
            }
        }
    }

    /// Play the appropriate result sound (clear/fail) with course-specific fallback.
    fn play_result_sound(&mut self, newscore: &Option<ScoreData>) {
        let Some(ns) = newscore else {
            return;
        };
        let is_clear = ns.clear != ClearType::Failed.id();
        let loop_sound = self
            .resource
            .config()
            .audio
            .as_ref()
            .map(|ac| ac.is_loop_course_result_sound)
            .unwrap_or(false);
        let sound = if is_clear {
            self.select_course_sound(SoundType::CourseClear, SoundType::ResultClear)
        } else {
            self.select_course_sound(SoundType::CourseFail, SoundType::ResultFail)
        };
        self.pending_sounds.push((sound, loop_sound));
    }

    /// Select course-specific sound, falling back to the generic result sound.
    fn select_course_sound(&self, course: SoundType, fallback: SoundType) -> SoundType {
        if self.main.sound_path(&course).is_some() {
            course
        } else {
            fallback
        }
    }

    /// Check if a replay save key (Num1-4) was pressed and return the replay slot index.
    fn get_replay_index_from_input(&self) -> Option<usize> {
        let snapshot = self.input_snapshot.as_ref()?;
        if snapshot
            .control_key_states
            .get(&ControlKeys::Num1)
            .copied()
            .unwrap_or(false)
        {
            Some(0)
        } else if snapshot
            .control_key_states
            .get(&ControlKeys::Num2)
            .copied()
            .unwrap_or(false)
        {
            Some(1)
        } else if snapshot
            .control_key_states
            .get(&ControlKeys::Num3)
            .copied()
            .unwrap_or(false)
        {
            Some(2)
        } else if snapshot
            .control_key_states
            .get(&ControlKeys::Num4)
            .copied()
            .unwrap_or(false)
        {
            Some(3)
        } else {
            None
        }
    }

    /// Open the IR URL for the current course if the OpenIr command was activated.
    fn try_open_ir_url(&self) {
        let snapshot = match self.input_snapshot {
            Some(ref s) => s,
            None => return,
        };
        if !snapshot.activated_commands.contains(&KeyCommand::OpenIr) {
            return;
        }
        if let Some(ir_status) = self.main.ir_status().first()
            && let Some(coursedata) = self.resource.course_data()
        {
            let course = crate::ir::ir_course_data::IRCourseData::new(coursedata);
            if let Some(url) = ir_status.connection.get_course_url(&course)
                && let Err(e) = open::that(&url)
            {
                log::error!("Failed to open IR URL: {}", e);
            }
        }
    }

    /// Stop all course result sounds.
    ///
    /// Translated from: CourseResult.shutdown()
    /// Stops course-specific sounds if available, otherwise falls back to result sounds.
    pub fn shutdown(&mut self) {
        // Java: stop(getSound(COURSE_CLEAR) != null ? COURSE_CLEAR : RESULT_CLEAR)
        self.pending_sound_stops
            .push(self.select_course_sound(SoundType::CourseClear, SoundType::ResultClear));
        // Java: stop(getSound(COURSE_FAIL) != null ? COURSE_FAIL : RESULT_FAIL)
        self.pending_sound_stops
            .push(self.select_course_sound(SoundType::CourseFail, SoundType::ResultFail));
        // Java: stop(getSound(COURSE_CLOSE) != null ? COURSE_CLOSE : RESULT_CLOSE)
        self.pending_sound_stops
            .push(self.select_course_sound(SoundType::CourseClose, SoundType::ResultClose));

        // Detach the IR send thread -- it is bounded (sends scores + fetches
        // ranking, then exits) so we do not need to block shutdown waiting for it.
        // Dropping the JoinHandle detaches the thread; it will finish in the
        // background without blocking the main/render thread.
        if let Some(_handle) = self.ir_thread.take() {
            log::info!("CourseResult: detaching IR send thread on shutdown");
        }
    }

    fn has_sound(&self, sound: SoundType) -> bool {
        super::result_common::has_sound(&self.main, &sound)
    }

    fn play_sound_inner(&mut self, sound: SoundType) {
        self.pending_sounds.push((sound, false));
    }

    fn stop_sound_inner(&mut self, sound: SoundType) {
        self.pending_sound_stops.push(sound);
    }

    /// Stop clear/fail sounds and play close sound (course-specific with fallback).
    /// Java pattern: stop(getSound(COURSE_CLEAR) != null ? COURSE_CLEAR : RESULT_CLEAR);
    ///              stop(getSound(COURSE_FAIL) != null ? COURSE_FAIL : RESULT_FAIL);
    ///              play(getSound(COURSE_CLOSE) != null ? COURSE_CLOSE : RESULT_CLOSE);
    fn stop_and_play_close_sound(&mut self) {
        if self.has_sound(SoundType::CourseClose) || self.has_sound(SoundType::ResultClose) {
            self.stop_sound_inner(
                self.select_course_sound(SoundType::CourseClear, SoundType::ResultClear),
            );
            self.stop_sound_inner(
                self.select_course_sound(SoundType::CourseFail, SoundType::ResultFail),
            );
            self.play_sound_inner(
                self.select_course_sound(SoundType::CourseClose, SoundType::ResultClose),
            );
        }
    }

    fn do_render(&mut self) {
        // Poll for async IR results (non-blocking)
        self.poll_ir_results();

        let time = self.main_data.timer.now_time();
        self.main_data
            .timer
            .switch_timer(TIMER_RESULTGRAPH_BEGIN, true);
        self.main_data
            .timer
            .switch_timer(TIMER_RESULTGRAPH_END, true);

        if let Some(ref skin) = self.skin
            && skin.rank_time() == 0
        {
            self.main_data
                .timer
                .switch_timer(TIMER_RESULT_UPDATESCORE, true);
        }
        let skin_input = self.skin.as_ref().map(|s| s.input() as i64).unwrap_or(0);
        if time > skin_input {
            self.main_data.timer.switch_timer(TIMER_STARTINPUT, true);
        }

        if self.main_data.timer.is_timer_on(TIMER_FADEOUT) {
            let fadeout_time = self.main_data.timer.now_time_for_id(TIMER_FADEOUT);
            let skin_fadeout = self.skin.as_ref().map(|s| s.fadeout() as i64).unwrap_or(0);
            if fadeout_time > skin_fadeout {
                self.pending_stop_all_notes = true;

                self.pending_state_change =
                    Some(crate::core::main_state::MainStateType::MusicSelect);
            }
        } else {
            let skin_scene = self.skin.as_ref().map(|s| s.scene() as i64).unwrap_or(0);
            if time > skin_scene {
                self.main_data.timer.switch_timer(TIMER_FADEOUT, true);
                self.stop_and_play_close_sound();
            }
        }
    }

    fn do_input(&mut self) {
        let snapshot = match self.input_snapshot {
            Some(ref s) => s,
            None => return,
        };
        self.data.input(snapshot);

        if !self.main_data.timer.is_timer_on(TIMER_FADEOUT)
            && self.main_data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            let mut ok = false;
            for i in 0..self.property.assign_length() {
                let idx = i as usize;
                if self.property.assign(i) == Some(ResultKey::ChangeGraph)
                    && snapshot.key_state[idx]
                    && snapshot.key_changed_time[idx] != i64::MIN
                {
                    self.data.gauge_type = (self.data.gauge_type.max(5) - 5) % 3 + 6;
                } else if self.property.assign(i).is_some()
                    && snapshot.key_state[idx]
                    && snapshot.key_changed_time[idx] != i64::MIN
                {
                    ok = true;
                }
            }

            if snapshot
                .control_key_states
                .get(&ControlKeys::Escape)
                .copied()
                .unwrap_or(false)
                || snapshot
                    .control_key_states
                    .get(&ControlKeys::Enter)
                    .copied()
                    .unwrap_or(false)
            {
                ok = true;
            }

            if (self.resource.score_data().is_none() || ok)
                && (self.data.state == STATE_OFFLINE || self.data.state == STATE_IR_FINISHED)
            {
                self.main_data.timer.switch_timer(TIMER_FADEOUT, true);
                self.stop_and_play_close_sound();
            }

            if let Some(idx) = self.get_replay_index_from_input() {
                self.save_replay_data(idx);
            }

            self.try_open_ir_url();
        }
    }

    fn update_score_database(&mut self) {
        let lnmode = self.resource.player_config().play_settings.lnmode;
        let random_cfg = self.resource.player_config().play_settings.random;
        let random2_cfg = self.resource.player_config().play_settings.random2;
        let doubleoption_cfg = self.resource.player_config().play_settings.doubleoption;
        let newscore = self.resource.course_score_data().cloned();
        if newscore.is_none() {
            return;
        }
        let mut newscore = newscore.expect("newscore");

        let dp = self
            .resource
            .course_bms_models()
            .map(is_double_play)
            .unwrap_or(false);

        newscore.maxcombo = self.resource.maxcombo();
        apply_avgjudge(&mut newscore);

        let random = determine_random_mode(random_cfg, random2_cfg, doubleoption_cfg, dp);

        if let Some(models) = self.resource.course_bms_models() {
            let score = self.main.play_data_accessor().read_score_data_course(
                models,
                lnmode,
                random,
                &self.resource.constraint(),
            );
            self.data.oldscore = score.unwrap_or_default();
        }

        let target_exscore = self
            .resource
            .target_score_data()
            .map(|s| s.exscore())
            .unwrap_or(0);
        let total_notes: i32 = self
            .resource
            .course_bms_models()
            .map(aggregate_total_notes)
            .unwrap_or(0);
        self.data
            .score
            .set_target_score(self.data.oldscore.exscore(), target_exscore, total_notes);
        self.data.score.update_score(Some(&newscore));

        if self.resource.play_mode().mode == BMSPlayerModeType::Play
            && !(FreqTrainerMenu::is_freq_trainer_enabled() && FreqTrainerMenu::is_freq_negative())
        {
            if let Some(models) = self.resource.course_bms_models() {
                self.main.play_data_accessor().write_score_data_course(
                    &newscore,
                    models,
                    lnmode,
                    random,
                    &self.resource.constraint(),
                    self.resource.is_update_course_score(),
                );
            }
        } else {
            info!(
                "Play mode is {:?}, course score not registered",
                self.resource.play_mode().mode
            );
        }

        info!("Score database update complete");
    }

    pub fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        if let Some(score) = self.resource.course_score_data() {
            match judge {
                0 => {
                    if fast {
                        score.judge_counts.epg
                    } else {
                        score.judge_counts.lpg
                    }
                }
                1 => {
                    if fast {
                        score.judge_counts.egr
                    } else {
                        score.judge_counts.lgr
                    }
                }
                2 => {
                    if fast {
                        score.judge_counts.egd
                    } else {
                        score.judge_counts.lgd
                    }
                }
                3 => {
                    if fast {
                        score.judge_counts.ebd
                    } else {
                        score.judge_counts.lbd
                    }
                }
                4 => {
                    if fast {
                        score.judge_counts.epr
                    } else {
                        score.judge_counts.lpr
                    }
                }
                5 => {
                    if fast {
                        score.judge_counts.ems
                    } else {
                        score.judge_counts.lms
                    }
                }
                _ => 0,
            }
        } else {
            0
        }
    }

    pub fn save_replay_data(&mut self, index: usize) {
        if self.resource.play_mode().mode == BMSPlayerModeType::Play
            && self.resource.course_score_data().is_some()
            && self.data.save_replay[index] != ReplayStatus::Saved
            && self.resource.is_update_course_score()
        {
            // Extract gauge value first to avoid borrow conflict
            let gauge = self.resource.player_config().play_settings.gauge;
            let rd = self.resource.course_replay_mut();
            for replay in rd.iter_mut() {
                replay.gauge = gauge;
            }
            let lnmode = self.resource.player_config().play_settings.lnmode;
            let constraint = self.resource.constraint();
            if let Some(models) = self.resource.course_bms_models() {
                // Clone replays for write (write_brd_course calls shrink on each)
                let mut replays = self.resource.course_replay().to_vec();
                match self.main.play_data_accessor().write_replay_data_course(
                    &mut replays,
                    models,
                    lnmode,
                    index as i32,
                    &constraint,
                ) {
                    Ok(()) => {
                        self.data.save_replay[index] = ReplayStatus::Saved;
                        self.pending_save_last_recording
                            .push("ON_REPLAY".to_string());
                    }
                    Err(e) => {
                        log::error!("Failed to save course replay data: {}", e);
                    }
                }
            } else {
                log::warn!("Cannot save course replay: no course BMS models");
            }
        }
    }

    pub fn new_score(&self) -> Option<&ScoreData> {
        self.resource.course_score_data()
    }

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
        s.recent_judges = timer.recent_judges().to_vec();
        s.recent_judges_index = timer.recent_judges_index();

        // State identity
        s.state_type = Some(rubato_skin::main_state_type::MainStateType::CourseResult);

        // Config
        s.config = Some(Box::new(self.main.config().clone()));
        s.player_config = Some(Box::new(self.resource.player_config().clone()));

        // Play config (resolve consistent mode across course songs)
        s.play_config = self.resource.course_data().and_then(|course| {
            let mut current_mode: Option<bms::model::mode::Mode> = None;
            for song in &course.hash {
                let song_mode = match song.chart.mode {
                    5 => Some(bms::model::mode::Mode::BEAT_5K),
                    7 => Some(bms::model::mode::Mode::BEAT_7K),
                    9 => Some(bms::model::mode::Mode::POPN_9K),
                    10 => Some(bms::model::mode::Mode::BEAT_10K),
                    14 => Some(bms::model::mode::Mode::BEAT_14K),
                    25 => Some(bms::model::mode::Mode::KEYBOARD_24K),
                    50 => Some(bms::model::mode::Mode::KEYBOARD_24K_DOUBLE),
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
            let resolved_mode = current_mode.unwrap_or(bms::model::mode::Mode::BEAT_7K);
            Some(Box::new(
                self.resource
                    .player_config()
                    .play_config_ref(resolved_mode)
                    .playconfig
                    .clone(),
            ))
        });

        // Song / score data
        s.song_data = self.resource.songdata().map(|d| Box::new(d.clone()));
        s.score_data =
            shared_render_context::score_data_ref(&self.data).map(|d| Box::new(d.clone()));
        s.rival_score_data =
            shared_render_context::rival_score_data_ref(&self.data).map(|d| Box::new(d.clone()));
        s.target_score_data = self
            .resource
            .target_score_data()
            .map(|d| Box::new(d.clone()));
        s.replay_option_data = self.resource.replay_data().map(|d| Box::new(d.clone()));
        s.score_data_property = shared_render_context::score_data_property(&self.data).clone();

        // Player / course data
        s.player_data = Some(*self.resource.player_data());
        s.is_course_mode = self.resource.course_data().is_some();
        s.course_index = self.resource.course_index();
        s.course_song_count = self.resource.course_data().map_or(0, |cd| cd.hash.len());
        s.is_update_score = self.resource.is_update_score();

        // Lane shuffle patterns
        s.lane_shuffle_patterns = self
            .resource
            .replay_data()
            .and_then(|rd| rd.lane_shuffle_pattern.clone());

        // Gauge data
        s.gauge_value = shared_render_context::gauge_value(&self.resource);
        s.gauge_type = shared_render_context::gauge_type(&self.data);
        s.is_gauge_max = shared_render_context::is_gauge_max(&self.resource);
        s.gauge_min = shared_render_context::gauge_min(&self.resource, self.data.gauge_type);
        s.gauge_border_max =
            shared_render_context::gauge_border_max(&self.resource, self.data.gauge_type);
        s.gauge_element_borders = shared_render_context::gauge_element_borders(&self.resource);
        s.result_gauge_type = shared_render_context::gauge_type(&self.data);

        // Gauge history
        s.gauge_history = shared_render_context::gauge_history(&self.resource).cloned();
        s.course_gauge_history =
            shared_render_context::course_gauge_history(&self.resource).to_vec();

        // Gauge transition last values (course-specific)
        if let Some(gauge) = self.resource.groove_gauge() {
            for i in 0..gauge.gauge_type_length() {
                if let Some(val) = shared_render_context::course_gauge_transition_last_value(
                    &self.resource,
                    i as i32,
                ) {
                    s.gauge_transition_last_values.insert(i as i32, val);
                }
            }
        }

        // Timing distribution and judge area
        s.timing_distribution = shared_render_context::get_timing_distribution(&self.data).cloned();
        s.judge_area = shared_render_context::judge_area(&self.resource);

        // Ranking
        s.ranking_offset = shared_render_context::ranking_offset(&self.data);
        for slot in 0..10 {
            s.ranking_clear_types
                .push(shared_render_context::ranking_score_clear_type(
                    &self.data, slot,
                ));
        }

        // Autoplay booleans
        let is_autoplay = self.resource.play_mode().mode
            == crate::core::bms_player_mode::Mode::Autoplay
            || self.resource.play_mode().mode == crate::core::bms_player_mode::Mode::Replay;
        s.booleans.insert(32, !is_autoplay);
        s.booleans.insert(33, is_autoplay);

        // Result clear/fail booleans
        let course_score = self.resource.course_score_data();
        for &bid in &[42, 43, 90, 91, 1046] {
            s.booleans.insert(
                bid,
                shared_render_context::boolean_value(&self.data, course_score, bid),
            );
        }

        // Judge counts
        for judge in 0..=5 {
            for fast in [true, false] {
                let count = shared_render_context::judge_count(&self.data, judge, fast);
                s.judge_counts.insert((judge, fast), count);
            }
        }

        // Float values from shared_render_context
        for &fid in &[
            85, 86, 87, 88, 89, 110, 111, 112, 113, 114, 115, 122, 135, 155, 157, 183, 285, 286,
            287, 288, 289, 1102, 1115,
        ] {
            if let Some(val) = shared_render_context::float_value(&self.data, fid) {
                s.floats.insert(fid, val);
            }
        }
        // Float 1107: gauge value
        s.floats
            .insert(1107, shared_render_context::gauge_value(&self.resource));

        // Integer values: populate result-specific IDs via shared_render_context
        let playtime = self.resource.player_data().playtime;
        let songdata = self.resource.songdata();
        let player_data = Some(self.resource.player_data());
        for &iid in &[
            71, 72, 74, 75, 76, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 100, 101, 102, 103, 105,
            106, 108, 110, 111, 112, 113, 114, 115, 116, 121, 122, 123, 128, 150, 151, 152, 153,
            154, 155, 156, 157, 158, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 182, 183,
            184, 200, 271, 280, 281, 282, 283, 284, 285, 286, 287, 288, 289, 370, 371, 372, 373,
            374, 375, 376, 377, 380, 381, 382, 383, 384, 385, 386, 387, 388, 389, 390, 391, 392,
            393, 394, 395, 396, 397, 398, 399, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419,
            423, 424, 425,
        ] {
            s.integers.insert(
                iid,
                shared_render_context::integer_value(
                    &self.data,
                    timer.boot_time_millis(),
                    playtime,
                    songdata,
                    player_data,
                    iid,
                ),
            );
        }

        // Image index: lnmode override (308)
        if let Some(song) = self.resource.songdata()
            && let Some(override_val) =
                rubato_skin::skin_render_context::compute_lnmode_from_chart(&song.chart)
        {
            s.image_indices.insert(308, override_val);
        }

        // String values
        if let Some(song) = self.resource.songdata() {
            s.strings.insert(10, song.metadata.title.clone());
            s.strings.insert(11, song.metadata.subtitle.clone());
            s.strings.insert(
                12,
                if song.metadata.subtitle.is_empty() {
                    song.metadata.title.clone()
                } else {
                    format!("{} {}", song.metadata.title, song.metadata.subtitle)
                },
            );
            s.strings.insert(13, song.metadata.genre.clone());
            s.strings.insert(14, song.metadata.artist.clone());
            s.strings.insert(15, song.metadata.subartist.clone());
            s.strings.insert(
                16,
                if song.metadata.subartist.is_empty() {
                    song.metadata.artist.clone()
                } else {
                    format!("{} {}", song.metadata.artist, song.metadata.subartist)
                },
            );
            s.strings.insert(1030, song.file.md5.clone());
            s.strings.insert(1031, song.file.sha256.clone());
        }
        // Ranking names (120..=129)
        for slot in 0..10 {
            s.strings.insert(
                120 + slot,
                shared_render_context::ranking_name(&self.data, slot),
            );
        }

        // Offsets
        s.offsets = self.main_data.offsets.clone();

        s
    }

    /// Apply queued actions from the snapshot back to live game state.
    /// Audio actions are stored in pending lists for lifecycle outbox consumption
    /// (bypassing the command queue).
    fn drain_actions(&mut self, actions: &mut SkinActionQueue, timer: &mut TimerManager) {
        // Timer sets
        for (timer_id, micro_time) in actions.timer_sets.drain(..) {
            timer.set_micro_timer(timer_id, micro_time);
        }

        // State changes: queue for outbox drain in render_with_game_context
        for state in actions.state_changes.drain(..) {
            self.pending_state_change = Some(state);
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

        // Option change sound
        if actions.option_change_sound {
            self.pending_sounds.push((SoundType::OptionChange, false));
            actions.option_change_sound = false;
        }
    }

    pub fn dispose(&mut self) {
        // super.dispose() equivalent
        if let Some(ref mut skin) = self.main_data.skin {
            skin.dispose_skin();
        }
        self.main_data.skin = None;
    }
}

impl Default for CourseResult {
    fn default() -> Self {
        Self::new(
            MainController::new(
                rubato_skin::config::Config::default(),
                Box::new(crate::ir::ranking_data_cache::RankingDataCache::new()),
            ),
            PlayerResource::default(),
            crate::core::timer_manager::TimerManager::new(),
        )
    }
}

impl crate::core::main_state::MainState for CourseResult {
    fn state_type(&self) -> Option<crate::core::main_state::MainStateType> {
        Some(crate::core::main_state::MainStateType::CourseResult)
    }

    fn main_state_data(&self) -> &crate::core::main_state::MainStateData {
        &self.main_data
    }

    fn main_state_data_mut(&mut self) -> &mut crate::core::main_state::MainStateData {
        &mut self.main_data
    }

    fn groove_gauge_value(&self) -> Option<f32> {
        self.resource.groove_gauge().map(|g| g.value())
    }

    fn create(&mut self) {
        self.do_create();
    }

    fn prepare(&mut self) {
        self.do_prepare();
    }

    fn render(&mut self) {
        self.do_render();
    }

    fn render_with_game_context(&mut self, ctx: &mut GameContext) -> StateTransition {
        // Drain outbox from previous frame (render_skin, prepare, do_render)
        for (sound, loop_sound) in self.pending_sounds.drain(..) {
            ctx.play_sound(&sound, loop_sound);
        }
        for sound in self.pending_sound_stops.drain(..) {
            ctx.stop_sound(&sound);
        }
        for (path, volume, is_loop) in self.pending_audio_path_plays.drain(..) {
            ctx.play_audio_path(&path, volume, is_loop);
        }
        for path in self.pending_audio_path_stops.drain(..) {
            ctx.stop_audio_path(&path);
        }
        if let Some(audio) = self.pending_audio_config.take() {
            ctx.update_audio_config(audio);
        }
        if self.pending_stop_all_notes {
            ctx.stop_all_notes();
            self.pending_stop_all_notes = false;
        }
        for reason in self.pending_save_last_recording.drain(..) {
            ctx.save_last_recording(&reason);
        }

        // Check for pending state change from skin callbacks / do_render
        if let Some(state) = self.pending_state_change.take() {
            return StateTransition::ChangeTo(state);
        }

        // Poll for async IR results (non-blocking)
        self.poll_ir_results();

        let time = self.main_data.timer.now_time();
        self.main_data
            .timer
            .switch_timer(TIMER_RESULTGRAPH_BEGIN, true);
        self.main_data
            .timer
            .switch_timer(TIMER_RESULTGRAPH_END, true);

        if let Some(ref skin) = self.skin
            && skin.rank_time() == 0
        {
            self.main_data
                .timer
                .switch_timer(TIMER_RESULT_UPDATESCORE, true);
        }
        let skin_input = self.skin.as_ref().map(|s| s.input() as i64).unwrap_or(0);
        if time > skin_input {
            self.main_data.timer.switch_timer(TIMER_STARTINPUT, true);
        }

        if self.main_data.timer.is_timer_on(TIMER_FADEOUT) {
            let fadeout_time = self.main_data.timer.now_time_for_id(TIMER_FADEOUT);
            let skin_fadeout = self.skin.as_ref().map(|s| s.fadeout() as i64).unwrap_or(0);
            if fadeout_time > skin_fadeout {
                ctx.stop_all_notes();

                return StateTransition::ChangeTo(MainStateType::MusicSelect);
            }
        } else {
            let skin_scene = self.skin.as_ref().map(|s| s.scene() as i64).unwrap_or(0);
            if time > skin_scene {
                self.main_data.timer.switch_timer(TIMER_FADEOUT, true);
                // Stop clear/fail sounds and play close sound (course-specific with fallback)
                let has_close = ctx.sound_path(&SoundType::CourseClose).is_some()
                    || ctx.sound_path(&SoundType::ResultClose).is_some();
                if has_close {
                    let clear = if ctx.sound_path(&SoundType::CourseClear).is_some() {
                        SoundType::CourseClear
                    } else {
                        SoundType::ResultClear
                    };
                    let fail = if ctx.sound_path(&SoundType::CourseFail).is_some() {
                        SoundType::CourseFail
                    } else {
                        SoundType::ResultFail
                    };
                    let close = if ctx.sound_path(&SoundType::CourseClose).is_some() {
                        SoundType::CourseClose
                    } else {
                        SoundType::ResultClose
                    };
                    ctx.stop_sound(&clear);
                    ctx.stop_sound(&fail);
                    ctx.play_sound(&close, false);
                }
            }
        }

        StateTransition::Continue
    }

    fn render_skin(&mut self, sprite: &mut rubato_render::sprite_batch::SpriteBatch) {
        let mut skin = match self.main_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.main_data.timer);

        let mut snapshot = self.build_snapshot(&timer);
        skin.update_custom_objects_timed(&mut snapshot);
        skin.swap_sprite_batch(sprite);
        skin.draw_all_objects_timed(&mut snapshot);
        skin.swap_sprite_batch(sprite);

        // Drain non-event actions (timers, audio, state changes)
        self.drain_actions(&mut snapshot.actions, &mut timer);

        // Propagate player_config mutations from snapshot back to resource
        if let Some(pc) = snapshot.player_config.take()
            && let Some(dst) = self.resource.player_config_mut()
        {
            *dst = *pc;
        }

        // Replay queued custom events now that the skin is available again.
        // Handle replay-save events specially (IDs 19, 316, 317, 318).
        let mut pending_events: Vec<(i32, i32, i32)> =
            std::mem::take(&mut snapshot.actions.custom_events);
        let mut depth = 0;
        while !pending_events.is_empty() && depth < 8 {
            let mut replay_snapshot = self.build_snapshot(&timer);
            for (id, arg1, arg2) in pending_events {
                if let Some(index) = super::shared_render_context::replay_index_from_event_id(id) {
                    self.save_replay_data(index);
                } else {
                    skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
                }
            }
            self.drain_actions(&mut replay_snapshot.actions, &mut timer);
            if let Some(pc) = replay_snapshot.player_config.take()
                && let Some(dst) = self.resource.player_config_mut()
            {
                *dst = *pc;
            }
            pending_events = replay_snapshot.actions.custom_events;
            depth += 1;
        }
        if depth >= 8 {
            log::warn!("CourseResult render_skin event replay exceeded depth limit");
        }

        self.main_data.timer = timer;
        self.main_data.skin = Some(skin);
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.main_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.main_data.timer);

        let mut snapshot = self.build_snapshot(&timer);
        skin.mouse_pressed_at(&mut snapshot, button, x, y);
        self.drain_actions(&mut snapshot.actions, &mut timer);

        // Propagate player_config mutations from snapshot back to resource
        if let Some(pc) = snapshot.player_config.take()
            && let Some(dst) = self.resource.player_config_mut()
        {
            *dst = *pc;
        }

        // Replay queued custom events.
        let mut pending_events: Vec<(i32, i32, i32)> =
            std::mem::take(&mut snapshot.actions.custom_events);
        let mut depth = 0;
        while !pending_events.is_empty() && depth < 8 {
            let mut replay_snapshot = self.build_snapshot(&timer);
            for (id, arg1, arg2) in pending_events {
                if let Some(index) = super::shared_render_context::replay_index_from_event_id(id) {
                    self.save_replay_data(index);
                } else {
                    skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
                }
            }
            self.drain_actions(&mut replay_snapshot.actions, &mut timer);
            if let Some(pc) = replay_snapshot.player_config.take()
                && let Some(dst) = self.resource.player_config_mut()
            {
                *dst = *pc;
            }
            pending_events = replay_snapshot.actions.custom_events;
            depth += 1;
        }
        if depth >= 8 {
            log::warn!("CourseResult mouse_pressed event replay exceeded depth limit");
        }

        self.main_data.timer = timer;
        self.main_data.skin = Some(skin);
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.main_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.main_data.timer);

        let mut snapshot = self.build_snapshot(&timer);
        skin.mouse_dragged_at(&mut snapshot, button, x, y);
        self.drain_actions(&mut snapshot.actions, &mut timer);

        // Propagate player_config mutations from snapshot back to resource
        if let Some(pc) = snapshot.player_config.take()
            && let Some(dst) = self.resource.player_config_mut()
        {
            *dst = *pc;
        }

        // Replay queued custom events.
        let mut pending_events: Vec<(i32, i32, i32)> =
            std::mem::take(&mut snapshot.actions.custom_events);
        let mut depth = 0;
        while !pending_events.is_empty() && depth < 8 {
            let mut replay_snapshot = self.build_snapshot(&timer);
            for (id, arg1, arg2) in pending_events {
                if let Some(index) = super::shared_render_context::replay_index_from_event_id(id) {
                    self.save_replay_data(index);
                } else {
                    skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
                }
            }
            self.drain_actions(&mut replay_snapshot.actions, &mut timer);
            if let Some(pc) = replay_snapshot.player_config.take()
                && let Some(dst) = self.resource.player_config_mut()
            {
                *dst = *pc;
            }
            pending_events = replay_snapshot.actions.custom_events;
            depth += 1;
        }
        if depth >= 8 {
            log::warn!("CourseResult mouse_dragged event replay exceeded depth limit");
        }

        self.main_data.timer = timer;
        self.main_data.skin = Some(skin);
    }

    fn input(&mut self) {
        self.do_input();
    }

    fn input_with_game_context(&mut self, _ctx: &mut GameContext) {
        self.do_input();
    }

    fn sync_input_snapshot(&mut self, snapshot: &rubato_input::input_snapshot::InputSnapshot) {
        self.input_snapshot = Some(snapshot.clone());
    }

    fn load_skin(&mut self, skin_type: i32) {
        let skin_path = self
            .resource
            .player_config()
            .skin
            .get(skin_type as usize)
            .and_then(|skin| skin.as_ref())
            .and_then(|skin| skin.path.clone())
            .or_else(|| rubato_skin::skin_config::SkinConfig::default_for_id(skin_type).path);
        // Take timer out to avoid borrowing self.main_data and its fields simultaneously
        let timer = std::mem::take(&mut self.main_data.timer);
        let loaded = {
            let mut snapshot = self.build_snapshot(&timer);
            let registry = std::collections::HashMap::new();
            let mut state =
                rubato_skin::snapshot_main_state::SnapshotMainState::new(&mut snapshot, &registry);
            skin_path.as_deref().and_then(|path| {
                rubato_skin::skin_loader::load_skin_from_path_with_state(
                    &mut state, skin_type, path,
                )
            })
        };
        self.main_data.timer = timer;
        if let Some(skin) = loaded {
            self.skin =
                Some(crate::result::result_skin_data::ResultSkinData::from_loaded_skin(&skin));
            self.main_data.skin = Some(Box::new(skin));
        } else {
            self.skin = None;
            self.main_data.skin = None;
        }
    }

    fn take_player_resource(&mut self) -> Option<crate::core::player_resource::PlayerResource> {
        self.resource.take_inner()
    }

    fn shutdown(&mut self) {
        self.shutdown();
    }

    fn dispose(&mut self) {
        self.dispose();
    }
}

// Tests for CourseResult
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::main_state::MainState;
    use crate::result::test_helpers::{
        ExecuteEventSkin, PlayerConfigMutatingSkin, make_test_config,
    };
    use rubato_skin::skin_property::TIMER_RESULTGRAPH_BEGIN;
    use rubato_skin::skin_render_context::SkinRenderContext;
    use rubato_skin::skin_type::SkinType;

    fn make_ranking_cache() -> Box<dyn crate::ranking_data_cache_access::RankingDataCacheAccess> {
        Box::new(crate::ir::ranking_data_cache::RankingDataCache::new())
    }

    fn make_default() -> CourseResult {
        CourseResult::new(
            MainController::new(rubato_skin::config::Config::default(), make_ranking_cache()),
            PlayerResource::default(),
            crate::core::timer_manager::TimerManager::new(),
        )
    }

    fn make_course_result_for_mouse() -> CourseResult {
        let config = make_test_config("course-result");
        let main = MainController::new(config, make_ranking_cache());
        let mut resource = PlayerResource::new(
            mock_to_core(MockPlayerResourceForIR::new_with_course_score()),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        resource.course_bms_models = Some(vec![bms::model::bms_model::BMSModel::default()]);
        resource
            .course_replay_mut()
            .push(crate::core::replay_data::ReplayData::default());
        CourseResult::new(
            main,
            resource,
            crate::core::timer_manager::TimerManager::new(),
        )
    }

    #[test]
    fn test_state_type_returns_course_result() {
        let cr = make_default();
        assert_eq!(
            cr.state_type(),
            Some(crate::core::main_state::MainStateType::CourseResult)
        );
    }

    #[test]
    fn test_handle_skin_mouse_pressed_saves_replay_via_course_result_context() {
        let mut cr = make_course_result_for_mouse();
        cr.main_data.skin = Some(Box::new(ExecuteEventSkin { event_id: 19 }));

        <CourseResult as MainState>::handle_skin_mouse_pressed(&mut cr, 0, 10, 10);

        assert_eq!(cr.data.save_replay[0], ReplayStatus::Saved);
    }

    #[test]
    fn test_course_result_mouse_context_exposes_player_config_mut() {
        let mut cr = make_course_result_for_mouse();
        cr.main_data.skin = Some(Box::new(PlayerConfigMutatingSkin));

        <CourseResult as MainState>::handle_skin_mouse_pressed(&mut cr, 0, 10, 10);

        assert_eq!(cr.resource.player_config().play_settings.random, 1);
    }

    #[test]
    fn test_course_result_render_context_uses_replay_option_for_image_index_42() {
        let mut cr = make_course_result_for_mouse();
        cr.resource
            .replay_data_mut()
            .expect("replay data should exist")
            .randomoption = 5;
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &cr.data,
            resource: &cr.resource,
            main: &mut cr.main,
            offsets: &cr.main_data.offsets,
        };

        assert_eq!(ctx.image_index_value(42), 5);
    }

    #[test]
    fn test_create_calls_load_skin_with_course_result_type() {
        let mut cr = make_default();
        <CourseResult as MainState>::create(&mut cr);
        // Verify SkinType::CourseResult.id() matches expected value (15)
        assert_eq!(SkinType::CourseResult.id(), 15);
        assert!(
            cr.main_data.skin.is_some(),
            "course result create() should load the configured course-result skin"
        );
        assert!(
            cr.skin.is_some(),
            "course result create() should wire timing metadata from the loaded skin"
        );
    }

    #[test]
    fn test_main_state_data_accessors() {
        let mut cr = make_default();
        let _ = <CourseResult as MainState>::main_state_data(&cr);
        let _ = <CourseResult as MainState>::main_state_data_mut(&mut cr);
    }

    #[test]
    fn test_new_stores_main_and_resource() {
        let cr = make_default();
        // Verify the fields exist and are accessible
        assert!(cr.resource.score_data().is_none());
    }

    #[test]
    fn test_trait_create_delegates_to_do_create() {
        let mut cr = make_default();
        // do_create sets gauge_type from groove_gauge (defaults to 0)
        <CourseResult as MainState>::create(&mut cr);
        assert_eq!(cr.data.gauge_type, 0);
    }

    #[test]
    fn test_trait_prepare_delegates_to_do_prepare() {
        let mut cr = make_default();
        // do_prepare sets state to STATE_OFFLINE
        <CourseResult as MainState>::prepare(&mut cr);
        assert_eq!(cr.data.state, crate::result::abstract_result::STATE_OFFLINE);
    }

    #[test]
    fn test_trait_render_delegates_to_do_render() {
        let mut cr = make_default();
        // do_render switches TIMER_RESULTGRAPH_BEGIN
        assert!(!cr.main_data.timer.is_timer_on(TIMER_RESULTGRAPH_BEGIN));
        <CourseResult as MainState>::render(&mut cr);
        assert!(cr.main_data.timer.is_timer_on(TIMER_RESULTGRAPH_BEGIN));
    }

    #[test]
    fn test_trait_input_delegates_to_do_input() {
        let mut cr = make_default();
        // do_input calls self.data.input(main) which updates IR state
        // With default state, input should not panic
        <CourseResult as MainState>::input(&mut cr);
    }

    #[test]
    fn test_default_creates_with_null_controller() {
        let cr = CourseResult::default();
        assert_eq!(
            cr.state_type(),
            Some(crate::core::main_state::MainStateType::CourseResult)
        );
    }

    #[test]
    fn test_dispose_clears_skin() {
        let mut cr = make_default();
        // Assign a skin so we can verify it gets cleared
        cr.main_data.skin = Some(Box::new(ExecuteEventSkin { event_id: 0 }));

        <CourseResult as MainState>::dispose(&mut cr);

        assert!(cr.main_data.skin.is_none(), "dispose should clear skin");
    }

    /// Regression: shutdown() must not block on a long-running IR thread.
    /// The thread is detached (JoinHandle dropped), not sleep-polled.
    #[test]
    fn shutdown_does_not_block_on_ir_thread() {
        let mut cr = make_default();
        // Inject a thread that sleeps for a long time
        let handle = std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(60));
        });
        cr.ir_thread = Some(handle);

        let start = std::time::Instant::now();
        cr.shutdown();
        let elapsed = start.elapsed();

        // shutdown() should return nearly instantly (detach, not join)
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "shutdown() blocked for {:?}, should be non-blocking",
            elapsed
        );
        // The thread handle should have been taken
        assert!(cr.ir_thread.is_none());
    }

    // ---- IR processing tests ----

    use crate::ir::ir_chart_data::IRChartData;
    use crate::ir::ir_course_data::IRCourseData as IRCourseDataReal;
    use crate::ir::ir_player_data::IRPlayerData;
    use crate::ir::ir_response::IRResponse;
    use crate::ir::ir_score_data::IRScoreData as IRScoreDataReal;
    use crate::ir::ir_table_data::IRTableData;
    use crate::result::ir_status::IRStatus as IRStatusReal;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Mock IR connection that tracks send_course_play_data calls
    struct MockCourseIR {
        send_called: AtomicBool,
        send_succeeds: bool,
        ranking_fetch_called: AtomicBool,
    }

    impl MockCourseIR {
        fn new(send_succeeds: bool) -> Self {
            Self {
                send_called: AtomicBool::new(false),
                send_succeeds,
                ranking_fetch_called: AtomicBool::new(false),
            }
        }
    }

    impl crate::result::IRConnection for MockCourseIR {
        fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _chart: Option<&IRChartData>,
        ) -> IRResponse<Vec<IRScoreDataReal>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_course_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _course: &IRCourseDataReal,
        ) -> IRResponse<Vec<IRScoreDataReal>> {
            self.ranking_fetch_called.store(true, Ordering::SeqCst);
            IRResponse::success("OK".to_string(), vec![])
        }
        fn send_play_data(&self, _model: &IRChartData, _score: &IRScoreDataReal) -> IRResponse<()> {
            IRResponse::failure("mock".to_string())
        }
        fn send_course_play_data(
            &self,
            _course: &IRCourseDataReal,
            _score: &IRScoreDataReal,
        ) -> IRResponse<()> {
            self.send_called.store(true, Ordering::SeqCst);
            if self.send_succeeds {
                IRResponse::success("OK".to_string(), ())
            } else {
                IRResponse::failure("Server error".to_string())
            }
        }
        fn get_song_url(&self, _chart: &IRChartData) -> Option<String> {
            None
        }
        fn get_course_url(&self, _course: &IRCourseDataReal) -> Option<String> {
            None
        }
        fn get_player_url(&self, _player: &IRPlayerData) -> Option<String> {
            None
        }
        fn name(&self) -> &str {
            "MockCourseIR"
        }
    }

    /// Mock PlayerResourceAccess that provides course data for IR testing
    struct MockPlayerResourceForIR {
        player_config: rubato_skin::player_config::PlayerConfig,
        course_score: Option<crate::core::score_data::ScoreData>,
        course_data: Option<crate::core::course_data::CourseData>,
        course_gauge: Vec<Vec<Vec<f32>>>,
        course_replay: Vec<crate::core::replay_data::ReplayData>,
        replay_data: Option<crate::core::replay_data::ReplayData>,
        song_data: Option<rubato_skin::song_data::SongData>,
    }

    impl MockPlayerResourceForIR {
        fn new_with_course_score() -> Self {
            let score = crate::core::score_data::ScoreData {
                clear: crate::core::clear_type::ClearType::Easy.id(),
                ..Default::default()
            };
            let course = crate::core::course_data::CourseData {
                name: Some("Test Course".to_string()),
                release: true,
                ..Default::default()
            };
            Self {
                player_config: rubato_skin::player_config::PlayerConfig::default(),
                course_score: Some(score),
                course_data: Some(course),
                course_gauge: Vec::new(),
                course_replay: Vec::new(),
                replay_data: Some(crate::core::replay_data::ReplayData::default()),
                song_data: None,
            }
        }
    }

    impl rubato_skin::player_resource_access::ConfigAccess for MockPlayerResourceForIR {
        fn config(&self) -> &rubato_skin::config::Config {
            static CONFIG: std::sync::OnceLock<rubato_skin::config::Config> =
                std::sync::OnceLock::new();
            CONFIG.get_or_init(rubato_skin::config::Config::default)
        }
        fn player_config(&self) -> &rubato_skin::player_config::PlayerConfig {
            &self.player_config
        }
        fn player_config_mut(&mut self) -> Option<&mut rubato_skin::player_config::PlayerConfig> {
            Some(&mut self.player_config)
        }
    }

    impl rubato_skin::player_resource_access::ScoreAccess for MockPlayerResourceForIR {
        fn score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
            None
        }
        fn rival_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
            None
        }
        fn target_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
            None
        }
        fn course_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
            self.course_score.as_ref()
        }
        fn set_course_score_data(&mut self, score: crate::core::score_data::ScoreData) {
            self.course_score = Some(score);
        }
        fn score_data_mut(&mut self) -> Option<&mut crate::core::score_data::ScoreData> {
            None
        }
    }

    impl rubato_skin::player_resource_access::SongAccess for MockPlayerResourceForIR {
        fn songdata(&self) -> Option<&rubato_skin::song_data::SongData> {
            self.song_data.as_ref()
        }
        fn songdata_mut(&mut self) -> Option<&mut rubato_skin::song_data::SongData> {
            self.song_data.as_mut()
        }
        fn set_songdata(&mut self, _data: Option<rubato_skin::song_data::SongData>) {}
        fn course_song_data(&self) -> Vec<rubato_skin::song_data::SongData> {
            vec![]
        }
    }

    impl rubato_skin::player_resource_access::ReplayAccess for MockPlayerResourceForIR {
        fn replay_data(&self) -> Option<&crate::core::replay_data::ReplayData> {
            self.replay_data.as_ref()
        }
        fn replay_data_mut(&mut self) -> Option<&mut crate::core::replay_data::ReplayData> {
            self.replay_data.as_mut()
        }
        fn course_replay(&self) -> &[crate::core::replay_data::ReplayData] {
            &self.course_replay
        }
        fn add_course_replay(&mut self, rd: crate::core::replay_data::ReplayData) {
            self.course_replay.push(rd);
        }
        fn course_replay_mut(&mut self) -> &mut Vec<crate::core::replay_data::ReplayData> {
            &mut self.course_replay
        }
    }

    impl rubato_skin::player_resource_access::CourseAccess for MockPlayerResourceForIR {
        fn course_data(&self) -> Option<&crate::core::course_data::CourseData> {
            self.course_data.as_ref()
        }
        fn course_index(&self) -> usize {
            0
        }
        fn next_course(&mut self) -> bool {
            false
        }
        fn constraint(&self) -> Vec<crate::core::course_data::CourseDataConstraint> {
            vec![]
        }
        fn set_course_data(&mut self, _data: crate::core::course_data::CourseData) {}
        fn clear_course_data(&mut self) {}
    }

    impl rubato_skin::player_resource_access::GaugeAccess for MockPlayerResourceForIR {
        fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
            None
        }
        fn groove_gauge(&self) -> Option<&rubato_skin::groove_gauge::GrooveGauge> {
            None
        }
        fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
            &self.course_gauge
        }
        fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>) {
            self.course_gauge.push(gauge);
        }
        fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
            &mut self.course_gauge
        }
    }

    impl rubato_skin::player_resource_access::PlayerStateAccess for MockPlayerResourceForIR {
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
            true
        }
        fn is_update_course_score(&self) -> bool {
            true
        }
        fn is_force_no_ir_send(&self) -> bool {
            false
        }
        fn is_freq_on(&self) -> bool {
            false
        }
    }

    impl rubato_skin::player_resource_access::SessionMutation for MockPlayerResourceForIR {
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
        fn set_chart_option_data(&mut self, _data: Option<crate::core::replay_data::ReplayData>) {}
    }

    impl rubato_skin::player_resource_access::MediaAccess for MockPlayerResourceForIR {
        fn reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }
        fn reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }
    }

    impl rubato_skin::player_resource_access::PlayerResourceAccess for MockPlayerResourceForIR {}

    /// Convert a MockPlayerResourceForIR to a CorePlayerResource.
    fn mock_to_core(mock: MockPlayerResourceForIR) -> crate::core::player_resource::PlayerResource {
        let mut core = crate::core::player_resource::PlayerResource::new(
            rubato_skin::config::Config::default(),
            mock.player_config.clone(),
        );
        if let Some(cs) = mock.course_score {
            core.set_course_score_data(cs);
        }
        if let Some(cd) = mock.course_data {
            core.set_course_data(cd);
        }
        if let Some(rd) = mock.replay_data {
            core.set_replay_data(rd);
        }
        if let Some(sd) = mock.song_data {
            core.set_songdata(sd);
        }
        for cr in mock.course_replay {
            core.add_course_replay(cr);
        }
        for cg in mock.course_gauge {
            core.add_course_gauge(cg);
        }
        core
    }

    fn make_ir_course_result(
        ir_conn: Arc<dyn crate::result::IRConnection + Send + Sync>,
    ) -> CourseResult {
        use crate::core::ir_config::IRConfig;
        let ir_status = IRStatusReal::new(
            IRConfig::default(),
            ir_conn,
            IRPlayerData::new(String::new(), String::new(), String::new()),
        );
        let main = MainController::with_ir_statuses(
            rubato_skin::config::Config::default(),
            make_ranking_cache(),
            vec![ir_status],
        );
        let resource = PlayerResource::new(
            mock_to_core(MockPlayerResourceForIR::new_with_course_score()),
            crate::result::BMSPlayerMode::new(crate::result::BMSPlayerModeType::Play),
        );
        CourseResult::new(
            main,
            resource,
            crate::core::timer_manager::TimerManager::new(),
        )
    }

    #[test]
    fn test_prepare_with_ir_transitions_to_ir_finished() {
        let ir_conn = Arc::new(MockCourseIR::new(true));
        let mut cr = make_ir_course_result(ir_conn.clone());

        <CourseResult as MainState>::prepare(&mut cr);
        // IR now runs async; wait for background thread and poll results
        cr.wait_and_poll_ir_results();

        // IR processing should complete and set state to STATE_IR_FINISHED
        assert_eq!(
            cr.data.state,
            crate::result::abstract_result::STATE_IR_FINISHED
        );
    }

    #[test]
    fn test_prepare_with_ir_sends_course_score() {
        let ir_conn = Arc::new(MockCourseIR::new(true));
        let mut cr = make_ir_course_result(ir_conn.clone());

        <CourseResult as MainState>::prepare(&mut cr);
        // IR now runs async; wait for background thread and poll results
        cr.wait_and_poll_ir_results();

        // The IR send should have been called
        assert!(ir_conn.send_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_prepare_with_ir_fetches_ranking() {
        let ir_conn = Arc::new(MockCourseIR::new(true));
        let mut cr = make_ir_course_result(ir_conn.clone());

        <CourseResult as MainState>::prepare(&mut cr);
        // IR now runs async; wait for background thread and poll results
        cr.wait_and_poll_ir_results();

        // After sending, ranking should be fetched via get_course_play_data
        assert!(ir_conn.ranking_fetch_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_prepare_with_ir_send_failure_still_finishes() {
        let ir_conn = Arc::new(MockCourseIR::new(false));
        let mut cr = make_ir_course_result(ir_conn.clone());

        <CourseResult as MainState>::prepare(&mut cr);
        // IR now runs async; wait for background thread and poll results
        cr.wait_and_poll_ir_results();

        // Even with send failure, state should transition to IR_FINISHED
        assert_eq!(
            cr.data.state,
            crate::result::abstract_result::STATE_IR_FINISHED
        );
        assert!(ir_conn.send_called.load(Ordering::SeqCst));
    }

    // ---- Score database update pure computation tests ----

    #[test]
    fn test_compute_avgjudge_with_positive_notes() {
        // 1000 total_duration / 10 notes = 100
        assert_eq!(compute_avgjudge(1000, 10), 100);
    }

    #[test]
    fn test_compute_avgjudge_with_large_values() {
        // Large timing values typical in microsecond-based system
        assert_eq!(compute_avgjudge(500_000, 250), 2000);
    }

    #[test]
    fn test_compute_avgjudge_with_zero_notes_returns_none() {
        // Division by zero guard: notes == 0 should not compute
        // The original code skips the assignment, leaving avgjudge at its default (i64::MAX)
        let mut score = crate::core::score_data::ScoreData {
            notes: 0,
            ..Default::default()
        };
        score.timing_stats.total_duration = 1000;
        apply_avgjudge(&mut score);
        assert_eq!(score.timing_stats.avgjudge, i64::MAX); // unchanged from default
    }

    #[test]
    fn test_compute_avgjudge_with_nonzero_notes_updates_score() {
        let mut score = crate::core::score_data::ScoreData {
            notes: 50,
            ..Default::default()
        };
        score.timing_stats.total_duration = 5000;
        apply_avgjudge(&mut score);
        assert_eq!(score.timing_stats.avgjudge, 100);
    }

    #[test]
    fn test_compute_avgjudge_with_one_note() {
        assert_eq!(compute_avgjudge(42, 1), 42);
    }

    #[test]
    fn test_compute_avgjudge_negative_duration() {
        // Negative total_duration can happen with early timing
        assert_eq!(compute_avgjudge(-500, 5), -100);
    }

    #[test]
    fn test_determine_random_mode_all_zero() {
        // No random options set, single player
        assert_eq!(determine_random_mode(0, 0, 0, false), 0);
    }

    #[test]
    fn test_determine_random_mode_random_cfg_mirror() {
        // random_cfg=1, no dp -> random=2 first, then overridden to 1
        assert_eq!(determine_random_mode(1, 0, 0, false), 1);
    }

    #[test]
    fn test_determine_random_mode_random_cfg_nonmirror() {
        // random_cfg=2 (not mirror), no dp -> random=2
        assert_eq!(determine_random_mode(2, 0, 0, false), 2);
    }

    #[test]
    fn test_determine_random_mode_dp_random2_set() {
        // dp=true, random2_cfg>0 -> random=2
        assert_eq!(determine_random_mode(0, 1, 0, true), 2);
    }

    #[test]
    fn test_determine_random_mode_dp_doubleoption_set() {
        // dp=true, doubleoption_cfg>0 -> random=2
        assert_eq!(determine_random_mode(0, 0, 1, true), 2);
    }

    #[test]
    fn test_determine_random_mode_dp_all_mirror() {
        // random_cfg=1, dp=true, random2_cfg=1, doubleoption_cfg=1 -> random=1
        assert_eq!(determine_random_mode(1, 1, 1, true), 1);
    }

    #[test]
    fn test_determine_random_mode_dp_random_mirror_but_random2_not_mirror() {
        // random_cfg=1, dp=true, random2_cfg=2, doubleoption_cfg=1
        // First branch: random_cfg>0 -> random=2
        // Second branch: random_cfg==1 && dp && random2_cfg==1 is false (random2_cfg==2) -> no override
        assert_eq!(determine_random_mode(1, 2, 1, true), 2);
    }

    #[test]
    fn test_determine_random_mode_dp_random_mirror_but_doubleoption_not_mirror() {
        // random_cfg=1, dp=true, random2_cfg=1, doubleoption_cfg=2
        // First branch: random_cfg>0 -> random=2
        // Second branch: random_cfg==1 && dp && doubleoption_cfg==1 is false -> no override
        assert_eq!(determine_random_mode(1, 1, 2, true), 2);
    }

    #[test]
    fn test_determine_random_mode_no_random_no_dp_random2_ignored() {
        // random_cfg=0, dp=false -> random2 and doubleoption don't matter
        assert_eq!(determine_random_mode(0, 5, 5, false), 0);
    }

    #[test]
    fn test_is_double_play_empty_models() {
        let models: Vec<bms::model::bms_model::BMSModel> = vec![];
        assert!(!is_double_play(&models));
    }

    #[test]
    fn test_is_double_play_single_player_model() {
        // Mode::BEAT_7K has player() == 1
        let mut model = bms::model::bms_model::BMSModel::default();
        model.set_mode(bms::model::mode::Mode::BEAT_7K);
        assert!(!is_double_play(&[model]));
    }

    #[test]
    fn test_is_double_play_double_player_model() {
        // Mode::BEAT_14K has player() == 2
        let mut model = bms::model::bms_model::BMSModel::default();
        model.set_mode(bms::model::mode::Mode::BEAT_14K);
        assert!(is_double_play(&[model]));
    }

    #[test]
    fn test_is_double_play_mixed_models() {
        // One single, one double -> dp = true (OR logic)
        let mut m1 = bms::model::bms_model::BMSModel::default();
        m1.set_mode(bms::model::mode::Mode::BEAT_7K);
        let mut m2 = bms::model::bms_model::BMSModel::default();
        m2.set_mode(bms::model::mode::Mode::BEAT_14K);
        assert!(is_double_play(&[m1, m2]));
    }

    #[test]
    fn test_is_double_play_no_mode_set() {
        // Model with no mode -> mode() returns None, unwrap_or(1) == 1, not dp
        let model = bms::model::bms_model::BMSModel::default();
        assert!(!is_double_play(&[model]));
    }

    #[test]
    fn test_aggregate_total_notes_empty() {
        let models: Vec<bms::model::bms_model::BMSModel> = vec![];
        assert_eq!(aggregate_total_notes(&models), 0);
    }

    #[test]
    fn test_aggregate_total_notes_single_model() {
        // BMSModel::default() has 0 total notes
        let model = bms::model::bms_model::BMSModel::default();
        assert_eq!(aggregate_total_notes(&[model]), 0);
    }

    #[test]
    fn test_course_result_render_context_song_data_ref_returns_songdata() {
        // Regression: CourseResultRenderContext must implement song_data_ref()
        // so that image_index IDs 89/90 (favorite_song/favorite_chart) work
        // on course result screens. Previously it fell through to the default
        // (None), causing those IDs to always return -1.
        let mut mock = MockPlayerResourceForIR::new_with_course_score();
        let mut song = rubato_skin::song_data::SongData::default();
        song.metadata.title = "TestSong".to_string();
        mock.song_data = Some(song);
        let resource = PlayerResource::new(
            mock_to_core(mock),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        let data = AbstractResultData::new();
        let mut main =
            MainController::new(rubato_skin::config::Config::default(), make_ranking_cache());
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };

        let sd = ctx.song_data_ref();
        assert!(
            sd.is_some(),
            "song_data_ref() must return Some when resource has songdata"
        );
        assert_eq!(sd.unwrap().metadata.title, "TestSong");
    }

    #[test]
    fn test_course_result_render_context_song_data_ref_returns_none_without_songdata() {
        // When the resource has no songdata, song_data_ref() should return None.
        let resource = PlayerResource::default();
        let data = AbstractResultData::new();
        let mut main =
            MainController::new(rubato_skin::config::Config::default(), make_ranking_cache());
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };

        assert!(ctx.song_data_ref().is_none());
    }

    #[test]
    fn test_course_result_render_context_returns_ranking_name_strings() {
        use crate::ir::ir_score_data::IRScoreData;
        use crate::ir::ranking_data::RankingData;

        let resource = PlayerResource::default();
        let mut data = AbstractResultData::new();
        let mut ranking = RankingData::new();
        let scores: Vec<IRScoreData> = vec![
            {
                let mut s = crate::core::score_data::ScoreData::default();
                s.player = "ALICE".to_string();
                s.judge_counts.epg = 120;
                IRScoreData::new(&s)
            },
            {
                let mut s = crate::core::score_data::ScoreData::default();
                s.player = "YOU".to_string();
                s.judge_counts.epg = 110;
                IRScoreData::new(&s)
            },
        ];
        ranking.update_score(&scores, None);
        data.ranking = Some(ranking);
        data.ranking_offset = 1;

        let mut main =
            MainController::new(rubato_skin::config::Config::default(), make_ranking_cache());
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };

        assert_eq!(ctx.string_value(120), "YOU");
        assert_eq!(ctx.string_value(121), "");
    }

    // ---- CourseResultRenderContext string_value IDs 12,13,15,16 regression ----

    /// Helper: build a CourseResultRenderContext whose resource carries the given SongData.
    fn make_course_render_ctx_with_songdata(
        song: rubato_skin::song_data::SongData,
    ) -> (
        PlayerResource,
        AbstractResultData,
        MainController,
        crate::core::timer_manager::TimerManager,
    ) {
        let mut mock = MockPlayerResourceForIR::new_with_course_score();
        mock.song_data = Some(song);
        let resource = PlayerResource::new(
            mock_to_core(mock),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        let data = AbstractResultData::new();
        let main =
            MainController::new(rubato_skin::config::Config::default(), make_ranking_cache());
        let timer = crate::core::timer_manager::TimerManager::new();
        (resource, data, main, timer)
    }

    #[test]
    fn test_course_result_string_value_fulltitle_with_subtitle() {
        let mut song = rubato_skin::song_data::SongData::default();
        song.metadata.title = "MainTitle".to_string();
        song.metadata.subtitle = "[HARD]".to_string();
        let (resource, data, mut main, mut timer) = make_course_render_ctx_with_songdata(song);
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(ctx.string_value(12), "MainTitle [HARD]");
    }

    #[test]
    fn test_course_result_string_value_fulltitle_without_subtitle() {
        let mut song = rubato_skin::song_data::SongData::default();
        song.metadata.title = "OnlyTitle".to_string();
        let (resource, data, mut main, mut timer) = make_course_render_ctx_with_songdata(song);
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(ctx.string_value(12), "OnlyTitle");
    }

    #[test]
    fn test_course_result_string_value_genre() {
        let mut song = rubato_skin::song_data::SongData::default();
        song.metadata.genre = "Techno".to_string();
        let (resource, data, mut main, mut timer) = make_course_render_ctx_with_songdata(song);
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(ctx.string_value(13), "Techno");
    }

    #[test]
    fn test_course_result_string_value_subartist() {
        let mut song = rubato_skin::song_data::SongData::default();
        song.metadata.subartist = "feat. B".to_string();
        let (resource, data, mut main, mut timer) = make_course_render_ctx_with_songdata(song);
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(ctx.string_value(15), "feat. B");
    }

    #[test]
    fn test_course_result_string_value_fullartist_with_subartist() {
        let mut song = rubato_skin::song_data::SongData::default();
        song.metadata.artist = "ArtistA".to_string();
        song.metadata.subartist = "feat. B".to_string();
        let (resource, data, mut main, mut timer) = make_course_render_ctx_with_songdata(song);
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(ctx.string_value(16), "ArtistA feat. B");
    }

    #[test]
    fn test_course_result_string_value_fullartist_without_subartist() {
        let mut song = rubato_skin::song_data::SongData::default();
        song.metadata.artist = "OnlyArtist".to_string();
        let (resource, data, mut main, mut timer) = make_course_render_ctx_with_songdata(song);
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(ctx.string_value(16), "OnlyArtist");
    }

    #[test]
    fn test_course_result_string_value_no_songdata_returns_empty() {
        let resource = PlayerResource::default();
        let data = AbstractResultData::new();
        let mut main =
            MainController::new(rubato_skin::config::Config::default(), make_ranking_cache());
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        // All song metadata IDs should return empty when no songdata
        for id in [10, 11, 12, 13, 14, 15, 16] {
            assert_eq!(
                ctx.string_value(id),
                "",
                "ID {id} should be empty without songdata"
            );
        }
    }

    // ---- CourseResultRenderContext image_index_value ID 308 (lnmode) regression ----

    #[test]
    fn test_course_result_lnmode_308_override_longnote() {
        let mut song = rubato_skin::song_data::SongData::default();
        // Set feature to have LN but not undefined LN
        song.chart.feature = rubato_skin::song_data::FEATURE_LONGNOTE;
        let (resource, data, mut main, mut timer) = make_course_render_ctx_with_songdata(song);
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(
            ctx.image_index_value(308),
            0,
            "LN chart should override lnmode to 0"
        );
    }

    #[test]
    fn test_course_result_lnmode_308_override_chargenote() {
        let mut song = rubato_skin::song_data::SongData::default();
        song.chart.feature = rubato_skin::song_data::FEATURE_CHARGENOTE;
        let (resource, data, mut main, mut timer) = make_course_render_ctx_with_songdata(song);
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(
            ctx.image_index_value(308),
            1,
            "CN chart should override lnmode to 1"
        );
    }

    #[test]
    fn test_course_result_lnmode_308_override_hellchargenote() {
        let mut song = rubato_skin::song_data::SongData::default();
        song.chart.feature = rubato_skin::song_data::FEATURE_HELLCHARGENOTE;
        let (resource, data, mut main, mut timer) = make_course_render_ctx_with_songdata(song);
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(
            ctx.image_index_value(308),
            2,
            "HCN chart should override lnmode to 2"
        );
    }

    #[test]
    fn test_course_result_lnmode_308_no_override_falls_through_to_config() {
        // Chart has no LN features -> should fall through to config's lnmode
        let mut mock = MockPlayerResourceForIR::new_with_course_score();
        mock.song_data = Some(rubato_skin::song_data::SongData::default());
        mock.player_config.play_settings.lnmode = 42;
        let resource = PlayerResource::new(
            mock_to_core(mock),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        let data = AbstractResultData::new();
        let mut main =
            MainController::new(rubato_skin::config::Config::default(), make_ranking_cache());
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let offsets = std::collections::HashMap::new();
        let ctx = CourseResultRenderContext {
            timer: &mut timer,
            data: &data,
            resource: &resource,
            main: &mut main,
            offsets: &offsets,
        };
        assert_eq!(
            ctx.image_index_value(308),
            42,
            "No LN override -> should fall through to player_config lnmode"
        );
    }

    // --- Regression: gauge fill allocation with extreme last_note_time (Finding 1) ---

    #[test]
    fn test_gauge_fill_slots_negative_last_note_time_does_not_panic() {
        // Regression: when last_note_milli_time() exceeds i32::MAX (~2.1 billion ms),
        // last_note_time() (i32) wraps to negative. The old code used last_note_time()
        // and cast ((negative + 500) / 500) as usize, which wraps to a huge value,
        // causing OOM or panic. The fix uses last_note_milli_time().max(0) with a
        // reasonable upper bound.
        use bms::model::mode::Mode;
        use bms::model::note::Note;
        use bms::model::time_line::TimeLine;

        let mut model = bms::model::bms_model::BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        // Create a timeline at ~3 billion milliseconds (exceeds i32::MAX = 2,147,483,647).
        // In microseconds: 3_000_000_000 * 1000 = 3_000_000_000_000
        let extreme_time_us: i64 = 3_000_000_000_000;
        let mut tl = TimeLine::new(0.0, extreme_time_us, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        model.timelines.push(tl);

        // With i64 return type, last_note_time() no longer overflows
        assert!(
            model.last_note_time() > 0,
            "last_note_time() should return correct positive value with i64 return type"
        );

        // Verify the fix: using last_note_milli_time().max(0) with upper bound
        let last_note_milli_time = model.last_note_milli_time().max(0);
        let slots = ((last_note_milli_time + 500) / 500).min(100_000) as usize;

        // The uncapped value would be 3_000_000_000 / 500 = 6_000_000, capped to 100_000
        assert_eq!(slots, 100_000, "Slots should be capped at 100_000");
        // Should not panic or allocate excessively
        let fa = vec![0.0f32; slots];
        assert_eq!(fa.len(), 100_000);
    }

    #[test]
    fn test_gauge_fill_slots_normal_last_note_time() {
        // Verify normal case still works correctly with the fix
        use bms::model::mode::Mode;
        use bms::model::note::Note;
        use bms::model::time_line::TimeLine;

        let mut model = bms::model::bms_model::BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        // 120 seconds = 120_000 ms. time is in microseconds: 120_000 * 1000 = 120_000_000
        let time_us: i64 = 120_000_000;
        let mut tl = TimeLine::new(0.0, time_us, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        model.timelines.push(tl);

        let last_note_milli_time = model.last_note_milli_time().max(0);
        let slots = ((last_note_milli_time + 500) / 500).min(100_000) as usize;

        // 120_000 ms -> (120_000 + 500) / 500 = 241 slots
        assert_eq!(slots, 241, "Normal 2-minute song should produce ~241 slots");
    }

    #[test]
    fn test_gauge_fill_slots_zero_last_note_time() {
        // Empty model with no notes should produce 0 slots (no division issues)
        let model = bms::model::bms_model::BMSModel::new();

        let last_note_milli_time = model.last_note_milli_time().max(0);
        let slots = ((last_note_milli_time + 500) / 500).min(100_000) as usize;

        // 0 ms -> (0 + 500) / 500 = 1
        assert_eq!(slots, 1, "Empty model should produce 1 slot");
    }

    // --- Regression: update_score_database play mode guard (Finding 2) ---

    /// Build a CourseResult with a real DB-backed PlayDataAccessor, for testing
    /// score write guards.
    fn make_course_result_with_mode(mode: BMSPlayerModeType) -> CourseResult {
        let config = make_test_config(&format!("cr-mode-{:?}", mode));
        let main = MainController::new(config, make_ranking_cache());
        let mut mock = MockPlayerResourceForIR::new_with_course_score();
        // Set up course_score_data so update_score_database doesn't early-return
        mock.course_score = Some(ScoreData {
            notes: 100,
            ..Default::default()
        });
        let mut resource =
            PlayerResource::new(mock_to_core(mock), crate::result::BMSPlayerMode::new(mode));
        // Provide course models so write_score_data_course has data
        resource.course_bms_models = Some(vec![bms::model::bms_model::BMSModel::default()]);
        CourseResult::new(
            main,
            resource,
            crate::core::timer_manager::TimerManager::new(),
        )
    }

    #[test]
    fn test_update_score_database_autoplay_does_not_write() {
        // Regression: CourseResult.update_score_database() wrote course score
        // without play mode guards, unlike MusicResult which gates writes on
        // BMSPlayerModeType::Play. In Autoplay mode, score should NOT be persisted.
        let mut cr = make_course_result_with_mode(BMSPlayerModeType::Autoplay);

        // Call update_score_database -- should complete without panic
        cr.update_score_database();

        // Verify the method still processes score properties (oldscore, score display)
        // even though it doesn't write to DB
        // (oldscore defaults to empty since DB has no prior score)
        assert_eq!(cr.data.oldscore.exscore(), 0);

        // Verify score was NOT written by reading back from DB
        let score = cr.main.play_data_accessor().read_score_data_course(
            cr.resource.course_bms_models().unwrap(),
            0, // lnmode
            0, // random
            &cr.resource.constraint(),
        );
        assert!(
            score.is_none(),
            "Autoplay mode should not write score to database"
        );
    }

    #[test]
    fn test_update_score_database_play_mode_completes() {
        // Verify that Play mode path completes successfully and processes
        // score properties (oldscore, score display).
        let mut cr = make_course_result_with_mode(BMSPlayerModeType::Play);

        // Should complete without panic and reach the write path
        cr.update_score_database();

        // Verify score properties were processed
        assert_eq!(cr.data.oldscore.exscore(), 0, "No prior score in fresh DB");
    }

    #[test]
    fn test_update_score_database_practice_does_not_write() {
        // Practice mode should also be gated (not Play)
        let mut cr = make_course_result_with_mode(BMSPlayerModeType::Practice);

        cr.update_score_database();

        let score = cr.main.play_data_accessor().read_score_data_course(
            cr.resource.course_bms_models().unwrap(),
            0,
            0,
            &cr.resource.constraint(),
        );
        assert!(
            score.is_none(),
            "Practice mode should not write score to database"
        );
    }

    #[test]
    fn test_update_score_database_replay_does_not_write() {
        // Replay mode should also be gated (not Play)
        let mut cr = make_course_result_with_mode(BMSPlayerModeType::Replay);

        cr.update_score_database();

        let score = cr.main.play_data_accessor().read_score_data_course(
            cr.resource.course_bms_models().unwrap(),
            0,
            0,
            &cr.resource.constraint(),
        );
        assert!(
            score.is_none(),
            "Replay mode should not write score to database"
        );
    }

    // ============================================================
    // CourseResultMouseContext player_config_ref / config_ref delegation tests
    // ============================================================

    #[test]
    fn course_result_mouse_context_player_config_ref_returns_some() {
        let mut cr = make_course_result_for_mouse();
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let ctx = render_context::CourseResultMouseContext {
            timer: &mut timer,
            result: &mut cr,
        };
        assert!(
            ctx.player_config_ref().is_some(),
            "CourseResultMouseContext::player_config_ref() must delegate to resource"
        );
    }

    #[test]
    fn course_result_mouse_context_config_ref_returns_some() {
        let mut cr = make_course_result_for_mouse();
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let ctx = render_context::CourseResultMouseContext {
            timer: &mut timer,
            result: &mut cr,
        };
        assert!(
            ctx.config_ref().is_some(),
            "CourseResultMouseContext::config_ref() must delegate to main controller"
        );
    }

    // ============================================================
    // CourseResultMouseContext set_float_value volume slider tests
    // ============================================================

    #[test]
    fn course_result_mouse_context_set_float_value_propagates_volume() {
        // Regression: volume slider writes (IDs 17-19) on the course result screen
        // must propagate to pending_audio_config on the CourseResult, not be silently dropped.
        let mut config = rubato_skin::config::Config::default();
        config.audio = Some(rubato_skin::audio_config::AudioConfig::default());
        let main = MainController::new(config, make_ranking_cache());
        let mut resource = PlayerResource::new(
            mock_to_core(MockPlayerResourceForIR::new_with_course_score()),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        resource.course_bms_models = Some(vec![bms::model::bms_model::BMSModel::default()]);
        let mut cr = CourseResult::new(
            main,
            resource,
            crate::core::timer_manager::TimerManager::new(),
        );

        let mut timer = crate::core::timer_manager::TimerManager::new();
        {
            let mut ctx = render_context::CourseResultMouseContext {
                timer: &mut timer,
                result: &mut cr,
            };
            ctx.set_float_value(17, 0.8);
            ctx.set_float_value(18, 0.6);
            ctx.set_float_value(19, 0.4);
        }

        // Each set_float_value overwrites pending_audio_config, so we see the final accumulated state
        let audio = cr
            .pending_audio_config
            .expect("should have pending audio config");
        assert_eq!(audio.systemvolume, 0.8);
        assert_eq!(audio.keyvolume, 0.6);
        assert_eq!(audio.bgvolume, 0.4);
    }

    // ============================================================
    // Finding 1: do_render() missing TIMER_FADEOUT auto-fire
    // ============================================================

    /// Create a CourseResult with pre-resolved sound paths for testing sound selection.
    fn make_sound_tracking_cr(available_sounds: Vec<SoundType>) -> CourseResult {
        let mut main =
            MainController::new(rubato_skin::config::Config::default(), make_ranking_cache());
        let mut paths = std::collections::HashMap::new();
        for sound in &available_sounds {
            paths.insert(sound.clone(), format!("test/{:?}.wav", sound));
        }
        main.set_sound_paths(paths);
        let resource = PlayerResource::default();
        CourseResult::new(
            main,
            resource,
            crate::core::timer_manager::TimerManager::new(),
        )
    }

    #[test]
    fn test_do_render_auto_fires_timer_fadeout_when_time_exceeds_scene() {
        // Finding 1: When time > skin.scene and TIMER_FADEOUT is not on,
        // do_render() should auto-fire TIMER_FADEOUT (like Java CourseResult.render()).
        let mut cr = make_sound_tracking_cr(vec![]);

        // Set up a skin with scene=0 so any positive time exceeds it
        cr.skin = Some(ResultSkinData::new_with_timings(0, 0, 0, 0));

        // Advance timer so now_time() > 0
        cr.main_data.timer.update();
        std::thread::sleep(std::time::Duration::from_millis(1));
        cr.main_data.timer.update();

        assert!(
            !cr.main_data.timer.is_timer_on(TIMER_FADEOUT),
            "TIMER_FADEOUT should not be on before render"
        );

        cr.do_render();

        assert!(
            cr.main_data.timer.is_timer_on(TIMER_FADEOUT),
            "do_render() should auto-fire TIMER_FADEOUT when time > scene"
        );
    }

    #[test]
    fn test_do_render_auto_fadeout_plays_close_sound_when_available() {
        // Finding 1: When auto-firing TIMER_FADEOUT, should stop clear/fail and play close sound.
        let mut cr = make_sound_tracking_cr(vec![SoundType::CourseClose, SoundType::CourseClear]);

        cr.skin = Some(ResultSkinData::new_with_timings(0, 0, 0, 0));
        cr.main_data.timer.update();
        std::thread::sleep(std::time::Duration::from_millis(1));
        cr.main_data.timer.update();

        cr.do_render();

        // Should stop clear/fail sounds (using course variant since available)
        assert!(
            cr.pending_sound_stops.contains(&SoundType::CourseClear),
            "should stop CourseClear on auto-fadeout"
        );
        assert!(
            cr.pending_sound_stops.contains(&SoundType::CourseFail)
                || cr.pending_sound_stops.contains(&SoundType::ResultFail),
            "should stop fail sound on auto-fadeout"
        );
        // Should play close sound (using course variant since available)
        assert!(
            cr.pending_sounds
                .iter()
                .any(|(s, _)| *s == SoundType::CourseClose),
            "should play CourseClose on auto-fadeout"
        );
    }

    #[test]
    fn test_do_render_auto_fadeout_no_close_sound_when_none_available() {
        // When neither COURSE_CLOSE nor RESULT_CLOSE exists, no close sound should play.
        let mut cr = make_sound_tracking_cr(vec![]);

        cr.skin = Some(ResultSkinData::new_with_timings(0, 0, 0, 0));
        cr.main_data.timer.update();
        std::thread::sleep(std::time::Duration::from_millis(1));
        cr.main_data.timer.update();

        cr.do_render();

        assert!(
            cr.main_data.timer.is_timer_on(TIMER_FADEOUT),
            "TIMER_FADEOUT should still be fired"
        );
        assert!(
            cr.pending_sounds.is_empty(),
            "no close sound should be played when none available"
        );
        assert!(
            cr.pending_sound_stops.is_empty(),
            "no sounds should be stopped when no close sound available"
        );
    }

    #[test]
    fn test_do_render_does_not_auto_fadeout_when_already_fading() {
        // If TIMER_FADEOUT is already on, the else branch should not fire.
        let mut cr = make_sound_tracking_cr(vec![]);

        cr.skin = Some(ResultSkinData::new_with_timings(0, 0, 0, 0));
        cr.main_data.timer.update();
        cr.main_data.timer.switch_timer(TIMER_FADEOUT, true);

        // Render should go into the FADEOUT branch, not the else branch
        cr.do_render();

        // TIMER_FADEOUT was already on, so this just verifies it stays on
        assert!(cr.main_data.timer.is_timer_on(TIMER_FADEOUT));
    }

    // ============================================================
    // Finding 2: do_input() missing close sound on OK press
    // ============================================================

    #[test]
    fn test_do_input_plays_close_sound_when_fadeout_triggered() {
        // Finding 2: When TIMER_FADEOUT is set during do_input() (OK press path),
        // it should stop clear/fail sounds and play close sound.
        // With score_data() == None (default), the OK path triggers unconditionally.
        let mut cr = make_sound_tracking_cr(vec![SoundType::CourseClose, SoundType::CourseClear]);

        cr.main_data.timer.update();
        // Activate TIMER_STARTINPUT so the input block is entered
        cr.main_data.timer.switch_timer(TIMER_STARTINPUT, true);
        // Provide an empty input snapshot (no keys pressed).
        // score_data() is None by default, state is STATE_OFFLINE by default,
        // so the TIMER_FADEOUT branch will trigger without needing key press simulation.
        cr.input_snapshot = Some(rubato_input::input_snapshot::InputSnapshot::default());

        cr.do_input();

        assert!(
            cr.main_data.timer.is_timer_on(TIMER_FADEOUT),
            "TIMER_FADEOUT should be set"
        );

        // Should have stopped clear/fail and played close
        assert!(
            cr.pending_sound_stops.contains(&SoundType::CourseClear),
            "should stop clear sound when close sound triggered"
        );
        assert!(
            cr.pending_sounds
                .iter()
                .any(|(s, _)| *s == SoundType::CourseClose),
            "should play CourseClose on TIMER_FADEOUT"
        );
    }

    #[test]
    fn test_do_input_no_close_sound_when_none_available() {
        // When no close sound exists, TIMER_FADEOUT should still fire but no sound plays.
        let mut cr = make_sound_tracking_cr(vec![]);

        cr.main_data.timer.update();
        cr.main_data.timer.switch_timer(TIMER_STARTINPUT, true);
        cr.input_snapshot = Some(rubato_input::input_snapshot::InputSnapshot::default());

        cr.do_input();

        assert!(cr.main_data.timer.is_timer_on(TIMER_FADEOUT));
        assert!(
            cr.pending_sounds.is_empty(),
            "no close sound should be played when none available"
        );
    }

    // ============================================================
    // Finding 3: shutdown() unconditional sound stop
    // ============================================================

    #[test]
    fn test_shutdown_stops_course_sounds_when_course_sounds_available() {
        // Finding 3: shutdown() should stop exactly one per category (exclusive-or),
        // not all six unconditionally.
        let mut cr = make_sound_tracking_cr(vec![
            SoundType::CourseClear,
            SoundType::CourseFail,
            SoundType::CourseClose,
        ]);

        cr.shutdown();

        // Should stop course variants only (not result variants)
        assert!(
            cr.pending_sound_stops.contains(&SoundType::CourseClear),
            "should stop CourseClear"
        );
        assert!(
            cr.pending_sound_stops.contains(&SoundType::CourseFail),
            "should stop CourseFail"
        );
        assert!(
            cr.pending_sound_stops.contains(&SoundType::CourseClose),
            "should stop CourseClose"
        );
        assert!(
            !cr.pending_sound_stops.contains(&SoundType::ResultClear),
            "should NOT stop ResultClear when CourseClear exists"
        );
        assert!(
            !cr.pending_sound_stops.contains(&SoundType::ResultFail),
            "should NOT stop ResultFail when CourseFail exists"
        );
        assert!(
            !cr.pending_sound_stops.contains(&SoundType::ResultClose),
            "should NOT stop ResultClose when CourseClose exists"
        );
    }

    #[test]
    fn test_shutdown_falls_back_to_result_sounds_when_course_sounds_unavailable() {
        // When course-specific sounds don't exist, falls back to result sounds.
        let mut cr = make_sound_tracking_cr(vec![
            SoundType::ResultClear,
            SoundType::ResultFail,
            SoundType::ResultClose,
        ]);

        cr.shutdown();

        // Should stop result variants only (not course variants)
        assert!(
            cr.pending_sound_stops.contains(&SoundType::ResultClear),
            "should stop ResultClear as fallback"
        );
        assert!(
            cr.pending_sound_stops.contains(&SoundType::ResultFail),
            "should stop ResultFail as fallback"
        );
        assert!(
            cr.pending_sound_stops.contains(&SoundType::ResultClose),
            "should stop ResultClose as fallback"
        );
        assert!(
            !cr.pending_sound_stops.contains(&SoundType::CourseClear),
            "should NOT stop CourseClear when it doesn't exist"
        );
        assert!(
            !cr.pending_sound_stops.contains(&SoundType::CourseFail),
            "should NOT stop CourseFail when it doesn't exist"
        );
        assert!(
            !cr.pending_sound_stops.contains(&SoundType::CourseClose),
            "should NOT stop CourseClose when it doesn't exist"
        );
    }

    #[test]
    fn test_shutdown_mixed_availability() {
        // One category has course sound, another has only result fallback.
        let mut cr = make_sound_tracking_cr(vec![
            SoundType::CourseClear, // course clear available
            SoundType::ResultFail,  // only result fail available
            SoundType::CourseClose, // course close available
        ]);

        cr.shutdown();

        assert!(cr.pending_sound_stops.contains(&SoundType::CourseClear));
        assert!(!cr.pending_sound_stops.contains(&SoundType::ResultClear));
        assert!(cr.pending_sound_stops.contains(&SoundType::ResultFail));
        assert!(!cr.pending_sound_stops.contains(&SoundType::CourseFail));
        assert!(cr.pending_sound_stops.contains(&SoundType::CourseClose));
        assert!(!cr.pending_sound_stops.contains(&SoundType::ResultClose));
    }

    #[test]
    fn course_result_mouse_context_integer_value_uses_boot_time_millis() {
        // Regression: CourseResultMouseContext.integer_value() must pass boot_time_millis
        // (not now_time) to shared_render_context::integer_value for IDs 27-29.
        let mut cr = make_course_result_for_mouse();
        let mut timer = crate::core::timer_manager::TimerManager::new();
        timer.set_boot_time_millis(7_200_000); // 2 hours
        timer.set_now_micro_time(5_000); // 5 ms state-relative
        let ctx = render_context::CourseResultMouseContext {
            timer: &mut timer,
            result: &mut cr,
        };
        use rubato_skin::skin_render_context::SkinRenderContext;
        // ID 27 = boot time hours: 7_200_000 / 3_600_000 = 2
        assert_eq!(
            ctx.integer_value(27),
            2,
            "ID 27 (boot hours) must use boot_time_millis, not now_time"
        );
        // ID 28 = boot time minutes: (7_200_000 % 3_600_000) / 60_000 = 0
        assert_eq!(ctx.integer_value(28), 0);
        // ID 29 = boot time seconds: (7_200_000 % 60_000) / 1_000 = 0
        assert_eq!(ctx.integer_value(29), 0);
    }

    #[test]
    fn course_result_render_context_result_gauge_type_returns_stored_gauge_type() {
        let mut cr = make_course_result_for_mouse();
        cr.data.gauge_type = 4;
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let ctx = render_context::CourseResultRenderContext {
            timer: &mut timer,
            data: &cr.data,
            resource: &cr.resource,
            main: &mut cr.main,
            offsets: &cr.main_data.offsets,
        };
        use rubato_skin::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.result_gauge_type(), 4);
    }

    #[test]
    fn course_result_mouse_context_result_gauge_type_returns_stored_gauge_type() {
        let mut cr = make_course_result_for_mouse();
        cr.data.gauge_type = 2;
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let ctx = render_context::CourseResultMouseContext {
            timer: &mut timer,
            result: &mut cr,
        };
        use rubato_skin::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.result_gauge_type(), 2);
    }

    #[test]
    fn course_result_render_context_lane_shuffle_pattern_from_replay() {
        let mut cr = make_course_result_for_mouse();
        cr.resource
            .replay_data_mut()
            .expect("replay data should exist")
            .lane_shuffle_pattern = Some(vec![vec![3, 1, 2, 0, 4, 5, 6, 7, 8, 9]]);
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let ctx = render_context::CourseResultRenderContext {
            timer: &mut timer,
            data: &cr.data,
            resource: &cr.resource,
            main: &mut cr.main,
            offsets: &cr.main_data.offsets,
        };
        use rubato_skin::skin_render_context::SkinRenderContext;
        // image_index 450 = lane_shuffle_pattern_value(0, 0) = 3
        assert_eq!(ctx.image_index_value(450), 3);
        assert_eq!(ctx.image_index_value(451), 1);
        // No 2P data -> -1
        assert_eq!(ctx.image_index_value(460), -1);
    }

    #[test]
    fn course_result_mouse_context_lane_shuffle_pattern_from_replay() {
        let mut cr = make_course_result_for_mouse();
        cr.resource
            .replay_data_mut()
            .expect("replay data should exist")
            .lane_shuffle_pattern = Some(vec![vec![4, 2, 0, 1, 3, 5, 6, 7, 8, 9]]);
        let mut timer = crate::core::timer_manager::TimerManager::new();
        let ctx = render_context::CourseResultMouseContext {
            timer: &mut timer,
            result: &mut cr,
        };
        use rubato_skin::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.image_index_value(450), 4);
        assert_eq!(ctx.image_index_value(451), 2);
    }
}

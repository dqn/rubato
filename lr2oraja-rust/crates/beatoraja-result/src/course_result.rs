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
    BMSPlayerModeType, ControlKeys, IRConnection, IRCourseData, IRScoreData, KeyCommand,
    MainController, PlayerResource, RankingData,
};
use beatoraja_core::ir_config::{IR_SEND_ALWAYS, IR_SEND_COMPLETE_SONG, IR_SEND_UPDATE_SCORE};

/// IR send status for course result
struct CourseIRSendStatus {
    pub ir: Arc<dyn IRConnection + Send + Sync>,
    pub course: beatoraja_core::course_data::CourseData,
    pub lnmode: i32,
    pub score: ScoreData,
    pub retry: i32,
}

impl CourseIRSendStatus {
    pub fn new(
        ir: Arc<dyn IRConnection + Send + Sync>,
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

// ============================================================
// Pure computation helpers extracted from update_score_database
// ============================================================

/// Compute average judge timing when notes > 0.
/// Returns `total_duration / notes`.
#[inline]
fn compute_avgjudge(total_duration: i64, notes: i32) -> i64 {
    total_duration / notes as i64
}

/// Apply avgjudge to a ScoreData in-place, guarding against division by zero.
/// When notes == 0, avgjudge is left unchanged (keeps its default of i64::MAX).
#[inline]
fn apply_avgjudge(score: &mut beatoraja_core::score_data::ScoreData) {
    if score.notes != 0 {
        score.avgjudge = compute_avgjudge(score.total_duration, score.notes);
    }
}

/// Determine the random mode value based on player config options and double-play flag.
///
/// Logic (translated from Java):
/// - Start with random = 0
/// - If random_cfg > 0 OR (dp AND (random2_cfg > 0 OR doubleoption_cfg > 0)): random = 2
/// - If random_cfg == 1 AND (!dp OR (random2_cfg == 1 AND doubleoption_cfg == 1)): random = 1
fn determine_random_mode(
    random_cfg: i32,
    random2_cfg: i32,
    doubleoption_cfg: i32,
    dp: bool,
) -> i32 {
    let mut random = 0;
    if random_cfg > 0 || (dp && (random2_cfg > 0 || doubleoption_cfg > 0)) {
        random = 2;
    }
    if random_cfg == 1 && (!dp || (random2_cfg == 1 && doubleoption_cfg == 1)) {
        random = 1;
    }
    random
}

/// Check if any course BMS model uses double-play mode (player count == 2).
fn is_double_play(models: &[bms_model::bms_model::BMSModel]) -> bool {
    models
        .iter()
        .any(|m| m.get_mode().map(|mode| mode.player()).unwrap_or(1) == 2)
}

/// Sum total notes across all course BMS models.
fn aggregate_total_notes(models: &[bms_model::bms_model::BMSModel]) -> i32 {
    models.iter().map(|m| m.get_total_notes()).sum()
}

/// Course result screen
pub struct CourseResult {
    pub data: AbstractResultData,
    pub main_data: beatoraja_core::main_state::MainStateData,
    pub main: MainController,
    pub resource: PlayerResource,
    ir_send_status: Vec<CourseIRSendStatus>,
    property: ResultKeyProperty,
    skin: Option<CourseResultSkin>,
}

impl CourseResult {
    pub fn new(
        main: MainController,
        resource: PlayerResource,
        timer: beatoraja_core::timer_manager::TimerManager,
    ) -> Self {
        Self {
            data: AbstractResultData::new(),
            main_data: beatoraja_core::main_state::MainStateData::new(timer),
            main,
            resource,
            ir_send_status: Vec::new(),
            property: ResultKeyProperty::beat_7k(),
            skin: None,
        }
    }

    fn do_create(&mut self) {
        for i in 0..REPLAY_SIZE {
            let models = self.resource.get_course_bms_models();
            if let Some(models) = models {
                self.data.save_replay[i] = if self
                    .main
                    .get_play_data_accessor()
                    .exists_replay_data_course(
                        models,
                        self.resource.get_player_config().lnmode,
                        i as i32,
                        &self.resource.get_constraint(),
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
        if let Some(models) = self.resource.get_course_bms_models() {
            let course_gauge_size = self.resource.get_course_gauge().len();
            let gauge_type_length = self
                .resource
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
            self.resource.get_course_gauge_mut().push(list);
        }

        if let Some(mode) = self.resource.get_bms_model().get_mode() {
            self.property = ResultKeyProperty::get(mode).unwrap_or_else(ResultKeyProperty::beat_7k);
        } else {
            self.property = ResultKeyProperty::beat_7k();
        }

        self.update_score_database();

        // Replay auto save
        if self.resource.get_play_mode().mode == BMSPlayerModeType::Play {
            for i in 0..REPLAY_SIZE {
                let auto_save = &self.resource.get_player_config().autosavereplay;
                if i < auto_save.len()
                    && let Some(new_score) = self.resource.get_course_score_data()
                    && ReplayAutoSaveConstraint::get(auto_save[i])
                        .is_qualified(&self.data.oldscore, new_score)
                {
                    self.save_replay_data(i);
                }
            }
        }

        self.data.gauge_type = self
            .resource
            .get_groove_gauge()
            .map(|g| g.get_type())
            .unwrap_or(0);

        // loadSkin(SkinType.COURSE_RESULT)
        beatoraja_core::main_state::MainState::load_skin(
            self,
            beatoraja_skin::skin_type::SkinType::CourseResult.id(),
        );
    }

    fn do_prepare(&mut self) {
        self.data.state = STATE_OFFLINE;
        let newscore = self.resource.get_course_score_data().cloned();

        self.data.ranking = if self.resource.get_ranking_data().is_some()
            && self.resource.get_course_bms_models().is_some()
        {
            self.resource.get_ranking_data().cloned()
        } else {
            Some(RankingData::new())
        };
        self.data.ranking_offset = 0;

        let ir = self.main.get_ir_status();
        if !ir.is_empty() && self.resource.get_play_mode().mode == BMSPlayerModeType::Play {
            self.data.state = STATE_IR_PROCESSING;

            let mut uln = false;
            if let Some(models) = self.resource.get_course_bms_models() {
                for model in models {
                    if model.contains_undefined_long_note() {
                        uln = true;
                        break;
                    }
                }
            }
            let lnmode = if uln {
                self.resource.get_player_config().lnmode
            } else {
                0
            };

            for irc in ir {
                let send = self.resource.is_update_course_score()
                    && !self.resource.is_force_no_ir_send()
                    && self
                        .resource
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
                    && let Some(course_data) = self.resource.get_course_data()
                {
                    self.ir_send_status.push(CourseIRSendStatus::new(
                        irc.connection.clone(),
                        course_data,
                        lnmode,
                        ns,
                    ));
                }
            }

            // IR processing in background thread (Java spawns a Thread)
            let ir_send_count = self.main.get_config().ir_send_count;
            if !self.ir_send_status.is_empty() {
                self.data
                    .timer
                    .switch_timer(beatoraja_skin::skin_property::TIMER_IR_CONNECT_BEGIN, true);
            }

            // Move statuses into the thread
            let mut statuses = std::mem::take(&mut self.ir_send_status);
            let ir_connection = self
                .main
                .get_ir_status()
                .first()
                .map(|s| s.connection.clone());
            let course_data_for_ranking = self.resource.get_course_data().cloned();
            let oldscore_exscore = self.data.oldscore.get_exscore();
            let newscore_clone = newscore.clone();

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let mut succeed = true;
                let mut irsend = 0;
                let mut remove_indices: Vec<usize> = Vec::new();
                for idx in 0..statuses.len() {
                    irsend += 1;
                    let send_ok = statuses[idx].send();
                    succeed &= send_ok;
                    if statuses[idx].retry < 0 || statuses[idx].retry > ir_send_count {
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
                        ranking_result = response.get_data().cloned();
                        info!("IR score fetch succeeded: {}", response.get_message());
                    } else {
                        warn!("IR score fetch failed: {}", response.get_message());
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

            // Block briefly to receive results (matches Java's thread.join() behavior)
            if let Ok((succeed, had_sends, ranking_scores, ns_clone, old_exscore)) = rx.recv()
                && had_sends
            {
                if succeed {
                    self.data.timer.switch_timer(
                        beatoraja_skin::skin_property::TIMER_IR_CONNECT_SUCCESS,
                        true,
                    );
                } else {
                    self.data
                        .timer
                        .switch_timer(beatoraja_skin::skin_property::TIMER_IR_CONNECT_FAIL, true);
                }
                if let Some(ir_scores) = ranking_scores {
                    let use_newscore = ns_clone
                        .as_ref()
                        .map(|ns| ns.get_exscore() > old_exscore)
                        .unwrap_or(false);
                    let score_for_rank: Option<&beatoraja_core::score_data::ScoreData> =
                        if use_newscore {
                            ns_clone.as_ref()
                        } else {
                            Some(&self.data.oldscore)
                        };
                    if let Some(ref mut ranking) = self.data.ranking {
                        ranking.update_score(&ir_scores, score_for_rank);
                        if ranking.get_rank() > 10 {
                            self.data.ranking_offset = ranking.get_rank() - 5;
                        } else {
                            self.data.ranking_offset = 0;
                        }
                    }
                }
            }
            self.data.state = STATE_IR_FINISHED;
        }

        // Play result sound
        if let Some(ref ns) = newscore {
            let is_clear = ns.clear != ClearType::Failed.id();
            let loop_sound = self
                .resource
                .get_config()
                .audio
                .as_ref()
                .map(|ac| ac.is_loop_course_result_sound)
                .unwrap_or(false);
            if is_clear {
                let sound = if self.main.get_sound_path(&SoundType::CourseClear).is_some() {
                    SoundType::CourseClear
                } else {
                    SoundType::ResultClear
                };
                self.main.play_sound(&sound, loop_sound);
            } else {
                let sound = if self.main.get_sound_path(&SoundType::CourseFail).is_some() {
                    SoundType::CourseFail
                } else {
                    SoundType::ResultFail
                };
                self.main.play_sound(&sound, loop_sound);
            }
        }
    }

    /// Stop all course result sounds.
    ///
    /// Translated from: CourseResult.shutdown()
    /// Stops course-specific sounds if available, otherwise falls back to result sounds.
    pub fn shutdown(&mut self) {
        // Java: stop(getSound(COURSE_CLEAR) != null ? COURSE_CLEAR : RESULT_CLEAR)
        self.stop_sound_inner(SoundType::CourseClear);
        self.stop_sound_inner(SoundType::ResultClear);
        // Java: stop(getSound(COURSE_FAIL) != null ? COURSE_FAIL : RESULT_FAIL)
        self.stop_sound_inner(SoundType::CourseFail);
        self.stop_sound_inner(SoundType::ResultFail);
        // Java: stop(getSound(COURSE_CLOSE) != null ? COURSE_CLOSE : RESULT_CLOSE)
        self.stop_sound_inner(SoundType::CourseClose);
        self.stop_sound_inner(SoundType::ResultClose);
    }

    fn stop_sound_inner(&mut self, sound: SoundType) {
        self.main.stop_sound(&sound);
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

                self.main
                    .change_state(beatoraja_core::main_state::MainStateType::MusicSelect);
            }
        }
    }

    fn do_input(&mut self) {
        self.data.input(&mut self.main);

        if !self.data.timer.is_timer_on(TIMER_FADEOUT)
            && self.data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            let mut ok = false;
            for i in 0..self.property.get_assign_length() {
                let input_processor = self.main.get_input_processor();
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
                let input_processor = self.main.get_input_processor();
                if input_processor.is_control_key_pressed(ControlKeys::Escape)
                    || input_processor.is_control_key_pressed(ControlKeys::Enter)
                {
                    ok = true;
                }
            }

            if (self.resource.get_score_data().is_none() || ok)
                && (self.data.state == STATE_OFFLINE || self.data.state == STATE_IR_FINISHED)
            {
                self.data.timer.switch_timer(TIMER_FADEOUT, true);
                // play close sound
            }

            let replay_index = {
                let input_processor = self.main.get_input_processor();
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
                self.save_replay_data(idx);
            }

            let open_ir = {
                let input_processor = self.main.get_input_processor();
                input_processor.is_activated(KeyCommand::OpenIr)
            };
            if open_ir
                && let Some(ir_status) = self.main.get_ir_status().first()
                && let Some(coursedata) = self.resource.get_course_data()
            {
                let course = beatoraja_ir::ir_course_data::IRCourseData::new(coursedata);
                if let Some(url) = ir_status.connection.get_course_url(&course)
                    && let Err(e) = open::that(&url)
                {
                    log::error!("Failed to open IR URL: {}", e);
                }
            }
        }
    }

    fn update_score_database(&mut self) {
        let lnmode = self.resource.get_player_config().lnmode;
        let random_cfg = self.resource.get_player_config().random;
        let random2_cfg = self.resource.get_player_config().random2;
        let doubleoption_cfg = self.resource.get_player_config().doubleoption;
        let newscore = self.resource.get_course_score_data().cloned();
        if newscore.is_none() {
            return;
        }
        let mut newscore = newscore.unwrap();

        let dp = self
            .resource
            .get_course_bms_models()
            .map(is_double_play)
            .unwrap_or(false);

        newscore.maxcombo = self.resource.get_maxcombo();
        apply_avgjudge(&mut newscore);

        let random = determine_random_mode(random_cfg, random2_cfg, doubleoption_cfg, dp);

        if let Some(models) = self.resource.get_course_bms_models() {
            let score = self.main.get_play_data_accessor().read_score_data_course(
                models,
                lnmode,
                random,
                &self.resource.get_constraint(),
            );
            self.data.oldscore = score.unwrap_or_default();
        }

        let target_exscore = self
            .resource
            .get_target_score_data()
            .map(|s| s.get_exscore())
            .unwrap_or(0);
        let total_notes: i32 = self
            .resource
            .get_course_bms_models()
            .map(aggregate_total_notes)
            .unwrap_or(0);
        self.data.score.set_target_score(
            self.data.oldscore.get_exscore(),
            target_exscore,
            total_notes,
        );
        self.data.score.update_score(Some(&newscore));

        if let Some(models) = self.resource.get_course_bms_models() {
            self.main.get_play_data_accessor().write_score_data_course(
                &newscore,
                models,
                lnmode,
                random,
                &self.resource.get_constraint(),
                self.resource.is_update_course_score(),
            );
        }

        info!("Score database update complete");
    }

    pub fn get_judge_count(&self, judge: i32, fast: bool) -> i32 {
        if let Some(score) = self.resource.get_course_score_data() {
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

    pub fn save_replay_data(&mut self, index: usize) {
        if self.resource.get_play_mode().mode == BMSPlayerModeType::Play
            && self.resource.get_course_score_data().is_some()
            && self.data.save_replay[index] != ReplayStatus::Saved
            && self.resource.is_update_course_score()
        {
            // Extract gauge value first to avoid borrow conflict
            let gauge = self.resource.get_player_config().gauge;
            let rd = self.resource.get_course_replay_mut();
            for replay in rd.iter_mut() {
                replay.gauge = gauge;
            }
            let lnmode = self.resource.get_player_config().lnmode;
            let constraint = self.resource.get_constraint();
            if let Some(models) = self.resource.get_course_bms_models() {
                // Clone replays for write (write_brd_course calls shrink on each)
                let mut replays = self.resource.get_course_replay().to_vec();
                self.main.get_play_data_accessor().write_replay_data_course(
                    &mut replays,
                    models,
                    lnmode,
                    index as i32,
                    &constraint,
                );
            }
            self.data.save_replay[index] = ReplayStatus::Saved;
            self.main.save_last_recording("ON_REPLAY");
        }
    }

    pub fn get_new_score(&self) -> Option<&ScoreData> {
        self.resource.get_course_score_data()
    }

    pub fn dispose(&mut self) {
        // super.dispose() equivalent
    }
}

impl Default for CourseResult {
    fn default() -> Self {
        use crate::stubs::NullMainController;
        Self::new(
            MainController::new(Box::new(NullMainController)),
            PlayerResource::default(),
            beatoraja_core::timer_manager::TimerManager::new(),
        )
    }
}

// ============================================================
// MainState trait implementation
// ============================================================

// Tests for CourseResult
#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_core::main_state::MainState;
    use beatoraja_skin::skin_property::{TIMER_RESULTGRAPH_BEGIN, TIMER_STARTINPUT};
    use beatoraja_skin::skin_type::SkinType;

    fn make_default() -> CourseResult {
        CourseResult::new(
            MainController::new(Box::new(crate::stubs::NullMainController)),
            PlayerResource::default(),
            beatoraja_core::timer_manager::TimerManager::new(),
        )
    }

    #[test]
    fn test_state_type_returns_course_result() {
        let cr = make_default();
        assert_eq!(
            cr.state_type(),
            Some(beatoraja_core::main_state::MainStateType::CourseResult)
        );
    }

    #[test]
    fn test_create_calls_load_skin_with_course_result_type() {
        // Verify SkinType::CourseResult.id() matches expected value (15)
        assert_eq!(SkinType::CourseResult.id(), 15);
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
        assert!(cr.resource.get_score_data().is_none());
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
        assert_eq!(cr.data.state, crate::abstract_result::STATE_OFFLINE);
    }

    #[test]
    fn test_trait_render_delegates_to_do_render() {
        let mut cr = make_default();
        // do_render switches TIMER_RESULTGRAPH_BEGIN
        assert!(!cr.data.timer.is_timer_on(TIMER_RESULTGRAPH_BEGIN));
        <CourseResult as MainState>::render(&mut cr);
        assert!(cr.data.timer.is_timer_on(TIMER_RESULTGRAPH_BEGIN));
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
            Some(beatoraja_core::main_state::MainStateType::CourseResult)
        );
    }

    // ---- IR processing tests ----

    use crate::ir_status::IRStatus as IRStatusReal;
    use beatoraja_ir::ir_chart_data::IRChartData;
    use beatoraja_ir::ir_course_data::IRCourseData as IRCourseDataReal;
    use beatoraja_ir::ir_player_data::IRPlayerData;
    use beatoraja_ir::ir_response::IRResponse;
    use beatoraja_ir::ir_score_data::IRScoreData as IRScoreDataReal;
    use beatoraja_ir::ir_table_data::IRTableData;
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

    impl crate::stubs::IRConnection for MockCourseIR {
        fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _chart: &IRChartData,
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
        course_score: Option<beatoraja_core::score_data::ScoreData>,
        course_data: Option<beatoraja_core::course_data::CourseData>,
        course_gauge: Vec<Vec<Vec<f32>>>,
        course_replay: Vec<beatoraja_core::replay_data::ReplayData>,
    }

    impl MockPlayerResourceForIR {
        fn new_with_course_score() -> Self {
            let score = beatoraja_core::score_data::ScoreData {
                clear: beatoraja_core::clear_type::ClearType::Easy.id(),
                ..Default::default()
            };
            let course = beatoraja_core::course_data::CourseData {
                name: Some("Test Course".to_string()),
                release: true,
                ..Default::default()
            };
            Self {
                course_score: Some(score),
                course_data: Some(course),
                course_gauge: Vec::new(),
                course_replay: Vec::new(),
            }
        }
    }

    impl beatoraja_types::player_resource_access::PlayerResourceAccess for MockPlayerResourceForIR {
        fn get_config(&self) -> &beatoraja_types::config::Config {
            static CONFIG: std::sync::OnceLock<beatoraja_types::config::Config> =
                std::sync::OnceLock::new();
            CONFIG.get_or_init(beatoraja_types::config::Config::default)
        }
        fn get_player_config(&self) -> &beatoraja_types::player_config::PlayerConfig {
            static PC: std::sync::OnceLock<beatoraja_types::player_config::PlayerConfig> =
                std::sync::OnceLock::new();
            PC.get_or_init(beatoraja_types::player_config::PlayerConfig::default)
        }
        fn get_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
            None
        }
        fn get_rival_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
            None
        }
        fn get_target_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
            None
        }
        fn get_course_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
            self.course_score.as_ref()
        }
        fn set_course_score_data(&mut self, score: beatoraja_core::score_data::ScoreData) {
            self.course_score = Some(score);
        }
        fn get_songdata(&self) -> Option<&beatoraja_types::song_data::SongData> {
            None
        }
        fn get_songdata_mut(&mut self) -> Option<&mut beatoraja_types::song_data::SongData> {
            None
        }
        fn get_replay_data(&self) -> Option<&beatoraja_core::replay_data::ReplayData> {
            None
        }
        fn get_replay_data_mut(&mut self) -> Option<&mut beatoraja_core::replay_data::ReplayData> {
            None
        }
        fn get_course_replay(&self) -> &[beatoraja_core::replay_data::ReplayData] {
            &self.course_replay
        }
        fn add_course_replay(&mut self, rd: beatoraja_core::replay_data::ReplayData) {
            self.course_replay.push(rd);
        }
        fn get_course_data(&self) -> Option<&beatoraja_core::course_data::CourseData> {
            self.course_data.as_ref()
        }
        fn get_course_index(&self) -> usize {
            0
        }
        fn next_course(&mut self) -> bool {
            false
        }
        fn get_constraint(&self) -> Vec<beatoraja_core::course_data::CourseDataConstraint> {
            vec![]
        }
        fn get_gauge(&self) -> Option<&Vec<Vec<f32>>> {
            None
        }
        fn get_groove_gauge(&self) -> Option<&beatoraja_types::groove_gauge::GrooveGauge> {
            None
        }
        fn get_course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
            &self.course_gauge
        }
        fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>) {
            self.course_gauge.push(gauge);
        }
        fn get_course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
            &mut self.course_gauge
        }
        fn get_score_data_mut(&mut self) -> Option<&mut beatoraja_core::score_data::ScoreData> {
            None
        }
        fn get_course_replay_mut(&mut self) -> &mut Vec<beatoraja_core::replay_data::ReplayData> {
            &mut self.course_replay
        }
        fn get_maxcombo(&self) -> i32 {
            0
        }
        fn get_org_gauge_option(&self) -> i32 {
            0
        }
        fn set_org_gauge_option(&mut self, _val: i32) {}
        fn get_assist(&self) -> i32 {
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
        fn get_reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }
        fn get_reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }
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
            _score: Option<beatoraja_core::score_data::ScoreData>,
        ) {
        }
        fn set_chart_option_data(
            &mut self,
            _data: Option<beatoraja_core::replay_data::ReplayData>,
        ) {
        }
        fn set_course_data(&mut self, _data: beatoraja_core::course_data::CourseData) {}
        fn clear_course_data(&mut self) {}
        fn set_songdata(&mut self, _data: Option<beatoraja_types::song_data::SongData>) {}
        fn get_course_song_data(&self) -> Vec<beatoraja_types::song_data::SongData> {
            vec![]
        }
    }

    fn make_ir_course_result(
        ir_conn: Arc<dyn crate::stubs::IRConnection + Send + Sync>,
    ) -> CourseResult {
        use beatoraja_core::ir_config::IRConfig;
        let ir_status = IRStatusReal::new(
            IRConfig::default(),
            ir_conn,
            IRPlayerData::new(String::new(), String::new(), String::new()),
        );
        let main = MainController::with_ir_statuses(
            Box::new(crate::stubs::NullMainController),
            vec![ir_status],
        );
        let resource = PlayerResource::new(
            Box::new(MockPlayerResourceForIR::new_with_course_score()),
            crate::stubs::BMSPlayerMode::new(crate::stubs::BMSPlayerModeType::Play),
        );
        CourseResult::new(
            main,
            resource,
            beatoraja_core::timer_manager::TimerManager::new(),
        )
    }

    #[test]
    fn test_prepare_with_ir_transitions_to_ir_finished() {
        let ir_conn = Arc::new(MockCourseIR::new(true));
        let mut cr = make_ir_course_result(ir_conn.clone());

        <CourseResult as MainState>::prepare(&mut cr);

        // IR processing should complete and set state to STATE_IR_FINISHED
        assert_eq!(cr.data.state, crate::abstract_result::STATE_IR_FINISHED);
    }

    #[test]
    fn test_prepare_with_ir_sends_course_score() {
        let ir_conn = Arc::new(MockCourseIR::new(true));
        let mut cr = make_ir_course_result(ir_conn.clone());

        <CourseResult as MainState>::prepare(&mut cr);

        // The IR send should have been called
        assert!(ir_conn.send_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_prepare_with_ir_fetches_ranking() {
        let ir_conn = Arc::new(MockCourseIR::new(true));
        let mut cr = make_ir_course_result(ir_conn.clone());

        <CourseResult as MainState>::prepare(&mut cr);

        // After sending, ranking should be fetched via get_course_play_data
        assert!(ir_conn.ranking_fetch_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_prepare_with_ir_send_failure_still_finishes() {
        let ir_conn = Arc::new(MockCourseIR::new(false));
        let mut cr = make_ir_course_result(ir_conn.clone());

        <CourseResult as MainState>::prepare(&mut cr);

        // Even with send failure, state should transition to IR_FINISHED
        assert_eq!(cr.data.state, crate::abstract_result::STATE_IR_FINISHED);
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
        let mut score = beatoraja_core::score_data::ScoreData::default();
        score.total_duration = 1000;
        score.notes = 0;
        apply_avgjudge(&mut score);
        assert_eq!(score.avgjudge, i64::MAX); // unchanged from default
    }

    #[test]
    fn test_compute_avgjudge_with_nonzero_notes_updates_score() {
        let mut score = beatoraja_core::score_data::ScoreData::default();
        score.total_duration = 5000;
        score.notes = 50;
        apply_avgjudge(&mut score);
        assert_eq!(score.avgjudge, 100);
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
        let models: Vec<bms_model::bms_model::BMSModel> = vec![];
        assert!(!is_double_play(&models));
    }

    #[test]
    fn test_is_double_play_single_player_model() {
        // Mode::BEAT_7K has player() == 1
        let mut model = bms_model::bms_model::BMSModel::default();
        model.set_mode(bms_model::mode::Mode::BEAT_7K);
        assert!(!is_double_play(&[model]));
    }

    #[test]
    fn test_is_double_play_double_player_model() {
        // Mode::BEAT_14K has player() == 2
        let mut model = bms_model::bms_model::BMSModel::default();
        model.set_mode(bms_model::mode::Mode::BEAT_14K);
        assert!(is_double_play(&[model]));
    }

    #[test]
    fn test_is_double_play_mixed_models() {
        // One single, one double -> dp = true (OR logic)
        let mut m1 = bms_model::bms_model::BMSModel::default();
        m1.set_mode(bms_model::mode::Mode::BEAT_7K);
        let mut m2 = bms_model::bms_model::BMSModel::default();
        m2.set_mode(bms_model::mode::Mode::BEAT_14K);
        assert!(is_double_play(&[m1, m2]));
    }

    #[test]
    fn test_is_double_play_no_mode_set() {
        // Model with no mode -> get_mode() returns None, unwrap_or(1) == 1, not dp
        let model = bms_model::bms_model::BMSModel::default();
        assert!(!is_double_play(&[model]));
    }

    #[test]
    fn test_aggregate_total_notes_empty() {
        let models: Vec<bms_model::bms_model::BMSModel> = vec![];
        assert_eq!(aggregate_total_notes(&models), 0);
    }

    #[test]
    fn test_aggregate_total_notes_single_model() {
        // BMSModel::default() has 0 total notes
        let model = bms_model::bms_model::BMSModel::default();
        assert_eq!(aggregate_total_notes(&[model]), 0);
    }
}

impl beatoraja_core::main_state::MainState for CourseResult {
    fn state_type(&self) -> Option<beatoraja_core::main_state::MainStateType> {
        Some(beatoraja_core::main_state::MainStateType::CourseResult)
    }

    fn main_state_data(&self) -> &beatoraja_core::main_state::MainStateData {
        &self.main_data
    }

    fn main_state_data_mut(&mut self) -> &mut beatoraja_core::main_state::MainStateData {
        &mut self.main_data
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

    fn input(&mut self) {
        self.do_input();
    }

    fn shutdown(&mut self) {
        self.shutdown();
    }

    fn dispose(&mut self) {
        self.dispose();
    }
}

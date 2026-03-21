// MusicResult.java -> music_result.rs
// Mechanical line-by-line translation.

use log::info;

use rubato_core::clear_type::ClearType;
use rubato_core::main_state::{MainState, MainStateData, MainStateType};
use rubato_core::score_data::ScoreData;
use rubato_core::system_sound_manager::SoundType;
use rubato_core::timer_manager::TimerManager;
use rubato_play::groove_gauge;
use rubato_skin::skin_property::*;

use super::abstract_result::{
    AbstractResultData, REPLAY_SIZE, ReplayAutoSaveConstraint, ReplayStatus, STATE_IR_FINISHED,
    STATE_IR_PROCESSING, STATE_OFFLINE,
};
use super::ir_send_status::IRSendStatusMain;
use super::result_key_property::{ResultKey, ResultKeyProperty};
use super::result_skin_data::ResultSkinData;
use super::{
    BMSPlayerModeType, ControlKeys, KeyCommand, MainController, NullMainController, PlayerResource,
    RankingData,
};
use rubato_core::ir_config::{IR_SEND_ALWAYS, IR_SEND_COMPLETE_SONG, IR_SEND_UPDATE_SCORE};

mod render_context;
mod score_handler;

use render_context::*;

/// IR send result for async processing.
type MusicIrResult = (
    bool,
    bool,
    Option<Vec<rubato_ir::ir_score_data::IRScoreData>>,
    Option<ScoreData>,
);

pub struct MusicResult {
    pub data: AbstractResultData,
    pub main_data: MainStateData,
    pub main: MainController,
    pub resource: PlayerResource,
    property: ResultKeyProperty,
    skin: Option<ResultSkinData>,
    /// Receiver for async IR results (non-blocking).
    ir_rx: Option<std::sync::mpsc::Receiver<MusicIrResult>>,
    /// JoinHandle for the IR send background thread.
    ir_thread: Option<std::thread::JoinHandle<()>>,
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
            ir_rx: None,
            ir_thread: None,
        }
    }

    fn do_create(&mut self) {
        for i in 0..REPLAY_SIZE {
            self.data.save_replay[i] = if self.main.play_data_accessor().exists_replay_data_model(
                self.resource.bms_model(),
                self.resource.player_config().play_settings.lnmode,
                i as i32,
            ) {
                ReplayStatus::Exist
            } else {
                ReplayStatus::NotExist
            };
        }

        if let Some(mode) = self.resource.bms_model().mode() {
            self.property = ResultKeyProperty::get(mode).unwrap_or_else(ResultKeyProperty::beat_7k);
        } else {
            self.property = ResultKeyProperty::beat_7k();
        }

        self.update_score_database();

        // Replay auto save
        if self.resource.play_mode().mode == BMSPlayerModeType::Play && !self.resource.is_freq_on()
        {
            for i in 0..REPLAY_SIZE {
                let auto_save = &self.resource.player_config().misc_settings.autosavereplay;
                if i < auto_save.len()
                    && let Some(score_data) = self.resource.score_data()
                    && ReplayAutoSaveConstraint::get(auto_save[i])
                        .is_qualified(&self.data.oldscore, score_data)
                {
                    self.save_replay_data(i);
                }
            }
        }

        // Stock replay data for course mode
        if self.resource.course_bms_models().is_some() {
            if let Some(replay_clone) = self.resource.replay_data().cloned() {
                self.resource.add_course_replay(replay_clone);
            }
            if let Some(gauge) = self.resource.gauge() {
                let gauge_clone = gauge.clone();
                self.resource.add_course_gauge(gauge_clone);
            }
        }

        self.data.gauge_type = super::result_common::set_gauge_type(&self.resource);

        // loadSkin(SkinType.RESULT)
        self.load_skin(rubato_skin::skin_type::SkinType::Result.id());
    }

    fn do_prepare(&mut self) {
        self.data.state = STATE_OFFLINE;
        let newscore_clone = self.new_score().cloned();

        self.data.ranking = if self.resource.ranking_data().is_some()
            && self.resource.course_bms_models().is_none()
        {
            self.resource.ranking_data().cloned()
        } else {
            Some(RankingData::new())
        };
        self.data.ranking_offset = 0;

        let ir = self.main.ir_status();
        if !ir.is_empty()
            && self.resource.play_mode().mode == BMSPlayerModeType::Play
            && !self.resource.is_freq_on()
        {
            self.data.state = STATE_IR_PROCESSING;

            let mut pending_ir_sends: Vec<IRSendStatusMain> = Vec::new();
            for irc in ir {
                let mut send =
                    self.resource.is_update_score() && !self.resource.is_force_no_ir_send();
                match irc.config.irsend {
                    IR_SEND_ALWAYS => {}
                    IR_SEND_COMPLETE_SONG => {
                        if let Some(groove_gauge) = self.resource.groove_gauge() {
                            send &= groove_gauge.value() > 0.0;
                        }
                    }
                    IR_SEND_UPDATE_SCORE => {
                        if let Some(ref ns) = newscore_clone {
                            send &= ns.exscore() > self.data.oldscore.exscore()
                                || ns.clear > self.data.oldscore.clear
                                || ns.maxcombo > self.data.oldscore.maxcombo
                                || ns.minbp < self.data.oldscore.minbp;
                        }
                    }
                    _ => {}
                }

                if send
                    && let Some(ref ns) = newscore_clone
                    && let Some(songdata) = self.resource.songdata()
                {
                    pending_ir_sends.push(IRSendStatusMain::new(
                        irc.connection.clone(),
                        songdata,
                        ns,
                    ));
                }
            }
            // Spawn IR processing thread (sends scores + fetches ranking)
            let ir_send_count = self.main.config().network.ir_send_count;
            let mut ir_send_list_snapshot: Vec<IRSendStatusMain> = pending_ir_sends;
            let ir_connection = self.main.ir_status().first().map(|s| s.connection.clone());
            let songdata_for_ranking = self.resource.songdata().cloned();
            let _oldscore_exscore = self.data.oldscore.exscore();
            let newscore_for_thread = newscore_clone.clone();

            self.data.timer.switch_timer(TIMER_IR_CONNECT_BEGIN, true);

            let (tx, rx) = std::sync::mpsc::channel();
            self.ir_rx = Some(rx);

            let handle = std::thread::spawn(move || {
                let mut irsend = 0;
                let mut succeed = true;
                for status in &mut ir_send_list_snapshot {
                    irsend += 1;
                    let send_ok = status.send();
                    succeed &= send_ok;
                    if status.retry < 0 || status.retry > ir_send_count {
                        // Discard failed sends (matches original removal logic)
                    }
                }

                let mut ranking_scores = None;
                if irsend > 0
                    && let Some(ref conn) = ir_connection
                    && let Some(ref songdata) = songdata_for_ranking
                {
                    let chart_data = rubato_ir::ir_chart_data::IRChartData::new(songdata);
                    let response = conn.get_play_data(None, &chart_data);
                    if response.is_succeeded() {
                        ranking_scores = response.data().cloned();
                        log::info!("IR score fetch succeeded: {}", response.message);
                    } else {
                        log::warn!("IR score fetch failed: {}", response.message);
                    }
                }

                let _ = tx.send((succeed, irsend > 0, ranking_scores, newscore_for_thread));
            });
            self.ir_thread = Some(handle);
        }

        // Play result sound
        if let Some(ref ns) = newscore_clone {
            let cscore = self.resource.course_score_data();
            let is_clear = ns.clear != ClearType::Failed.id()
                && cscore.is_none_or(|cs| cs.clear != ClearType::Failed.id());
            let loop_sound = self
                .resource
                .config()
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

        // Detach the IR send thread if it is still running.
        // The thread is self-bounded (sends scores + fetches ranking, then exits),
        // so it is safe to let it run past the result screen exit.
        // We avoid blocking the render thread with sleep-based polling.
        if let Some(handle) = self.ir_thread.take() {
            if handle.is_finished() {
                if let Err(e) = handle.join() {
                    log::warn!("MusicResult IR send thread panicked: {:?}", e);
                }
            } else {
                log::warn!("MusicResult IR send thread still running at shutdown; detaching");
                // Drop the JoinHandle without joining -- thread continues in background.
            }
        }
    }

    /// Poll for async IR results (non-blocking) and update ranking/timer state.
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
        let (succeed, had_sends, ranking_scores, newscore_clone) = result;
        self.data.state = STATE_IR_FINISHED;
        if had_sends {
            if succeed {
                self.data.timer.switch_timer(TIMER_IR_CONNECT_SUCCESS, true);
            } else {
                self.data.timer.switch_timer(TIMER_IR_CONNECT_FAIL, true);
            }
            if let Some(ir_scores) = ranking_scores {
                let use_newscore = newscore_clone
                    .as_ref()
                    .map(|ns| ns.exscore() > self.data.oldscore.exscore())
                    .unwrap_or(false);
                let score_for_rank: Option<&ScoreData> = if use_newscore {
                    newscore_clone.as_ref()
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

    fn do_render(&mut self) {
        // Poll for async IR results (non-blocking)
        self.poll_ir_results();

        let time = self.data.timer.now_time();
        self.data.timer.switch_timer(TIMER_RESULTGRAPH_BEGIN, true);
        self.data.timer.switch_timer(TIMER_RESULTGRAPH_END, true);

        if let Some(ref skin) = self.skin
            && skin.rank_time() == 0
        {
            self.data.timer.switch_timer(TIMER_RESULT_UPDATESCORE, true);
        }
        let skin_input = self.skin.as_ref().map(|s| s.input() as i64).unwrap_or(0);
        if time > skin_input {
            self.data.timer.switch_timer(TIMER_STARTINPUT, true);
        }

        if self.data.timer.is_timer_on(TIMER_FADEOUT) {
            let fadeout_time = self.data.timer.now_time_for_id(TIMER_FADEOUT);
            let skin_fadeout = self.skin.as_ref().map(|s| s.fadeout() as i64).unwrap_or(0);
            if fadeout_time > skin_fadeout {
                if let Some(audio) = self.main.audio_processor_mut() {
                    audio.stop_note(None);
                }
                {
                    let input = self.main.input_processor();
                    input.reset_all_key_changed_time();
                }

                if self.resource.course_bms_models().is_some() {
                    let last_gauge = self
                        .resource
                        .groove_gauge()
                        .map(|g| g.value())
                        .unwrap_or(0.0);

                    if last_gauge <= 0.0 {
                        if self.resource.course_score_data().is_some() {
                            // Add remaining course notes as POOR
                            // Collect note counts first to avoid borrow conflict
                            let course_gauge_size = self.resource.course_gauge().len();
                            let notes_to_add: Vec<i32> = self
                                .resource
                                .course_bms_models()
                                .map(|models| {
                                    models
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, _)| course_gauge_size <= *i)
                                        .map(|(_, m)| m.total_notes())
                                        .collect()
                                })
                                .unwrap_or_default();
                            for total_notes in notes_to_add {
                                if let Some(mut cscore) = self.resource.course_score_data().cloned()
                                {
                                    cscore.minbp += total_notes;
                                    cscore.timing_stats.total_duration +=
                                        1000000i64 * total_notes as i64;
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
                        let lnmode = self.resource.player_config().play_settings.lnmode;
                        if let Some(songdata) = self.resource.songdata() {
                            let songrank: Option<RankingData> = self
                                .main
                                .ranking_data_cache()
                                .song_any(songdata, lnmode)
                                .and_then(|any| any.downcast::<RankingData>().ok())
                                .map(|ranking| *ranking);
                            if !self.main.ir_status().is_empty() && songrank.is_none() {
                                let new_ranking = RankingData::new();
                                self.main.ranking_data_cache_mut().put_song_any(
                                    songdata,
                                    lnmode,
                                    Box::new(new_ranking.clone()),
                                );
                                self.resource.ranking_data = Some(new_ranking);
                            } else {
                                self.resource.ranking_data = songrank;
                            }
                        }
                        self.main.change_state(MainStateType::Play);
                    } else {
                        // Course pass result
                        self.main.change_state(MainStateType::CourseResult);
                    }
                } else {
                    // Non-course mode
                    let org_gauge = self.resource.org_gauge_option();
                    self.resource.set_player_config_gauge(org_gauge);

                    let mut key: Option<ResultKey> = None;
                    {
                        let input = self.main.input_processor();
                        for i in 0..self.property.assign_length() {
                            if self.property.assign(i) == Some(ResultKey::ReplayDifferent)
                                && input.key_state(i)
                            {
                                key = Some(ResultKey::ReplayDifferent);
                                break;
                            }
                            if self.property.assign(i) == Some(ResultKey::ReplaySame)
                                && input.key_state(i)
                            {
                                key = Some(ResultKey::ReplaySame);
                                break;
                            }
                        }
                    }

                    if self.resource.play_mode().mode == BMSPlayerModeType::Play
                        && key == Some(ResultKey::ReplayDifferent)
                    {
                        info!("Replay without changing options");
                        // Replay without changing options - same chart
                        if let Some(rd) = self.resource.replay_data_mut() {
                            rd.randomoptionseed = -1;
                        }
                        self.resource.reload_bms_file();
                        self.main.change_state(MainStateType::Play);
                    } else if self.resource.play_mode().mode == BMSPlayerModeType::Play
                        && key == Some(ResultKey::ReplaySame)
                    {
                        // Replay with same chart
                        if self.resource.is_update_score() {
                            info!("Replay with same chart");
                        } else {
                            info!("Cannot replay with same chart in assist mode");
                            if let Some(rd) = self.resource.replay_data_mut() {
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
            let skin_scene = self.skin.as_ref().map(|s| s.scene() as i64).unwrap_or(0);
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
        let time = self.data.timer.now_time();

        if !self.data.timer.is_timer_on(TIMER_FADEOUT)
            && self.data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            let skin_input = self.skin.as_ref().map(|s| s.input() as i64).unwrap_or(0);
            if time > skin_input {
                let mut ok = false;
                let mut replay_index: Option<usize> = None;
                let mut open_ir = false;
                {
                    let input_processor = self.main.input_processor();
                    for i in 0..self.property.assign_length() {
                        if self.property.assign(i) == Some(ResultKey::ChangeGraph)
                            && input_processor.key_state(i)
                            && input_processor.reset_key_changed_time(i)
                        {
                            if self.data.gauge_type >= groove_gauge::ASSISTEASY
                                && self.data.gauge_type <= groove_gauge::HAZARD
                            {
                                self.data.gauge_type = (self.data.gauge_type + 1) % 6;
                            } else {
                                self.data.gauge_type = (self.data.gauge_type.max(5) - 5) % 3 + 6;
                            }
                        } else if self.property.assign(i).is_some()
                            && input_processor.key_state(i)
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

                if self.resource.score_data().is_none() || ok {
                    let rank_time = self.skin.as_ref().map(|s| s.rank_time()).unwrap_or(0);
                    if rank_time != 0 && !self.data.timer.is_timer_on(TIMER_RESULT_UPDATESCORE) {
                        self.data.timer.switch_timer(TIMER_RESULT_UPDATESCORE, true);
                    } else if self.data.state == STATE_OFFLINE
                        || self.data.state == STATE_IR_FINISHED
                        || time - self.data.timer.timer(TIMER_IR_CONNECT_BEGIN) >= 1000
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
                    && let Some(ir_status) = self.main.ir_status().first()
                    && let Some(songdata) = self.resource.songdata()
                {
                    let chart = rubato_ir::ir_chart_data::IRChartData::new(songdata);
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
        if self.resource.play_mode().mode == BMSPlayerModeType::Play
            && self.resource.course_bms_models().is_none()
            && self.resource.score_data().is_some()
            && self.data.save_replay[index] != ReplayStatus::Saved
            && self.resource.is_update_score()
            && let Some(rd) = self.resource.replay_data()
        {
            match self.main.play_data_accessor().write_replay_data_model(
                &mut rd.clone(),
                self.resource.bms_model(),
                self.resource.player_config().play_settings.lnmode,
                index as i32,
            ) {
                Ok(()) => {
                    self.data.save_replay[index] = ReplayStatus::Saved;
                    self.main.save_last_recording("ON_REPLAY");
                }
                Err(e) => {
                    log::error!("Failed to save replay data: {}", e);
                }
            }
        }
    }

    pub fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        if let Some(score) = self.resource.score_data() {
            let jc = &score.judge_counts;
            match judge {
                0 => {
                    if fast {
                        jc.epg
                    } else {
                        jc.lpg
                    }
                }
                1 => {
                    if fast {
                        jc.egr
                    } else {
                        jc.lgr
                    }
                }
                2 => {
                    if fast {
                        jc.egd
                    } else {
                        jc.lgd
                    }
                }
                3 => {
                    if fast {
                        jc.ebd
                    } else {
                        jc.lbd
                    }
                }
                4 => {
                    if fast {
                        jc.epr
                    } else {
                        jc.lpr
                    }
                }
                5 => {
                    if fast {
                        jc.ems
                    } else {
                        jc.lms
                    }
                }
                _ => 0,
            }
        } else {
            0
        }
    }

    pub fn total_notes(&self) -> i32 {
        self.resource.bms_model().total_notes()
    }

    pub fn new_score(&self) -> Option<&ScoreData> {
        self.resource.score_data()
    }

    /// Get the skin as ResultSkinData
    pub fn skin(&self) -> Option<&ResultSkinData> {
        self.skin.as_ref()
    }

    /// Set the skin
    pub fn set_skin(&mut self, skin: ResultSkinData) {
        self.skin = Some(skin);
    }

    fn has_sound(&self, sound: SoundType) -> bool {
        super::result_common::has_sound(&self.main, &sound)
    }

    fn play_sound_inner(&mut self, sound: SoundType) {
        super::result_common::play_sound(&mut self.main, &sound);
    }

    fn play_sound_loop_inner(&mut self, sound: SoundType, loop_sound: bool) {
        super::result_common::play_sound_loop(&mut self.main, &sound, loop_sound);
    }

    fn stop_sound_inner(&mut self, sound: SoundType) {
        super::result_common::stop_sound(&mut self.main, &sound);
    }
}

// ============================================================
// MainState trait implementation
// ============================================================

impl MainState for MusicResult {
    super::impl_result_main_state!(MusicResult, Result, ResultRenderContext, ResultMouseContext);

    fn shutdown(&mut self) {
        self.do_shutdown();
    }

    fn dispose(&mut self) {
        if let Some(ref mut skin) = self.main_data.skin {
            skin.dispose_skin();
        }
        self.main_data.skin = None;
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
    use crate::result::abstract_result::{STATE_IR_FINISHED, STATE_IR_PROCESSING, STATE_OFFLINE};
    use crate::result::test_helpers::{
        ExecuteEventSkin, PlayerConfigMutatingSkin, TestMainControllerAccess, make_test_config,
    };
    use rubato_types::player_resource_access::PlayerResourceAccess;
    use rubato_types::skin_render_context::SkinRenderContext;
    use std::path::{Path, PathBuf};

    struct MouseResultResourceAccess {
        config: rubato_types::config::Config,
        player_config: rubato_types::player_config::PlayerConfig,
        score_data: Option<rubato_core::score_data::ScoreData>,
        replay_data: Option<rubato_core::replay_data::ReplayData>,
        song_data: Option<rubato_types::song_data::SongData>,
        course_data: Option<rubato_types::course_data::CourseData>,
        course_gauge: Vec<Vec<Vec<f32>>>,
        course_replay: Vec<rubato_core::replay_data::ReplayData>,
        update_score: bool,
    }

    impl MouseResultResourceAccess {
        fn new(config: rubato_types::config::Config) -> Self {
            Self {
                config,
                player_config: rubato_types::player_config::PlayerConfig::default(),
                score_data: Some(rubato_core::score_data::ScoreData::default()),
                replay_data: Some(rubato_core::replay_data::ReplayData::default()),
                song_data: None,
                course_data: None,
                course_gauge: Vec::new(),
                course_replay: Vec::new(),
                update_score: true,
            }
        }
    }

    impl rubato_types::player_resource_access::ConfigAccess for MouseResultResourceAccess {
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }

        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }

        fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
            Some(&mut self.player_config)
        }
    }

    impl rubato_types::player_resource_access::ScoreAccess for MouseResultResourceAccess {
        fn score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            self.score_data.as_ref()
        }

        fn rival_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }

        fn target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }

        fn course_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }

        fn set_course_score_data(&mut self, _score: rubato_core::score_data::ScoreData) {}

        fn score_data_mut(&mut self) -> Option<&mut rubato_core::score_data::ScoreData> {
            self.score_data.as_mut()
        }
    }

    impl rubato_types::player_resource_access::SongAccess for MouseResultResourceAccess {
        fn songdata(&self) -> Option<&rubato_types::song_data::SongData> {
            self.song_data.as_ref()
        }

        fn songdata_mut(&mut self) -> Option<&mut rubato_types::song_data::SongData> {
            self.song_data.as_mut()
        }

        fn set_songdata(&mut self, data: Option<rubato_types::song_data::SongData>) {
            self.song_data = data;
        }

        fn course_song_data(&self) -> Vec<rubato_types::song_data::SongData> {
            vec![]
        }
    }

    impl rubato_types::player_resource_access::ReplayAccess for MouseResultResourceAccess {
        fn replay_data(&self) -> Option<&rubato_core::replay_data::ReplayData> {
            self.replay_data.as_ref()
        }

        fn replay_data_mut(&mut self) -> Option<&mut rubato_core::replay_data::ReplayData> {
            self.replay_data.as_mut()
        }

        fn course_replay(&self) -> &[rubato_core::replay_data::ReplayData] {
            &self.course_replay
        }

        fn add_course_replay(&mut self, rd: rubato_core::replay_data::ReplayData) {
            self.course_replay.push(rd);
        }

        fn course_replay_mut(&mut self) -> &mut Vec<rubato_core::replay_data::ReplayData> {
            &mut self.course_replay
        }
    }

    impl rubato_types::player_resource_access::CourseAccess for MouseResultResourceAccess {
        fn course_data(&self) -> Option<&rubato_types::course_data::CourseData> {
            self.course_data.as_ref()
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

        fn set_course_data(&mut self, data: rubato_types::course_data::CourseData) {
            self.course_data = Some(data);
        }

        fn clear_course_data(&mut self) {
            self.course_data = None;
        }
    }

    impl rubato_types::player_resource_access::GaugeAccess for MouseResultResourceAccess {
        fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
            None
        }

        fn groove_gauge(&self) -> Option<&rubato_types::groove_gauge::GrooveGauge> {
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

    impl rubato_types::player_resource_access::PlayerStateAccess for MouseResultResourceAccess {
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
            self.update_score
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

    impl rubato_types::player_resource_access::SessionMutation for MouseResultResourceAccess {
        fn clear(&mut self) {}

        fn set_bms_file(&mut self, _path: &Path, _mode_type: i32, _mode_id: i32) -> bool {
            false
        }

        fn set_course_bms_files(&mut self, _files: &[PathBuf]) -> bool {
            false
        }

        fn set_tablename(&mut self, _name: &str) {}

        fn set_tablelevel(&mut self, _level: &str) {}

        fn set_rival_score_data_option(
            &mut self,
            _score: Option<rubato_core::score_data::ScoreData>,
        ) {
        }

        fn set_chart_option_data(&mut self, _option: Option<rubato_core::replay_data::ReplayData>) {
        }
    }

    impl rubato_types::player_resource_access::MediaAccess for MouseResultResourceAccess {
        fn reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }

        fn reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }
    }

    impl PlayerResourceAccess for MouseResultResourceAccess {
        fn into_any_send(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }
    }

    fn make_result_for_mouse() -> MusicResult {
        let config = make_test_config("music-result");
        let main = MainController::new(Box::new(TestMainControllerAccess::new(config.clone())));
        let resource = PlayerResource::new(
            Box::new(MouseResultResourceAccess::new(config)),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        MusicResult::new(main, resource, TimerManager::new())
    }

    #[test]
    fn test_music_result_new_defaults() {
        let mr = MusicResult::default();
        assert_eq!(mr.data.state, STATE_OFFLINE);
        assert_eq!(mr.data.gauge_type, 0);
        assert_eq!(mr.data.ranking_offset, 0);
        assert!(mr.skin.is_none());
    }

    #[test]
    fn test_handle_skin_mouse_pressed_saves_replay_via_result_context() {
        let mut mr = make_result_for_mouse();
        mr.main_data.skin = Some(Box::new(ExecuteEventSkin { event_id: 19 }));

        <MusicResult as MainState>::handle_skin_mouse_pressed(&mut mr, 0, 10, 10);

        assert_eq!(mr.data.save_replay[0], ReplayStatus::Saved);
    }

    #[test]
    fn test_result_mouse_context_exposes_player_config_mut() {
        let mut mr = make_result_for_mouse();
        mr.main_data.skin = Some(Box::new(PlayerConfigMutatingSkin));

        <MusicResult as MainState>::handle_skin_mouse_pressed(&mut mr, 0, 10, 10);

        assert_eq!(mr.resource.player_config().play_settings.random, 1);
    }

    #[test]
    fn test_result_render_context_uses_replay_option_for_image_index_42() {
        let mut mr = make_result_for_mouse();
        mr.resource
            .replay_data_mut()
            .expect("replay data should exist")
            .randomoption = 6;
        let mut timer = TimerManager::new();
        let ctx = ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };

        assert_eq!(ctx.image_index_value(42), 6);
    }

    #[test]
    fn test_state_type_returns_result() {
        let mr = MusicResult::default();
        assert_eq!(mr.state_type(), Some(MainStateType::Result));
    }

    #[test]
    fn test_get_new_score_none_by_default() {
        let mr = MusicResult::default();
        assert!(mr.new_score().is_none());
    }

    #[test]
    fn test_get_judge_count_no_score() {
        let mr = MusicResult::default();
        for judge in 0..6 {
            assert_eq!(mr.judge_count(judge, true), 0);
            assert_eq!(mr.judge_count(judge, false), 0);
        }
        // out of range
        assert_eq!(mr.judge_count(6, true), 0);
        assert_eq!(mr.judge_count(-1, false), 0);
    }

    #[test]
    fn test_total_notes_default() {
        let mr = MusicResult::default();
        assert_eq!(mr.total_notes(), 0);
    }

    #[test]
    fn test_dispose_clears_skin() {
        let mut mr = MusicResult::default();
        // main_data.skin should be None after dispose
        <MusicResult as MainState>::dispose(&mut mr);
        assert!(mr.main_data.skin.is_none());
    }

    #[test]
    fn test_shutdown_does_not_panic() {
        let mut mr = MusicResult::default();
        <MusicResult as MainState>::shutdown(&mut mr);
    }

    #[test]
    fn test_shutdown_does_not_block_on_long_running_ir_thread() {
        // Regression: do_shutdown() used to poll with thread::sleep(50ms) in a loop
        // up to 10 seconds, blocking the render thread. It should now return quickly
        // even if the IR thread is still running.
        let mut mr = MusicResult::default();
        // Spawn a thread that sleeps for a long time (simulating a slow IR send)
        let handle = std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(60));
        });
        mr.ir_thread = Some(handle);

        let start = std::time::Instant::now();
        mr.do_shutdown();
        let elapsed = start.elapsed();

        // Should complete in well under 1 second (non-blocking check + log warning)
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "do_shutdown() blocked for {:?}, expected non-blocking",
            elapsed
        );
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
        <MusicResult as MainState>::create(&mut mr);
        assert_eq!(rubato_skin::skin_type::SkinType::Result.id(), 7);
        assert!(
            mr.main_data.skin.is_some(),
            "result create() should load the configured result skin"
        );
        assert!(
            mr.skin.is_some(),
            "result create() should wire timing metadata from the loaded skin"
        );
    }

    #[test]
    fn test_create_loads_ecfn_result_lua_skin() {
        let mut mr = make_result_for_mouse();
        let result_skin_idx = rubato_skin::skin_type::SkinType::Result.id() as usize;
        mr.resource
            .player_config_mut()
            .expect("player config should be mutable")
            .skin[result_skin_idx] = Some(rubato_types::skin_config::SkinConfig::new_with_path(
            "skin/ECFN/RESULT/result.luaskin",
        ));

        <MusicResult as MainState>::create(&mut mr);

        assert!(
            mr.main_data.skin.is_some(),
            "MusicResult.create() should load ECFN result.luaskin when configured"
        );
        assert!(
            mr.skin.is_some(),
            "MusicResult.create() should keep ResultSkinData for ECFN result.luaskin"
        );
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
        // Update timer so now_time works
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
        assert!(mr.skin().is_none());
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
    }

    #[test]
    fn test_prepare_enters_ir_processing_when_ir_statuses_present() {
        use rubato_ir::ir_player_data::IRPlayerData;
        use std::sync::Arc;

        struct MockIRConnection;
        impl rubato_ir::ir_connection::IRConnection for MockIRConnection {
            fn get_rivals(&self) -> rubato_ir::ir_response::IRResponse<Vec<IRPlayerData>> {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_table_datas(
                &self,
            ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_table_data::IRTableData>>
            {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_play_data(
                &self,
                _player: Option<&IRPlayerData>,
                _chart: &rubato_ir::ir_chart_data::IRChartData,
            ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_score_data::IRScoreData>>
            {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_course_play_data(
                &self,
                _player: Option<&IRPlayerData>,
                _course: &rubato_ir::ir_course_data::IRCourseData,
            ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_score_data::IRScoreData>>
            {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn send_play_data(
                &self,
                _model: &rubato_ir::ir_chart_data::IRChartData,
                _score: &rubato_ir::ir_score_data::IRScoreData,
            ) -> rubato_ir::ir_response::IRResponse<()> {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn send_course_play_data(
                &self,
                _course: &rubato_ir::ir_course_data::IRCourseData,
                _score: &rubato_ir::ir_score_data::IRScoreData,
            ) -> rubato_ir::ir_response::IRResponse<()> {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_song_url(
                &self,
                _chart: &rubato_ir::ir_chart_data::IRChartData,
            ) -> Option<String> {
                None
            }
            fn get_course_url(
                &self,
                _course: &rubato_ir::ir_course_data::IRCourseData,
            ) -> Option<String> {
                None
            }
            fn get_player_url(&self, _player: &IRPlayerData) -> Option<String> {
                None
            }
            fn name(&self) -> &str {
                "MockIR"
            }
        }

        let config = make_test_config("ir-prepare");
        let ir_statuses = vec![super::super::ir_status::IRStatus::new(
            rubato_core::ir_config::IRConfig::default(),
            Arc::new(MockIRConnection)
                as Arc<dyn rubato_ir::ir_connection::IRConnection + Send + Sync>,
            IRPlayerData::new("id1".into(), "Player1".into(), "1st".into()),
        )];
        let main = MainController::with_ir_statuses(
            Box::new(TestMainControllerAccess::new(config.clone())),
            ir_statuses,
        );
        let resource = PlayerResource::new(
            Box::new(MouseResultResourceAccess::new(config)),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        let mut mr = MusicResult::new(main, resource, TimerManager::new());

        <MusicResult as MainState>::prepare(&mut mr);

        assert_eq!(
            mr.data.state, STATE_IR_PROCESSING,
            "prepare() should enter IR_PROCESSING state when IR statuses are present"
        );
    }

    // ============================================================
    // lnmode (ID 308) image_index_value override tests
    // ============================================================

    fn make_result_with_songdata(
        song_data: Option<rubato_types::song_data::SongData>,
    ) -> MusicResult {
        let config = make_test_config("lnmode-result");
        let main = MainController::new(Box::new(TestMainControllerAccess::new(config.clone())));
        let mut res_access = MouseResultResourceAccess::new(config);
        res_access.song_data = song_data;
        // Set lnmode config to a sentinel value (99) so we can verify fallback
        res_access.player_config.play_settings.lnmode = 99;
        let resource = PlayerResource::new(
            Box::new(res_access),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        MusicResult::new(main, resource, TimerManager::new())
    }

    #[test]
    fn result_lnmode_308_override_longnote() {
        use rubato_types::song_data::{ChartInfo, FEATURE_LONGNOTE, SongData};
        let mut mr = make_result_with_songdata(Some(SongData {
            chart: ChartInfo {
                feature: FEATURE_LONGNOTE,
                ..ChartInfo::default()
            },
            ..SongData::default()
        }));
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };
        assert_eq!(ctx.image_index_value(308), 0);
    }

    #[test]
    fn result_lnmode_308_override_chargenote() {
        use rubato_types::song_data::{ChartInfo, FEATURE_CHARGENOTE, SongData};
        let mut mr = make_result_with_songdata(Some(SongData {
            chart: ChartInfo {
                feature: FEATURE_CHARGENOTE,
                ..ChartInfo::default()
            },
            ..SongData::default()
        }));
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };
        assert_eq!(ctx.image_index_value(308), 1);
    }

    #[test]
    fn result_lnmode_308_override_hellchargenote() {
        use rubato_types::song_data::{ChartInfo, FEATURE_HELLCHARGENOTE, SongData};
        let mut mr = make_result_with_songdata(Some(SongData {
            chart: ChartInfo {
                feature: FEATURE_HELLCHARGENOTE,
                ..ChartInfo::default()
            },
            ..SongData::default()
        }));
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };
        assert_eq!(ctx.image_index_value(308), 2);
    }

    #[test]
    fn result_lnmode_308_undefined_ln_falls_through_to_config() {
        use rubato_types::song_data::{ChartInfo, FEATURE_UNDEFINEDLN, SongData};
        let mut mr = make_result_with_songdata(Some(SongData {
            chart: ChartInfo {
                feature: FEATURE_UNDEFINEDLN,
                ..ChartInfo::default()
            },
            ..SongData::default()
        }));
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };
        // Falls through to player_config.play_settings.lnmode = 99
        assert_eq!(ctx.image_index_value(308), 99);
    }

    #[test]
    fn result_lnmode_308_no_songdata_falls_through_to_config() {
        let mut mr = make_result_with_songdata(None);
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };
        // Falls through to player_config.play_settings.lnmode = 99
        assert_eq!(ctx.image_index_value(308), 99);
    }

    #[test]
    fn result_render_context_returns_ranking_name_strings() {
        use rubato_ir::ir_score_data::IRScoreData;
        use rubato_ir::ranking_data::RankingData;

        let mut mr = make_result_with_songdata(None);
        let mut ranking = RankingData::new();
        let scores: Vec<IRScoreData> = vec![
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.player = "ALICE".to_string();
                s.judge_counts.epg = 120;
                IRScoreData::new(&s)
            },
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.player = "YOU".to_string();
                s.judge_counts.epg = 110;
                IRScoreData::new(&s)
            },
        ];
        ranking.update_score(&scores, None);
        mr.data.ranking = Some(ranking);
        mr.data.ranking_offset = 1;

        let mut timer = TimerManager::new();
        let ctx = render_context::ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };

        assert_eq!(ctx.string_value(120), "YOU");
        assert_eq!(ctx.string_value(121), "");
    }

    // ============================================================
    // ResultMouseContext player_config_ref / config_ref delegation tests
    // ============================================================

    #[test]
    fn result_mouse_context_player_config_ref_returns_some() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        assert!(
            ctx.player_config_ref().is_some(),
            "ResultMouseContext::player_config_ref() must delegate to resource"
        );
    }

    #[test]
    fn result_mouse_context_config_ref_returns_some() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        assert!(
            ctx.config_ref().is_some(),
            "ResultMouseContext::config_ref() must delegate to main controller"
        );
    }

    // ============================================================
    // ResultMouseContext set_float_value volume slider tests
    // ============================================================

    /// Mock MainControllerAccess that captures update_audio_config calls.
    struct VolumeCapturingAccess {
        config: rubato_types::config::Config,
        player_config: rubato_types::player_config::PlayerConfig,
        captured_audio:
            std::sync::Arc<std::sync::Mutex<Vec<rubato_types::audio_config::AudioConfig>>>,
    }

    impl VolumeCapturingAccess {
        fn new(
            captured: std::sync::Arc<
                std::sync::Mutex<Vec<rubato_types::audio_config::AudioConfig>>,
            >,
        ) -> Self {
            let mut config = rubato_types::config::Config::default();
            config.audio = Some(rubato_types::audio_config::AudioConfig::default());
            Self {
                config,
                player_config: rubato_types::player_config::PlayerConfig::default(),
                captured_audio: captured,
            }
        }
    }

    impl rubato_types::main_controller_access::MainControllerAccess for VolumeCapturingAccess {
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }
        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }
        fn change_state(&mut self, _state: rubato_core::main_state::MainStateType) {}
        fn save_config(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn exit(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn save_last_recording(&self, _reason: &str) {}
        fn update_song(&mut self, _path: Option<&str>) {}
        fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
            None
        }
        fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
            None
        }
        fn update_audio_config(&self, audio: rubato_types::audio_config::AudioConfig) {
            self.captured_audio.lock().unwrap().push(audio);
        }
    }

    #[test]
    fn result_mouse_context_set_float_value_propagates_volume() {
        // Regression: volume slider writes (IDs 17-19) on the result screen
        // must propagate to MainController via update_audio_config, not be silently dropped.
        let captured: std::sync::Arc<
            std::sync::Mutex<Vec<rubato_types::audio_config::AudioConfig>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let config = make_test_config("volume-result");
        let main = MainController::new(Box::new(VolumeCapturingAccess::new(captured.clone())));
        let resource = PlayerResource::new(
            Box::new(MouseResultResourceAccess::new(config)),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        let mut mr = MusicResult::new(main, resource, TimerManager::new());

        let mut timer = TimerManager::new();
        {
            let mut ctx = render_context::ResultMouseContext {
                timer: &mut timer,
                result: &mut mr,
            };
            ctx.set_float_value(17, 0.75);
            ctx.set_float_value(18, 0.5);
            ctx.set_float_value(19, 0.25);
        }

        let calls = captured.lock().unwrap();
        assert_eq!(calls.len(), 3, "should have 3 update_audio_config calls");
        assert_eq!(calls[0].systemvolume, 0.75);
        assert_eq!(calls[1].keyvolume, 0.5);
        assert_eq!(calls[2].bgvolume, 0.25);
    }

    #[test]
    fn result_mouse_context_set_float_value_clamps_volume() {
        let captured: std::sync::Arc<
            std::sync::Mutex<Vec<rubato_types::audio_config::AudioConfig>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let config = make_test_config("volume-clamp-result");
        let main = MainController::new(Box::new(VolumeCapturingAccess::new(captured.clone())));
        let resource = PlayerResource::new(
            Box::new(MouseResultResourceAccess::new(config)),
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        let mut mr = MusicResult::new(main, resource, TimerManager::new());

        let mut timer = TimerManager::new();
        {
            let mut ctx = render_context::ResultMouseContext {
                timer: &mut timer,
                result: &mut mr,
            };
            ctx.set_float_value(17, -0.5);
            ctx.set_float_value(18, 1.5);
        }

        let calls = captured.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].systemvolume, 0.0, "negative should clamp to 0.0");
        assert_eq!(calls[1].keyvolume, 1.0, "above 1.0 should clamp to 1.0");
    }

    // ============================================================
    // ResultMouseContext missing delegation regression tests
    // ============================================================

    #[test]
    fn result_mouse_context_gauge_value_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        // Default resource has no gauge -> 0.0
        assert_eq!(
            ctx.gauge_value(),
            0.0,
            "ResultMouseContext::gauge_value() must delegate to shared_render_context"
        );
    }

    #[test]
    fn result_mouse_context_gauge_type_delegates() {
        let mut mr = make_result_for_mouse();
        mr.data.gauge_type = 3;
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        assert_eq!(
            ctx.gauge_type(),
            3,
            "ResultMouseContext::gauge_type() must delegate to data.gauge_type"
        );
    }

    #[test]
    fn result_mouse_context_is_gauge_max_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        assert!(
            !ctx.is_gauge_max(),
            "ResultMouseContext::is_gauge_max() must delegate"
        );
    }

    #[test]
    fn result_mouse_context_gauge_min_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        // No gauge -> 0.0
        assert_eq!(
            ctx.gauge_min(),
            0.0,
            "ResultMouseContext::gauge_min() must delegate"
        );
    }

    #[test]
    fn result_mouse_context_gauge_border_max_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        // No groove gauge -> None
        assert!(
            ctx.gauge_border_max().is_none(),
            "ResultMouseContext::gauge_border_max() must delegate"
        );
    }

    #[test]
    fn result_mouse_context_gauge_history_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        // Default resource has no gauge history
        assert!(
            ctx.gauge_history().is_none(),
            "ResultMouseContext::gauge_history() must delegate"
        );
    }

    #[test]
    fn result_mouse_context_judge_count_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        assert_eq!(
            ctx.judge_count(0, true),
            0,
            "ResultMouseContext::judge_count() must delegate"
        );
    }

    #[test]
    fn result_mouse_context_judge_area_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        // judge_area depends on bms_model; default resource returns Some with default windows
        let result = ctx.judge_area();
        assert!(
            result.is_some(),
            "ResultMouseContext::judge_area() must delegate"
        );
    }

    #[test]
    fn result_mouse_context_get_timing_distribution_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        // Default data has empty timing distribution
        assert!(
            ctx.get_timing_distribution().is_none(),
            "ResultMouseContext::get_timing_distribution() must delegate"
        );
    }

    #[test]
    fn result_mouse_context_score_data_property_delegates() {
        let mut mr = make_result_for_mouse();
        mr.data.score.nowrate = 0.42;
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        let prop = ctx.score_data_property();
        assert!(
            (prop.now_rate() - 0.42).abs() < f32::EPSILON,
            "ResultMouseContext::score_data_property() must delegate to data.score"
        );
    }

    #[test]
    fn result_mouse_context_replay_option_data_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        // replay_option_data delegates to resource
        let _ = ctx.replay_option_data();
        // Just verify it doesn't panic; returns None or Some depending on resource
    }

    #[test]
    fn result_mouse_context_target_score_data_delegates() {
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        // Default resource has no target score
        assert!(
            ctx.target_score_data().is_none(),
            "ResultMouseContext::target_score_data() must delegate"
        );
    }

    // ============================================================
    // is_clear logic: None course_score_data must not panic
    // ============================================================

    /// Regression: the is_clear computation in do_prepare formerly used
    /// `cscore.is_none() || cscore.expect("cscore").clear != ...` which panics
    /// if short-circuit evaluation is ever disrupted. Verify the safe `map_or`
    /// replacement handles None and Some cases correctly.
    #[test]
    fn is_clear_logic_none_course_score_is_clear() {
        let cscore: Option<&rubato_core::score_data::ScoreData> = None;
        let ns_clear = rubato_core::clear_type::ClearType::Normal.id();
        let is_clear = ns_clear != rubato_core::clear_type::ClearType::Failed.id()
            && cscore.is_none_or(|cs| cs.clear != rubato_core::clear_type::ClearType::Failed.id());
        assert!(
            is_clear,
            "When course_score_data is None and newscore is not Failed, is_clear should be true"
        );
    }

    #[test]
    fn is_clear_logic_course_score_failed_means_not_clear() {
        let mut course = rubato_core::score_data::ScoreData::default();
        course.clear = rubato_core::clear_type::ClearType::Failed.id();
        let cscore: Option<&rubato_core::score_data::ScoreData> = Some(&course);
        let ns_clear = rubato_core::clear_type::ClearType::Normal.id();
        let is_clear = ns_clear != rubato_core::clear_type::ClearType::Failed.id()
            && cscore.is_none_or(|cs| cs.clear != rubato_core::clear_type::ClearType::Failed.id());
        assert!(
            !is_clear,
            "When course_score_data is Failed, is_clear should be false even if newscore is not Failed"
        );
    }

    #[test]
    fn is_clear_logic_newscore_failed_means_not_clear() {
        let cscore: Option<&rubato_core::score_data::ScoreData> = None;
        let ns_clear = rubato_core::clear_type::ClearType::Failed.id();
        let is_clear = ns_clear != rubato_core::clear_type::ClearType::Failed.id()
            && cscore.is_none_or(|cs| cs.clear != rubato_core::clear_type::ClearType::Failed.id());
        assert!(
            !is_clear,
            "When newscore is Failed, is_clear should be false regardless of course_score_data"
        );
    }

    #[test]
    fn result_mouse_context_integer_value_uses_boot_time_millis() {
        // Regression: ResultMouseContext.integer_value() must pass boot_time_millis
        // (not now_time) to shared_render_context::integer_value for IDs 27-29.
        // boot_time_millis = 7_200_000 ms (2 hours), now_time = 5 ms (unrelated).
        let mut mr = make_result_for_mouse();
        let mut timer = TimerManager::new();
        timer.set_boot_time_millis(7_200_000); // 2 hours
        timer.set_now_micro_time(5_000); // 5 ms state-relative
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
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
}

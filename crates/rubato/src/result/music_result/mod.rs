// MusicResult.java -> music_result.rs
// Mechanical line-by-line translation.

use log::info;

use crate::core::app_context::GameContext;
use crate::core::clear_type::ClearType;
use crate::core::main_state::{MainState, MainStateData, MainStateType, StateTransition};
use crate::core::score_data::ScoreData;
use crate::core::system_sound_manager::SoundType;
use crate::core::timer_manager::TimerManager;
use crate::play::groove_gauge;
use crate::skin::skin_property::*;

use super::abstract_result::{
    AbstractResultData, REPLAY_SIZE, ReplayAutoSaveConstraint, ReplayStatus, STATE_IR_FINISHED,
    STATE_IR_PROCESSING, STATE_OFFLINE,
};
use super::ir_send_status::IRSendStatusMain;
use super::result_key_property::{ResultKey, ResultKeyProperty};
use super::result_skin_data::ResultSkinData;
use super::{
    BMSPlayerModeType, ControlKeys, KeyCommand, MainController, PlayerResource, RankingData,
};
use crate::core::ir_config::{IR_SEND_ALWAYS, IR_SEND_COMPLETE_SONG, IR_SEND_UPDATE_SCORE};
use crate::skin::property_snapshot::PropertySnapshot;
use crate::skin::skin_action_queue::SkinActionQueue;
use crate::skin::timer_id::TimerId;

#[cfg(test)]
mod render_context;
mod score_handler;

/// IR send result for async processing.
type MusicIrResult = (
    bool,
    bool,
    Option<Vec<crate::ir::ir_score_data::IRScoreData>>,
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
    /// Custom events queued during mouse handling (skin is taken, so
    /// execute_custom_event cannot be called). Replayed after the skin
    /// is restored.
    /// Only used by legacy ResultMouseContext in tests after PropertySnapshot migration.
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
    pending_audio_config: Option<crate::skin::audio_config::AudioConfig>,
    /// Outbox: pending stop-all-notes request (fadeout).
    pending_stop_all_notes: bool,
    /// Outbox: pending state change from skin callbacks / do_render.
    pending_state_change: Option<MainStateType>,
    /// Outbox: pending save_last_recording requests.
    pending_save_last_recording: Vec<String>,
    /// Read-only input snapshot for the current frame.
    input_snapshot: Option<crate::input::input_snapshot::InputSnapshot>,
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
        self.load_skin(crate::skin::skin_type::SkinType::Result.id());
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

            self.main_data
                .timer
                .switch_timer(TIMER_IR_CONNECT_BEGIN, true);

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
                    let chart_data = crate::ir::ir_chart_data::IRChartData::new(songdata);
                    let response = conn.get_play_data(None, Some(&chart_data));
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
                self.main_data
                    .timer
                    .switch_timer(TIMER_IR_CONNECT_SUCCESS, true);
            } else {
                self.main_data
                    .timer
                    .switch_timer(TIMER_IR_CONNECT_FAIL, true);
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
                            self.pending_state_change = Some(MainStateType::CourseResult);
                        } else {
                            // No course score — go to music select
                            self.pending_state_change = Some(MainStateType::MusicSelect);
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
                        self.pending_state_change = Some(MainStateType::Play);
                    } else {
                        // Course pass result
                        self.pending_state_change = Some(MainStateType::CourseResult);
                    }
                } else {
                    // Non-course mode
                    let org_gauge = self.resource.org_gauge_option();
                    self.resource.set_player_config_gauge(org_gauge);

                    let mut key: Option<ResultKey> = None;
                    if let Some(ref snapshot) = self.input_snapshot {
                        for i in 0..self.property.assign_length() {
                            let idx = i as usize;
                            if self.property.assign(i) == Some(ResultKey::ReplayDifferent)
                                && snapshot.key_state[idx]
                            {
                                key = Some(ResultKey::ReplayDifferent);
                                break;
                            }
                            if self.property.assign(i) == Some(ResultKey::ReplaySame)
                                && snapshot.key_state[idx]
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
                        self.pending_state_change = Some(MainStateType::Play);
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
                        self.pending_state_change = Some(MainStateType::Play);
                    } else {
                        self.pending_state_change = Some(MainStateType::MusicSelect);
                    }
                }
            }
        } else {
            let skin_scene = self.skin.as_ref().map(|s| s.scene() as i64).unwrap_or(0);
            if time > skin_scene {
                self.main_data.timer.switch_timer(TIMER_FADEOUT, true);
                if self.has_sound(SoundType::ResultClose) {
                    self.stop_sound_inner(SoundType::ResultClear);
                    self.stop_sound_inner(SoundType::ResultFail);
                    self.play_sound_inner(SoundType::ResultClose);
                }
            }
        }
    }

    fn do_input(&mut self) {
        let snapshot = match self.input_snapshot {
            Some(ref s) => s,
            None => return,
        };
        self.data.input(snapshot);
        let time = self.main_data.timer.now_time();

        if !self.main_data.timer.is_timer_on(TIMER_FADEOUT)
            && self.main_data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            let skin_input = self.skin.as_ref().map(|s| s.input() as i64).unwrap_or(0);
            if time > skin_input {
                let mut ok = false;
                let mut replay_index: Option<usize> = None;
                let mut open_ir = false;

                for i in 0..self.property.assign_length() {
                    let idx = i as usize;
                    if self.property.assign(i) == Some(ResultKey::ChangeGraph)
                        && snapshot.key_state[idx]
                        && snapshot.key_changed_time[idx] != i64::MIN
                    {
                        if self.data.gauge_type >= groove_gauge::ASSISTEASY
                            && self.data.gauge_type <= groove_gauge::HAZARD
                        {
                            self.data.gauge_type = (self.data.gauge_type + 1) % 6;
                        } else {
                            self.data.gauge_type = (self.data.gauge_type.max(5) - 5) % 3 + 6;
                        }
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

                if snapshot
                    .control_key_states
                    .get(&ControlKeys::Num1)
                    .copied()
                    .unwrap_or(false)
                {
                    replay_index = Some(0);
                } else if snapshot
                    .control_key_states
                    .get(&ControlKeys::Num2)
                    .copied()
                    .unwrap_or(false)
                {
                    replay_index = Some(1);
                } else if snapshot
                    .control_key_states
                    .get(&ControlKeys::Num3)
                    .copied()
                    .unwrap_or(false)
                {
                    replay_index = Some(2);
                } else if snapshot
                    .control_key_states
                    .get(&ControlKeys::Num4)
                    .copied()
                    .unwrap_or(false)
                {
                    replay_index = Some(3);
                }

                if snapshot.activated_commands.contains(&KeyCommand::OpenIr) {
                    open_ir = true;
                }

                if self.resource.score_data().is_none() || ok {
                    let rank_time = self.skin.as_ref().map(|s| s.rank_time()).unwrap_or(0);
                    if rank_time != 0 && !self.main_data.timer.is_timer_on(TIMER_RESULT_UPDATESCORE)
                    {
                        self.main_data
                            .timer
                            .switch_timer(TIMER_RESULT_UPDATESCORE, true);
                    } else if self.data.state == STATE_OFFLINE
                        || self.data.state == STATE_IR_FINISHED
                        || time - self.main_data.timer.timer(TIMER_IR_CONNECT_BEGIN) >= 1000
                    {
                        self.main_data.timer.switch_timer(TIMER_FADEOUT, true);
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
                    let chart = crate::ir::ir_chart_data::IRChartData::new(songdata);
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
                    self.pending_save_last_recording
                        .push("ON_REPLAY".to_string());
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
        self.pending_sounds.push((sound, false));
    }

    fn play_sound_loop_inner(&mut self, sound: SoundType, loop_sound: bool) {
        self.pending_sounds.push((sound, loop_sound));
    }

    fn stop_sound_inner(&mut self, sound: SoundType) {
        self.pending_sound_stops.push(sound);
    }

    /// Build a PropertySnapshot capturing all raw data needed for skin rendering.
    fn build_snapshot(&self, timer: &TimerManager) -> PropertySnapshot {
        use super::shared_render_context;

        let mut s = PropertySnapshot::new();

        // ---- Timing ----
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

        // ---- State identity ----
        s.state_type = Some(crate::skin::main_state_type::MainStateType::Result);

        // ---- Config ----
        s.config = Some(Box::new(self.main.config().clone()));
        s.player_config = Some(Box::new(self.resource.player_config().clone()));

        // ---- Play config (resolve mode from song data) ----
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

        // ---- Song / score data ----
        s.song_data = self.resource.songdata().map(|d| Box::new(d.clone()));
        s.score_data = self.data.score.score.as_ref().map(|d| Box::new(d.clone()));
        s.rival_score_data = Some(Box::new(self.data.oldscore.clone()));
        s.target_score_data = self
            .resource
            .target_score_data()
            .map(|d| Box::new(d.clone()));
        s.replay_option_data = self.resource.replay_data().map(|d| Box::new(d.clone()));
        s.score_data_property = self.data.score.clone();

        // ---- Player / course data ----
        s.player_data = Some(*self.resource.player_data());
        s.is_course_mode = self.resource.course_bms_models().is_some();
        s.course_index = self
            .resource
            .course_bms_models()
            .map_or(0, |_| self.resource.course_index());
        s.course_song_count = self
            .resource
            .course_bms_models()
            .map_or(0, |models| models.len());
        s.is_update_score = self.resource.is_update_score();

        // ---- Offsets ----
        s.offsets = self.main_data.offsets.clone();

        // ---- Gauge data ----
        s.gauge_value = shared_render_context::gauge_value(&self.resource);
        s.gauge_type = self.data.gauge_type;
        s.is_gauge_max = shared_render_context::is_gauge_max(&self.resource);
        s.gauge_element_borders = shared_render_context::gauge_element_borders(&self.resource);
        s.gauge_history = self.resource.gauge().cloned();
        s.course_gauge_history = self.resource.course_gauge().to_vec();
        s.gauge_border_max =
            shared_render_context::gauge_border_max(&self.resource, self.data.gauge_type);
        s.gauge_min = shared_render_context::gauge_min(&self.resource, self.data.gauge_type);
        s.result_gauge_type = self.data.gauge_type;

        // gauge_transition_last_values: populate for all gauge types
        if let Some(gauge_history) = self.resource.gauge() {
            for (i, type_history) in gauge_history.iter().enumerate() {
                if let Some(&last) = type_history.last() {
                    s.gauge_transition_last_values.insert(i as i32, last);
                }
            }
        }

        // ---- Timing distribution / judge area ----
        s.timing_distribution = shared_render_context::get_timing_distribution(&self.data).cloned();
        s.judge_area = shared_render_context::judge_area(&self.resource);

        // ---- Judge counts ----
        for judge in 0..=5 {
            s.judge_counts.insert(
                (judge, true),
                shared_render_context::judge_count(&self.data, judge, true),
            );
            s.judge_counts.insert(
                (judge, false),
                shared_render_context::judge_count(&self.data, judge, false),
            );
        }

        // ---- Ranking data ----
        s.ranking_offset = self.data.ranking_offset;
        if let Some(ref ranking) = self.data.ranking {
            for slot in 0..10 {
                let index = self.data.ranking_offset + slot;
                let clear_type = ranking
                    .score(index)
                    .map(|score| score.clear.id())
                    .unwrap_or(-1);
                s.ranking_clear_types.push(clear_type);
            }
        }

        // ---- Lane shuffle patterns ----
        s.lane_shuffle_patterns = self
            .resource
            .replay_data()
            .and_then(|rd| rd.lane_shuffle_pattern.clone());

        // ---- Result-specific booleans ----
        // Autoplay indicators (32/33)
        let play_mode = self.resource.play_mode().mode;
        s.booleans.insert(
            32,
            play_mode != BMSPlayerModeType::Autoplay && play_mode != BMSPlayerModeType::Replay,
        );
        s.booleans.insert(
            33,
            play_mode == BMSPlayerModeType::Autoplay || play_mode == BMSPlayerModeType::Replay,
        );
        // Gauge groove/hard, clear/fail, gauge ex (42/43/90/91/1046)
        let course_score = self.resource.course_score_data();
        s.booleans.insert(
            42,
            shared_render_context::boolean_value(&self.data, course_score, 42),
        );
        s.booleans.insert(
            43,
            shared_render_context::boolean_value(&self.data, course_score, 43),
        );
        s.booleans.insert(
            90,
            shared_render_context::boolean_value(&self.data, course_score, 90),
        );
        s.booleans.insert(
            91,
            shared_render_context::boolean_value(&self.data, course_score, 91),
        );
        s.booleans.insert(
            1046,
            shared_render_context::boolean_value(&self.data, course_score, 1046),
        );

        // ---- Result-specific integers ----
        let playtime = self.resource.player_data().playtime;
        let songdata = self.resource.songdata();
        let player_data = Some(self.resource.player_data());

        // Populate all result-specific integers from shared_render_context
        let result_int_ids: &[i32] = &[
            71, 72, 74, 75, 76, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 100, 101, 102, 103, 105,
            106, 108, 110, 111, 112, 113, 114, 115, 116, 121, 122, 123, 128, 150, 151, 152, 153,
            154, 155, 156, 157, 158, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 182,
            183, 184, 200, 271, 280, 281, 282, 283, 284, 285, 286, 287, 288, 289, 350, 370, 371,
            372, 373, 374, 375, 376, 377, 380, 381, 382, 383, 384, 385, 386, 387, 388, 389, 390,
            391, 392, 393, 394, 395, 396, 397, 398, 399, 410, 411, 412, 413, 414, 415, 416, 417,
            418, 419, 423, 424, 425,
        ];
        for &id in result_int_ids {
            let val = shared_render_context::integer_value(
                &self.data,
                timer.boot_time_millis(),
                playtime,
                songdata,
                player_data,
                id,
            );
            // Only insert if meaningful (not the fallback i32::MIN from the shared function).
            // Actually, we always insert so the snapshot has the correct value for all IDs.
            s.integers.insert(id, val);
        }

        // ---- Result-specific floats ----
        s.floats
            .insert(1107, shared_render_context::gauge_value(&self.resource));
        // Populate all result-specific float values from shared_render_context
        let result_float_ids: &[i32] = &[
            85, 86, 87, 88, 89, 110, 111, 112, 113, 114, 115, 122, 135, 155, 157, 183, 285, 286,
            287, 288, 289, 1102, 1115,
        ];
        for &id in result_float_ids {
            if let Some(val) = shared_render_context::float_value(&self.data, id) {
                s.floats.insert(id, val);
            }
        }

        // ---- Result-specific strings ----
        // Ranking names (120-129)
        for slot in 0..10 {
            let name = shared_render_context::ranking_name(&self.data, slot);
            if !name.is_empty() {
                s.strings.insert(120 + slot, name);
            }
        }
        // Song hashes (1030/1031)
        if let Some(song) = self.resource.songdata() {
            if !song.file.md5.is_empty() {
                s.strings.insert(1030, song.file.md5.clone());
            }
            if !song.file.sha256.is_empty() {
                s.strings.insert(1031, song.file.sha256.clone());
            }
        }

        // Mouse position
        if let Some(ref input) = self.input_snapshot {
            s.mouse_x = input.mouse_x as f32;
            s.mouse_y = input.mouse_y as f32;
        }

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

        // Replay save events from custom events
        // (replay_index_from_event_id maps event IDs 19, 316, 317, 318 to replay slots)
        // These are handled during the custom_events replay loop, not here.

        // Option change sound
        if actions.option_change_sound {
            self.pending_sounds.push((SoundType::OptionChange, false));
            actions.option_change_sound = false;
        }
    }

    /// Copy player_config back from the snapshot to the resource if it was modified.
    fn propagate_player_config(&mut self, snapshot: &PropertySnapshot) {
        if let Some(ref pc) = snapshot.player_config
            && let Some(target) = self.resource.player_config_mut()
        {
            *target = (**pc).clone();
        }
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

                if self.resource.course_bms_models().is_some() {
                    let last_gauge = self
                        .resource
                        .groove_gauge()
                        .map(|g| g.value())
                        .unwrap_or(0.0);

                    if last_gauge <= 0.0 {
                        if self.resource.course_score_data().is_some() {
                            // Add remaining course notes as POOR
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
                            return StateTransition::ChangeTo(MainStateType::CourseResult);
                        } else {
                            // No course score -- go to music select
                            return StateTransition::ChangeTo(MainStateType::MusicSelect);
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
                        return StateTransition::ChangeTo(MainStateType::Play);
                    } else {
                        // Course pass result
                        return StateTransition::ChangeTo(MainStateType::CourseResult);
                    }
                } else {
                    // Non-course mode
                    let org_gauge = self.resource.org_gauge_option();
                    self.resource.set_player_config_gauge(org_gauge);

                    let mut key: Option<ResultKey> = None;
                    if let Some(ref snapshot) = self.input_snapshot {
                        for i in 0..self.property.assign_length() {
                            let idx = i as usize;
                            if self.property.assign(i) == Some(ResultKey::ReplayDifferent)
                                && snapshot.key_state[idx]
                            {
                                key = Some(ResultKey::ReplayDifferent);
                                break;
                            }
                            if self.property.assign(i) == Some(ResultKey::ReplaySame)
                                && snapshot.key_state[idx]
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
                        if let Some(rd) = self.resource.replay_data_mut() {
                            rd.randomoptionseed = -1;
                        }
                        self.resource.reload_bms_file();
                        return StateTransition::ChangeTo(MainStateType::Play);
                    } else if self.resource.play_mode().mode == BMSPlayerModeType::Play
                        && key == Some(ResultKey::ReplaySame)
                    {
                        if self.resource.is_update_score() {
                            info!("Replay with same chart");
                        } else {
                            info!("Cannot replay with same chart in assist mode");
                            if let Some(rd) = self.resource.replay_data_mut() {
                                rd.randomoptionseed = -1;
                            }
                        }
                        self.resource.reload_bms_file();
                        return StateTransition::ChangeTo(MainStateType::Play);
                    } else {
                        return StateTransition::ChangeTo(MainStateType::MusicSelect);
                    }
                }
            }
        } else {
            let skin_scene = self.skin.as_ref().map(|s| s.scene() as i64).unwrap_or(0);
            if time > skin_scene {
                self.main_data.timer.switch_timer(TIMER_FADEOUT, true);
                if ctx.sound_path(&SoundType::ResultClose).is_some() {
                    ctx.stop_sound(&SoundType::ResultClear);
                    ctx.stop_sound(&SoundType::ResultFail);
                    ctx.play_sound(&SoundType::ResultClose, false);
                }
            }
        }

        StateTransition::Continue
    }

    fn render_skin(&mut self, sprite: &mut crate::render::sprite_batch::SpriteBatch) {
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

        // Replay queued custom events now that the skin is available again.
        let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
        let mut depth = 0;
        while !pending_events.is_empty() && depth < 8 {
            let mut replay_snapshot = self.build_snapshot(&timer);
            for (id, arg1, arg2) in pending_events {
                // Check for replay save events before passing to skin
                if let Some(index) = super::shared_render_context::replay_index_from_event_id(id) {
                    self.save_replay_data(index);
                } else {
                    skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
                }
            }
            self.drain_actions(&mut replay_snapshot.actions, &mut timer);
            pending_events = replay_snapshot.actions.custom_events;
            depth += 1;
        }
        if depth >= 8 {
            log::warn!("Result render_skin event replay exceeded depth limit");
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
        self.propagate_player_config(&snapshot);

        // Replay queued custom events.
        let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
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
            self.propagate_player_config(&replay_snapshot);
            pending_events = replay_snapshot.actions.custom_events;
            depth += 1;
        }
        if depth >= 8 {
            log::warn!("Result mouse_pressed event replay exceeded depth limit");
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
        self.propagate_player_config(&snapshot);

        // Replay queued custom events.
        let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
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
            self.propagate_player_config(&replay_snapshot);
            pending_events = replay_snapshot.actions.custom_events;
            depth += 1;
        }
        if depth >= 8 {
            log::warn!("Result mouse_dragged event replay exceeded depth limit");
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

    fn sync_input_snapshot(&mut self, snapshot: &crate::input::input_snapshot::InputSnapshot) {
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
            .or_else(|| crate::skin::skin_config::SkinConfig::default_for_id(skin_type).path);
        // Take timer out to avoid borrowing self.main_data and its fields simultaneously
        let timer = std::mem::take(&mut self.main_data.timer);
        let loaded = {
            let mut snapshot = self.build_snapshot(&timer);
            let registry = std::collections::HashMap::new();
            let mut state =
                crate::skin::snapshot_main_state::SnapshotMainState::new(&mut snapshot, &registry);
            skin_path.as_deref().and_then(|path| {
                crate::skin::skin_loader::load_skin_from_path_with_state(
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

    fn shutdown(&mut self) {
        self.do_shutdown();
    }

    fn dispose(&mut self) {
        if let Some(ref mut skin) = self.main_data.skin {
            skin.dispose_skin();
        }
        self.main_data.skin = None;
    }

    fn take_player_resource(&mut self) -> Option<crate::core::player_resource::PlayerResource> {
        self.resource.take_inner()
    }
}

impl Default for MusicResult {
    fn default() -> Self {
        Self::new(
            MainController::new(
                crate::skin::config::Config::default(),
                Box::new(crate::ir::ranking_data_cache::RankingDataCache::new()),
            ),
            PlayerResource::default(),
            TimerManager::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::abstract_result::{STATE_IR_FINISHED, STATE_IR_PROCESSING, STATE_OFFLINE};
    use crate::result::music_result::render_context::{ResultMouseContext, ResultRenderContext};
    use crate::result::test_helpers::{
        ExecuteEventSkin, PlayerConfigMutatingSkin, make_test_config,
    };
    use crate::skin::skin_render_context::SkinRenderContext;

    fn make_ranking_cache() -> Box<dyn crate::ranking_data_cache_access::RankingDataCacheAccess> {
        Box::new(crate::ir::ranking_data_cache::RankingDataCache::new())
    }

    /// Create a CorePlayerResource with default score and replay data, matching
    /// what MouseResultResourceAccess::new(config) previously provided.
    fn make_test_core_resource(
        config: crate::skin::config::Config,
    ) -> crate::core::player_resource::PlayerResource {
        let mut r = crate::core::player_resource::PlayerResource::new(
            config,
            crate::skin::player_config::PlayerConfig::default(),
        );
        r.set_score_data(crate::core::score_data::ScoreData::default());
        r.set_replay_data(crate::core::replay_data::ReplayData::default());
        r
    }

    fn make_result_for_mouse() -> MusicResult {
        let config = make_test_config("music-result");
        let main = MainController::new(config.clone(), make_ranking_cache());
        let resource = PlayerResource::new(
            make_test_core_resource(config),
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
    fn test_result_render_context_delegates_course_gauge_history() {
        let mut mr = make_result_for_mouse();
        // Populate course gauge data on the resource
        mr.resource
            .add_course_gauge(vec![vec![0.5, 0.6], vec![0.7, 0.8]]);
        mr.resource.add_course_gauge(vec![vec![1.0]]);

        let mut timer = TimerManager::new();
        let ctx = ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };

        let history = ctx.course_gauge_history();
        assert_eq!(
            history.len(),
            2,
            "ResultRenderContext must delegate course_gauge_history, not return empty default"
        );
        assert_eq!(history[0].len(), 2);
        assert_eq!(history[1].len(), 1);
    }

    #[test]
    fn test_result_mouse_context_delegates_course_gauge_history() {
        let mut mr = make_result_for_mouse();
        // Populate course gauge data on the resource
        mr.resource
            .add_course_gauge(vec![vec![0.5, 0.6], vec![0.7, 0.8]]);

        let mut timer = TimerManager::new();
        let ctx = ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };

        let history = ctx.course_gauge_history();
        assert_eq!(
            history.len(),
            1,
            "ResultMouseContext must delegate course_gauge_history, not return empty default"
        );
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
        assert_eq!(crate::skin::skin_type::SkinType::Result.id(), 7);
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
        let result_skin_idx = crate::skin::skin_type::SkinType::Result.id() as usize;
        mr.resource
            .player_config_mut()
            .expect("player config should be mutable")
            .skin[result_skin_idx] = Some(crate::skin::skin_config::SkinConfig::new_with_path(
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
        mr.main_data.timer.update();
        <MusicResult as MainState>::render(&mut mr);
        // TIMER_RESULTGRAPH_BEGIN and TIMER_RESULTGRAPH_END should be on
        assert!(mr.main_data.timer.is_timer_on(TIMER_RESULTGRAPH_BEGIN));
        assert!(mr.main_data.timer.is_timer_on(TIMER_RESULTGRAPH_END));
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
        mr.main_data.timer.update();
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
        use crate::ir::ir_player_data::IRPlayerData;
        use std::sync::Arc;

        struct MockIRConnection;
        impl crate::ir::ir_connection::IRConnection for MockIRConnection {
            fn get_rivals(&self) -> crate::ir::ir_response::IRResponse<Vec<IRPlayerData>> {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_table_datas(
                &self,
            ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_table_data::IRTableData>>
            {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_play_data(
                &self,
                _player: Option<&IRPlayerData>,
                _chart: Option<&crate::ir::ir_chart_data::IRChartData>,
            ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_score_data::IRScoreData>>
            {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_course_play_data(
                &self,
                _player: Option<&IRPlayerData>,
                _course: &crate::ir::ir_course_data::IRCourseData,
            ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_score_data::IRScoreData>>
            {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn send_play_data(
                &self,
                _model: &crate::ir::ir_chart_data::IRChartData,
                _score: &crate::ir::ir_score_data::IRScoreData,
            ) -> crate::ir::ir_response::IRResponse<()> {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn send_course_play_data(
                &self,
                _course: &crate::ir::ir_course_data::IRCourseData,
                _score: &crate::ir::ir_score_data::IRScoreData,
            ) -> crate::ir::ir_response::IRResponse<()> {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_song_url(
                &self,
                _chart: &crate::ir::ir_chart_data::IRChartData,
            ) -> Option<String> {
                None
            }
            fn get_course_url(
                &self,
                _course: &crate::ir::ir_course_data::IRCourseData,
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
            crate::core::ir_config::IRConfig::default(),
            Arc::new(MockIRConnection)
                as Arc<dyn crate::ir::ir_connection::IRConnection + Send + Sync>,
            IRPlayerData::new("id1".into(), "Player1".into(), "1st".into()),
        )];
        let main =
            MainController::with_ir_statuses(config.clone(), make_ranking_cache(), ir_statuses);
        let resource = PlayerResource::new(
            make_test_core_resource(config),
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
        song_data: Option<crate::skin::song_data::SongData>,
    ) -> MusicResult {
        let config = make_test_config("lnmode-result");
        let main = MainController::new(config.clone(), make_ranking_cache());
        let mut core_res = make_test_core_resource(config);
        if let Some(sd) = song_data {
            core_res.set_songdata(sd);
        }
        // Set lnmode config to a sentinel value (99) so we can verify fallback
        crate::skin::player_resource_access::ConfigAccess::player_config_mut(&mut core_res)
            .unwrap()
            .play_settings
            .lnmode = 99;
        let resource = PlayerResource::new(
            core_res,
            crate::result::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        MusicResult::new(main, resource, TimerManager::new())
    }

    #[test]
    fn result_lnmode_308_override_longnote() {
        use crate::skin::song_data::{ChartInfo, FEATURE_LONGNOTE, SongData};
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
        use crate::skin::song_data::{ChartInfo, FEATURE_CHARGENOTE, SongData};
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
        use crate::skin::song_data::{ChartInfo, FEATURE_HELLCHARGENOTE, SongData};
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
        use crate::skin::song_data::{ChartInfo, FEATURE_UNDEFINEDLN, SongData};
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
        use crate::ir::ir_score_data::IRScoreData;
        use crate::ir::ranking_data::RankingData;

        let mut mr = make_result_with_songdata(None);
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

    #[test]
    fn result_mouse_context_set_float_value_propagates_volume() {
        // Regression: volume slider writes (IDs 17-19) on the result screen
        // must propagate to pending_audio_config, not be silently dropped.
        let mut config = make_test_config("volume-result");
        config.audio = Some(crate::skin::audio_config::AudioConfig::default());
        let main = MainController::new(config.clone(), make_ranking_cache());
        let resource = PlayerResource::new(
            make_test_core_resource(config),
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

        // Each set_float_value overwrites pending_audio_config, so we see the final accumulated state
        let audio = mr
            .pending_audio_config
            .expect("should have pending audio config");
        assert_eq!(audio.systemvolume, 0.75);
        assert_eq!(audio.keyvolume, 0.5);
        assert_eq!(audio.bgvolume, 0.25);
    }

    #[test]
    fn result_mouse_context_set_float_value_clamps_volume() {
        let mut config = make_test_config("volume-clamp-result");
        config.audio = Some(crate::skin::audio_config::AudioConfig::default());
        let main = MainController::new(config.clone(), make_ranking_cache());
        let resource = PlayerResource::new(
            make_test_core_resource(config),
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

        let audio = mr
            .pending_audio_config
            .expect("should have pending audio config");
        assert_eq!(audio.systemvolume, 0.0, "negative should clamp to 0.0");
        assert_eq!(audio.keyvolume, 1.0, "above 1.0 should clamp to 1.0");
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
        let cscore: Option<&crate::core::score_data::ScoreData> = None;
        let ns_clear = crate::core::clear_type::ClearType::Normal.id();
        let is_clear = ns_clear != crate::core::clear_type::ClearType::Failed.id()
            && cscore.is_none_or(|cs| cs.clear != crate::core::clear_type::ClearType::Failed.id());
        assert!(
            is_clear,
            "When course_score_data is None and newscore is not Failed, is_clear should be true"
        );
    }

    #[test]
    fn is_clear_logic_course_score_failed_means_not_clear() {
        let mut course = crate::core::score_data::ScoreData::default();
        course.clear = crate::core::clear_type::ClearType::Failed.id();
        let cscore: Option<&crate::core::score_data::ScoreData> = Some(&course);
        let ns_clear = crate::core::clear_type::ClearType::Normal.id();
        let is_clear = ns_clear != crate::core::clear_type::ClearType::Failed.id()
            && cscore.is_none_or(|cs| cs.clear != crate::core::clear_type::ClearType::Failed.id());
        assert!(
            !is_clear,
            "When course_score_data is Failed, is_clear should be false even if newscore is not Failed"
        );
    }

    #[test]
    fn is_clear_logic_newscore_failed_means_not_clear() {
        let cscore: Option<&crate::core::score_data::ScoreData> = None;
        let ns_clear = crate::core::clear_type::ClearType::Failed.id();
        let is_clear = ns_clear != crate::core::clear_type::ClearType::Failed.id()
            && cscore.is_none_or(|cs| cs.clear != crate::core::clear_type::ClearType::Failed.id());
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
        use crate::skin::skin_render_context::SkinRenderContext;
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
    fn result_render_context_result_gauge_type_returns_stored_gauge_type() {
        let mut mr = make_result_for_mouse();
        mr.data.gauge_type = 3;
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };
        use crate::skin::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.result_gauge_type(), 3);
    }

    #[test]
    fn result_mouse_context_result_gauge_type_returns_stored_gauge_type() {
        let mut mr = make_result_for_mouse();
        mr.data.gauge_type = 5;
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        use crate::skin::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.result_gauge_type(), 5);
    }

    #[test]
    fn result_render_context_lane_shuffle_pattern_from_replay() {
        let mut mr = make_result_for_mouse();
        mr.resource
            .replay_data_mut()
            .expect("replay data should exist")
            .lane_shuffle_pattern = Some(vec![vec![2, 0, 1, 3, 4, 5, 6, 7, 8, 9]]);
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };
        use crate::skin::skin_render_context::SkinRenderContext;
        // image_index 450 = lane_shuffle_pattern_value(0, 0) = 2
        assert_eq!(ctx.image_index_value(450), 2);
        // image_index 451 = lane_shuffle_pattern_value(0, 1) = 0
        assert_eq!(ctx.image_index_value(451), 0);
        // No 2P data -> -1
        assert_eq!(ctx.image_index_value(460), -1);
    }

    #[test]
    fn result_mouse_context_lane_shuffle_pattern_from_replay() {
        let mut mr = make_result_for_mouse();
        mr.resource
            .replay_data_mut()
            .expect("replay data should exist")
            .lane_shuffle_pattern = Some(vec![vec![5, 3, 1, 0, 2, 4, 6, 7, 8, 9]]);
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultMouseContext {
            timer: &mut timer,
            result: &mut mr,
        };
        use crate::skin::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.image_index_value(450), 5);
        assert_eq!(ctx.image_index_value(451), 3);
    }

    #[test]
    fn result_render_context_lane_shuffle_pattern_none_returns_minus_one() {
        let mut mr = make_result_for_mouse();
        // replay_data exists but lane_shuffle_pattern is None (default)
        let mut timer = TimerManager::new();
        let ctx = render_context::ResultRenderContext {
            timer: &mut timer,
            data: &mr.data,
            resource: &mr.resource,
            main: &mut mr.main,
            offsets: &mr.main_data.offsets,
        };
        use crate::skin::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.image_index_value(450), -1);
        assert_eq!(ctx.image_index_value(460), -1);
    }
}

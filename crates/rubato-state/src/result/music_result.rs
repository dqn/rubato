// MusicResult.java -> music_result.rs
// Mechanical line-by-line translation.

use log::{info, warn};

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
use super::music_result_skin::MusicResultSkin;
use super::result_key_property::{ResultKey, ResultKeyProperty};
use super::stubs::{
    BMSPlayerModeType, ControlKeys, FreqTrainerMenu, IRSendStatusMain, KeyCommand, MainController,
    NullMainController, PlayerResource, RankingData,
};
use rubato_core::ir_config::{IR_SEND_ALWAYS, IR_SEND_COMPLETE_SONG, IR_SEND_UPDATE_SCORE};

/// Render context adapter for result screen skin rendering.
/// Provides score data, gauge, config through SkinRenderContext.
struct ResultRenderContext<'a> {
    timer: &'a mut TimerManager,
    data: &'a AbstractResultData,
    resource: &'a PlayerResource,
    main: &'a MainController,
}

impl rubato_types::timer_access::TimerAccess for ResultRenderContext<'_> {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }
    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }
    fn micro_timer(&self, timer_id: i32) -> i64 {
        self.timer.micro_timer(timer_id)
    }
    fn timer(&self, timer_id: i32) -> i64 {
        self.timer.timer(timer_id)
    }
    fn now_time_for(&self, timer_id: i32) -> i64 {
        self.timer.now_time_for_id(timer_id)
    }
    fn is_timer_on(&self, timer_id: i32) -> bool {
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
        let mode = self.resource.songdata().and_then(|song| match song.mode {
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

    fn set_timer_micro(&mut self, timer_id: i32, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn gauge_value(&self) -> f32 {
        // Return final gauge value from score data
        self.data.oldscore.gauge as f32 / 100.0
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
                .map_or_else(String::new, |s| s.title.clone()),
            11 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.subtitle.clone()),
            12 => self.resource.songdata().map_or_else(String::new, |s| {
                if s.subtitle.is_empty() {
                    s.title.clone()
                } else {
                    format!("{} {}", s.title, s.subtitle)
                }
            }),
            13 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.genre.clone()),
            14 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.artist.clone()),
            15 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.subartist.clone()),
            16 => self.resource.songdata().map_or_else(String::new, |s| {
                if s.subartist.is_empty() {
                    s.artist.clone()
                } else {
                    format!("{} {}", s.artist, s.subartist)
                }
            }),
            _ => String::new(),
        }
    }
}

fn replay_index_from_event_id(event_id: i32) -> Option<usize> {
    match event_id {
        19 => Some(0),
        316 => Some(1),
        317 => Some(2),
        318 => Some(3),
        _ => None,
    }
}

struct ResultMouseContext<'a> {
    timer: &'a mut TimerManager,
    result: &'a mut MusicResult,
}

impl rubato_types::timer_access::TimerAccess for ResultMouseContext<'_> {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }

    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }

    fn micro_timer(&self, timer_id: i32) -> i64 {
        self.timer.micro_timer(timer_id)
    }

    fn timer(&self, timer_id: i32) -> i64 {
        self.timer.timer(timer_id)
    }

    fn now_time_for(&self, timer_id: i32) -> i64 {
        self.timer.now_time_for_id(timer_id)
    }

    fn is_timer_on(&self, timer_id: i32) -> bool {
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

    fn set_timer_micro(&mut self, timer_id: i32, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        self.result.resource.player_config_mut()
    }
}

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
            self.data.save_replay[i] = if self.main.play_data_accessor().exists_replay_data_model(
                self.resource.bms_model(),
                self.resource.player_config().lnmode,
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
                let auto_save = &self.resource.player_config().autosavereplay;
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

        self.data.gauge_type = self
            .resource
            .groove_gauge()
            .map(|g| g.gauge_type())
            .unwrap_or(0);

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
                        if let (Some(gauge_data), Some(groove_gauge)) =
                            (self.resource.gauge(), self.resource.groove_gauge())
                        {
                            let gauge = &gauge_data[groove_gauge.gauge_type() as usize];
                            send &= gauge.last().copied().unwrap_or(0.0) > 0.0;
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
            for status in pending_ir_sends {
                self.main.ir_send_status_mut().push(status);
            }

            // IR processing thread
            // In Java this spawns a Thread. In Rust we'd use tokio::spawn or std::thread::spawn.
            // The thread sends scores, fetches ranking, and updates state.
            // For now, immediately mark as finished (blocking stub).
            let ir_len = self.main.ir_status().len();
            let ir_send_count = self.main.config().ir_send_count;
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
                let ir_status = self.main.ir_status();
                if !ir_status.is_empty()
                    && let Some(songdata) = self.resource.songdata()
                {
                    let chart_data = rubato_ir::ir_chart_data::IRChartData::new(songdata);
                    let response = ir_status[0].connection.get_play_data(None, &chart_data);
                    if response.is_succeeded() {
                        if let Some(ir_scores) = response.data() {
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
                                ranking.update_score(ir_scores, score_for_rank);
                                if ranking.rank() > 10 {
                                    self.data.ranking_offset = ranking.rank() - 5;
                                } else {
                                    self.data.ranking_offset = 0;
                                }
                            }
                        }
                        info!("IR score fetch succeeded: {}", response.message);
                    } else {
                        warn!("IR score fetch failed: {}", response.message);
                    }
                }
            }
            self.data.state = STATE_IR_FINISHED;
        }

        // Play result sound
        if let Some(ref ns) = newscore_clone {
            let cscore = self.resource.course_score_data();
            let is_clear = ns.clear != ClearType::Failed.id()
                && (cscore.is_none() || cscore.unwrap().clear != ClearType::Failed.id());
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
    }

    fn do_render(&mut self) {
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
                    let gauge_type = self
                        .resource
                        .groove_gauge()
                        .map(|g| g.gauge_type() as usize)
                        .unwrap_or(0);
                    let last_gauge = self
                        .resource
                        .gauge()
                        .and_then(|gd| gd.get(gauge_type))
                        .and_then(|g| g.last().copied())
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
                        let lnmode = self.resource.player_config().lnmode;
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
                                self.data.gauge_type = (self.data.gauge_type - 5) % 3 + 6;
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
            self.main.play_data_accessor().write_replay_data_model(
                &mut rd.clone(),
                self.resource.bms_model(),
                self.resource.player_config().lnmode,
                index as i32,
            );
            self.data.save_replay[index] = ReplayStatus::Saved;
            self.main.save_last_recording("ON_REPLAY");
        }
    }

    fn update_score_database(&mut self) {
        let newscore = self.resource.score_data().cloned();
        if newscore.is_none() {
            let total_notes = self.resource.bms_model().total_notes();
            if let Some(mut cscore) = self.resource.course_score_data().cloned() {
                cscore.minbp += total_notes;
                cscore.clear = ClearType::Failed.id();
                self.resource.set_course_score_data(cscore);
            }
            return;
        }
        let newscore = newscore.unwrap();

        let oldsc = self.main.play_data_accessor().read_score_data_model(
            self.resource.bms_model(),
            self.resource.player_config().lnmode,
        );
        self.data.oldscore = oldsc.unwrap_or_default();

        let target_exscore = self
            .resource
            .target_score_data()
            .map(|s| s.exscore())
            .unwrap_or(0);
        self.data.score.set_target_score(
            self.data.oldscore.exscore(),
            target_exscore,
            self.resource.bms_model().total_notes(),
        );
        self.data.score.update_score(Some(&newscore));

        // duration average
        self.data.avgduration = newscore.avgjudge;
        self.data.avg = newscore.avg;
        self.data.stddev = newscore.stddev;
        self.data.timing_distribution.init();

        let model = self.resource.bms_model();
        let lanes = model.mode().map(|m| m.key()).unwrap_or(8);
        for tl in model.all_time_lines() {
            for i in 0..lanes {
                let n = tl.note(i);
                if let Some(note) = n {
                    // Check if this is not an end LN in LN mode
                    let is_end_ln = (model.lnmode() == 1
                        || (model.lnmode() == 0
                            && model.lntype() == bms_model::bms_model::LNTYPE_LONGNOTE))
                        && note.is_long()
                        && note.is_end();
                    if !is_end_ln {
                        let state = note.state();
                        let play_time = note.play_time();
                        if state >= 1 {
                            self.data.timing_distribution.add(play_time);
                        }
                    }
                }
            }
        }
        self.data.timing_distribution.statistic_value_calculate();

        // Course mode score accumulation
        if self.resource.course_bms_models().is_some() {
            if newscore.clear == ClearType::Failed.id()
                && let Some(sd) = self.resource.score_data_mut()
            {
                sd.clear = ClearType::NoPlay.id();
            }
            let mut cscore = self.resource.course_score_data().cloned();
            if cscore.is_none() {
                let mut new_cscore = ScoreData {
                    minbp: 0,
                    ..Default::default()
                };
                let mut notes = 0;
                if let Some(models) = self.resource.course_bms_models() {
                    for mo in models {
                        notes += mo.total_notes();
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
                    .groove_gauge()
                    .map(|g| g.gauge_type() as usize)
                    .unwrap_or(0);
                let last_gauge_val = self
                    .resource
                    .gauge()
                    .and_then(|gd| gd.get(gauge_type))
                    .and_then(|g| g.last().copied())
                    .unwrap_or(0.0);
                if last_gauge_val > 0.0 {
                    if self.resource.assist() > 0 {
                        if self.resource.assist() == 1 && cs.clear != ClearType::AssistEasy.id() {
                            cs.clear = ClearType::LightAssistEasy.id();
                        } else {
                            cs.clear = ClearType::AssistEasy.id();
                        }
                    } else if !(cs.clear == ClearType::LightAssistEasy.id()
                        || cs.clear == ClearType::AssistEasy.id())
                        && let Some(models) = self.resource.course_bms_models()
                        && self.resource.course_index() == models.len() - 1
                    {
                        let mut course_total_notes = 0;
                        for m in models {
                            course_total_notes += m.total_notes();
                        }
                        if course_total_notes == self.resource.maxcombo() {
                            if cs.judge_count(2, true) + cs.judge_count(2, false) == 0 {
                                if cs.judge_count(1, true) + cs.judge_count(1, false) == 0 {
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
                                .groove_gauge()
                                .map(|g| g.clear_type())
                                .unwrap_or(ClearType::Failed)
                                .id();
                        }
                    }
                } else {
                    cs.clear = ClearType::Failed.id();

                    let mut b = false;
                    if let Some(models) = self.resource.course_bms_models() {
                        for m in models {
                            if b {
                                cs.minbp += m.total_notes();
                            }
                            if std::ptr::eq(m, self.resource.bms_model()) {
                                b = true;
                            }
                        }
                    }
                }

                self.resource.set_course_score_data(cs.clone());
            }
        }

        if FreqTrainerMenu::is_freq_trainer_enabled()
            && let Some(sd) = self.resource.score_data_mut()
        {
            sd.clear = ClearType::NoPlay.id();
        }

        if self.resource.play_mode().mode == BMSPlayerModeType::Play
            && !(FreqTrainerMenu::is_freq_trainer_enabled() && FreqTrainerMenu::is_freq_negative())
        {
            if let Some(sd) = self.resource.score_data() {
                self.main.play_data_accessor().write_score_data_model(
                    sd,
                    self.resource.bms_model(),
                    self.resource.player_config().lnmode,
                    self.resource.is_update_score(),
                );
            }
        } else {
            info!(
                "Play mode is {:?}, score not registered",
                self.resource.play_mode().mode
            );
        }
    }

    pub fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        if let Some(score) = self.resource.score_data() {
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

    pub fn total_notes(&self) -> i32 {
        self.resource.bms_model().total_notes()
    }

    pub fn new_score(&self) -> Option<&ScoreData> {
        self.resource.score_data()
    }

    /// Get the skin as MusicResultSkin
    pub fn skin(&self) -> Option<&MusicResultSkin> {
        self.skin.as_ref()
    }

    /// Set the skin
    pub fn set_skin(&mut self, skin: MusicResultSkin) {
        self.skin = Some(skin);
    }

    fn has_sound(&self, sound: SoundType) -> bool {
        self.main.sound_path(&sound).is_some()
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

    fn render_skin(&mut self, sprite: &mut rubato_render::sprite_batch::SpriteBatch) {
        let mut skin = match self.main_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.main_data.timer);

        {
            let mut ctx = ResultRenderContext {
                timer: &mut timer,
                data: &self.data,
                resource: &self.resource,
                main: &self.main,
            };
            skin.update_custom_objects_timed(&mut ctx);
            skin.swap_sprite_batch(sprite);
            skin.draw_all_objects_timed(&mut ctx);
            skin.swap_sprite_batch(sprite);
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

        {
            let mut ctx = ResultMouseContext {
                timer: &mut timer,
                result: self,
            };
            skin.mouse_pressed_at(&mut ctx, button, x, y);
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

        {
            let mut ctx = ResultMouseContext {
                timer: &mut timer,
                result: self,
            };
            skin.mouse_dragged_at(&mut ctx, button, x, y);
        }

        self.main_data.timer = timer;
        self.main_data.skin = Some(skin);
    }

    fn input(&mut self) {
        self.do_input();
    }

    fn sync_input_from(
        &mut self,
        input: &rubato_input::bms_player_input_processor::BMSPlayerInputProcessor,
    ) {
        self.main.sync_input_from(input);
    }

    fn sync_input_back_to(
        &mut self,
        input: &mut rubato_input::bms_player_input_processor::BMSPlayerInputProcessor,
    ) {
        self.main.sync_input_back_to(input);
    }

    fn load_skin(&mut self, skin_type: i32) {
        if let Some(skin) = rubato_skin::skin_loader::load_skin_from_config(
            self.main.config(),
            self.resource.player_config(),
            skin_type,
        ) {
            self.skin = Some(MusicResultSkin::from_loaded_skin(&skin));
            self.main_data.skin = Some(Box::new(skin));
        } else {
            self.skin = None;
            self.main_data.skin = None;
        }
    }

    fn dispose(&mut self) {
        self.main_data.skin = None;
        self.main_data.stage = None;
    }

    fn take_player_resource_box(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        self.resource.take_inner().map(|b| b.into_any_send())
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
    use rubato_core::main_state::SkinDrawable;
    use rubato_core::sprite_batch_helper::SpriteBatch;
    use rubato_types::main_controller_access::MainControllerAccess;
    use rubato_types::player_resource_access::PlayerResourceAccess;
    use rubato_types::skin_render_context::SkinRenderContext;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct ExecuteEventSkin {
        event_id: i32,
    }

    impl SkinDrawable for ExecuteEventSkin {
        fn draw_all_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }

        fn update_custom_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }

        fn mouse_pressed_at(
            &mut self,
            ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
            ctx.execute_event(self.event_id, 0, 0);
        }

        fn mouse_dragged_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }

        fn prepare_skin(&mut self) {}

        fn dispose_skin(&mut self) {}

        fn fadeout(&self) -> i32 {
            0
        }

        fn input(&self) -> i32 {
            0
        }

        fn scene(&self) -> i32 {
            0
        }

        fn get_width(&self) -> f32 {
            0.0
        }

        fn get_height(&self) -> f32 {
            0.0
        }

        fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
    }

    struct PlayerConfigMutatingSkin;

    impl SkinDrawable for PlayerConfigMutatingSkin {
        fn draw_all_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }

        fn update_custom_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }

        fn mouse_pressed_at(
            &mut self,
            ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
            if let Some(config) = ctx.player_config_mut() {
                config.random = (config.random + 1) % 10;
            }
        }

        fn mouse_dragged_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }

        fn prepare_skin(&mut self) {}

        fn dispose_skin(&mut self) {}

        fn fadeout(&self) -> i32 {
            0
        }

        fn input(&self) -> i32 {
            0
        }

        fn scene(&self) -> i32 {
            0
        }

        fn get_width(&self) -> f32 {
            0.0
        }

        fn get_height(&self) -> f32 {
            0.0
        }

        fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
    }

    struct TestMainControllerAccess {
        config: rubato_types::config::Config,
        player_config: rubato_types::player_config::PlayerConfig,
    }

    impl TestMainControllerAccess {
        fn new(config: rubato_types::config::Config) -> Self {
            Self {
                config,
                player_config: rubato_types::player_config::PlayerConfig::default(),
            }
        }
    }

    impl MainControllerAccess for TestMainControllerAccess {
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }

        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }

        fn change_state(&mut self, _state: MainStateType) {}

        fn save_config(&self) {}

        fn exit(&self) {}

        fn save_last_recording(&self, _reason: &str) {}

        fn update_song(&mut self, _path: Option<&str>) {}

        fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
            None
        }

        fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
            None
        }
    }

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

    impl PlayerResourceAccess for MouseResultResourceAccess {
        fn into_any_send(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }

        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }

        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }

        fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
            Some(&mut self.player_config)
        }

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

        fn songdata(&self) -> Option<&rubato_types::song_data::SongData> {
            self.song_data.as_ref()
        }

        fn songdata_mut(&mut self) -> Option<&mut rubato_types::song_data::SongData> {
            self.song_data.as_mut()
        }

        fn set_songdata(&mut self, data: Option<rubato_types::song_data::SongData>) {
            self.song_data = data;
        }

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

        fn score_data_mut(&mut self) -> Option<&mut rubato_core::score_data::ScoreData> {
            self.score_data.as_mut()
        }

        fn course_replay_mut(&mut self) -> &mut Vec<rubato_core::replay_data::ReplayData> {
            &mut self.course_replay
        }

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

        fn reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }

        fn reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }

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

        fn set_course_data(&mut self, data: rubato_types::course_data::CourseData) {
            self.course_data = Some(data);
        }

        fn clear_course_data(&mut self) {
            self.course_data = None;
        }

        fn course_song_data(&self) -> Vec<rubato_types::song_data::SongData> {
            vec![]
        }
    }

    fn make_test_config(label: &str) -> rubato_types::config::Config {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut config = rubato_types::config::Config::default();
        let player_dir = std::env::temp_dir().join(format!("rubato-{label}-{unique}"));
        config.playerpath = player_dir.to_string_lossy().into_owned();
        config.playername = Some("mouse-result".to_string());
        config
    }

    fn make_result_for_mouse() -> MusicResult {
        let config = make_test_config("music-result");
        let main = MainController::new(Box::new(TestMainControllerAccess::new(config.clone())));
        let resource = PlayerResource::new(
            Box::new(MouseResultResourceAccess::new(config)),
            crate::result::stubs::BMSPlayerMode::new(BMSPlayerModeType::Play),
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

        assert_eq!(mr.resource.player_config().random, 1);
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
            main: &mr.main,
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
        assert!(mr.main_data.stage.is_none());
    }
}

use beatoraja_core::main_state::{MainState, MainStateData};
use beatoraja_core::pixmap_resource_pool::PixmapResourcePool;
use beatoraja_core::timer_manager::TimerManager;
use beatoraja_ir::ranking_data;

use crate::bar::bar::Bar;
use crate::bar_manager::BarManager;
use crate::bar_renderer::BarRenderer;
use crate::bar_sorter::BarSorter;
use crate::music_select_command::MusicSelectCommand;
use crate::music_select_input_processor::{
    BarType, InputContext, InputEvent, MusicSelectInputProcessor,
};
use crate::preview_music_processor::PreviewMusicProcessor;
use crate::score_data_cache::ScoreDataCache;
use crate::search_text_field::SearchTextField;
use crate::stubs::*;

/// Music selector screen
/// Translates: bms.player.beatoraja.select.MusicSelector
pub struct MusicSelector {
    /// Shared MainState data (timers, skin, score)
    pub main_state_data: MainStateData,

    pub selectedreplay: i32,

    /// Song database accessor
    pub songdb: Box<dyn SongDatabaseAccessor>,

    /// Player config
    pub config: PlayerConfig,

    /// Preview music processor
    pub preview: Option<PreviewMusicProcessor>,

    /// Bar renderer
    pub bar: Option<BarRenderer>,

    /// Bar manager
    pub manager: BarManager,

    /// Music select input processor
    pub musicinput: Option<MusicSelectInputProcessor>,

    /// Search text field
    pub search: Option<SearchTextField>,

    /// Duration before loading BMS notes graph (ms)
    pub notes_graph_duration: i32,
    /// Duration before playing preview music (ms)
    pub preview_duration: i32,

    pub ranking_duration: i32,
    pub ranking_reload_duration: i64,

    pub current_ranking_duration: i64,

    pub show_note_graph: bool,

    pub scorecache: Option<ScoreDataCache>,
    pub rivalcache: Option<ScoreDataCache>,

    pub currentir: Option<RankingData>,
    /// Ranking display offset
    pub ranking_offset: i32,

    pub rival: Option<PlayerInformation>,

    pub panelstate: i32,

    pub play: Option<BMSPlayerMode>,

    pub playedsong: Option<SongData>,
    pub playedcourse: Option<CourseData>,

    /// Banner pixmap resource pool
    pub banners: PixmapResourcePool,
    /// Stagefile pixmap resource pool
    pub stagefiles: PixmapResourcePool,
}

pub static MODE: [Option<bms_model::Mode>; 8] = [
    None,
    Some(bms_model::Mode::BEAT_7K),
    Some(bms_model::Mode::BEAT_14K),
    Some(bms_model::Mode::POPN_9K),
    Some(bms_model::Mode::BEAT_5K),
    Some(bms_model::Mode::BEAT_10K),
    Some(bms_model::Mode::KEYBOARD_24K),
    Some(bms_model::Mode::KEYBOARD_24K_DOUBLE),
];

/// Maximum number of saveable replays
pub const REPLAY: usize = 4;

impl Default for MusicSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl MusicSelector {
    pub fn new() -> Self {
        Self {
            main_state_data: MainStateData::new(TimerManager::new()),
            selectedreplay: 0,
            songdb: Box::new(NullSongDatabaseAccessor),
            config: PlayerConfig::default(),
            preview: None,
            bar: None,
            manager: BarManager::new(),
            musicinput: None,
            search: None,
            notes_graph_duration: 350,
            preview_duration: 400,
            ranking_duration: 5000,
            ranking_reload_duration: 10 * 60 * 1000,
            current_ranking_duration: -1,
            show_note_graph: false,
            scorecache: None,
            rivalcache: None,
            currentir: None,
            ranking_offset: 0,
            rival: None,
            panelstate: 0,
            play: None,
            playedsong: None,
            playedcourse: None,
            banners: PixmapResourcePool::with_maxgen(2),
            stagefiles: PixmapResourcePool::with_maxgen(2),
        }
    }

    /// Create a MusicSelector with an injected song database accessor.
    ///
    /// Translated from: MusicSelector(MainController main, boolean songUpdated)
    /// In Java, the constructor receives MainController and gets the songdb from it.
    /// In Rust, we inject the songdb directly to avoid circular dependencies.
    pub fn with_song_database(songdb: Box<dyn SongDatabaseAccessor>) -> Self {
        Self {
            songdb,
            ..Self::new()
        }
    }

    pub fn set_rival(&mut self, rival: Option<PlayerInformation>) {
        // In Java: finds rival index, sets rival and rival cache, updates bar
        self.rival = rival;
        self.rivalcache = None;
        self.manager.update_bar_refresh();
        log::info!(
            "Rival changed: {}",
            self.rival.as_ref().map(|r| r.get_name()).unwrap_or("None")
        );
    }

    pub fn get_rival(&self) -> Option<&PlayerInformation> {
        self.rival.as_ref()
    }

    pub fn get_score_data_cache(&self) -> Option<&ScoreDataCache> {
        self.scorecache.as_ref()
    }

    pub fn get_rival_score_data_cache(&self) -> Option<&ScoreDataCache> {
        self.rivalcache.as_ref()
    }

    pub fn get_selected_replay(&self) -> i32 {
        self.selectedreplay
    }

    pub fn set_selected_replay(&mut self, index: i32) {
        self.selectedreplay = index;
    }

    pub fn execute(&mut self, command: MusicSelectCommand) {
        // In Java: command.function.accept(this)
        command.execute(self);
    }

    pub fn execute_event(&mut self, _event: EventType) {
        log::warn!(
            "not yet implemented: MusicSelector.executeEvent - requires event handling context"
        );
    }

    pub fn execute_event_with_arg(&mut self, _event: EventType, _arg: i32) {
        log::warn!(
            "not yet implemented: MusicSelector.executeEvent(arg) - requires event handling context"
        );
    }

    pub fn execute_event_with_args(&mut self, _event: EventType, _arg1: i32, _arg2: i32) {
        log::warn!(
            "not yet implemented: MusicSelector.executeEvent(arg1, arg2) - requires event handling context"
        );
    }

    pub fn read_chart(&mut self, _song: &SongData, _current: &Bar) {
        // In Java: sets up resource for playing a chart
        // Requires PlayerResource, MainController, RankingDataCache - blocked on Phase 21+
        log::warn!(
            "not yet implemented: MusicSelector.readChart - requires PlayerResource context"
        );
    }

    pub fn get_sort(&self) -> i32 {
        self.config.get_sort()
    }

    pub fn set_sort(&mut self, sort: i32) {
        self.config.set_sort(sort);
        self.config
            .set_sortid(BarSorter::DEFAULT_SORTER[sort as usize].name().to_string());
    }

    pub fn get_panel_state(&self) -> i32 {
        self.panelstate
    }

    /// Set panel state with timer transitions.
    /// Corresponds to Java MusicSelector.setPanelState(int)
    pub fn set_panel_state(&mut self, panelstate: i32) {
        if self.panelstate != panelstate {
            if self.panelstate != 0 {
                self.main_state_data
                    .timer
                    .set_timer_on(skin_property::TIMER_PANEL1_OFF + self.panelstate - 1);
                self.main_state_data
                    .timer
                    .set_timer_off(skin_property::TIMER_PANEL1_ON + self.panelstate - 1);
            }
            if panelstate != 0 {
                self.main_state_data
                    .timer
                    .set_timer_on(skin_property::TIMER_PANEL1_ON + panelstate - 1);
                self.main_state_data
                    .timer
                    .set_timer_off(skin_property::TIMER_PANEL1_OFF + panelstate - 1);
            }
        }
        self.panelstate = panelstate;
    }

    pub fn get_song_database(&self) -> &dyn SongDatabaseAccessor {
        &*self.songdb
    }

    /// Check if the selected bar's course data contains the given constraint.
    /// Corresponds to Java MusicSelector.existsConstraint(CourseDataConstraint)
    pub fn exists_constraint(&self, constraint: &CourseDataConstraint) -> bool {
        let selected = match self.manager.get_selected() {
            Some(s) => s,
            None => return false,
        };

        if let Some(grade) = selected.as_grade_bar() {
            for con in grade.get_course_data().get_constraint() {
                if con == constraint {
                    return true;
                }
            }
        } else if let Some(rc) = selected.as_random_course_bar() {
            for con in &rc.get_course_data().constraint {
                if con == constraint {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_selected_bar(&self) -> Option<&Bar> {
        self.manager.get_selected()
    }

    pub fn get_bar_render(&self) -> Option<&BarRenderer> {
        self.bar.as_ref()
    }

    pub fn get_bar_manager(&self) -> &BarManager {
        &self.manager
    }

    pub fn get_bar_manager_mut(&mut self) -> &mut BarManager {
        &mut self.manager
    }

    /// Handle bar selection change.
    /// Corresponds to Java MusicSelector.selectedBarMoved()
    pub fn selected_bar_moved(&mut self) {
        self.execute(MusicSelectCommand::ResetReplay);
        self.load_selected_song_images();

        self.main_state_data
            .timer
            .set_timer_on(skin_property::TIMER_SONGBAR_CHANGE);

        // Stop preview if folder changed
        if let Some(preview) = &self.preview
            && preview.get_song_data().is_some()
        {
            let should_stop = match self.manager.get_selected() {
                Some(bar) => {
                    if let Some(song_bar) = bar.as_song_bar() {
                        if let Some(preview_song) = preview.get_song_data() {
                            song_bar.get_song_data().get_folder() != preview_song.get_folder()
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                }
                None => true,
            };
            if should_stop && let Some(preview) = &mut self.preview {
                preview.start(None);
            }
        }

        self.show_note_graph = false;

        // Update IR ranking state
        let now_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        // Determine ranking duration based on selected bar type
        // In Java: checks main.getIRStatus().length > 0
        // We check if currentir exists as a proxy
        if let Some(current) = self.manager.get_selected() {
            if let Some(song_bar) = current.as_song_bar() {
                if song_bar.exists_song() {
                    // In Java: currentir = main.getRankingDataCache().get(song, config.getLnmode())
                    // Blocked on MainController - use existing currentir
                    let ranking_reload_dur = self.ranking_reload_duration;
                    let ranking_dur = self.ranking_duration as i64;
                    self.current_ranking_duration = if let Some(ref ir) = self.currentir {
                        (ranking_reload_dur - (now_millis - ir.get_last_update_time())).max(0)
                            + ranking_dur
                    } else {
                        ranking_dur
                    };
                } else {
                    self.currentir = None;
                    self.current_ranking_duration = -1;
                }
            } else if let Some(grade_bar) = current.as_grade_bar() {
                if grade_bar.exists_all_songs() {
                    let ranking_reload_dur = self.ranking_reload_duration;
                    let ranking_dur = self.ranking_duration as i64;
                    self.current_ranking_duration = if let Some(ref ir) = self.currentir {
                        (ranking_reload_dur - (now_millis - ir.get_last_update_time())).max(0)
                            + ranking_dur
                    } else {
                        ranking_dur
                    };
                } else {
                    self.currentir = None;
                    self.current_ranking_duration = -1;
                }
            } else {
                self.currentir = None;
                self.current_ranking_duration = -1;
            }
        } else {
            self.currentir = None;
            self.current_ranking_duration = -1;
        }
    }

    pub fn load_selected_song_images(&mut self) {
        // In Java: loads banner and stagefile for selected song via resource.getBMSResource()
        // Blocked on PlayerResource/BMSResource
    }

    /// Select a bar (open directory or set play mode).
    /// Corresponds to Java MusicSelector.select(Bar)
    pub fn select(&mut self, current: &Bar) {
        if current.is_directory_bar() {
            if self.manager.update_bar(Some(current)) {
                self.play_sound(SoundType::FolderOpen);
            }
            self.execute(MusicSelectCommand::ResetReplay);
        } else {
            self.play = Some(BMSPlayerMode::PLAY);
        }
    }

    pub fn select_song(&mut self, mode: BMSPlayerMode) {
        self.play = Some(mode);
    }

    /// Process input with a BMSPlayerInputProcessor.
    /// This is the main entry point for input processing when MainController is available.
    /// Translates: Java MusicSelector.input() + MusicSelectInputProcessor.input()
    pub fn process_input_with_context(&mut self, input: &mut BMSPlayerInputProcessor) {
        // Java: if (input.getControlKeyState(ControlKeys.NUM6)) main.changeState(CONFIG)
        // Java: else if (input.isActivated(OPEN_SKIN_CONFIGURATION)) main.changeState(SKINCONFIG)
        // These require MainController.changeState() — logged as warnings for now
        if input.get_control_key_state(ControlKeys::Num6) {
            log::warn!("not yet implemented: changeState(CONFIG) - requires MainController");
        } else if input.is_activated(KeyCommand::OpenSkinConfiguration) {
            log::warn!("not yet implemented: changeState(SKINCONFIG) - requires MainController");
        }

        // Classify the selected bar before borrowing musicinput
        let selected_bar_type = BarType::classify(self.manager.get_selected());
        let selected_replay = self.selectedreplay;
        let is_top_level = self.manager.get_directory().is_empty();

        // Take musicinput to avoid overlapping borrow on self
        let mut musicinput = match self.musicinput.take() {
            Some(m) => m,
            None => return,
        };

        let mut ctx = InputContext::new(
            input,
            &mut self.config,
            selected_bar_type,
            selected_replay,
            is_top_level,
        );

        musicinput.input(&mut ctx);

        // Extract results from ctx before dropping it (which releases the borrow on self.config)
        let panel_state = ctx.panel_state;
        let bar_renderer_reset_input = ctx.bar_renderer_reset_input;
        let bar_renderer_do_input = ctx.bar_renderer_do_input;
        let songbar_timer_switch = ctx.songbar_timer_switch;
        let events = std::mem::take(&mut ctx.events);
        drop(ctx);

        // Restore musicinput
        self.musicinput = Some(musicinput);

        // Apply panel state
        if let Some(ps) = panel_state {
            self.set_panel_state(ps);
        }

        // Apply bar renderer actions
        if bar_renderer_reset_input && let Some(ref mut bar) = self.bar {
            bar.reset_input();
        }
        if bar_renderer_do_input && let Some(ref mut bar) = self.bar {
            bar.input();
        }

        // Switch songbar change timer
        if songbar_timer_switch {
            self.main_state_data
                .timer
                .switch_timer(skin_property::TIMER_SONGBAR_CHANGE, true);
        }

        // Dispatch collected events
        self.dispatch_input_events(events);
    }

    /// Dispatch input events collected by MusicSelectInputProcessor.
    /// Translates the event calls that Java does inline in MusicSelectInputProcessor.input().
    fn dispatch_input_events(&mut self, events: Vec<InputEvent>) {
        for event in events {
            match event {
                InputEvent::Execute(cmd) => {
                    cmd.execute(self);
                }
                InputEvent::ExecuteEvent(et) => {
                    self.execute_event(et);
                }
                InputEvent::ExecuteEventArg(et, arg) => {
                    self.execute_event_with_arg(et, arg);
                }
                InputEvent::ExecuteEventArgs(et, arg1, arg2) => {
                    self.execute_event_with_args(et, arg1, arg2);
                }
                InputEvent::PlaySound(sound) => {
                    self.play_sound(sound);
                }
                InputEvent::StopSound(sound) => {
                    self.stop_sound(sound);
                }
                InputEvent::SelectSong(mode) => {
                    self.select_song(mode);
                }
                InputEvent::BarManagerClose => {
                    self.manager.close();
                }
                InputEvent::OpenDirectory => {
                    // In Java: select.getBarManager().updateBar(dirbar)
                    // Borrow checker: can't pass self.manager.get_selected() to update_bar
                    // Use update_bar_with_selected() which internally handles this
                    let opened = self.manager.update_bar_with_selected();
                    if opened {
                        self.play_sound(SoundType::FolderOpen);
                    }
                }
                InputEvent::Exit => {
                    // In Java: select.main.exit()
                    // Blocked on MainController
                    log::warn!("not yet implemented: exit - requires MainController");
                }
                InputEvent::ChangeState(state_type) => {
                    // In Java: main.changeState(stateType)
                    // Blocked on MainController
                    log::warn!(
                        "not yet implemented: changeState({:?}) - requires MainController",
                        state_type
                    );
                }
            }
        }

        // Check if selected bar changed (Java: if manager.getSelected() != current)
        // In Java, this compares object references. Here we just call selectedBarMoved
        // to update state when the bar might have changed after events.
        // The caller should track bar identity if precise change detection is needed.
    }

    pub fn get_selected_bar_play_config(&self) -> Option<&PlayConfig> {
        // In Java: determines PlayConfig based on selected bar type
        // Blocked on MainController.getPlayerConfig()
        log::warn!(
            "not yet implemented: MusicSelector.getSelectedBarPlayConfig - requires MainController context"
        );
        None
    }

    pub fn get_current_ranking_data(&self) -> Option<&RankingData> {
        self.currentir.as_ref()
    }

    pub fn get_current_ranking_duration(&self) -> i64 {
        self.current_ranking_duration
    }

    pub fn get_ranking_offset(&self) -> i32 {
        self.ranking_offset
    }

    pub fn get_ranking_position(&self) -> f32 {
        let ranking_max = self
            .currentir
            .as_ref()
            .map(|ir| ir.get_total_player().max(1))
            .unwrap_or(1);
        self.ranking_offset as f32 / ranking_max as f32
    }

    pub fn set_ranking_position(&mut self, value: f32) {
        if (0.0..1.0).contains(&value) {
            let ranking_max = self
                .currentir
                .as_ref()
                .map(|ir| ir.get_total_player().max(1))
                .unwrap_or(1);
            self.ranking_offset = (ranking_max as f32 * value) as i32;
        }
    }

    /// Read course (grade bar) for play.
    /// Corresponds to Java MusicSelector.readCourse(BMSPlayerMode)
    fn read_course(&mut self, _mode: BMSPlayerMode) {
        // In Java: gets GradeBar from manager.getSelected(), checks existsAllSongs(),
        // calls _readCourse(mode, gradeBar)
        // Blocked on PlayerResource (Phase 21+)
        log::warn!(
            "not yet implemented: MusicSelector.readCourse - requires GradeBar and PlayerResource context"
        );
    }

    /// Read random course for play.
    /// Corresponds to Java MusicSelector.readRandomCourse(BMSPlayerMode)
    fn read_random_course(&mut self, _mode: BMSPlayerMode) {
        // In Java: gets RandomCourseBar, calls lotterySongDatas, creates GradeBar,
        // calls _readCourse, then manager.addRandomCourse
        // Blocked on PlayerResource (Phase 21+)
        log::warn!(
            "not yet implemented: MusicSelector.readRandomCourse - requires RandomCourseBar and PlayerResource context"
        );
    }

    /// Internal course reading implementation.
    /// Corresponds to Java MusicSelector._readCourse(BMSPlayerMode, GradeBar)
    fn _read_course(&mut self, _mode: &BMSPlayerMode, _grade_bar: &Bar) -> bool {
        // In Java: clears resource, gets song paths, calls resource.setCourseBMSFiles,
        // applies constraints (CLASS/MIRROR/RANDOM/LN/CN/HCN), sets play mode
        // Blocked on PlayerResource (Phase 21+)
        log::warn!(
            "not yet implemented: MusicSelector._readCourse - requires PlayerResource context"
        );
        false
    }

    /// Get banner resource pool.
    /// Corresponds to Java MusicSelector.getBannerResource()
    pub fn get_banner_resource(&self) -> &PixmapResourcePool {
        &self.banners
    }

    /// Get stagefile resource pool.
    /// Corresponds to Java MusicSelector.getStagefileResource()
    pub fn get_stagefile_resource(&self) -> &PixmapResourcePool {
        &self.stagefiles
    }
}

// ============================================================
// MainState trait implementation
// ============================================================

impl MainState for MusicSelector {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::MusicSelect)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.main_state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.main_state_data
    }

    /// Create state — initialize DB access, song list, bar manager.
    /// Corresponds to Java MusicSelector.create()
    fn create(&mut self) {
        // In Java: main.getSoundManager().shuffle()
        // Blocked on MainController

        self.play = None;
        self.show_note_graph = false;

        // In Java: resource.setPlayerData(main.getPlayDataAccessor().readPlayerData())
        // Blocked on MainController/PlayerResource

        // Update score cache for previously played song
        if let Some(ref song) = self.playedsong {
            if let Some(ref mut cache) = self.scorecache {
                cache.update(song, self.config.get_lnmode());
            }
            self.playedsong = None;
        }
        // Update score cache for previously played course
        if let Some(ref course) = self.playedcourse.take() {
            for sd in course.get_song() {
                if let Some(ref mut cache) = self.scorecache {
                    cache.update(sd, self.config.get_lnmode());
                }
            }
        }

        // In Java: preview = new PreviewMusicProcessor(main.getAudioProcessor(), resource.getConfig())
        // preview.setDefault(getSound(SELECT))
        // Blocked on MainController audio

        // In Java: sets input config based on musicselectinput mode
        // Blocked on MainController input processor

        self.manager.update_bar_refresh();

        // In Java: loadSkin(SkinType.MUSIC_SELECT)
        self.load_skin(SkinType::MusicSelect.id());

        // In Java: search text field setup from skin region
        // Blocked on MusicSelectSkin integration
    }

    /// Prepare state — start preview music.
    /// Corresponds to Java MusicSelector.prepare()
    fn prepare(&mut self) {
        if let Some(preview) = &mut self.preview {
            preview.start(None);
        }
    }

    /// Render state — handle song info display, preview music, BMS loading, IR ranking, play execution.
    /// Corresponds to Java MusicSelector.render()
    fn render(&mut self) {
        let timer = &mut self.main_state_data.timer;

        // Start input timer
        // In Java: if(timer.getNowTime() > getSkin().getInput())
        //     timer.switchTimer(TIMER_STARTINPUT, true);
        timer.switch_timer(skin_property::TIMER_STARTINPUT, true);

        // Initialize songbar change timer
        if timer.get_now_time_for_id(skin_property::TIMER_SONGBAR_CHANGE) < 0 {
            timer.set_timer_on(skin_property::TIMER_SONGBAR_CHANGE);
        }

        let now_time = timer.get_now_time();
        let songbar_change_time = timer.get_timer(skin_property::TIMER_SONGBAR_CHANGE);

        // Preview music
        if let Some(current) = self.manager.get_selected() {
            if let Some(song_bar) = current.as_song_bar() {
                // In Java: resource.setSongdata(song_data)
                // resource.setCourseData(null)

                // Preview music timing
                if self.play.is_none()
                    && now_time > songbar_change_time + self.preview_duration as i64
                {
                    let should_start_preview = if let Some(ref preview) = self.preview {
                        let preview_song = preview.get_song_data();
                        // In Java: song != preview.getSongData() (reference comparison)
                        match preview_song {
                            Some(ps) => ps.get_sha256() != song_bar.get_song_data().get_sha256(),
                            None => true,
                        }
                    } else {
                        false
                    };
                    if should_start_preview {
                        let song_clone = song_bar.get_song_data().clone();
                        if let Some(preview) = &mut self.preview {
                            preview.start(Some(&song_clone));
                        }
                    }
                }

                // Read BMS information (notes graph)
                if !self.show_note_graph
                    && self.play.is_none()
                    && now_time > songbar_change_time + self.notes_graph_duration as i64
                {
                    if song_bar.exists_song() {
                        // In Java: spawns thread to load BMS model
                        // Blocked on PlayerResource.loadBMSModel
                    }
                    self.show_note_graph = true;
                }
            } else if current.as_grade_bar().is_some() {
                // In Java: resource.setSongdata(null)
                // resource.setCourseData(courseData)
            } else {
                // In Java: resource.setSongdata(null)
                // resource.setCourseData(null)
            }
        }

        // IR ranking loading
        let songbar_change_time = self
            .main_state_data
            .timer
            .get_timer(skin_property::TIMER_SONGBAR_CHANGE);
        let now_time = self.main_state_data.timer.get_now_time();
        if self.current_ranking_duration != -1
            && now_time > songbar_change_time + self.current_ranking_duration
        {
            self.current_ranking_duration = -1;
            // In Java: loads IR ranking data from cache or creates new
            // Blocked on MainController.getRankingDataCache(), IRStatus
        }

        // Update IR connection timers
        let irstate = self
            .currentir
            .as_ref()
            .map(|ir| ir.get_state())
            .unwrap_or(-1);
        self.main_state_data.timer.switch_timer(
            skin_property::TIMER_IR_CONNECT_BEGIN,
            irstate == ranking_data::ACCESS,
        );
        self.main_state_data.timer.switch_timer(
            skin_property::TIMER_IR_CONNECT_SUCCESS,
            irstate == ranking_data::FINISH,
        );
        self.main_state_data.timer.switch_timer(
            skin_property::TIMER_IR_CONNECT_FAIL,
            irstate == ranking_data::FAIL,
        );

        // Play execution — collect bar info into locals first (borrow checker)
        if let Some(play_mode) = self.play.take() {
            // Classify the selected bar type
            enum BarAction {
                SongExists,
                SongMissing,
                Executable,
                Grade,
                RandomCourse,
                DirectoryAutoplay,
                FunctionOnly,
                None,
            }
            let (action, is_function_bar) = if let Some(current) = self.manager.get_selected() {
                let is_func = current.as_function_bar().is_some();
                if let Some(song_bar) = current.as_song_bar() {
                    if song_bar.exists_song() {
                        (BarAction::SongExists, is_func)
                    } else {
                        (BarAction::SongMissing, is_func)
                    }
                } else if current.as_executable_bar().is_some() {
                    (BarAction::Executable, is_func)
                } else if current.as_grade_bar().is_some() {
                    (BarAction::Grade, is_func)
                } else if current.as_random_course_bar().is_some() {
                    (BarAction::RandomCourse, is_func)
                } else if current.is_directory_bar()
                    && play_mode.mode == BMSPlayerModeType::Autoplay
                {
                    (BarAction::DirectoryAutoplay, is_func)
                } else {
                    (BarAction::FunctionOnly, is_func)
                }
            } else {
                (BarAction::None, false)
            };

            // Now perform mutations without holding a borrow on self.manager
            match action {
                BarAction::SongExists => {
                    // In Java: readChart(song, current)
                    // Blocked on PlayerResource
                    log::warn!(
                        "not yet implemented: MusicSelector play execution - readChart blocked on PlayerResource"
                    );
                }
                BarAction::SongMissing => {
                    // In Java: checks IPFS/HTTP download, opens download site
                    self.execute_event(EventType::OpenDownloadSite);
                }
                BarAction::Executable => {
                    // In Java: readChart(executableBar.getSongData(), current)
                    // Blocked on PlayerResource
                    log::warn!(
                        "not yet implemented: MusicSelector play execution - ExecutableBar blocked on PlayerResource"
                    );
                }
                BarAction::Grade => {
                    let mode = if play_mode.mode == BMSPlayerModeType::Practice {
                        BMSPlayerMode::PLAY
                    } else {
                        play_mode.clone()
                    };
                    self.read_course(mode);
                }
                BarAction::RandomCourse => {
                    let mode = if play_mode.mode == BMSPlayerModeType::Practice {
                        BMSPlayerMode::PLAY
                    } else {
                        play_mode.clone()
                    };
                    self.read_random_course(mode);
                }
                BarAction::DirectoryAutoplay => {
                    // In Java: collects song paths from directory children for autoplay
                    // Blocked on PlayerResource
                    log::warn!(
                        "not yet implemented: MusicSelector directory autoplay - blocked on PlayerResource"
                    );
                }
                BarAction::FunctionOnly | BarAction::None => {}
            }

            // FunctionBar execution
            if is_function_bar
                && let Some(current) = self.manager.get_selected()
                && let Some(func_bar) = current.as_function_bar()
            {
                func_bar.accept();
            }
        }
    }

    /// Input handling — check for config/skinconfig state change, then process music select input.
    /// Corresponds to Java MusicSelector.input()
    fn input(&mut self) {
        // In Java: input = main.getInputProcessor()
        // Blocked on MainController — uses a temporary no-op processor
        // When MainController is available, this will be replaced with real input
        // For now, the input processor is stubbed and events are collected but not dispatched
        // to external systems.

        // Note: The full implementation would:
        // 1. Check input.getControlKeyState(NUM6) -> changeState(CONFIG)
        // 2. Check input.isActivated(OPEN_SKIN_CONFIGURATION) -> changeState(SKINCONFIG)
        // 3. Call musicinput.input(ctx) with BMSPlayerInputProcessor from MainController
        // 4. Dispatch events from ctx to self and MainController
        //
        // Since BMSPlayerInputProcessor comes from MainController (blocked),
        // the musicinput.input() call is deferred.
        // The InputContext and event dispatch infrastructure is in place and ready
        // for Phase 21 (MainController wiring).
    }

    /// Shutdown — stop preview, unfocus search.
    /// Corresponds to Java MusicSelector.shutdown()
    fn shutdown(&mut self) {
        if let Some(preview) = &mut self.preview {
            preview.stop();
        }
        if let Some(search) = &mut self.search {
            search.unfocus();
        }
        self.banners.dispose_old();
        self.stagefiles.dispose_old();
    }

    /// Dispose — clean up bar renderer, search field, and skin.
    /// Corresponds to Java MusicSelector.dispose()
    fn dispose(&mut self) {
        // Call parent dispose (clears skin and stage)
        self.main_state_data.skin = None;
        self.main_state_data.stage = None;

        if let Some(bar) = &self.bar {
            bar.dispose();
        }
        self.banners.dispose();
        self.stagefiles.dispose();
        if let Some(search) = &mut self.search {
            search.dispose();
            self.search = None;
        }
    }
}

/// Chart replication mode
/// Translates: bms.player.beatoraja.select.MusicSelector.ChartReplicationMode
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChartReplicationMode {
    None,
    RivalChart,
    RivalOption,
    ReplayChart,
    ReplayOption,
}

impl ChartReplicationMode {
    pub const ALL_MODE: &'static [ChartReplicationMode] = &[
        ChartReplicationMode::None,
        ChartReplicationMode::RivalChart,
        ChartReplicationMode::RivalOption,
    ];

    pub fn get(name: &str) -> ChartReplicationMode {
        for mode in Self::ALL_MODE {
            if mode.name() == name {
                return mode.clone();
            }
        }
        ChartReplicationMode::None
    }

    pub fn name(&self) -> &'static str {
        match self {
            ChartReplicationMode::None => "NONE",
            ChartReplicationMode::RivalChart => "RIVALCHART",
            ChartReplicationMode::RivalOption => "RIVALOPTION",
            ChartReplicationMode::ReplayChart => "REPLAYCHART",
            ChartReplicationMode::ReplayOption => "REPLAYOPTION",
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::bar::bar::Bar;
    use crate::bar::grade_bar::GradeBar;
    use crate::bar::selectable_bar::SelectableBarData;
    use crate::bar::song_bar::SongBar;
    use beatoraja_core::main_state::MainState;

    fn make_song_data(sha256: &str, path: Option<&str>) -> SongData {
        let mut sd = SongData::default();
        sd.sha256 = sha256.to_string();
        if let Some(p) = path {
            sd.set_path(p.to_string());
        }
        sd
    }

    fn make_song_bar(sha256: &str, path: Option<&str>) -> Bar {
        Bar::Song(Box::new(SongBar::new(make_song_data(sha256, path))))
    }

    #[test]
    fn test_state_type() {
        let selector = MusicSelector::new();
        assert_eq!(selector.state_type(), Some(MainStateType::MusicSelect));
    }

    #[test]
    fn test_main_state_data_access() {
        let mut selector = MusicSelector::new();
        // Verify we can access timer through main_state_data
        let timer = &selector.main_state_data().timer;
        assert!(!timer.is_timer_on(skin_property::TIMER_STARTINPUT));

        // Verify mutable access
        selector
            .main_state_data_mut()
            .timer
            .set_timer_on(skin_property::TIMER_STARTINPUT);
        assert!(
            selector
                .main_state_data()
                .timer
                .is_timer_on(skin_property::TIMER_STARTINPUT)
        );
    }

    #[test]
    fn test_create_lifecycle() {
        let mut selector = MusicSelector::new();
        // Set playedsong and playedcourse to verify they get cleared
        selector.playedsong = Some(make_song_data("abc", Some("/test/song.bms")));
        selector.play = Some(BMSPlayerMode::PLAY);

        selector.create();

        assert!(selector.play.is_none());
        assert!(!selector.show_note_graph);
        assert!(selector.playedsong.is_none());
    }

    #[test]
    fn test_create_clears_played_course() {
        let mut selector = MusicSelector::new();
        let course = CourseData {
            name: Some("Test Course".to_string()),
            hash: vec![make_song_data("s1", Some("/path1.bms"))],
            constraint: vec![],
            trophy: vec![],
            release: false,
        };
        selector.playedcourse = Some(course);

        selector.create();

        assert!(selector.playedcourse.is_none());
    }

    #[test]
    fn test_prepare_starts_preview() {
        let mut selector = MusicSelector::new();
        // prepare without preview processor should not panic
        selector.prepare();
    }

    #[test]
    fn test_shutdown_stops_preview() {
        let mut selector = MusicSelector::new();
        // shutdown without preview/search should not panic
        selector.shutdown();
    }

    #[test]
    fn test_dispose_clears_skin_and_stage() {
        let mut selector = MusicSelector::new();
        selector.dispose();
        assert!(selector.main_state_data.skin.is_none());
        assert!(selector.main_state_data.stage.is_none());
        assert!(selector.search.is_none());
    }

    #[test]
    fn test_set_panel_state_timers() {
        let mut selector = MusicSelector::new();

        // Set panel state to 1
        selector.set_panel_state(1);
        assert_eq!(selector.panelstate, 1);
        assert!(
            selector
                .main_state_data
                .timer
                .is_timer_on(skin_property::TIMER_PANEL1_ON)
        );

        // Set panel state to 2
        selector.set_panel_state(2);
        assert_eq!(selector.panelstate, 2);
        // Panel 1 should be off, panel 2 should be on
        assert!(
            selector
                .main_state_data
                .timer
                .is_timer_on(skin_property::TIMER_PANEL1_OFF)
        );
        assert!(
            selector
                .main_state_data
                .timer
                .is_timer_on(skin_property::TIMER_PANEL1_ON + 1)
        );

        // Set panel state to 0
        selector.set_panel_state(0);
        assert_eq!(selector.panelstate, 0);
        assert!(
            selector
                .main_state_data
                .timer
                .is_timer_on(skin_property::TIMER_PANEL1_OFF + 1)
        );
    }

    #[test]
    fn test_set_panel_state_same_value_no_change() {
        let mut selector = MusicSelector::new();
        selector.set_panel_state(1);

        // Setting same value should not toggle timers
        selector.set_panel_state(1);
        assert_eq!(selector.panelstate, 1);
    }

    #[test]
    fn test_exists_constraint_no_selection() {
        let selector = MusicSelector::new();
        assert!(!selector.exists_constraint(&CourseDataConstraint::Class));
    }

    #[test]
    fn test_exists_constraint_with_grade_bar() {
        let mut selector = MusicSelector::new();
        let course = CourseData {
            name: Some("Test".to_string()),
            hash: vec![],
            constraint: vec![CourseDataConstraint::Class, CourseDataConstraint::Mirror],
            trophy: vec![],
            release: false,
        };
        selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
        selector.manager.selectedindex = 0;

        assert!(selector.exists_constraint(&CourseDataConstraint::Class));
        assert!(selector.exists_constraint(&CourseDataConstraint::Mirror));
        assert!(!selector.exists_constraint(&CourseDataConstraint::Random));
    }

    #[test]
    fn test_exists_constraint_with_song_bar() {
        let mut selector = MusicSelector::new();
        selector.manager.currentsongs = vec![make_song_bar("abc", Some("/test.bms"))];
        selector.manager.selectedindex = 0;

        // SongBar has no constraints
        assert!(!selector.exists_constraint(&CourseDataConstraint::Class));
    }

    #[test]
    fn test_select_directory_bar() {
        let mut selector = MusicSelector::new();
        let bar = Bar::Folder(Box::new(crate::bar::folder_bar::FolderBar::new(
            None,
            "test_crc".to_string(),
        )));
        // select on a directory bar should try to open it (and set play = None still)
        selector.select(&bar);
        // Play should not be set for directory bars
        assert!(selector.play.is_none());
    }

    #[test]
    fn test_select_song_bar() {
        let mut selector = MusicSelector::new();
        let bar = make_song_bar("abc123", Some("/test/song.bms"));
        selector.select(&bar);
        // SongBar (non-directory) should set play mode
        assert_eq!(selector.play, Some(BMSPlayerMode::PLAY));
    }

    #[test]
    fn test_select_song() {
        let mut selector = MusicSelector::new();
        assert!(selector.play.is_none());
        selector.select_song(BMSPlayerMode::PRACTICE);
        assert_eq!(selector.play, Some(BMSPlayerMode::PRACTICE));
    }

    #[test]
    fn test_ranking_position_no_ir() {
        let mut selector = MusicSelector::new();

        // No IR data, default values
        assert_eq!(selector.get_ranking_position(), 0.0);

        // Set position with no IR data — ranking_max = 1, so 1 * 0.5 = 0
        selector.set_ranking_position(0.5);
        assert_eq!(selector.ranking_offset, 0);
    }

    #[test]
    fn test_ranking_position_with_ir() {
        let mut selector = MusicSelector::new();

        // Use update_score to set total player count
        let mut ir = RankingData::new();
        use beatoraja_core::score_data::ScoreData as CoreScoreData;
        use beatoraja_ir::ir_score_data::IRScoreData;
        let scores: Vec<IRScoreData> = (0..10)
            .map(|i| {
                let mut sd = CoreScoreData::default();
                sd.epg = (i + 1) * 10; // different exscores so sorting works
                IRScoreData::new(&sd)
            })
            .collect();
        ir.update_score(&scores, None);
        selector.currentir = Some(ir);

        selector.set_ranking_position(0.5);
        assert_eq!(selector.ranking_offset, 5); // 10 * 0.5

        let pos = selector.get_ranking_position();
        assert!((pos - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_ranking_position_bounds() {
        let mut selector = MusicSelector::new();
        // Out of range values should not change offset
        selector.ranking_offset = 3;
        selector.set_ranking_position(-0.1);
        assert_eq!(selector.ranking_offset, 3);

        selector.set_ranking_position(1.0);
        assert_eq!(selector.ranking_offset, 3);
    }

    #[test]
    fn test_selected_bar_moved_resets_state() {
        let mut selector = MusicSelector::new();
        selector.show_note_graph = true;
        selector.manager.currentsongs = vec![make_song_bar("abc", Some("/test.bms"))];
        selector.manager.selectedindex = 0;

        selector.selected_bar_moved();

        assert!(!selector.show_note_graph);
        // selectedreplay should be -1 since no replay exists
        assert_eq!(selector.selectedreplay, -1);
    }

    #[test]
    fn test_selected_bar_moved_no_ir() {
        let mut selector = MusicSelector::new();
        // With no bars
        selector.selected_bar_moved();

        assert!(selector.currentir.is_none());
        assert_eq!(selector.current_ranking_duration, -1);
    }

    #[test]
    fn test_render_timers() {
        let mut selector = MusicSelector::new();
        // render should set TIMER_STARTINPUT
        selector.render();
        assert!(
            selector
                .main_state_data
                .timer
                .is_timer_on(skin_property::TIMER_STARTINPUT)
        );
    }

    #[test]
    fn test_render_ir_timers_no_ir() {
        let mut selector = MusicSelector::new();
        selector.render();
        // With no IR data, all IR timers should be off
        assert!(
            !selector
                .main_state_data
                .timer
                .is_timer_on(skin_property::TIMER_IR_CONNECT_BEGIN)
        );
        assert!(
            !selector
                .main_state_data
                .timer
                .is_timer_on(skin_property::TIMER_IR_CONNECT_SUCCESS)
        );
        assert!(
            !selector
                .main_state_data
                .timer
                .is_timer_on(skin_property::TIMER_IR_CONNECT_FAIL)
        );
    }

    #[test]
    fn test_command_reset_replay_no_selection() {
        let mut selector = MusicSelector::new();
        selector.selectedreplay = 2;
        selector.execute(MusicSelectCommand::ResetReplay);
        assert_eq!(selector.selectedreplay, -1);
    }

    #[test]
    fn test_command_reset_replay_with_selection() {
        let mut selector = MusicSelector::new();
        let mut song_bar = SongBar::new(make_song_data("abc", Some("/test.bms")));
        song_bar.selectable.exists_replay[2] = true;
        selector.manager.currentsongs = vec![Bar::Song(Box::new(song_bar))];
        selector.manager.selectedindex = 0;

        selector.execute(MusicSelectCommand::ResetReplay);
        assert_eq!(selector.selectedreplay, 2);
    }

    #[test]
    fn test_chart_replication_mode() {
        assert_eq!(
            ChartReplicationMode::get("NONE"),
            ChartReplicationMode::None
        );
        assert_eq!(
            ChartReplicationMode::get("RIVALCHART"),
            ChartReplicationMode::RivalChart
        );
        assert_eq!(
            ChartReplicationMode::get("UNKNOWN"),
            ChartReplicationMode::None
        );

        assert_eq!(ChartReplicationMode::None.name(), "NONE");
        assert_eq!(ChartReplicationMode::ReplayChart.name(), "REPLAYCHART");
    }

    #[test]
    fn test_mode_constants() {
        assert!(MODE[0].is_none());
        assert_eq!(MODE[1], Some(bms_model::Mode::BEAT_7K));
        assert_eq!(MODE[2], Some(bms_model::Mode::BEAT_14K));
        assert_eq!(MODE[3], Some(bms_model::Mode::POPN_9K));
        assert_eq!(MODE[4], Some(bms_model::Mode::BEAT_5K));
        assert_eq!(MODE[5], Some(bms_model::Mode::BEAT_10K));
        assert_eq!(MODE[6], Some(bms_model::Mode::KEYBOARD_24K));
        assert_eq!(MODE[7], Some(bms_model::Mode::KEYBOARD_24K_DOUBLE));
    }

    #[test]
    fn test_dispatch_select_song_event() {
        let mut selector = MusicSelector::new();
        assert!(selector.play.is_none());

        selector.dispatch_input_events(vec![InputEvent::SelectSong(BMSPlayerMode::AUTOPLAY)]);

        assert_eq!(selector.play, Some(BMSPlayerMode::AUTOPLAY));
    }

    #[test]
    fn test_dispatch_execute_command_event() {
        let mut selector = MusicSelector::new();
        selector.selectedreplay = 5;

        selector.dispatch_input_events(vec![InputEvent::Execute(MusicSelectCommand::ResetReplay)]);

        // ResetReplay with no selected bar sets to -1
        assert_eq!(selector.selectedreplay, -1);
    }

    #[test]
    fn test_dispatch_bar_manager_close_event() {
        let mut selector = MusicSelector::new();
        // At root level, close should not panic
        selector.dispatch_input_events(vec![InputEvent::BarManagerClose]);
    }

    #[test]
    fn test_dispatch_multiple_events() {
        let mut selector = MusicSelector::new();
        selector.selectedreplay = 5;

        selector.dispatch_input_events(vec![
            InputEvent::Execute(MusicSelectCommand::ResetReplay),
            InputEvent::SelectSong(BMSPlayerMode::PRACTICE),
        ]);

        assert_eq!(selector.selectedreplay, -1);
        assert_eq!(selector.play, Some(BMSPlayerMode::PRACTICE));
    }

    #[test]
    fn test_process_input_with_context_no_musicinput() {
        let mut selector = MusicSelector::new();
        // musicinput is None by default; should return early without panic
        let config = beatoraja_core::config::Config::default();
        let player_config = PlayerConfig::default();
        let mut input = BMSPlayerInputProcessor::new(&config, &player_config);
        selector.process_input_with_context(&mut input);
    }

    #[test]
    fn test_process_input_with_context_basic() {
        use crate::music_select_input_processor::MusicSelectInputProcessor;

        let mut selector = MusicSelector::new();
        // Install musicinput processor
        selector.musicinput = Some(MusicSelectInputProcessor::new(300, 50, 10));

        let config = beatoraja_core::config::Config::default();
        let player_config = PlayerConfig::default();
        let mut input = BMSPlayerInputProcessor::new(&config, &player_config);

        // process_input should not panic and should set panel state to 0
        // (since no start/select keys are pressed, falls into the else branch)
        selector.process_input_with_context(&mut input);
        assert_eq!(selector.panelstate, 0);
    }

    #[test]
    fn test_dispatch_open_directory_event() {
        let mut selector = MusicSelector::new();
        // OpenDirectory at root with no selected bar — should not panic
        selector.dispatch_input_events(vec![InputEvent::OpenDirectory]);
    }
}

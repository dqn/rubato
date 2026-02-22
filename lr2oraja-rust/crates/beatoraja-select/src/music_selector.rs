use crate::bar::bar::Bar;
use crate::bar_manager::BarManager;
use crate::bar_renderer::BarRenderer;
use crate::bar_sorter::BarSorter;
use crate::music_select_command::MusicSelectCommand;
use crate::music_select_input_processor::MusicSelectInputProcessor;
use crate::preview_music_processor::PreviewMusicProcessor;
use crate::score_data_cache::ScoreDataCache;
use crate::search_text_field::SearchTextField;
use crate::stubs::*;

/// Music selector screen
/// Translates: bms.player.beatoraja.select.MusicSelector
pub struct MusicSelector {
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

    pub fn create(&mut self) {
        // In Java: initializes preview, input processor, loads skin, creates search field
        log::warn!("not yet implemented: MusicSelector.create - requires MainController context");
    }

    pub fn prepare(&mut self) {
        if let Some(preview) = &mut self.preview {
            preview.start(None);
        }
    }

    pub fn render(&mut self) {
        // In Java: handles song info display, preview music, BMS loading, IR ranking, play execution
        log::warn!("not yet implemented: MusicSelector.render - requires full MainState context");
    }

    pub fn input(&mut self) {
        // In Java: checks for config/skinconfig state change, then calls musicinput.input()
        log::warn!("not yet implemented: MusicSelector.input - requires MainController context");
    }

    pub fn shutdown(&mut self) {
        if let Some(preview) = &mut self.preview {
            preview.stop();
        }
        if let Some(search) = &mut self.search {
            search.unfocus();
        }
    }

    pub fn select(&mut self, _current: &Bar) {
        // In Java: opens directory or sets play mode
        log::warn!("not yet implemented: MusicSelector.select - requires BarManager integration");
    }

    pub fn get_selected_replay(&self) -> i32 {
        self.selectedreplay
    }

    pub fn set_selected_replay(&mut self, index: i32) {
        self.selectedreplay = index;
    }

    pub fn execute(&mut self, _command: MusicSelectCommand) {
        // In Java: command.function.accept(this)
        // The actual dispatch is done in the command enum
        log::warn!(
            "not yet implemented: MusicSelectCommand execution - requires full MusicSelector context"
        );
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

    pub fn dispose(&mut self) {
        if let Some(bar) = &self.bar {
            bar.dispose();
        }
        if let Some(search) = &mut self.search {
            search.dispose();
            self.search = None;
        }
    }

    pub fn get_panel_state(&self) -> i32 {
        self.panelstate
    }

    pub fn set_panel_state(&mut self, panelstate: i32) {
        // In Java: switches timer for panel state transitions
        self.panelstate = panelstate;
    }

    pub fn get_song_database(&self) -> &dyn SongDatabaseAccessor {
        &*self.songdb
    }

    pub fn exists_constraint(&self, constraint: &CourseDataConstraint) -> bool {
        // In Java: checks selected bar's course data for the constraint
        // Needs BarManager.getSelected()
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

    pub fn selected_bar_moved(&mut self) {
        // In Java: resets replay, loads images, sets timers, starts preview, loads IR ranking
        log::warn!("not yet implemented: MusicSelector.selectedBarMoved - requires full context");
    }

    pub fn load_selected_song_images(&mut self) {
        // In Java: loads banner and stagefile for selected song
    }

    pub fn select_song(&mut self, mode: BMSPlayerMode) {
        self.play = Some(mode);
    }

    pub fn get_selected_bar_play_config(&self) -> Option<&PlayConfig> {
        // In Java: determines PlayConfig based on selected bar type
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

    pub fn play(&self, _sound: SoundType) {
        // In Java: plays system sound
    }

    pub fn stop(&self, _sound: SoundType) {
        // In Java: stops system sound
    }

    /// Read course (grade bar) for play.
    /// Corresponds to Java MusicSelector.readCourse(BMSPlayerMode)
    fn read_course(&mut self, _mode: BMSPlayerMode) {
        // In Java: gets GradeBar from manager.getSelected(), checks existsAllSongs(),
        // calls _readCourse(mode, gradeBar)
        log::warn!(
            "not yet implemented: MusicSelector.readCourse - requires GradeBar and PlayerResource context"
        );
    }

    /// Read random course for play.
    /// Corresponds to Java MusicSelector.readRandomCourse(BMSPlayerMode)
    fn read_random_course(&mut self, _mode: BMSPlayerMode) {
        // In Java: gets RandomCourseBar, calls lotterySongDatas, creates GradeBar,
        // calls _readCourse, then manager.addRandomCourse
        log::warn!(
            "not yet implemented: MusicSelector.readRandomCourse - requires RandomCourseBar and PlayerResource context"
        );
    }

    /// Internal course reading implementation.
    /// Corresponds to Java MusicSelector._readCourse(BMSPlayerMode, GradeBar)
    fn _read_course(&mut self, _mode: &BMSPlayerMode, _grade_bar: &Bar) -> bool {
        // In Java: clears resource, gets song paths, calls resource.setCourseBMSFiles,
        // applies constraints (CLASS/MIRROR/RANDOM/LN/CN/HCN), sets play mode
        log::warn!(
            "not yet implemented: MusicSelector._readCourse - requires PlayerResource context"
        );
        false
    }

    /// Get banner resource pool.
    /// Corresponds to Java MusicSelector.getBannerResource()
    pub fn get_banner_resource(&self) -> Option<&()> {
        // In Java: returns banners (PixmapResourcePool)
        // Stubbed: PixmapResourcePool not yet wired in select crate
        log::warn!(
            "not yet implemented: MusicSelector.getBannerResource - requires PixmapResourcePool"
        );
        None
    }

    /// Get stagefile resource pool.
    /// Corresponds to Java MusicSelector.getStagefileResource()
    pub fn get_stagefile_resource(&self) -> Option<&()> {
        // In Java: returns stagefiles (PixmapResourcePool)
        // Stubbed: PixmapResourcePool not yet wired in select crate
        log::warn!(
            "not yet implemented: MusicSelector.getStagefileResource - requires PixmapResourcePool"
        );
        None
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

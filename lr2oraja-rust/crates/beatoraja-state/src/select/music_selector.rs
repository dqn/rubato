use std::path::PathBuf;

use beatoraja_core::main_state::{MainState, MainStateData};
use beatoraja_core::pixmap_resource_pool::PixmapResourcePool;
use beatoraja_core::timer_manager::TimerManager;
use beatoraja_ir::ranking_data;
use beatoraja_types::main_controller_access::MainControllerAccess;

use super::bar::bar::Bar;
use super::bar::grade_bar::GradeBar;
use super::bar_manager::BarManager;
use super::bar_renderer::BarRenderer;
use super::bar_sorter::BarSorter;
use super::music_select_command::MusicSelectCommand;
use super::music_select_input_processor::{
    BarType, InputContext, InputEvent, MusicSelectInputProcessor,
};
use super::music_select_key_property::MusicSelectKeyProperty;
use super::preview_music_processor::PreviewMusicProcessor;
use super::score_data_cache::ScoreDataCache;
use super::search_text_field::SearchTextField;
use super::stubs::*;

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

    /// App config (for skin loading)
    pub app_config: Config,

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

    /// MainController reference for state transitions and resource access
    pub main: Option<Box<dyn MainControllerAccess + Send>>,

    /// Input processor for keyboard/controller input (created from config)
    input_processor: Option<BMSPlayerInputProcessor>,
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
        Self::with_config(Config::default())
    }

    pub fn with_config(app_config: Config) -> Self {
        Self {
            main_state_data: MainStateData::new(TimerManager::new()),
            selectedreplay: 0,
            songdb: Box::new(NullSongDatabaseAccessor),
            config: PlayerConfig::default(),
            app_config,
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
            main: None,
            input_processor: None,
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

    /// Set the main controller reference.
    pub fn set_main_controller(&mut self, main: Box<dyn MainControllerAccess + Send>) {
        self.main = Some(main);
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

    pub fn execute_event(&mut self, event: EventType) {
        self.execute_event_with_args(event, 0, 0);
    }

    pub fn execute_event_with_arg(&mut self, event: EventType, arg: i32) {
        self.execute_event_with_args(event, arg, 0);
    }

    /// Dispatch an EventType with arguments.
    /// Translated from Java MainState.executeEvent(EventType, int, int)
    /// which calls e.event.exec(this, arg1, arg2).
    pub fn execute_event_with_args(&mut self, event: EventType, arg1: i32, _arg2: i32) {
        match event {
            EventType::Mode => {
                let current_mode = self.config.get_mode().cloned();
                let mut idx = 0;
                for (i, m) in MODE.iter().enumerate() {
                    if *m == current_mode {
                        idx = i;
                        break;
                    }
                }
                let step = if arg1 >= 0 { 1 } else { MODE.len() - 1 };
                self.config
                    .set_mode(MODE[(idx + step) % MODE.len()].clone());
                self.manager.update_bar_refresh();
                self.play_option_change();
            }
            EventType::Sort => {
                let count = BarSorter::DEFAULT_SORTER.len() as i32;
                let step = if arg1 >= 0 { 1 } else { count - 1 };
                self.set_sort((self.get_sort() + step) % count);
                self.manager.update_bar_refresh();
                self.play_option_change();
            }
            EventType::Lnmode => {
                let step = if arg1 >= 0 { 1 } else { 2 };
                self.config
                    .set_lnmode((self.config.get_lnmode() + step) % 3);
                self.play_option_change();
            }
            EventType::Option1p => {
                let step = if arg1 >= 0 { 1 } else { 9 };
                self.config
                    .set_random((self.config.get_random() + step) % 10);
                self.play_option_change();
            }
            EventType::Option2p => {
                let step = if arg1 >= 0 { 1 } else { 9 };
                self.config
                    .set_random2((self.config.get_random2() + step) % 10);
                self.play_option_change();
            }
            EventType::Optiondp => {
                let step = if arg1 >= 0 { 1 } else { 3 };
                self.config
                    .set_doubleoption((self.config.get_doubleoption() + step) % 4);
                self.play_option_change();
            }
            EventType::Gauge1p => {
                let step = if arg1 >= 0 { 1 } else { 5 };
                self.config.gauge = (self.config.gauge + step) % 6;
                self.play_option_change();
            }
            EventType::GaugeAutoShift => {
                let step = if arg1 >= 0 { 1 } else { 4 };
                self.config.gauge_auto_shift = (self.config.gauge_auto_shift + step) % 5;
                self.play_option_change();
            }
            EventType::Hsfix => {
                if let Some(pc) = self.get_selected_play_config_mut() {
                    let step = if arg1 >= 0 { 1 } else { 4 };
                    pc.set_fixhispeed((pc.get_fixhispeed() + step) % 5);
                }
                self.play_option_change();
            }
            EventType::Duration1p => {
                if let Some(pc) = self.get_selected_play_config_mut() {
                    let delta = if _arg2 != 0 { _arg2 } else { 1 };
                    let step = if arg1 >= 0 { delta } else { -delta };
                    let new_val = (pc.get_duration() + step).clamp(1, 5000);
                    pc.set_duration(new_val);
                }
                self.play_option_change();
            }
            EventType::Bga => {
                let step = if arg1 >= 0 { 1 } else { 2 };
                self.app_config.bga = (self.app_config.bga + step) % 3;
                self.play_option_change();
            }
            EventType::NotesDisplayTiming => {
                let step = if arg1 >= 0 { 1 } else { -1 };
                self.config.judgetiming = (self.config.judgetiming + step).clamp(-500, 500);
                self.play_option_change();
            }
            EventType::NotesDisplayTimingAutoAdjust => {
                self.config.notes_display_timing_auto_adjust =
                    !self.config.notes_display_timing_auto_adjust;
                self.play_option_change();
            }
            EventType::Target => {
                let targets = beatoraja_play::target_property::TargetProperty::get_targets();
                if !targets.is_empty() {
                    let mut index = targets.len();
                    for (i, t) in targets.iter().enumerate() {
                        if t == &self.config.targetid {
                            index = i;
                            break;
                        }
                    }
                    let step = if arg1 >= 0 { 1 } else { targets.len() - 1 };
                    let new_index = (index + step) % targets.len();
                    self.config.targetid = targets[new_index].clone();
                }
                self.play_option_change();
            }
            EventType::Rival => {
                if let Some(ref main) = self.main {
                    let rival_count = main.get_rival_count();
                    // Find current rival's index in the rival list
                    let mut index: i32 = -1;
                    for i in 0..rival_count {
                        if let Some(info) = main.get_rival_information(i)
                            && self.rival.as_ref() == Some(&info)
                        {
                            index = i as i32;
                            break;
                        }
                    }
                    // Cycle to next/previous rival (Java modular arithmetic)
                    let total = rival_count as i32 + 1;
                    let step = if arg1 >= 0 { 2 } else { total };
                    index = (index + step) % total - 1;
                    let new_rival = if index >= 0 {
                        main.get_rival_information(index as usize)
                    } else {
                        None
                    };
                    self.set_rival(new_rival);
                }
                self.play_option_change();
            }
            EventType::FavoriteSong => {
                let next = arg1 >= 0;
                if let Some(songbar) = self.manager.get_selected().and_then(|b| b.as_song_bar()) {
                    let mut sd = songbar.get_song_data().clone();
                    let fav = sd.favorite;
                    let current = if fav & beatoraja_types::song_data::FAVORITE_SONG != 0 {
                        1
                    } else if fav & beatoraja_types::song_data::INVISIBLE_SONG != 0 {
                        2
                    } else {
                        0
                    };
                    let new_type = (current + if next { 1 } else { 2 }) % 3;
                    sd.favorite = (fav
                        & !(beatoraja_types::song_data::FAVORITE_SONG
                            | beatoraja_types::song_data::INVISIBLE_SONG))
                        | match new_type {
                            1 => beatoraja_types::song_data::FAVORITE_SONG,
                            2 => beatoraja_types::song_data::INVISIBLE_SONG,
                            _ => 0,
                        };
                    self.songdb.set_song_datas(&[sd]);
                }
                self.play_option_change();
            }
            EventType::FavoriteChart => {
                let next = arg1 >= 0;
                if let Some(songbar) = self.manager.get_selected().and_then(|b| b.as_song_bar()) {
                    let mut sd = songbar.get_song_data().clone();
                    let fav = sd.favorite;
                    let current = if fav & beatoraja_types::song_data::FAVORITE_CHART != 0 {
                        1
                    } else if fav & beatoraja_types::song_data::INVISIBLE_CHART != 0 {
                        2
                    } else {
                        0
                    };
                    let new_type = (current + if next { 1 } else { 2 }) % 3;
                    sd.favorite = (fav
                        & !(beatoraja_types::song_data::FAVORITE_CHART
                            | beatoraja_types::song_data::INVISIBLE_CHART))
                        | match new_type {
                            1 => beatoraja_types::song_data::FAVORITE_CHART,
                            2 => beatoraja_types::song_data::INVISIBLE_CHART,
                            _ => 0,
                        };
                    self.songdb.set_song_datas(&[sd]);
                }
                self.play_option_change();
            }
            EventType::UpdateFolder => {
                if let Some(ref mut main) = self.main
                    && let Some(selected) = self.manager.get_selected()
                {
                    if let Some(folder) = selected.as_folder_bar()
                        && let Some(fd) = folder.get_folder_data()
                    {
                        let path = fd.get_path().to_string();
                        main.update_song(Some(&path));
                    } else if let Some(songbar) = selected.as_song_bar()
                        && let Some(path) = songbar.get_song_data().get_path()
                        && let Some(parent) =
                            std::path::Path::new(path).parent().and_then(|p| p.to_str())
                    {
                        main.update_song(Some(parent));
                    } else if let Some(table_bar) = selected.as_table_bar() {
                        let source = TableAccessorUpdateSource::new(table_bar.tr.clone());
                        main.update_table(Box::new(source));
                    }
                }
            }
            EventType::OpenDocument => {
                if let Some(songbar) = self.manager.get_selected().and_then(|b| b.as_song_bar())
                    && let Some(path) = songbar.get_song_data().get_path()
                    && let Some(parent) = std::path::Path::new(path).parent()
                    && let Ok(entries) = std::fs::read_dir(parent)
                {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if !p.is_dir()
                            && let Some(ext) = p.extension()
                            && ext.eq_ignore_ascii_case("txt")
                            && let Err(e) = open::that(&p)
                        {
                            log::error!("Failed to open document: {}", e);
                        }
                    }
                }
            }
            EventType::OpenWithExplorer => {
                if let Some(songbar) = self.manager.get_selected().and_then(|b| b.as_song_bar())
                    && let Some(path) = songbar.get_song_data().get_path()
                    && let Some(parent) = std::path::Path::new(path).parent()
                    && let Err(e) = open::that(parent)
                {
                    log::error!("Failed to open folder: {}", e);
                }
            }
            EventType::OpenIr => {
                if let Some(songbar) = self.manager.get_selected().and_then(|b| b.as_song_bar()) {
                    let sd = songbar.get_song_data();
                    if let Some(ref main) = self.main
                        && let Some(url) = main.get_ir_song_url(sd)
                        && let Err(e) = open::that(&url)
                    {
                        log::error!("Failed to open IR URL: {}", e);
                    }
                } else if let Some(gradebar) =
                    self.manager.get_selected().and_then(|b| b.as_grade_bar())
                {
                    let cd = gradebar.get_course_data();
                    if let Some(ref main) = self.main
                        && let Some(url) = main.get_ir_course_url(cd)
                        && let Err(e) = open::that(&url)
                    {
                        log::error!("Failed to open IR URL: {}", e);
                    }
                }
            }
            EventType::OpenDownloadSite => {
                if let Some(songbar) = self.manager.get_selected().and_then(|b| b.as_song_bar()) {
                    let sd = songbar.get_song_data();
                    let url = sd.get_url();
                    if !url.is_empty()
                        && let Err(e) = open::that(url)
                    {
                        log::error!("Failed to open download site: {}", e);
                    }
                    let appendurl = sd.get_appendurl();
                    if !appendurl.is_empty()
                        && let Err(e) = open::that(appendurl)
                    {
                        log::error!("Failed to open append URL: {}", e);
                    }
                }
            }
        }
    }

    /// Play the OPTION_CHANGE system sound.
    fn play_option_change(&mut self) {
        self.play_sound(SoundType::OptionChange);
    }

    /// Get mutable reference to the PlayConfig for the currently selected mode.
    /// Falls back to BEAT_7K if mode cannot be determined.
    fn get_selected_play_config_mut(&mut self) -> Option<&mut PlayConfig> {
        let mode = self
            .config
            .get_mode()
            .cloned()
            .unwrap_or(bms_model::Mode::BEAT_7K);
        Some(self.config.get_play_config(mode).get_playconfig_mut())
    }

    /// Read a chart for play.
    /// Corresponds to Java MusicSelector.readChart(SongData, Bar)
    pub fn read_chart(&mut self, song: &SongData, current: &Bar) {
        let main = match self.main.as_mut() {
            Some(m) => m,
            None => {
                log::warn!("read_chart: no MainController available");
                return;
            }
        };

        // Get play mode for set_bms_file encoding
        let (mode_type, mode_id) = Self::encode_bms_player_mode(self.play.as_ref());

        // resource.clear()
        if let Some(res) = main.get_player_resource_mut() {
            res.clear();
        }

        // resource.setBMSFile(path, play)
        let path_str = match song.get_path() {
            Some(p) => p,
            None => {
                ImGuiNotify::error("Failed to loading BMS : Song not found, or Song has error");
                return;
            }
        };
        let path = std::path::Path::new(&path_str);

        let load_success = main
            .get_player_resource_mut()
            .map(|res| res.set_bms_file(path, mode_type, mode_id))
            .unwrap_or(false);

        if load_success {
            // Set table name/level from directory hierarchy
            let dir = self.manager.get_directory();
            if !dir.is_empty()
                && !matches!(dir.last(), Some(bar) if matches!(**bar, Bar::SameFolder(_)))
            {
                let table_urls: Vec<String> = main
                    .get_config()
                    .get_table_url()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                let mut is_dtable = false;
                let mut tablename: Option<String> = None;
                let mut tablelevel: Option<String> = None;

                for bar in dir {
                    if let Some(tb) = bar.as_table_bar()
                        && let Some(url) = tb.get_url()
                        && table_urls.iter().any(|u| u == url)
                    {
                        is_dtable = true;
                        tablename = Some(bar.get_title());
                    }
                    if bar.as_hash_bar().is_some() && is_dtable {
                        tablelevel = Some(bar.get_title());
                        break;
                    }
                }

                if let Some(ref name) = tablename
                    && let Some(res) = main.get_player_resource_mut()
                {
                    res.set_tablename(name);
                }
                if let Some(ref level) = tablelevel
                    && let Some(res) = main.get_player_resource_mut()
                {
                    res.set_tablelevel(level);
                }
            }

            // Java L384-388: only create new RankingData when IR active AND currentir is null.
            // Do NOT null out currentir when IR inactive (selectedBarMoved already set it).
            if !main.get_ir_table_urls().is_empty() && self.currentir.is_none() {
                use beatoraja_ir::ranking_data::RankingData;
                let lnmode = main.get_player_config().get_lnmode();
                let rd = RankingData::new();
                self.currentir = Some(rd.clone());
                if let Some(cache) = main.get_ranking_data_cache_mut() {
                    cache.put_song_any(song, lnmode, Box::new(rd));
                }
            }
            // Java L388: resource.setRankingData(currentir)
            if let Some(res) = main.get_player_resource_mut() {
                let ranking_any = self
                    .currentir
                    .clone()
                    .map(|rd| Box::new(rd) as Box<dyn std::any::Any + Send + Sync>);
                res.set_ranking_data_any(ranking_any);
            }

            // Set rival score
            let rival_score = current.get_rival_score().cloned();
            if let Some(res) = main.get_player_resource_mut() {
                res.set_rival_score_data_option(rival_score);
            }

            // Chart replication mode
            let songdata = main
                .get_player_resource()
                .and_then(|r| r.get_songdata())
                .cloned();
            let replay_index = self.play.as_ref().map_or(0, |p| p.id);
            let chart_option = Self::compute_chart_option(
                &self.config,
                current.get_rival_score(),
                &**main,
                songdata.as_ref(),
                replay_index,
            );
            if let Some(res) = main.get_player_resource_mut() {
                res.set_chart_option_data(chart_option);
            }

            self.playedsong = Some(song.clone());
            main.change_state(MainStateType::Decide);
        } else {
            ImGuiNotify::error("Failed to loading BMS : Song not found, or Song has error");
        }
    }

    /// Encode BMSPlayerMode to (mode_type, mode_id) for PlayerResourceAccess::set_bms_file.
    fn encode_bms_player_mode(mode: Option<&BMSPlayerMode>) -> (i32, i32) {
        match mode {
            Some(m) => {
                let mode_type = match m.mode {
                    BMSPlayerModeType::Play => 0,
                    BMSPlayerModeType::Practice => 1,
                    BMSPlayerModeType::Autoplay => 2,
                    BMSPlayerModeType::Replay => 3,
                };
                (mode_type, m.id)
            }
            None => (0, 0), // default to Play
        }
    }

    /// Compute chart option based on chart replication mode and rival score.
    /// Corresponds to the ChartReplicationMode switch in Java readChart.
    fn compute_chart_option(
        config: &PlayerConfig,
        rival_score: Option<&ScoreData>,
        main: &dyn MainControllerAccess,
        songdata: Option<&SongData>,
        replay_index: i32,
    ) -> Option<beatoraja_types::replay_data::ReplayData> {
        let mode = ChartReplicationMode::get(config.get_chart_replication_mode());
        match mode {
            ChartReplicationMode::None => None,
            ChartReplicationMode::RivalChart => rival_score.map(|rival| {
                let mut opt = beatoraja_types::replay_data::ReplayData::new();
                opt.randomoption = rival.get_option() % 10;
                opt.randomoption2 = (rival.get_option() / 10) % 10;
                opt.doubleoption = rival.get_option() / 100;
                opt.randomoptionseed = rival.get_seed() % (65536 * 256);
                opt.randomoption2seed = rival.get_seed() / (65536 * 256);
                opt
            }),
            ChartReplicationMode::RivalOption => rival_score.map(|rival| {
                let mut opt = beatoraja_types::replay_data::ReplayData::new();
                opt.randomoption = rival.get_option() % 10;
                opt.randomoption2 = (rival.get_option() / 10) % 10;
                opt.doubleoption = rival.get_option() / 100;
                opt
            }),
            ChartReplicationMode::ReplayChart | ChartReplicationMode::ReplayOption => {
                let sd = songdata?;
                let sha256 = sd.get_sha256();
                let has_ln = sd.has_undefined_long_note();
                let replay = main.read_replay_data(sha256, has_ln, config.lnmode, replay_index)?;
                let mut opt = beatoraja_types::replay_data::ReplayData::new();
                opt.randomoption = replay.randomoption;
                opt.randomoption2 = replay.randomoption2;
                opt.doubleoption = replay.doubleoption;
                if mode == ChartReplicationMode::ReplayChart {
                    opt.randomoptionseed = replay.randomoptionseed;
                    opt.randomoption2seed = replay.randomoption2seed;
                    opt.rand = replay.rand.clone();
                }
                Some(opt)
            }
        }
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

        // Java L647-662: IR ranking lookup guarded by IR status check
        let ir_active = self
            .main
            .as_ref()
            .map(|m| !m.get_ir_table_urls().is_empty())
            .unwrap_or(false);

        if ir_active {
            if let Some(current) = self.manager.get_selected() {
                if let Some(song_bar) = current.as_song_bar() {
                    if song_bar.exists_song() {
                        // Refresh currentir from cache
                        if let Some(main) = self.main.as_ref() {
                            use beatoraja_ir::ranking_data::RankingData;
                            let lnmode = main.get_player_config().get_lnmode();
                            let song = song_bar.get_song_data();
                            self.currentir = main
                                .get_ranking_data_cache()
                                .and_then(|c| c.get_song_any(song, lnmode))
                                .and_then(|a| a.downcast_ref::<RankingData>())
                                .cloned();
                        }
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
                        // Refresh currentir from cache for course
                        if let Some(main) = self.main.as_ref() {
                            use beatoraja_ir::ranking_data::RankingData;
                            let lnmode = main.get_player_config().get_lnmode();
                            let course = grade_bar.get_course_data();
                            self.currentir = main
                                .get_ranking_data_cache()
                                .and_then(|c| c.get_course_any(course, lnmode))
                                .and_then(|a| a.downcast_ref::<RankingData>())
                                .cloned();
                        }
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
        } else {
            self.currentir = None;
            self.current_ranking_duration = -1;
        }
    }

    /// Load banner and stagefile images for the currently selected song bar
    /// onto the player resource's BMSResource.
    /// Java: MusicSelector.loadSelectedSongImages() (L665-673)
    pub fn load_selected_song_images(&mut self) {
        // Extract banner/stagefile raw data from the selected bar (if it's a SongBar)
        let (banner_data, stagefile_data) = match self.manager.get_selected() {
            Some(Bar::Song(song_bar)) => {
                let banner = song_bar
                    .get_banner()
                    .map(|p| (p.width, p.height, p.data().to_vec()));
                let stagefile = song_bar
                    .get_stagefile()
                    .map(|p| (p.width, p.height, p.data().to_vec()));
                (banner, stagefile)
            }
            _ => (None, None),
        };

        // Set banner and stagefile on the player resource's BMSResource
        if let Some(main) = &mut self.main
            && let Some(resource) = main.get_player_resource_mut()
        {
            resource.set_bms_banner_raw(banner_data);
            resource.set_bms_stagefile_raw(stagefile_data);
        }
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
            if let Some(ref mut main) = self.main {
                main.change_state(MainStateType::Config);
            }
        } else if input.is_activated(KeyCommand::OpenSkinConfiguration)
            && let Some(ref mut main) = self.main
        {
            main.change_state(MainStateType::SkinConfig);
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
        if bar_renderer_do_input {
            // Take bar out of self to avoid overlapping borrows with self.manager and input
            if let Some(mut bar) = self.bar.take() {
                let property_idx = self.config.get_musicselectinput() as usize;
                let property = &MusicSelectKeyProperty::VALUES
                    [property_idx.min(MusicSelectKeyProperty::VALUES.len() - 1)];
                let mut bar_input_ctx = crate::select::bar_renderer::BarInputContext {
                    input,
                    property,
                    manager: &mut self.manager,
                    play_scratch: &mut || {
                        // In Java: select.play(SCRATCH)
                        // Sound playback requires MainController — deferred
                    },
                    stop_scratch: &mut || {
                        // In Java: select.stop(SCRATCH)
                        // Sound playback requires MainController — deferred
                    },
                };
                bar.input(&mut bar_input_ctx);
                self.bar = Some(bar);
            }
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
                    if let Some(ref main) = self.main {
                        main.exit();
                    }
                }
                InputEvent::ChangeState(state_type) => {
                    if let Some(ref mut main) = self.main {
                        main.change_state(state_type);
                    }
                }
                InputEvent::SearchRequested => {
                    // In Java, opens a TextInputDialog for song search text.
                    // The search result is applied via MusicSelector::search().
                    // In Rust, the egui overlay handles text input; this event
                    // signals that the search UI should be shown.
                    log::info!("Search popup requested");
                }
            }
        }

        // Check if selected bar changed (Java: if manager.getSelected() != current)
        // In Java, this compares object references. Here we just call selectedBarMoved
        // to update state when the bar might have changed after events.
        // The caller should track bar identity if precise change detection is needed.
    }

    pub fn get_selected_bar_play_config(&self) -> Option<&PlayConfig> {
        let mode = self
            .config
            .get_mode()
            .cloned()
            .unwrap_or(bms_model::Mode::BEAT_7K);
        Some(self.config.get_play_config_ref(mode).get_playconfig())
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
    fn read_course(&mut self, mode: BMSPlayerMode) {
        // Get selected bar and check it's a GradeBar
        let grade_bar = match self.manager.get_selected() {
            Some(bar) if bar.as_grade_bar().is_some() => bar.clone(),
            _ => {
                log::warn!("read_course: selected bar is not a GradeBar");
                return;
            }
        };

        let gb = grade_bar.as_grade_bar().unwrap();
        if !gb.exists_all_songs() {
            log::info!("段位の楽曲が揃っていません (course songs are not all available)");
            if self
                .main
                .as_ref()
                .and_then(|m| m.get_http_downloader())
                .is_some()
            {
                self.execute(MusicSelectCommand::DownloadCourseHttp);
            }
            return;
        }

        if !self._read_course(&mode, &grade_bar) {
            ImGuiNotify::error("Failed to loading Course : Some of songs not found");
            log::info!("段位の楽曲が揃っていません (course songs are not all available)");
        }
    }

    /// Read random course for play.
    /// Corresponds to Java MusicSelector.readRandomCourse(BMSPlayerMode)
    fn read_random_course(&mut self, mode: BMSPlayerMode) {
        // Get selected bar and check it's a RandomCourseBar
        let rc_bar = match self.manager.get_selected() {
            Some(bar) if bar.as_random_course_bar().is_some() => bar.clone(),
            _ => {
                log::warn!("read_random_course: selected bar is not a RandomCourseBar");
                return;
            }
        };

        let rcb = rc_bar.as_random_course_bar().unwrap();
        if !rcb.exists_all_songs() {
            log::info!(
                "ランダムコースの楽曲が揃っていません (random course songs not all available)"
            );
            return;
        }

        // Run lottery: query DB for each stage's SQL, then pick random songs.
        let mut rcd = rcb.get_course_data().clone();
        {
            let songdb = self.get_song_database();
            let player_name = self.app_config.playername.as_deref().unwrap_or("default");
            let score_path = format!("{}/{}/score.db", self.app_config.playerpath, player_name);
            let scorelog_path =
                format!("{}/{}/scorelog.db", self.app_config.playerpath, player_name);
            let songinfo_path = self.app_config.get_songinfopath().to_string();
            rcd.lottery_song_datas(songdb, &score_path, &scorelog_path, Some(&songinfo_path));
        }
        let course_data = rcd.create_course_data();
        let grade_bar = Bar::Grade(Box::new(GradeBar::new(course_data)));

        if let Some(gb) = grade_bar.as_grade_bar()
            && !gb.exists_all_songs()
        {
            ImGuiNotify::error("Failed to loading Random Course : Some of songs not found");
            log::info!(
                "ランダムコースの楽曲が揃っていません (random course songs not all available)"
            );
            return;
        }

        if self._read_course(&mode, &grade_bar) {
            if let Some(gb) = grade_bar.as_grade_bar() {
                let dir_string = self.manager.get_directory_string().to_string();
                self.manager.add_random_course(gb.clone(), dir_string);
                self.manager.update_bar(None);
                self.manager.set_selected(&grade_bar);
            }
        } else {
            ImGuiNotify::error("Failed to loading Random Course : Some of songs not found");
            log::info!(
                "ランダムコースの楽曲が揃っていません (random course songs not all available)"
            );
        }
    }

    /// Start directory autoplay with the given song paths.
    /// Corresponds to Java MusicSelector handling of DirectoryBar in autoplay mode.
    fn read_directory_autoplay(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        if let Some(main) = self.main.as_mut() {
            let should_transition = if let Some(res) = main.get_player_resource_mut() {
                res.clear();
                res.set_auto_play_songs(paths, false);
                res.next_song()
            } else {
                false
            };
            if should_transition {
                main.change_state(MainStateType::Decide);
            }
        }
    }

    /// Internal course reading implementation.
    /// Corresponds to Java MusicSelector._readCourse(BMSPlayerMode, GradeBar)
    fn _read_course(&mut self, mode: &BMSPlayerMode, grade_bar: &Bar) -> bool {
        let main = match self.main.as_mut() {
            Some(m) => m,
            None => {
                log::warn!("_read_course: no MainController available");
                return false;
            }
        };

        // resource.clear()
        if let Some(res) = main.get_player_resource_mut() {
            res.clear();
        }

        // Get song paths from grade bar
        let gb = match grade_bar.as_grade_bar() {
            Some(gb) => gb,
            None => return false,
        };

        let songs = gb.get_song_datas();
        let files: Vec<PathBuf> = songs
            .iter()
            .filter_map(|s| s.get_path().map(PathBuf::from))
            .collect();

        if files.len() != songs.len() {
            log::warn!("_read_course: some songs have no path");
            return false;
        }

        // resource.setCourseBMSFiles(files)
        let load_success = main
            .get_player_resource_mut()
            .map(|res| res.set_course_bms_files(&files))
            .unwrap_or(false);

        if load_success {
            // Apply constraints for PLAY/AUTOPLAY modes only
            if mode.mode == BMSPlayerModeType::Play || mode.mode == BMSPlayerModeType::Autoplay {
                for constraint in gb.get_course_data().get_constraint() {
                    match constraint {
                        CourseDataConstraint::Class => {
                            self.config.set_random(0);
                            self.config.set_random2(0);
                            self.config.set_doubleoption(0);
                        }
                        CourseDataConstraint::Mirror => {
                            if self.config.get_random() == 1 {
                                self.config.set_random2(1);
                                self.config.set_doubleoption(1);
                            } else {
                                self.config.set_random(0);
                                self.config.set_random2(0);
                                self.config.set_doubleoption(0);
                            }
                        }
                        CourseDataConstraint::Random => {
                            if self.config.get_random() > 5 {
                                self.config.set_random(0);
                            }
                            if self.config.get_random2() > 5 {
                                self.config.set_random2(0);
                            }
                        }
                        CourseDataConstraint::Ln => {
                            self.config.set_lnmode(0);
                        }
                        CourseDataConstraint::Cn => {
                            self.config.set_lnmode(1);
                        }
                        CourseDataConstraint::Hcn => {
                            self.config.set_lnmode(2);
                        }
                        _ => {}
                    }
                }
            }

            // Update course data with song data from loaded models
            let course_song_data = main
                .get_player_resource()
                .map(|res| res.get_course_song_data())
                .unwrap_or_default();

            let mut course_data = gb.get_course_data().clone();
            course_data.set_song(course_song_data);

            // resource.setCourseData, setBMSFile for first song
            let (mode_type, mode_id) = Self::encode_bms_player_mode(Some(mode));
            if let Some(res) = main.get_player_resource_mut() {
                res.set_course_data(course_data.clone());
                if !files.is_empty() {
                    res.set_bms_file(&files[0], mode_type, mode_id);
                }
            }

            self.playedcourse = Some(course_data);

            // Load/create cached IR ranking data for course
            {
                use beatoraja_ir::ranking_data::RankingData;
                let lnmode = main.get_player_config().get_lnmode();
                let course = gb.get_course_data();
                let cached = main
                    .get_ranking_data_cache()
                    .and_then(|c| c.get_course_any(course, lnmode))
                    .and_then(|a| a.downcast_ref::<RankingData>())
                    .cloned();
                if let Some(rd) = cached {
                    self.currentir = Some(rd);
                } else {
                    let rd = RankingData::new();
                    self.currentir = Some(rd.clone());
                    if let Some(cache) = main.get_ranking_data_cache_mut() {
                        cache.put_course_any(course, lnmode, Box::new(rd));
                    }
                }
            }
            // Set rival score/chart option to None for course play
            if let Some(res) = main.get_player_resource_mut() {
                res.set_rival_score_data_option(None);
                res.set_chart_option_data(None);
            }

            main.change_state(MainStateType::Decide);
            true
        } else {
            false
        }
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
// SongSelectionAccess trait implementation
// ============================================================

impl beatoraja_types::song_selection_access::SongSelectionAccess for MusicSelector {
    fn get_selected_song_data(&self) -> Option<SongData> {
        let bar = self.get_selected_bar()?;
        bar.as_song_bar().map(|sb| sb.get_song_data().clone())
    }

    fn get_selected_score_data(&self) -> Option<ScoreData> {
        let bar = self.get_selected_bar()?;
        bar.as_song_bar()
            .and_then(|sb| sb.selectable.bar_data.get_score().cloned())
    }

    fn get_reverse_lookup_data(&self) -> Vec<String> {
        // Reverse lookup data comes from PlayerResource via MainController.
        // Currently returns empty; wire when PlayerResource is accessible.
        Vec::new()
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

    fn load_skin(&mut self, skin_type: i32) {
        match beatoraja_skin::skin_loader::load_skin_from_config(
            &self.app_config,
            &self.config,
            skin_type,
        ) {
            Some(skin) => {
                log::info!("Skin loaded for type {}", skin_type);
                self.main_state_data.skin = Some(Box::new(skin));
            }
            None => {
                log::warn!("Failed to load skin for type {}", skin_type);
            }
        }
    }

    fn get_sound(&self, sound: SoundType) -> Option<String> {
        self.main.as_ref().and_then(|m| m.get_sound_path(&sound))
    }

    fn play_sound_loop(&mut self, sound: SoundType, loop_sound: bool) {
        if let Some(ref mut main) = self.main {
            main.play_sound(&sound, loop_sound);
        }
    }

    fn stop_sound(&mut self, sound: SoundType) {
        if let Some(ref mut main) = self.main {
            main.stop_sound(&sound);
        }
    }

    /// Create state — initialize DB access, song list, bar manager.
    /// Corresponds to Java MusicSelector.create()
    fn create(&mut self) {
        // Java: main.getSoundManager().shuffle()
        if let Some(ref mut main) = self.main {
            main.shuffle_sounds();
        }

        self.play = None;
        self.show_note_graph = false;

        // In Java: resource.setPlayerData(main.getPlayDataAccessor().readPlayerData())
        if let Some(ref mut main) = self.main {
            let player_data = main.read_player_data();
            if let Some(pd) = player_data
                && let Some(res) = main.get_player_resource_mut()
            {
                res.set_player_data(pd);
            }
        }

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

        // Create preview music processor
        {
            let mut preview = PreviewMusicProcessor::new(&self.app_config);
            if let Some(sound_path) = self.get_sound(SoundType::Select) {
                preview.set_default(&sound_path);
            }
            self.preview = Some(preview);
        }

        // Configure input processor per musicselectinput mode (Java L183-188)
        // musicselectinput: 0 -> mode7, 1 -> mode9, _ -> mode14
        {
            let mut input = BMSPlayerInputProcessor::new(&self.app_config, &self.config);
            let pc = match self.config.get_musicselectinput() {
                0 => &self.config.mode7,
                1 => &self.config.mode9,
                _ => &self.config.mode14,
            };
            input.set_keyboard_config(pc.get_keyboard_config());
            input.set_controller_config(&mut pc.get_controller().to_vec());
            input.set_midi_config(pc.get_midi_config());
            self.input_processor = Some(input);
        }

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

        // Start input timer after skin input delay
        if let Some(ref skin) = self.main_state_data.skin
            && timer.get_now_time() > skin.get_input() as i64
        {
            timer.switch_timer(skin_property::TIMER_STARTINPUT, true);
        }

        // Initialize songbar change timer
        if timer.get_now_time_for_id(skin_property::TIMER_SONGBAR_CHANGE) < 0 {
            timer.set_timer_on(skin_property::TIMER_SONGBAR_CHANGE);
        }

        let now_time = timer.get_now_time();
        let songbar_change_time = timer.get_timer(skin_property::TIMER_SONGBAR_CHANGE);

        // Update resource with current bar's song/course data (Java MusicSelector L218-219)
        {
            let song_data = self
                .manager
                .get_selected()
                .and_then(|b| b.as_song_bar())
                .map(|sb| sb.get_song_data().clone());
            let course_data = self
                .manager
                .get_selected()
                .and_then(|b| b.as_grade_bar())
                .map(|gb| gb.get_course_data().clone());
            if let Some(ref mut main) = self.main
                && let Some(res) = main.get_player_resource_mut()
            {
                res.set_songdata(song_data);
                if let Some(cd) = course_data {
                    res.set_course_data(cd);
                } else {
                    res.clear_course_data();
                }
            }
        }

        // Preview music
        if let Some(current) = self.manager.get_selected() {
            if let Some(song_bar) = current.as_song_bar() {
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
                    if should_start_preview
                        && !matches!(self.app_config.song_preview, SongPreview::NONE)
                    {
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
                        // Java: spawns thread to call resource.loadBMSModel(path, lnmode)
                        // and sets result on SongData for the density graph.
                        // Rust: load synchronously (BMS parsing is fast).
                        let path = song_bar
                            .get_song_data()
                            .get_path()
                            .map(std::path::PathBuf::from);
                        let lnmode = self.config.get_lnmode();
                        if let Some(path) = path
                            && let Some((model, _margin)) =
                                beatoraja_core::player_resource::PlayerResource::load_bms_model(
                                    &path, lnmode,
                                )
                            && let Some(ref mut main) = self.main
                            && let Some(sd) = main
                                .get_player_resource_mut()
                                .and_then(|r| r.get_songdata_mut())
                        {
                            sd.set_bms_model(model);
                        }
                    }
                    self.show_note_graph = true;
                }
            } else if current.as_grade_bar().is_some() {
                // Grade bar: songdata/courseData already set above
            } else {
                // Other bar types: songdata/courseData already cleared above
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
            // Load/refresh ranking data from cache
            if let Some(current) = self.manager.get_selected()
                && let Some(main) = self.main.as_mut()
            {
                use beatoraja_ir::ranking_data::RankingData;
                let lnmode = main.get_player_config().get_lnmode();
                if let Some(song_bar) = current.as_song_bar()
                    && song_bar.exists_song()
                    && self.play.is_none()
                {
                    let song = song_bar.get_song_data();
                    let cached = main
                        .get_ranking_data_cache()
                        .and_then(|c| c.get_song_any(song, lnmode))
                        .and_then(|a| a.downcast_ref::<RankingData>())
                        .cloned();
                    if cached.is_none() {
                        let rd = RankingData::new();
                        self.currentir = Some(rd.clone());
                        if let Some(cache) = main.get_ranking_data_cache_mut() {
                            cache.put_song_any(song, lnmode, Box::new(rd));
                        }
                    } else {
                        self.currentir = cached;
                    }
                    // Java MusicSelector L251: irc.load(this, song)
                    if let Some(ref mut rd) = self.currentir {
                        use beatoraja_ir::ir_chart_data::IRChartData;
                        use beatoraja_ir::ir_connection::IRConnection;
                        use std::sync::Arc;
                        if let Some(conn_arc) = main.get_ir_connection_any().and_then(|any| {
                            any.downcast_ref::<Arc<dyn IRConnection + Send + Sync>>()
                                .cloned()
                        }) {
                            let chart = IRChartData::new(song);
                            let local_score = main.read_score_data_by_hash(
                                song.get_sha256(),
                                song.has_long_note(),
                                lnmode,
                            );
                            rd.load_song(conn_arc.as_ref(), &chart, local_score.as_ref());
                        }
                    }
                }
                // Java MusicSelector L254-263: GradeBar IR ranking data
                if let Some(grade_bar) = current.as_grade_bar()
                    && grade_bar.exists_all_songs()
                    && self.play.is_none()
                {
                    let course = grade_bar.get_course_data();
                    let cached = main
                        .get_ranking_data_cache()
                        .and_then(|c| c.get_course_any(course, lnmode))
                        .and_then(|a| a.downcast_ref::<RankingData>())
                        .cloned();
                    if cached.is_none() {
                        let rd = RankingData::new();
                        self.currentir = Some(rd.clone());
                        if let Some(cache) = main.get_ranking_data_cache_mut() {
                            cache.put_course_any(course, lnmode, Box::new(rd));
                        }
                    } else {
                        self.currentir = cached;
                    }
                    // Java MusicSelector L261: irc.load(this, course)
                    if let Some(ref mut rd) = self.currentir {
                        use beatoraja_ir::ir_connection::IRConnection;
                        use beatoraja_ir::ir_course_data::IRCourseData;
                        use std::sync::Arc;
                        if let Some(conn_arc) = main.get_ir_connection_any().and_then(|any| {
                            any.downcast_ref::<Arc<dyn IRConnection + Send + Sync>>()
                                .cloned()
                        }) {
                            let ir_course = IRCourseData::new_with_lntype(course, lnmode);
                            rd.load_course(conn_arc.as_ref(), &ir_course, None);
                        }
                    }
                }
            }
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
            // Classify the selected bar type and extract needed data into locals
            enum BarAction {
                SongChart { song: SongData, bar: Bar },
                SongMissing { song: SongData },
                ExecutableChart { song: SongData, bar: Bar },
                Grade,
                RandomCourse,
                DirectoryAutoplay { paths: Vec<PathBuf> },
                FunctionOnly,
                None,
            }
            let (action, is_function_bar) = if let Some(current) = self.manager.get_selected() {
                let is_func = current.as_function_bar().is_some();
                if let Some(song_bar) = current.as_song_bar() {
                    if song_bar.exists_song() {
                        (
                            BarAction::SongChart {
                                song: song_bar.get_song_data().clone(),
                                bar: current.clone(),
                            },
                            is_func,
                        )
                    } else {
                        (
                            BarAction::SongMissing {
                                song: song_bar.get_song_data().clone(),
                            },
                            is_func,
                        )
                    }
                } else if let Some(exec_bar) = current.as_executable_bar() {
                    (
                        BarAction::ExecutableChart {
                            song: exec_bar.get_song_data().clone(),
                            bar: current.clone(),
                        },
                        is_func,
                    )
                } else if current.as_grade_bar().is_some() {
                    (BarAction::Grade, is_func)
                } else if current.as_random_course_bar().is_some() {
                    (BarAction::RandomCourse, is_func)
                } else if current.is_directory_bar()
                    && play_mode.mode == BMSPlayerModeType::Autoplay
                {
                    let songdb = &*self.songdb;
                    let children: Vec<Bar> = match current {
                        Bar::Folder(b) => b.get_children(songdb),
                        Bar::Command(b) => {
                            let player_name =
                                self.app_config.playername.as_deref().unwrap_or("default");
                            let score_path =
                                format!("{}/{}/score.db", self.app_config.playerpath, player_name);
                            let scorelog_path = format!(
                                "{}/{}/scorelog.db",
                                self.app_config.playerpath, player_name
                            );
                            let songinfo_path = self.app_config.get_songinfopath().to_string();
                            let cmd_ctx = crate::select::bar::command_bar::CommandBarContext {
                                score_db_path: &score_path,
                                scorelog_db_path: &scorelog_path,
                                info_db_path: Some(&songinfo_path),
                            };
                            b.get_children(songdb, &cmd_ctx)
                        }
                        Bar::Container(b) => b.get_children().to_vec(),
                        Bar::Hash(b) => b.get_children(songdb),
                        Bar::Table(b) => b.get_children().to_vec(),
                        Bar::SearchWord(b) => b.get_children(songdb),
                        Bar::SameFolder(b) => b.get_children(songdb),
                        Bar::ContextMenu(b) => b.get_children(&self.manager.tables, songdb),
                        Bar::LeaderBoard(b) => b.get_children(),
                        _ => Vec::new(),
                    };
                    let paths: Vec<PathBuf> = children
                        .iter()
                        .filter_map(|bar| {
                            bar.as_song_bar()
                                .filter(|sb| sb.exists_song())
                                .and_then(|sb| sb.get_song_data().get_path())
                                .map(PathBuf::from)
                        })
                        .collect();
                    (BarAction::DirectoryAutoplay { paths }, is_func)
                } else {
                    (BarAction::FunctionOnly, is_func)
                }
            } else {
                (BarAction::None, false)
            };

            // Now perform mutations without holding a borrow on self.manager
            match action {
                BarAction::SongChart { song, bar } => {
                    self.read_chart(&song, &bar);
                }
                BarAction::SongMissing { song } => {
                    // Java: MusicSelector lines 275-282 — IPFS/HTTP download fallback
                    // 1. If song has IPFS hash and IPFS daemon is alive -> IPFS download
                    // 2. Else if HTTP download processor is available -> HTTP download
                    // 3. Else -> open download site in browser
                    let ipfs_available = !song.get_ipfs_str().is_empty()
                        && self
                            .main
                            .as_ref()
                            .is_some_and(|m| m.is_ipfs_download_alive());
                    let http_available = self
                        .main
                        .as_ref()
                        .and_then(|m| m.get_http_downloader())
                        .is_some();

                    if ipfs_available {
                        self.execute(MusicSelectCommand::DownloadIpfs);
                    } else if http_available {
                        self.execute(MusicSelectCommand::DownloadHttp);
                    } else {
                        self.execute_event(EventType::OpenDownloadSite);
                    }
                }
                BarAction::ExecutableChart { song, bar } => {
                    self.read_chart(&song, &bar);
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
                BarAction::DirectoryAutoplay { paths } => {
                    self.read_directory_autoplay(paths);
                }
                BarAction::FunctionOnly | BarAction::None => {}
            }

            // FunctionBar execution — extract callback to release immutable borrow
            // before passing &mut self to the closure
            if is_function_bar {
                let callback = self
                    .manager
                    .get_selected()
                    .and_then(|b| b.as_function_bar())
                    .and_then(|fb| fb.function.clone());
                if let Some(cb) = callback {
                    cb(self);
                }
            }
        }
    }

    /// Input handling — check for config/skinconfig state change, then process music select input.
    /// Corresponds to Java MusicSelector.input()
    fn input(&mut self) {
        // Initialize input processor on first call (lazy init from config)
        if self.input_processor.is_none() {
            self.input_processor =
                Some(BMSPlayerInputProcessor::new(&self.app_config, &self.config));
        }

        // Take the input processor out to avoid overlapping &mut self borrow
        let mut input_processor = match self.input_processor.take() {
            Some(ip) => ip,
            None => return,
        };

        // Poll keyboard/controller state
        input_processor.poll();

        // Delegate to process_input_with_context which handles:
        // 1. NUM6 -> CONFIG state change
        // 2. OpenSkinConfiguration -> SKINCONFIG state change
        // 3. musicinput.input() for bar navigation
        self.process_input_with_context(&mut input_processor);

        // Put it back
        self.input_processor = Some(input_processor);
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
    /// Java: allMode = {NONE, RIVALCHART, RIVALOPTION} — excludes REPLAYCHART/REPLAYOPTION
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
    use crate::select::bar::bar::Bar;
    use crate::select::bar::grade_bar::GradeBar;
    use crate::select::bar::selectable_bar::SelectableBarData;
    use crate::select::bar::song_bar::SongBar;
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
        let bar = Bar::Folder(Box::new(crate::select::bar::folder_bar::FolderBar::new(
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
        // Without a skin loaded, TIMER_STARTINPUT should NOT be set
        // (matches Java: timer.switchTimer(TIMER_STARTINPUT) is guarded by getSkin().getInput())
        selector.render();
        assert!(
            !selector
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
        use crate::select::music_select_input_processor::MusicSelectInputProcessor;

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

    // ============================================================
    // Mock MainController for read_chart/read_course tests
    // ============================================================

    use beatoraja_types::main_controller_access::MainControllerAccess;
    use beatoraja_types::player_resource_access::PlayerResourceAccess;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    /// Tracks state changes and resource operations for assertions.
    #[derive(Default)]
    struct MockState {
        state_changes: Vec<MainStateType>,
        cleared: bool,
        bms_file_path: Option<PathBuf>,
        bms_file_mode_type: Option<i32>,
        bms_file_mode_id: Option<i32>,
        bms_file_result: bool,
        course_files: Option<Vec<PathBuf>>,
        course_files_result: bool,
        tablename: Option<String>,
        tablelevel: Option<String>,
        rival_score: Option<Option<ScoreData>>,
        chart_option: Option<Option<beatoraja_types::replay_data::ReplayData>>,
        course_data: Option<CourseData>,
        course_song_data: Vec<SongData>,
        auto_play_songs: Option<Vec<PathBuf>>,
        auto_play_loop: Option<bool>,
        next_song_result: bool,
    }

    /// Mock PlayerResource that records operations.
    struct MockPlayerResource {
        state: Arc<Mutex<MockState>>,
        course_gauge: Vec<Vec<Vec<f32>>>,
        course_replay: Vec<beatoraja_types::replay_data::ReplayData>,
    }

    impl PlayerResourceAccess for MockPlayerResource {
        fn get_config(&self) -> &beatoraja_types::config::Config {
            static CFG: std::sync::OnceLock<beatoraja_types::config::Config> =
                std::sync::OnceLock::new();
            CFG.get_or_init(beatoraja_types::config::Config::default)
        }
        fn get_player_config(&self) -> &beatoraja_types::player_config::PlayerConfig {
            static PC: std::sync::OnceLock<beatoraja_types::player_config::PlayerConfig> =
                std::sync::OnceLock::new();
            PC.get_or_init(beatoraja_types::player_config::PlayerConfig::default)
        }
        fn get_score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn get_rival_score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn get_target_score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn get_course_score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn set_course_score_data(&mut self, _score: ScoreData) {}
        fn get_songdata(&self) -> Option<&SongData> {
            None
        }
        fn get_songdata_mut(&mut self) -> Option<&mut SongData> {
            None
        }
        fn set_songdata(&mut self, _data: Option<SongData>) {}
        fn get_replay_data(&self) -> Option<&beatoraja_types::replay_data::ReplayData> {
            None
        }
        fn get_replay_data_mut(&mut self) -> Option<&mut beatoraja_types::replay_data::ReplayData> {
            None
        }
        fn get_course_replay(&self) -> &[beatoraja_types::replay_data::ReplayData] {
            &[]
        }
        fn add_course_replay(&mut self, _rd: beatoraja_types::replay_data::ReplayData) {}
        fn get_course_data(&self) -> Option<&CourseData> {
            None
        }
        fn get_course_index(&self) -> usize {
            0
        }
        fn next_course(&mut self) -> bool {
            false
        }
        fn get_constraint(&self) -> Vec<beatoraja_types::course_data::CourseDataConstraint> {
            vec![]
        }
        fn get_gauge(&self) -> Option<&Vec<Vec<f32>>> {
            None
        }
        fn get_groove_gauge(&self) -> Option<&beatoraja_types::groove_gauge::GrooveGauge> {
            None
        }
        fn get_course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
            static EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
            &EMPTY
        }
        fn add_course_gauge(&mut self, _gauge: Vec<Vec<f32>>) {}
        fn get_course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
            &mut self.course_gauge
        }
        fn get_score_data_mut(&mut self) -> Option<&mut ScoreData> {
            None
        }
        fn get_course_replay_mut(&mut self) -> &mut Vec<beatoraja_types::replay_data::ReplayData> {
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
            false
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
        fn get_reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }
        fn get_reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }
        fn clear(&mut self) {
            self.state.lock().unwrap().cleared = true;
        }
        fn set_bms_file(&mut self, path: &Path, mode_type: i32, mode_id: i32) -> bool {
            let mut s = self.state.lock().unwrap();
            s.bms_file_path = Some(path.to_path_buf());
            s.bms_file_mode_type = Some(mode_type);
            s.bms_file_mode_id = Some(mode_id);
            s.bms_file_result
        }
        fn set_course_bms_files(&mut self, files: &[PathBuf]) -> bool {
            let mut s = self.state.lock().unwrap();
            s.course_files = Some(files.to_vec());
            s.course_files_result
        }
        fn set_tablename(&mut self, name: &str) {
            self.state.lock().unwrap().tablename = Some(name.to_string());
        }
        fn set_tablelevel(&mut self, level: &str) {
            self.state.lock().unwrap().tablelevel = Some(level.to_string());
        }
        fn set_rival_score_data_option(&mut self, score: Option<ScoreData>) {
            self.state.lock().unwrap().rival_score = Some(score);
        }
        fn set_chart_option_data(
            &mut self,
            option: Option<beatoraja_types::replay_data::ReplayData>,
        ) {
            self.state.lock().unwrap().chart_option = Some(option);
        }
        fn set_course_data(&mut self, data: CourseData) {
            self.state.lock().unwrap().course_data = Some(data);
        }
        fn clear_course_data(&mut self) {
            self.state.lock().unwrap().course_data = None;
        }
        fn get_course_song_data(&self) -> Vec<SongData> {
            self.state.lock().unwrap().course_song_data.clone()
        }
        fn set_auto_play_songs(&mut self, paths: Vec<PathBuf>, loop_play: bool) {
            let mut s = self.state.lock().unwrap();
            s.auto_play_songs = Some(paths);
            s.auto_play_loop = Some(loop_play);
        }
        fn next_song(&mut self) -> bool {
            self.state.lock().unwrap().next_song_result
        }
    }

    /// Mock MainController that delegates resource access to MockPlayerResource.
    struct MockMainController {
        state: Arc<Mutex<MockState>>,
        resource: MockPlayerResource,
    }

    impl MockMainController {
        fn new(state: Arc<Mutex<MockState>>) -> Self {
            let resource = MockPlayerResource {
                state: state.clone(),
                course_gauge: Vec::new(),
                course_replay: Vec::new(),
            };
            Self { state, resource }
        }
    }

    impl MainControllerAccess for MockMainController {
        fn get_config(&self) -> &beatoraja_types::config::Config {
            static CFG: std::sync::OnceLock<beatoraja_types::config::Config> =
                std::sync::OnceLock::new();
            CFG.get_or_init(beatoraja_types::config::Config::default)
        }
        fn get_player_config(&self) -> &beatoraja_types::player_config::PlayerConfig {
            static PC: std::sync::OnceLock<beatoraja_types::player_config::PlayerConfig> =
                std::sync::OnceLock::new();
            PC.get_or_init(beatoraja_types::player_config::PlayerConfig::default)
        }
        fn change_state(&mut self, state: MainStateType) {
            self.state.lock().unwrap().state_changes.push(state);
        }
        fn save_config(&self) {}
        fn exit(&self) {}
        fn save_last_recording(&self, _reason: &str) {}
        fn update_song(&mut self, _path: Option<&str>) {}
        fn get_player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
            Some(&self.resource)
        }
        fn get_player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
            Some(&mut self.resource)
        }
    }

    fn make_selector_with_mock() -> (MusicSelector, Arc<Mutex<MockState>>) {
        let state = Arc::new(Mutex::new(MockState::default()));
        let mock = MockMainController::new(state.clone());
        let mut selector = MusicSelector::new();
        selector.set_main_controller(Box::new(mock));
        (selector, state)
    }

    // ============================================================
    // read_chart tests
    // ============================================================

    #[test]
    fn test_read_chart_success_clears_resource_and_transitions() {
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().bms_file_result = true;
        selector.play = Some(BMSPlayerMode::PLAY);

        let song = make_song_data("abc123", Some("/test/song.bms"));
        let bar = make_song_bar("abc123", Some("/test/song.bms"));

        selector.read_chart(&song, &bar);

        let s = state.lock().unwrap();
        assert!(s.cleared, "resource.clear() should have been called");
        assert_eq!(
            s.bms_file_path.as_ref().map(|p| p.to_str().unwrap()),
            Some("/test/song.bms"),
            "set_bms_file should be called with song path"
        );
        assert_eq!(
            s.state_changes,
            vec![MainStateType::Decide],
            "should transition to DECIDE on success"
        );
        assert_eq!(
            selector.playedsong.as_ref().map(|sd| sd.sha256.as_str()),
            Some("abc123"),
            "playedsong should be set"
        );
    }

    #[test]
    fn test_read_chart_failure_does_not_transition() {
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().bms_file_result = false;
        selector.play = Some(BMSPlayerMode::PLAY);

        let song = make_song_data("abc123", Some("/nonexistent.bms"));
        let bar = make_song_bar("abc123", Some("/nonexistent.bms"));

        selector.read_chart(&song, &bar);

        let s = state.lock().unwrap();
        assert!(s.cleared, "resource.clear() should still be called");
        assert!(
            s.state_changes.is_empty(),
            "should NOT transition on failure"
        );
        assert!(
            selector.playedsong.is_none(),
            "playedsong should NOT be set on failure"
        );
    }

    #[test]
    fn test_read_chart_sets_rival_score_and_chart_option() {
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().bms_file_result = true;
        selector.play = Some(BMSPlayerMode::PLAY);

        let song = make_song_data("abc123", Some("/test/song.bms"));
        let bar = make_song_bar("abc123", Some("/test/song.bms"));

        selector.read_chart(&song, &bar);

        let s = state.lock().unwrap();
        // rival_score should have been set (to bar's rival score, which is None)
        assert!(
            s.rival_score.is_some(),
            "set_rival_score_data_option should have been called"
        );
        // chart_option should have been set (to None for ChartReplicationMode::None)
        assert!(
            s.chart_option.is_some(),
            "set_chart_option_data should have been called"
        );
    }

    #[test]
    fn test_read_chart_without_main_controller_does_not_panic() {
        let mut selector = MusicSelector::new();
        selector.play = Some(BMSPlayerMode::PLAY);

        let song = make_song_data("abc123", Some("/test/song.bms"));
        let bar = make_song_bar("abc123", Some("/test/song.bms"));

        // Should not panic, just log warning
        selector.read_chart(&song, &bar);
    }

    // ============================================================
    // read_course tests
    // ============================================================

    #[test]
    fn test_read_course_success_transitions_to_decide() {
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().course_files_result = true;
        state.lock().unwrap().bms_file_result = true;

        // Set up a GradeBar as the selected bar with valid songs
        let course = CourseData {
            name: Some("Test Course".to_string()),
            hash: vec![
                make_song_data("s1", Some("/path/song1.bms")),
                make_song_data("s2", Some("/path/song2.bms")),
            ],
            constraint: vec![],
            trophy: vec![],
            release: false,
        };
        selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
        selector.manager.selectedindex = 0;

        selector.read_course(BMSPlayerMode::PLAY);

        let s = state.lock().unwrap();
        assert!(s.cleared, "resource.clear() should have been called");
        assert!(
            s.course_files.is_some(),
            "set_course_bms_files should have been called"
        );
        assert_eq!(
            s.state_changes,
            vec![MainStateType::Decide],
            "should transition to DECIDE"
        );
        assert!(
            selector.playedcourse.is_some(),
            "playedcourse should be set"
        );
    }

    #[test]
    fn test_read_course_missing_songs_does_not_transition() {
        let (mut selector, state) = make_selector_with_mock();

        // GradeBar with a song that has no path
        let course = CourseData {
            name: Some("Incomplete Course".to_string()),
            hash: vec![
                make_song_data("s1", Some("/path/song1.bms")),
                make_song_data("s2", None), // missing path
            ],
            constraint: vec![],
            trophy: vec![],
            release: false,
        };
        selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
        selector.manager.selectedindex = 0;

        selector.read_course(BMSPlayerMode::PLAY);

        let s = state.lock().unwrap();
        assert!(
            s.state_changes.is_empty(),
            "should NOT transition when songs are missing"
        );
    }

    #[test]
    fn test_read_course_class_constraint_resets_random() {
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().course_files_result = true;
        state.lock().unwrap().bms_file_result = true;

        // Set non-zero random options
        selector.config.set_random(3);
        selector.config.set_random2(4);
        selector.config.set_doubleoption(2);

        let course = CourseData {
            name: Some("Class Course".to_string()),
            hash: vec![make_song_data("s1", Some("/path/song1.bms"))],
            constraint: vec![CourseDataConstraint::Class],
            trophy: vec![],
            release: false,
        };
        selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
        selector.manager.selectedindex = 0;

        selector.read_course(BMSPlayerMode::PLAY);

        assert_eq!(
            selector.config.get_random(),
            0,
            "CLASS should reset random to 0"
        );
        assert_eq!(
            selector.config.get_random2(),
            0,
            "CLASS should reset random2 to 0"
        );
        assert_eq!(
            selector.config.get_doubleoption(),
            0,
            "CLASS should reset doubleoption to 0"
        );
    }

    // ============================================================
    // _read_course tests
    // ============================================================

    #[test]
    fn test_internal_read_course_ln_constraint() {
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().course_files_result = true;
        state.lock().unwrap().bms_file_result = true;
        selector.config.set_lnmode(2);

        let course = CourseData {
            name: Some("LN Course".to_string()),
            hash: vec![make_song_data("s1", Some("/path/song1.bms"))],
            constraint: vec![CourseDataConstraint::Ln],
            trophy: vec![],
            release: false,
        };
        let bar = Bar::Grade(Box::new(GradeBar::new(course)));

        let result = selector._read_course(&BMSPlayerMode::PLAY, &bar);

        assert!(result, "_read_course should return true on success");
        assert_eq!(
            selector.config.get_lnmode(),
            0,
            "LN constraint should set lnmode to 0"
        );
    }

    #[test]
    fn test_internal_read_course_autoplay_applies_constraints() {
        // Java applies constraints for both PLAY and AUTOPLAY modes
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().course_files_result = true;
        state.lock().unwrap().bms_file_result = true;
        selector.config.set_random(5);

        let course = CourseData {
            name: Some("Class Course".to_string()),
            hash: vec![make_song_data("s1", Some("/path/song1.bms"))],
            constraint: vec![CourseDataConstraint::Class],
            trophy: vec![],
            release: false,
        };
        let bar = Bar::Grade(Box::new(GradeBar::new(course)));

        let result = selector._read_course(&BMSPlayerMode::AUTOPLAY, &bar);

        assert!(result);
        // AUTOPLAY applies CLASS constraint (same as PLAY)
        assert_eq!(
            selector.config.get_random(),
            0,
            "AUTOPLAY should apply CLASS constraint and reset random"
        );
    }

    #[test]
    fn test_internal_read_course_replay_skips_constraints() {
        // Java only applies constraints for PLAY and AUTOPLAY, not REPLAY
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().course_files_result = true;
        state.lock().unwrap().bms_file_result = true;
        selector.config.set_random(5);

        let course = CourseData {
            name: Some("Class Course".to_string()),
            hash: vec![make_song_data("s1", Some("/path/song1.bms"))],
            constraint: vec![CourseDataConstraint::Class],
            trophy: vec![],
            release: false,
        };
        let bar = Bar::Grade(Box::new(GradeBar::new(course)));

        let result = selector._read_course(&BMSPlayerMode::REPLAY_1, &bar);

        assert!(result);
        // REPLAY should NOT apply constraints
        assert_eq!(
            selector.config.get_random(),
            5,
            "REPLAY should not reset random"
        );
    }

    // ============================================================
    // read_random_course tests
    // ============================================================

    #[test]
    fn test_read_random_course_missing_songs_does_not_transition() {
        let (mut selector, state) = make_selector_with_mock();

        // RandomCourseBar with no stages — exists_all_songs returns false
        let rcd = RandomCourseData {
            name: Some("Random Course".to_string()),
            stage: vec![], // no stages = not all songs exist
            ..Default::default()
        };
        selector.manager.currentsongs = vec![Bar::RandomCourse(Box::new(
            crate::select::bar::random_course_bar::RandomCourseBar::new(rcd),
        ))];
        selector.manager.selectedindex = 0;

        selector.read_random_course(BMSPlayerMode::PLAY);

        let s = state.lock().unwrap();
        assert!(
            s.state_changes.is_empty(),
            "should NOT transition when random course has no stages"
        );
    }

    // ============================================================
    // directory autoplay tests
    // ============================================================

    #[test]
    fn test_directory_autoplay_transitions_to_decide() {
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().next_song_result = true;

        let paths = vec![
            PathBuf::from("/dir/song_a.bms"),
            PathBuf::from("/dir/song_b.bms"),
        ];

        selector.read_directory_autoplay(paths);

        let s = state.lock().unwrap();
        assert!(s.cleared, "resource.clear() should have been called");
        let paths = s
            .auto_play_songs
            .as_ref()
            .expect("auto_play_songs should be set");
        assert_eq!(paths.len(), 2, "should have 2 valid song paths");
        assert_eq!(paths[0], PathBuf::from("/dir/song_a.bms"));
        assert_eq!(paths[1], PathBuf::from("/dir/song_b.bms"));
        assert_eq!(s.auto_play_loop, Some(false), "loop_play should be false");
        assert_eq!(
            s.state_changes,
            vec![MainStateType::Decide],
            "should transition to DECIDE"
        );
    }

    #[test]
    fn test_directory_autoplay_no_transition_when_next_song_fails() {
        let (mut selector, state) = make_selector_with_mock();
        state.lock().unwrap().next_song_result = false;

        selector.read_directory_autoplay(vec![PathBuf::from("/dir/song_a.bms")]);

        let s = state.lock().unwrap();
        assert!(s.cleared, "resource.clear() should have been called");
        assert!(s.auto_play_songs.is_some(), "auto_play_songs should be set");
        assert!(
            s.state_changes.is_empty(),
            "should NOT transition when next_song returns false"
        );
    }

    #[test]
    fn test_directory_autoplay_empty_paths_does_nothing() {
        let (mut selector, state) = make_selector_with_mock();

        selector.read_directory_autoplay(vec![]);

        let s = state.lock().unwrap();
        assert!(!s.cleared, "should NOT clear when no valid paths");
        assert!(
            s.auto_play_songs.is_none(),
            "auto_play_songs should not be set"
        );
        assert!(
            s.state_changes.is_empty(),
            "should NOT transition with empty paths"
        );
    }

    #[test]
    fn test_directory_autoplay_path_extraction_from_container_bar() {
        // Verify path extraction logic from directory bar children
        let children = vec![
            make_song_bar("sha_a", Some("/dir/song_a.bms")),
            make_song_bar("sha_b", Some("/dir/song_b.bms")),
            make_song_bar("sha_c", None), // no path - should be filtered out
        ];
        let container =
            crate::select::bar::container_bar::ContainerBar::new(String::new(), children);
        let bar = Bar::Container(Box::new(container));

        assert!(bar.is_directory_bar());

        // Extract paths the same way render() does
        let paths: Vec<PathBuf> = if let Bar::Container(b) = &bar {
            b.get_children()
                .iter()
                .filter_map(|bar| {
                    bar.as_song_bar()
                        .filter(|sb| sb.exists_song())
                        .and_then(|sb| sb.get_song_data().get_path())
                        .map(PathBuf::from)
                })
                .collect()
        } else {
            vec![]
        };

        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("/dir/song_a.bms"));
        assert_eq!(paths[1], PathBuf::from("/dir/song_b.bms"));
    }
}

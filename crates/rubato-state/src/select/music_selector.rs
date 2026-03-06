use std::path::PathBuf;

use rubato_audio::audio_driver::AudioDriver;
use rubato_core::main_state::{MainState, MainStateData};
use rubato_core::pixmap_resource_pool::PixmapResourcePool;
use rubato_core::timer_manager::TimerManager;
use rubato_ir::ranking_data;
use rubato_types::main_controller_access::MainControllerAccess;
use rubato_types::player_resource_access::PlayerResourceAccess;

use super::bar::bar::Bar;
use super::bar::grade_bar::GradeBar;
use super::bar_manager::BarManager;
use super::bar_renderer::BarRenderer;
use super::bar_renderer::{PrepareContext, RenderContext};
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

fn delegated_event_type_from_id(event_id: i32) -> Option<EventType> {
    match event_id {
        17 => Some(EventType::OpenDocument),
        79 => Some(EventType::Rival),
        89 => Some(EventType::FavoriteSong),
        90 => Some(EventType::FavoriteChart),
        210 => Some(EventType::OpenIr),
        211 => Some(EventType::UpdateFolder),
        212 => Some(EventType::OpenWithExplorer),
        213 => Some(EventType::OpenDownloadSite),
        _ => None,
    }
}

fn normalized_play_config_mode(mode: bms_model::Mode) -> bms_model::Mode {
    match mode {
        bms_model::Mode::POPN_5K | bms_model::Mode::POPN_9K => bms_model::Mode::POPN_9K,
        _ => mode,
    }
}

fn play_config_mode_from_song(song: &SongData) -> Option<bms_model::Mode> {
    match song.mode {
        5 => Some(bms_model::Mode::BEAT_5K),
        7 => Some(bms_model::Mode::BEAT_7K),
        9 => Some(bms_model::Mode::POPN_9K),
        10 => Some(bms_model::Mode::BEAT_10K),
        14 => Some(bms_model::Mode::BEAT_14K),
        25 => Some(bms_model::Mode::KEYBOARD_24K),
        50 => Some(bms_model::Mode::KEYBOARD_24K_DOUBLE),
        _ => None,
    }
    .map(normalized_play_config_mode)
}

/// Rich skin context for music select rendering and mouse interaction.
/// This keeps skin-side events wired to the real selector instead of a timer-only stub.
struct SelectSkinContext<'a> {
    timer: &'a mut TimerManager,
    selector: &'a mut MusicSelector,
}

impl rubato_types::timer_access::TimerAccess for SelectSkinContext<'_> {
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

impl SelectSkinContext<'_> {
    fn selected_bar(&self) -> Option<&Bar> {
        self.selector.manager.selected()
    }

    fn selected_song_data(&self) -> Option<&rubato_types::song_data::SongData> {
        self.selected_bar()?.as_song_bar().map(|sb| sb.song_data())
    }

    fn selected_score(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.selected_bar()?.score()
    }

    fn selected_rival_score(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.selected_bar()?.rival_score()
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for SelectSkinContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::MusicSelect)
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(&self.selector.config)
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        Some(&mut self.selector.config)
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(&self.selector.app_config)
    }

    fn config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
        Some(&mut self.selector.app_config)
    }

    fn selected_play_config_mut(&mut self) -> Option<&mut rubato_types::play_config::PlayConfig> {
        self.selector.get_selected_play_config_mut()
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        self.selector.get_selected_play_config_ref()
    }

    fn target_score_data(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.selected_rival_score()
    }

    fn score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.selected_score()
    }

    fn rival_score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.selected_rival_score()
    }

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.selected_song_data()
    }

    fn mode_image_index(&self) -> Option<i32> {
        let current_mode = self.selector.config.mode();
        let mode_index = MODE.iter().position(|mode| mode.as_ref() == current_mode)?;
        let lr2_mode_indices = [0, 2, 4, 5, 1, 3];
        Some(
            lr2_mode_indices
                .get(mode_index)
                .copied()
                .unwrap_or(mode_index as i32),
        )
    }

    fn sort_image_index(&self) -> Option<i32> {
        Some(self.selector.sort())
    }

    fn set_timer_micro(&mut self, timer_id: i32, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
        if let Some(event) = delegated_event_type_from_id(id) {
            self.selector.execute_event_with_args(event, arg1, arg2);
        }
    }

    fn change_state(&mut self, state: MainStateType) {
        self.selector.pending_state_change = Some(state);
    }

    fn play_option_change_sound(&mut self) {
        self.selector.play_option_change();
    }

    fn update_bar_after_change(&mut self) {
        self.selector.refresh_bar_with_context();
    }

    fn select_song_mode(&mut self, event_id: i32) {
        let mode = match event_id {
            15 => Some(BMSPlayerMode::PLAY),
            16 => Some(BMSPlayerMode::AUTOPLAY),
            315 => Some(BMSPlayerMode::PRACTICE),
            _ => None,
        };
        if let Some(mode) = mode {
            self.selector.select_song(mode);
        }
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            // Volume (0-100 scale)
            57 => {
                (self
                    .selector
                    .app_config
                    .audio_config()
                    .map_or(0.5, |a| a.systemvolume)
                    * 100.0) as i32
            }
            58 => {
                (self
                    .selector
                    .app_config
                    .audio_config()
                    .map_or(0.5, |a| a.keyvolume)
                    * 100.0) as i32
            }
            59 => {
                (self
                    .selector
                    .app_config
                    .audio_config()
                    .map_or(0.5, |a| a.bgvolume)
                    * 100.0) as i32
            }
            // Display timing
            12 => self.selector.config.judgetiming,
            // Song BPM
            90 => self.selected_song_data().map_or(0, |s| s.maxbpm),
            91 => self.selected_song_data().map_or(0, |s| s.minbpm),
            92 => {
                // mainbpm: use maxbpm as approximation
                self.selected_song_data().map_or(0, |s| s.maxbpm)
            }
            // Song play/clear/fail counts
            77 => self.selected_score().map_or(0, |s| s.playcount),
            78 => self.selected_score().map_or(0, |s| s.clearcount),
            79 => {
                let score = self.selected_score();
                score.map_or(0, |s| s.playcount - s.clearcount)
            }
            // Song duration
            312 => self.selected_song_data().map_or(0, |s| s.length),
            1163 => self.selected_song_data().map_or(0, |s| s.length / 60000),
            1164 => self
                .selected_song_data()
                .map_or(0, |s| (s.length % 60000) / 1000),
            // Total notes
            350 => self.selected_song_data().map_or(0, |s| s.notes),
            // System time
            20 => 60, // placeholder FPS
            21 => {
                let now = chrono::Local::now();
                chrono::Datelike::year(&now)
            }
            22 => {
                let now = chrono::Local::now();
                chrono::Datelike::month(&now) as i32
            }
            23 => {
                let now = chrono::Local::now();
                chrono::Datelike::day(&now) as i32
            }
            24 => {
                let now = chrono::Local::now();
                chrono::Timelike::hour(&now) as i32
            }
            25 => {
                let now = chrono::Local::now();
                chrono::Timelike::minute(&now) as i32
            }
            26 => {
                let now = chrono::Local::now();
                chrono::Timelike::second(&now) as i32
            }
            // Playtime (hours/minutes/seconds from boot)
            17 => (self.timer.now_time() / 3_600_000) as i32,
            18 => ((self.timer.now_time() % 3_600_000) / 60_000) as i32,
            19 => ((self.timer.now_time() % 60_000) / 1_000) as i32,
            _ => 0,
        }
    }

    fn string_value(&self, id: i32) -> String {
        match id {
            // Song metadata
            10 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.title.clone()),
            11 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.subtitle.clone()),
            12 => self.selected_song_data().map_or_else(String::new, |s| {
                if s.subtitle.is_empty() {
                    s.title.clone()
                } else {
                    format!("{} {}", s.title, s.subtitle)
                }
            }),
            13 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.genre.clone()),
            14 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.artist.clone()),
            15 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.subartist.clone()),
            16 => self.selected_song_data().map_or_else(String::new, |s| {
                if s.subartist.is_empty() {
                    s.artist.clone()
                } else {
                    format!("{} {}", s.artist, s.subartist)
                }
            }),
            // Directory
            1000 => self.selected_bar().map_or_else(String::new, |b| {
                if let Some(sb) = b.as_song_bar() {
                    sb.song_data().folder.clone()
                } else {
                    String::new()
                }
            }),
            // Version
            1010 => String::from("rubato"),
            // Song hash
            1030 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.md5.clone()),
            1031 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.sha256.clone()),
            _ => String::new(),
        }
    }

    fn boolean_value(&self, id: i32) -> bool {
        use rubato_skin::skin_property::*;
        match id {
            // Bar type
            OPTION_SONGBAR => self
                .selected_bar()
                .is_some_and(|b| b.as_song_bar().is_some()),
            OPTION_FOLDERBAR => self.selected_bar().is_some_and(|b| b.is_directory_bar()),
            OPTION_GRADEBAR => self
                .selected_bar()
                .is_some_and(|b| b.as_grade_bar().is_some()),
            // Select bar clear conditions
            OPTION_SELECT_BAR_NOT_PLAYED => self.selected_bar().is_none_or(|b| b.lamp(true) == 0),
            OPTION_SELECT_BAR_FAILED => self.selected_bar().is_some_and(|b| b.lamp(true) == 1),
            OPTION_SELECT_BAR_ASSIST_EASY_CLEARED => {
                self.selected_bar().is_some_and(|b| b.lamp(true) == 2)
            }
            OPTION_SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED => {
                self.selected_bar().is_some_and(|b| b.lamp(true) == 3)
            }
            OPTION_SELECT_BAR_EASY_CLEARED => {
                self.selected_bar().is_some_and(|b| b.lamp(true) == 4)
            }
            OPTION_SELECT_BAR_NORMAL_CLEARED => {
                self.selected_bar().is_some_and(|b| b.lamp(true) == 5)
            }
            OPTION_SELECT_BAR_HARD_CLEARED => {
                self.selected_bar().is_some_and(|b| b.lamp(true) == 6)
            }
            OPTION_SELECT_BAR_EXHARD_CLEARED => {
                self.selected_bar().is_some_and(|b| b.lamp(true) == 7)
            }
            OPTION_SELECT_BAR_FULL_COMBO_CLEARED => {
                self.selected_bar().is_some_and(|b| b.lamp(true) == 8)
            }
            OPTION_SELECT_BAR_PERFECT_CLEARED => {
                self.selected_bar().is_some_and(|b| b.lamp(true) == 9)
            }
            OPTION_SELECT_BAR_MAX_CLEARED => {
                self.selected_bar().is_some_and(|b| b.lamp(true) == 10)
            }
            // Replay data (not yet wired - replay storage not implemented)
            197 | 1197 | 1200 | 1203 => false, // OPTION_REPLAYDATA variants
            196 | 1196 | 1199 | 1202 => true,  // OPTION_NO_REPLAYDATA variants
            // Autoplay
            33 => false, // OPTION_AUTOPLAYON - not in select screen
            32 => true,  // OPTION_AUTOPLAYOFF
            // Panels (always visible on select)
            21 => true, // OPTION_PANEL1
            _ => false,
        }
    }

    fn float_value(&self, id: i32) -> f32 {
        match id {
            // Music select scroll position
            1 => self.selector.manager.selected_position(),
            // Volume (0.0-1.0)
            17 => self
                .selector
                .app_config
                .audio_config()
                .map_or(0.5, |a| a.systemvolume),
            18 => self
                .selector
                .app_config
                .audio_config()
                .map_or(0.5, |a| a.keyvolume),
            19 => self
                .selector
                .app_config
                .audio_config()
                .map_or(0.5, |a| a.bgvolume),
            8 => self.selector.ranking_position(),
            // Level (0.0-1.0 normalized)
            103 => self
                .selected_song_data()
                .map_or(0.0, |s| s.level as f32 / 12.0),
            // Hi-speed (from default mode7 play config)
            310 => self.selector.config.mode7.playconfig.hispeed,
            _ => 0.0,
        }
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        match id {
            1 => self.selector.manager.set_selected_position(value),
            8 => self.selector.set_ranking_position(value),
            17 => {
                if let Some(audio) = self.selector.app_config.audio.as_mut() {
                    audio.systemvolume = value.clamp(0.0, 1.0);
                }
            }
            18 => {
                if let Some(audio) = self.selector.app_config.audio.as_mut() {
                    audio.keyvolume = value.clamp(0.0, 1.0);
                }
            }
            19 => {
                if let Some(audio) = self.selector.app_config.audio.as_mut() {
                    audio.bgvolume = value.clamp(0.0, 1.0);
                }
            }
            _ => {}
        }
    }
}

/// Minimal adapter implementing rubato_skin::stubs::MainState for BarRenderer's RenderContext.
/// Delegates timer() to a Timer snapshot; other methods use defaults.
struct MinimalSkinMainState<'a> {
    timer: &'a rubato_skin::stubs::Timer,
}

impl<'a> MinimalSkinMainState<'a> {
    fn new(timer: &'a rubato_skin::stubs::Timer) -> Self {
        Self { timer }
    }
}

impl rubato_skin::stubs::MainState for MinimalSkinMainState<'_> {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
        self.timer
    }

    fn get_offset_value(&self, _id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        None
    }

    fn get_main(&self) -> &rubato_skin::stubs::MainController {
        static MC: std::sync::OnceLock<rubato_skin::stubs::MainController> =
            std::sync::OnceLock::new();
        MC.get_or_init(|| rubato_skin::stubs::MainController { debug: false })
    }

    fn get_image(&self, _id: i32) -> Option<rubato_skin::stubs::TextureRegion> {
        None
    }

    fn get_resource(&self) -> &rubato_skin::stubs::PlayerResource {
        static RES: std::sync::OnceLock<rubato_skin::stubs::PlayerResource> =
            std::sync::OnceLock::new();
        RES.get_or_init(|| rubato_skin::stubs::PlayerResource)
    }
}

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

    /// Skin bar data (bar body images, lamps, text, etc.)
    pub skin_bar: Option<super::skin_bar::SkinBar>,

    /// Center bar index from skin
    pub select_center_bar: i32,

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

    /// Pending state change request (outbox pattern).
    /// MainController polls this via take_pending_state_change() each frame.
    pending_state_change: Option<MainStateType>,

    /// Local PlayerResource for BMS file loading in read_chart().
    /// Handed off to MainController via take_player_resource_box() during state transition.
    player_resource: Option<rubato_core::player_resource::PlayerResource>,
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
            skin_bar: None,
            select_center_bar: 0,
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
            pending_state_change: None,
            player_resource: None,
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

    /// Set the player config (play options, key bindings, etc.).
    pub fn set_player_config(&mut self, config: PlayerConfig) {
        self.config = config;
    }

    /// Refresh the bar list with song database context.
    /// Wraps BarManager::update_bar_refresh_with_context to supply the context
    /// from MusicSelector fields, ensuring songdb queries are not skipped.
    fn refresh_bar_with_context(&mut self) {
        let mut ctx = BarManager::make_context(
            &self.app_config,
            &mut self.config,
            &*self.songdb,
            self.scorecache.as_mut(),
        );
        self.manager.update_bar_refresh_with_context(Some(&mut ctx));
    }

    /// Navigate into a bar (directory, folder, etc.) with song database context.
    /// Used by MusicSelectCommand and ContextMenuBar executors.
    pub fn update_bar_with_songdb_context(&mut self, bar: Option<&Bar>) -> bool {
        let mut ctx = BarManager::make_context(
            &self.app_config,
            &mut self.config,
            &*self.songdb,
            self.scorecache.as_mut(),
        );
        self.manager.update_bar_with_context(bar, Some(&mut ctx))
    }

    pub fn set_rival(&mut self, rival: Option<PlayerInformation>) {
        // In Java: finds rival index, sets rival and rival cache, updates bar
        self.rival = rival;
        self.rivalcache = None;
        self.refresh_bar_with_context();
        log::info!(
            "Rival changed: {}",
            self.rival.as_ref().map(|r| r.name()).unwrap_or("None")
        );
    }

    pub fn rival(&self) -> Option<&PlayerInformation> {
        self.rival.as_ref()
    }

    pub fn score_data_cache(&self) -> Option<&ScoreDataCache> {
        self.scorecache.as_ref()
    }

    pub fn rival_score_data_cache(&self) -> Option<&ScoreDataCache> {
        self.rivalcache.as_ref()
    }

    pub fn selected_replay(&self) -> i32 {
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
                let current_mode = self.config.mode().cloned();
                let mut idx = 0;
                for (i, m) in MODE.iter().enumerate() {
                    if *m == current_mode {
                        idx = i;
                        break;
                    }
                }
                let step = if arg1 >= 0 { 1 } else { MODE.len() - 1 };
                self.config.mode = MODE[(idx + step) % MODE.len()].clone();
                self.refresh_bar_with_context();
                self.play_option_change();
            }
            EventType::Sort => {
                let count = BarSorter::DEFAULT_SORTER.len() as i32;
                let step = if arg1 >= 0 { 1 } else { count - 1 };
                self.set_sort((self.sort() + step) % count);
                self.refresh_bar_with_context();
                self.play_option_change();
            }
            EventType::Lnmode => {
                let step = if arg1 >= 0 { 1 } else { 2 };
                self.config.lnmode = (self.config.lnmode + step) % 3;
                self.play_option_change();
            }
            EventType::Option1p => {
                let step = if arg1 >= 0 { 1 } else { 9 };
                self.config.random = (self.config.random + step) % 10;
                self.play_option_change();
            }
            EventType::Option2p => {
                let step = if arg1 >= 0 { 1 } else { 9 };
                self.config.random2 = (self.config.random2 + step) % 10;
                self.play_option_change();
            }
            EventType::Optiondp => {
                let step = if arg1 >= 0 { 1 } else { 3 };
                self.config.doubleoption = (self.config.doubleoption + step) % 4;
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
                    pc.fixhispeed = (pc.fixhispeed + step) % 5;
                }
                self.play_option_change();
            }
            EventType::Duration1p => {
                if let Some(pc) = self.get_selected_play_config_mut() {
                    let delta = if _arg2 != 0 { _arg2 } else { 1 };
                    let step = if arg1 >= 0 { delta } else { -delta };
                    let new_val = (pc.duration + step).clamp(1, 5000);
                    pc.duration = new_val;
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
                let targets = rubato_play::target_property::TargetProperty::targets();
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
                    let rival_count = main.rival_count();
                    // Find current rival's index in the rival list
                    let mut index: i32 = -1;
                    for i in 0..rival_count {
                        if let Some(info) = main.rival_information(i)
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
                        main.rival_information(index as usize)
                    } else {
                        None
                    };
                    self.set_rival(new_rival);
                }
                self.play_option_change();
            }
            EventType::FavoriteSong => {
                let next = arg1 >= 0;
                if let Some(songbar) = self.manager.selected().and_then(|b| b.as_song_bar()) {
                    let mut sd = songbar.song_data().clone();
                    let fav = sd.favorite;
                    let current = if fav & rubato_types::song_data::FAVORITE_SONG != 0 {
                        1
                    } else if fav & rubato_types::song_data::INVISIBLE_SONG != 0 {
                        2
                    } else {
                        0
                    };
                    let new_type = (current + if next { 1 } else { 2 }) % 3;
                    sd.favorite = (fav
                        & !(rubato_types::song_data::FAVORITE_SONG
                            | rubato_types::song_data::INVISIBLE_SONG))
                        | match new_type {
                            1 => rubato_types::song_data::FAVORITE_SONG,
                            2 => rubato_types::song_data::INVISIBLE_SONG,
                            _ => 0,
                        };
                    self.songdb.set_song_datas(&[sd]);
                }
                self.play_option_change();
            }
            EventType::FavoriteChart => {
                let next = arg1 >= 0;
                if let Some(songbar) = self.manager.selected().and_then(|b| b.as_song_bar()) {
                    let mut sd = songbar.song_data().clone();
                    let fav = sd.favorite;
                    let current = if fav & rubato_types::song_data::FAVORITE_CHART != 0 {
                        1
                    } else if fav & rubato_types::song_data::INVISIBLE_CHART != 0 {
                        2
                    } else {
                        0
                    };
                    let new_type = (current + if next { 1 } else { 2 }) % 3;
                    sd.favorite = (fav
                        & !(rubato_types::song_data::FAVORITE_CHART
                            | rubato_types::song_data::INVISIBLE_CHART))
                        | match new_type {
                            1 => rubato_types::song_data::FAVORITE_CHART,
                            2 => rubato_types::song_data::INVISIBLE_CHART,
                            _ => 0,
                        };
                    self.songdb.set_song_datas(&[sd]);
                }
                self.play_option_change();
            }
            EventType::UpdateFolder => {
                if let Some(ref mut main) = self.main
                    && let Some(selected) = self.manager.selected()
                {
                    if let Some(folder) = selected.as_folder_bar()
                        && let Some(fd) = folder.folder_data()
                    {
                        let path = fd.path().to_string();
                        main.update_song(Some(&path));
                    } else if let Some(songbar) = selected.as_song_bar()
                        && let Some(path) = songbar.song_data().path()
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
                if let Some(songbar) = self.manager.selected().and_then(|b| b.as_song_bar())
                    && let Some(path) = songbar.song_data().path()
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
                if let Some(songbar) = self.manager.selected().and_then(|b| b.as_song_bar())
                    && let Some(path) = songbar.song_data().path()
                    && let Some(parent) = std::path::Path::new(path).parent()
                    && let Err(e) = open::that(parent)
                {
                    log::error!("Failed to open folder: {}", e);
                }
            }
            EventType::OpenIr => {
                if let Some(songbar) = self.manager.selected().and_then(|b| b.as_song_bar()) {
                    let sd = songbar.song_data();
                    if let Some(ref main) = self.main
                        && let Some(url) = main.ir_song_url(sd)
                        && let Err(e) = open::that(&url)
                    {
                        log::error!("Failed to open IR URL: {}", e);
                    }
                } else if let Some(gradebar) =
                    self.manager.selected().and_then(|b| b.as_grade_bar())
                {
                    let cd = gradebar.course_data();
                    if let Some(ref main) = self.main
                        && let Some(url) = main.ir_course_url(cd)
                        && let Err(e) = open::that(&url)
                    {
                        log::error!("Failed to open IR URL: {}", e);
                    }
                }
            }
            EventType::OpenDownloadSite => {
                if let Some(songbar) = self.manager.selected().and_then(|b| b.as_song_bar()) {
                    let sd = songbar.song_data();
                    let url = sd.url();
                    if !url.is_empty()
                        && let Err(e) = open::that(url)
                    {
                        log::error!("Failed to open download site: {}", e);
                    }
                    let appendurl = sd.appendurl();
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

    fn selected_play_config_mode(&self) -> Option<bms_model::Mode> {
        if let Some(song_bar) = self.manager.selected().and_then(|bar| bar.as_song_bar())
            && song_bar.exists_song()
        {
            return play_config_mode_from_song(song_bar.song_data());
        }

        if let Some(grade_bar) = self.manager.selected().and_then(|bar| bar.as_grade_bar())
            && grade_bar.exists_all_songs()
        {
            let mut selected_mode: Option<bms_model::Mode> = None;
            for song in grade_bar.song_datas() {
                let song_mode = play_config_mode_from_song(song)?;
                if let Some(current_mode) = selected_mode.as_ref() {
                    if *current_mode != song_mode {
                        return None;
                    }
                } else {
                    selected_mode = Some(song_mode);
                }
            }
            if selected_mode.is_some() {
                return selected_mode;
            }
        }

        Some(normalized_play_config_mode(
            self.config
                .mode()
                .cloned()
                .unwrap_or(bms_model::Mode::BEAT_7K),
        ))
    }

    fn get_selected_play_config_ref(&self) -> Option<&PlayConfig> {
        let mode = self.selected_play_config_mode()?;
        Some(&self.config.play_config_ref(mode).playconfig)
    }

    /// Get mutable reference to the PlayConfig for the currently selected mode.
    /// Matches Java MusicSelector.getSelectedBarPlayConfig().
    fn get_selected_play_config_mut(&mut self) -> Option<&mut PlayConfig> {
        let mode = self.selected_play_config_mode()?;
        Some(&mut self.config.play_config(mode).playconfig)
    }

    /// Read a chart for play.
    /// Corresponds to Java MusicSelector.readChart(SongData, Bar)
    pub fn read_chart(&mut self, song: &SongData, current: &Bar) {
        // Get play mode for set_bms_file encoding
        let (mode_type, mode_id) = Self::encode_bms_player_mode(self.play.as_ref());

        // Ensure local PlayerResource exists
        if self.player_resource.is_none() {
            self.player_resource = Some(rubato_core::player_resource::PlayerResource::new(
                self.app_config.clone(),
                self.config.clone(),
            ));
        }
        let res = self.player_resource.as_mut().unwrap();
        res.clear();

        // resource.setBMSFile(path, play)
        let path_str = match song.path() {
            Some(p) => p,
            None => {
                ImGuiNotify::error("Failed to loading BMS : Song not found, or Song has error");
                return;
            }
        };
        let path = std::path::Path::new(&path_str);

        let load_success = PlayerResourceAccess::set_bms_file(res, path, mode_type, mode_id);

        if load_success {
            // Set table name/level from directory hierarchy
            let table_urls: Vec<String> = self
                .main
                .as_ref()
                .map(|m| m.config().table_url.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default();

            let dir = self.manager.directory();
            if !dir.is_empty()
                && !matches!(dir.last(), Some(bar) if matches!(**bar, Bar::SameFolder(_)))
            {
                let mut is_dtable = false;
                let mut tablename: Option<String> = None;
                let mut tablelevel: Option<String> = None;

                for bar in dir {
                    if let Some(tb) = bar.as_table_bar()
                        && let Some(url) = tb.url()
                        && table_urls.iter().any(|u| u == url)
                    {
                        is_dtable = true;
                        tablename = Some(bar.title());
                    }
                    if bar.as_hash_bar().is_some() && is_dtable {
                        tablelevel = Some(bar.title());
                        break;
                    }
                }

                let res = self.player_resource.as_mut().unwrap();
                if let Some(ref name) = tablename {
                    res.set_tablename(name);
                }
                if let Some(ref level) = tablelevel {
                    res.set_tablelevel(level);
                }
            }

            // Java L384-388: only create new RankingData when IR active AND currentir is null.
            // Do NOT null out currentir when IR inactive (selectedBarMoved already set it).
            if let Some(ref mut main) = self.main
                && main.ir_connection_any().is_some()
                && self.currentir.is_none()
            {
                use rubato_ir::ranking_data::RankingData;
                let lnmode = main.player_config().lnmode;
                let rd = RankingData::new();
                self.currentir = Some(rd.clone());
                if let Some(cache) = main.ranking_data_cache_mut() {
                    cache.put_song_any(song, lnmode, Box::new(rd));
                }
            }
            // Java L388: resource.setRankingData(currentir)
            {
                let res = self.player_resource.as_mut().unwrap();
                let ranking_any = self
                    .currentir
                    .clone()
                    .map(|rd| Box::new(rd) as Box<dyn std::any::Any + Send + Sync>);
                res.set_ranking_data_any(ranking_any);

                // Set rival score
                let rival_score = current.rival_score().cloned();
                res.set_rival_score_data_option(rival_score);
            }

            // Chart replication mode
            let songdata = self
                .player_resource
                .as_ref()
                .and_then(|r| r.songdata())
                .cloned();
            let replay_index = self.play.as_ref().map_or(0, |p| p.id);
            let chart_option = if let Some(main_ref) = self.main.as_deref() {
                Self::compute_chart_option(
                    &self.config,
                    current.rival_score(),
                    main_ref,
                    songdata.as_ref(),
                    replay_index,
                )
            } else {
                None
            };
            self.player_resource
                .as_mut()
                .unwrap()
                .set_chart_option_data(chart_option);

            self.playedsong = Some(song.clone());
            self.pending_state_change = Some(MainStateType::Decide);
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
    ) -> Option<rubato_types::replay_data::ReplayData> {
        let mode = ChartReplicationMode::get(&config.chart_replication_mode);
        match mode {
            ChartReplicationMode::None => None,
            ChartReplicationMode::RivalChart => rival_score.map(|rival| {
                let mut opt = rubato_types::replay_data::ReplayData::new();
                opt.randomoption = rival.option % 10;
                opt.randomoption2 = (rival.option / 10) % 10;
                opt.doubleoption = rival.option / 100;
                opt.randomoptionseed = rival.seed % (65536 * 256);
                opt.randomoption2seed = rival.seed / (65536 * 256);
                opt
            }),
            ChartReplicationMode::RivalOption => rival_score.map(|rival| {
                let mut opt = rubato_types::replay_data::ReplayData::new();
                opt.randomoption = rival.option % 10;
                opt.randomoption2 = (rival.option / 10) % 10;
                opt.doubleoption = rival.option / 100;
                opt
            }),
            ChartReplicationMode::ReplayChart | ChartReplicationMode::ReplayOption => {
                let sd = songdata?;
                let sha256 = &sd.sha256;
                let has_ln = sd.has_undefined_long_note();
                let replay = main.read_replay_data(sha256, has_ln, config.lnmode, replay_index)?;
                let mut opt = rubato_types::replay_data::ReplayData::new();
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

    pub fn sort(&self) -> i32 {
        self.config.sort
    }

    pub fn set_sort(&mut self, sort: i32) {
        self.config.sort = sort;
        self.config
            .set_sortid(BarSorter::DEFAULT_SORTER[sort as usize].name().to_string());
    }

    pub fn panel_state(&self) -> i32 {
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

    pub fn song_database(&self) -> &dyn SongDatabaseAccessor {
        &*self.songdb
    }

    /// Check if the selected bar's course data contains the given constraint.
    /// Corresponds to Java MusicSelector.existsConstraint(CourseDataConstraint)
    pub fn exists_constraint(&self, constraint: &CourseDataConstraint) -> bool {
        let selected = match self.manager.selected() {
            Some(s) => s,
            None => return false,
        };

        if let Some(grade) = selected.as_grade_bar() {
            for con in &grade.course_data().constraint {
                if con == constraint {
                    return true;
                }
            }
        } else if let Some(rc) = selected.as_random_course_bar() {
            for con in &rc.course_data().constraint {
                if *con == *constraint {
                    return true;
                }
            }
        }
        false
    }

    pub fn selected_bar(&self) -> Option<&Bar> {
        self.manager.selected()
    }

    pub fn bar_render(&self) -> Option<&BarRenderer> {
        self.bar.as_ref()
    }

    pub fn bar_manager(&self) -> &BarManager {
        &self.manager
    }

    pub fn bar_manager_mut(&mut self) -> &mut BarManager {
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
            && preview.song_data().is_some()
        {
            let should_stop = match self.manager.selected() {
                Some(bar) => {
                    if let Some(song_bar) = bar.as_song_bar() {
                        if let Some(preview_song) = preview.song_data() {
                            song_bar.song_data().folder != preview_song.folder
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
            .map(|m| m.ir_connection_any().is_some())
            .unwrap_or(false);

        if ir_active {
            if let Some(current) = self.manager.selected() {
                if let Some(song_bar) = current.as_song_bar() {
                    if song_bar.exists_song() {
                        // Refresh currentir from cache
                        if let Some(main) = self.main.as_ref() {
                            use rubato_ir::ranking_data::RankingData;
                            let lnmode = main.player_config().lnmode;
                            let song = song_bar.song_data();
                            self.currentir = main
                                .ranking_data_cache()
                                .and_then(|c| c.song_any(song, lnmode))
                                .and_then(|a| a.downcast::<RankingData>().ok())
                                .map(|ranking| *ranking);
                        }
                        let ranking_reload_dur = self.ranking_reload_duration;
                        let ranking_dur = self.ranking_duration as i64;
                        self.current_ranking_duration = if let Some(ref ir) = self.currentir {
                            (ranking_reload_dur - (now_millis - ir.last_update_time())).max(0)
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
                            use rubato_ir::ranking_data::RankingData;
                            let lnmode = main.player_config().lnmode;
                            let course = grade_bar.course_data();
                            self.currentir = main
                                .ranking_data_cache()
                                .and_then(|c| c.course_any(course, lnmode))
                                .and_then(|a| a.downcast::<RankingData>().ok())
                                .map(|ranking| *ranking);
                        }
                        let ranking_reload_dur = self.ranking_reload_duration;
                        let ranking_dur = self.ranking_duration as i64;
                        self.current_ranking_duration = if let Some(ref ir) = self.currentir {
                            (ranking_reload_dur - (now_millis - ir.last_update_time())).max(0)
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
        let (banner_data, stagefile_data) = match self.manager.selected() {
            Some(Bar::Song(song_bar)) => {
                let banner = song_bar
                    .banner()
                    .map(|p| (p.width, p.height, p.data().to_vec()));
                let stagefile = song_bar
                    .stagefile()
                    .map(|p| (p.width, p.height, p.data().to_vec()));
                (banner, stagefile)
            }
            _ => (None, None),
        };

        // Set banner and stagefile on the player resource's BMSResource
        if let Some(res) = self.player_resource.as_mut() {
            res.set_bms_banner_raw(banner_data);
            res.set_bms_stagefile_raw(stagefile_data);
        }
    }

    /// Select a bar (open directory or set play mode).
    /// Corresponds to Java MusicSelector.select(Bar)
    pub fn select(&mut self, current: &Bar) {
        if current.is_directory_bar() {
            let mut ctx = BarManager::make_context(
                &self.app_config,
                &mut self.config,
                &*self.songdb,
                self.scorecache.as_mut(),
            );
            if self
                .manager
                .update_bar_with_context(Some(current), Some(&mut ctx))
            {
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
        if input.control_key_state(ControlKeys::Num6) {
            self.pending_state_change = Some(MainStateType::Config);
        } else if input.is_activated(KeyCommand::OpenSkinConfiguration) {
            self.pending_state_change = Some(MainStateType::SkinConfig);
        }

        // Classify the selected bar before borrowing musicinput
        let selected_bar_type = BarType::classify(self.manager.selected());
        let selected_replay = self.selectedreplay;
        let is_top_level = self.manager.directory().is_empty();

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
                let property_idx = self.config.musicselectinput as usize;
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
                    let mut ctx = BarManager::make_context(
                        &self.app_config,
                        &mut self.config,
                        &*self.songdb,
                        self.scorecache.as_mut(),
                    );
                    self.manager.close_with_context(Some(&mut ctx));
                }
                InputEvent::OpenDirectory => {
                    // In Java: select.getBarManager().updateBar(dirbar)
                    let mut ctx = BarManager::make_context(
                        &self.app_config,
                        &mut self.config,
                        &*self.songdb,
                        self.scorecache.as_mut(),
                    );
                    let opened = self
                        .manager
                        .update_bar_with_selected_and_context(Some(&mut ctx));
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
                    self.pending_state_change = Some(state_type);
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

    pub fn selected_bar_play_config(&self) -> Option<&PlayConfig> {
        let mode = self
            .config
            .mode()
            .cloned()
            .unwrap_or(bms_model::Mode::BEAT_7K);
        Some(&self.config.play_config_ref(mode).playconfig)
    }

    pub fn current_ranking_data(&self) -> Option<&RankingData> {
        self.currentir.as_ref()
    }

    pub fn current_ranking_duration(&self) -> i64 {
        self.current_ranking_duration
    }

    pub fn ranking_offset(&self) -> i32 {
        self.ranking_offset
    }

    pub fn ranking_position(&self) -> f32 {
        let ranking_max = self
            .currentir
            .as_ref()
            .map(|ir| ir.total_player().max(1))
            .unwrap_or(1);
        self.ranking_offset as f32 / ranking_max as f32
    }

    pub fn set_ranking_position(&mut self, value: f32) {
        if (0.0..1.0).contains(&value) {
            let ranking_max = self
                .currentir
                .as_ref()
                .map(|ir| ir.total_player().max(1))
                .unwrap_or(1);
            self.ranking_offset = (ranking_max as f32 * value) as i32;
        }
    }

    /// Read course (grade bar) for play.
    /// Corresponds to Java MusicSelector.readCourse(BMSPlayerMode)
    fn read_course(&mut self, mode: BMSPlayerMode) {
        // Get selected bar and check it's a GradeBar
        let grade_bar = match self.manager.selected() {
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
                .and_then(|m| m.http_downloader())
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
        let rc_bar = match self.manager.selected() {
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
        let mut rcd = rcb.course_data().clone();
        {
            let songdb = self.song_database();
            let player_name = self.app_config.playername.as_deref().unwrap_or("default");
            let score_path = format!("{}/{}/score.db", self.app_config.playerpath, player_name);
            let scorelog_path =
                format!("{}/{}/scorelog.db", self.app_config.playerpath, player_name);
            let songinfo_path = self.app_config.songinfopath.to_string();
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
                let dir_string = self.manager.directory_string().to_string();
                self.manager.add_random_course(gb.clone(), dir_string);
                {
                    let mut ctx = BarManager::make_context(
                        &self.app_config,
                        &mut self.config,
                        &*self.songdb,
                        self.scorecache.as_mut(),
                    );
                    self.manager.update_bar_with_context(None, Some(&mut ctx));
                }
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
        if self.player_resource.is_none() {
            self.player_resource = Some(rubato_core::player_resource::PlayerResource::new(
                self.app_config.clone(),
                self.config.clone(),
            ));
        }
        let res = self.player_resource.as_mut().unwrap();
        res.clear();
        res.set_auto_play_songs(paths, false);
        if res.next_song() {
            self.pending_state_change = Some(MainStateType::Decide);
        }
    }

    /// Internal course reading implementation.
    /// Corresponds to Java MusicSelector._readCourse(BMSPlayerMode, GradeBar)
    fn _read_course(&mut self, mode: &BMSPlayerMode, grade_bar: &Bar) -> bool {
        // Get song paths from grade bar
        let gb = match grade_bar.as_grade_bar() {
            Some(gb) => gb,
            None => return false,
        };

        let songs = gb.song_datas();
        let files: Vec<PathBuf> = songs
            .iter()
            .filter_map(|s| s.path().map(PathBuf::from))
            .collect();

        if files.len() != songs.len() {
            log::warn!("_read_course: some songs have no path");
            return false;
        }

        // Ensure local PlayerResource exists
        if self.player_resource.is_none() {
            self.player_resource = Some(rubato_core::player_resource::PlayerResource::new(
                self.app_config.clone(),
                self.config.clone(),
            ));
        }
        let res = self.player_resource.as_mut().unwrap();
        res.clear();

        // resource.setCourseBMSFiles(files)
        let load_success = res.set_course_bms_files(&files);

        if load_success {
            // Apply constraints for PLAY/AUTOPLAY modes only
            if mode.mode == BMSPlayerModeType::Play || mode.mode == BMSPlayerModeType::Autoplay {
                for constraint in &gb.course_data().constraint {
                    match constraint {
                        CourseDataConstraint::Class => {
                            self.config.random = 0;
                            self.config.random2 = 0;
                            self.config.doubleoption = 0;
                        }
                        CourseDataConstraint::Mirror => {
                            if self.config.random == 1 {
                                self.config.random2 = 1;
                                self.config.doubleoption = 1;
                            } else {
                                self.config.random = 0;
                                self.config.random2 = 0;
                                self.config.doubleoption = 0;
                            }
                        }
                        CourseDataConstraint::Random => {
                            if self.config.random > 5 {
                                self.config.random = 0;
                            }
                            if self.config.random2 > 5 {
                                self.config.random2 = 0;
                            }
                        }
                        CourseDataConstraint::Ln => {
                            self.config.lnmode = 0;
                        }
                        CourseDataConstraint::Cn => {
                            self.config.lnmode = 1;
                        }
                        CourseDataConstraint::Hcn => {
                            self.config.lnmode = 2;
                        }
                        _ => {}
                    }
                }
            }

            // Update course data with song data from loaded models
            let course_song_data = self
                .player_resource
                .as_ref()
                .map(|r| r.course_song_data())
                .unwrap_or_default();

            let mut course_data = gb.course_data().clone();
            course_data.hash = course_song_data;

            // resource.setCourseData, setBMSFile for first song
            let (mode_type, mode_id) = Self::encode_bms_player_mode(Some(mode));
            {
                let res = self.player_resource.as_mut().unwrap();
                res.set_course_data(course_data.clone());
                if !files.is_empty() {
                    PlayerResourceAccess::set_bms_file(res, &files[0], mode_type, mode_id);
                }
            }

            self.playedcourse = Some(course_data);

            // Load/create cached IR ranking data for course
            if let Some(ref mut main) = self.main {
                use rubato_ir::ranking_data::RankingData;
                let lnmode = main.player_config().lnmode;
                let course = gb.course_data();
                let cached = main
                    .ranking_data_cache()
                    .and_then(|c| c.course_any(course, lnmode))
                    .and_then(|a| a.downcast::<RankingData>().ok())
                    .map(|ranking| *ranking);
                if let Some(rd) = cached {
                    self.currentir = Some(rd);
                } else {
                    let rd = RankingData::new();
                    self.currentir = Some(rd.clone());
                    if let Some(cache) = main.ranking_data_cache_mut() {
                        cache.put_course_any(course, lnmode, Box::new(rd));
                    }
                }
            }
            // Set rival score/chart option to None for course play
            {
                let res = self.player_resource.as_mut().unwrap();
                res.set_rival_score_data_option(None);
                res.set_chart_option_data(None);
            }

            self.pending_state_change = Some(MainStateType::Decide);
            true
        } else {
            false
        }
    }

    /// Get banner resource pool.
    /// Corresponds to Java MusicSelector.getBannerResource()
    pub fn banner_resource(&self) -> &PixmapResourcePool {
        &self.banners
    }

    /// Get stagefile resource pool.
    /// Corresponds to Java MusicSelector.getStagefileResource()
    pub fn stagefile_resource(&self) -> &PixmapResourcePool {
        &self.stagefiles
    }
}

// ============================================================
// SongSelectionAccess trait implementation
// ============================================================

impl rubato_types::song_selection_access::SongSelectionAccess for MusicSelector {
    fn selected_song_data(&self) -> Option<SongData> {
        let bar = self.selected_bar()?;
        bar.as_song_bar().map(|sb| sb.song_data().clone())
    }

    fn selected_score_data(&self) -> Option<ScoreData> {
        let bar = self.selected_bar()?;
        bar.as_song_bar()
            .and_then(|sb| sb.selectable.bar_data.score().cloned())
    }

    fn reverse_lookup_data(&self) -> Vec<String> {
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
        match rubato_skin::skin_loader::load_skin_from_config(
            &self.app_config,
            &self.config,
            skin_type,
        ) {
            Some(mut skin) => {
                log::info!("Skin loaded for type {}", skin_type);

                // Extract bar data before boxing into dyn SkinDrawable
                if let Some(bar_data) = skin.take_select_bar_data() {
                    let mut skin_bar = super::skin_bar::SkinBar::new(bar_data.center_bar);
                    // Pad LR2's 20-element vecs to SkinBar's 60-element vecs
                    for (i, img) in bar_data.barimageon.into_iter().enumerate() {
                        if i < skin_bar.barimageon.len() {
                            skin_bar.barimageon[i] = img;
                        }
                    }
                    for (i, img) in bar_data.barimageoff.into_iter().enumerate() {
                        if i < skin_bar.barimageoff.len() {
                            skin_bar.barimageoff[i] = img;
                        }
                    }
                    // Transfer bar level SkinNumber objects
                    for (i, level) in bar_data.barlevel.into_iter().enumerate() {
                        if let Some(sn) = level {
                            skin_bar.set_barlevel(i as i32, sn);
                        }
                    }
                    // Transfer bar title SkinText objects
                    for (i, text) in bar_data.bartext.into_iter().enumerate() {
                        if let Some(t) = text {
                            skin_bar.set_text(i, t);
                        }
                    }
                    // Transfer lamp images
                    for (i, lamp) in bar_data.barlamp.into_iter().enumerate() {
                        if let Some(img) = lamp {
                            skin_bar.set_lamp_image(i as i32, img);
                        }
                    }
                    // Transfer player lamp images
                    for (i, lamp) in bar_data.barmylamp.into_iter().enumerate() {
                        if let Some(img) = lamp {
                            skin_bar.set_player_lamp(i as i32, img);
                        }
                    }
                    // Transfer rival lamp images
                    for (i, lamp) in bar_data.barrivallamp.into_iter().enumerate() {
                        if let Some(img) = lamp {
                            skin_bar.set_rival_lamp(i as i32, img);
                        }
                    }
                    // Transfer trophy images
                    for (i, trophy) in bar_data.bartrophy.into_iter().enumerate() {
                        if let Some(img) = trophy {
                            skin_bar.set_trophy(i as i32, img);
                        }
                    }
                    // Transfer label images
                    for (i, label) in bar_data.barlabel.into_iter().enumerate() {
                        if let Some(img) = label {
                            skin_bar.set_label(i as i32, img);
                        }
                    }
                    // Transfer distribution graph
                    if let Some(graph_type) = bar_data.graph_type {
                        let mut graph = if let Some(images) = bar_data.graph_images {
                            super::skin_distribution_graph::SkinDistributionGraph::new_with_images(
                                graph_type, images,
                            )
                        } else {
                            super::skin_distribution_graph::SkinDistributionGraph::new(graph_type)
                        };
                        graph.region.x = bar_data.graph_region.x;
                        graph.region.y = bar_data.graph_region.y;
                        graph.region.width = bar_data.graph_region.width;
                        graph.region.height = bar_data.graph_region.height;
                        skin_bar.set_graph(graph);
                    }
                    self.select_center_bar = bar_data.center_bar;
                    self.skin_bar = Some(skin_bar);
                    self.bar = Some(BarRenderer::new(300, 100, 5));
                    log::info!(
                        "Bar data extracted: center_bar={}, clickable={}",
                        bar_data.center_bar,
                        bar_data.clickable_bar.len()
                    );
                }

                self.main_state_data.skin = Some(Box::new(skin));
            }
            None => {
                log::warn!("Failed to load skin for type {}", skin_type);
            }
        }
    }

    fn sound(&self, sound: SoundType) -> Option<String> {
        self.main.as_ref().and_then(|m| m.sound_path(&sound))
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

    fn sync_audio(&mut self, audio: &mut dyn AudioDriver) {
        if let Some(preview) = &mut self.preview {
            preview.tick_preview(audio, &self.app_config);
        }
    }

    fn take_pending_state_change(&mut self) -> Option<MainStateType> {
        self.pending_state_change.take()
    }

    fn take_player_resource_box(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        let should_handoff = self.player_resource.as_ref().is_some_and(|resource| {
            resource.bms_model().is_some()
                || resource.songdata().is_some()
                || resource.course_data().is_some()
        });

        if !should_handoff {
            return None;
        }

        self.player_resource
            .take()
            .map(|r| Box::new(r) as Box<dyn std::any::Any + Send>)
    }

    /// Create state — initialize DB access, song list, bar manager.
    /// Corresponds to Java MusicSelector.create()
    fn create(&mut self) {
        if self.main.is_none() {
            log::warn!(
                "MusicSelector::create(): main controller not set - state transitions, sounds, and score access will be disabled"
            );
        }

        // Java: main.getSoundManager().shuffle()
        if let Some(ref mut main) = self.main {
            main.shuffle_sounds();
        }

        self.play = None;
        self.show_note_graph = false;

        // In Java: resource.setPlayerData(main.getPlayDataAccessor().readPlayerData())
        if let Some(ref mut main) = self.main {
            let player_data = main.read_player_data();
            if let Some(pd) = player_data {
                if self.player_resource.is_none() {
                    self.player_resource = Some(rubato_core::player_resource::PlayerResource::new(
                        self.app_config.clone(),
                        self.config.clone(),
                    ));
                }
                if let Some(res) = self.player_resource.as_mut() {
                    res.set_player_data(pd);
                }
            }
        }

        // Update score cache for previously played song
        if let Some(ref song) = self.playedsong {
            if let Some(ref mut cache) = self.scorecache {
                cache.update(song, self.config.lnmode);
            }
            self.playedsong = None;
        }
        // Update score cache for previously played course
        if let Some(ref course) = self.playedcourse.take() {
            for sd in &course.hash {
                if let Some(ref mut cache) = self.scorecache {
                    cache.update(sd, self.config.lnmode);
                }
            }
        }

        // Create preview music processor
        {
            let mut preview = PreviewMusicProcessor::new(&self.app_config);
            if let Some(sound_path) = self.sound(SoundType::Select) {
                preview.set_default(&sound_path);
            }
            self.preview = Some(preview);
        }

        // Configure input processor per musicselectinput mode (Java L183-188)
        // musicselectinput: 0 -> mode7, 1 -> mode9, _ -> mode14
        {
            let mut input = BMSPlayerInputProcessor::new(&self.app_config, &self.config);
            let pc = match self.config.musicselectinput {
                0 => &self.config.mode7,
                1 => &self.config.mode9,
                _ => &self.config.mode14,
            };
            input.set_keyboard_config(&pc.keyboard);
            input.set_controller_config(&mut pc.controller.to_vec());
            input.set_midi_config(&pc.midi);
            self.input_processor = Some(input);
        }

        // Java: musicinput = new MusicSelectInputProcessor(300, 50, MusicSelectInputProcessor.ANALOG_TICKS_PER_SCROLL)
        if self.musicinput.is_none() {
            self.musicinput = Some(MusicSelectInputProcessor::new(300, 50, 10));
        }

        // Build context so bar_manager can query the song database.
        // Java: BarManager has direct access to MusicSelector fields; in Rust
        // we must pass them explicitly via UpdateBarContext.
        {
            let mut ctx = BarManager::make_context(
                &self.app_config,
                &mut self.config,
                &*self.songdb,
                self.scorecache.as_mut(),
            );
            self.manager.update_bar_with_context(None, Some(&mut ctx));
        }

        // In Java: loadSkin(SkinType.MUSIC_SELECT)
        self.load_skin(SkinType::MusicSelect.id());

        // In Java: search text field setup from skin region
        // Blocked on MusicSelectSkin integration
    }

    /// Override skin rendering to add BarRenderer prepare/render around the default cycle.
    /// Java: MusicSelectSkin.render() wraps MainSkin.render() with bar logic.
    fn render_skin(&mut self, sprite: &mut rubato_render::sprite_batch::SpriteBatch) {
        use rubato_skin::skin_object::SkinObjectRenderer;
        let time = self.main_state_data.timer.now_time();

        // Prepare skin_bar sub-objects (sets data.draw = true on bar images).
        // Must be called before bar_renderer.prepare() which checks data.draw.
        if let Some(skin_bar) = &mut self.skin_bar {
            let timer_snapshot = rubato_skin::stubs::Timer::with_timers(
                self.main_state_data.timer.now_time(),
                self.main_state_data.timer.now_micro_time(),
                self.main_state_data.timer.export_timer_array(),
            );
            let adapter = MinimalSkinMainState::new(&timer_snapshot);
            skin_bar.prepare(time, &adapter);
        }

        // Bar prepare — compute bar positions
        if let (Some(bar_renderer), Some(skin_bar)) = (&mut self.bar, &self.skin_bar) {
            let ctx = PrepareContext {
                center_bar: self.select_center_bar,
                currentsongs: &self.manager.currentsongs,
                selectedindex: self.manager.selectedindex,
            };
            bar_renderer.prepare(skin_bar, time, &ctx);
        }

        // Skin draw cycle with rich render context (config + timer)
        {
            let mut skin = match self.main_state_data.skin.take() {
                Some(s) => s,
                None => return,
            };
            let mut timer = std::mem::take(&mut self.main_state_data.timer);

            {
                let mut ctx = SelectSkinContext {
                    timer: &mut timer,
                    selector: self,
                };
                skin.update_custom_objects_timed(&mut ctx);
                skin.swap_sprite_batch(sprite);
                skin.draw_all_objects_timed(&mut ctx);
                skin.swap_sprite_batch(sprite);
            }

            self.main_state_data.timer = timer;
            self.main_state_data.skin = Some(skin);
        }

        // Bar render — draw bar images, text, lamps, etc.
        {
            let timer_snapshot = rubato_skin::stubs::Timer::with_timers(
                self.main_state_data.timer.now_time(),
                self.main_state_data.timer.now_micro_time(),
                self.main_state_data.timer.export_timer_array(),
            );
            let adapter = MinimalSkinMainState::new(&timer_snapshot);

            let currentsongs = &self.manager.currentsongs;
            let rival = self.rival.is_some();
            let lnmode = self.config.lnmode;
            let center_bar = self.select_center_bar;

            if let (Some(bar_renderer), Some(skin_bar)) = (&mut self.bar, &mut self.skin_bar) {
                let mut renderer = SkinObjectRenderer::new();
                std::mem::swap(&mut renderer.sprite, sprite);
                let ctx = RenderContext {
                    center_bar,
                    currentsongs,
                    rival,
                    state: &adapter,
                    lnmode,
                    loader_finished: false,
                };
                bar_renderer.render(&mut renderer, skin_bar, &ctx);
                std::mem::swap(&mut renderer.sprite, sprite);
            }
        }
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.main_state_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.main_state_data.timer);

        {
            let mut ctx = SelectSkinContext {
                timer: &mut timer,
                selector: self,
            };
            skin.mouse_pressed_at(&mut ctx, button, x, y);
        }

        self.main_state_data.timer = timer;
        self.main_state_data.skin = Some(skin);
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.main_state_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.main_state_data.timer);

        {
            let mut ctx = SelectSkinContext {
                timer: &mut timer,
                selector: self,
            };
            skin.mouse_dragged_at(&mut ctx, button, x, y);
        }

        self.main_state_data.timer = timer;
        self.main_state_data.skin = Some(skin);
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
            && timer.now_time() > skin.input() as i64
        {
            timer.switch_timer(skin_property::TIMER_STARTINPUT, true);
        }

        // Initialize songbar change timer
        if timer.now_time_for_id(skin_property::TIMER_SONGBAR_CHANGE) < 0 {
            timer.set_timer_on(skin_property::TIMER_SONGBAR_CHANGE);
        }

        let now_time = timer.now_time();
        let songbar_change_time = timer.timer(skin_property::TIMER_SONGBAR_CHANGE);

        // Update resource with current bar's song/course data (Java MusicSelector L218-219)
        {
            let song_data = self
                .manager
                .selected()
                .and_then(|b| b.as_song_bar())
                .map(|sb| sb.song_data().clone());
            let course_data = self
                .manager
                .selected()
                .and_then(|b| b.as_grade_bar())
                .map(|gb| gb.course_data().clone());
            if let Some(res) = self.player_resource.as_mut() {
                PlayerResourceAccess::set_songdata(res, song_data);
                if let Some(cd) = course_data {
                    res.set_course_data(cd);
                } else {
                    res.clear_course_data();
                }
            }
        }

        // Preview music
        if let Some(current) = self.manager.selected() {
            if let Some(song_bar) = current.as_song_bar() {
                // Preview music timing
                if self.play.is_none()
                    && now_time > songbar_change_time + self.preview_duration as i64
                {
                    let should_start_preview = if let Some(ref preview) = self.preview {
                        let preview_song = preview.song_data();
                        // In Java: song != preview.getSongData() (reference comparison)
                        match preview_song {
                            Some(ps) => ps.sha256 != song_bar.song_data().sha256,
                            None => true,
                        }
                    } else {
                        false
                    };
                    if should_start_preview
                        && !matches!(self.app_config.song_preview, SongPreview::NONE)
                    {
                        let song_clone = song_bar.song_data().clone();
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
                        let path = song_bar.song_data().path().map(std::path::PathBuf::from);
                        let lnmode = self.config.lnmode;
                        if let Some(path) = path
                            && let Some((model, _margin)) =
                                rubato_core::player_resource::PlayerResource::load_bms_model(
                                    &path, lnmode,
                                )
                            && let Some(sd) =
                                self.player_resource.as_mut().and_then(|r| r.songdata_mut())
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
            .timer(skin_property::TIMER_SONGBAR_CHANGE);
        let now_time = self.main_state_data.timer.now_time();
        if self.current_ranking_duration != -1
            && now_time > songbar_change_time + self.current_ranking_duration
        {
            self.current_ranking_duration = -1;
            // Load/refresh ranking data from cache
            if let Some(current) = self.manager.selected()
                && let Some(main) = self.main.as_mut()
            {
                use rubato_ir::ranking_data::RankingData;
                let lnmode = main.player_config().lnmode;
                if let Some(song_bar) = current.as_song_bar()
                    && song_bar.exists_song()
                    && self.play.is_none()
                {
                    let song = song_bar.song_data();
                    let cached = main
                        .ranking_data_cache()
                        .and_then(|c| c.song_any(song, lnmode))
                        .and_then(|a| a.downcast::<RankingData>().ok())
                        .map(|ranking| *ranking);
                    if cached.is_none() {
                        let rd = RankingData::new();
                        self.currentir = Some(rd.clone());
                        if let Some(cache) = main.ranking_data_cache_mut() {
                            cache.put_song_any(song, lnmode, Box::new(rd));
                        }
                    } else {
                        self.currentir = cached;
                    }
                    // Java MusicSelector L251: irc.load(this, song)
                    if let Some(ref mut rd) = self.currentir {
                        use rubato_ir::ir_chart_data::IRChartData;
                        use rubato_ir::ir_connection::IRConnection;
                        use std::sync::Arc;
                        if let Some(conn_arc) = main.ir_connection_any().and_then(|any| {
                            any.downcast_ref::<Arc<dyn IRConnection + Send + Sync>>()
                                .cloned()
                        }) {
                            let chart = IRChartData::new(song);
                            let local_score = main.read_score_data_by_hash(
                                &song.sha256,
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
                    let course = grade_bar.course_data();
                    let cached = main
                        .ranking_data_cache()
                        .and_then(|c| c.course_any(course, lnmode))
                        .and_then(|a| a.downcast::<RankingData>().ok())
                        .map(|ranking| *ranking);
                    if cached.is_none() {
                        let rd = RankingData::new();
                        self.currentir = Some(rd.clone());
                        if let Some(cache) = main.ranking_data_cache_mut() {
                            cache.put_course_any(course, lnmode, Box::new(rd));
                        }
                    } else {
                        self.currentir = cached;
                    }
                    // Java MusicSelector L261: irc.load(this, course)
                    if let Some(ref mut rd) = self.currentir {
                        use rubato_ir::ir_connection::IRConnection;
                        use rubato_ir::ir_course_data::IRCourseData;
                        use std::sync::Arc;
                        if let Some(conn_arc) = main.ir_connection_any().and_then(|any| {
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
        let irstate = self.currentir.as_ref().map(|ir| ir.state()).unwrap_or(-1);
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
            let (action, is_function_bar) = if let Some(current) = self.manager.selected() {
                let is_func = current.as_function_bar().is_some();
                if let Some(song_bar) = current.as_song_bar() {
                    if song_bar.exists_song() {
                        (
                            BarAction::SongChart {
                                song: song_bar.song_data().clone(),
                                bar: current.clone(),
                            },
                            is_func,
                        )
                    } else {
                        (
                            BarAction::SongMissing {
                                song: song_bar.song_data().clone(),
                            },
                            is_func,
                        )
                    }
                } else if let Some(exec_bar) = current.as_executable_bar() {
                    (
                        BarAction::ExecutableChart {
                            song: exec_bar.song_data().clone(),
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
                        Bar::Folder(b) => b.children(songdb),
                        Bar::Command(b) => {
                            let player_name =
                                self.app_config.playername.as_deref().unwrap_or("default");
                            let score_path =
                                format!("{}/{}/score.db", self.app_config.playerpath, player_name);
                            let scorelog_path = format!(
                                "{}/{}/scorelog.db",
                                self.app_config.playerpath, player_name
                            );
                            let songinfo_path = self.app_config.songinfopath.to_string();
                            let cmd_ctx = crate::select::bar::command_bar::CommandBarContext {
                                score_db_path: &score_path,
                                scorelog_db_path: &scorelog_path,
                                info_db_path: Some(&songinfo_path),
                            };
                            b.children(songdb, &cmd_ctx)
                        }
                        Bar::Container(b) => b.children().to_vec(),
                        Bar::Hash(b) => b.children(songdb),
                        Bar::Table(b) => b.children().to_vec(),
                        Bar::SearchWord(b) => b.children(songdb),
                        Bar::SameFolder(b) => b.children(songdb),
                        Bar::ContextMenu(b) => b.children(&self.manager.tables, songdb),
                        Bar::LeaderBoard(b) => b.children(),
                        _ => Vec::new(),
                    };
                    let paths: Vec<PathBuf> = children
                        .iter()
                        .filter_map(|bar| {
                            bar.as_song_bar()
                                .filter(|sb| sb.exists_song())
                                .and_then(|sb| sb.song_data().path())
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
                    let ipfs_available = !song.ipfs_str().is_empty()
                        && self
                            .main
                            .as_ref()
                            .is_some_and(|m| m.is_ipfs_download_alive());
                    let http_available = self
                        .main
                        .as_ref()
                        .and_then(|m| m.http_downloader())
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
                    .selected()
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
    use ::bms_model::bms_model::BMSModel;
    use ::bms_model::note::Note;
    use rubato_audio::audio_driver::AudioDriver;
    use rubato_core::main_state::MainState;
    use rubato_types::skin_render_context::SkinRenderContext;

    struct MockAudioDriver {
        play_count: usize,
        stop_count: usize,
    }

    impl MockAudioDriver {
        fn new() -> Self {
            Self {
                play_count: 0,
                stop_count: 0,
            }
        }
    }

    impl AudioDriver for MockAudioDriver {
        fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {
            self.play_count += 1;
        }

        fn set_volume_path(&mut self, _path: &str, _volume: f32) {}

        fn is_playing_path(&self, _path: &str) -> bool {
            false
        }

        fn stop_path(&mut self, _path: &str) {
            self.stop_count += 1;
        }

        fn dispose_path(&mut self, _path: &str) {}

        fn set_model(&mut self, _model: &BMSModel) {}

        fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}

        fn abort(&mut self) {}

        fn get_progress(&self) -> f32 {
            1.0
        }

        fn play_note(&mut self, _n: &Note, _volume: f32, _pitch: i32) {}

        fn play_judge(&mut self, _judge: i32, _fast: bool) {}

        fn stop_note(&mut self, _n: Option<&Note>) {}

        fn set_volume_note(&mut self, _n: &Note, _volume: f32) {}

        fn set_global_pitch(&mut self, _pitch: f32) {}

        fn get_global_pitch(&self) -> f32 {
            1.0
        }

        fn dispose_old(&mut self) {}

        fn dispose(&mut self) {}
    }

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

    fn set_selected_bar(selector: &mut MusicSelector, bar: Bar) {
        selector.manager.currentsongs = vec![bar];
        selector.manager.selectedindex = 0;
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
    fn test_sync_audio_ticks_preview_processor() {
        let config = Config::default();
        let mut selector = MusicSelector::with_config(config.clone());
        let mut preview = PreviewMusicProcessor::new(&config);
        preview.set_default("/bgm/default.ogg");
        preview.start(None);
        selector.preview = Some(preview);

        let mut audio = MockAudioDriver::new();
        selector.sync_audio(&mut audio);

        assert_eq!(audio.play_count, 1);
    }

    #[test]
    fn test_select_skin_context_uses_sort_for_image_index_12() {
        let mut selector = MusicSelector::new();
        selector.config.sort = 5;
        selector.config.judgetiming = 17;
        let mut timer = TimerManager::new();
        let ctx = SelectSkinContext {
            timer: &mut timer,
            selector: &mut selector,
        };

        assert_eq!(ctx.image_index_value(12), 5);
    }

    #[test]
    fn test_select_skin_context_uses_random_for_image_index_42() {
        let mut selector = MusicSelector::new();
        selector.config.random = 4;
        let mut timer = TimerManager::new();
        let ctx = SelectSkinContext {
            timer: &mut timer,
            selector: &mut selector,
        };

        assert_eq!(ctx.image_index_value(42), 4);
    }

    #[test]
    fn test_select_skin_context_uses_mode_image_index_mapping() {
        let mut selector = MusicSelector::new();
        selector.config.mode = Some(bms_model::Mode::BEAT_5K);
        let mut timer = TimerManager::new();
        let ctx = SelectSkinContext {
            timer: &mut timer,
            selector: &mut selector,
        };

        assert_eq!(ctx.image_index_value(11), 1);
    }

    #[test]
    fn test_select_skin_context_uses_selected_song_play_config_for_image_index_330() {
        let mut selector = MusicSelector::new();
        selector.config.mode = Some(bms_model::Mode::BEAT_7K);
        selector.config.mode7.playconfig.enablelanecover = false;
        selector.config.mode5.playconfig.enablelanecover = true;
        let mut song = make_song_data("play-config", Some("/test/selected.bms"));
        song.mode = bms_model::Mode::BEAT_5K.id();
        set_selected_bar(&mut selector, Bar::Song(Box::new(SongBar::new(song))));
        let mut timer = TimerManager::new();
        let ctx = SelectSkinContext {
            timer: &mut timer,
            selector: &mut selector,
        };

        assert_eq!(ctx.image_index_value(330), 1);
    }

    #[test]
    fn test_select_skin_context_uses_selected_song_judge_algorithm_for_image_index_340() {
        let mut selector = MusicSelector::new();
        selector.config.mode = Some(bms_model::Mode::BEAT_7K);
        selector.config.mode7.playconfig.judgetype = "Combo".to_string();
        selector.config.mode5.playconfig.judgetype = "Lowest".to_string();
        let mut song = make_song_data("judge-type", Some("/test/judge.bms"));
        song.mode = bms_model::Mode::BEAT_5K.id();
        set_selected_bar(&mut selector, Bar::Song(Box::new(SongBar::new(song))));
        let mut timer = TimerManager::new();
        let ctx = SelectSkinContext {
            timer: &mut timer,
            selector: &mut selector,
        };

        assert_eq!(ctx.image_index_value(340), 2);
    }

    #[test]
    fn test_select_skin_context_uses_selected_score_clear_for_image_index_370() {
        let mut selector = MusicSelector::new();
        let mut bar = make_song_bar("clear", Some("/test/clear.bms"));
        let mut score = ScoreData::default();
        score.clear = 6;
        bar.set_score(Some(score));
        set_selected_bar(&mut selector, bar);
        let mut timer = TimerManager::new();
        let ctx = SelectSkinContext {
            timer: &mut timer,
            selector: &mut selector,
        };

        assert_eq!(ctx.image_index_value(370), 6);
    }

    #[test]
    fn test_select_skin_context_uses_rival_clear_for_image_index_371() {
        let mut selector = MusicSelector::new();
        let mut bar = make_song_bar("rival-clear", Some("/test/rival-clear.bms"));
        let mut rival = ScoreData::default();
        rival.clear = 8;
        bar.set_rival_score(Some(rival));
        set_selected_bar(&mut selector, bar);
        let mut timer = TimerManager::new();
        let ctx = SelectSkinContext {
            timer: &mut timer,
            selector: &mut selector,
        };

        assert_eq!(ctx.image_index_value(371), 8);
    }

    #[test]
    fn test_select_skin_context_uses_target_visual_index_for_image_index_77() {
        let mut selector = MusicSelector::new();
        selector.config.targetid = "MAX".to_string();
        let mut timer = TimerManager::new();
        let ctx = SelectSkinContext {
            timer: &mut timer,
            selector: &mut selector,
        };

        assert_eq!(ctx.image_index_value(77), 10);
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
        assert_eq!(selector.ranking_position(), 0.0);

        // Set position with no IR data — ranking_max = 1, so 1 * 0.5 = 0
        selector.set_ranking_position(0.5);
        assert_eq!(selector.ranking_offset, 0);
    }

    #[test]
    fn test_ranking_position_with_ir() {
        let mut selector = MusicSelector::new();

        // Use update_score to set total player count
        let mut ir = RankingData::new();
        use rubato_core::score_data::ScoreData as CoreScoreData;
        use rubato_ir::ir_score_data::IRScoreData;
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

        let pos = selector.ranking_position();
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
        let config = rubato_core::config::Config::default();
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

        let config = rubato_core::config::Config::default();
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

    use rubato_types::main_controller_access::MainControllerAccess;
    use rubato_types::player_resource_access::PlayerResourceAccess;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    /// Tracks state changes and resource operations for assertions.
    #[derive(Default)]
    struct MockState {
        state_changes: Vec<MainStateType>,
        played_audio_paths: Vec<String>,
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
        chart_option: Option<Option<rubato_types::replay_data::ReplayData>>,
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
        course_replay: Vec<rubato_types::replay_data::ReplayData>,
    }

    impl PlayerResourceAccess for MockPlayerResource {
        fn into_any_send(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }
        fn config(&self) -> &rubato_types::config::Config {
            static CFG: std::sync::OnceLock<rubato_types::config::Config> =
                std::sync::OnceLock::new();
            CFG.get_or_init(rubato_types::config::Config::default)
        }
        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            static PC: std::sync::OnceLock<rubato_types::player_config::PlayerConfig> =
                std::sync::OnceLock::new();
            PC.get_or_init(rubato_types::player_config::PlayerConfig::default)
        }
        fn score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn rival_score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn target_score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn course_score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn set_course_score_data(&mut self, _score: ScoreData) {}
        fn songdata(&self) -> Option<&SongData> {
            None
        }
        fn songdata_mut(&mut self) -> Option<&mut SongData> {
            None
        }
        fn set_songdata(&mut self, _data: Option<SongData>) {}
        fn replay_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
            None
        }
        fn replay_data_mut(&mut self) -> Option<&mut rubato_types::replay_data::ReplayData> {
            None
        }
        fn course_replay(&self) -> &[rubato_types::replay_data::ReplayData] {
            &[]
        }
        fn add_course_replay(&mut self, _rd: rubato_types::replay_data::ReplayData) {}
        fn course_data(&self) -> Option<&CourseData> {
            None
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
            static EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
            &EMPTY
        }
        fn add_course_gauge(&mut self, _gauge: Vec<Vec<f32>>) {}
        fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
            &mut self.course_gauge
        }
        fn score_data_mut(&mut self) -> Option<&mut ScoreData> {
            None
        }
        fn course_replay_mut(&mut self) -> &mut Vec<rubato_types::replay_data::ReplayData> {
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
        fn reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }
        fn reverse_lookup_levels(&self) -> Vec<String> {
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
        fn set_chart_option_data(&mut self, option: Option<rubato_types::replay_data::ReplayData>) {
            self.state.lock().unwrap().chart_option = Some(option);
        }
        fn set_course_data(&mut self, data: CourseData) {
            self.state.lock().unwrap().course_data = Some(data);
        }
        fn clear_course_data(&mut self) {
            self.state.lock().unwrap().course_data = None;
        }
        fn course_song_data(&self) -> Vec<SongData> {
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
        fn config(&self) -> &rubato_types::config::Config {
            static CFG: std::sync::OnceLock<rubato_types::config::Config> =
                std::sync::OnceLock::new();
            CFG.get_or_init(rubato_types::config::Config::default)
        }
        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            static PC: std::sync::OnceLock<rubato_types::player_config::PlayerConfig> =
                std::sync::OnceLock::new();
            PC.get_or_init(rubato_types::player_config::PlayerConfig::default)
        }
        fn change_state(&mut self, state: MainStateType) {
            self.state.lock().unwrap().state_changes.push(state);
        }
        fn save_config(&self) {}
        fn exit(&self) {}
        fn save_last_recording(&self, _reason: &str) {}
        fn update_song(&mut self, _path: Option<&str>) {}
        fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
            Some(&self.resource)
        }
        fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
            Some(&mut self.resource)
        }
        fn play_audio_path(&mut self, path: &str, _volume: f32, _loop_play: bool) {
            self.state
                .lock()
                .unwrap()
                .played_audio_paths
                .push(path.to_string());
        }
    }

    fn make_selector_with_mock() -> (MusicSelector, Arc<Mutex<MockState>>) {
        let state = Arc::new(Mutex::new(MockState::default()));
        let mock = MockMainController::new(state.clone());
        let mut selector = MusicSelector::new();
        selector.set_main_controller(Box::new(mock));
        (selector, state)
    }

    struct ChangeStateSkin;

    impl rubato_core::main_state::SkinDrawable for ChangeStateSkin {
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
            ctx.change_state(MainStateType::Config);
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
        fn swap_sprite_batch(&mut self, _batch: &mut rubato_render::sprite_batch::SpriteBatch) {}
    }

    // ============================================================
    // read_chart tests
    // ============================================================

    #[test]
    fn test_read_chart_success_clears_resource_and_transitions() {
        let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
        if !bms_path.exists() {
            return;
        }
        let path_str = bms_path.to_string_lossy().to_string();
        let mut selector = MusicSelector::new();
        selector.play = Some(BMSPlayerMode::PLAY);

        let song = make_song_data("abc123", Some(&path_str));
        let bar = make_song_bar("abc123", Some(&path_str));

        selector.read_chart(&song, &bar);

        assert!(
            selector.player_resource.is_some(),
            "player_resource should be created"
        );
        assert_eq!(
            selector.pending_state_change,
            Some(MainStateType::Decide),
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
        let mut selector = MusicSelector::new();
        selector.play = Some(BMSPlayerMode::PLAY);

        let song = make_song_data("abc123", Some("/nonexistent.bms"));
        let bar = make_song_bar("abc123", Some("/nonexistent.bms"));

        selector.read_chart(&song, &bar);

        assert!(
            selector.player_resource.is_some(),
            "player_resource should still be created"
        );
        assert_eq!(
            selector.pending_state_change, None,
            "should NOT transition on failure"
        );
        assert!(
            selector.playedsong.is_none(),
            "playedsong should NOT be set on failure"
        );
    }

    #[test]
    fn test_read_chart_sets_rival_score_and_chart_option() {
        let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
        if !bms_path.exists() {
            return;
        }
        let path_str = bms_path.to_string_lossy().to_string();
        let mut selector = MusicSelector::new();
        selector.play = Some(BMSPlayerMode::PLAY);

        let song = make_song_data("abc123", Some(&path_str));
        let bar = make_song_bar("abc123", Some(&path_str));

        selector.read_chart(&song, &bar);

        // Verify chart was loaded successfully and state transition requested
        assert_eq!(
            selector.pending_state_change,
            Some(MainStateType::Decide),
            "should transition on success"
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
        let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
        if !bms_path.exists() {
            return;
        }
        let path_str = bms_path.to_string_lossy().to_string();

        let mut selector = MusicSelector::new();
        let course = CourseData {
            name: Some("Test Course".to_string()),
            hash: vec![make_song_data("s1", Some(&path_str))],
            constraint: vec![],
            trophy: vec![],
            release: false,
        };
        selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
        selector.manager.selectedindex = 0;

        selector.read_course(BMSPlayerMode::PLAY);

        assert_eq!(
            selector.pending_state_change,
            Some(MainStateType::Decide),
            "should transition to DECIDE"
        );
        assert!(
            selector.playedcourse.is_some(),
            "playedcourse should be set"
        );
    }

    #[test]
    fn test_read_course_missing_songs_does_not_transition() {
        let mut selector = MusicSelector::new();

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

        assert_eq!(
            selector.pending_state_change, None,
            "should NOT transition when songs are missing"
        );
    }

    #[test]
    fn test_read_course_class_constraint_resets_random() {
        let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
        if !bms_path.exists() {
            return;
        }
        let path_str = bms_path.to_string_lossy().to_string();

        let mut selector = MusicSelector::new();
        // Set non-zero random options
        selector.config.random = 3;
        selector.config.random2 = 4;
        selector.config.doubleoption = 2;

        let course = CourseData {
            name: Some("Class Course".to_string()),
            hash: vec![make_song_data("s1", Some(&path_str))],
            constraint: vec![CourseDataConstraint::Class],
            trophy: vec![],
            release: false,
        };
        selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
        selector.manager.selectedindex = 0;

        selector.read_course(BMSPlayerMode::PLAY);

        assert_eq!(selector.config.random, 0, "CLASS should reset random to 0");
        assert_eq!(
            selector.config.random2, 0,
            "CLASS should reset random2 to 0"
        );
        assert_eq!(
            selector.config.doubleoption, 0,
            "CLASS should reset doubleoption to 0"
        );
    }

    // ============================================================
    // _read_course tests
    // ============================================================

    #[test]
    fn test_internal_read_course_ln_constraint() {
        let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
        if !bms_path.exists() {
            return;
        }
        let path_str = bms_path.to_string_lossy().to_string();

        let mut selector = MusicSelector::new();
        selector.config.lnmode = 2;

        let course = CourseData {
            name: Some("LN Course".to_string()),
            hash: vec![make_song_data("s1", Some(&path_str))],
            constraint: vec![CourseDataConstraint::Ln],
            trophy: vec![],
            release: false,
        };
        let bar = Bar::Grade(Box::new(GradeBar::new(course)));

        let result = selector._read_course(&BMSPlayerMode::PLAY, &bar);

        assert!(result, "_read_course should return true on success");
        assert_eq!(
            selector.config.lnmode, 0,
            "LN constraint should set lnmode to 0"
        );
    }

    #[test]
    fn test_internal_read_course_autoplay_applies_constraints() {
        let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
        if !bms_path.exists() {
            return;
        }
        let path_str = bms_path.to_string_lossy().to_string();

        let mut selector = MusicSelector::new();
        selector.config.random = 5;

        let course = CourseData {
            name: Some("Class Course".to_string()),
            hash: vec![make_song_data("s1", Some(&path_str))],
            constraint: vec![CourseDataConstraint::Class],
            trophy: vec![],
            release: false,
        };
        let bar = Bar::Grade(Box::new(GradeBar::new(course)));

        let result = selector._read_course(&BMSPlayerMode::AUTOPLAY, &bar);

        assert!(result);
        // AUTOPLAY applies CLASS constraint (same as PLAY)
        assert_eq!(
            selector.config.random, 0,
            "AUTOPLAY should apply CLASS constraint and reset random"
        );
    }

    #[test]
    fn test_internal_read_course_replay_skips_constraints() {
        let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
        if !bms_path.exists() {
            return;
        }
        let path_str = bms_path.to_string_lossy().to_string();

        let mut selector = MusicSelector::new();
        selector.config.random = 5;

        let course = CourseData {
            name: Some("Class Course".to_string()),
            hash: vec![make_song_data("s1", Some(&path_str))],
            constraint: vec![CourseDataConstraint::Class],
            trophy: vec![],
            release: false,
        };
        let bar = Bar::Grade(Box::new(GradeBar::new(course)));

        let result = selector._read_course(&BMSPlayerMode::REPLAY_1, &bar);

        assert!(result);
        // REPLAY should NOT apply constraints
        assert_eq!(selector.config.random, 5, "REPLAY should not reset random");
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
        // Use a real BMS file so next_song() succeeds
        let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/test.bms");
        if !bms_path.exists() {
            // Skip if test BMS file not available
            return;
        }
        let mut selector = MusicSelector::new();
        selector.read_directory_autoplay(vec![bms_path]);

        assert!(
            selector.player_resource.is_some(),
            "player_resource should be created"
        );
        assert_eq!(
            selector.pending_state_change,
            Some(MainStateType::Decide),
            "should transition to DECIDE"
        );
    }

    #[test]
    fn test_directory_autoplay_no_transition_when_next_song_fails() {
        // Non-existent paths cause next_song() to return false
        let mut selector = MusicSelector::new();
        selector.read_directory_autoplay(vec![PathBuf::from("/nonexistent/song_a.bms")]);

        assert!(
            selector.player_resource.is_some(),
            "player_resource should be created"
        );
        assert_eq!(
            selector.pending_state_change, None,
            "should NOT transition when next_song returns false"
        );
    }

    #[test]
    fn test_directory_autoplay_empty_paths_does_nothing() {
        let mut selector = MusicSelector::new();
        selector.read_directory_autoplay(vec![]);

        assert!(
            selector.player_resource.is_none(),
            "player_resource should NOT be created for empty paths"
        );
        assert_eq!(
            selector.pending_state_change, None,
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
            b.children()
                .iter()
                .filter_map(|bar| {
                    bar.as_song_bar()
                        .filter(|sb| sb.exists_song())
                        .and_then(|sb| sb.song_data().path())
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

    #[test]
    fn test_handle_skin_mouse_pressed_uses_selector_context() {
        let mut selector = MusicSelector::new();
        selector.main_state_data.skin = Some(Box::new(ChangeStateSkin));

        <MusicSelector as MainState>::handle_skin_mouse_pressed(&mut selector, 0, 32, 48);

        assert_eq!(selector.pending_state_change, Some(MainStateType::Config));
    }
}

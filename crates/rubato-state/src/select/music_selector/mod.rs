pub(crate) use std::path::PathBuf;

pub(crate) use rubato_audio::audio_driver::AudioDriver;
pub(crate) use rubato_core::main_state::{MainState, MainStateData};
pub(crate) use rubato_core::pixmap_resource_pool::PixmapResourcePool;
pub(crate) use rubato_core::timer_manager::TimerManager;
pub(crate) use rubato_ir::ranking_data;
pub(crate) use rubato_types::audio_config::DEFAULT_AUDIO_VOLUME;
pub(crate) use rubato_types::main_controller_access::MainControllerAccess;
pub(crate) use rubato_types::player_resource_access::{
    CourseAccess, MediaAccess, SessionMutation, SongAccess,
};

pub(crate) use super::bar::bar::Bar;
pub(crate) use super::bar::grade_bar::GradeBar;
pub(crate) use super::bar_manager::BarManager;
pub(crate) use super::bar_renderer::BarRenderer;
pub(crate) use super::bar_renderer::{PrepareContext, RenderContext};
pub(crate) use super::bar_sorter::BarSorter;
pub(crate) use super::music_select_command::MusicSelectCommand;
pub(crate) use super::music_select_input_processor::{
    BarType, InputContext, InputEvent, MusicSelectInputProcessor,
};
pub(crate) use super::music_select_key_property::MusicSelectKeyProperty;
pub(crate) use super::preview_music_processor::PreviewMusicProcessor;
pub(crate) use super::score_data_cache::ScoreDataCache;
pub(crate) use super::search_text_field::SearchTextField;
pub(crate) use super::*;

/// Receiver for a background BMS model parse: (requested path, parsed model + margin).
type PendingNoteGraphRx = (
    std::path::PathBuf,
    std::sync::mpsc::Receiver<Option<(::bms_model::bms_model::BMSModel, i64)>>,
);

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
    match song.chart.mode {
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
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for_id(timer_id)
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl SelectSkinContext<'_> {
    fn selected_bar(&self) -> Option<&Bar> {
        self.selector.manager.selected()
    }

    fn selected_directory_data(
        &self,
    ) -> Option<&crate::select::bar::directory_bar::DirectoryBarData> {
        self.selected_bar()?.as_directory_bar()
    }

    fn selected_song_data(&self) -> Option<&rubato_types::song_data::SongData> {
        self.selected_bar()?.as_song_bar().map(|sb| sb.song_data())
    }

    fn selected_score(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.selected_bar()?.score()
    }

    fn selected_score_property(
        &self,
    ) -> Option<rubato_core::score_data_property::ScoreDataProperty> {
        let score = self.selected_score()?;
        let mut property = rubato_core::score_data_property::ScoreDataProperty::new();
        property.update_score(Some(score));
        Some(property)
    }

    fn selected_rival_score(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.selected_bar()?.rival_score()
    }

    fn player_data(&self) -> Option<&rubato_types::player_data::PlayerData> {
        self.selector
            .player_resource
            .as_ref()
            .map(rubato_core::player_resource::PlayerResource::player_data)
    }

    fn selected_replay_exists(&self, slot: i32) -> bool {
        self.selected_bar()
            .and_then(|b| b.as_selectable_bar())
            .is_some_and(|sb| sb.exists_replay(slot))
    }

    fn search_word(&self) -> String {
        self.selector
            .search
            .as_ref()
            .map_or_else(String::new, |search| {
                if search.text.is_empty() {
                    search.message_text.clone()
                } else {
                    search.text.clone()
                }
            })
    }

    fn course_title_at(&self, index: usize) -> String {
        if let Some(course_bar) = self.selected_bar().and_then(|bar| bar.as_grade_bar()) {
            return course_bar
                .song_datas()
                .get(index)
                .map_or_else(String::new, |song| {
                    let title = song.metadata.title.clone();
                    if song.file.path().is_some() {
                        title
                    } else {
                        format!("(no song) {title}")
                    }
                });
        }

        self.selected_bar()
            .and_then(|bar| bar.as_random_course_bar())
            .and_then(|bar| bar.course_data().stage().get(index))
            .map(|stage| stage.title.clone().unwrap_or_else(|| "----".to_string()))
            .unwrap_or_default()
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

    fn notify_audio_config_changed(&mut self) {
        if let Some(audio) = self.selector.app_config.audio.clone()
            && let Some(ref main) = self.selector.main
        {
            main.update_audio_config(audio);
        }
    }

    fn selected_play_config_mut(&mut self) -> Option<&mut rubato_types::play_config::PlayConfig> {
        self.selector.get_selected_play_config_mut()
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        self.selector.get_selected_play_config_ref()
    }

    fn target_score_data(&self) -> Option<&rubato_types::score_data::ScoreData> {
        let targetid = &self.selector.config.select_settings.targetid;
        if targetid.starts_with("RIVAL_RANK_") || targetid.starts_with("RIVAL_NEXT_") {
            // RIVAL_RANK_* and RIVAL_NEXT_* require ranked rival data which is
            // computed during gameplay; on the select screen use the cached score.
            self.selector.cached_target_score.as_ref()
        } else if targetid.starts_with("RIVAL_") {
            // Direct rival targets (RIVAL_1, RIVAL_2, ...) use the rival score
            self.selected_rival_score()
        } else if targetid == "MYBEST" {
            // MYBEST target uses the player's own best score
            self.selected_score()
        } else {
            // Static rate targets, IR-based targets, and RANK_NEXT all use
            // the pre-computed cached target score
            self.selector.cached_target_score.as_ref()
        }
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
        // LR2 skin image order: 0=all, 1=5k, 2=7k, 3=10k, 4=14k, 5=9k, 6=24k, 7=24kDP
        // Maps MODE[i] -> LR2 image index for all 8 MODE entries.
        let lr2_mode_indices = [0, 2, 4, 5, 1, 3, 6, 7];
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

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
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

    fn ranking_score_clear_type(&self, slot: i32) -> i32 {
        if let Some(ref ranking) = self.selector.ranking.currentir {
            let index = self.selector.ranking.ranking_offset + slot;
            ranking
                .score(index)
                .map(|score| score.clear.id())
                .unwrap_or(-1)
        } else {
            -1
        }
    }

    fn ranking_offset(&self) -> i32 {
        self.selector.ranking.ranking_offset
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            // Player totals
            30 => self.player_data().map_or(0, |data| data.playcount as i32),
            333 => self.player_data().map_or(0, |data| {
                let total: i64 = (0..=3).map(|judge| data.judge_count(judge)).sum();
                total.min(i32::MAX as i64) as i32
            }),
            // Volume (0-100 scale)
            57 => {
                (self
                    .selector
                    .app_config
                    .audio_config()
                    .map_or(DEFAULT_AUDIO_VOLUME, |a| a.systemvolume)
                    * 100.0) as i32
            }
            58 => {
                (self
                    .selector
                    .app_config
                    .audio_config()
                    .map_or(DEFAULT_AUDIO_VOLUME, |a| a.keyvolume)
                    * 100.0) as i32
            }
            59 => {
                (self
                    .selector
                    .app_config
                    .audio_config()
                    .map_or(DEFAULT_AUDIO_VOLUME, |a| a.bgvolume)
                    * 100.0) as i32
            }
            // Display timing
            12 => self.selector.config.judge_settings.judgetiming,
            // Song BPM
            90 => self
                .selected_song_data()
                .map_or(i32::MIN, |s| s.chart.maxbpm),
            91 => self
                .selected_song_data()
                .map_or(i32::MIN, |s| s.chart.minbpm),
            92 => {
                // mainbpm: prefer SongInformation.mainbpm when available.
                // Java returns Integer.MIN_VALUE when SongInformation is absent,
                // signaling "no data" so skin renderers hide the value.
                self.selected_song_data().map_or(i32::MIN, |s| {
                    s.info
                        .as_ref()
                        .map(|i| i.mainbpm as i32)
                        .unwrap_or(i32::MIN)
                })
            }
            96 => self
                .selected_song_data()
                .map_or(i32::MIN, |s| s.chart.level),
            300 => self
                .selected_directory_data()
                .map(|directory| {
                    directory
                        .lamps
                        .iter()
                        .fold(0i32, |acc, &x| acc.saturating_add(x))
                })
                .unwrap_or(i32::MIN),
            // Song score-related stats
            71 => self.selected_score().map_or(i32::MIN, |s| s.exscore()),
            75 => self.selected_score().map_or(i32::MIN, |s| s.maxcombo),
            76 => self.selected_score().map_or(i32::MIN, |s| s.minbp),
            // Song play/clear/fail counts
            77 => self.selected_score().map_or(i32::MIN, |s| s.playcount),
            78 => self.selected_score().map_or(i32::MIN, |s| s.clearcount),
            79 => {
                let score = self.selected_score();
                score.map_or(i32::MIN, |s| s.playcount - s.clearcount)
            }
            102 => self
                .selected_score_property()
                .map_or(i32::MIN, |property| property.now_rate_int()),
            103 => self
                .selected_score_property()
                .map_or(i32::MIN, |property| property.now_rate_after_dot()),
            // Song duration
            312 => self.selected_song_data().map_or(0, |s| s.chart.length),
            1163 => self
                .selected_song_data()
                .map_or(0, |s| s.chart.length.max(0) / 60000),
            1164 => self
                .selected_song_data()
                .map_or(0, |s| (s.chart.length.max(0) % 60000) / 1000),
            // Total notes
            350 => self.selected_song_data().map_or(0, |s| s.chart.notes),
            // Playtime (hours/minutes/seconds from boot)
            17 => (self.timer.now_time() / 3_600_000) as i32,
            18 => ((self.timer.now_time() % 3_600_000) / 60_000) as i32,
            19 => ((self.timer.now_time() % 60_000) / 1_000) as i32,
            // IDs 20-26 (FPS, system date/time) handled by default_integer_value
            _ => self.default_integer_value(id),
        }
    }

    fn string_value(&self, id: i32) -> String {
        match id {
            // Song metadata
            10 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.metadata.title.clone()),
            11 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.metadata.subtitle.clone()),
            12 => self.selected_song_data().map_or_else(String::new, |s| {
                if s.metadata.subtitle.is_empty() {
                    s.metadata.title.clone()
                } else {
                    format!("{} {}", s.metadata.title, s.metadata.subtitle)
                }
            }),
            13 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.metadata.genre.clone()),
            14 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.metadata.artist.clone()),
            15 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.metadata.subartist.clone()),
            16 => self.selected_song_data().map_or_else(String::new, |s| {
                if s.metadata.subartist.is_empty() {
                    s.metadata.artist.clone()
                } else {
                    format!("{} {}", s.metadata.artist, s.metadata.subartist)
                }
            }),
            30 => self.search_word(),
            150..=159 => self.course_title_at((id - 150) as usize),
            // Directory
            1000 => self.selector.manager.directory_string().to_string(),
            // Version
            1010 => rubato_core::version::Version::get_version().to_string(),
            // Song hash (MD5)
            1030 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.file.md5.clone()),
            // Song hash (SHA256)
            1031 => self
                .selected_song_data()
                .map_or_else(String::new, |s| s.file.sha256.clone()),
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
            // Replay data availability per slot
            OPTION_REPLAYDATA => self.selected_replay_exists(0),
            OPTION_REPLAYDATA2 => self.selected_replay_exists(1),
            OPTION_REPLAYDATA3 => self.selected_replay_exists(2),
            OPTION_REPLAYDATA4 => self.selected_replay_exists(3),
            OPTION_NO_REPLAYDATA => !self.selected_replay_exists(0),
            OPTION_NO_REPLAYDATA2 => !self.selected_replay_exists(1),
            OPTION_NO_REPLAYDATA3 => !self.selected_replay_exists(2),
            OPTION_NO_REPLAYDATA4 => !self.selected_replay_exists(3),
            // Autoplay
            33 => false, // OPTION_AUTOPLAYON - not in select screen
            32 => true,  // OPTION_AUTOPLAYOFF
            // Panels (always visible on select)
            21 => true, // OPTION_PANEL1
            _ => self.default_boolean_value(id),
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
                .map_or(DEFAULT_AUDIO_VOLUME, |a| a.systemvolume),
            18 => self
                .selector
                .app_config
                .audio_config()
                .map_or(DEFAULT_AUDIO_VOLUME, |a| a.keyvolume),
            19 => self
                .selector
                .app_config
                .audio_config()
                .map_or(DEFAULT_AUDIO_VOLUME, |a| a.bgvolume),
            8 => self.selector.ranking_position(),
            // Level (0.0-1.0 normalized)
            103 => self
                .selected_song_data()
                .map_or(0.0, |s| s.chart.level as f32 / 12.0),
            // Hi-speed (from selected bar's play config)
            310 => self
                .selector
                .get_selected_play_config_ref()
                .map_or(0.0, |pc| pc.hispeed),
            _ => self.default_float_value(id),
        }
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        match id {
            1 => self.selector.manager.set_selected_position(value),
            8 => self.selector.set_ranking_position(value),
            17..=19 => {
                if let Some(audio) = self.selector.app_config.audio.as_mut() {
                    let clamped = value.clamp(0.0, 1.0);
                    match id {
                        17 => audio.systemvolume = clamped,
                        18 => audio.keyvolume = clamped,
                        19 => audio.bgvolume = clamped,
                        _ => unreachable!(),
                    }
                }
                // Propagate audio config change to MainController so it survives
                // state transitions and config saves.
                if let Some(audio) = self.selector.app_config.audio.clone()
                    && let Some(ref main) = self.selector.main
                {
                    main.update_audio_config(audio);
                }
            }
            _ => {}
        }
    }

    fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
        &self.selector.cached_score_data_property
    }

    fn get_offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.selector.main_state_data.offsets.get(&id)
    }

    fn get_distribution_data(&self) -> Option<rubato_types::distribution_data::DistributionData> {
        let dir = self.selected_directory_data()?;
        Some(rubato_types::distribution_data::DistributionData {
            lamps: *dir.lamps(),
            ranks: *dir.ranks(),
        })
    }
}

impl rubato_skin::reexports::MainState for SelectSkinContext<'_> {}

/// Minimal adapter implementing rubato_skin::reexports::MainState for BarRenderer's RenderContext.
/// Delegates timer() to a Timer snapshot; other methods use defaults.
struct MinimalSkinMainState<'a> {
    timer: &'a rubato_skin::reexports::Timer,
}

impl<'a> MinimalSkinMainState<'a> {
    fn new(timer: &'a rubato_skin::reexports::Timer) -> Self {
        Self { timer }
    }
}

impl rubato_types::timer_access::TimerAccess for MinimalSkinMainState<'_> {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }
    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for(timer_id)
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for MinimalSkinMainState<'_> {}

impl rubato_skin::reexports::MainState for MinimalSkinMainState<'_> {}

/// Preview music and note graph state.
pub struct PreviewState {
    pub preview: Option<PreviewMusicProcessor>,
    pub notes_graph_duration: i32,
    pub preview_duration: i32,
    pub show_note_graph: bool,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            preview: None,
            notes_graph_duration: 350,
            preview_duration: 400,
            show_note_graph: false,
        }
    }
}

/// IR ranking data and display state.
pub struct RankingState {
    pub scorecache: Option<ScoreDataCache>,
    pub rivalcache: Option<ScoreDataCache>,
    pub currentir: Option<RankingData>,
    pub ranking_offset: i32,
    pub ranking_duration: i32,
    pub ranking_reload_duration: i64,
    pub current_ranking_duration: i64,
}

impl Default for RankingState {
    fn default() -> Self {
        Self {
            scorecache: None,
            rivalcache: None,
            currentir: None,
            ranking_offset: 0,
            ranking_duration: 5000,
            ranking_reload_duration: 10 * 60 * 1000,
            current_ranking_duration: -1,
        }
    }
}

/// Bar renderer and skin bar state.
#[derive(Default)]
pub struct BarRenderingState {
    pub bar: Option<BarRenderer>,
    pub skin_bar: Option<super::skin_bar::SkinBar>,
    pub select_center_bar: i32,
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

    /// Preview music and note graph state.
    pub preview_state: PreviewState,

    /// Bar renderer and skin bar state.
    pub bar_rendering: BarRenderingState,

    /// Bar manager
    pub manager: BarManager,

    /// Music select input processor
    pub musicinput: Option<MusicSelectInputProcessor>,

    /// Search text field
    pub search: Option<SearchTextField>,

    /// IR ranking data and display state.
    pub ranking: RankingState,

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

    /// Cached target score for skin property display on the select screen.
    /// Recomputed each frame based on config.select_settings.targetid and selected song notes.
    cached_target_score: Option<rubato_types::score_data::ScoreData>,

    /// Cached ScoreDataProperty for skin property delegation on the select screen.
    /// Updated before each render from the currently selected bar's score data.
    /// Java: MusicSelector inherits MainState.getScoreDataProperty() which Lua skins
    /// call for main_state.rate()/exscore() etc.
    cached_score_data_property: rubato_types::score_data_property::ScoreDataProperty,

    /// Pending IR ranking fetch result (song).
    /// Stores (requested SongData, lnmode, receiver) so the result is cached under the correct key.
    pending_ir_song_fetch: Option<(SongData, i32, std::sync::mpsc::Receiver<RankingData>)>,
    /// Pending IR ranking fetch result (course).
    /// Stores (requested CourseData, lnmode, receiver) so the result is cached under the correct key.
    pending_ir_course_fetch: Option<(CourseData, i32, std::sync::mpsc::Receiver<RankingData>)>,
    /// Pending BMS model parse result.
    /// Stores (requested path, receiver) so the result is applied to the correct song.
    pending_note_graph: Option<PendingNoteGraphRx>,
    /// JoinHandles for background threads (BMS parse, IR song/course fetch).
    /// Joined on dispose() to ensure clean shutdown.
    background_threads: Vec<std::thread::JoinHandle<()>>,
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

mod bar_operations;
mod commands;
mod song_selection;
mod trait_impls;

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests;

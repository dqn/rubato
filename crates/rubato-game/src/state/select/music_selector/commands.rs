use super::*;

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
            preview_state: PreviewState::default(),
            bar_rendering: BarRenderingState::default(),
            manager: BarManager::new(),
            musicinput: None,
            search: None,
            search_text_region: None,
            ranking: RankingState::default(),
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
            pending_sounds: Vec::new(),
            pending_sound_stops: Vec::new(),
            pending_audio_path_plays: Vec::new(),
            pending_audio_path_stops: Vec::new(),
            pending_audio_config: None,
            pending_player_config_dirty: false,
            player_resource: None,
            cached_target_score: None,
            cached_score_data_property: rubato_types::score_data_property::ScoreDataProperty::new(),
            pending_ir_song_fetch: None,
            pending_ir_course_fetch: None,
            pending_note_graph: None,
            background_threads: Vec::new(),
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

    pub(super) fn ensure_local_score_cache(&mut self) {
        if self.ranking.scorecache.is_some() {
            return;
        }

        let accessor_config = self
            .main
            .as_ref()
            .map(|main| main.config().clone())
            .unwrap_or_else(|| self.app_config.clone());
        let accessor = std::sync::Arc::new(std::sync::Mutex::new(
            crate::core::play_data_accessor::PlayDataAccessor::new(&accessor_config),
        ));
        let single_accessor = std::sync::Arc::clone(&accessor);
        let multi_accessor = std::sync::Arc::clone(&accessor);

        self.ranking.scorecache = Some(ScoreDataCache::new(
            Box::new(move |song, lnmode| {
                let has_ln = song.chart.has_undefined_long_note();
                let accessor = match single_accessor.lock() {
                    Ok(accessor) => accessor,
                    Err(poisoned) => poisoned.into_inner(),
                };
                accessor.read_score_data_by_hash(&song.file.sha256, has_ln, lnmode)
            }),
            Box::new(move |collector, songs, lnmode| {
                let accessor = match multi_accessor.lock() {
                    Ok(accessor) => accessor,
                    Err(poisoned) => poisoned.into_inner(),
                };
                for song in songs {
                    let has_ln = song.chart.has_undefined_long_note();
                    let score = accessor.read_score_data_by_hash(&song.file.sha256, has_ln, lnmode);
                    collector(song, score.as_ref());
                }
            }),
        ));
    }

    pub(super) fn load_bar_contents(&mut self) {
        self.ensure_local_score_cache();

        let main = self.main.as_deref();
        let exists_replay = |sha256: &str, has_ln: bool, lnmode: i32, index: i32| {
            main.is_some_and(|main| main.exists_replay_data(sha256, has_ln, lnmode, index))
        };
        let read_score_by_hash = |hash: &str, has_ln: bool, lnmode: i32| {
            main.and_then(|main| main.read_score_data_by_hash(hash, has_ln, lnmode))
        };

        let mut ctx = crate::state::select::bar_manager::LoaderContext {
            player_config: &self.config,
            score_cache: self.ranking.scorecache.as_mut(),
            rival_cache: self.ranking.rivalcache.as_mut(),
            rival_name: self.rival.as_ref().map(|r| r.name().to_string()),
            is_folderlamp: false,
            banner_resource: Some(&self.banners),
            stagefile_resource: Some(&self.stagefiles),
            exists_replay_fn: main
                .map(|_| &exists_replay as crate::state::select::bar_manager::ExistsReplayFn<'_>),
            read_score_by_hash_fn: main.map(|_| {
                &read_score_by_hash as crate::state::select::bar_manager::ReadScoreByHashFn<'_>
            }),
            songdb: Some(&*self.songdb),
            song_info_db: main.and_then(|main| main.info_database()),
            command_bar_ctx: None,
        };
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        crate::state::select::bar_manager::BarContentsLoaderThread::new(stop)
            .run(&mut self.manager.currentsongs, &mut ctx);
    }

    /// Refresh the bar list with song database context.
    /// Wraps BarManager::update_bar_refresh_with_context to supply the context
    /// from MusicSelector fields, ensuring songdb queries are not skipped.
    pub(super) fn refresh_bar_with_context(&mut self) {
        self.ensure_local_score_cache();
        let mut ctx = BarManager::make_context(
            &self.app_config,
            &mut self.config,
            &*self.songdb,
            self.ranking.scorecache.as_mut(),
        );
        if self.manager.update_bar_refresh_with_context(Some(&mut ctx)) {
            self.load_bar_contents();
            if let Some(bar) = self.bar_rendering.bar.as_mut() {
                bar.update_bar_text();
            }
        }
    }

    /// Navigate into a bar (directory, folder, etc.) with song database context.
    /// Used by MusicSelectCommand and ContextMenuBar executors.
    pub fn update_bar_with_songdb_context(&mut self, bar: Option<&Bar>) -> bool {
        self.ensure_local_score_cache();
        let mut ctx = BarManager::make_context(
            &self.app_config,
            &mut self.config,
            &*self.songdb,
            self.ranking.scorecache.as_mut(),
        );
        let updated = self.manager.update_bar_with_context(bar, Some(&mut ctx));
        if updated {
            self.load_bar_contents();
            if let Some(bar) = self.bar_rendering.bar.as_mut() {
                bar.update_bar_text();
            }
        }
        updated
    }

    pub fn set_rival(&mut self, rival: Option<PlayerInformation>) {
        // In Java: finds rival index, sets rival and rival cache, updates bar
        self.rival = rival;
        self.ranking.rivalcache = None;
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
        self.ranking.scorecache.as_ref()
    }

    pub fn rival_score_data_cache(&self) -> Option<&ScoreDataCache> {
        self.ranking.rivalcache.as_ref()
    }

    pub fn selected_replay(&self) -> i32 {
        self.selectedreplay
    }

    pub fn execute(&mut self, command: MusicSelectCommand) {
        // In Java: command.function.accept(this)
        command.execute(self);
    }

    /// Submit the current search text to create a SearchWordBar and navigate into it.
    /// Translates the Java SearchTextField keyTyped Enter logic:
    ///   SearchWordBar swb = new SearchWordBar(selector, textField.getText());
    ///   selector.getBarManager().addSearch(swb);
    ///   selector.getBarManager().updateBar(null);
    ///   selector.getBarManager().setSelected(swb);
    pub fn submit_search(&mut self) {
        let text = match self.search {
            Some(ref search) if !search.text.is_empty() => search.text.clone(),
            _ => return,
        };

        let swb = super::bar::search_word_bar::SearchWordBar::from_text(text);
        let count = swb.children(&*self.songdb).len();

        if count > 0 {
            let max_count = self.app_config.select.max_search_bar_count;
            let search_bar = Bar::SearchWord(Box::new(swb));
            self.manager.add_search(
                search_bar
                    .as_search_word_bar()
                    .expect("just created")
                    .clone(),
                max_count,
            );
            self.update_bar_with_songdb_context(None);
            self.manager.set_selected(&search_bar);
            if let Some(ref mut search) = self.search {
                search.text.clear();
                search.message_text = format!("{count} song(s) found");
            }
        } else if let Some(ref mut search) = self.search {
            search.text.clear();
            search.message_text = "no song found".to_string();
        }
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
    pub fn execute_event_with_args(&mut self, event: EventType, arg1: i32, arg2: i32) {
        match event {
            EventType::Mode => {
                let current_mode = self.config.mode().copied();
                let mut idx = 0;
                for (i, m) in MODE.iter().enumerate() {
                    if *m == current_mode {
                        idx = i;
                        break;
                    }
                }
                let step = if arg1 >= 0 { 1 } else { MODE.len() - 1 };
                self.config.mode = MODE[(idx + step) % MODE.len()];
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
                self.config.play_settings.lnmode = (self.config.play_settings.lnmode + step) % 3;
                self.play_option_change();
            }
            EventType::Option1p => {
                let step = if arg1 >= 0 { 1 } else { 9 };
                self.config.play_settings.random = (self.config.play_settings.random + step) % 10;
                self.play_option_change();
            }
            EventType::Option2p => {
                let step = if arg1 >= 0 { 1 } else { 9 };
                self.config.play_settings.random2 = (self.config.play_settings.random2 + step) % 10;
                self.play_option_change();
            }
            EventType::Optiondp => {
                let step = if arg1 >= 0 { 1 } else { 3 };
                self.config.play_settings.doubleoption =
                    (self.config.play_settings.doubleoption + step) % 4;
                self.play_option_change();
            }
            EventType::Gauge1p => {
                let step = if arg1 >= 0 { 1 } else { 5 };
                self.config.play_settings.gauge = (self.config.play_settings.gauge + step) % 6;
                self.play_option_change();
            }
            EventType::GaugeAutoShift => {
                let step = if arg1 >= 0 { 1 } else { 4 };
                self.config.play_settings.gauge_auto_shift =
                    (self.config.play_settings.gauge_auto_shift + step) % 5;
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
                    let delta = if arg2 != 0 { arg2 } else { 1 };
                    let step = if arg1 >= 0 { delta } else { -delta };
                    let new_val = (pc.duration + step).clamp(1, 5000);
                    pc.duration = new_val;
                }
                self.play_option_change();
            }
            EventType::Bga => {
                let step = if arg1 >= 0 { 1 } else { 2 };
                self.app_config.render.bga =
                    BgaMode::from((self.app_config.render.bga as i32 + step) % 3);
                self.play_option_change();
            }
            EventType::NotesDisplayTiming => {
                let step = if arg1 >= 0 { 1 } else { -1 };
                self.config.judge_settings.judgetiming =
                    (self.config.judge_settings.judgetiming + step).clamp(-500, 500);
                self.play_option_change();
            }
            EventType::NotesDisplayTimingAutoAdjust => {
                self.config.judge_settings.notes_display_timing_auto_adjust =
                    !self.config.judge_settings.notes_display_timing_auto_adjust;
                self.play_option_change();
            }
            EventType::Target => {
                let mut targets = crate::play::target_property::TargetProperty::targets();
                if targets.is_empty() {
                    // Fall back to config targetlist when global cache is not yet initialized
                    targets = self.config.select_settings.targetlist.clone();
                }
                if !targets.is_empty() {
                    let mut index = None;
                    for (i, t) in targets.iter().enumerate() {
                        if t == &self.config.select_settings.targetid {
                            index = Some(i);
                            break;
                        }
                    }
                    let new_index = match index {
                        Some(i) => {
                            let step = if arg1 >= 0 { 1 } else { targets.len() - 1 };
                            (i + step) % targets.len()
                        }
                        None => 0,
                    };
                    self.config.select_settings.targetid = targets[new_index].clone();
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
                    if let Err(e) = self.songdb.set_song_datas(&[sd]) {
                        log::error!("Failed to set song data: {e}");
                    }
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
                    if let Err(e) = self.songdb.set_song_datas(&[sd]) {
                        log::error!("Failed to set song data: {e}");
                    }
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
                        && let Some(path) = songbar.song_data().file.path()
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
                    && let Some(path) = songbar.song_data().file.path()
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
                    && let Some(path) = songbar.song_data().file.path()
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
}

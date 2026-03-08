use super::*;

impl MusicSelector {
    /// Select a bar (open directory or set play mode).
    /// Corresponds to Java MusicSelector.select(Bar)
    pub fn select(&mut self, current: &Bar) {
        if current.is_directory_bar() {
            let mut ctx = BarManager::make_context(
                &self.app_config,
                &mut self.config,
                &*self.songdb,
                self.ranking.scorecache.as_mut(),
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
        if bar_renderer_reset_input && let Some(ref mut bar) = self.bar_rendering.bar {
            bar.reset_input();
        }
        if bar_renderer_do_input {
            // Take bar out of self to avoid overlapping borrows with self.manager and input
            if let Some(mut bar) = self.bar_rendering.bar.take() {
                let property_idx = self.config.select_settings.musicselectinput as usize;
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
                self.bar_rendering.bar = Some(bar);
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
    pub(super) fn dispatch_input_events(&mut self, events: Vec<InputEvent>) {
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
                        self.ranking.scorecache.as_mut(),
                    );
                    self.manager.close_with_context(Some(&mut ctx));
                }
                InputEvent::OpenDirectory => {
                    // In Java: select.getBarManager().updateBar(dirbar)
                    let mut ctx = BarManager::make_context(
                        &self.app_config,
                        &mut self.config,
                        &*self.songdb,
                        self.ranking.scorecache.as_mut(),
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
        self.ranking.currentir.as_ref()
    }

    pub fn current_ranking_duration(&self) -> i64 {
        self.ranking.current_ranking_duration
    }

    pub fn ranking_offset(&self) -> i32 {
        self.ranking.ranking_offset
    }

    pub fn ranking_position(&self) -> f32 {
        let ranking_max = self
            .ranking
            .currentir
            .as_ref()
            .map(|ir: &RankingData| ir.total_player().max(1))
            .unwrap_or(1);
        self.ranking.ranking_offset as f32 / ranking_max as f32
    }

    pub fn set_ranking_position(&mut self, value: f32) {
        if (0.0..1.0).contains(&value) {
            let ranking_max = self
                .ranking
                .currentir
                .as_ref()
                .map(|ir: &RankingData| ir.total_player().max(1))
                .unwrap_or(1);
            self.ranking.ranking_offset = (ranking_max as f32 * value) as i32;
        }
    }

    /// Read course (grade bar) for play.
    /// Corresponds to Java MusicSelector.readCourse(BMSPlayerMode)
    pub(super) fn read_course(&mut self, mode: BMSPlayerMode) {
        // Get selected bar and check it's a GradeBar
        let grade_bar = match self.manager.selected() {
            Some(bar) if bar.as_grade_bar().is_some() => bar.clone(),
            _ => {
                log::warn!("read_course: selected bar is not a GradeBar");
                return;
            }
        };

        let gb = grade_bar.as_grade_bar().expect("as_grade_bar");
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
    pub(super) fn read_random_course(&mut self, mode: BMSPlayerMode) {
        // Get selected bar and check it's a RandomCourseBar
        let rc_bar = match self.manager.selected() {
            Some(bar) if bar.as_random_course_bar().is_some() => bar.clone(),
            _ => {
                log::warn!("read_random_course: selected bar is not a RandomCourseBar");
                return;
            }
        };

        let rcb = rc_bar.as_random_course_bar().expect("as_random_course_bar");
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
            let score_path = format!(
                "{}/{}/score.db",
                self.app_config.paths.playerpath, player_name
            );
            let scorelog_path = format!(
                "{}/{}/scorelog.db",
                self.app_config.paths.playerpath, player_name
            );
            let songinfo_path = self.app_config.paths.songinfopath.to_string();
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
                        self.ranking.scorecache.as_mut(),
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
    pub(super) fn read_directory_autoplay(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        if self.player_resource.is_none() {
            self.player_resource = Some(rubato_core::player_resource::PlayerResource::new(
                self.app_config.clone(),
                self.config.clone(),
            ));
        }
        let res = self
            .player_resource
            .as_mut()
            .expect("player_resource is Some");
        res.clear();
        res.set_auto_play_songs(paths, false);
        if res.next_song() {
            self.pending_state_change = Some(MainStateType::Decide);
        }
    }

    /// Internal course reading implementation.
    /// Corresponds to Java MusicSelector._readCourse(BMSPlayerMode, GradeBar)
    pub(super) fn _read_course(&mut self, mode: &BMSPlayerMode, grade_bar: &Bar) -> bool {
        // Get song paths from grade bar
        let gb = match grade_bar.as_grade_bar() {
            Some(gb) => gb,
            None => return false,
        };

        let songs = gb.song_datas();
        let files: Vec<PathBuf> = songs
            .iter()
            .filter_map(|s| s.file.path().map(PathBuf::from))
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
        let res = self
            .player_resource
            .as_mut()
            .expect("player_resource is Some");
        res.clear();

        // resource.setCourseBMSFiles(files)
        let load_success = res.set_course_bms_files(&files);

        if load_success {
            // Apply constraints for PLAY/AUTOPLAY modes only
            if mode.mode == BMSPlayerModeType::Play || mode.mode == BMSPlayerModeType::Autoplay {
                for constraint in &gb.course_data().constraint {
                    match constraint {
                        CourseDataConstraint::Class => {
                            self.config.play_settings.random = 0;
                            self.config.play_settings.random2 = 0;
                            self.config.play_settings.doubleoption = 0;
                        }
                        CourseDataConstraint::Mirror => {
                            if self.config.play_settings.random == 1 {
                                self.config.play_settings.random2 = 1;
                                self.config.play_settings.doubleoption = 1;
                            } else {
                                self.config.play_settings.random = 0;
                                self.config.play_settings.random2 = 0;
                                self.config.play_settings.doubleoption = 0;
                            }
                        }
                        CourseDataConstraint::Random => {
                            if self.config.play_settings.random > 5 {
                                self.config.play_settings.random = 0;
                            }
                            if self.config.play_settings.random2 > 5 {
                                self.config.play_settings.random2 = 0;
                            }
                        }
                        CourseDataConstraint::Ln => {
                            self.config.play_settings.lnmode = 0;
                        }
                        CourseDataConstraint::Cn => {
                            self.config.play_settings.lnmode = 1;
                        }
                        CourseDataConstraint::Hcn => {
                            self.config.play_settings.lnmode = 2;
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
                let res = self
                    .player_resource
                    .as_mut()
                    .expect("player_resource is Some");
                res.set_course_data(course_data.clone());
                if !files.is_empty() {
                    PlayerResourceAccess::set_bms_file(res, &files[0], mode_type, mode_id);
                }
            }

            self.playedcourse = Some(course_data);

            // Load/create cached IR ranking data for course
            if let Some(ref mut main) = self.main {
                use rubato_ir::ranking_data::RankingData;
                let lnmode = main.player_config().play_settings.lnmode;
                let course = gb.course_data();
                let cached = main
                    .ranking_data_cache()
                    .and_then(|c| c.course_any(course, lnmode))
                    .and_then(|a| a.downcast::<RankingData>().ok())
                    .map(|ranking| *ranking);
                if let Some(rd) = cached {
                    self.ranking.currentir = Some(rd);
                } else {
                    let rd = RankingData::new();
                    self.ranking.currentir = Some(rd.clone());
                    if let Some(cache) = main.ranking_data_cache_mut() {
                        cache.put_course_any(course, lnmode, Box::new(rd));
                    }
                }
            }
            // Set rival score/chart option to None for course play
            {
                let res = self
                    .player_resource
                    .as_mut()
                    .expect("player_resource is Some");
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

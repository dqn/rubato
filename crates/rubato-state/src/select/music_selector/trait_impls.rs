use super::*;

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
                    let mut skin_bar = crate::select::skin_bar::SkinBar::new(bar_data.center_bar);
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
                            crate::select::skin_distribution_graph::SkinDistributionGraph::new_with_images(
                                graph_type, images,
                            )
                        } else {
                            crate::select::skin_distribution_graph::SkinDistributionGraph::new(
                                graph_type,
                            )
                        };
                        graph.region.x = bar_data.graph_region.x;
                        graph.region.y = bar_data.graph_region.y;
                        graph.region.width = bar_data.graph_region.width;
                        graph.region.height = bar_data.graph_region.height;
                        skin_bar.set_graph(graph);
                    }
                    self.bar_rendering.select_center_bar = bar_data.center_bar;
                    self.bar_rendering.skin_bar = Some(skin_bar);
                    self.bar_rendering.bar = Some(BarRenderer::new(300, 100, 5));
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
        if let Some(preview) = &mut self.preview_state.preview {
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
        self.preview_state.show_note_graph = false;

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
            if let Some(ref mut cache) = self.ranking.scorecache {
                cache.update(song, self.config.play_settings.lnmode);
            }
            self.playedsong = None;
        }
        // Update score cache for previously played course
        if let Some(ref course) = self.playedcourse.take() {
            for sd in &course.hash {
                if let Some(ref mut cache) = self.ranking.scorecache {
                    cache.update(sd, self.config.play_settings.lnmode);
                }
            }
        }

        // Create preview music processor
        {
            let mut preview = PreviewMusicProcessor::new(&self.app_config);
            if let Some(sound_path) = self.sound(SoundType::Select) {
                preview.set_default(&sound_path);
            }
            self.preview_state.preview = Some(preview);
        }

        // Configure input processor per musicselectinput mode (Java L183-188)
        // musicselectinput: 0 -> mode7, 1 -> mode9, _ -> mode14
        {
            let mut input = BMSPlayerInputProcessor::new(&self.app_config, &self.config);
            let pc = match self.config.select_settings.musicselectinput {
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
                self.ranking.scorecache.as_mut(),
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
        if let Some(skin_bar) = &mut self.bar_rendering.skin_bar {
            let timer_snapshot = rubato_skin::stubs::Timer::with_timers(
                self.main_state_data.timer.now_time(),
                self.main_state_data.timer.now_micro_time(),
                self.main_state_data.timer.export_timer_array(),
            );
            let adapter = MinimalSkinMainState::new(&timer_snapshot);
            skin_bar.prepare(time, &adapter);
        }

        // Bar prepare — compute bar positions
        if let (Some(bar_renderer), Some(skin_bar)) =
            (&mut self.bar_rendering.bar, &self.bar_rendering.skin_bar)
        {
            let ctx = PrepareContext {
                center_bar: self.bar_rendering.select_center_bar,
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
            let lnmode = self.config.play_settings.lnmode;
            let center_bar = self.bar_rendering.select_center_bar;

            if let (Some(bar_renderer), Some(skin_bar)) = (
                &mut self.bar_rendering.bar,
                &mut self.bar_rendering.skin_bar,
            ) {
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
        if let Some(preview) = &mut self.preview_state.preview {
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
                    && now_time > songbar_change_time + self.preview_state.preview_duration as i64
                {
                    let should_start_preview = if let Some(ref preview) = self.preview_state.preview
                    {
                        let preview_song = preview.song_data();
                        // In Java: song != preview.getSongData() (reference comparison)
                        match preview_song {
                            Some(ps) => ps.file.sha256 != song_bar.song_data().file.sha256,
                            None => true,
                        }
                    } else {
                        false
                    };
                    if should_start_preview
                        && !matches!(self.app_config.select.song_preview, SongPreview::NONE)
                    {
                        let song_clone = song_bar.song_data().clone();
                        if let Some(preview) = &mut self.preview_state.preview {
                            preview.start(Some(&song_clone));
                        }
                    }
                }

                // Read BMS information (notes graph)
                if !self.preview_state.show_note_graph
                    && self.play.is_none()
                    && now_time
                        > songbar_change_time + self.preview_state.notes_graph_duration as i64
                {
                    if song_bar.exists_song() {
                        // Java: spawns thread to call resource.loadBMSModel(path, lnmode)
                        // and sets result on SongData for the density graph.
                        // Rust: load synchronously (BMS parsing is fast).
                        let path = song_bar.song_data().path().map(std::path::PathBuf::from);
                        let lnmode = self.config.play_settings.lnmode;
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
                    self.preview_state.show_note_graph = true;
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
        if self.ranking.current_ranking_duration != -1
            && now_time > songbar_change_time + self.ranking.current_ranking_duration
        {
            self.ranking.current_ranking_duration = -1;
            // Load/refresh ranking data from cache
            if let Some(current) = self.manager.selected()
                && let Some(main) = self.main.as_mut()
            {
                use rubato_ir::ranking_data::RankingData;
                let lnmode = main.player_config().play_settings.lnmode;
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
                        self.ranking.currentir = Some(rd.clone());
                        if let Some(cache) = main.ranking_data_cache_mut() {
                            cache.put_song_any(song, lnmode, Box::new(rd));
                        }
                    } else {
                        self.ranking.currentir = cached;
                    }
                    // Java MusicSelector L251: irc.load(this, song)
                    if let Some(ref mut rd) = self.ranking.currentir {
                        use rubato_ir::ir_chart_data::IRChartData;
                        use rubato_ir::ir_connection::IRConnection;
                        use std::sync::Arc;
                        if let Some(conn_arc) = main.ir_connection_any().and_then(|any| {
                            any.downcast_ref::<Arc<dyn IRConnection + Send + Sync>>()
                                .cloned()
                        }) {
                            let chart = IRChartData::new(song);
                            let local_score = main.read_score_data_by_hash(
                                &song.file.sha256,
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
                        self.ranking.currentir = Some(rd.clone());
                        if let Some(cache) = main.ranking_data_cache_mut() {
                            cache.put_course_any(course, lnmode, Box::new(rd));
                        }
                    } else {
                        self.ranking.currentir = cached;
                    }
                    // Java MusicSelector L261: irc.load(this, course)
                    if let Some(ref mut rd) = self.ranking.currentir {
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
        let irstate = self
            .ranking
            .currentir
            .as_ref()
            .map(|ir| ir.state())
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
                            let score_path = format!(
                                "{}/{}/score.db",
                                self.app_config.paths.playerpath, player_name
                            );
                            let scorelog_path = format!(
                                "{}/{}/scorelog.db",
                                self.app_config.paths.playerpath, player_name
                            );
                            let songinfo_path = self.app_config.paths.songinfopath.to_string();
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
                    let ipfs_available = !song.get_ipfs_str().is_empty()
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
                        play_mode
                    };
                    self.read_course(mode);
                }
                BarAction::RandomCourse => {
                    let mode = if play_mode.mode == BMSPlayerModeType::Practice {
                        BMSPlayerMode::PLAY
                    } else {
                        play_mode
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
        if let Some(preview) = &mut self.preview_state.preview {
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

        if let Some(bar) = &self.bar_rendering.bar {
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

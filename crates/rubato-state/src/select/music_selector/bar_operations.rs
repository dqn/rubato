use super::*;

impl MusicSelector {
    /// Play the OPTION_CHANGE system sound.
    pub(super) fn play_option_change(&mut self) {
        self.play_sound(SoundType::OptionChange);
    }

    pub(super) fn selected_play_config_mode(&self) -> Option<bms_model::Mode> {
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

    pub(super) fn get_selected_play_config_ref(&self) -> Option<&PlayConfig> {
        let mode = self.selected_play_config_mode()?;
        Some(&self.config.play_config_ref(mode).playconfig)
    }

    /// Get mutable reference to the PlayConfig for the currently selected mode.
    /// Matches Java MusicSelector.getSelectedBarPlayConfig().
    pub(super) fn get_selected_play_config_mut(&mut self) -> Option<&mut PlayConfig> {
        let mode = self.selected_play_config_mode()?;
        Some(&mut self.config.play_config(mode).playconfig)
    }

    /// Read a chart for play.
    /// Corresponds to Java MusicSelector.readChart(SongData, Bar)
    ///
    /// `play_mode` is passed explicitly because the caller may have already
    /// consumed `self.play` via `.take()` before reaching this point.
    pub fn read_chart(
        &mut self,
        song: &SongData,
        current: &Bar,
        play_mode: Option<&BMSPlayerMode>,
    ) {
        // Get play mode for set_bms_file encoding
        let (mode_type, mode_id) = Self::encode_bms_player_mode(play_mode);

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

        // resource.setBMSFile(path, play)
        let path_str = match song.file.path() {
            Some(p) => p,
            None => {
                ImGuiNotify::error("Failed to loading BMS : Song not found, or Song has error");
                return;
            }
        };
        let path = std::path::Path::new(&path_str);

        let load_success = rubato_types::player_resource_access::SessionMutation::set_bms_file(
            res, path, mode_type, mode_id,
        );

        if load_success {
            // Set table name/level from directory hierarchy
            let table_urls: Vec<String> = self
                .main
                .as_ref()
                .map(|m| {
                    m.config()
                        .paths
                        .table_url
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                })
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
                        tablename = Some(bar.title().to_owned());
                    }
                    if bar.as_hash_bar().is_some() && is_dtable {
                        tablelevel = Some(bar.title().to_owned());
                        break;
                    }
                }

                let res = self
                    .player_resource
                    .as_mut()
                    .expect("player_resource is Some");
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
                && self.ranking.currentir.is_none()
            {
                use rubato_ir::ranking_data::RankingData;
                let lnmode = self.config.play_settings.lnmode;
                let rd = RankingData::new();
                self.ranking.currentir = Some(rd.clone());
                if let Some(cache) = main.ranking_data_cache_mut() {
                    cache.put_song_any(song, lnmode, Box::new(rd));
                }
            }
            // Java L388: resource.setRankingData(currentir)
            {
                let res = self
                    .player_resource
                    .as_mut()
                    .expect("player_resource is Some");
                let ranking_any = self
                    .ranking
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
            let replay_index = play_mode.map_or(0, |p| p.id);
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
                .expect("player_resource is Some")
                .set_chart_option_data(chart_option);

            self.playedsong = Some(song.clone());
            self.pending_state_change = Some(MainStateType::Decide);
        } else {
            ImGuiNotify::error("Failed to loading BMS : Song not found, or Song has error");
        }
    }

    /// Encode BMSPlayerMode to (mode_type, mode_id) for PlayerResourceAccess::set_bms_file.
    pub(super) fn encode_bms_player_mode(mode: Option<&BMSPlayerMode>) -> (i32, i32) {
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
        let mode = ChartReplicationMode::get(&config.play_settings.chart_replication_mode);
        match mode {
            ChartReplicationMode::None => None,
            ChartReplicationMode::RivalChart => rival_score.map(|rival| {
                let mut opt = rubato_types::replay_data::ReplayData::new();
                opt.randomoption = rival.play_option.option % 10;
                opt.randomoption2 = (rival.play_option.option / 10) % 10;
                opt.doubleoption = rival.play_option.option / 100;
                if rival.play_option.seed < 0 {
                    opt.randomoptionseed = -1;
                    opt.randomoption2seed = -1;
                } else {
                    opt.randomoptionseed = rival.play_option.seed % (65536 * 256);
                    opt.randomoption2seed = rival.play_option.seed / (65536 * 256);
                }
                opt
            }),
            ChartReplicationMode::RivalOption => rival_score.map(|rival| {
                let mut opt = rubato_types::replay_data::ReplayData::new();
                opt.randomoption = rival.play_option.option % 10;
                opt.randomoption2 = (rival.play_option.option / 10) % 10;
                opt.doubleoption = rival.play_option.option / 100;
                opt
            }),
            ChartReplicationMode::ReplayChart | ChartReplicationMode::ReplayOption => {
                let sd = songdata?;
                let sha256 = &sd.file.sha256;
                let has_ln = sd.chart.has_undefined_long_note();
                let replay = main.read_replay_data(
                    sha256,
                    has_ln,
                    config.play_settings.lnmode,
                    replay_index,
                )?;
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

    /// Recompute cached_target_score based on config.select_settings.targetid
    /// and the selected song's total notes. Called before rendering.
    pub(super) fn refresh_cached_target_score(&mut self) {
        let targetid = self.config.select_settings.targetid.clone();
        // MYBEST and direct RIVAL_N targets are resolved via existing bar data (no cache needed).
        // RIVAL_RANK_* and RIVAL_NEXT_* need ranked rival data computed during gameplay;
        // on the select screen they resolve to None (no cache).

        // Static rate targets
        let rate = match targetid.as_str() {
            "RATE_A-" => Some(100.0_f64 * 17.0 / 27.0),
            "RATE_A" => Some(100.0_f64 * 18.0 / 27.0),
            "RATE_A+" => Some(100.0_f64 * 19.0 / 27.0),
            "RATE_AA-" => Some(100.0_f64 * 20.0 / 27.0),
            "RATE_AA" => Some(100.0_f64 * 21.0 / 27.0),
            "RATE_AA+" => Some(100.0_f64 * 22.0 / 27.0),
            "RATE_AAA-" => Some(100.0_f64 * 23.0 / 27.0),
            "RATE_AAA" => Some(100.0_f64 * 24.0 / 27.0),
            "RATE_AAA+" => Some(100.0_f64 * 25.0 / 27.0),
            "RATE_MAX-" => Some(100.0_f64 * 26.0 / 27.0),
            "MAX" => Some(100.0_f64),
            _ => {
                // Handle custom RATE_<float> targets (e.g. "RATE_85.5")
                if let Some(suffix) = targetid.strip_prefix("RATE_")
                    && let Ok(val) = suffix.parse::<f64>()
                    && (0.0..=100.0).contains(&val)
                {
                    Some(val)
                } else {
                    None
                }
            }
        };

        if let Some(rate) = rate {
            self.cached_target_score = self
                .manager
                .selected()
                .and_then(|b| b.as_song_bar())
                .map(|sb| sb.song_data().chart.notes)
                .map(|total_notes| {
                    // Same formula as StaticTargetProperty::target()
                    let exscore = (total_notes as f64 * 2.0 * rate / 100.0).ceil() as i32;
                    let mut score = rubato_types::score_data::ScoreData::default();
                    score.judge_counts.epg = exscore / 2;
                    score.judge_counts.egr = exscore % 2;
                    score
                });
            return;
        }

        // RANK_NEXT target: compute next rank threshold from local score
        if targetid == "RANK_NEXT" {
            self.cached_target_score =
                self.manager
                    .selected()
                    .and_then(|b| b.as_song_bar())
                    .map(|sb| {
                        let total_notes = sb.song_data().chart.notes;
                        let max = total_notes * 2;
                        let nowscore = sb
                            .selectable
                            .bar_data
                            .score()
                            .map(|s| s.exscore())
                            .unwrap_or(0);
                        let mut targetscore = max;
                        for i in 15..27 {
                            let target = (max as f64 * i as f64 / 27.0).ceil() as i32;
                            if nowscore < target {
                                targetscore = target;
                                break;
                            }
                        }
                        let mut score = rubato_types::score_data::ScoreData::default();
                        score.judge_counts.epg = targetscore / 2;
                        score.judge_counts.egr = targetscore % 2;
                        score
                    });
            return;
        }

        // IR-based targets: resolve from ranking data if available
        if targetid.starts_with("IR_") {
            self.cached_target_score = self.resolve_ir_target_score(&targetid);
            return;
        }

        self.cached_target_score = None;
    }

    /// Recompute cached_score_data_property from the currently selected bar's score.
    /// Called before rendering so that `SelectSkinContext::score_data_property()` returns
    /// up-to-date values for Lua skins that read `main_state.rate()` / `main_state.exscore()`.
    pub(super) fn refresh_cached_score_data_property(&mut self) {
        let score = self.manager.selected().and_then(|b| b.score());
        let rival = self.manager.selected().and_then(|b| b.rival_score());
        self.cached_score_data_property =
            rubato_types::score_data_property::ScoreDataProperty::new();
        self.cached_score_data_property
            .update_score_and_rival(score, rival);
    }

    /// Resolve an IR-based target score from the current ranking data.
    fn resolve_ir_target_score(
        &self,
        targetid: &str,
    ) -> Option<rubato_types::score_data::ScoreData> {
        let rd = self.ranking.currentir.as_ref()?;
        if rd.state() != rubato_ir::ranking_data::FINISH || rd.total_player() == 0 {
            return None;
        }
        let total = rd.total_player();

        // Determine target index from the IR target type
        let target_index = if let Some(suffix) = targetid.strip_prefix("IR_NEXT_")
            && let Ok(value) = suffix.parse::<i32>()
            && value >= 1
        {
            // On the select screen, nowscore is the local best score
            let nowscore = self
                .manager
                .selected()
                .and_then(|b| b.as_song_bar())
                .and_then(|sb| sb.selectable.bar_data.score())
                .map(|s| s.exscore())
                .unwrap_or(0);
            // Default to rank 0 (best player) when local score is below every IR entry.
            let mut idx = 0;
            for i in 0..total {
                if let Some(score) = rd.score(i)
                    && score.exscore() <= nowscore
                {
                    idx = (i - value).max(0);
                    break;
                }
            }
            idx
        } else if let Some(suffix) = targetid.strip_prefix("IR_RANKRATE_")
            && let Ok(value) = suffix.parse::<i32>()
            && value > 0
            && value < 100
        {
            total * value / 100
        } else if let Some(suffix) = targetid.strip_prefix("IR_RANK_")
            && let Ok(value) = suffix.parse::<i32>()
            && value > 0
        {
            (value.min(total) - 1).max(0)
        } else {
            return None;
        };

        let ir_score = rd.score(target_index)?;
        let exscore = ir_score.exscore();
        let mut score = rubato_types::score_data::ScoreData::default();
        score.judge_counts.epg = exscore / 2;
        score.judge_counts.egr = exscore % 2;
        Some(score)
    }

    pub fn sort(&self) -> i32 {
        self.config.select_settings.sort
    }

    pub fn set_sort(&mut self, sort: i32) {
        let sort = sort.rem_euclid(BarSorter::DEFAULT_SORTER.len() as i32);
        self.config.select_settings.sort = sort;
        self.config.select_settings.sortid =
            Some(BarSorter::DEFAULT_SORTER[sort as usize].name().to_string());
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
                    .set_timer_on(rubato_types::timer_id::TimerId::new(
                        skin_property::TIMER_PANEL1_OFF.as_i32() + self.panelstate - 1,
                    ));
                self.main_state_data
                    .timer
                    .set_timer_off(rubato_types::timer_id::TimerId::new(
                        skin_property::TIMER_PANEL1_ON.as_i32() + self.panelstate - 1,
                    ));
            }
            if panelstate != 0 {
                self.main_state_data
                    .timer
                    .set_timer_on(rubato_types::timer_id::TimerId::new(
                        skin_property::TIMER_PANEL1_ON.as_i32() + panelstate - 1,
                    ));
                self.main_state_data
                    .timer
                    .set_timer_off(rubato_types::timer_id::TimerId::new(
                        skin_property::TIMER_PANEL1_OFF.as_i32() + panelstate - 1,
                    ));
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
        self.bar_rendering.bar.as_ref()
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
        if let Some(preview) = &self.preview_state.preview
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
            if should_stop && let Some(preview) = &mut self.preview_state.preview {
                preview.start(None);
            }
        }

        self.preview_state.show_note_graph = false;

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
                            let lnmode = self.config.play_settings.lnmode;
                            let song = song_bar.song_data();
                            self.ranking.currentir = main
                                .ranking_data_cache()
                                .and_then(|c| c.song_any(song, lnmode))
                                .and_then(|a| a.downcast::<RankingData>().ok())
                                .map(|ranking| *ranking);
                        }
                        let ranking_reload_dur = self.ranking.ranking_reload_duration;
                        let ranking_dur = self.ranking.ranking_duration as i64;
                        self.ranking.current_ranking_duration =
                            if let Some(ref ir) = self.ranking.currentir {
                                (ranking_reload_dur - (now_millis - ir.last_update_time())).max(0)
                                    + ranking_dur
                            } else {
                                ranking_dur
                            };
                    } else {
                        self.ranking.currentir = None;
                        self.ranking.current_ranking_duration = -1;
                    }
                } else if let Some(grade_bar) = current.as_grade_bar() {
                    if grade_bar.exists_all_songs() {
                        // Refresh currentir from cache for course
                        if let Some(main) = self.main.as_ref() {
                            use rubato_ir::ranking_data::RankingData;
                            let lnmode = self.config.play_settings.lnmode;
                            let course = grade_bar.course_data();
                            self.ranking.currentir = main
                                .ranking_data_cache()
                                .and_then(|c| c.course_any(course, lnmode))
                                .and_then(|a| a.downcast::<RankingData>().ok())
                                .map(|ranking| *ranking);
                        }
                        let ranking_reload_dur = self.ranking.ranking_reload_duration;
                        let ranking_dur = self.ranking.ranking_duration as i64;
                        self.ranking.current_ranking_duration =
                            if let Some(ref ir) = self.ranking.currentir {
                                (ranking_reload_dur - (now_millis - ir.last_update_time())).max(0)
                                    + ranking_dur
                            } else {
                                ranking_dur
                            };
                    } else {
                        self.ranking.currentir = None;
                        self.ranking.current_ranking_duration = -1;
                    }
                } else {
                    self.ranking.currentir = None;
                    self.ranking.current_ranking_duration = -1;
                }
            } else {
                self.ranking.currentir = None;
                self.ranking.current_ranking_duration = -1;
            }
        } else {
            self.ranking.currentir = None;
            self.ranking.current_ranking_duration = -1;
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
}

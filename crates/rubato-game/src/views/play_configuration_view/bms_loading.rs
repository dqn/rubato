use super::*;

impl PlayConfigurationView {
    /// Update play config
    /// Translates: public void updatePlayConfig()
    pub fn update_play_config(&mut self) {
        let player = match &mut self.player {
            Some(p) => p,
            None => return,
        };

        if let Some(ref pc) = self.pc {
            let mode = pc.to_mode();
            let conf = &mut player.play_config(mode).playconfig;
            conf.hispeed = self.hispeed as f32;
            conf.duration = self.gvalue;
            conf.enable_constant = self.enable_constant;
            conf.constant_fadein_time = self.const_fadein_time;
            conf.hispeedmargin = self.hispeedmargin as f32;
            conf.fixhispeed = self.fixhispeed.unwrap_or(0);
            conf.enablelanecover = self.enable_lanecover;
            conf.lanecover = self.lanecover as f32 / 1000.0;
            conf.lanecovermarginlow = self.lanecovermarginlow as f32 / 1000.0;
            conf.lanecovermarginhigh = self.lanecovermarginhigh as f32 / 1000.0;
            conf.lanecoverswitchduration = self.lanecoverswitchduration;
            conf.enablelift = self.enable_lift;
            conf.enablehidden = self.enable_hidden;
            conf.lift = self.lift as f32 / 1000.0;
            conf.hidden = self.hidden as f32 / 1000.0;
            // judgealgorithm → judgetype
            // JudgeAlgorithm.values()[judgealgorithm.getValue()].name()
            if let Some(alg_idx) = self.judgealgorithm {
                let judge_algs = rubato_types::JudgeAlgorithm::values();
                if (alg_idx as usize) < judge_algs.len() {
                    conf.judgetype = judge_algs[alg_idx as usize].name().to_string();
                }
            }
            conf.hispeedautoadjust = self.hispeedautoadjust;
        }

        self.pc = self.playconfig;

        if let Some(ref pc) = self.pc {
            let mode = pc.to_mode();
            let conf = &player.play_config(mode).playconfig;
            self.hispeed = conf.hispeed as f64;
            self.gvalue = conf.duration;
            self.enable_constant = conf.enable_constant;
            self.const_fadein_time = conf.constant_fadein_time;
            self.hispeedmargin = conf.hispeedmargin as f64;
            self.fixhispeed = Some(conf.fixhispeed);
            self.enable_lanecover = conf.enablelanecover;
            self.lanecover = (conf.lanecover * 1000.0) as i32;
            self.lanecovermarginlow = (conf.lanecovermarginlow * 1000.0) as i32;
            self.lanecovermarginhigh = (conf.lanecovermarginhigh * 1000.0) as i32;
            self.lanecoverswitchduration = conf.lanecoverswitchduration;
            self.enable_lift = conf.enablelift;
            self.enable_hidden = conf.enablehidden;
            self.lift = (conf.lift * 1000.0) as i32;
            self.hidden = (conf.hidden * 1000.0) as i32;
            self.judgealgorithm = Some(rubato_types::JudgeAlgorithm::index(&conf.judgetype).max(0));
            self.hispeedautoadjust = conf.hispeedautoadjust;
        }
    }

    /// Start game
    /// Translates: public void start()
    pub fn start(&mut self) {
        self.commit();
        self.player_panel_disabled = true;
        self.video_tab_disabled = true;
        self.audio_tab_disabled = true;
        self.input_tab_disabled = true;
        self.resource_tab_disabled = true;
        self.option_tab_disabled = true;
        self.other_tab_disabled = true;
        self.ir_tab_disabled = true;
        self.stream_tab_disabled = true;
        self.discord_tab_disabled = true;
        self.obs_tab_disabled = true;
        self.control_panel_disabled = true;

        // Minimise the stage after start
        // In egui, launcher closes when play_requested is set (handled by LauncherUi::update)

        if let (Some(config), Some(player)) = (&self.config, &self.player) {
            MainLoader::play(
                None,
                BMSPlayerMode::PLAY,
                true,
                config,
                player,
                self.song_updated,
            );
        }
    }

    /// Load all BMS
    /// Translates: public void loadAllBMS()
    pub fn load_all_bms(&mut self) {
        self.commit();
        self.load_bms(None, true);
    }

    /// Load diff BMS
    /// Translates: public void loadDiffBMS()
    pub fn load_diff_bms(&mut self) {
        self.commit();
        self.load_bms(None, false);
    }

    /// Load BMS path
    /// Translates: public void loadBMSPath(String updatepath)
    pub fn load_bms_path(&mut self, updatepath: &str) {
        self.commit();
        self.load_bms(Some(updatepath.to_string()), false);
    }

    /// Load BMS and update song database on a background thread.
    ///
    /// Translates: public void loadBMS(String updatepath, boolean updateAll)
    ///
    /// Java spawns two threads: one for the progress UI (JavaFX AnimationTimer)
    /// and one for the actual DB update. In Rust/egui, the UI polls
    /// `bms_loading_state()` each frame to display progress, so we only need
    /// a single worker thread.
    pub fn load_bms(&mut self, updatepath: Option<String>, update_all: bool) {
        self.commit();

        let config = match &self.config {
            Some(c) => c.clone(),
            None => {
                log::warn!("load_bms called without config");
                return;
            }
        };

        // Don't start a new load while one is already running
        if self.bms_loading_handle.is_some() {
            log::warn!("BMS loading already in progress");
            return;
        }

        // Reset any previous result
        self.bms_loading_result = None;

        let listener = Arc::new(SongListener::new());
        let listener_clone = Arc::clone(&listener);

        let songpath = config.paths.songpath.clone();
        let bmsroot = config.paths.bmsroot.clone();
        let use_song_info = config.use_song_info;
        let songinfopath = config.paths.songinfopath.clone();

        let join_handle = std::thread::spawn(move || -> anyhow::Result<()> {
            log::info!("song.db update started");

            let songdb = SQLiteSongDatabaseAccessor::new(&songpath, &bmsroot)?;

            let infodb = if use_song_info {
                match SongInformationAccessor::new(&songinfopath) {
                    Ok(db) => Some(db),
                    Err(e) => {
                        log::warn!("Failed to open song info DB: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            songdb.update_song_datas_with_listener(
                updatepath.as_deref(),
                &bmsroot,
                update_all,
                false,
                infodb
                    .as_ref()
                    .map(|db| db as &dyn rubato_types::song_information_db::SongInformationDb),
                &listener_clone,
            );

            log::info!("song.db update completed");
            Ok(())
        });

        self.bms_loading_handle = Some(BmsLoadingHandle {
            listener,
            join_handle,
        });
    }

    /// Get the current BMS loading state.
    ///
    /// Call this from the egui update loop to display progress.
    pub fn bms_loading_state(&self) -> BmsLoadingState {
        if let Some(handle) = &self.bms_loading_handle {
            BmsLoadingState::Loading {
                bms_files: handle.listener.bms_files_count(),
                processed_files: handle.listener.processed_bms_files_count(),
                new_files: handle.listener.new_bms_files_count(),
            }
        } else if let Some(result) = &self.bms_loading_result {
            match result {
                Ok(()) => BmsLoadingState::Completed,
                Err(msg) => BmsLoadingState::Failed(msg.clone()),
            }
        } else {
            BmsLoadingState::Idle
        }
    }

    /// Poll the background thread for completion.
    ///
    /// Call this each frame from the egui update loop. When the thread
    /// finishes, this sets `song_updated = true` and transitions the
    /// state to Completed or Failed.
    pub fn poll_bms_loading(&mut self) {
        let finished = self
            .bms_loading_handle
            .as_ref()
            .is_some_and(|h| h.join_handle.is_finished());

        if finished {
            let handle = self.bms_loading_handle.take().expect("take");
            match handle.join_handle.join() {
                Ok(Ok(())) => {
                    self.song_updated = true;
                    self.bms_loading_result = Some(Ok(()));
                    log::info!("BMS loading completed successfully");
                }
                Ok(Err(e)) => {
                    let msg = format!("{}", e);
                    log::error!("BMS loading failed: {}", msg);
                    self.bms_loading_result = Some(Err(msg));
                }
                Err(_panic) => {
                    let msg = "BMS loading thread panicked".to_string();
                    log::error!("{}", msg);
                    self.bms_loading_result = Some(Err(msg));
                }
            }
        }
    }

    /// Reset the loading state back to Idle.
    ///
    /// Call after the UI has acknowledged the Completed/Failed state.
    pub fn reset_bms_loading(&mut self) {
        self.bms_loading_result = None;
    }

    /// Returns true if BMS loading is currently in progress.
    pub fn is_bms_loading(&self) -> bool {
        self.bms_loading_handle.is_some()
    }

    /// Import score data from LR2
    /// Translates: public void importScoreDataFromLR2()
    pub fn import_score_data_from_lr2(&mut self) {
        let lr2_path = match crate::platform::show_file_chooser("Select LR2 score database") {
            Some(d) => d,
            None => return,
        };

        self.import_score_data_from_lr2_path(&lr2_path);
    }

    /// Import score data from LR2 given a path to the LR2 score.db.
    ///
    /// Separated from the file-chooser flow so the logic is testable.
    /// Runs the import in a background thread to avoid blocking the UI.
    pub(super) fn import_score_data_from_lr2_path(&mut self, lr2_path: &str) {
        if self.lr2_import_handle.is_some() {
            log::warn!("LR2 score import already in progress");
            return;
        }

        let (config, player_selected) = match (&self.config, &self.players_selected) {
            (Some(c), Some(p)) => (c, p),
            _ => return,
        };

        let sep = std::path::MAIN_SEPARATOR;
        let score_db_path = format!(
            "{}{sep}{}{sep}score.db",
            &config.paths.playerpath, player_selected
        );
        let songpath = config.paths.songpath.clone();
        let bmsroot = config.paths.bmsroot.clone();
        let lr2_path = lr2_path.to_string();

        let handle = std::thread::spawn(move || {
            let scoredb = match crate::core::score_database_accessor::ScoreDatabaseAccessor::new(
                &score_db_path,
            ) {
                Ok(db) => db,
                Err(e) => {
                    log::error!("Failed to open score database {}: {}", score_db_path, e);
                    return;
                }
            };

            let songdb = match SQLiteSongDatabaseAccessor::new(&songpath, &bmsroot) {
                Ok(db) => db,
                Err(e) => {
                    log::error!("Failed to open song database: {}", e);
                    return;
                }
            };

            let importer = crate::external::score_data_importer::ScoreDataImporter::new(scoredb);
            importer.import_from_lr2_score_database(&lr2_path, &songdb);
            log::info!("LR2 score import completed");
        });

        self.lr2_import_handle = Some(handle);
    }

    /// Poll for LR2 import completion. Call from the render loop.
    pub fn poll_lr2_import(&mut self) {
        if let Some(ref handle) = self.lr2_import_handle
            && handle.is_finished()
            && let Some(handle) = self.lr2_import_handle.take()
            && let Err(e) = handle.join()
        {
            log::error!("LR2 import thread panicked: {:?}", e);
        }
    }

    /// Returns true if LR2 import is in progress.
    pub fn is_lr2_importing(&self) -> bool {
        self.lr2_import_handle.is_some()
    }

    /// Wait for LR2 import to complete. Used in tests.
    #[cfg(test)]
    pub fn wait_for_lr2_import(&mut self) {
        if let Some(handle) = self.lr2_import_handle.take() {
            handle.join().expect("LR2 import thread panicked");
        }
    }

    /// Exit
    /// Translates: public void exit()
    pub fn exit(&mut self) {
        self.commit();
        self.exit_requested = true;
    }
}

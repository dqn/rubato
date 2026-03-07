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
            conf.hispeed = self.input.hispeed as f32;
            conf.duration = self.input.gvalue;
            conf.enable_constant = self.input.enable_constant;
            conf.constant_fadein_time = self.input.const_fadein_time;
            conf.hispeedmargin = self.input.hispeedmargin as f32;
            conf.fixhispeed = self.input.fixhispeed.unwrap_or(0);
            conf.enablelanecover = self.lane.enable_lanecover;
            conf.lanecover = self.lane.lanecover as f32 / 1000.0;
            conf.lanecovermarginlow = self.lane.lanecovermarginlow as f32 / 1000.0;
            conf.lanecovermarginhigh = self.lane.lanecovermarginhigh as f32 / 1000.0;
            conf.lanecoverswitchduration = self.lane.lanecoverswitchduration;
            conf.enablelift = self.lane.enable_lift;
            conf.enablehidden = self.lane.enable_hidden;
            conf.lift = self.lane.lift as f32 / 1000.0;
            conf.hidden = self.lane.hidden as f32 / 1000.0;
            // judgealgorithm -> judgetype
            // JudgeAlgorithm.values()[judgealgorithm.getValue()].name()
            if let Some(alg_idx) = self.judge.judgealgorithm {
                let judge_algs = rubato_core::stubs::JudgeAlgorithm::values();
                if (alg_idx as usize) < judge_algs.len() {
                    conf.judgetype = judge_algs[alg_idx as usize].name().to_string();
                }
            }
            conf.hispeedautoadjust = self.input.hispeedautoadjust;
        }

        self.pc = self.playconfig;

        if let Some(ref pc) = self.pc {
            let mode = pc.to_mode();
            let conf = &player.play_config(mode).playconfig.clone();
            self.input.hispeed = conf.hispeed as f64;
            self.input.gvalue = conf.duration;
            self.input.enable_constant = conf.enable_constant;
            self.input.const_fadein_time = conf.constant_fadein_time;
            self.input.hispeedmargin = conf.hispeedmargin as f64;
            self.input.fixhispeed = Some(conf.fixhispeed);
            self.lane.enable_lanecover = conf.enablelanecover;
            self.lane.lanecover = (conf.lanecover * 1000.0) as i32;
            self.lane.lanecovermarginlow = (conf.lanecovermarginlow * 1000.0) as i32;
            self.lane.lanecovermarginhigh = (conf.lanecovermarginhigh * 1000.0) as i32;
            self.lane.lanecoverswitchduration = conf.lanecoverswitchduration;
            self.lane.enable_lift = conf.enablelift;
            self.lane.enable_hidden = conf.enablehidden;
            self.lane.lift = (conf.lift * 1000.0) as i32;
            self.lane.hidden = (conf.hidden * 1000.0) as i32;
            self.judge.judgealgorithm =
                Some(rubato_core::stubs::JudgeAlgorithm::index(&conf.judgetype).max(0));
            self.input.hispeedautoadjust = conf.hispeedautoadjust;
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
        let lr2_path = match crate::stubs::show_file_chooser("Select LR2 score database") {
            Some(d) => d,
            None => return,
        };

        self.import_score_data_from_lr2_path(&lr2_path);
    }

    /// Import score data from LR2 given a path to the LR2 score.db.
    ///
    /// Separated from the file-chooser flow so the logic is testable.
    pub(super) fn import_score_data_from_lr2_path(&self, lr2_path: &str) {
        let (config, player_selected) = match (&self.config, &self.players_selected) {
            (Some(c), Some(p)) => (c, p),
            _ => return,
        };

        let sep = std::path::MAIN_SEPARATOR;
        let score_db_path = format!(
            "{}{sep}{}{sep}score.db",
            &config.paths.playerpath, player_selected
        );

        let scoredb = match rubato_core::score_database_accessor::ScoreDatabaseAccessor::new(
            &score_db_path,
        ) {
            Ok(db) => db,
            Err(e) => {
                log::error!("Failed to open score database {}: {}", score_db_path, e);
                return;
            }
        };

        let songdb =
            match SQLiteSongDatabaseAccessor::new(&config.paths.songpath, &config.paths.bmsroot) {
                Ok(db) => db,
                Err(e) => {
                    log::error!("Failed to open song database: {}", e);
                    return;
                }
            };

        let importer = rubato_external::score_data_importer::ScoreDataImporter::new(scoredb);
        importer.import_from_lr2_score_database(lr2_path, &songdb);
    }

    /// Start Twitter auth
    /// Translates: public void startTwitterAuth()
    pub fn start_twitter_auth(&mut self) {
        match TwitterAuth::start_auth(
            &self.txt_twitter_consumer_key,
            &self.txt_twitter_consumer_secret,
        ) {
            Ok((token, secret)) => {
                if let Some(ref mut player) = self.player {
                    player.twitter_consumer_key = Some(self.txt_twitter_consumer_key.clone());
                    player.twitter_consumer_secret = Some(self.txt_twitter_consumer_secret.clone());
                    player.twitter_access_token = Some(String::new());
                    player.twitter_access_token_secret = Some(String::new());
                }
                self.request_token = Some((token, secret));
                self.twitter_pin_enabled = true;
                self.txt_twitter_authenticated_visible = false;
                // Open browser with auth URL → todo
            }
            Err(e) => {
                warn!("Twitter auth error: {}", e);
            }
        }
    }

    /// Start PIN auth
    /// Translates: public void startPINAuth()
    pub fn start_pin_auth(&mut self) {
        let consumer_key = self
            .player
            .as_ref()
            .and_then(|p| p.twitter_consumer_key.clone())
            .unwrap_or_default();
        let consumer_secret = self
            .player
            .as_ref()
            .and_then(|p| p.twitter_consumer_secret.clone())
            .unwrap_or_default();

        if self.player.is_none() {
            return;
        }

        let request_token = self.request_token.clone();
        if let Some((ref token, ref secret)) = request_token {
            match TwitterAuth::complete_pin_auth(
                &consumer_key,
                &consumer_secret,
                token,
                secret,
                &self.txt_twitter_pin,
            ) {
                Ok((access_token, access_token_secret)) => {
                    if let Some(ref mut player) = self.player {
                        player.twitter_access_token = Some(access_token);
                        player.twitter_access_token_secret = Some(access_token_secret);
                    }
                    self.commit();
                    if let Some(config) = self.config.clone() {
                        self.update(config);
                    }
                }
                Err(e) => {
                    warn!("Twitter PIN auth error: {}", e);
                }
            }
        }
    }

    /// Exit
    /// Translates: public void exit()
    pub fn exit(&mut self) {
        self.commit();
        self.exit_requested = true;
    }
}

use super::*;

impl PlayConfigurationView {
    /// Show what's new popup
    /// Translates: private void whatsNewPopup()
    ///
    /// In Java, this creates a JavaFX Dialog with version changelog.
    /// In Rust/egui, this sets a flag so LauncherUi renders the popup window.
    /// The actual rendering is done by LauncherUi::render_popups().
    pub fn whats_new_popup(&self) {
        log::info!("What's New popup: version {}", Version::get_version());
    }

    /// Check for new version
    /// Translates: private void checkNewVersion()
    pub fn check_new_version(&mut self) {
        let mut version_checker = MainLoader::version_checker();
        let message = version_checker.message().to_string();
        let download_url = version_checker.download_url().map(|s| s.to_string());
        self.newversion_text = message;
        self.newversion_url = download_url;
    }

    /// Set BMS information loader
    /// Translates: public void setBMSInformationLoader(MainLoader loader)
    pub fn set_bms_information_loader(&mut self, loader: MainLoader) {
        self.loader = Some(loader);
    }

    /// Update dialog items
    /// Translates: public void update(Config config)
    pub fn update(&mut self, config: Config) {
        self.config = Some(config);
        let config = self.config.as_ref().expect("config is Some");

        // Show the What's New popup upon version change
        let current_version = Version::get_version().to_string();
        let last_version = config.last_booted_version.clone();
        // If current version is greater than last version
        if Version::compare_to_string(Some(&last_version)) > 0 {
            self.whats_new_popup();
            if let Some(ref mut c) = self.config {
                c.last_booted_version = current_version;
            }
        }

        let config = self.config.as_ref().expect("config is Some");
        let playerpath = config.paths.playerpath.clone();
        self.players = rubato_core::player_config::read_all_player_id(&playerpath);

        // videoController.update(config)
        self.video_controller.update(config);
        // audioController.update(config.getAudioConfig())
        if let Some(ref audio) = config.audio {
            self.audio_controller.update(audio.clone());
        }
        // musicselectController.update(config)
        self.music_select_controller.update(config);

        self.bgmpath = config.paths.bgmpath.clone();
        self.soundpath = config.paths.soundpath.clone();

        // resourceController.update(config)
        // discordController.update(config)
        // skinController.update(config)
        // These take &mut Config, so we temporarily take ownership
        {
            let mut config = self.config.take().expect("take");
            self.resource_controller.update(&mut config);
            self.discord_controller.update(&mut config);
            self.skin_controller.update_config(&config);
            // obsController.update(config) -- takes Config by value, give a clone
            self.obs_controller.update(config.clone());
            self.config = Some(config);
        }

        let config = self.config.as_ref().expect("config is Some");
        self.usecim = config.select.cache_skin_image;
        self.clipboard_screenshot = config.integration.set_clipboard_screenshot;

        self.enable_ipfs = config.network.enable_ipfs;
        self.ipfsurl = config.network.ipfsurl.clone();

        self.enable_http = config.network.enable_http;
        self.http_download_source_selected = Some(config.network.download_source.clone());
        self.default_download_url = config.network.default_download_url.clone();
        self.override_download_url = config.network.override_download_url.clone();

        let playername_config = config.playername.clone().unwrap_or_default();
        if self.players.contains(&playername_config) {
            self.players_selected = Some(playername_config);
        } else if !self.players.is_empty() {
            self.players_selected = Some(self.players[0].clone());
        }
        self.update_player();

        // tableController.init and update deferred to egui integration
        // (requires ScoreDatabaseAccessor which depends on runtime DB state)
    }

    /// Change player
    /// Translates: public void changePlayer()
    pub fn change_player(&mut self) {
        self.commit_player();
        self.update_player();
    }

    /// Add player
    /// Translates: public void addPlayer()
    pub fn add_player(&mut self) {
        let config = match &self.config {
            Some(c) => c,
            None => return,
        };
        let ids = rubato_core::player_config::read_all_player_id(&config.paths.playerpath);
        for i in 1..1000 {
            let playerid = format!("player{}", i);
            let mut b = true;
            for id in &ids {
                if *id == playerid {
                    b = false;
                    break;
                }
            }
            if b {
                let _ =
                    rubato_core::player_config::create_player(&config.paths.playerpath, &playerid);
                self.players.push(playerid);
                break;
            }
        }
    }

    /// Update player config into UI fields
    /// Translates: public void updatePlayer()
    pub fn update_player(&mut self) {
        let config = match &self.config {
            Some(c) => c,
            None => return,
        };
        let playerid = match &self.players_selected {
            Some(p) => p.clone(),
            None => return,
        };
        let mut player = match PlayerConfig::read_player_config(&config.paths.playerpath, &playerid)
        {
            Ok(p) => p,
            Err(e) => {
                warn!("Player config failed to load: {}", e);
                PlayerConfig::default()
            }
        };

        self.playername = player.name.clone();

        // videoController.updatePlayer(player)
        self.video_controller.update_player(&mut player);
        // musicselectController.updatePlayer(player)
        self.music_select_controller.update_player(&player);

        self.input.scoreop = Some(player.play_settings.random);
        self.input.scoreop2 = Some(player.play_settings.random2);
        self.input.doubleop = Some(player.play_settings.doubleoption);
        self.seventoninepattern = Some(player.note_modifier_settings.seven_to_nine_pattern);
        self.seventoninetype = Some(player.note_modifier_settings.seven_to_nine_type);
        self.exitpressduration = player.misc_settings.exit_press_duration;
        self.display.chartpreview = player.display_settings.chart_preview;
        self.display.guidese = player.display_settings.is_guide_se;
        self.windowhold = player.select_settings.is_window_hold;
        self.gauge.gaugeop = Some(player.play_settings.gauge);
        self.input.lntype = Some(player.play_settings.lnmode);

        self.judge.notesdisplaytiming = player.judge_settings.judgetiming;
        self.judge.notesdisplaytimingautoadjust =
            player.judge_settings.notes_display_timing_auto_adjust;

        self.judge.bpmguide = player.display_settings.bpmguide;
        self.gauge.gaugeautoshift = Some(player.play_settings.gauge_auto_shift);
        self.gauge.bottomshiftablegauge = Some(player.play_settings.bottom_shiftable_gauge);

        self.judge.customjudge = player.judge_settings.custom_judge;
        self.judge.njudgepg = player.judge_settings.key_judge_window_rate_perfect_great;
        self.judge.njudgegr = player.judge_settings.key_judge_window_rate_great;
        self.judge.njudgegd = player.judge_settings.key_judge_window_rate_good;
        self.judge.sjudgepg = player
            .judge_settings
            .scratch_judge_window_rate_perfect_great;
        self.judge.sjudgegr = player.judge_settings.scratch_judge_window_rate_great;
        self.judge.sjudgegd = player.judge_settings.scratch_judge_window_rate_good;
        self.minemode = Some(player.play_settings.mine_mode);
        self.scrollmode = Some(player.display_settings.scroll_mode);
        self.longnotemode = Some(player.note_modifier_settings.longnote_mode);
        self.forcedcnendings = player.play_settings.forcedcnendings;
        self.longnoterate = player.note_modifier_settings.longnote_rate;
        self.hranthresholdbpm = player.play_settings.hran_threshold_bpm;
        self.display.judgeregion = player.display_settings.showjudgearea;
        self.display.markprocessednote = player.display_settings.markprocessednote;
        self.display.extranotedepth = player.display_settings.extranote_depth;

        if player.misc_settings.autosavereplay.len() >= 4 {
            self.autosavereplay1 = Some(player.misc_settings.autosavereplay[0]);
            self.autosavereplay2 = Some(player.misc_settings.autosavereplay[1]);
            self.autosavereplay3 = Some(player.misc_settings.autosavereplay[2]);
            self.autosavereplay4 = Some(player.misc_settings.autosavereplay[3]);
        }

        self.display.target = player.select_settings.targetlist.clone();
        self.display.target_selected = Some(player.select_settings.targetid.clone());
        self.display.showhiddennote = player.display_settings.showhiddennote;
        self.display.showpastnote = player.display_settings.showpastnote;

        // irController.update(player)
        self.ir_controller.update(&mut player);
        // streamController.update(player)
        self.stream_controller.update(&player);

        self.twitter_pin_enabled = false;
        if let Some(ref token) = player.twitter_access_token {
            self.txt_twitter_authenticated_visible = !token.is_empty();
        } else {
            self.txt_twitter_authenticated_visible = false;
        }

        self.pc = None;
        self.playconfig = Some(PlayMode::BEAT_7K);
        self.player = Some(player);

        // update_play_config must happen before inputController/skinController updates
        // because Java calls updatePlayConfig() then inputController.update(player)
        self.update_play_config();

        // inputController.update(player) -- needs &mut PlayerConfig
        if let Some(ref mut player) = self.player {
            self.input_controller.update(player);
        }
        // skinController.update(player)
        if let Some(ref player) = self.player {
            self.skin_controller.update_player(player);
        }
    }

    /// Commit config to file
    /// Translates: public void commit()
    pub fn commit(&mut self) {
        // videoController.commit(config)
        if let Some(ref mut config) = self.config {
            self.video_controller.commit(config);
        }
        // audioController.commit()
        self.audio_controller.commit();
        // musicselectController.commit()
        self.music_select_controller.commit();

        if let Some(ref mut config) = self.config {
            config.playername = self.players_selected.clone();

            config.paths.bgmpath = self.bgmpath.clone();
            config.paths.soundpath = self.soundpath.clone();
        }

        // resourceController.commit()
        self.resource_controller.commit();
        // discordController.commit()
        self.discord_controller.commit();
        // obsController.commit()
        self.obs_controller.commit();

        if let Some(ref mut config) = self.config {
            config.select.cache_skin_image = self.usecim;

            config.network.enable_ipfs = self.enable_ipfs;
            config.network.ipfsurl = self.ipfsurl.clone();

            config.network.enable_http = self.enable_http;
            if let Some(ref source) = self.http_download_source_selected {
                config.network.download_source = source.clone();
            }
            config.network.override_download_url = self.override_download_url.clone();

            config.integration.set_clipboard_screenshot = self.clipboard_screenshot;
        }

        self.commit_player();

        if let Some(ref config) = self.config
            && let Err(e) = Config::write(config)
        {
            log::error!("Failed to write config: {}", e);
        }

        // tableController.commit()
        self.table_controller.commit();
    }

    /// Commit player config
    /// Translates: public void commitPlayer()
    pub fn commit_player(&mut self) {
        if self.player.is_none() {
            return;
        }

        {
            let player = self.player.as_mut().expect("player is Some");

            if !self.playername.is_empty() {
                player.name = self.playername.clone();
            }

            // videoController.commitPlayer(player)
            self.video_controller.commit_player(player);

            player.play_settings.random = self.input.scoreop.unwrap_or(0);
            player.play_settings.random2 = self.input.scoreop2.unwrap_or(0);
            player.play_settings.doubleoption = self.input.doubleop.unwrap_or(0);
            player.note_modifier_settings.seven_to_nine_pattern =
                self.seventoninepattern.unwrap_or(0);
            player.note_modifier_settings.seven_to_nine_type = self.seventoninetype.unwrap_or(0);
            player.misc_settings.exit_press_duration = self.exitpressduration;
            player.display_settings.chart_preview = self.display.chartpreview;
            player.display_settings.is_guide_se = self.display.guidese;
            player.select_settings.is_window_hold = self.windowhold;
            player.play_settings.gauge = self.gauge.gaugeop.unwrap_or(0);
            player.play_settings.lnmode = self.input.lntype.unwrap_or(0);
            player.judge_settings.judgetiming = self.judge.notesdisplaytiming;
            player.judge_settings.notes_display_timing_auto_adjust =
                self.judge.notesdisplaytimingautoadjust;

            player.display_settings.bpmguide = self.judge.bpmguide;
            player.play_settings.gauge_auto_shift = self.gauge.gaugeautoshift.unwrap_or(0);
            player.play_settings.bottom_shiftable_gauge =
                self.gauge.bottomshiftablegauge.unwrap_or(0);
            player.judge_settings.custom_judge = self.judge.customjudge;
            player.judge_settings.key_judge_window_rate_perfect_great = self.judge.njudgepg;
            player.judge_settings.key_judge_window_rate_great = self.judge.njudgegr;
            player.judge_settings.key_judge_window_rate_good = self.judge.njudgegd;
            player
                .judge_settings
                .scratch_judge_window_rate_perfect_great = self.judge.sjudgepg;
            player.judge_settings.scratch_judge_window_rate_great = self.judge.sjudgegr;
            player.judge_settings.scratch_judge_window_rate_good = self.judge.sjudgegd;
            player.play_settings.mine_mode = self.minemode.unwrap_or(0);
            player.display_settings.scroll_mode = self.scrollmode.unwrap_or(0);
            player.note_modifier_settings.longnote_mode = self.longnotemode.unwrap_or(0);
            player.play_settings.forcedcnendings = self.forcedcnendings;
            player.note_modifier_settings.longnote_rate = self.longnoterate;
            player.play_settings.hran_threshold_bpm = self.hranthresholdbpm;
            player.display_settings.markprocessednote = self.display.markprocessednote;
            player.display_settings.extranote_depth = self.display.extranotedepth;

            player.misc_settings.autosavereplay = vec![
                self.autosavereplay1.unwrap_or(0),
                self.autosavereplay2.unwrap_or(0),
                self.autosavereplay3.unwrap_or(0),
                self.autosavereplay4.unwrap_or(0),
            ];

            player.display_settings.showjudgearea = self.display.judgeregion;
            if let Some(ref target) = self.display.target_selected {
                player.select_settings.targetid = target.clone();
            }

            player.display_settings.showhiddennote = self.display.showhiddennote;
            player.display_settings.showpastnote = self.display.showpastnote;
        }

        // musicselectController.commitPlayer()
        self.music_select_controller.commit_player();
        // inputController.commit()
        self.input_controller.commit();
        // irController.commit()
        self.ir_controller.commit();
        // streamController.commit()
        self.stream_controller.commit();

        self.update_play_config();
        // skinController.commit()
        self.skin_controller.commit();

        if let (Some(config), Some(player)) = (&self.config, &self.player)
            && let Err(e) = PlayerConfig::write(&config.paths.playerpath, player)
        {
            log::error!("Failed to write player config: {}", e);
        }
    }

    /// Add BGM path
    /// Translates: public void addBGMPath()
    pub fn add_bgm_path(&mut self) {
        if let Some(s) = crate::stubs::show_directory_chooser("Select BGM root folder") {
            self.bgmpath = s;
        }
    }

    /// Add sound path
    /// Translates: public void addSoundPath()
    pub fn add_sound_path(&mut self) {
        if let Some(s) = crate::stubs::show_directory_chooser("Select sound effect root folder") {
            self.soundpath = s;
        }
    }

    /// Show file chooser
    /// Translates: private String showFileChooser(String title)
    fn _show_file_chooser(title: &str) -> Option<String> {
        crate::stubs::show_file_chooser(title)
    }

    /// Show directory chooser
    /// Translates: private String showDirectoryChooser(String title)
    fn _show_directory_chooser(title: &str) -> Option<String> {
        crate::stubs::show_directory_chooser(title)
    }
}

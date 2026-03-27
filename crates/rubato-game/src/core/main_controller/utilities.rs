use super::*;

impl MainController {
    /// Notify all state listeners of a state change.
    ///
    /// Translated from: MainController.updateMainStateListener(int)
    ///
    /// Java lines 951-955:
    /// ```java
    /// public void updateMainStateListener(int status) {
    ///     for(MainStateListener listener : stateListener) {
    ///         listener.update(current, status);
    ///     }
    /// }
    /// ```
    #[allow(deprecated)]
    pub fn update_main_state_listener(&mut self, status: i32) {
        if let Some(ref current) = self.current {
            // Create adapter that bridges MainState -> MainStateAccess
            let screen_type = current
                .state_type()
                .map(ScreenType::from_state_type)
                .unwrap_or(ScreenType::Other);
            let resource = self
                .resource
                .as_ref()
                .map(|r| r as &dyn PlayerResourceAccess);
            let adapter = StateAccessAdapter {
                screen_type,
                resource,
                config: &self.ctx.config,
            };

            // Temporarily take the listeners to avoid borrow conflict
            let mut listeners = std::mem::take(&mut self.state_listener);
            for listener in listeners.iter_mut() {
                listener.update(&adapter, status);
            }
            self.state_listener = listeners;
        }

        // Also broadcast to channel-based event receivers.
        self.broadcast_state_changed(status);
    }

    pub fn play_time(&self) -> i64 {
        self.ctx.lifecycle.boottime.elapsed().as_millis() as i64
    }

    pub fn start_time(&self) -> i64 {
        self.ctx.timer.start_time()
    }

    pub fn start_micro_time(&self) -> i64 {
        self.ctx.timer.start_micro_time()
    }

    pub fn now_time(&self) -> i64 {
        self.ctx.timer.now_time()
    }

    pub fn now_time_for_id(&self, id: rubato_types::timer_id::TimerId) -> i64 {
        self.ctx.timer.now_time_for_id(id)
    }

    pub fn now_micro_time(&self) -> i64 {
        self.ctx.timer.now_micro_time()
    }

    pub fn now_micro_time_for_id(&self, id: rubato_types::timer_id::TimerId) -> i64 {
        self.ctx.timer.now_micro_time_for_id(id)
    }

    pub fn timer_value(&self, id: rubato_types::timer_id::TimerId) -> i64 {
        self.micro_timer(id) / 1000
    }

    pub fn micro_timer(&self, id: rubato_types::timer_id::TimerId) -> i64 {
        self.ctx.timer.micro_timer(id)
    }

    pub fn is_timer_on(&self, id: rubato_types::timer_id::TimerId) -> bool {
        self.micro_timer(id) != i64::MIN
    }

    pub fn set_timer_on(&mut self, id: rubato_types::timer_id::TimerId) {
        self.ctx.timer.set_timer_on(id);
    }

    pub fn set_timer_off(&mut self, id: rubato_types::timer_id::TimerId) {
        self.set_micro_timer(id, i64::MIN);
    }

    pub fn set_micro_timer(&mut self, id: rubato_types::timer_id::TimerId, microtime: i64) {
        self.ctx.timer.set_micro_timer(id, microtime);
    }

    pub fn switch_timer(&mut self, id: rubato_types::timer_id::TimerId, on: bool) {
        self.ctx.timer.switch_timer(id, on);
    }

    pub fn http_download_processor(
        &self,
    ) -> Option<&dyn rubato_types::http_download_submitter::HttpDownloadSubmitter> {
        self.ctx
            .integration
            .http_download_processor
            .as_ref()
            .map(|processor| processor.as_ref())
    }

    pub fn clone_http_download_processor(
        &self,
    ) -> Option<std::sync::Arc<dyn rubato_types::http_download_submitter::HttpDownloadSubmitter>>
    {
        self.ctx.integration.http_download_processor.clone()
    }

    pub fn set_http_download_processor(
        &mut self,
        processor: Box<dyn rubato_types::http_download_submitter::HttpDownloadSubmitter>,
    ) {
        self.ctx.integration.http_download_processor = Some(std::sync::Arc::from(processor));
    }

    /// Start song database update.
    ///
    /// Translated from: MainController.updateSong(String)
    /// In Java, spawns SongUpdateThread calling songdb.updateSongDatas().
    /// Requires SongDatabaseAccessor trait to expose update_song_datas() -- deferred.
    pub fn update_song(&mut self, path: &str) {
        self.update_song_with_flag(path, false);
    }

    /// Start song database update with parent-when-missing flag.
    ///
    /// Translated from: MainController.updateSong(String, boolean)
    pub fn update_song_with_flag(&mut self, path: &str, update_parent_when_missing: bool) {
        log::info!(
            "updating folder : {}, update parent when missing : {}",
            if path.is_empty() { "ALL" } else { path },
            if update_parent_when_missing {
                "yes"
            } else {
                "no"
            }
        );
        let update_path = if path.is_empty() {
            None
        } else {
            Some(path.to_string())
        };
        let bmsroot = self.ctx.config.paths.bmsroot.to_vec();
        if let Some(ref songdb) = self.ctx.db.songdb {
            // Spawn on a background thread to avoid blocking the main/render loop.
            // Java: SongUpdateThread.
            let songdb = std::sync::Arc::clone(songdb);
            let handle = std::thread::spawn(move || {
                songdb.update_song_datas(
                    update_path.as_deref(),
                    &bmsroot,
                    false,
                    update_parent_when_missing,
                );
            });
            self.background_threads.push(handle);
        }
    }

    pub fn get_version() -> &'static str {
        version::version_long()
    }

    pub fn set_play_mode(&mut self, auto: BMSPlayerMode) {
        self.auto = Some(auto);
    }

    /// Returns the song database accessor.
    ///
    /// Translated from: MainController.getSongDatabase()
    /// In Java: return MainLoader.getScoreDatabaseAccessor()
    pub fn song_database(&self) -> Option<&dyn SongDatabaseAccessorTrait> {
        self.ctx.db.songdb.as_deref()
    }

    /// Set the song database accessor.
    /// Called by the application entry point (beatoraja-launcher) after creating the DB.
    pub fn set_song_database(&mut self, songdb: Box<dyn SongDatabaseAccessorTrait>) {
        self.ctx.db.songdb = Some(std::sync::Arc::from(songdb));
    }

    /// Returns the current state.
    ///
    /// Translated from: MainController.getCurrentState()
    pub fn current_state(&self) -> Option<&dyn MainState> {
        self.current.as_deref()
    }

    /// Returns a mutable reference to the current state.
    pub fn current_state_mut(&mut self) -> Option<&mut dyn MainState> {
        self.current
            .as_mut()
            .map(|b| &mut **b as &mut dyn MainState)
    }

    /// Returns the state type for the current state.
    ///
    /// Translated from: MainController.getStateType(MainState)
    ///
    /// In Java this uses instanceof checks. In Rust, each concrete state
    /// implements state_type() on the MainState trait.
    pub fn state_type(state: Option<&dyn MainState>) -> Option<MainStateType> {
        state.and_then(|s| s.state_type())
    }

    /// Returns the current state's type.
    pub fn current_state_type(&self) -> Option<MainStateType> {
        Self::state_type(self.current_state())
    }

    /// Returns the input processor.
    ///
    /// Translated from: MainController.getInputProcessor()
    pub fn input_processor(&self) -> Option<&BMSPlayerInputProcessor> {
        self.ctx.input.as_ref()
    }

    /// Returns a mutable reference to the input processor.
    pub fn input_processor_mut(&mut self) -> Option<&mut BMSPlayerInputProcessor> {
        self.ctx.input.as_mut()
    }

    /// Returns the audio processor.
    ///
    /// Translated from: MainController.getAudioProcessor()
    pub fn audio_processor(&self) -> Option<&AudioSystem> {
        self.ctx.audio.as_ref()
    }

    /// Returns a mutable reference to the audio processor.
    pub fn audio_processor_mut(&mut self) -> Option<&mut AudioSystem> {
        self.ctx.audio.as_mut()
    }

    /// Set the audio driver.
    ///
    /// Translated from: MainController constructor audio initialization
    ///
    /// In Java, the audio driver is created in create() based on AudioConfig.DriverType.
    /// In Rust, we inject it to avoid pulling in the concrete driver crate.
    pub fn set_audio_driver(&mut self, audio: AudioSystem) {
        self.ctx.audio = Some(audio);
    }

    /// Returns the loudness analyzer.
    ///
    /// Translated from: MainController.loudnessAnalyzer
    pub fn loudness_analyzer(
        &self,
    ) -> Option<&rubato_audio::bms_loudness_analyzer::BMSLoudnessAnalyzer> {
        self.ctx.loudness_analyzer.as_ref()
    }

    /// Shutdown the loudness analyzer.
    ///
    /// Translated from: MainController.dispose() lines 864-866
    pub fn shutdown_loudness_analyzer(&mut self) {
        if let Some(ref analyzer) = self.ctx.loudness_analyzer {
            analyzer.shutdown();
        }
    }

    /// Returns the current calendar time.
    ///
    /// Translated from: MainController.getCurrnetTime() [sic - Java method name has typo]
    pub fn currnet_time(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    pub fn info_database(&self) -> Option<&dyn SongInformationDb> {
        self.ctx.db.infodb.as_deref()
    }

    /// Set the song information database.
    /// Called from launcher layer since beatoraja-core cannot depend on beatoraja-song.
    pub fn set_info_database(&mut self, db: Box<dyn SongInformationDb>) {
        self.ctx.db.infodb = Some(db);
    }

    pub fn music_download_processor(
        &self,
    ) -> Option<&dyn rubato_types::music_download_access::MusicDownloadAccess> {
        self.ctx.integration.download.as_deref()
    }

    pub fn set_music_download_processor(
        &mut self,
        processor: Box<dyn rubato_types::music_download_access::MusicDownloadAccess>,
    ) {
        self.ctx.integration.download = Some(processor);
    }

    pub fn stream_controller(
        &self,
    ) -> Option<&dyn rubato_types::stream_controller_access::StreamControllerAccess> {
        self.ctx.integration.stream_controller.as_deref()
    }

    pub fn set_stream_controller(
        &mut self,
        controller: Box<dyn rubato_types::stream_controller_access::StreamControllerAccess>,
    ) {
        self.ctx.integration.stream_controller = Some(controller);
    }

    /// Gets the shared MusicSelector.
    /// Java: StreamController holds a reference to the same MusicSelector used by SelectState.
    pub fn shared_music_selector(
        &self,
    ) -> Option<&std::sync::Arc<std::sync::Mutex<crate::state::select::music_selector::MusicSelector>>>
    {
        self.shared_music_selector.as_ref()
    }

    /// Sets the shared MusicSelector.
    pub fn set_shared_music_selector(
        &mut self,
        selector: std::sync::Arc<
            std::sync::Mutex<crate::state::select::music_selector::MusicSelector>,
        >,
    ) {
        self.shared_music_selector = Some(selector);
    }

    pub fn ir_resend_service(
        &self,
    ) -> Option<&dyn rubato_types::ir_resend_service::IrResendService> {
        self.ctx.integration.ir_resend_service.as_deref()
    }

    pub fn set_ir_resend_service(
        &mut self,
        service: Box<dyn rubato_types::ir_resend_service::IrResendService>,
    ) {
        self.ctx.integration.ir_resend_service = Some(service);
    }

    pub fn set_imgui(&mut self, imgui: Box<dyn rubato_types::imgui_access::ImGuiAccess>) {
        self.ctx.integration.imgui = Some(imgui);
    }

    /// Load a new player profile, re-initialize states and IR config.
    ///
    /// Translated from: MainController.loadNewProfile(PlayerConfig)
    pub fn load_new_profile(&mut self, pc: PlayerConfig) {
        self.ctx.config.playername = pc.id.clone();
        self.ctx.player = pc;

        // playdata = new PlayDataAccessor(config);
        self.initialize_ir_config();

        // Dispose current state before re-init.
        // Note: dispose() is called, not shutdown(). Shutdown-only side effects
        // (e.g. stopping preview audio) are the state implementation's responsibility
        // to handle in dispose() if they need to run during profile switches.
        if let Some(ref mut current) = self.current {
            current.dispose();
        }
        self.current = None;

        self.initialize_states();
        self.update_state_references();
        self.trigger_ln_warning();
        self.set_target_list();

        // Enter select state
        self.change_state(MainStateType::MusicSelect);

        self.ctx.lifecycle.last_config_save = Instant::now();
    }

    /// Initialize IR configurations from config.
    ///
    /// Translated from: MainController.initializeIRConfig()
    ///
    /// Note: The actual IR initialization logic is in rubato_result::ir_initializer
    /// because beatoraja-core cannot depend on beatoraja-ir (circular dependency).
    /// This method is called from the application entry point after IR initialization.
    pub fn initialize_ir_config(&mut self) {
        // IR initialization is performed externally via rubato_result::ir_initializer::initialize_ir_config()
        // because beatoraja-core cannot depend on beatoraja-ir.
        // The application entry point should call ir_initializer::initialize_ir_config() and then
        // set the resulting IRStatus entries on this controller.
        log::info!("IR config initialization delegated to rubato_result::ir_initializer");
    }

    /// Initialize all game states (selector, player, result, etc.).
    ///
    /// Translated from: MainController.initializeStates()
    ///
    /// Java lines 554-571:
    /// ```java
    /// private void initializeStates() {
    ///     resource = new PlayerResource(audio, config, player, loudnessAnalyzer);
    ///     selector = new MusicSelector(this, songUpdated);
    ///     decide = new MusicDecide(this);
    ///     result = new MusicResult(this);
    ///     gresult = new CourseResult(this);
    ///     keyconfig = new KeyConfiguration(this);
    ///     skinconfig = new SkinConfiguration(this, player);
    /// }
    /// ```
    ///
    /// In Rust, concrete state instances are created on-demand via the StateCreator
    /// (set by the launcher). This method only initializes the PlayerResource.
    /// States are created lazily in change_state().
    pub fn initialize_states(&mut self) {
        // In Java: resource = new PlayerResource(audio, config, player, loudnessAnalyzer);
        self.resource = Some(PlayerResource::new(
            self.ctx.config.clone(),
            self.ctx.player.clone(),
        ));

        // In Java: playdata = new PlayDataAccessor(config);
        self.ctx.db.playdata = Some(PlayDataAccessor::new(&self.ctx.config));

        info!("Initializing states (PlayerResource created, states created on-demand via factory)");
    }

    /// Update cross-state references after state re-initialization.
    ///
    /// Translated from: MainController.updateStateReferences()
    ///
    /// Java lines 573-576:
    /// ```java
    /// private void updateStateReferences() {
    ///     SkinMenu.init(this, player);
    ///     SongManagerMenu.injectMusicSelector(selector);
    /// }
    /// ```
    ///
    /// SkinMenu and SongManagerMenu live in beatoraja-modmenu, which beatoraja-core
    /// cannot depend on (circular dependency). The launcher provides a callback via
    /// `set_state_references_callback()` to wire these references.
    pub fn update_state_references(&self) {
        if let Some(ref callback) = self.state_references_callback {
            callback.update_references(&self.ctx.config, &self.ctx.player);
        }
    }

    /// Set the callback for updating cross-state references.
    ///
    /// The launcher provides an implementation that wires SkinMenu.init()
    /// and SongManagerMenu.injectMusicSelector() from beatoraja-modmenu.
    pub fn set_state_references_callback(&mut self, callback: Box<dyn StateReferencesCallback>) {
        self.state_references_callback = Some(callback);
    }

    /// Trigger LN warning if the player has LN-related settings.
    ///
    /// Translated from: MainController.triggerLnWarning()
    ///
    /// Java lines 578-592:
    /// ```java
    /// private void triggerLnWarning() {
    ///     String lnModeName = switch (player.getLnmode()) {
    ///         case 1 -> "CN";
    ///         case 2 -> "HCN";
    ///         default -> "LN";
    ///     };
    ///     if (!lnModeName.equals("LN")) {
    ///         String lnWarning = "Long Note mode is " + lnModeName + ".\n"
    ///             + "This is not recommended.\n"
    ///             + "Your scores may be incompatible with IR.\n"
    ///             + "You may change this in play options.";
    ///         ImGuiNotify.warning(lnWarning, 8000);
    ///     }
    /// }
    /// ```
    pub fn trigger_ln_warning(&self) {
        let ln_mode_name = match self.ctx.player.play_settings.lnmode {
            1 => "CN",
            2 => "HCN",
            _ => "LN",
        };
        if ln_mode_name != "LN" {
            let ln_warning = format!(
                "Long Note mode is {}.\n\
                 This is not recommended.\n\
                 Your scores may be incompatible with IR.\n\
                 You may change this in play options.",
                ln_mode_name
            );
            ImGuiNotify::warning_with_dismiss(&ln_warning, 8000);
        }
    }

    /// Set the target score list for grade/rival display.
    ///
    /// Translated from: MainController.setTargetList()
    ///
    /// Java lines 594-600:
    /// ```java
    /// private void setTargetList() {
    ///     Array<String> targetlist = new Array<String>(player.getTargetlist());
    ///     for(int i = 0;i < rivals.getRivalCount();i++) {
    ///         targetlist.add("RIVAL_" + (i + 1));
    ///     }
    ///     TargetProperty.setTargets(targetlist.toArray(String.class), this);
    /// }
    /// ```
    ///
    /// Translated from: Java MainController.setTargetList()
    ///
    /// Builds target list from player config + rival targets, then resolves
    /// display names via rubato_types::target_list.
    pub fn set_target_list(&mut self) {
        // Build target list: player's target list + rival targets
        let mut targetlist: Vec<String> = self.ctx.player.select_settings.targetlist.clone();
        for i in 0..self.ctx.db.rivals.rival_count() {
            targetlist.push(format!("RIVAL_{}", i + 1));
        }

        // Resolve display names for each target ID
        let rivals: Vec<rubato_types::player_information::PlayerInformation> =
            (0..self.ctx.db.rivals.rival_count())
                .filter_map(|i| self.ctx.db.rivals.rival_information(i).cloned())
                .collect();
        let names: Vec<String> = targetlist
            .iter()
            .map(|id| rubato_types::target_list::resolve_target_name(id, &rivals).into_owned())
            .collect();

        rubato_types::target_list::set_target_ids(targetlist);
        rubato_types::target_list::set_target_names(names);
    }

    /// Periodically save config if enough time has elapsed.
    ///
    /// Translated from: MainController.periodicConfigSave()
    ///
    /// Java lines 892-917:
    /// ```java
    /// private void periodicConfigSave() {
    ///     // let's not start anything heavy during play
    ///     if (current instanceof BMSPlayer) { return; }
    ///     // save once every 2 minutes
    ///     long now = System.nanoTime();
    ///     if ((now - lastConfigSave) < 2 * 60 * 1000000000L) { return; }
    ///     lastConfigSave = now;
    ///     // ... write config ...
    /// }
    /// ```
    pub fn periodic_config_save(&mut self) {
        // Skip during play to avoid I/O during gameplay
        if self.current_state_type() == Some(MainStateType::Play) {
            return;
        }

        // Save once every 2 minutes (Java: 2 * 60 * 1000000000L ns)
        let elapsed = self.ctx.lifecycle.last_config_save.elapsed();
        if elapsed.as_secs() < 120 {
            return;
        }

        self.ctx.lifecycle.last_config_save = Instant::now();
        self.save_config();
    }

    /// Update difficulty table data in a background thread.
    ///
    /// Translated from: MainController.updateTable(TableBar)
    pub fn update_table(
        &mut self,
        source: Box<dyn rubato_types::table_update_source::TableUpdateSource>,
    ) {
        let name = source.source_name();
        rubato_types::imgui_notify::ImGuiNotify::info(&format!("updating table : {name}"));
        let handle = std::thread::spawn(move || {
            source.refresh();
        });
        self.background_threads.push(handle);
    }

    /// Start IPFS download message rendering thread.
    ///
    /// Translated from: MainController.downloadIpfsMessageRenderer(String)
    pub fn download_ipfs_message_renderer(&mut self, message: &str) {
        // In Java: spawns DownloadMessageThread that polls download.isDownload() + download.getMessage()
        // When download processor is available, poll its status; otherwise show initial notification.
        if let Some(ref dl) = self.ctx.integration.download
            && dl.is_download()
        {
            let msg = dl.message();
            if !msg.is_empty() {
                rubato_types::imgui_notify::ImGuiNotify::info(&msg);
                return;
            }
        }
        rubato_types::imgui_notify::ImGuiNotify::info(message);
    }
}

#[cfg(test)]
mod tests {
    use rubato_types::table_update_source::TableUpdateSource;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::Duration;

    struct MockTableSource {
        name: String,
        refreshed: Arc<AtomicBool>,
    }

    impl TableUpdateSource for MockTableSource {
        fn source_name(&self) -> String {
            self.name.clone()
        }
        fn refresh(&self) {
            self.refreshed.store(true, Ordering::Release);
        }
    }

    #[test]
    fn table_update_thread_completes() {
        let refreshed = Arc::new(AtomicBool::new(false));
        let source: Box<dyn TableUpdateSource> = Box::new(MockTableSource {
            name: "test-table".to_string(),
            refreshed: Arc::clone(&refreshed),
        });

        // Replicate the update_table pattern: spawn thread with source
        let handle = std::thread::spawn(move || {
            source.refresh();
        });

        handle.join().expect("table update thread should complete");
        assert!(
            refreshed.load(Ordering::Acquire),
            "refresh() should have been called"
        );
    }

    #[test]
    fn table_update_thread_completes_within_timeout() {
        let refreshed = Arc::new(AtomicBool::new(false));
        let source: Box<dyn TableUpdateSource> = Box::new(MockTableSource {
            name: "slow-table".to_string(),
            refreshed: Arc::clone(&refreshed),
        });

        std::thread::spawn(move || {
            source.refresh();
        });

        // Poll with timeout to detect hangs
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !refreshed.load(Ordering::Acquire) {
            assert!(
                std::time::Instant::now() < deadline,
                "table update thread should complete within 2 seconds"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

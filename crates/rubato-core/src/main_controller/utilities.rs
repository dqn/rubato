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
    pub fn update_main_state_listener(&mut self, status: i32) {
        if let Some(ref current) = self.current {
            // Create adapter that bridges MainState → MainStateAccess
            let screen_type = current
                .state_type()
                .map(ScreenType::from)
                .unwrap_or(ScreenType::Other);
            let resource = self
                .resource
                .as_ref()
                .map(|r| r as &dyn PlayerResourceAccess);
            let adapter = StateAccessAdapter {
                screen_type,
                resource,
                config: &self.config,
            };

            // Temporarily take the listeners to avoid borrow conflict
            let mut listeners = std::mem::take(&mut self.state_listener);
            for listener in listeners.iter_mut() {
                listener.update(&adapter, status);
            }
            self.state_listener = listeners;
        }
    }

    pub fn play_time(&self) -> i64 {
        self.lifecycle.boottime.elapsed().as_millis() as i64
    }

    pub fn start_time(&self) -> i64 {
        self.timer.start_time()
    }

    pub fn start_micro_time(&self) -> i64 {
        self.timer.start_micro_time()
    }

    pub fn now_time(&self) -> i64 {
        self.timer.now_time()
    }

    pub fn now_time_for_id(&self, id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for_id(id)
    }

    pub fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }

    pub fn now_micro_time_for_id(&self, id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_micro_time_for_id(id)
    }

    pub fn timer_value(&self, id: rubato_types::timer_id::TimerId) -> i64 {
        self.micro_timer(id) / 1000
    }

    pub fn micro_timer(&self, id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(id)
    }

    pub fn is_timer_on(&self, id: rubato_types::timer_id::TimerId) -> bool {
        self.micro_timer(id) != i64::MIN
    }

    pub fn set_timer_on(&mut self, id: rubato_types::timer_id::TimerId) {
        self.timer.set_timer_on(id);
    }

    pub fn set_timer_off(&mut self, id: rubato_types::timer_id::TimerId) {
        self.set_micro_timer(id, i64::MIN);
    }

    pub fn set_micro_timer(&mut self, id: rubato_types::timer_id::TimerId, microtime: i64) {
        self.timer.set_micro_timer(id, microtime);
    }

    pub fn switch_timer(&mut self, id: rubato_types::timer_id::TimerId, on: bool) {
        self.timer.switch_timer(id, on);
    }

    pub fn http_download_processor(
        &self,
    ) -> Option<&dyn rubato_types::http_download_submitter::HttpDownloadSubmitter> {
        self.integration
            .http_download_processor
            .as_ref()
            .map(|processor| processor.as_ref())
    }

    pub fn clone_http_download_processor(
        &self,
    ) -> Option<std::sync::Arc<dyn rubato_types::http_download_submitter::HttpDownloadSubmitter>>
    {
        self.integration.http_download_processor.clone()
    }

    pub fn set_http_download_processor(
        &mut self,
        processor: Box<dyn rubato_types::http_download_submitter::HttpDownloadSubmitter>,
    ) {
        self.integration.http_download_processor = Some(std::sync::Arc::from(processor));
    }

    /// Start song database update.
    ///
    /// Translated from: MainController.updateSong(String)
    /// In Java, spawns SongUpdateThread calling songdb.updateSongDatas().
    /// Requires SongDatabaseAccessor trait to expose update_song_datas() — deferred.
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
        let update_path = if path.is_empty() { None } else { Some(path) };
        let bmsroot = self.config.paths.bmsroot.to_vec();
        if let Some(ref songdb) = self.db.songdb {
            songdb.update_song_datas(update_path, &bmsroot, false, update_parent_when_missing);
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
        self.db.songdb.as_deref()
    }

    /// Set the song database accessor.
    /// Called by the application entry point (beatoraja-launcher) after creating the DB.
    pub fn set_song_database(&mut self, songdb: Box<dyn SongDatabaseAccessorTrait>) {
        self.db.songdb = Some(songdb);
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
        self.input.as_ref()
    }

    /// Returns a mutable reference to the input processor.
    pub fn input_processor_mut(&mut self) -> Option<&mut BMSPlayerInputProcessor> {
        self.input.as_mut()
    }

    /// Returns the audio processor.
    ///
    /// Translated from: MainController.getAudioProcessor()
    pub fn audio_processor(&self) -> Option<&dyn AudioDriver> {
        self.audio.as_deref()
    }

    /// Returns a mutable reference to the audio processor.
    pub fn audio_processor_mut(&mut self) -> Option<&mut dyn AudioDriver> {
        self.audio
            .as_mut()
            .map(|b| &mut **b as &mut dyn AudioDriver)
    }

    /// Set the audio driver.
    ///
    /// Translated from: MainController constructor audio initialization
    ///
    /// In Java, the audio driver is created in create() based on AudioConfig.DriverType.
    /// In Rust, we inject it to avoid pulling in the concrete driver crate.
    pub fn set_audio_driver(&mut self, audio: Box<dyn AudioDriver>) {
        self.audio = Some(audio);
    }

    /// Returns the loudness analyzer.
    ///
    /// Translated from: MainController.loudnessAnalyzer
    pub fn loudness_analyzer(
        &self,
    ) -> Option<&rubato_audio::bms_loudness_analyzer::BMSLoudnessAnalyzer> {
        self.loudness_analyzer.as_ref()
    }

    /// Shutdown the loudness analyzer.
    ///
    /// Translated from: MainController.dispose() lines 864-866
    pub fn shutdown_loudness_analyzer(&mut self) {
        if let Some(ref analyzer) = self.loudness_analyzer {
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
        self.db.infodb.as_deref()
    }

    /// Set the song information database.
    /// Called from launcher layer since beatoraja-core cannot depend on beatoraja-song.
    pub fn set_info_database(&mut self, db: Box<dyn SongInformationDb>) {
        self.db.infodb = Some(db);
    }

    pub fn music_download_processor(
        &self,
    ) -> Option<&dyn rubato_types::music_download_access::MusicDownloadAccess> {
        self.integration.download.as_deref()
    }

    pub fn set_music_download_processor(
        &mut self,
        processor: Box<dyn rubato_types::music_download_access::MusicDownloadAccess>,
    ) {
        self.integration.download = Some(processor);
    }

    pub fn stream_controller(
        &self,
    ) -> Option<&dyn rubato_types::stream_controller_access::StreamControllerAccess> {
        self.integration.stream_controller.as_deref()
    }

    pub fn set_stream_controller(
        &mut self,
        controller: Box<dyn rubato_types::stream_controller_access::StreamControllerAccess>,
    ) {
        self.integration.stream_controller = Some(controller);
    }

    /// Gets the shared MusicSelector as `&dyn Any`. Callers downcast via
    /// `any.downcast_ref::<Arc<Mutex<MusicSelector>>>()`.
    /// Java: StreamController holds a reference to the same MusicSelector used by SelectState.
    pub fn shared_music_selector(&self) -> Option<&(dyn std::any::Any + Send)> {
        self.shared_music_selector.as_deref()
    }

    /// Sets the shared MusicSelector (type-erased as `Box<dyn Any + Send>`).
    pub fn set_shared_music_selector(&mut self, selector: Box<dyn std::any::Any + Send>) {
        self.shared_music_selector = Some(selector);
    }

    pub fn ir_resend_service(
        &self,
    ) -> Option<&dyn rubato_types::ir_resend_service::IrResendService> {
        self.integration.ir_resend_service.as_deref()
    }

    pub fn set_ir_resend_service(
        &mut self,
        service: Box<dyn rubato_types::ir_resend_service::IrResendService>,
    ) {
        self.integration.ir_resend_service = Some(service);
    }

    pub fn set_imgui(&mut self, imgui: Box<dyn rubato_types::imgui_access::ImGuiAccess>) {
        self.integration.imgui = Some(imgui);
    }

    /// Load a new player profile, re-initialize states and IR config.
    ///
    /// Translated from: MainController.loadNewProfile(PlayerConfig)
    pub fn load_new_profile(&mut self, pc: PlayerConfig) {
        self.config.playername = pc.id.clone();
        self.player = pc;

        // playdata = new PlayDataAccessor(config);
        self.initialize_ir_config();

        // Dispose current state before re-init
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

        self.lifecycle.last_config_save = Instant::now();
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
    /// In Rust, concrete state instances are created on-demand via the StateFactory
    /// (set by the launcher). This method only initializes the PlayerResource.
    /// States are created lazily in change_state().
    pub fn initialize_states(&mut self) {
        // In Java: resource = new PlayerResource(audio, config, player, loudnessAnalyzer);
        self.resource = Some(PlayerResource::new(
            self.config.clone(),
            self.player.clone(),
        ));

        // In Java: playdata = new PlayDataAccessor(config);
        self.db.playdata = Some(PlayDataAccessor::new(&self.config));

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
            callback.update_references(&self.config, &self.player);
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
        let ln_mode_name = match self.player.play_settings.lnmode {
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
        let mut targetlist: Vec<String> = self.player.select_settings.targetlist.clone();
        for i in 0..self.db.rivals.rival_count() {
            targetlist.push(format!("RIVAL_{}", i + 1));
        }

        // Resolve display names for each target ID
        let rivals: Vec<rubato_types::player_information::PlayerInformation> =
            (0..self.db.rivals.rival_count())
                .filter_map(|i| self.db.rivals.rival_information(i).cloned())
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
        let elapsed = self.lifecycle.last_config_save.elapsed();
        if elapsed.as_secs() < 120 {
            return;
        }

        self.lifecycle.last_config_save = Instant::now();
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
        std::thread::spawn(move || {
            source.refresh();
        });
    }

    /// Start IPFS download message rendering thread.
    ///
    /// Translated from: MainController.downloadIpfsMessageRenderer(String)
    pub fn download_ipfs_message_renderer(&mut self, message: &str) {
        // In Java: spawns DownloadMessageThread that polls download.isDownload() + download.getMessage()
        // When download processor is available, poll its status; otherwise show initial notification.
        if let Some(ref dl) = self.integration.download
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

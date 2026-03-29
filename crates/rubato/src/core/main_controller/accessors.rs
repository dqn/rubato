use super::*;

impl MainController {
    pub fn new(
        f: Option<PathBuf>,
        mut config: Config,
        player: PlayerConfig,
        auto: Option<BMSPlayerMode>,
        song_updated: bool,
    ) -> Self {
        let offset: Vec<SkinOffset> = (0..OFFSET_COUNT).map(|_| SkinOffset::default()).collect();

        // IPFS directory setup (Java: MainController constructor lines 161-170)
        if config.network.enable_ipfs {
            let ipfspath = std::path::Path::new("ipfs")
                .canonicalize()
                .unwrap_or_else(|_| std::env::current_dir().expect("current_dir").join("ipfs"));
            let _ = std::fs::create_dir_all(&ipfspath);
            if ipfspath.exists() {
                let ipfs_str = ipfspath.to_string_lossy().to_string();
                if !config.paths.bmsroot.contains(&ipfs_str) {
                    config.paths.bmsroot.push(ipfs_str);
                }
            }
        }

        // HTTP download directory setup (Java: MainController constructor lines 171-180)
        if config.network.enable_http {
            let httpdl_path = std::path::Path::new(&config.network.download_directory)
                .canonicalize()
                .unwrap_or_else(|_| {
                    std::env::current_dir()
                        .expect("current_dir")
                        .join(&config.network.download_directory)
                });
            let _ = std::fs::create_dir_all(&httpdl_path);
            if httpdl_path.exists() {
                let http_str = httpdl_path.to_string_lossy().to_string();
                if !config.paths.bmsroot.contains(&http_str) {
                    config.paths.bmsroot.push(http_str);
                }
            }
        }

        let timer = TimerManager::new();
        let sound = SystemSoundManager::new(
            Some(config.paths.bgmpath.as_str()),
            Some(config.paths.soundpath.as_str()),
        );

        // Java: playdata = new PlayDataAccessor(config);
        let playdata = Some(PlayDataAccessor::new(&config));

        // Phase 5+: IR initialization, Discord RPC, OBS listener
        #[allow(deprecated)]
        let state_listener: Vec<Box<dyn MainStateListener>> = Vec::new();

        // Create input processor
        let input = BMSPlayerInputProcessor::new(&config, &player);

        Self {
            ctx: GameContext {
                config,
                player,
                audio: None,
                sound: Some(sound),
                loudness_analyzer: Some(
                    crate::audio::bms_loudness_analyzer::BMSLoudnessAnalyzer::new(),
                ),
                timer,
                input: Some(input),
                input_poll_quit: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                db: DatabaseState {
                    playdata,
                    songdb: None,
                    infodb: None,
                    rivals: RivalDataAccessor::new(),
                    ircache: None,
                    ir: Vec::new(),
                },
                offset,
                showfps: false,
                debug: false,
                integration: IntegrationState::default(),
                lifecycle: LifecycleState::new(),
                exit_requested: AtomicBool::new(false),
                resource: None,
                transition: None,
                commands: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            },
            auto,
            song_updated,
            resource: None,
            current: None,
            state_factory: None,
            sprite: None,
            bmsfile: f,
            state_listener,
            event_senders: Vec::new(),
            shared_music_selector: None,
            state_references_callback: None,
            background_threads: Vec::new(),
            state_event_log: None,
            decide_skin_cache: None,
            preloaded_play_skin: None,
        }
    }

    pub fn offset(&self, index: i32) -> Option<&SkinOffset> {
        if index >= 0 && (index as usize) < self.ctx.offset.len() {
            Some(&self.ctx.offset[index as usize])
        } else {
            None
        }
    }

    pub fn offset_mut(&mut self, index: i32) -> Option<&mut SkinOffset> {
        if index >= 0 && (index as usize) < self.ctx.offset.len() {
            Some(&mut self.ctx.offset[index as usize])
        } else {
            None
        }
    }

    pub fn player_resource(&self) -> Option<&PlayerResource> {
        self.resource.as_ref()
    }

    pub fn player_resource_mut(&mut self) -> Option<&mut PlayerResource> {
        self.resource.as_mut()
    }

    /// Take the PlayerResource out of MainController (leaving None).
    /// Used by the StateCreator to give states ownership during their lifecycle.
    pub fn take_player_resource(&mut self) -> Option<PlayerResource> {
        self.resource.take()
    }

    /// Restore a PlayerResource previously taken via `take_player_resource()`.
    pub fn restore_player_resource(&mut self, resource: PlayerResource) {
        self.resource = Some(resource);
    }

    pub fn config(&self) -> &Config {
        &self.ctx.config
    }

    pub fn player_config(&self) -> &PlayerConfig {
        &self.ctx.player
    }

    pub fn sprite_batch(&self) -> Option<&SpriteBatch> {
        self.sprite.as_ref()
    }

    pub fn sprite_batch_mut(&mut self) -> Option<&mut SpriteBatch> {
        self.sprite.as_mut()
    }

    pub fn play_data_accessor(&self) -> Option<&PlayDataAccessor> {
        self.ctx.db.playdata.as_ref()
    }

    pub fn rival_data_accessor(&self) -> &RivalDataAccessor {
        &self.ctx.db.rivals
    }

    pub fn rival_data_accessor_mut(&mut self) -> &mut RivalDataAccessor {
        &mut self.ctx.db.rivals
    }

    pub fn ranking_data_cache(&self) -> Option<&dyn RankingDataCacheAccess> {
        self.ctx.db.ircache.as_deref()
    }

    pub fn ranking_data_cache_mut(
        &mut self,
    ) -> Option<&mut (dyn RankingDataCacheAccess + 'static)> {
        self.ctx.db.ircache.as_deref_mut()
    }

    pub fn set_ranking_data_cache(&mut self, cache: Box<dyn RankingDataCacheAccess>) {
        self.ctx.db.ircache = Some(cache);
    }

    pub fn sound_manager(&self) -> Option<&SystemSoundManager> {
        self.ctx.sound.as_ref()
    }

    pub fn sound_manager_mut(&mut self) -> Option<&mut SystemSoundManager> {
        self.ctx.sound.as_mut()
    }

    pub fn ir_status(&self) -> &[IRStatus] {
        &self.ctx.db.ir
    }

    pub fn ir_status_mut(&mut self) -> &mut Vec<IRStatus> {
        &mut self.ctx.db.ir
    }

    /// Get the first IR connection (concrete type).
    ///
    /// Returns the concrete `Arc` directly, avoiding downcast overhead.
    pub fn ir_connection(
        &self,
    ) -> Option<&std::sync::Arc<dyn crate::ir::ir_connection::IRConnection + Send + Sync>> {
        self.ctx
            .db
            .ir
            .first()
            .and_then(|status| status.connection.as_ref())
    }

    /// Get the shared command queue for wiring modmenu callbacks.
    pub fn command_queue(
        &self,
    ) -> &std::sync::Arc<std::sync::Mutex<Vec<crate::core::command::Command>>> {
        &self.ctx.commands
    }

    pub fn timer(&self) -> &TimerManager {
        &self.ctx.timer
    }

    pub fn timer_mut(&mut self) -> &mut TimerManager {
        &mut self.ctx.timer
    }

    pub fn has_obs_client(&self) -> bool {
        self.ctx.integration.obs_client.is_some()
    }

    pub fn set_obs_client(&mut self, client: Box<dyn crate::obs_access::ObsAccess>) {
        self.ctx.integration.obs_client = Some(client);
    }

    pub fn save_last_recording(&self, reason: &str) {
        if let Some(ref client) = self.ctx.integration.obs_client {
            client.save_last_recording(reason);
        }
    }

    /// Set the state creator. Must be called before any state transitions.
    ///
    /// The creator is typically set by the application entry point (beatoraja-launcher)
    /// which has access to all concrete state types.
    pub fn set_state_factory(&mut self, factory: StateCreator) {
        self.state_factory = Some(factory);
    }

    /// Add a state listener (e.g. DiscordListener, ObsListener).
    ///
    /// Translated from Java: stateListener.add(...)
    ///
    /// **Deprecated**: Use `add_event_sender()` with an `AppEvent` channel instead.
    #[deprecated(note = "Use add_event_sender() with an AppEvent channel instead")]
    #[allow(deprecated)]
    pub fn add_state_listener(&mut self, listener: Box<dyn MainStateListener>) {
        self.state_listener.push(listener);
    }

    /// Register a channel sender for receiving `AppEvent`s.
    ///
    /// Events are sent via `try_send` to avoid blocking the render thread.
    /// Disconnected senders are pruned automatically on each broadcast.
    pub fn add_event_sender(
        &mut self,
        sender: std::sync::mpsc::SyncSender<crate::skin::app_event::AppEvent>,
    ) {
        self.event_senders.push(sender);
    }

    /// Set the state event log for observability (E2E testing).
    ///
    /// When set, state machine events (transitions, lifecycle, handoffs) are
    /// pushed to the shared log so test harnesses can assert on them.
    ///
    /// **Deprecated**: Use `add_event_sender()` with an `AppEvent` channel instead.
    /// The channel delivers `AppEvent::Lifecycle(StateEvent)` for the same events.
    #[deprecated(
        note = "Use add_event_sender() with an AppEvent channel instead. The channel delivers AppEvent::Lifecycle(StateEvent)."
    )]
    pub fn set_state_event_log(
        &mut self,
        log: std::sync::Arc<std::sync::Mutex<Vec<crate::skin::state_event::StateEvent>>>,
    ) {
        self.state_event_log = Some(log);
    }

    /// Set the input gate time override for the next render() call.
    /// The override is consumed (taken) during render(), so it must be
    /// re-set before each call that needs deterministic input processing.
    pub fn set_input_gate_time_override(&mut self, time_ms: i64) {
        self.ctx.lifecycle.override_input_gate_time = Some(time_ms);
    }

    /// Return the current input gate `prevtime` (milliseconds).
    ///
    /// Used by the E2E harness to seed its monotonic input gate counter so
    /// that an existing controller's time is not regressed.
    pub fn input_gate_prevtime(&self) -> i64 {
        self.ctx.lifecycle.prevtime
    }

    /// Emit a state event to the event log and to all channel-based receivers.
    pub(super) fn emit_state_event(&self, event: crate::skin::state_event::StateEvent) {
        if let Some(ref log) = self.state_event_log
            && let Ok(mut guard) = log.lock()
        {
            guard.push(event.clone());
        }
        self.broadcast_app_event(crate::skin::app_event::AppEvent::Lifecycle(event));
    }

    /// Broadcast an `AppEvent` to all registered channel senders.
    ///
    /// Uses `try_send` to avoid blocking the render thread. Disconnected
    /// senders are silently ignored (pruned on next mutable access).
    pub(super) fn broadcast_app_event(&self, event: crate::skin::app_event::AppEvent) {
        for sender in &self.event_senders {
            let _ = sender.try_send(event.clone());
        }
    }

    /// Build and broadcast a `StateChanged` event using current controller state.
    pub(super) fn broadcast_state_changed(&self, status: i32) {
        if self.event_senders.is_empty() {
            return;
        }
        if let Some(ref current) = self.current {
            let screen_type = current
                .state_type()
                .map(ScreenType::from_state_type)
                .unwrap_or(ScreenType::Other);
            let state_type = current.state_type();

            let song_info = self.resource.as_ref().and_then(|r| r.songdata()).map(|sd| {
                crate::skin::app_event::SongInfo {
                    title: sd.metadata.title.clone(),
                    subtitle: sd.metadata.subtitle.clone(),
                    artist: sd.metadata.artist.clone(),
                    mode: sd.chart.mode,
                }
            });

            let data = crate::skin::app_event::StateChangedData {
                screen_type,
                state_type,
                status,
                song_info,
            };
            self.broadcast_app_event(crate::skin::app_event::AppEvent::StateChanged(data));
        }
    }
}

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
        let state_listener: Vec<Box<dyn MainStateListener>> = Vec::new();

        // Create input processor
        let input = BMSPlayerInputProcessor::new(&config, &player);

        Self {
            config,
            player,
            auto,
            song_updated,
            lifecycle: LifecycleState::new(),
            audio: None,
            resource: None,
            current: None,
            state_factory: None,
            timer,
            sprite: None,
            bmsfile: f,
            input: Some(input),
            input_poll_quit: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            showfps: false,
            db: DatabaseState {
                playdata,
                songdb: None,
                infodb: None,
                rivals: RivalDataAccessor::new(),
                ircache: None,
                ir: Vec::new(),
            },
            sound: Some(sound),
            offset,
            state_listener,
            command_queue: rubato_types::main_controller_access::MainControllerCommandQueue::new(),
            integration: IntegrationState::default(),
            shared_music_selector: None,
            state_references_callback: None,
            background_threads: Vec::new(),
            exit_requested: AtomicBool::new(false),
            debug: false,
            state_event_log: None,
            loudness_analyzer: Some(
                rubato_audio::bms_loudness_analyzer::BMSLoudnessAnalyzer::new(),
            ),
        }
    }

    pub fn offset(&self, index: i32) -> Option<&SkinOffset> {
        if index >= 0 && (index as usize) < self.offset.len() {
            Some(&self.offset[index as usize])
        } else {
            None
        }
    }

    pub fn offset_mut(&mut self, index: i32) -> Option<&mut SkinOffset> {
        if index >= 0 && (index as usize) < self.offset.len() {
            Some(&mut self.offset[index as usize])
        } else {
            None
        }
    }

    pub fn player_resource(&self) -> Option<&PlayerResource> {
        self.resource.as_ref()
    }

    /// Take the PlayerResource out of MainController (leaving None).
    /// Used by StateFactory to give states ownership during their lifecycle.
    pub fn take_player_resource(&mut self) -> Option<PlayerResource> {
        self.resource.take()
    }

    /// Restore a PlayerResource previously taken via `take_player_resource()`.
    pub fn restore_player_resource(&mut self, resource: PlayerResource) {
        self.resource = Some(resource);
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn player_config(&self) -> &PlayerConfig {
        &self.player
    }

    pub fn sprite_batch(&self) -> Option<&SpriteBatch> {
        self.sprite.as_ref()
    }

    pub fn sprite_batch_mut(&mut self) -> Option<&mut SpriteBatch> {
        self.sprite.as_mut()
    }

    pub fn play_data_accessor(&self) -> Option<&PlayDataAccessor> {
        self.db.playdata.as_ref()
    }

    pub fn rival_data_accessor(&self) -> &RivalDataAccessor {
        &self.db.rivals
    }

    pub fn rival_data_accessor_mut(&mut self) -> &mut RivalDataAccessor {
        &mut self.db.rivals
    }

    pub fn ranking_data_cache(&self) -> Option<&dyn RankingDataCacheAccess> {
        self.db.ircache.as_deref()
    }

    pub fn ranking_data_cache_mut(
        &mut self,
    ) -> Option<&mut (dyn RankingDataCacheAccess + 'static)> {
        self.db.ircache.as_deref_mut()
    }

    pub fn set_ranking_data_cache(&mut self, cache: Box<dyn RankingDataCacheAccess>) {
        self.db.ircache = Some(cache);
    }

    pub fn sound_manager(&self) -> Option<&SystemSoundManager> {
        self.sound.as_ref()
    }

    pub fn sound_manager_mut(&mut self) -> Option<&mut SystemSoundManager> {
        self.sound.as_mut()
    }

    pub fn ir_status(&self) -> &[IRStatus] {
        &self.db.ir
    }

    pub fn ir_status_mut(&mut self) -> &mut Vec<IRStatus> {
        &mut self.db.ir
    }

    /// Clone the shared controller command queue used by launcher-side state proxies.
    pub fn controller_command_queue(
        &self,
    ) -> rubato_types::main_controller_access::MainControllerCommandQueue {
        self.command_queue.clone()
    }

    pub fn timer(&self) -> &TimerManager {
        &self.timer
    }

    pub fn timer_mut(&mut self) -> &mut TimerManager {
        &mut self.timer
    }

    pub fn has_obs_client(&self) -> bool {
        self.integration.obs_client.is_some()
    }

    pub fn set_obs_client(&mut self, client: Box<dyn rubato_types::obs_access::ObsAccess>) {
        self.integration.obs_client = Some(client);
    }

    pub fn save_last_recording(&self, reason: &str) {
        if let Some(ref client) = self.integration.obs_client {
            client.save_last_recording(reason);
        }
    }

    /// Set the state factory. Must be called before any state transitions.
    ///
    /// The factory is typically set by the application entry point (beatoraja-launcher)
    /// which has access to all concrete state types.
    pub fn set_state_factory(&mut self, factory: Box<dyn StateFactory>) {
        self.state_factory = Some(factory);
    }

    /// Add a state listener (e.g. DiscordListener, ObsListener).
    ///
    /// Translated from Java: stateListener.add(...)
    pub fn add_state_listener(&mut self, listener: Box<dyn MainStateListener>) {
        self.state_listener.push(listener);
    }

    /// Set the state event log for observability (E2E testing).
    ///
    /// When set, state machine events (transitions, lifecycle, handoffs) are
    /// pushed to the shared log so test harnesses can assert on them.
    pub fn set_state_event_log(
        &mut self,
        log: std::sync::Arc<std::sync::Mutex<Vec<rubato_types::state_event::StateEvent>>>,
    ) {
        self.state_event_log = Some(log);
    }

    /// Set the input gate time override for the next render() call.
    /// The override is consumed (taken) during render(), so it must be
    /// re-set before each call that needs deterministic input processing.
    pub fn set_input_gate_time_override(&mut self, time_ms: i64) {
        self.lifecycle.override_input_gate_time = Some(time_ms);
    }

    /// Emit a state event to the log (if set). No-op when log is None.
    pub(super) fn emit_state_event(&self, event: rubato_types::state_event::StateEvent) {
        if let Some(ref log) = self.state_event_log
            && let Ok(mut guard) = log.lock()
        {
            guard.push(event);
        }
    }
}

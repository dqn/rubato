//! Subsystem initialization for the play path.
//!
//! Each function wires a specific subsystem (audio, IR, download processors, etc.)
//! into the MainController. Extracted from play() to keep the top-level orchestrator short.

use std::sync::Arc;

use anyhow::Result;
use log::{info, warn};

use rubato_core::main_controller::MainController;

use crate::{HttpDownloadProcessorWrapper, SongDbMainControllerRef, SongDbMusicDatabaseAdapter};

/// Initialize the song database from config paths.
/// Must be called before MainLoader::play() which calls take_score_database_accessor().
pub(crate) fn init_song_database() {
    use rubato_core::config::Config;
    use rubato_core::main_loader::MainLoader;
    use rubato_types::validatable::Validatable;

    let mut config = Config::read().unwrap_or_default();
    config.validate();
    if config.paths.bmsroot.is_empty() {
        warn!("No bmsroot configured - song scan will find nothing");
    }
    match rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
        &config.paths.songpath,
        &config.paths.bmsroot,
    ) {
        Ok(accessor) => {
            // Scan BMS files and populate song.db so the select screen has songs.
            // Java: MainLoader calls updateSongDatas() before creating the controller.
            info!("Scanning BMS files from configured paths...");
            accessor.update_song_datas(None, &config.paths.bmsroot, false, false, None);
            info!("Song database initialized: {}", &config.paths.songpath);
            MainLoader::set_score_database_accessor(Box::new(accessor));
        }
        Err(e) => {
            warn!(
                "Song database init failed: {}. Continuing without song DB.",
                e
            );
        }
    }
}

/// Wire the Kira-based audio driver so keysounds, BGM, and UI sounds work.
pub(crate) fn init_audio_driver(controller: &mut MainController) -> Result<()> {
    // Java: audio = new GdxSoundDriver(config.getSongResourceGen())
    let song_resource_gen = controller.config().render.song_resource_gen;
    let audio_driver = rubato_audio::gdx_sound_driver::GdxSoundDriver::new(song_resource_gen)?;
    controller.set_audio_driver(Box::new(audio_driver));
    Ok(())
}

/// Wire Discord RPC and OBS WebSocket state listeners.
pub(crate) fn init_state_listeners(controller: &mut MainController) {
    // Java: if(config.isUseDiscordRPC()) { stateListener.add(new DiscordListener()); }
    let (use_discord_rpc, use_obs_ws, cfg_clone) = {
        let cfg = controller.config();
        (
            cfg.integration.use_discord_rpc,
            cfg.obs.use_obs_ws,
            cfg.clone(),
        )
    };
    if use_discord_rpc {
        let listener = rubato_external::discord_listener::DiscordListener::new();
        controller.add_state_listener(Box::new(listener));
    }
    if use_obs_ws {
        let obs_client = rubato_external::obs::obs_ws_client::ObsWsClient::new(&cfg_clone);
        let listener = rubato_external::obs::obs_listener::ObsListener::new(cfg_clone);
        controller.add_state_listener(Box::new(listener));
        if let Ok(client) = obs_client {
            controller.set_obs_client(Box::new(client));
        }
    }
}

/// Wire IR (Internet Ranking) initialization at startup.
pub(crate) fn init_ir_config(controller: &mut MainController) {
    let player_config = controller.player_config().clone();
    let ir_statuses = rubato_state::result::ir_initializer::initialize_ir_config(&player_config);
    for ir_status in ir_statuses {
        let rival_provider = rubato_ir::ir_rival_provider_impl::IRRivalProviderImpl::new(
            ir_status.connection.clone(),
            ir_status.player.clone(),
            ir_status.config.irname.clone(),
            ir_status.config.importscore,
            ir_status.config.importrival,
        );
        controller
            .ir_status_mut()
            .push(rubato_core::main_controller::IRStatus {
                config: ir_status.config,
                rival_provider: Some(Box::new(rival_provider)),
                connection: Some(Box::new(ir_status.connection.clone())),
            });
    }
    // Wire IR resend service
    let ir_send_count = controller.config().network.ir_send_count;
    let resend_service = rubato_state::result::ir_resend::IrResendServiceImpl::new(ir_send_count);
    controller.set_ir_resend_service(Box::new(resend_service));
}

/// Initialize IPFS and HTTP download processors.
///
/// Java: MainController.create() lines 496-513 creates download processors.
/// Each processor runs on background threads and needs its own DB access, so we open
/// separate SQLite connections rather than sharing MainController's Box<dyn> songdb.
pub(crate) fn init_download_processors(controller: &mut MainController) {
    let config = controller.config().clone();

    // IPFS download processor (Java: lines 496-506)
    if config.network.enable_ipfs {
        init_ipfs_download_processor(controller, &config);
    }

    // HTTP download processor (Java: lines 508-513)
    if config.network.enable_http {
        init_http_download_processor(controller, &config);
    }
}

fn init_ipfs_download_processor(
    controller: &mut MainController,
    config: &rubato_core::config::Config,
) {
    match rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
        &config.paths.songpath,
        &config.paths.bmsroot,
    ) {
        Ok(songdb) => {
            let adapter = Arc::new(SongDbMusicDatabaseAdapter { songdb });
            let processor =
                rubato_song::md_processor::music_download_processor::MusicDownloadProcessor::new(
                    config.network.ipfsurl.clone(),
                    adapter,
                );
            processor.start(None);
            controller.set_music_download_processor(Box::new(processor));
            info!("IPFS MusicDownloadProcessor initialized");
        }
        Err(e) => {
            warn!(
                "Cannot initialize MusicDownloadProcessor: song DB open failed: {}",
                e
            );
        }
    }
}

fn init_http_download_processor(
    controller: &mut MainController,
    config: &rubato_core::config::Config,
) {
    // Look up download source by config.network.download_source, fall back to default
    let source_meta = rubato_song::md_processor::http_download_processor::DOWNLOAD_SOURCES
        .get(&config.network.download_source)
        .copied()
        .unwrap_or_else(|| {
            rubato_song::md_processor::http_download_processor::HttpDownloadProcessor::default_download_source()
        });
    let http_download_source: Arc<
        dyn rubato_song::md_processor::http_download_source::HttpDownloadSource,
    > = Arc::from(source_meta.build(config));

    // The MainControllerRef adapter opens its own song DB connection so the background
    // download thread can call update_song() without borrowing MainController.
    match rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
        &config.paths.songpath,
        &config.paths.bmsroot,
    ) {
        Ok(songdb) => {
            let bmsroot = config.paths.bmsroot.clone();
            let main_ref: Arc<dyn rubato_song::md_processor::MainControllerRef> =
                Arc::new(SongDbMainControllerRef { songdb, bmsroot });
            let processor = Arc::new(
                rubato_song::md_processor::http_download_processor::HttpDownloadProcessor::new(
                    main_ref,
                    http_download_source,
                    config.network.download_directory.clone(),
                ),
            );

            // Java: DownloadTaskState.initialize(httpDownloadProcessor)
            rubato_song::md_processor::download_task_state::DownloadTaskState::initialize();
            // Java: DownloadTaskMenu.setProcessor(httpDownloadProcessor)
            rubato_state::modmenu::download_task_menu::DownloadTaskMenu::set_processor(Arc::clone(
                &processor,
            ));

            controller.set_http_download_processor(Box::new(HttpDownloadProcessorWrapper(
                Arc::clone(&processor),
            )));
            info!(
                "HTTP HttpDownloadProcessor initialized (source: {})",
                config.network.download_source
            );
        }
        Err(e) => {
            warn!(
                "Cannot initialize HttpDownloadProcessor: song DB open failed: {}",
                e
            );
        }
    }
}

/// Initialize the stream controller for request-mode song streaming.
///
/// Java: MainController.initializeStates() lines 561-564:
///   if(player.getRequestEnable()) {
///       streamController = new StreamController(selector);
///       streamController.run();
///   }
pub(crate) fn init_stream_controller(controller: &mut MainController) {
    if !controller.player_config().enable_request {
        return;
    }

    let config = controller.config();
    let mut selector =
        match rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
            &config.paths.songpath,
            &config.paths.bmsroot,
        ) {
            Ok(db) => rubato_state::select::music_selector::MusicSelector::with_song_database(
                Box::new(db),
            ),
            Err(e) => {
                log::warn!(
                    "Failed to open song database for shared MusicSelector: {}",
                    e
                );
                rubato_state::select::music_selector::MusicSelector::with_config(config.clone())
            }
        };
    // Wire dependencies so the shared selector can access config, sounds, scores, etc.
    {
        selector.set_main_controller(
            rubato_launcher::state_factory::new_state_main_controller_access(controller),
        );
        selector.config = controller.player_config().clone();
    }
    let selector = std::sync::Arc::new(std::sync::Mutex::new(selector));
    // Store the shared selector on MainController for StateFactory to retrieve
    controller.set_shared_music_selector(Box::new(std::sync::Arc::clone(&selector)));
    let mut stream_controller =
        rubato_state::stream::stream_controller::StreamController::new(selector);
    stream_controller.run();
    controller.set_stream_controller(Box::new(stream_controller));
}

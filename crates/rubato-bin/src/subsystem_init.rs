//! Subsystem initialization for the play path.
//!
//! Each function wires a specific subsystem (audio, IR, download processors, etc.)
//! into the MainController. Extracted from play() to keep the top-level orchestrator short.

use std::sync::Arc;

use anyhow::Result;
use log::{info, warn};

use rubato_game::core::main_controller::MainController;
use rubato_types::ir_resend_service::IrResendService;

use crate::{HttpDownloadProcessorWrapper, SongDbMainControllerRef, SongDbMusicDatabaseAdapter};

/// Initialize the song database from config paths.
/// Must be called before MainLoader::play() which calls take_score_database_accessor().
pub(crate) fn init_song_database() {
    init_song_database_impl(false, true);
}

/// Initialize the song database with explicit update_all flag.
/// Called from the launcher for Load All BMS / Load Diff BMS actions.
pub(crate) fn init_song_database_with_options(update_all: bool) {
    init_song_database_impl(update_all, false);
}

/// Initialize the song information database on MainController.
///
/// The select screen reads main BPM and density data through MainControllerAccess,
/// so the controller must hold a live SongInformationDb before any queued/select
/// state proxies are created.
pub(crate) fn init_song_information_database(controller: &mut MainController) {
    if controller.info_database().is_some() {
        return;
    }

    let songinfo_path = controller.config().paths.songinfopath.clone();
    match rubato_game::song::song_information_accessor::SongInformationAccessor::new(&songinfo_path)
    {
        Ok(db) => {
            controller.set_info_database(Box::new(db));
            info!("Song information database initialized: {}", songinfo_path);
        }
        Err(e) => {
            warn!(
                "Song information database init failed: {}. Continuing without song info DB.",
                e
            );
        }
    }
}

fn init_song_database_impl(update_all: bool, set_accessor: bool) {
    use rubato_game::core::config::Config;
    use rubato_game::core::main_loader::MainLoader;
    use rubato_types::validatable::Validatable;

    let mut config = Config::read().unwrap_or_default();
    config.validate();
    if config.paths.bmsroot.is_empty() {
        warn!("No bmsroot configured - song scan will find nothing");
    }
    match rubato_game::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
        &config.paths.songpath,
        &config.paths.bmsroot,
    ) {
        Ok(accessor) => {
            // Known limitation: runs a full scan on every launch. A background or
            // on-demand scan would reduce startup time for large libraries.
            // Java: MainLoader calls updateSongDatas() before creating the controller.
            info!("Scanning BMS files from configured paths...");
            accessor.update_song_datas(None, &config.paths.bmsroot, update_all, false, None);
            info!("Song database initialized: {}", &config.paths.songpath);
            if set_accessor {
                MainLoader::set_score_database_accessor(Box::new(accessor));
            }
        }
        Err(e) => {
            warn!(
                "Song database init failed: {}. Continuing without song DB.",
                e
            );
        }
    }
}

/// Import scores from LR2 score database.
/// Shows a file chooser dialog and imports the selected LR2 score.db.
pub(crate) fn import_lr2_scores(config: &rubato_game::core::config::Config) {
    let lr2_path = match rubato_game::platform::show_file_chooser("Select LR2 score database") {
        Some(p) => p,
        None => {
            info!("Import Score cancelled - no file selected.");
            return;
        }
    };

    let player_name = config.playername.as_deref().unwrap_or("default");
    let sep = std::path::MAIN_SEPARATOR;
    let score_db_path = format!(
        "{}{sep}{}{sep}score.db",
        &config.paths.playerpath, player_name
    );

    let scoredb = match rubato_game::core::score_database_accessor::ScoreDatabaseAccessor::new(
        &score_db_path,
    ) {
        Ok(db) => db,
        Err(e) => {
            warn!("Failed to open score database {}: {}", score_db_path, e);
            return;
        }
    };

    let songdb =
        match rubato_game::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
            &config.paths.songpath,
            &config.paths.bmsroot,
        ) {
            Ok(db) => db,
            Err(e) => {
                warn!("Failed to open song database: {}", e);
                return;
            }
        };

    info!("Importing scores from LR2 database: {}", lr2_path);
    let importer = rubato_game::external::score_data_importer::ScoreDataImporter::new(scoredb);
    importer.import_from_lr2_score_database(&lr2_path, &songdb);
    info!("LR2 score import complete.");
}

/// Wire the Kira-based audio driver so keysounds, BGM, and UI sounds work.
pub(crate) fn init_audio_driver(controller: &mut MainController) -> Result<()> {
    // Known limitation: audio driver selection from config is not yet implemented.
    // Always uses GdxSoundDriver regardless of config.audio.driver setting.
    let song_resource_gen = controller.config().render.song_resource_gen;
    let audio_driver = rubato_audio::gdx_sound_driver::GdxSoundDriver::new(song_resource_gen)?;
    controller.set_audio_driver(rubato_audio::audio_system::AudioSystem::GdxSound(
        audio_driver,
    ));
    Ok(())
}

/// Wire Discord RPC and OBS WebSocket state listeners.
///
/// Returns a Vec of listener handles that must be kept alive for the
/// background threads to continue running. Dropping the returned vec
/// will shut down the listeners gracefully.
pub(crate) fn init_state_listeners(controller: &mut MainController) -> Vec<Box<dyn std::any::Any>> {
    let mut handles: Vec<Box<dyn std::any::Any>> = Vec::new();

    let (use_discord_rpc, use_obs_ws, cfg_clone) = {
        let cfg = controller.config();
        (
            cfg.integration.use_discord_rpc,
            cfg.obs.use_obs_ws,
            cfg.clone(),
        )
    };
    if use_discord_rpc {
        let (sender, listener) = rubato_game::external::discord_listener::DiscordListener::new();
        controller.add_event_sender(sender);
        handles.push(Box::new(listener));
    }
    if use_obs_ws {
        let obs_client = rubato_game::external::obs::obs_ws_client::ObsWsClient::new(&cfg_clone);
        let (sender, listener) =
            rubato_game::external::obs::obs_listener::ObsListener::new(cfg_clone);
        controller.add_event_sender(sender);
        handles.push(Box::new(listener));
        if let Ok(client) = obs_client {
            controller.set_obs_client(Box::new(client));
        }
    }
    handles
}

/// Wire IR (Internet Ranking) initialization at startup.
pub(crate) fn init_ir_config(controller: &mut MainController) {
    // Register the LR2IR connection so IRConnectionManager can find it
    rubato_game::ir::ir_connection_manager::register_ir_connections(vec![
        rubato_game::ir::ir_connection_manager::IRConnectionEntry {
            name: rubato_game::ir::lr2_ir_connection_adapter::LR2IR_NAME.to_string(),
            home: Some("http://www.dream-pro.info/~lavalse/LR2IR/".to_string()),
            factory: Box::new(|| {
                Box::new(rubato_game::ir::lr2_ir_connection_adapter::LR2IRConnectionAdapter::new())
            }),
        },
    ]);

    let player_config = controller.player_config().clone();
    let ir_statuses =
        rubato_game::state::result::ir_initializer::initialize_ir_config(&player_config);
    for ir_status in ir_statuses {
        let rival_provider = rubato_game::ir::ir_rival_provider_impl::IRRivalProviderImpl::new(
            ir_status.connection.clone(),
            ir_status.player.clone(),
            ir_status.config.irname.clone(),
            ir_status.config.importscore,
            ir_status.config.importrival,
        );
        controller
            .ir_status_mut()
            .push(rubato_game::core::main_controller::IRStatus {
                config: ir_status.config,
                rival_provider: Some(Box::new(rival_provider)),
                connection: Some(ir_status.connection.clone()),
                player_data: Some(ir_status.player.clone()),
            });
    }
    // Wire IR resend service
    let ir_send_count = controller.config().network.ir_send_count;
    let resend_service =
        rubato_game::state::result::ir_resend::IrResendServiceImpl::new(ir_send_count);
    resend_service.start();
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
    config: &rubato_game::core::config::Config,
) {
    match rubato_game::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
        &config.paths.songpath,
        &config.paths.bmsroot,
    ) {
        Ok(songdb) => {
            let adapter = Arc::new(SongDbMusicDatabaseAdapter {
                songdb,
                bmsroot: config.paths.bmsroot.clone(),
            });
            let processor =
                rubato_game::song::md_processor::music_download_processor::MusicDownloadProcessor::new(
                    config.network.ipfsurl.clone(),
                    adapter,
                    std::path::PathBuf::from(&config.network.download_directory),
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
    config: &rubato_game::core::config::Config,
) {
    // Look up download source by config.network.download_source, fall back to default
    let source_meta = rubato_game::song::md_processor::http_download_processor::DOWNLOAD_SOURCES
        .get(&config.network.download_source)
        .copied()
        .unwrap_or_else(|| {
            rubato_game::song::md_processor::http_download_processor::HttpDownloadProcessor::default_download_source()
        });
    let http_download_source: Arc<
        dyn rubato_game::song::md_processor::http_download_source::HttpDownloadSource,
    > = Arc::from(source_meta.build(config));

    // The MainControllerRef adapter opens its own song DB connection so the background
    // download thread can call update_song() without borrowing MainController.
    match rubato_game::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
        &config.paths.songpath,
        &config.paths.bmsroot,
    ) {
        Ok(songdb) => {
            let bmsroot = config.paths.bmsroot.clone();
            let info_db: Option<Box<dyn rubato_types::song_information_db::SongInformationDb>> =
                rubato_game::song::song_information_accessor::SongInformationAccessor::new(
                    &config.paths.songinfopath,
                )
                .ok()
                .map(|db| {
                    Box::new(db) as Box<dyn rubato_types::song_information_db::SongInformationDb>
                });
            let main_ref: Arc<dyn rubato_game::song::md_processor::MainControllerRef> =
                Arc::new(SongDbMainControllerRef {
                    songdb,
                    bmsroot,
                    info_db,
                });
            let processor = Arc::new(
                rubato_game::song::md_processor::http_download_processor::HttpDownloadProcessor::new(
                    main_ref,
                    http_download_source,
                    config.network.download_directory.clone(),
                ),
            );

            // Java: DownloadTaskState.initialize(httpDownloadProcessor)
            rubato_game::song::md_processor::download_task_state::DownloadTaskState::initialize();
            // Java: DownloadTaskMenu.setProcessor(httpDownloadProcessor)
            rubato_game::state::modmenu::download_task_menu::DownloadTaskMenu::set_processor(
                Arc::clone(&processor),
            );

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

    let config = controller.config().clone();
    let mut selector =
        match rubato_game::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
            &config.paths.songpath,
            &config.paths.bmsroot,
        ) {
            Ok(db) => {
                rubato_game::state::select::music_selector::MusicSelector::with_song_database(
                    Box::new(db),
                )
            }
            Err(e) => {
                log::warn!(
                    "Failed to open song database for shared MusicSelector: {}",
                    e
                );
                rubato_game::state::select::music_selector::MusicSelector::with_config(
                    config.clone(),
                )
            }
        };
    // Wire dependencies so the shared selector can access config, sounds, scores, etc.
    {
        selector.set_main_controller(
            rubato_game::state_factory::new_state_main_controller_access(controller),
        );
        selector.config = controller.player_config().clone();
        selector.app_config = config;
    }
    let selector = std::sync::Arc::new(std::sync::Mutex::new(selector));
    // Store the shared selector on MainController for the StateCreator to retrieve
    controller.set_shared_music_selector(std::sync::Arc::clone(&selector));
    let mut stream_controller =
        rubato_game::state::stream::stream_controller::StreamController::new(selector);
    stream_controller.run();
    controller.set_stream_controller(Box::new(stream_controller));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_game::core::config::Config;
    use rubato_game::core::player_config::PlayerConfig;

    #[test]
    fn init_song_information_database_sets_controller_info_database() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let info_db_path = tempdir.path().join("songinfo.db");
        let mut config = Config::default();
        config.paths.songinfopath = info_db_path.to_string_lossy().to_string();
        let player = PlayerConfig::default();
        let mut controller = MainController::new(None, config, player, None, false);

        assert!(
            controller.info_database().is_none(),
            "controller should start without a song info database in this isolated test"
        );

        init_song_information_database(&mut controller);

        assert!(
            controller.info_database().is_some(),
            "play initialization should wire the song information database into MainController"
        );
    }
}

//! brs — main binary for the BMS rhythm game player.
//!
//! Integrates all workspace crates into a Bevy application with a state machine
//! driving MusicSelect → Decide → Play → Result screen transitions.
//! CLI arguments are parsed via clap, and Bevy resources wrap each subsystem
//! (config, database, input, audio, skin, external integrations).

mod app_state;
mod bevy_keyboard;
pub mod database_manager;
pub mod external_manager;
mod game_state;
mod hot_reload;
pub mod input_mapper;
mod player_resource;
mod preview_music;
mod skin_manager;
mod state;
mod system_sound;
mod table_updater;
mod target_property;
mod timer_manager;
mod window_manager;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bms_external::version_check::VersionStatus;
use parking_lot::{Mutex, RwLock};

use anyhow::Result;
use bevy::input::ButtonInput;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::prelude::*;
use clap::Parser;
use tracing::info;

use app_state::{AppStateType, StateRegistry, TickParams};
use database_manager::DatabaseManager;
use external_manager::ExternalManager;
use game_state::{SharedGameState, sync_timer_state};
use input_mapper::InputMapper;
use player_resource::{PlayerMode, PlayerResource};
use state::course_result::CourseResultState;
use state::decide::MusicDecideState;
use state::key_config::KeyConfigState;
use state::play::PlayState;
use state::result::ResultState;
use state::select::MusicSelectState;
use state::skin_config::SkinConfigState;
use state::{DownloadHandle, DownloadSourceKind};
use timer_manager::TimerManager;

#[derive(Parser, Debug)]
#[command(name = "brs", about = "BMS player (Rust port of lr2oraja)")]
struct Args {
    /// Path to a BMS file to play directly (skips MusicSelect).
    #[arg(long)]
    bms: Option<PathBuf>,

    /// Path to database directory.
    #[arg(long, default_value = "db")]
    db_path: PathBuf,

    /// Path to system config JSON file.
    #[arg(long, default_value = "config_sys.json")]
    config: PathBuf,

    /// Path to player config JSON file.
    #[arg(long, default_value = "config_player.json")]
    player_config: PathBuf,

    /// Skip the launcher GUI and start the game directly.
    #[arg(long)]
    no_launcher: bool,

    /// Autoplay mode.
    #[arg(short = 'a', long)]
    autoplay: bool,

    /// Practice mode.
    #[arg(short = 'p', long)]
    practice: bool,

    /// Replay mode with slot number (0-3).
    #[arg(short = 'r', long, value_name = "SLOT", value_parser = clap::value_parser!(u8).range(0..=3))]
    replay: Option<u8>,

    /// Normal play mode (default, explicit flag for scripting).
    #[arg(short = 's', long)]
    play: bool,

    /// BMS file path (positional alternative to --bms).
    #[arg(value_name = "BMS_PATH")]
    bms_positional: Option<PathBuf>,

    /// Internal: run launcher in subprocess and exit.
    #[arg(long, hide = true)]
    launcher_only: bool,

    /// Run without rendering (headless mode for testing/benchmarking).
    #[arg(long)]
    headless: bool,

    /// Exit after the Result screen completes (for E2E testing).
    #[arg(long)]
    exit_after_result: bool,
}

impl Args {
    /// Resolve the player mode from CLI flags.
    /// Priority: autoplay > practice > replay > play (default).
    fn resolve_mode(&self) -> PlayerMode {
        if self.autoplay {
            PlayerMode::Autoplay
        } else if self.practice {
            PlayerMode::Practice
        } else if let Some(slot) = self.replay {
            PlayerMode::Replay(slot)
        } else {
            PlayerMode::Play
        }
    }

    /// Resolve BMS path from --bms flag or positional argument.
    /// --bms takes priority over positional.
    fn resolve_bms_path(&self) -> Option<&PathBuf> {
        self.bms.as_ref().or(self.bms_positional.as_ref())
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    info!("brs starting");

    // Subprocess mode: run launcher and exit with status code
    if args.launcher_only {
        return run_launcher_subprocess(&args.config, &args.player_config);
    }

    let player_mode = args.resolve_mode();
    let bms_path = args.resolve_bms_path().cloned();

    // Launch settings GUI in a subprocess to avoid macOS event loop conflict.
    // macOS winit forbids creating a second event loop in the same process;
    // running eframe (launcher) then Bevy would trigger RecreationAttempt panic.
    if !args.no_launcher && bms_path.is_none() {
        let exe = std::env::current_exe()?;
        let status = std::process::Command::new(&exe)
            .arg("--launcher-only")
            .arg("--config")
            .arg(&args.config)
            .arg("--player-config")
            .arg(&args.player_config)
            .status()?;
        if !status.success() {
            info!("Launcher: user cancelled");
            return Ok(());
        }
        info!("Launcher: user clicked Start Game");
    }

    // Load BMS if specified
    let mut resource = PlayerResource {
        player_mode,
        exit_after_result: args.exit_after_result,
        ..Default::default()
    };
    if let Some(bms_path) = &bms_path {
        info!(path = %bms_path.display(), "Loading BMS file");
        let model = bms_model::BmsDecoder::decode(bms_path)?;
        resource.play_mode = model.mode;
        resource.bms_dir = bms_path.parent().map(|p| p.to_path_buf());
        resource.bms_model = Some(model);
    }

    // Load config from file, falling back to defaults if not found
    let config = match bms_config::Config::read(&args.config) {
        Ok(c) => {
            info!(path = %args.config.display(), "Loaded system config");
            c
        }
        Err(_) => {
            info!(
                path = %args.config.display(),
                "Config not found, using defaults"
            );
            bms_config::Config::default()
        }
    };
    let player_config = match bms_config::PlayerConfig::read(&args.player_config) {
        Ok(c) => {
            info!(path = %args.player_config.display(), "Loaded player config");
            c
        }
        Err(_) => {
            info!(
                path = %args.player_config.display(),
                "PlayerConfig not found, using defaults"
            );
            bms_config::PlayerConfig::default()
        }
    };

    // Spawn background version check (non-blocking)
    let version_rx = {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match rt {
                Ok(rt) => {
                    let status = rt.block_on(bms_external::version_check::check_latest_version(
                        env!("CARGO_PKG_VERSION"),
                        None,
                    ));
                    let _ = tx.send(status);
                }
                Err(e) => {
                    let _ = tx.send(VersionStatus::CheckFailed(format!(
                        "Failed to create runtime: {e}"
                    )));
                }
            }
        });
        rx
    };

    // Update last_booted_version in config
    let mut config = config;
    config.last_booted_version = env!("CARGO_PKG_VERSION").to_string();

    // Open databases
    let database = match DatabaseManager::open(&args.db_path) {
        Ok(db) => {
            info!(path = %args.db_path.display(), "Database opened");

            // Scan BMS root directories and update song database
            if !config.bmsroot.is_empty() {
                let update_all = config.updatesong;
                match db
                    .song_db
                    .update_song_datas(None, &config.bmsroot, update_all)
                {
                    Ok(stats) => {
                        info!(
                            scanned = stats.scanned,
                            added = stats.added,
                            updated = stats.updated,
                            removed = stats.removed,
                            "Song database updated"
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to update song database: {e}");
                    }
                }
            }

            Some(db)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to open database at {}: {}",
                args.db_path.display(),
                e
            );
            None
        }
    };

    // Initialize download processor from config
    let download_handle = if config.enable_http || config.enable_ipfs {
        let download_dir = PathBuf::from(&config.download_directory);
        if let Err(e) = std::fs::create_dir_all(&download_dir) {
            tracing::warn!(
                path = %download_dir.display(),
                "Failed to create download directory: {e}"
            );
        }
        let processor = Arc::new(bms_download::processor::HttpDownloadProcessor::new(
            &download_dir,
        ));
        let override_url = if config.override_download_url.is_empty() {
            None
        } else {
            Some(config.override_download_url.as_str())
        };
        let source = match config.download_source.as_str() {
            "wriggle" => DownloadSourceKind::Wriggle(
                bms_download::source::wriggle::WriggleDownloadSource::new(override_url),
            ),
            _ => DownloadSourceKind::Konmai(
                bms_download::source::konmai::KonmaiDownloadSource::new(override_url),
            ),
        };
        info!(
            enable_http = config.enable_http,
            enable_ipfs = config.enable_ipfs,
            source = config.download_source.as_str(),
            dir = %download_dir.display(),
            "Download processor initialized"
        );
        Some(Arc::new(DownloadHandle {
            processor,
            source,
            ipfs_gateway: config.ipfsurl.clone(),
            enable_http: config.enable_http,
            enable_ipfs: config.enable_ipfs,
        }))
    } else {
        None
    };

    // Build state registry
    let mut registry = StateRegistry::new(AppStateType::MusicSelect);
    registry.register(AppStateType::MusicSelect, Box::new(MusicSelectState::new()));
    registry.register(AppStateType::Decide, Box::new(MusicDecideState::new()));
    registry.register(AppStateType::Play, Box::new(PlayState::new()));
    registry.register(AppStateType::Result, Box::new(ResultState::new()));
    registry.register(
        AppStateType::CourseResult,
        Box::new(CourseResultState::new()),
    );
    registry.register(AppStateType::KeyConfig, Box::new(KeyConfigState::new()));
    registry.register(AppStateType::SkinConfig, Box::new(SkinConfigState::new()));

    // Initialize external integrations from config
    let external = ExternalManager::new(&config, &player_config);
    info!(
        discord = external.is_discord_enabled(),
        obs = external.is_obs_enabled(),
        stream = external.is_stream_enabled(),
        "External integrations initialized"
    );

    // Window size from config
    let window_width = config.window_width as f32;
    let window_height = config.window_height as f32;

    // Shared game state for skin renderer
    let shared_state = Arc::new(RwLock::new(SharedGameState::default()));

    // Preview music processor for select screen BGM/preview playback
    let preview_music = match preview_music::PreviewMusicProcessor::new() {
        Ok(pm) => {
            info!("Preview music processor initialized");
            Some(pm)
        }
        Err(e) => {
            tracing::warn!("Failed to initialize preview music: {e}");
            None
        }
    };

    // Store config paths for saving on exit
    let config_path = args.config.clone();
    let player_config_path = args.player_config.clone();

    // Initialize system sounds from config
    let mut system_sound_mgr = system_sound::SystemSoundManager::default();
    system_sound_mgr.load_sounds(Path::new(&config.soundpath));
    system_sound_mgr.set_volume(config.audio.systemvolume as f64);

    // Headless mode: run state machine without Bevy rendering
    if args.headless {
        return run_headless(registry, resource, config, player_config, database.as_ref());
    }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "brs".to_string(),
                resolution: bevy::window::WindowResolution::new(window_width, window_height),
                mode: window_manager::display_mode_to_window_mode(
                    config.displaymode,
                    bevy::window::MonitorSelection::Current,
                ),
                present_mode: window_manager::vsync_to_present_mode(config.vsync),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bms_render::plugin::BmsRenderPlugin)
        .insert_resource(BrsTimerManager(TimerManager::new()))
        .insert_resource(BrsPlayerResource(resource))
        .insert_resource(BrsConfig(config))
        .insert_resource(BrsPlayerConfig(player_config))
        .insert_resource(BrsStateRegistry {
            registry,
            shared_state: Arc::clone(&shared_state),
        })
        .insert_resource(BrsSharedState(shared_state))
        .insert_resource(BrsDatabase(Arc::new(Mutex::new(database))))
        .insert_resource(BrsInputMapper(InputMapper::new()))
        .insert_resource(BrsExternalManager(external))
        .insert_resource(BrsSkinManager::default())
        .insert_resource(BrsSystemSoundManager(system_sound_mgr))
        .insert_resource(StateUiResources {
            config_paths: BrsConfigPaths {
                config: config_path,
                player_config: player_config_path,
            },
            preview_music: BrsPreviewMusic(Mutex::new(preview_music)),
            download_handle: BrsDownloadHandle(download_handle),
        })
        .insert_resource(BrsVersionCheck {
            rx: Mutex::new(Some(version_rx)),
            status: None,
        })
        .add_systems(
            Update,
            (timer_update_system, state_machine_system, state_sync_system)
                .chain()
                .before(skin_load_system),
        )
        .add_systems(
            Update,
            (skin_load_system, bevy::ecs::schedule::apply_deferred)
                .chain()
                .before(bms_render::skin_renderer::skin_render_system),
        )
        .add_systems(Update, exit_check_system)
        .add_systems(Update, preview_music_update_system)
        .add_systems(Update, version_check_system)
        .add_systems(Update, hot_reload::hot_reload_system)
        .add_systems(Update, window_manager::window_shortcut_system)
        .add_systems(Update, window_manager::apply_window_settings_system)
        .add_systems(PostStartup, window_manager::apply_monitor_selection_system)
        .run();

    Ok(())
}

/// Run the game in headless mode (no rendering, no audio).
///
/// Uses a manual game loop with `TimerManager::update()` for wall-clock time.
/// Suitable for E2E testing and benchmarking without GPU.
fn run_headless(
    mut registry: StateRegistry,
    mut resource: PlayerResource,
    config: bms_config::Config,
    mut player_config: bms_config::PlayerConfig,
    database: Option<&DatabaseManager>,
) -> Result<()> {
    info!("Running in headless mode");

    let mut timer = TimerManager::new();

    loop {
        timer.update();
        let mut params = TickParams {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            keyboard_backend: None,
            database,
            input_state: None,
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
            download_handle: None,
        };
        registry.tick(&mut params);

        if resource.request_app_exit {
            info!("Headless: exit after result, shutting down");
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    Ok(())
}

/// Run the launcher GUI and exit with a status code.
/// Called in `--launcher-only` subprocess mode.
///   exit 0 = Start Game
///   exit 1 = Cancel
///   exit 2 = Error
fn run_launcher_subprocess(config_path: &Path, player_config_path: &Path) -> Result<()> {
    match bms_launcher::run_launcher(config_path, player_config_path) {
        Ok(Some(_)) => std::process::exit(0),
        Ok(None) => std::process::exit(1),
        Err(e) => {
            tracing::warn!("Launcher failed: {e}");
            std::process::exit(2);
        }
    }
}

// Bevy resource wrappers (newtype to satisfy Resource trait)

#[derive(Resource)]
struct BrsTimerManager(TimerManager);

#[derive(Resource)]
struct BrsPlayerResource(PlayerResource);

#[derive(Resource)]
struct BrsConfig(bms_config::Config);

#[derive(Resource)]
struct BrsPlayerConfig(bms_config::PlayerConfig);

#[derive(Resource)]
struct BrsStateRegistry {
    registry: StateRegistry,
    /// Arc clone for state_machine_system to access shared state without
    /// an extra Bevy system parameter (Bevy has a 16-parameter limit).
    shared_state: Arc<RwLock<SharedGameState>>,
}

#[derive(Resource)]
struct BrsSharedState(Arc<RwLock<SharedGameState>>);

/// Database wrapped in Mutex for Bevy's Send+Sync requirement
/// (rusqlite::Connection is not Sync).
#[derive(Resource)]
struct BrsDatabase(Arc<Mutex<Option<DatabaseManager>>>);

#[derive(Resource, Default)]
struct BrsInputMapper(InputMapper);

#[derive(Resource)]
struct BrsExternalManager(ExternalManager);

#[derive(Resource, Default)]
struct BrsSkinManager(skin_manager::SkinManager);

#[derive(Resource, Default)]
struct BrsSystemSoundManager(system_sound::SystemSoundManager);

/// Preview music processor wrapped in Mutex (Kira AudioManager is not Sync).
#[derive(Resource)]
struct BrsPreviewMusic(Mutex<Option<preview_music::PreviewMusicProcessor>>);

/// Download processor and configuration for background song downloads.
#[derive(Resource)]
struct BrsDownloadHandle(Option<Arc<DownloadHandle>>);

/// Background version check result receiver.
///
/// The version check runs in a background thread; the receiver is polled once
/// per frame until a result arrives, then logged and optionally displayed.
#[derive(Resource)]
struct BrsVersionCheck {
    rx: Mutex<Option<std::sync::mpsc::Receiver<VersionStatus>>>,
    status: Option<VersionStatus>,
}

#[derive(Resource)]
struct BrsConfigPaths {
    config: PathBuf,
    player_config: PathBuf,
}

fn timer_update_system(mut timer: ResMut<BrsTimerManager>) {
    timer.0.update();
}

/// Exit the app when a state handler requests it (e.g., --exit-after-result).
fn exit_check_system(resource: Res<BrsPlayerResource>, mut exit: EventWriter<AppExit>) {
    if resource.0.request_app_exit {
        exit.send(AppExit::Success);
    }
}

#[derive(Resource)]
struct StateUiResources {
    config_paths: BrsConfigPaths,
    preview_music: BrsPreviewMusic,
    download_handle: BrsDownloadHandle,
}

#[allow(clippy::too_many_arguments)] // Bevy system using dependency injection
fn state_machine_system(
    mut timer: ResMut<BrsTimerManager>,
    mut resource: ResMut<BrsPlayerResource>,
    config: Res<BrsConfig>,
    mut player_config: ResMut<BrsPlayerConfig>,
    mut registry: ResMut<BrsStateRegistry>,
    database: Res<BrsDatabase>,
    mut input_mapper: ResMut<BrsInputMapper>,
    mut external: ResMut<BrsExternalManager>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut keyboard_events: EventReader<KeyboardInput>,
    mut backend: Local<bevy_keyboard::BevyKeyboardBackend>,
    mut skin_mgr: ResMut<BrsSkinManager>,
    mut sound_mgr: ResMut<BrsSystemSoundManager>,
    mod_menu: Res<bms_render::mod_menu::ModMenuState>,
    mut bevy_images: ResMut<Assets<Image>>,
    ui_res: Res<StateUiResources>,
) {
    // When ModMenu has keyboard focus, skip game input processing.
    // Delete key is handled separately by the ModMenu plugin.
    let egui_has_focus = mod_menu.wants_keyboard || mod_menu.wants_pointer;

    backend.snapshot(&keyboard_input);
    let input_state = if egui_has_focus {
        Default::default()
    } else {
        input_mapper.0.update(&*backend)
    };

    // Collect typed characters from keyboard events (suppress when egui has focus)
    let received_chars: Vec<char> = if egui_has_focus {
        // Drain events to prevent stale input when focus returns
        keyboard_events.read().for_each(drop);
        Vec::new()
    } else {
        keyboard_events
            .read()
            .filter(|e| e.state.is_pressed())
            .filter_map(|e| {
                if let Key::Character(ref s) = e.logical_key {
                    Some(s.chars())
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    };

    let prev_state = registry.registry.current();

    // Lock database and shared state for this frame
    let db_guard = database.0.lock();
    let db_ref = db_guard.as_ref();
    let shared_arc = Arc::clone(&registry.shared_state);
    let mut shared_guard = shared_arc.write();
    let mut pm_guard = ui_res.preview_music.0.lock();
    let mut params = TickParams {
        timer: &mut timer.0,
        resource: &mut resource.0,
        config: &config.0,
        player_config: &mut player_config.0,
        keyboard_backend: Some(&*backend),
        database: db_ref,
        input_state: Some(&input_state),
        skin_manager: Some(&mut skin_mgr.0),
        sound_manager: Some(&mut sound_mgr.0),
        received_chars: &received_chars,
        bevy_images: Some(&mut bevy_images),
        shared_state: Some(&mut shared_guard),
        preview_music: pm_guard.as_mut(),
        download_handle: ui_res.download_handle.0.clone(),
    };
    registry.registry.tick(&mut params);
    drop(shared_guard);
    drop(pm_guard);

    // Save config if requested by a state (KeyConfig/SkinConfig shutdown)
    if resource.0.config_save_requested {
        resource.0.config_save_requested = false;
        if let Err(e) = player_config.0.write(&ui_res.config_paths.player_config) {
            tracing::warn!("Failed to save player config: {e}");
        } else {
            info!("Player config saved");
        }
    }

    // Notify external integrations on state transitions
    let current_state = registry.registry.current();
    if current_state != prev_state {
        let (song_title, artist, key_count) = resource
            .0
            .bms_model
            .as_ref()
            .map(|m| {
                (
                    Some(m.title.as_str()),
                    Some(m.artist.as_str()),
                    Some(m.mode.key_count()),
                )
            })
            .unwrap_or((None, None, None));
        external
            .0
            .on_state_change(&current_state.to_string(), song_title, artist, key_count);
    }
}

/// Poll for background version check result (runs each frame until result arrives).
fn version_check_system(mut vc: ResMut<BrsVersionCheck>) {
    if vc.status.is_some() {
        return; // Already received
    }
    let received = {
        let mut rx_guard = vc.rx.lock();
        if let Some(rx) = rx_guard.as_ref() {
            match rx.try_recv() {
                Ok(status) => {
                    *rx_guard = None; // Drop receiver
                    Some(Ok(status))
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => None, // Still waiting
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    *rx_guard = None;
                    Some(Err(()))
                }
            }
        } else {
            None
        }
    };
    match received {
        Some(Ok(status)) => {
            match &status {
                VersionStatus::UpToDate => {
                    info!("Version check: up to date");
                }
                VersionStatus::UpdateAvailable {
                    current,
                    latest,
                    download_url,
                } => {
                    info!(
                        current = %current,
                        latest = %latest,
                        url = %download_url,
                        "Version check: update available"
                    );
                }
                VersionStatus::Development { current, latest } => {
                    info!(
                        current = %current,
                        latest = %latest,
                        "Version check: development build"
                    );
                }
                VersionStatus::CheckFailed(e) => {
                    tracing::warn!("Version check failed: {e}");
                }
            }
            vc.status = Some(status);
        }
        Some(Err(())) => {
            tracing::warn!("Version check: channel disconnected");
        }
        None => {}
    }
}

/// Consumes pending skin load requests and loads the skin into the Bevy world.
///
/// Each frame, checks `SkinManager.take_request()`. If a request exists:
/// 1. Resolves skin path from player_config (with fallback to default)
/// 2. Reads and parses the skin file (.luaskin → JSON conversion, .json → preprocess)
/// 3. Loads source images via BevyImageLoader
/// 4. Calls setup_skin() to spawn entities and insert SkinRenderState
fn skin_load_system(
    mut commands: Commands,
    mut skin_mgr: ResMut<BrsSkinManager>,
    player_config: Res<BrsPlayerConfig>,
    config: Res<BrsConfig>,
    mut images: ResMut<Assets<Image>>,
    shared_state: Res<BrsSharedState>,
    skin_entities: Query<Entity, With<bms_render::skin_renderer::SkinObjectEntity>>,
) {
    let Some(skin_type) = skin_mgr.0.take_request() else {
        return;
    };

    if let Err(e) = do_skin_load(
        &mut commands,
        &mut skin_mgr.0,
        skin_type,
        &player_config.0,
        &config.0,
        &mut images,
        &shared_state.0,
        &skin_entities,
    ) {
        tracing::warn!("Skin load failed for {:?}: {e}", skin_type);
        skin_mgr.0.load_status = skin_manager::SkinLoadStatus::MinimalUi;
        skin_mgr.0.last_error = Some(format!("{e}"));
    }
}

/// Inner function for skin loading — returns Result for clean error handling.
#[allow(clippy::too_many_arguments)]
fn do_skin_load(
    commands: &mut Commands,
    skin_mgr: &mut skin_manager::SkinManager,
    skin_type: skin_manager::SkinType,
    player_config: &bms_config::PlayerConfig,
    config: &bms_config::Config,
    images: &mut Assets<Image>,
    shared_state: &Arc<RwLock<SharedGameState>>,
    skin_entities: &Query<Entity, With<bms_render::skin_renderer::SkinObjectEntity>>,
) -> Result<()> {
    let config_id = skin_type.to_config_id() as usize;
    let dest_resolution = config.resolution;

    // Resolve skin path from player_config, falling back to default
    let skin_path_str = player_config
        .skin
        .get(config_id)
        .and_then(|sc| sc.path.as_deref())
        .filter(|p| !p.is_empty());

    // Collect enabled options and offsets from skin config
    let (enabled_options, offsets) = player_config
        .skin
        .get(config_id)
        .and_then(|sc| sc.properties.as_ref())
        .map(|props| {
            let opts: HashSet<i32> = props.option.iter().map(|o| o.value).collect();
            let offs: Vec<(i32, bms_config::skin_config::Offset)> = props
                .offset
                .iter()
                .enumerate()
                .map(|(i, o)| (i as i32, o.clone()))
                .collect();
            (opts, offs)
        })
        .unwrap_or_default();

    // Try configured path, then default
    let skin_path = skin_path_str
        .map(PathBuf::from)
        .or_else(|| {
            let default = bms_config::skin_config::SkinConfig::get_default(config_id as i32);
            default.path.map(PathBuf::from)
        })
        .ok_or_else(|| anyhow::anyhow!("No skin path configured for {:?}", skin_type))?;

    info!(path = %skin_path.display(), ?skin_type, "Loading skin");

    let skin_source = std::fs::read_to_string(&skin_path)
        .map_err(|e| anyhow::anyhow!("Failed to read skin file {}: {e}", skin_path.display()))?;

    let skin_dir = skin_path.parent().unwrap_or(Path::new("."));
    let is_lua = skin_path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("luaskin"));

    // Convert to JSON if Lua, otherwise preprocess JSON
    let json_str = if is_lua {
        bms_skin::loader::lua_loader::lua_to_json_string(
            &skin_source,
            Some(&skin_path),
            &enabled_options,
            &offsets,
            None,
        )?
    } else {
        skin_source.clone()
    };

    let preprocessed = if is_lua {
        json_str.clone()
    } else {
        bms_skin::loader::json_loader::preprocess_json(&json_str)
    };

    // Parse JSON to extract source image paths
    let raw: serde_json::Value = serde_json::from_str(&preprocessed)?;

    // Phase 1: Load source images via BevyImageLoader (borrows images mutably)
    let mut texture_map = bms_render::texture_map::TextureMap::new();
    let mut source_images: HashMap<String, bms_skin::image_handle::ImageHandle> = HashMap::new();
    {
        let mut loader =
            bms_render::image_loader_bevy::BevyImageLoader::new(images, &mut texture_map, 0);

        if let Some(sources) = raw.get("source").and_then(|v| v.as_array()) {
            for src in sources {
                let id = match src.get("id") {
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(serde_json::Value::String(s)) => s.clone(),
                    _ => continue,
                };
                let path_str = match src.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p,
                    None => continue,
                };
                let img_path = skin_dir.join(path_str);

                // Try glob expansion for wildcard paths
                let paths_to_try = if path_str.contains('*') {
                    if let Some(pattern) = img_path.to_str() {
                        glob::glob(pattern)
                            .ok()
                            .map(|entries| entries.filter_map(|e| e.ok()).collect::<Vec<_>>())
                            .unwrap_or_default()
                    } else {
                        vec![]
                    }
                } else {
                    vec![img_path]
                };

                for actual_path in paths_to_try {
                    if let Some(handle) =
                        bms_skin::image_handle::ImageLoader::load(&mut loader, &actual_path)
                    {
                        source_images.insert(id.clone(), handle);
                        break;
                    }
                }
            }
        }
    } // loader dropped — images borrow released

    // Phase 2: Load skin with resolved images
    let skin = bms_skin::loader::json_loader::load_skin_with_images(
        &json_str,
        &enabled_options,
        dest_resolution,
        Some(&skin_path),
        &source_images,
    )?;

    // Phase 3: Load embedded textures and LR2 bitmap fonts
    let mut font_map = bms_render::font_map::FontMap::new();
    bms_render::embedded_textures::load_embedded_textures(images, &mut texture_map);
    font_map.load_lr2_fonts(&skin, images);

    // Phase 4: Despawn old skin entities and remove old render state
    for entity in skin_entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<bms_render::skin_renderer::SkinRenderState>();

    // Phase 5: Set up new skin
    let state_provider = game_state::GameStateProvider::new(Arc::clone(shared_state));
    bms_render::skin_renderer::setup_skin(
        commands,
        skin,
        texture_map,
        font_map,
        Box::new(state_provider),
    );
    skin_mgr.mark_loaded(skin_type);
    skin_mgr.load_status = skin_manager::SkinLoadStatus::Loaded;
    skin_mgr.last_error = None;

    info!(?skin_type, "Skin loaded successfully");
    Ok(())
}

/// Updates the preview music processor each frame.
///
/// When a non-looping preview finishes, this switches back to default BGM.
fn preview_music_update_system(ui_res: Res<StateUiResources>) {
    if let Some(pm) = ui_res.preview_music.0.lock().as_mut() {
        pm.update();
    }
}

fn state_sync_system(
    timer: Res<BrsTimerManager>,
    shared: Res<BrsSharedState>,
    config: Res<BrsConfig>,
    render_state: Option<ResMut<bms_render::skin_renderer::SkinRenderState>>,
) {
    sync_timer_state(&timer.0, &shared.0);
    let mut shared_guard = shared.0.write();
    game_state::sync_common_state(&mut shared_guard, &config.0);

    // Sync bar scroll state and graph data to the skin renderer
    if let Some(mut rs) = render_state {
        rs.bar_scroll_state = shared_guard.bar_scroll_state.take();
        rs.bpm_events.clone_from(&shared_guard.bpm_events);
        rs.note_distribution
            .clone_from(&shared_guard.note_distribution);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(
        bms: Option<&str>,
        bms_positional: Option<&str>,
        autoplay: bool,
        practice: bool,
        replay: Option<u8>,
        play: bool,
    ) -> Args {
        Args {
            bms: bms.map(PathBuf::from),
            db_path: PathBuf::from("db"),
            config: PathBuf::from("config_sys.json"),
            player_config: PathBuf::from("config_player.json"),
            no_launcher: false,
            autoplay,
            practice,
            replay,
            play,
            bms_positional: bms_positional.map(PathBuf::from),
            launcher_only: false,
            headless: false,
            exit_after_result: false,
        }
    }

    #[test]
    fn resolve_mode_default_is_play() {
        let args = make_args(None, None, false, false, None, false);
        assert_eq!(args.resolve_mode(), PlayerMode::Play);
    }

    #[test]
    fn resolve_mode_explicit_play_flag() {
        let args = make_args(None, None, false, false, None, true);
        assert_eq!(args.resolve_mode(), PlayerMode::Play);
    }

    #[test]
    fn resolve_mode_autoplay() {
        let args = make_args(None, None, true, false, None, false);
        assert_eq!(args.resolve_mode(), PlayerMode::Autoplay);
    }

    #[test]
    fn resolve_mode_practice() {
        let args = make_args(None, None, false, true, None, false);
        assert_eq!(args.resolve_mode(), PlayerMode::Practice);
    }

    #[test]
    fn resolve_mode_replay() {
        let args = make_args(None, None, false, false, Some(2), false);
        assert_eq!(args.resolve_mode(), PlayerMode::Replay(2));
    }

    #[test]
    fn resolve_mode_replay_slot_zero() {
        let args = make_args(None, None, false, false, Some(0), false);
        assert_eq!(args.resolve_mode(), PlayerMode::Replay(0));
    }

    #[test]
    fn resolve_mode_replay_slot_three() {
        let args = make_args(None, None, false, false, Some(3), false);
        assert_eq!(args.resolve_mode(), PlayerMode::Replay(3));
    }

    #[test]
    fn resolve_mode_autoplay_takes_priority_over_practice() {
        let args = make_args(None, None, true, true, None, false);
        assert_eq!(args.resolve_mode(), PlayerMode::Autoplay);
    }

    #[test]
    fn resolve_mode_autoplay_takes_priority_over_replay() {
        let args = make_args(None, None, true, false, Some(1), false);
        assert_eq!(args.resolve_mode(), PlayerMode::Autoplay);
    }

    #[test]
    fn resolve_mode_practice_takes_priority_over_replay() {
        let args = make_args(None, None, false, true, Some(1), false);
        assert_eq!(args.resolve_mode(), PlayerMode::Practice);
    }

    #[test]
    fn resolve_bms_path_none_when_both_absent() {
        let args = make_args(None, None, false, false, None, false);
        assert!(args.resolve_bms_path().is_none());
    }

    #[test]
    fn resolve_bms_path_from_flag() {
        let args = make_args(Some("/path/to/song.bms"), None, false, false, None, false);
        assert_eq!(
            args.resolve_bms_path(),
            Some(&PathBuf::from("/path/to/song.bms"))
        );
    }

    #[test]
    fn resolve_bms_path_from_positional() {
        let args = make_args(None, Some("/path/to/song.bms"), false, false, None, false);
        assert_eq!(
            args.resolve_bms_path(),
            Some(&PathBuf::from("/path/to/song.bms"))
        );
    }

    #[test]
    fn resolve_bms_path_flag_takes_priority() {
        let args = make_args(
            Some("/flag/song.bms"),
            Some("/positional/song.bms"),
            false,
            false,
            None,
            false,
        );
        assert_eq!(
            args.resolve_bms_path(),
            Some(&PathBuf::from("/flag/song.bms"))
        );
    }

    #[test]
    fn version_check_resource_receives_result() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(VersionStatus::UpToDate).unwrap();
        let mut vc = BrsVersionCheck {
            rx: Mutex::new(Some(rx)),
            status: None,
        };

        // Simulate the polling logic
        {
            let mut rx_guard = vc.rx.lock();
            if let Some(rx) = rx_guard.as_ref() {
                if let Ok(status) = rx.try_recv() {
                    vc.status = Some(status);
                    *rx_guard = None;
                }
            }
        }
        assert_eq!(vc.status, Some(VersionStatus::UpToDate));
    }

    #[test]
    fn version_check_resource_handles_update_available() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(VersionStatus::UpdateAvailable {
            current: "0.1.0".to_string(),
            latest: "0.2.0".to_string(),
            download_url: "https://example.com".to_string(),
        })
        .unwrap();
        let mut vc = BrsVersionCheck {
            rx: Mutex::new(Some(rx)),
            status: None,
        };
        {
            let mut rx_guard = vc.rx.lock();
            if let Some(rx) = rx_guard.as_ref() {
                if let Ok(status) = rx.try_recv() {
                    vc.status = Some(status);
                    *rx_guard = None;
                }
            }
        }
        assert!(matches!(
            vc.status,
            Some(VersionStatus::UpdateAvailable { .. })
        ));
    }

    // --- E2E tests ---

    fn make_tick_params<'a>(
        timer: &'a mut TimerManager,
        resource: &'a mut PlayerResource,
        config: &'a bms_config::Config,
        player_config: &'a mut bms_config::PlayerConfig,
    ) -> app_state::TickParams<'a> {
        app_state::TickParams {
            timer,
            resource,
            config,
            player_config,
            keyboard_backend: None,
            database: None,
            input_state: None,
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
            download_handle: None,
        }
    }

    /// In-process E2E test: runs the full state machine flow
    /// MusicSelect → Decide → Play (autoplay) → Result → exit.
    ///
    /// Advances the timer manually (1ms per tick) to drive all state transitions.
    /// Verifies: state flow, score data, and exit_after_result behavior.
    #[test]
    fn e2e_autoplay_full_flow() {
        let bms_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/minimal_7k.bms");
        let model =
            bms_model::BmsDecoder::decode(&bms_path).expect("Failed to decode minimal_7k.bms");

        let mut resource = PlayerResource {
            player_mode: PlayerMode::Autoplay,
            play_mode: model.mode,
            bms_dir: bms_path.parent().map(|p| p.to_path_buf()),
            bms_model: Some(model),
            exit_after_result: true,
            ..Default::default()
        };

        let mut registry = StateRegistry::new(AppStateType::MusicSelect);
        registry.register(AppStateType::MusicSelect, Box::new(MusicSelectState::new()));
        registry.register(AppStateType::Decide, Box::new(MusicDecideState::new()));
        registry.register(AppStateType::Play, Box::new(PlayState::new()));
        registry.register(AppStateType::Result, Box::new(ResultState::new()));
        registry.register(
            AppStateType::CourseResult,
            Box::new(CourseResultState::new()),
        );
        registry.register(AppStateType::KeyConfig, Box::new(KeyConfigState::new()));
        registry.register(AppStateType::SkinConfig, Box::new(SkinConfigState::new()));

        let mut timer = TimerManager::new();
        let config = bms_config::Config::default();
        let mut player_config = bms_config::PlayerConfig::default();

        // Run the game loop, advancing 1ms per tick
        // Maximum ~30s worth of ticks (generous timeout for the full flow)
        let max_ticks = 30_000;
        let mut visited_states = vec![AppStateType::MusicSelect];

        for _ in 0..max_ticks {
            let current = timer.now_micro_time();
            timer.set_now_micro_time(current + 1_000);
            let mut params =
                make_tick_params(&mut timer, &mut resource, &config, &mut player_config);
            registry.tick(&mut params);

            // Track state transitions
            let state = registry.current();
            if visited_states.last() != Some(&state) {
                visited_states.push(state);
            }

            if resource.request_app_exit {
                break;
            }
        }

        // Verify the app exited after result
        assert!(
            resource.request_app_exit,
            "App should request exit after result"
        );

        // Verify full state flow was traversed
        assert!(
            visited_states.contains(&AppStateType::Decide),
            "Should visit Decide, visited: {visited_states:?}"
        );
        assert!(
            visited_states.contains(&AppStateType::Play),
            "Should visit Play, visited: {visited_states:?}"
        );
        assert!(
            visited_states.contains(&AppStateType::Result),
            "Should visit Result, visited: {visited_states:?}"
        );

        // Verify score data (autoplay should judge all notes as PGREAT)
        assert!(
            resource.score_data.epg > 0,
            "Autoplay should produce PGREAT judgments (epg={})",
            resource.score_data.epg
        );
        assert!(
            resource.maxcombo > 0,
            "Autoplay should produce combo (maxcombo={})",
            resource.maxcombo
        );
    }
}

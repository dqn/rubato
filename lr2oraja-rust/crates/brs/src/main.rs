// brs — main binary for the BMS player.
//
// Integrates all crates via Bevy app with state machine.

mod app_state;
mod bevy_keyboard;
pub mod database_manager;
pub mod external_manager;
mod game_state;
pub mod input_mapper;
mod player_resource;
mod preview_music;
mod skin_manager;
mod state;
mod system_sound;
mod timer_manager;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

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
use player_resource::PlayerResource;
use state::course_result::CourseResultState;
use state::decide::MusicDecideState;
use state::key_config::KeyConfigState;
use state::play::PlayState;
use state::result::ResultState;
use state::select::MusicSelectState;
use state::skin_config::SkinConfigState;
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

    // Launch settings GUI unless skipped
    if !args.no_launcher && args.bms.is_none() {
        match bms_launcher::run_launcher(&args.config, &args.player_config) {
            Ok(Some((_, _))) => {
                info!("Launcher: user clicked Start Game");
            }
            Ok(None) => {
                info!("Launcher: user cancelled");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("Launcher failed: {e}, continuing with saved config");
            }
        }
    }

    // Load BMS if specified
    let mut resource = PlayerResource::default();
    if let Some(bms_path) = &args.bms {
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

    // Open databases
    let database = match DatabaseManager::open(&args.db_path) {
        Ok(db) => {
            info!(path = %args.db_path.display(), "Database opened");
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
    let external = ExternalManager::new(&config);
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

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "brs".to_string(),
                resolution: bevy::window::WindowResolution::new(window_width, window_height),
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
        .insert_resource(BrsSystemSoundManager::default())
        .insert_resource(StateUiResources {
            config_paths: BrsConfigPaths {
                config: config_path,
                player_config: player_config_path,
            },
            preview_music: BrsPreviewMusic(Mutex::new(preview_music)),
        })
        .add_systems(Update, timer_update_system)
        .add_systems(Update, state_machine_system)
        .add_systems(Update, state_sync_system)
        .run();

    Ok(())
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

#[derive(Resource)]
struct BrsConfigPaths {
    #[allow(dead_code)] // Reserved for future system config saving
    config: PathBuf,
    player_config: PathBuf,
}

fn timer_update_system(mut timer: ResMut<BrsTimerManager>) {
    timer.0.update();
}

#[derive(Resource)]
struct StateUiResources {
    config_paths: BrsConfigPaths,
    preview_music: BrsPreviewMusic,
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
    external: Res<BrsExternalManager>,
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
    let db_guard = database.0.lock().unwrap();
    let db_ref = db_guard.as_ref();
    let shared_arc = Arc::clone(&registry.shared_state);
    let mut shared_guard = shared_arc.write().unwrap();
    let mut pm_guard = ui_res.preview_music.0.lock().unwrap();
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
        let song_title = resource.0.bms_model.as_ref().map(|m| m.title.as_str());
        external
            .0
            .on_state_change(&current_state.to_string(), song_title);
    }
}

fn state_sync_system(
    timer: Res<BrsTimerManager>,
    shared: Res<BrsSharedState>,
    config: Res<BrsConfig>,
    render_state: Option<ResMut<bms_render::skin_renderer::SkinRenderState>>,
) {
    sync_timer_state(&timer.0, &shared.0);
    let mut shared_guard = shared.0.write().unwrap();
    game_state::sync_common_state(&mut shared_guard, &config.0);

    // Sync bar scroll state and graph data to the skin renderer
    if let Some(mut rs) = render_state {
        rs.bar_scroll_state = shared_guard.bar_scroll_state.take();
        rs.bpm_events.clone_from(&shared_guard.bpm_events);
        rs.note_distribution
            .clone_from(&shared_guard.note_distribution);
    }
}

//! Binary entry point for the rubato BMS player application.

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use log::{error, info, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use rubato_core::bms_player_mode::BMSPlayerMode;
use rubato_core::config::DisplayMode;
use rubato_core::main_controller::MainController;
use rubato_core::version;
use rubato_launcher::LauncherStateFactory;
use rubato_render::egui_integration::EguiIntegration;
use rubato_render::gpu_context::GpuContext;
use rubato_render::gpu_texture_manager::GpuTextureManager;
use rubato_render::render_pipeline::SpriteRenderPipeline;

/// rubato - BMS player
#[derive(Parser, Debug)]
#[command(name = "rubato", version, about = "rubato - BMS player")]
struct Args {
    /// BMS file path to play
    #[arg(value_name = "BMS_FILE")]
    bms_path: Option<PathBuf>,

    /// Autoplay mode
    #[arg(short = 'a', long)]
    autoplay: bool,

    /// Practice mode
    #[arg(short = 'p', long)]
    practice: bool,

    /// Replay mode (1-4)
    #[arg(short = 'r', long, value_name = "NUM")]
    replay: Option<u8>,

    /// Direct play mode
    #[arg(short = 's', long)]
    play: bool,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let args = Args::parse();

    // Determine player mode from arguments
    // Java: MainLoader.main() parses -a, -p, -r, -r1..r4, -s flags
    let player_mode: Option<BMSPlayerMode> = if args.autoplay {
        Some(BMSPlayerMode::AUTOPLAY)
    } else if args.practice {
        Some(BMSPlayerMode::PRACTICE)
    } else if args.play {
        Some(BMSPlayerMode::PLAY)
    } else if let Some(num) = args.replay {
        match num {
            2 => Some(BMSPlayerMode::REPLAY_2),
            3 => Some(BMSPlayerMode::REPLAY_3),
            4 => Some(BMSPlayerMode::REPLAY_4),
            _ => Some(BMSPlayerMode::REPLAY_1),
        }
    } else if args.bms_path.is_some() {
        // Java: if bmsPath provided without mode flag, default to PLAY
        Some(BMSPlayerMode::PLAY)
    } else {
        None
    };

    // Java: if (Files.exists(Config.configpath) && (bmsPath != null || auto != null))
    let config_exists =
        PathBuf::from("config_sys.json").exists() || PathBuf::from("config.json").exists();

    if config_exists && (args.bms_path.is_some() || player_mode.is_some()) {
        play(args.bms_path, player_mode)?;
    } else {
        // Java: launch(args) → JavaFX Application.start() → PlayConfigurationView
        info!("No config found or no play mode specified. Launching configuration UI...");
        launch()?;
    }

    Ok(())
}

/// Java: MainLoader.start(Stage) → opens the launcher/configuration UI.
///
/// Delegates to MainLoader::start() for Config/PlayerConfig loading,
/// then launches the eframe launcher window via run_launcher().
/// If the user clicks "Start", delegates to play() to launch the game.
fn launch() -> Result<()> {
    use rubato_core::main_loader::MainLoader;

    // Java: MainLoader.start(Stage) — reads config, creates PlayConfigurationView
    let (config, player, title) = MainLoader::start();

    // Java: primaryStage.setScene(scene); primaryStage.show();
    // eframe::run_native() blocks until the window is closed.
    let result = rubato_launcher::run_launcher(config, player, &title)?;

    // Java: PlayConfigurationView.start() calls MainLoader.play()
    // Re-exec as a child process because winit does not allow creating a second
    // EventLoop in the same process (eframe already consumed the first one).
    if result.play_requested {
        info!("Launcher requested play, re-launching as child process...");
        let exe = std::env::current_exe()?;
        let status = std::process::Command::new(exe).arg("-s").status()?;
        if !status.success() {
            anyhow::bail!("Game process exited with {}", status);
        }
    }

    Ok(())
}

/// Java: MainLoader.play() — creates MainController and launches the application window.
///
/// Delegates to MainLoader::play() for Config reading, illegal songs check,
/// PlayerConfig reading, and MainController creation. Then creates the winit
/// EventLoop + wgpu context for the render loop.
fn play(bms_path: Option<PathBuf>, player_mode: Option<BMSPlayerMode>) -> Result<()> {
    use rubato_core::main_loader::MainLoader;

    // Wire song database before MainLoader::play(), which calls take_score_database_accessor().
    // In the launcher path, LauncherMainLoader::play() handles this via init_score_database_accessor().
    // In the direct play path (-s flag or bms_path), we must do it here.
    {
        use rubato_core::config::Config;
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
                // In the launcher path this happens via the egui "Start" button, but in
                // the direct play path we must do it here.
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

    // Java: MainLoader.play() handles config, illegal songs, player config, and controller creation.
    // It sets config.windowWidth/Height from resolution before creating MainController.
    let mut main_controller = MainLoader::play(bms_path, player_mode, true, None, None, false)?;

    // Java: audio = new GdxSoundDriver(config.getSongResourceGen())
    // Wire the Kira-based audio driver so keysounds, BGM, and UI sounds work.
    {
        let song_resource_gen = main_controller.config().render.song_resource_gen;
        let audio_driver = rubato_audio::gdx_sound_driver::GdxSoundDriver::new(song_resource_gen)?;
        main_controller.set_audio_driver(Box::new(audio_driver));
    }

    // Set the state factory so that change_state() can create concrete state instances.
    // Without this, the controller has no factory and all state transitions silently fail,
    // resulting in a black screen.
    main_controller.set_state_factory(Box::new(LauncherStateFactory::new()));

    // Java: if(config.isUseDiscordRPC()) { stateListener.add(new DiscordListener()); }
    {
        let (use_discord_rpc, use_obs_ws, cfg_clone) = {
            let cfg = main_controller.config();
            (
                cfg.integration.use_discord_rpc,
                cfg.obs.use_obs_ws,
                cfg.clone(),
            )
        };
        if use_discord_rpc {
            let listener = rubato_external::discord_listener::DiscordListener::new();
            main_controller.add_state_listener(Box::new(listener));
        }
        if use_obs_ws {
            let obs_client = rubato_external::obs::obs_ws_client::ObsWsClient::new(&cfg_clone);
            let listener = rubato_external::obs::obs_listener::ObsListener::new(cfg_clone);
            main_controller.add_state_listener(Box::new(listener));
            if let Ok(client) = obs_client {
                main_controller.set_obs_client(Box::new(client));
            }
        }
    }

    // Wire IR initialization at startup
    {
        let player_config = main_controller.player_config().clone();
        let ir_statuses =
            rubato_state::result::ir_initializer::initialize_ir_config(&player_config);
        for ir_status in ir_statuses {
            let rival_provider = rubato_ir::ir_rival_provider_impl::IRRivalProviderImpl::new(
                ir_status.connection.clone(),
                ir_status.player.clone(),
                ir_status.config.irname.clone(),
                ir_status.config.importscore,
                ir_status.config.importrival,
            );
            main_controller
                .ir_status_mut()
                .push(rubato_core::main_controller::IRStatus {
                    config: ir_status.config,
                    rival_provider: Some(Box::new(rival_provider)),
                    connection: Some(Box::new(ir_status.connection.clone())),
                });
        }
        // Wire IR resend service
        let ir_send_count = main_controller.config().network.ir_send_count;
        let resend_service =
            rubato_state::result::ir_resend::IrResendServiceImpl::new(ir_send_count);
        main_controller.set_ir_resend_service(Box::new(resend_service));
    }

    // Java: MainController.create() lines 496-513 creates download processors.
    // Each processor runs on background threads and needs its own DB access, so we open
    // separate SQLite connections rather than sharing MainController's Box<dyn> songdb.
    {
        let config = main_controller.config().clone();

        // IPFS download processor (Java: lines 496-506)
        if config.network.enable_ipfs {
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
                    main_controller.set_music_download_processor(Box::new(processor));
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

        // HTTP download processor (Java: lines 508-513)
        if config.network.enable_http {
            // Look up download source by config.network.download_source, fall back to default
            let source_meta = rubato_song::md_processor::http_download_processor::DOWNLOAD_SOURCES
                .get(&config.network.download_source)
                .copied()
                .unwrap_or_else(|| {
                    rubato_song::md_processor::http_download_processor::HttpDownloadProcessor::default_download_source()
                });
            let http_download_source: Arc<
                dyn rubato_song::md_processor::http_download_source::HttpDownloadSource,
            > = Arc::from(source_meta.build(&config));

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
                    rubato_state::modmenu::download_task_menu::DownloadTaskMenu::set_processor(
                        Arc::clone(&processor),
                    );

                    main_controller.set_http_download_processor(Box::new(
                        HttpDownloadProcessorWrapper(Arc::clone(&processor)),
                    ));
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
    }

    // Java: MainController.initializeStates() lines 561-564:
    //   if(player.getRequestEnable()) {
    //       streamController = new StreamController(selector);
    //       streamController.run();
    //   }
    //
    // Java: StreamController shares the same MusicSelector as the MusicSelect screen state,
    // so stream request songs appear in the selector's bar list.
    // In Rust, we create a shared Arc<Mutex<MusicSelector>> and store it on MainController.
    // Both StreamController and StateFactory (MusicSelect arm) use the same instance.
    if main_controller.player_config().enable_request {
        let config = main_controller.config();
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
                rubato_launcher::state_factory::new_state_main_controller_access(
                    &mut main_controller,
                ),
            );
            selector.config = main_controller.player_config().clone();
        }
        let selector = std::sync::Arc::new(std::sync::Mutex::new(selector));
        // Store the shared selector on MainController for StateFactory to retrieve
        main_controller.set_shared_music_selector(Box::new(std::sync::Arc::clone(&selector)));
        let mut stream_controller =
            rubato_state::stream::stream_controller::StreamController::new(selector);
        stream_controller.run();
        main_controller.set_stream_controller(Box::new(stream_controller));
    }

    // Extract window config from the controller's Config
    // Java: these were set by MainLoader.play() → config.setWindowWidth/Height
    let config = main_controller.config();
    let w = config.display.window_width;
    let h = config.display.window_height;
    let vsync = config.display.vsync;
    let display_mode = config.display.displaymode;
    let max_fps = config.display.max_frame_per_second;
    // Java: gdxConfig.setTitle(MainController.getVersion())
    let title = version::version_long().to_string();

    info!("Starting {}", version::version_long());
    if let Some(hash) = version::git_commit_hash() {
        info!("[Build info] Commit: {}", hash);
    }
    if let Some(date) = version::build_date() {
        info!("[Build info] Build date: {}", date);
    }

    // Java: new Lwjgl3Application(new ApplicationListener() { ... }, gdxConfig)
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    // Initialize shared key state for winit->input bridge
    let key_state = rubato_input::winit_input_bridge::SharedKeyState::new();
    rubato_input::gdx_compat::set_shared_key_state(key_state.clone());

    let mut app = RubatoApp {
        controller: main_controller,
        window: None,
        gpu: None,
        sprite_pipeline: None,
        texture_manager: None,
        egui_integration: None,
        egui_state: None,
        title,
        width: w as u32,
        height: h as u32,
        _vsync: vsync,
        display_mode,
        max_fps,
        last_frame_time: Instant::now(),
        initialized: false,
        key_state,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}

/// Application handler that bridges winit events to MainController lifecycle.
///
/// Java equivalent: the anonymous ApplicationListener passed to Lwjgl3Application
/// with create(), render(), resize(), pause(), resume(), dispose() callbacks.
struct RubatoApp {
    controller: MainController,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    /// Sprite render pipeline for skin object rendering (Phase 22a)
    sprite_pipeline: Option<SpriteRenderPipeline>,
    /// GPU texture cache for skin image rendering
    texture_manager: Option<GpuTextureManager>,
    egui_integration: Option<EguiIntegration>,
    egui_state: Option<egui_winit::State>,
    title: String,
    width: u32,
    height: u32,
    _vsync: bool,
    display_mode: DisplayMode,
    /// Maximum FPS (from Config.maxFramePerSecond, default 240)
    /// Java: gdxConfig.setForegroundFPS(config.getMaxFramePerSecond())
    max_fps: i32,
    /// Last frame time for FPS capping
    last_frame_time: Instant,
    initialized: bool,
    /// Shared key state bridging winit keyboard events to the input system
    key_state: rubato_input::winit_input_bridge::SharedKeyState,
}

impl ApplicationHandler for RubatoApp {
    /// Java: ApplicationListener.create() — called when the application is first created.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Populate monitor cache for VideoConfigurationView
        rubato_launcher::stubs::update_monitors_from_winit(event_loop);

        // Sync display mode cache to core MainLoader
        {
            use rubato_core::main_loader::MainLoader;
            let modes = rubato_launcher::stubs::cached_display_modes();
            if !modes.is_empty() {
                MainLoader::set_display_modes(modes);
            }
            let desktop = rubato_launcher::stubs::cached_desktop_display_mode();
            if desktop != (0, 0) {
                MainLoader::set_desktop_display_mode(desktop);
            }
        }

        if self.window.is_none() {
            // Java: Find target monitor by config.monitorName
            // Format: "MonitorName [virtualX, virtualY]"
            let config = self.controller.config();
            let monitor_name = config.integration.monitor_name.clone();

            let target_monitor = if !monitor_name.is_empty() {
                event_loop.available_monitors().find(|handle| {
                    let name = handle.name().unwrap_or_default();
                    let pos = handle.position();
                    let formatted = format!("{} [{}, {}]", name, pos.x, pos.y);
                    formatted == monitor_name
                })
            } else {
                None
            };

            // Java: gdxConfig.setWindowedMode(w, h); gdxConfig.setTitle(MainController.getVersion())
            let decorated = !matches!(
                self.display_mode,
                DisplayMode::FULLSCREEN | DisplayMode::BORDERLESS
            );
            let mut window_attributes = Window::default_attributes()
                .with_title(&self.title)
                .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height))
                .with_decorations(decorated);

            if matches!(self.display_mode, DisplayMode::FULLSCREEN) {
                // Java: MainLoader.play() lines 177-208 — fullscreen setup
                // Find best matching video mode on target (or primary) monitor
                let monitor = target_monitor
                    .clone()
                    .or_else(|| event_loop.primary_monitor());

                if let Some(monitor) = monitor {
                    // Java: find display mode matching w,h with highest refreshRate and bitsPerPixel
                    let best_mode = monitor
                        .video_modes()
                        .filter(|m| m.size().width == self.width && m.size().height == self.height)
                        .max_by_key(|m| (m.refresh_rate_millihertz(), m.bit_depth()));

                    if let Some(mode) = best_mode {
                        info!(
                            "Fullscreen: {}x{} @{}mHz on monitor",
                            mode.size().width,
                            mode.size().height,
                            mode.refresh_rate_millihertz()
                        );
                        window_attributes = window_attributes
                            .with_fullscreen(Some(winit::window::Fullscreen::Exclusive(mode)));
                    } else {
                        warn!(
                            "Resolution {}x{} not available for exclusive fullscreen, using borderless",
                            self.width, self.height
                        );
                        window_attributes = window_attributes.with_fullscreen(Some(
                            winit::window::Fullscreen::Borderless(Some(monitor)),
                        ));
                    }
                }
            } else if matches!(self.display_mode, DisplayMode::BORDERLESS) {
                // Java: borderless mode with target monitor
                let monitor = target_monitor
                    .clone()
                    .or_else(|| event_loop.primary_monitor());
                if let Some(monitor) = monitor {
                    window_attributes = window_attributes.with_fullscreen(Some(
                        winit::window::Fullscreen::Borderless(Some(monitor)),
                    ));
                }
            } else if let Some(ref monitor) = target_monitor {
                // Java: windowed mode — position at target monitor origin
                let pos = monitor.position();
                window_attributes = window_attributes
                    .with_position(winit::dpi::PhysicalPosition::new(pos.x, pos.y));
            }

            match event_loop.create_window(window_attributes) {
                Ok(window) => {
                    let window = Arc::new(window);

                    // Create wgpu GPU context bound to this window's surface.
                    // wgpu SurfaceConfiguration expects physical pixels, not logical.
                    let physical = window.inner_size();
                    match pollster::block_on(GpuContext::new_with_surface(
                        Arc::clone(&window),
                        physical.width,
                        physical.height,
                    )) {
                        Ok(gpu) => {
                            info!("wgpu GPU context created successfully");

                            // Create sprite render pipeline for skin object rendering
                            // Java: ShaderManager creates shader programs for SpriteBatch
                            let sprite_pipeline =
                                SpriteRenderPipeline::new(&gpu.device, gpu.surface_format());
                            info!(
                                "SpriteRenderPipeline created with {} pipelines",
                                sprite_pipeline.pipeline_count()
                            );

                            // Create GPU texture manager for skin image rendering
                            let texture_manager = GpuTextureManager::new(
                                &gpu.device,
                                &gpu.queue,
                                &sprite_pipeline.texture_layout,
                                &sprite_pipeline.sampler_nearest,
                                &sprite_pipeline.sampler_linear,
                            );
                            self.texture_manager = Some(texture_manager);

                            self.sprite_pipeline = Some(sprite_pipeline);

                            // Initialize egui integration
                            // Java: ImGui.createContext() + imGuiGl3.init() + imGuiGlfw.init()
                            let egui_integration =
                                EguiIntegration::new(&gpu.device, gpu.surface_format());
                            let egui_state = egui_winit::State::new(
                                egui_integration.ctx.clone(),
                                egui::ViewportId::ROOT,
                                event_loop,
                                Some(window.scale_factor() as f32),
                                None,
                                Some(gpu.device.limits().max_texture_dimension_2d as usize),
                            );
                            self.egui_integration = Some(egui_integration);
                            self.egui_state = Some(egui_state);
                            self.gpu = Some(gpu);
                        }
                        Err(e) => {
                            error!("Failed to create GPU context: {}", e);
                            event_loop.exit();
                            return;
                        }
                    }

                    self.window = Some(window);
                }
                Err(e) => {
                    error!("Failed to create window: {}", e);
                    event_loop.exit();
                    return;
                }
            }
        }

        if !self.initialized {
            // Java: main.create()
            self.controller.create();
            self.initialized = true;
        } else {
            // Java: main.resume()
            self.controller.resume();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Forward events to egui
        // Java: imGuiGlfw processes GLFW events for ImGui input
        if let Some(state) = &mut self.egui_state
            && let Some(window) = &self.window
        {
            let response = state.on_window_event(window, &event);
            // Skip game logic only for events that egui exclusively consumes
            // (e.g. text input into an egui widget). Keyboard, mouse, and
            // redraw events must ALWAYS reach the game's input system.
            if response.consumed
                && !matches!(
                    event,
                    WindowEvent::RedrawRequested
                        | WindowEvent::KeyboardInput { .. }
                        | WindowEvent::MouseInput { .. }
                        | WindowEvent::CursorMoved { .. }
                        | WindowEvent::MouseWheel { .. }
                )
            {
                return;
            }
        }

        match event {
            // Bridge winit keyboard events to the input system
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let java_key = rubato_input::winit_input_bridge::winit_keycode_to_java(
                        winit_to_bridge_keycode(keycode),
                    );
                    if java_key >= 0 {
                        self.key_state
                            .set_key_pressed(java_key, event.state.is_pressed());
                    }
                }
            }
            // Bridge winit mouse position to SharedKeyState
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
                self.key_state
                    .set_mouse_position((position.x / scale) as i32, (position.y / scale) as i32);
            }
            // Bridge winit mouse button events to SharedKeyState
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = match button {
                    winit::event::MouseButton::Left => {
                        rubato_input::winit_input_bridge::MOUSE_BUTTON_LEFT
                    }
                    winit::event::MouseButton::Right => {
                        rubato_input::winit_input_bridge::MOUSE_BUTTON_RIGHT
                    }
                    winit::event::MouseButton::Middle => {
                        rubato_input::winit_input_bridge::MOUSE_BUTTON_MIDDLE
                    }
                    _ => -1,
                };
                if btn >= 0 {
                    self.key_state.set_mouse_button(btn, state.is_pressed());
                }
            }
            // Bridge winit scroll events to SharedKeyState
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (x, y),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        (pos.x as f32 / 40.0, pos.y as f32 / 40.0)
                    }
                };
                self.key_state.add_scroll(dx, dy);
            }
            // Java: dispose() is called when the window is closed
            WindowEvent::CloseRequested => {
                self.controller.dispose();
                event_loop.exit();
            }
            // Java: main.resize(width, height)
            WindowEvent::Resized(size) => {
                // Surface uses physical pixels
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                // Game coordinate space uses logical pixels (matching skin/config resolution)
                let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
                let logical_w = (size.width as f64 / scale) as i32;
                let logical_h = (size.height as f64 / scale) as i32;
                self.controller.resize(logical_w, logical_h);
                // Keep SharedKeyState window size in sync for GdxGraphics
                self.key_state.set_window_size(logical_w, logical_h);
            }
            // Java: main.render() — called every frame via ApplicationListener.render()
            WindowEvent::RedrawRequested => {
                // FPS capping
                // Java: gdxConfig.setForegroundFPS(config.getMaxFramePerSecond())
                // Java: gdxConfig.setIdleFPS(config.getMaxFramePerSecond())
                if self.max_fps > 0 {
                    let target_frame_duration = Duration::from_secs_f64(1.0 / self.max_fps as f64);
                    let elapsed = self.last_frame_time.elapsed();
                    if elapsed < target_frame_duration {
                        std::thread::sleep(target_frame_duration - elapsed);
                    }
                }
                self.last_frame_time = Instant::now();

                // Game logic update (timer, state render, sprite batch begin/end, input)
                self.controller.render();

                let Some(window) = &self.window else { return };
                let Some(gpu) = &self.gpu else { return };

                // Process window commands from MainController
                if rubato_core::window_command::take_fullscreen_toggle() {
                    if window.fullscreen().is_some() {
                        window.set_fullscreen(None);
                    } else {
                        let monitor = window.current_monitor();
                        window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(monitor)));
                    }
                }
                if rubato_core::window_command::take_screenshot_request() {
                    self.capture_screenshot(gpu, window);
                }

                // Gather diagnostic info before egui frame (avoids borrow conflicts)
                let diag_state_type = self.controller.current_state_type();
                let diag_has_skin = self
                    .controller
                    .current_state()
                    .map(|s| s.main_state_data().skin.is_some())
                    .unwrap_or(false);

                // Run egui frame
                // Java: ImGuiRenderer.start() → ImGuiRenderer.render() → ImGuiRenderer.end()
                let full_output = if let (Some(egui_state), Some(egui_integration)) =
                    (&mut self.egui_state, &self.egui_integration)
                {
                    let raw_input = egui_state.take_egui_input(window);
                    let full_output = egui_integration.ctx.run(raw_input, |ctx| {
                        rubato_state::modmenu::imgui_renderer::ImGuiRenderer::render_ui(ctx);

                        // Diagnostic overlay: show current state and skin status
                        egui::Area::new(egui::Id::new("diag_overlay"))
                            .fixed_pos(egui::pos2(10.0, 10.0))
                            .show(ctx, |ui| {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_black_alpha(180))
                                    .inner_margin(8.0)
                                    .corner_radius(4.0)
                                    .show(ui, |ui| {
                                        let state_str = match diag_state_type {
                                            Some(st) => format!("{:?}", st),
                                            None => "None".to_string(),
                                        };
                                        ui.colored_label(
                                            egui::Color32::WHITE,
                                            format!("State: {}", state_str),
                                        );
                                        let skin_color = if diag_has_skin {
                                            egui::Color32::GREEN
                                        } else {
                                            egui::Color32::from_rgb(255, 100, 100)
                                        };
                                        ui.colored_label(
                                            skin_color,
                                            format!(
                                                "Skin: {}",
                                                if diag_has_skin {
                                                    "loaded"
                                                } else {
                                                    "NOT loaded (load_skin stub)"
                                                }
                                            ),
                                        );
                                    });
                            });
                    });
                    egui_state.handle_platform_output(window, full_output.platform_output.clone());
                    Some(full_output)
                } else {
                    None
                };

                // wgpu render pass: clear screen, sprite batch flush, egui overlay, present
                match gpu.current_texture() {
                    Ok(output) => {
                        let view = output
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        let mut encoder =
                            gpu.device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("rubato frame encoder"),
                                });

                        // Upload pending textures and prepare sprite batch resources
                        // before the render pass (bind groups must outlive the render pass)
                        let sprite_resources = if let Some(sprite_pipeline) = &self.sprite_pipeline
                            && let Some(texture_manager) = &mut self.texture_manager
                            && let Some(sprite_batch) = self.controller.sprite_batch_mut()
                            && !sprite_batch.vertices().is_empty()
                        {
                            // Upload any new textures encountered this frame
                            let pending = sprite_batch.drain_pending_textures();
                            for (key, tex) in &pending {
                                texture_manager.ensure_uploaded(
                                    key,
                                    tex.width,
                                    tex.height,
                                    &tex.rgba_data,
                                    &rubato_render::gpu_texture_manager::TextureUploadContext {
                                        device: &gpu.device,
                                        queue: &gpu.queue,
                                        texture_layout: &sprite_pipeline.texture_layout,
                                        sampler_nearest: &sprite_pipeline.sampler_nearest,
                                        sampler_linear: &sprite_pipeline.sampler_linear,
                                    },
                                );
                            }

                            // Create uniform bind group with projection matrix
                            let projection_data = sprite_batch.projection();
                            let uniform_buffer =
                                gpu.device.create_buffer(&wgpu::BufferDescriptor {
                                    label: Some("sprite uniform buffer"),
                                    size: 64, // 4x4 f32 matrix
                                    usage: wgpu::BufferUsages::UNIFORM
                                        | wgpu::BufferUsages::COPY_DST,
                                    mapped_at_creation: false,
                                });
                            gpu.queue.write_buffer(
                                &uniform_buffer,
                                0,
                                bytemuck::cast_slice(projection_data),
                            );
                            let uniform_bind_group =
                                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                    label: Some("sprite uniform bind group"),
                                    layout: &sprite_pipeline.uniform_layout,
                                    entries: &[wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: uniform_buffer.as_entire_binding(),
                                    }],
                                });

                            Some(uniform_bind_group)
                        } else {
                            None
                        };

                        // Render pass: clear screen + SpriteBatch GPU flush
                        // Java: Gdx.gl.glClear(GL20.GL_COLOR_BUFFER_BIT) + SpriteBatch draw
                        // Build GpuRenderContext before render_pass so it outlives the pass
                        let gpu_ctx = if let Some(ref uniform_bind_group) = sprite_resources
                            && let Some(sprite_pipeline) = &self.sprite_pipeline
                            && let Some(texture_manager) = &self.texture_manager
                        {
                            Some(rubato_render::sprite_batch::GpuRenderContext {
                                device: &gpu.device,
                                queue: &gpu.queue,
                                pipeline: sprite_pipeline,
                                uniform_bind_group,
                                texture_manager,
                            })
                        } else {
                            None
                        };

                        {
                            let mut render_pass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("rubato sprite pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                });

                            // Flush SpriteBatch vertices to GPU via the render pipeline
                            // Java: SpriteBatch.flush() submits batched quads to GL
                            if let Some(ref gpu_ctx) = gpu_ctx
                                && let Some(sprite_batch) = self.controller.sprite_batch_mut()
                            {
                                sprite_batch.flush_to_gpu(&mut render_pass, gpu_ctx);
                            }
                        }

                        // Render egui overlay on top of the game scene
                        if let Some(full_output) = full_output
                            && let Some(egui_integration) = &mut self.egui_integration
                        {
                            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                                size_in_pixels: [
                                    gpu.surface_config.as_ref().map_or(self.width, |c| c.width),
                                    gpu.surface_config
                                        .as_ref()
                                        .map_or(self.height, |c| c.height),
                                ],
                                pixels_per_point: window.scale_factor() as f32,
                            };
                            egui_integration.render(
                                &mut encoder,
                                &view,
                                &gpu.device,
                                &gpu.queue,
                                &screen_descriptor,
                                full_output,
                            );
                        }

                        gpu.queue.submit(std::iter::once(encoder.finish()));
                        output.present();
                    }
                    Err(e) => {
                        warn!("Failed to get surface texture: {}", e);
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    /// Called when the event loop is about to wait for new events.
    /// Request continuous redraws to match the Java game loop behavior.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Java: MainController checks exit flag and calls Platform.exit()
        if self.controller.is_exit_requested() {
            event_loop.exit();
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Java: main.pause()
        self.controller.pause();
    }
}

impl RubatoApp {
    /// Capture the current frame and save as a PNG screenshot.
    fn capture_screenshot(&self, gpu: &GpuContext, _window: &Window) {
        let Some(ref surface_config) = gpu.surface_config else {
            warn!("Cannot capture screenshot: no surface config");
            return;
        };
        let Some(ref surface) = gpu.surface else {
            warn!("Cannot capture screenshot: no surface");
            return;
        };
        let width = surface_config.width;
        let height = surface_config.height;
        if width == 0 || height == 0 {
            warn!("Cannot capture screenshot: surface size is 0");
            return;
        }

        // Create a buffer to read pixels from the surface
        let bytes_per_row = (width * 4 + 255) & !255; // align to 256 bytes
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot_buffer"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Get the current surface texture
        let surface_texture = match surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                warn!("Cannot capture screenshot: {}", e);
                return;
            }
        };

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("screenshot_encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &surface_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        gpu.queue.submit(std::iter::once(encoder.finish()));

        // Map the buffer and save
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        gpu.device.poll(wgpu::Maintain::Wait);

        match rx.recv() {
            Ok(Ok(())) => {
                let data = buffer_slice.get_mapped_range();
                // Remove row padding
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for row in 0..height {
                    let start = (row * bytes_per_row) as usize;
                    let end = start + (width * 4) as usize;
                    rgba.extend_from_slice(&data[start..end]);
                }
                drop(data);
                buffer.unmap();

                // Save as PNG
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let path = format!("screenshot_{}.png", timestamp);
                if let Some(img) = image::RgbaImage::from_raw(width, height, rgba) {
                    match img.save(&path) {
                        Ok(()) => info!("Screenshot saved: {}", path),
                        Err(e) => warn!("Failed to save screenshot: {}", e),
                    }
                } else {
                    warn!("Failed to create image from screenshot data");
                }
            }
            _ => {
                warn!("Failed to map screenshot buffer");
            }
        }
    }
}

// -- Download processor adapter structs --

/// Adapter: bridges `SQLiteSongDatabaseAccessor` to `rubato_song::md_processor::MusicDatabaseAccessor`.
///
/// Java equivalent: the inline lambda `(md5) -> { SongData[] s = getSongDatabase().getSongDatas(md5); ... }`
/// in MainController.create() line 497. Opens its own SQLite connection so the IPFS download
/// background thread can query the song DB without borrowing MainController.
struct SongDbMusicDatabaseAdapter {
    songdb: rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor,
}

// SAFETY: SongDbMusicDatabaseAdapter contains SQLiteSongDatabaseAccessor which wraps
// rusqlite::Connection behind an internal Mutex. Connection is !Sync, but the Mutex
// serializes all access, making it safe to share across threads.
unsafe impl Sync for SongDbMusicDatabaseAdapter {}

impl rubato_song::md_processor::music_database_accessor::MusicDatabaseAccessor
    for SongDbMusicDatabaseAdapter
{
    fn get_music_paths(&self, md5: &[String]) -> Vec<String> {
        use rubato_types::song_database_accessor::SongDatabaseAccessor;
        let songs = self.songdb.song_datas_by_hashes(md5);
        songs
            .iter()
            .filter_map(|s| s.file.path().map(|p| p.to_string()))
            .collect()
    }
}

/// Adapter: bridges a standalone song DB connection to `rubato_song::md_processor::MainControllerRef`.
///
/// Java equivalent: `this` (MainController) passed to HttpDownloadProcessor constructor.
/// The only method called is `update_song(path, force)` which ultimately calls
/// `songdb.updateSongDatas()`. We call it directly on our own connection.
struct SongDbMainControllerRef {
    songdb: rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor,
    bmsroot: Vec<String>,
}

// SAFETY: SongDbMainControllerRef contains SQLiteSongDatabaseAccessor which wraps
// rusqlite::Connection behind an internal Mutex, and a Vec<String> (bmsroot) which is
// inherently Sync. The Mutex serializes all DB access, making it safe to share across threads.
unsafe impl Sync for SongDbMainControllerRef {}

impl rubato_song::md_processor::MainControllerRef for SongDbMainControllerRef {
    fn update_song(&self, path: &str, _force: bool) {
        let update_path = if path.is_empty() { None } else { Some(path) };
        self.songdb
            .update_song_datas(update_path, &self.bmsroot, false, false, None);
    }
}

/// Wrapper to implement `HttpDownloadSubmitter` for `Arc<HttpDownloadProcessor>`.
///
/// `DownloadTaskMenu::set_processor` needs `Arc<HttpDownloadProcessor>` directly, while
/// `MainController::set_http_download_processor` needs `Box<dyn HttpDownloadSubmitter>`.
/// This wrapper bridges the two ownership models.
struct HttpDownloadProcessorWrapper(
    Arc<rubato_song::md_processor::http_download_processor::HttpDownloadProcessor>,
);

impl rubato_types::http_download_submitter::HttpDownloadSubmitter for HttpDownloadProcessorWrapper {
    fn submit_md5_task(&self, md5: &str, task_name: &str) {
        self.0.submit_md5_task(md5, task_name);
    }
}

/// Convert winit's KeyCode to the bridge's WinitKeyCode enum.
fn winit_to_bridge_keycode(
    key: winit::keyboard::KeyCode,
) -> rubato_input::winit_input_bridge::WinitKeyCode {
    use rubato_input::winit_input_bridge::WinitKeyCode as B;
    use winit::keyboard::KeyCode as W;
    match key {
        W::KeyA => B::KeyA,
        W::KeyB => B::KeyB,
        W::KeyC => B::KeyC,
        W::KeyD => B::KeyD,
        W::KeyE => B::KeyE,
        W::KeyF => B::KeyF,
        W::KeyG => B::KeyG,
        W::KeyH => B::KeyH,
        W::KeyI => B::KeyI,
        W::KeyJ => B::KeyJ,
        W::KeyK => B::KeyK,
        W::KeyL => B::KeyL,
        W::KeyM => B::KeyM,
        W::KeyN => B::KeyN,
        W::KeyO => B::KeyO,
        W::KeyP => B::KeyP,
        W::KeyQ => B::KeyQ,
        W::KeyR => B::KeyR,
        W::KeyS => B::KeyS,
        W::KeyT => B::KeyT,
        W::KeyU => B::KeyU,
        W::KeyV => B::KeyV,
        W::KeyW => B::KeyW,
        W::KeyX => B::KeyX,
        W::KeyY => B::KeyY,
        W::KeyZ => B::KeyZ,
        W::Digit0 => B::Digit0,
        W::Digit1 => B::Digit1,
        W::Digit2 => B::Digit2,
        W::Digit3 => B::Digit3,
        W::Digit4 => B::Digit4,
        W::Digit5 => B::Digit5,
        W::Digit6 => B::Digit6,
        W::Digit7 => B::Digit7,
        W::Digit8 => B::Digit8,
        W::Digit9 => B::Digit9,
        W::ArrowUp => B::ArrowUp,
        W::ArrowDown => B::ArrowDown,
        W::ArrowLeft => B::ArrowLeft,
        W::ArrowRight => B::ArrowRight,
        W::Home => B::Home,
        W::End => B::End,
        W::PageUp => B::PageUp,
        W::PageDown => B::PageDown,
        W::Enter => B::Enter,
        W::Escape => B::Escape,
        W::Backspace => B::Backspace,
        W::Tab => B::Tab,
        W::Space => B::Space,
        W::Delete => B::Delete,
        W::Insert => B::Insert,
        W::ShiftLeft => B::ShiftLeft,
        W::ShiftRight => B::ShiftRight,
        W::ControlLeft => B::ControlLeft,
        W::ControlRight => B::ControlRight,
        W::AltLeft => B::AltLeft,
        W::AltRight => B::AltRight,
        W::Comma => B::Comma,
        W::Period => B::Period,
        W::Semicolon => B::Semicolon,
        W::Quote => B::Quote,
        W::Slash => B::Slash,
        W::Backslash => B::Backslash,
        W::Minus => B::Minus,
        W::Equal => B::Equal,
        W::BracketLeft => B::BracketLeft,
        W::BracketRight => B::BracketRight,
        W::Backquote => B::Backquote,
        W::F1 => B::F1,
        W::F2 => B::F2,
        W::F3 => B::F3,
        W::F4 => B::F4,
        W::F5 => B::F5,
        W::F6 => B::F6,
        W::F7 => B::F7,
        W::F8 => B::F8,
        W::F9 => B::F9,
        W::F10 => B::F10,
        W::F11 => B::F11,
        W::F12 => B::F12,
        _ => B::Unknown,
    }
}

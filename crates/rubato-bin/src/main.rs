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

use rubato_game::LauncherStateFactory;
use rubato_game::core::bms_player_mode::BMSPlayerMode;
use rubato_game::core::config::DisplayMode;
use rubato_game::core::main_controller::MainController;
use rubato_game::core::version;
use rubato_render::egui_integration::EguiIntegration;
use rubato_render::gpu_context::GpuContext;
use rubato_render::gpu_texture_manager::GpuTextureManager;
use rubato_render::render_pipeline::SpriteRenderPipeline;

mod keymap;
mod subsystem_init;

use keymap::winit_to_bridge_keycode;

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

    /// Replay mode (1-4). Unlike Java beatoraja, bare `-r` without a value is not supported;
    /// use `-r1` or `--replay 1` explicitly (clap 4.x requires a value).
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

    let mut args = Args::parse();

    // Canonicalize BMS path before any CWD change so relative paths resolve
    // against the original working directory.
    if let Some(ref bms) = args.bms_path {
        if bms.is_relative() {
            if let Ok(abs) = bms.canonicalize() {
                args.bms_path = Some(abs);
            }
        }
    }

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
    let config_exists = {
        let cwd = std::env::current_dir().unwrap_or_default();
        match rubato_types::config::resolve_config_dir(&cwd) {
            Some(config_dir) => {
                // Anchor CWD to the resolved config root so all relative paths
                // (songpath, skinpath, etc.) resolve correctly when launched
                // from a subdirectory.
                if let Err(e) = std::env::set_current_dir(&config_dir) {
                    warn!("Failed to set CWD to config dir {:?}: {}", config_dir, e);
                }
                true
            }
            None => false,
        }
    };

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
    use rubato_game::core::main_loader::MainLoader;

    // Java: MainLoader.start(Stage) — reads config, creates PlayConfigurationView
    let (config, player, title) = MainLoader::start();

    // Java: primaryStage.setScene(scene); primaryStage.show();
    // eframe::run_native() blocks until the window is closed.
    let result = rubato_game::run_launcher(config, player, &title)?;

    // Known limitation: launcher actions (Load All BMS, etc.) are one-shot and exit the
    // process instead of returning to the launcher UI.
    if result.load_all_bms_requested {
        info!("Load All BMS requested, performing full song database scan...");
        subsystem_init::init_song_database_with_options(true);
        info!("Full song database scan complete.");
    }
    if result.load_diff_bms_requested {
        info!("Load Diff BMS requested, performing incremental song database scan...");
        subsystem_init::init_song_database_with_options(false);
        info!("Incremental song database scan complete.");
    }
    if result.import_score_requested {
        info!("Import Score requested, importing scores from LR2 database...");
        subsystem_init::import_lr2_scores(&result.config);
    }

    // Java: PlayConfigurationView.start() calls MainLoader.play()
    // Re-exec as a child process because winit does not allow creating a second
    // EventLoop in the same process (eframe already consumed the first one).
    if result.play_requested {
        info!("Launcher requested play, re-launching as child process...");
        let exe = std::env::current_exe()?;
        let status = spawn_child_with_timeout(exe, &["-s"])?;
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
    use rubato_game::core::main_loader::MainLoader;

    subsystem_init::init_song_database();

    // Java: MainLoader.play() handles config, illegal songs, player config, and controller creation.
    // It sets config.windowWidth/Height from resolution before creating MainController.
    let mut main_controller = MainLoader::play(bms_path, player_mode, true, None, None, false)?;

    subsystem_init::init_audio_driver(&mut main_controller)?;
    subsystem_init::init_song_information_database(&mut main_controller);

    // Set the state factory so that change_state() can create concrete state instances.
    // Without this, the controller has no factory and all state transitions silently fail,
    // resulting in a black screen.
    main_controller.set_state_factory(LauncherStateFactory::new().into_creator());

    let _listener_handles = subsystem_init::init_state_listeners(&mut main_controller);
    subsystem_init::init_ir_config(&mut main_controller);
    subsystem_init::init_download_processors(&mut main_controller);
    subsystem_init::init_stream_controller(&mut main_controller);

    // Wire modmenu with real PlayerConfig and command queue so UI changes propagate back
    rubato_game::state::modmenu::misc_setting_menu::MiscSettingMenu::set_player_config(
        main_controller.player_config().clone(),
        main_controller.config().clone(),
        main_controller.controller_command_queue(),
    );

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
        sprite_uniform_buffer: None,
        sprite_uniform_bind_group: None,
        egui_integration: None,
        egui_state: None,
        title,
        width: w as u32,
        height: h as u32,
        _vsync: vsync,
        display_mode,
        max_fps,
        last_frame_time: Instant::now(),
        fps_tracker: rubato_types::fps_counter::FpsTracker::new(),
        initialized: false,
        key_state,
        disposed: false,
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
    /// Persistent uniform buffer for sprite projection matrix (64 bytes, reused each frame).
    sprite_uniform_buffer: Option<wgpu::Buffer>,
    /// Bind group for the persistent uniform buffer. Only recreated if the buffer is recreated.
    sprite_uniform_bind_group: Option<wgpu::BindGroup>,
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
    /// FPS tracker for computing actual frame rate
    fps_tracker: rubato_types::fps_counter::FpsTracker,
    initialized: bool,
    /// Shared key state bridging winit keyboard events to the input system
    key_state: rubato_input::winit_input_bridge::SharedKeyState,
    /// Set after dispose() is called to prevent redraws on a disposed controller.
    /// Between event_loop.exit() and actual loop termination, about_to_wait can
    /// still fire; this flag gates redraw requests.
    disposed: bool,
}

impl ApplicationHandler for RubatoApp {
    /// Java: ApplicationListener.create() — called when the application is first created.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Populate monitor cache for VideoConfigurationView
        rubato_game::platform::update_monitors_from_winit(event_loop);

        // Sync display mode cache to core MainLoader
        {
            use rubato_game::core::main_loader::MainLoader;
            let modes = rubato_game::platform::cached_display_modes();
            if !modes.is_empty() {
                MainLoader::set_display_modes(modes);
            }
            let desktop = rubato_game::platform::cached_desktop_display_mode();
            if desktop != (0, 0) {
                MainLoader::set_desktop_display_mode(desktop);
            }
        }

        if self.window.is_none() && !self.create_window_and_gpu(event_loop) {
            return;
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
            // Skip game logic for events that egui has consumed. For keyboard
            // events, egui-winit sets consumed=true only when an egui widget
            // has keyboard focus (e.g. search text field, modmenu input).
            // RedrawRequested is never consumed by egui but exempted as a safety net.
            if response.consumed && !matches!(event, WindowEvent::RedrawRequested) {
                return;
            }
        }

        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(&event);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
                self.key_state
                    .set_mouse_position((position.x / scale) as i32, (position.y / scale) as i32);
                // If any mouse button is held, flag as drag so MainController
                // dispatches handle_skin_mouse_dragged() on the next render.
                if self
                    .key_state
                    .is_mouse_button_pressed(rubato_input::winit_input_bridge::MOUSE_BUTTON_LEFT)
                    || self.key_state.is_mouse_button_pressed(
                        rubato_input::winit_input_bridge::MOUSE_BUTTON_RIGHT,
                    )
                    || self.key_state.is_mouse_button_pressed(
                        rubato_input::winit_input_bridge::MOUSE_BUTTON_MIDDLE,
                    )
                {
                    self.key_state.set_mouse_dragged(true);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(state, button);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (x, y),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        (pos.x as f32 / 40.0, pos.y as f32 / 40.0)
                    }
                };
                self.key_state.add_scroll(dx, dy);
            }
            WindowEvent::CloseRequested => {
                self.disposed = true;
                self.controller.dispose();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.handle_resize(size);
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw();
            }
            _ => {}
        }
    }

    /// Called when the event loop is about to wait for new events.
    /// Request continuous redraws to match the Java game loop behavior.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.disposed {
            return;
        }
        // Java: MainController checks exit flag and calls Platform.exit()
        if self.controller.is_exit_requested() {
            self.disposed = true;
            self.controller.dispose();
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
    // -----------------------------------------------------------------------
    // Window & GPU initialization
    // -----------------------------------------------------------------------

    /// Create the application window and initialize GPU context.
    /// Returns false if initialization failed and the event loop should exit.
    fn create_window_and_gpu(&mut self, event_loop: &ActiveEventLoop) -> bool {
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
                window_attributes = window_attributes
                    .with_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(monitor))));
            }
        } else if let Some(ref monitor) = target_monitor {
            // Java: windowed mode — position at target monitor origin
            let pos = monitor.position();
            window_attributes =
                window_attributes.with_position(winit::dpi::PhysicalPosition::new(pos.x, pos.y));
        }

        match event_loop.create_window(window_attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                if !self.init_gpu(event_loop, &window) {
                    return false;
                }
                self.window = Some(window);
                true
            }
            Err(e) => {
                error!("Failed to create window: {}", e);
                event_loop.exit();
                false
            }
        }
    }

    /// Initialize wgpu GPU context, sprite pipeline, egui, and texture manager.
    fn init_gpu(&mut self, event_loop: &ActiveEventLoop, window: &Arc<Window>) -> bool {
        // wgpu SurfaceConfiguration expects physical pixels, not logical.
        let physical = window.inner_size();
        match pollster::block_on(GpuContext::new_with_surface(
            Arc::clone(window),
            physical.width,
            physical.height,
        )) {
            Ok(gpu) => {
                info!("wgpu GPU context created successfully");

                // Create sprite render pipeline for skin object rendering
                // wgpu replaces LibGDX ShaderManager; pipelines are created directly
                let sprite_pipeline = SpriteRenderPipeline::new(&gpu.device, gpu.surface_format());
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
                let egui_integration = EguiIntegration::new(&gpu.device, gpu.surface_format());
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
                true
            }
            Err(e) => {
                error!("Failed to create GPU context: {}", e);
                event_loop.exit();
                false
            }
        }
    }

    // -----------------------------------------------------------------------
    // Input event handlers
    // -----------------------------------------------------------------------

    /// Bridge winit keyboard events to the input system.
    fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) {
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

    /// Bridge winit mouse button events to SharedKeyState.
    fn handle_mouse_input(
        &mut self,
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) {
        let btn = match button {
            winit::event::MouseButton::Left => rubato_input::winit_input_bridge::MOUSE_BUTTON_LEFT,
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

    /// Handle window resize: update GPU surface and game coordinate space.
    fn handle_resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
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

    // -----------------------------------------------------------------------
    // Render pipeline
    // -----------------------------------------------------------------------

    /// Main frame render: game logic, egui overlay, wgpu present.
    fn handle_redraw(&mut self) {
        if self.disposed {
            return;
        }
        // FPS capping
        // Java: gdxConfig.setForegroundFPS(config.getMaxFramePerSecond())
        if self.max_fps > 0 {
            let target_frame_duration = Duration::from_secs_f64(1.0 / self.max_fps as f64);
            let elapsed = self.last_frame_time.elapsed();
            if elapsed < target_frame_duration {
                std::thread::sleep(target_frame_duration - elapsed);
            }
        }
        self.last_frame_time = Instant::now();
        self.fps_tracker.tick();

        // Game logic update (timer, state render, sprite batch begin/end, input)
        self.controller.render();

        // Clone Arc<Window> and temporarily take GpuContext out of self to avoid
        // holding immutable borrows on self across &mut self method calls.
        let Some(window) = self.window.clone() else {
            return;
        };
        let Some(gpu) = self.gpu.take() else {
            return;
        };

        // Wrap in catch_unwind so that gpu is restored even if a panic occurs
        // between take and put-back (panic safety, same pattern as sprite
        // take/put-back in lifecycle.rs).
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Process window commands from MainController
            if rubato_game::core::window_command::take_fullscreen_toggle() {
                if window.fullscreen().is_some() {
                    window.set_fullscreen(None);
                } else {
                    let monitor = window.current_monitor();
                    window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(monitor)));
                }
            }
            let screenshot_requested = rubato_game::core::window_command::take_screenshot_request();

            let full_output = self.run_egui_frame(&window);

            self.submit_gpu_frame(&gpu, &window, full_output, screenshot_requested);
        })) {
            Ok(()) => {}
            Err(payload) => {
                self.gpu = Some(gpu);
                std::panic::resume_unwind(payload);
            }
        }

        // Put gpu back
        self.gpu = Some(gpu);

        window.request_redraw();
    }

    /// Run the egui frame: gather input, render UI overlay, return output.
    fn run_egui_frame(&mut self, window: &Window) -> Option<egui::FullOutput> {
        // Gather diagnostic info before egui frame (avoids borrow conflicts)
        let diag_state_type = self.controller.current_state_type();
        let diag_has_skin = self
            .controller
            .current_state()
            .map(|s| s.main_state_data().skin.is_some())
            .unwrap_or(false);

        let (Some(egui_state), Some(egui_integration)) =
            (&mut self.egui_state, &self.egui_integration)
        else {
            return None;
        };

        // Java: ImGuiRenderer.start() → ImGuiRenderer.render() → ImGuiRenderer.end()
        let raw_input = egui_state.take_egui_input(window);
        let full_output = egui_integration.ctx.run(raw_input, |ctx| {
            rubato_game::state::modmenu::imgui_renderer::ImGuiRenderer::render_ui(ctx);

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
                            ui.colored_label(egui::Color32::WHITE, format!("State: {}", state_str));
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
    }

    /// Submit the GPU frame: sprite batch, egui overlay, present.
    fn submit_gpu_frame(
        &mut self,
        gpu: &GpuContext,
        window: &Window,
        full_output: Option<egui::FullOutput>,
        screenshot_requested: bool,
    ) {
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
                let sprite_ready = self.prepare_sprite_resources(gpu, &mut encoder);

                // Build GpuRenderContext before render_pass so it outlives the pass
                let gpu_ctx = if sprite_ready
                    && let Some(ref uniform_bind_group) = self.sprite_uniform_bind_group
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

                // Render pass: clear screen + SpriteBatch GPU flush
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

                // Capture screenshot after render pass, before present
                if screenshot_requested {
                    self.capture_screenshot(gpu, &output.texture);
                }

                output.present();

                // Evict GPU textures not used this frame (e.g., stale BGA video frames)
                if let Some(texture_manager) = &mut self.texture_manager {
                    texture_manager.evict_unused();
                }
            }
            Err(e) => {
                warn!("Failed to get surface texture: {}", e);
            }
        }
    }

    /// Upload pending textures and update sprite batch uniform resources.
    /// Returns `true` if sprite resources are ready for rendering.
    /// The uniform buffer and bind group are stored persistently on `self`
    /// and reused across frames; only the projection data is re-uploaded.
    fn prepare_sprite_resources(
        &mut self,
        gpu: &GpuContext,
        _encoder: &mut wgpu::CommandEncoder,
    ) -> bool {
        let sprite_pipeline = match self.sprite_pipeline.as_ref() {
            Some(p) => p,
            None => return false,
        };
        let texture_manager = match self.texture_manager.as_mut() {
            Some(t) => t,
            None => return false,
        };
        let sprite_batch = match self.controller.sprite_batch_mut() {
            Some(b) => b,
            None => return false,
        };
        if !sprite_batch.has_pending_draw_data() {
            return false;
        }

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

        // Reuse persistent uniform buffer; create once on first use
        let projection_data = sprite_batch.projection();
        if self.sprite_uniform_buffer.is_none() {
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite uniform buffer"),
                size: 64, // 4x4 f32 matrix
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sprite uniform bind group"),
                layout: &sprite_pipeline.uniform_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            self.sprite_uniform_buffer = Some(buffer);
            self.sprite_uniform_bind_group = Some(bind_group);
        }
        let uniform_buffer = self.sprite_uniform_buffer.as_ref().unwrap();
        gpu.queue
            .write_buffer(uniform_buffer, 0, bytemuck::cast_slice(projection_data));

        true
    }

    /// Capture the rendered frame and save as a PNG screenshot.
    /// Must be called after the render pass with the rendered texture.
    fn capture_screenshot(&self, gpu: &GpuContext, texture: &wgpu::Texture) {
        let Some(ref surface_config) = gpu.surface_config else {
            warn!("Cannot capture screenshot: no surface config");
            return;
        };
        let width = surface_config.width;
        let height = surface_config.height;
        if width == 0 || height == 0 {
            warn!("Cannot capture screenshot: surface size is 0");
            return;
        }

        // Create a buffer to read pixels from the rendered texture
        let bytes_per_row = (width * 4 + 255) & !255; // align to 256 bytes
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot_buffer"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("screenshot_encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
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
        // Accepted single-frame stall: screenshots are user-triggered one-at-a-time operations.
        // Synchronous poll + recv keeps the implementation simple with negligible UX impact.
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

                // On BGRA backends, swap R and B channels so the PNG has correct colors.
                if surface_config.format == wgpu::TextureFormat::Bgra8Unorm
                    || surface_config.format == wgpu::TextureFormat::Bgra8UnormSrgb
                {
                    for pixel in rgba.chunks_exact_mut(4) {
                        pixel.swap(0, 2);
                    }
                }

                // Save as PNG
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let path = format!("screenshot_{}.png", timestamp);
                if let Some(img) = image::RgbaImage::from_raw(width, height, rgba) {
                    match img.save(&path) {
                        Ok(()) => {
                            info!("Screenshot saved: {}", path);
                            self.post_screenshot_actions(&path);
                        }
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

    /// Run post-screenshot actions: clipboard copy and webhook send if configured.
    fn post_screenshot_actions(&self, path: &str) {
        let config = self.controller.config();

        // Clipboard copy
        if config.integration.set_clipboard_screenshot {
            match rubato_game::external::clipboard_helper::ClipboardHelper::copy_image_to_clipboard(
                path,
            ) {
                Ok(()) => info!("Screenshot copied to clipboard"),
                Err(e) => warn!("Failed to copy screenshot to clipboard: {}", e),
            }
        }

        // Webhook send
        if config.integration.webhook_option != 0 && !config.integration.webhook_url.is_empty() {
            let webhook_urls = config.integration.webhook_url.clone();
            let webhook_name = if config.integration.webhook_name.is_empty() {
                "Endless Dream".to_string()
            } else {
                config.integration.webhook_name.clone()
            };
            let webhook_avatar = config.integration.webhook_avatar.clone();

            // Build a minimal payload (state-aware rich embed requires MainState wiring)
            let payload = serde_json::json!({
                "username": webhook_name,
                "avatar_url": webhook_avatar,
            });
            let payload_str = payload.to_string();
            let path = path.to_string();

            std::thread::spawn(move || {
                let handler = rubato_game::external::webhook_handler::WebhookHandler::new();
                for webhook_url in &webhook_urls {
                    handler.send_webhook_with_image(&payload_str, &path, webhook_url);
                }
            });
        }
    }
}

// -- Download processor adapter structs --

/// Adapter: bridges `SQLiteSongDatabaseAccessor` to `rubato_game::song::md_processor::MusicDatabaseAccessor`.
///
/// Java equivalent: the inline lambda `(md5) -> { SongData[] s = getSongDatabase().getSongDatas(md5); ... }`
/// in MainController.create() line 497. Opens its own SQLite connection so the IPFS download
/// background thread can query the song DB without borrowing MainController.
pub(crate) struct SongDbMusicDatabaseAdapter {
    pub(crate) songdb: rubato_game::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor,
    pub(crate) bmsroot: Vec<String>,
}

// SAFETY: SongDbMusicDatabaseAdapter contains SQLiteSongDatabaseAccessor which wraps
// rusqlite::Connection behind an internal Mutex. Connection is !Sync, but the Mutex
// serializes all access, making it safe to share across threads.
unsafe impl Sync for SongDbMusicDatabaseAdapter {}

impl rubato_game::song::md_processor::music_database_accessor::MusicDatabaseAccessor
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

    fn update_song(&self, path: &str) {
        let update_path = if path.is_empty() { None } else { Some(path) };
        self.songdb
            .update_song_datas(update_path, &self.bmsroot, false, false, None);
    }
}

/// Adapter: bridges a standalone song DB connection to `rubato_game::song::md_processor::MainControllerRef`.
///
/// Java equivalent: `this` (MainController) passed to HttpDownloadProcessor constructor.
/// The only method called is `update_song(path, force)` which ultimately calls
/// `songdb.updateSongDatas()`. We call it directly on our own connection.
pub(crate) struct SongDbMainControllerRef {
    pub(crate) songdb: rubato_game::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor,
    pub(crate) bmsroot: Vec<String>,
    pub(crate) info_db: Option<Box<dyn rubato_types::song_information_db::SongInformationDb>>,
}

// SAFETY: SongDbMainControllerRef contains SQLiteSongDatabaseAccessor which wraps
// rusqlite::Connection behind an internal Mutex, and a Vec<String> (bmsroot) which is
// inherently Sync. The info_db field is Box<dyn SongInformationDb> which requires Send + Sync.
// The Mutex serializes all DB access, making it safe to share across threads.
unsafe impl Sync for SongDbMainControllerRef {}

impl rubato_game::song::md_processor::MainControllerRef for SongDbMainControllerRef {
    fn update_song(&self, path: &str, _force: bool) {
        let update_path = if path.is_empty() { None } else { Some(path) };
        self.songdb.update_song_datas(
            update_path,
            &self.bmsroot,
            false,
            false,
            self.info_db.as_deref(),
        );
    }
}

/// Wrapper to implement `HttpDownloadSubmitter` for `Arc<HttpDownloadProcessor>`.
///
/// `DownloadTaskMenu::set_processor` needs `Arc<HttpDownloadProcessor>` directly, while
/// `MainController::set_http_download_processor` needs `Box<dyn HttpDownloadSubmitter>`.
/// This wrapper bridges the two ownership models.
pub(crate) struct HttpDownloadProcessorWrapper(
    pub(crate) Arc<rubato_game::song::md_processor::http_download_processor::HttpDownloadProcessor>,
);

impl rubato_types::http_download_submitter::HttpDownloadSubmitter for HttpDownloadProcessorWrapper {
    fn submit_md5_task(&self, md5: &str, task_name: &str) {
        self.0.submit_md5_task(md5, task_name);
    }
}

/// Spawn a child process and wait for it with an optional timeout.
///
/// When `RUBATO_CHILD_TIMEOUT_SECS` is set, the child is killed after that many seconds.
/// This prevents hangs in headless/CI environments where the GUI event loop may never exit.
/// In normal interactive use, the env var is unset and the wait is unbounded.
fn spawn_child_with_timeout(exe: PathBuf, args: &[&str]) -> Result<std::process::ExitStatus> {
    let mut child = std::process::Command::new(&exe).args(args).spawn()?;

    let timeout_secs: Option<u64> = std::env::var("RUBATO_CHILD_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok());

    match timeout_secs {
        Some(secs) => {
            let deadline = Instant::now() + Duration::from_secs(secs);
            loop {
                if let Some(status) = child.try_wait()? {
                    return Ok(status);
                }
                if Instant::now() >= deadline {
                    warn!("Child process did not exit within {}s, killing", secs);
                    child.kill()?;
                    return child.wait().map_err(Into::into);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        None => child.wait().map_err(Into::into),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Known limitation: tests use Unix-specific commands (true/false/sleep).
    // This project targets macOS only.
    #[test]
    fn spawn_child_with_timeout_succeeds_for_fast_command() {
        // `true` exits immediately with status 0
        let status = spawn_child_with_timeout(PathBuf::from("true"), &[])
            .expect("should spawn successfully");
        assert!(status.success());
    }

    #[test]
    fn spawn_child_with_timeout_reports_failure() {
        let status = spawn_child_with_timeout(PathBuf::from("false"), &[])
            .expect("should spawn successfully");
        assert!(!status.success());
    }

    #[test]
    fn spawn_child_with_timeout_kills_on_timeout() {
        // Set a 1-second timeout and run `sleep 60` which would otherwise block
        // SAFETY: This test is single-threaded and the env var is only used by spawn_child_with_timeout.
        unsafe { std::env::set_var("RUBATO_CHILD_TIMEOUT_SECS", "1") };
        let start = Instant::now();
        let status = spawn_child_with_timeout(PathBuf::from("sleep"), &["60"])
            .expect("should spawn successfully");
        let elapsed = start.elapsed();

        // SAFETY: This test is single-threaded and the env var is only used by spawn_child_with_timeout.
        unsafe { std::env::remove_var("RUBATO_CHILD_TIMEOUT_SECS") };

        // Should have been killed, not waited 60 seconds
        assert!(
            elapsed.as_secs() < 10,
            "should have been killed by timeout, but took {:?}",
            elapsed
        );
        // Killed processes have non-success status
        assert!(!status.success());
    }

    #[test]
    fn spawn_child_with_timeout_invalid_exe_returns_error() {
        let result = spawn_child_with_timeout(PathBuf::from("/nonexistent/binary/path"), &[]);
        assert!(result.is_err());
    }
}

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use log::{error, info, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use beatoraja_core::bms_player_mode::BMSPlayerMode;
use beatoraja_core::config::{Config, DisplayMode};
use beatoraja_core::main_controller::MainController;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_core::version;
use beatoraja_render::egui_integration::EguiIntegration;
use beatoraja_render::gpu_context::GpuContext;

/// LR2oraja Endless Dream - BMS player
#[derive(Parser, Debug)]
#[command(
    name = "beatoraja",
    version,
    about = "LR2oraja Endless Dream - BMS player"
)]
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
    env_logger::init();

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

/// Java: MainLoader.launch() — opens the launcher/configuration UI.
fn launch() -> Result<()> {
    let config = Config::read().unwrap_or_else(|_| Config::default());
    let player = {
        let playerpath = &config.playerpath;
        let playername = config.playername.as_deref().unwrap_or("default");
        PlayerConfig::read_player_config(playerpath, playername)
            .unwrap_or_else(|_| PlayerConfig::default())
    };

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = LauncherApp {
        window: None,
        gpu: None,
        egui_integration: None,
        egui_state: None,
        launcher: beatoraja_launcher::LauncherUi::new(config, player),
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}

/// Launcher application handler — standalone egui window for configuration.
///
/// Java equivalent: JavaFX Application.start() → PlayConfigurationView
struct LauncherApp {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    egui_integration: Option<EguiIntegration>,
    egui_state: Option<egui_winit::State>,
    launcher: beatoraja_launcher::LauncherUi,
}

impl ApplicationHandler for LauncherApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Populate monitor cache for VideoConfigurationView
        beatoraja_launcher::stubs::update_monitors_from_winit(event_loop);

        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("beatoraja Configuration")
            .with_inner_size(winit::dpi::LogicalSize::new(1000u32, 700u32));

        match event_loop.create_window(window_attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                match pollster::block_on(GpuContext::new_with_surface(
                    Arc::clone(&window),
                    1000,
                    700,
                )) {
                    Ok(gpu) => {
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
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Forward events to egui
        if let Some(state) = &mut self.egui_state
            && let Some(window) = &self.window
        {
            let response = state.on_window_event(window, &event);
            if response.consumed {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(window) = &self.window else { return };
                let Some(gpu) = &self.gpu else { return };
                let Some(egui_state) = &mut self.egui_state else {
                    return;
                };
                let Some(egui_integration) = &mut self.egui_integration else {
                    return;
                };

                let raw_input = egui_state.take_egui_input(window);
                let full_output = egui_integration.ctx.run(raw_input, |ctx| {
                    self.launcher.render_ui(ctx);
                });
                egui_state.handle_platform_output(window, full_output.platform_output.clone());

                match gpu.get_current_texture() {
                    Ok(output) => {
                        let view = output
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        let mut encoder =
                            gpu.device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("launcher frame encoder"),
                                });
                        // Clear screen
                        {
                            let _rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("launcher clear pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color {
                                            r: 0.15,
                                            g: 0.15,
                                            b: 0.15,
                                            a: 1.0,
                                        }),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });
                        }

                        let screen_descriptor = egui_wgpu::ScreenDescriptor {
                            size_in_pixels: [
                                gpu.surface_config.as_ref().map_or(1000, |c| c.width),
                                gpu.surface_config.as_ref().map_or(700, |c| c.height),
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

                        gpu.queue.submit(std::iter::once(encoder.finish()));
                        output.present();
                    }
                    Err(e) => {
                        warn!("Failed to get surface texture: {}", e);
                    }
                }

                window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}
}

/// Java: MainLoader.play() — creates MainController and launches the application window.
fn play(bms_path: Option<PathBuf>, player_mode: Option<BMSPlayerMode>) -> Result<()> {
    // Java: config = Config.read()
    let config = Config::read().unwrap_or_else(|e| {
        error!("Config read failed: {}", e);
        Config::default()
    });

    // Java: player = PlayerConfig.readPlayerConfig(config.getPlayerpath(), playername)
    let player = {
        let playerpath = &config.playerpath;
        let playername = config.playername.as_deref().unwrap_or("default");
        PlayerConfig::read_player_config(playerpath, playername).unwrap_or_else(|e| {
            error!("Player config read failed: {}", e);
            PlayerConfig::default()
        })
    };

    // Java: final int w = config.getResolution().width; final int h = config.getResolution().height;
    let w = config.resolution.width();
    let h = config.resolution.height();
    let vsync = config.vsync;
    let display_mode = config.displaymode.clone();
    let title = version::version_long().to_string();

    // Java: MainController main = new MainController(bmsPath, config, player, playerMode, songUpdated)
    let main_controller = MainController::new(bms_path, config, player, player_mode, false);

    info!("Starting {}", version::version_long());
    if let Some(hash) = version::get_git_commit_hash() {
        info!("[Build info] Commit: {}", hash);
    }
    if let Some(date) = version::get_build_date() {
        info!("[Build info] Build date: {}", date);
    }

    // Java: new Lwjgl3Application(new ApplicationListener() { ... }, gdxConfig)
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = BeatorajaApp {
        controller: main_controller,
        window: None,
        gpu: None,
        egui_integration: None,
        egui_state: None,
        title,
        width: w as u32,
        height: h as u32,
        _vsync: vsync,
        display_mode,
        initialized: false,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}

/// Application handler that bridges winit events to MainController lifecycle.
///
/// Java equivalent: the anonymous ApplicationListener passed to Lwjgl3Application
/// with create(), render(), resize(), pause(), resume(), dispose() callbacks.
struct BeatorajaApp {
    controller: MainController,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    egui_integration: Option<EguiIntegration>,
    egui_state: Option<egui_winit::State>,
    title: String,
    width: u32,
    height: u32,
    _vsync: bool,
    display_mode: DisplayMode,
    initialized: bool,
}

impl ApplicationHandler for BeatorajaApp {
    /// Java: ApplicationListener.create() — called when the application is first created.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Populate monitor cache for VideoConfigurationView
        beatoraja_launcher::stubs::update_monitors_from_winit(event_loop);

        if self.window.is_none() {
            // Java: gdxConfig.setWindowedMode(w, h); gdxConfig.setTitle(MainController.getVersion())
            let decorated = !matches!(
                self.display_mode,
                DisplayMode::FULLSCREEN | DisplayMode::BORDERLESS
            );
            let window_attributes = Window::default_attributes()
                .with_title(&self.title)
                .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height))
                .with_decorations(decorated);

            match event_loop.create_window(window_attributes) {
                Ok(window) => {
                    if matches!(self.display_mode, DisplayMode::FULLSCREEN) {
                        // Java: Gdx.graphics.setFullscreenMode(finalGdxDisplayMode)
                        warn!("Fullscreen mode requested but not yet implemented");
                    }
                    let window = Arc::new(window);

                    // Create wgpu GPU context bound to this window's surface
                    match pollster::block_on(GpuContext::new_with_surface(
                        Arc::clone(&window),
                        self.width,
                        self.height,
                    )) {
                        Ok(gpu) => {
                            info!("wgpu GPU context created successfully");

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
            if response.consumed {
                return;
            }
        }

        match event {
            // Java: dispose() is called when the window is closed
            WindowEvent::CloseRequested => {
                self.controller.dispose();
                event_loop.exit();
            }
            // Java: main.resize(width, height)
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                self.controller
                    .resize(size.width as i32, size.height as i32);
            }
            // Java: main.render() — called every frame via ApplicationListener.render()
            WindowEvent::RedrawRequested => {
                // Game logic update
                self.controller.render();

                let Some(window) = &self.window else { return };
                let Some(gpu) = &self.gpu else { return };

                // Run egui frame
                // Java: ImGuiRenderer.start() → ImGuiRenderer.render() → ImGuiRenderer.end()
                let full_output = if let (Some(egui_state), Some(egui_integration)) =
                    (&mut self.egui_state, &self.egui_integration)
                {
                    let raw_input = egui_state.take_egui_input(window);
                    let full_output = egui_integration.ctx.run(raw_input, |ctx| {
                        beatoraja_modmenu::imgui_renderer::ImGuiRenderer::render_ui(ctx);
                    });
                    egui_state.handle_platform_output(window, full_output.platform_output.clone());
                    Some(full_output)
                } else {
                    None
                };

                // wgpu render pass: clear screen and present
                match gpu.get_current_texture() {
                    Ok(output) => {
                        let view = output
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        let mut encoder =
                            gpu.device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("beatoraja frame encoder"),
                                });
                        // Clear screen with black; SpriteBatch draw calls will be added here
                        {
                            let _render_pass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("beatoraja render pass"),
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
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Java: main.pause()
        self.controller.pause();
    }
}

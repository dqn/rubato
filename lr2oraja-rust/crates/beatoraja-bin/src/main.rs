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
use winit::window::{Window, WindowId};

use beatoraja_core::bms_player_mode::BMSPlayerMode;
use beatoraja_core::config::DisplayMode;
use beatoraja_core::main_controller::MainController;
use beatoraja_core::version;
use beatoraja_render::egui_integration::EguiIntegration;
use beatoraja_render::gpu_context::GpuContext;
use beatoraja_render::render_pipeline::SpriteRenderPipeline;

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
    use beatoraja_core::main_loader::MainLoader;

    // Java: MainLoader.start(Stage) — reads config, creates PlayConfigurationView
    let (config, player, title) = MainLoader::start();

    // Java: primaryStage.setScene(scene); primaryStage.show();
    // eframe::run_native() blocks until the window is closed.
    let result = beatoraja_launcher::run_launcher(config, player, &title)?;

    // Java: PlayConfigurationView.start() calls MainLoader.play()
    if result.play_requested {
        info!("Launcher requested play, starting game...");
        play(None, Some(BMSPlayerMode::PLAY))?;
    }

    Ok(())
}

/// Java: MainLoader.play() — creates MainController and launches the application window.
///
/// Delegates to MainLoader::play() for Config reading, illegal songs check,
/// PlayerConfig reading, and MainController creation. Then creates the winit
/// EventLoop + wgpu context for the render loop.
fn play(bms_path: Option<PathBuf>, player_mode: Option<BMSPlayerMode>) -> Result<()> {
    use beatoraja_core::main_loader::MainLoader;

    // Java: MainLoader.play() handles config, illegal songs, player config, and controller creation.
    // It sets config.windowWidth/Height from resolution before creating MainController.
    let mut main_controller = MainLoader::play(bms_path, player_mode, true, None, None, false);

    // Java: if(config.isUseDiscordRPC()) { stateListener.add(new DiscordListener()); }
    {
        let (use_discord_rpc, use_obs_ws, cfg_clone) = {
            let cfg = main_controller.get_config();
            (cfg.use_discord_rpc, cfg.use_obs_ws, cfg.clone())
        };
        if use_discord_rpc {
            let listener = beatoraja_external::discord_listener::DiscordListener::new();
            main_controller.add_state_listener(Box::new(listener));
        }
        if use_obs_ws {
            let listener = beatoraja_obs::obs_listener::ObsListener::new(cfg_clone);
            main_controller.add_state_listener(Box::new(listener));
        }
    }

    // Extract window config from the controller's Config
    // Java: these were set by MainLoader.play() → config.setWindowWidth/Height
    let config = main_controller.get_config();
    let w = config.window_width;
    let h = config.window_height;
    let vsync = config.vsync;
    let display_mode = config.displaymode.clone();
    let max_fps = config.max_frame_per_second;
    // Java: gdxConfig.setTitle(MainController.getVersion())
    let title = version::version_long().to_string();

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
        sprite_pipeline: None,
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
    /// Sprite render pipeline for skin object rendering (Phase 22a)
    sprite_pipeline: Option<SpriteRenderPipeline>,
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
}

impl ApplicationHandler for BeatorajaApp {
    /// Java: ApplicationListener.create() — called when the application is first created.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Populate monitor cache for VideoConfigurationView
        beatoraja_launcher::stubs::update_monitors_from_winit(event_loop);

        // Sync display mode cache to core MainLoader
        {
            use beatoraja_core::main_loader::MainLoader;
            let modes = beatoraja_launcher::stubs::get_cached_display_modes();
            if !modes.is_empty() {
                MainLoader::set_display_modes(modes);
            }
            let desktop = beatoraja_launcher::stubs::get_cached_desktop_display_mode();
            if desktop != (0, 0) {
                MainLoader::set_desktop_display_mode(desktop);
            }
        }

        if self.window.is_none() {
            // Java: Find target monitor by config.monitorName
            // Format: "MonitorName [virtualX, virtualY]"
            let config = self.controller.get_config();
            let monitor_name = config.monitor_name.clone();

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

                    // Create wgpu GPU context bound to this window's surface
                    match pollster::block_on(GpuContext::new_with_surface(
                        Arc::clone(&window),
                        self.width,
                        self.height,
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

                // wgpu render pass: clear screen, sprite batch flush, egui overlay, present
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

                        // Prepare sprite batch resources before the render pass
                        // (bind groups must outlive the render pass)
                        let sprite_resources = if let Some(sprite_pipeline) = &self.sprite_pipeline
                            && let Some(sprite_batch) = self.controller.get_sprite_batch_mut()
                            && !sprite_batch.vertices().is_empty()
                        {
                            // Create a dummy 1x1 white texture for untextured sprites
                            // Phase 22+: Real texture management will provide proper bind groups
                            let dummy_texture =
                                gpu.device.create_texture(&wgpu::TextureDescriptor {
                                    label: Some("dummy white texture"),
                                    size: wgpu::Extent3d {
                                        width: 1,
                                        height: 1,
                                        depth_or_array_layers: 1,
                                    },
                                    mip_level_count: 1,
                                    sample_count: 1,
                                    dimension: wgpu::TextureDimension::D2,
                                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                                        | wgpu::TextureUsages::COPY_DST,
                                    view_formats: &[],
                                });
                            gpu.queue.write_texture(
                                wgpu::TexelCopyTextureInfo {
                                    texture: &dummy_texture,
                                    mip_level: 0,
                                    origin: wgpu::Origin3d::ZERO,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                &[255u8, 255, 255, 255],
                                wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(4),
                                    rows_per_image: Some(1),
                                },
                                wgpu::Extent3d {
                                    width: 1,
                                    height: 1,
                                    depth_or_array_layers: 1,
                                },
                            );
                            let dummy_view =
                                dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());
                            let sampler =
                                sprite_pipeline.get_sampler(sprite_batch.get_shader_type());

                            let texture_bind_group =
                                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                    label: Some("sprite texture bind group"),
                                    layout: &sprite_pipeline.texture_layout,
                                    entries: &[
                                        wgpu::BindGroupEntry {
                                            binding: 0,
                                            resource: wgpu::BindingResource::TextureView(
                                                &dummy_view,
                                            ),
                                        },
                                        wgpu::BindGroupEntry {
                                            binding: 1,
                                            resource: wgpu::BindingResource::Sampler(sampler),
                                        },
                                    ],
                                });

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

                            Some((uniform_bind_group, texture_bind_group))
                        } else {
                            None
                        };

                        // Render pass: clear screen + SpriteBatch GPU flush
                        // Java: Gdx.gl.glClear(GL20.GL_COLOR_BUFFER_BIT) + SpriteBatch draw
                        {
                            let mut render_pass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("beatoraja sprite pass"),
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
                            if let Some((ref uniform_bind_group, ref texture_bind_group)) =
                                sprite_resources
                                && let Some(sprite_pipeline) = &self.sprite_pipeline
                                && let Some(sprite_batch) = self.controller.get_sprite_batch_mut()
                            {
                                sprite_batch.flush_to_gpu(
                                    &mut render_pass,
                                    &gpu.device,
                                    &gpu.queue,
                                    sprite_pipeline,
                                    uniform_bind_group,
                                    texture_bind_group,
                                );
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

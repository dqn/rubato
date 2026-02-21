use std::path::PathBuf;

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
        todo!("Phase 13 dependency: egui launcher UI")
    }

    Ok(())
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
    // Replaced with winit event loop (Bevy rendering integration in Phase 13)
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = BeatorajaApp {
        controller: main_controller,
        window: None,
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
    window: Option<Window>,
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
                        // Phase 13: actual fullscreen mode via winit/bevy
                        warn!("Fullscreen mode requested but not yet implemented (Phase 13)");
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
        match event {
            // Java: dispose() is called when the window is closed
            WindowEvent::CloseRequested => {
                self.controller.dispose();
                event_loop.exit();
            }
            // Java: main.resize(width, height)
            WindowEvent::Resized(size) => {
                self.controller
                    .resize(size.width as i32, size.height as i32);
            }
            // Java: main.render() — called every frame via ApplicationListener.render()
            WindowEvent::RedrawRequested => {
                self.controller.render();
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

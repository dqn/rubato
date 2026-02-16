mod app;
pub mod monitor;
pub mod panel;
pub mod panels;
mod tab;
mod widgets;

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use bms_config::{Config, PlayerConfig};

/// Launch the settings GUI. Returns updated configs when user clicks "Start Game".
/// Returns None if user cancelled.
pub fn run_launcher(
    config_path: &Path,
    player_config_path: &Path,
) -> Result<Option<(Config, PlayerConfig)>> {
    let config = Config::read(config_path).unwrap_or_default();
    let player_config = PlayerConfig::read(player_config_path).unwrap_or_default();

    let result: Arc<Mutex<Option<(Config, PlayerConfig)>>> = Arc::new(Mutex::new(None));
    let result_clone = result.clone();

    let cfg_path = config_path.to_path_buf();
    let pcfg_path = player_config_path.to_path_buf();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("brs Launcher"),
        ..Default::default()
    };

    eframe::run_native(
        "brs Launcher",
        options,
        Box::new(move |_cc| {
            let inner = app::LauncherApp::new(config, player_config, cfg_path, pcfg_path);
            Ok(Box::new(AppWrapper {
                inner,
                result: result_clone,
            }))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    let out = result.lock().unwrap().take();
    Ok(out)
}

/// Wrapper to capture final state when window closes.
struct AppWrapper {
    inner: app::LauncherApp,
    result: Arc<Mutex<Option<(Config, PlayerConfig)>>>,
}

impl eframe::App for AppWrapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.inner.update(ctx, frame);

        if self.inner.should_start_game {
            *self.result.lock().unwrap() =
                Some((self.inner.config.clone(), self.inner.player_config.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use bms_config::{Config, PlayerConfig};

    #[test]
    fn launcher_default_config() {
        let config = Config::default();
        let player_config = PlayerConfig::default();
        assert_eq!(config.window_width, 1280);
        assert_eq!(player_config.name, "NO NAME");
    }
}

//! Settings launcher GUI built on egui/eframe.
//!
//! Provides [`run_launcher`] as the entry point, which opens a native window
//! with tabbed panels for video, audio, input, skin, IR, and other settings.
//! Returns updated [`bms_config::Config`] and [`bms_config::PlayerConfig`] when
//! the user clicks "Start Game", or `None` if cancelled.

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
    use std::collections::HashSet;

    use bms_config::{Config, PlayerConfig};

    use crate::panel::LauncherPanel;
    use crate::panels::audio::AudioPanel;
    use crate::panels::discord::DiscordPanel;
    use crate::panels::input::InputPanel;
    use crate::panels::ir::IrPanel;
    use crate::panels::music_select::MusicSelectPanel;
    use crate::panels::obs::ObsPanel;
    use crate::panels::play_option::PlayOptionPanel;
    use crate::panels::resource::ResourcePanel;
    use crate::panels::skin::SkinPanel;
    use crate::panels::stream::StreamPanel;
    use crate::panels::table_editor::TableEditorPanel;
    use crate::panels::video::VideoPanel;
    use crate::tab::Tab;

    #[test]
    fn launcher_default_config() {
        let config = Config::default();
        let player_config = PlayerConfig::default();
        assert_eq!(config.window_width, 1280);
        assert_eq!(player_config.name, "NO NAME");
    }

    /// Helper: create all concrete panel instances.
    fn all_panels() -> Vec<Box<dyn LauncherPanel>> {
        vec![
            Box::new(VideoPanel::default()),
            Box::new(AudioPanel::default()),
            Box::new(InputPanel::default()),
            Box::new(ResourcePanel::default()),
            Box::new(MusicSelectPanel::default()),
            Box::new(PlayOptionPanel::default()),
            Box::new(SkinPanel::default()),
            Box::new(TableEditorPanel::default()),
            Box::new(IrPanel::default()),
            Box::new(DiscordPanel::default()),
            Box::new(ObsPanel::default()),
            Box::new(StreamPanel::default()),
        ]
    }

    /// For each concrete panel type, verify that load->apply is idempotent:
    /// a second load->apply from the output of the first produces identical
    /// Config/PlayerConfig. This accounts for panels that normalize values
    /// (e.g. Option<Vec> -> Some(Vec)).
    /// TableEditorPanel is excluded because its apply() is a no-op.
    #[test]
    fn panel_load_apply_roundtrip() {
        let config = Config::default();
        let player_config = PlayerConfig::default();

        for mut panel in all_panels() {
            let tab_label = panel.tab().label();
            if tab_label == "Table Editor" {
                // apply() is a no-op for TableEditorPanel; just verify no panic.
                panel.load(&config, &player_config);
                let mut c = Config::default();
                let mut p = PlayerConfig::default();
                panel.apply(&mut c, &mut p);
                continue;
            }

            // First roundtrip: load from defaults, apply to fresh configs.
            panel.load(&config, &player_config);
            let mut cfg1 = Config::default();
            let mut pcfg1 = PlayerConfig::default();
            panel.apply(&mut cfg1, &mut pcfg1);

            // Second roundtrip: load from first output, apply again.
            panel.load(&cfg1, &pcfg1);
            let mut cfg2 = Config::default();
            let mut pcfg2 = PlayerConfig::default();
            panel.apply(&mut cfg2, &mut pcfg2);

            let cfg1_json = serde_json::to_string(&cfg1).unwrap();
            let cfg2_json = serde_json::to_string(&cfg2).unwrap();
            assert_eq!(
                cfg1_json, cfg2_json,
                "Config not idempotent for panel: {tab_label}"
            );

            let pcfg1_json = serde_json::to_string(&pcfg1).unwrap();
            let pcfg2_json = serde_json::to_string(&pcfg2).unwrap();
            assert_eq!(
                pcfg1_json, pcfg2_json,
                "PlayerConfig not idempotent for panel: {tab_label}"
            );
        }
    }

    /// After load(), has_changes() should be false. This verifies that load()
    /// resets the dirty flag on all panels.
    #[test]
    fn panel_has_changes_after_load_is_false() {
        let config = Config::default();
        let player_config = PlayerConfig::default();

        for mut panel in all_panels() {
            panel.load(&config, &player_config);
            assert!(
                !panel.has_changes(),
                "has_changes() should be false after load() for panel: {}",
                panel.tab().label()
            );
        }
    }

    /// Collect tab() from all panels, verify no duplicates using a HashSet.
    #[test]
    fn panel_tab_uniqueness() {
        let panels = all_panels();
        let mut seen = HashSet::new();
        for panel in &panels {
            let label = panel.tab().label();
            assert!(seen.insert(label), "Duplicate tab label found: {label}");
        }
        // Also verify that all panels cover all known tabs.
        assert_eq!(seen.len(), Tab::ALL.len());
    }
}

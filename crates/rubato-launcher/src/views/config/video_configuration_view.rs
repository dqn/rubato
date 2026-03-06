// Translates: bms.player.beatoraja.launcher.VideoConfigurationView

use rubato_core::config::{Config, DisplayMode};
use rubato_core::player_config::PlayerConfig;
use rubato_core::resolution::Resolution;

use egui;

use crate::stubs::{MainLoader, monitors};

/// Translates: VideoConfigurationView (JavaFX → egui)
///
/// Video/display configuration UI with resolution, display mode,
/// BGA options, VSync, max FPS, and monitor selection.
#[derive(Default)]
pub struct VideoConfigurationView {
    // @FXML private ComboBox<Resolution> resolution;
    resolution: Option<Resolution>,
    resolution_items: Vec<Resolution>,
    // @FXML private ComboBox<Config.DisplayMode> displayMode;
    display_mode: Option<DisplayMode>,
    // @FXML private ComboBox<String> bgaOp;
    bga_op: i32,
    // @FXML private ComboBox<String> bgaExpand;
    bga_expand: i32,
    // @FXML private CheckBox vSync;
    vsync: bool,
    // @FXML private Spinner<Integer> maxFps;
    max_fps: i32,
    // @FXML private Spinner<Integer> missLayerTime;
    miss_layer_time: i32,
    // @FXML private ComboBox<String> monitor;
    monitor: Option<String>,
    monitor_items: Vec<String>,
}

/// All Resolution variants, matching Java's Resolution.values()
const ALL_RESOLUTIONS: &[Resolution] = &[
    Resolution::SD,
    Resolution::SVGA,
    Resolution::XGA,
    Resolution::HD,
    Resolution::QUADVGA,
    Resolution::FWXGA,
    Resolution::SXGAPLUS,
    Resolution::HDPLUS,
    Resolution::UXGA,
    Resolution::WSXGAPLUS,
    Resolution::FULLHD,
    Resolution::WUXGA,
    Resolution::QXGA,
    Resolution::WQHD,
    Resolution::ULTRAHD,
];

impl VideoConfigurationView {
    // public void initialize(URL location, ResourceBundle resources)
    pub fn initialize(&mut self) {
        // updateResolutions();
        self.update_resolutions();

        // displayMode.getItems().setAll(Config.DisplayMode.values());
        // (DisplayMode variants: FULLSCREEN, BORDERLESS, WINDOW)

        // monitor.getItems().setAll(Arrays.stream(Lwjgl3ApplicationConfiguration.getMonitors())
        //     .map(monitor -> String.format("%s [%s, %s]", monitor.name, Integer.toString(monitor.virtualX), Integer.toString(monitor.virtualY)))
        //     .toList());
        let monitors = monitors();
        self.monitor_items = monitors
            .iter()
            .map(|m| format!("{} [{}, {}]", m.name, m.virtual_x, m.virtual_y))
            .collect();
    }

    // public void update(Config config)
    pub fn update(&mut self, config: &Config) {
        // displayMode.setValue(config.getDisplaymode());
        self.display_mode = Some(config.displaymode.clone());
        // resolution.setValue(config.getResolution());
        self.resolution = Some(config.resolution);
        // vSync.setSelected(config.isVsync());
        self.vsync = config.vsync;
        // monitor.setValue(config.getMonitorName());
        self.monitor = Some(config.monitor_name.clone());
        // bgaOp.getSelectionModel().select(config.getBga());
        self.bga_op = config.bga;
        // bgaExpand.getSelectionModel().select(config.getBgaExpand());
        self.bga_expand = config.bga_expand;
        // maxFps.getValueFactory().setValue(config.getMaxFramePerSecond());
        self.max_fps = config.max_frame_per_second;
    }

    // public void updatePlayer(PlayerConfig player)
    pub fn update_player(&mut self, player: &mut PlayerConfig) {
        // missLayerTime.getValueFactory().setValue(player.getMisslayerDuration());
        self.miss_layer_time = player.misslayer_duration();
    }

    // public void commit(Config config)
    pub fn commit(&self, config: &mut Config) {
        // config.setResolution(resolution.getValue());
        if let Some(ref r) = self.resolution {
            config.resolution = *r;
        }
        // config.setDisplaymode(displayMode.getValue());
        if let Some(ref dm) = self.display_mode {
            config.displaymode = dm.clone();
        }
        // config.setVsync(vSync.isSelected());
        config.vsync = self.vsync;
        // config.setMonitorName(monitor.getValue());
        if let Some(ref m) = self.monitor {
            config.monitor_name = m.clone();
        }
        // config.setBga(bgaOp.getSelectionModel().getSelectedIndex());
        config.bga = self.bga_op;
        // config.setBgaExpand(bgaExpand.getSelectionModel().getSelectedIndex());
        config.bga_expand = self.bga_expand;
        // config.setMaxFramePerSecond(maxFps.getValue());
        config.max_frame_per_second = self.max_fps;
    }

    // public void commitPlayer(PlayerConfig player)
    pub fn commit_player(&self, player: &mut PlayerConfig) {
        // player.setMisslayerDuration(missLayerTime.getValue());
        player.misslayer_duration = self.miss_layer_time;
    }

    /// Get the current resolution items (available resolutions for the current display mode).
    pub fn resolution_items(&self) -> &[Resolution] {
        &self.resolution_items
    }

    /// Get the current selected resolution.
    pub fn resolution(&self) -> Option<Resolution> {
        self.resolution
    }

    /// Get the current monitor items (formatted monitor strings).
    pub fn monitor_items(&self) -> &[String] {
        &self.monitor_items
    }

    /// Get the current selected monitor.
    pub fn monitor(&self) -> Option<&str> {
        self.monitor.as_deref()
    }

    /// Get the current display mode.
    pub fn display_mode(&self) -> Option<&DisplayMode> {
        self.display_mode.as_ref()
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let old_dm_label = self
            .display_mode
            .as_ref()
            .map(|dm| format!("{:?}", dm))
            .unwrap_or_default();

        ui.heading("Display");
        egui::Grid::new("video_display_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Display Mode:");
                let dm_label = old_dm_label.clone();
                egui::ComboBox::from_id_salt("video_display_mode")
                    .selected_text(&dm_label)
                    .show_ui(ui, |ui| {
                        let modes = [
                            DisplayMode::FULLSCREEN,
                            DisplayMode::BORDERLESS,
                            DisplayMode::WINDOW,
                        ];
                        for mode in &modes {
                            let label = format!("{:?}", mode);
                            let selected = dm_label == label;
                            if ui.selectable_label(selected, &label).clicked() {
                                self.display_mode = Some(mode.clone());
                            }
                        }
                    });
                ui.end_row();

                ui.label("Resolution:");
                let res_label = self
                    .resolution
                    .map(|r| format!("{}", r))
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("video_resolution")
                    .selected_text(&res_label)
                    .show_ui(ui, |ui| {
                        for r in &self.resolution_items.clone() {
                            ui.selectable_value(&mut self.resolution, Some(*r), format!("{}", r));
                        }
                    });
                ui.end_row();

                ui.label("Monitor:");
                let mon_label = self.monitor.clone().unwrap_or_default();
                egui::ComboBox::from_id_salt("video_monitor")
                    .selected_text(&mon_label)
                    .show_ui(ui, |ui| {
                        for m in &self.monitor_items.clone() {
                            ui.selectable_value(&mut self.monitor, Some(m.clone()), m);
                        }
                    });
                ui.end_row();

                ui.label("VSync:");
                ui.checkbox(&mut self.vsync, "");
                ui.end_row();

                ui.label("Max FPS:");
                ui.add(egui::DragValue::new(&mut self.max_fps).range(1..=1000));
                ui.end_row();
            });

        // Update resolutions when display mode changes
        let new_dm_label = self
            .display_mode
            .as_ref()
            .map(|dm| format!("{:?}", dm))
            .unwrap_or_default();
        if old_dm_label != new_dm_label {
            self.update_resolutions();
        }

        ui.separator();
        ui.heading("BGA");
        egui::Grid::new("video_bga_grid")
            .num_columns(2)
            .show(ui, |ui| {
                let bga_labels = ["ON", "AUTO", "OFF"];
                ui.label("BGA:");
                egui::ComboBox::from_id_salt("video_bga_op")
                    .selected_text(*bga_labels.get(self.bga_op as usize).unwrap_or(&"Unknown"))
                    .show_ui(ui, |ui| {
                        for (i, label) in bga_labels.iter().enumerate() {
                            ui.selectable_value(&mut self.bga_op, i as i32, *label);
                        }
                    });
                ui.end_row();

                let expand_labels = ["Full", "Keep Aspect Ratio", "Off"];
                ui.label("BGA Expand:");
                egui::ComboBox::from_id_salt("video_bga_expand")
                    .selected_text(
                        *expand_labels
                            .get(self.bga_expand as usize)
                            .unwrap_or(&"Unknown"),
                    )
                    .show_ui(ui, |ui| {
                        for (i, label) in expand_labels.iter().enumerate() {
                            ui.selectable_value(&mut self.bga_expand, i as i32, *label);
                        }
                    });
                ui.end_row();

                ui.label("Miss Layer Time (ms):");
                ui.add(egui::DragValue::new(&mut self.miss_layer_time).range(0..=10000));
                ui.end_row();
            });
    }

    // @FXML public void updateResolutions()
    pub fn update_resolutions(&mut self) {
        // Resolution oldValue = resolution.getValue();
        let old_value = self.resolution;
        // resolution.getItems().clear();
        self.resolution_items.clear();

        // if (displayMode.getValue() == Config.DisplayMode.FULLSCREEN) {
        if matches!(self.display_mode, Some(DisplayMode::FULLSCREEN)) {
            // Graphics.DisplayMode[] displays = MainLoader.getAvailableDisplayMode();
            let displays = MainLoader::available_display_mode();
            // for(Resolution r : Resolution.values()) {
            for r in ALL_RESOLUTIONS {
                // for(Graphics.DisplayMode display : displays) {
                for display in &displays {
                    // if(display.width == r.width && display.height == r.height) {
                    if display.width == r.width() && display.height == r.height() {
                        // resolution.getItems().add(r);
                        self.resolution_items.push(*r);
                        // break;
                        break;
                    }
                }
            }
        } else {
            // Graphics.DisplayMode display = MainLoader.getDesktopDisplayMode();
            let display = MainLoader::desktop_display_mode();
            // for(Resolution r : Resolution.values()) {
            for r in ALL_RESOLUTIONS {
                // if (r.width <= display.width && r.height <= display.height) {
                if r.width() <= display.width && r.height() <= display.height {
                    // resolution.getItems().add(r);
                    self.resolution_items.push(*r);
                }
            }
        }

        // resolution.setValue(resolution.getItems().contains(oldValue)
        //     ? oldValue : resolution.getItems().get(resolution.getItems().size() - 1));
        if let Some(ov) = old_value {
            if self.resolution_items.contains(&ov) {
                self.resolution = Some(ov);
            } else if let Some(last) = self.resolution_items.last() {
                self.resolution = Some(*last);
            }
        } else if let Some(last) = self.resolution_items.last() {
            self.resolution = Some(*last);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_core::config::BGA_ON;

    // --- Default / initialization tests ---

    #[test]
    fn default_has_no_selection() {
        let view = VideoConfigurationView::default();
        assert!(view.resolution().is_none());
        assert!(view.display_mode().is_none());
        assert!(view.monitor().is_none());
        assert!(view.resolution_items().is_empty());
        assert!(view.monitor_items().is_empty());
    }

    #[test]
    fn initialize_populates_resolutions() {
        let mut view = VideoConfigurationView::default();
        view.initialize();
        // In WINDOW mode (default), resolutions up to desktop mode (1920x1080 fallback) are available
        assert!(!view.resolution_items().is_empty());
        // The last item should be selected when no previous resolution exists
        assert!(view.resolution().is_some());
    }

    // --- update() tests ---

    #[test]
    fn update_copies_config_fields() {
        let mut view = VideoConfigurationView::default();
        let config = Config {
            displaymode: DisplayMode::FULLSCREEN,
            resolution: Resolution::FULLHD,
            vsync: true,
            monitor_name: "Test Monitor [0, 0]".to_string(),
            bga: 2,
            bga_expand: 1,
            max_frame_per_second: 120,
            ..Config::default()
        };
        view.update(&config);

        assert!(matches!(view.display_mode(), Some(DisplayMode::FULLSCREEN)));
        assert_eq!(view.resolution(), Some(Resolution::FULLHD));
        assert_eq!(view.monitor(), Some("Test Monitor [0, 0]"));
    }

    // --- commit() roundtrip tests ---

    #[test]
    fn commit_roundtrip_preserves_values() {
        let mut view = VideoConfigurationView::default();
        let input = Config {
            displaymode: DisplayMode::BORDERLESS,
            resolution: Resolution::HD,
            vsync: true,
            monitor_name: "Display 1 [0, 0]".to_string(),
            bga: 2,
            bga_expand: 0,
            max_frame_per_second: 144,
            ..Config::default()
        };
        view.update(&input);

        let mut output = Config::default();
        view.commit(&mut output);

        assert!(matches!(output.displaymode, DisplayMode::BORDERLESS));
        assert_eq!(output.resolution, Resolution::HD);
        assert!(output.vsync);
        assert_eq!(output.monitor_name, "Display 1 [0, 0]");
        assert_eq!(output.bga, 2);
        assert_eq!(output.bga_expand, 0);
        assert_eq!(output.max_frame_per_second, 144);
    }

    // --- update_player / commit_player tests ---

    #[test]
    fn update_and_commit_player_roundtrip() {
        let mut view = VideoConfigurationView::default();
        let mut player = PlayerConfig {
            misslayer_duration: 500,
            ..Default::default()
        };

        view.update_player(&mut player);

        let mut out_player = PlayerConfig::default();
        view.commit_player(&mut out_player);
        assert_eq!(out_player.misslayer_duration, 500);
    }

    // --- update_resolutions() logic tests ---

    #[test]
    fn update_resolutions_window_mode_filters_by_desktop() {
        // In WINDOW mode (non-fullscreen), resolutions up to desktop mode are included
        let mut view = VideoConfigurationView {
            display_mode: Some(DisplayMode::WINDOW),
            ..Default::default()
        };
        view.update_resolutions();

        // Desktop fallback is 1920x1080, so all resolutions <= 1920x1080 should be included
        let items = view.resolution_items();
        assert!(!items.is_empty());

        // FULLHD (1920x1080) should be included
        assert!(items.contains(&Resolution::FULLHD));
        // ULTRAHD (3840x2160) should NOT be included (exceeds desktop mode)
        assert!(!items.contains(&Resolution::ULTRAHD));
        // SD (640x480) should be included
        assert!(items.contains(&Resolution::SD));
    }

    #[test]
    fn update_resolutions_fullscreen_mode_filters_by_available() {
        // In FULLSCREEN mode, only resolutions matching available display modes are included
        let mut view = VideoConfigurationView {
            display_mode: Some(DisplayMode::FULLSCREEN),
            ..Default::default()
        };
        view.update_resolutions();

        let items = view.resolution_items();
        assert!(!items.is_empty());
        // The fallback display modes include 1920x1080
        assert!(items.contains(&Resolution::FULLHD));
    }

    #[test]
    fn update_resolutions_preserves_old_value_if_available() {
        let mut view = VideoConfigurationView {
            display_mode: Some(DisplayMode::WINDOW),
            resolution: Some(Resolution::HD),
            ..Default::default()
        }; // 1280x720
        view.update_resolutions();

        // HD (1280x720) should be preserved since it's within desktop mode
        assert_eq!(view.resolution(), Some(Resolution::HD));
    }

    #[test]
    fn update_resolutions_selects_last_if_old_not_available() {
        let mut view = VideoConfigurationView {
            display_mode: Some(DisplayMode::WINDOW),
            resolution: Some(Resolution::ULTRAHD),
            ..Default::default()
        }; // 3840x2160, larger than desktop
        view.update_resolutions();

        // ULTRAHD exceeds desktop mode so old value is not in the list
        // Should select last available item (FULLHD at 1920x1080)
        let selected = view.resolution().unwrap();
        assert_ne!(selected, Resolution::ULTRAHD);
        // Should be the last item in the list
        assert_eq!(selected, *view.resolution_items().last().unwrap());
    }

    #[test]
    fn update_resolutions_borderless_same_as_window() {
        // BORDERLESS mode uses the same logic as WINDOW mode (non-fullscreen path)
        let mut view_window = VideoConfigurationView {
            display_mode: Some(DisplayMode::WINDOW),
            ..Default::default()
        };
        view_window.update_resolutions();

        let mut view_borderless = VideoConfigurationView {
            display_mode: Some(DisplayMode::BORDERLESS),
            ..Default::default()
        };
        view_borderless.update_resolutions();

        assert_eq!(
            view_window.resolution_items(),
            view_borderless.resolution_items()
        );
    }

    // --- Accessor tests ---

    #[test]
    fn all_resolutions_matches_resolution_values() {
        // Verify ALL_RESOLUTIONS matches Resolution enum variants count
        assert_eq!(ALL_RESOLUTIONS.len(), 15);
        assert_eq!(ALL_RESOLUTIONS[0], Resolution::SD);
        assert_eq!(ALL_RESOLUTIONS[14], Resolution::ULTRAHD);
    }

    #[test]
    fn update_default_config_uses_bga_on() {
        let mut view = VideoConfigurationView::default();
        let config = Config::default();
        view.update(&config);

        let mut out = Config::default();
        view.commit(&mut out);
        assert_eq!(out.bga, BGA_ON);
    }
}

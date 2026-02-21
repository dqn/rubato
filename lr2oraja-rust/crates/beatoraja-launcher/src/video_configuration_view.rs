// Translates: bms.player.beatoraja.launcher.VideoConfigurationView

use beatoraja_core::config::{Config, DisplayMode};
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_core::resolution::Resolution;

use crate::stubs::{get_monitors, MainLoader};

/// Translates: VideoConfigurationView (JavaFX → egui)
///
/// Video/display configuration UI with resolution, display mode,
/// BGA options, VSync, max FPS, and monitor selection.
#[allow(dead_code)]
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

impl Default for VideoConfigurationView {
    fn default() -> Self {
        VideoConfigurationView {
            resolution: None,
            resolution_items: Vec::new(),
            display_mode: None,
            bga_op: 0,
            bga_expand: 0,
            vsync: false,
            max_fps: 0,
            miss_layer_time: 0,
            monitor: None,
            monitor_items: Vec::new(),
        }
    }
}

#[allow(dead_code)]
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
        let monitors = get_monitors();
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
        self.miss_layer_time = player.get_misslayer_duration();
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

    // @FXML public void updateResolutions()
    pub fn update_resolutions(&mut self) {
        // Resolution oldValue = resolution.getValue();
        let old_value = self.resolution;
        // resolution.getItems().clear();
        self.resolution_items.clear();

        // if (displayMode.getValue() == Config.DisplayMode.FULLSCREEN) {
        if matches!(self.display_mode, Some(DisplayMode::FULLSCREEN)) {
            // Graphics.DisplayMode[] displays = MainLoader.getAvailableDisplayMode();
            let displays = MainLoader::get_available_display_mode();
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
            let display = MainLoader::get_desktop_display_mode();
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

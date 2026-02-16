use bms_config::{Config, DisplayMode, PlayerConfig, Resolution};

use crate::panel::LauncherPanel;
use crate::tab::Tab;
use crate::widgets::clamped::clamped_i32;

const DISPLAY_MODES: &[DisplayMode] = &[
    DisplayMode::Window,
    DisplayMode::Fullscreen,
    DisplayMode::Borderless,
];

const RESOLUTIONS: &[Resolution] = &[
    Resolution::Sd,
    Resolution::Svga,
    Resolution::Xga,
    Resolution::Hd,
    Resolution::Quadvga,
    Resolution::Fwxga,
    Resolution::Sxgaplus,
    Resolution::Hdplus,
    Resolution::Uxga,
    Resolution::Wsxgaplus,
    Resolution::Fullhd,
    Resolution::Wuxga,
    Resolution::Qxga,
    Resolution::Wqhd,
    Resolution::Ultrahd,
];

pub struct VideoPanel {
    displaymode: DisplayMode,
    resolution: Resolution,
    use_resolution: bool,
    window_width: i32,
    window_height: i32,
    vsync: bool,
    bga: i32,
    bga_expand: i32,
    max_frame_per_second: i32,
    frameskip: i32,
    monitor_name: String,
    misslayer_duration: i32,
    dirty: bool,
}

impl Default for VideoPanel {
    fn default() -> Self {
        let config = Config::default();
        let player_config = PlayerConfig::default();
        Self {
            displaymode: config.displaymode,
            resolution: config.resolution,
            use_resolution: config.use_resolution,
            window_width: config.window_width,
            window_height: config.window_height,
            vsync: config.vsync,
            bga: config.bga,
            bga_expand: config.bga_expand,
            max_frame_per_second: config.max_frame_per_second,
            frameskip: config.frameskip,
            monitor_name: config.monitor_name.clone(),
            misslayer_duration: player_config.misslayer_duration,
            dirty: false,
        }
    }
}

impl LauncherPanel for VideoPanel {
    fn tab(&self) -> Tab {
        Tab::Video
    }

    fn load(&mut self, config: &Config, player_config: &PlayerConfig) {
        self.displaymode = config.displaymode;
        self.resolution = config.resolution;
        self.use_resolution = config.use_resolution;
        self.window_width = config.window_width;
        self.window_height = config.window_height;
        self.vsync = config.vsync;
        self.bga = config.bga;
        self.bga_expand = config.bga_expand;
        self.max_frame_per_second = config.max_frame_per_second;
        self.frameskip = config.frameskip;
        self.monitor_name = config.monitor_name.clone();
        self.misslayer_duration = player_config.misslayer_duration;
        self.dirty = false;
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Video Settings");
        ui.separator();

        // Display mode
        let prev_mode = self.displaymode;
        egui::ComboBox::from_label("Display Mode")
            .selected_text(format!("{:?}", self.displaymode))
            .show_ui(ui, |ui| {
                for &mode in DISPLAY_MODES {
                    ui.selectable_value(&mut self.displaymode, mode, format!("{mode:?}"));
                }
            });
        if self.displaymode != prev_mode {
            self.dirty = true;
        }

        // Resolution preset
        let changed = ui.checkbox(&mut self.use_resolution, "Use preset resolution");
        if changed.changed() {
            self.dirty = true;
        }

        if self.use_resolution {
            let prev_res = self.resolution;
            egui::ComboBox::from_label("Resolution")
                .selected_text(self.resolution.to_string())
                .show_ui(ui, |ui| {
                    for &res in RESOLUTIONS {
                        ui.selectable_value(&mut self.resolution, res, res.to_string());
                    }
                });
            if self.resolution != prev_res {
                self.window_width = self.resolution.width();
                self.window_height = self.resolution.height();
                self.dirty = true;
            }
        } else {
            let prev_w = self.window_width;
            let prev_h = self.window_height;
            clamped_i32(
                ui,
                "Window Width",
                &mut self.window_width,
                Resolution::Sd.width(),
                Resolution::Ultrahd.width(),
            );
            clamped_i32(
                ui,
                "Window Height",
                &mut self.window_height,
                Resolution::Sd.height(),
                Resolution::Ultrahd.height(),
            );
            if self.window_width != prev_w || self.window_height != prev_h {
                self.dirty = true;
            }
        }

        ui.separator();

        let changed = ui.checkbox(&mut self.vsync, "VSync");
        if changed.changed() {
            self.dirty = true;
        }

        let prev = self.max_frame_per_second;
        clamped_i32(ui, "Max FPS", &mut self.max_frame_per_second, 0, 50000);
        if self.max_frame_per_second != prev {
            self.dirty = true;
        }

        ui.separator();
        ui.label("BGA");
        let prev = self.bga;
        egui::ComboBox::from_label("BGA Display")
            .selected_text(match self.bga {
                0 => "Off",
                1 => "On",
                _ => "Auto",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.bga, 0, "Off");
                ui.selectable_value(&mut self.bga, 1, "On");
                ui.selectable_value(&mut self.bga, 2, "Auto");
            });
        if self.bga != prev {
            self.dirty = true;
        }

        let prev = self.bga_expand;
        egui::ComboBox::from_label("BGA Expand")
            .selected_text(match self.bga_expand {
                0 => "Off",
                1 => "Stage File",
                _ => "Full",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.bga_expand, 0, "Off");
                ui.selectable_value(&mut self.bga_expand, 1, "Stage File");
                ui.selectable_value(&mut self.bga_expand, 2, "Full");
            });
        if self.bga_expand != prev {
            self.dirty = true;
        }

        let prev = self.frameskip;
        clamped_i32(ui, "Frame Skip", &mut self.frameskip, 0, 10);
        if self.frameskip != prev {
            self.dirty = true;
        }

        ui.separator();
        ui.label("Monitor");
        let prev = self.monitor_name.clone();
        ui.horizontal(|ui| {
            ui.label("Monitor Name:");
            ui.text_edit_singleline(&mut self.monitor_name);
        });
        if self.monitor_name != prev {
            self.dirty = true;
        }

        ui.separator();
        let prev = self.misslayer_duration;
        clamped_i32(
            ui,
            "Miss Layer Duration (ms)",
            &mut self.misslayer_duration,
            0,
            5000,
        );
        if self.misslayer_duration != prev {
            self.dirty = true;
        }
    }

    fn apply(&self, config: &mut Config, player_config: &mut PlayerConfig) {
        config.displaymode = self.displaymode;
        config.resolution = self.resolution;
        config.use_resolution = self.use_resolution;
        config.window_width = self.window_width;
        config.window_height = self.window_height;
        config.vsync = self.vsync;
        config.bga = self.bga;
        config.bga_expand = self.bga_expand;
        config.max_frame_per_second = self.max_frame_per_second;
        config.frameskip = self.frameskip;
        config.monitor_name = self.monitor_name.clone();
        player_config.misslayer_duration = self.misslayer_duration;
    }

    fn has_changes(&self) -> bool {
        self.dirty
    }
}

// LauncherUi — egui-based launcher configuration window
// Java equivalent: PlayConfigurationView (JavaFX Application)

use beatoraja_core::config::Config;
use beatoraja_core::player_config::PlayerConfig;

use crate::play_configuration_view::PlayMode;

/// Tab selection for the launcher UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
enum Tab {
    Video,
    Audio,
    Input,
    MusicSelect,
    Skin,
    Option,
    Other,
    IR,
    Stream,
    Discord,
    OBS,
}

impl Tab {
    fn label(&self) -> &'static str {
        match self {
            Tab::Video => "Video",
            Tab::Audio => "Audio",
            Tab::Input => "Input",
            Tab::MusicSelect => "Music Select",
            Tab::Skin => "Skin",
            Tab::Option => "Option",
            Tab::Other => "Other",
            Tab::IR => "IR",
            Tab::Stream => "Stream",
            Tab::Discord => "Discord",
            Tab::OBS => "OBS",
        }
    }

    fn all() -> &'static [Tab] {
        &[
            Tab::Video,
            Tab::Audio,
            Tab::Input,
            Tab::MusicSelect,
            Tab::Skin,
            Tab::Option,
            Tab::Other,
            Tab::IR,
            Tab::Stream,
            Tab::Discord,
            Tab::OBS,
        ]
    }
}

/// Main launcher UI state.
///
/// Java equivalent: PlayConfigurationView — manages all configuration sub-views
/// and provides the top-level player selector + action buttons.
pub struct LauncherUi {
    config: Config,
    player: PlayerConfig,
    selected_tab: Tab,
    player_name: String,
    selected_play_mode: usize,
    bms_paths: Vec<String>,
}

impl LauncherUi {
    pub fn new(config: Config, player: PlayerConfig) -> Self {
        let player_name = config
            .playername
            .clone()
            .unwrap_or_else(|| "default".to_string());
        Self {
            config,
            player,
            selected_tab: Tab::Option,
            player_name,
            selected_play_mode: 1, // BEAT_7K
            bms_paths: Vec::new(),
        }
    }

    /// Render the launcher configuration UI.
    ///
    /// Java equivalent: PlayConfigurationView.start(Stage primaryStage) builds
    /// the JavaFX scene graph with tabs, combo boxes, and action buttons.
    pub fn render_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header: player name + play mode selector
            ui.horizontal(|ui| {
                ui.label("Player:");
                ui.text_edit_singleline(&mut self.player_name);

                ui.separator();

                let play_modes = PlayMode::values();
                let selected_text = play_modes
                    .get(self.selected_play_mode)
                    .map(|m| m.display_name())
                    .unwrap_or("7KEYS");
                egui::ComboBox::from_label("Mode")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        for (i, mode) in play_modes.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.selected_play_mode,
                                i,
                                mode.display_name(),
                            );
                        }
                    });
            });

            ui.separator();

            // Tab bar
            ui.horizontal(|ui| {
                for tab in Tab::all() {
                    if ui
                        .selectable_label(self.selected_tab == *tab, tab.label())
                        .clicked()
                    {
                        self.selected_tab = *tab;
                    }
                }
            });

            ui.separator();

            // Tab content
            egui::ScrollArea::vertical().show(ui, |ui| match self.selected_tab {
                Tab::Video => self.render_video_tab(ui),
                Tab::Audio => self.render_audio_tab(ui),
                Tab::Input => self.render_input_tab(ui),
                Tab::MusicSelect => self.render_music_select_tab(ui),
                Tab::Skin => self.render_skin_tab(ui),
                Tab::Option => self.render_option_tab(ui),
                Tab::Other => self.render_other_tab(ui),
                Tab::IR => self.render_ir_tab(ui),
                Tab::Stream => self.render_stream_tab(ui),
                Tab::Discord => self.render_discord_tab(ui),
                Tab::OBS => self.render_obs_tab(ui),
            });

            ui.separator();

            // Action buttons at the bottom
            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    self.commit_config();
                    log::info!("Start requested");
                }
                if ui.button("Load All BMS").clicked() {
                    log::info!("Load All BMS requested");
                }
                if ui.button("Load Diff BMS").clicked() {
                    log::info!("Load Diff BMS requested");
                }
                if ui.button("Import Score").clicked() {
                    log::info!("Import Score requested");
                }
                if ui.button("Exit").clicked() {
                    std::process::exit(0);
                }
            });
        });
    }

    fn render_video_tab(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("video_grid").show(ui, |ui| {
            ui.label("Resolution:");
            ui.label(format!(
                "{}x{}",
                self.config.resolution.width(),
                self.config.resolution.height()
            ));
            ui.end_row();

            ui.label("Display Mode:");
            ui.label(format!("{:?}", self.config.displaymode));
            ui.end_row();

            ui.label("VSync:");
            ui.checkbox(&mut self.config.vsync, "");
            ui.end_row();

            ui.label("Max FPS:");
            ui.add(egui::DragValue::new(&mut self.config.max_frame_per_second).range(0..=999));
            ui.end_row();
        });
    }

    fn render_audio_tab(&mut self, ui: &mut egui::Ui) {
        let audio = self.config.audio.get_or_insert_with(Default::default);
        egui::Grid::new("audio_grid").show(ui, |ui| {
            ui.label("Audio Buffer:");
            ui.add(egui::DragValue::new(&mut audio.device_buffer_size).range(0..=9999));
            ui.end_row();

            ui.label("Max Simultaneous:");
            ui.add(egui::DragValue::new(&mut audio.device_simultaneous_sources).range(1..=256));
            ui.end_row();

            ui.label("System Volume:");
            ui.add(egui::Slider::new(&mut audio.systemvolume, 0.0..=1.0));
            ui.end_row();

            ui.label("Key Volume:");
            ui.add(egui::Slider::new(&mut audio.keyvolume, 0.0..=1.0));
            ui.end_row();

            ui.label("BG Volume:");
            ui.add(egui::Slider::new(&mut audio.bgvolume, 0.0..=1.0));
            ui.end_row();
        });
    }

    fn render_input_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Input configuration");
        ui.label("(Key bindings and controller settings will be available here)");
    }

    fn render_music_select_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Music Select configuration");
        ui.label("BMS paths:");
        for path in &self.bms_paths {
            ui.label(path);
        }
        if ui.button("Add BMS folder...").clicked()
            && let Some(path) = crate::stubs::show_directory_chooser("Select BMS folder")
        {
            self.bms_paths.push(path);
        }
    }

    fn render_skin_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Skin configuration");
        ui.label("(Skin selection and customization will be available here)");
    }

    fn render_option_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Play Options");

        egui::Grid::new("option_grid").show(ui, |ui| {
            ui.label("HiSpeed:");
            ui.label("(configured per play mode)");
            ui.end_row();

            ui.label("Target:");
            ui.label(self.player.targetid.to_string());
            ui.end_row();
        });
    }

    fn render_other_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Other settings");
        ui.label("(IPFS, HTTP download, screenshot settings will be available here)");
    }

    fn render_ir_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Internet Ranking configuration");
        ui.label("(IR server settings will be available here)");
    }

    fn render_stream_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Stream configuration");
        ui.label("(Streaming settings will be available here)");
    }

    fn render_discord_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Discord Rich Presence");
        ui.checkbox(
            &mut self.config.use_discord_rpc,
            "Enable Discord Rich Presence",
        );
    }

    fn render_obs_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("OBS WebSocket configuration");
        ui.label("(OBS integration settings will be available here)");
    }

    fn commit_config(&mut self) {
        self.config.playername = Some(self.player_name.clone());
        if let Err(e) = Config::write(&self.config) {
            log::error!("Failed to save config: {}", e);
        }
    }
}

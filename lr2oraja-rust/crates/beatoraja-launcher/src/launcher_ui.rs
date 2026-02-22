// LauncherUi — egui-based launcher configuration window
// Java equivalent: PlayConfigurationView (JavaFX Application)

use beatoraja_core::config::Config;
use beatoraja_core::ir_config::IRConfig;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_skin::skin_type::SkinType;
use bms_model::mode::Mode;

use crate::play_configuration_view::PlayMode;
use crate::skin_configuration_view::{SkinConfigItem, SkinConfigurationView};

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

const IR_SEND_LABELS: [&str; 3] = ["ALWAYS", "COMPLETE SONG", "UPDATE SCORE"];
const OBS_REC_MODE_LABELS: [&str; 3] = ["DEFAULT", "ON SCREENSHOT", "ON REPLAY"];

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
    selected_ir_index: usize,
    /// Decrypted IR userid buffer for egui text editing.
    ir_userid_buf: String,
    /// Decrypted IR password buffer for egui text editing.
    ir_password_buf: String,
    /// Previous IR index to detect slot switches.
    ir_prev_index: Option<usize>,
    /// Skin configuration sub-view (skin type/header selection + custom options).
    skin_view: SkinConfigurationView,
}

impl LauncherUi {
    pub fn new(config: Config, player: PlayerConfig) -> Self {
        let player_name = config
            .playername
            .clone()
            .unwrap_or_else(|| "default".to_string());
        // Initialize skin configuration view: scan filesystem + load player config
        let mut skin_view = SkinConfigurationView::new();
        skin_view.initialize();
        skin_view.update_config(&config);
        skin_view.update_player(&player);
        Self {
            config,
            player,
            selected_tab: Tab::Option,
            player_name,
            selected_play_mode: 1, // BEAT_7K
            bms_paths: Vec::new(),
            selected_ir_index: 0,
            ir_userid_buf: String::new(),
            ir_password_buf: String::new(),
            ir_prev_index: None,
            skin_view,
        }
    }

    fn current_mode(&self) -> Mode {
        PlayMode::values()
            .get(self.selected_play_mode)
            .map(|m| m.to_mode())
            .unwrap_or(Mode::BEAT_7K)
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

    /// Java equivalent: InputConfigurationView
    /// Keyboard/controller/mouse scratch settings per play mode.
    fn render_input_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Input Configuration");

        let mode = self.current_mode();
        let pmc = self.player.get_play_config(mode);

        // Keyboard settings
        ui.label("Keyboard");
        egui::Grid::new("keyboard_grid").show(ui, |ui| {
            ui.label("Duration:");
            ui.add(egui::DragValue::new(&mut pmc.keyboard.duration).range(0..=100));
            ui.end_row();
        });

        ui.separator();

        // Controller settings (per player side)
        for (i, controller) in pmc.controller.iter_mut().enumerate() {
            ui.label(format!("Controller {} ({}P)", i + 1, i + 1));
            egui::Grid::new(format!("controller_grid_{}", i)).show(ui, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut controller.name);
                ui.end_row();

                ui.label("Duration:");
                ui.add(egui::DragValue::new(&mut controller.duration).range(0..=100));
                ui.end_row();

                ui.label("JKOC Hack:");
                ui.checkbox(&mut controller.jkoc_hack, "");
                ui.end_row();

                ui.label("Analog Scratch:");
                ui.checkbox(&mut controller.analog_scratch, "");
                ui.end_row();

                if controller.analog_scratch {
                    ui.label("Analog Threshold:");
                    ui.add(
                        egui::DragValue::new(&mut controller.analog_scratch_threshold)
                            .range(1..=1000),
                    );
                    ui.end_row();

                    let analog_modes = ["Ver 2", "Ver 1"];
                    let selected_label = analog_modes
                        .get(controller.analog_scratch_mode as usize)
                        .unwrap_or(&"Ver 2");
                    ui.label("Analog Mode:");
                    egui::ComboBox::from_id_salt(format!("analog_mode_{}", i))
                        .selected_text(*selected_label)
                        .show_ui(ui, |ui| {
                            for (idx, label) in analog_modes.iter().enumerate() {
                                ui.selectable_value(
                                    &mut controller.analog_scratch_mode,
                                    idx as i32,
                                    *label,
                                );
                            }
                        });
                    ui.end_row();
                }
            });
            ui.separator();
        }

        // Mouse scratch settings
        let ms = &mut pmc.keyboard.mouse_scratch_config;
        ui.label("Mouse Scratch");
        egui::Grid::new("mouse_scratch_grid").show(ui, |ui| {
            ui.label("Enable:");
            ui.checkbox(&mut ms.mouse_scratch_enabled, "");
            ui.end_row();

            if ms.mouse_scratch_enabled {
                ui.label("Time Threshold:");
                ui.add(egui::DragValue::new(&mut ms.mouse_scratch_time_threshold).range(1..=10000));
                ui.end_row();

                ui.label("Distance:");
                ui.add(egui::DragValue::new(&mut ms.mouse_scratch_distance).range(1..=10000));
                ui.end_row();

                let scratch_modes = ["Ver 2", "Ver 1"];
                let selected_label = scratch_modes
                    .get(ms.mouse_scratch_mode as usize)
                    .unwrap_or(&"Ver 2");
                ui.label("Mode:");
                egui::ComboBox::from_id_salt("mouse_scratch_mode")
                    .selected_text(*selected_label)
                    .show_ui(ui, |ui| {
                        for (idx, label) in scratch_modes.iter().enumerate() {
                            ui.selectable_value(&mut ms.mouse_scratch_mode, idx as i32, *label);
                        }
                    });
                ui.end_row();
            }
        });
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

    /// Java equivalent: SkinConfigurationView
    /// Skin type selection, skin header browsing, and custom options/files/offsets.
    fn render_skin_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Skin Configuration");

        ui.checkbox(&mut self.config.cache_skin_image, "Cache Skin Image (CIM)");

        ui.separator();

        // Skin type selector
        let skin_types = SkinType::values();
        let current_type = self
            .skin_view
            .get_skintype_selector()
            .unwrap_or(SkinType::Play7Keys);
        let selected_text = SkinConfigurationView::skin_type_display_name(&current_type);
        ui.horizontal(|ui| {
            ui.label("Category:");
            let mut new_type = current_type;
            egui::ComboBox::from_id_salt("skin_type_selector")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    for st in &skin_types {
                        ui.selectable_value(
                            &mut new_type,
                            *st,
                            SkinConfigurationView::skin_type_display_name(st),
                        );
                    }
                });
            if new_type != current_type {
                self.skin_view.set_skintype_selector(new_type);
                self.skin_view.change_skin_type();
            }
        });

        // Skin header selector
        let headers = self.skin_view.get_current_headers().to_owned();
        let header_count = headers.len();
        let selected_idx = self.skin_view.get_skinheader_selector();
        ui.horizontal(|ui| {
            ui.label("Skin:");
            if header_count == 0 {
                ui.label("(no skins found)");
            } else {
                let display = selected_idx
                    .and_then(|i| headers.get(i))
                    .map(SkinConfigurationView::skin_header_display_name)
                    .unwrap_or_else(|| "(none)".to_string());
                let mut new_idx = selected_idx.unwrap_or(0);
                egui::ComboBox::from_id_salt("skin_header_selector")
                    .selected_text(display)
                    .show_ui(ui, |ui| {
                        for (i, header) in headers.iter().enumerate() {
                            let name = SkinConfigurationView::skin_header_display_name(header);
                            ui.selectable_value(&mut new_idx, i, name);
                        }
                    });
                if Some(new_idx) != selected_idx {
                    self.skin_view.set_skinheader_selector(new_idx);
                    self.skin_view.change_skin_header();
                }
            }
        });

        ui.separator();

        // Render dynamic skin config items (options, files, offsets)
        let items = self.skin_view.get_skinconfig_items_mut();
        for item in items.iter_mut() {
            match item {
                SkinConfigItem::Label(text) => {
                    if text.is_empty() {
                        ui.add_space(4.0);
                    } else {
                        ui.label(egui::RichText::new(text.as_str()).strong());
                    }
                }
                SkinConfigItem::Option {
                    name,
                    items: combo_items,
                    selected_index,
                } => {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}:", name));
                        let display = combo_items
                            .get(*selected_index)
                            .cloned()
                            .unwrap_or_default();
                        egui::ComboBox::from_id_salt(format!("skin_opt_{}", name))
                            .selected_text(display)
                            .show_ui(ui, |ui| {
                                for (i, label) in combo_items.iter().enumerate() {
                                    ui.selectable_value(selected_index, i, label.as_str());
                                }
                            });
                    });
                }
                SkinConfigItem::File {
                    name,
                    items: combo_items,
                    selected_value,
                } => {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}:", name));
                        let display = selected_value.clone().unwrap_or_default();
                        let mut new_val = display.clone();
                        egui::ComboBox::from_id_salt(format!("skin_file_{}", name))
                            .selected_text(&display)
                            .show_ui(ui, |ui| {
                                for label in combo_items.iter() {
                                    ui.selectable_value(
                                        &mut new_val,
                                        label.clone(),
                                        label.as_str(),
                                    );
                                }
                            });
                        if new_val != display {
                            *selected_value = Some(new_val);
                        }
                    });
                }
                SkinConfigItem::Offset {
                    name,
                    values,
                    enabled,
                } => {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}:", name));
                        let labels = ["x", "y", "w", "h", "r", "a"];
                        for (i, &label) in labels.iter().enumerate() {
                            if enabled[i] {
                                ui.label(label);
                                ui.add(egui::DragValue::new(&mut values[i]).range(-9999..=9999));
                            }
                        }
                    });
                }
            }
        }
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

    /// Java equivalent: PlayConfigurationView "Other" tab
    /// IPFS, HTTP download, and screenshot settings.
    fn render_other_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Other Settings");

        // Screenshot
        ui.checkbox(
            &mut self.config.set_clipboard_screenshot,
            "Clipboard Screenshot",
        );

        ui.separator();

        // IPFS settings
        ui.label("IPFS");
        egui::Grid::new("ipfs_grid").show(ui, |ui| {
            ui.label("Enable:");
            ui.checkbox(&mut self.config.enable_ipfs, "");
            ui.end_row();

            if self.config.enable_ipfs {
                ui.label("IPFS URL:");
                ui.text_edit_singleline(&mut self.config.ipfsurl);
                ui.end_row();
            }
        });

        ui.separator();

        // HTTP download settings
        ui.label("HTTP Download");
        egui::Grid::new("http_grid").show(ui, |ui| {
            ui.label("Enable:");
            ui.checkbox(&mut self.config.enable_http, "");
            ui.end_row();

            if self.config.enable_http {
                ui.label("Download Source:");
                ui.text_edit_singleline(&mut self.config.download_source);
                ui.end_row();

                ui.label("Default URL:");
                ui.text_edit_singleline(&mut self.config.default_download_url);
                ui.end_row();

                ui.label("Override URL:");
                ui.text_edit_singleline(&mut self.config.override_download_url);
                ui.end_row();
            }
        });
    }

    /// Flush current IR userid/password buffers back to IRConfig via
    /// set_userid/set_password (triggers AES encryption).
    /// Java equivalent: IRConfigurationView.updateIRConnection() save-side.
    fn flush_ir_buffers(&mut self) {
        if let Some(prev) = self.ir_prev_index
            && let Some(Some(ir)) = self.player.irconfig.get_mut(prev)
        {
            ir.set_userid(self.ir_userid_buf.clone());
            ir.set_password(self.ir_password_buf.clone());
        }
    }

    /// Load decrypted IR userid/password into buffers for the given index.
    /// Java equivalent: IRConfigurationView.updateIRConnection() load-side.
    fn load_ir_buffers(&mut self, idx: usize) {
        if let Some(Some(ir)) = self.player.irconfig.get(idx) {
            self.ir_userid_buf = ir.get_userid();
            self.ir_password_buf = ir.get_password();
        } else {
            self.ir_userid_buf.clear();
            self.ir_password_buf.clear();
        }
        self.ir_prev_index = Some(idx);
    }

    /// Java equivalent: IRConfigurationView
    /// Internet Ranking server settings.
    fn render_ir_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Internet Ranking");

        if self.player.irconfig.is_empty() {
            ui.label("No IR configurations.");
            if ui.button("Add IR Configuration").clicked() {
                self.player.irconfig.push(Some(IRConfig::default()));
            }
            return;
        }

        // IR config selector
        let ir_count = self.player.irconfig.len();
        let idx = self.selected_ir_index;
        if idx >= ir_count {
            self.selected_ir_index = 0;
        }
        let idx = self.selected_ir_index;

        ui.horizontal(|ui| {
            ui.label("IR Slot:");
            for i in 0..ir_count {
                if ui
                    .selectable_label(idx == i, format!("{}", i + 1))
                    .clicked()
                {
                    self.selected_ir_index = i;
                }
            }
            if ui.button("+").clicked() {
                self.player.irconfig.push(Some(IRConfig::default()));
            }
        });

        ui.separator();

        // Detect IR slot switch: flush old buffers, load new decrypted values
        if self.ir_prev_index != Some(idx) {
            self.flush_ir_buffers();
            self.load_ir_buffers(idx);
        }

        let idx = self.selected_ir_index;
        if let Some(Some(ir)) = self.player.irconfig.get_mut(idx) {
            egui::Grid::new("ir_grid").show(ui, |ui| {
                ui.label("IR Name:");
                ui.text_edit_singleline(&mut ir.irname);
                ui.end_row();

                ui.label("User ID:");
                ui.text_edit_singleline(&mut self.ir_userid_buf);
                ui.end_row();

                ui.label("Password:");
                ui.add(egui::TextEdit::singleline(&mut self.ir_password_buf).password(true));
                ui.end_row();

                let selected_label = IR_SEND_LABELS.get(ir.irsend as usize).unwrap_or(&"ALWAYS");
                ui.label("Send Mode:");
                egui::ComboBox::from_id_salt("ir_send_mode")
                    .selected_text(*selected_label)
                    .show_ui(ui, |ui| {
                        for (i, label) in IR_SEND_LABELS.iter().enumerate() {
                            ui.selectable_value(&mut ir.irsend, i as i32, *label);
                        }
                    });
                ui.end_row();

                ui.label("Import Rival:");
                ui.checkbox(&mut ir.importrival, "");
                ui.end_row();

                ui.label("Import Score:");
                ui.checkbox(&mut ir.importscore, "");
                ui.end_row();
            });
        }
    }

    /// Java equivalent: StreamEditorView
    /// Stream request settings.
    fn render_stream_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Stream Configuration");

        egui::Grid::new("stream_grid").show(ui, |ui| {
            ui.label("Enable Request:");
            ui.checkbox(&mut self.player.enable_request, "");
            ui.end_row();

            ui.label("Notify Request:");
            ui.checkbox(&mut self.player.notify_request, "");
            ui.end_row();

            ui.label("Max Request Count:");
            ui.add(egui::DragValue::new(&mut self.player.max_request_count).range(0..=100));
            ui.end_row();
        });
    }

    fn render_discord_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Discord Rich Presence");
        ui.checkbox(
            &mut self.config.use_discord_rpc,
            "Enable Discord Rich Presence",
        );
    }

    /// Java equivalent: ObsConfigurationView
    /// OBS WebSocket integration settings.
    fn render_obs_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("OBS WebSocket");

        egui::Grid::new("obs_grid").show(ui, |ui| {
            ui.label("Enable:");
            ui.checkbox(&mut self.config.use_obs_ws, "");
            ui.end_row();

            if self.config.use_obs_ws {
                ui.label("Host:");
                ui.text_edit_singleline(&mut self.config.obs_ws_host);
                ui.end_row();

                ui.label("Port:");
                ui.add(egui::DragValue::new(&mut self.config.obs_ws_port).range(1..=65535));
                ui.end_row();

                ui.label("Password:");
                ui.text_edit_singleline(&mut self.config.obs_ws_pass);
                ui.end_row();

                let selected_label = OBS_REC_MODE_LABELS
                    .get(self.config.obs_ws_rec_mode as usize)
                    .unwrap_or(&"DEFAULT");
                ui.label("Recording Mode:");
                egui::ComboBox::from_id_salt("obs_rec_mode")
                    .selected_text(*selected_label)
                    .show_ui(ui, |ui| {
                        for (i, label) in OBS_REC_MODE_LABELS.iter().enumerate() {
                            ui.selectable_value(&mut self.config.obs_ws_rec_mode, i as i32, *label);
                        }
                    });
                ui.end_row();

                ui.label("Rec Stop Wait:");
                ui.add(
                    egui::DragValue::new(&mut self.config.obs_ws_rec_stop_wait).range(0..=60000),
                );
                ui.end_row();
            }
        });
    }

    fn commit_config(&mut self) {
        self.config.playername = Some(self.player_name.clone());
        // Flush IR userid/password buffers (triggers AES encryption)
        self.flush_ir_buffers();
        // Commit skin configuration (saves to player.skin + skin_history)
        self.skin_view.commit();
        if let Some(updated_player) = self.skin_view.get_player() {
            self.player.skin = updated_player.skin.clone();
            self.player.skin_history = updated_player.skin_history.clone();
        }
        if let Err(e) = Config::write(&self.config) {
            log::error!("Failed to save config: {}", e);
        }
        if let Err(e) = PlayerConfig::write(&self.config.playerpath, &self.player) {
            log::error!("Failed to save player config: {}", e);
        }
    }
}

// LauncherUi — egui-based launcher configuration window
// Java equivalent: PlayConfigurationView (JavaFX Application)

use bms_model::mode::Mode;
use rubato_core::audio_config::{DriverType, FrequencyType};
use rubato_core::config::Config;
use rubato_core::ir_config::IRConfig;
use rubato_core::main_state::MainStateType;
use rubato_core::player_config::PlayerConfig;
use rubato_skin::skin_type::SkinType;

use crate::views::config::obs_configuration_view::{ACTION_NONE, SCENE_NONE};
use crate::views::play_configuration_view::PlayMode;
use crate::views::skin_configuration_view::{SkinConfigItem, SkinConfigurationView};

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
    /// Discord webhook URL list for editing.
    webhook_urls: Vec<String>,
    /// New webhook URL input buffer.
    webhook_url_input: String,
    /// OBS state names (ordered list for consistent rendering).
    obs_states: Vec<String>,
    /// OBS scene selection per state.
    obs_scene_selections: std::collections::HashMap<String, String>,
    /// OBS action selection per state.
    obs_action_selections: std::collections::HashMap<String, String>,
    /// Whether the "What's New" popup is open.
    show_whats_new: bool,
    /// What's New message text.
    whats_new_text: String,
    /// Chart details dialog state.
    chart_details_open: bool,
    /// Chart details dialog data (label, value) pairs.
    chart_details_data: Vec<(String, String)>,
    /// Set to true when the user clicks "Start" — signals the caller to launch play.
    /// Java: PlayConfigurationView.start() calls MainLoader.play()
    play_requested: bool,
    /// Set to true when the user clicks "Exit".
    /// Java: PlayConfigurationView.exit() calls commit() + System.exit(0)
    exit_requested: bool,
    /// Shared flag for play_requested, survives after eframe drops the App.
    /// Used by run_launcher() to detect whether play should be launched.
    shared_play_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
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
        let webhook_urls = config.webhook_url.clone();

        // Initialize OBS state rows
        let mut obs_states = Vec::new();
        let obs_state_types = [
            MainStateType::MusicSelect,
            MainStateType::Decide,
            MainStateType::Play,
            MainStateType::Result,
            MainStateType::CourseResult,
            MainStateType::Config,
            MainStateType::SkinConfig,
        ];
        let mut obs_scene_selections = std::collections::HashMap::new();
        let mut obs_action_selections = std::collections::HashMap::new();
        for state in &obs_state_types {
            let name = format!("{:?}", state);
            obs_states.push(name.clone());
            let scene = config.obs_scene(&name).cloned().unwrap_or_default();
            obs_scene_selections.insert(
                name.clone(),
                if scene.is_empty() {
                    SCENE_NONE.to_string()
                } else {
                    scene
                },
            );
            let action_label = config
                .obs_action(&name)
                .and_then(|a| rubato_external::obs::obs_ws_client::action_label(a))
                .unwrap_or_else(|| ACTION_NONE.to_string());
            obs_action_selections.insert(name.clone(), action_label);

            if name == "Play" {
                obs_states.push("PLAY_ENDED".to_string());
                let scene_ended = config.obs_scene("PLAY_ENDED").cloned().unwrap_or_default();
                obs_scene_selections.insert(
                    "PLAY_ENDED".to_string(),
                    if scene_ended.is_empty() {
                        SCENE_NONE.to_string()
                    } else {
                        scene_ended
                    },
                );
                let action_ended = config
                    .obs_action("PLAY_ENDED")
                    .and_then(|a| rubato_external::obs::obs_ws_client::action_label(a))
                    .unwrap_or_else(|| ACTION_NONE.to_string());
                obs_action_selections.insert("PLAY_ENDED".to_string(), action_ended);
            }
        }

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
            webhook_urls,
            webhook_url_input: String::new(),
            obs_states,
            obs_scene_selections,
            obs_action_selections,
            show_whats_new: false,
            whats_new_text: String::new(),
            chart_details_open: false,
            chart_details_data: Vec::new(),
            play_requested: false,
            exit_requested: false,
            shared_play_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Create a LauncherUi with a shared play_requested flag.
    /// Used by run_launcher() to detect play requests after eframe drops the App.
    fn new_with_shared_flag(
        config: Config,
        player: PlayerConfig,
        shared_play_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        let mut ui = Self::new(config, player);
        ui.shared_play_requested = shared_play_requested;
        ui
    }

    /// Returns true if the user has clicked "Start" and play should be launched.
    /// Java: PlayConfigurationView.start() triggers MainLoader.play()
    pub fn is_play_requested(&self) -> bool {
        self.play_requested
    }

    /// Returns a clone of the current Config.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a clone of the current PlayerConfig.
    pub fn player(&self) -> &PlayerConfig {
        &self.player
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

            // Popups
            self.render_popups(ui.ctx());

            // Action buttons at the bottom
            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    self.commit_config();
                    self.play_requested = true;
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
                    self.commit_config();
                    self.exit_requested = true;
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
            // Driver type selector
            let driver_label = match audio.driver {
                DriverType::OpenAL => "OpenAL",
                DriverType::PortAudio => "PortAudio",
            };
            ui.label("Driver:");
            egui::ComboBox::from_id_salt("audio_driver")
                .selected_text(driver_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut audio.driver, DriverType::OpenAL, "OpenAL");
                    ui.selectable_value(&mut audio.driver, DriverType::PortAudio, "PortAudio");
                });
            ui.end_row();

            // Driver name (PortAudio device selection)
            if audio.driver == DriverType::PortAudio {
                let driver_name_display = audio
                    .driver_name
                    .as_deref()
                    .unwrap_or("(default)")
                    .to_string();
                ui.label("Device:");
                egui::ComboBox::from_id_salt("audio_device_name")
                    .selected_text(&driver_name_display)
                    .show_ui(ui, |ui| {
                        if let Ok(devices) = crate::stubs::port_audio_devices() {
                            for device in &devices {
                                let mut name = audio.driver_name.clone().unwrap_or_default();
                                if ui
                                    .selectable_value(&mut name, device.name.clone(), &device.name)
                                    .changed()
                                {
                                    audio.driver_name = Some(name);
                                }
                            }
                        }
                    });
                ui.end_row();
            }

            ui.label("Audio Buffer:");
            ui.add(egui::DragValue::new(&mut audio.device_buffer_size).range(0..=9999));
            ui.end_row();

            ui.label("Max Simultaneous:");
            ui.add(egui::DragValue::new(&mut audio.device_simultaneous_sources).range(1..=256));
            ui.end_row();

            // Sample rate selector
            let sample_rate_label = if audio.sample_rate > 0 {
                audio.sample_rate.to_string()
            } else {
                "Auto".to_string()
            };
            ui.label("Sample Rate:");
            egui::ComboBox::from_id_salt("audio_sample_rate")
                .selected_text(&sample_rate_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut audio.sample_rate, 0, "Auto");
                    ui.selectable_value(&mut audio.sample_rate, 44100, "44100");
                    ui.selectable_value(&mut audio.sample_rate, 48000, "48000");
                });
            ui.end_row();

            // Frequency option
            let freq_label = match audio.freq_option {
                FrequencyType::UNPROCESSED => "Unprocessed",
                FrequencyType::FREQUENCY => "Frequency",
            };
            ui.label("Freq Option:");
            egui::ComboBox::from_id_salt("audio_freq_option")
                .selected_text(freq_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut audio.freq_option,
                        FrequencyType::UNPROCESSED,
                        "Unprocessed",
                    );
                    ui.selectable_value(
                        &mut audio.freq_option,
                        FrequencyType::FREQUENCY,
                        "Frequency",
                    );
                });
            ui.end_row();

            // Fast forward
            let ff_label = match audio.fast_forward {
                FrequencyType::UNPROCESSED => "Unprocessed",
                FrequencyType::FREQUENCY => "Frequency",
            };
            ui.label("Fast Forward:");
            egui::ComboBox::from_id_salt("audio_fast_forward")
                .selected_text(ff_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut audio.fast_forward,
                        FrequencyType::UNPROCESSED,
                        "Unprocessed",
                    );
                    ui.selectable_value(
                        &mut audio.fast_forward,
                        FrequencyType::FREQUENCY,
                        "Frequency",
                    );
                });
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

            ui.label("Normalize Volume:");
            ui.checkbox(&mut audio.normalize_volume, "");
            ui.end_row();

            ui.label("Loop Result Sound:");
            ui.checkbox(&mut audio.is_loop_result_sound, "");
            ui.end_row();

            ui.label("Loop Course Result Sound:");
            ui.checkbox(&mut audio.is_loop_course_result_sound, "");
            ui.end_row();
        });
    }

    /// Java equivalent: InputConfigurationView
    /// Keyboard/controller/mouse scratch settings per play mode.
    fn render_input_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Input Configuration");

        let mode = self.current_mode();
        let pmc = self.player.play_config(mode);

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
            .skintype_selector()
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
        let headers = self.skin_view.current_headers().to_owned();
        let header_count = headers.len();
        let selected_idx = self.skin_view.skinheader_selector();
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
        let items = self.skin_view.skinconfig_items_mut();
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
            self.ir_userid_buf = ir.userid();
            self.ir_password_buf = ir.password();
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
        ui.heading("Discord");

        ui.checkbox(
            &mut self.config.use_discord_rpc,
            "Enable Discord Rich Presence",
        );

        ui.separator();

        // Webhook configuration
        ui.heading("Webhook");

        egui::Grid::new("discord_webhook_grid").show(ui, |ui| {
            let webhook_options = ["All Clear", "FC / AAA", "Clear"];
            let selected_label = webhook_options
                .get(self.config.webhook_option as usize)
                .unwrap_or(&"All Clear");
            ui.label("Send On:");
            egui::ComboBox::from_id_salt("webhook_option")
                .selected_text(*selected_label)
                .show_ui(ui, |ui| {
                    for (i, label) in webhook_options.iter().enumerate() {
                        ui.selectable_value(&mut self.config.webhook_option, i as i32, *label);
                    }
                });
            ui.end_row();

            ui.label("Bot Name:");
            ui.text_edit_singleline(&mut self.config.webhook_name);
            ui.end_row();

            ui.label("Avatar URL:");
            ui.text_edit_singleline(&mut self.config.webhook_avatar);
            ui.end_row();
        });

        ui.separator();

        // Webhook URL table
        ui.label("Webhook URLs:");
        let mut remove_idx = None;
        for (i, url) in self.webhook_urls.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(url);
                if ui.small_button("Remove").clicked() {
                    remove_idx = Some(i);
                }
            });
        }
        if let Some(idx) = remove_idx {
            self.webhook_urls.remove(idx);
        }

        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.webhook_url_input);
            if ui.button("Add").clicked() && !self.webhook_url_input.is_empty() {
                let url = self.webhook_url_input.clone();
                if !self.webhook_urls.contains(&url) {
                    self.webhook_urls.push(url);
                }
                self.webhook_url_input.clear();
            }
        });
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

        if self.config.use_obs_ws {
            ui.separator();
            ui.heading("State Actions");

            // Action labels from OBS module
            let actions = rubato_external::obs::obs_ws_client::obs_actions();
            let action_labels: Vec<String> = std::iter::once(ACTION_NONE.to_string())
                .chain(actions.keys().cloned())
                .collect();

            let obs_states = self.obs_states.clone();
            egui::Grid::new("obs_state_grid")
                .min_col_width(100.0)
                .show(ui, |ui| {
                    ui.label("State");
                    ui.label("Scene");
                    ui.label("Action");
                    ui.end_row();

                    for state in &obs_states {
                        ui.label(state);

                        // Scene selector (disabled until connected, show saved value)
                        let scene_val = self
                            .obs_scene_selections
                            .entry(state.clone())
                            .or_insert_with(|| SCENE_NONE.to_string());
                        egui::ComboBox::from_id_salt(format!("obs_scene_{}", state))
                            .selected_text(scene_val.as_str())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(scene_val, SCENE_NONE.to_string(), SCENE_NONE);
                            });

                        // Action selector
                        let action_val = self
                            .obs_action_selections
                            .entry(state.clone())
                            .or_insert_with(|| ACTION_NONE.to_string());
                        egui::ComboBox::from_id_salt(format!("obs_action_{}", state))
                            .selected_text(action_val.as_str())
                            .show_ui(ui, |ui| {
                                for label in &action_labels {
                                    ui.selectable_value(action_val, label.clone(), label.as_str());
                                }
                            });

                        ui.end_row();
                    }
                });
        }
    }

    /// Render popup windows (What's New, Chart Details).
    fn render_popups(&mut self, ctx: &egui::Context) {
        if self.show_whats_new {
            let mut open = self.show_whats_new;
            egui::Window::new("What's New")
                .open(&mut open)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(&self.whats_new_text);
                    });
                    if ui.button("OK").clicked() {
                        self.show_whats_new = false;
                    }
                });
            self.show_whats_new = open;
        }

        if self.chart_details_open {
            let mut open = self.chart_details_open;
            egui::Window::new("Chart Details")
                .open(&mut open)
                .resizable(true)
                .default_width(500.0)
                .show(ctx, |ui| {
                    egui::Grid::new("chart_details_grid").show(ui, |ui| {
                        for (label, value) in &self.chart_details_data {
                            ui.label(label);
                            let mut val = value.clone();
                            ui.add(egui::TextEdit::singleline(&mut val));
                            ui.end_row();
                        }
                    });
                    if ui.button("OK").clicked() {
                        self.chart_details_open = false;
                    }
                });
            self.chart_details_open = open;
        }
    }

    /// Show the What's New popup with the given text.
    pub fn show_whats_new_popup(&mut self, text: String) {
        self.whats_new_text = text;
        self.show_whats_new = true;
    }

    /// Show the chart details dialog with the given data.
    pub fn show_chart_details(&mut self, data: Vec<(String, String)>) {
        self.chart_details_data = data;
        self.chart_details_open = true;
    }

    fn commit_config(&mut self) {
        self.config.playername = Some(self.player_name.clone());
        // Commit webhook URLs
        self.config.webhook_url = self.webhook_urls.clone();
        // Commit OBS scene/action selections
        let actions = rubato_external::obs::obs_ws_client::obs_actions();
        for state in &self.obs_states {
            if let Some(scene) = self.obs_scene_selections.get(state) {
                let scene_val = if scene == SCENE_NONE {
                    String::new()
                } else {
                    scene.clone()
                };
                self.config.set_obs_scene(state.clone(), Some(scene_val));
            }
            if let Some(action_label) = self.obs_action_selections.get(state) {
                if action_label == ACTION_NONE {
                    self.config
                        .set_obs_action(state.clone(), Some(String::new()));
                } else if let Some(action_req) = actions.get(action_label) {
                    self.config
                        .set_obs_action(state.clone(), Some(action_req.clone()));
                }
            }
        }
        // Flush IR userid/password buffers (triggers AES encryption)
        self.flush_ir_buffers();
        // Commit skin configuration (saves to player.skin + skin_history)
        self.skin_view.commit();
        if let Some(updated_player) = self.skin_view.player() {
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

/// eframe::App implementation for LauncherUi.
///
/// Java equivalent: JavaFX Application.start(Stage) → PlayConfigurationView scene rendering.
/// In Java, the JavaFX framework calls into the scene graph each frame.
/// In Rust, eframe calls update() each frame, which delegates to render_ui().
impl eframe::App for LauncherUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_ui(ctx);

        // Java: PlayConfigurationView.exit() calls commit() + System.exit(0)
        // In eframe, we close the viewport instead.
        if self.exit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Java: PlayConfigurationView.start() triggers MainLoader.play()
        // The play_requested flag is checked by the caller after run_native() returns.
        // When using eframe, we close the launcher window so play can begin.
        if self.play_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Java: PlayConfigurationView.exit() calls commit() before closing.
    /// eframe calls on_exit() when the window is being closed.
    fn on_exit(&mut self) {
        self.commit_config();
        // Persist play_requested to the shared atomic flag so run_launcher() can read it.
        self.shared_play_requested
            .store(self.play_requested, std::sync::atomic::Ordering::Release);
    }
}

/// Result of running the launcher UI.
///
/// After the eframe window closes, this struct holds the final Config/PlayerConfig
/// (re-read from disk after commit_config saved them) and whether "Start" was clicked.
pub struct LauncherResult {
    pub config: Config,
    pub player: PlayerConfig,
    pub play_requested: bool,
}

/// Launch the egui configuration window using eframe.
///
/// Java equivalent: MainLoader.start(Stage) → creates JavaFX Stage with PlayConfigurationView.
/// In Rust, this creates an eframe window with LauncherUi.
///
/// Returns LauncherResult after the window is closed, so the caller
/// can check play_requested and retrieve config/player for play().
pub fn run_launcher(
    config: Config,
    player: PlayerConfig,
    title: &str,
) -> anyhow::Result<LauncherResult> {
    // Shared atomic flag: survives after eframe drops the App.
    let shared_play_requested = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shared_clone = shared_play_requested.clone();

    let launcher = LauncherUi::new_with_shared_flag(config, player, shared_clone);

    // Java: primaryStage.setScene(scene); primaryStage.show();
    // eframe::run_native() blocks until the window is closed.
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        title,
        native_options,
        Box::new(move |_cc| Ok(Box::new(launcher))),
    )
    .map_err(|e| anyhow::anyhow!("eframe::run_native failed: {}", e))?;

    // After run_native returns, the App has been dropped (on_exit saved state).
    let play_requested = shared_play_requested.load(std::sync::atomic::Ordering::Acquire);

    // Re-read config/player from disk (commit_config saved them in on_exit).
    let config = Config::read().unwrap_or_default();
    let playerpath = &config.playerpath;
    let playername = config.playername.as_deref().unwrap_or("default");
    let player = PlayerConfig::read_player_config(playerpath, playername)
        .unwrap_or_else(|_| PlayerConfig::default());

    Ok(LauncherResult {
        config,
        player,
        play_requested,
    })
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn test_launcher_ui_new_defaults() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let ui = LauncherUi::new(config, player);

        assert!(!ui.is_play_requested());
        assert!(!ui.exit_requested);
        assert_eq!(ui.selected_tab, Tab::Option);
        assert_eq!(ui.selected_play_mode, 1); // BEAT_7K
    }

    #[test]
    fn test_launcher_ui_config_accessors() {
        let mut config = Config::default();
        config.vsync = true;
        config.max_frame_per_second = 120;
        let player = PlayerConfig::default();
        let ui = LauncherUi::new(config, player);

        assert!(ui.config().vsync);
        assert_eq!(ui.config().max_frame_per_second, 120);
    }

    #[test]
    fn test_launcher_ui_player_accessor() {
        let config = Config::default();
        let mut player = PlayerConfig::default();
        player.name = "test_player".to_string();
        let ui = LauncherUi::new(config, player);

        assert_eq!(ui.player().name, "test_player");
    }

    #[test]
    fn test_play_requested_initially_false() {
        let ui = LauncherUi::new(Config::default(), PlayerConfig::default());
        assert!(!ui.is_play_requested());
    }

    #[test]
    fn test_tab_all_returns_11_tabs() {
        // Java: PlayConfigurationView has 11 tabs
        assert_eq!(Tab::all().len(), 11);
    }

    #[test]
    fn test_tab_labels_non_empty() {
        for tab in Tab::all() {
            assert!(!tab.label().is_empty());
        }
    }
}

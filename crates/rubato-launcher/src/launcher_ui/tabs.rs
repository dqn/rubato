// Tab rendering methods for LauncherUi.
// Each method renders one configuration tab in the egui launcher.

use rubato_core::audio_config::{DriverType, FrequencyType};
use rubato_core::ir_config::IRConfig;
use rubato_skin::skin_type::SkinType;

use crate::views::skin_configuration_view::{SkinConfigItem, SkinConfigurationView};

use super::{IR_SEND_LABELS, LauncherUi};

impl LauncherUi {
    pub(super) fn render_video_tab(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("video_grid").show(ui, |ui| {
            ui.label("Resolution:");
            ui.label(format!(
                "{}x{}",
                self.config.display.resolution.width(),
                self.config.display.resolution.height()
            ));
            ui.end_row();

            ui.label("Display Mode:");
            ui.label(format!("{:?}", self.config.display.displaymode));
            ui.end_row();

            ui.label("VSync:");
            ui.checkbox(&mut self.config.display.vsync, "");
            ui.end_row();

            ui.label("Max FPS:");
            ui.add(
                egui::DragValue::new(&mut self.config.display.max_frame_per_second).range(0..=999),
            );
            ui.end_row();
        });
    }

    pub(super) fn render_audio_tab(&mut self, ui: &mut egui::Ui) {
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
                        if let Ok(devices) = crate::platform::port_audio_devices() {
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
    pub(super) fn render_input_tab(&mut self, ui: &mut egui::Ui) {
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

                    // Config values used as indices: negative i32 wraps to huge usize via
                    // `as usize`, but .get() returns None and falls through to the default.
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

    pub(super) fn render_music_select_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Music Select configuration");
        ui.label("BMS paths:");
        for path in &self.bms_paths {
            ui.label(path);
        }
        if ui.button("Add BMS folder...").clicked()
            && let Some(path) = crate::platform::show_directory_chooser("Select BMS folder")
        {
            self.bms_paths.push(path);
        }
    }

    /// Java equivalent: SkinConfigurationView
    /// Skin type selection, skin header browsing, and custom options/files/offsets.
    pub(super) fn render_skin_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Skin Configuration");

        ui.checkbox(
            &mut self.config.select.cache_skin_image,
            "Cache Skin Image (CIM)",
        );

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

    pub(super) fn render_option_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Play Options");

        egui::Grid::new("option_grid").show(ui, |ui| {
            ui.label("HiSpeed:");
            ui.label("(configured per play mode)");
            ui.end_row();

            ui.label("Target:");
            ui.label(self.player.select_settings.targetid.to_string());
            ui.end_row();
        });
    }

    /// Java equivalent: PlayConfigurationView "Other" tab
    /// IPFS, HTTP download, and screenshot settings.
    pub(super) fn render_other_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Other Settings");

        // Screenshot
        ui.checkbox(
            &mut self.config.integration.set_clipboard_screenshot,
            "Clipboard Screenshot",
        );

        ui.separator();

        // IPFS settings
        ui.label("IPFS");
        egui::Grid::new("ipfs_grid").show(ui, |ui| {
            ui.label("Enable:");
            ui.checkbox(&mut self.config.network.enable_ipfs, "");
            ui.end_row();

            if self.config.network.enable_ipfs {
                ui.label("IPFS URL:");
                ui.text_edit_singleline(&mut self.config.network.ipfsurl);
                ui.end_row();
            }
        });

        ui.separator();

        // HTTP download settings
        ui.label("HTTP Download");
        egui::Grid::new("http_grid").show(ui, |ui| {
            ui.label("Enable:");
            ui.checkbox(&mut self.config.network.enable_http, "");
            ui.end_row();

            if self.config.network.enable_http {
                ui.label("Download Source:");
                ui.text_edit_singleline(&mut self.config.network.download_source);
                ui.end_row();

                ui.label("Default URL:");
                ui.text_edit_singleline(&mut self.config.network.default_download_url);
                ui.end_row();

                ui.label("Override URL:");
                ui.text_edit_singleline(&mut self.config.network.override_download_url);
                ui.end_row();
            }
        });
    }

    /// Flush current IR userid/password buffers back to IRConfig via
    /// set_userid/set_password (triggers AES encryption).
    /// Java equivalent: IRConfigurationView.updateIRConnection() save-side.
    pub(super) fn flush_ir_buffers(&mut self) {
        if let Some(prev) = self.ir_prev_index
            && let Some(Some(ir)) = self.player.irconfig.get_mut(prev)
        {
            ir.set_userid(self.ir_userid_buf.clone());
            ir.set_password(self.ir_password_buf.clone());
        }
    }

    /// Load decrypted IR userid/password into buffers for the given index.
    /// Java equivalent: IRConfigurationView.updateIRConnection() load-side.
    pub(super) fn load_ir_buffers(&mut self, idx: usize) {
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
    pub(super) fn render_ir_tab(&mut self, ui: &mut egui::Ui) {
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
    pub(super) fn render_stream_tab(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn render_discord_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Discord");

        ui.checkbox(
            &mut self.config.integration.use_discord_rpc,
            "Enable Discord Rich Presence",
        );

        ui.separator();

        // Webhook configuration
        ui.heading("Webhook");

        egui::Grid::new("discord_webhook_grid").show(ui, |ui| {
            let webhook_options = ["Off", "FC / AAA", "Clear"];
            let selected_label = webhook_options
                .get(self.config.integration.webhook_option as usize)
                .unwrap_or(&"All Clear");
            ui.label("Send On:");
            egui::ComboBox::from_id_salt("webhook_option")
                .selected_text(*selected_label)
                .show_ui(ui, |ui| {
                    for (i, label) in webhook_options.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.config.integration.webhook_option,
                            i as i32,
                            *label,
                        );
                    }
                });
            ui.end_row();

            ui.label("Bot Name:");
            ui.text_edit_singleline(&mut self.config.integration.webhook_name);
            ui.end_row();

            ui.label("Avatar URL:");
            ui.text_edit_singleline(&mut self.config.integration.webhook_avatar);
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
    /// Delegates to ObsConfigurationView which handles connection, scene
    /// fetching, and per-state scene/action selectors.
    pub(super) fn render_obs_tab(&mut self, ui: &mut egui::Ui) {
        self.obs_view.render(ui);
    }
}

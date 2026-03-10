// egui rendering logic for SkinConfigurationView.
// Extracted from mod.rs for navigability.

use super::{SkinConfigItem, SkinConfigurationView};
use rubato_skin::skin_header::TYPE_BEATORJASKIN;
use rubato_skin::skin_type::SkinType;

impl SkinConfigurationView {
    /// Render the skin configuration view using egui.
    ///
    /// Displays skin type selector, skin header selector, and all dynamic
    /// config items (options, files, offsets) built by the `create()` method.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // Skin type selector
        let skin_types = SkinType::values();
        let current_type = self.skintype_selector.unwrap_or(SkinType::Play7Keys);
        ui.horizontal(|ui| {
            ui.label("Category:");
            let mut new_type = current_type;
            egui::ComboBox::from_id_salt("skin_type_selector")
                .selected_text(Self::skin_type_display_name(&current_type))
                .show_ui(ui, |ui| {
                    for st in &skin_types {
                        ui.selectable_value(&mut new_type, *st, Self::skin_type_display_name(st));
                    }
                });
            if new_type != current_type {
                self.skintype_selector = Some(new_type);
                self.change_skin_type();
            }
        });

        // Skin header selector
        let headers = self.current_headers.clone();
        let selected_idx = self.skinheader_selector;
        if headers.is_empty() {
            ui.label("(no skins found)");
        } else {
            let display = selected_idx
                .and_then(|i| headers.get(i))
                .map(Self::skin_header_display_name)
                .unwrap_or_else(|| "(none)".to_string());
            let mut new_idx = selected_idx.unwrap_or(0);
            ui.horizontal(|ui| {
                ui.label("Skin:");
                egui::ComboBox::from_id_salt("skin_header_selector")
                    .selected_text(display)
                    .show_ui(ui, |ui| {
                        for (i, header) in headers.iter().enumerate() {
                            let name = Self::skin_header_display_name(header);
                            ui.selectable_value(&mut new_idx, i, name);
                        }
                    });
            });
            if Some(new_idx) != selected_idx {
                self.skinheader_selector = Some(new_idx);
                self.change_skin_header();
            }
        }

        ui.separator();

        // Dynamic skin config items (options, files, offsets)
        egui::Grid::new("skin_config_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                for item in self.skinconfig_items.iter_mut() {
                    match item {
                        SkinConfigItem::Label(text) => {
                            if text.is_empty() {
                                ui.add_space(4.0);
                                ui.add_space(4.0);
                            } else {
                                ui.label(egui::RichText::new(text.as_str()).strong());
                                ui.label(""); // empty second column
                            }
                            ui.end_row();
                        }
                        SkinConfigItem::Option {
                            name,
                            items: combo_items,
                            selected_index,
                        } => {
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
                            ui.end_row();
                        }
                        SkinConfigItem::File {
                            name,
                            items: combo_items,
                            selected_value,
                        } => {
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
                            ui.end_row();
                        }
                        SkinConfigItem::Offset {
                            name,
                            values,
                            enabled,
                        } => {
                            ui.label(format!("{}:", name));
                            ui.horizontal(|ui| {
                                let labels = ["x", "y", "w", "h", "r", "a"];
                                for (i, &label) in labels.iter().enumerate() {
                                    if enabled[i] {
                                        ui.label(label);
                                        ui.add(
                                            egui::DragValue::new(&mut values[i])
                                                .range(-9999..=9999),
                                        );
                                    }
                                }
                            });
                            ui.end_row();
                        }
                    }
                }
            });
    }

    /// Helper: Get skin type display name for SkinTypeCell
    /// Translates: SkinTypeCell.updateItem(SkinType, boolean)
    pub fn skin_type_display_name(skin_type: &SkinType) -> &'static str {
        skin_type.name()
    }

    /// Helper: Get skin header display name for SkinListCell
    /// Translates: SkinListCell.updateItem(SkinHeader, boolean)
    pub fn skin_header_display_name(header: &rubato_skin::skin_header::SkinHeader) -> String {
        let name = header.name().unwrap_or("");
        if header.toast_type() == TYPE_BEATORJASKIN {
            name.to_string()
        } else {
            format!("{} (LR2 Skin)", name)
        }
    }
}

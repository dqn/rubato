// Skin options menu — displays and edits current skin's custom options and files.
//
// Shows #CUSTOMOPTION and #CUSTOMFILE entries parsed from the skin.
// Options are editable via ComboBox widgets.

/// State for the skin options panel.
#[derive(Debug, Clone, Default)]
pub struct SkinOptionsState {
    /// Skin name.
    pub skin_name: String,
    /// Custom options from the skin definition.
    pub options: Vec<CustomOption>,
    /// Custom file entries from the skin definition.
    pub files: Vec<CustomFile>,
}

/// A #CUSTOMOPTION entry.
#[derive(Debug, Clone)]
pub struct CustomOption {
    pub name: String,
    pub op_index: i32,
    pub choices: Vec<String>,
    pub selected: usize,
}

/// A #CUSTOMFILE entry.
#[derive(Debug, Clone)]
pub struct CustomFile {
    pub name: String,
    pub path_pattern: String,
    pub selected_path: Option<String>,
    /// Available file paths matching the pattern.
    pub available_paths: Vec<String>,
}

impl SkinOptionsState {
    /// Load skin option data.
    pub fn load(&mut self, skin_name: String, options: Vec<CustomOption>, files: Vec<CustomFile>) {
        self.skin_name = skin_name;
        self.options = options;
        self.files = files;
    }

    /// Clear all data.
    pub fn clear(&mut self) {
        self.skin_name.clear();
        self.options.clear();
        self.files.clear();
    }
}

/// Render the skin options panel. Returns `true` if any option was changed.
pub fn render(ctx: &egui::Context, open: &mut bool, state: &mut SkinOptionsState) -> bool {
    let mut changed = false;
    egui::Window::new("Skin Options")
        .open(open)
        .resizable(true)
        .default_width(400.0)
        .show(ctx, |ui| {
            if state.skin_name.is_empty() {
                ui.label("No skin loaded.");
                return;
            }

            ui.heading(&state.skin_name);
            ui.separator();

            // Custom options section
            egui::CollapsingHeader::new(format!("Custom Options ({})", state.options.len()))
                .default_open(true)
                .show(ui, |ui| {
                    if state.options.is_empty() {
                        ui.label("No custom options defined.");
                    } else {
                        egui::Grid::new("skin_options_grid")
                            .num_columns(3)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.strong("Name");
                                ui.strong("OP");
                                ui.strong("Value");
                                ui.end_row();

                                for (i, opt) in state.options.iter_mut().enumerate() {
                                    ui.label(&opt.name);
                                    ui.label(format!("OP{}", opt.op_index));
                                    let display = opt
                                        .choices
                                        .get(opt.selected)
                                        .cloned()
                                        .unwrap_or_else(|| format!("#{}", opt.selected));
                                    let combo =
                                        egui::ComboBox::from_id_salt(format!("skin_opt_{i}"))
                                            .selected_text(display)
                                            .show_ui(ui, |ui| {
                                                for (j, label) in opt.choices.iter().enumerate() {
                                                    if ui
                                                        .selectable_value(
                                                            &mut opt.selected,
                                                            j,
                                                            label,
                                                        )
                                                        .changed()
                                                    {
                                                        changed = true;
                                                    }
                                                }
                                            });
                                    let _ = combo;
                                    ui.end_row();
                                }
                            });
                    }
                });

            // Custom files section
            egui::CollapsingHeader::new(format!("Custom Files ({})", state.files.len()))
                .default_open(true)
                .show(ui, |ui| {
                    if state.files.is_empty() {
                        ui.label("No custom files defined.");
                    } else {
                        egui::Grid::new("skin_files_grid")
                            .num_columns(3)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.strong("Name");
                                ui.strong("Pattern");
                                ui.strong("Selected");
                                ui.end_row();

                                for (i, file) in state.files.iter_mut().enumerate() {
                                    ui.label(&file.name);
                                    ui.label(&file.path_pattern);
                                    if file.available_paths.is_empty() {
                                        ui.label(file.selected_path.as_deref().unwrap_or("(none)"));
                                    } else {
                                        let display = file
                                            .selected_path
                                            .as_deref()
                                            .unwrap_or("(none)")
                                            .to_string();
                                        egui::ComboBox::from_id_salt(format!("skin_file_{i}"))
                                            .selected_text(&display)
                                            .show_ui(ui, |ui| {
                                                for path in &file.available_paths {
                                                    let is_selected = file.selected_path.as_deref()
                                                        == Some(path.as_str());
                                                    if ui
                                                        .selectable_label(is_selected, path)
                                                        .clicked()
                                                        && !is_selected
                                                    {
                                                        file.selected_path = Some(path.clone());
                                                        changed = true;
                                                    }
                                                }
                                            });
                                    }
                                    ui.end_row();
                                }
                            });
                    }
                });
        });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty() {
        let state = SkinOptionsState::default();
        assert!(state.skin_name.is_empty());
        assert!(state.options.is_empty());
        assert!(state.files.is_empty());
    }

    #[test]
    fn load_and_clear() {
        let mut state = SkinOptionsState::default();
        state.load(
            "TestSkin".into(),
            vec![CustomOption {
                name: "BG Color".into(),
                op_index: 900,
                choices: vec!["Black".into(), "White".into()],
                selected: 0,
            }],
            vec![CustomFile {
                name: "Bomb".into(),
                path_pattern: "img/bomb/*.png".into(),
                selected_path: Some("img/bomb/default.png".into()),
                available_paths: vec!["img/bomb/default.png".into(), "img/bomb/blue.png".into()],
            }],
        );
        assert_eq!(state.skin_name, "TestSkin");
        assert_eq!(state.options.len(), 1);
        assert_eq!(state.files.len(), 1);

        state.clear();
        assert!(state.skin_name.is_empty());
        assert!(state.options.is_empty());
        assert!(state.files.is_empty());
    }

    #[test]
    fn custom_option_selected_change() {
        let mut opt = CustomOption {
            name: "Lane Cover".into(),
            op_index: 910,
            choices: vec!["Off".into(), "On".into(), "Lift".into()],
            selected: 0,
        };
        assert_eq!(opt.selected, 0);
        opt.selected = 2;
        assert_eq!(opt.selected, 2);
        assert_eq!(opt.choices[opt.selected], "Lift");
    }

    #[test]
    fn custom_file_available_paths() {
        let mut file = CustomFile {
            name: "Bomb".into(),
            path_pattern: "img/bomb/*.png".into(),
            selected_path: None,
            available_paths: vec!["img/bomb/a.png".into(), "img/bomb/b.png".into()],
        };
        assert!(file.selected_path.is_none());
        file.selected_path = Some(file.available_paths[1].clone());
        assert_eq!(file.selected_path.as_deref(), Some("img/bomb/b.png"));
    }
}

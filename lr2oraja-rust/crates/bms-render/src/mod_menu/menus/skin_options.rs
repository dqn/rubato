// Skin options menu — displays current skin's custom options and files.
//
// Shows #CUSTOMOPTION and #CUSTOMFILE entries parsed from the skin.
// Value editing is a stub (display only) for now.

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

pub fn render(ctx: &egui::Context, open: &mut bool, state: &mut SkinOptionsState) {
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

                                for opt in &state.options {
                                    ui.label(&opt.name);
                                    ui.label(format!("OP{}", opt.op_index));
                                    let display = opt
                                        .choices
                                        .get(opt.selected)
                                        .cloned()
                                        .unwrap_or_else(|| format!("#{}", opt.selected));
                                    ui.label(display);
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

                                for file in &state.files {
                                    ui.label(&file.name);
                                    ui.label(&file.path_pattern);
                                    ui.label(file.selected_path.as_deref().unwrap_or("(none)"));
                                    ui.end_row();
                                }
                            });
                    }
                });
        });
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
}

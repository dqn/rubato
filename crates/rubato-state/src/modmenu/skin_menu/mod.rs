mod config;
mod header_converters;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

use super::imgui_renderer;
use super::{
    CustomCategoryItem, CustomFile, CustomOffset, CustomOption, MainController,
    OPTION_RANDOM_VALUE, PlayerConfig, SkinHeader, SkinType,
};

use config::{
    dirty, get_file_setting, get_offset_setting, get_option_setting, refresh,
    reset_current_skin_config, switch_current_scene_skin,
};
use rubato_types::sync_utils::lock_or_recover;

static MAIN: Mutex<Option<MainController>> = Mutex::new(None);
static PLAYER_CONFIG: Mutex<Option<PlayerConfig>> = Mutex::new(None);

static READY: Mutex<bool> = Mutex::new(false);
static LIVE_EDITING: Mutex<bool> = Mutex::new(true);
static FREEZE_TIMERS: Mutex<bool> = Mutex::new(false);

static CURRENT_SKIN_TYPE: Mutex<Option<SkinType>> = Mutex::new(None);
static CURRENT_SKIN: Mutex<Option<SkinHeader>> = Mutex::new(None);
static SET_OPTIONS: Mutex<Option<HashMap<String, i32>>> = Mutex::new(None);
static AVAILABLE_FILES: Mutex<Option<HashMap<String, Vec<String>>>> = Mutex::new(None);
static SET_FILES: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
static SET_OFFSETS: Mutex<Option<HashMap<String, OffsetValue>>> = Mutex::new(None);
static SKINS: Mutex<Vec<SkinHeader>> = Mutex::new(Vec::new());
static DIRTY_CONFIG: Mutex<bool> = Mutex::new(false);

#[derive(Clone, Copy, Debug)]
pub struct OffsetValue {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub r: i32,
    pub a: i32,
}

impl OffsetValue {
    pub fn new(x: i32, y: i32, w: i32, h: i32, r: i32, a: i32) -> Self {
        OffsetValue { x, y, w, h, r, a }
    }
}

pub struct SkinMenu;

impl SkinMenu {
    pub fn init(main: MainController, player_config: PlayerConfig) {
        *lock_or_recover(&MAIN) = Some(main);
        *lock_or_recover(&PLAYER_CONFIG) = Some(player_config);
    }

    pub fn invalidate() {
        *lock_or_recover(&READY) = false;
    }

    /// Render the skin configuration window using egui.
    ///
    /// Translated from: SkinMenu.show(ImBoolean)
    pub fn show_ui(ctx: &egui::Context) {
        let main = lock_or_recover(&MAIN);
        if main.is_none() {
            return;
        }
        drop(main);

        let ready = *lock_or_recover(&READY);
        if !ready {
            refresh();
        }

        // Window size hint: height = 30% of window (Java: ImGui.setNextWindowSize(0, windowHeight * 0.3f, FirstUseEver))
        let default_height = imgui_renderer::window_height() as f32 * 0.3;

        let mut open = true;
        egui::Window::new("Skin")
            .open(&mut open)
            .default_height(default_height)
            .show(ctx, |ui| {
                menu_header(ui);
                ui.separator();
                skin_config_menu(ui);
            });
    }
}

/// Render the skin menu header: skin selector, save/reset buttons, live editing, freeze timers.
///
/// Translated from: SkinMenu.menuHeader()
fn menu_header(ui: &mut egui::Ui) {
    let skins = lock_or_recover(&SKINS);
    let current_skin = lock_or_recover(&CURRENT_SKIN);

    if let Some(ref skin) = *current_skin {
        let current_name = skin.name().map(|n| n.to_string()).unwrap_or_default();
        let current_path = skin
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let skin_count = skins.len();

        // Find current index in skins list
        let current_index = skins.iter().position(|s| s.name() == skin.name());

        // Collect skin names for the combo
        let _skin_names: Vec<String> = skins
            .iter()
            .filter_map(|s| s.name().map(|n| n.to_string()))
            .collect();

        drop(current_skin);
        drop(skins);

        // Arrow left + Combo + Arrow right for skin selection
        ui.horizontal(|ui| {
            // Left arrow button
            if ui.button("\u{25C0}").clicked()
                && let Some(idx) = current_index
            {
                let new_idx = (idx + skin_count - 1) % skin_count;
                let skins = lock_or_recover(&SKINS);
                if new_idx < skins.len() {
                    let header = skins[new_idx].clone();
                    drop(skins);
                    switch_current_scene_skin(header);
                }
            }

            // Skin selector combo
            let mut selected_name = current_name.clone();
            egui::ComboBox::from_id_salt("skin-select-combo")
                .selected_text(&selected_name)
                .width(ui.available_width() * 0.5)
                .show_ui(ui, |ui| {
                    let skins = lock_or_recover(&SKINS);
                    for header in skins.iter() {
                        let name = header.name().map(|n| n.to_string()).unwrap_or_default();
                        if ui.selectable_label(name == selected_name, &name).clicked() {
                            selected_name = name;
                        }
                    }
                });
            // If a different skin was selected via combo, switch to it
            if selected_name != current_name {
                let skins = lock_or_recover(&SKINS);
                if let Some(header) = skins
                    .iter()
                    .find(|s| s.name().map(|n| n.to_string()).unwrap_or_default() == selected_name)
                {
                    let h = header.clone();
                    drop(skins);
                    switch_current_scene_skin(h);
                }
            }

            // Right arrow button
            if ui.button("\u{25B6}").clicked()
                && let Some(idx) = current_index
            {
                let skins = lock_or_recover(&SKINS);
                let new_idx = (idx + 1) % skins.len();
                if new_idx < skins.len() {
                    let header = skins[new_idx].clone();
                    drop(skins);
                    switch_current_scene_skin(header);
                }
            }

            // Open skin location button
            if ui.button("Open").clicked() {
                // Desktop.open — platform-specific, best effort
                log::info!("Open skin location: {}", current_path);
            }
        });

        // Skin path display
        ui.label(egui::RichText::new(format!("> {}", current_path)).weak());

        // Save / Live Editing / Reset / Freeze timers
        ui.horizontal(|ui| {
            let is_dirty = *lock_or_recover(&DIRTY_CONFIG);
            let live_editing = *lock_or_recover(&LIVE_EDITING);
            let save_available = is_dirty && !live_editing;

            // Save button
            ui.add_enabled_ui(save_available, |ui| {
                let save_requested = ui.button(" Save ").clicked();
                if save_requested || (is_dirty && live_editing) {
                    let current_skin = lock_or_recover(&CURRENT_SKIN);
                    if let Some(ref cs) = *current_skin {
                        let h = cs.clone();
                        drop(current_skin);
                        switch_current_scene_skin(h);
                    }
                }
            });

            // Live Editing checkbox
            let mut le = *lock_or_recover(&LIVE_EDITING);
            if ui.checkbox(&mut le, "Live Editing").changed() {
                dirty(true);
            }
            *lock_or_recover(&LIVE_EDITING) = le;

            // Reset button with confirmation popup
            let reset_popup_id = ui.make_persistent_id("skin-setting-reset-confirmation");
            let reset_response = ui.button(" Reset ");
            if reset_response.clicked() {
                ui.memory_mut(|mem| mem.toggle_popup(reset_popup_id));
            }

            egui::popup_below_widget(
                ui,
                reset_popup_id,
                &reset_response,
                egui::PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    ui.label("Reset current skin's settings to default");
                    ui.label("ARE YOU SURE?");
                    ui.horizontal(|ui| {
                        if ui.button(" Confirm ").clicked() {
                            reset_current_skin_config();
                            let current_skin = lock_or_recover(&CURRENT_SKIN);
                            if let Some(ref cs) = *current_skin {
                                let h = cs.clone();
                                drop(current_skin);
                                switch_current_scene_skin(h);
                            }
                            ui.memory_mut(|mem| mem.toggle_popup(reset_popup_id));
                        }
                        ui.label(egui::RichText::new("(click outside popup to close)").weak());
                    });
                },
            );

            // Freeze timers checkbox
            let mut ft = *lock_or_recover(&FREEZE_TIMERS);
            if ui.checkbox(&mut ft, "Freeze timers").changed() {
                // Wire to TimerManager.frozen via MainController.
                // TimerManager::frozen controls whether update() advances time.
                if let Some(ref mut _main) = *lock_or_recover(&MAIN) {
                    // NullMainController has no timer; freeze-timers requires a real MainController.
                    log::warn!(
                        "Freeze timers toggled but MainController is NullMainController — no-op"
                    );
                }
                log::info!("Freeze timers: {}", ft);
            }
            *lock_or_recover(&FREEZE_TIMERS) = ft;
        });
    } else {
        drop(current_skin);
        drop(skins);
        ui.label("No skin loaded");
    }
}

/// Render the skin configuration options/files/offsets with category tab bar.
///
/// Translated from: SkinMenu.skinConfigMenu()
fn skin_config_menu(ui: &mut egui::Ui) {
    let current_skin = lock_or_recover(&CURRENT_SKIN);
    if current_skin.is_none() {
        return;
    }
    let skin = current_skin.as_ref().expect("current_skin is Some").clone();
    drop(current_skin);

    let mut shown = HashSet::new();
    let categories = skin.custom_categories().to_vec();
    let has_tabs = !categories.is_empty();

    if has_tabs {
        // Tab bar with categories
        // egui doesn't have native tab bars, so we use a horizontal row of selectable labels
        // combined with a persistent selected tab index.
        let tab_id = ui.make_persistent_id("skin-config-tab");
        let mut selected_tab: usize = ui.data(|data| data.get_temp::<usize>(tab_id)).unwrap_or(0);

        ui.horizontal(|ui| {
            for (idx, category) in categories.iter().enumerate() {
                if ui
                    .selectable_label(selected_tab == idx, &category.name)
                    .clicked()
                {
                    selected_tab = idx;
                }
            }
            // "Other" tab for uncategorized items
            if ui
                .selectable_label(selected_tab == categories.len(), "Other")
                .clicked()
            {
                selected_tab = categories.len();
            }
        });

        ui.data_mut(|data| data.insert_temp(tab_id, selected_tab));
        ui.separator();

        if selected_tab < categories.len() {
            // Render the selected category tab
            let category = &categories[selected_tab];
            // Track shown items across ALL categories (for the "Other" tab)
            for cat in &categories {
                for item in &cat.items {
                    match item {
                        CustomCategoryItem::Option(option) => {
                            shown.insert(option.name.clone());
                        }
                        CustomCategoryItem::File(file) => {
                            shown.insert(file.name.clone());
                        }
                        CustomCategoryItem::Offset(offset) => {
                            shown.insert(offset.name.clone());
                        }
                    }
                }
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for item in &category.items {
                    match item {
                        CustomCategoryItem::Option(option) => {
                            skin_config_option(ui, option);
                        }
                        CustomCategoryItem::File(file) => {
                            skin_config_file(ui, file);
                        }
                        CustomCategoryItem::Offset(offset) => {
                            skin_config_offset(ui, offset);
                        }
                    }
                }
            });
        } else {
            // "Other" tab — show uncategorized items
            for cat in &categories {
                for item in &cat.items {
                    match item {
                        CustomCategoryItem::Option(option) => {
                            shown.insert(option.name.clone());
                        }
                        CustomCategoryItem::File(file) => {
                            shown.insert(file.name.clone());
                        }
                        CustomCategoryItem::Offset(offset) => {
                            shown.insert(offset.name.clone());
                        }
                    }
                }
            }
            render_uncategorized(ui, &skin, &shown);
        }
    } else {
        // No tabs — render all items directly
        egui::ScrollArea::vertical().show(ui, |ui| {
            render_uncategorized(ui, &skin, &shown);
        });
    }
}

/// Render uncategorized options/files/offsets (items not in any category).
fn render_uncategorized(ui: &mut egui::Ui, skin: &SkinHeader, shown: &HashSet<String>) {
    let options = skin.custom_options();
    for option in options {
        if shown.contains(&option.name) {
            continue;
        }
        skin_config_option(ui, option);
    }
    let files = skin.custom_files();
    for file in files {
        if shown.contains(&file.name) {
            continue;
        }
        skin_config_file(ui, file);
    }
    let offsets = skin.custom_offsets();
    for offset in offsets {
        if shown.contains(&offset.name) {
            continue;
        }
        skin_config_offset(ui, offset);
    }
}

/// Render a skin option with arrow buttons and combo box.
///
/// Translated from: SkinMenu.skinConfigOption(CustomOption)
fn skin_config_option(ui: &mut egui::Ui, option: &CustomOption) {
    // Options with 2 choices (3 with random) use radio buttons instead
    let options_count = option.contents.len() + 1; // +1 for "Random"
    if options_count == 3 {
        skin_config_option_radio(ui, option);
        return;
    }

    let value = get_option_setting(option);
    let mut selected = option_index(option, value);
    let chosen = if selected == OPTION_RANDOM_VALUE {
        "Random".to_string()
    } else if (selected as usize) < option.contents.len() {
        option.contents[selected as usize].clone()
    } else {
        "Random".to_string()
    };

    let mut arrow_changed = false;

    ui.push_id(&option.name, |ui| {
        ui.horizontal(|ui| {
            // Left arrow button
            if ui.button("\u{25C0}").clicked() {
                selected = (selected + options_count as i32 - 1) % options_count as i32;
                arrow_changed = true;
            }

            // Combo box for option selection
            let mut new_chosen = chosen.clone();
            let combo_width = (ui.available_width() / 3.5).max(80.0);
            egui::ComboBox::from_id_salt("##combo")
                .selected_text(&chosen)
                .width(combo_width)
                .show_ui(ui, |ui| {
                    for (i, content) in option.contents.iter().enumerate() {
                        if ui
                            .selectable_label(content == &chosen, content.as_str())
                            .clicked()
                        {
                            if let Some(ref mut opts) = *lock_or_recover(&SET_OPTIONS) {
                                opts.insert(option.name.clone(), option.option[i]);
                            }
                            dirty(true);
                            new_chosen = content.clone();
                        }
                    }
                    if ui.selectable_label("Random" == chosen, "Random").clicked() {
                        if let Some(ref mut opts) = *lock_or_recover(&SET_OPTIONS) {
                            opts.insert(option.name.clone(), OPTION_RANDOM_VALUE);
                        }
                        dirty(true);
                        new_chosen = "Random".to_string();
                    }
                });

            // Right arrow button
            if ui.button("\u{25B6}").clicked() {
                selected = (selected + 1) % options_count as i32;
                arrow_changed = true;
            }

            if arrow_changed {
                if selected as usize == option.contents.len() {
                    if let Some(ref mut opts) = *lock_or_recover(&SET_OPTIONS) {
                        opts.insert(option.name.clone(), OPTION_RANDOM_VALUE);
                    }
                } else if (selected as usize) < option.option.len()
                    && let Some(ref mut opts) = *lock_or_recover(&SET_OPTIONS)
                {
                    opts.insert(option.name.clone(), option.option[selected as usize]);
                }
                dirty(true);
            }

            ui.label(&option.name);
        });
    });
}

/// Render a skin option with radio buttons (for options with exactly 2 choices).
///
/// Translated from: SkinMenu.skinConfigOptionRadio(CustomOption)
fn skin_config_option_radio(ui: &mut egui::Ui, option: &CustomOption) {
    ui.push_id(&option.name, |ui| {
        ui.label(&option.name);
        let mut value = get_option_setting(option);
        let original_value = value;

        ui.indent("radio-indent", |ui| {
            ui.horizontal(|ui| {
                for (&opt, content) in option.option.iter().zip(option.contents.iter()) {
                    ui.radio_value(&mut value, opt, content.as_str());
                }
                ui.radio_value(&mut value, OPTION_RANDOM_VALUE, "Random");
            });
        });

        if value != original_value {
            if let Some(ref mut opts) = *lock_or_recover(&SET_OPTIONS) {
                opts.insert(option.name.clone(), value);
            }
            dirty(true);
        }
    });
}

fn option_index(option: &CustomOption, value: i32) -> i32 {
    option
        .option
        .iter()
        .position(|&o| o == value)
        .map_or(OPTION_RANDOM_VALUE, |i| i as i32)
}

/// Render a skin file selector with arrow buttons and combo box.
///
/// Translated from: SkinMenu.skinConfigFile(CustomFile)
fn skin_config_file(ui: &mut egui::Ui, file: &CustomFile) {
    let selection = get_file_setting(file);
    let available = lock_or_recover(&AVAILABLE_FILES);
    if selection.is_none() || available.as_ref().and_then(|m| m.get(&file.name)).is_none() {
        return;
    }
    let choices = available
        .as_ref()
        .expect("available checked above")
        .get(&file.name)
        .cloned()
        .unwrap_or_default();
    drop(available);

    if choices.is_empty() {
        return;
    }

    let selection = selection.unwrap_or_default();
    let mut index = choices.iter().position(|c| c == &selection).unwrap_or(0);
    let max = choices.len();

    ui.push_id(&file.name, |ui| {
        ui.horizontal(|ui| {
            // Tinted frame background for file selectors (Java: pushStyleColor FrameBg)
            // Left arrow
            if ui.button("\u{25C0}").clicked() {
                index = (index + max - 1) % max;
                if let Some(ref mut files) = *lock_or_recover(&SET_FILES) {
                    files.insert(file.name.clone(), choices[index].clone());
                }
                dirty(true);
            }

            // File combo
            let combo_width = (ui.available_width() / 3.0).max(100.0);
            egui::ComboBox::from_id_salt("##file-combo")
                .selected_text(&selection)
                .width(combo_width)
                .show_ui(ui, |ui| {
                    for path in &choices {
                        if ui
                            .selectable_label(path == &selection, path.as_str())
                            .clicked()
                        {
                            if let Some(ref mut files) = *lock_or_recover(&SET_FILES) {
                                files.insert(file.name.clone(), path.clone());
                            }
                            dirty(true);
                        }
                    }
                });

            // Right arrow
            if ui.button("\u{25B6}").clicked() {
                index = (index + 1) % max;
                if let Some(ref mut files) = *lock_or_recover(&SET_FILES) {
                    files.insert(file.name.clone(), choices[index].clone());
                }
                dirty(true);
            }

            ui.label(&file.name);
        });

        // Show normalized path below
        let normalized = file
            .path
            .replace('*', "_WILDCARDESCAPE_")
            .replace('|', "_PIPEESCAPE_");
        let normalized_path = PathBuf::from(&normalized);
        let display = normalized_path
            .to_string_lossy()
            .replace("_WILDCARDESCAPE_", "*")
            .replace("_PIPEESCAPE_", "|");
        ui.label(egui::RichText::new(format!("  > {}", display)).weak());
    });
}

/// Render a drag-int widget for an offset component.
///
/// Translated from: SkinMenu.spawnDragInt(String, boolean, int[])
fn spawn_drag_int(ui: &mut egui::Ui, name: &str, offset_enabled: bool, value: &mut i32) {
    if offset_enabled {
        let drag = egui::DragValue::new(value)
            .speed(0.166)
            .prefix(format!("{} = ", name));
        if ui.add(drag).changed() {
            dirty(true);
        }
    } else {
        // Empty placeholder for alignment (Java: ImGui.dummy(100, 0))
        ui.add_space(100.0);
    }
}

/// Render a skin offset configuration with drag values for X/Y/W/H/R/A.
///
/// Translated from: SkinMenu.skinConfigOffset(CustomOffset)
fn skin_config_offset(ui: &mut egui::Ui, offset: &CustomOffset) {
    let mut value = get_offset_setting(offset);

    ui.push_id(&offset.name, |ui| {
        ui.label(&offset.name);

        ui.indent("offset-indent", |ui| {
            // Row 1: X, W, a
            if offset.caps.x || offset.caps.w || offset.caps.a {
                ui.horizontal(|ui| {
                    spawn_drag_int(ui, "X", offset.caps.x, &mut value.x);
                    spawn_drag_int(ui, "W", offset.caps.w, &mut value.w);
                    spawn_drag_int(ui, "a", offset.caps.a, &mut value.a);
                });
            }

            // Row 2: Y, H, R
            if offset.caps.y || offset.caps.h || offset.caps.r {
                ui.horizontal(|ui| {
                    spawn_drag_int(ui, "Y", offset.caps.y, &mut value.y);
                    spawn_drag_int(ui, "H", offset.caps.h, &mut value.h);
                    spawn_drag_int(ui, "R", offset.caps.r, &mut value.r);
                });
            }
        });
    });

    // Write back the modified offset values
    let mut offsets = lock_or_recover(&SET_OFFSETS);
    let map = offsets.get_or_insert_with(HashMap::new);
    map.insert(offset.name.clone(), value);
}

#[cfg(test)]
mod tests {
    use super::super::CustomFile;
    use super::*;
    use config::parse_custom_file;

    // ---- OffsetValue tests ----

    #[test]
    fn test_offset_value_new() {
        let ov = OffsetValue::new(1, 2, 3, 4, 5, 6);
        assert_eq!(ov.x, 1);
        assert_eq!(ov.y, 2);
        assert_eq!(ov.w, 3);
        assert_eq!(ov.h, 4);
        assert_eq!(ov.r, 5);
        assert_eq!(ov.a, 6);
    }

    #[test]
    fn test_offset_value_clone() {
        let ov = OffsetValue::new(10, 20, 30, 40, 50, 60);
        let cloned = ov;
        assert_eq!(cloned.x, 10);
        assert_eq!(cloned.y, 20);
        assert_eq!(cloned.w, 30);
        assert_eq!(cloned.h, 40);
        assert_eq!(cloned.r, 50);
        assert_eq!(cloned.a, 60);
    }

    #[test]
    fn test_offset_value_zero() {
        let ov = OffsetValue::new(0, 0, 0, 0, 0, 0);
        assert_eq!(ov.x, 0);
        assert_eq!(ov.y, 0);
        assert_eq!(ov.w, 0);
        assert_eq!(ov.h, 0);
        assert_eq!(ov.r, 0);
        assert_eq!(ov.a, 0);
    }

    #[test]
    fn test_offset_value_negative() {
        let ov = OffsetValue::new(-10, -20, -30, -40, -50, -60);
        assert_eq!(ov.x, -10);
        assert_eq!(ov.y, -20);
    }

    // ---- option_index tests ----

    #[test]
    fn test_option_index_found() {
        let option = CustomOption::new(
            "test".to_string(),
            vec![100, 200, 300],
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
        );
        assert_eq!(option_index(&option, 100), 0);
        assert_eq!(option_index(&option, 200), 1);
        assert_eq!(option_index(&option, 300), 2);
    }

    #[test]
    fn test_option_index_not_found_returns_random_value() {
        let option = CustomOption::new(
            "test".to_string(),
            vec![100, 200, 300],
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
        );
        assert_eq!(option_index(&option, 999), OPTION_RANDOM_VALUE);
    }

    #[test]
    fn test_option_index_empty_option() {
        let option = CustomOption::new("test".to_string(), vec![], vec![]);
        assert_eq!(option_index(&option, 0), OPTION_RANDOM_VALUE);
    }

    #[test]
    fn test_parse_custom_file_matches_wildcards_case_insensitively() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let lane_dir = tmp_dir.path().join("lane");
        std::fs::create_dir_all(&lane_dir).unwrap();
        std::fs::write(lane_dir.join("lane_default.png"), []).unwrap();
        std::fs::write(lane_dir.join("LANE_ALT.PNG"), []).unwrap();
        std::fs::write(lane_dir.join("notes.txt"), []).unwrap();

        let file = CustomFile::new(
            "Lane".to_string(),
            format!("{}/lane*.png", lane_dir.to_string_lossy()),
            None,
        );

        let choices = parse_custom_file(&file).expect("existing directory should be scanned");
        assert!(
            choices.iter().any(|choice| choice == "lane_default.png"),
            "wildcard matching should include lowercase files"
        );
        assert!(
            choices.iter().any(|choice| choice == "LANE_ALT.PNG"),
            "wildcard matching should include uppercase files"
        );
        assert!(
            choices.iter().any(|choice| choice == "Random"),
            "Random fallback should still be appended"
        );
        assert!(
            !choices.iter().any(|choice| choice == "notes.txt"),
            "non-matching files should not be included"
        );
    }
}

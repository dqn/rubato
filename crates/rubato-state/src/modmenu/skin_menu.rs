use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::imgui_renderer;
use super::stubs::{
    CustomCategory, CustomCategoryItem, CustomFile, CustomOffset, CustomOption, JSONSkinLoader,
    LR2SkinHeaderLoader, LuaSkinLoader, MainController, OPTION_RANDOM_VALUE, PlayerConfig,
    SkinConfig, SkinFilePath, SkinHeader, SkinOffset, SkinOption, SkinProperty, SkinType,
    TYPE_LR2SKIN, Validatable,
};
use rubato_skin::json::json_skin_loader::{CustomItemData, SkinHeaderData};
use rubato_skin::lr2::lr2_skin_header_loader::LR2SkinHeaderData;

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
        *MAIN.lock().expect("MAIN lock poisoned") = Some(main);
        *PLAYER_CONFIG.lock().expect("PLAYER_CONFIG lock poisoned") = Some(player_config);
    }

    pub fn invalidate() {
        *READY.lock().expect("READY lock poisoned") = false;
    }

    /// Render the skin configuration window using egui.
    ///
    /// Translated from: SkinMenu.show(ImBoolean)
    pub fn show_ui(ctx: &egui::Context) {
        let main = MAIN.lock().expect("MAIN lock poisoned");
        if main.is_none() {
            return;
        }
        drop(main);

        let ready = *READY.lock().expect("READY lock poisoned");
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
    let skins = SKINS.lock().expect("SKINS lock poisoned");
    let current_skin = CURRENT_SKIN.lock().expect("CURRENT_SKIN lock poisoned");

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
                let skins = SKINS.lock().expect("SKINS lock poisoned");
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
                    let skins = SKINS.lock().expect("SKINS lock poisoned");
                    for header in skins.iter() {
                        let name = header.name().map(|n| n.to_string()).unwrap_or_default();
                        if ui.selectable_label(name == selected_name, &name).clicked() {
                            selected_name = name;
                        }
                    }
                });
            // If a different skin was selected via combo, switch to it
            if selected_name != current_name {
                let skins = SKINS.lock().expect("SKINS lock poisoned");
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
                let skins = SKINS.lock().expect("SKINS lock poisoned");
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
            let is_dirty = *DIRTY_CONFIG.lock().expect("DIRTY_CONFIG lock poisoned");
            let live_editing = *LIVE_EDITING.lock().expect("LIVE_EDITING lock poisoned");
            let save_available = is_dirty && !live_editing;

            // Save button
            ui.add_enabled_ui(save_available, |ui| {
                let save_requested = ui.button(" Save ").clicked();
                if save_requested || (is_dirty && live_editing) {
                    let current_skin = CURRENT_SKIN.lock().expect("CURRENT_SKIN lock poisoned");
                    if let Some(ref cs) = *current_skin {
                        let h = cs.clone();
                        drop(current_skin);
                        switch_current_scene_skin(h);
                    }
                }
            });

            // Live Editing checkbox
            let mut le = *LIVE_EDITING.lock().expect("LIVE_EDITING lock poisoned");
            if ui.checkbox(&mut le, "Live Editing").changed() {
                dirty(true);
            }
            *LIVE_EDITING.lock().expect("LIVE_EDITING lock poisoned") = le;

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
                            let current_skin =
                                CURRENT_SKIN.lock().expect("CURRENT_SKIN lock poisoned");
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
            let mut ft = *FREEZE_TIMERS.lock().expect("FREEZE_TIMERS lock poisoned");
            if ui.checkbox(&mut ft, "Freeze timers").changed() {
                // main.getTimer().setFrozen(freezeTimers) — stub
                log::info!("Freeze timers: {}", ft);
            }
            *FREEZE_TIMERS.lock().expect("FREEZE_TIMERS lock poisoned") = ft;
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
    let current_skin = CURRENT_SKIN.lock().expect("CURRENT_SKIN lock poisoned");
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
                            if let Some(ref mut opts) =
                                *SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned")
                            {
                                opts.insert(option.name.clone(), option.option[i]);
                            }
                            dirty(true);
                            new_chosen = content.clone();
                        }
                    }
                    if ui.selectable_label("Random" == chosen, "Random").clicked() {
                        if let Some(ref mut opts) =
                            *SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned")
                        {
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
                    if let Some(ref mut opts) =
                        *SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned")
                    {
                        opts.insert(option.name.clone(), OPTION_RANDOM_VALUE);
                    }
                } else if (selected as usize) < option.option.len()
                    && let Some(ref mut opts) =
                        *SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned")
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
            if let Some(ref mut opts) = *SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned") {
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
    let available = AVAILABLE_FILES
        .lock()
        .expect("AVAILABLE_FILES lock poisoned");
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

    let selection = selection.unwrap_or_default();
    let mut index = choices.iter().position(|c| c == &selection).unwrap_or(0);
    let max = choices.len();

    ui.push_id(&file.name, |ui| {
        ui.horizontal(|ui| {
            // Tinted frame background for file selectors (Java: pushStyleColor FrameBg)
            // Left arrow
            if ui.button("\u{25C0}").clicked() {
                index = (index + max - 1) % max;
                if let Some(ref mut files) = *SET_FILES.lock().expect("SET_FILES lock poisoned") {
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
                            if let Some(ref mut files) =
                                *SET_FILES.lock().expect("SET_FILES lock poisoned")
                            {
                                files.insert(file.name.clone(), path.clone());
                            }
                            dirty(true);
                        }
                    }
                });

            // Right arrow
            if ui.button("\u{25B6}").clicked() {
                index = (index + 1) % max;
                if let Some(ref mut files) = *SET_FILES.lock().expect("SET_FILES lock poisoned") {
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
            if offset.x || offset.w || offset.a {
                ui.horizontal(|ui| {
                    spawn_drag_int(ui, "X", offset.x, &mut value.x);
                    spawn_drag_int(ui, "W", offset.w, &mut value.w);
                    spawn_drag_int(ui, "a", offset.a, &mut value.a);
                });
            }

            // Row 2: Y, H, R
            if offset.y || offset.h || offset.r {
                ui.horizontal(|ui| {
                    spawn_drag_int(ui, "Y", offset.y, &mut value.y);
                    spawn_drag_int(ui, "H", offset.h, &mut value.h);
                    spawn_drag_int(ui, "R", offset.r, &mut value.r);
                });
            }
        });
    });

    // Write back the modified offset values
    let mut offsets = SET_OFFSETS.lock().expect("SET_OFFSETS lock poisoned");
    let map = offsets.get_or_insert_with(HashMap::new);
    map.insert(offset.name.clone(), value);
}

fn refresh() {
    *SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned") = None;
    *AVAILABLE_FILES
        .lock()
        .expect("AVAILABLE_FILES lock poisoned") = None;
    *SET_FILES.lock().expect("SET_FILES lock poisoned") = None;
    *SET_OFFSETS.lock().expect("SET_OFFSETS lock poisoned") = None;

    // observedState = main.getCurrentState();
    // SkinHeader currentSceneSkin = observedState.getSkin().header;
    // currentSkinType = currentSceneSkin.getSkinType();
    // currentSkin = null;
    // switchCurrentSceneSkin(currentSceneSkin);
    // skins = loadAllSkins(currentSkinType);
    *READY.lock().expect("READY lock poisoned") = true;
}

#[allow(dead_code)]
fn load_all_skins(skin_type: &SkinType) -> Vec<SkinHeader> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let skins_dir = PathBuf::from("skin");
    scan_skins(&skins_dir, &mut paths);

    let mut skins: Vec<SkinHeader> = Vec::new();
    let current_skin = CURRENT_SKIN.lock().expect("CURRENT_SKIN lock poisoned");

    for path in &paths {
        let path_string = path.to_string_lossy().to_lowercase();
        let mut header: Option<SkinHeader> = None;

        if let Some(ref cs) = *current_skin
            && cs.path().is_some_and(|p| path == p)
        {
            header = Some(cs.clone());
        }

        if header.is_none() {
            if path_string.ends_with(".json") {
                let mut loader = JSONSkinLoader::new();
                header = loader.load_header(path).map(skin_header_from_json_data);
            } else if path_string.ends_with(".luaskin") {
                let mut loader = LuaSkinLoader::new();
                let _ = loader.load_header(path);
                // header stays None -- lua skin loader not yet fully implemented
            } else if path_string.ends_with(".lr2skin") {
                let main = MAIN.lock().expect("MAIN lock poisoned");
                if main.is_some() {
                    drop(main);
                    let mut loader = LR2SkinHeaderLoader::new("");
                    match loader.load_skin(path, None).map(skin_header_from_lr2_data) {
                        Ok(mut h) => {
                            // 7/14key skin can also be used for 5/10key
                            if *skin_type == SkinType::Play5Keys
                                && h.skin_type().is_some_and(|st| *st == SkinType::Play7Keys)
                                && h.toast_type() == TYPE_LR2SKIN
                                && let Ok(mut h2) =
                                    loader.load_skin(path, None).map(skin_header_from_lr2_data)
                            {
                                h2.set_skin_type(SkinType::Play5Keys);
                                if !h2.name().unwrap_or("").to_lowercase().contains("7key") {
                                    let new_name = format!("{} (7KEYS) ", h2.name().unwrap_or(""));
                                    h2.set_name(new_name);
                                }
                                h = h2;
                            }
                            if *skin_type == SkinType::Play10Keys
                                && h.skin_type().is_some_and(|st| *st == SkinType::Play14Keys)
                                && h.toast_type() == TYPE_LR2SKIN
                                && let Ok(mut h2) =
                                    loader.load_skin(path, None).map(skin_header_from_lr2_data)
                            {
                                h2.set_skin_type(SkinType::Play10Keys);
                                if !h2.name().unwrap_or("").to_lowercase().contains("14key") {
                                    let new_name = format!("{} (14KEYS) ", h2.name().unwrap_or(""));
                                    h2.set_name(new_name);
                                }
                                h = h2;
                            }
                            header = Some(h);
                        }
                        Err(_e) => {
                            // e.printStackTrace()
                        }
                    }
                }
            }
        }

        if let Some(h) = header
            && h.skin_type().is_some_and(|st| st == skin_type)
        {
            skins.push(h);
        }
    }

    skins
}

#[allow(dead_code)]
fn scan_skins(path: &Path, paths: &mut Vec<PathBuf>) {
    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                scan_skins(&entry.path(), paths);
            }
        }
    } else {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if name.ends_with(".lr2skin") || name.ends_with(".luaskin") || name.ends_with(".json") {
            paths.push(path.to_path_buf());
        }
    }
}

fn matches_skin_file_pattern_case_insensitive(filename: &str, pattern: &str) -> bool {
    let normalized_filename = filename.to_ascii_lowercase();
    let normalized_pattern = pattern.to_ascii_lowercase();

    if !normalized_pattern.contains('*') {
        return normalized_filename == normalized_pattern;
    }

    let parts: Vec<&str> = normalized_pattern
        .split('*')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return true;
    }

    let mut search_start = 0usize;
    for (index, part) in parts.iter().enumerate() {
        if index == 0 && !normalized_pattern.starts_with('*') {
            if !normalized_filename[search_start..].starts_with(part) {
                return false;
            }
            search_start += part.len();
            continue;
        }

        let Some(relative_pos) = normalized_filename[search_start..].find(part) else {
            return false;
        };
        search_start += relative_pos + part.len();
    }

    if !normalized_pattern.ends_with('*')
        && let Some(last_part) = parts.last()
    {
        return normalized_filename.ends_with(last_part);
    }

    true
}

fn parse_custom_file(file: &CustomFile) -> Option<Vec<String>> {
    let mut file_selection: Vec<String> = Vec::new();

    let last_slash = file.path.rfind('/');
    let last_slash_idx = last_slash.unwrap_or(0);
    let name = if last_slash.is_some() {
        &file.path[last_slash_idx + 1..]
    } else {
        &file.path
    };

    let name = if file.path.contains('|') {
        let pipe_idx = file.path.rfind('|').expect("contains '|'");
        if file.path.len() > pipe_idx + 1 {
            let first_pipe = file.path.find('|').expect("contains '|'");
            let start = if last_slash.is_some() {
                last_slash_idx + 1
            } else {
                0
            };
            format!(
                "{}{}",
                &file.path[start..first_pipe],
                &file.path[pipe_idx + 1..]
            )
        } else {
            let first_pipe = file.path.find('|').expect("contains '|'");
            let start = if last_slash.is_some() {
                last_slash_idx + 1
            } else {
                0
            };
            file.path[start..first_pipe].to_string()
        }
    } else {
        name.to_string()
    };

    let dirpath = if last_slash.is_some() {
        PathBuf::from(&file.path[..last_slash_idx])
    } else {
        PathBuf::from(&file.path)
    };

    if !dirpath.exists() {
        return None;
    }

    // In Java: Files.newDirectoryStream(dirpath, "{name.lower(),name.upper()}")
    if let Ok(entries) = fs::read_dir(&dirpath) {
        for entry in entries.flatten() {
            let entry_name = entry.file_name().to_string_lossy().to_string();
            if matches_skin_file_pattern_case_insensitive(&entry_name, &name) {
                file_selection.push(entry_name);
            }
        }
    }
    file_selection.push("Random".to_string());

    Some(file_selection)
}

fn load_saved_skin_settings(header: &SkinHeader) {
    let skin_path = header
        .path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let player_config = PLAYER_CONFIG.lock().expect("PLAYER_CONFIG lock poisoned");

    if player_config.is_none() {
        return;
    }
    let pc = player_config.as_ref().expect("player_config is Some");

    let mut saved_properties: Option<&SkinProperty> = None;

    let skin_type_id = header.skin_type().map(|st| st.id()).unwrap_or(0) as usize;
    if skin_type_id < pc.skin.len()
        && let Some(ref live_config) = pc.skin[skin_type_id]
        && live_config.path().is_some_and(|p| p == skin_path)
    {
        saved_properties = live_config.properties();
    }

    if saved_properties.is_none() {
        for saved_config in &pc.skin_history {
            if saved_config.path().is_some_and(|p| p == skin_path) {
                saved_properties = saved_config.properties();
                break;
            }
        }
    }

    if let Some(props) = saved_properties {
        let mut options = SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned");
        let opt_map = options.get_or_insert_with(HashMap::new);
        for option in props.option.iter().flatten() {
            if let Some(ref name) = option.name {
                opt_map.insert(name.clone(), option.value);
            }
        }

        let mut files = SET_FILES.lock().expect("SET_FILES lock poisoned");
        let file_map = files.get_or_insert_with(HashMap::new);
        for file in props.file.iter().flatten() {
            if let (Some(name), Some(path)) = (&file.name, &file.path) {
                file_map.insert(name.clone(), path.clone());
            }
        }

        let mut offsets = SET_OFFSETS.lock().expect("SET_OFFSETS lock poisoned");
        let offset_map = offsets.get_or_insert_with(HashMap::new);
        for offset in props.offset.iter().flatten() {
            if let Some(ref name) = offset.name {
                offset_map.insert(
                    name.clone(),
                    OffsetValue::new(offset.x, offset.y, offset.w, offset.h, offset.r, offset.a),
                );
            }
        }
    }
}

fn get_option_setting(option: &CustomOption) -> i32 {
    let options = SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned");
    if let Some(ref map) = *options
        && let Some(&value) = map.get(&option.name)
    {
        return value;
    }
    option.default_option()
}

fn get_file_setting(file: &CustomFile) -> Option<String> {
    let files = SET_FILES.lock().expect("SET_FILES lock poisoned");
    if let Some(ref map) = *files
        && let Some(path) = map.get(&file.name)
    {
        return Some(path.clone());
    }
    file.def.clone()
}

fn get_offset_setting(offset: &CustomOffset) -> OffsetValue {
    let mut offsets = SET_OFFSETS.lock().expect("SET_OFFSETS lock poisoned");
    let map = offsets.get_or_insert_with(HashMap::new);
    *map.entry(offset.name.clone())
        .or_insert_with(|| OffsetValue::new(0, 0, 0, 0, 0, 0))
}

fn complete_property(header: &SkinHeader) -> SkinProperty {
    // default out all unset properties and collect everything into the property object
    let mut options: Vec<Option<SkinOption>> = Vec::new();
    let mut files: Vec<Option<SkinFilePath>> = Vec::new();
    let mut offsets: Vec<Option<SkinOffset>> = Vec::new();

    for option in header.custom_options() {
        let value = get_option_setting(option);
        let mut opt_map = SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned");
        let map = opt_map.get_or_insert_with(HashMap::new);
        map.insert(option.name.clone(), value);
        options.push(Some(SkinOption {
            name: Some(option.name.clone()),
            value,
        }));
    }

    for file in header.custom_files() {
        let file_selection = parse_custom_file(file).unwrap_or_else(|| vec!["Random".to_string()]);

        {
            let mut available = AVAILABLE_FILES
                .lock()
                .expect("AVAILABLE_FILES lock poisoned");
            let map = available.get_or_insert_with(HashMap::new);
            map.insert(file.name.clone(), file_selection.clone());
        }

        let mut selection = {
            let files_map = SET_FILES.lock().expect("SET_FILES lock poisoned");
            files_map.as_ref().and_then(|m| m.get(&file.name).cloned())
        };

        if selection.is_none()
            && let Some(ref def) = file.def
        {
            for filename in &file_selection {
                if filename.eq_ignore_ascii_case(def) {
                    selection = Some(filename.clone());
                    break;
                }
                if let Some(point) = filename.rfind('.')
                    && filename[..point].eq_ignore_ascii_case(def)
                {
                    selection = Some(filename.clone());
                    break;
                }
            }
        }

        // fileSelection[0] always present due to inserted 'Random'
        if selection.is_none() {
            selection = file_selection.first().cloned();
        }

        let sel = selection.unwrap_or_default();
        {
            let mut files_map = SET_FILES.lock().expect("SET_FILES lock poisoned");
            let map = files_map.get_or_insert_with(HashMap::new);
            map.insert(file.name.clone(), sel.clone());
        }

        files.push(Some(SkinFilePath {
            name: Some(file.name.clone()),
            path: Some(sel),
        }));
    }

    for offset in header.custom_offsets() {
        let value = get_offset_setting(offset);
        offsets.push(Some(SkinOffset {
            name: Some(offset.name.clone()),
            x: value.x,
            y: value.y,
            w: value.w,
            h: value.h,
            r: value.r,
            a: value.a,
        }));
    }

    SkinProperty {
        option: options,
        file: files,
        offset: offsets,
    }
}

fn dirty(flag: bool) {
    if flag {
        *DIRTY_CONFIG.lock().expect("DIRTY_CONFIG lock poisoned") = true;
    }
}

fn save_current_config(next_skin: &SkinHeader) {
    *DIRTY_CONFIG.lock().expect("DIRTY_CONFIG lock poisoned") = false;

    let current_skin = CURRENT_SKIN.lock().expect("CURRENT_SKIN lock poisoned");
    if current_skin.is_none() {
        return;
    }
    let cs = current_skin.as_ref().expect("current_skin is Some");

    let skin_path = cs
        .path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let property = complete_property(cs);
    let config = SkinConfig {
        path: Some(skin_path.clone()),
        properties: Some(property),
    };

    let mut player_config = PLAYER_CONFIG.lock().expect("PLAYER_CONFIG lock poisoned");
    if player_config.is_none() {
        return;
    }
    let pc = player_config.as_mut().expect("player_config is Some");

    let current_type = CURRENT_SKIN_TYPE
        .lock()
        .expect("CURRENT_SKIN_TYPE lock poisoned");
    if let Some(ref st) = *current_type
        && next_skin.name() == cs.name()
    {
        let id = st.id() as usize;
        if id < pc.skin.len() {
            pc.skin[id] = Some(config);
        }
        return;
    }

    if let Some(entry) = pc
        .skin_history
        .iter_mut()
        .find(|h| h.path().is_some_and(|p| p == skin_path))
    {
        *entry = config;
        return;
    }

    // this skin hasn't been in the config history before, add it
    pc.skin_history.push(config);
}

fn reset_current_skin_config() {
    *SET_OPTIONS.lock().expect("SET_OPTIONS lock poisoned") = Some(HashMap::new());
    *AVAILABLE_FILES
        .lock()
        .expect("AVAILABLE_FILES lock poisoned") = Some(HashMap::new());
    *SET_FILES.lock().expect("SET_FILES lock poisoned") = Some(HashMap::new());
    *SET_OFFSETS.lock().expect("SET_OFFSETS lock poisoned") = Some(HashMap::new());
}

fn switch_current_scene_skin(header: SkinHeader) {
    {
        let current = CURRENT_SKIN.lock().expect("CURRENT_SKIN lock poisoned");
        if current.is_some() {
            drop(current);
            save_current_config(&header);
        }
    }

    reset_current_skin_config();
    load_saved_skin_settings(&header);

    *CURRENT_SKIN.lock().expect("CURRENT_SKIN lock poisoned") = Some(header.clone());
    let _property = complete_property(&header);

    let skin_path = header
        .path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut config = SkinConfig {
        path: Some(skin_path),
        properties: Some(_property),
    };
    config.validate();

    // MainState scene = main.getCurrentState();
    // Skin skin = SkinLoader.load(scene, currentSkinType, config);
    // if (skin == null) { ... fallback ... }
    // playerConfig.getSkin()[currentSkinType.getId()] = config;
    // scene.setSkin(skin);
    // skin.prepare(scene);
    // if (scene instanceof MusicSelector) { ((MusicSelector)scene).getBarRender().updateBarText(); }
}

// =========================================================================
// Conversion helpers: SkinHeaderData / LR2SkinHeaderData -> SkinHeader
// =========================================================================

#[allow(dead_code)]
fn skin_header_from_json_data(data: SkinHeaderData) -> SkinHeader {
    let mut header = SkinHeader::new();
    header.skin_type_id = data.header_type;
    header.set_path(data.path);
    header.set_name(data.name);
    if let Some(st) = SkinType::skin_type_by_id(data.skin_type) {
        header.set_skin_type(st);
    }
    let options: Vec<CustomOption> = data
        .custom_options
        .into_iter()
        .map(|co| CustomOption::new(co.name, co.option, co.names))
        .collect();
    header.options = options;
    let files: Vec<CustomFile> = data
        .custom_files
        .into_iter()
        .map(|cf| CustomFile::new(cf.name, cf.path, cf.def))
        .collect();
    header.files = files;
    let offsets: Vec<CustomOffset> = data
        .custom_offsets
        .into_iter()
        .map(|co| CustomOffset::new(co.name, co.id, co.x, co.y, co.w, co.h, co.r, co.a))
        .collect();
    header.offsets = offsets;
    let categories: Vec<CustomCategory> = data
        .custom_categories
        .into_iter()
        .map(|cc| {
            let items: Vec<CustomCategoryItem> = cc
                .items
                .into_iter()
                .map(|item| match item {
                    CustomItemData::Option(co) => {
                        CustomCategoryItem::Option(CustomOption::new(co.name, co.option, co.names))
                    }
                    CustomItemData::File(cf) => {
                        CustomCategoryItem::File(CustomFile::new(cf.name, cf.path, cf.def))
                    }
                    CustomItemData::Offset(co) => CustomCategoryItem::Offset(CustomOffset::new(
                        co.name, co.id, co.x, co.y, co.w, co.h, co.r, co.a,
                    )),
                })
                .collect();
            CustomCategory::new(cc.name, items)
        })
        .collect();
    header.categories = categories;
    if let Some(res) = data.source_resolution {
        header.set_source_resolution(res);
    }
    if let Some(res) = data.destination_resolution {
        header.set_destination_resolution(res);
    }
    header
}

#[allow(dead_code)]
fn skin_header_from_lr2_data(data: LR2SkinHeaderData) -> SkinHeader {
    let mut header = SkinHeader::new();
    if let Some(path) = data.path {
        header.set_path(path);
    }
    if let Some(st) = data.skin_type {
        header.set_skin_type(st);
    }
    header.set_name(data.name);
    // Convert LR2-specific types to skin_header types
    let options: Vec<CustomOption> = data
        .custom_options
        .into_iter()
        .map(|co| CustomOption::new(co.name, co.option, co.contents))
        .collect();
    header.options = options;
    let files: Vec<CustomFile> = data
        .custom_files
        .into_iter()
        .map(|cf| CustomFile::new(cf.name, cf.path, cf.def))
        .collect();
    header.files = files;
    let offsets: Vec<CustomOffset> = data
        .custom_offsets
        .into_iter()
        .map(|co| CustomOffset::new(co.name, co.id, co.x, co.y, co.w, co.h, co.r, co.a))
        .collect();
    header.offsets = offsets;
    header
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let cloned = ov.clone();
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

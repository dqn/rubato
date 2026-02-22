use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::imgui_notify::ImGuiNotify;
use crate::stubs::{
    Config, CustomCategoryItem, CustomFile, CustomOffset, CustomOption, ImBoolean, ImInt,
    JSONSkinLoader, LR2SkinHeaderLoader, LuaSkinLoader, MainController, MainState, MusicSelector,
    OPTION_RANDOM_VALUE, PlayerConfig, Skin, SkinConfig, SkinConfigDefault, SkinConfigFilePath,
    SkinConfigOffset, SkinConfigOption, SkinConfigProperty, SkinHeader, SkinLoader, SkinType,
    TYPE_LR2SKIN,
};

static MAIN: Mutex<Option<MainController>> = Mutex::new(None);
static PLAYER_CONFIG: Mutex<Option<PlayerConfig>> = Mutex::new(None);

static READY: Mutex<bool> = Mutex::new(false);
static LIVE_EDITING: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: true });
static FREEZE_TIMERS: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });

static CURRENT_SKIN_TYPE: Mutex<Option<SkinType>> = Mutex::new(None);
static CURRENT_SKIN: Mutex<Option<SkinHeader>> = Mutex::new(None);
static SET_OPTIONS: Mutex<Option<HashMap<String, i32>>> = Mutex::new(None);
static AVAILABLE_FILES: Mutex<Option<HashMap<String, Vec<String>>>> = Mutex::new(None);
static SET_FILES: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
static SET_OFFSETS: Mutex<Option<HashMap<String, OffsetValue>>> = Mutex::new(None);
static SKINS: Mutex<Vec<SkinHeader>> = Mutex::new(Vec::new());
static DIRTY_CONFIG: Mutex<bool> = Mutex::new(false);

#[derive(Clone, Debug)]
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
        *MAIN.lock().unwrap() = Some(main);
        *PLAYER_CONFIG.lock().unwrap() = Some(player_config);
    }

    pub fn show(_show_skin_menu: &mut ImBoolean) {
        let main = MAIN.lock().unwrap();
        if main.is_none() {
            return;
        }
        drop(main);

        // if (observedState != main.getCurrentState()) { invalidate(); }
        let ready = *READY.lock().unwrap();
        if !ready {
            refresh();
        }

        // int windowHeight = Gdx.graphics.getHeight();
        // ImGui.setNextWindowSize(0.f, windowHeight * 0.3f, ImGuiCond.FirstUseEver);

        // if (ImGui.begin("Skin", showSkinMenu))
        {
            menu_header();
            // ImGui.separator();
            let current_skin = CURRENT_SKIN.lock().unwrap();
            if let Some(ref skin) = *current_skin {
                // ImGui.pushID(skin.get_name());
                let _ = skin;
                drop(current_skin);
                skin_config_menu();
                // ImGui.popID();
            }
        }
        // ImGui.end();
        log::warn!("not yet implemented: SkinMenu::show - egui integration");
    }

    pub fn invalidate() {
        *READY.lock().unwrap() = false;
    }

    /// Render the skin configuration window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        let main = MAIN.lock().unwrap();
        if main.is_none() {
            return;
        }
        drop(main);

        let ready = *READY.lock().unwrap();
        if !ready {
            refresh();
        }

        let mut open = true;
        egui::Window::new("Skin").open(&mut open).show(ctx, |ui| {
            // Skin selector
            let skins = SKINS.lock().unwrap();
            let current = CURRENT_SKIN.lock().unwrap();
            let current_name = current
                .as_ref()
                .map(|s| s.get_name().to_string())
                .unwrap_or_else(|| "(none)".to_string());
            drop(current);

            ui.horizontal(|ui| {
                ui.label("Current skin:");
                ui.label(&current_name);
            });

            ui.label(format!("{} skins available", skins.len()));
            drop(skins);

            ui.separator();

            // Skin config options
            let current_skin = CURRENT_SKIN.lock().unwrap();
            if let Some(ref skin) = *current_skin {
                for option in skin.get_custom_options() {
                    let mut val = SET_OPTIONS
                        .lock()
                        .unwrap()
                        .as_ref()
                        .and_then(|m| m.get(&option.name).copied())
                        .unwrap_or(option.get_default_option());
                    let selected = option
                        .contents
                        .get(val as usize)
                        .map(|s| s.as_str())
                        .unwrap_or("Default");
                    egui::ComboBox::from_label(&option.name)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (i, content) in option.contents.iter().enumerate() {
                                ui.selectable_value(&mut val, i as i32, content.as_str());
                            }
                        });
                    if let Some(ref mut opts) = *SET_OPTIONS.lock().unwrap() {
                        opts.insert(option.name.clone(), val);
                    }
                }
            }
        });
    }
}

fn menu_header() {
    let skins = SKINS.lock().unwrap();
    let current_skin = CURRENT_SKIN.lock().unwrap();

    if let Some(ref skin) = *current_skin {
        // Arrow left button
        // if (ImGui.arrowButton("##skin-select-left", 0))
        {
            let index = skins.iter().position(|s| s.get_name() == skin.get_name());
            if let Some(idx) = index {
                let _new_idx = (idx + skins.len() - 1) % skins.len();
                // switchCurrentSceneSkin(skins.get(new_idx));
            }
        }

        // Combo selector
        // if (ImGui.beginCombo("##skin-select-combo", currentSkin.getName(), ImGuiComboFlags.HeightLarge))
        // { for header in skins { ... ImGui.selectable(header.getName()) ... } ImGui.endCombo(); }

        // Arrow right button
        // if (ImGui.arrowButton("##skin-select-right", 1))
        {
            let index = skins.iter().position(|s| s.get_name() == skin.get_name());
            if let Some(idx) = index {
                let _new_idx = (idx + 1) % skins.len();
                // switchCurrentSceneSkin(skins.get(new_idx));
            }
        }

        // Open skin location button
        // if (ImGui.button("Open##open-skin-location")) { Desktop.open(...) }

        let _path_display = format!("> {}", skin.get_path().display());
        // ImGui.textDisabled(path_display);

        let dirty = *DIRTY_CONFIG.lock().unwrap();
        let live_editing = LIVE_EDITING.lock().unwrap().get();
        let _save_available = dirty && !live_editing;
        // ImGui.beginDisabled(!saveAvailable);
        // boolean saveRequested = ImGui.button(" Save ##reload-current-skin");
        // ImGui.endDisabled();
        // if (saveRequested || (dirtyConfig && liveEditing.get())) { switchCurrentSceneSkin(currentSkin); }
        // dirty(ImGui.checkbox("Live Editing###live-edit-mode", liveEditing));

        // Reset button
        // if (ImGui.button(" Reset ##skin-setting-reset-request")) { ImGui.openPopup("skin-setting-reset-confirmation"); }
        // ... confirmation popup ...

        // Freeze timers checkbox
        // if (ImGui.checkbox("Freeze timers###freeze-mode", freezeTimers))
        // { main.getTimer().setFrozen(freezeTimers.get()); }
    }
}

fn skin_config_menu() {
    let current_skin = CURRENT_SKIN.lock().unwrap();
    if current_skin.is_none() {
        return;
    }
    let skin = current_skin.as_ref().unwrap();
    let mut shown = HashSet::new();

    let categories = skin.get_custom_categories();
    let tabbar = !categories.is_empty();

    // if (tabbar) { ImGui.beginTabBar("#tab-bar"); }
    for category in categories {
        // boolean tabOpen = ImGui.beginTabItem(category.name + "##category-tab");
        // if (tabOpen) { ImGui.beginChild("skin-config", 0, 0, true); }
        for item in &category.items {
            match item {
                CustomCategoryItem::Option(option) => {
                    shown.insert(option.name.clone());
                    skin_config_option(option);
                }
                CustomCategoryItem::File(file) => {
                    shown.insert(file.name.clone());
                    skin_config_file(file);
                }
                CustomCategoryItem::Offset(offset) => {
                    shown.insert(offset.name.clone());
                    skin_config_offset(offset);
                }
            }
        }
        // if (tabOpen) { ImGui.endChild(); ImGui.endTabItem(); }
    }

    // Other tab (uncategorized items)
    // boolean otherTab = tabbar && ImGui.beginTabItem("Other##category-tab");
    // if (!tabbar || otherTab) {
    let options = skin.get_custom_options();
    for option in options {
        if shown.contains(&option.name) {
            continue;
        }
        skin_config_option(option);
    }
    let files = skin.get_custom_files();
    for file in files {
        if shown.contains(&file.name) {
            continue;
        }
        skin_config_file(file);
    }
    let offsets = skin.get_custom_offsets();
    for offset in offsets {
        if shown.contains(&offset.name) {
            continue;
        }
        skin_config_offset(offset);
    }
    // }
    // if (otherTab) { ImGui.endTabItem(); }
    // if (tabbar) { ImGui.endTabBar(); }
}

fn skin_config_option(option: &CustomOption) {
    // we pretend 'Random' is at the end of the list
    let options_count = option.contents.len() + 1;
    // for options with 2 choices (3 with random), show a radio instead
    if 3 == options_count {
        skin_config_option_radio(option);
        return;
    }

    // ImGui.pushID(option.name);
    let value = get_option_setting(option);
    let selected = option_index(option, value);
    let _chosen = if selected == OPTION_RANDOM_VALUE {
        "Random".to_string()
    } else if (selected as usize) < option.contents.len() {
        option.contents[selected as usize].clone()
    } else {
        "Random".to_string()
    };

    // Arrow buttons + combo for selection
    // ... ImGui calls stubbed ...

    // ImGui.popID();
}

fn skin_config_option_radio(option: &CustomOption) {
    // ImGui.pushID(option.name);
    // ImGui.text(option.name);
    let _value = get_option_setting(option);
    // Radio buttons for each option + "Random"
    // ... ImGui calls stubbed ...
    // ImGui.popID();
}

fn option_index(option: &CustomOption, value: i32) -> i32 {
    for i in 0..option.option.len() {
        if option.option[i] == value {
            return i as i32;
        }
    }
    OPTION_RANDOM_VALUE
}

fn skin_config_file(file: &CustomFile) {
    let selection = get_file_setting(file);
    let available = AVAILABLE_FILES.lock().unwrap();
    if selection.is_none() || available.as_ref().and_then(|m| m.get(&file.name)).is_none() {
        return;
    }
    drop(available);

    // ImGui color push, arrow buttons, combo, text display
    // ... ImGui calls stubbed ...
}

fn spawn_drag_int(_name: &str, offset: bool, _value: &mut i32) {
    if offset {
        // ImGui.dragInt(...)
        // dirty(ImGui.isItemDeactivatedAfterEdit());
    } else {
        // ImGui.dummy(100, 0);
    }
}

fn skin_config_offset(offset: &CustomOffset) {
    let _value = get_offset_setting(offset);
    let _width = 100;

    // ImGui.text(offset.name);
    // ImGui.pushID(offset.name);
    // ImGui.pushItemWidth(width);
    // ImGui.indent();

    // if (offset.x || offset.w || offset.a) { spawnDragInt("X"...) ... }
    // if (offset.y || offset.h || offset.r) { spawnDragInt("Y"...) ... }

    // ImGui.unindent();
    // ImGui.popItemWidth();
    // ImGui.popID();
}

fn refresh() {
    *SET_OPTIONS.lock().unwrap() = None;
    *AVAILABLE_FILES.lock().unwrap() = None;
    *SET_FILES.lock().unwrap() = None;
    *SET_OFFSETS.lock().unwrap() = None;

    // observedState = main.getCurrentState();
    // SkinHeader currentSceneSkin = observedState.getSkin().header;
    // currentSkinType = currentSceneSkin.getSkinType();
    // currentSkin = null;
    // switchCurrentSceneSkin(currentSceneSkin);
    // skins = loadAllSkins(currentSkinType);
    *READY.lock().unwrap() = true;
}

fn load_all_skins(skin_type: &SkinType) -> Vec<SkinHeader> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let skins_dir = PathBuf::from("skin");
    scan_skins(&skins_dir, &mut paths);

    let mut skins: Vec<SkinHeader> = Vec::new();
    let current_skin = CURRENT_SKIN.lock().unwrap();

    for path in &paths {
        let path_string = path.to_string_lossy().to_lowercase();
        let mut header: Option<SkinHeader> = None;

        if let Some(ref cs) = *current_skin
            && path == cs.get_path()
        {
            header = Some(cs.clone());
        }

        if header.is_none() {
            if path_string.ends_with(".json") {
                let loader = JSONSkinLoader::new();
                header = loader.load_header(path);
            } else if path_string.ends_with(".luaskin") {
                let loader = LuaSkinLoader::new();
                header = loader.load_header(path);
            } else if path_string.ends_with(".lr2skin") {
                let main = MAIN.lock().unwrap();
                if let Some(ref m) = *main {
                    let config = m.get_config();
                    let loader = LR2SkinHeaderLoader::new(&config);
                    match loader.load_skin(path, None) {
                        Ok(mut h) => {
                            // 7/14key skin can also be used for 5/10key
                            if *skin_type == SkinType::Play5Keys
                                && *h.get_skin_type() == SkinType::Play7Keys
                                && h.get_type() == TYPE_LR2SKIN
                                && let Ok(mut h2) = loader.load_skin(path, None)
                            {
                                h2.set_skin_type(SkinType::Play5Keys);
                                if !h2.get_name().to_lowercase().contains("7key") {
                                    let new_name = format!("{} (7KEYS) ", h2.get_name());
                                    h2.set_name(new_name);
                                }
                                h = h2;
                            }
                            if *skin_type == SkinType::Play10Keys
                                && *h.get_skin_type() == SkinType::Play14Keys
                                && h.get_type() == TYPE_LR2SKIN
                                && let Ok(mut h2) = loader.load_skin(path, None)
                            {
                                h2.set_skin_type(SkinType::Play10Keys);
                                if !h2.get_name().to_lowercase().contains("14key") {
                                    let new_name = format!("{} (14KEYS) ", h2.get_name());
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
            && h.get_skin_type() == skin_type
        {
            skins.push(h);
        }
    }

    skins
}

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
        let pipe_idx = file.path.rfind('|').unwrap();
        if file.path.len() > pipe_idx + 1 {
            let first_pipe = file.path.find('|').unwrap();
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
            let first_pipe = file.path.find('|').unwrap();
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
            if entry_name.to_lowercase() == name.to_lowercase()
                || entry_name.to_uppercase() == name.to_uppercase()
            {
                file_selection.push(entry_name);
            }
        }
    }
    file_selection.push("Random".to_string());

    Some(file_selection)
}

fn load_saved_skin_settings(header: &SkinHeader) {
    let skin_path = header.get_path().to_string_lossy().to_string();
    let player_config = PLAYER_CONFIG.lock().unwrap();

    if player_config.is_none() {
        return;
    }
    let pc = player_config.as_ref().unwrap();

    let mut saved_properties: Option<&SkinConfigProperty> = None;

    let skin_type_id = header.get_skin_type().get_id() as usize;
    if skin_type_id < pc.get_skin().len() {
        let live_config = &pc.get_skin()[skin_type_id];
        if skin_path == live_config.get_path() {
            saved_properties = Some(live_config.get_properties());
        }
    }

    if saved_properties.is_none() {
        for saved_config in pc.get_skin_history() {
            if saved_config.get_path() == skin_path {
                saved_properties = Some(saved_config.get_properties());
                break;
            }
        }
    }

    if let Some(props) = saved_properties {
        let mut options = SET_OPTIONS.lock().unwrap();
        let opt_map = options.get_or_insert_with(HashMap::new);
        for option in props.get_option() {
            opt_map.insert(option.name.clone(), option.value);
        }

        let mut files = SET_FILES.lock().unwrap();
        let file_map = files.get_or_insert_with(HashMap::new);
        for file in props.get_file() {
            file_map.insert(file.name.clone(), file.path.clone());
        }

        let mut offsets = SET_OFFSETS.lock().unwrap();
        let offset_map = offsets.get_or_insert_with(HashMap::new);
        for offset in props.get_offset() {
            offset_map.insert(
                offset.name.clone(),
                OffsetValue::new(offset.x, offset.y, offset.w, offset.h, offset.r, offset.a),
            );
        }
    }
}

fn get_option_setting(option: &CustomOption) -> i32 {
    let options = SET_OPTIONS.lock().unwrap();
    if let Some(ref map) = *options
        && let Some(&value) = map.get(&option.name)
    {
        return value;
    }
    option.get_default_option()
}

fn get_file_setting(file: &CustomFile) -> Option<String> {
    let files = SET_FILES.lock().unwrap();
    if let Some(ref map) = *files
        && let Some(path) = map.get(&file.name)
    {
        return Some(path.clone());
    }
    file.def.clone()
}

fn get_offset_setting(offset: &CustomOffset) -> OffsetValue {
    let mut offsets = SET_OFFSETS.lock().unwrap();
    let map = offsets.get_or_insert_with(HashMap::new);
    map.entry(offset.name.clone())
        .or_insert_with(|| OffsetValue::new(0, 0, 0, 0, 0, 0))
        .clone()
}

fn complete_property(header: &SkinHeader) -> SkinConfigProperty {
    // default out all unset properties and collect everything into the property object
    let mut options: Vec<SkinConfigOption> = Vec::new();
    let mut files: Vec<SkinConfigFilePath> = Vec::new();
    let mut offsets: Vec<SkinConfigOffset> = Vec::new();

    for option in header.get_custom_options() {
        let value = get_option_setting(option);
        let mut opt_map = SET_OPTIONS.lock().unwrap();
        let map = opt_map.get_or_insert_with(HashMap::new);
        map.insert(option.name.clone(), value);
        options.push(SkinConfigOption {
            name: option.name.clone(),
            value,
        });
    }

    for file in header.get_custom_files() {
        let file_selection = parse_custom_file(file).unwrap_or_else(|| vec!["Random".to_string()]);

        {
            let mut available = AVAILABLE_FILES.lock().unwrap();
            let map = available.get_or_insert_with(HashMap::new);
            map.insert(file.name.clone(), file_selection.clone());
        }

        let mut selection = {
            let files_map = SET_FILES.lock().unwrap();
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
            let mut files_map = SET_FILES.lock().unwrap();
            let map = files_map.get_or_insert_with(HashMap::new);
            map.insert(file.name.clone(), sel.clone());
        }

        files.push(SkinConfigFilePath {
            name: file.name.clone(),
            path: sel,
        });
    }

    for offset in header.get_custom_offsets() {
        let value = get_offset_setting(offset);
        offsets.push(SkinConfigOffset {
            name: offset.name.clone(),
            x: value.x,
            y: value.y,
            w: value.w,
            h: value.h,
            r: value.r,
            a: value.a,
        });
    }

    SkinConfigProperty {
        option: options,
        file: files,
        offset: offsets,
    }
}

fn dirty(flag: bool) {
    if flag {
        *DIRTY_CONFIG.lock().unwrap() = true;
    }
}

fn save_current_config(next_skin: &SkinHeader) {
    *DIRTY_CONFIG.lock().unwrap() = false;

    let current_skin = CURRENT_SKIN.lock().unwrap();
    if current_skin.is_none() {
        return;
    }
    let cs = current_skin.as_ref().unwrap();

    let skin_path = cs.get_path().to_string_lossy().to_string();
    let property = complete_property(cs);
    let config = SkinConfig {
        path: skin_path.clone(),
        properties: property,
    };

    let mut player_config = PLAYER_CONFIG.lock().unwrap();
    if player_config.is_none() {
        return;
    }
    let pc = player_config.as_mut().unwrap();

    let current_type = CURRENT_SKIN_TYPE.lock().unwrap();
    if let Some(ref st) = *current_type
        && next_skin.get_name() == cs.get_name()
    {
        let id = st.get_id() as usize;
        if id < pc.get_skin().len() {
            pc.get_skin_mut()[id] = config;
        }
        return;
    }

    for i in 0..pc.get_skin_history().len() {
        if pc.get_skin_history()[i].get_path() == skin_path {
            let mut history = pc.get_skin_history().clone();
            history[i] = config;
            pc.set_skin_history(history);
            return;
        }
    }

    // this skin hasn't been in the config history before, add it
    let mut history = pc.get_skin_history().clone();
    history.push(config);
    pc.set_skin_history(history);
}

fn reset_current_skin_config() {
    *SET_OPTIONS.lock().unwrap() = Some(HashMap::new());
    *AVAILABLE_FILES.lock().unwrap() = Some(HashMap::new());
    *SET_FILES.lock().unwrap() = Some(HashMap::new());
    *SET_OFFSETS.lock().unwrap() = Some(HashMap::new());
}

fn switch_current_scene_skin(header: SkinHeader) {
    {
        let current = CURRENT_SKIN.lock().unwrap();
        if current.is_some() {
            drop(current);
            save_current_config(&header);
        }
    }

    reset_current_skin_config();
    load_saved_skin_settings(&header);

    *CURRENT_SKIN.lock().unwrap() = Some(header.clone());
    let _property = complete_property(&header);

    let skin_path = header.get_path().to_string_lossy().to_string();
    let mut config = SkinConfig {
        path: skin_path,
        properties: _property,
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

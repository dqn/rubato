use std::fs;
use std::path::{Path, PathBuf};

use crate::main_controller::MainController;
use crate::main_state::{MainState, MainStateData, MainStateType};
use crate::player_config::PlayerConfig;
use crate::skin_config::{SkinConfig, SkinFilePath, SkinOffset, SkinOption, SkinProperty};
use crate::timer_manager::TimerManager;
use beatoraja_types::skin_type::SkinType;

/// OPTION_RANDOM_VALUE constant (mirrors beatoraja_skin::skin_property::OPTION_RANDOM_VALUE).
/// Defined locally because beatoraja-skin is not a dependency of beatoraja-core (circular dep).
const OPTION_RANDOM_VALUE: i32 = -1;

/// SkinProperty button constants (mirrors beatoraja_skin::skin_property).
/// Defined locally to avoid circular dependency on beatoraja-skin.
const BUTTON_CHANGE_SKIN: i32 = 190;
const BUTTON_SKIN_CUSTOMIZE1: i32 = 220;
const BUTTON_SKIN_CUSTOMIZE10: i32 = 229;
const BUTTON_SKINSELECT_7KEY: i32 = 170;
const BUTTON_SKINSELECT_COURSE_RESULT: i32 = 185;
const BUTTON_SKINSELECT_24KEY: i32 = 386;
const BUTTON_SKINSELECT_24KEY_BATTLE: i32 = 388;

// Local SkinPropertyMapper helpers (mirrors beatoraja_skin::skin_property_mapper).
// Defined locally to avoid circular dependency on beatoraja-skin.

fn is_skin_customize_button(id: i32) -> bool {
    (BUTTON_SKIN_CUSTOMIZE1..BUTTON_SKIN_CUSTOMIZE10).contains(&id)
}

fn get_skin_customize_index(id: i32) -> i32 {
    id - BUTTON_SKIN_CUSTOMIZE1
}

fn is_skin_select_type_id(id: i32) -> bool {
    (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id)
        || (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id)
}

fn get_skin_select_type(id: i32) -> Option<SkinType> {
    if (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id) {
        SkinType::get_skin_type_by_id(id - BUTTON_SKINSELECT_7KEY)
    } else if (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id) {
        SkinType::get_skin_type_by_id(id - BUTTON_SKINSELECT_24KEY + 16)
    } else {
        None
    }
}

/// Lightweight skin header for use within beatoraja-core.
///
/// beatoraja-skin's `SkinHeader` cannot be imported here due to circular dependencies.
/// This struct contains the subset of fields needed by `SkinConfiguration`.
#[derive(Clone, Debug, Default)]
pub struct SkinHeaderInfo {
    pub path: Option<PathBuf>,
    pub skin_type: Option<SkinType>,
    pub skin_type_id: i32,
    pub name: Option<String>,
    pub custom_options: Vec<CustomOptionDef>,
    pub custom_files: Vec<CustomFileDef>,
    pub custom_offsets: Vec<CustomOffsetDef>,
}

/// Definition of a custom option from the skin header.
#[derive(Clone, Debug)]
pub struct CustomOptionDef {
    pub name: String,
    pub option: Vec<i32>,
    pub contents: Vec<String>,
    pub def: Option<String>,
}

/// Definition of a custom file from the skin header.
#[derive(Clone, Debug)]
pub struct CustomFileDef {
    pub name: String,
    pub path: String,
    pub def: Option<String>,
}

/// Definition of a custom offset from the skin header.
#[derive(Clone, Debug)]
pub struct CustomOffsetDef {
    pub name: String,
    pub x: bool,
    pub y: bool,
    pub w: bool,
    pub h: bool,
    pub r: bool,
    pub a: bool,
}

/// UI item for skin configuration (replaces Java inner classes CustomItemBase hierarchy).
#[derive(Clone, Debug)]
pub enum CustomItem {
    Option {
        category_name: String,
        contents: Vec<String>,
        options: Vec<i32>,
        selection: usize,
        display_value: String,
    },
    File {
        category_name: String,
        display_values: Vec<String>,
        actual_values: Vec<String>,
        selection: usize,
        display_value: String,
    },
    Offset {
        category_name: String,
        offset_name: String,
        kind: usize,
        min: i32,
        max: i32,
        value: i32,
    },
}

impl CustomItem {
    pub fn get_category_name(&self) -> &str {
        match self {
            CustomItem::Option { category_name, .. } => category_name,
            CustomItem::File { category_name, .. } => category_name,
            CustomItem::Offset { category_name, .. } => category_name,
        }
    }

    pub fn get_display_value(&self) -> String {
        match self {
            CustomItem::Option { display_value, .. } => display_value.clone(),
            CustomItem::File { display_value, .. } => display_value.clone(),
            CustomItem::Offset { value, .. } => value.to_string(),
        }
    }

    pub fn get_value(&self) -> i32 {
        match self {
            CustomItem::Option { selection, .. } => *selection as i32,
            CustomItem::File { selection, .. } => *selection as i32,
            CustomItem::Offset { value, .. } => *value,
        }
    }

    pub fn get_min(&self) -> i32 {
        match self {
            CustomItem::Option { .. } => 0,
            CustomItem::File { .. } => 0,
            CustomItem::Offset { min, .. } => *min,
        }
    }

    pub fn get_max(&self) -> i32 {
        match self {
            CustomItem::Option { contents, .. } => contents.len() as i32 - 1,
            CustomItem::File { actual_values, .. } => actual_values.len() as i32 - 1,
            CustomItem::Offset { max, .. } => *max,
        }
    }
}

/// Helper for deferring persistence actions in set_custom_item_value to avoid borrow conflicts.
enum PersistAction {
    Option {
        name: String,
        value: i32,
    },
    File {
        name: String,
        path: String,
    },
    Offset {
        name: String,
        kind: usize,
        value: i32,
    },
}

/// Skin configuration screen.
/// Translated from Java: SkinConfiguration extends MainState
#[allow(dead_code)]
pub struct SkinConfiguration {
    state_data: MainStateData,
    skin_type: Option<SkinType>,
    config: Option<SkinConfig>,
    all_skins: Vec<SkinHeaderInfo>,
    available_skins: Vec<SkinHeaderInfo>,
    selected_skin_index: i32,
    selected_skin_header: Option<SkinHeaderInfo>,
    custom_options: Option<Vec<CustomItem>>,
    custom_option_offset: i32,
    custom_option_offset_max: i32,
    player: PlayerConfig,
    custom_property_count: i32,
}

impl SkinConfiguration {
    pub fn new(_main: &MainController, player: &PlayerConfig) -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            skin_type: None,
            config: None,
            all_skins: Vec::new(),
            available_skins: Vec::new(),
            selected_skin_index: -1,
            selected_skin_header: None,
            custom_options: None,
            custom_option_offset: 0,
            custom_option_offset_max: 0,
            player: player.clone(),
            custom_property_count: -1,
        }
    }

    pub fn create_internal(&mut self) {
        // In-game skin configuration is replaced by the launcher's SkinConfigurationView.
        // beatoraja-launcher/src/skin_configuration_view.rs provides the full egui UI
        // for skin browsing, header selection, and custom option editing.
    }

    pub fn render_internal(&mut self) {
        // In-game skin configuration rendering is replaced by the launcher's egui UI.
        // See beatoraja-launcher/src/launcher_ui.rs render_skin_tab() for the implementation.
    }

    /// Handle scroll input for navigating skin custom option lists.
    ///
    /// Java: SkinConfiguration.input()
    pub fn input_internal(
        &mut self,
        input: &mut dyn beatoraja_types::input_processor_access::InputProcessorAccess,
    ) {
        let mov = -input.get_scroll();
        input.reset_scroll();
        if mov != 0 && self.custom_options.is_some() {
            self.custom_option_offset = 0.max(
                self.custom_option_offset_max
                    .min(self.custom_option_offset + mov),
            );
        }
    }

    pub fn get_skin_type(&self) -> Option<SkinType> {
        self.skin_type
    }

    pub fn get_skin_select_position(&self) -> f32 {
        if self.custom_option_offset_max == 0 {
            0.0
        } else {
            self.custom_option_offset as f32 / self.custom_option_offset_max as f32
        }
    }

    pub fn set_skin_select_position(&mut self, value: f32) {
        if (0.0..1.0).contains(&value) {
            self.custom_option_offset = (self.custom_option_offset_max as f32 * value) as i32;
        }
    }

    pub fn get_category_name(&self, index: usize) -> &str {
        if let Some(ref options) = self.custom_options {
            let actual_index = index + self.custom_option_offset as usize;
            if actual_index < options.len() {
                return options[actual_index].get_category_name();
            }
        }
        ""
    }

    pub fn get_display_value(&self, index: usize) -> String {
        if let Some(ref options) = self.custom_options {
            let actual_index = index + self.custom_option_offset as usize;
            if actual_index < options.len() {
                return options[actual_index].get_display_value();
            }
        }
        String::new()
    }

    // ----------------------------------------------------------------
    // New methods (Phase 57)
    // ----------------------------------------------------------------

    /// Returns the currently selected skin header.
    pub fn get_selected_skin_header(&self) -> Option<&SkinHeaderInfo> {
        self.selected_skin_header.as_ref()
    }

    /// Set a file path in the current skin config properties.
    /// If the name already exists, update it; otherwise append a new entry.
    pub fn set_file_path(&mut self, name: &str, path: &str) {
        let props = self.ensure_properties();
        // Find existing entry
        for f in props.file.iter_mut().flatten() {
            if f.name.as_deref() == Some(name) {
                f.path = Some(path.to_string());
                return;
            }
        }
        // Not found — append new entry
        props.file.push(Some(SkinFilePath {
            name: Some(name.to_string()),
            path: Some(path.to_string()),
        }));
    }

    /// Set a custom option value in the current skin config properties.
    /// If the name already exists, update it; otherwise append a new entry.
    pub fn set_custom_option(&mut self, name: &str, value: i32) {
        let props = self.ensure_properties();
        for o in props.option.iter_mut().flatten() {
            if o.name.as_deref() == Some(name) {
                o.value = value;
                return;
            }
        }
        // Not found — append
        props.option.push(Some(SkinOption {
            name: Some(name.to_string()),
            value,
        }));
    }

    /// Set a custom offset value in the current skin config properties.
    /// If the name already exists, update the specified dimension; otherwise append a new entry.
    pub fn set_custom_offset(&mut self, name: &str, kind: usize, value: i32) {
        let props = self.ensure_properties();
        // Find existing entry
        for o in props.offset.iter_mut().flatten() {
            if o.name.as_deref() == Some(name) {
                Self::set_offset_value(o, kind, value);
                return;
            }
        }
        // Not found — append
        let mut new_offset = SkinOffset {
            name: Some(name.to_string()),
            ..SkinOffset::default()
        };
        Self::set_offset_value(&mut new_offset, kind, value);
        props.offset.push(Some(new_offset));
    }

    /// Save the current skin config to the player's skin history.
    pub fn save_skin_history(&mut self) {
        let config = match &self.config {
            Some(c) => c,
            None => return,
        };
        let config_path = match config.path.as_deref() {
            Some(p) if !p.is_empty() => p,
            _ => return,
        };

        // Find existing entry index
        let mut index: Option<usize> = None;
        for (i, hist) in self.player.skin_history.iter().enumerate() {
            if hist.get_path() == Some(config_path) {
                index = Some(i);
                break;
            }
        }

        let new_entry = SkinConfig {
            path: Some(config_path.to_string()),
            properties: config.properties.clone(),
        };

        if let Some(idx) = index {
            self.player.skin_history[idx] = new_entry;
        } else {
            self.player.skin_history.push(new_entry);
        }
    }

    /// Build CustomItem::Option list from skin header custom options.
    pub fn update_custom_options(&mut self) {
        let header = match &self.selected_skin_header {
            Some(h) => h.clone(),
            None => return,
        };

        // Collect deferred set_custom_option calls to avoid double borrow
        let mut deferred_options: Vec<(String, i32)> = Vec::new();
        let mut new_items: Vec<CustomItem> = Vec::new();

        for opt_def in &header.custom_options {
            let mut selection: Option<usize> = None;

            // Look up saved value in config properties
            if let Some(config) = &self.config
                && let Some(props) = &config.properties
            {
                for o in props.option.iter().flatten() {
                    if o.name.as_deref() == Some(&opt_def.name) {
                        let val = o.value;
                        if val != OPTION_RANDOM_VALUE {
                            for (j, &opt_val) in opt_def.option.iter().enumerate() {
                                if opt_val == val {
                                    selection = Some(j);
                                    break;
                                }
                            }
                        } else {
                            selection = Some(opt_def.option.len());
                        }
                        break;
                    }
                }
            }

            // Fallback to default
            if selection.is_none() {
                if let Some(def) = &opt_def.def {
                    for (j, content) in opt_def.contents.iter().enumerate() {
                        if content == def {
                            selection = Some(j);
                            break;
                        }
                    }
                }
                if selection.is_none() {
                    selection = Some(0);
                }
                // Defer persist of the default
                if !opt_def.option.is_empty() {
                    let sel = selection.unwrap_or(0);
                    let value = if sel < opt_def.option.len() {
                        opt_def.option[sel]
                    } else {
                        OPTION_RANDOM_VALUE
                    };
                    deferred_options.push((opt_def.name.clone(), value));
                }
            }

            let sel = selection.unwrap_or(0);

            let mut contents_with_random = opt_def.contents.clone();
            contents_with_random.push("Random".to_string());

            let mut options_with_random = opt_def.option.clone();
            options_with_random.push(OPTION_RANDOM_VALUE);

            let display = if sel < contents_with_random.len() {
                contents_with_random[sel].clone()
            } else {
                String::new()
            };

            new_items.push(CustomItem::Option {
                category_name: opt_def.name.clone(),
                contents: contents_with_random,
                options: options_with_random,
                selection: sel,
                display_value: display,
            });
        }

        // Apply deferred option writes
        for (name, value) in deferred_options {
            self.set_custom_option(&name, value);
        }

        // Append new items
        let options = self.custom_options.get_or_insert_with(Vec::new);
        options.extend(new_items);
    }

    /// Scan filesystem and build CustomItem::File list from skin header custom files.
    pub fn update_custom_files(&mut self) {
        let header = match &self.selected_skin_header {
            Some(h) => h.clone(),
            None => return,
        };

        let mut deferred_file_paths: Vec<(String, String)> = Vec::new();
        let mut new_items: Vec<CustomItem> = Vec::new();

        for file_def in &header.custom_files {
            let name = Self::extract_file_pattern(&file_def.path);

            let last_slash = file_def.path.rfind('/');
            let dir_str = match last_slash {
                Some(idx) => &file_def.path[..idx],
                None => continue,
            };

            let dir_path = Path::new(dir_str);
            if !dir_path.exists() {
                continue;
            }

            let mut items: Vec<String> = Vec::new();
            if let Ok(entries) = fs::read_dir(dir_path) {
                let name_lower = name.to_lowercase();
                let name_upper = name.to_uppercase();
                for entry in entries.flatten() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if fname.to_lowercase().ends_with(&name_lower)
                        || fname.to_uppercase().ends_with(&name_upper)
                    {
                        items.push(fname);
                    }
                }
            }
            items.push("Random".to_string());

            // Look up saved selection
            let mut selection: Option<String> = None;
            if let Some(config) = &self.config
                && let Some(props) = &config.properties
            {
                for f in props.file.iter().flatten() {
                    if f.name.as_deref() == Some(&file_def.name) {
                        selection = f.path.clone();
                        break;
                    }
                }
            }

            // Fallback to default
            if selection.is_none()
                && let Some(def) = &file_def.def
            {
                for item in &items {
                    if item.eq_ignore_ascii_case(def) {
                        selection = Some(item.clone());
                        break;
                    }
                    if let Some(point) = item.rfind('.')
                        && item[..point].eq_ignore_ascii_case(def)
                    {
                        selection = Some(item.clone());
                        break;
                    }
                }
                if let Some(sel) = &selection {
                    deferred_file_paths.push((file_def.name.clone(), sel.clone()));
                }
            }

            if selection.is_none() && !items.is_empty() {
                selection = Some(items[0].clone());
                deferred_file_paths.push((file_def.name.clone(), items[0].clone()));
            }

            let selection_str = selection.unwrap_or_default();

            let mut display_values: Vec<String> = Vec::new();
            let mut selected_index: usize = 0;
            for (i, item) in items.iter().enumerate() {
                let display = if let Some(point) = item.rfind('.') {
                    item[..point].to_string()
                } else {
                    item.clone()
                };
                display_values.push(display);
                if *item == selection_str {
                    selected_index = i;
                }
            }

            let display = if selected_index < display_values.len() {
                display_values[selected_index].clone()
            } else {
                String::new()
            };

            new_items.push(CustomItem::File {
                category_name: file_def.name.clone(),
                display_values,
                actual_values: items,
                selection: selected_index,
                display_value: display,
            });
        }

        // Apply deferred file path writes
        for (name, path) in deferred_file_paths {
            self.set_file_path(&name, &path);
        }

        let options = self.custom_options.get_or_insert_with(Vec::new);
        options.extend(new_items);
    }

    /// Build CustomItem::Offset list from skin header custom offsets.
    pub fn update_custom_offsets(&mut self) {
        let header = match &self.selected_skin_header {
            Some(h) => h.clone(),
            None => return,
        };

        let dimension_names = ["x", "y", "w", "h", "r", "a"];
        let mut missing_offsets: Vec<String> = Vec::new();
        let mut new_items: Vec<CustomItem> = Vec::new();

        for offset_def in &header.custom_offsets {
            let flags = [
                offset_def.x,
                offset_def.y,
                offset_def.w,
                offset_def.h,
                offset_def.r,
                offset_def.a,
            ];

            let mut offset_values = [0i32; 6];
            let mut found = false;

            if let Some(config) = &self.config
                && let Some(props) = &config.properties
            {
                for o in props.offset.iter().flatten() {
                    if o.name.as_deref() == Some(&offset_def.name) {
                        offset_values = [o.x, o.y, o.w, o.h, o.r, o.a];
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                missing_offsets.push(offset_def.name.clone());
            }

            for (i, &flag) in flags.iter().enumerate() {
                if flag {
                    new_items.push(CustomItem::Offset {
                        category_name: format!("{} - {}", offset_def.name, dimension_names[i]),
                        offset_name: offset_def.name.clone(),
                        kind: i,
                        min: -9999,
                        max: 9999,
                        value: offset_values[i],
                    });
                }
            }
        }

        // Create missing offset entries in config
        for name in missing_offsets {
            let props = self.ensure_properties();
            props.offset.push(Some(SkinOffset {
                name: Some(name),
                ..SkinOffset::default()
            }));
        }

        let options = self.custom_options.get_or_insert_with(Vec::new);
        options.extend(new_items);
    }

    /// Select a skin by index into available_skins.
    pub fn select_skin(&mut self, index: i32) {
        self.selected_skin_index = index;
        if index >= 0 {
            let idx = index as usize;
            if idx < self.available_skins.len() {
                self.selected_skin_header = Some(self.available_skins[idx].clone());
            }
            self.custom_options = Some(Vec::new());
            self.custom_option_offset = 0;

            // Load properties from skin history
            if let Some(ref header) = self.selected_skin_header
                && let Some(ref header_path) = header.path
            {
                let header_path_str = header_path.to_string_lossy().to_string();
                for hist in &self.player.skin_history {
                    if hist.get_path() == Some(header_path_str.as_str()) {
                        if let Some(ref mut config) = self.config {
                            config.properties = hist.properties.clone();
                        }
                        break;
                    }
                }
            }

            // Ensure properties exist
            if let Some(ref mut config) = self.config
                && config.properties.is_none()
            {
                config.properties = Some(SkinProperty::default());
            }

            self.update_custom_options();
            self.update_custom_files();
            self.update_custom_offsets();

            let option_count = self
                .custom_options
                .as_ref()
                .map(|o| o.len() as i32)
                .unwrap_or(0);
            self.custom_option_offset_max = (option_count - self.custom_property_count).max(0);
        } else {
            self.selected_skin_header = None;
            self.custom_options = None;
        }
    }

    /// Switch to another skin by relative index difference (wrapping).
    pub fn set_other_skin(&mut self, index_diff: i32) {
        if self.available_skins.is_empty() {
            log::warn!("No available skins");
            return;
        }

        if self.config.is_none() {
            self.config = Some(SkinConfig::default());
            if let Some(ref skin_type) = self.skin_type {
                let id = skin_type.get_id() as usize;
                let skin_vec = &mut self.player.skin;
                if id < skin_vec.len() {
                    skin_vec[id] = self.config.clone();
                }
            }
        } else {
            self.save_skin_history();
        }

        let len = self.available_skins.len() as i32;
        let index = if self.selected_skin_index < 0 {
            0
        } else {
            ((self.selected_skin_index + index_diff) % len + len) % len
        };

        // Update config path
        if let Some(ref mut config) = self.config {
            if let Some(ref path) = self.available_skins[index as usize].path {
                config.path = Some(path.to_string_lossy().to_string());
            }
            config.properties = Some(SkinProperty::default());
        }

        self.select_skin(index);
    }

    /// Select the next available skin.
    pub fn set_next_skin(&mut self) {
        self.set_other_skin(1);
    }

    /// Select the previous available skin.
    pub fn set_prev_skin(&mut self) {
        self.set_other_skin(-1);
    }

    /// Change the current skin type, filtering available skins and selecting the appropriate one.
    pub fn change_skin_type(&mut self, skin_type: Option<SkinType>) {
        self.save_skin_history();
        let st = skin_type.unwrap_or(SkinType::Play7Keys);
        self.skin_type = Some(st);

        // Load config for this skin type from player
        let id = st.get_id() as usize;
        self.config = if id < self.player.skin.len() {
            self.player.skin[id].clone()
        } else {
            None
        };

        // Filter available skins
        self.available_skins = self
            .all_skins
            .iter()
            .filter(|h| h.skin_type == Some(st))
            .cloned()
            .collect();

        // Find matching skin by path
        if let Some(ref config) = self.config
            && let Some(ref config_path) = config.path
            && !config_path.is_empty()
        {
            let config_path_buf = PathBuf::from(config_path);
            let mut found_index: i32 = -1;
            for (i, header) in self.available_skins.iter().enumerate() {
                if let Some(ref header_path) = header.path
                    && *header_path == config_path_buf
                {
                    found_index = i as i32;
                }
            }
            self.select_skin(found_index);
            return;
        }
        self.select_skin(-1);
    }

    /// Load all skin headers by recursively scanning the "skin" directory.
    ///
    /// The `loader` callback dispatches to the appropriate skin header parser based on
    /// file extension. It is injected from outside beatoraja-core (typically beatoraja-skin)
    /// to avoid circular dependencies.
    ///
    /// The callback receives a path and returns zero or more `SkinHeaderInfo` entries
    /// (LR2 7/14-key skins may produce a second 5/10-key variant).
    pub fn load_all_skins(&mut self, loader: &dyn Fn(&Path) -> Vec<SkinHeaderInfo>) {
        self.all_skins = Vec::new();
        let mut skin_paths: Vec<PathBuf> = Vec::new();
        Self::scan_skins(Path::new("skin"), &mut skin_paths);

        for path in &skin_paths {
            let path_str = path.to_string_lossy().to_lowercase();
            if path_str.ends_with(".json")
                || path_str.ends_with(".luaskin")
                || path_str.ends_with(".lr2skin")
            {
                let headers = loader(path);
                self.all_skins.extend(headers);
            }
        }

        if self.all_skins.is_empty() {
            log::warn!("load_all_skins: no skin headers loaded from 'skin' directory");
        }
    }

    /// Inject pre-loaded skin headers (for use by external loaders or tests).
    pub fn set_all_skins(&mut self, skins: Vec<SkinHeaderInfo>) {
        self.all_skins = skins;
    }

    /// Set the custom property count (normally read from SkinConfigurationSkin).
    pub fn set_custom_property_count(&mut self, count: i32) {
        self.custom_property_count = count;
    }

    /// Get a reference to the current player config.
    pub fn get_player(&self) -> &PlayerConfig {
        &self.player
    }

    /// Get a mutable reference to the current player config.
    pub fn get_player_mut(&mut self) -> &mut PlayerConfig {
        &mut self.player
    }

    pub fn execute_event(&mut self, id: i32, arg1: i32, _arg2: i32) {
        match id {
            BUTTON_CHANGE_SKIN => {
                if arg1 >= 0 {
                    self.set_next_skin();
                } else {
                    self.set_prev_skin();
                }
            }
            _ => {
                if is_skin_customize_button(id) {
                    let index = get_skin_customize_index(id) + self.custom_option_offset;
                    if let Some(ref mut options) = self.custom_options {
                        let idx = index as usize;
                        if idx < options.len() {
                            let current_value = options[idx].get_value();
                            let min = options[idx].get_min();
                            let max = options[idx].get_max();

                            let new_value = if arg1 >= 0 {
                                if current_value < max {
                                    current_value + 1
                                } else {
                                    min
                                }
                            } else if current_value > min {
                                current_value - 1
                            } else {
                                max
                            };

                            // Update item and persist to config
                            self.set_custom_item_value(idx, new_value);
                        }
                    }
                } else if is_skin_select_type_id(id) {
                    let skin_type = get_skin_select_type(id);
                    self.change_skin_type(skin_type);
                }
                // Java: super.executeEvent(id, arg1, arg2) — default no-op in Rust
            }
        }
    }

    pub fn dispose_resources(&mut self) {
        // Java SkinConfiguration.dispose() only calls super.dispose().
        // Skin/stage cleanup is handled by the MainState::dispose() impl.
    }

    /// Update a CustomItem's value and persist the change to the skin config.
    ///
    /// Mirrors Java's CustomItemBase.setValue() dispatch:
    /// - CustomOptionItem: persists via setCustomOption(categoryName, options[value])
    /// - CustomFileItem: persists via setFilePath(categoryName, actualValues[value])
    /// - CustomOffsetItem: persists via setCustomOffset(offsetName, kind, value)
    fn set_custom_item_value(&mut self, index: usize, new_value: i32) {
        // Extract info needed for persistence before mutating the item
        let persist_action = {
            let options = match self.custom_options.as_ref() {
                Some(o) => o,
                None => return,
            };
            let item = match options.get(index) {
                Some(i) => i,
                None => return,
            };
            match item {
                CustomItem::Option {
                    category_name,
                    options,
                    ..
                } => {
                    let val = new_value as usize;
                    let opt_val = if val < options.len() {
                        options[val]
                    } else {
                        return;
                    };
                    PersistAction::Option {
                        name: category_name.clone(),
                        value: opt_val,
                    }
                }
                CustomItem::File {
                    category_name,
                    actual_values,
                    ..
                } => {
                    let val = new_value as usize;
                    let path = if val < actual_values.len() {
                        actual_values[val].clone()
                    } else {
                        return;
                    };
                    PersistAction::File {
                        name: category_name.clone(),
                        path,
                    }
                }
                CustomItem::Offset {
                    offset_name, kind, ..
                } => PersistAction::Offset {
                    name: offset_name.clone(),
                    kind: *kind,
                    value: new_value,
                },
            }
        };

        // Now mutate the item
        if let Some(ref mut options) = self.custom_options
            && let Some(item) = options.get_mut(index)
        {
            match item {
                CustomItem::Option {
                    selection,
                    display_value,
                    contents,
                    ..
                } => {
                    let val = new_value as usize;
                    *selection = val;
                    *display_value = if val < contents.len() {
                        contents[val].clone()
                    } else {
                        String::new()
                    };
                }
                CustomItem::File {
                    selection,
                    display_value,
                    display_values,
                    ..
                } => {
                    let val = new_value as usize;
                    *selection = val;
                    *display_value = if val < display_values.len() {
                        display_values[val].clone()
                    } else {
                        String::new()
                    };
                }
                CustomItem::Offset { value, .. } => {
                    *value = new_value;
                }
            }
        }

        // Persist the change to the skin config
        match persist_action {
            PersistAction::Option { name, value } => {
                self.set_custom_option(&name, value);
            }
            PersistAction::File { name, path } => {
                self.set_file_path(&name, &path);
            }
            PersistAction::Offset { name, kind, value } => {
                self.set_custom_offset(&name, kind, value);
            }
        }
    }

    // ----------------------------------------------------------------
    // Private helpers
    // ----------------------------------------------------------------

    /// Ensure that `self.config` exists and has `properties`, returning a mutable reference.
    fn ensure_properties(&mut self) -> &mut SkinProperty {
        self.config
            .get_or_insert_with(SkinConfig::default)
            .properties
            .get_or_insert_with(SkinProperty::default)
    }

    /// Set a specific dimension on a SkinOffset by kind index.
    fn set_offset_value(offset: &mut SkinOffset, kind: usize, value: i32) {
        match kind {
            0 => offset.x = value,
            1 => offset.y = value,
            2 => offset.w = value,
            3 => offset.h = value,
            4 => offset.r = value,
            5 => offset.a = value,
            _ => {}
        }
    }

    /// Extract the file pattern from a skin file path spec (handles '|' separator).
    fn extract_file_pattern(path: &str) -> String {
        let after_slash = if let Some(idx) = path.rfind('/') {
            &path[idx + 1..]
        } else {
            path
        };

        if path.contains('|') {
            let slash_pos = path.rfind('/').map(|i| i + 1).unwrap_or(0);
            let pipe_first = path.find('|').expect("contains('|') guarantees Some");
            let pipe_last = path.rfind('|').expect("contains('|') guarantees Some");
            if path.len() > pipe_last + 1 {
                format!("{}{}", &path[slash_pos..pipe_first], &path[pipe_last + 1..])
            } else {
                path[slash_pos..pipe_first].to_string()
            }
        } else {
            after_slash.to_string()
        }
    }

    /// Scan skin files recursively.
    fn scan_skins(path: &Path, paths: &mut Vec<PathBuf>) {
        if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    Self::scan_skins(&entry.path(), paths);
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
}

impl MainState for SkinConfiguration {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::SkinConfig)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {
        self.create_internal();
    }

    fn render(&mut self) {
        self.render_internal();
    }

    fn input(&mut self) {
        // input_internal requires InputProcessorAccess which is not available
        // through the MainState::input() trait method. Scroll input handling
        // is deferred to the launcher's egui SkinConfigurationView.
    }

    fn dispose(&mut self) {
        self.dispose_resources();
        // Call default trait dispose for skin/stage cleanup
        let data = self.main_state_data_mut();
        data.skin = None;
        data.stage = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_header(path: &str, skin_type: SkinType) -> SkinHeaderInfo {
        SkinHeaderInfo {
            path: Some(PathBuf::from(path)),
            skin_type: Some(skin_type),
            name: Some(format!("Test Skin {}", path)),
            ..SkinHeaderInfo::default()
        }
    }

    fn make_config_with_path(path: &str) -> SkinConfig {
        SkinConfig {
            path: Some(path.to_string()),
            properties: Some(SkinProperty::default()),
        }
    }

    /// Helper to create a minimal SkinConfiguration for testing (bypasses MainController).
    fn make_test_skin_config() -> SkinConfiguration {
        SkinConfiguration {
            state_data: MainStateData::new(TimerManager::new()),
            skin_type: None,
            config: Some(SkinConfig {
                path: Some("skin/test.json".to_string()),
                properties: Some(SkinProperty::default()),
            }),
            all_skins: Vec::new(),
            available_skins: Vec::new(),
            selected_skin_index: -1,
            selected_skin_header: None,
            custom_options: None,
            custom_option_offset: 0,
            custom_option_offset_max: 0,
            player: PlayerConfig::default(),
            custom_property_count: -1,
        }
    }

    #[test]
    fn test_get_selected_skin_header_none() {
        let sc = make_test_skin_config();
        assert!(sc.get_selected_skin_header().is_none());
    }

    #[test]
    fn test_get_selected_skin_header_some() {
        let mut sc = make_test_skin_config();
        sc.selected_skin_header = Some(make_test_header("skin/play7.json", SkinType::Play7Keys));
        let header = sc.get_selected_skin_header().unwrap();
        assert_eq!(header.path, Some(PathBuf::from("skin/play7.json")));
        assert_eq!(header.skin_type, Some(SkinType::Play7Keys));
    }

    #[test]
    fn test_set_file_path_new() {
        let mut sc = make_test_skin_config();
        sc.set_file_path("bg_image", "background.png");

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.file.len(), 1);
        let f = props.file[0].as_ref().unwrap();
        assert_eq!(f.name.as_deref(), Some("bg_image"));
        assert_eq!(f.path.as_deref(), Some("background.png"));
    }

    #[test]
    fn test_set_file_path_update_existing() {
        let mut sc = make_test_skin_config();
        sc.set_file_path("bg_image", "old.png");
        sc.set_file_path("bg_image", "new.png");

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.file.len(), 1);
        let f = props.file[0].as_ref().unwrap();
        assert_eq!(f.path.as_deref(), Some("new.png"));
    }

    #[test]
    fn test_set_custom_option_new() {
        let mut sc = make_test_skin_config();
        sc.set_custom_option("judge_timing", 42);

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.option.len(), 1);
        let o = props.option[0].as_ref().unwrap();
        assert_eq!(o.name.as_deref(), Some("judge_timing"));
        assert_eq!(o.value, 42);
    }

    #[test]
    fn test_set_custom_option_update_existing() {
        let mut sc = make_test_skin_config();
        sc.set_custom_option("judge_timing", 42);
        sc.set_custom_option("judge_timing", 100);

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.option.len(), 1);
        let o = props.option[0].as_ref().unwrap();
        assert_eq!(o.value, 100);
    }

    #[test]
    fn test_set_custom_offset_new() {
        let mut sc = make_test_skin_config();
        sc.set_custom_offset("judge_offset", 0, 10); // x = 10

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.offset.len(), 1);
        let o = props.offset[0].as_ref().unwrap();
        assert_eq!(o.name.as_deref(), Some("judge_offset"));
        assert_eq!(o.x, 10);
        assert_eq!(o.y, 0);
    }

    #[test]
    fn test_set_custom_offset_update_existing() {
        let mut sc = make_test_skin_config();
        sc.set_custom_offset("judge_offset", 0, 10); // x = 10
        sc.set_custom_offset("judge_offset", 1, 20); // y = 20

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.offset.len(), 1);
        let o = props.offset[0].as_ref().unwrap();
        assert_eq!(o.x, 10);
        assert_eq!(o.y, 20);
    }

    #[test]
    fn test_set_custom_offset_all_kinds() {
        let mut sc = make_test_skin_config();
        for kind in 0..6 {
            sc.set_custom_offset("test", kind, (kind as i32 + 1) * 10);
        }
        // First call creates, subsequent calls update
        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.offset.len(), 1);
        let o = props.offset[0].as_ref().unwrap();
        assert_eq!(o.x, 10);
        assert_eq!(o.y, 20);
        assert_eq!(o.w, 30);
        assert_eq!(o.h, 40);
        assert_eq!(o.r, 50);
        assert_eq!(o.a, 60);
    }

    #[test]
    fn test_save_skin_history_new_entry() {
        let mut sc = make_test_skin_config();
        sc.config = Some(make_config_with_path("skin/play7.json"));
        assert!(sc.player.skin_history.is_empty());

        sc.save_skin_history();
        assert_eq!(sc.player.skin_history.len(), 1);
        assert_eq!(
            sc.player.skin_history[0].get_path(),
            Some("skin/play7.json")
        );
    }

    #[test]
    fn test_save_skin_history_update_existing() {
        let mut sc = make_test_skin_config();
        sc.player.skin_history.push(SkinConfig {
            path: Some("skin/play7.json".to_string()),
            properties: None,
        });
        sc.config = Some(SkinConfig {
            path: Some("skin/play7.json".to_string()),
            properties: Some(SkinProperty::default()),
        });

        sc.save_skin_history();
        assert_eq!(sc.player.skin_history.len(), 1);
        // Should have updated properties (not None anymore)
        assert!(sc.player.skin_history[0].properties.is_some());
    }

    #[test]
    fn test_save_skin_history_no_config() {
        let mut sc = make_test_skin_config();
        sc.config = None;
        sc.save_skin_history();
        assert!(sc.player.skin_history.is_empty());
    }

    #[test]
    fn test_save_skin_history_empty_path() {
        let mut sc = make_test_skin_config();
        sc.config = Some(SkinConfig {
            path: Some(String::new()),
            properties: None,
        });
        sc.save_skin_history();
        assert!(sc.player.skin_history.is_empty());
    }

    #[test]
    fn test_change_skin_type_filters_available() {
        let mut sc = make_test_skin_config();
        sc.all_skins = vec![
            make_test_header("skin/play7.json", SkinType::Play7Keys),
            make_test_header("skin/select.json", SkinType::MusicSelect),
            make_test_header("skin/play7_alt.json", SkinType::Play7Keys),
        ];

        sc.change_skin_type(Some(SkinType::Play7Keys));
        assert_eq!(sc.available_skins.len(), 2);
        assert_eq!(sc.skin_type, Some(SkinType::Play7Keys));
    }

    #[test]
    fn test_change_skin_type_defaults_to_play7keys() {
        let mut sc = make_test_skin_config();
        sc.change_skin_type(None);
        assert_eq!(sc.skin_type, Some(SkinType::Play7Keys));
    }

    #[test]
    fn test_change_skin_type_selects_matching_config_path() {
        let mut sc = make_test_skin_config();
        sc.all_skins = vec![
            make_test_header("skin/play7.json", SkinType::Play7Keys),
            make_test_header("skin/play7_alt.json", SkinType::Play7Keys),
        ];
        // Set up player config with a skin path for Play7Keys (id=0)
        if sc.player.skin.is_empty() {
            sc.player.skin.resize_with(19, || None);
        }
        sc.player.skin[0] = Some(make_config_with_path("skin/play7_alt.json"));

        sc.change_skin_type(Some(SkinType::Play7Keys));
        assert_eq!(sc.selected_skin_index, 1); // second entry
    }

    #[test]
    fn test_select_skin_positive_index() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/b.json"));

        sc.select_skin(1);
        assert_eq!(sc.selected_skin_index, 1);
        let header = sc.selected_skin_header.as_ref().unwrap();
        assert_eq!(header.path, Some(PathBuf::from("skin/b.json")));
        assert!(sc.custom_options.is_some());
    }

    #[test]
    fn test_select_skin_negative_clears() {
        let mut sc = make_test_skin_config();
        sc.selected_skin_header = Some(make_test_header("skin/a.json", SkinType::Play7Keys));
        sc.custom_options = Some(vec![]);

        sc.select_skin(-1);
        assert_eq!(sc.selected_skin_index, -1);
        assert!(sc.selected_skin_header.is_none());
        assert!(sc.custom_options.is_none());
    }

    #[test]
    fn test_set_other_skin_wraps_forward() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
            make_test_header("skin/c.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/c.json"));
        sc.selected_skin_index = 2; // last skin

        sc.set_other_skin(1); // should wrap to 0
        assert_eq!(sc.selected_skin_index, 0);
    }

    #[test]
    fn test_set_other_skin_wraps_backward() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
            make_test_header("skin/c.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/a.json"));
        sc.selected_skin_index = 0;

        sc.set_other_skin(-1); // should wrap to 2
        assert_eq!(sc.selected_skin_index, 2);
    }

    #[test]
    fn test_set_next_prev_skin() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/a.json"));
        sc.selected_skin_index = 0;

        sc.set_next_skin();
        assert_eq!(sc.selected_skin_index, 1);

        sc.set_prev_skin();
        assert_eq!(sc.selected_skin_index, 0);
    }

    #[test]
    fn test_set_other_skin_empty_available() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![];
        sc.selected_skin_index = 0;

        sc.set_other_skin(1); // should not panic
        assert_eq!(sc.selected_skin_index, 0); // unchanged
    }

    #[test]
    fn test_update_custom_options_basic() {
        let mut sc = make_test_skin_config();
        sc.selected_skin_header = Some(SkinHeaderInfo {
            custom_options: vec![CustomOptionDef {
                name: "judge_type".to_string(),
                option: vec![0, 1, 2],
                contents: vec!["Normal".to_string(), "Hard".to_string(), "Easy".to_string()],
                def: Some("Normal".to_string()),
            }],
            ..SkinHeaderInfo::default()
        });
        sc.custom_options = Some(Vec::new());

        sc.update_custom_options();

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].get_category_name(), "judge_type");
        // "Normal" + "Hard" + "Easy" + "Random" = 4 display values, max = 3
        assert_eq!(options[0].get_max(), 3);
        assert_eq!(options[0].get_value(), 0); // default selection = 0 (Normal)
    }

    #[test]
    fn test_update_custom_options_with_saved_value() {
        let mut sc = make_test_skin_config();
        // Set up saved option
        {
            let props = sc.ensure_properties();
            props.option.push(Some(SkinOption {
                name: Some("judge_type".to_string()),
                value: 2, // "Easy"
            }));
        }
        sc.selected_skin_header = Some(SkinHeaderInfo {
            custom_options: vec![CustomOptionDef {
                name: "judge_type".to_string(),
                option: vec![0, 1, 2],
                contents: vec!["Normal".to_string(), "Hard".to_string(), "Easy".to_string()],
                def: None,
            }],
            ..SkinHeaderInfo::default()
        });
        sc.custom_options = Some(Vec::new());

        sc.update_custom_options();

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].get_value(), 2); // Easy is at index 2
    }

    #[test]
    fn test_update_custom_options_random_value() {
        let mut sc = make_test_skin_config();
        {
            let props = sc.ensure_properties();
            props.option.push(Some(SkinOption {
                name: Some("judge_type".to_string()),
                value: OPTION_RANDOM_VALUE,
            }));
        }
        sc.selected_skin_header = Some(SkinHeaderInfo {
            custom_options: vec![CustomOptionDef {
                name: "judge_type".to_string(),
                option: vec![0, 1],
                contents: vec!["Normal".to_string(), "Hard".to_string()],
                def: None,
            }],
            ..SkinHeaderInfo::default()
        });
        sc.custom_options = Some(Vec::new());

        sc.update_custom_options();

        let options = sc.custom_options.as_ref().unwrap();
        // Random is at index option.len() = 2
        assert_eq!(options[0].get_value(), 2);
        assert_eq!(options[0].get_display_value(), "Random");
    }

    #[test]
    fn test_update_custom_offsets_basic() {
        let mut sc = make_test_skin_config();
        sc.selected_skin_header = Some(SkinHeaderInfo {
            custom_offsets: vec![CustomOffsetDef {
                name: "judge_pos".to_string(),
                x: true,
                y: true,
                w: false,
                h: false,
                r: false,
                a: false,
            }],
            ..SkinHeaderInfo::default()
        });
        sc.custom_options = Some(Vec::new());

        sc.update_custom_offsets();

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options.len(), 2); // x and y enabled
        assert_eq!(options[0].get_category_name(), "judge_pos - x");
        assert_eq!(options[1].get_category_name(), "judge_pos - y");
        assert_eq!(options[0].get_min(), -9999);
        assert_eq!(options[0].get_max(), 9999);
    }

    #[test]
    fn test_custom_item_option_set_value() {
        let mut item = CustomItem::Option {
            category_name: "test".to_string(),
            contents: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            options: vec![10, 20, 30],
            selection: 0,
            display_value: "A".to_string(),
        };
        assert_eq!(item.get_value(), 0);
        assert_eq!(item.get_display_value(), "A");

        // Simulate changing selection
        if let CustomItem::Option {
            ref mut selection,
            ref mut display_value,
            ref contents,
            ..
        } = item
        {
            *selection = 2;
            *display_value = contents[2].clone();
        }
        assert_eq!(item.get_value(), 2);
        assert_eq!(item.get_display_value(), "C");
    }

    #[test]
    fn test_custom_item_offset_properties() {
        let item = CustomItem::Offset {
            category_name: "pos - x".to_string(),
            offset_name: "pos".to_string(),
            kind: 0,
            min: -100,
            max: 100,
            value: 42,
        };
        assert_eq!(item.get_category_name(), "pos - x");
        assert_eq!(item.get_value(), 42);
        assert_eq!(item.get_min(), -100);
        assert_eq!(item.get_max(), 100);
        assert_eq!(item.get_display_value(), "42");
    }

    #[test]
    fn test_extract_file_pattern_simple() {
        assert_eq!(
            SkinConfiguration::extract_file_pattern("skin/images/*.png"),
            "*.png"
        );
    }

    #[test]
    fn test_extract_file_pattern_with_pipe() {
        // "skin/images/bg*.png|.jpg" -> "bg*.png.jpg"
        assert_eq!(
            SkinConfiguration::extract_file_pattern("skin/images/bg*.png|.jpg"),
            "bg*.png.jpg"
        );
    }

    #[test]
    fn test_extract_file_pattern_with_pipe_empty_suffix() {
        // "skin/images/bg*.png|" -> "bg*.png"
        assert_eq!(
            SkinConfiguration::extract_file_pattern("skin/images/bg*.png|"),
            "bg*.png"
        );
    }

    #[test]
    fn test_get_category_name_with_offset() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![
            CustomItem::Option {
                category_name: "first".to_string(),
                contents: vec![],
                options: vec![],
                selection: 0,
                display_value: String::new(),
            },
            CustomItem::Option {
                category_name: "second".to_string(),
                contents: vec![],
                options: vec![],
                selection: 0,
                display_value: String::new(),
            },
        ]);
        sc.custom_option_offset = 1;

        assert_eq!(sc.get_category_name(0), "second");
    }

    #[test]
    fn test_get_category_name_out_of_bounds() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![]);
        assert_eq!(sc.get_category_name(0), "");
    }

    #[test]
    fn test_ensure_properties_creates_config() {
        let mut sc = make_test_skin_config();
        sc.config = None;
        let _props = sc.ensure_properties();
        assert!(sc.config.is_some());
        assert!(sc.config.as_ref().unwrap().properties.is_some());
    }

    #[test]
    fn test_skin_select_position() {
        let mut sc = make_test_skin_config();
        sc.custom_option_offset_max = 10;

        sc.set_skin_select_position(0.5);
        assert_eq!(sc.custom_option_offset, 5);
        assert!((sc.get_skin_select_position() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_skin_select_position_zero_max() {
        let sc = make_test_skin_config();
        assert_eq!(sc.get_skin_select_position(), 0.0);
    }

    #[test]
    fn test_load_all_skins_stub_no_panic() {
        let mut sc = make_test_skin_config();
        // Should not panic even though no skin dir exists
        sc.load_all_skins(&|_path| Vec::new());
        assert!(sc.all_skins.is_empty());
    }

    #[test]
    fn test_set_all_skins() {
        let mut sc = make_test_skin_config();
        let skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::MusicSelect),
        ];
        sc.set_all_skins(skins);
        assert_eq!(sc.all_skins.len(), 2);
    }

    // ----------------------------------------------------------------
    // execute_event tests
    // ----------------------------------------------------------------

    #[test]
    fn test_execute_event_change_skin_forward() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/a.json"));
        sc.selected_skin_index = 0;

        // BUTTON_CHANGE_SKIN = 190, arg1 >= 0 => set_next_skin
        sc.execute_event(BUTTON_CHANGE_SKIN, 1, 0);
        assert_eq!(sc.selected_skin_index, 1);
    }

    #[test]
    fn test_execute_event_change_skin_backward() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/b.json"));
        sc.selected_skin_index = 1;

        // arg1 < 0 => set_prev_skin
        sc.execute_event(BUTTON_CHANGE_SKIN, -1, 0);
        assert_eq!(sc.selected_skin_index, 0);
    }

    #[test]
    fn test_execute_event_customize_button_increment() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "judge_type".to_string(),
            contents: vec![
                "Normal".to_string(),
                "Hard".to_string(),
                "Random".to_string(),
            ],
            options: vec![0, 1, OPTION_RANDOM_VALUE],
            selection: 0,
            display_value: "Normal".to_string(),
        }]);
        sc.custom_option_offset = 0;

        // BUTTON_SKIN_CUSTOMIZE1 = 220, arg1 >= 0 => increment
        sc.execute_event(220, 1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].get_value(), 1); // selection moved to index 1
        assert_eq!(options[0].get_display_value(), "Hard");
    }

    #[test]
    fn test_execute_event_customize_button_decrement() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "judge_type".to_string(),
            contents: vec![
                "Normal".to_string(),
                "Hard".to_string(),
                "Random".to_string(),
            ],
            options: vec![0, 1, OPTION_RANDOM_VALUE],
            selection: 1,
            display_value: "Hard".to_string(),
        }]);
        sc.custom_option_offset = 0;

        // arg1 < 0 => decrement
        sc.execute_event(220, -1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].get_value(), 0);
        assert_eq!(options[0].get_display_value(), "Normal");
    }

    #[test]
    fn test_execute_event_customize_button_wrap_forward() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "judge_type".to_string(),
            contents: vec!["A".to_string(), "B".to_string()],
            options: vec![0, 1],
            selection: 1, // at max
            display_value: "B".to_string(),
        }]);
        sc.custom_option_offset = 0;

        // At max, increment should wrap to min (0)
        sc.execute_event(220, 1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].get_value(), 0);
        assert_eq!(options[0].get_display_value(), "A");
    }

    #[test]
    fn test_execute_event_customize_button_wrap_backward() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "judge_type".to_string(),
            contents: vec!["A".to_string(), "B".to_string()],
            options: vec![0, 1],
            selection: 0, // at min
            display_value: "A".to_string(),
        }]);
        sc.custom_option_offset = 0;

        // At min, decrement should wrap to max (1)
        sc.execute_event(220, -1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].get_value(), 1);
        assert_eq!(options[0].get_display_value(), "B");
    }

    #[test]
    fn test_execute_event_customize_button_with_offset() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![
            CustomItem::Option {
                category_name: "first".to_string(),
                contents: vec!["X".to_string(), "Y".to_string()],
                options: vec![10, 20],
                selection: 0,
                display_value: "X".to_string(),
            },
            CustomItem::Option {
                category_name: "second".to_string(),
                contents: vec!["A".to_string(), "B".to_string(), "C".to_string()],
                options: vec![100, 200, 300],
                selection: 0,
                display_value: "A".to_string(),
            },
        ]);
        sc.custom_option_offset = 1; // offset by 1, so CUSTOMIZE1 (index 0) maps to items[1]

        // BUTTON_SKIN_CUSTOMIZE1 = 220, index = 0 + offset 1 = items[1]
        sc.execute_event(220, 1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        // First item should be unchanged
        assert_eq!(options[0].get_value(), 0);
        // Second item should have incremented
        assert_eq!(options[1].get_value(), 1);
        assert_eq!(options[1].get_display_value(), "B");
    }

    #[test]
    fn test_execute_event_customize_persists_option() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "my_opt".to_string(),
            contents: vec!["Off".to_string(), "On".to_string()],
            options: vec![0, 42],
            selection: 0,
            display_value: "Off".to_string(),
        }]);
        sc.custom_option_offset = 0;

        sc.execute_event(220, 1, 0); // increment to selection=1, option value=42

        // Verify the option was persisted to config
        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        let saved = props
            .option
            .iter()
            .flatten()
            .find(|o| o.name.as_deref() == Some("my_opt"));
        assert!(saved.is_some());
        assert_eq!(saved.unwrap().value, 42);
    }

    #[test]
    fn test_execute_event_customize_offset_item() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Offset {
            category_name: "pos - x".to_string(),
            offset_name: "pos".to_string(),
            kind: 0,
            min: -9999,
            max: 9999,
            value: 50,
        }]);
        sc.custom_option_offset = 0;

        // Increment from 50 to 51
        sc.execute_event(220, 1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].get_value(), 51);
    }

    #[test]
    fn test_execute_event_skin_select_type() {
        let mut sc = make_test_skin_config();
        sc.all_skins = vec![
            make_test_header("skin/play7.json", SkinType::Play7Keys),
            make_test_header("skin/select.json", SkinType::MusicSelect),
        ];

        // BUTTON_SKINSELECT_7KEY = 170 => SkinType::Play7Keys (id 0)
        sc.execute_event(170, 0, 0);
        assert_eq!(sc.skin_type, Some(SkinType::Play7Keys));
        assert_eq!(sc.available_skins.len(), 1);
    }

    #[test]
    fn test_execute_event_skin_select_type_music_select() {
        let mut sc = make_test_skin_config();
        sc.all_skins = vec![
            make_test_header("skin/play7.json", SkinType::Play7Keys),
            make_test_header("skin/select.json", SkinType::MusicSelect),
        ];

        // BUTTON_SKINSELECT_MUSIC_SELECT = 175 (7KEY=170, offset 5 = MusicSelect)
        sc.execute_event(175, 0, 0);
        assert_eq!(sc.skin_type, Some(SkinType::MusicSelect));
        assert_eq!(sc.available_skins.len(), 1);
    }

    #[test]
    fn test_execute_event_unknown_id_no_panic() {
        let mut sc = make_test_skin_config();
        // Unknown event id — should not panic (falls through to no-op)
        sc.execute_event(9999, 0, 0);
    }

    // ----------------------------------------------------------------
    // Local SkinPropertyMapper function tests
    // ----------------------------------------------------------------

    #[test]
    fn test_is_skin_customize_button() {
        // Range: [220, 229) — exclusive upper bound
        assert!(is_skin_customize_button(220));
        assert!(is_skin_customize_button(224));
        assert!(is_skin_customize_button(228));
        assert!(!is_skin_customize_button(219));
        assert!(!is_skin_customize_button(229));
    }

    #[test]
    fn test_get_skin_customize_index() {
        assert_eq!(get_skin_customize_index(220), 0);
        assert_eq!(get_skin_customize_index(225), 5);
        assert_eq!(get_skin_customize_index(228), 8);
    }

    #[test]
    fn test_is_skin_select_type_id() {
        // Primary range: [170, 185]
        assert!(is_skin_select_type_id(170)); // 7KEY
        assert!(is_skin_select_type_id(185)); // COURSE_RESULT
        assert!(!is_skin_select_type_id(169));
        assert!(!is_skin_select_type_id(186));
        // 24KEY range: [386, 388]
        assert!(is_skin_select_type_id(386));
        assert!(is_skin_select_type_id(388));
        assert!(!is_skin_select_type_id(385));
        assert!(!is_skin_select_type_id(389));
    }

    #[test]
    fn test_get_skin_select_type() {
        // 170 = BUTTON_SKINSELECT_7KEY => SkinType id 0 = Play7Keys
        assert_eq!(get_skin_select_type(170), Some(SkinType::Play7Keys));
        // 175 = Music Select => SkinType id 5 = MusicSelect
        assert_eq!(get_skin_select_type(175), Some(SkinType::MusicSelect));
        // Out of range
        assert_eq!(get_skin_select_type(0), None);
        assert_eq!(get_skin_select_type(999), None);
    }
}

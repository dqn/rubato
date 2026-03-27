use std::fs;
use std::path::{Path, PathBuf};

use crate::core::main_controller::MainController;
use crate::core::app_context::GameContext;
use crate::core::main_state::{MainState, MainStateData, MainStateType, StateTransition};
use crate::core::player_config::PlayerConfig;
use crate::core::skin_config::{SkinConfig, SkinFilePath, SkinOffset, SkinOption, SkinProperty};
use crate::core::timer_manager::TimerManager;
use rubato_types::skin_type::SkinType;

use super::SkinConfiguration;
use super::types::*;

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
        input: &mut dyn rubato_types::input_processor_access::InputProcessorAccess,
    ) {
        let mov = -input.scroll();
        input.reset_scroll();
        if mov != 0 && self.custom_options.is_some() {
            self.custom_option_offset = 0.max(
                self.custom_option_offset_max
                    .min(self.custom_option_offset + mov),
            );
        }
    }

    pub fn skin_type(&self) -> Option<SkinType> {
        self.skin_type
    }

    pub fn skin_select_position(&self) -> f32 {
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

    pub fn category_name(&self, index: usize) -> &str {
        if let Some(ref options) = self.custom_options {
            let actual_index = index + self.custom_option_offset as usize;
            if actual_index < options.len() {
                return options[actual_index].category_name();
            }
        }
        ""
    }

    pub fn display_value(&self, index: usize) -> String {
        if let Some(ref options) = self.custom_options {
            let actual_index = index + self.custom_option_offset as usize;
            if actual_index < options.len() {
                return options[actual_index].display_value();
            }
        }
        String::new()
    }

    // ----------------------------------------------------------------
    // New methods (Phase 57)
    // ----------------------------------------------------------------

    /// Returns the currently selected skin header.
    pub fn selected_skin_header(&self) -> Option<&SkinHeaderInfo> {
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
            if hist.path() == Some(config_path) {
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
                None => ".",
            };

            let dir_path = Path::new(dir_str);
            if !dir_path.exists() {
                continue;
            }

            let mut items: Vec<String> = Vec::new();
            if let Ok(entries) = fs::read_dir(dir_path) {
                for entry in entries.flatten() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if matches_wildcard_case_insensitive(&fname, &name) {
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
                offset_def.caps.x,
                offset_def.caps.y,
                offset_def.caps.w,
                offset_def.caps.h,
                offset_def.caps.r,
                offset_def.caps.a,
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
                    if hist.path() == Some(header_path_str.as_str()) {
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
                let id = skin_type.id() as usize;
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
        let id = st.id() as usize;
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
        self.load_all_skins_from(Path::new("skin"), loader);
    }

    /// Load skins from a specific skin root directory.
    pub fn load_all_skins_from(
        &mut self,
        skin_root: &Path,
        loader: &dyn Fn(&Path) -> Vec<SkinHeaderInfo>,
    ) {
        self.all_skins = Vec::new();
        let mut skin_paths: Vec<PathBuf> = Vec::new();
        Self::scan_skins(skin_root, &mut skin_paths);

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
    /// Get a reference to the current player config.
    pub fn player(&self) -> &PlayerConfig {
        &self.player
    }

    /// Get a mutable reference to the current player config.
    pub fn player_mut(&mut self) -> &mut PlayerConfig {
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
                    let index = skin_customize_index(id) + self.custom_option_offset;
                    if let Some(ref mut options) = self.custom_options {
                        let idx = index as usize;
                        if idx < options.len() {
                            let current_value = options[idx].value();
                            let min = options[idx].min();
                            let max = options[idx].max();

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
                    let skin_type = skin_select_type(id);
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
    pub(super) fn ensure_properties(&mut self) -> &mut SkinProperty {
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
    pub(super) fn extract_file_pattern(path: &str) -> String {
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

/// Case-insensitive wildcard matching for skin file patterns (e.g., `*.png`, `bg*.png`).
fn matches_wildcard_case_insensitive(filename: &str, pattern: &str) -> bool {
    let filename_lower = filename.to_ascii_lowercase();
    let pattern_lower = pattern.to_ascii_lowercase();

    if !pattern_lower.contains('*') {
        return filename_lower == pattern_lower;
    }

    let parts: Vec<&str> = pattern_lower.split('*').collect();
    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // First part must be a prefix
            if !filename_lower.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
            // Last part must be a suffix
            if !filename_lower[pos..].ends_with(part) {
                return false;
            }
            pos = filename_lower.len();
        } else if let Some(found) = filename_lower[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
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
        // Call default trait dispose for skin cleanup
        let data = self.main_state_data_mut();
        if let Some(ref mut skin) = data.skin {
            skin.dispose_skin();
        }
        data.skin = None;
    }

    fn render_with_game_context(&mut self, _ctx: &mut GameContext) -> Option<StateTransition> {
        self.render();
        Some(StateTransition::Continue)
    }

    fn input_with_game_context(&mut self, _ctx: &mut GameContext) -> Option<()> {
        self.input();
        Some(())
    }
}

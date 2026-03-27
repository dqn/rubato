// UI item builder logic for SkinConfigurationView.
// Extracted from mod.rs for navigability.
//
// Contains the create() method which builds SkinConfigItem entries
// from a SkinHeader + SkinProperty.

use std::path::PathBuf;

use log::error;

use crate::core::skin_config::{SkinOffset, SkinProperty};
use rubato_skin::skin_header::{CustomItemEnum, SkinHeader};
use rubato_skin::skin_property::OPTION_RANDOM_VALUE;

use super::{SkinConfigItem, SkinConfigurationView};

/// Internal enum for the create() method's item list
enum CreateItem {
    Label(String),
    OptionIdx(usize),
    FileIdx(usize),
    OffsetIdx(usize),
}

impl SkinConfigurationView {
    /// Translates: create(SkinHeader header, SkinConfig.Property property)
    /// Builds the skin configuration UI items.
    pub(super) fn create(&mut self, header: &SkinHeader, property: Option<&SkinProperty>) {
        // selected = header;
        self.selected = Some(header.clone());

        // if(property == null) { property = new SkinConfig.Property(); }
        let default_property = SkinProperty::default();
        let property = property.unwrap_or(&default_property);

        // List items = new ArrayList();
        // List<CustomItem> otheritems = new ArrayList<CustomItem>();
        let mut items: Vec<CreateItem> = Vec::new();
        let mut other_options: Vec<usize> = (0..header.custom_options().len()).collect();
        let mut other_files: Vec<usize> = (0..header.custom_files().len()).collect();
        let mut other_offsets: Vec<usize> = (0..header.custom_offsets().len()).collect();

        // for(CustomCategory category : header.getCustomCategories()) {
        for category in header.custom_categories() {
            // items.add(category.name);
            items.push(CreateItem::Label(category.name.clone()));
            // for(Object item : category.items) { items.add(item); otheritems.remove(item); }
            for cat_item in &category.items {
                match cat_item {
                    CustomItemEnum::Option(opt) => {
                        // Find and remove from other_options
                        if let Some(pos) = other_options
                            .iter()
                            .position(|&i| header.custom_options()[i].name == opt.name)
                        {
                            let idx = other_options.remove(pos);
                            items.push(CreateItem::OptionIdx(idx));
                        }
                    }
                    CustomItemEnum::File(file) => {
                        if let Some(pos) = other_files
                            .iter()
                            .position(|&i| header.custom_files()[i].name == file.name)
                        {
                            let idx = other_files.remove(pos);
                            items.push(CreateItem::FileIdx(idx));
                        }
                    }
                    CustomItemEnum::Offset(offset) => {
                        if let Some(pos) = other_offsets
                            .iter()
                            .position(|&i| header.custom_offsets()[i].name == offset.name)
                        {
                            let idx = other_offsets.remove(pos);
                            items.push(CreateItem::OffsetIdx(idx));
                        }
                    }
                }
            }
            // items.add("");
            items.push(CreateItem::Label(String::new()));
        }

        // if(items.size() > 0 && otheritems.size() > 0) { items.add("Other"); }
        let has_others =
            !other_options.is_empty() || !other_files.is_empty() || !other_offsets.is_empty();
        if !items.is_empty() && has_others {
            items.push(CreateItem::Label("Other".to_string()));
        }
        // items.addAll(otheritems);
        for idx in &other_options {
            items.push(CreateItem::OptionIdx(*idx));
        }
        for idx in &other_files {
            items.push(CreateItem::FileIdx(*idx));
        }
        for idx in &other_offsets {
            items.push(CreateItem::OffsetIdx(*idx));
        }

        // optionbox.clear(); filebox.clear(); offsetbox.clear();
        self.optionbox.clear();
        self.filebox.clear();
        self.offsetbox.clear();
        self.skinconfig_items.clear();

        // for(Object item : items) { ... }
        for item in &items {
            match item {
                CreateItem::OptionIdx(opt_idx) => {
                    self.build_option_item(header, property, *opt_idx);
                }
                CreateItem::FileIdx(file_idx) => {
                    self.build_file_item(header, property, *file_idx);
                }
                CreateItem::OffsetIdx(offset_idx) => {
                    self.build_offset_item(header, property, *offset_idx);
                }
                CreateItem::Label(text) => {
                    self.skinconfig_items
                        .push(SkinConfigItem::Label(text.clone()));
                }
            }
        }
    }

    fn build_option_item(&mut self, header: &SkinHeader, property: &SkinProperty, opt_idx: usize) {
        let option = &header.custom_options()[opt_idx];
        let mut combo_items: Vec<String> = option.contents.clone();
        combo_items.push("Random".to_string());

        let mut selection: usize = 0;
        let mut found_selection: Option<usize> = None;

        for o in property.option.iter().flatten() {
            if o.name.as_deref() == Some(&option.name) {
                let val = o.value;
                if val != OPTION_RANDOM_VALUE {
                    for (index, &opt_val) in option.option.iter().enumerate() {
                        if opt_val == val {
                            found_selection = Some(index);
                            break;
                        }
                    }
                } else {
                    found_selection = Some(combo_items.len() - 1);
                }
                break;
            }
        }

        if found_selection.is_none()
            && let Some(ref def) = option.def
        {
            for (index, content) in option.contents.iter().enumerate() {
                if content == def {
                    found_selection = Some(index);
                }
            }
        }

        if let Some(sel) = found_selection {
            selection = sel;
        }

        let item_idx = self.skinconfig_items.len();
        self.optionbox.insert(option.name.clone(), item_idx);
        self.skinconfig_items.push(SkinConfigItem::Option {
            name: option.name.clone(),
            items: combo_items,
            selected_index: selection,
        });
    }

    fn build_file_item(&mut self, header: &SkinHeader, property: &SkinProperty, file_idx: usize) {
        let file = &header.custom_files()[file_idx];

        let mut name = file
            .path
            .rfind('/')
            .map(|i| &file.path[i + 1..])
            .unwrap_or(&file.path)
            .to_string();

        if file.path.contains('|') {
            let last_pipe = file.path.rfind('|').expect("contains '|'");
            let last_slash = file.path.rfind('/').map(|i| i + 1).unwrap_or(0);
            let first_pipe = file.path.find('|').expect("contains '|'");
            if file.path.len() > last_pipe + 1 {
                name = format!(
                    "{}{}",
                    &file.path[last_slash..first_pipe],
                    &file.path[last_pipe + 1..]
                );
            } else {
                name = file.path[last_slash..first_pipe].to_string();
            }
        }

        let slashindex = file.path.rfind('/');
        let raw_dir = match slashindex {
            Some(idx) => PathBuf::from(&file.path[..idx]),
            None => PathBuf::from("."),
        };
        // Resolve relative custom-file paths from the skin directory,
        // not from the process working directory.
        let dirpath = if raw_dir.is_relative() {
            if let Some(skin_dir) = header.path().and_then(|p| p.parent()) {
                skin_dir.join(&raw_dir)
            } else {
                raw_dir
            }
        } else {
            raw_dir
        };

        if !dirpath.exists() {
            return;
        }

        let mut combo_items: Vec<String> = Vec::new();
        match std::fs::read_dir(&dirpath) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if matches_skin_file_pattern_case_insensitive(&filename, &name) {
                        combo_items.push(filename);
                    }
                }
            }
            Err(e) => {
                error!("Failed to read directory {:?}: {}", dirpath, e);
                return;
            }
        }
        combo_items.push("Random".to_string());

        let mut selection: Option<String> = None;
        for f in property.file.iter().flatten() {
            if f.name.as_deref() == Some(&file.name) {
                selection = f.path.clone();
                break;
            }
        }

        if selection.is_none()
            && let Some(ref def) = file.def
        {
            for filename in &combo_items {
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

        let selected_value = if selection.is_some() {
            selection
        } else if !combo_items.is_empty() {
            Some(combo_items[0].clone())
        } else {
            None
        };

        let item_idx = self.skinconfig_items.len();
        self.filebox.insert(file.name.clone(), item_idx);
        self.skinconfig_items.push(SkinConfigItem::File {
            name: file.name.clone(),
            items: combo_items,
            selected_value,
        });
    }

    fn build_offset_item(
        &mut self,
        header: &SkinHeader,
        property: &SkinProperty,
        offset_idx: usize,
    ) {
        let offset = &header.custom_offsets()[offset_idx];
        let enabled = [
            offset.caps.x,
            offset.caps.y,
            offset.caps.w,
            offset.caps.h,
            offset.caps.r,
            offset.caps.a,
        ];

        let mut found_offset: Option<&SkinOffset> = None;
        for o in &property.offset {
            if let Some(o) = o
                && o.name.as_deref() == Some(&offset.name)
            {
                found_offset = Some(o);
                break;
            }
        }

        let v = if let Some(o) = found_offset {
            [o.x, o.y, o.w, o.h, o.r, o.a]
        } else {
            [0, 0, 0, 0, 0, 0]
        };

        let item_idx = self.skinconfig_items.len();
        self.offsetbox.insert(offset.name.clone(), item_idx);
        self.skinconfig_items.push(SkinConfigItem::Offset {
            name: offset.name.clone(),
            values: v,
            enabled,
        });
    }
}

pub(super) fn matches_skin_file_pattern_case_insensitive(filename: &str, pattern: &str) -> bool {
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

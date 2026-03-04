// SkinHeader.java -> skin_header.rs
// Mechanical line-by-line translation.

use std::path::PathBuf;

use crate::skin_property;
use crate::skin_type::SkinType;
use crate::stubs::{Resolution, SkinConfigOffset};

/// Skin header/metadata
///
/// Translated from SkinHeader.java
#[derive(Clone)]
pub struct SkinHeader {
    /// Skin type constant
    skin_type_id: i32,
    /// Skin file path
    path: Option<PathBuf>,
    /// Skin type enum
    mode: Option<SkinType>,
    /// Skin name
    name: Option<String>,
    /// Skin author
    author: Option<String>,
    /// Custom options
    options: Vec<CustomOption>,
    /// Custom files
    files: Vec<CustomFile>,
    /// Custom offsets
    offsets: Vec<CustomOffset>,
    /// Custom categories
    categories: Vec<CustomCategory>,
    /// Skin resolution
    resolution: Resolution,
    /// Source resolution
    source_resolution: Option<Resolution>,
    /// Destination resolution
    destination_resolution: Option<Resolution>,
}

/// Skin type constants
pub const TYPE_LR2SKIN: i32 = 0;
pub const TYPE_BEATORJASKIN: i32 = 1;

impl Default for SkinHeader {
    fn default() -> Self {
        Self {
            skin_type_id: 0,
            path: None,
            mode: None,
            name: None,
            author: None,
            options: Vec::new(),
            files: Vec::new(),
            offsets: Vec::new(),
            categories: Vec::new(),
            resolution: Resolution {
                width: 640.0,
                height: 480.0,
            },
            source_resolution: None,
            destination_resolution: None,
        }
    }
}

impl SkinHeader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_skin_type(&self) -> Option<&SkinType> {
        self.mode.as_ref()
    }

    pub fn set_skin_type(&mut self, mode: SkinType) {
        self.mode = Some(mode);
    }

    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn get_author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    pub fn set_author(&mut self, author: String) {
        self.author = Some(author);
    }

    pub fn get_custom_options(&self) -> &[CustomOption] {
        &self.options
    }

    pub fn set_custom_options(&mut self, options: Vec<CustomOption>) {
        self.options = options;
    }

    pub fn get_custom_files(&self) -> &[CustomFile] {
        &self.files
    }

    pub fn set_custom_files(&mut self, files: Vec<CustomFile>) {
        self.files = files;
    }

    pub fn get_path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn get_resolution(&self) -> &Resolution {
        &self.resolution
    }

    pub fn set_resolution(&mut self, resolution: Resolution) {
        self.resolution = resolution;
    }

    pub fn get_type(&self) -> i32 {
        self.skin_type_id
    }

    pub fn set_type(&mut self, type_id: i32) {
        self.skin_type_id = type_id;
    }

    pub fn get_custom_offsets(&self) -> &[CustomOffset] {
        &self.offsets
    }

    pub fn set_custom_offsets(&mut self, offsets: Vec<CustomOffset>) {
        self.offsets = offsets;
    }

    pub fn get_custom_categories(&self) -> &[CustomCategory] {
        &self.categories
    }

    pub fn set_custom_categories(&mut self, categories: Vec<CustomCategory>) {
        self.categories = categories;
    }

    pub fn set_skin_config_property(&mut self, property: &SkinConfigProperty) {
        for custom_option in &mut self.options {
            let mut op = custom_option.get_default_option();
            for option in &property.option {
                if option.name == custom_option.name {
                    if option.value != skin_property::OPTION_RANDOM_VALUE {
                        op = option.value;
                    } else if !custom_option.option.is_empty() {
                        let idx =
                            (rand::random::<f64>() * custom_option.option.len() as f64) as usize;
                        op = custom_option.option[idx];
                    }
                    break;
                }
            }
            for i in 0..custom_option.option.len() {
                if custom_option.option[i] == op {
                    custom_option.selected_index = i as i32;
                }
            }
        }

        for custom_file in &mut self.files {
            for file in &property.file {
                if custom_file.name == file.name {
                    if file.path != "Random" {
                        custom_file.filename = Some(file.path.clone());
                    } else {
                        let ext_start = custom_file.path.rfind('*').map(|i| i + 1).unwrap_or(0);
                        let ext;
                        if custom_file.path.contains('|') {
                            let pipe_idx = custom_file
                                .path
                                .rfind('|')
                                .expect("pipe delimiter guaranteed by contains check");
                            if custom_file.path.len() > pipe_idx + 1 {
                                let star_idx =
                                    custom_file.path.rfind('*').map(|i| i + 1).unwrap_or(0);
                                let bar_idx = custom_file
                                    .path
                                    .find('|')
                                    .expect("pipe delimiter guaranteed by contains check");
                                ext = format!(
                                    "{}{}",
                                    &custom_file.path[star_idx..bar_idx],
                                    &custom_file.path[pipe_idx + 1..]
                                );
                            } else {
                                let star_idx =
                                    custom_file.path.rfind('*').map(|i| i + 1).unwrap_or(0);
                                let bar_idx = custom_file
                                    .path
                                    .find('|')
                                    .expect("pipe delimiter guaranteed by contains check");
                                ext = custom_file.path[star_idx..bar_idx].to_string();
                            }
                        } else {
                            ext = custom_file.path[ext_start..].to_string();
                        }
                        let slash_index = custom_file.path.rfind('/');
                        let dir_path = if let Some(idx) = slash_index {
                            &custom_file.path[..idx]
                        } else {
                            &custom_file.path
                        };
                        let dir = std::path::Path::new(dir_path);
                        if dir.exists() && dir.is_dir() {
                            let mut l: Vec<std::path::PathBuf> = Vec::new();
                            if let Ok(entries) = std::fs::read_dir(dir) {
                                for entry in entries.flatten() {
                                    let path = entry.path();
                                    if let Some(path_str) = path.to_str()
                                        && path_str.to_lowercase().ends_with(&ext.to_lowercase())
                                    {
                                        l.push(path);
                                    }
                                }
                            }
                            if !l.is_empty() {
                                let idx = (rand::random::<f64>() * l.len() as f64) as usize;
                                if let Some(filename) = l[idx].file_name() {
                                    custom_file.filename =
                                        Some(filename.to_string_lossy().into_owned());
                                }
                            }
                        }
                    }
                }
            }
        }

        for of in &mut self.offsets {
            let mut off: Option<&SkinConfigOffsetEntry> = None;
            for off2 in &property.offset {
                if off2.name == of.name {
                    off = Some(off2);
                    break;
                }
            }
            if let Some(o) = off {
                of.offset = Some(SkinConfigOffset {
                    name: o.name.clone(),
                    x: o.x as f32,
                    y: o.y as f32,
                    w: o.w as f32,
                    h: o.h as f32,
                    r: o.r as f32,
                    a: o.a as f32,
                    enabled: true,
                });
            } else {
                of.offset = Some(SkinConfigOffset {
                    name: of.name.clone(),
                    ..SkinConfigOffset::default()
                });
            }
        }
    }

    pub fn get_source_resolution(&self) -> &Resolution {
        self.source_resolution.as_ref().unwrap_or(&self.resolution)
    }

    pub fn set_source_resolution(&mut self, source_resolution: Resolution) {
        self.source_resolution = Some(source_resolution);
    }

    pub fn get_destination_resolution(&self) -> &Resolution {
        self.destination_resolution
            .as_ref()
            .unwrap_or(&self.resolution)
    }

    pub fn set_destination_resolution(&mut self, destination_resolution: Resolution) {
        self.destination_resolution = Some(destination_resolution);
    }
}

// ============================================================
// Inner types for SkinHeader
// ============================================================

/// SkinConfig.Property stub (used for setSkinConfigProperty)
pub struct SkinConfigProperty {
    pub option: Vec<SkinConfigOptionEntry>,
    pub file: Vec<SkinConfigFileEntry>,
    pub offset: Vec<SkinConfigOffsetEntry>,
}

impl SkinConfigProperty {
    pub fn get_option(&self) -> &[SkinConfigOptionEntry] {
        &self.option
    }

    pub fn get_file(&self) -> &[SkinConfigFileEntry] {
        &self.file
    }

    pub fn get_offset(&self) -> &[SkinConfigOffsetEntry] {
        &self.offset
    }
}

/// SkinConfig.Option stub
pub struct SkinConfigOptionEntry {
    pub name: String,
    pub value: i32,
}

/// SkinConfig.FilePath stub
pub struct SkinConfigFileEntry {
    pub name: String,
    pub path: String,
}

/// SkinConfig.Offset stub
pub struct SkinConfigOffsetEntry {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub r: i32,
    pub a: i32,
}

/// Custom option (user-selectable option)
#[derive(Clone)]
pub struct CustomOption {
    /// Option name
    pub name: String,
    /// Option IDs
    pub option: Vec<i32>,
    /// Option display names
    pub contents: Vec<String>,
    /// Default option name
    pub def: Option<String>,
    /// Selected index
    pub selected_index: i32,
}

impl CustomOption {
    pub fn new(name: String, option: Vec<i32>, contents: Vec<String>) -> Self {
        Self {
            name,
            option,
            contents,
            def: None,
            selected_index: -1,
        }
    }

    pub fn new_with_def(
        name: String,
        option: Vec<i32>,
        contents: Vec<String>,
        def: String,
    ) -> Self {
        Self {
            name,
            option,
            contents,
            def: Some(def),
            selected_index: -1,
        }
    }

    pub fn get_default_option(&self) -> i32 {
        if let Some(ref def) = self.def {
            for i in 0..self.option.len() {
                if i < self.contents.len() && self.contents[i] == *def {
                    return self.option[i];
                }
            }
        }
        if !self.option.is_empty() {
            self.option[0]
        } else {
            skin_property::OPTION_RANDOM_VALUE
        }
    }

    pub fn get_selected_option(&self) -> i32 {
        if self.selected_index >= 0 && (self.selected_index as usize) < self.option.len() {
            self.option[self.selected_index as usize]
        } else {
            skin_property::OPTION_RANDOM_VALUE
        }
    }
}

/// Custom file (user-selectable file)
#[derive(Clone)]
pub struct CustomFile {
    /// File name
    pub name: String,
    /// File path pattern
    pub path: String,
    /// Default filename
    pub def: Option<String>,
    /// Selected filename
    pub filename: Option<String>,
}

impl CustomFile {
    pub fn new(name: String, path: String, def: Option<String>) -> Self {
        Self {
            name,
            path,
            def,
            filename: None,
        }
    }

    pub fn get_selected_filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }
}

/// Custom offset (user-adjustable offset)
#[derive(Clone)]
pub struct CustomOffset {
    /// Offset name
    pub name: String,
    /// Offset ID
    pub id: i32,
    /// Whether each value can be changed
    pub x: bool,
    pub y: bool,
    pub w: bool,
    pub h: bool,
    pub r: bool,
    pub a: bool,
    /// Offset value
    pub offset: Option<SkinConfigOffset>,
}

impl CustomOffset {
    pub fn new(
        name: String,
        id: i32,
        x: bool,
        y: bool,
        w: bool,
        h: bool,
        r: bool,
        a: bool,
    ) -> Self {
        Self {
            name,
            id,
            x,
            y,
            w,
            h,
            r,
            a,
            offset: None,
        }
    }

    pub fn get_offset(&self) -> Option<&SkinConfigOffset> {
        self.offset.as_ref()
    }
}

/// Custom category
#[derive(Clone)]
pub struct CustomCategory {
    /// Category name
    pub name: String,
    /// Category custom items
    pub items: Vec<CustomItemEnum>,
}

impl CustomCategory {
    pub fn new(name: String, items: Vec<CustomItemEnum>) -> Self {
        Self { name, items }
    }
}

/// Enum to represent the abstract CustomItem hierarchy
#[derive(Clone)]
pub enum CustomItemEnum {
    Option(CustomOption),
    File(CustomFile),
    Offset(CustomOffset),
}

// SkinHeader.java -> skin_header.rs
// Mechanical line-by-line translation.

use std::path::PathBuf;

use rubato_types::offset_capabilities::OffsetCapabilities;

use crate::skin_property;
use crate::stubs::{Resolution, SkinConfigOffset};
use crate::types::skin_type::SkinType;

/// Skin header/metadata
///
/// Translated from SkinHeader.java
#[derive(Clone)]
pub struct SkinHeader {
    /// Skin type constant
    pub skin_type_id: i32,
    /// Skin file path
    path: Option<PathBuf>,
    /// Skin type enum
    mode: Option<SkinType>,
    /// Skin name
    name: Option<String>,
    /// Skin author
    author: Option<String>,
    /// Custom options
    pub options: Vec<CustomOption>,
    /// Custom files
    pub files: Vec<CustomFile>,
    /// Custom offsets
    pub offsets: Vec<CustomOffset>,
    /// Custom categories
    pub categories: Vec<CustomCategory>,
    /// Skin resolution
    pub resolution: Resolution,
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

    pub fn skin_type(&self) -> Option<&SkinType> {
        self.mode.as_ref()
    }

    pub fn set_skin_type(&mut self, mode: SkinType) {
        self.mode = Some(mode);
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    pub fn set_author(&mut self, author: String) {
        self.author = Some(author);
    }

    pub fn custom_options(&self) -> &[CustomOption] {
        &self.options
    }

    pub fn custom_files(&self) -> &[CustomFile] {
        &self.files
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn resolution(&self) -> &Resolution {
        &self.resolution
    }

    pub fn toast_type(&self) -> i32 {
        self.skin_type_id
    }

    pub fn custom_offsets(&self) -> &[CustomOffset] {
        &self.offsets
    }

    pub fn custom_categories(&self) -> &[CustomCategory] {
        &self.categories
    }

    pub fn set_skin_config_property(&mut self, property: &SkinConfigProperty) {
        for custom_option in &mut self.options {
            let mut op = custom_option.default_option();
            for option in &property.option {
                if option.name == custom_option.name {
                    if option.value != skin_property::OPTION_RANDOM_VALUE {
                        op = option.value;
                    } else if !custom_option.option.is_empty() {
                        let idx =
                            (rand::random::<f64>() * custom_option.option.len() as f64) as usize;
                        let idx = idx.min(custom_option.option.len() - 1);
                        op = custom_option.option[idx];
                    }
                    break;
                }
            }
            if let Some(pos) = custom_option.option.iter().position(|&o| o == op) {
                custom_option.selected_index = pos as i32;
            }
        }

        for custom_file in &mut self.files {
            for file in &property.file {
                if custom_file.name == file.name {
                    if file.path != "Random" {
                        custom_file.filename = Some(file.path.clone());
                    } else {
                        let file_pattern = extract_file_pattern(&custom_file.path);
                        let slash_index = custom_file.path.rfind('/');
                        let dir_path = if let Some(idx) = slash_index {
                            &custom_file.path[..idx]
                        } else {
                            "."
                        };
                        let dir = std::path::Path::new(dir_path);
                        if dir.exists() && dir.is_dir() {
                            let mut l: Vec<std::path::PathBuf> = Vec::new();
                            if let Ok(entries) = std::fs::read_dir(dir) {
                                for entry in entries.flatten() {
                                    let path = entry.path();
                                    if let Some(fname) = path.file_name()
                                        && matches_wildcard_case_insensitive(
                                            &fname.to_string_lossy(),
                                            &file_pattern,
                                        )
                                    {
                                        l.push(path);
                                    }
                                }
                            }
                            if !l.is_empty() {
                                let idx = (rand::random::<f64>() * l.len() as f64) as usize;
                                let idx = idx.min(l.len() - 1);
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

    pub fn source_resolution(&self) -> &Resolution {
        self.source_resolution.as_ref().unwrap_or(&self.resolution)
    }

    pub fn set_source_resolution(&mut self, source_resolution: Resolution) {
        self.source_resolution = Some(source_resolution);
    }

    pub fn destination_resolution(&self) -> &Resolution {
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
    pub fn option(&self) -> &[SkinConfigOptionEntry] {
        &self.option
    }

    pub fn file(&self) -> &[SkinConfigFileEntry] {
        &self.file
    }

    pub fn offset(&self) -> &[SkinConfigOffsetEntry] {
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

    pub fn default_option(&self) -> i32 {
        if let Some(ref def) = self.def {
            for (&opt, content) in self.option.iter().zip(self.contents.iter()) {
                if *content == *def {
                    return opt;
                }
            }
        }
        if !self.option.is_empty() {
            self.option[0]
        } else {
            skin_property::OPTION_RANDOM_VALUE
        }
    }

    pub fn selected_option(&self) -> i32 {
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

    pub fn selected_filename(&self) -> Option<&str> {
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
    /// Which offset dimensions can be changed
    pub caps: OffsetCapabilities,
    /// Offset value
    pub offset: Option<SkinConfigOffset>,
}

impl CustomOffset {
    pub fn new(name: String, id: i32, caps: OffsetCapabilities) -> Self {
        Self {
            name,
            id,
            caps,
            offset: None,
        }
    }

    pub fn offset(&self) -> Option<&SkinConfigOffset> {
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

/// Extract filename pattern from a custom file path spec (handles `|` separator).
/// For `bg/*.png` returns `*.png`; for `bg/bg*|.png|.bmp` returns `bg*.png.bmp`.
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
            if !filename_lower.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_file_pattern_with_directory() {
        assert_eq!(extract_file_pattern("bg/*.png"), "*.png");
    }

    #[test]
    fn extract_file_pattern_with_prefix_wildcard() {
        assert_eq!(extract_file_pattern("bg/bg*.png"), "bg*.png");
    }

    #[test]
    fn extract_file_pattern_no_directory() {
        assert_eq!(extract_file_pattern("*.png"), "*.png");
    }

    #[test]
    fn extract_file_pattern_with_pipe() {
        // bg/bg*|.png|.bmp -> bg*.png.bmp
        assert_eq!(extract_file_pattern("bg/bg*|.png|.bmp"), "bg*.bmp");
    }

    #[test]
    fn extract_file_pattern_pipe_no_trailing() {
        assert_eq!(extract_file_pattern("bg/bg*|.png|"), "bg*");
    }

    #[test]
    fn wildcard_star_dot_png_matches_any_png() {
        assert!(matches_wildcard_case_insensitive("background.png", "*.png"));
        assert!(matches_wildcard_case_insensitive("a.png", "*.png"));
        assert!(!matches_wildcard_case_insensitive("a.jpg", "*.png"));
    }

    #[test]
    fn wildcard_prefix_star_suffix() {
        assert!(matches_wildcard_case_insensitive("bg01.png", "bg*.png"));
        assert!(matches_wildcard_case_insensitive("bg.png", "bg*.png"));
        assert!(!matches_wildcard_case_insensitive("other.png", "bg*.png"));
    }

    #[test]
    fn wildcard_case_insensitive() {
        assert!(matches_wildcard_case_insensitive("BG01.PNG", "bg*.png"));
        assert!(matches_wildcard_case_insensitive("Bg01.Png", "bg*.png"));
    }

    #[test]
    fn wildcard_no_star_exact_match() {
        assert!(matches_wildcard_case_insensitive("file.png", "file.png"));
        assert!(matches_wildcard_case_insensitive("FILE.PNG", "file.png"));
        assert!(!matches_wildcard_case_insensitive("file2.png", "file.png"));
    }

    #[test]
    fn wildcard_star_only_matches_everything() {
        assert!(matches_wildcard_case_insensitive("anything.txt", "*"));
    }
}

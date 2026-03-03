use std::path::Path;

use crate::lr2::lr2_skin_loader::LR2SkinLoaderState;
use crate::skin_property::{OFFSET_ALL, OFFSET_JUDGE_1P, OFFSET_JUDGEDETAIL_1P, OFFSET_NOTES_1P};
use crate::stubs::{MainState, Resolution};

/// LR2 skin header loader
///
/// Translated from LR2SkinHeaderLoader.java
/// Loads LR2 skin header files (.lr2skin) to extract skin metadata,
/// custom options, custom files, and custom offsets.
///
/// Custom option definition
#[derive(Clone, Debug)]
pub struct CustomOption {
    pub name: String,
    pub option: Vec<i32>,
    pub contents: Vec<String>,
    selected_option: i32,
}

impl CustomOption {
    pub fn new(name: &str, option: Vec<i32>, contents: Vec<String>) -> Self {
        let selected = option.first().copied().unwrap_or(0);
        Self {
            name: name.to_string(),
            option,
            contents,
            selected_option: selected,
        }
    }

    pub fn get_selected_option(&self) -> i32 {
        self.selected_option
    }

    pub fn set_selected_option(&mut self, value: i32) {
        self.selected_option = value;
    }
}

/// Custom file definition
#[derive(Clone, Debug)]
pub struct CustomFile {
    pub name: String,
    pub path: String,
    pub def: Option<String>,
    selected_filename: Option<String>,
}

impl CustomFile {
    pub fn new(name: &str, path: &str, def: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            def: def.map(|s| s.to_string()),
            selected_filename: None,
        }
    }

    pub fn get_selected_filename(&self) -> Option<&str> {
        self.selected_filename.as_deref()
    }

    pub fn set_selected_filename(&mut self, filename: Option<String>) {
        self.selected_filename = filename;
    }
}

/// Custom offset definition
#[derive(Clone, Debug)]
pub struct CustomOffset {
    pub name: String,
    pub id: i32,
    pub x: bool,
    pub y: bool,
    pub w: bool,
    pub h: bool,
    pub r: bool,
    pub a: bool,
}

impl CustomOffset {
    pub fn new(name: &str, id: i32, x: bool, y: bool, w: bool, h: bool, r: bool, a: bool) -> Self {
        Self {
            name: name.to_string(),
            id,
            x,
            y,
            w,
            h,
            r,
            a,
        }
    }
}

/// Skin header data
#[derive(Clone, Debug, Default)]
pub struct LR2SkinHeaderData {
    pub path: Option<std::path::PathBuf>,
    pub skin_type: Option<crate::skin_type::SkinType>,
    pub name: String,
    pub author: String,
    pub resolution: Option<Resolution>,
    pub custom_options: Vec<CustomOption>,
    pub custom_files: Vec<CustomFile>,
    pub custom_offsets: Vec<CustomOffset>,
}

/// LR2 skin header loader
pub struct LR2SkinHeaderLoader {
    pub header: LR2SkinHeaderData,
    pub files: Vec<CustomFile>,
    pub options: Vec<CustomOption>,
    pub offsets: Vec<CustomOffset>,
    pub skinpath: String,
    pub base: LR2SkinLoaderState,
}

impl LR2SkinHeaderLoader {
    pub fn new(skinpath: &str) -> Self {
        Self {
            header: LR2SkinHeaderData::default(),
            files: Vec::new(),
            options: Vec::new(),
            offsets: Vec::new(),
            skinpath: skinpath.to_string(),
            base: LR2SkinLoaderState::new(),
        }
    }

    pub fn load_skin(
        &mut self,
        f: &Path,
        _state: Option<&dyn MainState>,
    ) -> anyhow::Result<LR2SkinHeaderData> {
        self.header = LR2SkinHeaderData::default();
        self.files.clear();
        self.options.clear();
        self.offsets.clear();

        self.header.path = Some(f.to_path_buf());

        let raw_bytes = std::fs::read(f)?;
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
        let content = decoded.into_owned();

        for line in content.lines() {
            if let Some((cmd, str_parts)) = self.base.process_line_directives(line, _state) {
                self.process_header_command(&cmd, &str_parts);
            }
        }

        self.header.custom_options = self.options.clone();
        self.header.custom_files = self.files.clone();
        self.header.custom_offsets = self.offsets.clone();

        // Set up options in op map
        for option in &self.header.custom_options {
            for i in 0..option.option.len() {
                let val = if option.get_selected_option() == option.option[i] {
                    1
                } else {
                    0
                };
                self.base.op.insert(option.option[i], val);
            }
        }

        Ok(self.header.clone())
    }

    fn process_header_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            "INFORMATION" => {
                if str_parts.len() >= 4 {
                    if let Ok(type_id) = str_parts[1].trim().parse::<i32>() {
                        self.header.skin_type =
                            crate::skin_type::SkinType::get_skin_type_by_id(type_id);
                    }
                    self.header.name = str_parts[2].clone();
                    self.header.author = str_parts[3].clone();

                    // Add default options for play skin types
                    if let Some(
                        crate::skin_type::SkinType::Play5Keys
                        | crate::skin_type::SkinType::Play7Keys
                        | crate::skin_type::SkinType::Play9Keys
                        | crate::skin_type::SkinType::Play10Keys
                        | crate::skin_type::SkinType::Play14Keys
                        | crate::skin_type::SkinType::Play24Keys
                        | crate::skin_type::SkinType::Play24KeysDouble,
                    ) = self.header.skin_type
                    {
                        self.options.push(CustomOption::new(
                            "BGA Size",
                            vec![30, 31],
                            vec!["Normal".to_string(), "Extend".to_string()],
                        ));
                        self.options.push(CustomOption::new(
                            "Ghost",
                            vec![34, 35, 36, 37],
                            vec![
                                "Off".to_string(),
                                "Type A".to_string(),
                                "Type B".to_string(),
                                "Type C".to_string(),
                            ],
                        ));
                        self.options.push(CustomOption::new(
                            "Score Graph",
                            vec![38, 39],
                            vec!["Off".to_string(), "On".to_string()],
                        ));
                        self.options.push(CustomOption::new(
                            "Judge Detail",
                            vec![1997, 1998, 1999],
                            vec![
                                "Off".to_string(),
                                "EARLY/LATE".to_string(),
                                "+-ms".to_string(),
                            ],
                        ));

                        self.offsets.push(CustomOffset::new(
                            "All offset(%)",
                            OFFSET_ALL,
                            true,
                            true,
                            true,
                            true,
                            false,
                            false,
                        ));
                        self.offsets.push(CustomOffset::new(
                            "Notes offset",
                            OFFSET_NOTES_1P,
                            false,
                            false,
                            false,
                            true,
                            false,
                            false,
                        ));
                        self.offsets.push(CustomOffset::new(
                            "Judge offset",
                            OFFSET_JUDGE_1P,
                            true,
                            true,
                            true,
                            true,
                            false,
                            true,
                        ));
                        self.offsets.push(CustomOffset::new(
                            "Judge Detail offset",
                            OFFSET_JUDGEDETAIL_1P,
                            true,
                            true,
                            true,
                            true,
                            false,
                            true,
                        ));
                    }
                }
            }
            "RESOLUTION" => {
                if str_parts.len() > 1 {
                    let res_values = [
                        Resolution {
                            width: 640.0,
                            height: 480.0,
                        }, // SD
                        Resolution {
                            width: 1280.0,
                            height: 720.0,
                        }, // HD
                        Resolution {
                            width: 1920.0,
                            height: 1080.0,
                        }, // FULLHD
                        Resolution {
                            width: 3840.0,
                            height: 2160.0,
                        }, // ULTRAHD
                    ];
                    if let Ok(idx) = str_parts[1].trim().parse::<usize>()
                        && idx < res_values.len()
                    {
                        self.header.resolution = Some(res_values[idx].clone());
                    }
                }
            }
            "CUSTOMOPTION" => {
                if str_parts.len() >= 3 {
                    let mut contents: Vec<String> = Vec::new();
                    for i in 3..str_parts.len() {
                        if !str_parts[i].is_empty() {
                            contents.push(str_parts[i].clone());
                        }
                    }
                    let base_op: i32 = str_parts[2].trim().parse().unwrap_or(0);
                    let mut op = vec![0i32; contents.len()];
                    for i in 0..op.len() {
                        op[i] = base_op + i as i32;
                    }
                    self.options
                        .push(CustomOption::new(&str_parts[1], op, contents));
                }
            }
            "CUSTOMFILE" => {
                if str_parts.len() >= 3 {
                    let path = str_parts[2]
                        .replace("LR2files\\Theme", &self.skinpath)
                        .replace('\\', "/");
                    let def = if str_parts.len() >= 4 {
                        Some(str_parts[3].as_str())
                    } else {
                        None
                    };
                    self.files.push(CustomFile::new(&str_parts[1], &path, def));
                }
            }
            "CUSTOMOFFSET" => {
                if str_parts.len() >= 3 {
                    let mut op = [true; 6];
                    for i in 0..6 {
                        if i + 3 < str_parts.len()
                            && let Ok(v) = str_parts[i + 3].trim().parse::<i32>()
                        {
                            op[i] = v > 0;
                        }
                    }
                    let id: i32 = str_parts[2].trim().parse().unwrap_or(0);
                    self.offsets.push(CustomOffset::new(
                        &str_parts[1],
                        id,
                        op[0],
                        op[1],
                        op[2],
                        op[3],
                        op[4],
                        op[5],
                    ));
                }
            }
            "CUSTOMOPTION_ADDITION_SETTING" => {
                // #CUSTOMOPTION_ADDITION_SETTING, BGA Size, Ghost, Score Graph, Judge Detail
                // 0 = No Add, 1 = Add
                let addition_names = ["BGA Size", "Ghost", "Score Graph", "Judge Detail"];
                let mut addition_indices: [Option<usize>; 4] = [None; 4];
                for (idx, co) in self.options.iter().enumerate() {
                    for (i, name) in addition_names.iter().enumerate() {
                        if co.name == *name {
                            addition_indices[i] = Some(idx);
                        }
                    }
                }
                // Remove in reverse order to maintain indices
                let mut to_remove: Vec<usize> = Vec::new();
                for i in 0..addition_names.len() {
                    if i + 1 < str_parts.len() {
                        let cleaned: String = str_parts[i + 1]
                            .chars()
                            .filter(|c| c.is_ascii_digit() || *c == '-')
                            .collect();
                        if cleaned == "0"
                            && let Some(idx) = addition_indices[i]
                        {
                            to_remove.push(idx);
                        }
                    }
                }
                to_remove.sort_unstable();
                to_remove.dedup();
                for idx in to_remove.into_iter().rev() {
                    if idx < self.options.len() {
                        self.options.remove(idx);
                    }
                }
            }
            "INCLUDE" => {
                // No-op in header loader
            }
            _ => {}
        }
    }
}

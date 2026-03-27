// SkinConfigurationView.java -> skin_configuration_view.rs
// Mechanical line-by-line translation.

mod builder;
mod header_loader;
mod render;

pub use header_loader::load_skin_header;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::core::config::Config;
use crate::core::player_config::PlayerConfig;
use crate::core::skin_config::{SkinConfig, SkinFilePath, SkinOffset, SkinOption, SkinProperty};
use rubato_skin::skin_header::SkinHeader;
use rubato_skin::skin_property::OPTION_RANDOM_VALUE;
use rubato_skin::skin_type::SkinType;

/// Skin configuration item for egui rendering
/// Translates the dynamic VBox content built by create() method.
#[derive(Clone, Debug)]
pub enum SkinConfigItem {
    /// Category label or separator
    Label(String),
    /// CustomOption with combo box selection index
    Option {
        name: String,
        items: Vec<String>,
        selected_index: usize,
    },
    /// CustomFile with combo box selection
    File {
        name: String,
        items: Vec<String>,
        selected_value: Option<String>,
    },
    /// CustomOffset with spinner values
    Offset {
        name: String,
        values: [i32; 6],
        enabled: [bool; 6],
    },
}

/// SkinConfigurationView - skin configuration UI
/// Translates: SkinConfigurationView (JavaFX -> egui)
///
/// Skin type selection, skin file selection, custom options/files/offsets configuration.
pub struct SkinConfigurationView {
    // @FXML private ComboBox<SkinType> skintypeSelector;
    skintype_selector: Option<SkinType>,

    // @FXML private ComboBox<SkinHeader> skinheaderSelector;
    skinheader_selector: Option<usize>,

    // @FXML private ScrollPane skinconfig;
    skinconfig_items: Vec<SkinConfigItem>,

    // SkinType mode;
    mode: Option<SkinType>,

    // SkinHeader selected;
    selected: Option<SkinHeader>,

    // PlayerConfig player;
    player: Option<PlayerConfig>,

    // SkinHeader[] skinheader;
    skinheader: Vec<SkinHeader>,

    // Map<CustomOption, ComboBox<String>> optionbox = new HashMap<>();
    optionbox: HashMap<String, usize>,

    // Map<CustomFile, ComboBox<String>> filebox = new HashMap<>();
    filebox: HashMap<String, usize>,

    // Map<CustomOffset, Spinner[]> offsetbox = new HashMap<>();
    offsetbox: HashMap<String, usize>,

    // Current headers for the selected skin type
    current_headers: Vec<SkinHeader>,
}

impl Default for SkinConfigurationView {
    fn default() -> Self {
        Self::new()
    }
}

impl SkinConfigurationView {
    pub fn new() -> Self {
        Self {
            skintype_selector: None,
            skinheader_selector: None,
            skinconfig_items: Vec::new(),
            mode: None,
            selected: None,
            player: None,
            skinheader: Vec::new(),
            optionbox: HashMap::new(),
            filebox: HashMap::new(),
            offsetbox: HashMap::new(),
            current_headers: Vec::new(),
        }
    }

    // ---- Accessors for egui integration ----

    pub fn skintype_selector(&self) -> Option<SkinType> {
        self.skintype_selector
    }

    pub fn set_skintype_selector(&mut self, skin_type: SkinType) {
        self.skintype_selector = Some(skin_type);
    }

    pub fn current_headers(&self) -> &[SkinHeader] {
        &self.current_headers
    }

    pub fn skinheader_selector(&self) -> Option<usize> {
        self.skinheader_selector
    }

    pub fn set_skinheader_selector(&mut self, index: usize) {
        self.skinheader_selector = Some(index);
    }

    pub fn skinconfig_items_mut(&mut self) -> &mut Vec<SkinConfigItem> {
        &mut self.skinconfig_items
    }

    pub fn player(&self) -> Option<&PlayerConfig> {
        self.player.as_ref()
    }

    /// Translates: initialize(URL, ResourceBundle)
    pub fn initialize(&mut self) {
        // skintypeSelector.setCellFactory((param) -> new SkinTypeCell());
        // skintypeSelector.setButtonCell(new SkinTypeCell());
        // skintypeSelector.getItems().addAll(SkinType.values());
        // → SkinType values available via SkinType::values(), cell rendering deferred to egui.

        // skinheaderSelector.setCellFactory((param) -> new SkinListCell());
        // skinheaderSelector.setButtonCell(new SkinListCell());
        // → cell rendering deferred to egui.
    }

    /// Translates: getSelectedHeader()
    pub fn selected_header(&self) -> Option<&SkinHeader> {
        self.selected.as_ref()
    }

    /// Translates: getProperty()
    /// Reads current UI state and returns a SkinProperty.
    pub fn property(&self) -> SkinProperty {
        // SkinConfig.Property property = new SkinConfig.Property();
        let mut property = SkinProperty::default();

        let selected = match &self.selected {
            Some(s) => s,
            None => return property,
        };

        // List<SkinConfig.Option> options = new ArrayList<>();
        let mut options: Vec<Option<SkinOption>> = Vec::new();
        // for (CustomOption option : selected.getCustomOptions()) {
        for option in selected.custom_options() {
            // if (optionbox.get(option) != null) {
            if let Some(&item_idx) = self.optionbox.get(&option.name)
                && let Some(SkinConfigItem::Option {
                    selected_index,
                    items,
                    ..
                }) = self.skinconfig_items.get(item_idx)
            {
                let index = *selected_index;
                let o_value = if index != items.len().saturating_sub(1) {
                    if index < option.option.len() {
                        option.option[index]
                    } else {
                        0
                    }
                } else {
                    OPTION_RANDOM_VALUE
                };
                let o = SkinOption {
                    name: Some(option.name.clone()),
                    value: o_value,
                };
                options.push(Some(o));
            }
        }
        property.option = options;

        // List<SkinConfig.FilePath> files = new ArrayList<>();
        let mut files: Vec<Option<SkinFilePath>> = Vec::new();
        for file in selected.custom_files() {
            if let Some(&item_idx) = self.filebox.get(&file.name)
                && let Some(SkinConfigItem::File { selected_value, .. }) =
                    self.skinconfig_items.get(item_idx)
            {
                let o = SkinFilePath {
                    name: Some(file.name.clone()),
                    path: selected_value.clone(),
                };
                files.push(Some(o));
            }
        }
        property.file = files;

        // List<SkinConfig.Offset> offsets = new ArrayList<>();
        let mut offsets: Vec<Option<SkinOffset>> = Vec::new();
        for offset in selected.custom_offsets() {
            if let Some(&item_idx) = self.offsetbox.get(&offset.name)
                && let Some(SkinConfigItem::Offset { values, .. }) =
                    self.skinconfig_items.get(item_idx)
            {
                let o = SkinOffset {
                    name: Some(offset.name.clone()),
                    x: values[0],
                    y: values[1],
                    w: values[2],
                    h: values[3],
                    r: values[4],
                    a: values[5],
                };
                offsets.push(Some(o));
            }
        }
        property.offset = offsets;

        property
    }

    /// Translates: getSkinHeader(SkinType mode)
    pub fn skin_header(&self, mode: &SkinType) -> Vec<&SkinHeader> {
        self.skinheader
            .iter()
            .filter(|header| header.skin_type() == Some(mode))
            .collect()
    }

    /// Translates: changeSkinType()
    pub fn change_skin_type(&mut self) {
        self.commit_skin_type();
        if let Some(skin_type) = self.skintype_selector {
            self.update_skin_type(&skin_type);
        }
    }

    /// Translates: updateSkinType(SkinType type)
    pub fn update_skin_type(&mut self, skin_type: &SkinType) {
        self.mode = Some(*skin_type);

        self.current_headers.clear();
        let headers: Vec<SkinHeader> = self
            .skinheader
            .iter()
            .filter(|h| h.skin_type() == Some(skin_type))
            .cloned()
            .collect();
        self.current_headers = headers;

        if let Some(ref player) = self.player {
            let type_id = skin_type.id() as usize;
            if type_id < player.skin.len()
                && let Some(ref skinconf) = player.skin[type_id]
            {
                let mut found = false;
                for (i, header) in self.current_headers.iter().enumerate() {
                    if let (Some(header_path), Some(skin_path)) = (header.path(), &skinconf.path)
                        && header_path == &PathBuf::from(skin_path)
                    {
                        self.skinheader_selector = Some(i);
                        let header_clone = header.clone();
                        let props = skinconf.properties.clone();
                        self.create(&header_clone, props.as_ref());
                        found = true;
                        break;
                    }
                }
                if !found {
                    self.skinheader_selector = if self.current_headers.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                }
            }
        }
    }

    /// Translates: commitSkinType()
    pub fn commit_skin_type(&mut self) {
        if self.player.is_none() {
            return;
        }

        if let Some(selected) = self.selected.clone() {
            let path_str: String = selected
                .path()
                .map(|p: &PathBuf| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let mut skin = SkinConfig::new_with_path(&path_str);
            skin.properties = Some(self.property());
            if let Some(skin_type) = selected.skin_type() {
                let type_id = skin_type.id() as usize;
                let player = self.player.as_mut().expect("player is Some");
                while player.skin.len() <= type_id {
                    player.skin.push(None);
                }
                player.skin[type_id] = Some(skin);
            }
        } else if let Some(mode) = self.mode {
            let type_id = mode.id() as usize;
            let player = self.player.as_mut().expect("player is Some");
            if type_id < player.skin.len() {
                player.skin[type_id] = None;
            }
        }
    }

    /// Translates: update(Config config)
    /// Scans for skin files in the configured skin path.
    pub fn update_config(&mut self, config: &Config) {
        let mut skinpaths: Vec<PathBuf> = Vec::new();
        Self::scan(&PathBuf::from(&config.paths.skinpath), &mut skinpaths);

        for path in &skinpaths {
            if let Some(header) = load_skin_header(path, config) {
                // 7/14key skinは5/10keyにも加える (add 7/14key skins as 5/10key too)
                if header.toast_type() == rubato_skin::skin_header::TYPE_LR2SKIN
                    && let Some(skin_type) = header.skin_type()
                    && (*skin_type == SkinType::Play7Keys || *skin_type == SkinType::Play14Keys)
                    && let Some(mut variant) = load_skin_header(path, config)
                {
                    let variant_type = *variant.skin_type().expect("skin_type");
                    if variant_type == SkinType::Play7Keys {
                        let name = variant.name().unwrap_or("").to_string();
                        if !name.to_lowercase().contains("7key") {
                            variant.set_name(format!("{} (7KEYS) ", name));
                        }
                        variant.set_skin_type(SkinType::Play5Keys);
                    } else if variant_type == SkinType::Play14Keys {
                        let name = variant.name().unwrap_or("").to_string();
                        if !name.to_lowercase().contains("14key") {
                            variant.set_name(format!("{} (14KEYS) ", name));
                        }
                        variant.set_skin_type(SkinType::Play10Keys);
                    }
                    self.skinheader.push(variant);
                }
                self.skinheader.push(header);
            }
        }
    }

    /// Translates: scan(Path p, List<Path> paths)
    /// Recursively scans for skin definition files.
    fn scan(p: &Path, paths: &mut Vec<PathBuf>) {
        if p.is_dir() {
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    Self::scan(&entry.path(), paths);
                }
            }
        } else if let Some(filename) = p.file_name() {
            let lower = filename.to_string_lossy().to_lowercase();
            if lower.ends_with(".lr2skin")
                || lower.ends_with(".luaskin")
                || lower.ends_with(".json")
            {
                paths.push(p.to_path_buf());
            }
        }
    }

    /// Translates: update(PlayerConfig player)
    pub fn update_player(&mut self, player: &PlayerConfig) {
        self.player = Some(player.clone());
        self.skintype_selector = Some(SkinType::Play7Keys);
        self.update_skin_type(&SkinType::Play7Keys);
    }

    /// Translates: commit()
    pub fn commit(&mut self) {
        self.commit_skin_type();
        self.commit_skin_header();
    }

    /// Translates: changeSkinHeader()
    pub fn change_skin_header(&mut self) {
        self.commit_skin_header();
        let header = self
            .skinheader_selector
            .and_then(|i| self.current_headers.get(i))
            .cloned();
        self.update_skin_header(header.as_ref());
    }

    /// Translates: updateSkinHeader(SkinHeader header)
    pub fn update_skin_header(&mut self, header: Option<&SkinHeader>) {
        let mut property: Option<SkinProperty> = None;
        if let Some(header) = header
            && let Some(ref player) = self.player
        {
            for skinc in &player.skin_history {
                if let (Some(skin_path), Some(header_path)) = (&skinc.path, header.path())
                    && skin_path == &header_path.to_string_lossy().to_string()
                {
                    property = skinc.properties.clone();
                    break;
                }
            }
        }
        if let Some(header) = header {
            let header_clone = header.clone();
            self.create(&header_clone, property.as_ref());
        } else {
            self.selected = None;
            self.skinconfig_items.clear();
            self.optionbox.clear();
            self.filebox.clear();
            self.offsetbox.clear();
        }
    }

    /// Translates: commitSkinHeader()
    /// Saves current skin config to skin history.
    pub fn commit_skin_header(&mut self) {
        let selected = match self.selected.clone() {
            Some(s) => s,
            None => return,
        };

        let property = self.property();

        let player = match self.player.as_mut() {
            Some(p) => p,
            None => return,
        };

        let mut index: Option<usize> = None;
        for (i, history_entry) in player.skin_history.iter().enumerate() {
            let sel_path_str: Option<String> = selected
                .path()
                .map(|p: &PathBuf| p.to_string_lossy().to_string());
            if let (Some(hist_path), Some(sel_path)) = (&history_entry.path, &sel_path_str)
                && hist_path == sel_path
            {
                index = Some(i);
                break;
            }
        }

        let sc = SkinConfig {
            path: selected
                .path()
                .map(|p: &PathBuf| p.to_string_lossy().to_string()),
            properties: Some(property),
        };

        if let Some(idx) = index {
            player.skin_history[idx] = sc;
        } else {
            player.skin_history.push(sc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::header_loader::convert_lr2_header_data;
    use super::*;
    use rubato_skin::skin_header::CustomFile;
    use rubato_skin::skin_header::TYPE_LR2SKIN;

    /// Helper to get the path to the test skin directory
    fn test_skin_dir() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir.join("../../skin/default")
    }

    #[test]
    fn load_skin_header_json_returns_header_with_name_and_type() {
        // Create a minimal valid JSON skin file
        let tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        let json = r#"{"type":0,"name":"Test Skin","w":1280,"h":720}"#;
        std::fs::write(tmp.path(), json).unwrap();

        let config = Config::default();
        let header = load_skin_header(tmp.path(), &config);

        assert!(
            header.is_some(),
            "JSON skin header should be loaded from {:?}",
            tmp.path()
        );
        let header = header.unwrap();
        assert_eq!(header.name(), Some("Test Skin"));
        assert_eq!(header.skin_type(), Some(&SkinType::Play7Keys));
    }

    #[test]
    fn load_skin_header_json_real_play7_file() {
        let skin_dir = test_skin_dir();
        let json_path = skin_dir.join("play7.json");
        if !json_path.exists() {
            return;
        }
        let json_path = json_path.canonicalize().unwrap();

        let config = Config::default();
        let header = load_skin_header(&json_path, &config);

        if let Some(header) = header {
            assert!(header.name().is_some());
            assert!(header.skin_type().is_some());
        }
    }

    #[test]
    fn load_skin_header_luaskin_returns_header_with_name_and_type() {
        let skin_dir = test_skin_dir();
        let lua_path = skin_dir.join("decide/decide.luaskin");
        if !lua_path.exists() {
            return;
        }

        let config = Config::default();
        let header = load_skin_header(&lua_path, &config);

        assert!(header.is_some(), "Lua skin header should be loaded");
        let header = header.unwrap();
        assert!(header.name().is_some(), "Loaded header should have a name");
    }

    #[test]
    fn load_skin_header_invalid_json_returns_none() {
        let tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        std::fs::write(tmp.path(), "this is not valid json").unwrap();

        let config = Config::default();
        let header = load_skin_header(tmp.path(), &config);

        assert!(header.is_none(), "Invalid JSON should return None");
    }

    #[test]
    fn load_skin_header_nonexistent_path_returns_none() {
        let config = Config::default();
        let header = load_skin_header(Path::new("/nonexistent/path/skin.json"), &config);

        assert!(header.is_none(), "Non-existent path should return None");
    }

    #[test]
    fn update_config_loads_json_skin_headers_from_directory() {
        let skin_dir = test_skin_dir();
        if !skin_dir.exists() {
            return;
        }

        let config = Config {
            paths: crate::core::config::PathConfig {
                skinpath: skin_dir.to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut view = SkinConfigurationView::new();
        view.update_config(&config);

        assert!(
            !view.skinheader.is_empty(),
            "update_config should load at least one skin header from the default skin directory"
        );
    }

    #[test]
    fn update_config_skin_headers_have_valid_skin_types() {
        let skin_dir = test_skin_dir();
        if !skin_dir.exists() {
            return;
        }

        let config = Config {
            paths: crate::core::config::PathConfig {
                skinpath: skin_dir.to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut view = SkinConfigurationView::new();
        view.update_config(&config);

        let headers_with_types: Vec<_> = view
            .skinheader
            .iter()
            .filter(|h| h.skin_type().is_some())
            .collect();
        assert!(
            !headers_with_types.is_empty(),
            "At least some headers should have valid skin types"
        );
    }

    #[test]
    fn update_config_empty_directory_loads_no_headers() {
        let tmp_dir = tempfile::tempdir().unwrap();

        let config = Config {
            paths: crate::core::config::PathConfig {
                skinpath: tmp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut view = SkinConfigurationView::new();
        view.update_config(&config);

        assert!(
            view.skinheader.is_empty(),
            "Empty directory should yield no skin headers"
        );
    }

    #[test]
    fn update_config_filters_headers_by_skin_type() {
        let skin_dir = test_skin_dir();
        if !skin_dir.exists() {
            return;
        }

        let config = Config {
            paths: crate::core::config::PathConfig {
                skinpath: skin_dir.to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut view = SkinConfigurationView::new();
        view.update_config(&config);

        let play7_headers = view.skin_header(&SkinType::Play7Keys);
        for header in &play7_headers {
            assert_eq!(
                header.skin_type(),
                Some(&SkinType::Play7Keys),
                "Filtered headers should have the correct skin type"
            );
        }
    }

    #[test]
    fn convert_lr2_header_data_sets_type_lr2skin() {
        use rubato_skin::lr2::lr2_skin_header_loader::LR2SkinHeaderData;

        let lr2_data = LR2SkinHeaderData {
            name: "Test LR2 Skin".to_string(),
            skin_type: Some(SkinType::Play7Keys),
            path: Some(PathBuf::from("/test/skin.lr2skin")),
            ..Default::default()
        };

        let header = convert_lr2_header_data(&lr2_data);

        assert_eq!(header.toast_type(), TYPE_LR2SKIN);
        assert_eq!(header.name(), Some("Test LR2 Skin"));
        assert_eq!(header.skin_type(), Some(&SkinType::Play7Keys));
        assert_eq!(header.path(), Some(&PathBuf::from("/test/skin.lr2skin")));
    }

    #[test]
    fn convert_lr2_header_data_converts_custom_options() {
        use rubato_skin::lr2::lr2_skin_header_loader::{
            CustomOption as LR2CustomOption, LR2SkinHeaderData,
        };

        let lr2_data = LR2SkinHeaderData {
            custom_options: vec![LR2CustomOption::new(
                "BGA Size",
                vec![30, 31],
                vec!["Normal".to_string(), "Extend".to_string()],
            )],
            ..Default::default()
        };

        let header = convert_lr2_header_data(&lr2_data);

        assert_eq!(header.custom_options().len(), 1);
        assert_eq!(header.custom_options()[0].name, "BGA Size");
        assert_eq!(header.custom_options()[0].option, vec![30, 31]);
    }

    #[test]
    fn convert_lr2_header_data_converts_custom_files() {
        use rubato_skin::lr2::lr2_skin_header_loader::{
            CustomFile as LR2CustomFile, LR2SkinHeaderData,
        };

        let lr2_data = LR2SkinHeaderData {
            custom_files: vec![LR2CustomFile::new(
                "Lane",
                "skin/lane/*.png",
                Some("default"),
            )],
            ..Default::default()
        };

        let header = convert_lr2_header_data(&lr2_data);

        assert_eq!(header.custom_files().len(), 1);
        assert_eq!(header.custom_files()[0].name, "Lane");
        assert_eq!(header.custom_files()[0].path, "skin/lane/*.png");
        assert_eq!(header.custom_files()[0].def, Some("default".to_string()));
    }

    #[test]
    fn convert_lr2_header_data_converts_custom_offsets() {
        use rubato_skin::lr2::lr2_skin_header_loader::{
            CustomOffset as LR2CustomOffset, LR2SkinHeaderData,
        };

        let lr2_data = LR2SkinHeaderData {
            custom_offsets: vec![LR2CustomOffset::new(
                "All offset(%)",
                0,
                rubato_types::offset_capabilities::OffsetCapabilities {
                    x: true,
                    y: true,
                    w: true,
                    h: true,
                    ..Default::default()
                },
            )],
            ..Default::default()
        };

        let header = convert_lr2_header_data(&lr2_data);

        assert_eq!(header.custom_offsets().len(), 1);
        assert_eq!(header.custom_offsets()[0].name, "All offset(%)");
        assert_eq!(header.custom_offsets()[0].id, 0);
        assert!(header.custom_offsets()[0].caps.x);
        assert!(header.custom_offsets()[0].caps.y);
        assert!(!header.custom_offsets()[0].caps.r);
    }

    #[test]
    fn load_skin_header_lr2_with_temp_file() {
        // Create a minimal LR2 skin file
        let tmp = tempfile::NamedTempFile::with_suffix(".lr2skin").unwrap();
        let content = "#INFORMATION,0,Test LR2,Author,\n";
        let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(content);
        std::fs::write(tmp.path(), encoded.as_ref()).unwrap();

        let config = Config::default();
        let header = load_skin_header(tmp.path(), &config);

        assert!(header.is_some(), "LR2 skin header should be loaded");
        let header = header.unwrap();
        assert_eq!(header.toast_type(), TYPE_LR2SKIN);
        assert_eq!(header.name(), Some("Test LR2"));
    }

    #[test]
    fn create_file_item_matches_wildcard_paths_case_insensitively() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let lane_dir = tmp_dir.path().join("lane");
        std::fs::create_dir_all(&lane_dir).unwrap();
        std::fs::write(lane_dir.join("lane_default.png"), []).unwrap();
        std::fs::write(lane_dir.join("LANE_ALT.PNG"), []).unwrap();
        std::fs::write(lane_dir.join("notes.txt"), []).unwrap();

        let mut header = SkinHeader::new();
        header.files = vec![CustomFile::new(
            "Lane".to_string(),
            format!("{}/lane*.png", lane_dir.to_string_lossy()),
            Some("lane_default".to_string()),
        )];

        let mut view = SkinConfigurationView::new();
        view.create(&header, None);

        let file_item = view.skinconfig_items.iter().find_map(|item| match item {
            SkinConfigItem::File {
                name,
                items,
                selected_value,
            } if name == "Lane" => Some((items, selected_value)),
            _ => None,
        });

        let (items, selected_value) = file_item.expect("custom file item should be created");
        assert!(
            items.iter().any(|item| item == "lane_default.png"),
            "wildcard file list should include the lowercase match"
        );
        assert!(
            items.iter().any(|item| item == "LANE_ALT.PNG"),
            "wildcard file list should include the uppercase match"
        );
        assert!(
            items.iter().any(|item| item == "Random"),
            "file list should keep the Random fallback"
        );
        assert_eq!(
            selected_value.as_deref(),
            Some("lane_default.png"),
            "default selection should resolve to the wildcard-matched file"
        );
    }
}

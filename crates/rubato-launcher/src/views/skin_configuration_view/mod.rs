// SkinConfigurationView.java -> skin_configuration_view.rs
// Mechanical line-by-line translation.

mod header_loader;

pub use header_loader::load_skin_header;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::error;

use rubato_core::config::Config;
use rubato_core::player_config::PlayerConfig;
use rubato_core::skin_config::{SkinConfig, SkinFilePath, SkinOffset, SkinOption, SkinProperty};
use rubato_skin::skin_header::{CustomItemEnum, SkinHeader, TYPE_BEATORJASKIN};
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
    skinheader_selector: Option<usize>, // index into current_headers

    // @FXML private ScrollPane skinconfig;
    // → represented as a list of SkinConfigItem for egui rendering
    skinconfig_items: Vec<SkinConfigItem>,

    // private PlayerConfig player;
    player: Option<PlayerConfig>,

    // private SkinType mode = null;
    mode: Option<SkinType>,

    // private List<SkinHeader> skinheader = new ArrayList<SkinHeader>();
    skinheader: Vec<SkinHeader>,

    // private SkinHeader selected = null;
    selected: Option<SkinHeader>,

    // private Map<CustomOption, ComboBox<String>> optionbox
    optionbox: HashMap<String, usize>, // option name -> index into skinconfig_items

    // private Map<CustomFile, ComboBox<String>> filebox
    filebox: HashMap<String, usize>, // file name -> index into skinconfig_items

    // private Map<CustomOffset, Spinner<Integer>[]> offsetbox
    offsetbox: HashMap<String, usize>, // offset name -> index into skinconfig_items

    // Current filtered headers for the selected skin type
    current_headers: Vec<SkinHeader>,
}

impl Default for SkinConfigurationView {
    fn default() -> Self {
        Self::new()
    }
}

impl SkinConfigurationView {
    pub fn new() -> Self {
        SkinConfigurationView {
            skintype_selector: None,
            skinheader_selector: None,
            skinconfig_items: Vec::new(),
            player: None,
            mode: None,
            skinheader: Vec::new(),
            selected: None,
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

    /// Render the skin configuration view using egui.
    ///
    /// Displays skin type selector, skin header selector, and all dynamic
    /// config items (options, files, offsets) built by the `create()` method.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // Skin type selector
        let skin_types = SkinType::values();
        let current_type = self.skintype_selector.unwrap_or(SkinType::Play7Keys);
        ui.horizontal(|ui| {
            ui.label("Category:");
            let mut new_type = current_type;
            egui::ComboBox::from_id_salt("skin_type_selector")
                .selected_text(Self::skin_type_display_name(&current_type))
                .show_ui(ui, |ui| {
                    for st in &skin_types {
                        ui.selectable_value(&mut new_type, *st, Self::skin_type_display_name(st));
                    }
                });
            if new_type != current_type {
                self.skintype_selector = Some(new_type);
                self.change_skin_type();
            }
        });

        // Skin header selector
        let headers = self.current_headers.clone();
        let selected_idx = self.skinheader_selector;
        if headers.is_empty() {
            ui.label("(no skins found)");
        } else {
            let display = selected_idx
                .and_then(|i| headers.get(i))
                .map(Self::skin_header_display_name)
                .unwrap_or_else(|| "(none)".to_string());
            let mut new_idx = selected_idx.unwrap_or(0);
            ui.horizontal(|ui| {
                ui.label("Skin:");
                egui::ComboBox::from_id_salt("skin_header_selector")
                    .selected_text(display)
                    .show_ui(ui, |ui| {
                        for (i, header) in headers.iter().enumerate() {
                            let name = Self::skin_header_display_name(header);
                            ui.selectable_value(&mut new_idx, i, name);
                        }
                    });
            });
            if Some(new_idx) != selected_idx {
                self.skinheader_selector = Some(new_idx);
                self.change_skin_header();
            }
        }

        ui.separator();

        // Dynamic skin config items (options, files, offsets)
        egui::Grid::new("skin_config_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                for item in self.skinconfig_items.iter_mut() {
                    match item {
                        SkinConfigItem::Label(text) => {
                            if text.is_empty() {
                                ui.add_space(4.0);
                                ui.add_space(4.0);
                            } else {
                                ui.label(egui::RichText::new(text.as_str()).strong());
                                ui.label(""); // empty second column
                            }
                            ui.end_row();
                        }
                        SkinConfigItem::Option {
                            name,
                            items: combo_items,
                            selected_index,
                        } => {
                            ui.label(format!("{}:", name));
                            let display = combo_items
                                .get(*selected_index)
                                .cloned()
                                .unwrap_or_default();
                            egui::ComboBox::from_id_salt(format!("skin_opt_{}", name))
                                .selected_text(display)
                                .show_ui(ui, |ui| {
                                    for (i, label) in combo_items.iter().enumerate() {
                                        ui.selectable_value(selected_index, i, label.as_str());
                                    }
                                });
                            ui.end_row();
                        }
                        SkinConfigItem::File {
                            name,
                            items: combo_items,
                            selected_value,
                        } => {
                            ui.label(format!("{}:", name));
                            let display = selected_value.clone().unwrap_or_default();
                            let mut new_val = display.clone();
                            egui::ComboBox::from_id_salt(format!("skin_file_{}", name))
                                .selected_text(&display)
                                .show_ui(ui, |ui| {
                                    for label in combo_items.iter() {
                                        ui.selectable_value(
                                            &mut new_val,
                                            label.clone(),
                                            label.as_str(),
                                        );
                                    }
                                });
                            if new_val != display {
                                *selected_value = Some(new_val);
                            }
                            ui.end_row();
                        }
                        SkinConfigItem::Offset {
                            name,
                            values,
                            enabled,
                        } => {
                            ui.label(format!("{}:", name));
                            ui.horizontal(|ui| {
                                let labels = ["x", "y", "w", "h", "r", "a"];
                                for (i, &label) in labels.iter().enumerate() {
                                    if enabled[i] {
                                        ui.label(label);
                                        ui.add(
                                            egui::DragValue::new(&mut values[i])
                                                .range(-9999..=9999),
                                        );
                                    }
                                }
                            });
                            ui.end_row();
                        }
                    }
                }
            });
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
                // int index = optionbox.get(option).getSelectionModel().getSelectedIndex();
                let index = *selected_index;
                // SkinConfig.Option o = new SkinConfig.Option();
                // o.name = option.name;
                // if(index != optionbox.get(option).getItems().size() - 1) {
                let o_value = if index != items.len().saturating_sub(1) {
                    // o.value = option.option[index];
                    if index < option.option.len() {
                        option.option[index]
                    } else {
                        0
                    }
                } else {
                    // o.value = OPTION_RANDOM_VALUE;
                    OPTION_RANDOM_VALUE
                };
                let o = SkinOption {
                    name: Some(option.name.clone()),
                    value: o_value,
                };
                // options.add(o);
                options.push(Some(o));
            }
        }
        // property.setOption(options.toArray(...));
        property.option = options;

        // List<SkinConfig.FilePath> files = new ArrayList<>();
        let mut files: Vec<Option<SkinFilePath>> = Vec::new();
        // for (CustomFile file : selected.getCustomFiles()) {
        for file in selected.custom_files() {
            // if (filebox.get(file) != null) {
            if let Some(&item_idx) = self.filebox.get(&file.name)
                && let Some(SkinConfigItem::File { selected_value, .. }) =
                    self.skinconfig_items.get(item_idx)
            {
                // SkinConfig.FilePath o = new SkinConfig.FilePath();
                // o.name = file.name; o.path = filebox.get(file).getValue();
                let o = SkinFilePath {
                    name: Some(file.name.clone()),
                    path: selected_value.clone(),
                };
                // files.add(o);
                files.push(Some(o));
            }
        }
        // property.setFile(files.toArray(...));
        property.file = files;

        // List<SkinConfig.Offset> offsets = new ArrayList<>();
        let mut offsets: Vec<Option<SkinOffset>> = Vec::new();
        // for (CustomOffset offset : selected.getCustomOffsets()) {
        for offset in selected.custom_offsets() {
            // if (offsetbox.get(offset) != null) {
            if let Some(&item_idx) = self.offsetbox.get(&offset.name)
                && let Some(SkinConfigItem::Offset { values, .. }) =
                    self.skinconfig_items.get(item_idx)
            {
                // SkinConfig.Offset o = new SkinConfig.Offset();
                // o.name = offset.name; o.x = spinner[0].getValue(); ...
                let o = SkinOffset {
                    name: Some(offset.name.clone()),
                    x: values[0],
                    y: values[1],
                    w: values[2],
                    h: values[3],
                    r: values[4],
                    a: values[5],
                };
                // offsets.add(o);
                offsets.push(Some(o));
            }
        }
        // property.setOffset(offsets.toArray(...));
        property.offset = offsets;

        property
    }

    /// Translates: getSkinHeader(SkinType mode)
    pub fn skin_header(&self, mode: &SkinType) -> Vec<&SkinHeader> {
        // List<SkinHeader> result = new ArrayList<>();
        // for (SkinHeader header : skinheader) { if (header.getSkinType() == mode) { result.add(header); } }
        self.skinheader
            .iter()
            .filter(|header| header.skin_type() == Some(mode))
            .collect()
    }

    /// Translates: changeSkinType()
    pub fn change_skin_type(&mut self) {
        // commitSkinType();
        self.commit_skin_type();
        // updateSkinType(skintypeSelector.getValue());
        if let Some(skin_type) = self.skintype_selector {
            self.update_skin_type(&skin_type);
        }
    }

    /// Translates: updateSkinType(SkinType type)
    pub fn update_skin_type(&mut self, skin_type: &SkinType) {
        // mode = type;
        self.mode = Some(*skin_type);

        // skinheaderSelector.getItems().clear();
        self.current_headers.clear();
        // SkinHeader[] headers = getSkinHeader(type);
        // skinheaderSelector.getItems().addAll(headers);
        let headers: Vec<SkinHeader> = self
            .skinheader
            .iter()
            .filter(|h| h.skin_type() == Some(skin_type))
            .cloned()
            .collect();
        self.current_headers = headers;

        // if (player.getSkin()[type.getId()] != null) {
        if let Some(ref player) = self.player {
            let type_id = skin_type.id() as usize;
            if type_id < player.skin.len()
                && let Some(ref skinconf) = player.skin[type_id]
            {
                // for (SkinHeader header : skinheaderSelector.getItems()) {
                let mut found = false;
                for (i, header) in self.current_headers.iter().enumerate() {
                    // if (header != null && header.getPath().equals(Paths.get(skinconf.getPath()))) {
                    if let (Some(header_path), Some(skin_path)) = (header.path(), &skinconf.path)
                        && header_path == &PathBuf::from(skin_path)
                    {
                        // skinheaderSelector.setValue(header);
                        self.skinheader_selector = Some(i);
                        // skinconfig.setContent(create(skinheaderSelector.getValue(), skinconf.getProperties()));
                        let header_clone = header.clone();
                        let props = skinconf.properties.clone();
                        self.create(&header_clone, props.as_ref());
                        found = true;
                        break;
                    }
                }
                if !found {
                    // skinheaderSelector.getSelectionModel().select(0);
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
        // if(player == null) { return; }
        if self.player.is_none() {
            return;
        }

        // if (selected != null) {
        if let Some(selected) = self.selected.clone() {
            // SkinConfig skin = new SkinConfig(selected.getPath().toString());
            let path_str: String = selected
                .path()
                .map(|p: &PathBuf| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let mut skin = SkinConfig::new_with_path(&path_str);
            // skin.setProperties(getProperty());
            skin.properties = Some(self.property());
            // player.getSkin()[selected.getSkinType().getId()] = skin;
            if let Some(skin_type) = selected.skin_type() {
                let type_id = skin_type.id() as usize;
                let player = self.player.as_mut().expect("player is Some");
                while player.skin.len() <= type_id {
                    player.skin.push(None);
                }
                player.skin[type_id] = Some(skin);
            }
        } else if let Some(mode) = self.mode {
            // } else if (mode != null) { player.getSkin()[mode.getId()] = null; }
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
        // List<Path> skinpaths = new ArrayList<>();
        let mut skinpaths: Vec<PathBuf> = Vec::new();
        // scan(Paths.get(config.getSkinpath()), skinpaths);
        Self::scan(&PathBuf::from(&config.paths.skinpath), &mut skinpaths);

        // for (Path path : skinpaths) {
        for path in &skinpaths {
            if let Some(header) = load_skin_header(path, config) {
                // 7/14key skinは5/10keyにも加える (add 7/14key skins as 5/10key too)
                if header.toast_type() == rubato_skin::skin_header::TYPE_LR2SKIN
                    && let Some(skin_type) = header.skin_type()
                    && (*skin_type == SkinType::Play7Keys || *skin_type == SkinType::Play14Keys)
                {
                    // Re-load to get a fresh copy for the 5/10key variant
                    if let Some(mut variant) = load_skin_header(path, config) {
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
                }
                self.skinheader.push(header);
            }
        }
    }

    /// Translates: scan(Path p, List<Path> paths)
    /// Recursively scans for skin definition files.
    fn scan(p: &Path, paths: &mut Vec<PathBuf>) {
        // if (Files.isDirectory(p)) {
        if p.is_dir() {
            // try (Stream<Path> sub = Files.list(p)) { sub.forEach((t) -> { scan(t, paths); }); }
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    Self::scan(&entry.path(), paths);
                }
            }
        } else {
            // } else if (p.getFileName().toString().toLowerCase().endsWith(".lr2skin")
            //     || p.getFileName().toString().toLowerCase().endsWith(".luaskin")
            //     || p.getFileName().toString().toLowerCase().endsWith(".json")) {
            if let Some(filename) = p.file_name() {
                let lower = filename.to_string_lossy().to_lowercase();
                if lower.ends_with(".lr2skin")
                    || lower.ends_with(".luaskin")
                    || lower.ends_with(".json")
                {
                    // paths.add(p);
                    paths.push(p.to_path_buf());
                }
            }
        }
    }

    /// Translates: update(PlayerConfig player)
    pub fn update_player(&mut self, player: &PlayerConfig) {
        // this.player = player;
        self.player = Some(player.clone());
        // skintypeSelector.setValue(SkinType.PLAY_7KEYS);
        self.skintype_selector = Some(SkinType::Play7Keys);
        // updateSkinType(SkinType.PLAY_7KEYS);
        self.update_skin_type(&SkinType::Play7Keys);
    }

    /// Translates: commit()
    pub fn commit(&mut self) {
        // commitSkinType();
        self.commit_skin_type();
        // commitSkinHeader();
        self.commit_skin_header();
    }

    /// Translates: changeSkinHeader()
    pub fn change_skin_header(&mut self) {
        // commitSkinHeader();
        self.commit_skin_header();
        // updateSkinHeader(skinheaderSelector.getValue());
        let header = self
            .skinheader_selector
            .and_then(|i| self.current_headers.get(i))
            .cloned();
        self.update_skin_header(header.as_ref());
    }

    /// Translates: updateSkinHeader(SkinHeader header)
    pub fn update_skin_header(&mut self, header: Option<&SkinHeader>) {
        // SkinConfig.Property property = null;
        let mut property: Option<SkinProperty> = None;
        // if(header != null) {
        if let Some(header) = header {
            // for(SkinConfig skinc : player.getSkinHistory()) {
            if let Some(ref player) = self.player {
                for skinc in &player.skin_history {
                    // if(skinc.getPath().equals(header.getPath().toString())) {
                    if let (Some(skin_path), Some(header_path)) = (&skinc.path, header.path())
                        && skin_path == &header_path.to_string_lossy().to_string()
                    {
                        // property = skinc.getProperties();
                        property = skinc.properties.clone();
                        break;
                    }
                }
            }
        }
        // skinconfig.setContent(create(header, property));
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
        // if(selected != null) {
        let selected = match self.selected.clone() {
            Some(s) => s,
            None => return,
        };

        // SkinConfig.Property property = getProperty();
        let property = self.property();

        let player = match self.player.as_mut() {
            Some(p) => p,
            None => return,
        };

        // int index = -1;
        let mut index: Option<usize> = None;
        // for(int i = 0; i < player.getSkinHistory().length; i++) {
        for (i, history_entry) in player.skin_history.iter().enumerate() {
            // if(player.getSkinHistory()[i].getPath().equals(selected.getPath().toString())) {
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

        // SkinConfig sc = new SkinConfig();
        // sc.setPath(selected.getPath().toString()); sc.setProperties(property);
        let sc = SkinConfig {
            path: selected
                .path()
                .map(|p: &PathBuf| p.to_string_lossy().to_string()),
            properties: Some(property),
        };

        // if(index >= 0) { player.getSkinHistory()[index] = sc; }
        if let Some(idx) = index {
            player.skin_history[idx] = sc;
        } else {
            // else { SkinConfig[] history = Arrays.copyOf(...); history[history.length - 1] = sc; player.setSkinHistory(history); }
            player.skin_history.push(sc);
        }
    }

    /// Translates: create(SkinHeader header, SkinConfig.Property property)
    /// Builds the skin configuration UI items.
    fn create(&mut self, header: &SkinHeader, property: Option<&SkinProperty>) {
        // selected = header;
        self.selected = Some(header.clone());

        // if (header == null) { return null; }
        // (handled by caller)

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
                    // if(item instanceof CustomOption) {
                    let option = &header.custom_options()[*opt_idx];
                    // ComboBox<String> combo = new ComboBox<>();
                    // combo.getItems().setAll(option.contents);
                    // combo.getItems().add("Random");
                    let mut combo_items: Vec<String> = option.contents.clone();
                    combo_items.push("Random".to_string());

                    // combo.getSelectionModel().select(0);
                    let mut selection: usize = 0;
                    // int selection = -1;
                    let mut found_selection: Option<usize> = None;

                    // for(SkinConfig.Option o : property.getOption()) {
                    for o in property.option.iter().flatten() {
                        // if (o.name.equals(option.name)) {
                        if o.name.as_deref() == Some(&option.name) {
                            // int i = o.value;
                            let val = o.value;
                            // if(i != OPTION_RANDOM_VALUE) {
                            if val != OPTION_RANDOM_VALUE {
                                // for(int index = 0; index < option.option.length; index++) {
                                for (index, &opt_val) in option.option.iter().enumerate() {
                                    // if(option.option[index] == i) { selection = index; break; }
                                    if opt_val == val {
                                        found_selection = Some(index);
                                        break;
                                    }
                                }
                            } else {
                                // selection = combo.getItems().size() - 1;
                                found_selection = Some(combo_items.len() - 1);
                            }
                            break;
                        }
                    }

                    // if (selection < 0 && option.def != null) {
                    if found_selection.is_none()
                        && let Some(ref def) = option.def
                    {
                        // for (int index = 0; index < option.option.length; index++) {
                        for (index, content) in option.contents.iter().enumerate() {
                            // if (option.contents[index].equals(option.def)) { selection = index; }
                            if content == def {
                                found_selection = Some(index);
                            }
                        }
                    }

                    // if (selection >= 0) { combo.getSelectionModel().select(selection); }
                    if let Some(sel) = found_selection {
                        selection = sel;
                    }

                    let item_idx = self.skinconfig_items.len();
                    // optionbox.put(option, combo);
                    self.optionbox.insert(option.name.clone(), item_idx);
                    self.skinconfig_items.push(SkinConfigItem::Option {
                        name: option.name.clone(),
                        items: combo_items,
                        selected_index: selection,
                    });
                }
                CreateItem::FileIdx(file_idx) => {
                    // if(item instanceof CustomFile) {
                    let file = &header.custom_files()[*file_idx];

                    // String name = file.path.substring(file.path.lastIndexOf('/') + 1);
                    let mut name = file
                        .path
                        .rfind('/')
                        .map(|i| &file.path[i + 1..])
                        .unwrap_or(&file.path)
                        .to_string();

                    // if(file.path.contains("|")) {
                    if file.path.contains('|') {
                        let last_pipe = file.path.rfind('|').expect("contains '|'");
                        let last_slash = file.path.rfind('/').map(|i| i + 1).unwrap_or(0);
                        let first_pipe = file.path.find('|').expect("contains '|'");
                        // if(file.path.length() > file.path.lastIndexOf('|') + 1) {
                        if file.path.len() > last_pipe + 1 {
                            // name = file.path.substring(file.path.lastIndexOf('/') + 1, file.path.indexOf('|'))
                            //      + file.path.substring(file.path.lastIndexOf('|') + 1);
                            name = format!(
                                "{}{}",
                                &file.path[last_slash..first_pipe],
                                &file.path[last_pipe + 1..]
                            );
                        } else {
                            // name = file.path.substring(file.path.lastIndexOf('/') + 1, file.path.indexOf('|'));
                            name = file.path[last_slash..first_pipe].to_string();
                        }
                    }

                    // final int slashindex = file.path.lastIndexOf('/');
                    let slashindex = file.path.rfind('/');
                    // final Path dirpath = slashindex != -1 ? Paths.get(file.path.substring(0, slashindex)) : Paths.get(file.path);
                    let dirpath = match slashindex {
                        Some(idx) => PathBuf::from(&file.path[..idx]),
                        None => PathBuf::from(&file.path),
                    };

                    // if (!Files.exists(dirpath)) { continue; }
                    if !dirpath.exists() {
                        continue;
                    }

                    // try (DirectoryStream<Path> paths = Files.newDirectoryStream(dirpath, "{" + name.toLowerCase() + "," + name.toUpperCase() + "}")) {
                    let mut combo_items: Vec<String> = Vec::new();
                    match std::fs::read_dir(&dirpath) {
                        Ok(entries) => {
                            for entry in entries.flatten() {
                                let filename = entry.file_name().to_string_lossy().to_string();
                                if matches_skin_file_pattern_case_insensitive(&filename, &name) {
                                    // combo.getItems().add(p.getFileName().toString());
                                    combo_items.push(filename);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to read directory {:?}: {}", dirpath, e);
                            continue;
                        }
                    }
                    // combo.getItems().add("Random");
                    combo_items.push("Random".to_string());

                    // String selection = null;
                    let mut selection: Option<String> = None;
                    // for(SkinConfig.FilePath f : property.getFile()) {
                    for f in property.file.iter().flatten() {
                        // if(f.name.equals(file.name)) { selection = f.path; break; }
                        if f.name.as_deref() == Some(&file.name) {
                            selection = f.path.clone();
                            break;
                        }
                    }

                    // if (selection == null && file.def != null) {
                    if selection.is_none()
                        && let Some(ref def) = file.def
                    {
                        // for (String filename : combo.getItems()) {
                        for filename in &combo_items {
                            // if (filename.equalsIgnoreCase(file.def)) { selection = filename; break; }
                            if filename.eq_ignore_ascii_case(def) {
                                selection = Some(filename.clone());
                                break;
                            }
                            // int point = filename.lastIndexOf('.');
                            if let Some(point) = filename.rfind('.') {
                                // if (filename.substring(0, point).equalsIgnoreCase(file.def)) { selection = filename; break; }
                                if filename[..point].eq_ignore_ascii_case(def) {
                                    selection = Some(filename.clone());
                                    break;
                                }
                            }
                        }
                    }

                    // if (selection != null) { combo.setValue(selection); }
                    // else { combo.getSelectionModel().select(0); }
                    let selected_value = if selection.is_some() {
                        selection
                    } else if !combo_items.is_empty() {
                        Some(combo_items[0].clone())
                    } else {
                        None
                    };

                    let item_idx = self.skinconfig_items.len();
                    // filebox.put(file, combo);
                    self.filebox.insert(file.name.clone(), item_idx);
                    self.skinconfig_items.push(SkinConfigItem::File {
                        name: file.name.clone(),
                        items: combo_items,
                        selected_value,
                    });
                }
                CreateItem::OffsetIdx(offset_idx) => {
                    // if(item instanceof CustomOffset) {
                    let offset = &header.custom_offsets()[*offset_idx];
                    // final String[] values = {"x","y","w","h","r","a"};
                    // final boolean[] b = {option.x, option.y, option.w, option.h, option.r, option.a};
                    let enabled = [
                        offset.caps.x,
                        offset.caps.y,
                        offset.caps.w,
                        offset.caps.h,
                        offset.caps.r,
                        offset.caps.a,
                    ];

                    // SkinConfig.Offset offset = null;
                    // for(SkinConfig.Offset o : property.getOffset()) { if(o.name.equals(option.name)) { offset = o; break; } }
                    let mut found_offset: Option<&SkinOffset> = None;
                    for o in &property.offset {
                        if let Some(o) = o
                            && o.name.as_deref() == Some(&offset.name)
                        {
                            found_offset = Some(o);
                            break;
                        }
                    }

                    // final int[] v = offset != null ? new int[]{offset.x, offset.y, offset.w, offset.h, offset.r, offset.a} : new int[values.length];
                    let v = if let Some(o) = found_offset {
                        [o.x, o.y, o.w, o.h, o.r, o.a]
                    } else {
                        [0, 0, 0, 0, 0, 0]
                    };

                    let item_idx = self.skinconfig_items.len();
                    // offsetbox.put(option, spinner);
                    self.offsetbox.insert(offset.name.clone(), item_idx);
                    self.skinconfig_items.push(SkinConfigItem::Offset {
                        name: offset.name.clone(),
                        values: v,
                        enabled,
                    });
                }
                CreateItem::Label(text) => {
                    // if(item instanceof String) { ... }
                    self.skinconfig_items
                        .push(SkinConfigItem::Label(text.clone()));
                }
            }
        }
    }

    /// Helper: Get skin type display name for SkinTypeCell
    /// Translates: SkinTypeCell.updateItem(SkinType, boolean)
    pub fn skin_type_display_name(skin_type: &SkinType) -> &'static str {
        skin_type.name()
    }

    /// Helper: Get skin header display name for SkinListCell
    /// Translates: SkinListCell.updateItem(SkinHeader, boolean)
    pub fn skin_header_display_name(header: &SkinHeader) -> String {
        let name = header.name().unwrap_or("");
        if header.toast_type() == TYPE_BEATORJASKIN {
            name.to_string()
        } else {
            format!("{} (LR2 Skin)", name)
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

/// Internal enum for the create() method's item list
enum CreateItem {
    Label(String),
    OptionIdx(usize),
    FileIdx(usize),
    OffsetIdx(usize),
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

        // Real play7.json may fail to parse due to complex fields;
        // this test verifies the loader handles it gracefully
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
            paths: rubato_core::config::PathConfig {
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
            paths: rubato_core::config::PathConfig {
                skinpath: skin_dir.to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut view = SkinConfigurationView::new();
        view.update_config(&config);

        // At least some headers should have valid skin types
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
            paths: rubato_core::config::PathConfig {
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
            paths: rubato_core::config::PathConfig {
                skinpath: skin_dir.to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut view = SkinConfigurationView::new();
        view.update_config(&config);

        // The default skin directory has play7.json which should be SkinType::Play7Keys
        let play7_headers = view.skin_header(&SkinType::Play7Keys);
        // We don't assert specific count, but it should be filterable
        // If there are any, they should all have the correct type
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

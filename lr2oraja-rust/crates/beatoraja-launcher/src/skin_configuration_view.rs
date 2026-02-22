// SkinConfigurationView.java -> skin_configuration_view.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::error;

use beatoraja_core::config::Config;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_core::skin_config::{SkinConfig, SkinFilePath, SkinOffset, SkinOption, SkinProperty};
use beatoraja_skin::skin_header::{CustomItemEnum, SkinHeader, TYPE_BEATORJASKIN};
use beatoraja_skin::skin_property::OPTION_RANDOM_VALUE;
use beatoraja_skin::skin_type::SkinType;

/// Skin configuration item for egui rendering
/// Translates the dynamic VBox content built by create() method.
#[derive(Clone, Debug)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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

#[allow(dead_code)]
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

    pub fn get_skintype_selector(&self) -> Option<SkinType> {
        self.skintype_selector
    }

    pub fn set_skintype_selector(&mut self, skin_type: SkinType) {
        self.skintype_selector = Some(skin_type);
    }

    pub fn get_current_headers(&self) -> &[SkinHeader] {
        &self.current_headers
    }

    pub fn get_skinheader_selector(&self) -> Option<usize> {
        self.skinheader_selector
    }

    pub fn set_skinheader_selector(&mut self, index: usize) {
        self.skinheader_selector = Some(index);
    }

    pub fn get_skinconfig_items_mut(&mut self) -> &mut Vec<SkinConfigItem> {
        &mut self.skinconfig_items
    }

    pub fn get_player(&self) -> Option<&PlayerConfig> {
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
    pub fn get_selected_header(&self) -> Option<&SkinHeader> {
        self.selected.as_ref()
    }

    /// Translates: getProperty()
    /// Reads current UI state and returns a SkinProperty.
    pub fn get_property(&self) -> SkinProperty {
        // SkinConfig.Property property = new SkinConfig.Property();
        let mut property = SkinProperty::default();

        let selected = match &self.selected {
            Some(s) => s,
            None => return property,
        };

        // List<SkinConfig.Option> options = new ArrayList<>();
        let mut options: Vec<Option<SkinOption>> = Vec::new();
        // for (CustomOption option : selected.getCustomOptions()) {
        for option in selected.get_custom_options() {
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
        for file in selected.get_custom_files() {
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
        for offset in selected.get_custom_offsets() {
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
    pub fn get_skin_header(&self, mode: &SkinType) -> Vec<&SkinHeader> {
        // List<SkinHeader> result = new ArrayList<>();
        // for (SkinHeader header : skinheader) { if (header.getSkinType() == mode) { result.add(header); } }
        self.skinheader
            .iter()
            .filter(|header| header.get_skin_type() == Some(mode))
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
            .filter(|h| h.get_skin_type() == Some(skin_type))
            .cloned()
            .collect();
        self.current_headers = headers;

        // if (player.getSkin()[type.getId()] != null) {
        if let Some(ref player) = self.player {
            let type_id = skin_type.get_id() as usize;
            if type_id < player.skin.len()
                && let Some(ref skinconf) = player.skin[type_id]
            {
                // for (SkinHeader header : skinheaderSelector.getItems()) {
                let mut found = false;
                for (i, header) in self.current_headers.iter().enumerate() {
                    // if (header != null && header.getPath().equals(Paths.get(skinconf.getPath()))) {
                    if let (Some(header_path), Some(skin_path)) =
                        (header.get_path(), &skinconf.path)
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
                .get_path()
                .map(|p: &PathBuf| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let mut skin = SkinConfig::new_with_path(&path_str);
            // skin.setProperties(getProperty());
            skin.properties = Some(self.get_property());
            // player.getSkin()[selected.getSkinType().getId()] = skin;
            if let Some(skin_type) = selected.get_skin_type() {
                let type_id = skin_type.get_id() as usize;
                let player = self.player.as_mut().unwrap();
                while player.skin.len() <= type_id {
                    player.skin.push(None);
                }
                player.skin[type_id] = Some(skin);
            }
        } else if let Some(mode) = self.mode {
            // } else if (mode != null) { player.getSkin()[mode.getId()] = null; }
            let type_id = mode.get_id() as usize;
            let player = self.player.as_mut().unwrap();
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
        Self::scan(&PathBuf::from(&config.skinpath), &mut skinpaths);

        // for (Path path : skinpaths) {
        for path in &skinpaths {
            let path_string = path.to_string_lossy().to_lowercase();
            if path_string.ends_with(".json") {
                // JSONSkinLoader loader = new JSONSkinLoader();
                // SkinHeader header = loader.loadHeader(path);
                // if (header != null) { skinheader.add(header); }
                // TODO: JSONSkinLoader integration
                // Stub: would load JSON skin header
            } else if path_string.ends_with(".luaskin") {
                // LuaSkinLoader loader = new LuaSkinLoader();
                // SkinHeader header = loader.loadHeader(path);
                // if (header != null) { skinheader.add(header); }
                // TODO: LuaSkinLoader integration
                // Stub: would load Lua skin header
            } else {
                // LR2SkinHeaderLoader loader = new LR2SkinHeaderLoader(config);
                // SkinHeader header = loader.loadSkin(path, null);
                // skinheader.add(header);
                // TODO: LR2SkinHeaderLoader integration
                // Stub: would load LR2 skin header

                // 7/14key skinは5/10keyにも加える
                // if(header.getType() == SkinHeader.TYPE_LR2SKIN &&
                //     (header.getSkinType() == SkinType.PLAY_7KEYS || header.getSkinType() == SkinType.PLAY_14KEYS)) {
                //     header = loader.loadSkin(path, null);
                //     ... rename and re-map skin type ...
                //     skinheader.add(header);
                // }
                // (LR2 skin 7/14key → 5/10key duplication deferred to loader integration)
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
                    if let (Some(skin_path), Some(header_path)) = (&skinc.path, header.get_path())
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
        let property = self.get_property();

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
                .get_path()
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
                .get_path()
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
        let mut other_options: Vec<usize> = Vec::new(); // indices into header's custom options
        let mut other_files: Vec<usize> = Vec::new(); // indices into header's custom files
        let mut other_offsets: Vec<usize> = Vec::new(); // indices into header's custom offsets

        // otheritems.addAll(Arrays.asList(header.getCustomOptions()));
        for i in 0..header.get_custom_options().len() {
            other_options.push(i);
        }
        // otheritems.addAll(Arrays.asList(header.getCustomFiles()));
        for i in 0..header.get_custom_files().len() {
            other_files.push(i);
        }
        // otheritems.addAll(Arrays.asList(header.getCustomOffsets()));
        for i in 0..header.get_custom_offsets().len() {
            other_offsets.push(i);
        }

        // for(CustomCategory category : header.getCustomCategories()) {
        for category in header.get_custom_categories() {
            // items.add(category.name);
            items.push(CreateItem::Label(category.name.clone()));
            // for(Object item : category.items) { items.add(item); otheritems.remove(item); }
            for cat_item in &category.items {
                match cat_item {
                    CustomItemEnum::Option(opt) => {
                        // Find and remove from other_options
                        if let Some(pos) = other_options
                            .iter()
                            .position(|&i| header.get_custom_options()[i].name == opt.name)
                        {
                            let idx = other_options.remove(pos);
                            items.push(CreateItem::OptionIdx(idx));
                        }
                    }
                    CustomItemEnum::File(file) => {
                        if let Some(pos) = other_files
                            .iter()
                            .position(|&i| header.get_custom_files()[i].name == file.name)
                        {
                            let idx = other_files.remove(pos);
                            items.push(CreateItem::FileIdx(idx));
                        }
                    }
                    CustomItemEnum::Offset(offset) => {
                        if let Some(pos) = other_offsets
                            .iter()
                            .position(|&i| header.get_custom_offsets()[i].name == offset.name)
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
                    let option = &header.get_custom_options()[*opt_idx];
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
                    let file = &header.get_custom_files()[*file_idx];

                    // String name = file.path.substring(file.path.lastIndexOf('/') + 1);
                    let mut name = file
                        .path
                        .rfind('/')
                        .map(|i| &file.path[i + 1..])
                        .unwrap_or(&file.path)
                        .to_string();

                    // if(file.path.contains("|")) {
                    if file.path.contains('|') {
                        let last_pipe = file.path.rfind('|').unwrap();
                        let last_slash = file.path.rfind('/').map(|i| i + 1).unwrap_or(0);
                        let first_pipe = file.path.find('|').unwrap();
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
                            let name_lower = name.to_lowercase();
                            let name_upper = name.to_uppercase();
                            for entry in entries.flatten() {
                                let filename = entry.file_name().to_string_lossy().to_string();
                                // Glob matching: filename matches name pattern (case-insensitive)
                                if filename.to_lowercase().contains(&name_lower)
                                    || filename.to_uppercase().contains(&name_upper)
                                {
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
                    let offset = &header.get_custom_offsets()[*offset_idx];
                    // final String[] values = {"x","y","w","h","r","a"};
                    // final boolean[] b = {option.x, option.y, option.w, option.h, option.r, option.a};
                    let enabled = [offset.x, offset.y, offset.w, offset.h, offset.r, offset.a];

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
        skin_type.get_name()
    }

    /// Helper: Get skin header display name for SkinListCell
    /// Translates: SkinListCell.updateItem(SkinHeader, boolean)
    pub fn skin_header_display_name(header: &SkinHeader) -> String {
        let name = header.get_name().unwrap_or("");
        if header.get_type() == TYPE_BEATORJASKIN {
            name.to_string()
        } else {
            format!("{} (LR2 Skin)", name)
        }
    }
}

/// Internal enum for the create() method's item list
#[allow(dead_code)]
enum CreateItem {
    Label(String),
    OptionIdx(usize),
    FileIdx(usize),
    OffsetIdx(usize),
}

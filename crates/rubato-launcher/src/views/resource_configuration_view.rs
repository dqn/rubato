// ResourceConfigurationView.java -> resource_configuration_view.rs
// Mechanical line-by-line translation.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::thread::JoinHandle;

use log::error;

use rubato_core::config::{AVAILABLE_TABLEURL, Config};
use rubato_core::table_data_accessor::TableDataAccessor;

use crate::platform::show_directory_chooser;
use crate::views::play_configuration_view::PlayConfigurationView;

/// TableInfo - inner data class for table URL entries
/// Translates: ResourceConfigurationView.TableInfo (JavaFX property → plain struct)
#[derive(Clone, Debug, Default)]
pub struct TableInfo {
    pub url: String,
    pub name_status: String,
    pub comment: String,
}

impl TableInfo {
    /// Constructor: looks up name/comment from static table
    pub fn new(url: &str) -> Self {
        let table_name_comment = table_name_comment();
        if let Some((name, comment)) = table_name_comment.get(url) {
            TableInfo {
                url: url.to_string(),
                name_status: name.clone(),
                comment: comment.clone(),
            }
        } else {
            TableInfo {
                url: url.to_string(),
                name_status: String::new(),
                comment: String::new(),
            }
        }
    }

    pub fn set_name_status(&mut self, name_status: &str) {
        self.name_status = name_status.to_string();
    }

    /// Translates: TableInfo.toUrlArray(List<TableInfo>)
    pub fn to_url_array(list: &[TableInfo]) -> Vec<String> {
        list.iter().map(|t| t.url.clone()).collect()
    }

    /// Translates: TableInfo.populateList(List<TableInfo>, String[])
    pub fn populate_list(list: &mut Vec<TableInfo>, urls: &[String]) {
        list.clear();
        for url in urls {
            list.push(TableInfo::new(url));
        }
    }
}

/// Static table of known table URLs with name and comment.
/// Translates: ResourceConfigurationView.tableNameComment (Map.ofEntries(...))
/// Unicode escapes are preserved from Java source.
pub fn table_name_comment() -> HashMap<String, (String, String)> {
    let entries: Vec<(&str, &str, &str)> = vec![
        // sl,st,stardust,starlight
        (
            "https://mqppppp.neocities.org/StardustTable.html",
            "Stardust",
            "Beginner \u{2606}1-\u{2606}7",
        ),
        (
            "https://djkuroakari.github.io/starlighttable.html",
            "Stardust",
            "Intermediate \u{2606}7-\u{2606}12",
        ),
        (
            "https://stellabms.xyz/sl/table.html",
            "Satellite",
            "Insane \u{2606}11-\u{2605}19",
        ),
        (
            "https://stellabms.xyz/st/table.html",
            "Stella",
            "High Insane to Overjoy \u{2605}19-\u{2605}\u{2605}7",
        ),
        // the insanes
        (
            "https://darksabun.club/table/archive/normal1/",
            "\u{901a}\u{5e38}\u{96e3}\u{6613}\u{5ea6}\u{8868} (Normal 1)",
            "Beginner to Intermediate \u{2606}1-\u{2606}12",
        ),
        (
            "https://darksabun.club/table/archive/insane1/",
            "\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868} (Insane 1)",
            "Insane \u{2605}1-\u{2605}25",
        ),
        (
            "http://rattoto10.jounin.jp/table.html",
            "NEW GENERATION \u{901a}\u{5e38} (Normal 2)",
            "Post 2016 Normal Table \u{2606}1-\u{2606}12",
        ),
        (
            "http://rattoto10.jounin.jp/table_insane.html",
            "NEW GENERATION \u{767a}\u{72c2} (Insane 2)",
            "Post 2016 Insane Table \u{2605}1-\u{2605}25",
        ),
        // overjoy
        (
            "https://rattoto10.jounin.jp/table_overjoy.html",
            "NEW GENERATION overjoy",
            "New overjoy. \u{2605}\u{2605}0-\u{2605}\u{2605}7",
        ),
        // stream + chordjack
        (
            "https://lets-go-time-hell.github.io/code-stream-table/",
            "16\u{5206}\u{4e71} (16th streams)",
            "Chordstream focus. Wide difficulty \u{2606}11-\u{2605}20+",
        ),
        (
            "https://lets-go-time-hell.github.io/Arm-Shougakkou-table/",
            "\u{30a6}\u{30fc}\u{30c7}\u{30aa}\u{30b7}\u{5c0f}\u{5b66}\u{6821} (Ude table)",
            "Chordjack/wide chords focus. Satellite difficulty",
        ),
        (
            "https://su565fx.web.fc2.com/Gachimijoy/gachimijoy.html",
            "gachimijoy",
            "Hard chordjack. \u{2605}\u{2605}0-\u{2605}\u{2605}7",
        ),
        // stellaverse quirked up
        (
            "https://stellabms.xyz/so/table.html",
            "Solar",
            "Insane-style charts. Satellite difficulty",
        ),
        (
            "https://stellabms.xyz/sn/table.html",
            "Supernova",
            "Insane-style charts. Stella difficulty",
        ),
        // osu
        (
            "https://air-afother.github.io/osu-table/",
            "osu!",
            "Table for osu! star rating",
        ),
        // AI
        (
            "https://bms.hexlataia.xyz/tables/ai.html",
            "Hex's AI",
            "Algorithmically assigned difficulty. Insane and LN range",
        ),
        // Library
        (
            "https://bms.hexlataia.xyz/tables/db.html",
            "\u{767a}\u{72c2}\u{96e3}\u{6613}\u{5ea6}\u{30c7}\u{30fc}\u{30bf}\u{30d9}\u{30fc}\u{30b9} (Hex's DB)",
            "Manually assigned difficulty. Insane \u{2605}0-\u{2605}25+",
        ),
        (
            "https://bms.hexlataia.xyz/tables/olduploader.html",
            "\u{65e7}\u{30a2}\u{30d7}\u{30ed}\u{30c0}\u{8868} (Hex's Old uploader)",
            "Manually assigned difficulty. Mostly Insane \u{2606}10-\u{2605}25+ with LN + Scratch ratings",
        ),
        (
            "https://stellabms.xyz/upload.html",
            "Stella Uploader",
            "Stellaverse uploader. Insane \u{2605}1-\u{2605}25+",
        ),
        (
            "https://exturbow.github.io/github.io/index.html",
            "BMS\u{56f3}\u{66f8}\u{9928} (Turbow's Toshokan)",
            "Rates BMS event submissions. Wide difficulty \u{2606}1-\u{2605}25+",
        ),
        // beginner
        (
            "http://fezikedifficulty.futene.net/list.html",
            "\u{6c60}\u{7530}\u{7684} (Ikeda's Beginner)",
            "Beginner focused table. 19 levels \u{2606}1-\u{2606}11+",
        ),
        // LN
        (
            "https://ladymade-star.github.io/luminous/table.html",
            "Luminous",
            "Active LN table. \u{25c6}1-\u{25c6}27",
        ),
        (
            "https://vinylhouse.web.fc2.com/lntougou/difficulty.html",
            "Longnote\u{7d71}\u{5408}\u{8868} (LN Combined)",
            "\u{25c6}1-\u{25c6}27",
        ),
        (
            "http://flowermaster.web.fc2.com/lrnanido/gla/LN.html",
            "LN\u{96e3}\u{6613}\u{5ea6}",
            "Old LN table \u{25c6}1-\u{25c6}26",
        ),
        (
            "https://skar-wem.github.io/ln/",
            "LN Curtain",
            "Full/inverse LN charts. \u{25c6}1-\u{25c6}26",
        ),
        (
            "http://cerqant.web.fc2.com/zindy/table.html",
            "zindy LN",
            "Difficult shield stair patterns. Hard LN \u{25c6}15-\u{25c6}27+",
        ),
        (
            "https://notepara.com/glassist/lnoj",
            "LNoverjoy",
            "Hard LN table. \u{25c6}15-\u{25c6}27",
        ),
        // Scratch
        (
            "https://egret9.github.io/Scramble/",
            "Scramble",
            "Active scratch table",
        ),
        (
            "http://minddnim.web.fc2.com/sara/3rd_hard/bms_sara_3rd_hard.html",
            "\u{76bf}\u{96e3}\u{6613}\u{5ea6}\u{8868}(3rd) (Sara 3rd)",
            "Old scratch table",
        ),
        // delay
        (
            "https://lets-go-time-hell.github.io/Delay-joy-table/",
            "\u{30c7}\u{30a3}\u{30ec}\u{30a4}joy (delayjoy)",
            "Delay focus. Wide difficulty with heavy stella bias",
        ),
        (
            "https://kamikaze12345.github.io/github.io/delaytrainingtable/table.html",
            "DELAY Training Table",
            "Comprehensive delay table. Wide difficulty \u{2605}1-\u{2605}\u{2605}7",
        ),
        (
            "https://wrench616.github.io/Delay/",
            "Delay\u{5c0f}\u{5b66}\u{6821}",
            "Intermediate delay table. \u{2605}1-\u{2605}24",
        ),
        // High Diff
        (
            "https://darksabun.club/table/archive/old-overjoy/",
            "Overjoy (\u{65e7}) (Old overjoy)",
            "Pre-2018 overjoy table. \u{2605}\u{2605}0-\u{2605}\u{2605}7",
        ),
        (
            "https://monibms.github.io/Dystopia/dystopia.html",
            "Dystopia",
            "Active hard table. dy0-dy7 is st5-st12. dy8+ is st12+",
        ),
        (
            "https://www.firiex.com/tables/joverjoy",
            "joverjoy",
            "Large alternative to overjoy, last updated 2021. \u{2605}\u{2605}0-\u{2605}\u{2605}7+",
        ),
        // Hard Judge
        (
            "https://plyfrm.github.io/table/timing/",
            "Timing Table (Hard judge Table)",
            "Exclusively Hard judge. Judge turns easy to clear charts into challenges \u{2606}7-\u{2605}2++",
        ),
        // Artist search
        (
            "https://plyfrm.github.io/table/bmssearch/index.html",
            "BMSSearch Artists",
            "Contains 2400+ unique artists and nearly 100k bms",
        ),
        // DP
        (
            "https://yaruki0.net/DPlibrary/",
            "DPBMS\u{3068}\u{8af8}\u{611f} (Bluvel table)",
            "DP beginner focus. \u{2606}1-\u{2606}12",
        ),
        (
            "https://stellabms.xyz/dp/table.html",
            "DP Satellite",
            "Stellaverse. Roughly tracks \u{2606}10-\u{2605}10",
        ),
        (
            "https://stellabms.xyz/dpst/table.html",
            "DP Stella",
            "Stellaverse. Roughly tracks \u{2605}10-\u{2605}\u{2605}8+",
        ),
        (
            "https://deltabms.yaruki0.net/table/data/dpdelta_head.json",
            "\u{03b4}\u{96e3}\u{6613}\u{5ea6}\u{8868} (delta table)",
            "DP beginner focus. Contains IIDX equivalent GENOSIDE dans",
        ),
        (
            "https://deltabms.yaruki0.net/table/data/insane_head.json",
            "\u{767a}\u{72c2}DP\u{96e3}\u{6613}\u{5ea6}\u{8868} (DP Insane)",
            "Rated \u{2605}1-\u{2605}13",
        ),
        (
            "http://ereter.net/dpoverjoy/",
            "DP overjoy",
            "Hard DP table \u{2605}10+",
        ),
        // Stella Extensions
        (
            "https://notmichaelchen.github.io/stella-table-extensions/satellite-easy.html",
            "Satellite EASY",
            "Rated by difficulty to attain EC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/satellite-normal.html",
            "Satellite NORMAL",
            "Rated by difficulty to attain NC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/satellite-hard.html",
            "Satellite HARD",
            "Rated by difficulty to attain HC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/satellite-fullcombo.html",
            "Satellite FULLCOMBO",
            "Rated by difficulty to attain FC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/stella-easy.html",
            "Stella EASY",
            "Rated by difficulty to attain EC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/stella-normal.html",
            "Stella NORMAL",
            "Rated by difficulty to attain NC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/stella-hard.html",
            "Stella HARD",
            "Rated by difficulty to attain HC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/stella-fullcombo.html",
            "Stella FULLCOMBO",
            "Rated by difficulty to attain FC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/dp-satellite-easy.html",
            "DP Satellite EASY",
            "Rated by difficulty to attain EC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/dp-satellite-normal.html",
            "DP Satellite NORMAL",
            "Rated by difficulty to attain NC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/dp-satellite-hard.html",
            "DP Satellite HARD",
            "Rated by difficulty to attain HC",
        ),
        (
            "https://notmichaelchen.github.io/stella-table-extensions/dp-satellite-fullcombo.html",
            "DP Satellite FULLCOMBO",
            "Rated by difficulty to attain FC",
        ),
        // Walkure
        (
            "http://walkure.net/hakkyou/for_glassist/bms/?lamp=easy",
            "\u{767a}\u{72c2}BMS\u{96e3}\u{5ea6}\u{63a8}\u{5b9a}\u{8868} EASY",
            "Rated by difficulty to attain EC",
        ),
        (
            "http://walkure.net/hakkyou/for_glassist/bms/?lamp=normal",
            "\u{767a}\u{72c2}BMS\u{96e3}\u{5ea6}\u{63a8}\u{5b9a}\u{8868} NORMAL",
            "Rated by difficulty to attain NC",
        ),
        (
            "http://walkure.net/hakkyou/for_glassist/bms/?lamp=hard",
            "\u{767a}\u{72c2}BMS\u{96e3}\u{5ea6}\u{63a8}\u{5b9a}\u{8868} HARD",
            "Rated by difficulty to attain HC",
        ),
        (
            "http://walkure.net/hakkyou/for_glassist/bms/?lamp=fc",
            "\u{767a}\u{72c2}BMS\u{96e3}\u{5ea6}\u{63a8}\u{5b9a}\u{8868} FULLCOMBO",
            "Rated by difficulty to attain FC",
        ),
    ];

    let mut map = HashMap::new();
    for (url, name, comment) in entries {
        map.insert(url.to_string(), (name.to_string(), comment.to_string()));
    }
    map
}

/// ResourceConfigurationView - resource configuration UI
/// Translates: ResourceConfigurationView (JavaFX -> egui)
///
/// BMS root paths, table URL management, song update settings.
pub struct ResourceConfigurationView {
    // @FXML private ListView<String> bmsroot;
    bmsroot: Vec<String>,
    bmsroot_selected_items: Vec<String>,
    bmsroot_selected_item: Option<String>,

    // @FXML private TextField url;
    url: String,

    // @FXML private EditableTableView<TableInfo> tableurl;
    tableurl: Vec<TableInfo>,
    tableurl_selected_items: Vec<usize>,

    // @FXML private EditableTableView<TableInfo> available_tables;
    available_tables: Vec<TableInfo>,
    available_tables_selected_items: Vec<usize>,

    // @FXML private CheckBox updatesong;
    updatesong: bool,

    // private Config config;
    config: Option<Config>,

    // private PlayConfigurationView main;
    // (stored as a flag; actual reference to main is handled by the caller)

    // private String downloadDirectory;
    download_directory: String,

    /// Background table loading thread handle
    table_load_handle: Option<JoinHandle<()>>,
}

impl Default for ResourceConfigurationView {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceConfigurationView {
    pub fn new() -> Self {
        ResourceConfigurationView {
            bmsroot: Vec::new(),
            bmsroot_selected_items: Vec::new(),
            bmsroot_selected_item: None,
            url: String::new(),
            tableurl: Vec::new(),
            tableurl_selected_items: Vec::new(),
            available_tables: Vec::new(),
            available_tables_selected_items: Vec::new(),
            updatesong: false,
            config: None,
            download_directory: String::new(),
            table_load_handle: None,
        }
    }

    /// Translates: initialize(URL, ResourceBundle)
    /// Cell factory and selection mode → deferred to egui rendering.
    pub fn initialize(&mut self) {
        // bmsroot.setCellFactory(...)
        // → Cell rendering (download directory highlight) deferred to egui rendering.
        // Table column configuration deferred to egui rendering.
        // Selection mode and key press handlers deferred to egui rendering.
    }

    /// Translates: init(PlayConfigurationView main)
    /// Table column setup → deferred to egui rendering.
    pub fn init(&mut self, _main: &PlayConfigurationView) {
        // Selected Tables columns: NAME/STATUS, COMMENT, URL
        // Available Tables columns: NAME/STATUS, COMMENT, URL
        // Selection mode: MULTIPLE
        // Cross-selection clearing and Ctrl+C handlers deferred to egui.
    }

    /// Translates: update(Config config)
    pub fn update(&mut self, config: &mut Config) {
        // this.config = config;
        self.config = Some(config.clone());
        // this.downloadDirectory = config.getDownloadDirectory();
        self.download_directory = config.network.download_directory.clone();
        // bmsroot.getItems().setAll(config.getBmsroot());
        self.bmsroot = config.paths.bmsroot.clone();
        // updatesong.setSelected(config.isUpdatesong());
        self.updatesong = config.updatesong;

        // Make sure that all available tables are present in the list prior to deduplicating with the user tables
        // String[] intermediate = addUniqueTable(Config.AVAILABLE_TABLEURL, config.getAvailableURL());
        let available_tableurl: Vec<String> =
            AVAILABLE_TABLEURL.iter().map(|s| s.to_string()).collect();
        let intermediate = Self::add_unique_table(&available_tableurl, &config.paths.available_url);
        // Remove user tables that have already been added to the active list
        // intermediate = subtractTable(intermediate, config.getTableURL());
        let intermediate = Self::subtract_table(&intermediate, &config.paths.table_url);
        // config.setAvailableURL(intermediate);
        config.paths.available_url = intermediate.clone();
        self.config
            .as_mut()
            .expect("config is Some")
            .paths
            .available_url = intermediate.clone();
        // TableInfo.populateList(tableurl.getItems(), config.getTableURL());
        TableInfo::populate_list(&mut self.tableurl, &config.paths.table_url);
        // TableInfo.populateList(available_tables.getItems(), config.getAvailableURL());
        TableInfo::populate_list(&mut self.available_tables, &intermediate);
    }

    /// Translates: commit()
    pub fn commit(&mut self) {
        if let Some(ref mut config) = self.config {
            // config.setBmsroot(bmsroot.getItems().toArray(new String[0]));
            config.paths.bmsroot = self.bmsroot.clone();
            // config.setUpdatesong(updatesong.isSelected());
            config.updatesong = self.updatesong;
            // config.setTableURL(TableInfo.toUrlArray(tableurl.getItems()));
            config.paths.table_url = TableInfo::to_url_array(&self.tableurl);
            // config.setDownloadDirectory(downloadDirectory);
            config.network.download_directory = self.download_directory.clone();
        }
    }

    /// Translates: refreshLocalTableInfo()
    /// JavaFX progress bar and threading → simplified to synchronous call.
    pub fn refresh_local_table_info(&mut self) {
        // String[] urls = TableInfo.toUrlArray(tableurl.getItems());
        let urls = TableInfo::to_url_array(&self.tableurl);
        let url_refs: Vec<&str> = urls.iter().map(|s| s.as_str()).collect();

        if let Some(ref config) = self.config {
            // TableDataAccessor tda = new TableDataAccessor(config.getTablepath());
            let tda = TableDataAccessor::new(&config.paths.tablepath);
            // HashMap<String,String> urlToTableNameMap = tda.readLocalTableNames(urls);
            let url_to_table_name_map = tda.read_local_table_names(&url_refs);
            // for (TableInfo tableInfo : tableurl.getItems()) {
            for table_info in &mut self.tableurl {
                // String tableName = (urlToTableNameMap == null) ? null : urlToTableNameMap.get(tableInfo.getUrl());
                let table_name = url_to_table_name_map
                    .as_ref()
                    .and_then(|m| m.get(&table_info.url));
                // tableInfo.setNameStatus((tableName == null) ? "not loaded" : tableName);
                if let Some(name) = table_name {
                    table_info.set_name_status(name);
                } else {
                    table_info.set_name_status("not loaded");
                }
            }
        }
    }

    /// Poll for background table load completion. Call from the egui render
    /// loop so that `refresh_local_table_info()` runs on the UI thread once
    /// the background fetch finishes.
    pub fn poll_table_load(&mut self) {
        if let Some(ref handle) = self.table_load_handle
            && handle.is_finished()
        {
            self.table_load_handle = None;
            self.refresh_local_table_info();
        }
    }

    /// Returns true while a background table load is in progress.
    pub fn is_table_loading(&self) -> bool {
        self.table_load_handle.is_some()
    }

    /// Translates: loadAllTables()
    /// Blocking HTTP work runs on a background thread so the egui event loop
    /// is not frozen.
    pub fn load_all_tables(&mut self) {
        if self.table_load_handle.is_some() {
            return;
        }
        self.commit();

        if let Some(ref config) = self.config {
            let _ = fs::create_dir_all(&config.paths.tablepath);

            // Existing .bmt files are preserved until each replacement is
            // successfully fetched and written by update_table_data(), so a
            // refresh that fails mid-way (offline, timeout) does not wipe the
            // local cache.

            let tablepath = config.paths.tablepath.clone();
            let urls: Vec<String> = config.paths.table_url.clone();
            self.table_load_handle = Some(std::thread::spawn(move || {
                let tda = TableDataAccessor::new(&tablepath);
                let url_refs: Vec<&str> = urls.iter().map(|s| s.as_str()).collect();
                tda.update_table_data(&url_refs);
            }));
        }
    }

    /// Translates: loadSelectedTables()
    /// Blocking HTTP work runs on a background thread.
    pub fn load_selected_tables(&mut self) {
        if self.table_load_handle.is_some() {
            return;
        }
        self.commit();

        if let Some(ref config) = self.config {
            let _ = fs::create_dir_all(&config.paths.tablepath);

            let tablepath = config.paths.tablepath.clone();
            let selected: Vec<TableInfo> = self
                .tableurl_selected_items
                .iter()
                .filter_map(|&i| self.tableurl.get(i).cloned())
                .collect();
            let urls: Vec<String> = TableInfo::to_url_array(&selected);
            self.table_load_handle = Some(std::thread::spawn(move || {
                let tda = TableDataAccessor::new(&tablepath);
                let url_refs: Vec<&str> = urls.iter().map(|s| s.as_str()).collect();
                tda.update_table_data(&url_refs);
            }));
        }
    }

    /// Translates: loadNewTables()
    /// Blocking HTTP work runs on a background thread.
    pub fn load_new_tables(&mut self) {
        if self.table_load_handle.is_some() {
            return;
        }
        self.commit();

        if let Some(ref config) = self.config {
            let _ = fs::create_dir_all(&config.paths.tablepath);

            let tablepath = config.paths.tablepath.clone();
            let urls: Vec<String> = config.paths.table_url.clone();
            self.table_load_handle = Some(std::thread::spawn(move || {
                let tda = TableDataAccessor::new(&tablepath);
                let url_refs: Vec<&str> = urls.iter().map(|s| s.as_str()).collect();
                tda.load_new_table_data(&url_refs);
            }));
        }
    }

    /// Translates: addSongPath()
    pub fn add_song_path(&mut self, main: &mut PlayConfigurationView) {
        // DirectoryChooser chooser = new DirectoryChooser();
        // chooser.setTitle("...");
        // File f = chooser.showDialog(null);
        if let Some(dir) = show_directory_chooser(
            "\u{697d}\u{66f2}\u{306e}\u{30eb}\u{30fc}\u{30c8}\u{30d5}\u{30a9}\u{30eb}\u{30c0}\u{3092}\u{9078}\u{629e}\u{3057}\u{3066}\u{304f}\u{3060}\u{3055}\u{3044}",
        ) {
            // final String defaultPath = new File(".").getAbsoluteFile().getParent() + File.separatorChar;
            let default_path = std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
                + std::path::MAIN_SEPARATOR_STR;
            // String targetPath = f.getAbsolutePath();
            let target_path_full = Path::new(&dir)
                .canonicalize()
                .unwrap_or_else(|_| dir.clone().into());
            let target_path_str = target_path_full.to_string_lossy().to_string();
            // if(targetPath.startsWith(defaultPath)) { targetPath = ...; }
            let target_path = if target_path_str.starts_with(&default_path) {
                target_path_str[default_path.len()..].to_string()
            } else {
                target_path_str
            };

            // boolean unique = true;
            let mut unique = true;
            // for (String path : bmsroot.getItems()) { ... }
            for path in &self.bmsroot {
                if path == &target_path
                    || target_path.starts_with(&format!("{}{}", path, std::path::MAIN_SEPARATOR))
                    || path.starts_with(&format!("{}{}", target_path, std::path::MAIN_SEPARATOR))
                {
                    unique = false;
                    break;
                }
            }
            // if (unique) { bmsroot.getItems().add(targetPath); main.loadBMSPath(targetPath); }
            if unique {
                self.bmsroot.push(target_path.clone());
                main.load_bms_path(&target_path);
            }
        }
    }

    /// Translates: songPathDragDropped(DragEvent)
    /// In egui, drag-drop is handled differently. This method takes a list of paths.
    pub fn song_path_drag_dropped(&mut self, paths: &[String], main: &mut PlayConfigurationView) {
        // if (db.hasFiles()) { for (File f : db.getFiles()) { if (f.isDirectory()) { ... } } }
        for dir in paths {
            let p = Path::new(dir);
            if p.is_dir() {
                let default_path = std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
                    + std::path::MAIN_SEPARATOR_STR;
                let target_path_full = p.canonicalize().unwrap_or_else(|_| dir.clone().into());
                let target_path_str = target_path_full.to_string_lossy().to_string();
                let target_path = if target_path_str.starts_with(&default_path) {
                    target_path_str[default_path.len()..].to_string()
                } else {
                    target_path_str
                };

                let mut unique = true;
                for path in &self.bmsroot {
                    if path == &target_path
                        || target_path.starts_with(&format!(
                            "{}{}",
                            path,
                            std::path::MAIN_SEPARATOR
                        ))
                        || path.starts_with(&format!(
                            "{}{}",
                            target_path,
                            std::path::MAIN_SEPARATOR
                        ))
                    {
                        unique = false;
                        break;
                    }
                }
                if unique {
                    self.bmsroot.push(target_path.clone());
                    main.load_bms_path(&target_path);
                }
            }
        }
    }

    /// Translates: removeSongPath()
    pub fn remove_song_path(&mut self) {
        // ObservableList<String> removingItem = bmsroot.getSelectionModel().getSelectedItems();
        // if (removingItem.contains(downloadDirectory)) { Alert...; return; }
        if self
            .bmsroot_selected_items
            .contains(&self.download_directory)
        {
            error!("You cannot remove the download directory!");
            return;
        }
        // bmsroot.getItems().removeAll(removingItem);
        let removing: HashSet<&String> = self.bmsroot_selected_items.iter().collect();
        self.bmsroot.retain(|item| !removing.contains(item));
    }

    /// Translates: markAsDownloadDirectory()
    pub fn mark_as_download_directory(&mut self) {
        // downloadDirectory = bmsroot.getSelectionModel().getSelectedItem();
        if let Some(ref selected) = self.bmsroot_selected_item {
            self.download_directory = selected.clone();
        }
        // bmsroot.refresh();
        // (Refresh deferred to egui rendering)
    }

    /// Translates: addTableURL()
    pub fn add_table_url(&mut self) {
        // String s = url.getText();
        let s = self.url.clone();
        // if (s.startsWith("http") && !tableurl.getItems().contains(s)) {
        if s.starts_with("http") {
            let already_exists = self.tableurl.iter().any(|t| t.url == s);
            if !already_exists {
                // tableurl.addItem(new TableInfo(url.getText()));
                self.tableurl.push(TableInfo::new(&s));
            }
        }
    }

    /// Translates: removeTableURL()
    pub fn remove_table_url(&mut self) {
        // if (tableurl.getSelectionModel().getSelectedItems().isEmpty()) {
        //     available_tables.removeSelectedItems();
        // } else {
        //     tableurl.removeSelectedItems();
        // }
        if self.tableurl_selected_items.is_empty() {
            // Remove from available_tables
            let mut indices = self.available_tables_selected_items.clone();
            indices.sort_unstable();
            indices.reverse();
            for idx in indices {
                if idx < self.available_tables.len() {
                    self.available_tables.remove(idx);
                }
            }
            self.available_tables_selected_items.clear();
        } else {
            // Remove from tableurl
            let mut indices = self.tableurl_selected_items.clone();
            indices.sort_unstable();
            indices.reverse();
            for idx in indices {
                if idx < self.tableurl.len() {
                    self.tableurl.remove(idx);
                }
            }
            self.tableurl_selected_items.clear();
        }
    }

    /// Translates: moveTableURLUp()
    pub fn move_table_url_up(&mut self) {
        if self.tableurl_selected_items.is_empty() {
            Self::move_selected_items_up(
                &mut self.available_tables,
                &mut self.available_tables_selected_items,
            );
        } else {
            Self::move_selected_items_up(&mut self.tableurl, &mut self.tableurl_selected_items);
        }
    }

    /// Translates: moveTableURLDown()
    pub fn move_table_url_down(&mut self) {
        if self.tableurl_selected_items.is_empty() {
            Self::move_selected_items_down(
                &mut self.available_tables,
                &mut self.available_tables_selected_items,
            );
        } else {
            Self::move_selected_items_down(&mut self.tableurl, &mut self.tableurl_selected_items);
        }
    }

    /// Translates: moveTableURLIn()
    pub fn move_table_url_in(&mut self) {
        // transferSelection(available_tables, tableurl);
        Self::transfer_selection(
            &mut self.available_tables,
            &mut self.available_tables_selected_items,
            &mut self.tableurl,
        );
    }

    /// Translates: moveTableURLOut()
    pub fn move_table_url_out(&mut self) {
        // transferSelection(tableurl, available_tables);
        Self::transfer_selection(
            &mut self.tableurl,
            &mut self.tableurl_selected_items,
            &mut self.available_tables,
        );
    }

    /// Translates: transferSelection(EditableTableView<T> source, EditableTableView<T> destination)
    fn transfer_selection(
        source: &mut Vec<TableInfo>,
        source_selected: &mut Vec<usize>,
        destination: &mut Vec<TableInfo>,
    ) {
        // List<T> copy = new ArrayList<T>(selection);
        // Collections.reverse(copy);
        let mut selected_items: Vec<TableInfo> = source_selected
            .iter()
            .filter_map(|&i| source.get(i).cloned())
            .collect();
        selected_items.reverse();
        // for (T item : copy) { destination.getItems().add(0, item); }
        for item in selected_items {
            destination.insert(0, item);
        }
        // source.removeSelectedItems();
        let mut indices = source_selected.clone();
        indices.sort_unstable();
        indices.reverse();
        for idx in indices {
            if idx < source.len() {
                source.remove(idx);
            }
        }
        // source.getSelectionModel().clearSelection();
        source_selected.clear();
    }

    /// Translates: addUniqueTable(String[], String[])
    /// Adds unique elements of the latter to the former.
    pub fn add_unique_table(former: &[String], latter: &[String]) -> Vec<String> {
        let former_set: HashSet<&String> = former.iter().collect();
        let mut result: Vec<String> = former.to_vec();
        for url in latter {
            if !former_set.contains(url) {
                result.push(url.clone());
            }
        }
        result
    }

    /// Translates: subtractTable(String[], String[])
    /// Subtract members of the latter from the former.
    pub fn subtract_table(former: &[String], latter: &[String]) -> Vec<String> {
        let latter_set: HashSet<&String> = latter.iter().collect();
        former
            .iter()
            .filter(|url| !latter_set.contains(url))
            .cloned()
            .collect()
    }

    /// Helper: move selected items up in a list
    fn move_selected_items_up(list: &mut [TableInfo], selected: &mut [usize]) {
        selected.sort_unstable();
        for sel in selected.iter_mut() {
            if *sel > 0 {
                list.swap(*sel, *sel - 1);
                *sel -= 1;
            }
        }
    }

    /// Helper: move selected items down in a list
    fn move_selected_items_down(list: &mut [TableInfo], selected: &mut [usize]) {
        selected.sort_unstable();
        selected.reverse();
        for sel in selected.iter_mut() {
            if *sel < list.len().saturating_sub(1) {
                list.swap(*sel, *sel + 1);
                *sel += 1;
            }
        }
    }

    /// Helper: check if a path is the download directory
    pub fn is_download_directory(&self, path: &str) -> bool {
        let entry_abs = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(path).to_path_buf());
        let download_abs = Path::new(&self.download_directory)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(&self.download_directory).to_path_buf());
        entry_abs == download_abs
    }

    /// Copy selected URLs to clipboard (Ctrl+C handler helper)
    pub fn copy_tableurl_selection_to_clipboard(&self) {
        let selection: String = self
            .tableurl_selected_items
            .iter()
            .filter_map(|&i| self.tableurl.get(i))
            .map(|t| t.url.clone())
            .collect::<Vec<_>>()
            .join("\n");
        crate::platform::copy_to_clipboard(&selection);
    }

    /// Copy selected available table URLs to clipboard (Ctrl+C handler helper)
    pub fn copy_available_tables_selection_to_clipboard(&self) {
        let selection: String = self
            .available_tables_selected_items
            .iter()
            .filter_map(|&i| self.available_tables.get(i))
            .map(|t| t.url.clone())
            .collect::<Vec<_>>()
            .join("\n");
        crate::platform::copy_to_clipboard(&selection);
    }

    /// Render the resource configuration UI.
    /// Translates JavaFX FXML layout to egui widgets.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.poll_table_load();
        ui.heading("Resource Configuration");

        // --- BMS Root Paths ---
        ui.separator();
        ui.label("BMS Root Paths:");

        ui.horizontal(|ui| {
            if ui.button("Add").clicked() {
                // Trigger add_song_path externally (requires PlayConfigurationView reference)
            }
            if ui.button("Remove").clicked() {
                self.remove_song_path();
            }
            if ui.button("Set as Download Dir").clicked() {
                self.mark_as_download_directory();
            }
        });

        egui::ScrollArea::vertical()
            .id_salt("bmsroot_list_scroll")
            .max_height(120.0)
            .show(ui, |ui| {
                for (i, path) in self.bmsroot.iter().enumerate() {
                    let selected = self.bmsroot_selected_item.as_deref() == Some(path.as_str());
                    let label = if self.is_download_directory(path) {
                        format!("{} [Download]", path)
                    } else {
                        path.clone()
                    };
                    if ui.selectable_label(selected, &label).clicked() {
                        self.bmsroot_selected_item = Some(path.clone());
                        if !self.bmsroot_selected_items.contains(path) {
                            self.bmsroot_selected_items.clear();
                            self.bmsroot_selected_items.push(path.clone());
                        }
                    }
                    let _ = i;
                }
            });

        // --- Update Song checkbox ---
        ui.checkbox(&mut self.updatesong, "Update songs on startup");

        // --- Table URL management ---
        ui.separator();
        ui.label("Table URL:");

        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.url);
            if ui.button("Add URL").clicked() {
                self.add_table_url();
            }
        });

        // --- Selected Tables ---
        ui.separator();
        ui.label("Selected Tables:");

        ui.horizontal(|ui| {
            if ui.button("Remove").clicked() {
                self.remove_table_url();
            }
            if ui.button("Up").clicked() {
                self.move_table_url_up();
            }
            if ui.button("Down").clicked() {
                self.move_table_url_down();
            }
            if ui.button("<<").clicked() {
                self.move_table_url_out();
            }
            let loading = self.is_table_loading();
            if ui
                .add_enabled(!loading, egui::Button::new("Load All"))
                .clicked()
            {
                self.load_all_tables();
            }
            if ui
                .add_enabled(!loading, egui::Button::new("Load Selected"))
                .clicked()
            {
                self.load_selected_tables();
            }
            if ui
                .add_enabled(!loading, egui::Button::new("Load New"))
                .clicked()
            {
                self.load_new_tables();
            }
            if loading {
                ui.spinner();
            }
            if ui.button("Refresh Info").clicked() {
                self.refresh_local_table_info();
            }
        });

        egui::ScrollArea::vertical()
            .id_salt("tableurl_scroll")
            .max_height(150.0)
            .show(ui, |ui| {
                egui::Grid::new("tableurl_grid")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Name/Status");
                        ui.label("Comment");
                        ui.label("URL");
                        ui.end_row();

                        let mut clicked_index = None;
                        for (i, info) in self.tableurl.iter().enumerate() {
                            let selected = self.tableurl_selected_items.contains(&i);
                            if ui.selectable_label(selected, &info.name_status).clicked() {
                                clicked_index = Some(i);
                            }
                            ui.label(&info.comment);
                            ui.label(&info.url);
                            ui.end_row();
                        }
                        if let Some(i) = clicked_index {
                            if ui.input(|inp| inp.modifiers.ctrl || inp.modifiers.command) {
                                if self.tableurl_selected_items.contains(&i) {
                                    self.tableurl_selected_items.retain(|&x| x != i);
                                } else {
                                    self.tableurl_selected_items.push(i);
                                }
                            } else {
                                self.tableurl_selected_items.clear();
                                self.tableurl_selected_items.push(i);
                            }
                        }
                    });
            });

        // --- Available Tables ---
        ui.separator();
        ui.label("Available Tables:");

        ui.horizontal(|ui| {
            if ui.button(">>").clicked() {
                self.move_table_url_in();
            }
        });

        egui::ScrollArea::vertical()
            .id_salt("available_tables_scroll")
            .max_height(150.0)
            .show(ui, |ui| {
                egui::Grid::new("available_tables_grid")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Name/Status");
                        ui.label("Comment");
                        ui.label("URL");
                        ui.end_row();

                        let mut clicked_index = None;
                        for (i, info) in self.available_tables.iter().enumerate() {
                            let selected = self.available_tables_selected_items.contains(&i);
                            if ui.selectable_label(selected, &info.name_status).clicked() {
                                clicked_index = Some(i);
                            }
                            ui.label(&info.comment);
                            ui.label(&info.url);
                            ui.end_row();
                        }
                        if let Some(i) = clicked_index {
                            if ui.input(|inp| inp.modifiers.ctrl || inp.modifiers.command) {
                                if self.available_tables_selected_items.contains(&i) {
                                    self.available_tables_selected_items.retain(|&x| x != i);
                                } else {
                                    self.available_tables_selected_items.push(i);
                                }
                            } else {
                                self.available_tables_selected_items.clear();
                                self.available_tables_selected_items.push(i);
                            }
                        }
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- add_unique_table --

    #[test]
    fn add_unique_table_both_empty() {
        let result = ResourceConfigurationView::add_unique_table(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn add_unique_table_former_empty() {
        let latter = vec!["a".to_string(), "b".to_string()];
        let result = ResourceConfigurationView::add_unique_table(&[], &latter);
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn add_unique_table_latter_empty() {
        let former = vec!["a".to_string(), "b".to_string()];
        let result = ResourceConfigurationView::add_unique_table(&former, &[]);
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn add_unique_table_no_overlap() {
        let former = vec!["a".to_string(), "b".to_string()];
        let latter = vec!["c".to_string(), "d".to_string()];
        let result = ResourceConfigurationView::add_unique_table(&former, &latter);
        assert_eq!(result, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn add_unique_table_full_overlap() {
        let former = vec!["a".to_string(), "b".to_string()];
        let latter = vec!["a".to_string(), "b".to_string()];
        let result = ResourceConfigurationView::add_unique_table(&former, &latter);
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn add_unique_table_partial_overlap() {
        let former = vec!["a".to_string(), "b".to_string()];
        let latter = vec!["b".to_string(), "c".to_string()];
        let result = ResourceConfigurationView::add_unique_table(&former, &latter);
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn add_unique_table_latter_has_duplicates() {
        let former = vec!["a".to_string()];
        let latter = vec!["b".to_string(), "b".to_string()];
        let result = ResourceConfigurationView::add_unique_table(&former, &latter);
        // Both "b" entries are added since neither is in former_set
        assert_eq!(result, vec!["a", "b", "b"]);
    }

    // -- subtract_table --

    #[test]
    fn subtract_table_both_empty() {
        let result = ResourceConfigurationView::subtract_table(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn subtract_table_former_empty() {
        let latter = vec!["a".to_string(), "b".to_string()];
        let result = ResourceConfigurationView::subtract_table(&[], &latter);
        assert!(result.is_empty());
    }

    #[test]
    fn subtract_table_latter_empty() {
        let former = vec!["a".to_string(), "b".to_string()];
        let result = ResourceConfigurationView::subtract_table(&former, &[]);
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn subtract_table_no_overlap() {
        let former = vec!["a".to_string(), "b".to_string()];
        let latter = vec!["c".to_string(), "d".to_string()];
        let result = ResourceConfigurationView::subtract_table(&former, &latter);
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn subtract_table_full_overlap() {
        let former = vec!["a".to_string(), "b".to_string()];
        let latter = vec!["a".to_string(), "b".to_string()];
        let result = ResourceConfigurationView::subtract_table(&former, &latter);
        assert!(result.is_empty());
    }

    #[test]
    fn subtract_table_partial_overlap() {
        let former = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let latter = vec!["b".to_string()];
        let result = ResourceConfigurationView::subtract_table(&former, &latter);
        assert_eq!(result, vec!["a", "c"]);
    }

    #[test]
    fn subtract_table_former_has_duplicates() {
        let former = vec!["a".to_string(), "a".to_string(), "b".to_string()];
        let latter = vec!["a".to_string()];
        let result = ResourceConfigurationView::subtract_table(&former, &latter);
        assert_eq!(result, vec!["b"]);
    }
}

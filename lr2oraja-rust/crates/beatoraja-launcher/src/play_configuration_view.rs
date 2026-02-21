// PlayConfigurationView.java -> play_configuration_view.rs
// Mechanical line-by-line translation.

use log::{info, warn};

use beatoraja_core::config::Config;
use beatoraja_core::player_config::PlayerConfig;
use bms_model::mode::Mode;
use md_processor::http_download_processor::DOWNLOAD_SOURCES;

use crate::audio_configuration_view::AudioConfigurationView;
use crate::discord_configuration_view::DiscordConfigurationView;
use crate::input_configuration_view::InputConfigurationView;
use crate::ir_configuration_view::IRConfigurationView;
use crate::music_select_configuration_view::MusicSelectConfigurationView;
use crate::obs_configuration_view::ObsConfigurationView;
use crate::resource_configuration_view::ResourceConfigurationView;
use crate::skin_configuration_view::SkinConfigurationView;
use crate::stream_editor_view::StreamEditorView;
use crate::stubs::{
    BMSPlayerMode, MainLoader, SongDatabaseUpdateListener, TwitterAuth, Version,
};
use crate::table_editor_view::TableEditorView;
use crate::trainer_view::TrainerView;
use crate::video_configuration_view::VideoConfigurationView;

/// PlayMode enum
/// Translated from PlayConfigurationView.PlayMode
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, non_camel_case_types)]
pub enum PlayMode {
    BEAT_5K,
    BEAT_7K,
    BEAT_10K,
    BEAT_14K,
    POPN_9K,
    KEYBOARD_24K,
    KEYBOARD_24K_DOUBLE,
}

impl PlayMode {
    #[allow(dead_code)]
    pub fn display_name(&self) -> &'static str {
        match self {
            PlayMode::BEAT_5K => "5KEYS",
            PlayMode::BEAT_7K => "7KEYS",
            PlayMode::BEAT_10K => "10KEYS",
            PlayMode::BEAT_14K => "14KEYS",
            PlayMode::POPN_9K => "9KEYS",
            PlayMode::KEYBOARD_24K => "24KEYS",
            PlayMode::KEYBOARD_24K_DOUBLE => "24KEYS DOUBLE",
        }
    }

    #[allow(dead_code)]
    pub fn to_mode(&self) -> Mode {
        match self {
            PlayMode::BEAT_5K => Mode::BEAT_5K,
            PlayMode::BEAT_7K => Mode::BEAT_7K,
            PlayMode::BEAT_10K => Mode::BEAT_10K,
            PlayMode::BEAT_14K => Mode::BEAT_14K,
            PlayMode::POPN_9K => Mode::POPN_9K,
            PlayMode::KEYBOARD_24K => Mode::KEYBOARD_24K,
            PlayMode::KEYBOARD_24K_DOUBLE => Mode::KEYBOARD_24K_DOUBLE,
        }
    }

    #[allow(dead_code)]
    pub fn values() -> Vec<PlayMode> {
        vec![
            PlayMode::BEAT_5K,
            PlayMode::BEAT_7K,
            PlayMode::BEAT_10K,
            PlayMode::BEAT_14K,
            PlayMode::POPN_9K,
            PlayMode::KEYBOARD_24K,
            PlayMode::KEYBOARD_24K_DOUBLE,
        ]
    }
}

impl std::fmt::Display for PlayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// OptionListCell — translates the JavaFX ListCell<Integer>
/// In egui, we just store the label mapping.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct OptionListCell {
    pub strings: Vec<String>,
}

#[allow(dead_code)]
impl OptionListCell {
    pub fn new(strings: Vec<String>) -> Self {
        OptionListCell { strings }
    }

    pub fn get_text(&self, index: Option<i32>) -> String {
        if let Some(idx) = index {
            if idx >= 0 && (idx as usize) < self.strings.len() {
                return self.strings[idx as usize].clone();
            }
        }
        String::new()
    }
}

/// Beatoraja configuration dialog
///
/// Translated from PlayConfigurationView.java
#[allow(dead_code)]
pub struct PlayConfigurationView {
    // UI fields (JavaFX widgets → egui state)
    pub newversion_text: String,
    pub newversion_url: Option<String>,

    // Player selector
    pub players: Vec<String>,
    pub players_selected: Option<String>,
    pub playername: String,

    // Play config selector
    pub playconfig: Option<PlayMode>,

    // Hi-speed
    pub hispeed: f64,

    // Layout spacing (grid)
    pub lr2configuration_hgap: f64,
    pub lr2configuration_vgap: f64,
    pub lr2configurationassist_hgap: f64,
    pub lr2configurationassist_vgap: f64,

    // Fix hispeed
    pub fixhispeed: Option<i32>,
    pub gvalue: i32,
    pub enable_constant: bool,
    pub const_fadein_time: i32,
    pub hispeedmargin: f64,
    pub hispeedautoadjust: bool,

    // Score options
    pub scoreop: Option<i32>,
    pub scoreop2: Option<i32>,
    pub doubleop: Option<i32>,
    pub gaugeop: Option<i32>,
    pub lntype: Option<i32>,

    // Lane cover
    pub enable_lanecover: bool,
    pub lanecover: i32,
    pub lanecovermarginlow: i32,
    pub lanecovermarginhigh: i32,
    pub lanecoverswitchduration: i32,
    pub enable_lift: bool,
    pub lift: i32,
    pub enable_hidden: bool,
    pub hidden: i32,

    // Paths
    pub bgmpath: String,
    pub soundpath: String,

    // Timing
    pub notesdisplaytiming: i32,
    pub notesdisplaytimingautoadjust: bool,
    pub bpmguide: bool,
    pub gaugeautoshift: Option<i32>,
    pub bottomshiftablegauge: Option<i32>,

    // Custom judge
    pub customjudge: bool,
    pub njudgepg: i32,
    pub njudgegr: i32,
    pub njudgegd: i32,
    pub sjudgepg: i32,
    pub sjudgegr: i32,
    pub sjudgegd: i32,

    // Mine/scroll/LN modes
    pub minemode: Option<i32>,
    pub scrollmode: Option<i32>,
    pub longnotemode: Option<i32>,
    pub forcedcnendings: bool,
    pub longnoterate: f64,
    pub hranthresholdbpm: i32,
    pub seventoninepattern: Option<i32>,
    pub seventoninetype: Option<i32>,
    pub exitpressduration: i32,
    pub chartpreview: bool,
    pub guidese: bool,
    pub windowhold: bool,
    pub extranotedepth: i32,

    // Visual options
    pub judgeregion: bool,
    pub markprocessednote: bool,
    pub showhiddennote: bool,
    pub showpastnote: bool,
    pub target: Vec<String>,
    pub target_selected: Option<String>,

    // Judge algorithm
    pub judgealgorithm: Option<i32>,

    // Auto save replay
    pub autosavereplay1: Option<i32>,
    pub autosavereplay2: Option<i32>,
    pub autosavereplay3: Option<i32>,
    pub autosavereplay4: Option<i32>,

    // CIM
    pub usecim: bool,

    // Twitter
    pub txt_twitter_consumer_key: String,
    pub txt_twitter_consumer_secret: String,
    pub txt_twitter_authenticated_visible: bool,
    pub txt_twitter_pin: String,
    pub twitter_pin_enabled: bool,
    pub twitter_auth_button_enabled: bool,

    // IPFS
    pub enable_ipfs: bool,
    pub ipfsurl: String,

    // HTTP download
    pub enable_http: bool,
    pub http_download_source: Vec<String>,
    pub http_download_source_selected: Option<String>,
    pub default_download_url: String,
    pub override_download_url: String,

    // Clipboard screenshot
    pub clipboard_screenshot: bool,

    // ComboBox option labels
    pub score_options_labels: Vec<String>,
    pub double_options_labels: Vec<String>,
    pub seven_to_nine_pattern_labels: Vec<String>,
    pub seven_to_nine_type_labels: Vec<String>,
    pub gauge_options_labels: Vec<String>,
    pub fixhispeed_labels: Vec<String>,
    pub lntype_labels: Vec<String>,
    pub gaugeautoshift_labels: Vec<String>,
    pub bottomshiftablegauge_labels: Vec<String>,
    pub minemode_labels: Vec<String>,
    pub scrollmode_labels: Vec<String>,
    pub longnotemode_labels: Vec<String>,
    pub judgealgorithm_labels: Vec<String>,
    pub autosave_labels: Vec<String>,

    // Sub-controllers
    pub video_controller: VideoConfigurationView,
    pub audio_controller: AudioConfigurationView,
    pub input_controller: InputConfigurationView,
    pub resource_controller: ResourceConfigurationView,
    pub music_select_controller: MusicSelectConfigurationView,
    pub skin_controller: SkinConfigurationView,
    pub ir_controller: IRConfigurationView,
    pub table_controller: TableEditorView,
    pub stream_controller: StreamEditorView,
    pub discord_controller: DiscordConfigurationView,
    pub obs_controller: ObsConfigurationView,
    pub trainer_controller: TrainerView,

    // Internal state
    config: Option<Config>,
    player: Option<PlayerConfig>,
    loader: Option<MainLoader>,
    song_updated: bool,
    request_token: Option<(String, String)>,
    pc: Option<PlayMode>,

    // Tab/panel disabled state
    pub player_panel_disabled: bool,
    pub video_tab_disabled: bool,
    pub audio_tab_disabled: bool,
    pub input_tab_disabled: bool,
    pub resource_tab_disabled: bool,
    pub option_tab_disabled: bool,
    pub other_tab_disabled: bool,
    pub ir_tab_disabled: bool,
    pub stream_tab_disabled: bool,
    pub discord_tab_disabled: bool,
    pub obs_tab_disabled: bool,
    pub control_panel_disabled: bool,
}

#[allow(dead_code)]
impl PlayConfigurationView {
    pub fn new() -> Self {
        PlayConfigurationView {
            newversion_text: String::new(),
            newversion_url: None,
            players: Vec::new(),
            players_selected: None,
            playername: String::new(),
            playconfig: None,
            hispeed: 1.0,
            lr2configuration_hgap: 25.0,
            lr2configuration_vgap: 4.0,
            lr2configurationassist_hgap: 25.0,
            lr2configurationassist_vgap: 4.0,
            fixhispeed: None,
            gvalue: 500,
            enable_constant: false,
            const_fadein_time: 100,
            hispeedmargin: 0.25,
            hispeedautoadjust: false,
            scoreop: None,
            scoreop2: None,
            doubleop: None,
            gaugeop: None,
            lntype: None,
            enable_lanecover: true,
            lanecover: 200,
            lanecovermarginlow: 1,
            lanecovermarginhigh: 10,
            lanecoverswitchduration: 500,
            enable_lift: false,
            lift: 100,
            enable_hidden: false,
            hidden: 100,
            bgmpath: String::new(),
            soundpath: String::new(),
            notesdisplaytiming: 0,
            notesdisplaytimingautoadjust: false,
            bpmguide: false,
            gaugeautoshift: None,
            bottomshiftablegauge: None,
            customjudge: false,
            njudgepg: 400,
            njudgegr: 400,
            njudgegd: 100,
            sjudgepg: 400,
            sjudgegr: 400,
            sjudgegd: 100,
            minemode: None,
            scrollmode: None,
            longnotemode: None,
            forcedcnendings: false,
            longnoterate: 1.0,
            hranthresholdbpm: 120,
            seventoninepattern: None,
            seventoninetype: None,
            exitpressduration: 1000,
            chartpreview: true,
            guidese: false,
            windowhold: false,
            extranotedepth: 0,
            judgeregion: false,
            markprocessednote: false,
            showhiddennote: false,
            showpastnote: false,
            target: Vec::new(),
            target_selected: None,
            judgealgorithm: None,
            autosavereplay1: None,
            autosavereplay2: None,
            autosavereplay3: None,
            autosavereplay4: None,
            usecim: false,
            txt_twitter_consumer_key: String::new(),
            txt_twitter_consumer_secret: String::new(),
            txt_twitter_authenticated_visible: false,
            txt_twitter_pin: String::new(),
            twitter_pin_enabled: false,
            twitter_auth_button_enabled: true,
            enable_ipfs: false,
            ipfsurl: String::new(),
            enable_http: false,
            http_download_source: Vec::new(),
            http_download_source_selected: None,
            default_download_url: String::new(),
            override_download_url: String::new(),
            clipboard_screenshot: false,
            score_options_labels: Vec::new(),
            double_options_labels: Vec::new(),
            seven_to_nine_pattern_labels: Vec::new(),
            seven_to_nine_type_labels: Vec::new(),
            gauge_options_labels: Vec::new(),
            fixhispeed_labels: Vec::new(),
            lntype_labels: Vec::new(),
            gaugeautoshift_labels: Vec::new(),
            bottomshiftablegauge_labels: Vec::new(),
            minemode_labels: Vec::new(),
            scrollmode_labels: Vec::new(),
            longnotemode_labels: Vec::new(),
            judgealgorithm_labels: Vec::new(),
            autosave_labels: Vec::new(),
            video_controller: VideoConfigurationView::default(),
            audio_controller: AudioConfigurationView::default(),
            input_controller: InputConfigurationView::default(),
            resource_controller: ResourceConfigurationView::new(),
            music_select_controller: MusicSelectConfigurationView::default(),
            skin_controller: SkinConfigurationView::new(),
            ir_controller: IRConfigurationView::default(),
            table_controller: TableEditorView::new(),
            stream_controller: StreamEditorView::default(),
            discord_controller: DiscordConfigurationView::default(),
            obs_controller: ObsConfigurationView::new(),
            trainer_controller: TrainerView::default(),
            config: None,
            player: None,
            loader: None,
            song_updated: false,
            request_token: None,
            pc: None,
            player_panel_disabled: false,
            video_tab_disabled: false,
            audio_tab_disabled: false,
            input_tab_disabled: false,
            resource_tab_disabled: false,
            option_tab_disabled: false,
            other_tab_disabled: false,
            ir_tab_disabled: false,
            stream_tab_disabled: false,
            discord_tab_disabled: false,
            obs_tab_disabled: false,
            control_panel_disabled: false,
        }
    }

    /// Initialize combo box labels (static helper)
    /// Translates: static void initComboBox(ComboBox<Integer> combo, final String[] values)
    pub fn init_combo_box_labels(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    /// Initialize the view
    /// Translates: public void initialize(URL arg0, ResourceBundle arg1)
    pub fn initialize(&mut self) {
        let t = std::time::Instant::now();
        self.lr2configuration_hgap = 25.0;
        self.lr2configuration_vgap = 4.0;
        self.lr2configurationassist_hgap = 25.0;
        self.lr2configurationassist_vgap = 4.0;

        let score_options = vec![
            "OFF", "MIRROR", "RANDOM", "R-RANDOM", "S-RANDOM", "SPIRAL", "H-RANDOM",
            "ALL-SCR", "RANDOM-EX", "S-RANDOM-EX",
        ];
        self.score_options_labels = Self::init_combo_box_labels(&score_options);

        self.double_options_labels =
            Self::init_combo_box_labels(&["OFF", "FLIP", "BATTLE", "BATTLE AS"]);

        self.seven_to_nine_pattern_labels = Self::init_combo_box_labels(&[
            "OFF",
            "SC1KEY2~8",
            "SC1KEY3~9",
            "SC2KEY3~9",
            "SC8KEY1~7",
            "SC9KEY1~7",
            "SC9KEY2~8",
        ]);

        // These would normally come from resource bundle, using defaults
        self.seven_to_nine_type_labels =
            Self::init_combo_box_labels(&["OFF", "NO MASHING", "ALTERNATION"]);

        self.gauge_options_labels = Self::init_combo_box_labels(&[
            "ASSIST EASY",
            "EASY",
            "NORMAL",
            "HARD",
            "EX-HARD",
            "HAZARD",
        ]);

        self.fixhispeed_labels = Self::init_combo_box_labels(&[
            "OFF",
            "START BPM",
            "MAX BPM",
            "MAIN BPM",
            "MIN BPM",
        ]);

        self.lntype_labels =
            Self::init_combo_box_labels(&["LONG NOTE", "CHARGE NOTE", "HELL CHARGE NOTE"]);

        self.gaugeautoshift_labels = Self::init_combo_box_labels(&[
            "NONE",
            "CONTINUE",
            "SURVIVAL TO GROOVE",
            "BEST CLEAR",
            "SELECT TO UNDER",
        ]);

        self.bottomshiftablegauge_labels =
            Self::init_combo_box_labels(&["ASSIST EASY", "EASY", "NORMAL"]);

        self.minemode_labels =
            Self::init_combo_box_labels(&["OFF", "REMOVE", "ADD RANDOM", "ADD NEAR", "ADD ALL"]);

        self.scrollmode_labels = Self::init_combo_box_labels(&["OFF", "REMOVE", "ADD"]);

        self.longnotemode_labels = Self::init_combo_box_labels(&[
            "OFF",
            "REMOVE",
            "ADD LN",
            "ADD CN",
            "ADD HCN",
            "ADD ALL",
        ]);

        // These would normally come from resource bundle
        self.judgealgorithm_labels =
            Self::init_combo_box_labels(&["LR2", "AC", "BOTTOM PRIORITY"]);

        self.autosave_labels = Self::init_combo_box_labels(&[
            "NONE",
            "BETTER SCORE",
            "BETTER OR SAME SCORE",
            "BETTER MISSCOUNT",
            "BETTER OR SAME MISSCOUNT",
            "BETTER COMBO",
            "BETTER OR SAME COMBO",
            "BETTER LAMP",
            "BETTER OR SAME LAMP",
            "BETTER ALL",
            "ALWAYS",
        ]);

        self.http_download_source = DOWNLOAD_SOURCES.keys().cloned().collect();

        // resourceController.init(this) → deferred to egui integration
        // discordController.init(this) → deferred to egui integration
        // obsController.init(this) → deferred to egui integration

        self.check_new_version();
        let elapsed = t.elapsed().as_millis();
        info!("Initialization time (ms): {}", elapsed);
    }

    /// Show what's new popup
    /// Translates: private void whatsNewPopup()
    pub fn whats_new_popup(&self) {
        todo!("egui integration")
    }

    /// Check for new version
    /// Translates: private void checkNewVersion()
    pub fn check_new_version(&mut self) {
        let version_checker = MainLoader::get_version_checker();
        let message = version_checker.get_message().to_string();
        let download_url = version_checker.get_download_url().map(|s| s.to_string());
        self.newversion_text = message;
        self.newversion_url = download_url;
    }

    /// Set BMS information loader
    /// Translates: public void setBMSInformationLoader(MainLoader loader)
    pub fn set_bms_information_loader(&mut self, loader: MainLoader) {
        self.loader = Some(loader);
    }

    /// Update dialog items
    /// Translates: public void update(Config config)
    pub fn update(&mut self, config: Config) {
        self.config = Some(config);
        let config = self.config.as_ref().unwrap();

        // Show the What's New popup upon version change
        let current_version = Version::get_version();
        let last_version = config.last_booted_version.clone();
        // If current version is greater than last version
        if Version::compare_to_string(&last_version) > 0 {
            self.whats_new_popup();
            if let Some(ref mut c) = self.config {
                c.last_booted_version = current_version;
            }
        }

        let config = self.config.as_ref().unwrap();
        let playerpath = config.playerpath.clone();
        self.players = beatoraja_core::player_config::read_all_player_id(&playerpath);

        // videoController.update(config) → todo
        // audioController.update(config.getAudioConfig()) → todo
        // musicselectController.update(config) → todo

        self.bgmpath = config.bgmpath.clone();
        self.soundpath = config.soundpath.clone();

        // resourceController.update(config) → todo
        // discordController.update(config) → todo
        // obsController.update(config) → todo
        // skinController.update(config) → todo

        self.usecim = config.cache_skin_image;
        self.clipboard_screenshot = config.set_clipboard_screenshot;

        self.enable_ipfs = config.enable_ipfs;
        self.ipfsurl = config.ipfsurl.clone();

        self.enable_http = config.enable_http;
        self.http_download_source_selected = Some(config.download_source.clone());
        self.default_download_url = config.default_download_url.clone();
        self.override_download_url = config.override_download_url.clone();

        let playername_config = config.playername.clone().unwrap_or_default();
        if self.players.contains(&playername_config) {
            self.players_selected = Some(playername_config);
        } else if !self.players.is_empty() {
            self.players_selected = Some(self.players[0].clone());
        }
        self.update_player();

        // tableController.init and update → todo
    }

    /// Change player
    /// Translates: public void changePlayer()
    pub fn change_player(&mut self) {
        self.commit_player();
        self.update_player();
    }

    /// Add player
    /// Translates: public void addPlayer()
    pub fn add_player(&mut self) {
        let config = match &self.config {
            Some(c) => c,
            None => return,
        };
        let ids = beatoraja_core::player_config::read_all_player_id(&config.playerpath);
        for i in 1..1000 {
            let playerid = format!("player{}", i);
            let mut b = true;
            for id in &ids {
                if *id == playerid {
                    b = false;
                    break;
                }
            }
            if b {
                let _ = beatoraja_core::player_config::create_player(&config.playerpath, &playerid);
                self.players.push(playerid);
                break;
            }
        }
    }

    /// Update player config into UI fields
    /// Translates: public void updatePlayer()
    pub fn update_player(&mut self) {
        let config = match &self.config {
            Some(c) => c,
            None => return,
        };
        let playerid = match &self.players_selected {
            Some(p) => p.clone(),
            None => return,
        };
        let player = match PlayerConfig::read_player_config(&config.playerpath, &playerid) {
            Ok(p) => p,
            Err(e) => {
                warn!("Player config failed to load: {}", e);
                PlayerConfig::default()
            }
        };

        self.playername = player.name.clone();

        // videoController.updatePlayer(player) → todo
        // musicselectController.updatePlayer(player) → todo

        self.scoreop = Some(player.random);
        self.scoreop2 = Some(player.random2);
        self.doubleop = Some(player.doubleoption);
        self.seventoninepattern = Some(player.seven_to_nine_pattern);
        self.seventoninetype = Some(player.seven_to_nine_type);
        self.exitpressduration = player.exit_press_duration;
        self.chartpreview = player.chart_preview;
        self.guidese = player.is_guide_se;
        self.windowhold = player.is_window_hold;
        self.gaugeop = Some(player.gauge);
        self.lntype = Some(player.lnmode);

        self.notesdisplaytiming = player.judgetiming;
        self.notesdisplaytimingautoadjust = player.notes_display_timing_auto_adjust;

        self.bpmguide = player.bpmguide;
        self.gaugeautoshift = Some(player.gauge_auto_shift);
        self.bottomshiftablegauge = Some(player.bottom_shiftable_gauge);

        self.customjudge = player.custom_judge;
        self.njudgepg = player.key_judge_window_rate_perfect_great;
        self.njudgegr = player.key_judge_window_rate_great;
        self.njudgegd = player.key_judge_window_rate_good;
        self.sjudgepg = player.scratch_judge_window_rate_perfect_great;
        self.sjudgegr = player.scratch_judge_window_rate_great;
        self.sjudgegd = player.scratch_judge_window_rate_good;
        self.minemode = Some(player.mine_mode);
        self.scrollmode = Some(player.scroll_mode);
        self.longnotemode = Some(player.longnote_mode);
        self.forcedcnendings = player.forcedcnendings;
        self.longnoterate = player.longnote_rate;
        self.hranthresholdbpm = player.hran_threshold_bpm;
        self.judgeregion = player.showjudgearea;
        self.markprocessednote = player.markprocessednote;
        self.extranotedepth = player.extranote_depth;

        if player.autosavereplay.len() >= 4 {
            self.autosavereplay1 = Some(player.autosavereplay[0]);
            self.autosavereplay2 = Some(player.autosavereplay[1]);
            self.autosavereplay3 = Some(player.autosavereplay[2]);
            self.autosavereplay4 = Some(player.autosavereplay[3]);
        }

        self.target = player.targetlist.clone();
        self.target_selected = Some(player.targetid.clone());
        self.showhiddennote = player.showhiddennote;
        self.showpastnote = player.showpastnote;

        // irController.update(player) → todo
        // streamController.update(player) → todo

        self.twitter_pin_enabled = false;
        if let Some(ref token) = player.twitter_access_token {
            if !token.is_empty() {
                self.txt_twitter_authenticated_visible = true;
            } else {
                self.txt_twitter_authenticated_visible = false;
            }
        } else {
            self.txt_twitter_authenticated_visible = false;
        }

        self.pc = None;
        self.playconfig = Some(PlayMode::BEAT_7K);
        self.player = Some(player);
        self.update_play_config();

        // inputController.update(player) → todo
        // skinController.update(player) → todo
    }

    /// Commit config to file
    /// Translates: public void commit()
    pub fn commit(&mut self) {
        // videoController.commit(config) → todo
        // audioController.commit() → todo
        // musicselectController.commit() → todo

        if let Some(ref mut config) = self.config {
            config.playername = self.players_selected.clone();

            config.bgmpath = self.bgmpath.clone();
            config.soundpath = self.soundpath.clone();

            // resourceController.commit() → todo
            // discordController.commit() → todo
            // obsController.commit() → todo

            config.cache_skin_image = self.usecim;

            config.enable_ipfs = self.enable_ipfs;
            config.ipfsurl = self.ipfsurl.clone();

            config.enable_http = self.enable_http;
            if let Some(ref source) = self.http_download_source_selected {
                config.download_source = source.clone();
            }
            config.override_download_url = self.override_download_url.clone();

            config.set_clipboard_screenshot = self.clipboard_screenshot;
        }

        self.commit_player();

        if let Some(ref config) = self.config {
            let _ = Config::write(config);
        }

        // tableController.commit() → todo
    }

    /// Commit player config
    /// Translates: public void commitPlayer()
    pub fn commit_player(&mut self) {
        if self.player.is_none() {
            return;
        }

        {
            let player = self.player.as_mut().unwrap();

            if !self.playername.is_empty() {
                player.name = self.playername.clone();
            }

            // videoController.commitPlayer(player) -> todo
            // musicselectController.commitPlayer() -> todo

            player.random = self.scoreop.unwrap_or(0);
            player.random2 = self.scoreop2.unwrap_or(0);
            player.doubleoption = self.doubleop.unwrap_or(0);
            player.seven_to_nine_pattern = self.seventoninepattern.unwrap_or(0);
            player.seven_to_nine_type = self.seventoninetype.unwrap_or(0);
            player.exit_press_duration = self.exitpressduration;
            player.chart_preview = self.chartpreview;
            player.is_guide_se = self.guidese;
            player.is_window_hold = self.windowhold;
            player.gauge = self.gaugeop.unwrap_or(0);
            player.lnmode = self.lntype.unwrap_or(0);
            player.judgetiming = self.notesdisplaytiming;
            player.notes_display_timing_auto_adjust = self.notesdisplaytimingautoadjust;

            player.bpmguide = self.bpmguide;
            player.gauge_auto_shift = self.gaugeautoshift.unwrap_or(0);
            player.bottom_shiftable_gauge = self.bottomshiftablegauge.unwrap_or(0);
            player.custom_judge = self.customjudge;
            player.key_judge_window_rate_perfect_great = self.njudgepg;
            player.key_judge_window_rate_great = self.njudgegr;
            player.key_judge_window_rate_good = self.njudgegd;
            player.scratch_judge_window_rate_perfect_great = self.sjudgepg;
            player.scratch_judge_window_rate_great = self.sjudgegr;
            player.scratch_judge_window_rate_good = self.sjudgegd;
            player.mine_mode = self.minemode.unwrap_or(0);
            player.scroll_mode = self.scrollmode.unwrap_or(0);
            player.longnote_mode = self.longnotemode.unwrap_or(0);
            player.forcedcnendings = self.forcedcnendings;
            player.longnote_rate = self.longnoterate;
            player.hran_threshold_bpm = self.hranthresholdbpm;
            player.markprocessednote = self.markprocessednote;
            player.extranote_depth = self.extranotedepth;

            player.autosavereplay = vec![
                self.autosavereplay1.unwrap_or(0),
                self.autosavereplay2.unwrap_or(0),
                self.autosavereplay3.unwrap_or(0),
                self.autosavereplay4.unwrap_or(0),
            ];

            player.showjudgearea = self.judgeregion;
            if let Some(ref target) = self.target_selected {
                player.targetid = target.clone();
            }

            player.showhiddennote = self.showhiddennote;
            player.showpastnote = self.showpastnote;
        }

        // inputController.commit() -> todo
        // irController.commit() -> todo
        // streamController.commit() -> todo

        self.update_play_config();
        // skinController.commit() -> todo

        if let (Some(config), Some(player)) = (&self.config, &self.player) {
            let _ = PlayerConfig::write(&config.playerpath, player);
        }
    }

    /// Add BGM path
    /// Translates: public void addBGMPath()
    pub fn add_bgm_path(&mut self) {
        if let Some(s) = crate::stubs::show_directory_chooser("Select BGM root folder") {
            self.bgmpath = s;
        }
    }

    /// Add sound path
    /// Translates: public void addSoundPath()
    pub fn add_sound_path(&mut self) {
        if let Some(s) = crate::stubs::show_directory_chooser("Select sound effect root folder") {
            self.soundpath = s;
        }
    }

    /// Show file chooser
    /// Translates: private String showFileChooser(String title)
    fn show_file_chooser(title: &str) -> Option<String> {
        crate::stubs::show_file_chooser(title)
    }

    /// Show directory chooser
    /// Translates: private String showDirectoryChooser(String title)
    fn show_directory_chooser(title: &str) -> Option<String> {
        crate::stubs::show_directory_chooser(title)
    }

    /// Update play config
    /// Translates: public void updatePlayConfig()
    pub fn update_play_config(&mut self) {
        let player = match &mut self.player {
            Some(p) => p,
            None => return,
        };

        if let Some(ref pc) = self.pc {
            let mode = pc.to_mode();
            let conf = &mut player.get_play_config(mode).playconfig;
            conf.hispeed = self.hispeed as f32;
            conf.duration = self.gvalue;
            conf.enable_constant = self.enable_constant;
            conf.constant_fadein_time = self.const_fadein_time;
            conf.hispeedmargin = self.hispeedmargin as f32;
            conf.fixhispeed = self.fixhispeed.unwrap_or(0);
            conf.enablelanecover = self.enable_lanecover;
            conf.lanecover = self.lanecover as f32 / 1000.0;
            conf.lanecovermarginlow = self.lanecovermarginlow as f32 / 1000.0;
            conf.lanecovermarginhigh = self.lanecovermarginhigh as f32 / 1000.0;
            conf.lanecoverswitchduration = self.lanecoverswitchduration;
            conf.enablelift = self.enable_lift;
            conf.enablehidden = self.enable_hidden;
            conf.lift = self.lift as f32 / 1000.0;
            conf.hidden = self.hidden as f32 / 1000.0;
            // judgealgorithm → judgetype
            // JudgeAlgorithm.values()[judgealgorithm.getValue()].name()
            if let Some(alg_idx) = self.judgealgorithm {
                let judge_algs =
                    beatoraja_core::stubs::JudgeAlgorithm::values();
                if (alg_idx as usize) < judge_algs.len() {
                    conf.judgetype = judge_algs[alg_idx as usize].name().to_string();
                }
            }
            conf.hispeedautoadjust = self.hispeedautoadjust;
        }

        self.pc = self.playconfig.clone();

        if let Some(ref pc) = self.pc {
            let mode = pc.to_mode();
            let conf = &player.get_play_config(mode).playconfig.clone();
            self.hispeed = conf.hispeed as f64;
            self.gvalue = conf.duration;
            self.enable_constant = conf.enable_constant;
            self.const_fadein_time = conf.constant_fadein_time;
            self.hispeedmargin = conf.hispeedmargin as f64;
            self.fixhispeed = Some(conf.fixhispeed);
            self.enable_lanecover = conf.enablelanecover;
            self.lanecover = (conf.lanecover * 1000.0) as i32;
            self.lanecovermarginlow = (conf.lanecovermarginlow * 1000.0) as i32;
            self.lanecovermarginhigh = (conf.lanecovermarginhigh * 1000.0) as i32;
            self.lanecoverswitchduration = conf.lanecoverswitchduration;
            self.enable_lift = conf.enablelift;
            self.enable_hidden = conf.enablehidden;
            self.lift = (conf.lift * 1000.0) as i32;
            self.hidden = (conf.hidden * 1000.0) as i32;
            self.judgealgorithm = Some(
                beatoraja_core::stubs::JudgeAlgorithm::get_index(&conf.judgetype).max(0),
            );
            self.hispeedautoadjust = conf.hispeedautoadjust;
        }
    }

    /// Start game
    /// Translates: public void start()
    pub fn start(&mut self) {
        self.commit();
        self.player_panel_disabled = true;
        self.video_tab_disabled = true;
        self.audio_tab_disabled = true;
        self.input_tab_disabled = true;
        self.resource_tab_disabled = true;
        self.option_tab_disabled = true;
        self.other_tab_disabled = true;
        self.ir_tab_disabled = true;
        self.stream_tab_disabled = true;
        self.discord_tab_disabled = true;
        self.obs_tab_disabled = true;
        self.control_panel_disabled = true;

        // Minimise the stage after start → todo!("egui integration")

        if let (Some(config), Some(player)) = (&self.config, &self.player) {
            MainLoader::play(
                None,
                BMSPlayerMode::Play,
                true,
                config,
                player,
                self.song_updated,
            );
        }
    }

    /// Load all BMS
    /// Translates: public void loadAllBMS()
    pub fn load_all_bms(&mut self) {
        self.commit();
        self.load_bms(None, true);
    }

    /// Load diff BMS
    /// Translates: public void loadDiffBMS()
    pub fn load_diff_bms(&mut self) {
        self.commit();
        self.load_bms(None, false);
    }

    /// Load BMS path
    /// Translates: public void loadBMSPath(String updatepath)
    pub fn load_bms_path(&mut self, updatepath: &str) {
        self.commit();
        self.load_bms(Some(updatepath.to_string()), false);
    }

    /// Load BMS and update song database
    /// Translates: public void loadBMS(String updatepath, boolean updateAll)
    pub fn load_bms(&mut self, _updatepath: Option<String>, _update_all: bool) {
        self.commit();

        // The Java version shows a progress bar dialog and runs the DB update on a separate thread.
        // In Rust, this will be an async operation with egui progress display.
        // For now, stub the actual loading.
        let _listener = SongDatabaseUpdateListener::default();

        // The actual song database update logic:
        // let songdb = MainLoader::get_score_database_accessor();
        // let infodb = if config.use_song_info { Some(SongInformationAccessor::new("songinfo.db")) } else { None };
        // songdb.update_song_datas(updatepath, &config.bmsroot, update_all, false, infodb, &listener);
        // self.song_updated = true;

        todo!("egui integration — progress bar and async BMS loading")
    }

    /// Import score data from LR2
    /// Translates: public void importScoreDataFromLR2()
    pub fn import_score_data_from_lr2(&mut self) {
        let _dir = match crate::stubs::show_file_chooser("Select LR2 score database") {
            Some(d) => d,
            None => return,
        };

        // The Java version uses JDBC + ScoreDatabaseAccessor + ScoreDataImporter.
        // These use different stub types across crates (beatoraja-core vs beatoraja-external).
        // Stubbed pending rusqlite integration and type unification.
        if let (Some(_config), Some(_player_selected)) =
            (&self.config, &self.players_selected)
        {
            todo!("LR2 score import via rusqlite - pending type unification")
        }
    }

    /// Start Twitter auth
    /// Translates: public void startTwitterAuth()
    pub fn start_twitter_auth(&mut self) {
        match TwitterAuth::start_auth(
            &self.txt_twitter_consumer_key,
            &self.txt_twitter_consumer_secret,
        ) {
            Ok((token, secret)) => {
                if let Some(ref mut player) = self.player {
                    player.twitter_consumer_key =
                        Some(self.txt_twitter_consumer_key.clone());
                    player.twitter_consumer_secret =
                        Some(self.txt_twitter_consumer_secret.clone());
                    player.twitter_access_token = Some(String::new());
                    player.twitter_access_token_secret = Some(String::new());
                }
                self.request_token = Some((token, secret));
                self.twitter_pin_enabled = true;
                self.txt_twitter_authenticated_visible = false;
                // Open browser with auth URL → todo
            }
            Err(e) => {
                warn!("Twitter auth error: {}", e);
            }
        }
    }

    /// Start PIN auth
    /// Translates: public void startPINAuth()
    pub fn start_pin_auth(&mut self) {
        let consumer_key = self
            .player
            .as_ref()
            .and_then(|p| p.twitter_consumer_key.clone())
            .unwrap_or_default();
        let consumer_secret = self
            .player
            .as_ref()
            .and_then(|p| p.twitter_consumer_secret.clone())
            .unwrap_or_default();

        if self.player.is_none() {
            return;
        }

        let request_token = self.request_token.clone();
        if let Some((ref token, ref secret)) = request_token {
            match TwitterAuth::complete_pin_auth(
                &consumer_key,
                &consumer_secret,
                token,
                secret,
                &self.txt_twitter_pin,
            ) {
                Ok((access_token, access_token_secret)) => {
                    if let Some(ref mut player) = self.player {
                        player.twitter_access_token = Some(access_token);
                        player.twitter_access_token_secret = Some(access_token_secret);
                    }
                    self.commit();
                    if let Some(config) = self.config.clone() {
                        self.update(config);
                    }
                }
                Err(e) => {
                    warn!("Twitter PIN auth error: {}", e);
                }
            }
        }
    }

    /// Exit
    /// Translates: public void exit()
    pub fn exit(&mut self) {
        self.commit();
        std::process::exit(0);
    }

    /// Render the UI
    /// In egui, this replaces the JavaFX FXML layout
    pub fn render(&mut self) {
        todo!("egui integration")
    }
}

impl Default for PlayConfigurationView {
    fn default() -> Self {
        Self::new()
    }
}

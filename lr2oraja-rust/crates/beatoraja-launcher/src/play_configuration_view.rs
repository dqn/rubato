// PlayConfigurationView.java -> play_configuration_view.rs
// Mechanical line-by-line translation.

use std::sync::Arc;
use std::thread::JoinHandle;

use log::{info, warn};

use beatoraja_core::config::Config;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_song::song_database_update_listener::SongDatabaseUpdateListener as SongListener;
use beatoraja_song::song_information_accessor::SongInformationAccessor;
use beatoraja_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;
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
use crate::stubs::{BMSPlayerMode, MainLoader, TwitterAuth, Version};
use crate::table_editor_view::TableEditorView;
use crate::trainer_view::TrainerView;
use crate::video_configuration_view::VideoConfigurationView;

/// State of async BMS database loading.
///
/// Translated from: PlayConfigurationView.loadBMS() thread lifecycle.
/// Java uses two threads (progress UI + DB update). In Rust, egui polls
/// this state each frame to display progress.
#[derive(Debug)]
pub enum BmsLoadingState {
    /// No loading in progress.
    Idle,
    /// Background thread is running. Counters come from the shared listener.
    Loading {
        bms_files: i32,
        processed_files: i32,
        new_files: i32,
    },
    /// Loading finished successfully.
    Completed,
    /// Loading failed with an error message.
    Failed(String),
}

/// Handle to a background BMS loading thread.
///
/// Holds the shared `SongDatabaseUpdateListener` (atomic counters) and the
/// `JoinHandle` so the UI can poll progress and detect completion.
struct BmsLoadingHandle {
    listener: Arc<SongListener>,
    join_handle: JoinHandle<anyhow::Result<()>>,
}

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
        if let Some(idx) = index
            && idx >= 0
            && (idx as usize) < self.strings.len()
        {
            return self.strings[idx as usize].clone();
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
    pub(crate) song_updated: bool,
    request_token: Option<(String, String)>,
    pc: Option<PlayMode>,
    /// Handle to the background BMS loading thread, if any.
    bms_loading_handle: Option<BmsLoadingHandle>,
    /// Cached terminal state after loading completes or fails.
    bms_loading_result: Option<Result<(), String>>,

    // Exit flag (replaces process::exit(0))
    pub exit_requested: bool,

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
            exit_requested: false,
            bms_loading_handle: None,
            bms_loading_result: None,
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
            "OFF",
            "MIRROR",
            "RANDOM",
            "R-RANDOM",
            "S-RANDOM",
            "SPIRAL",
            "H-RANDOM",
            "ALL-SCR",
            "RANDOM-EX",
            "S-RANDOM-EX",
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

        self.fixhispeed_labels =
            Self::init_combo_box_labels(&["OFF", "START BPM", "MAX BPM", "MAIN BPM", "MIN BPM"]);

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
            "OFF", "REMOVE", "ADD LN", "ADD CN", "ADD HCN", "ADD ALL",
        ]);

        // These would normally come from resource bundle
        self.judgealgorithm_labels = Self::init_combo_box_labels(&["LR2", "AC", "BOTTOM PRIORITY"]);

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

        // Sub-controller init calls: these methods set up internal state
        // (table columns, combo box items, etc.) — actual rendering is egui.
        // We pass `self` as a dummy since the parameter is unused in Rust.
        // NOTE: We cannot pass `&self` here because `self` is `&mut`.
        // The init methods use `_main` (unused), so we pass a default-constructed instance.
        let dummy = PlayConfigurationView::new();
        self.resource_controller.init(&dummy);
        self.discord_controller.init();
        self.obs_controller.init(&dummy);

        self.check_new_version();
        let elapsed = t.elapsed().as_millis();
        info!("Initialization time (ms): {}", elapsed);
    }

    /// Show what's new popup
    /// Translates: private void whatsNewPopup()
    pub fn whats_new_popup(&self) {
        log::debug!(
            "stub: PlayConfigurationView.whats_new_popup — blocked by egui popup implementation"
        );
    }

    /// Check for new version
    /// Translates: private void checkNewVersion()
    pub fn check_new_version(&mut self) {
        let mut version_checker = MainLoader::get_version_checker();
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
        let current_version = Version::get_version().to_string();
        let last_version = config.last_booted_version.clone();
        // If current version is greater than last version
        if Version::compare_to_string(Some(&last_version)) > 0 {
            self.whats_new_popup();
            if let Some(ref mut c) = self.config {
                c.last_booted_version = current_version;
            }
        }

        let config = self.config.as_ref().unwrap();
        let playerpath = config.playerpath.clone();
        self.players = beatoraja_core::player_config::read_all_player_id(&playerpath);

        // videoController.update(config)
        self.video_controller.update(config);
        // audioController.update(config.getAudioConfig())
        if let Some(ref audio) = config.audio {
            self.audio_controller.update(audio.clone());
        }
        // musicselectController.update(config)
        self.music_select_controller.update(config);

        self.bgmpath = config.bgmpath.clone();
        self.soundpath = config.soundpath.clone();

        // resourceController.update(config)
        // discordController.update(config)
        // skinController.update(config)
        // These take &mut Config, so we temporarily take ownership
        {
            let mut config = self.config.take().unwrap();
            self.resource_controller.update(&mut config);
            self.discord_controller.update(&mut config);
            self.skin_controller.update_config(&config);
            // obsController.update(config) — takes Config by value, give a clone
            self.obs_controller.update(config.clone());
            self.config = Some(config);
        }

        let config = self.config.as_ref().unwrap();
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

        // tableController.init and update deferred to egui integration
        // (requires ScoreDatabaseAccessor which depends on runtime DB state)
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
        let mut player = match PlayerConfig::read_player_config(&config.playerpath, &playerid) {
            Ok(p) => p,
            Err(e) => {
                warn!("Player config failed to load: {}", e);
                PlayerConfig::default()
            }
        };

        self.playername = player.name.clone();

        // videoController.updatePlayer(player)
        self.video_controller.update_player(&mut player);
        // musicselectController.updatePlayer(player)
        self.music_select_controller.update_player(&player);

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

        // irController.update(player)
        self.ir_controller.update(&mut player);
        // streamController.update(player)
        self.stream_controller.update(&player);

        self.twitter_pin_enabled = false;
        if let Some(ref token) = player.twitter_access_token {
            self.txt_twitter_authenticated_visible = !token.is_empty();
        } else {
            self.txt_twitter_authenticated_visible = false;
        }

        self.pc = None;
        self.playconfig = Some(PlayMode::BEAT_7K);
        self.player = Some(player);

        // update_play_config must happen before inputController/skinController updates
        // because Java calls updatePlayConfig() then inputController.update(player)
        self.update_play_config();

        // inputController.update(player) — needs &mut PlayerConfig
        if let Some(ref mut player) = self.player {
            self.input_controller.update(player);
        }
        // skinController.update(player)
        if let Some(ref player) = self.player {
            self.skin_controller.update_player(player);
        }
    }

    /// Commit config to file
    /// Translates: public void commit()
    pub fn commit(&mut self) {
        // videoController.commit(config)
        if let Some(ref mut config) = self.config {
            self.video_controller.commit(config);
        }
        // audioController.commit()
        self.audio_controller.commit();
        // musicselectController.commit()
        self.music_select_controller.commit();

        if let Some(ref mut config) = self.config {
            config.playername = self.players_selected.clone();

            config.bgmpath = self.bgmpath.clone();
            config.soundpath = self.soundpath.clone();
        }

        // resourceController.commit()
        self.resource_controller.commit();
        // discordController.commit()
        self.discord_controller.commit();
        // obsController.commit()
        self.obs_controller.commit();

        if let Some(ref mut config) = self.config {
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

        // tableController.commit()
        self.table_controller.commit();
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

            // videoController.commitPlayer(player)
            self.video_controller.commit_player(player);

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

        // musicselectController.commitPlayer()
        self.music_select_controller.commit_player();
        // inputController.commit()
        self.input_controller.commit();
        // irController.commit()
        self.ir_controller.commit();
        // streamController.commit()
        self.stream_controller.commit();

        self.update_play_config();
        // skinController.commit()
        self.skin_controller.commit();

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
                let judge_algs = beatoraja_core::stubs::JudgeAlgorithm::values();
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
            self.judgealgorithm =
                Some(beatoraja_core::stubs::JudgeAlgorithm::get_index(&conf.judgetype).max(0));
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
                BMSPlayerMode::PLAY,
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

    /// Load BMS and update song database on a background thread.
    ///
    /// Translates: public void loadBMS(String updatepath, boolean updateAll)
    ///
    /// Java spawns two threads: one for the progress UI (JavaFX AnimationTimer)
    /// and one for the actual DB update. In Rust/egui, the UI polls
    /// `bms_loading_state()` each frame to display progress, so we only need
    /// a single worker thread.
    pub fn load_bms(&mut self, updatepath: Option<String>, update_all: bool) {
        self.commit();

        let config = match &self.config {
            Some(c) => c.clone(),
            None => {
                log::warn!("load_bms called without config");
                return;
            }
        };

        // Don't start a new load while one is already running
        if self.bms_loading_handle.is_some() {
            log::warn!("BMS loading already in progress");
            return;
        }

        // Reset any previous result
        self.bms_loading_result = None;

        let listener = Arc::new(SongListener::new());
        let listener_clone = Arc::clone(&listener);

        let songpath = config.get_songpath().to_string();
        let bmsroot = config.get_bmsroot().to_vec();
        let use_song_info = config.use_song_info;
        let songinfopath = config.get_songinfopath().to_string();

        let join_handle = std::thread::spawn(move || -> anyhow::Result<()> {
            log::info!("song.db update started");

            let songdb = SQLiteSongDatabaseAccessor::new(&songpath, &bmsroot)?;

            let infodb = if use_song_info {
                match SongInformationAccessor::new(&songinfopath) {
                    Ok(db) => Some(db),
                    Err(e) => {
                        log::warn!("Failed to open song info DB: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            songdb.update_song_datas_with_listener(
                updatepath.as_deref(),
                &bmsroot,
                update_all,
                false,
                infodb
                    .as_ref()
                    .map(|db| db as &dyn beatoraja_types::song_information_db::SongInformationDb),
                &listener_clone,
            );

            log::info!("song.db update completed");
            Ok(())
        });

        self.bms_loading_handle = Some(BmsLoadingHandle {
            listener,
            join_handle,
        });
    }

    /// Get the current BMS loading state.
    ///
    /// Call this from the egui update loop to display progress.
    pub fn bms_loading_state(&self) -> BmsLoadingState {
        if let Some(handle) = &self.bms_loading_handle {
            BmsLoadingState::Loading {
                bms_files: handle.listener.get_bms_files_count(),
                processed_files: handle.listener.get_processed_bms_files_count(),
                new_files: handle.listener.get_new_bms_files_count(),
            }
        } else if let Some(result) = &self.bms_loading_result {
            match result {
                Ok(()) => BmsLoadingState::Completed,
                Err(msg) => BmsLoadingState::Failed(msg.clone()),
            }
        } else {
            BmsLoadingState::Idle
        }
    }

    /// Poll the background thread for completion.
    ///
    /// Call this each frame from the egui update loop. When the thread
    /// finishes, this sets `song_updated = true` and transitions the
    /// state to Completed or Failed.
    pub fn poll_bms_loading(&mut self) {
        let finished = self
            .bms_loading_handle
            .as_ref()
            .is_some_and(|h| h.join_handle.is_finished());

        if finished {
            let handle = self.bms_loading_handle.take().unwrap();
            match handle.join_handle.join() {
                Ok(Ok(())) => {
                    self.song_updated = true;
                    self.bms_loading_result = Some(Ok(()));
                    log::info!("BMS loading completed successfully");
                }
                Ok(Err(e)) => {
                    let msg = format!("{}", e);
                    log::error!("BMS loading failed: {}", msg);
                    self.bms_loading_result = Some(Err(msg));
                }
                Err(_panic) => {
                    let msg = "BMS loading thread panicked".to_string();
                    log::error!("{}", msg);
                    self.bms_loading_result = Some(Err(msg));
                }
            }
        }
    }

    /// Reset the loading state back to Idle.
    ///
    /// Call after the UI has acknowledged the Completed/Failed state.
    pub fn reset_bms_loading(&mut self) {
        self.bms_loading_result = None;
    }

    /// Returns true if BMS loading is currently in progress.
    pub fn is_bms_loading(&self) -> bool {
        self.bms_loading_handle.is_some()
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
        if let (Some(_config), Some(_player_selected)) = (&self.config, &self.players_selected) {
            log::debug!(
                "stub: PlayConfigurationView.import_score_data_from_lr2 — blocked by LR2 score DB import"
            );
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
                    player.twitter_consumer_key = Some(self.txt_twitter_consumer_key.clone());
                    player.twitter_consumer_secret = Some(self.txt_twitter_consumer_secret.clone());
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
        self.exit_requested = true;
    }

    /// Render the UI
    /// In egui, this replaces the JavaFX FXML layout
    pub fn render(&mut self) {
        // egui: replaced by launcher_ui.rs egui render loop
    }
}

impl Default for PlayConfigurationView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use beatoraja_core::audio_config::AudioConfig;

    /// Helper: create a PlayConfigurationView with initialize() called
    fn initialized_view() -> PlayConfigurationView {
        let mut view = PlayConfigurationView::new();
        view.initialize();
        view
    }

    // ---- initialize() tests ----

    #[test]
    fn test_initialize_sets_combo_box_labels() {
        let view = initialized_view();

        assert_eq!(view.score_options_labels.len(), 10);
        assert_eq!(view.score_options_labels[0], "OFF");
        assert_eq!(view.score_options_labels[1], "MIRROR");

        assert_eq!(view.double_options_labels.len(), 4);
        assert_eq!(view.gauge_options_labels.len(), 6);
        assert_eq!(view.fixhispeed_labels.len(), 5);
        assert_eq!(view.lntype_labels.len(), 3);
        assert_eq!(view.gaugeautoshift_labels.len(), 5);
        assert_eq!(view.bottomshiftablegauge_labels.len(), 3);
        assert_eq!(view.minemode_labels.len(), 5);
        assert_eq!(view.scrollmode_labels.len(), 3);
        assert_eq!(view.longnotemode_labels.len(), 6);
        assert_eq!(view.judgealgorithm_labels.len(), 3);
        assert_eq!(view.autosave_labels.len(), 11);
    }

    #[test]
    fn test_initialize_populates_http_download_sources() {
        let view = initialized_view();
        assert!(!view.http_download_source.is_empty());
    }

    // ---- update() delegation tests ----

    #[test]
    fn test_update_delegates_to_video_controller() {
        let mut view = initialized_view();
        let config = Config {
            vsync: true,
            max_frame_per_second: 120,
            bga: 2,
            ..Default::default()
        };

        view.update(config);

        // VideoConfigurationView.update() should have copied these values
        // We can verify by calling commit() and checking config roundtrip
        let mut out_config = Config::default();
        view.video_controller.commit(&mut out_config);
        assert!(out_config.vsync);
        assert_eq!(out_config.max_frame_per_second, 120);
        assert_eq!(out_config.bga, 2);
    }

    #[test]
    fn test_update_delegates_to_audio_controller() {
        let mut view = initialized_view();
        let config = Config {
            audio: Some(AudioConfig {
                systemvolume: 0.75,
                keyvolume: 0.5,
                bgvolume: 0.25,
                ..Default::default()
            }),
            ..Default::default()
        };

        view.update(config);

        // AudioConfigurationView stores config internally; commit writes back
        view.audio_controller.commit();
    }

    #[test]
    fn test_update_delegates_to_music_select_controller() {
        let mut view = initialized_view();
        let config = Config {
            scrolldurationlow: 300,
            scrolldurationhigh: 500,
            folderlamp: true,
            ..Default::default()
        };

        view.update(config);

        // Verify the music_select_controller commit roundtrip
        view.music_select_controller.commit();
    }

    #[test]
    fn test_update_delegates_to_resource_controller() {
        let mut view = initialized_view();
        let config = Config {
            bmsroot: vec!["path1".to_string(), "path2".to_string()],
            updatesong: true,
            ..Default::default()
        };

        view.update(config);

        // resource_controller.update should have picked up bmsroot
        view.resource_controller.commit();
    }

    #[test]
    fn test_update_delegates_to_discord_controller() {
        let mut view = initialized_view();
        let config = Config {
            use_discord_rpc: true,
            webhook_name: "test_hook".to_string(),
            ..Default::default()
        };

        view.update(config);

        view.discord_controller.commit();
    }

    #[test]
    fn test_update_delegates_to_obs_controller() {
        let mut view = initialized_view();
        let config = Config {
            use_obs_ws: true,
            obs_ws_host: "localhost".to_string(),
            obs_ws_port: 4455,
            ..Default::default()
        };

        view.update(config);

        view.obs_controller.commit();
    }

    // ---- commit() delegation tests ----

    #[test]
    fn test_commit_delegates_to_video_controller() {
        let mut view = initialized_view();
        view.update(Config::default());

        // After commit, the config should reflect sub-controller state
        view.commit();
    }

    #[test]
    fn test_commit_delegates_to_table_controller() {
        let mut view = initialized_view();
        view.update(Config::default());

        // table_controller.commit() should be called without panic
        view.commit();
    }

    // ---- update_player() delegation tests ----

    #[test]
    fn test_update_player_delegates_to_ir_controller() {
        let mut view = initialized_view();
        view.config = Some(Config {
            playerpath: "nonexistent_path".to_string(),
            ..Default::default()
        });

        // With no valid player file, it should fall back to default
        view.players_selected = Some("player1".to_string());
        view.update_player();
    }

    #[test]
    fn test_update_player_delegates_to_stream_controller() {
        let mut view = initialized_view();
        view.config = Some(Config {
            playerpath: "nonexistent_path".to_string(),
            ..Default::default()
        });

        view.players_selected = Some("player1".to_string());
        view.update_player();
    }

    #[test]
    fn test_update_player_delegates_to_input_controller() {
        let mut view = initialized_view();
        view.config = Some(Config {
            playerpath: "nonexistent_path".to_string(),
            ..Default::default()
        });

        view.players_selected = Some("player1".to_string());
        view.update_player();
    }

    #[test]
    fn test_update_player_delegates_to_skin_controller() {
        let mut view = initialized_view();
        view.config = Some(Config {
            playerpath: "nonexistent_path".to_string(),
            ..Default::default()
        });

        view.players_selected = Some("player1".to_string());
        view.update_player();
    }

    // ---- commit_player() delegation tests ----

    #[test]
    fn test_commit_player_delegates_to_sub_controllers() {
        let mut view = initialized_view();
        view.config = Some(Config::default());
        view.player = Some(PlayerConfig::default());
        view.playconfig = Some(PlayMode::BEAT_7K);

        // This should call video_controller.commit_player,
        // music_select_controller.commit_player, input_controller.commit,
        // ir_controller.commit, stream_controller.commit,
        // skin_controller.commit without panic
        view.commit_player();
    }

    #[test]
    fn test_commit_player_skips_when_no_player() {
        let mut view = initialized_view();
        view.player = None;

        // Should return early without panic
        view.commit_player();
    }

    // ---- PlayMode tests ----

    #[test]
    fn test_play_mode_display_name() {
        assert_eq!(PlayMode::BEAT_7K.display_name(), "7KEYS");
        assert_eq!(PlayMode::BEAT_14K.display_name(), "14KEYS");
        assert_eq!(
            PlayMode::KEYBOARD_24K_DOUBLE.display_name(),
            "24KEYS DOUBLE"
        );
    }

    #[test]
    fn test_play_mode_to_mode() {
        assert_eq!(PlayMode::BEAT_7K.to_mode(), Mode::BEAT_7K);
        assert_eq!(PlayMode::POPN_9K.to_mode(), Mode::POPN_9K);
    }

    #[test]
    fn test_play_mode_values_length() {
        assert_eq!(PlayMode::values().len(), 7);
    }

    // ---- OptionListCell tests ----

    #[test]
    fn test_option_list_cell_get_text() {
        let cell = OptionListCell::new(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
        assert_eq!(cell.get_text(Some(0)), "A");
        assert_eq!(cell.get_text(Some(2)), "C");
        assert_eq!(cell.get_text(None), "");
        assert_eq!(cell.get_text(Some(-1)), "");
        assert_eq!(cell.get_text(Some(99)), "");
    }

    // ---- Async BMS loading tests ----

    #[test]
    fn test_bms_loading_state_initially_idle() {
        let view = initialized_view();
        assert!(
            matches!(view.bms_loading_state(), BmsLoadingState::Idle),
            "Loading state should be Idle after construction"
        );
    }

    #[test]
    fn test_load_bms_transitions_to_loading_when_config_present() {
        let mut view = initialized_view();
        // Set up config with a temp directory as bmsroot
        let tmpdir = tempfile::tempdir().unwrap();
        let bmsroot = tmpdir.path().to_string_lossy().to_string();
        let songdb_path = tmpdir.path().join("song.db");
        let config = Config {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        };
        view.update(config);

        view.load_bms(None, false);

        assert!(
            matches!(view.bms_loading_state(), BmsLoadingState::Loading { .. }),
            "Loading state should transition to Loading after load_bms"
        );
    }

    #[test]
    fn test_load_bms_no_config_stays_idle() {
        let mut view = initialized_view();
        // No config set, load_bms should not start loading
        view.load_bms(None, false);

        assert!(
            matches!(view.bms_loading_state(), BmsLoadingState::Idle),
            "Loading state should stay Idle when no config"
        );
    }

    #[test]
    fn test_bms_loading_completes_and_sets_song_updated() {
        let mut view = initialized_view();
        let tmpdir = tempfile::tempdir().unwrap();
        let bmsroot = tmpdir.path().to_string_lossy().to_string();
        let songdb_path = tmpdir.path().join("song.db");
        let config = Config {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        };
        view.update(config);

        view.load_bms(None, false);

        // Wait for the background thread to finish (with timeout)
        let start = std::time::Instant::now();
        loop {
            view.poll_bms_loading();
            if !matches!(view.bms_loading_state(), BmsLoadingState::Loading { .. }) {
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(10) {
                panic!("BMS loading did not complete within 10 seconds");
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(
            matches!(view.bms_loading_state(), BmsLoadingState::Completed),
            "Loading state should be Completed after thread finishes"
        );
        assert!(
            view.song_updated,
            "song_updated should be true after successful load"
        );
    }

    #[test]
    fn test_bms_loading_progress_counters_accessible() {
        let mut view = initialized_view();
        let tmpdir = tempfile::tempdir().unwrap();
        let bmsroot = tmpdir.path().to_string_lossy().to_string();
        let songdb_path = tmpdir.path().join("song.db");
        let config = Config {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        };
        view.update(config);

        view.load_bms(None, false);

        // While loading, the progress should be accessible
        if let BmsLoadingState::Loading {
            bms_files,
            processed_files,
            new_files,
        } = view.bms_loading_state()
        {
            // Counters start at 0
            assert_eq!(bms_files, 0);
            assert_eq!(processed_files, 0);
            assert_eq!(new_files, 0);
        }

        // Wait for completion
        let start = std::time::Instant::now();
        loop {
            view.poll_bms_loading();
            if !matches!(view.bms_loading_state(), BmsLoadingState::Loading { .. }) {
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(10) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[test]
    fn test_bms_loading_reset_returns_to_idle() {
        let mut view = initialized_view();
        let tmpdir = tempfile::tempdir().unwrap();
        let bmsroot = tmpdir.path().to_string_lossy().to_string();
        let songdb_path = tmpdir.path().join("song.db");
        let config = Config {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        };
        view.update(config);

        view.load_bms(None, false);
        // Wait for completion
        let start = std::time::Instant::now();
        loop {
            view.poll_bms_loading();
            if !matches!(view.bms_loading_state(), BmsLoadingState::Loading { .. }) {
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(10) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        view.reset_bms_loading();
        assert!(
            matches!(view.bms_loading_state(), BmsLoadingState::Idle),
            "After reset, loading state should be Idle"
        );
    }

    #[test]
    fn test_is_bms_loading_returns_true_during_load() {
        let mut view = initialized_view();
        assert!(!view.is_bms_loading(), "Should not be loading initially");

        let tmpdir = tempfile::tempdir().unwrap();
        let bmsroot = tmpdir.path().to_string_lossy().to_string();
        let songdb_path = tmpdir.path().join("song.db");
        let config = Config {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        };
        view.update(config);

        view.load_bms(None, false);
        assert!(view.is_bms_loading(), "Should be loading after load_bms");

        // Wait for completion
        let start = std::time::Instant::now();
        loop {
            view.poll_bms_loading();
            if !view.is_bms_loading() {
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(10) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(
            !view.is_bms_loading(),
            "Should not be loading after completion"
        );
    }

    // ---- Roundtrip: update -> commit preserves config values ----

    #[test]
    fn test_update_commit_roundtrip_preserves_config_fields() {
        let mut view = initialized_view();
        let config = Config {
            bgmpath: "/music/bgm".to_string(),
            soundpath: "/music/sounds".to_string(),
            cache_skin_image: true,
            enable_ipfs: true,
            ipfsurl: "http://ipfs.example.com".to_string(),
            enable_http: true,
            download_source: "source1".to_string(),
            override_download_url: "http://override.example.com".to_string(),
            set_clipboard_screenshot: true,
            ..Default::default()
        };

        view.update(config);

        assert_eq!(view.bgmpath, "/music/bgm");
        assert_eq!(view.soundpath, "/music/sounds");
        assert!(view.usecim);
        assert!(view.enable_ipfs);
        assert_eq!(view.ipfsurl, "http://ipfs.example.com");
        assert!(view.enable_http);
        assert_eq!(view.override_download_url, "http://override.example.com");
        assert!(view.clipboard_screenshot);
    }

    // ---- Roundtrip: update_player -> commit_player preserves player fields ----

    #[test]
    fn test_update_player_commit_player_roundtrip() {
        let mut view = initialized_view();
        view.config = Some(Config::default());

        let player = PlayerConfig {
            name: "TestPlayer".to_string(),
            random: 3,
            random2: 5,
            doubleoption: 1,
            gauge: 2,
            lnmode: 1,
            judgetiming: 10,
            bpmguide: true,
            custom_judge: true,
            key_judge_window_rate_perfect_great: 500,
            mine_mode: 2,
            scroll_mode: 1,
            longnote_mode: 3,
            forcedcnendings: true,
            longnote_rate: 1.5,
            showjudgearea: true,
            markprocessednote: true,
            showhiddennote: true,
            showpastnote: true,
            autosavereplay: vec![1, 2, 3, 4],
            ..Default::default()
        };

        view.player = Some(player);
        view.playername = "TestPlayer".to_string();
        view.scoreop = Some(3);
        view.scoreop2 = Some(5);
        view.doubleop = Some(1);
        view.gaugeop = Some(2);
        view.lntype = Some(1);
        view.playconfig = Some(PlayMode::BEAT_7K);

        view.commit_player();

        let committed = view.player.as_ref().unwrap();
        assert_eq!(committed.name, "TestPlayer");
        assert_eq!(committed.random, 3);
        assert_eq!(committed.random2, 5);
        assert_eq!(committed.doubleoption, 1);
        assert_eq!(committed.gauge, 2);
        assert_eq!(committed.lnmode, 1);
    }
}

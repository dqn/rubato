use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::audio_config::AudioConfig;
use crate::player_config::PlayerConfig;
use crate::resolution::Resolution;
use crate::validatable::{Validatable, remove_empty_strings};

pub const SONGPATH_DEFAULT: &str = "songdata.db";
pub const SONGINFOPATH_DEFAULT: &str = "songinfo.db";
pub const TABLEPATH_DEFAULT: &str = "table";
pub const PLAYERPATH_DEFAULT: &str = "player";
pub const SKINPATH_DEFAULT: &str = "skin";
pub const DEFAULT_DOWNLOAD_DIRECTORY: &str = "http_download";

pub const BGA_ON: i32 = 0;
pub const BGA_AUTO: i32 = 1;
pub const BGA_OFF: i32 = 2;

pub const BGAEXPAND_FULL: i32 = 0;
pub const BGAEXPAND_KEEP_ASPECT_RATIO: i32 = 1;
pub const BGAEXPAND_OFF: i32 = 2;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
pub enum DisplayMode {
    FULLSCREEN,
    BORDERLESS,
    #[default]
    WINDOW,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
pub enum SongPreview {
    NONE,
    ONCE,
    #[default]
    LOOP,
}

pub static AVAILABLE_TABLEURL: &[&str] = &[
    // stardust, starlight, satellite, stella
    "https://mqppppp.neocities.org/StardustTable.html",
    "https://djkuroakari.github.io/starlighttable.html",
    "https://stellabms.xyz/sl/table.html",
    "https://stellabms.xyz/st/table.html",
    // normal 1/2 insane 1/2
    "https://darksabun.club/table/archive/normal1/",
    "https://darksabun.club/table/archive/insane1/",
    "http://rattoto10.jounin.jp/table.html",
    "http://rattoto10.jounin.jp/table_insane.html",
    // overjoy
    "https://rattoto10.jounin.jp/table_overjoy.html",
    // Optional list
    "https://lets-go-time-hell.github.io/code-stream-table/",
    "https://lets-go-time-hell.github.io/Arm-Shougakkou-table/",
    "https://su565fx.web.fc2.com/Gachimijoy/gachimijoy.html",
    "https://stellabms.xyz/so/table.html",
    "https://stellabms.xyz/sn/table.html",
    "https://air-afother.github.io/osu-table/",
    "https://bms.hexlataia.xyz/tables/ai.html",
    "https://bms.hexlataia.xyz/tables/db.html",
    "https://stellabms.xyz/upload.html",
    "https://exturbow.github.io/github.io/index.html",
    "https://bms.hexlataia.xyz/tables/olduploader.html",
    "http://fezikedifficulty.futene.net/list.html",
    "https://ladymade-star.github.io/luminous/table.html",
    "https://vinylhouse.web.fc2.com/lntougou/difficulty.html",
    "http://flowermaster.web.fc2.com/lrnanido/gla/LN.html",
    "https://skar-wem.github.io/ln/",
    "http://cerqant.web.fc2.com/zindy/table.html",
    "https://notepara.com/glassist/lnoj",
    "https://egret9.github.io/Scramble/",
    "http://minddnim.web.fc2.com/sara/3rd_hard/bms_sara_3rd_hard.html",
    "https://lets-go-time-hell.github.io/Delay-joy-table/",
    "https://kamikaze12345.github.io/github.io/delaytrainingtable/table.html",
    "https://wrench616.github.io/Delay/",
    "https://darksabun.club/table/archive/old-overjoy/",
    "https://monibms.github.io/Dystopia/dystopia.html",
    "https://www.firiex.com/tables/joverjoy",
    "https://plyfrm.github.io/table/timing/",
    "https://plyfrm.github.io/table/bmssearch/index.html",
    "https://yaruki0.net/DPlibrary/",
    "https://stellabms.xyz/dp/table.html",
    "https://stellabms.xyz/dpst/table.html",
    "https://deltabms.yaruki0.net/table/data/dpdelta_head.json",
    "https://deltabms.yaruki0.net/table/data/insane_head.json",
    "http://ereter.net/dpoverjoy/",
    "https://notmichaelchen.github.io/stella-table-extensions/satellite-easy.html",
    "https://notmichaelchen.github.io/stella-table-extensions/satellite-normal.html",
    "https://notmichaelchen.github.io/stella-table-extensions/satellite-hard.html",
    "https://notmichaelchen.github.io/stella-table-extensions/satellite-fullcombo.html",
    "https://notmichaelchen.github.io/stella-table-extensions/stella-easy.html",
    "https://notmichaelchen.github.io/stella-table-extensions/stella-normal.html",
    "https://notmichaelchen.github.io/stella-table-extensions/stella-hard.html",
    "https://notmichaelchen.github.io/stella-table-extensions/stella-fullcombo.html",
    "https://notmichaelchen.github.io/stella-table-extensions/dp-satellite-easy.html",
    "https://notmichaelchen.github.io/stella-table-extensions/dp-satellite-normal.html",
    "https://notmichaelchen.github.io/stella-table-extensions/dp-satellite-hard.html",
    "https://notmichaelchen.github.io/stella-table-extensions/dp-satellite-fullcombo.html",
    "http://walkure.net/hakkyou/for_glassist/bms/?lamp=easy",
    "http://walkure.net/hakkyou/for_glassist/bms/?lamp=normal",
    "http://walkure.net/hakkyou/for_glassist/bms/?lamp=hard",
    "http://walkure.net/hakkyou/for_glassist/bms/?lamp=fc",
];

static DEFAULT_TABLEURL: &[&str] = &[
    "https://mqppppp.neocities.org/StardustTable.html",
    "https://djkuroakari.github.io/starlighttable.html",
    "https://stellabms.xyz/sl/table.html",
    "https://stellabms.xyz/st/table.html",
    "https://darksabun.club/table/archive/normal1/",
    "https://darksabun.club/table/archive/insane1/",
    "http://rattoto10.jounin.jp/table.html",
    "http://rattoto10.jounin.jp/table_insane.html",
    "https://rattoto10.jounin.jp/table_overjoy.html",
];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    pub playername: Option<String>,
    #[serde(rename = "lastBootedVersion")]
    pub last_booted_version: String,
    pub displaymode: DisplayMode,
    pub vsync: bool,
    pub resolution: Resolution,
    #[serde(rename = "useResolution")]
    pub use_resolution: bool,
    #[serde(rename = "windowWidth")]
    pub window_width: i32,
    #[serde(rename = "windowHeight")]
    pub window_height: i32,
    pub folderlamp: bool,
    pub audio: Option<AudioConfig>,
    #[serde(rename = "maxFramePerSecond")]
    pub max_frame_per_second: i32,
    #[serde(rename = "prepareFramePerSecond")]
    pub prepare_frame_per_second: i32,
    #[serde(rename = "maxSearchBarCount")]
    pub max_search_bar_count: i32,
    #[serde(rename = "skipDecideScreen")]
    pub skip_decide_screen: bool,
    #[serde(rename = "showNoSongExistingBar")]
    pub show_no_song_existing_bar: bool,
    pub scrolldurationlow: i32,
    pub scrolldurationhigh: i32,
    #[serde(rename = "analogScroll")]
    pub analog_scroll: bool,
    #[serde(rename = "analogTicksPerScroll")]
    pub analog_ticks_per_scroll: i32,
    #[serde(rename = "songPreview")]
    pub song_preview: SongPreview,
    #[serde(rename = "cacheSkinImage")]
    pub cache_skin_image: bool,
    #[serde(rename = "useSongInfo")]
    pub use_song_info: bool,
    pub songpath: String,
    pub songinfopath: String,
    pub tablepath: String,
    pub playerpath: String,
    pub skinpath: String,
    pub bgmpath: String,
    pub soundpath: String,
    pub systemfontpath: String,
    pub messagefontpath: String,
    pub bmsroot: Vec<String>,
    #[serde(rename = "tableURL")]
    pub table_url: Vec<String>,
    #[serde(rename = "availableURL")]
    pub available_url: Vec<String>,
    pub bga: i32,
    #[serde(rename = "bgaExpand")]
    pub bga_expand: i32,
    pub frameskip: i32,
    pub updatesong: bool,
    #[serde(rename = "skinPixmapGen")]
    pub skin_pixmap_gen: i32,
    #[serde(rename = "stagefilePixmapGen")]
    pub stagefile_pixmap_gen: i32,
    #[serde(rename = "bannerPixmapGen")]
    pub banner_pixmap_gen: i32,
    #[serde(rename = "songResourceGen")]
    pub song_resource_gen: i32,
    #[serde(rename = "enableIpfs")]
    pub enable_ipfs: bool,
    pub ipfsurl: String,
    #[serde(rename = "enableHttp")]
    pub enable_http: bool,
    #[serde(rename = "downloadSource")]
    pub download_source: String,
    #[serde(rename = "defaultDownloadUrl")]
    pub default_download_url: String,
    #[serde(rename = "overrideDownloadUrl")]
    pub override_download_url: String,
    #[serde(rename = "downloadDirectory")]
    pub download_directory: String,
    #[serde(rename = "irSendCount")]
    pub ir_send_count: i32,
    #[serde(rename = "useDiscordRpc")]
    pub use_discord_rpc: bool,
    #[serde(rename = "setClipboardScreenshot")]
    pub set_clipboard_screenshot: bool,
    #[serde(rename = "monitorName")]
    pub monitor_name: String,
    #[serde(rename = "webhookOption")]
    pub webhook_option: i32,
    #[serde(rename = "webhookName")]
    pub webhook_name: String,
    #[serde(rename = "webhookAvatar")]
    pub webhook_avatar: String,
    #[serde(rename = "webhookUrl")]
    pub webhook_url: Vec<String>,
    #[serde(rename = "useObsWs")]
    pub use_obs_ws: bool,
    #[serde(rename = "obsWsHost")]
    pub obs_ws_host: String,
    #[serde(rename = "obsWsPort")]
    pub obs_ws_port: i32,
    #[serde(rename = "obsWsPass")]
    pub obs_ws_pass: String,
    #[serde(rename = "obsWsRecStopWait")]
    pub obs_ws_rec_stop_wait: i32,
    #[serde(rename = "obsWsRecMode")]
    pub obs_ws_rec_mode: i32,
    #[serde(rename = "obsScenes")]
    pub obs_scenes: HashMap<String, String>,
    #[serde(rename = "obsActions")]
    pub obs_actions: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            playername: None,
            last_booted_version: String::new(),
            displaymode: DisplayMode::WINDOW,
            vsync: false,
            resolution: Resolution::HD,
            use_resolution: true,
            window_width: 1280,
            window_height: 720,
            folderlamp: true,
            audio: None,
            max_frame_per_second: 240,
            prepare_frame_per_second: 0,
            max_search_bar_count: 10,
            skip_decide_screen: false,
            show_no_song_existing_bar: true,
            scrolldurationlow: 300,
            scrolldurationhigh: 50,
            analog_scroll: true,
            analog_ticks_per_scroll: 3,
            song_preview: SongPreview::LOOP,
            cache_skin_image: false,
            use_song_info: true,
            songpath: SONGPATH_DEFAULT.to_string(),
            songinfopath: SONGINFOPATH_DEFAULT.to_string(),
            tablepath: TABLEPATH_DEFAULT.to_string(),
            playerpath: PLAYERPATH_DEFAULT.to_string(),
            skinpath: SKINPATH_DEFAULT.to_string(),
            bgmpath: "bgm".to_string(),
            soundpath: "sound".to_string(),
            systemfontpath: "font/VL-Gothic-Regular.ttf".to_string(),
            messagefontpath: "font/VL-Gothic-Regular.ttf".to_string(),
            bmsroot: Vec::new(),
            table_url: DEFAULT_TABLEURL.iter().map(|s| s.to_string()).collect(),
            available_url: AVAILABLE_TABLEURL.iter().map(|s| s.to_string()).collect(),
            bga: BGA_ON,
            bga_expand: BGAEXPAND_KEEP_ASPECT_RATIO,
            frameskip: 1,
            updatesong: false,
            skin_pixmap_gen: 4,
            stagefile_pixmap_gen: 2,
            banner_pixmap_gen: 2,
            song_resource_gen: 1,
            enable_ipfs: true,
            ipfsurl: "https://gateway.ipfs.io/".to_string(),
            enable_http: true,
            download_source: String::new(),
            default_download_url: String::new(),
            override_download_url: String::new(),
            download_directory: DEFAULT_DOWNLOAD_DIRECTORY.to_string(),
            ir_send_count: 5,
            use_discord_rpc: false,
            set_clipboard_screenshot: false,
            monitor_name: String::new(),
            webhook_option: 0,
            webhook_name: String::new(),
            webhook_avatar: String::new(),
            webhook_url: Vec::new(),
            use_obs_ws: false,
            obs_ws_host: "localhost".to_string(),
            obs_ws_port: 4455,
            obs_ws_pass: String::new(),
            obs_ws_rec_stop_wait: 5000,
            obs_ws_rec_mode: 0,
            obs_scenes: HashMap::new(),
            obs_actions: HashMap::new(),
        }
    }
}

impl Config {
    pub fn is_show_no_song_existing_bar(&self) -> bool {
        self.show_no_song_existing_bar || self.enable_http
    }

    pub fn set_analog_ticks_per_scroll(&mut self, value: i32) {
        self.analog_ticks_per_scroll = value.max(1);
    }

    pub fn get_obs_ws_pass(&self) -> Option<&str> {
        if self.obs_ws_pass.is_empty() || self.obs_ws_pass.trim().is_empty() {
            None
        } else {
            Some(&self.obs_ws_pass)
        }
    }

    pub fn set_obs_ws_port(&mut self, port: i32) {
        self.obs_ws_port = port.clamp(0, 65535);
    }

    pub fn set_obs_ws_rec_stop_wait(&mut self, wait: i32) {
        self.obs_ws_rec_stop_wait = wait.clamp(0, 10000);
    }

    pub fn get_obs_scene(&self, state_name: &str) -> Option<&String> {
        self.obs_scenes.get(state_name)
    }

    pub fn set_obs_scene(&mut self, state_name: String, scene_name: Option<String>) {
        match scene_name {
            None => {
                self.obs_scenes.remove(&state_name);
            }
            Some(s) if s.is_empty() => {
                self.obs_scenes.remove(&state_name);
            }
            Some(s) => {
                self.obs_scenes.insert(state_name, s);
            }
        }
    }

    pub fn get_obs_action(&self, state_name: &str) -> Option<&String> {
        self.obs_actions.get(state_name)
    }

    pub fn set_obs_action(&mut self, state_name: String, action_name: Option<String>) {
        match action_name {
            None => {
                self.obs_actions.remove(&state_name);
            }
            Some(s) if s.is_empty() => {
                self.obs_actions.remove(&state_name);
            }
            Some(s) => {
                self.obs_actions.insert(state_name, s);
            }
        }
    }

    pub fn get_playername(&self) -> Option<&str> {
        self.playername.as_deref()
    }

    pub fn is_set_clipboard_screenshot(&self) -> bool {
        self.set_clipboard_screenshot
    }

    pub fn get_webhook_option(&self) -> i32 {
        self.webhook_option
    }

    pub fn get_webhook_url(&self) -> &[String] {
        &self.webhook_url
    }

    pub fn get_webhook_name(&self) -> &str {
        &self.webhook_name
    }

    pub fn get_webhook_avatar(&self) -> &str {
        &self.webhook_avatar
    }

    pub fn get_bmsroot(&self) -> &[String] {
        &self.bmsroot
    }

    pub fn get_table_url(&self) -> &[String] {
        &self.table_url
    }

    pub fn get_songpath(&self) -> &str {
        &self.songpath
    }

    pub fn get_songinfopath(&self) -> &str {
        &self.songinfopath
    }

    pub fn get_tablepath(&self) -> &str {
        &self.tablepath
    }

    pub fn get_playerpath(&self) -> &str {
        &self.playerpath
    }

    pub fn get_skinpath(&self) -> &str {
        &self.skinpath
    }

    pub fn get_bgmpath(&self) -> &str {
        &self.bgmpath
    }

    pub fn get_soundpath(&self) -> &str {
        &self.soundpath
    }

    pub fn get_systemfontpath(&self) -> &str {
        &self.systemfontpath
    }

    pub fn get_messagefontpath(&self) -> &str {
        &self.messagefontpath
    }

    pub fn get_max_frame_per_second(&self) -> i32 {
        self.max_frame_per_second
    }

    pub fn get_max_search_bar_count(&self) -> i32 {
        self.max_search_bar_count
    }

    pub fn get_bga(&self) -> i32 {
        self.bga
    }

    pub fn get_bga_expand(&self) -> i32 {
        self.bga_expand
    }

    pub fn get_frameskip(&self) -> i32 {
        self.frameskip
    }

    pub fn get_override_download_url(&self) -> Option<&str> {
        if self.override_download_url.is_empty() {
            None
        } else {
            Some(&self.override_download_url)
        }
    }

    pub fn get_download_directory(&self) -> &str {
        &self.download_directory
    }

    pub fn get_monitor_name(&self) -> &str {
        &self.monitor_name
    }

    pub fn is_analog_scroll(&self) -> bool {
        self.analog_scroll
    }

    pub fn get_resolution(&self) -> Resolution {
        self.resolution
    }

    pub fn get_audio_config(&self) -> Option<&AudioConfig> {
        self.audio.as_ref()
    }

    pub fn get_song_resource_gen(&self) -> i32 {
        self.song_resource_gen
    }

    pub fn get_config_json(config: &Config) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(config)?)
    }

    pub fn validate_config(mut config: Config) -> anyhow::Result<Config> {
        config.validate();
        PlayerConfig::init(&config)?;
        Ok(config)
    }

    pub fn read() -> anyhow::Result<Config> {
        let configpath = PathBuf::from("config_sys.json");
        let configpath_old = PathBuf::from("config.json");

        let mut config: Option<Config> = None;
        if configpath.exists() {
            match std::fs::read_to_string(&configpath) {
                Ok(data) => match serde_json::from_str::<Config>(&data) {
                    Ok(c) => config = Some(c),
                    Err(e) => {
                        log::error!("Failed to parse config: {}", e);
                        write_backup_config_file(&configpath);
                    }
                },
                Err(e) => {
                    log::error!("Failed to read config: {}", e);
                    write_backup_config_file(&configpath);
                }
            }
        } else if configpath_old.exists() {
            match std::fs::read_to_string(&configpath_old) {
                Ok(data) => match serde_json::from_str::<Config>(&data) {
                    Ok(c) => config = Some(c),
                    Err(e) => {
                        log::error!("Failed to parse old config: {}", e);
                    }
                },
                Err(e) => {
                    log::error!("Failed to read old config: {}", e);
                }
            }
        }

        let config = config.unwrap_or_default();
        Config::validate_config(config)
    }

    pub fn write(config: &Config) -> anyhow::Result<()> {
        let configpath = PathBuf::from("config_sys.json");
        let json = serde_json::to_string_pretty(config)?;
        std::fs::write(configpath, json.as_bytes())?;
        Ok(())
    }
}

impl Validatable for Config {
    fn validate(&mut self) -> bool {
        self.window_width = self
            .window_width
            .clamp(Resolution::SD.width(), Resolution::ULTRAHD.width());
        self.window_height = self
            .window_height
            .clamp(Resolution::SD.height(), Resolution::ULTRAHD.height());

        if self.audio.is_none() {
            self.audio = Some(AudioConfig::default());
        }
        if let Some(ref mut audio) = self.audio {
            audio.validate();
        }
        self.max_frame_per_second = self.max_frame_per_second.clamp(0, 50000);
        self.prepare_frame_per_second = self.prepare_frame_per_second.clamp(0, 100000);
        self.max_search_bar_count = self.max_search_bar_count.clamp(1, 100);

        self.scrolldurationlow = self.scrolldurationlow.clamp(2, 1000);
        self.scrolldurationhigh = self.scrolldurationhigh.clamp(1, 1000);
        self.ir_send_count = self.ir_send_count.clamp(1, 100);

        self.skin_pixmap_gen = self.skin_pixmap_gen.clamp(0, 100);
        self.stagefile_pixmap_gen = self.stagefile_pixmap_gen.clamp(0, 100);
        self.banner_pixmap_gen = self.banner_pixmap_gen.clamp(0, 100);
        self.song_resource_gen = self.song_resource_gen.clamp(0, 100);

        self.bmsroot = remove_empty_strings(&self.bmsroot);

        if self.table_url.is_empty() {
            self.table_url = DEFAULT_TABLEURL.iter().map(|s| s.to_string()).collect();
        }
        self.table_url = remove_empty_strings(&self.table_url);

        self.bga = self.bga.clamp(0, 2);
        self.bga_expand = self.bga_expand.clamp(0, 2);
        if self.ipfsurl.is_empty() {
            self.ipfsurl = "https://gateway.ipfs.io/".to_string();
        }

        if self.songpath.is_empty() {
            self.songpath = SONGPATH_DEFAULT.to_string();
        }
        if self.songinfopath.is_empty() {
            self.songinfopath = SONGINFOPATH_DEFAULT.to_string();
        }
        if self.tablepath.is_empty() {
            self.tablepath = TABLEPATH_DEFAULT.to_string();
        }
        if self.playerpath.is_empty() {
            self.playerpath = PLAYERPATH_DEFAULT.to_string();
        }
        if self.skinpath.is_empty() {
            self.skinpath = SKINPATH_DEFAULT.to_string();
        }
        if !validate_path(&self.download_directory) {
            self.download_directory = DEFAULT_DOWNLOAD_DIRECTORY.to_string();
        }
        true
    }
}

fn validate_path(path: &str) -> bool {
    // Check if the path is valid by trying to create a PathBuf
    let p = Path::new(path);
    // A path is considered valid if it's non-empty and doesn't cause issues
    !path.is_empty() && p.to_str().is_some()
}

fn write_backup_config_file(configpath: &Path) {
    let backup_path = configpath.with_file_name("config_sys_backup.json");
    match std::fs::copy(configpath, &backup_path) {
        Ok(_) => log::info!("Backup config written to {:?}", backup_path),
        Err(e) => log::error!("Failed to write backup config file: {}", e),
    }
}

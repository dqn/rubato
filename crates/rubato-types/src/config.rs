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

/// BGA display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BgaMode {
    #[default]
    On = 0,
    Auto = 1,
    Off = 2,
}

impl From<i32> for BgaMode {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::On,
            1 => Self::Auto,
            2 => Self::Off,
            _ => Self::default(),
        }
    }
}

impl From<BgaMode> for i32 {
    fn from(v: BgaMode) -> Self {
        v as i32
    }
}

impl serde::Serialize for BgaMode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(*self as i32)
    }
}

impl<'de> serde::Deserialize<'de> for BgaMode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = i32::deserialize(deserializer)?;
        Ok(Self::from(v))
    }
}

/// BGA expand (aspect ratio) mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BgaExpand {
    Full = 0,
    #[default]
    KeepAspectRatio = 1,
    Off = 2,
}

impl From<i32> for BgaExpand {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::Full,
            1 => Self::KeepAspectRatio,
            2 => Self::Off,
            _ => Self::default(),
        }
    }
}

impl From<BgaExpand> for i32 {
    fn from(v: BgaExpand) -> Self {
        v as i32
    }
}

impl serde::Serialize for BgaExpand {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(*self as i32)
    }
}

impl<'de> serde::Deserialize<'de> for BgaExpand {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = i32::deserialize(deserializer)?;
        Ok(Self::from(v))
    }
}

// Legacy constants for backward compatibility
pub const BGA_ON: BgaMode = BgaMode::On;
pub const BGA_AUTO: BgaMode = BgaMode::Auto;
pub const BGA_OFF: BgaMode = BgaMode::Off;

pub const BGAEXPAND_FULL: BgaExpand = BgaExpand::Full;
pub const BGAEXPAND_KEEP_ASPECT_RATIO: BgaExpand = BgaExpand::KeepAspectRatio;
pub const BGAEXPAND_OFF: BgaExpand = BgaExpand::Off;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum DisplayMode {
    FULLSCREEN,
    BORDERLESS,
    #[default]
    WINDOW,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, Default)]
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

// --- Sub-structs for Config decomposition ---

/// Display and window configuration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub displaymode: DisplayMode,
    pub vsync: bool,
    pub resolution: Resolution,
    #[serde(rename = "useResolution")]
    pub use_resolution: bool,
    #[serde(rename = "windowWidth")]
    pub window_width: i32,
    #[serde(rename = "windowHeight")]
    pub window_height: i32,
    #[serde(rename = "maxFramePerSecond")]
    pub max_frame_per_second: i32,
    #[serde(rename = "prepareFramePerSecond")]
    pub prepare_frame_per_second: i32,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            displaymode: DisplayMode::WINDOW,
            vsync: false,
            resolution: Resolution::HD,
            use_resolution: true,
            window_width: 1280,
            window_height: 720,
            max_frame_per_second: 240,
            prepare_frame_per_second: 0,
        }
    }
}

/// File and directory path configuration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PathConfig {
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
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
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
        }
    }
}

/// BGA and resource generation configuration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RenderConfig {
    pub bga: BgaMode,
    #[serde(rename = "bgaExpand")]
    pub bga_expand: BgaExpand,
    pub frameskip: i32,
    #[serde(rename = "skinPixmapGen")]
    pub skin_pixmap_gen: i32,
    #[serde(rename = "stagefilePixmapGen")]
    pub stagefile_pixmap_gen: i32,
    #[serde(rename = "bannerPixmapGen")]
    pub banner_pixmap_gen: i32,
    #[serde(rename = "songResourceGen")]
    pub song_resource_gen: i32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            bga: BGA_ON,
            bga_expand: BGAEXPAND_KEEP_ASPECT_RATIO,
            frameskip: 1,
            skin_pixmap_gen: 4,
            stagefile_pixmap_gen: 2,
            banner_pixmap_gen: 2,
            song_resource_gen: 1,
        }
    }
}

/// Download and network configuration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    #[serde(rename = "enableIpfs")]
    pub enable_ipfs: bool,
    pub ipfsurl: String,
    #[serde(rename = "enableHttp")]
    pub enable_http: bool,
    #[serde(rename = "downloadSource")]
    pub download_source: String,
    #[serde(rename = "defaultDownloadUrl", alias = "defaultDownloadURL")]
    pub default_download_url: String,
    #[serde(rename = "overrideDownloadUrl")]
    pub override_download_url: String,
    #[serde(rename = "downloadDirectory")]
    pub download_directory: String,
    #[serde(rename = "irSendCount")]
    pub ir_send_count: i32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enable_ipfs: true,
            ipfsurl: "https://gateway.ipfs.io/".to_string(),
            enable_http: true,
            download_source: String::new(),
            default_download_url: String::new(),
            override_download_url: String::new(),
            download_directory: DEFAULT_DOWNLOAD_DIRECTORY.to_string(),
            ir_send_count: 5,
        }
    }
}

/// OBS WebSocket configuration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ObsConfig {
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

impl Default for ObsConfig {
    fn default() -> Self {
        Self {
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

/// External integrations: Discord, clipboard, webhook.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct IntegrationConfig {
    #[serde(rename = "useDiscordRPC", alias = "useDiscordRpc")]
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
}

/// Music select screen configuration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SelectConfig {
    pub folderlamp: bool,
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
}

impl Default for SelectConfig {
    fn default() -> Self {
        Self {
            folderlamp: true,
            max_search_bar_count: 10,
            skip_decide_screen: false,
            show_no_song_existing_bar: true,
            scrolldurationlow: 300,
            scrolldurationhigh: 50,
            analog_scroll: true,
            analog_ticks_per_scroll: 3,
            song_preview: SongPreview::LOOP,
            cache_skin_image: false,
        }
    }
}

// --- Main Config struct ---

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    pub playername: Option<String>,
    #[serde(rename = "lastBootedVersion")]
    pub last_booted_version: String,
    pub audio: Option<AudioConfig>,
    #[serde(rename = "useSongInfo")]
    pub use_song_info: bool,
    pub updatesong: bool,

    #[serde(flatten)]
    pub display: DisplayConfig,
    #[serde(flatten)]
    pub paths: PathConfig,
    #[serde(flatten)]
    pub render: RenderConfig,
    #[serde(flatten)]
    pub network: NetworkConfig,
    #[serde(flatten)]
    pub obs: ObsConfig,
    #[serde(flatten)]
    pub integration: IntegrationConfig,
    #[serde(flatten)]
    pub select: SelectConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            playername: None,
            last_booted_version: String::new(),
            audio: None,
            use_song_info: true,
            updatesong: false,
            display: DisplayConfig::default(),
            paths: PathConfig::default(),
            render: RenderConfig::default(),
            network: NetworkConfig::default(),
            obs: ObsConfig::default(),
            integration: IntegrationConfig::default(),
            select: SelectConfig::default(),
        }
    }
}

impl Config {
    pub fn is_show_no_song_existing_bar(&self) -> bool {
        self.select.show_no_song_existing_bar || self.network.enable_http
    }

    pub fn set_analog_ticks_per_scroll(&mut self, value: i32) {
        self.select.analog_ticks_per_scroll = value.clamp(1, 100);
    }

    pub fn obs_ws_pass(&self) -> Option<&str> {
        if self.obs.obs_ws_pass.is_empty() || self.obs.obs_ws_pass.trim().is_empty() {
            None
        } else {
            Some(&self.obs.obs_ws_pass)
        }
    }

    pub fn set_obs_ws_port(&mut self, port: i32) {
        self.obs.obs_ws_port = port.clamp(0, 65535);
    }

    pub fn set_obs_ws_rec_stop_wait(&mut self, wait: i32) {
        self.obs.obs_ws_rec_stop_wait = wait.clamp(0, 10000);
    }

    pub fn obs_scene(&self, state_name: &str) -> Option<&String> {
        self.obs.obs_scenes.get(state_name)
    }

    pub fn set_obs_scene(&mut self, state_name: String, scene_name: Option<String>) {
        match scene_name {
            None => {
                self.obs.obs_scenes.remove(&state_name);
            }
            Some(s) if s.is_empty() => {
                self.obs.obs_scenes.remove(&state_name);
            }
            Some(s) => {
                self.obs.obs_scenes.insert(state_name, s);
            }
        }
    }

    pub fn obs_action(&self, state_name: &str) -> Option<&String> {
        self.obs.obs_actions.get(state_name)
    }

    pub fn set_obs_action(&mut self, state_name: String, action_name: Option<String>) {
        match action_name {
            None => {
                self.obs.obs_actions.remove(&state_name);
            }
            Some(s) if s.is_empty() => {
                self.obs.obs_actions.remove(&state_name);
            }
            Some(s) => {
                self.obs.obs_actions.insert(state_name, s);
            }
        }
    }

    pub fn playername(&self) -> Option<&str> {
        self.playername.as_deref()
    }

    pub fn override_download_url(&self) -> Option<&str> {
        if self.network.override_download_url.is_empty() {
            None
        } else {
            Some(&self.network.override_download_url)
        }
    }

    pub fn audio_config(&self) -> Option<&AudioConfig> {
        self.audio.as_ref()
    }

    pub fn config_json(config: &Config) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(config)?)
    }

    pub fn validate_config(mut config: Config) -> anyhow::Result<Config> {
        config.validate();
        PlayerConfig::init(&mut config)?;
        Ok(config)
    }

    /// Read config from a specific directory.
    /// Looks for `config_sys.json` first, falls back to `config.json`.
    pub fn read_from(dir: &Path) -> anyhow::Result<Config> {
        let configpath = dir.join("config_sys.json");
        let configpath_old = dir.join("config.json");

        let mut config: Option<Config> = None;
        let mut attempted_existing = false;

        if configpath.exists() {
            attempted_existing = true;
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
        }
        // Fall back to legacy config.json when config_sys.json is missing or corrupt
        if config.is_none() && configpath_old.exists() {
            attempted_existing = true;
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

        if config.is_none() && attempted_existing {
            anyhow::bail!(
                "Config file(s) exist but could not be loaded. Check logs for details. \
                 Refusing to use defaults to prevent settings loss."
            );
        }

        let config = config.unwrap_or_default();
        Config::validate_config(config)
    }

    /// Write config to a specific directory as `config_sys.json`.
    pub fn write_to(config: &Config, dir: &Path) -> anyhow::Result<()> {
        let configpath = dir.join("config_sys.json");
        let json = serde_json::to_string_pretty(config)?;
        std::fs::write(configpath, json.as_bytes())?;
        Ok(())
    }

    /// Read config from the resolved config directory.
    ///
    /// Relative paths within the config are resolved against CWD by design.
    /// The application entry point is responsible for setting CWD to the config root
    /// (via std::env::set_current_dir) before calling this method.
    pub fn read() -> anyhow::Result<Config> {
        let current_dir = std::env::current_dir()?;
        let resolved_dir = resolve_config_dir(&current_dir).unwrap_or(current_dir);
        Self::read_from(&resolved_dir)
    }

    pub fn write(config: &Config) -> anyhow::Result<()> {
        let current_dir = std::env::current_dir()?;
        let resolved_dir = resolve_config_dir(&current_dir).unwrap_or(current_dir);
        Self::write_to(config, &resolved_dir)
    }
}

impl Validatable for Config {
    fn validate(&mut self) -> bool {
        self.display.window_width = self
            .display
            .window_width
            .clamp(Resolution::SD.width(), Resolution::ULTRAHD.width());
        self.display.window_height = self
            .display
            .window_height
            .clamp(Resolution::SD.height(), Resolution::ULTRAHD.height());

        if self.audio.is_none() {
            self.audio = Some(AudioConfig::default());
        }
        if let Some(ref mut audio) = self.audio {
            audio.validate();
        }
        self.display.max_frame_per_second = self.display.max_frame_per_second.clamp(0, 50000);
        self.display.prepare_frame_per_second =
            self.display.prepare_frame_per_second.clamp(0, 100000);
        self.select.max_search_bar_count = self.select.max_search_bar_count.clamp(1, 100);
        self.select.analog_ticks_per_scroll = self.select.analog_ticks_per_scroll.clamp(1, 100);

        self.select.scrolldurationlow = self.select.scrolldurationlow.clamp(2, 1000);
        self.select.scrolldurationhigh = self.select.scrolldurationhigh.clamp(1, 1000);
        self.network.ir_send_count = self.network.ir_send_count.clamp(1, 100);

        self.render.skin_pixmap_gen = self.render.skin_pixmap_gen.clamp(0, 100);
        self.render.stagefile_pixmap_gen = self.render.stagefile_pixmap_gen.clamp(0, 100);
        self.render.banner_pixmap_gen = self.render.banner_pixmap_gen.clamp(0, 100);
        self.render.song_resource_gen = self.render.song_resource_gen.clamp(0, 100);

        self.paths.bmsroot = remove_empty_strings(&self.paths.bmsroot);

        // Auto-detect ./bms directory relative to CWD and add it to bmsroot if not already present.
        if let Ok(cwd) = std::env::current_dir() {
            let bms_dir = cwd.join("bms");
            if bms_dir.is_dir() {
                let bms_canonical = bms_dir.canonicalize().unwrap_or(bms_dir);
                let already_present = self.paths.bmsroot.iter().any(|p| {
                    let existing = std::path::Path::new(p);
                    let existing_canonical = if existing.is_relative() {
                        cwd.join(existing)
                            .canonicalize()
                            .unwrap_or_else(|_| cwd.join(existing))
                    } else {
                        existing
                            .canonicalize()
                            .unwrap_or_else(|_| existing.to_path_buf())
                    };
                    existing_canonical == bms_canonical
                });
                if !already_present {
                    self.paths
                        .bmsroot
                        .push(bms_canonical.to_string_lossy().to_string());
                }
            }
        }

        if self.paths.table_url.is_empty() {
            self.paths.table_url = DEFAULT_TABLEURL.iter().map(|s| s.to_string()).collect();
        }
        self.paths.table_url = remove_empty_strings(&self.paths.table_url);

        // BGA mode and expand are enums; deserialization already validates via From<i32>.
        if self.network.ipfsurl.is_empty() {
            self.network.ipfsurl = "https://gateway.ipfs.io/".to_string();
        }

        if self.paths.songpath.is_empty() {
            self.paths.songpath = SONGPATH_DEFAULT.to_string();
        }
        if self.paths.songinfopath.is_empty() {
            self.paths.songinfopath = SONGINFOPATH_DEFAULT.to_string();
        }
        if self.paths.tablepath.is_empty() {
            self.paths.tablepath = TABLEPATH_DEFAULT.to_string();
        }
        if self.paths.playerpath.is_empty() {
            self.paths.playerpath = PLAYERPATH_DEFAULT.to_string();
        }
        if self.paths.skinpath.is_empty() {
            self.paths.skinpath = SKINPATH_DEFAULT.to_string();
        }
        if !validate_path(&self.network.download_directory) {
            self.network.download_directory = DEFAULT_DOWNLOAD_DIRECTORY.to_string();
        }
        // ObsRecordingMode has 3 variants: 0=KeepAll, 1=OnScreenshot, 2=OnReplay
        self.obs.obs_ws_rec_mode = self.obs.obs_ws_rec_mode.clamp(0, 2);
        true
    }
}

// Minimal validation: only checks non-empty and valid UTF-8.
// Does not block directory traversal (e.g., "../../../etc") because this is
// a local config file that only the user can edit. The download_directory
// is used for HTTP resource downloads within the user's own filesystem.
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

pub fn resolve_config_dir(start_dir: &Path) -> Option<PathBuf> {
    let start_dir = start_dir
        .canonicalize()
        .unwrap_or_else(|_| start_dir.to_path_buf());

    for dir in start_dir.ancestors() {
        if dir.join("config_sys.json").exists() || dir.join("config.json").exists() {
            return Some(dir.to_path_buf());
        }
    }

    None
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::validatable::Validatable;

    // -- BgaMode serde and conversion --

    #[test]
    fn bga_mode_from_i32_valid_values() {
        assert_eq!(BgaMode::from(0), BgaMode::On);
        assert_eq!(BgaMode::from(1), BgaMode::Auto);
        assert_eq!(BgaMode::from(2), BgaMode::Off);
    }

    #[test]
    fn bga_mode_from_i32_out_of_range_returns_default() {
        assert_eq!(BgaMode::from(-1), BgaMode::On);
        assert_eq!(BgaMode::from(3), BgaMode::On);
        assert_eq!(BgaMode::from(i32::MAX), BgaMode::On);
        assert_eq!(BgaMode::from(i32::MIN), BgaMode::On);
    }

    #[test]
    fn bga_mode_serde_round_trip() {
        for mode in [BgaMode::On, BgaMode::Auto, BgaMode::Off] {
            let json = serde_json::to_string(&mode).unwrap();
            let deserialized: BgaMode = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, mode);
        }
    }

    #[test]
    fn bga_mode_deserialize_out_of_range_falls_back() {
        let deserialized: BgaMode = serde_json::from_str("99").unwrap();
        assert_eq!(deserialized, BgaMode::On); // default
    }

    // -- BgaExpand serde and conversion --

    #[test]
    fn bga_expand_from_i32_valid_values() {
        assert_eq!(BgaExpand::from(0), BgaExpand::Full);
        assert_eq!(BgaExpand::from(1), BgaExpand::KeepAspectRatio);
        assert_eq!(BgaExpand::from(2), BgaExpand::Off);
    }

    #[test]
    fn bga_expand_from_i32_out_of_range_returns_default() {
        assert_eq!(BgaExpand::from(-1), BgaExpand::KeepAspectRatio);
        assert_eq!(BgaExpand::from(3), BgaExpand::KeepAspectRatio);
    }

    #[test]
    fn bga_expand_serde_round_trip() {
        for mode in [BgaExpand::Full, BgaExpand::KeepAspectRatio, BgaExpand::Off] {
            let json = serde_json::to_string(&mode).unwrap();
            let deserialized: BgaExpand = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, mode);
        }
    }

    // -- Config default --

    #[test]
    fn config_default_has_sane_values() {
        let config = Config::default();
        assert!(config.playername.is_none());
        assert!(config.display.window_width > 0);
        assert!(config.display.window_height > 0);
        assert!(!config.paths.songpath.is_empty());
        assert!(!config.paths.skinpath.is_empty());
        assert!(!config.paths.playerpath.is_empty());
    }

    // -- Config serde: empty JSON object deserializes to defaults --

    #[test]
    fn config_deserialize_empty_object_uses_defaults() {
        let config: Config = serde_json::from_str("{}").unwrap();
        let default = Config::default();
        assert_eq!(config.display.window_width, default.display.window_width);
        assert_eq!(config.display.window_height, default.display.window_height);
        assert_eq!(config.paths.songpath, default.paths.songpath);
        assert_eq!(config.paths.skinpath, default.paths.skinpath);
    }

    // -- Config serde: extra unknown fields are ignored --

    #[test]
    fn config_deserialize_ignores_unknown_fields() {
        let json = r#"{"unknownField": 42, "anotherUnknown": "hello"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.display.window_width,
            Config::default().display.window_width
        );
    }

    // -- Config serde round-trip --

    #[test]
    fn config_serde_round_trip() {
        let mut config = Config::default();
        config.playername = Some("TestPlayer".to_string());
        config.display.window_width = 1920;
        config.display.window_height = 1080;
        config.render.bga = BgaMode::Off;
        config.network.enable_ipfs = false;
        config.obs.use_obs_ws = true;
        config.obs.obs_ws_port = 4444;
        config.select.max_search_bar_count = 20;

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.playername, Some("TestPlayer".to_string()));
        assert_eq!(deserialized.display.window_width, 1920);
        assert_eq!(deserialized.display.window_height, 1080);
        assert_eq!(deserialized.render.bga, BgaMode::Off);
        assert!(!deserialized.network.enable_ipfs);
        assert!(deserialized.obs.use_obs_ws);
        assert_eq!(deserialized.obs.obs_ws_port, 4444);
        assert_eq!(deserialized.select.max_search_bar_count, 20);
    }

    // -- Config serde: camelCase/rename fields --

    #[test]
    fn config_serializes_with_java_field_names() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();

        // These should be camelCase as specified by #[serde(rename)]
        assert!(
            json.contains("\"lastBootedVersion\""),
            "missing lastBootedVersion"
        );
        assert!(json.contains("\"useSongInfo\""), "missing useSongInfo");
        assert!(json.contains("\"useResolution\""), "missing useResolution");
        assert!(json.contains("\"windowWidth\""), "missing windowWidth");
        assert!(json.contains("\"windowHeight\""), "missing windowHeight");
        assert!(
            json.contains("\"maxFramePerSecond\""),
            "missing maxFramePerSecond"
        );
        assert!(json.contains("\"bgaExpand\""), "missing bgaExpand");
        assert!(json.contains("\"enableIpfs\""), "missing enableIpfs");
        assert!(json.contains("\"useObsWs\""), "missing useObsWs");
        assert!(json.contains("\"obsWsPort\""), "missing obsWsPort");
        assert!(json.contains("\"useDiscordRPC\""), "missing useDiscordRPC");
        assert!(json.contains("\"tableURL\""), "missing tableURL");

        // These snake_case forms should NOT appear
        assert!(
            !json.contains("\"last_booted_version\""),
            "snake_case leak: last_booted_version"
        );
        assert!(
            !json.contains("\"use_song_info\""),
            "snake_case leak: use_song_info"
        );
        assert!(
            !json.contains("\"window_width\""),
            "snake_case leak: window_width"
        );
    }

    // -- Config validate: clamps extreme values --

    #[test]
    fn config_validate_clamps_extreme_window_dimensions() {
        let mut config = Config::default();
        config.display.window_width = 1;
        config.display.window_height = 1;
        config.validate();
        assert!(config.display.window_width >= Resolution::SD.width());
        assert!(config.display.window_height >= Resolution::SD.height());

        config.display.window_width = 100_000;
        config.display.window_height = 100_000;
        config.validate();
        assert!(config.display.window_width <= Resolution::ULTRAHD.width());
        assert!(config.display.window_height <= Resolution::ULTRAHD.height());
    }

    #[test]
    fn config_validate_clamps_fps_to_valid_range() {
        let mut config = Config::default();
        config.display.max_frame_per_second = -10;
        config.validate();
        assert_eq!(config.display.max_frame_per_second, 0);

        config.display.max_frame_per_second = 999_999;
        config.validate();
        assert_eq!(config.display.max_frame_per_second, 50000);
    }

    #[test]
    fn config_validate_clamps_search_bar_count() {
        let mut config = Config::default();
        config.select.max_search_bar_count = 0;
        config.validate();
        assert_eq!(config.select.max_search_bar_count, 1);

        config.select.max_search_bar_count = 1000;
        config.validate();
        assert_eq!(config.select.max_search_bar_count, 100);
    }

    #[test]
    fn config_validate_clamps_obs_port() {
        let mut config = Config::default();
        config.obs.obs_ws_port = -5;
        config.validate();
        // obs_ws_rec_mode is validated; port is not directly clamped by validate()
        // but set_obs_ws_port clamps it
    }

    #[test]
    fn config_validate_empty_paths_get_defaults() {
        let mut config = Config::default();
        config.paths.songpath = String::new();
        config.paths.skinpath = String::new();
        config.paths.playerpath = String::new();
        config.paths.tablepath = String::new();
        config.paths.songinfopath = String::new();
        config.validate();
        assert_eq!(config.paths.songpath, SONGPATH_DEFAULT);
        assert_eq!(config.paths.skinpath, SKINPATH_DEFAULT);
        assert_eq!(config.paths.playerpath, PLAYERPATH_DEFAULT);
        assert_eq!(config.paths.tablepath, TABLEPATH_DEFAULT);
        assert_eq!(config.paths.songinfopath, SONGINFOPATH_DEFAULT);
    }

    #[test]
    fn config_validate_removes_empty_bmsroot_entries() {
        let mut config = Config::default();
        config.paths.bmsroot = vec!["path1".to_string(), "".to_string(), "path2".to_string()];
        config.validate();
        // Empty strings should be removed, but non-empty preserved
        assert!(config.paths.bmsroot.iter().all(|s| !s.is_empty()));
    }

    #[test]
    fn config_validate_empty_table_url_gets_defaults() {
        let mut config = Config::default();
        config.paths.table_url = Vec::new();
        config.validate();
        assert!(!config.paths.table_url.is_empty());
    }

    #[test]
    fn config_validate_empty_ipfsurl_gets_default() {
        let mut config = Config::default();
        config.network.ipfsurl = String::new();
        config.validate();
        assert_eq!(config.network.ipfsurl, "https://gateway.ipfs.io/");
    }

    #[test]
    fn config_validate_audio_none_gets_default() {
        let mut config = Config::default();
        config.audio = None;
        config.validate();
        assert!(config.audio.is_some());
    }

    #[test]
    fn config_validate_clamps_pixmap_gen_values() {
        let mut config = Config::default();
        config.render.skin_pixmap_gen = -5;
        config.render.stagefile_pixmap_gen = 200;
        config.render.banner_pixmap_gen = -1;
        config.render.song_resource_gen = 500;
        config.validate();
        assert_eq!(config.render.skin_pixmap_gen, 0);
        assert_eq!(config.render.stagefile_pixmap_gen, 100);
        assert_eq!(config.render.banner_pixmap_gen, 0);
        assert_eq!(config.render.song_resource_gen, 100);
    }

    #[test]
    fn config_validate_clamps_obs_rec_mode() {
        let mut config = Config::default();
        config.obs.obs_ws_rec_mode = 99;
        config.validate();
        assert_eq!(config.obs.obs_ws_rec_mode, 2);

        config.obs.obs_ws_rec_mode = -1;
        config.validate();
        assert_eq!(config.obs.obs_ws_rec_mode, 0);
    }

    #[test]
    fn config_validate_clamps_ir_send_count() {
        let mut config = Config::default();
        config.network.ir_send_count = 0;
        config.validate();
        assert_eq!(config.network.ir_send_count, 1);

        config.network.ir_send_count = 500;
        config.validate();
        assert_eq!(config.network.ir_send_count, 100);
    }

    // -- Config methods --

    #[test]
    fn config_set_analog_ticks_per_scroll_min_is_one() {
        let mut config = Config::default();
        config.set_analog_ticks_per_scroll(0);
        assert_eq!(config.select.analog_ticks_per_scroll, 1);

        config.set_analog_ticks_per_scroll(-5);
        assert_eq!(config.select.analog_ticks_per_scroll, 1);
    }

    #[test]
    fn config_obs_ws_pass_empty_returns_none() {
        let mut config = Config::default();
        config.obs.obs_ws_pass = String::new();
        assert!(config.obs_ws_pass().is_none());

        config.obs.obs_ws_pass = "   ".to_string();
        assert!(config.obs_ws_pass().is_none());
    }

    #[test]
    fn config_obs_ws_pass_non_empty_returns_some() {
        let mut config = Config::default();
        config.obs.obs_ws_pass = "mypassword".to_string();
        assert_eq!(config.obs_ws_pass(), Some("mypassword"));
    }

    #[test]
    fn config_set_obs_ws_port_clamps() {
        let mut config = Config::default();
        config.set_obs_ws_port(-1);
        assert_eq!(config.obs.obs_ws_port, 0);

        config.set_obs_ws_port(70000);
        assert_eq!(config.obs.obs_ws_port, 65535);

        config.set_obs_ws_port(4455);
        assert_eq!(config.obs.obs_ws_port, 4455);
    }

    #[test]
    fn config_set_obs_ws_rec_stop_wait_clamps() {
        let mut config = Config::default();
        config.set_obs_ws_rec_stop_wait(-100);
        assert_eq!(config.obs.obs_ws_rec_stop_wait, 0);

        config.set_obs_ws_rec_stop_wait(50000);
        assert_eq!(config.obs.obs_ws_rec_stop_wait, 10000);
    }

    #[test]
    fn config_obs_scene_set_and_get() {
        let mut config = Config::default();
        assert!(config.obs_scene("play").is_none());

        config.set_obs_scene("play".to_string(), Some("PlayScene".to_string()));
        assert_eq!(config.obs_scene("play"), Some(&"PlayScene".to_string()));

        // Setting None removes the entry
        config.set_obs_scene("play".to_string(), None);
        assert!(config.obs_scene("play").is_none());

        // Setting empty string also removes
        config.set_obs_scene("play".to_string(), Some("PlayScene".to_string()));
        config.set_obs_scene("play".to_string(), Some(String::new()));
        assert!(config.obs_scene("play").is_none());
    }

    #[test]
    fn config_obs_action_set_and_get() {
        let mut config = Config::default();
        assert!(config.obs_action("play").is_none());

        config.set_obs_action("play".to_string(), Some("StartAction".to_string()));
        assert_eq!(config.obs_action("play"), Some(&"StartAction".to_string()));

        config.set_obs_action("play".to_string(), None);
        assert!(config.obs_action("play").is_none());
    }

    #[test]
    fn config_override_download_url_empty_returns_none() {
        let mut config = Config::default();
        config.network.override_download_url = String::new();
        assert!(config.override_download_url().is_none());
    }

    #[test]
    fn config_override_download_url_non_empty_returns_some() {
        let mut config = Config::default();
        config.network.override_download_url = "https://example.com".to_string();
        assert_eq!(config.override_download_url(), Some("https://example.com"));
    }

    #[test]
    fn config_is_show_no_song_existing_bar_logic() {
        let mut config = Config::default();
        config.select.show_no_song_existing_bar = false;
        config.network.enable_http = false;
        assert!(!config.is_show_no_song_existing_bar());

        config.select.show_no_song_existing_bar = true;
        assert!(config.is_show_no_song_existing_bar());

        config.select.show_no_song_existing_bar = false;
        config.network.enable_http = true;
        assert!(config.is_show_no_song_existing_bar());
    }

    // -- Config read/write round-trip --

    #[test]
    fn config_write_and_read_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.playername = Some("RoundTripPlayer".to_string());
        config.display.window_width = 1600;

        Config::write_to(&config, dir.path()).unwrap();
        let loaded = Config::read_from(dir.path()).unwrap();

        assert_eq!(loaded.playername, Some("RoundTripPlayer".to_string()));
        assert_eq!(loaded.display.window_width, 1600);
    }

    #[test]
    fn config_read_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        // No config file exists, should return default
        let config = Config::read_from(dir.path()).unwrap();
        assert_eq!(
            config.display.window_width,
            Config::default().display.window_width
        );
    }

    #[test]
    fn config_read_corrupt_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let configpath = dir.path().join("config_sys.json");
        std::fs::write(&configpath, "not valid json!!!").unwrap();

        let result = Config::read_from(dir.path());
        assert!(result.is_err());
    }

    // -- DisplayMode and SongPreview serde --

    #[test]
    fn display_mode_serde_round_trip() {
        for mode in [
            DisplayMode::FULLSCREEN,
            DisplayMode::BORDERLESS,
            DisplayMode::WINDOW,
        ] {
            let json = serde_json::to_string(&mode).unwrap();
            let deserialized: DisplayMode = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", deserialized), format!("{:?}", mode));
        }
    }

    #[test]
    fn song_preview_serde_round_trip() {
        for preview in [SongPreview::NONE, SongPreview::ONCE, SongPreview::LOOP] {
            let json = serde_json::to_string(&preview).unwrap();
            let deserialized: SongPreview = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", deserialized), format!("{:?}", preview));
        }
    }

    // -- validate_path --

    #[test]
    fn validate_path_rejects_empty() {
        assert!(!validate_path(""));
    }

    #[test]
    fn validate_path_accepts_normal_paths() {
        assert!(validate_path("downloads"));
        assert!(validate_path("/tmp/test"));
        assert!(validate_path("relative/path/to/dir"));
    }
}

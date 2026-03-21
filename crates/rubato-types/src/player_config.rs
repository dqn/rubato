use std::path::{Path, PathBuf};

use bms_model::mode::Mode;

use crate::bar_sorter::BarSorter;
use crate::config::Config;
use crate::groove_gauge::GrooveGauge;
use crate::ir_config::IRConfig;
use crate::ir_connection_registry::IRConnectionManager;
use crate::play_mode_config::PlayModeConfig;
use crate::scroll_speed_modifier;
use crate::skin_config::SkinConfig;
use crate::skin_type::SkinType;
use crate::validatable::{Validatable, remove_invalid_elements};

pub const JUDGETIMING_MAX: i32 = 500;
pub const JUDGETIMING_MIN: i32 = -500;

pub const GAUGEAUTOSHIFT_NONE: i32 = 0;
pub const GAUGEAUTOSHIFT_CONTINUE: i32 = 1;
pub const GAUGEAUTOSHIFT_SURVIVAL_TO_GROOVE: i32 = 2;
pub const GAUGEAUTOSHIFT_BESTCLEAR: i32 = 3;
pub const GAUGEAUTOSHIFT_SELECT_TO_UNDER: i32 = 4;

/// Play-related settings (gauge, random, option modes)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PlaySettings {
    pub gauge: i32,
    pub random: i32,
    pub random2: i32,
    pub doubleoption: i32,
    #[serde(rename = "chartReplicationMode")]
    pub chart_replication_mode: String,
    pub lnmode: i32,
    pub forcedcnendings: bool,
    #[serde(rename = "gaugeAutoShift")]
    pub gauge_auto_shift: i32,
    #[serde(rename = "bottomShiftableGauge")]
    pub bottom_shiftable_gauge: i32,
    #[serde(rename = "hranThresholdBPM", alias = "hranThresholdBpm")]
    pub hran_threshold_bpm: i32,
    #[serde(rename = "mineMode")]
    pub mine_mode: i32,
}

impl Default for PlaySettings {
    fn default() -> Self {
        Self {
            gauge: 0,
            random: 0,
            random2: 0,
            doubleoption: 0,
            chart_replication_mode: "RIVALCHART".to_string(),
            lnmode: 0,
            forcedcnendings: false,
            gauge_auto_shift: GAUGEAUTOSHIFT_NONE,
            bottom_shiftable_gauge: GrooveGauge::ASSISTEASY,
            hran_threshold_bpm: 120,
            mine_mode: 0,
        }
    }
}

/// Judge timing and window rate settings
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct JudgeSettings {
    pub judgetiming: i32,
    #[serde(rename = "notesDisplayTimingAutoAdjust")]
    pub notes_display_timing_auto_adjust: bool,
    #[serde(rename = "customJudge")]
    pub custom_judge: bool,
    #[serde(rename = "keyJudgeWindowRatePerfectGreat")]
    pub key_judge_window_rate_perfect_great: i32,
    #[serde(rename = "keyJudgeWindowRateGreat")]
    pub key_judge_window_rate_great: i32,
    #[serde(rename = "keyJudgeWindowRateGood")]
    pub key_judge_window_rate_good: i32,
    #[serde(rename = "scratchJudgeWindowRatePerfectGreat")]
    pub scratch_judge_window_rate_perfect_great: i32,
    #[serde(rename = "scratchJudgeWindowRateGreat")]
    pub scratch_judge_window_rate_great: i32,
    #[serde(rename = "scratchJudgeWindowRateGood")]
    pub scratch_judge_window_rate_good: i32,
}

impl Default for JudgeSettings {
    fn default() -> Self {
        Self {
            judgetiming: 0,
            notes_display_timing_auto_adjust: false,
            custom_judge: false,
            key_judge_window_rate_perfect_great: 400,
            key_judge_window_rate_great: 400,
            key_judge_window_rate_good: 100,
            scratch_judge_window_rate_perfect_great: 400,
            scratch_judge_window_rate_great: 400,
            scratch_judge_window_rate_good: 100,
        }
    }
}

/// Display and visual settings
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DisplaySettings {
    pub bpmguide: bool,
    pub showjudgearea: bool,
    pub markprocessednote: bool,
    pub showhiddennote: bool,
    pub showpastnote: bool,
    #[serde(rename = "chartPreview")]
    pub chart_preview: bool,
    #[serde(rename = "isGuideSE", alias = "isGuideSe")]
    pub is_guide_se: bool,
    #[serde(rename = "misslayerDuration")]
    pub misslayer_duration: i32,
    #[serde(rename = "extranoteType")]
    pub extranote_type: i32,
    #[serde(rename = "extranoteDepth")]
    pub extranote_depth: i32,
    #[serde(rename = "extranoteScratch")]
    pub extranote_scratch: bool,
    #[serde(rename = "scrollMode")]
    pub scroll_mode: i32,
    #[serde(rename = "scrollSection")]
    pub scroll_section: i32,
    #[serde(rename = "scrollRate")]
    pub scroll_rate: f64,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            bpmguide: false,
            showjudgearea: false,
            markprocessednote: false,
            showhiddennote: false,
            showpastnote: false,
            chart_preview: true,
            is_guide_se: false,
            misslayer_duration: 500,
            extranote_type: 0,
            extranote_depth: 0,
            extranote_scratch: false,
            scroll_mode: 0,
            scroll_section: 4,
            scroll_rate: 0.5,
        }
    }
}

/// Note modifier settings (longnote, 7-to-9 conversion)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct NoteModifierSettings {
    #[serde(rename = "longnoteMode")]
    pub longnote_mode: i32,
    #[serde(rename = "longnoteRate")]
    pub longnote_rate: f64,
    #[serde(rename = "sevenToNinePattern")]
    pub seven_to_nine_pattern: i32,
    #[serde(rename = "sevenToNineType")]
    pub seven_to_nine_type: i32,
}

impl Default for NoteModifierSettings {
    fn default() -> Self {
        Self {
            longnote_mode: 0,
            longnote_rate: 1.0,
            seven_to_nine_pattern: 0,
            seven_to_nine_type: 0,
        }
    }
}

/// Music select and sorting settings
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SelectSettings {
    pub sort: i32,
    pub sortid: Option<String>,
    pub musicselectinput: i32,
    #[serde(rename = "isRandomSelect")]
    pub is_random_select: bool,
    #[serde(rename = "isWindowHold")]
    pub is_window_hold: bool,
    #[serde(rename = "eventMode")]
    pub event_mode: bool,
    pub targetid: String,
    pub targetlist: Vec<String>,
}

impl Default for SelectSettings {
    fn default() -> Self {
        Self {
            sort: 0,
            sortid: None,
            musicselectinput: 0,
            is_random_select: false,
            is_window_hold: false,
            event_mode: false,
            targetid: "MAX".to_string(),
            targetlist: vec![
                "RATE_A-",
                "RATE_A",
                "RATE_A+",
                "RATE_AA-",
                "RATE_AA",
                "RATE_AA+",
                "RATE_AAA-",
                "RATE_AAA",
                "RATE_AAA+",
                "RATE_MAX-",
                "MAX",
                "RANK_NEXT",
                "IR_NEXT_1",
                "IR_NEXT_2",
                "IR_NEXT_3",
                "IR_NEXT_4",
                "IR_NEXT_5",
                "IR_NEXT_10",
                "IR_RANK_1",
                "IR_RANK_5",
                "IR_RANK_10",
                "IR_RANK_20",
                "IR_RANK_30",
                "IR_RANK_40",
                "IR_RANK_50",
                "IR_RANKRATE_5",
                "IR_RANKRATE_10",
                "IR_RANKRATE_15",
                "IR_RANKRATE_20",
                "IR_RANKRATE_25",
                "IR_RANKRATE_30",
                "IR_RANKRATE_35",
                "IR_RANKRATE_40",
                "IR_RANKRATE_45",
                "IR_RANKRATE_50",
                "RIVAL_RANK_1",
                "RIVAL_RANK_2",
                "RIVAL_RANK_3",
                "RIVAL_NEXT_1",
                "RIVAL_NEXT_2",
                "RIVAL_NEXT_3",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        }
    }
}

/// Miscellaneous settings (replay, exit)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MiscSettings {
    pub autosavereplay: Vec<i32>,
    #[serde(rename = "exitPressDuration")]
    pub exit_press_duration: i32,
}

impl Default for MiscSettings {
    fn default() -> Self {
        Self {
            autosavereplay: vec![0; 4],
            exit_press_duration: 1000,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PlayerConfig {
    pub id: Option<String>,
    pub name: String,
    #[serde(flatten)]
    pub play_settings: PlaySettings,
    #[serde(flatten)]
    pub judge_settings: JudgeSettings,
    #[serde(flatten)]
    pub display_settings: DisplaySettings,
    #[serde(flatten)]
    pub note_modifier_settings: NoteModifierSettings,
    #[serde(flatten)]
    pub select_settings: SelectSettings,
    #[serde(flatten)]
    pub misc_settings: MiscSettings,
    pub mode: Option<Mode>,
    pub skin: Vec<Option<SkinConfig>>,
    #[serde(rename = "skinHistory")]
    pub skin_history: Vec<SkinConfig>,
    pub mode5: PlayModeConfig,
    pub mode7: PlayModeConfig,
    pub mode10: PlayModeConfig,
    pub mode14: PlayModeConfig,
    pub mode9: PlayModeConfig,
    pub mode24: PlayModeConfig,
    pub mode24double: PlayModeConfig,
    pub irconfig: Vec<Option<IRConfig>>,
    #[serde(rename = "twitterConsumerKey")]
    pub twitter_consumer_key: Option<String>,
    #[serde(rename = "twitterConsumerSecret")]
    pub twitter_consumer_secret: Option<String>,
    #[serde(rename = "twitterAccessToken")]
    pub twitter_access_token: Option<String>,
    #[serde(rename = "twitterAccessTokenSecret")]
    pub twitter_access_token_secret: Option<String>,
    #[serde(rename = "enableRequest")]
    pub enable_request: bool,
    #[serde(rename = "notifyRequest")]
    pub notify_request: bool,
    #[serde(rename = "maxRequestCount")]
    pub max_request_count: i32,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        let max_skin_id = SkinType::max_skin_type_id();
        let skin: Vec<Option<SkinConfig>> = (0..=max_skin_id)
            .map(|i| Some(SkinConfig::default_for_id(i)))
            .collect();

        PlayerConfig {
            id: None,
            name: "NO NAME".to_string(),
            play_settings: PlaySettings::default(),
            judge_settings: JudgeSettings::default(),
            display_settings: DisplaySettings::default(),
            note_modifier_settings: NoteModifierSettings {
                longnote_rate: 1.0,
                ..Default::default()
            },
            select_settings: SelectSettings::default(),
            misc_settings: MiscSettings::default(),
            mode: None,
            skin,
            skin_history: Vec::new(),
            mode5: PlayModeConfig::new(Mode::BEAT_5K),
            mode7: PlayModeConfig::new(Mode::BEAT_7K),
            mode10: PlayModeConfig::new(Mode::BEAT_10K),
            mode14: PlayModeConfig::new(Mode::BEAT_14K),
            mode9: PlayModeConfig::new(Mode::POPN_9K),
            mode24: PlayModeConfig::new(Mode::KEYBOARD_24K),
            mode24double: PlayModeConfig::new(Mode::KEYBOARD_24K_DOUBLE),
            irconfig: Vec::new(),
            twitter_consumer_key: None,
            twitter_consumer_secret: None,
            twitter_access_token: None,
            twitter_access_token_secret: None,
            enable_request: false,
            notify_request: false,
            max_request_count: 30,
        }
    }
}

impl PlayerConfig {
    pub fn play_config_ref(&self, mode_id: Mode) -> &PlayModeConfig {
        match mode_id {
            Mode::BEAT_5K => &self.mode5,
            Mode::BEAT_7K => &self.mode7,
            Mode::BEAT_10K => &self.mode10,
            Mode::BEAT_14K => &self.mode14,
            Mode::POPN_5K | Mode::POPN_9K => &self.mode9,
            Mode::KEYBOARD_24K => &self.mode24,
            Mode::KEYBOARD_24K_DOUBLE => &self.mode24double,
        }
    }

    pub fn play_config(&mut self, mode_id: Mode) -> &mut PlayModeConfig {
        match mode_id {
            Mode::BEAT_5K => &mut self.mode5,
            Mode::BEAT_7K => &mut self.mode7,
            Mode::BEAT_10K => self.mode10(),
            Mode::BEAT_14K => self.mode14(),
            Mode::POPN_5K | Mode::POPN_9K => &mut self.mode9,
            Mode::KEYBOARD_24K => &mut self.mode24,
            Mode::KEYBOARD_24K_DOUBLE => self.mode24double(),
        }
    }

    pub fn play_config_by_id(&mut self, mode_id: i32) -> &mut PlayModeConfig {
        match mode_id {
            5 => &mut self.mode5,
            7 => &mut self.mode7,
            10 => self.mode10(),
            14 => self.mode14(),
            9 => &mut self.mode9,
            25 => &mut self.mode24,
            50 => self.mode24double(),
            _ => &mut self.mode7,
        }
    }

    fn mode10(&mut self) -> &mut PlayModeConfig {
        if self.mode10.controller.len() < 2 {
            self.mode10 = PlayModeConfig::new(Mode::BEAT_10K);
            log::warn!("mode10 PlayConfig reconstructed");
        }
        &mut self.mode10
    }

    fn mode14(&mut self) -> &mut PlayModeConfig {
        if self.mode14.controller.len() < 2 {
            self.mode14 = PlayModeConfig::new(Mode::BEAT_14K);
            log::warn!("mode14 PlayConfig reconstructed");
        }
        &mut self.mode14
    }

    fn mode24double(&mut self) -> &mut PlayModeConfig {
        if self.mode24double.controller.len() < 2 {
            self.mode24double = PlayModeConfig::new(Mode::KEYBOARD_24K_DOUBLE);
            log::warn!("mode24double PlayConfig reconstructed");
        }
        &mut self.mode24double
    }

    pub fn twitter_consumer_key(&self) -> Option<&str> {
        self.twitter_consumer_key.as_deref()
    }

    pub fn twitter_consumer_secret(&self) -> Option<&str> {
        self.twitter_consumer_secret.as_deref()
    }

    pub fn twitter_access_token(&self) -> Option<&str> {
        self.twitter_access_token.as_deref()
    }

    pub fn twitter_access_token_secret(&self) -> Option<&str> {
        self.twitter_access_token_secret.as_deref()
    }

    pub fn mode(&self) -> Option<&Mode> {
        self.mode.as_ref()
    }

    pub fn get_misslayer_duration(&mut self) -> i32 {
        if self.display_settings.misslayer_duration < 0 {
            self.display_settings.misslayer_duration = 0;
        }
        self.display_settings.misslayer_duration
    }

    pub fn skin(&mut self) -> &mut Vec<Option<SkinConfig>> {
        let max_id = SkinType::max_skin_type_id() as usize;
        if self.skin.len() <= max_id {
            self.skin.resize_with(max_id + 1, || None);
            log::warn!("skin reconstructed");
        }
        &mut self.skin
    }

    pub fn validate(&mut self) {
        let max_skin_id = SkinType::max_skin_type_id() as usize;

        if self.skin.len() != max_skin_id + 1 {
            self.skin.resize_with(max_skin_id + 1, || None);
        }
        for (i, skin) in self.skin.iter_mut().enumerate() {
            if skin.is_none() {
                *skin = Some(SkinConfig::default_for_id(i as i32));
            }
            if let Some(s) = skin {
                s.validate();
            }
        }

        self.mode5.validate(7);
        self.mode7.validate(9);
        self.mode10.validate(14);
        self.mode14.validate(18);
        self.mode9.validate(9);
        self.mode24.validate(26);
        self.mode24double.validate(52);

        // MusicSelectKeyProperty::VALUES has 3 entries (Beat7k, Popn9k, Beat14k)
        self.select_settings.musicselectinput = self.select_settings.musicselectinput.clamp(0, 2);

        let max_sort = BarSorter::DEFAULT_SORTER.len() as i32 - 1;
        self.select_settings.sort = self.select_settings.sort.clamp(0, max_sort);
        if self.select_settings.sortid.is_none() {
            self.select_settings.sortid = Some(
                BarSorter::DEFAULT_SORTER[self.select_settings.sort as usize]
                    .name()
                    .to_string(),
            );
        }

        self.play_settings.gauge = self.play_settings.gauge.clamp(0, 5);
        self.play_settings.random = self.play_settings.random.clamp(0, 9);
        self.play_settings.random2 = self.play_settings.random2.clamp(0, 9);
        self.play_settings.doubleoption = self.play_settings.doubleoption.clamp(0, 3);
        // gauge_auto_shift: 5 named constants (NONE/CONTINUE/SURVIVAL_TO_GROOVE/BESTCLEAR/SELECT_TO_UNDER)
        self.play_settings.gauge_auto_shift = self.play_settings.gauge_auto_shift.clamp(0, 4);
        // bottom_shiftable_gauge: ASSISTEASY(0)/EASY(1)/NORMAL(2), matching event factory count=3
        self.play_settings.bottom_shiftable_gauge =
            self.play_settings.bottom_shiftable_gauge.clamp(0, 2);
        if self.play_settings.chart_replication_mode.is_empty() {
            self.play_settings.chart_replication_mode = "NONE".to_string();
        }
        if self.select_settings.targetid.is_empty() {
            self.select_settings.targetid = "MAX".to_string();
        }
        if self.select_settings.targetlist.is_empty() {
            // keep as-is if non-empty, otherwise leave empty
        }
        self.judge_settings.judgetiming = self
            .judge_settings
            .judgetiming
            .clamp(JUDGETIMING_MIN, JUDGETIMING_MAX);
        self.display_settings.misslayer_duration =
            self.display_settings.misslayer_duration.clamp(0, 5000);
        self.play_settings.lnmode = self.play_settings.lnmode.clamp(0, 2);
        self.judge_settings.key_judge_window_rate_perfect_great = self
            .judge_settings
            .key_judge_window_rate_perfect_great
            .clamp(25, 400);
        self.judge_settings.key_judge_window_rate_great = self
            .judge_settings
            .key_judge_window_rate_great
            .clamp(0, 400);
        self.judge_settings.key_judge_window_rate_good =
            self.judge_settings.key_judge_window_rate_good.clamp(0, 400);
        self.judge_settings.scratch_judge_window_rate_perfect_great = self
            .judge_settings
            .scratch_judge_window_rate_perfect_great
            .clamp(25, 400);
        self.judge_settings.scratch_judge_window_rate_great = self
            .judge_settings
            .scratch_judge_window_rate_great
            .clamp(0, 400);
        self.judge_settings.scratch_judge_window_rate_good = self
            .judge_settings
            .scratch_judge_window_rate_good
            .clamp(0, 400);
        self.play_settings.hran_threshold_bpm =
            self.play_settings.hran_threshold_bpm.clamp(1, 1000);

        if self.misc_settings.autosavereplay.len() != 4 {
            self.misc_settings.autosavereplay.resize(4, 0);
        }
        self.note_modifier_settings.seven_to_nine_pattern = self
            .note_modifier_settings
            .seven_to_nine_pattern
            .clamp(0, 6);
        self.note_modifier_settings.seven_to_nine_type =
            self.note_modifier_settings.seven_to_nine_type.clamp(0, 2);
        self.misc_settings.exit_press_duration =
            self.misc_settings.exit_press_duration.clamp(0, 100000);

        self.display_settings.scroll_mode = self
            .display_settings
            .scroll_mode
            .clamp(0, scroll_speed_modifier::Mode::values().len() as i32 - 1);
        self.display_settings.scroll_section = self.display_settings.scroll_section.clamp(1, 1024);
        self.display_settings.scroll_rate = self.display_settings.scroll_rate.clamp(0.0, 1.0);
        // longnote_mode: 0=off, 1-5=Remove/AddLn/AddCn/AddHcn/AddAll (6 values total,
        // matching skin event cycling count). rubato-types enum is incomplete; the full
        // set lives in rubato-core::pattern::long_note_modifier::Mode which has 5 variants.
        self.note_modifier_settings.longnote_mode =
            self.note_modifier_settings.longnote_mode.clamp(0, 5);
        self.note_modifier_settings.longnote_rate =
            self.note_modifier_settings.longnote_rate.clamp(0.0, 1.0);
        // mine_mode: 0=off, 1-4=modifier modes (5 values total, matching skin event cycling count).
        self.play_settings.mine_mode = self.play_settings.mine_mode.clamp(0, 4);
        // ExtraNoteModifier places up to `depth` background notes per timeline
        // position. Values above ~4 produce extremely dense, unplayable charts.
        // Java uses [0, 100] but practical use is [0, 4].
        self.display_settings.extranote_depth = self.display_settings.extranote_depth.clamp(0, 10);

        if self.irconfig.is_empty() {
            let irnames = IRConnectionManager::all_available_ir_connection_name();
            self.irconfig = irnames
                .iter()
                .map(|name| {
                    Some(IRConfig {
                        irname: name.clone(),
                        ..Default::default()
                    })
                })
                .collect();
        }

        // Remove duplicate IR configs
        for i in 0..self.irconfig.len() {
            if self.irconfig[i].is_none() {
                continue;
            }
            let Some(name_i) = self.irconfig[i].as_ref().map(|c| c.irname.clone()) else {
                continue;
            };
            for j in (i + 1)..self.irconfig.len() {
                if let Some(ref mut cfg_j) = self.irconfig[j]
                    && cfg_j.irname == name_i
                {
                    cfg_j.irname = String::new();
                }
            }
        }
        let taken = std::mem::take(&mut self.irconfig);
        let valid_configs: Vec<IRConfig> = remove_invalid_elements(taken);
        self.irconfig = valid_configs.into_iter().map(Some).collect();

        // --Stream
        self.max_request_count = self.max_request_count.clamp(0, 100);
    }

    pub fn init(config: &mut Config) -> anyhow::Result<()> {
        Self::init_with_db_source_dir(config, None)
    }

    /// Like `init()`, but takes an explicit directory to search for `playerscore.db`
    /// and `replay/` instead of using the current working directory.
    ///
    /// Pass `None` to use CWD (the default / backward-compatible behavior).
    pub fn init_with_db_source_dir(
        config: &mut Config,
        db_source_dir: Option<&Path>,
    ) -> anyhow::Result<()> {
        let playerpath = Path::new(&config.paths.playerpath);
        if !playerpath.exists() {
            std::fs::create_dir_all(playerpath)?;
        }

        // Design: init() only auto-selects "player1" when creating a fresh player directory.
        // When players already exist but playername is None, the caller (MainLoader) falls
        // back to "default". This is intentional - init does not auto-select existing players.
        if read_all_player_id(&config.paths.playerpath).is_empty() {
            create_player(&config.paths.playerpath, "player1")?;
            // Copy score data if exists
            // Java parity: resolved relative to CWD (the beatoraja installation directory).
            // Requires the launcher to set CWD to the config root for migration to work.
            let parent_score_db = match db_source_dir {
                Some(dir) => dir.join("playerscore.db"),
                None => PathBuf::from("playerscore.db"),
            };
            if parent_score_db.exists() {
                let dest = PathBuf::from(format!("{}/player1/score.db", config.paths.playerpath));
                if let Err(e) = std::fs::copy(&parent_score_db, &dest) {
                    log::error!("Failed to copy playerscore.db: {}", e);
                }
            }
            // Copy replays
            copy_replays_from(config, db_source_dir);

            config.playername = Some("player1".to_string());
        }

        Ok(())
    }

    pub fn read_player_config(playerpath: &str, playerid: &str) -> anyhow::Result<PlayerConfig> {
        let configpath = PathBuf::from(format!("{}/{}/config_player.json", playerpath, playerid));
        let configpath_old = PathBuf::from(format!("{}/{}/config.json", playerpath, playerid));

        let mut player = if configpath.exists() {
            match load_player_config(playerpath, playerid, &configpath) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!(
                        "Failed to load config_player.json, trying legacy config.json: {}",
                        e
                    );
                    if configpath_old.exists() {
                        load_player_config_from_old_path(&configpath_old).map_err(|e2| {
                            anyhow::anyhow!(
                                "Both player config files are corrupt: primary={}, legacy={}",
                                e,
                                e2
                            )
                        })?
                    } else {
                        anyhow::bail!(
                            "Player config {} exists but could not be loaded: {}. \
                             Refusing to use defaults to prevent settings loss.",
                            configpath.display(),
                            e
                        );
                    }
                }
            }
        } else if configpath_old.exists() {
            load_player_config_from_old_path(&configpath_old)?
        } else {
            // Fallback: check root-level config_player.json (outside player dir).
            // Some setups (e.g., Java beatoraja migration) place the player config
            // at the workspace root rather than inside player/{id}/.
            if let Some(root_config) = resolve_root_player_config_path(playerpath) {
                log::info!(
                    "Player config not found at {}, using root config_player.json",
                    configpath.display()
                );
                load_player_config(playerpath, playerid, &root_config)?
            } else {
                PlayerConfig::default()
            }
        };

        player.id = Some(playerid.to_string());
        player.validate();
        Ok(player)
    }

    pub fn config_json(player: &PlayerConfig) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(player)?)
    }

    pub fn validate_player_config(playerid: &str, mut player: PlayerConfig) -> PlayerConfig {
        player.id = Some(playerid.to_string());
        player.validate();
        player
    }

    pub fn write(playerpath: &str, player: &PlayerConfig) -> anyhow::Result<()> {
        let id = player.id.as_deref().unwrap_or("unknown");
        let path = PathBuf::from(format!("{}/{}/config_player.json", playerpath, id));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(player)?;
        std::fs::write(path, json.as_bytes())?;
        Ok(())
    }
}

pub fn read_all_player_id(playerpath: &str) -> Vec<String> {
    let mut result = Vec::new();
    let path = Path::new(playerpath);
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && let Some(name) = entry.file_name().to_str()
            {
                result.push(name.to_string());
            }
        }
    }
    result
}

pub fn create_player(playerpath: &str, playerid: &str) -> anyhow::Result<()> {
    let p = PathBuf::from(format!("{}/{}", playerpath, playerid));
    if p.exists() {
        return Ok(());
    }
    std::fs::create_dir(&p)?;
    let player = PlayerConfig {
        id: Some(playerid.to_string()),
        ..Default::default()
    };
    PlayerConfig::write(playerpath, &player)?;
    Ok(())
}

fn copy_replays_from(config: &Config, db_source_dir: Option<&Path>) {
    let player1_replay_dir = PathBuf::from(format!("{}/player1/replay", config.paths.playerpath));
    let parent_replay_dir = match db_source_dir {
        Some(dir) => dir.join("replay"),
        None => PathBuf::from("replay"),
    };

    if let Err(e) = std::fs::create_dir_all(&player1_replay_dir) {
        log::error!("Failed to create replay dir: {}", e);
        return;
    }
    if !parent_replay_dir.exists() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(&parent_replay_dir) {
        for entry in entries.flatten() {
            let dest = player1_replay_dir.join(entry.file_name());
            if let Err(e) = std::fs::copy(entry.path(), &dest) {
                log::warn!("Error while copying replays: {}", e);
            }
        }
    }
}

fn load_player_config(
    playerpath: &str,
    playerid: &str,
    path: &Path,
) -> anyhow::Result<PlayerConfig> {
    let data = std::fs::read_to_string(path).map_err(|e| {
        write_backup_player_config(playerpath, playerid, path);
        anyhow::anyhow!("Failed to read player config: {}", e)
    })?;
    let player: PlayerConfig = serde_json::from_str(&data).map_err(|e| {
        write_backup_player_config(playerpath, playerid, path);
        anyhow::anyhow!("Failed to parse player config: {}", e)
    })?;
    Ok(player)
}

fn load_player_config_from_old_path(path: &Path) -> anyhow::Result<PlayerConfig> {
    let data = std::fs::read_to_string(path)?;
    let player: PlayerConfig = serde_json::from_str(&data)?;
    Ok(player)
}

fn resolve_root_player_config_path(playerpath: &str) -> Option<PathBuf> {
    if let Some(parent) = Path::new(playerpath).parent() {
        let from_player_root = parent.join("config_player.json");
        if from_player_root.exists() {
            return Some(from_player_root);
        }
    }

    let cwd = std::env::current_dir().ok()?;
    for ancestor in cwd.ancestors() {
        let candidate = ancestor.join("config_player.json");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn write_backup_player_config(playerpath: &str, playerid: &str, path: &Path) {
    let backup_path = PathBuf::from(format!("{}/{}/config_backup.json", playerpath, playerid));
    match std::fs::copy(path, &backup_path) {
        Ok(_) => log::info!("Backup config written to {:?}", backup_path),
        Err(e) => log::error!("Failed to write backup config file: {}", e),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    // -- Default values --

    #[test]
    fn player_config_default_has_sane_values() {
        let pc = PlayerConfig::default();
        assert!(pc.id.is_none());
        assert_eq!(pc.name, "NO NAME");
        assert_eq!(pc.play_settings.gauge, 0);
        assert_eq!(pc.judge_settings.judgetiming, 0);
        assert!(!pc.skin.is_empty());
        assert_eq!(pc.misc_settings.autosavereplay.len(), 4);
    }

    // -- Serde round-trip --

    #[test]
    fn player_config_serde_round_trip() {
        let mut pc = PlayerConfig::default();
        pc.name = "TestPlayer".to_string();
        pc.play_settings.gauge = 3;
        pc.judge_settings.judgetiming = 50;

        let json = serde_json::to_string(&pc).unwrap();
        let deserialized: PlayerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "TestPlayer");
        assert_eq!(deserialized.play_settings.gauge, 3);
        assert_eq!(deserialized.judge_settings.judgetiming, 50);
    }

    #[test]
    fn player_config_deserialize_empty_object() {
        let pc: PlayerConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(pc.name, "NO NAME");
        assert_eq!(pc.play_settings.gauge, 0);
    }

    // -- Validate: clamping --

    #[test]
    fn player_config_validate_clamps_gauge() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.gauge = 99;
        pc.validate();
        assert_eq!(pc.play_settings.gauge, 5);

        pc.play_settings.gauge = -1;
        pc.validate();
        assert_eq!(pc.play_settings.gauge, 0);
    }

    #[test]
    fn player_config_validate_clamps_random() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.random = 99;
        pc.validate();
        assert_eq!(pc.play_settings.random, 9);

        pc.play_settings.random = -1;
        pc.validate();
        assert_eq!(pc.play_settings.random, 0);
    }

    #[test]
    fn player_config_validate_clamps_judgetiming() {
        let mut pc = PlayerConfig::default();
        pc.judge_settings.judgetiming = 9999;
        pc.validate();
        assert_eq!(pc.judge_settings.judgetiming, JUDGETIMING_MAX);

        pc.judge_settings.judgetiming = -9999;
        pc.validate();
        assert_eq!(pc.judge_settings.judgetiming, JUDGETIMING_MIN);
    }

    #[test]
    fn player_config_validate_clamps_misslayer_duration() {
        let mut pc = PlayerConfig::default();
        pc.display_settings.misslayer_duration = -100;
        pc.validate();
        assert_eq!(pc.display_settings.misslayer_duration, 0);

        pc.display_settings.misslayer_duration = 99999;
        pc.validate();
        assert_eq!(pc.display_settings.misslayer_duration, 5000);
    }

    #[test]
    fn player_config_validate_clamps_judge_window_rates() {
        let mut pc = PlayerConfig::default();
        pc.judge_settings.key_judge_window_rate_perfect_great = 0;
        pc.judge_settings.key_judge_window_rate_great = -10;
        pc.judge_settings.key_judge_window_rate_good = 999;
        pc.validate();
        assert_eq!(pc.judge_settings.key_judge_window_rate_perfect_great, 25);
        assert_eq!(pc.judge_settings.key_judge_window_rate_great, 0);
        assert_eq!(pc.judge_settings.key_judge_window_rate_good, 400);
    }

    #[test]
    fn player_config_validate_clamps_hran_threshold_bpm() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.hran_threshold_bpm = 0;
        pc.validate();
        assert_eq!(pc.play_settings.hran_threshold_bpm, 1);

        pc.play_settings.hran_threshold_bpm = 99999;
        pc.validate();
        assert_eq!(pc.play_settings.hran_threshold_bpm, 1000);
    }

    #[test]
    fn player_config_validate_fixes_autosavereplay_length() {
        let mut pc = PlayerConfig::default();
        pc.misc_settings.autosavereplay = vec![1, 2]; // too short
        pc.validate();
        assert_eq!(pc.misc_settings.autosavereplay.len(), 4);
    }

    #[test]
    fn player_config_validate_empty_chart_replication_mode_gets_default() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.chart_replication_mode = String::new();
        pc.validate();
        assert_eq!(pc.play_settings.chart_replication_mode, "NONE");
    }

    #[test]
    fn player_config_validate_empty_targetid_gets_default() {
        let mut pc = PlayerConfig::default();
        pc.select_settings.targetid = String::new();
        pc.validate();
        assert_eq!(pc.select_settings.targetid, "MAX");
    }

    #[test]
    fn player_config_validate_clamps_longnote_mode() {
        let mut pc = PlayerConfig::default();
        pc.note_modifier_settings.longnote_mode = 99;
        pc.validate();
        assert_eq!(pc.note_modifier_settings.longnote_mode, 5);

        pc.note_modifier_settings.longnote_mode = -1;
        pc.validate();
        assert_eq!(pc.note_modifier_settings.longnote_mode, 0);
    }

    #[test]
    fn note_modifier_settings_default_longnote_rate_is_one() {
        let settings = NoteModifierSettings::default();
        assert_eq!(settings.longnote_rate, 1.0, "Java default is 1.0, not 0.0");
    }

    #[test]
    fn note_modifier_settings_deserialize_missing_longnote_rate_uses_one() {
        let json = r#"{"longnoteMode": 0}"#;
        let settings: NoteModifierSettings = serde_json::from_str(json).unwrap();
        assert_eq!(
            settings.longnote_rate, 1.0,
            "Missing field should default to 1.0 via serde(default)"
        );
    }

    #[test]
    fn player_config_validate_clamps_longnote_rate() {
        let mut pc = PlayerConfig::default();
        pc.note_modifier_settings.longnote_rate = 5.0;
        pc.validate();
        assert_eq!(pc.note_modifier_settings.longnote_rate, 1.0);

        pc.note_modifier_settings.longnote_rate = -1.0;
        pc.validate();
        assert_eq!(pc.note_modifier_settings.longnote_rate, 0.0);
    }

    #[test]
    fn player_config_validate_clamps_mine_mode() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.mine_mode = 99;
        pc.validate();
        assert_eq!(pc.play_settings.mine_mode, 4);
    }

    #[test]
    fn player_config_validate_clamps_scroll_settings() {
        let mut pc = PlayerConfig::default();
        pc.display_settings.scroll_section = 0;
        pc.display_settings.scroll_rate = -1.0;
        pc.validate();
        assert_eq!(pc.display_settings.scroll_section, 1);
        assert_eq!(pc.display_settings.scroll_rate, 0.0);
    }

    #[test]
    fn player_config_validate_clamps_gauge_auto_shift() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.gauge_auto_shift = 50;
        pc.validate();
        assert_eq!(pc.play_settings.gauge_auto_shift, 4);

        pc.play_settings.gauge_auto_shift = -1;
        pc.validate();
        assert_eq!(pc.play_settings.gauge_auto_shift, 0);
    }

    #[test]
    fn player_config_validate_clamps_bottom_shiftable_gauge() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.bottom_shiftable_gauge = 50;
        pc.validate();
        assert_eq!(pc.play_settings.bottom_shiftable_gauge, 2);

        pc.play_settings.bottom_shiftable_gauge = -1;
        pc.validate();
        assert_eq!(pc.play_settings.bottom_shiftable_gauge, 0);
    }

    // -- play_config_ref / play_config --

    #[test]
    fn play_config_ref_returns_correct_mode_config() {
        let pc = PlayerConfig::default();
        let mode5 = pc.play_config_ref(Mode::BEAT_5K);
        // Just verify it returns without panic and has right mode
        assert!(!mode5.controller.is_empty());
    }

    #[test]
    fn play_config_by_id_default_fallback() {
        let mut pc = PlayerConfig::default();
        // Unknown mode_id should fall back to mode7
        let config = pc.play_config_by_id(999);
        // Should not panic and return mode7
        assert!(!config.controller.is_empty());
    }

    // -- get_misslayer_duration --

    #[test]
    fn get_misslayer_duration_clamps_negative() {
        let mut pc = PlayerConfig::default();
        pc.display_settings.misslayer_duration = -100;
        let duration = pc.get_misslayer_duration();
        assert_eq!(duration, 0);
    }

    // -- skin accessor --

    #[test]
    fn skin_accessor_resizes_if_too_short() {
        let mut pc = PlayerConfig::default();
        pc.skin = vec![]; // empty
        let skins = pc.skin();
        assert!(skins.len() > 0);
    }

    // -- Read/write round-trip --

    #[test]
    fn player_config_write_and_read_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let playerpath = dir.path().to_str().unwrap();

        // Create player directory
        let player_dir = dir.path().join("test_player");
        std::fs::create_dir(&player_dir).unwrap();

        let mut pc = PlayerConfig::default();
        pc.id = Some("test_player".to_string());
        pc.name = "RoundTrip".to_string();
        pc.play_settings.gauge = 2;

        PlayerConfig::write(playerpath, &pc).unwrap();
        let loaded = PlayerConfig::read_player_config(playerpath, "test_player").unwrap();

        assert_eq!(loaded.name, "RoundTrip");
        assert_eq!(loaded.play_settings.gauge, 2);
        assert_eq!(loaded.id, Some("test_player".to_string()));
    }

    #[test]
    fn player_config_read_missing_creates_default() {
        let dir = tempfile::tempdir().unwrap();
        let playerpath = dir.path().to_str().unwrap();

        let player_dir = dir.path().join("new_player");
        std::fs::create_dir(&player_dir).unwrap();

        let pc = PlayerConfig::read_player_config(playerpath, "new_player").unwrap();
        assert_eq!(pc.name, "NO NAME");
        assert_eq!(pc.id, Some("new_player".to_string()));
    }

    #[test]
    fn resolve_root_player_config_path_finds_parent_of_absolute_playerpath() {
        let dir = tempfile::tempdir().unwrap();
        let player_root = dir.path().join("player");
        std::fs::create_dir_all(&player_root).unwrap();
        let root_config = dir.path().join("config_player.json");
        std::fs::write(&root_config, "{}").unwrap();

        let resolved = resolve_root_player_config_path(player_root.to_str().unwrap());

        assert_eq!(resolved.as_deref(), Some(root_config.as_path()));
    }

    #[test]
    fn player_config_reads_root_config_next_to_absolute_playerpath() {
        let dir = tempfile::tempdir().unwrap();
        let player_root = dir.path().join("player");
        std::fs::create_dir_all(&player_root).unwrap();
        std::fs::create_dir_all(player_root.join("default")).unwrap();

        let mut pc = PlayerConfig::default();
        pc.skin[SkinType::Play7Keys.id() as usize] =
            Some(SkinConfig::new_with_path("skin/ECFN/play/play7.luaskin"));
        std::fs::write(
            dir.path().join("config_player.json"),
            serde_json::to_string_pretty(&pc).unwrap(),
        )
        .unwrap();

        let loaded =
            PlayerConfig::read_player_config(player_root.to_str().unwrap(), "default").unwrap();

        assert_eq!(
            loaded.skin[SkinType::Play7Keys.id() as usize]
                .as_ref()
                .and_then(|skin| skin.path()),
            Some("skin/ECFN/play/play7.luaskin")
        );
    }

    // -- read_all_player_id --

    #[test]
    fn read_all_player_id_returns_directories() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("player1")).unwrap();
        std::fs::create_dir(dir.path().join("player2")).unwrap();
        // Create a file (should be excluded)
        std::fs::write(dir.path().join("not_a_player.txt"), "data").unwrap();

        let ids = read_all_player_id(dir.path().to_str().unwrap());
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"player1".to_string()));
        assert!(ids.contains(&"player2".to_string()));
    }

    #[test]
    fn read_all_player_id_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let ids = read_all_player_id(dir.path().to_str().unwrap());
        assert!(ids.is_empty());
    }

    #[test]
    fn read_all_player_id_nonexistent_dir() {
        let ids = read_all_player_id("/nonexistent/path/that/does/not/exist");
        assert!(ids.is_empty());
    }

    // -- create_player --

    #[test]
    fn create_player_creates_directory_and_config() {
        let dir = tempfile::tempdir().unwrap();
        let playerpath = dir.path().to_str().unwrap();

        create_player(playerpath, "new_player").unwrap();

        assert!(dir.path().join("new_player").exists());
        assert!(dir.path().join("new_player/config_player.json").exists());
    }

    #[test]
    fn create_player_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let playerpath = dir.path().to_str().unwrap();

        create_player(playerpath, "player1").unwrap();
        create_player(playerpath, "player1").unwrap(); // second call should be no-op

        assert!(dir.path().join("player1").exists());
    }

    // -- Serde: camelCase fields --

    #[test]
    fn player_config_serializes_with_java_field_names() {
        let pc = PlayerConfig::default();
        let json = serde_json::to_string(&pc).unwrap();

        assert!(
            json.contains("\"gaugeAutoShift\""),
            "missing gaugeAutoShift"
        );
        assert!(
            json.contains("\"chartReplicationMode\""),
            "missing chartReplicationMode"
        );
        assert!(
            json.contains("\"notesDisplayTimingAutoAdjust\""),
            "missing notesDisplayTimingAutoAdjust"
        );
        assert!(
            json.contains("\"isGuideSe\"") || json.contains("\"isGuideSE\""),
            "missing isGuideSe"
        );

        // These snake_case forms should NOT appear
        assert!(!json.contains("\"gauge_auto_shift\""), "snake_case leak");
        assert!(
            !json.contains("\"chart_replication_mode\""),
            "snake_case leak"
        );
    }
}

// Phase 5+ stubs - will be replaced when later phases are translated

/// Stub for beatoraja.play.GrooveGauge
pub struct GrooveGauge;

#[allow(dead_code)]
impl GrooveGauge {
    pub const ASSISTEASY: i32 = 0;
    pub const EASY: i32 = 1;
    pub const NORMAL: i32 = 2;
    pub const HARD: i32 = 3;
    pub const EXHARD: i32 = 4;
    pub const HAZARD: i32 = 5;
    pub const GRADE_NORMAL: i32 = 6;
    pub const GRADE_HARD: i32 = 7;
    pub const GRADE_EXHARD: i32 = 8;
}

/// Stub for beatoraja.play.JudgeAlgorithm
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JudgeAlgorithm {
    Combo,
    Duration,
    Lowest,
    Timing,
}

impl JudgeAlgorithm {
    pub fn name(&self) -> &str {
        match self {
            JudgeAlgorithm::Combo => "Combo",
            JudgeAlgorithm::Duration => "Duration",
            JudgeAlgorithm::Lowest => "Lowest",
            JudgeAlgorithm::Timing => "Timing",
        }
    }

    pub fn get_index(name: &str) -> i32 {
        match name {
            "Combo" => 0,
            "Duration" => 1,
            "Lowest" => 2,
            "Timing" => 3,
            _ => -1,
        }
    }

    pub fn values() -> &'static [JudgeAlgorithm] {
        &[
            JudgeAlgorithm::Combo,
            JudgeAlgorithm::Duration,
            JudgeAlgorithm::Lowest,
            JudgeAlgorithm::Timing,
        ]
    }
}

/// Stub for beatoraja.play.BMSPlayerRule
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BMSPlayerRule {
    LR2,
    Beatoraja,
}

/// Stub for beatoraja.skin.SkinType
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinType {
    PLAY_7KEYS,
    PLAY_5KEYS,
    PLAY_14KEYS,
    PLAY_10KEYS,
    PLAY_9KEYS,
    PLAY_24KEYS,
    PLAY_24KEYS_DOUBLE,
    MUSIC_SELECT,
    DECIDE,
    RESULT,
    COURSE_RESULT,
    KEY_CONFIG,
    SKIN_SELECT,
}

impl SkinType {
    pub fn get_max_skin_type_id() -> usize {
        12
    }

    pub fn get_skin_type_by_id(id: usize) -> Option<SkinType> {
        match id {
            0 => Some(SkinType::PLAY_7KEYS),
            1 => Some(SkinType::PLAY_5KEYS),
            2 => Some(SkinType::PLAY_14KEYS),
            3 => Some(SkinType::PLAY_10KEYS),
            4 => Some(SkinType::PLAY_9KEYS),
            5 => Some(SkinType::PLAY_24KEYS),
            6 => Some(SkinType::PLAY_24KEYS_DOUBLE),
            7 => Some(SkinType::MUSIC_SELECT),
            8 => Some(SkinType::DECIDE),
            9 => Some(SkinType::RESULT),
            10 => Some(SkinType::COURSE_RESULT),
            11 => Some(SkinType::KEY_CONFIG),
            12 => Some(SkinType::SKIN_SELECT),
            _ => None,
        }
    }
}

/// Stub for beatoraja.select.BarSorter
pub struct BarSorter;

#[derive(Clone, Debug)]
pub struct BarSorterEntry {
    name: &'static str,
}

impl BarSorterEntry {
    pub fn name(&self) -> &str {
        self.name
    }
}

#[allow(dead_code)]
impl BarSorter {
    pub const DEFAULT_SORTER: &'static [BarSorterEntry] = &[
        BarSorterEntry { name: "TITLE" },
        BarSorterEntry { name: "CLEAR" },
        BarSorterEntry { name: "SCORE" },
        BarSorterEntry { name: "MISSCOUNT" },
        BarSorterEntry { name: "DATE" },
        BarSorterEntry { name: "LEVEL" },
    ];
}

/// Stub for beatoraja.pattern.ScrollSpeedModifier
pub struct ScrollSpeedModifier;

pub mod scroll_speed_modifier {
    #[derive(Clone, Debug)]
    pub enum Mode {
        Off,
        Variable,
        Fixed,
    }

    impl Mode {
        pub fn values() -> &'static [Mode] {
            &[Mode::Off, Mode::Variable, Mode::Fixed]
        }
    }
}

/// Stub for beatoraja.pattern.LongNoteModifier
pub struct LongNoteModifier;

pub mod long_note_modifier {
    #[derive(Clone, Debug)]
    pub enum Mode {
        Off,
        Add,
        Remove,
    }

    impl Mode {
        pub fn values() -> &'static [Mode] {
            &[Mode::Off, Mode::Add, Mode::Remove]
        }
    }
}

/// Stub for beatoraja.pattern.MineNoteModifier
pub struct MineNoteModifier;

pub mod mine_note_modifier {
    #[derive(Clone, Debug)]
    pub enum Mode {
        Off,
        Remove,
    }

    impl Mode {
        pub fn values() -> &'static [Mode] {
            &[Mode::Off, Mode::Remove]
        }
    }
}

/// Stub for beatoraja.ir.IRConnectionManager
pub struct IRConnectionManager;

#[allow(dead_code)]
impl IRConnectionManager {
    pub fn get_all_available_ir_connection_name() -> Vec<String> {
        vec![]
    }

    pub fn get_ir_connection_class(_name: &str) -> Option<()> {
        Some(())
    }
}

/// Stub for beatoraja.input.BMSPlayerInputDevice
pub mod bms_player_input_device {
    #[allow(non_camel_case_types)]
    #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum Type {
        BM_CONTROLLER,
        KEYBOARD,
        MIDI,
        MOUSE,
    }
}

/// Stub for MainController (Phase 7+)
pub struct MainController;

/// Stub for MainState (Phase 7+)
pub trait MainStateTrait {}

/// Stub for Gdx types
pub struct Gdx;
pub struct SpriteBatch;
pub struct ShaderProgram;
pub struct Pixmap;
pub struct Texture;

/// Stub for beatoraja.song.SongData
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SongData {
    pub sha256: Option<String>,
    pub title: Option<String>,
    pub md5: Option<String>,
    pub url: Option<String>,
}

impl SongData {
    pub fn shrink(&mut self) {
        // Placeholder for shrinking large fields
    }

    pub fn validate(&mut self) -> bool {
        self.sha256.as_ref().is_some_and(|s| !s.is_empty())
            || self.md5.as_ref().is_some_and(|s| !s.is_empty())
    }
}

/// Stub for beatoraja.input.KeyInputLog
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KeyInputLog {
    pub time: i64,
    pub keycode: i32,
    pub pressed: bool,
}

impl KeyInputLog {
    pub fn validate(&self) -> bool {
        true
    }
}

/// Stub for beatoraja.pattern.PatternModifyLog
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PatternModifyLog {
    pub old_lane: i32,
    pub new_lane: i32,
}

impl PatternModifyLog {
    pub fn validate(&self) -> bool {
        true
    }
}

/// Stub for beatoraja.PlayConfig
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PlayConfig;

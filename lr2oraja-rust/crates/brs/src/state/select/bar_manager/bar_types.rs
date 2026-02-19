// Bar type definitions — SortMode, Bar enum, FunctionAction, GradeBarData, etc.

use bms_database::{CourseData, CourseDataConstraint, RandomCourseData, SongData, TableFolder};

/// Sort modes for the bar list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Default,
    Title,
    Artist,
    Level,
    Bpm,
    Length,
    Clear,
    Score,
    MissCount,
    Duration,
    LastUpdate,
    RivalCompareClear,
    RivalCompareScore,
}

impl SortMode {
    /// Convert a string ID to a SortMode.
    ///
    /// Java parity: `BarSorter.valueOf()` for sortid persistence.
    pub fn from_id(id: &str) -> Self {
        match id {
            "DEFAULT" => Self::Default,
            "TITLE" => Self::Title,
            "ARTIST" => Self::Artist,
            "LEVEL" => Self::Level,
            "BPM" => Self::Bpm,
            "LENGTH" => Self::Length,
            "CLEAR" => Self::Clear,
            "SCORE" => Self::Score,
            "MISSCOUNT" => Self::MissCount,
            "DURATION" => Self::Duration,
            "LASTUPDATE" => Self::LastUpdate,
            "RIVALCOMPARECLEAR" => Self::RivalCompareClear,
            "RIVALCOMPARESCORE" => Self::RivalCompareScore,
            _ => Self::Default,
        }
    }

    /// Convert this SortMode to a string ID for persistence.
    ///
    /// Java parity: `BarSorter.name()` for sortid persistence.
    pub fn to_id(self) -> &'static str {
        match self {
            Self::Default => "DEFAULT",
            Self::Title => "TITLE",
            Self::Artist => "ARTIST",
            Self::Level => "LEVEL",
            Self::Bpm => "BPM",
            Self::Length => "LENGTH",
            Self::Clear => "CLEAR",
            Self::Score => "SCORE",
            Self::MissCount => "MISSCOUNT",
            Self::Duration => "DURATION",
            Self::LastUpdate => "LASTUPDATE",
            Self::RivalCompareClear => "RIVALCOMPARECLEAR",
            Self::RivalCompareScore => "RIVALCOMPARESCORE",
        }
    }

    /// Cycle to the next sort mode.
    ///
    /// When `has_rival` is true, includes RivalCompareClear and RivalCompareScore
    /// in the cycle. When false, skips them.
    pub fn next(self, has_rival: bool) -> Self {
        match self {
            Self::Default => Self::Title,
            Self::Title => Self::Artist,
            Self::Artist => Self::Level,
            Self::Level => Self::Bpm,
            Self::Bpm => Self::Length,
            Self::Length => Self::Clear,
            Self::Clear => Self::Score,
            Self::Score => Self::MissCount,
            Self::MissCount => Self::Duration,
            Self::Duration => Self::LastUpdate,
            Self::LastUpdate => {
                if has_rival {
                    Self::RivalCompareClear
                } else {
                    Self::Default
                }
            }
            Self::RivalCompareClear => Self::RivalCompareScore,
            Self::RivalCompareScore => Self::Default,
        }
    }
}

/// Action associated with a function bar.
#[derive(Debug, Clone)]
pub enum FunctionAction {
    None,
    Autoplay(Box<SongData>),
    Practice(Box<SongData>),
    ShowSameFolder {
        title: String,
        folder_crc: String,
    },
    CopyToClipboard(String),
    OpenUrl(String),
    ToggleFavorite {
        sha256: String,
        flag: i32,
    },
    PlayReplay {
        song_data: Box<SongData>,
        replay_index: usize,
    },
    GhostBattle {
        song_data: Box<SongData>,
        lr2_id: i64,
        lane_sequence: i32,
    },
    ViewLeaderboard {
        song_data: Box<SongData>,
    },
}

/// Grade bar data containing a course with grade constraints.
#[derive(Debug, Clone)]
pub struct GradeBarData {
    pub name: String,
    pub course: CourseData,
    pub constraints: Vec<CourseDataConstraint>,
}

/// Context menu data for a bar (right-click menu).
#[derive(Debug, Clone)]
pub struct ContextMenuData {
    pub source_bar: Box<Bar>,
    pub items: Vec<ContextMenuItem>,
}

/// A single item in a context menu.
#[derive(Debug, Clone)]
pub struct ContextMenuItem {
    pub label: String,
    pub action: FunctionAction,
}

/// A single bar entry in the song list.
#[derive(Debug, Clone)]
pub enum Bar {
    // --- Selectable bars ---
    Song(Box<SongData>),
    Folder {
        name: String,
        path: String,
    },
    Course(Box<CourseData>),
    TableRoot {
        name: String,
        url: Option<String>,
        folders: Vec<TableFolder>,
        courses: Vec<CourseData>,
    },
    HashFolder {
        name: String,
        hashes: Vec<String>, // sha256 preferred, md5 fallback
    },
    /// Executable bar -- runs a set of songs (e.g., autoplay playlist).
    // Used in tests and bar navigation; compiler can't detect construction via test helpers
    #[allow(dead_code)]
    Executable {
        name: String,
        songs: Vec<SongData>,
    },
    /// Function bar -- a generic action item (autoplay, practice, clipboard, etc.).
    Function {
        title: String,
        subtitle: Option<String>,
        display_bar_type: i32,
        action: FunctionAction,
        lamp: i32,
    },
    /// Grade/dan-i bar -- wraps a course with grade constraints.
    Grade(Box<GradeBarData>),
    /// Random course bar -- selects random songs from SQL queries.
    RandomCourse(Box<RandomCourseData>),
    // --- Directory bars (expand into child bars on enter) ---
    /// Command bar -- executes a SQL query against the song DB.
    Command {
        name: String,
        sql: String,
    },
    /// Container bar -- holds an explicit list of child bars.
    Container {
        name: String,
        children: Vec<Bar>,
    },
    /// Same-folder bar -- finds songs sharing the same folder CRC.
    SameFolder {
        name: String,
        folder_crc: String,
    },
    /// Search word bar -- pre-configured text search.
    SearchWord {
        query: String,
    },
    /// Context menu bar -- right-click actions for a bar.
    ContextMenu(Box<ContextMenuData>),
}

impl Bar {
    /// Returns the display name for this bar.
    pub fn bar_name(&self) -> &str {
        match self {
            Bar::Song(s) => &s.title,
            Bar::Folder { name, .. } => name,
            Bar::Course(c) => &c.name,
            Bar::TableRoot { name, .. } => name,
            Bar::HashFolder { name, .. } => name,
            Bar::Executable { name, .. } => name,
            Bar::Function { title, .. } => title,
            Bar::Grade(g) => &g.name,
            Bar::RandomCourse(rc) => &rc.name,
            Bar::Command { name, .. } => name,
            Bar::Container { name, .. } => name,
            Bar::SameFolder { name, .. } => name,
            Bar::SearchWord { query } => query,
            Bar::ContextMenu(cm) => cm.source_bar.bar_name(),
        }
    }

    /// Returns a reference to the inner SongData if this is a Song bar.
    pub fn as_song(&self) -> Option<&bms_database::SongData> {
        match self {
            Bar::Song(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the display type index for bar rendering.
    ///
    /// 0 = Song, 1 = Folder/Directory, 2 = Grade/Course,
    /// 3 = Command, 4 = Search, 5 = Function/Other.
    pub fn bar_display_type(&self) -> i32 {
        match self {
            Bar::Song(_) | Bar::Executable { .. } => 0,
            Bar::Folder { .. }
            | Bar::TableRoot { .. }
            | Bar::HashFolder { .. }
            | Bar::Container { .. }
            | Bar::SameFolder { .. } => 1,
            Bar::Course(_) | Bar::Grade(_) | Bar::RandomCourse(_) => 2,
            Bar::Command { .. } | Bar::ContextMenu(_) => 3,
            Bar::SearchWord { .. } => 4,
            Bar::Function {
                display_bar_type, ..
            } => *display_bar_type,
        }
    }
}

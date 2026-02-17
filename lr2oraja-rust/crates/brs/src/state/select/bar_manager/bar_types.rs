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
}

impl SortMode {
    /// Cycle to the next sort mode.
    pub fn next(self) -> Self {
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
            Self::LastUpdate => Self::Default,
        }
    }
}

/// Action associated with a function bar.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Parsed for completeness (Java FunctionAction enum)
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
    },
}

/// Grade bar data containing a course with grade constraints.
#[derive(Debug, Clone)]
pub struct GradeBarData {
    pub name: String,
    #[allow(dead_code)] // TODO: integrate with course system
    pub course: CourseData,
    #[allow(dead_code)] // TODO: integrate with course system
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
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    Folder {
        name: String,
        path: String,
    },
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    Course(Box<CourseData>),
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    TableRoot {
        name: String,
        folders: Vec<TableFolder>,
        courses: Vec<CourseData>,
    },
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    HashFolder {
        name: String,
        hashes: Vec<String>, // sha256 preferred, md5 fallback
    },
    /// Executable bar -- runs a set of songs (e.g., autoplay playlist).
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    Executable {
        name: String,
        songs: Vec<SongData>,
    },
    /// Function bar -- a generic action item (autoplay, practice, clipboard, etc.).
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    Function {
        title: String,
        subtitle: Option<String>,
        display_bar_type: i32,
        action: FunctionAction,
        lamp: i32,
    },
    /// Grade/dan-i bar -- wraps a course with grade constraints.
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    Grade(Box<GradeBarData>),
    /// Random course bar -- selects random songs from SQL queries.
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    RandomCourse(Box<RandomCourseData>),
    // --- Directory bars (expand into child bars on enter) ---
    /// Command bar -- executes a SQL query against the song DB.
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    Command {
        name: String,
        sql: String,
    },
    /// Container bar -- holds an explicit list of child bars.
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    Container {
        name: String,
        children: Vec<Bar>,
    },
    /// Same-folder bar -- finds songs sharing the same folder CRC.
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    SameFolder {
        name: String,
        folder_crc: String,
    },
    /// Search word bar -- pre-configured text search.
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    SearchWord {
        query: String,
    },
    /// Leaderboard bar -- shows rankings for a song.
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
    LeaderBoard {
        song_data: Box<SongData>,
        from_lr2ir: bool,
    },
    /// Context menu bar -- right-click actions for a bar.
    #[allow(dead_code)] // Parsed for completeness (Java Bar enum)
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
            Bar::LeaderBoard { song_data, .. } => &song_data.title,
            Bar::ContextMenu(cm) => cm.source_bar.bar_name(),
        }
    }

    /// Returns the display type index for bar rendering.
    ///
    /// 0 = Song, 1 = Folder/Directory, 2 = Grade/Course,
    /// 3 = Command, 4 = Search, 5 = Function/Other.
    #[allow(dead_code)] // Used in tests
    pub fn bar_display_type(&self) -> i32 {
        match self {
            Bar::Song(_) | Bar::Executable { .. } | Bar::LeaderBoard { .. } => 0,
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

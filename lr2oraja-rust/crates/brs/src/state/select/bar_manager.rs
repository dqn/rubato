// BarManager — manages the song/folder bar list and cursor navigation.
//
// Provides a hierarchical browser with folder push/pop navigation.

use std::collections::HashMap;

use bms_database::{
    CourseData, CourseDataConstraint, RandomCourseData, SongData, SongDatabase, TableData,
    TableFolder,
};
use bms_rule::ScoreData;

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
#[allow(dead_code)] // Most actions reserved for future implementation
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
    #[allow(dead_code)] // Reserved for course system integration
    pub course: CourseData,
    #[allow(dead_code)] // Reserved for course system integration
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
    #[allow(dead_code)] // Used in tests and folder navigation
    Folder {
        name: String,
        path: String,
    },
    #[allow(dead_code)] // Used in tests and course selection
    Course(Box<CourseData>),
    #[allow(dead_code)] // Used in table folder display
    TableRoot {
        name: String,
        folders: Vec<TableFolder>,
        courses: Vec<CourseData>,
    },
    #[allow(dead_code)] // Used in table folder display
    HashFolder {
        name: String,
        hashes: Vec<String>, // sha256 preferred, md5 fallback
    },
    /// Executable bar — runs a set of songs (e.g., autoplay playlist).
    #[allow(dead_code)]
    Executable {
        name: String,
        songs: Vec<SongData>,
    },
    /// Function bar — a generic action item (autoplay, practice, clipboard, etc.).
    #[allow(dead_code)]
    Function {
        title: String,
        subtitle: Option<String>,
        display_bar_type: i32,
        action: FunctionAction,
        lamp: i32,
    },
    /// Grade/dan-i bar — wraps a course with grade constraints.
    #[allow(dead_code)]
    Grade(Box<GradeBarData>),
    /// Random course bar — selects random songs from SQL queries.
    #[allow(dead_code)]
    RandomCourse(Box<RandomCourseData>),
    // --- Directory bars (expand into child bars on enter) ---
    /// Command bar — executes a SQL query against the song DB.
    #[allow(dead_code)]
    Command {
        name: String,
        sql: String,
    },
    /// Container bar — holds an explicit list of child bars.
    #[allow(dead_code)]
    Container {
        name: String,
        children: Vec<Bar>,
    },
    /// Same-folder bar — finds songs sharing the same folder CRC.
    #[allow(dead_code)]
    SameFolder {
        name: String,
        folder_crc: String,
    },
    /// Search word bar — pre-configured text search.
    #[allow(dead_code)]
    SearchWord {
        query: String,
    },
    /// Leaderboard bar — shows rankings for a song.
    #[allow(dead_code)]
    LeaderBoard {
        song_data: Box<SongData>,
        from_lr2ir: bool,
    },
    /// Context menu bar — right-click actions for a bar.
    #[allow(dead_code)]
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
    #[allow(dead_code)] // Reserved for skin DST field integration
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

/// Manages the bar list, cursor position, and folder navigation stack.
pub struct BarManager {
    bars: Vec<Bar>,
    cursor: usize,
    folder_stack: Vec<(Vec<Bar>, usize)>,
}

impl BarManager {
    pub fn new() -> Self {
        Self {
            bars: Vec::new(),
            cursor: 0,
            folder_stack: Vec::new(),
        }
    }

    /// Load all songs from the database as a flat list.
    pub fn load_root(&mut self, song_db: &SongDatabase) {
        let songs = song_db.get_all_song_datas().unwrap_or_default();
        self.bars = songs.into_iter().map(|s| Bar::Song(Box::new(s))).collect();
        self.cursor = 0;
        self.folder_stack.clear();
    }

    /// Move cursor by delta with wrap-around.
    pub fn move_cursor(&mut self, delta: i32) {
        if self.bars.is_empty() {
            return;
        }
        let len = self.bars.len() as i32;
        let new_pos = ((self.cursor as i32 + delta) % len + len) % len;
        self.cursor = new_pos as usize;
    }

    /// Enter the currently selected folder.
    /// Pushes current bars and cursor onto the stack, loads folder contents.
    pub fn enter_folder(&mut self, song_db: &SongDatabase) {
        match self.bars.get(self.cursor) {
            Some(Bar::Folder { path, .. }) => {
                let folder_path = path.clone();
                let old_bars = std::mem::take(&mut self.bars);
                let old_cursor = self.cursor;
                self.folder_stack.push((old_bars, old_cursor));

                let songs = song_db
                    .get_song_datas("folder", &folder_path)
                    .unwrap_or_default();
                self.bars = songs.into_iter().map(|s| Bar::Song(Box::new(s))).collect();
                self.cursor = 0;
            }
            Some(Bar::TableRoot {
                folders, courses, ..
            }) => {
                let folders = folders.clone();
                let courses = courses.clone();
                let old_bars = std::mem::take(&mut self.bars);
                let old_cursor = self.cursor;
                self.folder_stack.push((old_bars, old_cursor));

                let mut new_bars: Vec<Bar> = Vec::new();
                // Add level folders as HashFolder bars
                for folder in &folders {
                    let hashes: Vec<String> = folder
                        .songs
                        .iter()
                        .map(|s| {
                            if !s.sha256.is_empty() {
                                s.sha256.clone()
                            } else {
                                s.md5.clone()
                            }
                        })
                        .collect();
                    new_bars.push(Bar::HashFolder {
                        name: folder.name.clone(),
                        hashes,
                    });
                }
                // Add courses
                for course in &courses {
                    new_bars.push(Bar::Course(Box::new(course.clone())));
                }
                self.bars = new_bars;
                self.cursor = 0;
            }
            Some(Bar::HashFolder { hashes, .. }) => {
                let hashes = hashes.clone();
                let old_bars = std::mem::take(&mut self.bars);
                let old_cursor = self.cursor;
                self.folder_stack.push((old_bars, old_cursor));

                let hash_refs: Vec<&str> = hashes.iter().map(String::as_str).collect();
                let songs = song_db
                    .get_song_datas_by_hashes(&hash_refs)
                    .unwrap_or_default();
                self.bars = songs.into_iter().map(|s| Bar::Song(Box::new(s))).collect();
                self.cursor = 0;
            }
            Some(Bar::Container { children, .. }) => {
                let children = children.clone();
                let old_bars = std::mem::take(&mut self.bars);
                let old_cursor = self.cursor;
                self.folder_stack.push((old_bars, old_cursor));
                self.bars = children;
                self.cursor = 0;
            }
            Some(Bar::SameFolder { folder_crc, .. }) => {
                let crc = folder_crc.clone();
                let old_bars = std::mem::take(&mut self.bars);
                let old_cursor = self.cursor;
                self.folder_stack.push((old_bars, old_cursor));

                // Search for songs by folder CRC (stub: returns empty if method unavailable)
                let songs = song_db.get_song_datas("folder", &crc).unwrap_or_default();
                self.bars = songs.into_iter().map(|s| Bar::Song(Box::new(s))).collect();
                self.cursor = 0;
            }
            Some(Bar::SearchWord { query }) => {
                let query = query.clone();
                let old_bars = std::mem::take(&mut self.bars);
                let old_cursor = self.cursor;
                self.folder_stack.push((old_bars, old_cursor));

                let songs = song_db.get_song_datas_by_text(&query).unwrap_or_default();
                self.bars = songs.into_iter().map(|s| Bar::Song(Box::new(s))).collect();
                self.cursor = 0;
            }
            Some(Bar::Command { sql, .. }) => {
                let sql = sql.clone();
                let old_bars = std::mem::take(&mut self.bars);
                let old_cursor = self.cursor;
                self.folder_stack.push((old_bars, old_cursor));

                // Execute custom SQL query (stub: uses text search as safe fallback)
                let songs = song_db.get_song_datas_by_text(&sql).unwrap_or_default();
                self.bars = songs.into_iter().map(|s| Bar::Song(Box::new(s))).collect();
                self.cursor = 0;
            }
            Some(Bar::ContextMenu(cm)) => {
                let items = cm.items.clone();
                let old_bars = std::mem::take(&mut self.bars);
                let old_cursor = self.cursor;
                self.folder_stack.push((old_bars, old_cursor));

                // Expand context menu items as Function bars
                self.bars = items
                    .into_iter()
                    .map(|item| Bar::Function {
                        title: item.label,
                        subtitle: None,
                        display_bar_type: 3,
                        action: item.action,
                        lamp: 0,
                    })
                    .collect();
                self.cursor = 0;
            }
            _ => (),
        }
    }

    /// Leave the current folder, restoring the parent bar list and cursor.
    pub fn leave_folder(&mut self) {
        if let Some((bars, cursor)) = self.folder_stack.pop() {
            self.bars = bars;
            self.cursor = cursor;
        }
    }

    /// Returns the bar at the current cursor position.
    pub fn current(&self) -> Option<&Bar> {
        self.bars.get(self.cursor)
    }

    /// Returns the total number of bars.
    pub fn bar_count(&self) -> usize {
        self.bars.len()
    }

    /// Returns the current cursor position.
    pub fn cursor_pos(&self) -> usize {
        self.cursor
    }

    /// Returns a slice of all bars in the current list.
    pub fn bars(&self) -> &[Bar] {
        &self.bars
    }

    /// Returns true if currently inside a folder (not at root).
    pub fn is_in_folder(&self) -> bool {
        !self.folder_stack.is_empty()
    }

    /// Load table data from cache and add TableRoot bars to the root bar list.
    pub fn load_tables(&mut self, tables: &[TableData]) {
        for table in tables {
            self.bars.push(Bar::TableRoot {
                name: table.name.clone(),
                folders: table.folder.clone(),
                courses: table.course.clone(),
            });
        }
    }

    /// Load course data and add them as bars.
    #[allow(dead_code)] // Used in tests and course mode
    pub fn add_courses(&mut self, courses: &[CourseData]) {
        for course in courses {
            self.bars.push(Bar::Course(Box::new(course.clone())));
        }
    }

    /// Sort bars by the given mode.
    ///
    /// Sort order for non-Song bars: Folders first, then Courses (by name).
    /// Score-dependent modes (Clear, Score, MissCount, Duration, LastUpdate) use
    /// the `score_cache` keyed by SHA-256.
    pub fn sort(&mut self, mode: SortMode, score_cache: &HashMap<String, ScoreData>) {
        match mode {
            SortMode::Default => {} // Keep original order
            SortMode::Title => {
                self.bars.sort_by(|a, b| {
                    a.bar_name()
                        .to_lowercase()
                        .cmp(&b.bar_name().to_lowercase())
                });
            }
            SortMode::Artist => {
                self.bars.sort_by(|a, b| {
                    let artist_a = match a {
                        Bar::Song(s) => s.artist.as_str(),
                        _ => "",
                    };
                    let artist_b = match b {
                        Bar::Song(s) => s.artist.as_str(),
                        _ => "",
                    };
                    artist_a.to_lowercase().cmp(&artist_b.to_lowercase())
                });
            }
            SortMode::Level => {
                self.bars.sort_by(|a, b| {
                    let level_a = match a {
                        Bar::Song(s) => s.level,
                        _ => 0,
                    };
                    let level_b = match b {
                        Bar::Song(s) => s.level,
                        _ => 0,
                    };
                    level_a.cmp(&level_b)
                });
            }
            SortMode::Bpm => {
                self.bars.sort_by(|a, b| {
                    let bpm_a = match a {
                        Bar::Song(s) => s.maxbpm,
                        _ => 0,
                    };
                    let bpm_b = match b {
                        Bar::Song(s) => s.maxbpm,
                        _ => 0,
                    };
                    bpm_a.cmp(&bpm_b)
                });
            }
            SortMode::Length => {
                self.bars.sort_by(|a, b| {
                    let len_a = match a {
                        Bar::Song(s) => s.length,
                        _ => 0,
                    };
                    let len_b = match b {
                        Bar::Song(s) => s.length,
                        _ => 0,
                    };
                    len_a.cmp(&len_b)
                });
            }
            SortMode::Clear => {
                self.bars.sort_by(|a, b| {
                    let clear_a = bar_score_field(a, score_cache, |sd| sd.clear.id() as i32, 0);
                    let clear_b = bar_score_field(b, score_cache, |sd| sd.clear.id() as i32, 0);
                    clear_a.cmp(&clear_b)
                });
            }
            SortMode::Score => {
                self.bars.sort_by(|a, b| {
                    let score_a = bar_score_field(a, score_cache, |sd| sd.exscore(), 0);
                    let score_b = bar_score_field(b, score_cache, |sd| sd.exscore(), 0);
                    score_b.cmp(&score_a) // Descending
                });
            }
            SortMode::MissCount => {
                self.bars.sort_by(|a, b| {
                    let bp_a = bar_score_field(a, score_cache, |sd| sd.minbp, i32::MAX);
                    let bp_b = bar_score_field(b, score_cache, |sd| sd.minbp, i32::MAX);
                    bp_a.cmp(&bp_b) // Ascending (fewer misses first)
                });
            }
            SortMode::Duration => {
                self.bars.sort_by(|a, b| {
                    let pc_a = bar_score_field(a, score_cache, |sd| sd.playcount, 0);
                    let pc_b = bar_score_field(b, score_cache, |sd| sd.playcount, 0);
                    pc_b.cmp(&pc_a) // Descending (most played first)
                });
            }
            SortMode::LastUpdate => {
                self.bars.sort_by(|a, b| {
                    let date_a = bar_score_field_i64(a, score_cache, |sd| sd.date, 0);
                    let date_b = bar_score_field_i64(b, score_cache, |sd| sd.date, 0);
                    date_b.cmp(&date_a) // Descending (most recent first)
                });
            }
        }
        self.cursor = 0;
    }

    /// Filter bars to retain only songs matching the given mode ID.
    /// Non-Song bars are always retained.
    pub fn filter_by_mode(&mut self, mode: Option<i32>) {
        if let Some(mode_id) = mode {
            self.bars.retain(|bar| match bar {
                Bar::Song(s) => s.mode == mode_id,
                _ => true,
            });
            self.cursor = 0;
        }
    }

    /// Replace the current folder's bars with new bars (e.g., from IR fetch).
    pub fn replace_current_bars(&mut self, bars: Vec<Bar>) {
        self.bars = bars;
        self.cursor = 0;
    }

    /// Push the current bars onto the folder stack and set new bars.
    ///
    /// Used by leaderboard entry where we don't have a `SongDatabase` reference
    /// but still need the push/pop folder navigation pattern.
    pub fn push_and_set_bars(&mut self, bars: Vec<Bar>) {
        let old_bars = std::mem::take(&mut self.bars);
        let old_cursor = self.cursor;
        self.folder_stack.push((old_bars, old_cursor));
        self.bars = bars;
        self.cursor = 0;
    }

    /// Search for songs matching the query text, pushing the current bar list onto the folder stack.
    pub fn search(&mut self, song_db: &SongDatabase, query: &str) {
        let songs = song_db.get_song_datas_by_text(query).unwrap_or_default();
        // Save current state to folder stack
        let old_bars = std::mem::take(&mut self.bars);
        let old_cursor = self.cursor;
        self.folder_stack.push((old_bars, old_cursor));
        self.bars = songs.into_iter().map(|s| Bar::Song(Box::new(s))).collect();
        self.cursor = 0;
    }
}

/// Extract an i32 field from a score associated with a Bar::Song.
fn bar_score_field(
    bar: &Bar,
    cache: &HashMap<String, ScoreData>,
    extract: impl Fn(&ScoreData) -> i32,
    default: i32,
) -> i32 {
    match bar {
        Bar::Song(s) => cache.get(&s.sha256).map(&extract).unwrap_or(default),
        _ => default,
    }
}

/// Extract an i64 field from a score associated with a Bar::Song.
fn bar_score_field_i64(
    bar: &Bar,
    cache: &HashMap<String, ScoreData>,
    extract: impl Fn(&ScoreData) -> i64,
    default: i64,
) -> i64 {
    match bar {
        Bar::Song(s) => cache.get(&s.sha256).map(&extract).unwrap_or(default),
        _ => default,
    }
}

impl Default for BarManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl BarManager {
    /// Set bars directly for testing purposes.
    pub fn set_bars_for_test(&mut self, bars: Vec<Bar>) {
        self.bars = bars;
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let bm = BarManager::new();
        assert_eq!(bm.bar_count(), 0);
        assert_eq!(bm.cursor_pos(), 0);
        assert!(bm.current().is_none());
    }

    #[test]
    fn load_root_populates_bars() {
        let db = SongDatabase::open_in_memory().unwrap();
        let songs = vec![
            SongData {
                md5: "aaa".to_string(),
                sha256: "aaa_sha".to_string(),
                title: "Song A".to_string(),
                path: "a.bms".to_string(),
                ..Default::default()
            },
            SongData {
                md5: "bbb".to_string(),
                sha256: "bbb_sha".to_string(),
                title: "Song B".to_string(),
                path: "b.bms".to_string(),
                ..Default::default()
            },
        ];
        db.set_song_datas(&songs).unwrap();

        let mut bm = BarManager::new();
        bm.load_root(&db);
        assert_eq!(bm.bar_count(), 2);
        assert_eq!(bm.cursor_pos(), 0);
    }

    #[test]
    fn load_root_empty_db() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.load_root(&db);
        assert_eq!(bm.bar_count(), 0);
    }

    #[test]
    fn move_cursor_wraps_forward() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "A".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "B".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "C".to_string(),
                ..Default::default()
            })),
        ];
        bm.cursor = 2;
        bm.move_cursor(1);
        assert_eq!(bm.cursor_pos(), 0);
    }

    #[test]
    fn move_cursor_wraps_backward() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "A".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "B".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "C".to_string(),
                ..Default::default()
            })),
        ];
        bm.cursor = 0;
        bm.move_cursor(-1);
        assert_eq!(bm.cursor_pos(), 2);
    }

    #[test]
    fn move_cursor_empty_is_noop() {
        let mut bm = BarManager::new();
        bm.move_cursor(1);
        assert_eq!(bm.cursor_pos(), 0);
    }

    #[test]
    fn enter_and_leave_folder() {
        let db = SongDatabase::open_in_memory().unwrap();
        // Insert songs in a folder
        let songs = vec![SongData {
            md5: "ccc".to_string(),
            sha256: "ccc_sha".to_string(),
            title: "Folder Song".to_string(),
            path: "folder/c.bms".to_string(),
            folder: "my_folder".to_string(),
            ..Default::default()
        }];
        db.set_song_datas(&songs).unwrap();

        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Folder {
                name: "My Folder".to_string(),
                path: "my_folder".to_string(),
            },
            Bar::Song(Box::new(SongData {
                title: "Root Song".to_string(),
                ..Default::default()
            })),
        ];
        bm.cursor = 0;

        // Enter folder
        bm.enter_folder(&db);
        assert_eq!(bm.bar_count(), 1);
        assert_eq!(bm.cursor_pos(), 0);
        assert!(bm.is_in_folder());
        match bm.current() {
            Some(Bar::Song(sd)) => assert_eq!(sd.title, "Folder Song"),
            _ => panic!("expected Song bar"),
        }

        // Leave folder
        bm.leave_folder();
        assert_eq!(bm.bar_count(), 2);
        assert_eq!(bm.cursor_pos(), 0);
        assert!(!bm.is_in_folder());
    }

    #[test]
    fn enter_folder_on_song_is_noop() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Song(Box::new(SongData {
            title: "Song".to_string(),
            ..Default::default()
        }))];
        bm.cursor = 0;
        bm.enter_folder(&db);
        // Should not push to stack
        assert!(!bm.is_in_folder());
        assert_eq!(bm.bar_count(), 1);
    }

    #[test]
    fn leave_folder_at_root_is_noop() {
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Song(Box::new(SongData {
            title: "Song".to_string(),
            ..Default::default()
        }))];
        bm.leave_folder();
        assert_eq!(bm.bar_count(), 1);
    }

    #[test]
    fn sort_by_title_alphabetical() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Charlie".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Alpha".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Bravo".to_string(),
                ..Default::default()
            })),
        ];
        bm.cursor = 2;
        bm.sort(SortMode::Title, &HashMap::new());
        assert_eq!(bm.cursor_pos(), 0);
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Alpha"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.title, "Bravo"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[2] {
            Bar::Song(s) => assert_eq!(s.title, "Charlie"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_by_level_ascending() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Hard".to_string(),
                level: 12,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Easy".to_string(),
                level: 3,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Medium".to_string(),
                level: 7,
                ..Default::default()
            })),
        ];
        bm.sort(SortMode::Level, &HashMap::new());
        assert_eq!(bm.cursor_pos(), 0);
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.level, 3),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.level, 7),
            _ => panic!("expected Song"),
        }
        match &bm.bars[2] {
            Bar::Song(s) => assert_eq!(s.level, 12),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn filter_by_mode_retains_matching() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "7K Song".to_string(),
                mode: 7,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "14K Song".to_string(),
                mode: 14,
                ..Default::default()
            })),
            Bar::Folder {
                name: "Folder".to_string(),
                path: "f".to_string(),
            },
            Bar::Song(Box::new(SongData {
                title: "Another 7K".to_string(),
                mode: 7,
                ..Default::default()
            })),
        ];
        bm.cursor = 2;
        bm.filter_by_mode(Some(7));
        assert_eq!(bm.cursor_pos(), 0);
        // Should retain: 7K Song, Folder, Another 7K (3 bars)
        assert_eq!(bm.bar_count(), 3);
        // 14K Song should be removed
        for bar in &bm.bars {
            if let Bar::Song(s) = bar {
                assert_eq!(s.mode, 7);
            }
        }
    }

    #[test]
    fn filter_by_mode_none_is_noop() {
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Song(Box::new(SongData {
            title: "Song".to_string(),
            mode: 7,
            ..Default::default()
        }))];
        bm.filter_by_mode(None);
        assert_eq!(bm.bar_count(), 1);
    }

    #[test]
    fn search_pushes_to_folder_stack() {
        let db = SongDatabase::open_in_memory().unwrap();
        let songs = vec![
            SongData {
                md5: "aaa".to_string(),
                sha256: "aaa_sha".to_string(),
                title: "Find Me".to_string(),
                path: "a.bms".to_string(),
                ..Default::default()
            },
            SongData {
                md5: "bbb".to_string(),
                sha256: "bbb_sha".to_string(),
                title: "Other Song".to_string(),
                path: "b.bms".to_string(),
                ..Default::default()
            },
        ];
        db.set_song_datas(&songs).unwrap();

        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Song(Box::new(SongData {
            title: "Root".to_string(),
            ..Default::default()
        }))];
        bm.cursor = 0;

        bm.search(&db, "Find");
        assert!(bm.is_in_folder());
        assert_eq!(bm.bar_count(), 1);
        assert_eq!(bm.cursor_pos(), 0);
        match bm.current() {
            Some(Bar::Song(s)) => assert_eq!(s.title, "Find Me"),
            _ => panic!("expected Song with title 'Find Me'"),
        }

        // Leave search results should restore original
        bm.leave_folder();
        assert_eq!(bm.bar_count(), 1);
        assert!(!bm.is_in_folder());
    }

    #[test]
    fn sort_mode_cycles() {
        assert_eq!(SortMode::Default.next(), SortMode::Title);
        assert_eq!(SortMode::Title.next(), SortMode::Artist);
        assert_eq!(SortMode::Artist.next(), SortMode::Level);
        assert_eq!(SortMode::Level.next(), SortMode::Bpm);
        assert_eq!(SortMode::Bpm.next(), SortMode::Length);
        assert_eq!(SortMode::Length.next(), SortMode::Clear);
        assert_eq!(SortMode::Clear.next(), SortMode::Score);
        assert_eq!(SortMode::Score.next(), SortMode::MissCount);
        assert_eq!(SortMode::MissCount.next(), SortMode::Duration);
        assert_eq!(SortMode::Duration.next(), SortMode::LastUpdate);
        assert_eq!(SortMode::LastUpdate.next(), SortMode::Default);
    }

    fn sample_course(name: &str) -> CourseData {
        use bms_database::CourseSongData;
        CourseData {
            name: name.to_string(),
            hash: vec![CourseSongData {
                sha256: "abc".to_string(),
                md5: String::new(),
                title: "Stage 1".to_string(),
            }],
            constraint: Vec::new(),
            trophy: Vec::new(),
            release: true,
        }
    }

    #[test]
    fn add_courses_appends_bars() {
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Song(Box::new(SongData {
            title: "Song".to_string(),
            ..Default::default()
        }))];

        let courses = vec![sample_course("Course A"), sample_course("Course B")];
        bm.add_courses(&courses);
        assert_eq!(bm.bar_count(), 3);
        match &bm.bars[1] {
            Bar::Course(c) => assert_eq!(c.name, "Course A"),
            _ => panic!("expected Course bar"),
        }
        match &bm.bars[2] {
            Bar::Course(c) => assert_eq!(c.name, "Course B"),
            _ => panic!("expected Course bar"),
        }
    }

    #[test]
    fn sort_by_title_includes_courses() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Zebra".to_string(),
                ..Default::default()
            })),
            Bar::Course(Box::new(sample_course("Alpha Course"))),
            Bar::Folder {
                name: "Middle Folder".to_string(),
                path: "f".to_string(),
            },
        ];
        bm.sort(SortMode::Title, &HashMap::new());

        // Expected order: "Alpha Course", "Middle Folder", "Zebra"
        match &bm.bars[0] {
            Bar::Course(c) => assert_eq!(c.name, "Alpha Course"),
            _ => panic!("expected Course bar at index 0"),
        }
        match &bm.bars[1] {
            Bar::Folder { name, .. } => assert_eq!(name, "Middle Folder"),
            _ => panic!("expected Folder bar at index 1"),
        }
        match &bm.bars[2] {
            Bar::Song(s) => assert_eq!(s.title, "Zebra"),
            _ => panic!("expected Song bar at index 2"),
        }
    }

    #[test]
    fn filter_by_mode_retains_courses() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "7K Song".to_string(),
                mode: 7,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "14K Song".to_string(),
                mode: 14,
                ..Default::default()
            })),
            Bar::Course(Box::new(sample_course("My Course"))),
        ];
        bm.filter_by_mode(Some(7));
        // Should retain: 7K Song + Course (2 bars), 14K removed
        assert_eq!(bm.bar_count(), 2);
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.mode, 7),
            _ => panic!("expected 7K Song"),
        }
        match &bm.bars[1] {
            Bar::Course(c) => assert_eq!(c.name, "My Course"),
            _ => panic!("expected Course bar"),
        }
    }

    #[test]
    fn sort_by_artist_courses_sort_as_empty() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Song".to_string(),
                artist: "Beta Artist".to_string(),
                ..Default::default()
            })),
            Bar::Course(Box::new(sample_course("Course"))),
        ];
        bm.sort(SortMode::Artist, &HashMap::new());
        // Course has empty artist, so it sorts before "Beta Artist"
        match &bm.bars[0] {
            Bar::Course(c) => assert_eq!(c.name, "Course"),
            _ => panic!("expected Course bar first"),
        }
    }

    #[test]
    fn sort_by_level_courses_sort_as_zero() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Hard".to_string(),
                level: 12,
                ..Default::default()
            })),
            Bar::Course(Box::new(sample_course("Course"))),
        ];
        bm.sort(SortMode::Level, &HashMap::new());
        // Course has level 0, so it sorts before level 12
        match &bm.bars[0] {
            Bar::Course(c) => assert_eq!(c.name, "Course"),
            _ => panic!("expected Course bar first"),
        }
    }

    // --- Table / HashFolder tests ---

    fn sample_table_folder(name: &str, hashes: &[&str]) -> TableFolder {
        use bms_database::CourseSongData;
        TableFolder {
            name: name.to_string(),
            songs: hashes
                .iter()
                .map(|h| CourseSongData {
                    sha256: h.to_string(),
                    md5: String::new(),
                    title: format!("Song {h}"),
                })
                .collect(),
        }
    }

    fn sample_table_data(name: &str) -> TableData {
        TableData {
            url: "https://example.com/table".to_string(),
            name: name.to_string(),
            tag: "T".to_string(),
            folder: vec![
                sample_table_folder("Level 1", &["sha_a", "sha_b"]),
                sample_table_folder("Level 2", &["sha_c"]),
            ],
            course: vec![CourseData {
                name: "Dan Course".to_string(),
                hash: vec![bms_database::CourseSongData {
                    sha256: "sha_d".to_string(),
                    md5: String::new(),
                    title: "Stage 1".to_string(),
                }],
                constraint: Vec::new(),
                trophy: Vec::new(),
                release: true,
            }],
        }
    }

    #[test]
    fn load_tables_adds_table_root_bars() {
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Song(Box::new(SongData {
            title: "Existing".to_string(),
            ..Default::default()
        }))];

        let tables = vec![
            sample_table_data("Insane Table"),
            sample_table_data("Normal Table"),
        ];
        bm.load_tables(&tables);

        // Original bar + 2 TableRoot bars = 3
        assert_eq!(bm.bar_count(), 3);
        match &bm.bars[1] {
            Bar::TableRoot {
                name,
                folders,
                courses,
            } => {
                assert_eq!(name, "Insane Table");
                assert_eq!(folders.len(), 2);
                assert_eq!(courses.len(), 1);
            }
            _ => panic!("expected TableRoot bar at index 1"),
        }
        match &bm.bars[2] {
            Bar::TableRoot { name, .. } => assert_eq!(name, "Normal Table"),
            _ => panic!("expected TableRoot bar at index 2"),
        }
    }

    #[test]
    fn enter_table_root_expands_to_hash_folders_and_courses() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();

        let table = sample_table_data("Test Table");
        bm.bars = vec![Bar::TableRoot {
            name: table.name.clone(),
            folders: table.folder.clone(),
            courses: table.course.clone(),
        }];
        bm.cursor = 0;

        bm.enter_folder(&db);

        // Should have 2 HashFolder bars + 1 Course bar = 3
        assert_eq!(bm.bar_count(), 3);
        assert!(bm.is_in_folder());

        match &bm.bars[0] {
            Bar::HashFolder { name, hashes } => {
                assert_eq!(name, "Level 1");
                assert_eq!(hashes, &["sha_a", "sha_b"]);
            }
            _ => panic!("expected HashFolder bar at index 0"),
        }
        match &bm.bars[1] {
            Bar::HashFolder { name, hashes } => {
                assert_eq!(name, "Level 2");
                assert_eq!(hashes, &["sha_c"]);
            }
            _ => panic!("expected HashFolder bar at index 1"),
        }
        match &bm.bars[2] {
            Bar::Course(c) => assert_eq!(c.name, "Dan Course"),
            _ => panic!("expected Course bar at index 2"),
        }
    }

    #[test]
    fn enter_hash_folder_resolves_songs() {
        let db = SongDatabase::open_in_memory().unwrap();
        let songs = vec![
            SongData {
                md5: "md5_a".to_string(),
                sha256: "sha_aaa_long_enough_for_sha256_detection_aaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
                title: "Song A".to_string(),
                path: "a.bms".to_string(),
                ..Default::default()
            },
            SongData {
                md5: "md5_b".to_string(),
                sha256: "sha_bbb_long_enough_for_sha256_detection_bbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_string(),
                title: "Song B".to_string(),
                path: "b.bms".to_string(),
                ..Default::default()
            },
        ];
        db.set_song_datas(&songs).unwrap();

        let mut bm = BarManager::new();
        bm.bars = vec![Bar::HashFolder {
            name: "Level 1".to_string(),
            hashes: vec![
                "sha_aaa_long_enough_for_sha256_detection_aaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                "sha_bbb_long_enough_for_sha256_detection_bbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                "sha_nonexistent".to_string(), // This one won't match
            ],
        }];
        bm.cursor = 0;

        bm.enter_folder(&db);

        assert!(bm.is_in_folder());
        assert_eq!(bm.bar_count(), 2);
        // Verify all bars are Song bars
        for bar in &bm.bars {
            assert!(matches!(bar, Bar::Song(_)));
        }
    }

    #[test]
    fn sort_handles_table_root_and_hash_folder() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Zebra Song".to_string(),
                ..Default::default()
            })),
            Bar::TableRoot {
                name: "Alpha Table".to_string(),
                folders: Vec::new(),
                courses: Vec::new(),
            },
            Bar::HashFolder {
                name: "Middle Folder".to_string(),
                hashes: Vec::new(),
            },
        ];
        bm.sort(SortMode::Title, &HashMap::new());

        // Expected order: "Alpha Table", "Middle Folder", "Zebra Song"
        match &bm.bars[0] {
            Bar::TableRoot { name, .. } => assert_eq!(name, "Alpha Table"),
            _ => panic!("expected TableRoot bar at index 0"),
        }
        match &bm.bars[1] {
            Bar::HashFolder { name, .. } => assert_eq!(name, "Middle Folder"),
            _ => panic!("expected HashFolder bar at index 1"),
        }
        match &bm.bars[2] {
            Bar::Song(s) => assert_eq!(s.title, "Zebra Song"),
            _ => panic!("expected Song bar at index 2"),
        }
    }

    // --- New sort mode tests ---

    fn make_score(
        sha256: &str,
        clear: bms_rule::ClearType,
        exscore_epg: i32,
        minbp: i32,
        playcount: i32,
        date: i64,
    ) -> ScoreData {
        ScoreData {
            sha256: sha256.to_string(),
            clear,
            epg: exscore_epg, // exscore = epg*2
            minbp,
            playcount,
            date,
            ..Default::default()
        }
    }

    fn make_score_cache(scores: &[ScoreData]) -> HashMap<String, ScoreData> {
        scores
            .iter()
            .map(|s| (s.sha256.clone(), s.clone()))
            .collect()
    }

    #[test]
    fn sort_by_bpm_ascending() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Fast".to_string(),
                maxbpm: 200,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Slow".to_string(),
                maxbpm: 100,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Medium".to_string(),
                maxbpm: 150,
                ..Default::default()
            })),
        ];
        bm.sort(SortMode::Bpm, &HashMap::new());
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.maxbpm, 100),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.maxbpm, 150),
            _ => panic!("expected Song"),
        }
        match &bm.bars[2] {
            Bar::Song(s) => assert_eq!(s.maxbpm, 200),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_by_length_ascending() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Long".to_string(),
                length: 300_000,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Short".to_string(),
                length: 60_000,
                ..Default::default()
            })),
        ];
        bm.sort(SortMode::Length, &HashMap::new());
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Short"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.title, "Long"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_by_clear_ascending() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_a".to_string(),
                title: "Hard Clear".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_b".to_string(),
                title: "No Play".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_c".to_string(),
                title: "Easy Clear".to_string(),
                ..Default::default()
            })),
        ];
        let cache = make_score_cache(&[
            make_score("sha_a", bms_rule::ClearType::Hard, 0, 0, 0, 0),
            make_score("sha_c", bms_rule::ClearType::Easy, 0, 0, 0, 0),
        ]);
        bm.sort(SortMode::Clear, &cache);
        // No Play (0) < Easy (4) < Hard (6)
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "No Play"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.title, "Easy Clear"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[2] {
            Bar::Song(s) => assert_eq!(s.title, "Hard Clear"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_by_score_descending() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_low".to_string(),
                title: "Low Score".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_high".to_string(),
                title: "High Score".to_string(),
                ..Default::default()
            })),
        ];
        let cache = make_score_cache(&[
            make_score("sha_low", bms_rule::ClearType::default(), 50, 0, 0, 0),
            make_score("sha_high", bms_rule::ClearType::default(), 200, 0, 0, 0),
        ]);
        bm.sort(SortMode::Score, &cache);
        // Descending: high first
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "High Score"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_by_misscount_ascending_no_score_at_end() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_none".to_string(),
                title: "No Score".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_few".to_string(),
                title: "Few Miss".to_string(),
                ..Default::default()
            })),
        ];
        let cache = make_score_cache(&[make_score(
            "sha_few",
            bms_rule::ClearType::default(),
            0,
            5,
            0,
            0,
        )]);
        bm.sort(SortMode::MissCount, &cache);
        // Few Miss (5) < No Score (i32::MAX)
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Few Miss"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.title, "No Score"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_by_duration_descending() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_a".to_string(),
                title: "Rarely Played".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_b".to_string(),
                title: "Often Played".to_string(),
                ..Default::default()
            })),
        ];
        let cache = make_score_cache(&[
            make_score("sha_a", bms_rule::ClearType::default(), 0, 0, 3, 0),
            make_score("sha_b", bms_rule::ClearType::default(), 0, 0, 100, 0),
        ]);
        bm.sort(SortMode::Duration, &cache);
        // Descending: most played first
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Often Played"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_by_last_update_descending() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_old".to_string(),
                title: "Old".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_new".to_string(),
                title: "New".to_string(),
                ..Default::default()
            })),
        ];
        let cache = make_score_cache(&[
            make_score("sha_old", bms_rule::ClearType::default(), 0, 0, 0, 1000),
            make_score("sha_new", bms_rule::ClearType::default(), 0, 0, 0, 9999),
        ]);
        bm.sort(SortMode::LastUpdate, &cache);
        // Descending: most recent first
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "New"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.title, "Old"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_bpm_non_song_bars_at_start() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Song".to_string(),
                maxbpm: 180,
                ..Default::default()
            })),
            Bar::Folder {
                name: "Folder".to_string(),
                path: "f".to_string(),
            },
        ];
        bm.sort(SortMode::Bpm, &HashMap::new());
        // Folder has bpm 0, so it sorts before bpm 180
        match &bm.bars[0] {
            Bar::Folder { name, .. } => assert_eq!(name, "Folder"),
            _ => panic!("expected Folder first"),
        }
    }

    #[test]
    fn filter_retains_table_root_and_hash_folder() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "7K Song".to_string(),
                mode: 7,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "14K Song".to_string(),
                mode: 14,
                ..Default::default()
            })),
            Bar::TableRoot {
                name: "Table".to_string(),
                folders: Vec::new(),
                courses: Vec::new(),
            },
            Bar::HashFolder {
                name: "Hash".to_string(),
                hashes: Vec::new(),
            },
        ];
        bm.filter_by_mode(Some(7));
        // Should retain: 7K Song + TableRoot + HashFolder = 3 (14K removed)
        assert_eq!(bm.bar_count(), 3);
        assert!(matches!(&bm.bars[0], Bar::Song(s) if s.mode == 7));
        assert!(matches!(&bm.bars[1], Bar::TableRoot { .. }));
        assert!(matches!(&bm.bars[2], Bar::HashFolder { .. }));
    }

    // --- New bar variant tests ---

    #[test]
    fn bar_name_returns_correct_names() {
        let song = Bar::Song(Box::new(SongData {
            title: "My Song".to_string(),
            ..Default::default()
        }));
        assert_eq!(song.bar_name(), "My Song");

        let folder = Bar::Folder {
            name: "My Folder".to_string(),
            path: "f".to_string(),
        };
        assert_eq!(folder.bar_name(), "My Folder");

        let exec = Bar::Executable {
            name: "Exec Bar".to_string(),
            songs: Vec::new(),
        };
        assert_eq!(exec.bar_name(), "Exec Bar");

        let func = Bar::Function {
            title: "Func Title".to_string(),
            subtitle: None,
            display_bar_type: 0,
            action: FunctionAction::None,
            lamp: 0,
        };
        assert_eq!(func.bar_name(), "Func Title");

        let grade = Bar::Grade(Box::new(GradeBarData {
            name: "Dan 10".to_string(),
            course: CourseData::default(),
            constraints: Vec::new(),
        }));
        assert_eq!(grade.bar_name(), "Dan 10");

        let rc = Bar::RandomCourse(Box::new(bms_database::RandomCourseData {
            name: "Random Dan".to_string(),
            ..Default::default()
        }));
        assert_eq!(rc.bar_name(), "Random Dan");

        let cmd = Bar::Command {
            name: "Recent".to_string(),
            sql: "SELECT *".to_string(),
        };
        assert_eq!(cmd.bar_name(), "Recent");

        let container = Bar::Container {
            name: "Container".to_string(),
            children: Vec::new(),
        };
        assert_eq!(container.bar_name(), "Container");

        let same = Bar::SameFolder {
            name: "Same Folder".to_string(),
            folder_crc: "abc".to_string(),
        };
        assert_eq!(same.bar_name(), "Same Folder");

        let search = Bar::SearchWord {
            query: "freedom".to_string(),
        };
        assert_eq!(search.bar_name(), "freedom");

        let leader = Bar::LeaderBoard {
            song_data: Box::new(SongData {
                title: "Leader Song".to_string(),
                ..Default::default()
            }),
            from_lr2ir: true,
        };
        assert_eq!(leader.bar_name(), "Leader Song");

        let ctx_menu = Bar::ContextMenu(Box::new(ContextMenuData {
            source_bar: Box::new(Bar::Song(Box::new(SongData {
                title: "Source".to_string(),
                ..Default::default()
            }))),
            items: Vec::new(),
        }));
        assert_eq!(ctx_menu.bar_name(), "Source");
    }

    #[test]
    fn bar_display_type_classification() {
        assert_eq!(
            Bar::Song(Box::new(SongData::default())).bar_display_type(),
            0
        );
        assert_eq!(
            Bar::Executable {
                name: "x".to_string(),
                songs: Vec::new()
            }
            .bar_display_type(),
            0
        );
        assert_eq!(
            Bar::LeaderBoard {
                song_data: Box::new(SongData::default()),
                from_lr2ir: false
            }
            .bar_display_type(),
            0
        );
        assert_eq!(
            Bar::Folder {
                name: "f".to_string(),
                path: "p".to_string()
            }
            .bar_display_type(),
            1
        );
        assert_eq!(
            Bar::Container {
                name: "c".to_string(),
                children: Vec::new()
            }
            .bar_display_type(),
            1
        );
        assert_eq!(
            Bar::SameFolder {
                name: "sf".to_string(),
                folder_crc: "crc".to_string()
            }
            .bar_display_type(),
            1
        );
        assert_eq!(
            Bar::Course(Box::new(CourseData::default())).bar_display_type(),
            2
        );
        assert_eq!(
            Bar::Grade(Box::new(GradeBarData {
                name: "g".to_string(),
                course: CourseData::default(),
                constraints: Vec::new(),
            }))
            .bar_display_type(),
            2
        );
        assert_eq!(
            Bar::Command {
                name: "c".to_string(),
                sql: "s".to_string()
            }
            .bar_display_type(),
            3
        );
        assert_eq!(
            Bar::SearchWord {
                query: "q".to_string()
            }
            .bar_display_type(),
            4
        );
        assert_eq!(
            Bar::Function {
                title: "f".to_string(),
                subtitle: None,
                display_bar_type: 7,
                action: FunctionAction::None,
                lamp: 0
            }
            .bar_display_type(),
            7
        );
    }

    #[test]
    fn enter_container_expands_children() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Container {
            name: "My Container".to_string(),
            children: vec![
                Bar::Song(Box::new(SongData {
                    title: "Child A".to_string(),
                    ..Default::default()
                })),
                Bar::Song(Box::new(SongData {
                    title: "Child B".to_string(),
                    ..Default::default()
                })),
            ],
        }];
        bm.cursor = 0;
        bm.enter_folder(&db);

        assert!(bm.is_in_folder());
        assert_eq!(bm.bar_count(), 2);
        assert_eq!(bm.bars[0].bar_name(), "Child A");
        assert_eq!(bm.bars[1].bar_name(), "Child B");

        bm.leave_folder();
        assert!(!bm.is_in_folder());
        assert_eq!(bm.bar_count(), 1);
    }

    #[test]
    fn enter_search_word_executes_text_search() {
        let db = SongDatabase::open_in_memory().unwrap();
        let songs = vec![SongData {
            md5: "md5_search".to_string(),
            sha256: "sha_search".to_string(),
            title: "Searchable Song".to_string(),
            path: "search.bms".to_string(),
            ..Default::default()
        }];
        db.set_song_datas(&songs).unwrap();

        let mut bm = BarManager::new();
        bm.bars = vec![Bar::SearchWord {
            query: "Searchable".to_string(),
        }];
        bm.cursor = 0;
        bm.enter_folder(&db);

        assert!(bm.is_in_folder());
        assert_eq!(bm.bar_count(), 1);
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Searchable Song"),
            _ => panic!("expected Song bar"),
        }
    }

    #[test]
    fn enter_same_folder_queries_by_folder() {
        let db = SongDatabase::open_in_memory().unwrap();
        let songs = vec![SongData {
            md5: "md5_same".to_string(),
            sha256: "sha_same".to_string(),
            title: "Same Folder Song".to_string(),
            path: "same/a.bms".to_string(),
            folder: "my_folder_crc".to_string(),
            ..Default::default()
        }];
        db.set_song_datas(&songs).unwrap();

        let mut bm = BarManager::new();
        bm.bars = vec![Bar::SameFolder {
            name: "Same Folder".to_string(),
            folder_crc: "my_folder_crc".to_string(),
        }];
        bm.cursor = 0;
        bm.enter_folder(&db);

        assert!(bm.is_in_folder());
        assert_eq!(bm.bar_count(), 1);
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Same Folder Song"),
            _ => panic!("expected Song bar"),
        }
    }

    #[test]
    fn enter_command_bar_pushes_to_stack() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Command {
            name: "Recent".to_string(),
            sql: "nonexistent_query".to_string(),
        }];
        bm.cursor = 0;
        bm.enter_folder(&db);

        assert!(bm.is_in_folder());
        // SQL query won't match anything, so empty results
        assert_eq!(bm.bar_count(), 0);
    }

    #[test]
    fn enter_context_menu_expands_to_function_bars() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::ContextMenu(Box::new(ContextMenuData {
            source_bar: Box::new(Bar::Song(Box::new(SongData {
                title: "Source Song".to_string(),
                ..Default::default()
            }))),
            items: vec![
                ContextMenuItem {
                    label: "Copy Hash".to_string(),
                    action: FunctionAction::CopyToClipboard("abc".to_string()),
                },
                ContextMenuItem {
                    label: "Open URL".to_string(),
                    action: FunctionAction::OpenUrl("https://example.com".to_string()),
                },
            ],
        }))];
        bm.cursor = 0;
        bm.enter_folder(&db);

        assert!(bm.is_in_folder());
        assert_eq!(bm.bar_count(), 2);
        assert_eq!(bm.bars[0].bar_name(), "Copy Hash");
        assert_eq!(bm.bars[1].bar_name(), "Open URL");
        assert!(matches!(&bm.bars[0], Bar::Function { .. }));
        assert!(matches!(&bm.bars[1], Bar::Function { .. }));
    }

    #[test]
    fn sort_includes_new_bar_types_by_title() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Zebra".to_string(),
                ..Default::default()
            })),
            Bar::Container {
                name: "Alpha Container".to_string(),
                children: Vec::new(),
            },
            Bar::Command {
                name: "Middle Command".to_string(),
                sql: String::new(),
            },
        ];
        bm.sort(SortMode::Title, &HashMap::new());

        assert_eq!(bm.bars[0].bar_name(), "Alpha Container");
        assert_eq!(bm.bars[1].bar_name(), "Middle Command");
        assert_eq!(bm.bars[2].bar_name(), "Zebra");
    }

    #[test]
    fn filter_retains_new_non_song_bars() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "7K Song".to_string(),
                mode: 7,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "14K Song".to_string(),
                mode: 14,
                ..Default::default()
            })),
            Bar::Container {
                name: "Container".to_string(),
                children: Vec::new(),
            },
            Bar::Command {
                name: "Cmd".to_string(),
                sql: String::new(),
            },
            Bar::SearchWord {
                query: "q".to_string(),
            },
            Bar::Grade(Box::new(GradeBarData {
                name: "Dan".to_string(),
                course: CourseData::default(),
                constraints: Vec::new(),
            })),
        ];
        bm.filter_by_mode(Some(7));
        // 14K removed, everything else retained = 5
        assert_eq!(bm.bar_count(), 5);
        // Verify 14K is gone
        for bar in &bm.bars {
            if let Bar::Song(s) = bar {
                assert_eq!(s.mode, 7);
            }
        }
    }

    #[test]
    fn enter_folder_on_executable_is_noop() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Executable {
            name: "Exec".to_string(),
            songs: Vec::new(),
        }];
        bm.cursor = 0;
        bm.enter_folder(&db);
        // Executable is not a directory type, should be noop
        assert!(!bm.is_in_folder());
        assert_eq!(bm.bar_count(), 1);
    }

    #[test]
    fn enter_folder_on_function_is_noop() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Function {
            title: "Func".to_string(),
            subtitle: None,
            display_bar_type: 0,
            action: FunctionAction::None,
            lamp: 0,
        }];
        bm.cursor = 0;
        bm.enter_folder(&db);
        assert!(!bm.is_in_folder());
        assert_eq!(bm.bar_count(), 1);
    }
}

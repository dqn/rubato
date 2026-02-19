// BarManager -- manages the song/folder bar list and cursor navigation.
//
// Provides a hierarchical browser with folder push/pop navigation.

mod bar_navigation;
mod bar_sort;
mod bar_types;

use std::collections::HashMap;

use bms_rule::ScoreData;

pub use bar_types::*;

/// Manages the bar list, cursor position, and folder navigation stack.
pub struct BarManager {
    pub(super) bars: Vec<Bar>,
    pub(super) cursor: usize,
    pub(super) folder_stack: Vec<(Vec<Bar>, usize)>,
    /// Search history (most recent at end).
    search_history: Vec<String>,
    /// Maximum number of search history entries.
    max_search_bar_count: usize,
    /// Rival score cache: sha256 → ScoreData (for rival compare sort modes).
    pub(super) rival_scores: HashMap<String, ScoreData>,
    /// Whether to show bars for songs whose files don't exist on disk.
    show_no_song_existing_bar: bool,
}

impl BarManager {
    pub fn new() -> Self {
        Self {
            bars: Vec::new(),
            cursor: 0,
            folder_stack: Vec::new(),
            search_history: Vec::new(),
            max_search_bar_count: 10,
            rival_scores: HashMap::new(),
            show_no_song_existing_bar: true,
        }
    }

    /// Set the maximum number of search history entries.
    pub fn set_max_search_bar_count(&mut self, count: usize) {
        self.max_search_bar_count = count;
    }

    /// Set whether to show bars for songs whose files don't exist on disk.
    ///
    /// Java parity: `Config.isShowNoSongExistingBar()`.
    pub fn set_show_no_song_existing_bar(&mut self, show: bool) {
        self.show_no_song_existing_bar = show;
    }

    /// Add a search query to the history.
    ///
    /// Java parity: `BarManager.addSearch()` L550-561. Deduplicates by title,
    /// respects `maxSearchBarCount` limit, removes oldest when full.
    pub fn add_search(&mut self, query: String) {
        // Remove duplicate if already in history
        self.search_history.retain(|s| s != &query);
        // Enforce max count (remove oldest)
        if self.search_history.len() >= self.max_search_bar_count {
            self.search_history.remove(0);
        }
        self.search_history.push(query);
    }

    /// Returns the search history entries.
    #[allow(dead_code)] // Used in tests; skin state wiring deferred
    pub fn search_history(&self) -> &[String] {
        &self.search_history
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

    /// Set the rival score cache for rival compare sort modes.
    pub fn set_rival_scores(&mut self, scores: HashMap<String, ScoreData>) {
        self.rival_scores = scores;
    }

    /// Returns true if rival scores are loaded (for sort mode cycling).
    pub fn has_rival(&self) -> bool {
        !self.rival_scores.is_empty()
    }

    /// Look up a rival's score by song sha256 hash.
    pub fn rival_score(&self, sha256: &str) -> Option<&ScoreData> {
        self.rival_scores.get(sha256)
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

    use std::collections::HashMap;

    use bms_database::{CourseData, SongData, SongDatabase, TableData};
    use bms_rule::ScoreData;

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
        // Songs must have distinct folder CRCs to produce separate Folder bars.
        let songs = vec![
            SongData {
                md5: "aaa".to_string(),
                sha256: "aaa_sha".to_string(),
                title: "Song A".to_string(),
                path: "folder_a/a.bms".to_string(),
                folder: "crc_a".to_string(),
                ..Default::default()
            },
            SongData {
                md5: "bbb".to_string(),
                sha256: "bbb_sha".to_string(),
                title: "Song B".to_string(),
                path: "folder_b/b.bms".to_string(),
                folder: "crc_b".to_string(),
                ..Default::default()
            },
        ];
        db.set_song_datas(&songs).unwrap();

        let mut bm = BarManager::new();
        bm.load_root(&db);
        // Two distinct folder CRCs → two Folder bars
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
    fn sort_mode_cycles_without_rival() {
        assert_eq!(SortMode::Default.next(false), SortMode::Title);
        assert_eq!(SortMode::Title.next(false), SortMode::Artist);
        assert_eq!(SortMode::Artist.next(false), SortMode::Level);
        assert_eq!(SortMode::Level.next(false), SortMode::Bpm);
        assert_eq!(SortMode::Bpm.next(false), SortMode::Length);
        assert_eq!(SortMode::Length.next(false), SortMode::Clear);
        assert_eq!(SortMode::Clear.next(false), SortMode::Score);
        assert_eq!(SortMode::Score.next(false), SortMode::MissCount);
        assert_eq!(SortMode::MissCount.next(false), SortMode::Duration);
        assert_eq!(SortMode::Duration.next(false), SortMode::LastUpdate);
        assert_eq!(SortMode::LastUpdate.next(false), SortMode::Default);
    }

    #[test]
    fn sort_mode_cycles_with_rival() {
        assert_eq!(SortMode::LastUpdate.next(true), SortMode::RivalCompareClear);
        assert_eq!(
            SortMode::RivalCompareClear.next(true),
            SortMode::RivalCompareScore
        );
        assert_eq!(SortMode::RivalCompareScore.next(true), SortMode::Default);
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
        // Non-Song falls back to TITLE sort: "Course" < "Song" alphabetically
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
        // Non-Song falls back to TITLE sort: "Course" < "Hard" alphabetically
        match &bm.bars[0] {
            Bar::Course(c) => assert_eq!(c.name, "Course"),
            _ => panic!("expected Course bar first"),
        }
    }

    // --- Table / HashFolder tests ---

    fn sample_table_folder(name: &str, hashes: &[&str]) -> bms_database::TableFolder {
        use bms_database::CourseSongData;
        bms_database::TableFolder {
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
                ..
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
            url: None,
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
                url: None,
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
                title: "No Score".to_string(),
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
        // Java parity: Easy(4) < Hard(6) < No Score(end)
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Easy Clear"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.title, "Hard Clear"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[2] {
            Bar::Song(s) => assert_eq!(s.title, "No Score"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_by_score_ratio_ascending() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_high_ratio".to_string(),
                title: "High Ratio".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_low_ratio".to_string(),
                title: "Low Ratio".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_no_notes".to_string(),
                title: "No Notes".to_string(),
                ..Default::default()
            })),
        ];
        // High Ratio: exscore=200 (epg=100), notes=200 → ratio=1.0
        // Low Ratio: exscore=100 (epg=50), notes=500 → ratio=0.2
        // No Notes: notes=0 → end
        let mut s1 = make_score(
            "sha_high_ratio",
            bms_rule::ClearType::default(),
            100,
            0,
            0,
            0,
        );
        s1.notes = 200;
        let mut s2 = make_score("sha_low_ratio", bms_rule::ClearType::default(), 50, 0, 0, 0);
        s2.notes = 500;
        let mut s3 = make_score("sha_no_notes", bms_rule::ClearType::default(), 50, 0, 0, 0);
        s3.notes = 0;
        let cache = make_score_cache(&[s1, s2, s3]);
        bm.sort(SortMode::Score, &cache);
        // Java parity ascending: Low Ratio(0.2) < High Ratio(1.0) < No Notes(end)
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Low Ratio"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.title, "High Ratio"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[2] {
            Bar::Song(s) => assert_eq!(s.title, "No Notes"),
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
    fn sort_by_duration_avgjudge_ascending() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_long".to_string(),
                title: "Long Duration".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_short".to_string(),
                title: "Short Duration".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_none".to_string(),
                title: "No Duration".to_string(),
                ..Default::default()
            })),
        ];
        // avgjudge: lower = shorter duration
        let mut s1 = make_score("sha_long", bms_rule::ClearType::default(), 0, 0, 0, 0);
        s1.avgjudge = 90000;
        let mut s2 = make_score("sha_short", bms_rule::ClearType::default(), 0, 0, 0, 0);
        s2.avgjudge = 30000;
        // sha_none: default avgjudge = i64::MAX → treated as no data
        let s3 = make_score("sha_none", bms_rule::ClearType::default(), 0, 0, 0, 0);
        let cache = make_score_cache(&[s1, s2, s3]);
        bm.sort(SortMode::Duration, &cache);
        // Java parity ascending: Short(30000) < Long(90000) < No Duration(MAX → end)
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Short Duration"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.title, "Long Duration"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[2] {
            Bar::Song(s) => assert_eq!(s.title, "No Duration"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_by_last_update_ascending() {
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
        // Java parity ascending: Old(1000) < New(9999)
        match &bm.bars[0] {
            Bar::Song(s) => assert_eq!(s.title, "Old"),
            _ => panic!("expected Song"),
        }
        match &bm.bars[1] {
            Bar::Song(s) => assert_eq!(s.title, "New"),
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
        // Non-Song falls back to TITLE sort: "Folder" < "Song" alphabetically
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
                url: None,
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
    fn enter_folder_on_executable_expands_songs() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Executable {
            name: "Exec".to_string(),
            songs: vec![
                SongData {
                    title: "Song A".to_string(),
                    ..Default::default()
                },
                SongData {
                    title: "Song B".to_string(),
                    ..Default::default()
                },
            ],
        }];
        bm.cursor = 0;
        bm.enter_folder(&db);
        assert!(bm.is_in_folder());
        assert_eq!(bm.bar_count(), 2);
        assert_eq!(bm.bars[0].bar_name(), "Song A");
        assert_eq!(bm.bars[1].bar_name(), "Song B");
    }

    #[test]
    fn enter_folder_on_empty_executable_pushes_empty() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Executable {
            name: "Empty".to_string(),
            songs: Vec::new(),
        }];
        bm.cursor = 0;
        bm.enter_folder(&db);
        assert!(bm.is_in_folder());
        assert_eq!(bm.bar_count(), 0);
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

    #[test]
    fn load_builtin_containers_adds_two_containers() {
        let mut bm = BarManager::new();
        bm.load_builtin_containers();
        assert_eq!(bm.bar_count(), 2);
        assert_eq!(bm.bars[0].bar_name(), "LAMP UPDATE");
        assert_eq!(bm.bars[1].bar_name(), "SCORE UPDATE");
    }

    #[test]
    fn search_adds_to_history() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.bars = vec![Bar::Song(Box::new(SongData::default()))];
        bm.search(&db, "test_query");
        assert_eq!(bm.search_history(), &["test_query"]);
    }

    #[test]
    fn search_history_deduplicates() {
        let mut bm = BarManager::new();
        bm.add_search("alpha".to_string());
        bm.add_search("beta".to_string());
        bm.add_search("alpha".to_string()); // duplicate
        assert_eq!(bm.search_history(), &["beta", "alpha"]);
    }

    #[test]
    fn search_history_respects_max_count() {
        let mut bm = BarManager::new();
        bm.set_max_search_bar_count(3);
        bm.add_search("a".to_string());
        bm.add_search("b".to_string());
        bm.add_search("c".to_string());
        bm.add_search("d".to_string()); // evicts "a"
        assert_eq!(bm.search_history(), &["b", "c", "d"]);
    }

    #[test]
    fn load_root_includes_search_history() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut bm = BarManager::new();
        bm.add_search("freedom".to_string());
        bm.add_search("zenith".to_string());
        bm.load_root(&db);
        // Empty DB → no folder bars, but search history should be present
        assert_eq!(bm.bar_count(), 2);
        assert!(matches!(&bm.bars[0], Bar::SearchWord { query } if query == "freedom"));
        assert!(matches!(&bm.bars[1], Bar::SearchWord { query } if query == "zenith"));
    }

    #[test]
    fn sort_mode_id_round_trip() {
        let modes = [
            SortMode::Default,
            SortMode::Title,
            SortMode::Artist,
            SortMode::Level,
            SortMode::Bpm,
            SortMode::Length,
            SortMode::Clear,
            SortMode::Score,
            SortMode::MissCount,
            SortMode::Duration,
            SortMode::LastUpdate,
            SortMode::RivalCompareClear,
            SortMode::RivalCompareScore,
        ];
        for mode in modes {
            let id = mode.to_id();
            let restored = SortMode::from_id(id);
            assert_eq!(mode, restored, "round-trip failed for {id}");
        }
    }

    #[test]
    fn sort_mode_from_id_unknown_returns_default() {
        assert_eq!(SortMode::from_id("UNKNOWN"), SortMode::Default);
        assert_eq!(SortMode::from_id(""), SortMode::Default);
    }

    #[test]
    fn sort_restores_cursor_to_same_song() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_c".to_string(),
                title: "Charlie".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_a".to_string(),
                title: "Alpha".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_b".to_string(),
                title: "Bravo".to_string(),
                ..Default::default()
            })),
        ];
        // Cursor on "Bravo" (index 2)
        bm.cursor = 2;
        bm.sort(SortMode::Title, &HashMap::new());
        // After sort: ["Alpha", "Bravo", "Charlie"]
        // Cursor should be restored to "Bravo" at index 1
        assert_eq!(bm.cursor_pos(), 1);
        match &bm.bars[bm.cursor_pos()] {
            Bar::Song(s) => assert_eq!(s.title, "Bravo"),
            _ => panic!("expected Song"),
        }
    }

    #[test]
    fn sort_restores_cursor_to_non_song_bar() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_z".to_string(),
                title: "Zebra".to_string(),
                ..Default::default()
            })),
            Bar::Folder {
                name: "My Folder".to_string(),
                path: "f".to_string(),
            },
            Bar::Song(Box::new(SongData {
                sha256: "sha_a".to_string(),
                title: "Alpha".to_string(),
                ..Default::default()
            })),
        ];
        // Cursor on "My Folder" (index 1)
        bm.cursor = 1;
        bm.sort(SortMode::Title, &HashMap::new());
        // After sort: ["Alpha", "My Folder", "Zebra"]
        // Cursor should be restored to "My Folder" at index 1
        assert_eq!(bm.cursor_pos(), 1);
        assert_eq!(bm.bars[bm.cursor_pos()].bar_name(), "My Folder");
    }

    #[test]
    fn command_folder_with_rcourse_creates_random_course_bars() {
        let mut bm = BarManager::new();
        let dir = tempfile::tempdir().unwrap();
        let json_path = dir.path().join("default.json");
        std::fs::write(
            &json_path,
            r#"[{
                "name": "Random Courses",
                "folder": [{"name": "Sub", "sql": "level > 5"}],
                "rcourse": [
                    {"name": "Random Dan", "stage": [{"title": "S1", "sql": "level > 10"}]},
                    {"name": "Random Dan 2", "stage": [{"title": "S1", "sql": "level > 12"}]}
                ]
            }]"#,
        )
        .unwrap();
        bm.load_command_folders(json_path.to_str().unwrap());

        assert_eq!(bm.bar_count(), 1);
        match &bm.bars[0] {
            Bar::Container { name, children } => {
                assert_eq!(name, "Random Courses");
                // 1 subfolder + 2 random courses = 3 children
                assert_eq!(children.len(), 3);
                assert!(matches!(&children[0], Bar::Command { name, .. } if name == "Sub"));
                assert!(matches!(&children[1], Bar::RandomCourse(rc) if rc.name == "Random Dan"));
                assert!(matches!(&children[2], Bar::RandomCourse(rc) if rc.name == "Random Dan 2"));
            }
            _ => panic!("expected Container bar"),
        }
    }

    #[test]
    fn filter_invisible_removes_flagged_songs() {
        use bms_database::song_data::{FAVORITE_SONG, INVISIBLE_CHART, INVISIBLE_SONG};

        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                title: "Visible".to_string(),
                favorite: 0,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Invisible Song".to_string(),
                favorite: INVISIBLE_SONG,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Invisible Chart".to_string(),
                favorite: INVISIBLE_CHART,
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                title: "Both Flags".to_string(),
                favorite: INVISIBLE_SONG | INVISIBLE_CHART,
                ..Default::default()
            })),
            Bar::Folder {
                name: "Folder".to_string(),
                path: "f".to_string(),
            },
            Bar::Song(Box::new(SongData {
                title: "Fav Only".to_string(),
                favorite: FAVORITE_SONG,
                ..Default::default()
            })),
        ];
        bm.filter_invisible();
        // Should retain: Visible, Folder, Fav Only = 3
        assert_eq!(bm.bar_count(), 3);
        assert_eq!(bm.bars[0].bar_name(), "Visible");
        assert_eq!(bm.bars[1].bar_name(), "Folder");
        assert_eq!(bm.bars[2].bar_name(), "Fav Only");
    }

    #[test]
    fn builtin_containers_have_30_command_children() {
        let mut bm = BarManager::new();
        bm.load_builtin_containers();

        for bar in &bm.bars {
            match bar {
                Bar::Container { children, .. } => {
                    assert_eq!(children.len(), 30);
                    assert_eq!(children[0].bar_name(), "TODAY");
                    assert_eq!(children[1].bar_name(), "1DAYS AGO");
                    assert_eq!(children[29].bar_name(), "29DAYS AGO");
                    for child in children {
                        assert!(matches!(child, Bar::Command { .. }));
                    }
                }
                _ => panic!("expected Container bar"),
            }
        }
    }

    #[test]
    fn sort_rival_compare_clear_descending_diff() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_a".to_string(),
                title: "Song A".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_b".to_string(),
                title: "Song B".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_c".to_string(),
                title: "Song C".to_string(),
                ..Default::default()
            })),
        ];
        // Player scores: A=Hard(6), B=Easy(4), C=Normal(5)
        let player_cache = make_score_cache(&[
            make_score("sha_a", bms_rule::ClearType::Hard, 0, 0, 0, 0),
            make_score("sha_b", bms_rule::ClearType::Easy, 0, 0, 0, 0),
            make_score("sha_c", bms_rule::ClearType::Normal, 0, 0, 0, 0),
        ]);
        // Rival scores: A=Easy(4), B=Hard(6), C=Normal(5)
        let rival_cache: HashMap<String, ScoreData> = [
            make_score("sha_a", bms_rule::ClearType::Easy, 0, 0, 0, 0),
            make_score("sha_b", bms_rule::ClearType::Hard, 0, 0, 0, 0),
            make_score("sha_c", bms_rule::ClearType::Normal, 0, 0, 0, 0),
        ]
        .iter()
        .map(|s| (s.sha256.clone(), s.clone()))
        .collect();
        bm.rival_scores = rival_cache;

        bm.sort(SortMode::RivalCompareClear, &player_cache);
        // Diffs: A = 6-4=2, B = 4-6=-2, C = 5-5=0
        // Descending: A(2), C(0), B(-2)
        assert_eq!(bm.bars[0].bar_name(), "Song A");
        assert_eq!(bm.bars[1].bar_name(), "Song C");
        assert_eq!(bm.bars[2].bar_name(), "Song B");
    }

    #[test]
    fn sort_rival_compare_score_descending_diff() {
        let mut bm = BarManager::new();
        bm.bars = vec![
            Bar::Song(Box::new(SongData {
                sha256: "sha_x".to_string(),
                title: "Song X".to_string(),
                ..Default::default()
            })),
            Bar::Song(Box::new(SongData {
                sha256: "sha_y".to_string(),
                title: "Song Y".to_string(),
                ..Default::default()
            })),
        ];
        // Player: X exscore=200 (epg=100), Y exscore=100 (epg=50)
        let player_cache = make_score_cache(&[
            make_score("sha_x", bms_rule::ClearType::default(), 100, 0, 0, 0),
            make_score("sha_y", bms_rule::ClearType::default(), 50, 0, 0, 0),
        ]);
        // Rival: X exscore=300 (epg=150), Y exscore=50 (epg=25)
        let rival_cache: HashMap<String, ScoreData> = [
            make_score("sha_x", bms_rule::ClearType::default(), 150, 0, 0, 0),
            make_score("sha_y", bms_rule::ClearType::default(), 25, 0, 0, 0),
        ]
        .iter()
        .map(|s| (s.sha256.clone(), s.clone()))
        .collect();
        bm.rival_scores = rival_cache;

        bm.sort(SortMode::RivalCompareScore, &player_cache);
        // Diffs: X = 200-300=-100, Y = 100-50=50
        // Descending: Y(50), X(-100)
        assert_eq!(bm.bars[0].bar_name(), "Song Y");
        assert_eq!(bm.bars[1].bar_name(), "Song X");
    }

    #[test]
    fn has_rival_reflects_scores() {
        let mut bm = BarManager::new();
        assert!(!bm.has_rival());

        let mut scores = HashMap::new();
        scores.insert(
            "sha".to_string(),
            ScoreData {
                sha256: "sha".to_string(),
                ..Default::default()
            },
        );
        bm.set_rival_scores(scores);
        assert!(bm.has_rival());

        bm.set_rival_scores(HashMap::new());
        assert!(!bm.has_rival());
    }
}

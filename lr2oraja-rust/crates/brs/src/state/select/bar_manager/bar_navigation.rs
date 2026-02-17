// Bar navigation — folder enter/leave, load, and search methods for BarManager.

use bms_database::{SongDatabase, TableData};

use super::BarManager;
use super::bar_types::Bar;

impl BarManager {
    /// Load all songs from the database as a flat list.
    pub fn load_root(&mut self, song_db: &SongDatabase) {
        let songs = song_db.get_all_song_datas().unwrap_or_default();
        self.bars = songs.into_iter().map(|s| Bar::Song(Box::new(s))).collect();
        self.cursor = 0;
        self.folder_stack.clear();
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
    #[allow(dead_code)] // Used in tests
    pub fn add_courses(&mut self, courses: &[bms_database::CourseData]) {
        for course in courses {
            self.bars.push(Bar::Course(Box::new(course.clone())));
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

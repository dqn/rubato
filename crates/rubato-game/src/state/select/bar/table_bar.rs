use std::sync::Arc;

use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::grade_bar::GradeBar;
use super::hash_bar::HashBar;
use crate::state::select::*;

/// Difficulty table bar
/// Translates: bms.player.beatoraja.select.bar.TableBar
#[derive(Clone)]
pub struct TableBar {
    pub directory: DirectoryBarData,
    /// Table data
    pub td: TableData,
    /// Level bars
    pub levels: Vec<HashBar>,
    /// Course bars
    pub grades: Vec<GradeBar>,
    /// Level bars + course bars combined
    pub children: Vec<Bar>,
    /// Table accessor (Arc for cheap cloning)
    pub tr: Arc<dyn TableAccessor>,
}

impl TableBar {
    pub fn new(td: TableData, tr: Arc<dyn TableAccessor>) -> Self {
        let mut bar = Self {
            directory: DirectoryBarData::default(),
            td: TableData::default(),
            levels: Vec::new(),
            grades: Vec::new(),
            children: Vec::new(),
            tr,
        };
        bar.set_table_data(td);
        bar
    }

    pub fn title(&self) -> &str {
        &self.td.name
    }

    pub fn url(&self) -> Option<&str> {
        self.td.url_opt()
    }

    pub fn accessor(&self) -> &dyn TableAccessor {
        self.tr.as_ref()
    }

    pub fn set_table_data(&mut self, td: TableData) {
        self.levels = td
            .folder
            .iter()
            .map(|folder| HashBar::new(folder.name().to_string(), folder.songs.to_vec()))
            .collect();

        self.grades = td
            .course
            .iter()
            .map(|course| GradeBar::new(course.clone()))
            .collect();

        // children = levels + grades combined (same order as Java: levels first, then grades)
        self.children = self
            .levels
            .iter()
            .map(|h| Bar::Hash(Box::new(h.clone())))
            .chain(self.grades.iter().map(|g| Bar::Grade(Box::new(g.clone()))))
            .collect();

        self.td = td;
    }

    /// Resolve course song data from the song database.
    /// Merges local SongData (with file paths) into each GradeBar's course entries,
    /// so that `GradeBar::exists_all_songs()` returns true when charts are available locally.
    pub fn resolve_grades(&mut self, db: &dyn SongDatabaseAccessor) {
        // Collect all unique hashes from all courses
        let mut all_hashes: Vec<String> = Vec::new();
        for grade in &self.grades {
            for song in &grade.course.hash {
                let hash = if !song.file.sha256.is_empty() {
                    song.file.sha256.clone()
                } else {
                    song.file.md5.clone()
                };
                if !hash.is_empty() {
                    all_hashes.push(hash);
                }
            }
        }

        if all_hashes.is_empty() {
            return;
        }

        // Batch lookup from DB
        let db_songs = db.song_datas_by_hashes(&all_hashes);

        // Build a lookup map: hash -> SongData (with file path)
        let mut song_map: std::collections::HashMap<&str, &SongData> =
            std::collections::HashMap::new();
        for song in &db_songs {
            if !song.file.sha256.is_empty() {
                song_map.insert(&song.file.sha256, song);
            }
            if !song.file.md5.is_empty() {
                song_map.insert(&song.file.md5, song);
            }
        }

        // Merge DB results into each grade's course songs
        for grade in &mut self.grades {
            for song in &mut grade.course.hash {
                let matched = if !song.file.sha256.is_empty() {
                    song_map.get(song.file.sha256.as_str())
                } else {
                    song_map.get(song.file.md5.as_str())
                };
                if let Some(db_song) = matched {
                    // Merge the DB song's file info (path, etc.) into the table entry
                    song.file = db_song.file.clone();
                    // Preserve metadata from DB if table entry is sparse
                    if song.metadata.title.is_empty() {
                        song.metadata = db_song.metadata.clone();
                    }
                }
            }
        }

        // Rebuild children with resolved grades
        self.children = self
            .levels
            .iter()
            .map(|h| Bar::Hash(Box::new(h.clone())))
            .chain(self.grades.iter().map(|g| Bar::Grade(Box::new(g.clone()))))
            .collect();
    }

    pub fn levels(&self) -> &[HashBar] {
        &self.levels
    }

    pub fn grades(&self) -> &[GradeBar] {
        &self.grades
    }

    pub fn children(&self) -> &[Bar] {
        &self.children
    }

    pub fn table_data(&self) -> &TableData {
        &self.td
    }
}

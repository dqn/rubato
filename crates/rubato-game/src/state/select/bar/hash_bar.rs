use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::state::select::*;

/// Hash collection folder bar
/// Translates: bms.player.beatoraja.select.bar.HashBar
#[derive(Clone)]
pub struct HashBar {
    pub directory: DirectoryBarData,
    pub title: String,
    pub elements: Vec<SongData>,
    pub elements_hash: Vec<String>,
}

impl HashBar {
    pub fn new(title: String, elements: Vec<SongData>) -> Self {
        let elements_hash = elements
            .iter()
            .map(|e| {
                if !e.file.sha256.is_empty() {
                    e.file.sha256.to_string()
                } else {
                    e.file.md5.to_string()
                }
            })
            .collect();
        Self {
            directory: DirectoryBarData::default(),
            title,
            elements,
            elements_hash,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn elements(&self) -> &[SongData] {
        &self.elements
    }

    pub fn set_elements(&mut self, elements: Vec<SongData>) {
        self.elements_hash = elements
            .iter()
            .map(|e| {
                if !e.file.sha256.is_empty() {
                    e.file.sha256.to_string()
                } else {
                    e.file.md5.to_string()
                }
            })
            .collect();
        self.elements = elements;
    }

    /// Get children bars for this hash collection.
    /// Queries the song database by hashes and matches against elements.
    ///
    /// Translates: Java HashBar.getChildren()
    pub fn children(&self, db: &dyn SongDatabaseAccessor) -> Vec<Bar> {
        let mut songs: Vec<Option<SongData>> = db
            .song_datas_by_hashes(&self.elements_hash)
            .into_iter()
            .map(Some)
            .collect();
        let mut elements = self.elements.clone();
        SongBar::to_song_bar_array_with_elements(&mut songs, &mut elements)
    }

    pub fn update_folder_status(&mut self, db: &dyn SongDatabaseAccessor) {
        let songs = db.song_datas_by_hashes(&self.elements_hash);
        self.directory
            .update_folder_status_with_songs(&songs, None, |_| None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestSongDb;

    #[test]
    fn hash_bar_get_children_returns_matched_songs() {
        let mut element = SongData::default();
        element.metadata.title = "Element Song".to_string();
        element.file.sha256 = "hash_abc".to_string();

        let mut db_song = SongData::default();
        db_song.metadata.title = "DB Song".to_string();
        db_song.file.sha256 = "hash_abc".to_string();
        db_song.file.set_path("test/path.bms".to_string());

        let db = TestSongDb::new().with_songs_by_hashes(vec![db_song]);
        let bar = HashBar::new("Test Hash".to_string(), vec![element]);
        let children = bar.children(&db);

        assert!(!children.is_empty());
        // Should contain the matched song
        assert!(children.iter().any(|c| c.as_song_bar().is_some()));
    }

    #[test]
    fn hash_bar_get_children_shows_missing_elements() {
        let mut element = SongData::default();
        element.metadata.title = "Missing Song".to_string();
        element.file.sha256 = "hash_missing".to_string();

        let db = TestSongDb::new().with_songs_by_hashes(vec![]); // No songs in DB
        let bar = HashBar::new("Test Hash".to_string(), vec![element]);
        let children = bar.children(&db);

        // Missing elements should still appear as SongBars (without path)
        assert_eq!(children.len(), 1);
        assert!(children[0].as_song_bar().is_some());
    }

    #[test]
    fn hash_bar_get_children_empty_elements() {
        let db = TestSongDb::new().with_songs_by_hashes(vec![]);
        let bar = HashBar::new("Empty".to_string(), vec![]);
        let children = bar.children(&db);
        assert!(children.is_empty());
    }
}

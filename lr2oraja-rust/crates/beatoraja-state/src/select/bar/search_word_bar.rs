use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::select::stubs::*;

/// Search bar
/// Translates: bms.player.beatoraja.select.bar.SearchWordBar
#[derive(Clone)]
pub struct SearchWordBar {
    pub directory: DirectoryBarData,
    pub text: String,
    pub title: String,
}

impl SearchWordBar {
    pub fn new(title: String, text: String) -> Self {
        Self {
            directory: DirectoryBarData::default(),
            text,
            title,
        }
    }

    /// Create a SearchWordBar with auto-generated title.
    pub fn from_text(text: String) -> Self {
        let title = format!("Search : '{}'", text);
        Self::new(title, text)
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    /// Get children bars by searching song database by text.
    ///
    /// Translates: Java SearchWordBar.getChildren()
    pub fn get_children(&self, db: &dyn SongDatabaseAccessor) -> Vec<Bar> {
        let songs = db.get_song_datas_by_text(&self.text);
        SongBar::to_song_bar_array(&songs)
    }

    pub fn update_folder_status(&mut self, db: &dyn SongDatabaseAccessor) {
        let songs = db.get_song_datas_by_text(&self.text);
        self.directory
            .update_folder_status_with_songs(&songs, None, |_| None);
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_types::folder_data::FolderData;

    struct MockSongDb {
        text_songs: Vec<(String, Vec<SongData>)>,
    }

    impl MockSongDb {
        fn new() -> Self {
            Self {
                text_songs: Vec::new(),
            }
        }

        fn with_text_results(mut self, text: &str, songs: Vec<SongData>) -> Self {
            self.text_songs.push((text.to_string(), songs));
            self
        }
    }

    impl SongDatabaseAccessor for MockSongDb {
        fn get_song_datas(&self, _key: &str, _value: &str) -> Vec<SongData> {
            Vec::new()
        }
        fn get_song_datas_by_hashes(&self, _hashes: &[String]) -> Vec<SongData> {
            Vec::new()
        }
        fn get_song_datas_by_sql(
            &self,
            _sql: &str,
            _score: &str,
            _scorelog: &str,
            _info: Option<&str>,
        ) -> Vec<SongData> {
            Vec::new()
        }
        fn set_song_datas(&self, _songs: &[SongData]) {}
        fn get_song_datas_by_text(&self, text: &str) -> Vec<SongData> {
            for (t, songs) in &self.text_songs {
                if t == text {
                    return songs.clone();
                }
            }
            Vec::new()
        }
        fn get_folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
            Vec::new()
        }
    }

    #[test]
    fn search_word_bar_get_children_returns_matching_songs() {
        let mut song = SongData::default();
        song.set_title("Freedom Dive".to_string());
        song.set_sha256("fd_hash".to_string());

        let db = MockSongDb::new().with_text_results("freedom", vec![song]);

        let bar = SearchWordBar::from_text("freedom".to_string());
        let children = bar.get_children(&db);

        assert_eq!(children.len(), 1);
        assert!(children[0].as_song_bar().is_some());
        assert!(children[0].get_title().contains("Freedom Dive"));
    }

    #[test]
    fn search_word_bar_get_children_returns_empty_for_no_match() {
        let db = MockSongDb::new();

        let bar = SearchWordBar::from_text("nonexistent".to_string());
        let children = bar.get_children(&db);

        assert!(children.is_empty());
    }
}

use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::select::stubs::*;

/// Bar showing all charts in the same directory
/// Translates: bms.player.beatoraja.select.bar.SameFolderBar
#[derive(Clone)]
pub struct SameFolderBar {
    pub directory: DirectoryBarData,
    pub crc: String,
    pub title: String,
}

impl SameFolderBar {
    pub fn new(title: String, crc: String) -> Self {
        Self {
            directory: DirectoryBarData::default(),
            crc,
            title,
        }
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    /// Get children bars for same-folder songs.
    /// Queries the song database for songs with folder=crc.
    ///
    /// Translates: Java SameFolderBar.getChildren()
    pub fn get_children(&self, db: &dyn SongDatabaseAccessor) -> Vec<Bar> {
        let songs = db.get_song_datas("folder", &self.crc);
        SongBar::to_song_bar_array(&songs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_types::folder_data::FolderData;

    struct MockSongDb {
        songs: Vec<(String, String, Vec<SongData>)>,
    }

    impl MockSongDb {
        fn new() -> Self {
            Self { songs: Vec::new() }
        }

        fn with_songs(mut self, key: &str, value: &str, songs: Vec<SongData>) -> Self {
            self.songs.push((key.to_string(), value.to_string(), songs));
            self
        }
    }

    impl SongDatabaseAccessor for MockSongDb {
        fn get_song_datas(&self, key: &str, value: &str) -> Vec<SongData> {
            for (k, v, songs) in &self.songs {
                if k == key && v == value {
                    return songs.clone();
                }
            }
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
        fn get_song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
            Vec::new()
        }
        fn get_folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
            Vec::new()
        }
    }

    #[test]
    fn same_folder_bar_get_children_returns_songs_in_folder() {
        let mut song1 = SongData::default();
        song1.set_title("Song A".to_string());
        song1.set_sha256("sha_a".to_string());

        let mut song2 = SongData::default();
        song2.set_title("Song B".to_string());
        song2.set_sha256("sha_b".to_string());

        let db = MockSongDb::new().with_songs("folder", "folder_crc", vec![song1, song2]);

        let bar = SameFolderBar::new("Same Folder".to_string(), "folder_crc".to_string());
        let children = bar.get_children(&db);

        assert_eq!(children.len(), 2);
        assert!(children[0].as_song_bar().is_some());
        assert!(children[1].as_song_bar().is_some());
    }

    #[test]
    fn same_folder_bar_get_children_returns_empty_when_no_songs() {
        let db = MockSongDb::new();
        let bar = SameFolderBar::new("Empty".to_string(), "no_crc".to_string());
        let children = bar.get_children(&db);
        assert!(children.is_empty());
    }
}

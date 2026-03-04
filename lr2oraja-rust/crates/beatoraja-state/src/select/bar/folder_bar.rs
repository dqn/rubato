use std::path::Path;

use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::select::stubs::*;

/// File system-linked folder bar
/// Translates: bms.player.beatoraja.select.bar.FolderBar
#[derive(Clone)]
pub struct FolderBar {
    pub directory: DirectoryBarData,
    pub folder: Option<FolderData>,
    pub crc: String,
}

impl FolderBar {
    pub fn new(folder: Option<FolderData>, crc: String) -> Self {
        Self {
            directory: DirectoryBarData::default(),
            folder,
            crc,
        }
    }

    pub fn get_folder_data(&self) -> Option<&FolderData> {
        self.folder.as_ref()
    }

    pub fn get_crc(&self) -> &str {
        &self.crc
    }

    pub fn get_title(&self) -> String {
        self.folder
            .as_ref()
            .map(|f| f.get_title().to_string())
            .unwrap_or_default()
    }

    /// Get children bars for this folder.
    /// Queries the song database for songs with parent=crc.
    /// If songs are found, returns SongBar array.
    /// Otherwise, returns sub-folder FolderBars.
    ///
    /// Translates: Java FolderBar.getChildren()
    pub fn get_children(&self, db: &dyn SongDatabaseAccessor) -> Vec<Bar> {
        let songs = db.get_song_datas("parent", &self.crc);
        if !songs.is_empty() {
            return SongBar::to_song_bar_array(&songs);
        }

        // No songs found - return sub-folders
        let rootpath = Path::new(".")
            .canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let folders = db.get_folder_datas("parent", &self.crc);
        folders
            .into_iter()
            .map(|folder| {
                let mut path = folder.get_path().to_string();
                if path.ends_with(std::path::MAIN_SEPARATOR) {
                    path.pop();
                }
                let ccrc = beatoraja_song::song_utils::crc32(&path, &[], &rootpath);
                Bar::Folder(Box::new(FolderBar::new(Some(folder), ccrc)))
            })
            .collect()
    }

    pub fn update_folder_status(&mut self, db: &dyn SongDatabaseAccessor) {
        if let Some(ref folder) = self.folder {
            let mut path = folder.get_path().to_string();
            if path.ends_with(std::path::MAIN_SEPARATOR) {
                path.pop();
            }
            let rootpath = Path::new(".")
                .canonicalize()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let ccrc = beatoraja_song::song_utils::crc32(&path, &[], &rootpath);
            let songs = db.get_song_datas("parent", &ccrc);
            self.directory
                .update_folder_status_with_songs(&songs, None, |_| None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_types::folder_data::FolderData;
    use beatoraja_types::song_data::SongData;

    /// Mock SongDatabaseAccessor for testing
    struct MockSongDb {
        songs: Vec<(String, String, Vec<SongData>)>,
        folders: Vec<(String, String, Vec<FolderData>)>,
    }

    impl MockSongDb {
        fn new() -> Self {
            Self {
                songs: Vec::new(),
                folders: Vec::new(),
            }
        }

        fn with_songs(mut self, key: &str, value: &str, songs: Vec<SongData>) -> Self {
            self.songs.push((key.to_string(), value.to_string(), songs));
            self
        }

        fn with_folders(mut self, key: &str, value: &str, folders: Vec<FolderData>) -> Self {
            self.folders
                .push((key.to_string(), value.to_string(), folders));
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

        fn get_folder_datas(&self, key: &str, value: &str) -> Vec<FolderData> {
            for (k, v, folders) in &self.folders {
                if k == key && v == value {
                    return folders.clone();
                }
            }
            Vec::new()
        }
    }

    #[test]
    fn folder_bar_get_children_returns_song_bars_when_songs_exist() {
        let mut song = SongData::default();
        song.set_title("Test Song".to_string());
        song.set_sha256("abc123".to_string());

        let db = MockSongDb::new().with_songs("parent", "test_crc", vec![song]);

        let bar = FolderBar::new(None, "test_crc".to_string());
        let children = bar.get_children(&db);

        assert_eq!(children.len(), 1);
        assert!(children[0].as_song_bar().is_some());
        assert!(children[0].get_title().contains("Test Song"));
    }

    #[test]
    fn folder_bar_get_children_returns_folder_bars_when_no_songs() {
        let mut folder = FolderData::default();
        folder.title = "Sub Folder".to_string();
        folder.path = "/test/path".to_string();

        let db = MockSongDb::new().with_folders("parent", "test_crc", vec![folder]);

        let bar = FolderBar::new(None, "test_crc".to_string());
        let children = bar.get_children(&db);

        assert_eq!(children.len(), 1);
        assert!(children[0].as_folder_bar().is_some());
        assert_eq!(children[0].get_title(), "Sub Folder");
    }

    #[test]
    fn folder_bar_get_children_returns_empty_when_no_data() {
        let db = MockSongDb::new();
        let bar = FolderBar::new(None, "nonexistent".to_string());
        let children = bar.get_children(&db);
        assert!(children.is_empty());
    }

    #[test]
    fn folder_bar_get_children_prefers_songs_over_folders() {
        let mut song = SongData::default();
        song.set_title("Song".to_string());
        song.set_sha256("sha1".to_string());

        let mut folder = FolderData::default();
        folder.title = "Folder".to_string();
        folder.path = "/test".to_string();

        let db = MockSongDb::new()
            .with_songs("parent", "crc1", vec![song])
            .with_folders("parent", "crc1", vec![folder]);

        let bar = FolderBar::new(None, "crc1".to_string());
        let children = bar.get_children(&db);

        // Should return songs, not folders
        assert_eq!(children.len(), 1);
        assert!(children[0].as_song_bar().is_some());
    }
}

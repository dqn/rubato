use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::state::select::*;

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

    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get children bars for same-folder songs.
    /// Queries the song database for songs with folder=crc.
    ///
    /// Translates: Java SameFolderBar.getChildren()
    pub fn children(&self, db: &dyn SongDatabaseAccessor) -> Vec<Bar> {
        let songs = db.song_datas("folder", &self.crc);
        SongBar::to_song_bar_array(&songs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestSongDb;

    #[test]
    fn same_folder_bar_get_children_returns_songs_in_folder() {
        let mut song1 = SongData::default();
        song1.metadata.title = "Song A".to_string();
        song1.file.sha256 = "sha_a".to_string();

        let mut song2 = SongData::default();
        song2.metadata.title = "Song B".to_string();
        song2.file.sha256 = "sha_b".to_string();

        let db = TestSongDb::new().with_songs("folder", "folder_crc", vec![song1, song2]);

        let bar = SameFolderBar::new("Same Folder".to_string(), "folder_crc".to_string());
        let children = bar.children(&db);

        assert_eq!(children.len(), 2);
        assert!(children[0].as_song_bar().is_some());
        assert!(children[1].as_song_bar().is_some());
    }

    #[test]
    fn same_folder_bar_get_children_returns_empty_when_no_songs() {
        let db = TestSongDb::new();
        let bar = SameFolderBar::new("Empty".to_string(), "no_crc".to_string());
        let children = bar.children(&db);
        assert!(children.is_empty());
    }
}

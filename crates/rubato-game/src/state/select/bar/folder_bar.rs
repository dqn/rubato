use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::state::select::*;

/// File system-linked folder bar
/// Translates: bms.player.beatoraja.select.bar.FolderBar
#[derive(Clone)]
pub struct FolderBar {
    pub directory: DirectoryBarData,
    pub folder: Option<FolderData>,
    /// Cached title (computed from folder.title())
    title: String,
    pub crc: String,
}

impl FolderBar {
    pub fn new(folder: Option<FolderData>, crc: String) -> Self {
        let title = folder
            .as_ref()
            .map(|f| f.title().to_string())
            .unwrap_or_default();
        Self {
            directory: DirectoryBarData::default(),
            folder,
            title,
            crc,
        }
    }

    pub fn folder_data(&self) -> Option<&FolderData> {
        self.folder.as_ref()
    }

    pub fn crc(&self) -> &str {
        &self.crc
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get children bars for this folder.
    /// Queries the song database for songs with parent=crc.
    /// If songs are found, returns SongBar array.
    /// Otherwise, returns sub-folder FolderBars.
    ///
    /// Translates: Java FolderBar.getChildren()
    pub fn children(&self, db: &dyn SongDatabaseAccessor) -> Vec<Bar> {
        log::debug!("[FolderBar] children crc={}", self.crc);
        let songs = db.song_datas("parent", &self.crc);
        log::debug!("[FolderBar] songs found: {}", songs.len());
        if !songs.is_empty() {
            return SongBar::to_song_bar_array(&songs);
        }

        // No songs found - return sub-folders
        // Use "." as bmspath to match the scanner's root (SQLiteSongDatabaseAccessor.root = ".").
        // The scanner computes CRCs with bmspath=".", so folder CRCs here must use the same
        // parameter to ensure consistency when navigating into sub-folders.
        let rootpath = ".".to_string();

        let folders = db.folder_datas("parent", &self.crc);
        log::debug!("[FolderBar] folders found: {}", folders.len());
        let result: Vec<Bar> = folders
            .into_iter()
            .map(|folder| {
                let mut path = folder.path().to_string();
                if path.ends_with(std::path::MAIN_SEPARATOR) {
                    path.pop();
                }
                let ccrc = crate::song::song_utils::crc32(&path, &[], &rootpath);
                log::debug!(
                    "[FolderBar] sub-folder '{}' path='{}' crc={}",
                    folder.title(),
                    path,
                    ccrc
                );
                Bar::Folder(Box::new(FolderBar::new(Some(folder), ccrc)))
            })
            .collect();
        result
    }

    pub fn update_folder_status(&mut self, db: &dyn SongDatabaseAccessor) {
        if let Some(ref folder) = self.folder {
            let mut path = folder.path().to_string();
            if path.ends_with(std::path::MAIN_SEPARATOR) {
                path.pop();
            }
            let rootpath = ".".to_string();
            let ccrc = crate::song::song_utils::crc32(&path, &[], &rootpath);
            let songs = db.song_datas("parent", &ccrc);
            self.directory
                .update_folder_status_with_songs(&songs, None, |_| None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestSongDb;

    #[test]
    fn folder_bar_get_children_returns_song_bars_when_songs_exist() {
        let mut song = SongData::default();
        song.metadata.title = "Test Song".to_string();
        song.file.sha256 = "abc123".to_string();

        let db = TestSongDb::new().with_songs("parent", "test_crc", vec![song]);

        let bar = FolderBar::new(None, "test_crc".to_string());
        let children = bar.children(&db);

        assert_eq!(children.len(), 1);
        assert!(children[0].as_song_bar().is_some());
        assert!(children[0].title().contains("Test Song"));
    }

    #[test]
    fn folder_bar_get_children_returns_folder_bars_when_no_songs() {
        let folder = FolderData {
            title: "Sub Folder".to_string(),
            path: "/test/path".to_string(),
            ..Default::default()
        };

        let db = TestSongDb::new().with_folders("parent", "test_crc", vec![folder]);

        let bar = FolderBar::new(None, "test_crc".to_string());
        let children = bar.children(&db);

        assert_eq!(children.len(), 1);
        assert!(children[0].as_folder_bar().is_some());
        assert_eq!(children[0].title(), "Sub Folder");
    }

    #[test]
    fn folder_bar_get_children_returns_empty_when_no_data() {
        let db = TestSongDb::new();
        let bar = FolderBar::new(None, "nonexistent".to_string());
        let children = bar.children(&db);
        assert!(children.is_empty());
    }

    #[test]
    fn folder_bar_get_children_prefers_songs_over_folders() {
        let mut song = SongData::default();
        song.metadata.title = "Song".to_string();
        song.file.sha256 = "sha1".to_string();

        let folder = FolderData {
            title: "Folder".to_string(),
            path: "/test".to_string(),
            ..Default::default()
        };

        let db = TestSongDb::new()
            .with_songs("parent", "crc1", vec![song])
            .with_folders("parent", "crc1", vec![folder]);

        let bar = FolderBar::new(None, "crc1".to_string());
        let children = bar.children(&db);

        // Should return songs, not folders
        assert_eq!(children.len(), 1);
        assert!(children[0].as_song_bar().is_some());
    }

    /// Verify that FolderBar::children() computes CRCs consistent with the scanner.
    ///
    /// The scanner (SQLiteSongDatabaseAccessor) computes song.parent and folder.parent
    /// using crc32(path, bmsroot, ".") where "." is the literal root string.
    /// FolderBar::children() must use the same bmspath parameter so that navigating
    /// into a folder finds the songs/sub-folders stored by the scanner.
    #[test]
    fn folder_bar_crc_consistent_with_scanner() {
        // Simulate scanner behavior: compute song parent CRC the way the scanner does.
        // Scanner uses bmspath = "." (accessor.root = PathBuf::from(".")).
        let scanner_bmspath = ".";
        let abs_folder_path = "/some/absolute/path/to/bms";

        // Scanner: sd.parent = crc32(grandparent, bmsroot, ".")
        // For a song in /some/absolute/path/to/bms/song_dir/file.bms,
        // grandparent = /some/absolute/path/to/bms
        // Since abs path doesn't start with ".", no prefix stripping occurs.
        let scanner_parent_crc =
            crate::song::song_utils::crc32(abs_folder_path, &[], scanner_bmspath);

        // Now simulate FolderBar browsing: root folder finds folder records,
        // then computes CRC for each folder to create child FolderBars.
        // The folder path from DB is stored with trailing separator.
        let folder_path_from_db = format!("{}/", abs_folder_path);

        // Create a mock DB where the "bms" folder exists at root level
        // and songs exist under the scanner's parent CRC.
        let mut song = SongData::default();
        song.metadata.title = "Test Song".to_string();
        song.file.sha256 = "test_sha256".to_string();
        song.file.set_path("song_dir/file.bms".to_string());

        let folder = FolderData {
            title: "bms".to_string(),
            path: folder_path_from_db,
            ..Default::default()
        };

        let db = TestSongDb::new()
            // Root level: folder with parent "e2977170"
            .with_folders("parent", "e2977170", vec![folder])
            // Songs stored under scanner's parent CRC
            .with_songs("parent", &scanner_parent_crc, vec![song]);

        // Step 1: Navigate from root
        let root_bar = FolderBar::new(None, "e2977170".to_string());
        let root_children = root_bar.children(&db);

        assert_eq!(root_children.len(), 1, "Root should have 1 folder child");
        let child_folder = root_children[0]
            .as_folder_bar()
            .expect("Should be a FolderBar");

        // Step 2: Navigate into the folder - this is the critical test.
        // The CRC computed by children() must match scanner_parent_crc
        // so that songs are found.
        let folder_crc = child_folder.crc();
        assert_eq!(
            folder_crc, scanner_parent_crc,
            "FolderBar CRC must match scanner parent CRC. \
             FolderBar computed '{}' but scanner stored '{}'",
            folder_crc, scanner_parent_crc
        );

        // Step 3: Verify that navigating into the folder actually finds songs
        let songs = child_folder.children(&db);
        assert_eq!(songs.len(), 1, "Should find 1 song in sub-folder");
        assert!(
            songs[0].as_song_bar().is_some(),
            "Child should be a SongBar"
        );
        assert!(
            songs[0].title().contains("Test Song"),
            "Song title should match"
        );
    }

    /// Verify CRC consistency with absolute paths under the CWD.
    /// This test ensures the fix works regardless of the absolute path used.
    #[test]
    fn folder_bar_crc_consistent_with_absolute_paths() {
        // Use a path that looks like a real absolute path
        let abs_paths = [
            "/Users/user/music/bms",
            "/home/user/games/beatoraja/bms",
            "C:\\Users\\user\\bms",
        ];

        for abs_path in &abs_paths {
            let scanner_crc = crate::song::song_utils::crc32(abs_path, &[], ".");

            // Simulate what FolderBar does: trim trailing separator, compute CRC with "."
            let folder_path = format!("{}/", abs_path);
            let mut trimmed = folder_path.clone();
            if trimmed.ends_with('/') || trimmed.ends_with('\\') {
                trimmed.pop();
            }
            let browse_crc = crate::song::song_utils::crc32(&trimmed, &[], ".");

            assert_eq!(
                scanner_crc, browse_crc,
                "CRC mismatch for path '{}': scanner='{}', browser='{}'",
                abs_path, scanner_crc, browse_crc
            );
        }
    }
}

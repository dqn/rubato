use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::hash_bar::HashBar;
use super::table_bar::TableBar;
use crate::stubs::*;

/// Context menu bar for right-click actions
/// Translates: bms.player.beatoraja.select.bar.ContextMenuBar
pub struct ContextMenuBar {
    pub directory: DirectoryBarData,
    pub song: Option<SongData>,
    pub table: Option<usize>,  // index reference to table
    pub folder: Option<usize>, // index reference to folder
    pub show_meta: bool,
    pub title: String,
}

impl ContextMenuBar {
    pub fn new_for_song(song: SongData) -> Self {
        let title = song.get_title().to_string();
        let mut bar = Self {
            directory: DirectoryBarData::new(true),
            song: Some(song),
            table: None,
            folder: None,
            show_meta: false,
            title,
        };
        bar.directory.set_sortable(false);
        bar
    }

    pub fn new_for_table(table_title: String) -> Self {
        let mut bar = Self {
            directory: DirectoryBarData::new(true),
            song: None,
            table: Some(0),
            folder: None,
            show_meta: false,
            title: table_title,
        };
        bar.directory.set_sortable(false);
        bar
    }

    pub fn new_for_table_folder(folder_title: String) -> Self {
        let mut bar = Self {
            directory: DirectoryBarData::new(true),
            song: None,
            table: Some(0),
            folder: Some(0),
            show_meta: false,
            title: folder_title,
        };
        bar.directory.set_sortable(false);
        bar
    }

    pub fn browser_open(url: &str) -> bool {
        Clipboard::set_contents(url);
        // In Java: Desktop.getDesktop().browse(uri)
        // Stub: just log and return
        log::info!("Browser open: {}", url);
        ImGuiNotify::info("Copied URL to clipboard.");
        true
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    pub fn get_lamp(&self, _is_player: bool) -> i32 {
        0
    }

    pub fn get_children(&self) -> Vec<Bar> {
        if self.song.is_some() && self.song.as_ref().unwrap().get_path().is_some() {
            return self.song_context();
        } else if self.song.is_some() && self.song.as_ref().unwrap().get_path().is_none() {
            return self.missing_song_context();
        } else if self.folder.is_some() && self.table.is_some() {
            return self.table_folder_context();
        } else if self.table.is_some() {
            return self.table_context();
        }
        Vec::new()
    }

    fn missing_song_context(&self) -> Vec<Bar> {
        // In Java: creates SongBar, adds leaderboard, meta, tag entries
        log::warn!(
            "not yet implemented: ContextMenuBar.missingSongContext - requires MusicSelector context"
        );
        Vec::new()
    }

    fn song_context(&self) -> Vec<Bar> {
        // In Java: creates play/autoplay/practice bars, leaderboard, related, open folder, etc.
        log::warn!(
            "not yet implemented: ContextMenuBar.songContext - requires MusicSelector context"
        );
        Vec::new()
    }

    fn table_context(&self) -> Vec<Bar> {
        // In Java: creates table context menu entries
        log::warn!(
            "not yet implemented: ContextMenuBar.tableContext - requires MusicSelector context"
        );
        Vec::new()
    }

    fn table_folder_context(&self) -> Vec<Bar> {
        // In Java: creates fill missing charts entry
        log::warn!(
            "not yet implemented: ContextMenuBar.tableFolderContext - requires MusicSelector context"
        );
        Vec::new()
    }
}

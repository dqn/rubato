use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;

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

    pub fn get_children(&self) -> Vec<Bar> {
        // In Java: SongBar.toSongBarArray(selector.getSongDatabase().getSongDatasByText(text))
        log::warn!(
            "not yet implemented: SearchWordBar.getChildren - requires SongDatabaseAccessor context"
        );
        Vec::new()
    }

    pub fn update_folder_status(&mut self) {
        // In Java: updateFolderStatus(selector.getSongDatabase().getSongDatasByText(text))
        log::warn!(
            "not yet implemented: SearchWordBar.updateFolderStatus - requires SongDatabaseAccessor context"
        );
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }
}

use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;

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

    pub fn get_children(&self) -> Vec<Bar> {
        // In Java: SongBar.toSongBarArray(selector.getSongDatabase().getSongDatas("folder", crc))
        log::warn!(
            "not yet implemented: SameFolderBar.getChildren - requires SongDatabaseAccessor context"
        );
        Vec::new()
    }
}

use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::stubs::*;

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

    pub fn get_children(&self) -> Vec<Bar> {
        // In Java: selector.getSongDatabase().getSongDatas("parent", crc)
        // Then if songs > 0 return SongBar array, else return sub-folders
        log::warn!(
            "not yet implemented: FolderBar.getChildren - requires SongDatabaseAccessor context"
        );
        Vec::new()
    }

    pub fn update_folder_status(&mut self) {
        log::warn!(
            "not yet implemented: FolderBar.updateFolderStatus - requires SongDatabaseAccessor context"
        );
    }
}

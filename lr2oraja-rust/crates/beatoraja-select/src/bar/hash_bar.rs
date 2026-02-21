use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::stubs::*;

/// Hash collection folder bar
/// Translates: bms.player.beatoraja.select.bar.HashBar
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
                if !e.get_sha256().is_empty() {
                    e.get_sha256().to_string()
                } else {
                    e.get_md5().to_string()
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

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    pub fn get_elements(&self) -> &[SongData] {
        &self.elements
    }

    pub fn set_elements(&mut self, elements: Vec<SongData>) {
        self.elements_hash = elements
            .iter()
            .map(|e| {
                if !e.get_sha256().is_empty() {
                    e.get_sha256().to_string()
                } else {
                    e.get_md5().to_string()
                }
            })
            .collect();
        self.elements = elements;
    }

    pub fn get_children(&self) -> Vec<Bar> {
        // In Java: SongBar.toSongBarArray(selector.getSongDatabase().getSongDatas(elementsHash), elements)
        log::warn!(
            "not yet implemented: HashBar.getChildren - requires SongDatabaseAccessor context"
        );
        Vec::new()
    }

    pub fn update_folder_status(&mut self) {
        // In Java: updateFolderStatus(selector.getSongDatabase().getSongDatas(elementsHash))
        log::warn!(
            "not yet implemented: HashBar.updateFolderStatus - requires SongDatabaseAccessor context"
        );
    }
}

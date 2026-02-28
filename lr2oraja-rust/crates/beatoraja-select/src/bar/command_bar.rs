use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::stubs::*;

/// SQL command-based directory bar
/// Translates: bms.player.beatoraja.select.bar.CommandBar
#[derive(Clone)]
pub struct CommandBar {
    pub directory: DirectoryBarData,
    /// Bar title
    pub title: String,
    /// SQL query
    pub sql: String,
}

impl CommandBar {
    pub fn new(title: String, sql: String) -> Self {
        Self::new_with_visibility(title, sql, false)
    }

    pub fn new_with_visibility(title: String, sql: String, show_invisible_chart: bool) -> Self {
        Self {
            directory: DirectoryBarData::new(show_invisible_chart),
            title,
            sql,
        }
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    pub fn get_children(&self) -> Vec<Bar> {
        // In Java: main.getSongDatabase().getSongDatas(sql, scoreDb, scoreLogDb, infoDb)
        // Requires MusicSelector reference for DB access
        log::warn!("not yet implemented: CommandBar.getChildren - requires MusicSelector context");
        Vec::new()
    }

    pub fn update_folder_status(&mut self) {
        // In Java: updateFolderStatus(main.getSongDatabase().getSongDatas(...))
        log::warn!(
            "not yet implemented: CommandBar.updateFolderStatus - requires MusicSelector context"
        );
    }
}

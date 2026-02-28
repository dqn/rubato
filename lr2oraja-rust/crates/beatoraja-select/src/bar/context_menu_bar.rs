use std::collections::HashSet;

use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::function_bar::{FunctionBar, STYLE_FOLDER, STYLE_SEARCH, STYLE_SPECIAL, STYLE_TEXT_NEW};
use super::hash_bar::HashBar;
use super::leader_board_bar::LeaderBoardBar;
use super::song_bar::SongBar;
use super::table_bar::TableBar;
use crate::stubs::*;

/// Context menu bar for right-click actions
/// Translates: bms.player.beatoraja.select.bar.ContextMenuBar
#[derive(Clone)]
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
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(e) = clipboard.set_text(url) {
                    log::error!("Failed to copy to clipboard: {}", e);
                }
            }
            Err(e) => {
                log::error!("Failed to access clipboard: {}", e);
            }
        }
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

    /// Returns the parent bar (table).
    /// Corresponds to Java ContextMenuBar.getPrevious()
    pub fn get_previous(&self) -> Option<&TableBar> {
        // In Java: return table (the TableBar reference)
        // We don't store TableBar directly here but could return a reference
        // Stubbed since we store table as Option<usize> index
        log::warn!("not yet implemented: ContextMenuBar.getPrevious - requires TableBar reference");
        None
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

    /// Add leaderboard entries to the context menu.
    /// Corresponds to Java ContextMenuBar.addLeaderboardEntries(ArrayList<Bar>)
    #[allow(clippy::ptr_arg)]
    fn add_leaderboard_entries(&self, options: &mut Vec<Bar>) {
        // In Java: creates FunctionBars for leaderboard and LR2IR leaderboard
        // Requires MusicSelector.main.getIRStatus(), LeaderBoardBar, play(FOLDER_OPEN)
        log::warn!(
            "not yet implemented: ContextMenuBar.addLeaderboardEntries - requires MusicSelector context"
        );
    }

    /// Add metadata copy entries to the context menu.
    /// Corresponds to Java ContextMenuBar.addMetaEntries(ArrayList<Bar>)
    #[allow(clippy::ptr_arg)]
    fn add_meta_entries(&self, options: &mut Vec<Bar>) {
        // In Java: creates FunctionBars for LR2IR page, Chart Viewer, Metadata (Copy Title/MD5/SHA256/Path/URL)
        log::warn!(
            "not yet implemented: ContextMenuBar.addMetaEntries - requires MusicSelector context"
        );
    }

    /// Add table tag display entries to the context menu.
    /// Corresponds to Java ContextMenuBar.addTagDisplayEntries(ArrayList<Bar>)
    #[allow(clippy::ptr_arg)]
    fn add_tag_display_entries(&self, options: &mut Vec<Bar>) {
        // In Java: reverse-looks up song in difficulty tables and creates navigable entries
        log::warn!(
            "not yet implemented: ContextMenuBar.addTagDisplayEntries - requires BarManager.getTables()"
        );
    }

    /// Add a single table entry for tag display.
    /// Corresponds to Java ContextMenuBar.addTableEntry(ArrayList<Bar>, TableBar, HashBar)
    #[allow(clippy::ptr_arg)]
    fn add_table_entry(&self, options: &mut Vec<Bar>, _table: &TableBar, _level: &HashBar) {
        // In Java: creates FunctionBar that navigates to the table/level, with calculated lamps
        log::warn!(
            "not yet implemented: ContextMenuBar.addTableEntry - requires BarManager navigation"
        );
    }

    /// Calculate clear lamp distribution for a set of songs.
    /// Corresponds to Java ContextMenuBar.calculateLamps(MusicSelector, SongData[])
    fn calculate_lamps(
        songs: &[SongData],
        score_fn: impl Fn(&SongData) -> Option<ScoreData>,
        mode: Option<&bms_model::Mode>,
    ) -> Vec<i32> {
        let mut lamps = vec![0i32; 11];
        for song in songs {
            if song.get_path().is_none() {
                continue;
            }
            if let Some(m) = mode
                && song.get_mode() != 0
                && song.get_mode() != m.id()
            {
                continue;
            }
            let score = score_fn(song);
            let lamp_index = if let Some(ref s) = score {
                s.get_clear() as usize
            } else {
                0
            };
            if lamp_index < lamps.len() {
                lamps[lamp_index] += 1;
            }
        }
        lamps
    }

    /// Fill missing charts by submitting download tasks.
    /// Corresponds to Java ContextMenuBar.fillMissingCharts(SongData[], MainController)
    fn fill_missing_charts(want: &[SongData]) -> i32 {
        let md5_and_names: Vec<(String, String)> = want
            .iter()
            .filter_map(|sd| {
                let md5 = sd.get_md5().to_string();
                let title = sd.get_title().to_string();
                if !md5.is_empty() && !title.is_empty() {
                    Some((md5, title))
                } else {
                    None
                }
            })
            .collect();
        if md5_and_names.is_empty() {
            return 0;
        }
        // In Java: queries songdb for existing songs, filters out those already present,
        // submits HTTP download tasks for missing ones
        // Requires MainController.getSongDatabase(), HttpDownloadProcessor
        log::warn!(
            "not yet implemented: ContextMenuBar.fillMissingCharts - requires HttpDownloadProcessor"
        );
        0
    }
}

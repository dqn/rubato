use std::collections::HashSet;
use std::sync::Arc;

use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::function_bar::{
    FunctionBar, FunctionBarCallback, STYLE_COURSE, STYLE_FOLDER, STYLE_MISSING, STYLE_SEARCH,
    STYLE_SPECIAL, STYLE_TABLE, STYLE_TEXT_MISSING, STYLE_TEXT_NEW, STYLE_TEXT_PLAIN,
};
use super::hash_bar::HashBar;
use super::leader_board_bar::LeaderBoardBar;
use super::same_folder_bar::SameFolderBar;
use super::song_bar::SongBar;
use super::table_bar::TableBar;
use rubato_core::main_state::MainState;
use rubato_types::http_download_submitter::HttpDownloadSubmitter;
use rubato_types::song_database_accessor::SongDatabaseAccessor;

use crate::select::stubs::*;

/// Context menu bar for right-click actions
/// Translates: bms.player.beatoraja.select.bar.ContextMenuBar
#[derive(Clone)]
pub struct ContextMenuBar {
    pub directory: DirectoryBarData,
    pub song: Option<SongData>,
    pub table: Option<TableBar>,
    pub folder: Option<HashBar>,
    pub show_meta: bool,
    pub title: String,
}

impl ContextMenuBar {
    pub fn new_for_song(song: SongData) -> Self {
        let title = song.title.clone();
        let mut bar = Self {
            directory: DirectoryBarData::new(true),
            song: Some(song),
            table: None,
            folder: None,
            show_meta: false,
            title,
        };
        bar.directory.sortable = false;
        bar
    }

    pub fn new_for_table(table: TableBar) -> Self {
        let title = table.title().to_owned();
        let mut bar = Self {
            directory: DirectoryBarData::new(true),
            song: None,
            table: Some(table),
            folder: None,
            show_meta: false,
            title,
        };
        bar.directory.sortable = false;
        bar
    }

    pub fn new_for_table_folder(table: TableBar, folder: HashBar) -> Self {
        let title = folder.title().to_owned();
        let mut bar = Self {
            directory: DirectoryBarData::new(true),
            song: None,
            table: Some(table),
            folder: Some(folder),
            show_meta: false,
            title,
        };
        bar.directory.sortable = false;
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
        log::info!("Browser open: {}", url);
        ImGuiNotify::info("Copied URL to clipboard.");
        true
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn lamp(&self, _is_player: bool) -> i32 {
        0
    }

    /// Returns the parent bar (table).
    /// Corresponds to Java ContextMenuBar.getPrevious()
    pub fn previous(&self) -> Option<&TableBar> {
        self.table.as_ref()
    }

    pub fn children(&self, tables: &[TableBar], songdb: &dyn SongDatabaseAccessor) -> Vec<Bar> {
        if let Some(ref song) = self.song {
            if song.path().is_some() {
                return self.song_context(tables, songdb);
            } else {
                return self.missing_song_context(tables);
            }
        } else if self.folder.is_some() && self.table.is_some() {
            return self.table_folder_context();
        } else if self.table.is_some() {
            return self.table_context();
        }
        Vec::new()
    }

    fn missing_song_context(&self, tables: &[TableBar]) -> Vec<Bar> {
        let mut options = Vec::new();
        let song = match &self.song {
            Some(s) => s,
            None => return options,
        };

        // Song entry
        options.push(Bar::Song(Box::new(SongBar::new(song.clone()))));

        // Leaderboard entries
        self.add_leaderboard_entries(&mut options);

        // Meta entries (show by default for missing songs)
        self.add_meta_entries(&mut options, true);

        // Tag display entries
        self.add_tag_display_entries(&mut options, tables);

        options
    }

    fn song_context(&self, tables: &[TableBar], songdb: &dyn SongDatabaseAccessor) -> Vec<Bar> {
        let mut options = Vec::new();
        let song = match &self.song {
            Some(s) => s,
            None => return options,
        };

        // Song entry (play)
        options.push(Bar::Song(Box::new(SongBar::new(song.clone()))));

        // Autoplay
        let mut autoplay = FunctionBar::new("Autoplay".to_string(), STYLE_TABLE);
        autoplay.set_function(Arc::new(|selector| {
            selector.select_song(BMSPlayerMode::AUTOPLAY);
        }));
        options.push(Bar::Function(Box::new(autoplay)));

        // Practice
        let mut practice = FunctionBar::new("Practice".to_string(), STYLE_TABLE);
        practice.set_function(Arc::new(|selector| {
            selector.select_song(BMSPlayerMode::PRACTICE);
        }));
        options.push(Bar::Function(Box::new(practice)));

        // Leaderboard
        self.add_leaderboard_entries(&mut options);

        // Related — navigate to SameFolderBar showing same-folder songs
        {
            let song_title = song.full_title();
            let song_folder = song.folder.clone();
            let mut related = FunctionBar::new("Related".to_string(), STYLE_TABLE);
            let title_clone = song_title.clone();
            let folder_clone = song_folder.clone();
            related.set_function(Arc::new(move |selector| {
                let same = SameFolderBar::new(title_clone.clone(), folder_clone.clone());
                let bar = Bar::SameFolder(Box::new(same));
                selector.update_bar_with_songdb_context(Some(&bar));
                selector.play_sound(SoundType::FolderOpen);
            }));
            let folder_songs = songdb.song_datas("folder", &song_folder);
            let lamps = Self::calculate_lamps(&folder_songs, |_| None, None);
            related.lamps = lamps;
            options.push(Bar::Function(Box::new(related)));
        }

        // Open Song Folder
        let mut open_folder = FunctionBar::new("Open Song Folder".to_string(), STYLE_FOLDER);
        {
            let song_path = song.path().map(|p| p.to_string());
            open_folder.set_function(Arc::new(move |_selector| {
                if let Some(ref path) = song_path
                    && let Some(parent) = std::path::Path::new(path).parent()
                    && let Err(e) = open::that(parent)
                {
                    log::error!("Failed to open folder: {}", e);
                }
            }));
        }
        options.push(Bar::Function(Box::new(open_folder)));

        // Open URL
        {
            let url = song.url();
            if !url.is_empty() {
                let url_owned = url.to_string();
                let mut open_url = FunctionBar::new("Open URL".to_string(), STYLE_FOLDER);
                open_url.set_function(Arc::new(move |_selector| {
                    ContextMenuBar::browser_open(&url_owned);
                }));
                options.push(Bar::Function(Box::new(open_url)));
            }
        }

        // Open Append URL
        {
            let append_url = song.appendurl();
            let main_url = song.url();
            if !append_url.is_empty() && append_url != main_url {
                let append_url_owned = append_url.to_string();
                let mut open_append = FunctionBar::new("Open Append URL".to_string(), STYLE_FOLDER);
                open_append.set_function(Arc::new(move |_selector| {
                    ContextMenuBar::browser_open(&append_url_owned);
                }));
                options.push(Bar::Function(Box::new(open_append)));
            }
        }

        // Meta entries
        self.add_meta_entries(&mut options, self.show_meta);

        // Favorite Chart
        let is_fav_chart = (song.favorite & FAVORITE_CHART) != 0;
        let mut fav_chart = FunctionBar::new_with_text_type(
            "Favorite Chart".to_string(),
            if is_fav_chart {
                STYLE_COURSE
            } else {
                STYLE_MISSING
            },
            if is_fav_chart {
                STYLE_TEXT_PLAIN
            } else {
                STYLE_TEXT_MISSING
            },
        );
        fav_chart.set_song_data(song.clone());
        {
            let song_for_fav = song.clone();
            fav_chart.set_function(Arc::new(move |selector| {
                let mut sd = song_for_fav.clone();
                let new_fav = sd.favorite ^ FAVORITE_CHART;
                sd.favorite = new_fav;
                selector.songdb.set_song_datas(&[sd]);
            }));
        }
        options.push(Bar::Function(Box::new(fav_chart)));

        // Favorite Song
        let is_fav_song = (song.favorite & FAVORITE_SONG) != 0;
        let mut fav_song = FunctionBar::new_with_text_type(
            "Favorite Song".to_string(),
            if is_fav_song {
                STYLE_COURSE
            } else {
                STYLE_MISSING
            },
            if is_fav_song {
                STYLE_TEXT_PLAIN
            } else {
                STYLE_TEXT_MISSING
            },
        );
        fav_song.set_song_data(song.clone());
        {
            let song_for_fav = song.clone();
            fav_song.set_function(Arc::new(move |selector| {
                let mut sd = song_for_fav.clone();
                let new_fav = sd.favorite ^ FAVORITE_SONG;
                sd.favorite = new_fav;
                selector.songdb.set_song_datas(&[sd]);
            }));
        }
        options.push(Bar::Function(Box::new(fav_song)));

        // Tag display entries
        self.add_tag_display_entries(&mut options, tables);

        options
    }

    fn table_context(&self) -> Vec<Bar> {
        let mut options = Vec::new();

        // Table title entry
        let title_bar = FunctionBar::new(self.title.clone(), STYLE_TABLE);
        options.push(Bar::Function(Box::new(title_bar)));

        // Open URL
        if let Some(ref table) = self.table
            && let Some(url) = table.url()
        {
            let url_owned = url.to_string();
            let mut open_url = FunctionBar::new("Open URL".to_string(), STYLE_FOLDER);
            open_url.set_function(Arc::new(move |_selector| {
                ContextMenuBar::browser_open(&url_owned);
            }));
            options.push(Bar::Function(Box::new(open_url)));
        }

        // Copy Table Name
        {
            let name = self.title.clone();
            let mut copy_name = FunctionBar::new_with_text_type(
                "Copy Table Name".to_string(),
                STYLE_SEARCH,
                STYLE_TEXT_NEW,
            );
            copy_name.set_function(clipboard_copy_callback(
                &name,
                "Copied table name to clipboard.",
            ));
            options.push(Bar::Function(Box::new(copy_name)));
        }

        // Copy URL
        if let Some(ref table) = self.table
            && let Some(url) = table.url()
        {
            let mut copy_url = FunctionBar::new_with_text_type(
                "Copy URL".to_string(),
                STYLE_SEARCH,
                STYLE_TEXT_NEW,
            );
            copy_url.set_function(clipboard_copy_callback(
                url,
                "Copied table URL to clipboard.",
            ));
            options.push(Bar::Function(Box::new(copy_url)));
        }

        // Fill Missing Charts — flatten all table folders and submit missing songs
        if let Some(ref table) = self.table {
            let table_clone = table.clone();
            let mut fill_missing = FunctionBar::new_with_text_type(
                "Fill Missing Charts".to_string(),
                STYLE_SPECIAL,
                STYLE_TEXT_NEW,
            );
            fill_missing.set_function(Arc::new(move |selector| {
                let folders = &table_clone.table_data().folder;
                let want: Vec<SongData> = folders
                    .iter()
                    .flat_map(|f| f.songs.iter().cloned())
                    .collect();
                if let Some(downloader) = selector.main.as_ref().and_then(|m| m.http_downloader()) {
                    let fill_count =
                        ContextMenuBar::fill_missing_charts(&want, &*selector.songdb, downloader);
                    if fill_count == 0 {
                        log::info!("Nothing to fill");
                    }
                }
            }));
            options.push(Bar::Function(Box::new(fill_missing)));
        }

        options
    }

    fn table_folder_context(&self) -> Vec<Bar> {
        let mut options = Vec::new();

        // Folder title entry
        let folder_bar = FunctionBar::new(self.title.clone(), STYLE_TABLE);
        options.push(Bar::Function(Box::new(folder_bar)));

        // Fill Missing Charts — submit download tasks for songs in this folder
        if let Some(ref folder) = self.folder {
            let elements: Vec<SongData> = folder.elements().to_vec();
            let mut fill_missing = FunctionBar::new_with_text_type(
                "Fill Missing Charts".to_string(),
                STYLE_SPECIAL,
                STYLE_TEXT_NEW,
            );
            fill_missing.set_function(Arc::new(move |selector| {
                if let Some(downloader) = selector.main.as_ref().and_then(|m| m.http_downloader()) {
                    let fill_count = ContextMenuBar::fill_missing_charts(
                        &elements,
                        &*selector.songdb,
                        downloader,
                    );
                    if fill_count == 0 {
                        log::info!("Nothing to fill");
                    }
                }
            }));
            options.push(Bar::Function(Box::new(fill_missing)));
        }

        options
    }

    /// Add leaderboard entries to the context menu.
    /// Corresponds to Java ContextMenuBar.addLeaderboardEntries(ArrayList<Bar>)
    fn add_leaderboard_entries(&self, options: &mut Vec<Bar>) {
        let song = match &self.song {
            Some(s) => s,
            None => return,
        };

        // Leaderboard (IR) — in Java, only shown when IR connections exist
        {
            let song_clone = song.clone();
            let mut leaderboard = FunctionBar::new("Leaderboard".to_string(), STYLE_SPECIAL);
            leaderboard.set_function(Arc::new(move |selector| {
                let lb = LeaderBoardBar::new(song_clone.clone(), false);
                let bar = Bar::LeaderBoard(Box::new(lb));
                selector.update_bar_with_songdb_context(Some(&bar));
                selector.play_sound(SoundType::FolderOpen);
            }));
            options.push(Bar::Function(Box::new(leaderboard)));
        }

        // LR2IR Leaderboard (always shown)
        {
            let song_clone = song.clone();
            let mut lr2ir = FunctionBar::new("LR2IR Leaderboard".to_string(), STYLE_SPECIAL);
            lr2ir.set_function(Arc::new(move |selector| {
                let lb = LeaderBoardBar::new(song_clone.clone(), true);
                let bar = Bar::LeaderBoard(Box::new(lb));
                selector.update_bar_with_songdb_context(Some(&bar));
                selector.play_sound(SoundType::FolderOpen);
            }));
            options.push(Bar::Function(Box::new(lr2ir)));
        }
    }

    /// Add metadata copy entries to the context menu.
    /// Corresponds to Java ContextMenuBar.addMetaEntries(ArrayList<Bar>)
    fn add_meta_entries(&self, options: &mut Vec<Bar>, show_meta: bool) {
        let song = match &self.song {
            Some(s) => s,
            None => return,
        };

        let md5 = &song.md5;

        // Open LR2IR page
        if !md5.is_empty() {
            let url = format!(
                "http://www.dream-pro.info/~lavalse/LR2IR/search.cgi?mode=ranking&bmsmd5={}",
                md5
            );
            let mut lr2ir_page = FunctionBar::new("Open LR2IR page".to_string(), STYLE_FOLDER);
            lr2ir_page.set_function(Arc::new(move |_selector| {
                ContextMenuBar::browser_open(&url);
            }));
            options.push(Bar::Function(Box::new(lr2ir_page)));
        }

        // Open Chart Viewer
        if !md5.is_empty() {
            let url = format!("https://bms-score-viewer.pages.dev/view?md5={}", md5);
            let mut chart_viewer = FunctionBar::new("Open Chart Viewer".to_string(), STYLE_FOLDER);
            chart_viewer.set_function(Arc::new(move |_selector| {
                ContextMenuBar::browser_open(&url);
            }));
            options.push(Bar::Function(Box::new(chart_viewer)));
        }

        // Metadata toggle — creates a new ContextMenuBar with toggled show_meta
        let meta_style = if show_meta { STYLE_TABLE } else { STYLE_SEARCH };
        let mut metadata = FunctionBar::new("Metadata".to_string(), meta_style);
        {
            let song_clone = song.clone();
            let new_show_meta = !show_meta;
            metadata.set_function(Arc::new(move |selector| {
                let mut new_menu = ContextMenuBar::new_for_song(song_clone.clone());
                new_menu.show_meta = new_show_meta;
                let bar = Bar::ContextMenu(Box::new(new_menu));
                selector.update_bar_with_songdb_context(Some(&bar));
                selector.play_sound(SoundType::OptionChange);
            }));
        }
        options.push(Bar::Function(Box::new(metadata)));

        if show_meta {
            // Copy Title
            let title = song.title.clone();
            if !title.is_empty() {
                let mut copy_title = FunctionBar::new_with_text_type(
                    "Copy Title".to_string(),
                    STYLE_SEARCH,
                    STYLE_TEXT_NEW,
                );
                copy_title.set_subtitle(title.clone());
                copy_title.set_function(clipboard_copy_callback(
                    &title,
                    "Copied song title to clipboard.",
                ));
                options.push(Bar::Function(Box::new(copy_title)));
            }

            // Copy MD5
            let md5_str = song.md5.clone();
            if !md5_str.is_empty() {
                let mut copy_md5 = FunctionBar::new_with_text_type(
                    "Copy MD5".to_string(),
                    STYLE_SEARCH,
                    STYLE_TEXT_NEW,
                );
                copy_md5.set_subtitle(md5_str.clone());
                copy_md5.set_function(clipboard_copy_callback(
                    &md5_str,
                    "Copied MD5 to clipboard.",
                ));
                options.push(Bar::Function(Box::new(copy_md5)));
            }

            // Copy SHA256
            let sha256 = song.sha256.clone();
            if !sha256.is_empty() {
                let mut copy_sha256 = FunctionBar::new_with_text_type(
                    "Copy SHA256".to_string(),
                    STYLE_SEARCH,
                    STYLE_TEXT_NEW,
                );
                copy_sha256.set_subtitle(sha256.clone());
                copy_sha256.set_function(clipboard_copy_callback(
                    &sha256,
                    "Copied SHA256 to clipboard.",
                ));
                options.push(Bar::Function(Box::new(copy_sha256)));
            }

            // Copy Path
            if let Some(path) = song.path() {
                let path_str = path.to_string();
                let mut copy_path = FunctionBar::new_with_text_type(
                    "Copy Path".to_string(),
                    STYLE_SEARCH,
                    STYLE_TEXT_NEW,
                );
                copy_path.set_subtitle(path_str.clone());
                copy_path.set_function(clipboard_copy_callback(
                    &path_str,
                    "Copied song path to clipboard.",
                ));
                options.push(Bar::Function(Box::new(copy_path)));
            }

            // Copy URL
            {
                let url = song.url();
                if !url.is_empty() {
                    let url_str = url.to_string();
                    let mut copy_url = FunctionBar::new_with_text_type(
                        "Copy URL".to_string(),
                        STYLE_SEARCH,
                        STYLE_TEXT_NEW,
                    );
                    copy_url.set_subtitle(url_str.clone());
                    copy_url.set_function(clipboard_copy_callback(
                        &url_str,
                        "Copied URL to clipboard.",
                    ));
                    options.push(Bar::Function(Box::new(copy_url)));
                }
            }

            // Copy Append URL
            {
                let append_url = song.appendurl();
                let main_url = song.url();
                if !append_url.is_empty() && append_url != main_url {
                    let append_str = append_url.to_string();
                    let mut copy_append = FunctionBar::new_with_text_type(
                        "Copy Append URL".to_string(),
                        STYLE_SEARCH,
                        STYLE_TEXT_NEW,
                    );
                    copy_append.set_subtitle(append_str.clone());
                    copy_append.set_function(clipboard_copy_callback(
                        &append_str,
                        "Copied append URL to clipboard.",
                    ));
                    options.push(Bar::Function(Box::new(copy_append)));
                }
            }
        }
    }

    /// Add table tag display entries to the context menu.
    /// Reverse-looks up song in difficulty tables and creates navigable entries.
    /// Corresponds to Java ContextMenuBar.addTagDisplayEntries(ArrayList<Bar>)
    fn add_tag_display_entries(&self, options: &mut Vec<Bar>, tables: &[TableBar]) {
        let song = match &self.song {
            Some(s) => s,
            None => return,
        };
        let md5 = &song.md5;
        let sha256 = &song.sha256;
        if md5.is_empty() && sha256.is_empty() {
            return;
        }

        for table in tables {
            for level in table.levels() {
                let mut found = false;
                for table_song in level.elements() {
                    let song_md5 = &table_song.md5;
                    let song_sha256 = &table_song.sha256;
                    if (!md5.is_empty() && !song_md5.is_empty() && md5 == song_md5)
                        || (!sha256.is_empty() && !song_sha256.is_empty() && sha256 == song_sha256)
                    {
                        found = true;
                        break;
                    }
                }
                if found {
                    self.add_table_entry(options, table, level);
                }
            }
        }
    }

    /// Add a single table entry for tag display.
    /// Corresponds to Java ContextMenuBar.addTableEntry(ArrayList<Bar>, TableBar, HashBar)
    fn add_table_entry(&self, options: &mut Vec<Bar>, table: &TableBar, level: &HashBar) {
        let entry = format!("{} {}", level.title(), table.title());
        let mut show_tables = FunctionBar::new(entry, STYLE_SEARCH);
        let table_bar = Bar::Table(Box::new(table.clone()));
        let level_bar = Bar::Hash(Box::new(level.clone()));
        let song = self.song.clone();
        show_tables.set_function(Arc::new(move |selector| {
            // Navigate: root → table → level → select song
            selector.update_bar_with_songdb_context(None);
            selector.manager.set_selected(&table_bar);
            selector.update_bar_with_songdb_context(Some(&table_bar));
            selector.manager.set_selected(&level_bar);
            selector.update_bar_with_songdb_context(Some(&level_bar));
            if let Some(ref s) = song {
                let song_bar = Bar::Song(Box::new(SongBar::new(s.clone())));
                selector.manager.set_selected(&song_bar);
            }
            selector.play_sound(SoundType::FolderOpen);
        }));
        let lamps = Self::calculate_lamps(level.elements(), |_| None, None);
        show_tables.lamps = lamps;
        options.push(Bar::Function(Box::new(show_tables)));
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
            if song.path().is_none() {
                continue;
            }
            if let Some(m) = mode
                && song.mode != 0
                && song.mode != m.id()
            {
                continue;
            }
            let score = score_fn(song);
            let lamp_index = if let Some(ref s) = score {
                s.clear as usize
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
    fn fill_missing_charts(
        want: &[SongData],
        songdb: &dyn SongDatabaseAccessor,
        downloader: &dyn HttpDownloadSubmitter,
    ) -> i32 {
        let md5_and_names: Vec<(String, String)> = want
            .iter()
            .filter_map(|sd| {
                let md5 = sd.md5.clone();
                let title = sd.title.clone();
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
        let md5_array: Vec<String> = md5_and_names.iter().map(|(md5, _)| md5.clone()).collect();
        let in_hand = songdb.song_datas_by_hashes(&md5_array);
        let in_hand_md5s: HashSet<String> = in_hand.iter().map(|sd| sd.md5.clone()).collect();
        let missing: Vec<&(String, String)> = md5_and_names
            .iter()
            .filter(|(md5, _)| !in_hand_md5s.contains(md5))
            .collect();
        for (md5, title) in &missing {
            downloader.submit_md5_task(md5, title);
        }
        missing.len() as i32
    }
}

/// Create a clipboard copy callback that copies the given text and shows a notification.
fn clipboard_copy_callback(text: &str, message: &str) -> FunctionBarCallback {
    let text = text.to_string();
    let message = message.to_string();
    Arc::new(move |_selector| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.clone());
            ImGuiNotify::info(&message);
        }
    })
}

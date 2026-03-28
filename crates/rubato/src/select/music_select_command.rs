use std::collections::HashSet;

use crate::core::main_state::MainState;

use super::bar::bar::Bar;
use super::bar::container_bar::ContainerBar;
use super::bar::context_menu_bar::ContextMenuBar;
use super::bar::same_folder_bar::SameFolderBar;
use super::bar::song_bar::SongBar;
use super::music_selector::{MusicSelector, REPLAY};
use super::*;

/// Music select commands
/// Translates: bms.player.beatoraja.select.MusicSelectCommand
///
/// In Java, each enum variant holds a Consumer<MusicSelector>.
/// In Rust, we use an enum and dispatch via a method on MusicSelector
/// (since the commands need MusicSelector context to execute).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicSelectCommand {
    ResetReplay,
    NextReplay,
    PrevReplay,
    CopyMd5Hash,
    CopySha256Hash,
    DownloadIpfs,
    DownloadHttp,
    DownloadCourseHttp,
    ShowSongsOnSameFolder,
    ShowContextMenu,
    CopyHighlightedMenuText,
}

impl MusicSelectCommand {
    /// Execute this command on the given MusicSelector.
    /// Corresponds to Java MusicSelectCommand.function.accept(selector)
    pub fn execute(self, selector: &mut MusicSelector) {
        match self {
            MusicSelectCommand::ResetReplay => {
                // In Java: finds first existing replay for selected selectable bar
                if let Some(selected) = selector.manager.selected()
                    && let Some(selectable) = selected.as_selectable_bar()
                {
                    for i in 0..REPLAY {
                        if selectable.exists_replay(i as i32) {
                            selector.selectedreplay = i as i32;
                            return;
                        }
                    }
                }
                selector.selectedreplay = -1;
            }
            MusicSelectCommand::NextReplay => {
                if let Some(selected) = selector.manager.selected()
                    && let Some(selectable) = selected.as_selectable_bar()
                {
                    let current = selector.selectedreplay;
                    for i in 1..REPLAY as i32 {
                        let index = (i + current) % REPLAY as i32;
                        if selectable.exists_replay(index) {
                            selector.selectedreplay = index;
                            selector.play_sound(SoundType::OptionChange);
                            break;
                        }
                    }
                }
            }
            MusicSelectCommand::PrevReplay => {
                if let Some(selected) = selector.manager.selected()
                    && let Some(selectable) = selected.as_selectable_bar()
                {
                    let current = selector.selectedreplay;
                    for i in 1..REPLAY as i32 {
                        let index = (current + REPLAY as i32 - i) % REPLAY as i32;
                        if selectable.exists_replay(index) {
                            selector.selectedreplay = index;
                            selector.play_sound(SoundType::OptionChange);
                            break;
                        }
                    }
                }
            }
            MusicSelectCommand::CopyMd5Hash => {
                if let Some(selected) = selector.manager.selected()
                    && let Some(song_bar) = selected.as_song_bar()
                {
                    let hash = &song_bar.song_data().file.md5;
                    if !hash.is_empty()
                        && let Ok(mut clipboard) = arboard::Clipboard::new()
                    {
                        let _ = clipboard.set_text(hash.to_string());
                        ImGuiNotify::info(&format!("MD5 hash copied: {}", hash));
                    }
                }
            }
            MusicSelectCommand::CopySha256Hash => {
                if let Some(selected) = selector.manager.selected()
                    && let Some(song_bar) = selected.as_song_bar()
                {
                    let hash = &song_bar.song_data().file.sha256;
                    if !hash.is_empty()
                        && let Ok(mut clipboard) = arboard::Clipboard::new()
                    {
                        let _ = clipboard.set_text(hash.to_string());
                        ImGuiNotify::info(&format!("SHA256 hash copied: {}", hash));
                    }
                }
            }
            MusicSelectCommand::DownloadIpfs => {
                // Check if we're inside a TableBar directory with a URL
                let has_table_url = selector
                    .manager
                    .dir
                    .iter()
                    .any(|d| d.as_table_bar().is_some_and(|t| t.url().is_some()));
                if has_table_url {
                    if let Some(selected) = selector.manager.selected()
                        && let Some(song_bar) = selected.as_song_bar()
                    {
                        let song = song_bar.song_data();
                        if !song.ipfs_str().is_empty() && selector.ipfs_download_alive {
                            selector.pending_start_ipfs.push(song.clone());
                            return;
                        }
                    }
                    log::info!("Download was not started.");
                }
            }
            MusicSelectCommand::DownloadHttp => {
                if let Some(selected) = selector.manager.selected()
                    && let Some(song_bar) = selected.as_song_bar()
                {
                    let song = song_bar.song_data();
                    let md5 = &song.file.md5;
                    if !md5.is_empty() {
                        log::info!("Missing song md5: {}", md5);
                        if let Some(ref downloader) = selector.http_downloader {
                            downloader.submit_md5_task(md5, &song.metadata.title);
                        }
                    } else {
                        log::info!("Not a valid song bar? Skipped...");
                    }
                }
            }
            MusicSelectCommand::DownloadCourseHttp => {
                if let Some(selected) = selector.manager.selected()
                    && let Some(grade_bar) = selected.as_grade_bar()
                    && let Some(ref downloader) = selector.http_downloader
                {
                    for song in grade_bar.song_datas() {
                        let md5 = &song.file.md5;
                        if !md5.is_empty() {
                            log::info!("Missing song md5: {}", md5);
                            downloader.submit_md5_task(md5, &song.metadata.title);
                        }
                    }
                }
            }
            MusicSelectCommand::ShowSongsOnSameFolder => {
                let selected = selector.manager.selected().cloned();
                if let Some(ref current) = selected {
                    if let Some(song_bar) = current.as_song_bar() {
                        if song_bar.exists_song() {
                            // Check not already in a SameFolderBar directory
                            let already_in_same_folder = selector
                                .manager
                                .dir
                                .last()
                                .is_some_and(|b| matches!(**b, Bar::SameFolder(_)));
                            if !already_in_same_folder {
                                let sd = song_bar.song_data();
                                let same =
                                    SameFolderBar::new(sd.metadata.full_title(), sd.folder.clone());
                                let bar = Bar::SameFolder(Box::new(same));
                                selector.update_bar_with_songdb_context(Some(&bar));
                                selector.play_sound(SoundType::FolderOpen);
                            }
                        }
                    } else if let Some(grade_bar) = current.as_grade_bar() {
                        // Show course songs in a ContainerBar (deduplicated)
                        let mut seen = HashSet::new();
                        let songbars: Vec<Bar> = grade_bar
                            .song_datas()
                            .iter()
                            .filter(|sd| seen.insert(sd.file.sha256.clone()))
                            .map(|sd| Bar::Song(Box::new(SongBar::new(sd.clone()))))
                            .collect();
                        let container = ContainerBar::new(current.title().to_owned(), songbars);
                        let bar = Bar::Container(Box::new(container));
                        selector.update_bar_with_songdb_context(Some(&bar));
                        selector.play_sound(SoundType::FolderOpen);
                    }
                }
            }
            MusicSelectCommand::ShowContextMenu => {
                // In Java: opens ContextMenuBar for song/table/hash bars
                let selected = selector.manager.selected().cloned();
                let previous = selector.manager.dir.last().map(|b| (**b).clone());
                let already_in_context_menu = previous
                    .as_ref()
                    .is_some_and(|b| b.as_context_menu_bar().is_some());

                if let Some(ref current) = selected {
                    if let Some(song_bar) = current.as_song_bar() {
                        if !already_in_context_menu {
                            let menu = ContextMenuBar::new_for_song(song_bar.song_data().clone());
                            let bar = Bar::ContextMenu(Box::new(menu));
                            selector.update_bar_with_songdb_context(Some(&bar));
                            selector.play_sound(SoundType::FolderOpen);
                        } else {
                            selector.select_song(BMSPlayerMode::PLAY);
                        }
                    } else if let Some(table_bar) = current.as_table_bar() {
                        if !already_in_context_menu {
                            let menu = ContextMenuBar::new_for_table(table_bar.clone());
                            let bar = Bar::ContextMenu(Box::new(menu));
                            selector.update_bar_with_songdb_context(Some(&bar));
                            selector.play_sound(SoundType::FolderOpen);
                        } else if selector.update_bar_with_songdb_context(Some(current)) {
                            selector.play_sound(SoundType::FolderOpen);
                        }
                    } else if let Some(hash_bar) = current.as_hash_bar()
                        && let Some(prev_table) = previous.as_ref().and_then(|p| p.as_table_bar())
                    {
                        let enable_http = selector.app_config.network.enable_http;
                        if !already_in_context_menu && enable_http {
                            let menu = ContextMenuBar::new_for_table_folder(
                                prev_table.clone(),
                                hash_bar.clone(),
                            );
                            let bar = Bar::ContextMenu(Box::new(menu));
                            selector.update_bar_with_songdb_context(Some(&bar));
                            selector.play_sound(SoundType::FolderOpen);
                        } else if selector.update_bar_with_songdb_context(Some(current)) {
                            selector.play_sound(SoundType::FolderOpen);
                        }
                    }
                }
            }
            MusicSelectCommand::CopyHighlightedMenuText => {
                if let Some(selected) = selector.manager.selected() {
                    let content = selected.title();
                    if !content.is_empty()
                        && let Ok(mut clipboard) = arboard::Clipboard::new()
                    {
                        let _ = clipboard.set_text(content);
                        ImGuiNotify::info(&format!("Copied highlighted menu text: {}", content));
                    }
                }
            }
        }
    }
}

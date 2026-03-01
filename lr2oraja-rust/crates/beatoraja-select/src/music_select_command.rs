use beatoraja_core::main_state::MainState;

use crate::bar::bar::Bar;
use crate::bar::context_menu_bar::ContextMenuBar;
use crate::music_selector::{MusicSelector, REPLAY};
use crate::stubs::*;

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
                if let Some(selected) = selector.manager.get_selected()
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
                if let Some(selected) = selector.manager.get_selected()
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
                if let Some(selected) = selector.manager.get_selected()
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
                if let Some(selected) = selector.manager.get_selected()
                    && let Some(song_bar) = selected.as_song_bar()
                {
                    let hash = song_bar.get_song_data().get_md5();
                    if !hash.is_empty()
                        && let Ok(mut clipboard) = arboard::Clipboard::new()
                    {
                        let _ = clipboard.set_text(hash.to_string());
                        ImGuiNotify::info(&format!("MD5 hash copied: {}", hash));
                    }
                }
            }
            MusicSelectCommand::CopySha256Hash => {
                if let Some(selected) = selector.manager.get_selected()
                    && let Some(song_bar) = selected.as_song_bar()
                {
                    let hash = song_bar.get_song_data().get_sha256();
                    if !hash.is_empty()
                        && let Ok(mut clipboard) = arboard::Clipboard::new()
                    {
                        let _ = clipboard.set_text(hash.to_string());
                        ImGuiNotify::info(&format!("SHA256 hash copied: {}", hash));
                    }
                }
            }
            MusicSelectCommand::DownloadIpfs => {
                // In Java: checks directory for TableBar, starts IPFS download
                // Blocked on MainController.getMusicDownloadProcessor()
                log::warn!(
                    "stub: DOWNLOAD_IPFS — blocked by MainController.getMusicDownloadProcessor()"
                );
            }
            MusicSelectCommand::DownloadHttp => {
                // In Java: submits HTTP download task for missing song
                // Blocked on MainController.getHttpDownloadProcessor()
                log::warn!(
                    "stub: DOWNLOAD_HTTP — blocked by MainController.getHttpDownloadProcessor()"
                );
            }
            MusicSelectCommand::DownloadCourseHttp => {
                // In Java: submits HTTP download tasks for missing course songs
                // Blocked on MainController.getHttpDownloadProcessor()
                log::warn!(
                    "stub: DOWNLOAD_COURSE_HTTP — blocked by MainController.getHttpDownloadProcessor()"
                );
            }
            MusicSelectCommand::ShowSongsOnSameFolder => {
                // In Java: opens SameFolderBar or ContainerBar for course songs
                // Blocked on SameFolderBar(selector, ...) constructor needing MusicSelector
                log::warn!(
                    "stub: SHOW_SONGS_ON_SAME_FOLDER — blocked by SameFolderBar/ContainerBar integration"
                );
            }
            MusicSelectCommand::ShowContextMenu => {
                // In Java: opens ContextMenuBar for song/table/hash bars
                let selected = selector.manager.get_selected().cloned();
                let previous = selector.manager.dir.last().map(|b| (**b).clone());
                let already_in_context_menu = previous
                    .as_ref()
                    .is_some_and(|b| b.as_context_menu_bar().is_some());

                if let Some(ref current) = selected {
                    if let Some(song_bar) = current.as_song_bar() {
                        if !already_in_context_menu {
                            let menu =
                                ContextMenuBar::new_for_song(song_bar.get_song_data().clone());
                            let bar = Bar::ContextMenu(Box::new(menu));
                            selector.manager.update_bar(Some(&bar));
                            selector.play_sound(SoundType::FolderOpen);
                        } else {
                            selector.select_song(BMSPlayerMode::PLAY);
                        }
                    } else if current.as_table_bar().is_some() {
                        if !already_in_context_menu {
                            let title = current.get_title();
                            let menu = ContextMenuBar::new_for_table(title);
                            let bar = Bar::ContextMenu(Box::new(menu));
                            selector.manager.update_bar(Some(&bar));
                            selector.play_sound(SoundType::FolderOpen);
                        } else if selector.manager.update_bar(Some(current)) {
                            selector.play_sound(SoundType::FolderOpen);
                        }
                    } else if current.as_hash_bar().is_some()
                        && previous
                            .as_ref()
                            .is_some_and(|p| p.as_table_bar().is_some())
                    {
                        let enable_http = selector
                            .main
                            .as_ref()
                            .is_some_and(|m| m.get_config().enable_http);
                        if !already_in_context_menu && enable_http {
                            let title = current.get_title();
                            let menu = ContextMenuBar::new_for_table_folder(title);
                            let bar = Bar::ContextMenu(Box::new(menu));
                            selector.manager.update_bar(Some(&bar));
                            selector.play_sound(SoundType::FolderOpen);
                        } else if selector.manager.update_bar(Some(current)) {
                            selector.play_sound(SoundType::FolderOpen);
                        }
                    }
                }
            }
            MusicSelectCommand::CopyHighlightedMenuText => {
                if let Some(selected) = selector.manager.get_selected() {
                    let content = selected.get_title();
                    if !content.is_empty()
                        && let Ok(mut clipboard) = arboard::Clipboard::new()
                    {
                        let _ = clipboard.set_text(content.clone());
                        ImGuiNotify::info(&format!("Copied highlighted menu text: {}", content));
                    }
                }
            }
        }
    }
}

// CommandExecutor — handles MusicSelectCommand execution.
//
// Ported from Java MusicSelectCommand enum. Each variant maps to a command
// that can be triggered from the select screen (e.g., keyboard shortcuts,
// context menu items).

use bms_database::SongData;
use bms_database::song_data::{FAVORITE_CHART, FAVORITE_SONG};

use super::bar_manager::{Bar, BarManager, ContextMenuItem, FunctionAction};

/// Maximum number of replay slots.
const MAX_REPLAY: i32 = 4;

/// Result of executing a music select command.
///
/// Commands that require state changes beyond the executor's scope
/// return a result that the caller (MusicSelectState) handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    /// No action needed.
    None,
    /// Show songs in the same folder as the selected song.
    ShowSameFolder { title: String, folder_crc: String },
    /// Show the context menu for the current bar.
    ShowContextMenu,
    /// Cycle to the next rival.
    NextRival,
}

/// Commands available on the music select screen.
///
/// Matches the Java `MusicSelectCommand` enum variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicSelectCommand {
    /// Reset replay selection to the first available replay.
    ResetReplay,
    /// Select the next available replay slot.
    NextReplay,
    /// Select the previous available replay slot.
    PrevReplay,
    /// Copy the selected song's MD5 hash to the clipboard.
    CopyMd5Hash,
    /// Copy the selected song's SHA-256 hash to the clipboard.
    CopySha256Hash,
    /// Copy the highlighted bar's display text to the clipboard.
    CopyHighlightedMenuText,
    /// Download the selected song via IPFS.
    #[allow(dead_code)] // TODO: integrate with download feature
    DownloadIpfs,
    /// Download the selected song via HTTP (by MD5).
    #[allow(dead_code)] // TODO: integrate with download feature
    DownloadHttp,
    /// Download all songs in the selected course via HTTP.
    #[allow(dead_code)] // TODO: integrate with download feature
    DownloadCourseHttp,
    /// Show all songs in the same folder as the selected song.
    ShowSongsOnSameFolder,
    /// Open the context menu for the current bar.
    ShowContextMenu,
    /// Cycle to the next rival.
    NextRival,
}

/// Executes music select commands.
///
/// Owns clipboard access and replay selection state.
pub struct CommandExecutor {
    clipboard: Option<arboard::Clipboard>,
    selected_replay: i32,
}

impl CommandExecutor {
    pub fn new() -> Self {
        Self {
            clipboard: arboard::Clipboard::new().ok(),
            selected_replay: 0,
        }
    }

    /// Returns the currently selected replay index.
    pub fn selected_replay(&self) -> i32 {
        self.selected_replay
    }

    /// Execute a command against the current bar manager state.
    ///
    /// Returns a `CommandResult` that the caller should handle for actions
    /// requiring broader state changes (e.g., pushing new bars).
    pub fn execute(&mut self, cmd: MusicSelectCommand, bar_manager: &BarManager) -> CommandResult {
        match cmd {
            MusicSelectCommand::ResetReplay => {
                self.selected_replay = 0;
                CommandResult::None
            }
            MusicSelectCommand::NextReplay => {
                self.selected_replay = (self.selected_replay + 1).min(MAX_REPLAY - 1);
                CommandResult::None
            }
            MusicSelectCommand::PrevReplay => {
                self.selected_replay = (self.selected_replay - 1).max(0);
                CommandResult::None
            }
            MusicSelectCommand::CopyMd5Hash => {
                if let Some(Bar::Song(s)) = bar_manager.current() {
                    self.set_clipboard(&s.md5);
                }
                CommandResult::None
            }
            MusicSelectCommand::CopySha256Hash => {
                if let Some(Bar::Song(s)) = bar_manager.current() {
                    self.set_clipboard(&s.sha256);
                }
                CommandResult::None
            }
            MusicSelectCommand::CopyHighlightedMenuText => {
                if let Some(bar) = bar_manager.current() {
                    self.set_clipboard(bar.bar_name());
                }
                CommandResult::None
            }
            MusicSelectCommand::ShowSongsOnSameFolder => {
                if let Some(Bar::Song(s)) = bar_manager.current() {
                    return CommandResult::ShowSameFolder {
                        title: s.title.clone(),
                        folder_crc: s.folder.clone(),
                    };
                }
                CommandResult::None
            }
            MusicSelectCommand::ShowContextMenu => CommandResult::ShowContextMenu,
            MusicSelectCommand::NextRival => CommandResult::NextRival,
            MusicSelectCommand::DownloadIpfs
            | MusicSelectCommand::DownloadHttp
            | MusicSelectCommand::DownloadCourseHttp => {
                tracing::info!(?cmd, "Download command requested (stub)");
                CommandResult::None
            }
        }
    }

    pub fn set_clipboard(&mut self, text: &str) {
        if let Some(cb) = &mut self.clipboard {
            if let Err(e) = cb.set_text(text.to_string()) {
                tracing::warn!("Failed to set clipboard: {e}");
            } else {
                tracing::info!(text, "Copied to clipboard");
            }
        }
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Build context menu items for a song bar.
///
/// Matches the Java `ContextMenuBar.songContext()` menu layout.
pub fn build_song_context_menu(song: &SongData) -> Vec<ContextMenuItem> {
    vec![
        ContextMenuItem {
            label: "Autoplay".to_string(),
            action: FunctionAction::Autoplay(Box::new(song.clone())),
        },
        ContextMenuItem {
            label: "Practice".to_string(),
            action: FunctionAction::Practice(Box::new(song.clone())),
        },
        ContextMenuItem {
            label: "Related (Same Folder)".to_string(),
            action: FunctionAction::ShowSameFolder {
                title: song.title.clone(),
                folder_crc: song.folder.clone(),
            },
        },
        ContextMenuItem {
            label: "Copy MD5".to_string(),
            action: FunctionAction::CopyToClipboard(song.md5.clone()),
        },
        ContextMenuItem {
            label: "Copy SHA256".to_string(),
            action: FunctionAction::CopyToClipboard(song.sha256.clone()),
        },
        ContextMenuItem {
            label: "Copy Title".to_string(),
            action: FunctionAction::CopyToClipboard(song.title.clone()),
        },
        ContextMenuItem {
            label: "Favorite Song".to_string(),
            action: FunctionAction::ToggleFavorite {
                sha256: song.sha256.clone(),
                flag: FAVORITE_SONG,
            },
        },
        ContextMenuItem {
            label: "Favorite Chart".to_string(),
            action: FunctionAction::ToggleFavorite {
                sha256: song.sha256.clone(),
                flag: FAVORITE_CHART,
            },
        },
        ContextMenuItem {
            label: "View Leaderboard".to_string(),
            action: FunctionAction::ViewLeaderboard {
                song_data: Box::new(song.clone()),
            },
        },
    ]
}

/// Build context menu items for a TableRoot bar.
///
/// Matches the Java `ContextMenuBar.tableContext()` menu layout.
pub fn build_table_context_menu(name: &str, url: Option<&str>) -> Vec<ContextMenuItem> {
    let mut items = vec![ContextMenuItem {
        label: "Copy Table Name".to_string(),
        action: FunctionAction::CopyToClipboard(name.to_string()),
    }];
    if let Some(url) = url {
        items.push(ContextMenuItem {
            label: "Open URL".to_string(),
            action: FunctionAction::OpenUrl(url.to_string()),
        });
    }
    items
}

/// Build context menu items for a HashFolder bar (table level folder).
///
/// Matches the Java `ContextMenuBar.tableFolderContext()` menu layout.
pub fn build_table_folder_context_menu(name: &str) -> Vec<ContextMenuItem> {
    vec![ContextMenuItem {
        label: "Copy Folder Name".to_string(),
        action: FunctionAction::CopyToClipboard(name.to_string()),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bar_manager_with_song(md5: &str, sha256: &str, title: &str) -> BarManager {
        let mut bm = BarManager::new();
        bm.set_bars_for_test(vec![Bar::Song(Box::new(SongData {
            md5: md5.to_string(),
            sha256: sha256.to_string(),
            title: title.to_string(),
            folder: "/songs/test".to_string(),
            ..Default::default()
        }))]);
        bm
    }

    fn make_bar_manager_with_folder(name: &str) -> BarManager {
        let mut bm = BarManager::new();
        bm.set_bars_for_test(vec![Bar::Folder {
            name: name.to_string(),
            path: "/path".to_string(),
        }]);
        bm
    }

    // --- Replay cycling tests ---

    #[test]
    fn reset_replay_sets_to_zero() {
        let mut exec = CommandExecutor::new();
        exec.selected_replay = 2;
        let bm = BarManager::new();
        exec.execute(MusicSelectCommand::ResetReplay, &bm);
        assert_eq!(exec.selected_replay(), 0);
    }

    #[test]
    fn next_replay_increments() {
        let mut exec = CommandExecutor::new();
        let bm = BarManager::new();
        assert_eq!(exec.selected_replay(), 0);

        exec.execute(MusicSelectCommand::NextReplay, &bm);
        assert_eq!(exec.selected_replay(), 1);

        exec.execute(MusicSelectCommand::NextReplay, &bm);
        assert_eq!(exec.selected_replay(), 2);

        exec.execute(MusicSelectCommand::NextReplay, &bm);
        assert_eq!(exec.selected_replay(), 3);
    }

    #[test]
    fn next_replay_clamps_at_max() {
        let mut exec = CommandExecutor::new();
        exec.selected_replay = 3;
        let bm = BarManager::new();
        exec.execute(MusicSelectCommand::NextReplay, &bm);
        assert_eq!(exec.selected_replay(), 3); // MAX_REPLAY - 1 = 3
    }

    #[test]
    fn prev_replay_decrements() {
        let mut exec = CommandExecutor::new();
        exec.selected_replay = 2;
        let bm = BarManager::new();

        exec.execute(MusicSelectCommand::PrevReplay, &bm);
        assert_eq!(exec.selected_replay(), 1);

        exec.execute(MusicSelectCommand::PrevReplay, &bm);
        assert_eq!(exec.selected_replay(), 0);
    }

    #[test]
    fn prev_replay_clamps_at_zero() {
        let mut exec = CommandExecutor::new();
        let bm = BarManager::new();
        exec.execute(MusicSelectCommand::PrevReplay, &bm);
        assert_eq!(exec.selected_replay(), 0);
    }

    // --- Clipboard tests (actual clipboard operations are skipped in CI) ---

    #[test]
    fn copy_md5_hash_on_non_song_is_noop() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_folder("My Folder");
        // Should not panic, just skip
        exec.execute(MusicSelectCommand::CopyMd5Hash, &bm);
    }

    #[test]
    fn copy_sha256_hash_on_non_song_is_noop() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_folder("My Folder");
        exec.execute(MusicSelectCommand::CopySha256Hash, &bm);
    }

    #[test]
    fn copy_md5_hash_on_empty_bar_is_noop() {
        let mut exec = CommandExecutor::new();
        let bm = BarManager::new();
        exec.execute(MusicSelectCommand::CopyMd5Hash, &bm);
    }

    #[test]
    fn copy_highlighted_text_on_empty_bar_is_noop() {
        let mut exec = CommandExecutor::new();
        let bm = BarManager::new();
        exec.execute(MusicSelectCommand::CopyHighlightedMenuText, &bm);
    }

    // --- Clipboard integration test (requires display server) ---

    #[test]
    #[ignore] // Requires a display server / clipboard access
    fn copy_md5_hash_sets_clipboard() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_song("abc123md5", "abc123sha256", "Test Song");
        exec.execute(MusicSelectCommand::CopyMd5Hash, &bm);
        // Verify clipboard content
        if let Some(cb) = &mut exec.clipboard {
            assert_eq!(cb.get_text().unwrap(), "abc123md5");
        }
    }

    #[test]
    #[ignore] // Requires a display server / clipboard access
    fn copy_sha256_hash_sets_clipboard() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_song("abc123md5", "abc123sha256", "Test Song");
        exec.execute(MusicSelectCommand::CopySha256Hash, &bm);
        if let Some(cb) = &mut exec.clipboard {
            assert_eq!(cb.get_text().unwrap(), "abc123sha256");
        }
    }

    #[test]
    #[ignore] // Requires a display server / clipboard access
    fn copy_highlighted_text_sets_clipboard() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_song("md5", "sha256", "My Song Title");
        exec.execute(MusicSelectCommand::CopyHighlightedMenuText, &bm);
        if let Some(cb) = &mut exec.clipboard {
            assert_eq!(cb.get_text().unwrap(), "My Song Title");
        }
    }

    #[test]
    #[ignore] // Requires a display server / clipboard access
    fn copy_highlighted_text_copies_folder_name() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_folder("My Folder");
        exec.execute(MusicSelectCommand::CopyHighlightedMenuText, &bm);
        if let Some(cb) = &mut exec.clipboard {
            assert_eq!(cb.get_text().unwrap(), "My Folder");
        }
    }

    // --- Download stub tests ---

    #[test]
    fn download_ipfs_on_song_does_not_panic() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_song("md5", "sha256", "Song");
        exec.execute(MusicSelectCommand::DownloadIpfs, &bm);
    }

    #[test]
    fn download_http_on_song_does_not_panic() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_song("md5", "sha256", "Song");
        exec.execute(MusicSelectCommand::DownloadHttp, &bm);
    }

    #[test]
    fn download_course_http_does_not_panic() {
        let mut exec = CommandExecutor::new();
        let bm = BarManager::new();
        exec.execute(MusicSelectCommand::DownloadCourseHttp, &bm);
    }

    // --- ShowSongsOnSameFolder / ShowContextMenu ---

    #[test]
    fn show_same_folder_returns_result_for_song() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_song("md5", "sha256", "Song");
        let result = exec.execute(MusicSelectCommand::ShowSongsOnSameFolder, &bm);
        assert_eq!(
            result,
            CommandResult::ShowSameFolder {
                title: "Song".to_string(),
                folder_crc: "/songs/test".to_string(),
            }
        );
    }

    #[test]
    fn show_same_folder_on_non_song_returns_none() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_folder("Folder");
        let result = exec.execute(MusicSelectCommand::ShowSongsOnSameFolder, &bm);
        assert_eq!(result, CommandResult::None);
    }

    #[test]
    fn show_context_menu_returns_result() {
        let mut exec = CommandExecutor::new();
        let bm = BarManager::new();
        let result = exec.execute(MusicSelectCommand::ShowContextMenu, &bm);
        assert_eq!(result, CommandResult::ShowContextMenu);
    }

    // --- Command enum coverage ---

    #[test]
    fn all_commands_are_distinct() {
        let commands = [
            MusicSelectCommand::ResetReplay,
            MusicSelectCommand::NextReplay,
            MusicSelectCommand::PrevReplay,
            MusicSelectCommand::CopyMd5Hash,
            MusicSelectCommand::CopySha256Hash,
            MusicSelectCommand::CopyHighlightedMenuText,
            MusicSelectCommand::DownloadIpfs,
            MusicSelectCommand::DownloadHttp,
            MusicSelectCommand::DownloadCourseHttp,
            MusicSelectCommand::ShowSongsOnSameFolder,
            MusicSelectCommand::ShowContextMenu,
            MusicSelectCommand::NextRival,
        ];
        // Verify all 12 variants are present
        assert_eq!(commands.len(), 12);
        // Verify each is distinct
        for (i, a) in commands.iter().enumerate() {
            for (j, b) in commands.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn next_rival_returns_result() {
        let mut exec = CommandExecutor::new();
        let bm = BarManager::new();
        let result = exec.execute(MusicSelectCommand::NextRival, &bm);
        assert_eq!(result, CommandResult::NextRival);
    }

    #[test]
    fn default_executor_has_replay_zero() {
        let exec = CommandExecutor::default();
        assert_eq!(exec.selected_replay(), 0);
    }

    // --- build_song_context_menu tests ---

    fn make_song_data() -> SongData {
        SongData {
            md5: "test_md5".to_string(),
            sha256: "test_sha256".to_string(),
            title: "Test Song".to_string(),
            folder: "/songs/test".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn build_song_context_menu_contains_autoplay_practice() {
        let song = make_song_data();
        let items = build_song_context_menu(&song);
        assert_eq!(items[0].label, "Autoplay");
        assert!(matches!(items[0].action, FunctionAction::Autoplay(_)));
        assert_eq!(items[1].label, "Practice");
        assert!(matches!(items[1].action, FunctionAction::Practice(_)));
    }

    #[test]
    fn build_song_context_menu_contains_copy_and_favorites() {
        let song = make_song_data();
        let items = build_song_context_menu(&song);
        assert_eq!(items.len(), 9);

        // Related (Same Folder)
        assert_eq!(items[2].label, "Related (Same Folder)");
        assert!(matches!(
            items[2].action,
            FunctionAction::ShowSameFolder { .. }
        ));

        // Copy items
        assert_eq!(items[3].label, "Copy MD5");
        assert!(
            matches!(items[3].action, FunctionAction::CopyToClipboard(ref s) if s == "test_md5")
        );
        assert_eq!(items[4].label, "Copy SHA256");
        assert!(
            matches!(items[4].action, FunctionAction::CopyToClipboard(ref s) if s == "test_sha256")
        );
        assert_eq!(items[5].label, "Copy Title");
        assert!(
            matches!(items[5].action, FunctionAction::CopyToClipboard(ref s) if s == "Test Song")
        );

        // Favorites
        assert_eq!(items[6].label, "Favorite Song");
        assert!(matches!(
            items[6].action,
            FunctionAction::ToggleFavorite { flag, .. } if flag == FAVORITE_SONG
        ));
        assert_eq!(items[7].label, "Favorite Chart");
        assert!(matches!(
            items[7].action,
            FunctionAction::ToggleFavorite { flag, .. } if flag == FAVORITE_CHART
        ));
    }
}

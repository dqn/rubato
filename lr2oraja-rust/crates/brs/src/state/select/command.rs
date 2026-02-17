// CommandExecutor — handles MusicSelectCommand execution.
//
// Ported from Java MusicSelectCommand enum. Each variant maps to a command
// that can be triggered from the select screen (e.g., keyboard shortcuts,
// context menu items).

use super::bar_manager::{Bar, BarManager};

/// Maximum number of replay slots.
#[allow(dead_code)] // Used in tests (via execute)
const MAX_REPLAY: i32 = 4;

/// Commands available on the music select screen.
///
/// Matches the Java `MusicSelectCommand` enum variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Used in tests
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
    DownloadIpfs,
    /// Download the selected song via HTTP (by MD5).
    DownloadHttp,
    /// Download all songs in the selected course via HTTP.
    DownloadCourseHttp,
    /// Show all songs in the same folder as the selected song.
    ShowSongsOnSameFolder,
    /// Open the context menu for the current bar.
    ShowContextMenu,
}

/// Executes music select commands.
///
/// Owns clipboard access and replay selection state.
pub struct CommandExecutor {
    #[allow(dead_code)] // Used in tests (via execute)
    clipboard: Option<arboard::Clipboard>,
    #[allow(dead_code)] // Used in tests (via execute/selected_replay)
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
    #[allow(dead_code)] // Used in tests
    pub fn selected_replay(&self) -> i32 {
        self.selected_replay
    }

    /// Execute a command against the current bar manager state.
    #[allow(dead_code)] // Used in tests
    pub fn execute(&mut self, cmd: MusicSelectCommand, bar_manager: &BarManager) {
        match cmd {
            MusicSelectCommand::ResetReplay => {
                self.selected_replay = 0;
            }
            MusicSelectCommand::NextReplay => {
                self.selected_replay = (self.selected_replay + 1).min(MAX_REPLAY - 1);
            }
            MusicSelectCommand::PrevReplay => {
                self.selected_replay = (self.selected_replay - 1).max(0);
            }
            MusicSelectCommand::CopyMd5Hash => {
                if let Some(Bar::Song(s)) = bar_manager.current() {
                    self.set_clipboard(&s.md5);
                }
            }
            MusicSelectCommand::CopySha256Hash => {
                if let Some(Bar::Song(s)) = bar_manager.current() {
                    self.set_clipboard(&s.sha256);
                }
            }
            MusicSelectCommand::CopyHighlightedMenuText => {
                if let Some(bar) = bar_manager.current() {
                    self.set_clipboard(bar.bar_name());
                }
            }
            MusicSelectCommand::ShowSongsOnSameFolder => {
                if let Some(Bar::Song(s)) = bar_manager.current() {
                    tracing::info!(
                        folder = %s.folder,
                        title = %s.title,
                        "ShowSongsOnSameFolder"
                    );
                }
            }
            MusicSelectCommand::ShowContextMenu => {
                tracing::info!("ShowContextMenu requested");
            }
            MusicSelectCommand::DownloadIpfs
            | MusicSelectCommand::DownloadHttp
            | MusicSelectCommand::DownloadCourseHttp => {
                tracing::info!(?cmd, "Download command requested (stub)");
            }
        }
    }

    #[allow(dead_code)] // Used in tests (via execute)
    fn set_clipboard(&mut self, text: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bms_database::SongData;

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
    fn show_songs_on_same_folder_does_not_panic() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_song("md5", "sha256", "Song");
        exec.execute(MusicSelectCommand::ShowSongsOnSameFolder, &bm);
    }

    #[test]
    fn show_songs_on_same_folder_on_non_song_is_noop() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_folder("Folder");
        exec.execute(MusicSelectCommand::ShowSongsOnSameFolder, &bm);
    }

    #[test]
    fn show_context_menu_does_not_panic() {
        let mut exec = CommandExecutor::new();
        let bm = BarManager::new();
        exec.execute(MusicSelectCommand::ShowContextMenu, &bm);
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
        ];
        // Verify all 11 variants are present
        assert_eq!(commands.len(), 11);
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
    fn default_executor_has_replay_zero() {
        let exec = CommandExecutor::default();
        assert_eq!(exec.selected_replay(), 0);
    }
}

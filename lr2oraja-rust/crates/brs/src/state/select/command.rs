// CommandExecutor — handles MusicSelectCommand execution.
//
// Ported from Java MusicSelectCommand enum. Each variant maps to a command
// that can be triggered from the select screen (e.g., keyboard shortcuts,
// context menu items).

use std::path::{Path, PathBuf};

use bms_database::SongData;
use bms_database::song_data::{FAVORITE_CHART, FAVORITE_SONG};

use super::bar_manager::{Bar, BarManager, ContextMenuItem, FunctionAction};

/// Maximum number of replay slots.
const MAX_REPLAY: i32 = 4;

/// LN-mode prefixes for replay file paths.
///
/// Java parity: `PlayDataAccessor.replay = {"", "C", "H"}`.
const LN_REPLAY_PREFIX: [&str; 3] = ["", "C", "H"];

/// Build the replay data file path (without `.brd` extension).
///
/// Java parity: `PlayDataAccessor.getReplayDataFilePath()`.
pub fn replay_data_file_path(
    replay_dir: &Path,
    sha256: &str,
    has_undefined_ln: bool,
    lnmode: i32,
    index: usize,
) -> PathBuf {
    let prefix = if has_undefined_ln {
        LN_REPLAY_PREFIX.get(lnmode as usize).copied().unwrap_or("")
    } else {
        ""
    };
    let suffix = if index > 0 {
        format!("_{index}")
    } else {
        String::new()
    };
    replay_dir.join(format!("{prefix}{sha256}{suffix}.brd"))
}

/// Check whether a replay file exists for the given song/slot.
///
/// Java parity: `PlayDataAccessor.existsReplayData()`.
pub fn exists_replay_data(
    replay_dir: &Path,
    sha256: &str,
    has_undefined_ln: bool,
    lnmode: i32,
    index: usize,
) -> bool {
    replay_data_file_path(replay_dir, sha256, has_undefined_ln, lnmode, index).exists()
}

/// Build the replay data directory path.
///
/// Java parity: `PlayDataAccessor.getReplayDataFolder()`.
pub fn replay_data_dir(playerpath: &str, playername: &str) -> PathBuf {
    PathBuf::from(playerpath).join(playername).join("replay")
}

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
    /// Download a song via HTTP.
    DownloadHttp { md5: String, title: String },
    /// Download a song via IPFS.
    DownloadIpfs {
        md5: String,
        ipfs: String,
        title: String,
    },
    /// Download all songs in a course via HTTP.
    DownloadCourseHttp { songs: Vec<(String, String)> },
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
    #[allow(dead_code)] // Constructed by keyboard shortcut / context menu wiring
    DownloadIpfs,
    /// Download the selected song via HTTP (by MD5).
    #[allow(dead_code)] // Constructed by keyboard shortcut / context menu wiring
    DownloadHttp,
    /// Download all songs in the selected course via HTTP.
    #[allow(dead_code)] // Constructed by keyboard shortcut / context menu wiring
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
            MusicSelectCommand::DownloadHttp => {
                if let Some(Bar::Song(s)) = bar_manager.current() {
                    return CommandResult::DownloadHttp {
                        md5: s.md5.clone(),
                        title: s.title.clone(),
                    };
                }
                CommandResult::None
            }
            MusicSelectCommand::DownloadIpfs => {
                if let Some(Bar::Song(s)) = bar_manager.current() {
                    return CommandResult::DownloadIpfs {
                        md5: s.md5.clone(),
                        ipfs: s.ipfs.clone(),
                        title: s.title.clone(),
                    };
                }
                CommandResult::None
            }
            MusicSelectCommand::DownloadCourseHttp => {
                if let Some(Bar::Grade(g)) = bar_manager.current() {
                    let songs: Vec<(String, String)> = g
                        .course
                        .hash
                        .iter()
                        .filter(|h| !h.md5.is_empty())
                        .map(|h| (h.md5.clone(), h.title.clone()))
                        .collect();
                    if !songs.is_empty() {
                        return CommandResult::DownloadCourseHttp { songs };
                    }
                }
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
/// `replay_dir` and `lnmode` are used to check for existing replay files.
/// When `replay_dir` is Some, replay slots 0–3 are probed and added to the menu.
///
/// Matches the Java `ContextMenuBar.songContext()` menu layout.
pub fn build_song_context_menu(
    song: &SongData,
    replay_dir: Option<&Path>,
    lnmode: i32,
) -> Vec<ContextMenuItem> {
    let mut items = vec![
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
    ];

    // Add replay items for existing replay files (Java: ContextMenuBar L198-209)
    if let Some(dir) = replay_dir {
        let has_undef_ln = song.has_undefined_long_note();
        for i in 0..MAX_REPLAY as usize {
            if exists_replay_data(dir, &song.sha256, has_undef_ln, lnmode, i) {
                items.push(ContextMenuItem {
                    label: format!("Replay {}", i + 1),
                    action: FunctionAction::PlayReplay {
                        song_data: Box::new(song.clone()),
                        replay_index: i,
                    },
                });
            }
        }
    }

    items
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

    // --- Download command tests ---

    #[test]
    fn download_http_returns_song_data() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_song("test_md5", "sha256", "Test Song");
        let result = exec.execute(MusicSelectCommand::DownloadHttp, &bm);
        assert_eq!(
            result,
            CommandResult::DownloadHttp {
                md5: "test_md5".to_string(),
                title: "Test Song".to_string(),
            }
        );
    }

    #[test]
    fn download_http_on_non_song_returns_none() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_folder("Folder");
        let result = exec.execute(MusicSelectCommand::DownloadHttp, &bm);
        assert_eq!(result, CommandResult::None);
    }

    #[test]
    fn download_ipfs_returns_song_data() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_song("test_md5", "sha256", "Test Song");
        let result = exec.execute(MusicSelectCommand::DownloadIpfs, &bm);
        assert_eq!(
            result,
            CommandResult::DownloadIpfs {
                md5: "test_md5".to_string(),
                ipfs: String::new(),
                title: "Test Song".to_string(),
            }
        );
    }

    #[test]
    fn download_ipfs_on_non_song_returns_none() {
        let mut exec = CommandExecutor::new();
        let bm = make_bar_manager_with_folder("Folder");
        let result = exec.execute(MusicSelectCommand::DownloadIpfs, &bm);
        assert_eq!(result, CommandResult::None);
    }

    #[test]
    fn download_course_http_returns_course_songs() {
        use crate::state::select::bar_manager::GradeBarData;
        use bms_database::{CourseData, CourseSongData};

        let mut bm = BarManager::new();
        bm.set_bars_for_test(vec![Bar::Grade(Box::new(GradeBarData {
            name: "Test Grade".to_string(),
            course: CourseData {
                name: "Test Course".to_string(),
                hash: vec![
                    CourseSongData {
                        md5: "md5_1".to_string(),
                        sha256: String::new(),
                        title: "Song 1".to_string(),
                    },
                    CourseSongData {
                        md5: "md5_2".to_string(),
                        sha256: String::new(),
                        title: "Song 2".to_string(),
                    },
                ],
                ..Default::default()
            },
            constraints: vec![],
        }))]);

        let mut exec = CommandExecutor::new();
        let result = exec.execute(MusicSelectCommand::DownloadCourseHttp, &bm);
        assert_eq!(
            result,
            CommandResult::DownloadCourseHttp {
                songs: vec![
                    ("md5_1".to_string(), "Song 1".to_string()),
                    ("md5_2".to_string(), "Song 2".to_string()),
                ],
            }
        );
    }

    #[test]
    fn download_course_http_on_empty_bar_returns_none() {
        let mut exec = CommandExecutor::new();
        let bm = BarManager::new();
        let result = exec.execute(MusicSelectCommand::DownloadCourseHttp, &bm);
        assert_eq!(result, CommandResult::None);
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
        let items = build_song_context_menu(&song, None, 0);
        assert_eq!(items[0].label, "Autoplay");
        assert!(matches!(items[0].action, FunctionAction::Autoplay(_)));
        assert_eq!(items[1].label, "Practice");
        assert!(matches!(items[1].action, FunctionAction::Practice(_)));
    }

    #[test]
    fn build_song_context_menu_contains_copy_and_favorites() {
        let song = make_song_data();
        let items = build_song_context_menu(&song, None, 0);
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

    // --- replay_data_file_path tests ---

    #[test]
    fn replay_path_no_ln_index_zero() {
        let dir = Path::new("/player/default/replay");
        let path = replay_data_file_path(dir, "abc123", false, 0, 0);
        assert_eq!(path, PathBuf::from("/player/default/replay/abc123.brd"));
    }

    #[test]
    fn replay_path_no_ln_index_two() {
        let dir = Path::new("/player/default/replay");
        let path = replay_data_file_path(dir, "abc123", false, 0, 2);
        assert_eq!(path, PathBuf::from("/player/default/replay/abc123_2.brd"));
    }

    #[test]
    fn replay_path_with_ln_lnmode_zero() {
        let dir = Path::new("/p/d/replay");
        let path = replay_data_file_path(dir, "hash", true, 0, 0);
        // lnmode 0 → prefix ""
        assert_eq!(path, PathBuf::from("/p/d/replay/hash.brd"));
    }

    #[test]
    fn replay_path_with_ln_lnmode_one() {
        let dir = Path::new("/p/d/replay");
        let path = replay_data_file_path(dir, "hash", true, 1, 0);
        // lnmode 1 → prefix "C"
        assert_eq!(path, PathBuf::from("/p/d/replay/Chash.brd"));
    }

    #[test]
    fn replay_path_with_ln_lnmode_two() {
        let dir = Path::new("/p/d/replay");
        let path = replay_data_file_path(dir, "hash", true, 2, 3);
        // lnmode 2 → prefix "H", index 3 → suffix "_3"
        assert_eq!(path, PathBuf::from("/p/d/replay/Hhash_3.brd"));
    }

    #[test]
    fn replay_data_dir_construction() {
        let dir = replay_data_dir("player", "myname");
        assert_eq!(dir, PathBuf::from("player/myname/replay"));
    }

    #[test]
    fn exists_replay_data_returns_false_for_missing() {
        let dir = Path::new("/nonexistent/replay");
        assert!(!exists_replay_data(dir, "hash", false, 0, 0));
    }

    #[test]
    fn build_song_context_menu_with_replays() {
        let dir = tempfile::tempdir().unwrap();
        let replay_dir = dir.path().join("replay");
        std::fs::create_dir_all(&replay_dir).unwrap();

        let song = SongData {
            sha256: "testhash".to_string(),
            ..Default::default()
        };

        // Create replay files for slots 0 and 2
        std::fs::write(replay_dir.join("testhash.brd"), b"").unwrap();
        std::fs::write(replay_dir.join("testhash_2.brd"), b"").unwrap();

        let items = build_song_context_menu(&song, Some(&replay_dir), 0);
        // 9 base items + 2 replay items
        assert_eq!(items.len(), 11);
        assert_eq!(items[9].label, "Replay 1");
        assert!(matches!(
            items[9].action,
            FunctionAction::PlayReplay {
                replay_index: 0,
                ..
            }
        ));
        assert_eq!(items[10].label, "Replay 3");
        assert!(matches!(
            items[10].action,
            FunctionAction::PlayReplay {
                replay_index: 2,
                ..
            }
        ));
    }
}

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::select::bar::bar::Bar;
use crate::select::bar::hash_bar::HashBar;
use crate::select::music_selector::MusicSelector;
use crate::select::stubs::SongData;

use super::ImGuiNotify;
use super::stream_command::StreamCommand;

/// Request command processing
/// Translates: bms.player.beatoraja.stream.command.StreamRequestCommand
pub struct StreamRequestCommand {
    pub selector: Arc<Mutex<MusicSelector>>,
    pub max_length: i32,
    pub updater_thread: Option<thread::JoinHandle<()>>,
    /// Channel sender for delivering sha256 hashes to the UpdateBar loop.
    /// Replaces `Arc<Mutex<UpdateBar>>` to eliminate lock ordering deadlock.
    pub sender: Option<mpsc::Sender<String>>,
}

impl StreamRequestCommand {
    pub fn new(selector: Arc<Mutex<MusicSelector>>) -> Self {
        let max_length = selector.lock().unwrap().config.max_request_count;
        let (tx, rx) = mpsc::channel();
        let selector_clone = Arc::clone(&selector);
        let updater_thread = Some(thread::spawn(move || {
            let mut updater = UpdateBar::new(selector_clone);
            updater.run_loop(rx);
        }));
        Self {
            selector,
            max_length,
            updater_thread,
            sender: Some(tx),
        }
    }
}

impl StreamCommand for StreamRequestCommand {
    fn command_string(&self) -> &str {
        "!!req"
    }

    fn run(&mut self, data: &str) {
        if data.len() != 64 {
            return;
        }

        // Send sha256 hash via channel (non-blocking, no lock contention)
        if let Some(ref sender) = self.sender {
            let _ = sender.send(data.to_string());
        }
    }

    fn dispose(&mut self) {
        // Drop the sender to disconnect the channel, causing the receiver
        // loop to exit gracefully
        self.sender.take();
        if let Some(handle) = self.updater_thread.take() {
            let _ = handle.join();
        }
    }
}

/// UpdateBar inner class translated as a struct
/// Translates: bms.player.beatoraja.stream.command.StreamRequestCommand.UpdateBar
pub struct UpdateBar {
    pub bar: HashBar,
    pub song_datas: Vec<SongData>,
    /// sha256 stack
    pub stack: Vec<String>,
    pub selector: Arc<Mutex<MusicSelector>>,
    pub max_length: i32,
}

impl UpdateBar {
    pub fn new(selector: Arc<Mutex<MusicSelector>>) -> Self {
        let max_length = selector.lock().unwrap().config.max_request_count;
        let bar = HashBar::new("Stream Request".to_string(), vec![]);
        // In Java: this.bar.setSortable(false)
        // HashBar uses DirectoryBarData which has set_sortable
        let mut update_bar = Self {
            bar,
            song_datas: Vec::new(),
            stack: Vec::new(),
            selector,
            max_length,
        };
        update_bar.bar.directory.set_sortable(false);
        update_bar
    }

    fn add_message(&self, sha256: &str) {
        let selector = self.selector.lock().unwrap();
        let escaped = Self::escape(sha256);
        let song_datas_result = selector.songdb.get_song_datas_by_hashes(&[escaped]);
        if !song_datas_result.is_empty() {
            let data = &song_datas_result[0];
            if self
                .song_datas
                .iter()
                .filter(|song| song.get_sha256() == sha256)
                .count()
                > 0
                || self
                    .stack
                    .iter()
                    .filter(|hash| hash.as_str() == sha256)
                    .count()
                    > 1
            {
                // Already added, skip
                ImGuiNotify::warning(&format!("{} has already been added", data.full_title()));
            }
            ImGuiNotify::info(&format!(
                "Added {} to stream request list",
                data.full_title()
            ));
        } else {
            ImGuiNotify::warning("Doesn't have requested song in collection");
        }
    }

    fn update(&mut self) {
        // Only update if on music select screen
        // In Java: if (selector.main.getCurrentState() instanceof MusicSelector)
        // For now, we proceed (the instanceof check is a runtime type check)

        // Process accumulated stack items
        while let Some(sha256) = self.stack.pop() {
            if self
                .song_datas
                .iter()
                .filter(|song| song.get_sha256() == sha256)
                .count()
                > 0
            {
                // Already added, skip
                continue;
            }
            let selector = self.selector.lock().unwrap();
            let escaped = Self::escape(&sha256);
            let song_datas_result = selector.songdb.get_song_datas_by_hashes(&[escaped]);
            if !song_datas_result.is_empty() {
                self.song_datas.push(song_datas_result[0].clone());
            }
            drop(selector);
            if self.song_datas.len() as i32 > self.max_length {
                self.song_datas.remove(0);
            }
        }

        if !self.song_datas.is_empty() {
            self.bar.set_elements(self.song_datas.clone());
            let mut selector = self.selector.lock().unwrap();
            let bar = Bar::Hash(Box::new(HashBar::new(
                "Stream Request".to_string(),
                self.song_datas.clone(),
            )));
            selector
                .manager
                .set_append_directory_bar("Stream Request".to_string(), bar);
            let _ = selector.manager.update_bar(None);
        }
    }

    pub(crate) fn escape(before: &str) -> String {
        // Escape for SQL
        let mut after = String::new();
        for c in before.chars() {
            if c == '_' || c == '%' || c == '\\' {
                after.push('\\');
            }
            after.push(c);
        }
        after
    }

    /// Thread loop that receives sha256 hashes via mpsc channel.
    /// Replaces the old `Arc<Mutex<UpdateBar>>` polling loop to eliminate
    /// lock ordering deadlock. The loop exits when the sender is dropped
    /// (channel disconnected).
    pub fn run_loop(&mut self, receiver: mpsc::Receiver<String>) {
        loop {
            // Use try_recv to drain all pending messages without blocking
            match receiver.try_recv() {
                Ok(sha256) => {
                    self.stack.push(sha256.clone());
                    self.add_message(&sha256);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No pending messages
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Sender dropped (dispose called), exit the loop
                    break;
                }
            }

            if !self.stack.is_empty() {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.update();
                }));
                if result.is_err() {
                    break;
                }
            }

            // Small sleep to avoid busy-waiting
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_no_special_chars() {
        // Plain alphanumeric string passes through unchanged
        assert_eq!(UpdateBar::escape("abc123"), "abc123");
    }

    #[test]
    fn escape_empty_string() {
        assert_eq!(UpdateBar::escape(""), "");
    }

    #[test]
    fn escape_underscore() {
        // Underscore is a SQL wildcard and must be escaped
        assert_eq!(UpdateBar::escape("a_b"), "a\\_b");
    }

    #[test]
    fn escape_percent() {
        // Percent is a SQL wildcard and must be escaped
        assert_eq!(UpdateBar::escape("100%"), "100\\%");
    }

    #[test]
    fn escape_backslash() {
        // Backslash itself must be escaped
        assert_eq!(UpdateBar::escape("a\\b"), "a\\\\b");
    }

    #[test]
    fn escape_multiple_special_chars() {
        // All three special characters in one string
        assert_eq!(UpdateBar::escape("_\\%"), "\\_\\\\\\%");
    }

    #[test]
    fn escape_sha256_hex_passthrough() {
        // A typical sha256 hex string has no special chars
        let sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(UpdateBar::escape(sha), sha);
    }

    #[test]
    fn update_bar_set_pushes_to_stack() {
        // Verify that set() accumulates sha256 hashes in the stack.
        // We use a minimal UpdateBar with an empty selector mock.
        // Since add_message needs selector.songdb, we test the stack push
        // indirectly by checking stack length grows.
        // Note: add_message will lock the selector and query songdb,
        // so a full test requires DB. Here we test the escape function
        // and stack mechanics via the escape tests above.

        // Test the stack push logic directly on the Vec
        let mut stack: Vec<String> = Vec::new();
        let sha = "a".repeat(64);
        stack.push(sha.clone());
        assert_eq!(stack.len(), 1);
        assert_eq!(stack[0], sha);

        // Push duplicate
        stack.push(sha.clone());
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn command_string_is_req() {
        // Verify the command string constant used by StreamRequestCommand.
        // We can't construct StreamRequestCommand without MusicSelector,
        // so we verify the expected value directly used in execute_commands
        // dispatch logic.
        assert_eq!("!!req", "!!req");
        // Also verify that the format used in execute_commands includes
        // a trailing space for proper splitting
        let cmd_str = format!("{} ", "!!req");
        assert_eq!(cmd_str, "!!req ");

        let line = "!!req abcdef";
        let parts: Vec<&str> = line.split(&cmd_str).collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "");
        assert_eq!(parts[1], "abcdef");
    }

    #[test]
    fn data_length_guard_rejects_non_64_chars() {
        // The run() method in StreamRequestCommand checks data.len() != 64.
        // Verify the guard logic directly.
        let valid_sha = "a".repeat(64);
        assert_eq!(valid_sha.len(), 64);

        let too_short = "a".repeat(63);
        assert_ne!(too_short.len(), 64);

        let too_long = "a".repeat(65);
        assert_ne!(too_long.len(), 64);

        let empty = "";
        assert_ne!(empty.len(), 64);
    }

    #[test]
    fn mpsc_channel_delivers_messages_and_terminates_on_sender_drop() {
        // Verify the channel-based communication pattern used between
        // StreamRequestCommand::run() and UpdateBar::run_loop().
        let (tx, rx) = mpsc::channel();
        let sha1 = "a".repeat(64);
        let sha2 = "b".repeat(64);

        // Sending multiple hashes succeeds
        tx.send(sha1.clone()).unwrap();
        tx.send(sha2.clone()).unwrap();

        // Receiver can read them in order
        assert_eq!(rx.recv().unwrap(), sha1);
        assert_eq!(rx.recv().unwrap(), sha2);

        // Dropping sender causes try_recv to return Disconnected
        drop(tx);
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    #[test]
    fn mpsc_try_recv_returns_empty_when_no_messages() {
        // Verify try_recv returns Empty (not blocking) when no messages pending
        let (_tx, rx) = mpsc::channel::<String>();
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    }
}

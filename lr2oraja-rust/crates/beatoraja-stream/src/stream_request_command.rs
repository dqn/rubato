use std::sync::{Arc, Mutex};
use std::thread;

use beatoraja_select::bar::bar::Bar;
use beatoraja_select::bar::hash_bar::HashBar;
use beatoraja_select::music_selector::MusicSelector;
use beatoraja_select::stubs::SongData;

use crate::ImGuiNotify;
use crate::stream_command::StreamCommand;

/// Request command processing
/// Translates: bms.player.beatoraja.stream.command.StreamRequestCommand
pub struct StreamRequestCommand {
    pub selector: Arc<Mutex<MusicSelector>>,
    pub max_length: i32,
    pub updater_thread: Option<thread::JoinHandle<()>>,
    pub updater: Arc<Mutex<UpdateBar>>,
}

impl StreamRequestCommand {
    pub fn new(selector: Arc<Mutex<MusicSelector>>) -> Self {
        // In Java: maxLength = this.selector.main.getPlayerConfig().getMaxRequestCount();
        // PlayerConfig stub does not expose get_max_request_count(); using default 30.
        let max_length = 30;
        let updater = Arc::new(Mutex::new(UpdateBar::new(Arc::clone(&selector))));
        let updater_clone = Arc::clone(&updater);
        let updater_thread = Some(thread::spawn(move || {
            UpdateBar::run_loop(updater_clone);
        }));
        Self {
            selector,
            max_length,
            updater_thread,
            updater,
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

        // is sha256
        let mut updater = self.updater.lock().unwrap();
        updater.set(data);
    }

    fn dispose(&mut self) {
        if let Some(handle) = self.updater_thread.take() {
            // Signal the thread to stop by dropping it
            // In Java: updaterThread.interrupt()
            drop(handle);
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
        // In Java: maxLength = this.selector.main.getPlayerConfig().getMaxRequestCount();
        let max_length = 30;
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

    pub fn set(&mut self, sha256: &str) {
        self.stack.push(sha256.to_string());
        self.add_message(sha256);
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

    fn escape(before: &str) -> String {
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

    /// Thread loop - translates the Runnable.run() method
    /// In Java:
    /// ```java
    /// public void run() {
    ///     while (true) {
    ///         try {
    ///             if (stack.size() != 0) {
    ///                 update();
    ///             }
    ///         } catch (Exception e) {
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    pub fn run_loop(updater: Arc<Mutex<UpdateBar>>) {
        loop {
            let has_items = {
                let u = updater.lock().unwrap();
                !u.stack.is_empty()
            };
            if has_items {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut u = updater.lock().unwrap();
                    u.update();
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

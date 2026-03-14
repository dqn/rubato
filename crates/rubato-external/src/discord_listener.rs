use crate::discord_rpc::rich_presence::{RichPresence, RichPresenceData};

use crate::stubs::{MainStateListener, ScreenType};
use rubato_types::main_state_access::MainStateAccess;

static APPLICATION_ID: &str = "1054234988167561277";

/// Command sent from the render thread to the background Discord IPC thread.
enum DiscordCommand {
    /// Update the Rich Presence activity (boxed to avoid large enum variant).
    Update(Box<RichPresenceData>),
    /// Shut down: close the connection and exit the thread.
    Shutdown,
}

/// Discord Rich Presence listener.
/// Translated from Java: DiscordListener implements MainStateListener
///
/// All IPC calls (write + read) are performed on a dedicated background thread
/// to avoid blocking the render thread. The render thread communicates via an
/// mpsc channel, sending `RichPresenceData` snapshots each frame.
pub struct DiscordListener {
    /// Channel sender for the background thread. None if connection failed.
    sender: Option<std::sync::mpsc::Sender<DiscordCommand>>,
    /// Background thread handle, joined on close().
    thread_handle: Option<std::thread::JoinHandle<()>>,
    /// Cached start timestamp (Unix seconds), captured once per activity change
    start_timestamp: i64,
    /// Last observed screen type, used to detect activity changes
    last_screen_type: Option<ScreenType>,
}

impl DiscordListener {
    pub fn new() -> Self {
        match Self::try_connect() {
            Ok((sender, handle)) => {
                log::info!("Discord RPC Ready!");
                Self {
                    sender: Some(sender),
                    thread_handle: Some(handle),
                    start_timestamp: 0,
                    last_screen_type: None,
                }
            }
            Err(e) => {
                log::warn!("Failed to initialize Discord RPC: {}", e);
                Self {
                    sender: None,
                    thread_handle: None,
                    start_timestamp: 0,
                    last_screen_type: None,
                }
            }
        }
    }

    fn try_connect() -> anyhow::Result<(
        std::sync::mpsc::Sender<DiscordCommand>,
        std::thread::JoinHandle<()>,
    )> {
        let mut rp = RichPresence::new(APPLICATION_ID.to_string());
        rp.connect()?;

        let (tx, rx) = std::sync::mpsc::channel::<DiscordCommand>();
        let handle = std::thread::Builder::new()
            .name("discord-rpc".to_string())
            .spawn(move || {
                Self::background_loop(rp, rx);
            })?;

        Ok((tx, handle))
    }

    /// Background thread loop: receives commands and performs IPC.
    fn background_loop(mut rp: RichPresence, rx: std::sync::mpsc::Receiver<DiscordCommand>) {
        loop {
            match rx.recv() {
                Ok(DiscordCommand::Update(data)) => {
                    if let Err(e) = rp.update(*data) {
                        log::warn!("Failed to update Discord Rich Presence: {}", e);
                    }
                }
                Ok(DiscordCommand::Shutdown) | Err(_) => {
                    rp.close();
                    break;
                }
            }
        }
    }

    pub fn close(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(DiscordCommand::Shutdown);
        }
        if let Some(handle) = self.thread_handle.take()
            && let Err(e) = handle.join()
        {
            log::warn!("Discord RPC thread panicked: {:?}", e);
        }
    }
}

impl Default for DiscordListener {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DiscordListener {
    fn drop(&mut self) {
        self.close();
    }
}

impl MainStateListener for DiscordListener {
    fn update(&mut self, state: &dyn MainStateAccess, _status: i32) {
        let sender = match self.sender.as_ref() {
            Some(s) => s,
            None => return,
        };

        let screen_type = state.screen_type();

        // Capture start_timestamp once when the activity (screen) changes
        if self.last_screen_type != Some(screen_type) {
            self.last_screen_type = Some(screen_type);
            self.start_timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
        }

        let mut data = RichPresenceData::new()
            .set_start_timestamp(self.start_timestamp)
            .set_large_image("bms".to_string(), String::new());

        match screen_type {
            ScreenType::MusicSelector => {
                data = data.set_state("In Music Select Menu".to_string());
            }
            ScreenType::MusicDecide => {
                data = data.set_state("Decide Screen".to_string());
            }
            ScreenType::BMSPlayer => {
                if let Some(resource) = state.resource()
                    && let Some(songdata) = resource.songdata()
                {
                    let full_title = if songdata.metadata.subtitle.is_empty() {
                        songdata.metadata.title.clone()
                    } else {
                        format!("{} {}", songdata.metadata.title, songdata.metadata.subtitle)
                    };
                    data =
                        data.set_details(format!("{} / {}", full_title, songdata.metadata.artist));
                    data = data.set_state(format!("Playing: {}Keys", songdata.chart.mode));
                }
            }
            ScreenType::MusicResult => {
                data = data.set_state("Result Screen".to_string());
            }
            ScreenType::CourseResult => {
                data = data.set_state("Course Result Screen".to_string());
            }
            _ => {}
        }

        if sender.send(DiscordCommand::Update(Box::new(data))).is_err() {
            log::warn!("Discord RPC background thread disconnected");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord_rpc::connection::IPCConnection;
    use crate::discord_rpc::rich_presence::RichPresence;
    use std::sync::{Arc, Mutex};

    struct MockConnection {
        written: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl IPCConnection for MockConnection {
        fn connect(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
        fn write(&mut self, buffer: &[u8]) -> anyhow::Result<()> {
            self.written.lock().unwrap().push(buffer.to_vec());
            Ok(())
        }
        fn read(&mut self, size: usize) -> anyhow::Result<Vec<u8>> {
            if size == 8 {
                let mut header = Vec::new();
                header.extend_from_slice(&1_i32.to_le_bytes());
                header.extend_from_slice(&2_i32.to_le_bytes());
                Ok(header)
            } else {
                Ok(vec![0u8; size])
            }
        }
        fn close(&mut self) {}
    }

    #[test]
    fn test_discord_listener_background_thread_processes_update() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mock = MockConnection {
            written: Arc::clone(&written),
        };

        let mut rp = RichPresence::with_connection(APPLICATION_ID.to_string(), Box::new(mock));
        rp.connect().unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            DiscordListener::background_loop(rp, rx);
        });

        let data = RichPresenceData::new().set_state("Test".to_string());
        tx.send(DiscordCommand::Update(Box::new(data))).unwrap();
        tx.send(DiscordCommand::Shutdown).unwrap();

        handle.join().expect("background thread should complete");

        // Handshake write + update write = at least 2 writes
        let writes = written.lock().unwrap();
        assert!(
            writes.len() >= 2,
            "expected at least 2 writes (handshake + update), got {}",
            writes.len()
        );
    }

    #[test]
    fn test_discord_listener_close_joins_thread() {
        let mock = MockConnection {
            written: Arc::new(Mutex::new(Vec::new())),
        };

        let mut rp = RichPresence::with_connection(APPLICATION_ID.to_string(), Box::new(mock));
        rp.connect().unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            DiscordListener::background_loop(rp, rx);
        });

        let mut listener = DiscordListener {
            sender: Some(tx),
            thread_handle: Some(handle),
            start_timestamp: 0,
            last_screen_type: None,
        };

        // close() should send Shutdown and join the thread without hanging
        listener.close();
        assert!(listener.sender.is_none());
        assert!(listener.thread_handle.is_none());
    }

    #[test]
    fn test_discord_listener_drop_sends_shutdown() {
        let mock = MockConnection {
            written: Arc::new(Mutex::new(Vec::new())),
        };

        let mut rp = RichPresence::with_connection(APPLICATION_ID.to_string(), Box::new(mock));
        rp.connect().unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            DiscordListener::background_loop(rp, rx);
        });

        // Drop should trigger close() via Drop impl
        drop(DiscordListener {
            sender: Some(tx),
            thread_handle: Some(handle),
            start_timestamp: 0,
            last_screen_type: None,
        });
        // If we get here without hanging, Drop worked correctly
    }
}

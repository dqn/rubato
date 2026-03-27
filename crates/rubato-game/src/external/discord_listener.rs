use crate::external::discord_rpc::rich_presence::{RichPresence, RichPresenceData};

use rubato_types::app_event::{AppEvent, StateChangedData};
use rubato_types::screen_type::ScreenType;

static APPLICATION_ID: &str = "1054234988167561277";

/// Command sent from the event-processing thread to the Discord IPC thread.
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
/// `AppEvent` channel, and a bridge thread translates relevant events into
/// `DiscordCommand`s for the IPC thread.
pub struct DiscordListener {
    /// Channel sender for the IPC background thread. None if connection failed.
    ipc_sender: Option<std::sync::mpsc::SyncSender<DiscordCommand>>,
    /// IPC background thread handle, joined on close().
    ipc_thread: Option<std::thread::JoinHandle<()>>,
    /// Bridge thread that reads AppEvent and forwards to IPC thread.
    bridge_thread: Option<std::thread::JoinHandle<()>>,
}

impl DiscordListener {
    /// Create a new DiscordListener and return `(app_event_sender, listener)`.
    ///
    /// The caller should register `app_event_sender` with `MainController::add_event_sender()`.
    /// The listener must be kept alive (not dropped) for the background threads to run.
    pub fn new() -> (std::sync::mpsc::SyncSender<AppEvent>, Self) {
        match Self::try_connect() {
            Ok((app_tx, listener)) => {
                log::info!("Discord RPC Ready!");
                (app_tx, listener)
            }
            Err(e) => {
                log::warn!("Failed to initialize Discord RPC: {}", e);
                // Return a sender that goes nowhere (receiver dropped immediately)
                let (app_tx, _) = std::sync::mpsc::sync_channel(1);
                (
                    app_tx,
                    Self {
                        ipc_sender: None,
                        ipc_thread: None,
                        bridge_thread: None,
                    },
                )
            }
        }
    }

    fn try_connect() -> anyhow::Result<(std::sync::mpsc::SyncSender<AppEvent>, Self)> {
        let mut rp = RichPresence::new(APPLICATION_ID.to_string());
        rp.connect()?;

        // IPC channel: bridge thread -> Discord IPC thread
        let (ipc_tx, ipc_rx) = std::sync::mpsc::sync_channel::<DiscordCommand>(2);
        let ipc_handle = std::thread::Builder::new()
            .name("discord-rpc".to_string())
            .spawn(move || {
                Self::ipc_loop(rp, ipc_rx);
            })?;

        // AppEvent channel: MainController -> bridge thread
        let (app_tx, app_rx) = std::sync::mpsc::sync_channel::<AppEvent>(256);
        let ipc_tx_clone = ipc_tx.clone();
        let bridge_handle = std::thread::Builder::new()
            .name("discord-bridge".to_string())
            .spawn(move || {
                Self::bridge_loop(app_rx, ipc_tx_clone);
            })?;

        Ok((
            app_tx,
            Self {
                ipc_sender: Some(ipc_tx),
                ipc_thread: Some(ipc_handle),
                bridge_thread: Some(bridge_handle),
            },
        ))
    }

    /// Bridge thread: reads `AppEvent`s and translates `StateChanged` events
    /// into Discord Rich Presence updates.
    fn bridge_loop(
        rx: std::sync::mpsc::Receiver<AppEvent>,
        ipc_tx: std::sync::mpsc::SyncSender<DiscordCommand>,
    ) {
        let mut start_timestamp: i64 = 0;
        let mut last_screen_type: Option<ScreenType> = None;

        loop {
            match rx.recv() {
                Ok(AppEvent::StateChanged(data)) => {
                    if let Some(rp_data) =
                        Self::build_presence(&data, &mut start_timestamp, &mut last_screen_type)
                    {
                        let _ = ipc_tx.try_send(DiscordCommand::Update(Box::new(rp_data)));
                    }
                }
                Ok(AppEvent::Lifecycle(_)) => {
                    // Lifecycle events are not relevant for Discord RPC.
                }
                Err(_) => {
                    // Channel disconnected; send shutdown to IPC thread.
                    let _ = ipc_tx.send(DiscordCommand::Shutdown);
                    break;
                }
            }
        }
    }

    /// Build a `RichPresenceData` from a `StateChangedData` snapshot.
    /// Returns `None` if the screen type is not relevant for Discord display.
    fn build_presence(
        data: &StateChangedData,
        start_timestamp: &mut i64,
        last_screen_type: &mut Option<ScreenType>,
    ) -> Option<RichPresenceData> {
        let screen_type = data.screen_type;

        // Capture start_timestamp once when the activity (screen) changes
        if *last_screen_type != Some(screen_type) {
            *last_screen_type = Some(screen_type);
            *start_timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
        }

        let mut rp_data = RichPresenceData::new()
            .set_start_timestamp(*start_timestamp)
            .set_large_image("bms".to_string(), String::new());

        match screen_type {
            ScreenType::MusicSelector => {
                rp_data = rp_data.set_state("In Music Select Menu".to_string());
            }
            ScreenType::MusicDecide => {
                rp_data = rp_data.set_state("Decide Screen".to_string());
            }
            ScreenType::BMSPlayer => {
                if let Some(ref song_info) = data.song_info {
                    let full_title = if song_info.subtitle.is_empty() {
                        song_info.title.clone()
                    } else {
                        format!("{} {}", song_info.title, song_info.subtitle)
                    };
                    rp_data = rp_data.set_details(format!("{} / {}", full_title, song_info.artist));
                    rp_data = rp_data.set_state(format!("Playing: {}Keys", song_info.mode));
                }
            }
            ScreenType::MusicResult => {
                rp_data = rp_data.set_state("Result Screen".to_string());
            }
            ScreenType::CourseResult => {
                rp_data = rp_data.set_state("Course Result Screen".to_string());
            }
            _ => {}
        }

        Some(rp_data)
    }

    /// IPC thread loop: receives commands and performs Discord IPC.
    ///
    /// Drains to the latest `Update` before processing so that rapid state
    /// transitions don't queue stale updates behind slow IPC calls.
    fn ipc_loop(mut rp: RichPresence, rx: std::sync::mpsc::Receiver<DiscordCommand>) {
        loop {
            match rx.recv() {
                Ok(DiscordCommand::Update(mut data)) => {
                    // Drain any queued updates, keeping only the latest.
                    let mut shutdown_after = false;
                    while let Ok(cmd) = rx.try_recv() {
                        match cmd {
                            DiscordCommand::Update(newer) => data = newer,
                            DiscordCommand::Shutdown => {
                                shutdown_after = true;
                                break;
                            }
                        }
                    }
                    if let Err(e) = rp.update(*data) {
                        log::warn!("Failed to update Discord Rich Presence: {}", e);
                    }
                    if shutdown_after {
                        rp.close();
                        return;
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
        if let Some(sender) = self.ipc_sender.take() {
            let _ = sender.send(DiscordCommand::Shutdown);
        }
        if let Some(handle) = self.bridge_thread.take()
            && let Err(e) = handle.join()
        {
            log::warn!("Discord bridge thread panicked: {:?}", e);
        }
        if let Some(handle) = self.ipc_thread.take()
            && let Err(e) = handle.join()
        {
            log::warn!("Discord RPC thread panicked: {:?}", e);
        }
    }
}

impl Default for DiscordListener {
    fn default() -> Self {
        Self::new().1
    }
}

impl Drop for DiscordListener {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::discord_rpc::connection::IPCConnection;
    use crate::external::discord_rpc::rich_presence::RichPresence;
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
    fn test_discord_listener_ipc_thread_processes_update() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mock = MockConnection {
            written: Arc::clone(&written),
        };

        let mut rp = RichPresence::with_connection(APPLICATION_ID.to_string(), Box::new(mock));
        rp.connect().unwrap();

        let (tx, rx) = std::sync::mpsc::sync_channel::<DiscordCommand>(2);
        let handle = std::thread::spawn(move || {
            DiscordListener::ipc_loop(rp, rx);
        });

        let data = RichPresenceData::new().set_state("Test".to_string());
        tx.send(DiscordCommand::Update(Box::new(data))).unwrap();
        tx.send(DiscordCommand::Shutdown).unwrap();

        handle.join().expect("IPC thread should complete");

        // Handshake write + update write = at least 2 writes
        let writes = written.lock().unwrap();
        assert!(
            writes.len() >= 2,
            "expected at least 2 writes (handshake + update), got {}",
            writes.len()
        );
    }

    #[test]
    fn test_discord_listener_close_joins_threads() {
        let mock = MockConnection {
            written: Arc::new(Mutex::new(Vec::new())),
        };

        let mut rp = RichPresence::with_connection(APPLICATION_ID.to_string(), Box::new(mock));
        rp.connect().unwrap();

        let (ipc_tx, ipc_rx) = std::sync::mpsc::sync_channel::<DiscordCommand>(2);
        let ipc_handle = std::thread::spawn(move || {
            DiscordListener::ipc_loop(rp, ipc_rx);
        });

        let mut listener = DiscordListener {
            ipc_sender: Some(ipc_tx),
            ipc_thread: Some(ipc_handle),
            bridge_thread: None,
        };

        // close() should send Shutdown and join the thread without hanging
        listener.close();
        assert!(listener.ipc_sender.is_none());
        assert!(listener.ipc_thread.is_none());
    }

    #[test]
    fn test_discord_listener_drop_sends_shutdown() {
        let mock = MockConnection {
            written: Arc::new(Mutex::new(Vec::new())),
        };

        let mut rp = RichPresence::with_connection(APPLICATION_ID.to_string(), Box::new(mock));
        rp.connect().unwrap();

        let (ipc_tx, ipc_rx) = std::sync::mpsc::sync_channel::<DiscordCommand>(2);
        let ipc_handle = std::thread::spawn(move || {
            DiscordListener::ipc_loop(rp, ipc_rx);
        });

        // Drop should trigger close() via Drop impl
        drop(DiscordListener {
            ipc_sender: Some(ipc_tx),
            ipc_thread: Some(ipc_handle),
            bridge_thread: None,
        });
        // If we get here without hanging, Drop worked correctly
    }

    #[test]
    fn test_build_presence_music_select() {
        let mut start_ts = 0i64;
        let mut last_screen = None;

        let data = StateChangedData {
            screen_type: ScreenType::MusicSelector,
            state_type: Some(rubato_types::main_state_type::MainStateType::MusicSelect),
            status: 0,
            song_info: None,
        };

        let result = DiscordListener::build_presence(&data, &mut start_ts, &mut last_screen);
        assert!(result.is_some());
        assert!(start_ts > 0);
        assert_eq!(last_screen, Some(ScreenType::MusicSelector));
    }

    #[test]
    fn test_build_presence_bms_player_with_song_info() {
        let mut start_ts = 0i64;
        let mut last_screen = None;

        let data = StateChangedData {
            screen_type: ScreenType::BMSPlayer,
            state_type: Some(rubato_types::main_state_type::MainStateType::Play),
            status: 0,
            song_info: Some(rubato_types::app_event::SongInfo {
                title: "Test Song".to_string(),
                subtitle: "".to_string(),
                artist: "Test Artist".to_string(),
                mode: 7,
            }),
        };

        let result = DiscordListener::build_presence(&data, &mut start_ts, &mut last_screen);
        assert!(result.is_some());
    }
}

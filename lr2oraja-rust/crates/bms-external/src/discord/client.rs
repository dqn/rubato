use tokio::sync::mpsc;
use tracing::{info, warn};

use super::listener::APP_ID;
use super::platform_ipc::PlatformIpcConnection;
use super::rich_presence::RichPresenceClient;

/// Commands sent to the Discord background task.
#[derive(Debug)]
pub enum DiscordCommand {
    /// Connect to Discord IPC.
    Connect,
    /// Update Rich Presence display.
    UpdatePresence { details: String, state: String },
    /// Disconnect from Discord IPC.
    Disconnect,
}

/// Channel-based Discord RPC client.
///
/// Provides a synchronous API by sending commands to a background tokio task.
/// Follows the same pattern as `ObsWsClient`.
pub struct DiscordRpcClient {
    command_tx: mpsc::UnboundedSender<DiscordCommand>,
}

impl Default for DiscordRpcClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordRpcClient {
    /// Create a new client and spawn the background connection task.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(discord_task(rx));
        Self { command_tx: tx }
    }

    /// Request connection to Discord IPC.
    pub fn connect(&self) {
        if let Err(e) = self.command_tx.send(DiscordCommand::Connect) {
            warn!("Discord command channel closed: {}", e);
        }
    }

    /// Update Rich Presence.
    pub fn update_presence(&self, details: &str, state: &str) {
        if let Err(e) = self.command_tx.send(DiscordCommand::UpdatePresence {
            details: details.to_string(),
            state: state.to_string(),
        }) {
            warn!("Discord command channel closed: {}", e);
        }
    }

    /// Request disconnection from Discord IPC.
    pub fn disconnect(&self) {
        if let Err(e) = self.command_tx.send(DiscordCommand::Disconnect) {
            warn!("Discord command channel closed: {}", e);
        }
    }
}

/// Background task that manages the Discord IPC connection.
async fn discord_task(mut rx: mpsc::UnboundedReceiver<DiscordCommand>) {
    let mut client: Option<RichPresenceClient<PlatformIpcConnection>> = None;

    while let Some(cmd) = rx.recv().await {
        match cmd {
            DiscordCommand::Connect => {
                // Disconnect existing connection first
                if let Some(ref mut c) = client {
                    if let Err(e) = c.close().await {
                        warn!("Failed to close previous Discord connection: {}", e);
                    }
                    client = None;
                }

                let conn = PlatformIpcConnection::new();
                let mut rpc = RichPresenceClient::new(conn, APP_ID);
                match rpc.connect().await {
                    Ok(()) => {
                        info!("Discord RPC connected");
                        client = Some(rpc);
                    }
                    Err(e) => {
                        warn!("Failed to connect to Discord RPC: {}", e);
                    }
                }
            }
            DiscordCommand::UpdatePresence { details, state } => {
                if let Some(ref mut c) = client
                    && let Err(e) = c.set_activity(&details, &state, "icon", "brs").await
                {
                    warn!("Failed to update Discord presence: {}", e);
                }
            }
            DiscordCommand::Disconnect => {
                if let Some(ref mut c) = client {
                    if let Err(e) = c.close().await {
                        warn!("Failed to close Discord connection: {}", e);
                    }
                    client = None;
                    info!("Discord RPC disconnected");
                }
            }
        }
    }

    // Channel closed, clean up
    if let Some(ref mut c) = client {
        let _ = c.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify DiscordCommand enum variants exist
    #[test]
    fn command_variants() {
        let _ = DiscordCommand::Connect;
        let _ = DiscordCommand::UpdatePresence {
            details: "test".to_string(),
            state: "test".to_string(),
        };
        let _ = DiscordCommand::Disconnect;
    }

    // Test that client methods don't panic when the background task has exited
    #[tokio::test]
    async fn methods_no_panic_after_task_exit() {
        let (tx, rx) = mpsc::unbounded_channel();
        // Drop rx immediately to simulate task exit
        drop(rx);

        let client = DiscordRpcClient { command_tx: tx };
        // These should log warnings but not panic
        client.connect();
        client.update_presence("test", "test");
        client.disconnect();
    }
}

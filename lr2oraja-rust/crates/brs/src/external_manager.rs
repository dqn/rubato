use std::sync::Arc;

use bms_config::{Config, PlayerConfig};
use bms_external::discord::client::DiscordRpcClient;
use bms_external::obs::client::ObsWsClient;
use bms_external::obs::listener::ObsListener;
use bms_stream::command::StreamRequestCommand;
use bms_stream::controller::StreamController;
use tracing::{info, warn};

/// Manages external integrations (Discord RPC, OBS, Streaming).
///
/// Holds a dedicated tokio `Runtime` because Bevy's main thread is synchronous.
/// Each client is `Option` — `None` means the integration is disabled or failed to initialize.
pub struct ExternalManager {
    #[allow(dead_code)] // Used via runtime.enter() which doesn't count as a field read
    runtime: tokio::runtime::Runtime,
    discord: Option<DiscordRpcClient>,
    obs: Option<ObsListener>,
    stream: Option<StreamController>,
}

impl Default for ExternalManager {
    fn default() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for ExternalManager");
        Self {
            runtime,
            discord: None,
            obs: None,
            stream: None,
        }
    }
}

impl ExternalManager {
    /// Create from config. Connects to enabled integrations.
    pub fn new(config: &Config, player_config: &PlayerConfig) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for ExternalManager");

        // Discord RPC
        let discord = if config.use_discord_rpc {
            let _guard = runtime.enter();
            let client = DiscordRpcClient::new();
            client.connect();
            info!("Discord RPC enabled");
            Some(client)
        } else {
            None
        };

        // OBS WebSocket
        let obs = if config.use_obs_ws {
            let _guard = runtime.enter();
            let ws_client = ObsWsClient::new();
            let listener = ObsListener::new(
                ws_client,
                config.obs_scenes.clone(),
                config.obs_actions.clone(),
            );
            if let Err(e) = listener.connect(
                &config.obs_ws_host,
                config.obs_ws_port as u16,
                &config.obs_ws_pass,
            ) {
                warn!("Failed to connect to OBS WebSocket: {}", e);
            } else {
                info!("OBS WebSocket enabled");
            }
            Some(listener)
        } else {
            None
        };

        // Stream controller
        let stream = if player_config.enable_request {
            let _guard = runtime.enter();
            let cmd = Arc::new(StreamRequestCommand::default());
            let mut controller = StreamController::new(vec![cmd]);
            controller.start();
            info!("Stream controller enabled");
            Some(controller)
        } else {
            None
        };

        Self {
            runtime,
            discord,
            obs,
            stream,
        }
    }

    /// Called on state transitions to update external integrations.
    pub fn on_state_change(
        &mut self,
        state_name: &str,
        song_title: Option<&str>,
        artist: Option<&str>,
        key_count: Option<usize>,
    ) {
        // Discord Rich Presence
        if let Some(ref discord) = self.discord {
            let (details, state) = match state_name {
                "Play" => {
                    let details = match (song_title, artist) {
                        (Some(title), Some(art)) => format!("{} / {}", title, art),
                        (Some(title), None) => title.to_string(),
                        _ => "Playing".to_string(),
                    };
                    let state = match key_count {
                        Some(k) => format!("Playing: {}Keys", k),
                        None => "Playing".to_string(),
                    };
                    (details, state)
                }
                "MusicSelect" => (String::new(), "Music Select".to_string()),
                _ => (String::new(), state_name.to_string()),
            };
            discord.update_presence(&details, &state);
        }

        // OBS scene switching (Java-compatible state name format)
        if let Some(ref obs) = self.obs {
            let obs_state = to_obs_state_name(state_name);
            if let Err(e) = obs.on_state_changed(&obs_state) {
                warn!("OBS state change failed: {}", e);
            }
        }
    }

    /// Called on shutdown to clean up connections.
    pub fn shutdown(&mut self) {
        if let Some(ref discord) = self.discord {
            discord.disconnect();
            info!("Discord RPC disconnected");
        }
        self.discord = None;

        if let Some(ref obs) = self.obs {
            if let Err(e) = obs.disconnect() {
                warn!("OBS disconnect failed: {}", e);
            } else {
                info!("OBS WebSocket disconnected");
            }
        }
        self.obs = None;

        if let Some(ref mut stream) = self.stream {
            stream.stop();
            info!("Stream controller stopped");
        }
        self.stream = None;
    }

    pub fn is_discord_enabled(&self) -> bool {
        self.discord.is_some()
    }

    pub fn is_obs_enabled(&self) -> bool {
        self.obs.is_some()
    }

    pub fn is_stream_enabled(&self) -> bool {
        self.stream.is_some()
    }

    /// Access the tokio runtime (for tests or advanced usage).
    #[cfg(test)]
    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        &self.runtime
    }
}

impl Drop for ExternalManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Convert game state name to Java-compatible OBS state name.
///
/// Java uses uppercase format: `MUSICSELECT`, `PLAY`, `RESULT`, etc.
fn to_obs_state_name(state_name: &str) -> String {
    match state_name {
        "MusicSelect" => "MUSICSELECT".to_string(),
        "CourseResult" => "COURSERESULT".to_string(),
        "KeyConfig" => "KEYCONFIG".to_string(),
        "SkinConfig" => "SKINCONFIG".to_string(),
        other => other.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_all_disabled() {
        let mgr = ExternalManager::default();
        assert!(!mgr.is_discord_enabled());
        assert!(!mgr.is_obs_enabled());
        assert!(!mgr.is_stream_enabled());
    }

    #[test]
    fn new_from_default_config_all_disabled() {
        let config = Config::default();
        let player_config = PlayerConfig::default();
        let mgr = ExternalManager::new(&config, &player_config);
        assert!(!mgr.is_discord_enabled());
        assert!(!mgr.is_obs_enabled());
        assert!(!mgr.is_stream_enabled());
    }

    #[test]
    fn on_state_change_no_panic_when_disabled() {
        let mut mgr = ExternalManager::default();
        mgr.on_state_change("Play", Some("test song"), Some("artist"), Some(7));
        mgr.on_state_change("Result", None, None, None);
        mgr.on_state_change("MusicSelect", None, None, None);
    }

    #[test]
    fn shutdown_no_panic_when_disabled() {
        let mut mgr = ExternalManager::default();
        mgr.shutdown();
    }

    #[test]
    fn obs_state_name_conversion() {
        assert_eq!(to_obs_state_name("MusicSelect"), "MUSICSELECT");
        assert_eq!(to_obs_state_name("Play"), "PLAY");
        assert_eq!(to_obs_state_name("Result"), "RESULT");
        assert_eq!(to_obs_state_name("Decide"), "DECIDE");
        assert_eq!(to_obs_state_name("CourseResult"), "COURSERESULT");
        assert_eq!(to_obs_state_name("KeyConfig"), "KEYCONFIG");
        assert_eq!(to_obs_state_name("SkinConfig"), "SKINCONFIG");
    }

    #[test]
    fn drop_calls_shutdown() {
        // Verify Drop doesn't panic
        let mgr = ExternalManager::default();
        drop(mgr);
    }
}

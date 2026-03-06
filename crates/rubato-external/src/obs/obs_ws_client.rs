use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use futures_util::{SinkExt, StreamExt};
use log::{info, warn};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use rubato_core::config::Config;

use rubato_types::imgui_notify::ImGuiNotify;

use super::lock_or_recover;

/// ObsRecordingMode - recording mode enum
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObsRecordingMode {
    KeepAll = 0,
    OnScreenshot = 1,
    OnReplay = 2,
}

impl ObsRecordingMode {
    pub fn value(&self) -> i32 {
        *self as i32
    }

    pub fn from_value(value: i32) -> Result<Self> {
        match value {
            0 => Ok(ObsRecordingMode::KeepAll),
            1 => Ok(ObsRecordingMode::OnScreenshot),
            2 => Ok(ObsRecordingMode::OnReplay),
            _ => Err(anyhow::anyhow!("No matching enum for value: {}", value)),
        }
    }
}

/// ObsVersionInfo - holds OBS version information
#[derive(Clone, Debug)]
pub struct ObsVersionInfo {
    pub obs_version: String,
    pub ws_version: String,
}

impl ObsVersionInfo {
    pub fn new(obs_version: String, ws_version: String) -> Self {
        Self {
            obs_version,
            ws_version,
        }
    }

    pub fn get_obs_version(&self) -> &str {
        &self.obs_version
    }

    pub fn get_ws_version(&self) -> &str {
        &self.ws_version
    }
}

impl std::fmt::Display for ObsVersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OBS v{} (WS v{})", self.obs_version, self.ws_version)
    }
}

/// OBS_ACTIONS - mapping of action labels to action names
pub fn obs_actions() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("Stop Recording".to_string(), "StopRecord".to_string());
    map.insert("Start Recording".to_string(), "StartRecord".to_string());
    map
}

/// Get the label for a given action
pub fn get_action_label(action: &str) -> Option<String> {
    for (key, value) in obs_actions() {
        if value == action {
            return Some(key);
        }
    }
    None
}

/// Type for WebSocket write half
type WsSink = futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

/// Shared inner state for ObsWsClient
struct ObsWsClientInner {
    ws_sink: Option<WsSink>,
    is_connected: bool,
    is_identified: bool,
    is_recording: bool,
    restart_recording: bool,
    auto_reconnect: bool,
    is_reconnecting: bool,
    is_shutting_down: bool,
    save_requested: bool,
    recording_mode: ObsRecordingMode,
    output_path: String,
    last_output_path: String,
    current_reconnect_delay: i32,
    /// Stored for reconnection — needed by schedule_reconnect which only has access to inner.
    server_uri: String,
    /// Stored for reconnection — needed by schedule_reconnect which only has access to inner.
    password: String,
    /// Stored for reconnection — shutdown_notify is needed to pass to do_connect.
    shutdown_notify: Arc<Notify>,

    on_close_handler: Option<Arc<dyn Fn() + Send + Sync>>,
    on_error_handler: Option<Arc<dyn Fn(String) + Send + Sync>>,
    on_version_received: Option<Arc<dyn Fn(ObsVersionInfo) + Send + Sync>>,
    on_scenes_received: Option<Arc<dyn Fn(Vec<String>) + Send + Sync>>,
    on_record_state_changed: Option<Arc<dyn Fn(String) + Send + Sync>>,
    custom_message_handler: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

/// ObsWsClient - WebSocket client for OBS Studio
pub struct ObsWsClient {
    inner: Arc<Mutex<ObsWsClientInner>>,
    password: String,
    request_id_counter: AtomicI64,
    server_uri: String,
    shutdown_notify: Arc<Notify>,
    runtime: tokio::runtime::Handle,
}

// Constants
const INITIAL_RECONNECT_DELAY_MS: i32 = 2000;
const MAX_RECONNECT_DELAY_MS: i32 = 15000;
const RECONNECT_BACKOFF_MULTIPLIER: f64 = 1.25;
const _: () = {
    assert!(INITIAL_RECONNECT_DELAY_MS > 0);
    assert!(MAX_RECONNECT_DELAY_MS > INITIAL_RECONNECT_DELAY_MS);
    assert!(RECONNECT_BACKOFF_MULTIPLIER > 1.0);
};

/// Compute the next reconnect delay using exponential backoff, clamped to the maximum.
fn compute_next_reconnect_delay(current_delay: i32) -> i32 {
    let new_delay = ((current_delay as f64) * RECONNECT_BACKOFF_MULTIPLIER) as i32;
    new_delay.min(MAX_RECONNECT_DELAY_MS)
}

impl ObsWsClient {
    pub fn new(config: &Config) -> Result<Self> {
        let server_uri = format!("ws://{}:{}", config.obs_ws_host, config.obs_ws_port);
        let password = config.obs_ws_pass.clone();
        let recording_mode = ObsRecordingMode::from_value(config.obs_ws_rec_mode)?;
        let shutdown_notify = Arc::new(Notify::new());

        let inner = Arc::new(Mutex::new(ObsWsClientInner {
            ws_sink: None,
            is_connected: false,
            is_identified: false,
            is_recording: false,
            restart_recording: false,
            auto_reconnect: true,
            is_reconnecting: false,
            is_shutting_down: false,
            save_requested: false,
            recording_mode,
            output_path: String::new(),
            last_output_path: String::new(),
            current_reconnect_delay: INITIAL_RECONNECT_DELAY_MS,
            server_uri: server_uri.clone(),
            password: password.clone(),
            shutdown_notify: Arc::clone(&shutdown_notify),
            on_close_handler: None,
            on_error_handler: None,
            on_version_received: None,
            on_scenes_received: None,
            on_record_state_changed: None,
            custom_message_handler: None,
        }));

        let runtime = tokio::runtime::Handle::current();

        Ok(Self {
            inner,
            password,
            request_id_counter: AtomicI64::new(0),
            server_uri,
            shutdown_notify,
            runtime,
        })
    }

    /// Connect asynchronously (non-blocking)
    pub fn connect_async(&self) {
        let inner = Arc::clone(&self.inner);
        let server_uri = self.server_uri.clone();
        let password = self.password.clone();
        let shutdown_notify = Arc::clone(&self.shutdown_notify);

        self.runtime.spawn(async move {
            match Self::do_connect(inner, &server_uri, &password, shutdown_notify).await {
                Ok(()) => {}
                Err(_e) => {
                    // Initial connection failed — reconnect will be scheduled from on_close
                }
            }
        });
    }

    /// Connect synchronously (blocking with timeout)
    pub fn connect(&self) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let server_uri = self.server_uri.clone();
        let password = self.password.clone();
        let shutdown_notify = Arc::clone(&self.shutdown_notify);

        self.runtime.block_on(async move {
            let result = tokio::time::timeout(
                Duration::from_secs(5),
                Self::do_connect(inner.clone(), &server_uri, &password, shutdown_notify),
            )
            .await;

            match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => {
                    let guard = lock_or_recover(&inner);
                    if guard.auto_reconnect {
                        drop(guard);
                        warn!("Initial connection failed: {}", e);
                        // schedule_reconnect would be called from on_close
                        Ok(())
                    } else {
                        Err(e)
                    }
                }
                Err(_) => Err(anyhow::anyhow!("Connection timeout")),
            }
        })
    }

    /// Internal async connection logic
    async fn do_connect(
        inner: Arc<Mutex<ObsWsClientInner>>,
        server_uri: &str,
        password: &str,
        shutdown_notify: Arc<Notify>,
    ) -> Result<()> {
        let (ws_stream, _) = connect_async(server_uri).await?;
        let (sink, mut stream) = ws_stream.split();

        // Store the sink
        {
            let mut guard = lock_or_recover(&inner);
            guard.ws_sink = Some(sink);
            guard.is_connected = true;
            guard.is_reconnecting = false;
            guard.current_reconnect_delay = INITIAL_RECONNECT_DELAY_MS;
        }

        let inner_clone = Arc::clone(&inner);
        let password = password.to_string();
        let shutdown_clone = Arc::clone(&shutdown_notify);

        // Spawn the message processing loop
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = stream.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                Self::on_message(&inner_clone, &password, &text).await;
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                Self::on_close(&inner_clone).await;
                                break;
                            }
                            Some(Err(e)) => {
                                let msg = e.to_string();
                                if !msg.is_empty() {
                                    warn!("OBS WebSocket error: {}", msg);
                                }
                                let handler = {
                                    let guard = lock_or_recover(&inner_clone);
                                    guard.on_error_handler.clone()
                                };
                                if let Some(handler) = handler {
                                    handler(msg);
                                }
                                Self::on_close(&inner_clone).await;
                                break;
                            }
                            _ => {
                                // Ignore other message types (Binary, Ping, Pong, Frame)
                            }
                        }
                    }
                    _ = shutdown_clone.notified() => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle incoming message
    async fn on_message(inner: &Arc<Mutex<ObsWsClientInner>>, password: &str, message: &str) {
        let result: std::result::Result<Value, _> = serde_json::from_str(message);
        let json = match result {
            Ok(json) => json,
            Err(_) => {
                warn!("Received malformed JSON: {}", message);
                return;
            }
        };

        if json.get("op").is_none() {
            warn!("Received malformed JSON (no op): {}", message);
            return;
        }

        let op = json["op"].as_i64().unwrap_or(-1);

        match op {
            0 => {
                // Hello
                Self::handle_hello(inner, password, &json).await;
            }
            2 => {
                // Identified
                {
                    let mut guard = lock_or_recover(inner);
                    guard.is_identified = true;
                }
                Self::send_request_inner(inner, "GetVersion").await;
                Self::send_request_inner(inner, "GetSceneList").await;
                Self::send_request_inner(inner, "GetRecordStatus").await;
            }
            5 => {
                // Event
                Self::handle_event(inner, &json).await;
            }
            7 => {
                // RequestResponse
                Self::handle_request_response(inner, &json);
            }
            _ => {}
        }

        // Custom message handler -- clone before calling to avoid re-entrancy deadlock
        let custom_handler = {
            let guard = lock_or_recover(inner);
            guard.custom_message_handler.clone()
        };
        if let Some(handler) = custom_handler {
            handler(message.to_string());
        }
    }

    /// Handle Hello message (op 0)
    async fn handle_hello(inner: &Arc<Mutex<ObsWsClientInner>>, password: &str, json: &Value) {
        let d = match json.get("d") {
            Some(d) => d,
            None => return,
        };

        let auth_required = d.get("authentication").is_some();

        let mut identify_data = serde_json::Map::new();
        identify_data.insert("rpcVersion".to_string(), json!(1));

        if auth_required {
            if password.is_empty() {
                warn!("Authentication required but no password provided");
                {
                    let mut guard = lock_or_recover(inner);
                    guard.auto_reconnect = false;
                }
                Self::do_close(inner).await;
                return;
            }

            let auth = match d.get("authentication") {
                Some(a) => a,
                None => return,
            };
            let challenge = auth["challenge"].as_str().unwrap_or("");
            let salt = auth["salt"].as_str().unwrap_or("");

            // SHA-256 hash: password + salt
            let mut digest = Sha256::new();
            digest.update(format!("{}{}", password, salt).as_bytes());
            let secret_hash = digest.finalize();
            let secret = BASE64_STANDARD.encode(secret_hash);

            // SHA-256 hash: secret + challenge
            let mut digest2 = Sha256::new();
            digest2.update(format!("{}{}", secret, challenge).as_bytes());
            let auth_hash = digest2.finalize();
            let auth_string = BASE64_STANDARD.encode(auth_hash);

            identify_data.insert("authentication".to_string(), json!(auth_string));
        }

        let identify = json!({
            "op": 1,
            "d": Value::Object(identify_data),
        });

        match serde_json::to_string(&identify) {
            Ok(msg) => {
                Self::send_raw(inner, &msg).await;
            }
            Err(e) => {
                warn!("Error sending Identify: {}", e);
            }
        }
    }

    /// Handle Event message (op 5)
    async fn handle_event(inner: &Arc<Mutex<ObsWsClientInner>>, json: &Value) {
        let d = match json.get("d") {
            Some(d) => d,
            None => return,
        };

        if d.get("eventType").is_none() || d.get("eventData").is_none() {
            return;
        }

        let event_type = d["eventType"].as_str().unwrap_or("");
        let event_data = &d["eventData"];

        match event_type {
            "ExitStarted" => {
                info!("OBS is shutting down");
                Self::do_close(inner).await;
            }
            "AuthenticationFailure" | "AuthenticationFailed" => {
                warn!("OBS authentication failed!");
                {
                    let mut guard = lock_or_recover(inner);
                    guard.auto_reconnect = false;
                }
                Self::do_close(inner).await;
            }
            "RecordStateChanged" => {
                if event_data.get("outputState").is_some() {
                    let output_state = event_data["outputState"].as_str().unwrap_or("");
                    let output_path_val = if event_data.get("outputPath").is_some() {
                        event_data["outputPath"].as_str().unwrap_or("").to_string()
                    } else {
                        String::new()
                    };

                    let mut notify_message = String::new();

                    match output_state {
                        "OBS_WEBSOCKET_OUTPUT_STOPPED" => {
                            let (should_restart, _recording_mode, path_to_delete) = {
                                let mut guard = lock_or_recover(inner);
                                guard.is_recording = false;
                                guard.output_path = output_path_val.clone();
                                let should_restart = guard.restart_recording;
                                let recording_mode = guard.recording_mode;
                                if should_restart {
                                    guard.restart_recording = false;
                                }
                                let path_to_delete = if should_restart
                                    && recording_mode != ObsRecordingMode::KeepAll
                                {
                                    Some(output_path_val.clone())
                                } else {
                                    None
                                };
                                guard.last_output_path = output_path_val;
                                (should_restart, recording_mode, path_to_delete)
                            };

                            notify_message = "Recording stopped".to_string();

                            if let Some(path) = path_to_delete {
                                tokio::spawn(async move {
                                    let p = Path::new(&path);
                                    if p.exists() && p.is_file() {
                                        let _ = tokio::fs::remove_file(p).await;
                                    }
                                });
                            }

                            if should_restart {
                                let inner_clone = Arc::clone(inner);
                                tokio::spawn(async move {
                                    sleep(Duration::from_millis(500)).await;
                                    Self::request_start_record_inner(&inner_clone).await;
                                });
                            }
                        }
                        "OBS_WEBSOCKET_OUTPUT_STARTED" => {
                            let (recording_mode, save_requested, last_output_path) = {
                                let mut guard = lock_or_recover(inner);
                                guard.is_recording = true;
                                guard.output_path = output_path_val;
                                let rm = guard.recording_mode;
                                let sr = guard.save_requested;
                                let lop = guard.last_output_path.clone();
                                (rm, sr, lop)
                            };

                            notify_message = "Recording started".to_string();

                            if recording_mode != ObsRecordingMode::KeepAll {
                                if save_requested {
                                    {
                                        let mut guard = lock_or_recover(inner);
                                        guard.save_requested = false;
                                    }
                                    notify_message += ", last recording saved";
                                } else if !last_output_path.is_empty() {
                                    let path_to_delete = last_output_path;
                                    tokio::spawn(async move {
                                        let p = Path::new(&path_to_delete);
                                        if p.exists() && p.is_file() {
                                            let _ = tokio::fs::remove_file(p).await;
                                        }
                                    });
                                    notify_message += ", last recording deleted";
                                }
                            }
                        }
                        _ => {}
                    }

                    if !notify_message.is_empty() {
                        ImGuiNotify::info(&format!("OBS: {}.", notify_message));
                    }

                    let record_handler = {
                        let guard = lock_or_recover(inner);
                        guard.on_record_state_changed.clone()
                    };
                    if let Some(handler) = record_handler {
                        handler(output_state.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle RequestResponse message (op 7)
    fn handle_request_response(inner: &Arc<Mutex<ObsWsClientInner>>, json: &Value) {
        let d = match json.get("d") {
            Some(d) => d,
            None => return,
        };

        if d.get("responseData").is_none() || d.get("requestType").is_none() {
            return;
        }

        let response_data = &d["responseData"];
        let request_type = d["requestType"].as_str().unwrap_or("");

        match request_type {
            "GetVersion" => {
                if response_data.get("obsVersion").is_some()
                    && response_data.get("obsWebSocketVersion").is_some()
                {
                    let obs_version = response_data["obsVersion"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let ws_version = response_data["obsWebSocketVersion"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();

                    let handler = {
                        let guard = lock_or_recover(inner);
                        guard.on_version_received.clone()
                    };
                    if let Some(handler) = handler {
                        handler(ObsVersionInfo::new(obs_version, ws_version));
                    }
                }
            }
            "GetSceneList" => {
                let scenes_node = &response_data["scenes"];
                let mut scene_names: Vec<String> = Vec::new();

                if let Some(scenes_array) = scenes_node.as_array() {
                    for scene_node in scenes_array {
                        if scene_node.get("sceneName").is_some()
                            && let Some(name) = scene_node["sceneName"].as_str()
                        {
                            scene_names.push(name.to_string());
                        }
                    }
                }

                scene_names.reverse();

                let handler = {
                    let guard = lock_or_recover(inner);
                    guard.on_scenes_received.clone()
                };
                if let Some(handler) = handler {
                    handler(scene_names);
                }
            }
            "GetRecordStatus" => {
                if response_data.get("outputActive").is_some() {
                    let output_active = response_data["outputActive"].as_bool().unwrap_or(false);
                    let mut guard = lock_or_recover(inner);
                    guard.is_recording = output_active;
                }
            }
            _ => {}
        }
    }

    /// Close from an inner context (used by event handlers)
    async fn do_close(inner: &Arc<Mutex<ObsWsClientInner>>) {
        {
            let mut guard = lock_or_recover(inner);
            guard.is_shutting_down = true;
            guard.auto_reconnect = false;
            guard.ws_sink = None;
            guard.is_connected = false;
            guard.is_identified = false;
        }
    }

    /// Handle connection close
    async fn on_close(inner: &Arc<Mutex<ObsWsClientInner>>) {
        let (was_connected, auto_reconnect, is_reconnecting, is_shutting_down) = {
            let mut guard = lock_or_recover(inner);
            let was_connected = guard.is_connected;
            guard.is_connected = false;
            guard.is_identified = false;
            guard.ws_sink = None;
            (
                was_connected,
                guard.auto_reconnect,
                guard.is_reconnecting,
                guard.is_shutting_down,
            )
        };

        let close_handler = {
            let guard = lock_or_recover(inner);
            guard.on_close_handler.clone()
        };
        if let Some(handler) = close_handler {
            handler();
        }

        if auto_reconnect && was_connected && !is_reconnecting && !is_shutting_down {
            Self::schedule_reconnect(inner);
        }
    }

    /// Schedule a reconnection attempt with exponential backoff.
    ///
    /// This is a sync function (not async) to break the async type cycle:
    /// do_connect → on_close → schedule_reconnect → do_connect.
    /// All async work is inside the tokio::spawn.
    fn schedule_reconnect(inner: &Arc<Mutex<ObsWsClientInner>>) {
        let (is_reconnecting, auto_reconnect, is_shutting_down, delay) = {
            let guard = lock_or_recover(inner);
            (
                guard.is_reconnecting,
                guard.auto_reconnect,
                guard.is_shutting_down,
                guard.current_reconnect_delay,
            )
        };

        if is_reconnecting || !auto_reconnect || is_shutting_down {
            return;
        }

        {
            let mut guard = lock_or_recover(inner);
            guard.is_reconnecting = true;
        }

        let inner_clone = Arc::clone(inner);
        tokio::spawn(async move {
            sleep(Duration::from_millis(delay as u64)).await;

            // Close existing connection
            {
                let mut guard = lock_or_recover(&inner_clone);
                guard.ws_sink = None;
            }

            // Update backoff delay for next attempt
            {
                let mut guard = lock_or_recover(&inner_clone);
                guard.current_reconnect_delay =
                    compute_next_reconnect_delay(guard.current_reconnect_delay);

                if guard.auto_reconnect && !guard.is_shutting_down {
                    guard.is_reconnecting = false;
                }
            }
            let (server_uri, password, shutdown_notify) = {
                let guard = lock_or_recover(&inner_clone);
                (
                    guard.server_uri.clone(),
                    guard.password.clone(),
                    guard.shutdown_notify.clone(),
                )
            };
            if let Err(e) =
                ObsWsClient::do_connect(inner_clone, &server_uri, &password, shutdown_notify).await
            {
                log::warn!("OBS WebSocket reconnection failed: {}", e);
            }
        });
    }

    /// Send a raw message through the WebSocket
    async fn send_raw(inner: &Arc<Mutex<ObsWsClientInner>>, message: &str) {
        let mut sink = {
            let mut guard = lock_or_recover(inner);
            guard.ws_sink.take()
        };

        if let Some(ref mut s) = sink {
            let _ = s.send(Message::Text(message.to_string())).await;
        }

        // Put sink back
        {
            let mut guard = lock_or_recover(inner);
            guard.ws_sink = sink;
        }
    }

    /// Check if requests can be sent
    fn can_send_request(inner: &Arc<Mutex<ObsWsClientInner>>) -> bool {
        let guard = lock_or_recover(inner);
        guard.is_connected && guard.is_identified && !guard.is_reconnecting
    }

    /// Internal send request (used by async contexts)
    async fn send_request_inner(inner: &Arc<Mutex<ObsWsClientInner>>, request_type: &str) {
        if !Self::can_send_request(inner) {
            return;
        }

        let request_id = format!("{}-{}", request_type.to_lowercase(), 0);

        let request = json!({
            "op": 6,
            "d": {
                "requestType": request_type,
                "requestId": request_id,
            }
        });

        match serde_json::to_string(&request) {
            Ok(msg) => {
                Self::send_raw(inner, &msg).await;
            }
            Err(e) => {
                warn!("Error sending request: {}", e);
            }
        }
    }

    /// Internal request start record
    async fn request_start_record_inner(inner: &Arc<Mutex<ObsWsClientInner>>) {
        let can_send = {
            let guard = lock_or_recover(inner);
            guard.is_connected
                && guard.is_identified
                && !guard.is_reconnecting
                && !guard.is_recording
        };
        if !can_send {
            return;
        }
        Self::send_request_inner(inner, "StartRecord").await;
    }

    // ---- Public API ----

    pub fn is_connected(&self) -> bool {
        let guard = lock_or_recover(&self.inner);
        guard.is_connected
    }

    pub fn is_identified(&self) -> bool {
        let guard = lock_or_recover(&self.inner);
        guard.is_identified
    }

    pub fn is_recording(&self) -> bool {
        let guard = lock_or_recover(&self.inner);
        guard.is_recording
    }

    pub fn request_start_record(&self) {
        let inner = Arc::clone(&self.inner);
        {
            let guard = lock_or_recover(&inner);
            if !(guard.is_connected && guard.is_identified && !guard.is_reconnecting)
                || guard.is_recording
            {
                return;
            }
        }
        let inner_clone = Arc::clone(&self.inner);
        self.runtime.spawn(async move {
            Self::send_request_inner(&inner_clone, "StartRecord").await;
        });
    }

    #[allow(clippy::nonminimal_bool)]
    pub fn request_stop_record(&self) {
        let inner = Arc::clone(&self.inner);
        {
            let guard = lock_or_recover(&inner);
            if !(guard.is_connected && guard.is_identified && !guard.is_reconnecting)
                || !guard.is_recording
            {
                return;
            }
        }
        let inner_clone = Arc::clone(&self.inner);
        self.runtime.spawn(async move {
            Self::send_request_inner(&inner_clone, "StopRecord").await;
        });
    }

    #[allow(clippy::overly_complex_bool_expr)]
    pub fn save_last_recording(&self, reason: &str) {
        let guard = lock_or_recover(&self.inner);
        // Java: if (!this.isConnected && !canSendRequest()) — faithfully translated
        if !guard.is_connected
            && !(guard.is_connected && guard.is_identified && !guard.is_reconnecting)
        {
            return;
        }

        let reason_mode = match reason {
            "ON_SCREENSHOT" => ObsRecordingMode::OnScreenshot,
            "ON_REPLAY" => ObsRecordingMode::OnReplay,
            "KEEP_ALL" => ObsRecordingMode::KeepAll,
            _ => return,
        };

        if guard.save_requested
            || guard.recording_mode == ObsRecordingMode::KeepAll
            || reason_mode != guard.recording_mode
        {
            return;
        }

        drop(guard);
        {
            let mut guard = lock_or_recover(&self.inner);
            guard.save_requested = true;
        }
        ImGuiNotify::info("OBS: Recording will be kept.");
    }

    pub fn set_scene(&self, scene_name: &str) {
        if !Self::can_send_request(&self.inner) {
            return;
        }

        let request_id_val = self.request_id_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let request = json!({
            "op": 6,
            "d": {
                "requestType": "SetCurrentProgramScene",
                "requestId": format!("set-scene-{}", request_id_val),
                "requestData": {
                    "sceneName": scene_name,
                },
            }
        });

        match serde_json::to_string(&request) {
            Ok(msg) => {
                let inner = Arc::clone(&self.inner);
                self.runtime.spawn(async move {
                    Self::send_raw(&inner, &msg).await;
                });
            }
            Err(e) => {
                warn!("Error setting scene: {}", e);
            }
        }
    }

    pub fn send_request(&self, request_type: &str) {
        if !Self::can_send_request(&self.inner) {
            return;
        }

        let request_id_val = self.request_id_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let request = json!({
            "op": 6,
            "d": {
                "requestType": request_type,
                "requestId": format!("{}-{}", request_type.to_lowercase(), request_id_val),
            }
        });

        match serde_json::to_string(&request) {
            Ok(msg) => {
                let inner = Arc::clone(&self.inner);
                self.runtime.spawn(async move {
                    Self::send_raw(&inner, &msg).await;
                });
            }
            Err(e) => {
                warn!("Error sending request: {}", e);
            }
        }
    }

    pub fn restart_recording(&self) {
        let mut guard = lock_or_recover(&self.inner);
        if !(guard.is_connected && guard.is_identified && !guard.is_reconnecting)
            || guard.restart_recording
        {
            return;
        }
        if !guard.is_recording {
            drop(guard);
            self.request_start_record();
            return;
        }
        guard.restart_recording = true;
        drop(guard);
        self.request_stop_record();
    }

    pub fn set_auto_reconnect(&self, enabled: bool) {
        let mut guard = lock_or_recover(&self.inner);
        guard.auto_reconnect = enabled;
    }

    pub fn close(&self) {
        {
            let mut guard = lock_or_recover(&self.inner);
            guard.is_shutting_down = true;
            guard.auto_reconnect = false;
            guard.ws_sink = None;
        }
        self.shutdown_notify.notify_waiters();
    }

    // ---- Callback setters ----

    pub fn set_on_close(&self, handler: impl Fn() + Send + Sync + 'static) {
        let mut guard = lock_or_recover(&self.inner);
        guard.on_close_handler = Some(Arc::new(handler));
    }

    pub fn set_on_error(&self, handler: impl Fn(String) + Send + Sync + 'static) {
        let mut guard = lock_or_recover(&self.inner);
        guard.on_error_handler = Some(Arc::new(handler));
    }

    pub fn set_on_version_received(
        &self,
        handler: impl Fn(ObsVersionInfo) + Send + Sync + 'static,
    ) {
        let mut guard = lock_or_recover(&self.inner);
        guard.on_version_received = Some(Arc::new(handler));
    }

    pub fn set_on_scenes_received(&self, handler: impl Fn(Vec<String>) + Send + Sync + 'static) {
        let mut guard = lock_or_recover(&self.inner);
        guard.on_scenes_received = Some(Arc::new(handler));
    }

    pub fn set_on_record_state_changed(&self, handler: impl Fn(String) + Send + Sync + 'static) {
        let mut guard = lock_or_recover(&self.inner);
        guard.on_record_state_changed = Some(Arc::new(handler));
    }

    pub fn set_custom_message_handler(&self, handler: impl Fn(String) + Send + Sync + 'static) {
        let mut guard = lock_or_recover(&self.inner);
        guard.custom_message_handler = Some(Arc::new(handler));
    }
}

impl rubato_types::obs_access::ObsAccess for ObsWsClient {
    fn save_last_recording(&self, reason: &str) {
        ObsWsClient::save_last_recording(self, reason);
    }

    fn is_connected(&self) -> bool {
        ObsWsClient::is_connected(self)
    }

    fn is_recording(&self) -> bool {
        ObsWsClient::is_recording(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- ObsRecordingMode --

    #[test]
    fn recording_mode_roundtrip() {
        for (val, expected) in [
            (0, ObsRecordingMode::KeepAll),
            (1, ObsRecordingMode::OnScreenshot),
            (2, ObsRecordingMode::OnReplay),
        ] {
            let mode = ObsRecordingMode::from_value(val).unwrap();
            assert_eq!(mode, expected);
            assert_eq!(mode.value(), val);
        }
    }

    #[test]
    fn recording_mode_invalid_value() {
        assert!(ObsRecordingMode::from_value(3).is_err());
        assert!(ObsRecordingMode::from_value(-1).is_err());
    }

    // -- ObsVersionInfo --

    #[test]
    fn version_info_display() {
        let info = ObsVersionInfo::new("30.0.0".to_string(), "5.3.0".to_string());
        assert_eq!(info.get_obs_version(), "30.0.0");
        assert_eq!(info.get_ws_version(), "5.3.0");
        assert_eq!(format!("{}", info), "OBS v30.0.0 (WS v5.3.0)");
    }

    // -- obs_actions / get_action_label --

    #[test]
    fn obs_actions_contains_expected_entries() {
        let actions = obs_actions();
        assert_eq!(actions.get("Stop Recording").unwrap(), "StopRecord");
        assert_eq!(actions.get("Start Recording").unwrap(), "StartRecord");
    }

    #[test]
    fn get_action_label_found() {
        assert_eq!(
            get_action_label("StopRecord"),
            Some("Stop Recording".to_string())
        );
        assert_eq!(
            get_action_label("StartRecord"),
            Some("Start Recording".to_string())
        );
    }

    #[test]
    fn get_action_label_not_found() {
        assert_eq!(get_action_label("NonExistent"), None);
    }

    // -- compute_next_reconnect_delay (exponential backoff) --

    #[test]
    fn backoff_from_initial_delay() {
        // 2000 * 1.25 = 2500
        let next = compute_next_reconnect_delay(INITIAL_RECONNECT_DELAY_MS);
        assert_eq!(next, 2500);
    }

    #[test]
    fn backoff_progression_sequence() {
        // Verify the full exponential backoff sequence from initial to max
        let mut delay = INITIAL_RECONNECT_DELAY_MS;
        let expected = [
            2500, 3125, 3906, 4882, 6102, 7627, 9533, 11916, 14895, 15000,
        ];
        for &exp in &expected {
            delay = compute_next_reconnect_delay(delay);
            assert_eq!(delay, exp, "backoff mismatch at expected value {}", exp);
        }
    }

    #[test]
    fn backoff_clamps_at_maximum() {
        // Starting at max should stay at max
        let next = compute_next_reconnect_delay(MAX_RECONNECT_DELAY_MS);
        assert_eq!(next, MAX_RECONNECT_DELAY_MS);
    }

    #[test]
    fn backoff_from_just_below_max() {
        // 14000 * 1.25 = 17500, clamped to 15000
        let next = compute_next_reconnect_delay(14000);
        assert_eq!(next, MAX_RECONNECT_DELAY_MS);
    }

    #[test]
    fn backoff_from_zero() {
        // 0 * 1.25 = 0 (edge case: should not go negative)
        let next = compute_next_reconnect_delay(0);
        assert_eq!(next, 0);
    }
}

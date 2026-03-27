use anyhow::{Result, bail};
use serde::Serialize;
use uuid::Uuid;

use super::connection::IPCConnection;

/// Translates: RichPresence.java
///
/// ```java
/// public class RichPresence {
///     private final String clientId;
///     private final IPCConnection connection;
///     private boolean connected = false;
/// ```
pub struct RichPresence {
    client_id: String,
    connection: Box<dyn IPCConnection>,
    connected: bool,
}

/// Translates:
/// ```java
/// private static IPCConnection createConnection() {
///     String os = System.getProperty("os.name").toLowerCase();
///     return os.startsWith("windows") ? new WindowsIPCConnection() : new UnixIPCConnection();
/// }
/// ```
fn create_connection() -> Box<dyn IPCConnection> {
    #[cfg(unix)]
    {
        Box::new(super::unix::UnixIPCConnection::new())
    }
    #[cfg(windows)]
    {
        Box::new(super::windows::WindowsIPCConnection::new())
    }
}

impl RichPresence {
    /// Translates:
    /// ```java
    /// public RichPresence(String clientId) {
    ///     this.clientId = clientId;
    ///     this.connection = createConnection();
    /// }
    /// ```
    pub fn new(client_id: String) -> Self {
        RichPresence {
            client_id,
            connection: create_connection(),
            connected: false,
        }
    }

    /// Create with a custom connection (for testing)
    pub fn with_connection(client_id: String, connection: Box<dyn IPCConnection>) -> Self {
        RichPresence {
            client_id,
            connection,
            connected: false,
        }
    }

    /// Translates:
    /// ```java
    /// public void connect() throws IOException {
    ///     connection.connect();
    ///     handshake();
    ///     connected = true;
    /// }
    /// ```
    pub fn connect(&mut self) -> Result<()> {
        self.connection.connect()?;
        self.handshake()?;
        self.connected = true;
        Ok(())
    }

    /// Translates:
    /// ```java
    /// private void handshake() throws IOException {
    ///     Map<String, Object> handshake = new HashMap<>();
    ///     handshake.put("v", 1);
    ///     handshake.put("client_id", clientId);
    ///
    ///     sendPacket(0, MAPPER.writeValueAsString(handshake));
    ///     byte[] response = receivePacket();
    /// }
    /// ```
    fn handshake(&mut self) -> Result<()> {
        let handshake = serde_json::json!({
            "v": 1,
            "client_id": self.client_id
        });

        self.send_packet(0, &serde_json::to_string(&handshake)?)?;
        let _response = self.receive_packet()?;
        Ok(())
    }

    /// Translates:
    /// ```java
    /// private void sendPacket(int opCode, String payload) throws IOException {
    ///     byte[] payloadBytes = payload.getBytes(StandardCharsets.UTF_8);
    ///     ByteBuffer buffer = ByteBuffer.allocate(8 + payloadBytes.length);
    ///     buffer.order(ByteOrder.LITTLE_ENDIAN);
    ///     buffer.putInt(opCode);
    ///     buffer.putInt(payloadBytes.length);
    ///     buffer.put(payloadBytes);
    ///     buffer.flip();
    ///     connection.write(buffer);
    /// }
    /// ```
    fn send_packet(&mut self, op_code: i32, payload: &str) -> Result<()> {
        let payload_bytes = payload.as_bytes();
        let mut buffer = Vec::with_capacity(8 + payload_bytes.len());
        buffer.extend_from_slice(&op_code.to_le_bytes());
        buffer.extend_from_slice(&(payload_bytes.len() as i32).to_le_bytes());
        buffer.extend_from_slice(payload_bytes);
        self.connection.write(&buffer)?;
        Ok(())
    }

    /// Translates:
    /// ```java
    /// private byte[] receivePacket() throws IOException {
    ///     ByteBuffer header = connection.read(8);
    ///     header.order(ByteOrder.LITTLE_ENDIAN);
    ///     int opCode = header.getInt();
    ///     int length = header.getInt();
    ///
    ///     ByteBuffer payload = connection.read(length);
    ///     return payload.array();
    /// }
    /// ```
    fn receive_packet(&mut self) -> Result<Vec<u8>> {
        let header = self.connection.read(8)?;
        let _op_code = i32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        let length = i32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        if !(0..=1024 * 1024).contains(&length) {
            anyhow::bail!("Discord IPC: invalid packet length {}", length);
        }

        let payload = self.connection.read(length as usize)?;
        Ok(payload)
    }

    /// Translates:
    /// ```java
    /// public void update(RichPresenceData data) throws IOException {
    ///     if (!connected) throw new IllegalStateException("Not connected to Discord");
    ///
    ///     ActivityPayload payload = new ActivityPayload();
    ///     payload.cmd = "SET_ACTIVITY";
    ///     payload.nonce = UUID.randomUUID().toString();
    ///     payload.args = new ActivityArgs();
    ///     payload.args.pid = ProcessHandle.current().pid();
    ///     payload.args.activity = data;
    ///
    ///     sendPacket(1, MAPPER.writeValueAsString(payload));
    ///     byte[] response = receivePacket();
    /// }
    /// ```
    pub fn update(&mut self, data: RichPresenceData) -> Result<()> {
        if !self.connected {
            bail!("Not connected to Discord");
        }

        let payload = ActivityPayload {
            cmd: "SET_ACTIVITY".to_string(),
            nonce: Uuid::new_v4().to_string(),
            args: ActivityArgs {
                pid: std::process::id() as i64,
                activity: data,
            },
        };

        self.send_packet(1, &serde_json::to_string(&payload)?)?;
        let _response = self.receive_packet()?;
        Ok(())
    }

    /// Translates:
    /// ```java
    /// public void close() {
    ///     connection.close();
    ///     connected = false;
    /// }
    /// ```
    pub fn close(&mut self) {
        self.connection.close();
        self.connected = false;
    }
}

// Data classes for serde serialization

/// Translates:
/// ```java
/// @JsonInclude(JsonInclude.Include.NON_NULL)
/// public static class RichPresenceData {
///     @JsonProperty("state") public String state;
///     @JsonProperty("details") public String details;
///     @JsonProperty("timestamps") public Timestamps timestamps;
///     @JsonProperty("assets") public Assets assets;
///     @JsonProperty("party") public Party party;
///     @JsonProperty("secrets") public Secrets secrets;
///     @JsonProperty("instance") public Boolean instance = true;
/// ```
#[derive(Serialize, Clone, Debug, Default)]
pub struct RichPresenceData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<Timestamps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Assets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<Party>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Secrets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<bool>,
}

impl RichPresenceData {
    pub fn new() -> Self {
        RichPresenceData {
            instance: Some(true),
            ..Default::default()
        }
    }

    /// Translates:
    /// ```java
    /// public RichPresenceData setState(String state) {
    ///     this.state = state;
    ///     return this;
    /// }
    /// ```
    pub fn set_state(mut self, state: String) -> Self {
        self.state = Some(state);
        self
    }

    /// Translates:
    /// ```java
    /// public RichPresenceData setDetails(String details) {
    ///     this.details = details;
    ///     return this;
    /// }
    /// ```
    pub fn set_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Translates:
    /// ```java
    /// public RichPresenceData setStartTimestamp(long start) {
    ///     if (timestamps == null) timestamps = new Timestamps();
    ///     timestamps.start = start;
    ///     return this;
    /// }
    /// ```
    pub fn set_start_timestamp(mut self, start: i64) -> Self {
        if self.timestamps.is_none() {
            self.timestamps = Some(Timestamps::default());
        }
        self.timestamps.as_mut().expect("timestamps is Some").start = Some(start);
        self
    }

    /// Translates:
    /// ```java
    /// public RichPresenceData setEndTimestamp(long end) {
    ///     if (timestamps == null) timestamps = new Timestamps();
    ///     timestamps.end = end;
    ///     return this;
    /// }
    /// ```
    pub fn set_end_timestamp(mut self, end: i64) -> Self {
        if self.timestamps.is_none() {
            self.timestamps = Some(Timestamps::default());
        }
        self.timestamps.as_mut().expect("timestamps is Some").end = Some(end);
        self
    }

    /// Translates:
    /// ```java
    /// public RichPresenceData setLargeImage(String key, String text) {
    ///     if (assets == null) assets = new Assets();
    ///     assets.largeImage = key;
    ///     assets.largeText = text;
    ///     return this;
    /// }
    /// ```
    pub fn set_large_image(mut self, key: String, text: String) -> Self {
        if self.assets.is_none() {
            self.assets = Some(Assets::default());
        }
        let assets = self.assets.as_mut().expect("assets is Some");
        assets.large_image = Some(key);
        assets.large_text = Some(text);
        self
    }

    /// Translates:
    /// ```java
    /// public RichPresenceData setSmallImage(String key, String text) {
    ///     if (assets == null) assets = new Assets();
    ///     assets.smallImage = key;
    ///     assets.smallText = text;
    ///     return this;
    /// }
    /// ```
    pub fn set_small_image(mut self, key: String, text: String) -> Self {
        if self.assets.is_none() {
            self.assets = Some(Assets::default());
        }
        let assets = self.assets.as_mut().expect("assets is Some");
        assets.small_image = Some(key);
        assets.small_text = Some(text);
        self
    }

    /// Translates:
    /// ```java
    /// public RichPresenceData setParty(String id, int size, int max) {
    ///     if (party == null) party = new Party();
    ///     party.id = id;
    ///     party.size = new int[]{size, max};
    ///     return this;
    /// }
    /// ```
    pub fn set_party(mut self, id: String, size: i32, max: i32) -> Self {
        if self.party.is_none() {
            self.party = Some(Party::default());
        }
        let party = self.party.as_mut().expect("party is Some");
        party.id = Some(id);
        party.size = Some(vec![size, max]);
        self
    }

    /// Translates:
    /// ```java
    /// public RichPresenceData setSecrets(String match, String join, String spectate) {
    ///     if (secrets == null) secrets = new Secrets();
    ///     secrets.match = match;
    ///     secrets.join = join;
    ///     secrets.spectate = spectate;
    ///     return this;
    /// }
    /// ```
    pub fn set_secrets(mut self, match_key: String, join: String, spectate: String) -> Self {
        if self.secrets.is_none() {
            self.secrets = Some(Secrets::default());
        }
        let secrets = self.secrets.as_mut().expect("secrets is Some");
        secrets.match_key = Some(match_key);
        secrets.join = Some(join);
        secrets.spectate = Some(spectate);
        self
    }
}

/// Translates:
/// ```java
/// @JsonInclude(JsonInclude.Include.NON_NULL)
/// public static class Timestamps {
///     @JsonProperty("start") public Long start;
///     @JsonProperty("end") public Long end;
/// }
/// ```
#[derive(Serialize, Clone, Debug, Default)]
pub struct Timestamps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
}

/// Translates:
/// ```java
/// @JsonInclude(JsonInclude.Include.NON_NULL)
/// public static class Assets {
///     @JsonProperty("large_image") public String largeImage;
///     @JsonProperty("large_text") public String largeText;
///     @JsonProperty("small_image") public String smallImage;
///     @JsonProperty("small_text") public String smallText;
/// }
/// ```
#[derive(Serialize, Clone, Debug, Default)]
pub struct Assets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

/// Translates:
/// ```java
/// @JsonInclude(JsonInclude.Include.NON_NULL)
/// public static class Party {
///     @JsonProperty("id") public String id;
///     @JsonProperty("size") public int[] size;
/// }
/// ```
#[derive(Serialize, Clone, Debug, Default)]
pub struct Party {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Vec<i32>>,
}

/// Translates:
/// ```java
/// @JsonInclude(JsonInclude.Include.NON_NULL)
/// public static class Secrets {
///     @JsonProperty("match") public String match;
///     @JsonProperty("join") public String join;
///     @JsonProperty("spectate") public String spectate;
/// }
/// ```
#[derive(Serialize, Clone, Debug, Default)]
pub struct Secrets {
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectate: Option<String>,
}

/// Translates:
/// ```java
/// private static class ActivityPayload {
///     @JsonProperty("cmd") public String cmd;
///     @JsonProperty("args") public ActivityArgs args;
///     @JsonProperty("nonce") public String nonce;
/// }
/// ```
#[derive(Serialize)]
struct ActivityPayload {
    cmd: String,
    args: ActivityArgs,
    nonce: String,
}

/// Translates:
/// ```java
/// private static class ActivityArgs {
///     @JsonProperty("pid") public long pid;
///     @JsonProperty("activity") public RichPresenceData activity;
/// }
/// ```
#[derive(Serialize)]
struct ActivityArgs {
    pid: i64,
    activity: RichPresenceData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rich_presence_data_new() {
        let data = RichPresenceData::new();
        assert!(data.state.is_none());
        assert!(data.details.is_none());
        assert!(data.timestamps.is_none());
        assert!(data.assets.is_none());
        assert!(data.party.is_none());
        assert!(data.secrets.is_none());
        assert_eq!(data.instance, Some(true));
    }

    #[test]
    fn test_rich_presence_data_builder() {
        let data = RichPresenceData::new()
            .set_state("Playing".to_string())
            .set_details("Song Title".to_string())
            .set_start_timestamp(1000)
            .set_end_timestamp(2000)
            .set_large_image("large_key".to_string(), "Large Text".to_string())
            .set_small_image("small_key".to_string(), "Small Text".to_string())
            .set_party("party_id".to_string(), 1, 4)
            .set_secrets(
                "match_secret".to_string(),
                "join_secret".to_string(),
                "spectate_secret".to_string(),
            );

        assert_eq!(data.state, Some("Playing".to_string()));
        assert_eq!(data.details, Some("Song Title".to_string()));
        assert_eq!(data.timestamps.as_ref().unwrap().start, Some(1000));
        assert_eq!(data.timestamps.as_ref().unwrap().end, Some(2000));
        assert_eq!(
            data.assets.as_ref().unwrap().large_image,
            Some("large_key".to_string())
        );
        assert_eq!(
            data.assets.as_ref().unwrap().large_text,
            Some("Large Text".to_string())
        );
        assert_eq!(
            data.assets.as_ref().unwrap().small_image,
            Some("small_key".to_string())
        );
        assert_eq!(
            data.assets.as_ref().unwrap().small_text,
            Some("Small Text".to_string())
        );
        assert_eq!(
            data.party.as_ref().unwrap().id,
            Some("party_id".to_string())
        );
        assert_eq!(data.party.as_ref().unwrap().size, Some(vec![1, 4]));
        assert_eq!(
            data.secrets.as_ref().unwrap().match_key,
            Some("match_secret".to_string())
        );
        assert_eq!(
            data.secrets.as_ref().unwrap().join,
            Some("join_secret".to_string())
        );
        assert_eq!(
            data.secrets.as_ref().unwrap().spectate,
            Some("spectate_secret".to_string())
        );
    }

    #[test]
    fn test_rich_presence_data_serialization_null_fields_omitted() {
        let data = RichPresenceData::new().set_state("Playing".to_string());

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"state\":\"Playing\""));
        assert!(json.contains("\"instance\":true"));
        // Null fields should not appear
        assert!(!json.contains("\"details\""));
        assert!(!json.contains("\"timestamps\""));
        assert!(!json.contains("\"assets\""));
        assert!(!json.contains("\"party\""));
        assert!(!json.contains("\"secrets\""));
    }

    #[test]
    fn test_secrets_match_renamed_in_json() {
        let secrets = Secrets {
            match_key: Some("secret".to_string()),
            join: None,
            spectate: None,
        };

        let json = serde_json::to_string(&secrets).unwrap();
        // "match_key" in Rust should serialize as "match" in JSON
        assert!(json.contains("\"match\":\"secret\""));
        assert!(!json.contains("\"match_key\""));
    }

    #[test]
    fn test_timestamps_serialization() {
        let ts = Timestamps {
            start: Some(1000),
            end: None,
        };

        let json = serde_json::to_string(&ts).unwrap();
        assert!(json.contains("\"start\":1000"));
        assert!(!json.contains("\"end\""));
    }

    #[test]
    fn test_assets_serialization() {
        let assets = Assets {
            large_image: Some("img_key".to_string()),
            large_text: Some("Image Text".to_string()),
            small_image: None,
            small_text: None,
        };

        let json = serde_json::to_string(&assets).unwrap();
        assert!(json.contains("\"large_image\":\"img_key\""));
        assert!(json.contains("\"large_text\":\"Image Text\""));
        assert!(!json.contains("\"small_image\""));
        assert!(!json.contains("\"small_text\""));
    }

    #[test]
    fn test_party_serialization() {
        let party = Party {
            id: Some("party123".to_string()),
            size: Some(vec![2, 5]),
        };

        let json = serde_json::to_string(&party).unwrap();
        assert!(json.contains("\"id\":\"party123\""));
        assert!(json.contains("\"size\":[2,5]"));
    }

    #[test]
    fn test_send_packet_format() {
        // Verify the packet format: [op_code: i32 LE][length: i32 LE][payload bytes]
        let op_code: i32 = 0;
        let payload = r#"{"v":1,"client_id":"12345"}"#;
        let payload_bytes = payload.as_bytes();

        let mut buffer = Vec::with_capacity(8 + payload_bytes.len());
        buffer.extend_from_slice(&op_code.to_le_bytes());
        buffer.extend_from_slice(&(payload_bytes.len() as i32).to_le_bytes());
        buffer.extend_from_slice(payload_bytes);

        // Verify header
        assert_eq!(
            i32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]),
            0
        );
        assert_eq!(
            i32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]),
            payload_bytes.len() as i32
        );
        // Verify payload
        assert_eq!(&buffer[8..], payload_bytes);
    }

    #[test]
    fn test_update_without_connect_fails() {
        use anyhow::Result;

        struct MockConnection;

        impl IPCConnection for MockConnection {
            fn connect(&mut self) -> Result<()> {
                Ok(())
            }
            fn write(&mut self, _buffer: &[u8]) -> Result<()> {
                Ok(())
            }
            fn read(&mut self, size: usize) -> Result<Vec<u8>> {
                Ok(vec![0u8; size])
            }
            fn close(&mut self) {}
        }

        let mut rp =
            RichPresence::with_connection("test_client_id".to_string(), Box::new(MockConnection));
        let data = RichPresenceData::new().set_state("Playing".to_string());

        // Should fail because we haven't connected
        let result = rp.update(data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not connected"));
    }

    #[test]
    fn test_connect_and_update_with_mock() {
        use anyhow::Result;

        struct MockConnection {
            written: Vec<Vec<u8>>,
        }

        impl IPCConnection for MockConnection {
            fn connect(&mut self) -> Result<()> {
                Ok(())
            }
            fn write(&mut self, buffer: &[u8]) -> Result<()> {
                self.written.push(buffer.to_vec());
                Ok(())
            }
            fn read(&mut self, size: usize) -> Result<Vec<u8>> {
                // Return a valid header (op_code=1, length=2) + payload
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

        let mock = MockConnection {
            written: Vec::new(),
        };

        let mut rp = RichPresence::with_connection("test_client_id".to_string(), Box::new(mock));

        // Connect (includes handshake)
        rp.connect().unwrap();

        // Update
        let data = RichPresenceData::new()
            .set_state("Playing".to_string())
            .set_details("Test Song".to_string());

        rp.update(data).unwrap();

        // Close
        rp.close();
    }

    #[test]
    fn test_activity_payload_serialization() {
        let payload = ActivityPayload {
            cmd: "SET_ACTIVITY".to_string(),
            nonce: "test-nonce-123".to_string(),
            args: ActivityArgs {
                pid: 12345,
                activity: RichPresenceData::new()
                    .set_state("Playing".to_string())
                    .set_details("Test".to_string()),
            },
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"cmd\":\"SET_ACTIVITY\""));
        assert!(json.contains("\"nonce\":\"test-nonce-123\""));
        assert!(json.contains("\"pid\":12345"));
        assert!(json.contains("\"state\":\"Playing\""));
        assert!(json.contains("\"details\":\"Test\""));
    }
}

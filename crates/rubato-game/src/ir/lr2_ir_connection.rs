use std::collections::HashMap;
use std::io::Write;
use std::sync::{Mutex, MutexGuard, OnceLock};

use log::error;
use serde::Deserialize;

use crate::core::score_data::ScoreData;
use bms::model::mode::Mode;

use crate::ir::ir_chart_data::IRChartData;
use crate::ir::ir_score_data::IRScoreData;
use crate::ir::leaderboard_entry::LeaderboardEntry;
use crate::ir::lr2_ghost_data::LR2GhostData;
use rubato_types::imgui_notify::ImGuiNotify;

/// LR2 IR connection
///
/// Translated from: LR2IRConnection.java
///
/// Original repo from https://github.com/SayakaIsBaka/lr2ir-read-only
///
/// This class is not a real IR connection, but the original repo is. It keeps the
/// original form to make things easier.
// Accepted trade-off: plain HTTP, matching the Java original. The LR2IR server
// does not support HTTPS. Credentials (IR account ID/password) are transmitted
// unencrypted. Users should be aware of this limitation.
static IR_URL: &str = "http://dream-pro.info/~lavalse/LR2IR/2";

/// Maximum allowed HTTP response size (64 MB).
/// Prevents memory exhaustion from malicious or misconfigured servers.
const MAX_RESPONSE_SIZE: u64 = 64 * 1024 * 1024;

/// Maximum number of cached ranking entries before the cache is cleared.
/// A typical game session queries fewer than 100 unique songs, so 256 provides
/// ample headroom while preventing unbounded growth in long-running sessions.
const RANKING_CACHE_MAX_ENTRIES: usize = 256;

/// Shared HTTP client for LR2IR requests.
///
/// `reqwest::blocking::Client` maintains an internal connection pool, so reusing
/// a single instance avoids the overhead of TLS/TCP setup on every call.
/// Per-request timeouts are set at the call site via `RequestBuilder::timeout()`.
static HTTP_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

/// Get or initialize the shared blocking HTTP client.
fn http_client() -> &'static reqwest::blocking::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
}

lazy_static::lazy_static! {
    static ref LR2_IR_RANKING_CACHE: Mutex<HashMap<String, Vec<LeaderboardEntry>>> = Mutex::new(HashMap::new());
    static ref SCORE_DATABASE_ACCESSOR: Mutex<Option<Box<dyn ScoreDatabaseAccess>>> = Mutex::new(None);
}

/// Trait for score database access (avoids direct dependency on ScoreDatabaseAccessor)
pub trait ScoreDatabaseAccess: Send + Sync {
    fn score_data(&self, sha256: &str, mode: i32) -> Option<ScoreData>;
}

/// A `Write` adapter that caps buffered data at a fixed size.
/// Returns `WriteZero` when the accumulated bytes would exceed the limit,
/// causing `Response::copy_to()` to abort the transfer early.
struct LimitedWriter {
    buf: Vec<u8>,
    limit: usize,
}

impl Write for LimitedWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.buf.len() + data.len() > self.limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "response exceeded size limit",
            ));
        }
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Read response body with streaming size enforcement.
///
/// Unlike `response.bytes()`, this rejects oversized responses during streaming,
/// preventing memory exhaustion from chunked responses that omit Content-Length.
fn read_response_bytes_limited(
    mut response: reqwest::blocking::Response,
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    if let Some(content_length) = response.content_length()
        && content_length > max_bytes
    {
        return Err(format!("response too large ({} bytes)", content_length));
    }

    let mut writer = LimitedWriter {
        buf: Vec::new(),
        limit: max_bytes as usize,
    };

    match response.copy_to(&mut writer) {
        Ok(_) => Ok(writer.buf),
        Err(_) if writer.buf.len() >= writer.limit => {
            Err(format!("response too large (>{} bytes)", max_bytes))
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Acquire a mutex lock, recovering from poison if a thread panicked while holding it.
fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct LR2IRConnection;

impl LR2IRConnection {
    pub fn set_score_database_accessor(accessor: Box<dyn ScoreDatabaseAccess>) {
        let mut guard = lock_or_recover(&SCORE_DATABASE_ACCESSOR);
        *guard = Some(accessor);
    }

    fn convert_xml_to_ranking(xml: &str) -> Option<Ranking> {
        // In Java, this uses Jackson XmlMapper with case-insensitive properties.
        // In Rust, we use quick-xml + serde for XML deserialization.
        // For now, use a simplified parsing approach.
        match quick_xml::de::from_str::<Ranking>(xml) {
            Ok(ranking) => Some(ranking),
            Err(e) => {
                error!("XML parse error: {}", e);
                None
            }
        }
    }

    /// Send a blocking HTTP POST to the LR2IR server.
    ///
    /// Uses `reqwest::blocking::Client` with a 10-second timeout. This is
    /// intentionally blocking because ALL call sites already run on background
    /// threads (see `music_result::std::thread::spawn`, `ir_resend` thread,
    /// and `select::trait_impls` spawn). Must NOT be called from the
    /// main/render thread.
    fn make_post_request(uri: &str, data: &str) -> Option<String> {
        let url = format!("{}{}", IR_URL, uri);
        let client = http_client();
        match client
            .post(&url)
            .timeout(std::time::Duration::from_secs(10))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Connection", "close")
            .body(data.to_string())
            .send()
        {
            Ok(response) => {
                if response.status() != reqwest::StatusCode::OK {
                    ImGuiNotify::error(&format!(
                        "Failed to send request to LR2IR: HTTP error code: {}",
                        response.status()
                    ));
                    return None;
                }
                // Enforce size limit during streaming to protect against
                // chunked responses that omit Content-Length.
                match read_response_bytes_limited(response, MAX_RESPONSE_SIZE) {
                    Ok(bytes) => {
                        // In Java, response is read with Shift_JIS encoding.
                        // reqwest returns bytes; we decode with encoding_rs.
                        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                        Some(decoded.to_string())
                    }
                    Err(e) => {
                        ImGuiNotify::error(&format!("Failed to send request to LR2IR: {}", e));
                        None
                    }
                }
            }
            Err(e) => {
                ImGuiNotify::error(&format!("Failed to send request to LR2IR: {}", e));
                None
            }
        }
    }

    /// Get LR2IR scores and personal score.
    ///
    /// Returns `Some((local_score, leaderboard_entries))` on success,
    /// or `None` on fetch failure (network error, XML parse error, empty md5).
    /// The local score inside the tuple can itself be `None` when the player
    /// has no recorded score for this chart.
    pub fn score_data(
        chart: &IRChartData,
        player_id: &str,
    ) -> Option<(Option<IRScoreData>, Vec<LeaderboardEntry>)> {
        if chart.md5.is_empty() {
            return None;
        }
        let request_url = format!(
            "songmd5={}&id={}&lastupdate=",
            urlencoding::encode(&chart.md5),
            urlencoding::encode(player_id),
        );
        // Use songmd5 alone as cache key so that varying `lastupdate` or `id`
        // parameters don't create duplicate entries for the same chart.
        let cache_key = chart.md5.clone();

        let score_data = {
            let cache = lock_or_recover(&LR2_IR_RANKING_CACHE);
            cache.get(&cache_key).cloned()
        };

        let score_data = match score_data {
            Some(cached) => cached,
            None => {
                match Self::make_post_request("/getrankingxml.cgi", &request_url) {
                    Some(res) => {
                        // Java: res.substring(1).replace("<lastupdate></lastupdate>", "")
                        // Skip the first character safely (may be multi-byte after Shift_JIS decode)
                        let xml = {
                            let start = res.char_indices().nth(1).map_or(res.len(), |(i, _)| i);
                            res[start..].replace("<lastupdate></lastupdate>", "")
                        };
                        match Self::convert_xml_to_ranking(&xml) {
                            Some(ranking) => {
                                let entries = ranking.to_rubato_score_data(chart);
                                let mut cache = lock_or_recover(&LR2_IR_RANKING_CACHE);
                                // Evict an arbitrary quarter of entries at capacity to avoid
                                // thundering-herd re-fetches from a full clear.
                                if cache.len() >= RANKING_CACHE_MAX_ENTRIES {
                                    let to_remove = cache.len() / 4;
                                    let keys: Vec<String> =
                                        cache.keys().take(to_remove.max(1)).cloned().collect();
                                    for key in keys {
                                        cache.remove(&key);
                                    }
                                }
                                cache.insert(cache_key, entries.clone());
                                entries
                            }
                            None => {
                                ImGuiNotify::error(
                                    "Failed to get score data from LR2IR: XML parse error",
                                );
                                return None;
                            }
                        }
                    }
                    None => {
                        return None;
                    }
                }
            }
        };

        // Get local score
        let local_score = {
            let accessor = lock_or_recover(&SCORE_DATABASE_ACCESSOR);
            if let Some(ref acc) = *accessor {
                let lntype = if chart.has_undefined_ln {
                    chart.lntype
                } else {
                    0
                };
                acc.score_data(&chart.sha256, lntype).map(|mut s| {
                    // This is intentional behavior, see IRScoreData's player definition
                    // and how we use this feature in LeaderBoardBar
                    s.player = String::new();
                    IRScoreData::new(&s)
                })
            } else {
                None
            }
        };

        Some((local_score, score_data))
    }

    /// Fetch ghost replay data from LR2IR (blocking HTTP GET, 5-second timeout).
    ///
    /// Same threading contract as `make_post_request`: must only be called
    /// from a background thread, never from the main/render thread.
    pub fn ghost_data(md5: &str, score_id: i64) -> Option<LR2GhostData> {
        let api = format!(
            "/getghost.cgi?songmd5={}&mode=top&targetid={}",
            urlencoding::encode(md5),
            score_id // i64, no encoding needed
        );
        let url = format!("{}{}", IR_URL, api);
        let client = http_client();
        match client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
        {
            Ok(response) => {
                let status = response.status();
                if status != reqwest::StatusCode::OK {
                    error!("Unexpected http response code: {}", status);
                    ImGuiNotify::error("Failed to load ghost data.");
                    return None;
                }
                // Enforce size limit during streaming to protect against
                // chunked responses that omit Content-Length.
                match read_response_bytes_limited(response, MAX_RESPONSE_SIZE) {
                    Ok(bytes) => {
                        // LR2IR sends Shift_JIS responses (ghost CSV can contain
                        // Japanese player names). Decode with encoding_rs.
                        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                        LR2GhostData::parse(&decoded)
                    }
                    Err(e) => {
                        error!("{}", e);
                        ImGuiNotify::error("Failed to load ghost data.");
                        None
                    }
                }
            }
            Err(e) => {
                error!("{}", e);
                ImGuiNotify::error("Failed to load ghost data.");
                None
            }
        }
    }
}

/// LR2 IR song data request parameters
pub struct LR2IRSongData {
    pub md5: String,
    pub id: String,
    pub last_update: String,
}

impl LR2IRSongData {
    pub fn new(md5: String, id: String) -> Self {
        Self {
            md5,
            id,
            last_update: String::new(),
        }
    }

    pub fn to_url_encoded_form(&self) -> String {
        format!(
            "songmd5={}&id={}&lastupdate={}",
            urlencoding::encode(&self.md5),
            urlencoding::encode(&self.id),
            urlencoding::encode(&self.last_update),
        )
    }
}

/// Ranking XML response
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Ranking {
    #[serde(default)]
    pub score: Vec<Score>,
}

impl Ranking {
    pub fn to_rubato_score_data(&self, model: &IRChartData) -> Vec<LeaderboardEntry> {
        let mut res: Vec<LeaderboardEntry> = Vec::new();
        for s in &self.score {
            let mode = model.mode.unwrap_or(Mode::BEAT_7K);
            let mut tmp = ScoreData::new(mode);
            tmp.sha256 = model.sha256.clone();
            tmp.player = s.name.clone().unwrap_or_default();
            tmp.clear = s.rubato_clear();
            tmp.notes = s.notes;
            tmp.maxcombo = s.combo;
            tmp.judge_counts.epg = s.pg;
            tmp.judge_counts.egr = s.gr;
            tmp.minbp = s.minbp;
            res.push(LeaderboardEntry::new_entry_lr2_ir(
                IRScoreData::new(&tmp),
                s.id as i64,
            ));
        }

        res.sort_by_key(|b| std::cmp::Reverse(b.ir_score().exscore()));
        res
    }
}

/// Score entry from LR2IR XML
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Score {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub clear: i32,
    #[serde(default)]
    pub notes: i32,
    #[serde(default)]
    pub combo: i32,
    #[serde(default)]
    pub pg: i32,
    #[serde(default)]
    pub gr: i32,
    #[serde(default)]
    pub minbp: i32,
}

impl Score {
    pub fn rubato_clear(&self) -> i32 {
        match self.clear {
            1 => 1, // Failed
            2 => 4, // Easy
            3 => 5, // Groove
            4 => 6, // Hard
            5 => {
                // FC
                if self.pg + self.gr == self.notes {
                    9 // Perfect
                } else {
                    8
                }
            }
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- LimitedWriter tests ---

    #[test]
    fn limited_writer_accepts_data_within_limit() {
        let mut w = LimitedWriter {
            buf: Vec::new(),
            limit: 10,
        };
        assert!(w.write_all(b"hello").is_ok());
        assert_eq!(w.buf, b"hello");
    }

    #[test]
    fn limited_writer_accepts_data_at_exact_limit() {
        let mut w = LimitedWriter {
            buf: Vec::new(),
            limit: 5,
        };
        assert!(w.write_all(b"12345").is_ok());
        assert_eq!(w.buf.len(), 5);
    }

    #[test]
    fn limited_writer_rejects_data_exceeding_limit() {
        let mut w = LimitedWriter {
            buf: Vec::new(),
            limit: 5,
        };
        assert!(w.write_all(b"123").is_ok());
        let err = w.write(b"456").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::WriteZero);
        // Buffer should not have been extended past the rejection point.
        assert_eq!(w.buf, b"123");
    }

    #[test]
    fn limited_writer_rejects_single_write_exceeding_limit() {
        let mut w = LimitedWriter {
            buf: Vec::new(),
            limit: 3,
        };
        let err = w.write(b"abcdef").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::WriteZero);
        assert!(w.buf.is_empty());
    }

    // --- Score.get_rubato_clear tests ---

    #[test]
    fn test_score_clear_failed() {
        let s = Score {
            clear: 1,
            ..Default::default()
        };
        assert_eq!(s.rubato_clear(), 1);
    }

    #[test]
    fn test_score_clear_easy() {
        let s = Score {
            clear: 2,
            ..Default::default()
        };
        assert_eq!(s.rubato_clear(), 4);
    }

    #[test]
    fn test_score_clear_groove() {
        let s = Score {
            clear: 3,
            ..Default::default()
        };
        assert_eq!(s.rubato_clear(), 5);
    }

    #[test]
    fn test_score_clear_hard() {
        let s = Score {
            clear: 4,
            ..Default::default()
        };
        assert_eq!(s.rubato_clear(), 6);
    }

    #[test]
    fn test_score_clear_fc_perfect() {
        // FC with pg + gr == notes -> Perfect (9)
        let s = Score {
            clear: 5,
            pg: 200,
            gr: 100,
            notes: 300,
            ..Default::default()
        };
        assert_eq!(s.rubato_clear(), 9);
    }

    #[test]
    fn test_score_clear_fc_not_perfect() {
        // FC with pg + gr != notes -> FullCombo (8)
        let s = Score {
            clear: 5,
            pg: 200,
            gr: 50,
            notes: 300,
            ..Default::default()
        };
        assert_eq!(s.rubato_clear(), 8);
    }

    #[test]
    fn test_score_clear_unknown_returns_zero() {
        let s = Score {
            clear: 0,
            ..Default::default()
        };
        assert_eq!(s.rubato_clear(), 0);
        let s = Score {
            clear: 99,
            ..Default::default()
        };
        assert_eq!(s.rubato_clear(), 0);
    }

    // --- LR2IRSongData tests ---

    #[test]
    fn test_lr2_ir_song_data_url_encoded_form() {
        let data = LR2IRSongData::new("abc123".to_string(), "114328".to_string());
        let form = data.to_url_encoded_form();
        assert_eq!(form, "songmd5=abc123&id=114328&lastupdate=");
    }

    #[test]
    fn test_lr2_ir_song_data_with_last_update() {
        let mut data = LR2IRSongData::new("md5hash".to_string(), "999".to_string());
        data.last_update = "2024-01-01".to_string();
        let form = data.to_url_encoded_form();
        assert_eq!(form, "songmd5=md5hash&id=999&lastupdate=2024-01-01");
    }

    // --- Ranking XML parsing tests ---

    #[test]
    fn test_convert_xml_to_ranking_valid() {
        let xml = r#"<ranking><score><name>Player1</name><id>100</id><clear>3</clear><notes>500</notes><combo>450</combo><pg>200</pg><gr>100</gr><minbp>5</minbp></score></ranking>"#;
        let ranking = LR2IRConnection::convert_xml_to_ranking(xml);
        assert!(ranking.is_some());
        let ranking = ranking.unwrap();
        assert_eq!(ranking.score.len(), 1);
        assert_eq!(ranking.score[0].name, Some("Player1".to_string()));
        assert_eq!(ranking.score[0].id, 100);
        assert_eq!(ranking.score[0].clear, 3);
        assert_eq!(ranking.score[0].notes, 500);
        assert_eq!(ranking.score[0].combo, 450);
        assert_eq!(ranking.score[0].pg, 200);
        assert_eq!(ranking.score[0].gr, 100);
        assert_eq!(ranking.score[0].minbp, 5);
    }

    #[test]
    fn test_convert_xml_to_ranking_multiple_scores() {
        let xml = r#"<ranking><score><name>A</name><pg>100</pg><gr>50</gr></score><score><name>B</name><pg>80</pg><gr>60</gr></score></ranking>"#;
        let ranking = LR2IRConnection::convert_xml_to_ranking(xml).unwrap();
        assert_eq!(ranking.score.len(), 2);
        assert_eq!(ranking.score[0].name, Some("A".to_string()));
        assert_eq!(ranking.score[1].name, Some("B".to_string()));
    }

    #[test]
    fn test_convert_xml_to_ranking_empty() {
        let xml = r#"<ranking></ranking>"#;
        let ranking = LR2IRConnection::convert_xml_to_ranking(xml).unwrap();
        assert!(ranking.score.is_empty());
    }

    #[test]
    fn test_convert_xml_to_ranking_invalid_xml() {
        let xml = "not xml at all";
        let ranking = LR2IRConnection::convert_xml_to_ranking(xml);
        assert!(ranking.is_none());
    }

    // --- Ranking.to_rubato_score_data tests ---

    #[test]
    fn test_ranking_to_rubato_score_data_sorted_by_exscore() {
        let ranking = Ranking {
            score: vec![
                Score {
                    name: Some("Low".to_string()),
                    pg: 50,
                    gr: 30,
                    notes: 300,
                    ..Default::default()
                },
                Score {
                    name: Some("High".to_string()),
                    pg: 200,
                    gr: 100,
                    notes: 500,
                    ..Default::default()
                },
            ],
        };
        let chart = IRChartData {
            md5: "test_md5".to_string(),
            sha256: "test_sha256".to_string(),
            title: String::new(),
            subtitle: String::new(),
            genre: String::new(),
            artist: String::new(),
            subartist: String::new(),
            url: String::new(),
            appendurl: String::new(),
            level: 0,
            total: 0,
            mode: Some(Mode::BEAT_7K),
            lntype: 0,
            judge: 0,
            minbpm: 0,
            maxbpm: 0,
            notes: 0,
            has_undefined_ln: false,
            has_ln: false,
            has_cn: false,
            has_hcn: false,
            has_mine: false,
            has_random: false,
            has_stop: false,
            values: std::collections::HashMap::new(),
        };

        let entries = ranking.to_rubato_score_data(&chart);
        assert_eq!(entries.len(), 2);
        // Higher exscore should be first
        assert!(entries[0].ir_score().exscore() >= entries[1].ir_score().exscore());
        assert_eq!(entries[0].ir_score().player, "High");
        assert_eq!(entries[1].ir_score().player, "Low");
    }

    // --- LR2IRConnection.score_data empty md5 test ---

    #[test]
    fn test_get_score_data_empty_md5_returns_empty() {
        let chart = IRChartData {
            md5: String::new(),
            sha256: "sha".to_string(),
            title: String::new(),
            subtitle: String::new(),
            genre: String::new(),
            artist: String::new(),
            subartist: String::new(),
            url: String::new(),
            appendurl: String::new(),
            level: 0,
            total: 0,
            mode: None,
            lntype: 0,
            judge: 0,
            minbpm: 0,
            maxbpm: 0,
            notes: 0,
            has_undefined_ln: false,
            has_ln: false,
            has_cn: false,
            has_hcn: false,
            has_mine: false,
            has_random: false,
            has_stop: false,
            values: std::collections::HashMap::new(),
        };
        let result = LR2IRConnection::score_data(&chart, "0");
        assert!(result.is_none());
    }

    // --- Ranking cache eviction tests ---
    //
    // Combined into a single test because LR2_IR_RANKING_CACHE is a global
    // static shared across all test threads.

    #[test]
    fn test_ranking_cache_eviction() {
        // Verify the constant is a reasonable value
        assert_eq!(RANKING_CACHE_MAX_ENTRIES, 256);

        // Hold the cache lock for the entire test to avoid interference
        // from concurrent tests that also touch the global cache.
        let mut cache = lock_or_recover(&LR2_IR_RANKING_CACHE);
        cache.clear();

        // --- Below capacity: no eviction ---
        for i in 0..10 {
            cache.insert(format!("below_{}", i), Vec::new());
        }
        assert_eq!(cache.len(), 10);

        // One more insert should NOT trigger eviction
        // (simulate the eviction check from score_data / insert_ranking_cache)
        if cache.len() >= RANKING_CACHE_MAX_ENTRIES {
            cache.clear();
        }
        cache.insert("below_10".to_string(), Vec::new());
        assert_eq!(cache.len(), 11);

        // --- At capacity: partial eviction triggers ---
        cache.clear();
        for i in 0..RANKING_CACHE_MAX_ENTRIES {
            cache.insert(format!("key_{}", i), Vec::new());
        }
        assert_eq!(cache.len(), RANKING_CACHE_MAX_ENTRIES);

        // Next insert should trigger partial eviction (remove ~25% of entries)
        if cache.len() >= RANKING_CACHE_MAX_ENTRIES {
            let to_remove = cache.len() / 4;
            let keys: Vec<String> = cache.keys().take(to_remove.max(1)).cloned().collect();
            for key in keys {
                cache.remove(&key);
            }
        }
        cache.insert("overflow_key".to_string(), Vec::new());
        // After removing 64 entries (256/4) and inserting 1: 256 - 64 + 1 = 193
        assert_eq!(
            cache.len(),
            RANKING_CACHE_MAX_ENTRIES - RANKING_CACHE_MAX_ENTRIES / 4 + 1
        );
        assert!(cache.contains_key("overflow_key"));

        // Clean up
        cache.clear();
    }

    // --- lock_or_recover tests ---

    #[test]
    fn test_lock_or_recover_healthy_mutex() {
        let mutex = Mutex::new(42);
        let guard = lock_or_recover(&mutex);
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_lock_or_recover_poisoned_mutex() {
        let mutex = Mutex::new(42);
        // Poison the mutex by panicking while holding the lock.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = mutex.lock().expect("lock");
            panic!("intentional poison");
        }));
        assert!(mutex.is_poisoned());

        // lock_or_recover should recover instead of panicking.
        let guard = lock_or_recover(&mutex);
        assert_eq!(*guard, 42);
    }
}

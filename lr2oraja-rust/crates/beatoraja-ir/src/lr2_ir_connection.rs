use std::collections::HashMap;
use std::sync::Mutex;

use log::error;
use serde::Deserialize;

use beatoraja_core::score_data::ScoreData;
use bms_model::mode::Mode;

use crate::ir_chart_data::IRChartData;
use crate::ir_score_data::IRScoreData;
use crate::leaderboard_entry::LeaderboardEntry;
use crate::lr2_ghost_data::LR2GhostData;
use beatoraja_types::imgui_notify::ImGuiNotify;

/// LR2 IR connection
///
/// Translated from: LR2IRConnection.java
///
/// Original repo from https://github.com/SayakaIsBaka/lr2ir-read-only
///
/// This class is not a real IR connection, but the original repo is. It keeps the
/// original form to make things easier.
static IR_URL: &str = "http://dream-pro.info/~lavalse/LR2IR/2";

lazy_static::lazy_static! {
    static ref LR2_IR_RANKING_CACHE: Mutex<HashMap<String, Vec<LeaderboardEntry>>> = Mutex::new(HashMap::new());
    static ref SCORE_DATABASE_ACCESSOR: Mutex<Option<ScoreDatabaseAccessorRef>> = Mutex::new(None);
}

/// Trait for score database access (avoids direct dependency on ScoreDatabaseAccessor)
pub trait ScoreDatabaseAccess: Send + Sync {
    fn get_score_data(&self, sha256: &str, mode: i32) -> Option<ScoreData>;
}

type ScoreDatabaseAccessorRef = Box<dyn ScoreDatabaseAccess>;

pub struct LR2IRConnection;

impl LR2IRConnection {
    pub fn set_score_database_accessor(accessor: Box<dyn ScoreDatabaseAccess>) {
        let mut guard = SCORE_DATABASE_ACCESSOR.lock().unwrap();
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

    fn make_post_request(uri: &str, data: &str) -> Option<String> {
        let url = format!("{}{}", IR_URL, uri);
        let client = reqwest::blocking::Client::new();
        match client
            .post(&url)
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
                // In Java, response is read with Shift_JIS encoding.
                // reqwest returns bytes; we decode with encoding_rs.
                match response.bytes() {
                    Ok(bytes) => {
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
    /// Returns a pair: (local_score, leaderboard_entries).
    /// The local score can be None.
    pub fn get_score_data(chart: &IRChartData) -> (Option<IRScoreData>, Vec<LeaderboardEntry>) {
        if chart.md5.is_empty() {
            return (None, Vec::new());
        }
        let request_url = format!("songmd5={}&id={}&lastupdate=", chart.md5, "114328");

        let score_data = {
            let cache = LR2_IR_RANKING_CACHE.lock().unwrap();
            cache.get(&request_url).cloned()
        };

        let score_data = match score_data {
            Some(cached) => cached,
            None => {
                match Self::make_post_request("/getrankingxml.cgi", &request_url) {
                    Some(res) => {
                        // Java: res.substring(1).replace("<lastupdate></lastupdate>", "")
                        let xml = if res.len() > 1 {
                            res[1..].replace("<lastupdate></lastupdate>", "")
                        } else {
                            res
                        };
                        match Self::convert_xml_to_ranking(&xml) {
                            Some(ranking) => {
                                let entries = ranking.to_beatoraja_score_data(chart);
                                let mut cache = LR2_IR_RANKING_CACHE.lock().unwrap();
                                cache.insert(request_url, entries.clone());
                                entries
                            }
                            None => {
                                ImGuiNotify::error(
                                    "Failed to get score data from LR2IR: XML parse error",
                                );
                                return (None, Vec::new());
                            }
                        }
                    }
                    None => {
                        return (None, Vec::new());
                    }
                }
            }
        };

        // Get local score
        let local_score = {
            let accessor = SCORE_DATABASE_ACCESSOR.lock().unwrap();
            if let Some(ref acc) = *accessor {
                let lntype = if chart.has_undefined_ln {
                    chart.lntype
                } else {
                    0
                };
                acc.get_score_data(&chart.sha256, lntype).map(|mut s| {
                    // This is intentional behavior, see IRScoreData's player definition
                    // and how we use this feature in LeaderBoardBar
                    s.player = String::new();
                    IRScoreData::new(&s)
                })
            } else {
                None
            }
        };

        (local_score, score_data)
    }

    pub fn get_ghost_data(md5: &str, score_id: i64) -> Option<LR2GhostData> {
        let api = format!(
            "/getghost.cgi?songmd5={}&mode=top&targetid={}",
            md5, score_id
        );
        let url = format!("{}{}", IR_URL, api);
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build();
        let client = match client {
            Ok(c) => c,
            Err(e) => {
                error!("{}", e);
                ImGuiNotify::error("Failed to load ghost data.");
                return None;
            }
        };
        match client.get(&url).send() {
            Ok(response) => {
                let status = response.status();
                if status != reqwest::StatusCode::OK {
                    error!("Unexpected http response code: {}", status);
                    ImGuiNotify::error("Failed to load ghost data.");
                    return None;
                }
                match response.text() {
                    Ok(body) => LR2GhostData::parse(&body),
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
            self.md5, self.id, self.last_update
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
    pub fn to_beatoraja_score_data(&self, model: &IRChartData) -> Vec<LeaderboardEntry> {
        let mut res: Vec<LeaderboardEntry> = Vec::new();
        for s in &self.score {
            let mode = model.mode.clone().unwrap_or(Mode::BEAT_7K);
            let mut tmp = ScoreData::new(mode);
            tmp.sha256 = model.sha256.clone();
            tmp.player = s.name.clone().unwrap_or_default();
            tmp.clear = s.get_beatoraja_clear();
            tmp.notes = s.notes;
            tmp.combo = s.combo;
            tmp.epg = s.pg;
            tmp.egr = s.gr;
            tmp.minbp = s.minbp;
            res.push(LeaderboardEntry::new_entry_lr2_ir(
                IRScoreData::new(&tmp),
                s.id as i64,
            ));
        }

        res.sort_by(|a, b| {
            b.get_ir_score()
                .get_exscore()
                .cmp(&a.get_ir_score().get_exscore())
        });
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
    pub fn get_beatoraja_clear(&self) -> i32 {
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

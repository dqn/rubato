use std::collections::HashMap;
use std::sync::Mutex;

use log::error;
use serde::Deserialize;

use bms_model::mode::Mode;
use rubato_core::score_data::ScoreData;

use crate::ir_chart_data::IRChartData;
use crate::ir_score_data::IRScoreData;
use crate::leaderboard_entry::LeaderboardEntry;
use crate::lr2_ghost_data::LR2GhostData;
use rubato_types::imgui_notify::ImGuiNotify;

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
    fn score_data(&self, sha256: &str, mode: i32) -> Option<ScoreData>;
}

type ScoreDatabaseAccessorRef = Box<dyn ScoreDatabaseAccess>;

pub struct LR2IRConnection;

impl LR2IRConnection {
    pub fn set_score_database_accessor(accessor: Box<dyn ScoreDatabaseAccess>) {
        let mut guard = SCORE_DATABASE_ACCESSOR
            .lock()
            .expect("SCORE_DATABASE_ACCESSOR lock poisoned");
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
    pub fn score_data(chart: &IRChartData) -> (Option<IRScoreData>, Vec<LeaderboardEntry>) {
        if chart.md5.is_empty() {
            return (None, Vec::new());
        }
        let request_url = format!("songmd5={}&id={}&lastupdate=", chart.md5, "114328");

        let score_data = {
            let cache = LR2_IR_RANKING_CACHE
                .lock()
                .expect("LR2_IR_RANKING_CACHE lock poisoned");
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
                                let entries = ranking.to_rubato_score_data(chart);
                                let mut cache = LR2_IR_RANKING_CACHE
                                    .lock()
                                    .expect("LR2_IR_RANKING_CACHE lock poisoned");
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
            let accessor = SCORE_DATABASE_ACCESSOR
                .lock()
                .expect("SCORE_DATABASE_ACCESSOR lock poisoned");
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

        (local_score, score_data)
    }

    pub fn ghost_data(md5: &str, score_id: i64) -> Option<LR2GhostData> {
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
        let (local, entries) = LR2IRConnection::score_data(&chart);
        assert!(local.is_none());
        assert!(entries.is_empty());
    }
}

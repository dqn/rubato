use parking_lot::Mutex;
use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Deserialize;

use bms_rule::{ClearType, ScoreData};

use crate::chart_data::IRChartData;
use crate::connection::IRConnection;
use crate::leaderboard::LeaderboardEntry;
use crate::player_data::IRPlayerData;
use crate::response::IRResponse;
use crate::score_data::IRScoreData;

const IR_URL: &str = "http://dream-pro.info/~lavalse/LR2IR/2";

/// LR2IR connection.
///
/// Corresponds to Java `LR2IRConnection`.
pub struct LR2IRConnection {
    client: reqwest::Client,
    base_url: String,
    cache: Mutex<HashMap<String, Vec<LeaderboardEntry>>>,
}

impl LR2IRConnection {
    pub fn new() -> Self {
        Self::with_base_url(IR_URL.to_string())
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Get LR2IR ranking scores for a chart.
    ///
    /// Returns cached results if available.
    pub async fn get_score_data(&self, chart: &IRChartData) -> Result<Vec<LeaderboardEntry>> {
        if chart.md5.is_empty() {
            return Ok(Vec::new());
        }

        let request_body = format!("songmd5={}&id=114328&lastupdate=", chart.md5);

        // Check cache
        {
            let cache = self.cache.lock();
            if let Some(cached) = cache.get(&request_body) {
                return Ok(cached.clone());
            }
        }

        let url = format!("{}/getrankingxml.cgi", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Connection", "close")
            .body(request_body.clone())
            .send()
            .await
            .context("failed to send request to LR2IR")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("HTTP error code: {}", status);
        }

        let bytes = response.bytes().await?;
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
        let xml_str = decoded.into_owned();

        // Skip first byte and remove empty lastupdate tags (Java behavior)
        let xml_cleaned = if xml_str.len() > 1 {
            xml_str[1..].replace("<lastupdate></lastupdate>", "")
        } else {
            xml_str
        };

        let ranking: Ranking =
            quick_xml::de::from_str(&xml_cleaned).context("failed to parse LR2IR ranking XML")?;

        let mut entries = ranking.to_leaderboard_entries(chart);
        entries.sort_by(|a, b| b.ir_score.exscore().cmp(&a.ir_score.exscore()));

        // Store in cache
        {
            let mut cache = self.cache.lock();
            cache.insert(request_body, entries.clone());
        }

        Ok(entries)
    }

    /// Get ghost data from LR2IR.
    pub async fn get_ghost_data(
        &self,
        md5: &str,
        score_id: i64,
    ) -> Result<bms_replay::LR2GhostData> {
        let url = format!(
            "{}/getghost.cgi?songmd5={}&mode=top&targetid={}",
            self.base_url, md5, score_id
        );

        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .context("failed to fetch ghost data")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("unexpected HTTP response code: {}", status);
        }

        let body = response.text().await?;
        bms_replay::LR2GhostData::parse(&body).context("failed to parse ghost data")
    }
}

impl Default for LR2IRConnection {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl IRConnection for LR2IRConnection {
    async fn get_play_data(
        &self,
        _player: Option<&IRPlayerData>,
        chart: &IRChartData,
    ) -> Result<IRResponse<Vec<IRScoreData>>> {
        let entries = self.get_score_data(chart).await?;
        let scores: Vec<IRScoreData> = entries.into_iter().map(|e| e.ir_score).collect();
        Ok(IRResponse::success(scores))
    }

    async fn get_song_url(&self, chart: &IRChartData) -> Option<String> {
        if chart.md5.is_empty() {
            return None;
        }
        Some(format!("{}/song.cgi?md5={}", self.base_url, chart.md5))
    }
}

/// Convert LR2 clear type to beatoraja clear type.
///
/// Corresponds to Java `Score.getBeatorajaClear()`.
fn lr2_clear_to_beatoraja(clear: i32, pg: i32, gr: i32, notes: i32) -> ClearType {
    match clear {
        1 => ClearType::Failed,
        2 => ClearType::Easy,
        3 => ClearType::Normal,
        4 => ClearType::Hard,
        5 => {
            if pg + gr == notes {
                ClearType::Perfect
            } else {
                ClearType::FullCombo
            }
        }
        _ => ClearType::NoPlay,
    }
}

// XML deserialization structures for LR2IR ranking response

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct Ranking {
    #[serde(default)]
    score: Vec<LR2Score>,
}

impl Ranking {
    fn to_leaderboard_entries(&self, chart: &IRChartData) -> Vec<LeaderboardEntry> {
        self.score
            .iter()
            .map(|s| {
                let clear = lr2_clear_to_beatoraja(s.clear, s.pg, s.gr, s.notes);
                let sd = ScoreData {
                    sha256: chart.sha256.clone(),
                    player: s.name.clone().unwrap_or_default(),
                    clear,
                    notes: s.notes,
                    maxcombo: s.combo,
                    epg: s.pg,
                    egr: s.gr,
                    minbp: s.minbp,
                    ..Default::default()
                };
                let ir_score = IRScoreData::from(&sd);
                LeaderboardEntry::new_lr2(ir_score, s.id as i64)
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct LR2Score {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    id: i32,
    #[serde(default)]
    clear: i32,
    #[serde(default)]
    notes: i32,
    #[serde(default)]
    combo: i32,
    #[serde(default)]
    pg: i32,
    #[serde(default)]
    gr: i32,
    #[serde(default)]
    minbp: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_chart(md5: &str, sha256: &str) -> IRChartData {
        IRChartData {
            md5: md5.to_string(),
            sha256: sha256.to_string(),
            title: String::new(),
            subtitle: String::new(),
            genre: String::new(),
            artist: String::new(),
            subartist: String::new(),
            url: String::new(),
            appendurl: String::new(),
            level: 0,
            total: 0,
            mode: 0,
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
            values: HashMap::new(),
        }
    }

    fn encode_ranking_xml(xml: &str) -> Vec<u8> {
        let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(xml);
        let mut body = vec![0u8]; // leading byte (skipped by parser)
        body.extend_from_slice(&encoded);
        body
    }

    #[test]
    fn lr2_clear_conversion() {
        assert_eq!(lr2_clear_to_beatoraja(0, 0, 0, 0), ClearType::NoPlay);
        assert_eq!(lr2_clear_to_beatoraja(1, 0, 0, 0), ClearType::Failed);
        assert_eq!(lr2_clear_to_beatoraja(2, 0, 0, 0), ClearType::Easy);
        assert_eq!(lr2_clear_to_beatoraja(3, 0, 0, 0), ClearType::Normal);
        assert_eq!(lr2_clear_to_beatoraja(4, 0, 0, 0), ClearType::Hard);
        // FC with pg+gr != notes
        assert_eq!(
            lr2_clear_to_beatoraja(5, 500, 100, 700),
            ClearType::FullCombo
        );
        // FC with pg+gr == notes → Perfect
        assert_eq!(lr2_clear_to_beatoraja(5, 500, 200, 700), ClearType::Perfect);
    }

    #[test]
    fn parse_ranking_xml_empty() {
        let xml = r#"<ranking></ranking>"#;
        let ranking: Ranking = quick_xml::de::from_str(xml).unwrap();
        assert!(ranking.score.is_empty());
    }

    #[test]
    fn parse_ranking_xml_single_score() {
        let xml = r#"<ranking>
            <score>
                <name>Player1</name>
                <id>12345</id>
                <clear>4</clear>
                <notes>1000</notes>
                <combo>800</combo>
                <pg>500</pg>
                <gr>300</gr>
                <minbp>10</minbp>
            </score>
        </ranking>"#;
        let ranking: Ranking = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(ranking.score.len(), 1);
        assert_eq!(ranking.score[0].name, Some("Player1".to_string()));
        assert_eq!(ranking.score[0].id, 12345);
        assert_eq!(ranking.score[0].clear, 4);
        assert_eq!(ranking.score[0].notes, 1000);
        assert_eq!(ranking.score[0].combo, 800);
        assert_eq!(ranking.score[0].pg, 500);
        assert_eq!(ranking.score[0].gr, 300);
        assert_eq!(ranking.score[0].minbp, 10);
    }

    #[test]
    fn parse_ranking_xml_multiple_scores() {
        let xml = r#"<ranking>
            <score>
                <name>Player1</name>
                <id>1</id>
                <clear>5</clear>
                <notes>500</notes>
                <combo>500</combo>
                <pg>400</pg>
                <gr>100</gr>
                <minbp>0</minbp>
            </score>
            <score>
                <name>Player2</name>
                <id>2</id>
                <clear>3</clear>
                <notes>500</notes>
                <combo>300</combo>
                <pg>200</pg>
                <gr>150</gr>
                <minbp>20</minbp>
            </score>
        </ranking>"#;
        let ranking: Ranking = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(ranking.score.len(), 2);
        assert_eq!(ranking.score[0].name, Some("Player1".to_string()));
        assert_eq!(ranking.score[1].name, Some("Player2".to_string()));
    }

    #[test]
    fn ranking_to_leaderboard_entries() {
        let xml = r#"<ranking>
            <score>
                <name>Player1</name>
                <id>100</id>
                <clear>4</clear>
                <notes>1000</notes>
                <combo>800</combo>
                <pg>500</pg>
                <gr>300</gr>
                <minbp>10</minbp>
            </score>
        </ranking>"#;
        let ranking: Ranking = quick_xml::de::from_str(xml).unwrap();
        let chart = IRChartData {
            md5: String::new(),
            sha256: "abc123".to_string(),
            title: String::new(),
            subtitle: String::new(),
            genre: String::new(),
            artist: String::new(),
            subartist: String::new(),
            url: String::new(),
            appendurl: String::new(),
            level: 0,
            total: 0,
            mode: 0,
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
            values: HashMap::new(),
        };
        let entries = ranking.to_leaderboard_entries(&chart);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_lr2_ir());
        assert_eq!(entries[0].lr2_id, 100);
        assert_eq!(entries[0].ir_score.clear, ClearType::Hard);
        assert_eq!(entries[0].ir_score.epg, 500);
        assert_eq!(entries[0].ir_score.egr, 300);
        // exscore = 500*2 + 300 = 1300
        assert_eq!(entries[0].ir_score.exscore(), 1300);
    }

    #[test]
    fn lr2ir_connection_default() {
        let conn = LR2IRConnection::default();
        let cache = conn.cache.lock();
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn get_score_data_http_mock() {
        let server = MockServer::start().await;
        let xml = r#"<ranking>
            <score>
                <name>Alice</name>
                <id>1</id>
                <clear>5</clear>
                <notes>500</notes>
                <combo>500</combo>
                <pg>400</pg>
                <gr>100</gr>
                <minbp>0</minbp>
            </score>
            <score>
                <name>Bob</name>
                <id>2</id>
                <clear>3</clear>
                <notes>500</notes>
                <combo>300</combo>
                <pg>200</pg>
                <gr>150</gr>
                <minbp>20</minbp>
            </score>
        </ranking>"#;

        Mock::given(method("POST"))
            .and(path("/getrankingxml.cgi"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(encode_ranking_xml(xml), "application/xml"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let conn = LR2IRConnection::with_base_url(server.uri());
        let chart = test_chart("abc123", "sha256hash");
        let entries = conn.get_score_data(&chart).await.unwrap();

        assert_eq!(entries.len(), 2);
        // Sorted by exscore descending: Alice 400*2+100=900, Bob 200*2+150=550
        assert_eq!(entries[0].ir_score.epg, 400);
        assert_eq!(entries[0].ir_score.player, "Alice");
        assert_eq!(entries[0].ir_score.exscore(), 900);
        assert_eq!(entries[0].ir_score.clear, ClearType::Perfect);
        assert_eq!(entries[1].ir_score.epg, 200);
        assert_eq!(entries[1].ir_score.player, "Bob");
        assert_eq!(entries[1].ir_score.exscore(), 550);
        assert_eq!(entries[1].ir_score.clear, ClearType::Normal);
    }

    #[tokio::test]
    async fn get_score_data_cache_hit() {
        let server = MockServer::start().await;
        let xml = r#"<ranking>
            <score>
                <name>CacheTest</name>
                <id>1</id>
                <clear>2</clear>
                <notes>100</notes>
                <combo>50</combo>
                <pg>30</pg>
                <gr>20</gr>
                <minbp>5</minbp>
            </score>
        </ranking>"#;

        Mock::given(method("POST"))
            .and(path("/getrankingxml.cgi"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(encode_ranking_xml(xml), "application/xml"),
            )
            .expect(1) // Exactly 1 request — second call must hit cache
            .mount(&server)
            .await;

        let conn = LR2IRConnection::with_base_url(server.uri());
        let chart = test_chart("md5hash", "sha256hash");

        let first = conn.get_score_data(&chart).await.unwrap();
        let second = conn.get_score_data(&chart).await.unwrap();
        assert_eq!(first.len(), second.len());
        assert_eq!(first[0].ir_score.player, second[0].ir_score.player);
    }

    #[tokio::test]
    async fn get_score_data_empty_md5() {
        let conn = LR2IRConnection::with_base_url("http://unused".to_string());
        let chart = test_chart("", "sha256hash");
        let entries = conn.get_score_data(&chart).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn get_score_data_http_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/getrankingxml.cgi"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let conn = LR2IRConnection::with_base_url(server.uri());
        let chart = test_chart("abc123", "sha256hash");
        let result = conn.get_score_data(&chart).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP error code"));
    }

    #[tokio::test]
    async fn get_score_data_malformed_xml() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/getrankingxml.cgi"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(encode_ranking_xml("<not-valid-xml>"), "application/xml"),
            )
            .mount(&server)
            .await;

        let conn = LR2IRConnection::with_base_url(server.uri());
        let chart = test_chart("abc123", "sha256hash");
        let result = conn.get_score_data(&chart).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_ghost_data_http_mock() {
        let server = MockServer::start().await;
        // options=0010 means random_digit=(10/10)%10=1 → Mirror
        // ghost="E2D2" → PGreat×2, Great×2 (RLE format)
        let csv_body = "name,options,seed,ghost\nPlayer1,0010,12345,E2D2";

        Mock::given(method("GET"))
            .and(path("/getghost.cgi"))
            .respond_with(ResponseTemplate::new(200).set_body_string(csv_body))
            .expect(1)
            .mount(&server)
            .await;

        let conn = LR2IRConnection::with_base_url(server.uri());
        let ghost = conn.get_ghost_data("abc123", 999).await.unwrap();
        assert_eq!(ghost.seed, 12345);
        assert_eq!(
            ghost.random,
            bms_replay::lr2_ghost_data::LR2RandomOption::Mirror
        );
        assert_eq!(ghost.pgreat, 2);
        assert_eq!(ghost.great, 2);
    }

    #[tokio::test]
    async fn get_ghost_data_http_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/getghost.cgi"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let conn = LR2IRConnection::with_base_url(server.uri());
        let result = conn.get_ghost_data("abc123", 999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_play_data_via_trait() {
        let server = MockServer::start().await;
        let xml = r#"<ranking>
            <score>
                <name>TraitTest</name>
                <id>10</id>
                <clear>4</clear>
                <notes>200</notes>
                <combo>180</combo>
                <pg>150</pg>
                <gr>40</gr>
                <minbp>3</minbp>
            </score>
        </ranking>"#;

        Mock::given(method("POST"))
            .and(path("/getrankingxml.cgi"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(encode_ranking_xml(xml), "application/xml"),
            )
            .mount(&server)
            .await;

        let conn = LR2IRConnection::with_base_url(server.uri());
        let chart = test_chart("md5test", "sha256test");
        let response = conn.get_play_data(None, &chart).await.unwrap();
        assert!(response.succeeded);
        let scores = response.data.unwrap();
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].epg, 150);
        assert_eq!(scores[0].egr, 40);
        assert_eq!(scores[0].clear, ClearType::Hard);
    }
}

//! Adapter that wraps LR2IRConnection static methods into the IRConnection trait.
//!
//! LR2IR is a stateless, read-only Internet Ranking protocol. There is no
//! login endpoint; the player ID is passed with each API call. This adapter
//! stores the player ID from the login call and delegates to
//! LR2IRConnection's static methods.

use std::sync::Mutex;

use rubato_types::sync_utils::lock_or_recover;

use crate::ir_account::IRAccount;
use crate::ir_chart_data::IRChartData;
use crate::ir_connection::IRConnection;
use crate::ir_course_data::IRCourseData;
use crate::ir_player_data::IRPlayerData;
use crate::ir_response::IRResponse;
use crate::ir_score_data::IRScoreData;
use crate::ir_table_data::IRTableData;
use crate::lr2_ir_connection::LR2IRConnection;

/// Name constant used for IRConnectionManager registration.
pub const LR2IR_NAME: &str = "LR2IR";

/// LR2IR ranking page base URL.
const LR2IR_RANKING_URL: &str =
    "http://www.dream-pro.info/~lavalse/LR2IR/search.cgi?mode=ranking&bmsmd5=";

/// LR2IR player page base URL.
const LR2IR_PLAYER_URL: &str =
    "http://www.dream-pro.info/~lavalse/LR2IR/search.cgi?mode=mypage&playerid=";

/// Adapter implementing IRConnection for LR2IR.
pub struct LR2IRConnectionAdapter {
    player_id: Mutex<String>,
}

impl LR2IRConnectionAdapter {
    pub fn new() -> Self {
        Self {
            player_id: Mutex::new(String::new()),
        }
    }
}

impl Default for LR2IRConnectionAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IRConnection for LR2IRConnectionAdapter {
    fn login(&self, account: &IRAccount) -> IRResponse<IRPlayerData> {
        let userid = &account.id;
        if userid.is_empty() {
            return IRResponse::failure("Empty user ID".to_string());
        }
        // Store the player ID for subsequent API calls
        {
            let mut id = lock_or_recover(&self.player_id);
            *id = userid.clone();
        }
        // LR2IR has no login endpoint; accept any non-empty userid
        let player_data = IRPlayerData::new(userid.clone(), userid.clone(), String::new());
        IRResponse::success("OK".to_string(), player_data)
    }

    fn login_with_credentials(&self, id: &str, _pass: &str) -> IRResponse<IRPlayerData> {
        if id.is_empty() {
            return IRResponse::failure("Empty user ID".to_string());
        }
        {
            let mut pid = lock_or_recover(&self.player_id);
            *pid = id.to_string();
        }
        let player_data = IRPlayerData::new(id.to_string(), id.to_string(), String::new());
        IRResponse::success("OK".to_string(), player_data)
    }

    fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>> {
        IRResponse::failure("LR2IR does not support rivals".to_string())
    }

    fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>> {
        IRResponse::failure("LR2IR does not support table data".to_string())
    }

    fn get_play_data(
        &self,
        _player: Option<&IRPlayerData>,
        chart: &IRChartData,
    ) -> IRResponse<Vec<IRScoreData>> {
        let player_id = lock_or_recover(&self.player_id).clone();
        let (_local_score, leaderboard) = LR2IRConnection::score_data(chart, &player_id);
        let scores: Vec<IRScoreData> = leaderboard.into_iter().map(|e| e.into_ir_score()).collect();
        IRResponse::success("OK".to_string(), scores)
    }

    fn get_course_play_data(
        &self,
        _player: Option<&IRPlayerData>,
        _course: &IRCourseData,
    ) -> IRResponse<Vec<IRScoreData>> {
        IRResponse::failure("LR2IR does not support course play data".to_string())
    }

    fn send_play_data(&self, _model: &IRChartData, _score: &IRScoreData) -> IRResponse<()> {
        // LR2IR is read-only; score submission is not supported
        IRResponse::failure("LR2IR does not support score submission".to_string())
    }

    fn send_course_play_data(
        &self,
        _course: &IRCourseData,
        _score: &IRScoreData,
    ) -> IRResponse<()> {
        IRResponse::failure("LR2IR does not support course score submission".to_string())
    }

    fn get_song_url(&self, chart: &IRChartData) -> Option<String> {
        if chart.md5.is_empty() {
            None
        } else {
            Some(format!("{}{}", LR2IR_RANKING_URL, chart.md5))
        }
    }

    fn get_course_url(&self, _course: &IRCourseData) -> Option<String> {
        None
    }

    fn get_player_url(&self, player: &IRPlayerData) -> Option<String> {
        if player.id.is_empty() {
            None
        } else {
            Some(format!("{}{}", LR2IR_PLAYER_URL, player.id))
        }
    }

    fn name(&self) -> &str {
        LR2IR_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_stores_player_id() {
        let adapter = LR2IRConnectionAdapter::new();
        let account = IRAccount::new("12345".to_string(), "pass".to_string(), String::new());
        let resp = adapter.login(&account);
        assert!(resp.is_succeeded());
        assert_eq!(resp.data.unwrap().id, "12345");
        assert_eq!(*adapter.player_id.lock().unwrap(), "12345");
    }

    #[test]
    fn test_login_empty_userid_fails() {
        let adapter = LR2IRConnectionAdapter::new();
        let account = IRAccount::new(String::new(), "pass".to_string(), String::new());
        let resp = adapter.login(&account);
        assert!(!resp.is_succeeded());
    }

    #[test]
    fn test_login_with_credentials() {
        let adapter = LR2IRConnectionAdapter::new();
        let resp = adapter.login_with_credentials("67890", "pass");
        assert!(resp.is_succeeded());
        assert_eq!(*adapter.player_id.lock().unwrap(), "67890");
    }

    #[test]
    fn test_name() {
        let adapter = LR2IRConnectionAdapter::new();
        assert_eq!(adapter.name(), "LR2IR");
    }

    #[test]
    fn test_get_song_url() {
        let adapter = LR2IRConnectionAdapter::new();
        let chart = IRChartData {
            md5: "abc123".to_string(),
            ..Default::default()
        };
        let url = adapter.get_song_url(&chart);
        assert!(url.is_some());
        assert!(url.unwrap().contains("abc123"));
    }

    #[test]
    fn test_get_song_url_empty_md5() {
        let adapter = LR2IRConnectionAdapter::new();
        let chart = IRChartData::default();
        assert!(adapter.get_song_url(&chart).is_none());
    }

    #[test]
    fn test_get_player_url() {
        let adapter = LR2IRConnectionAdapter::new();
        let player = IRPlayerData::new("42".to_string(), "Player".to_string(), String::new());
        let url = adapter.get_player_url(&player);
        assert!(url.is_some());
        assert!(url.unwrap().contains("42"));
    }

    #[test]
    fn test_unsupported_methods_return_failure() {
        let adapter = LR2IRConnectionAdapter::new();
        assert!(!adapter.get_rivals().is_succeeded());
        assert!(!adapter.get_table_datas().is_succeeded());

        let score = IRScoreData::new(&rubato_core::score_data::ScoreData::default());
        assert!(
            !adapter
                .send_play_data(&IRChartData::default(), &score)
                .is_succeeded()
        );
    }
}

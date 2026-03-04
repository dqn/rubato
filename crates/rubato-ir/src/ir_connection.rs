use crate::ir_account::IRAccount;
use crate::ir_chart_data::IRChartData;
use crate::ir_course_data::IRCourseData;
use crate::ir_player_data::IRPlayerData;
use crate::ir_response::IRResponse;
use crate::ir_score_data::IRScoreData;
use crate::ir_table_data::IRTableData;

/// IR connection interface
///
/// Translated from: IRConnection.java (interface)
pub trait IRConnection {
    /// Register a new user on IR.
    fn register(&self, account: &IRAccount) -> IRResponse<IRPlayerData> {
        let _ = account;
        IRResponse::failure("register() not implemented for this IR connection".to_string())
    }

    /// Register a new user on IR with id, pass, name.
    fn register_with_credentials(
        &self,
        id: &str,
        pass: &str,
        name: &str,
    ) -> IRResponse<IRPlayerData> {
        let _ = (id, pass, name);
        IRResponse::failure(
            "register_with_credentials() not implemented for this IR connection".to_string(),
        )
    }

    /// Login to IR. Called at startup.
    fn login(&self, account: &IRAccount) -> IRResponse<IRPlayerData> {
        let _ = account;
        IRResponse::failure("login() not implemented for this IR connection".to_string())
    }

    /// Login to IR with id and pass.
    fn login_with_credentials(&self, id: &str, pass: &str) -> IRResponse<IRPlayerData> {
        let _ = (id, pass);
        IRResponse::failure(
            "login_with_credentials() not implemented for this IR connection".to_string(),
        )
    }

    /// Get rival data
    fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>>;

    /// Get table data configured on IR
    fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>>;

    /// Get score data
    fn get_play_data(
        &self,
        player: Option<&IRPlayerData>,
        chart: &IRChartData,
    ) -> IRResponse<Vec<IRScoreData>>;

    /// Get course play data
    fn get_course_play_data(
        &self,
        player: Option<&IRPlayerData>,
        course: &IRCourseData,
    ) -> IRResponse<Vec<IRScoreData>>;

    /// Send score data
    fn send_play_data(&self, model: &IRChartData, score: &IRScoreData) -> IRResponse<()>;

    /// Send course score data
    fn send_course_play_data(&self, course: &IRCourseData, score: &IRScoreData) -> IRResponse<()>;

    /// Get song URL. Returns None if not found.
    fn get_song_url(&self, chart: &IRChartData) -> Option<String>;

    /// Get course URL. Returns None if not found.
    fn get_course_url(&self, course: &IRCourseData) -> Option<String>;

    /// Get player URL.
    fn get_player_url(&self, player: &IRPlayerData) -> Option<String>;

    /// Get the NAME constant for this IR connection
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal IRConnection implementor that only provides required methods,
    /// relying on default implementations for register/login.
    struct MinimalIR;

    impl IRConnection for MinimalIR {
        fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>> {
            IRResponse::failure("not implemented".to_string())
        }
        fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>> {
            IRResponse::failure("not implemented".to_string())
        }
        fn get_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _chart: &IRChartData,
        ) -> IRResponse<Vec<IRScoreData>> {
            IRResponse::failure("not implemented".to_string())
        }
        fn get_course_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _course: &IRCourseData,
        ) -> IRResponse<Vec<IRScoreData>> {
            IRResponse::failure("not implemented".to_string())
        }
        fn send_play_data(&self, _model: &IRChartData, _score: &IRScoreData) -> IRResponse<()> {
            IRResponse::failure("not implemented".to_string())
        }
        fn send_course_play_data(
            &self,
            _course: &IRCourseData,
            _score: &IRScoreData,
        ) -> IRResponse<()> {
            IRResponse::failure("not implemented".to_string())
        }
        fn get_song_url(&self, _chart: &IRChartData) -> Option<String> {
            None
        }
        fn get_course_url(&self, _course: &IRCourseData) -> Option<String> {
            None
        }
        fn get_player_url(&self, _player: &IRPlayerData) -> Option<String> {
            None
        }
        fn name(&self) -> &str {
            "MinimalIR"
        }
    }

    #[test]
    fn test_default_register_returns_failure_not_panic() {
        let ir = MinimalIR;
        let account = IRAccount::new("id".to_string(), "pass".to_string(), "name".to_string());
        let resp = ir.register(&account);
        assert!(!resp.is_succeeded());
        assert!(resp.get_message().contains("register()"));
    }

    #[test]
    fn test_default_register_with_credentials_returns_failure_not_panic() {
        let ir = MinimalIR;
        let resp = ir.register_with_credentials("id", "pass", "name");
        assert!(!resp.is_succeeded());
        assert!(resp.get_message().contains("register_with_credentials()"));
    }

    #[test]
    fn test_default_login_returns_failure_not_panic() {
        let ir = MinimalIR;
        let account = IRAccount::new("id".to_string(), "pass".to_string(), "name".to_string());
        let resp = ir.login(&account);
        assert!(!resp.is_succeeded());
        assert!(resp.get_message().contains("login()"));
    }

    #[test]
    fn test_default_login_with_credentials_returns_failure_not_panic() {
        let ir = MinimalIR;
        let resp = ir.login_with_credentials("id", "pass");
        assert!(!resp.is_succeeded());
        assert!(resp.get_message().contains("login_with_credentials()"));
    }
}

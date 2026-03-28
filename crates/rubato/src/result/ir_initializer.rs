// IR initialization logic
// Translated from: MainController.initializeIRConfig() line 169 (Java)
//
// This module lives in beatoraja-result instead of beatoraja-core because
// beatoraja-core cannot depend on beatoraja-ir (circular dependency).

use std::sync::Arc;

use crate::ir::ir_account::IRAccount;
use crate::ir::ir_connection::IRConnection;
use crate::ir::ir_connection_manager::IRConnectionManager;
use rubato_skin::player_config::PlayerConfig;

use super::ir_status::IRStatus;

/// Initialize IR connections from player config.
///
/// Translated from: MainController.initializeIRConfig() line 169
///
/// Iterates the player's IR configs, attempts to connect and login to each,
/// and returns the successfully connected IRStatus entries.
pub fn initialize_ir_config(player: &PlayerConfig) -> Vec<IRStatus> {
    let mut ir_array: Vec<IRStatus> = Vec::new();

    for irconfig_opt in &player.irconfig {
        let irconfig = match irconfig_opt {
            Some(c) => c,
            None => continue,
        };
        let ir: Option<Box<dyn IRConnection + Send + Sync>> =
            IRConnectionManager::ir_connection(&irconfig.irname);
        if let Some(ir) = ir {
            let userid = irconfig.userid();
            let password = irconfig.password();
            if userid.is_empty() || password.is_empty() {
                // Java: empty block -- skip if no credentials
            } else {
                let ir: Arc<dyn IRConnection + Send + Sync> = Arc::from(ir);
                let account = IRAccount::new(userid.clone(), password.clone(), String::new());
                // Note: ir.login() is called synchronously on the startup thread.
                // LR2IR's login() returns immediately (stores player ID), so this
                // is not blocking in practice despite the synchronous call pattern.
                let response = ir.login(&account);
                if response.is_succeeded() {
                    if let Some(player_data) = response.data {
                        ir_array.push(IRStatus::new(irconfig.clone(), ir, player_data));
                    }
                } else {
                    log::warn!("IR login failed: {}", response.message);
                }
            }
        }
    }

    ir_array
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    use crate::ir::ir_chart_data::IRChartData;
    use crate::ir::ir_connection_manager::{IRConnectionEntry, register_ir_connections};
    use crate::ir::ir_course_data::IRCourseData;
    use crate::ir::ir_player_data::IRPlayerData;
    use crate::ir::ir_response::IRResponse;
    use crate::ir::ir_score_data::IRScoreData;
    use crate::ir::ir_table_data::IRTableData;

    /// Mock IRConnection whose login() always returns failure.
    struct FailingLoginIR;

    impl IRConnection for FailingLoginIR {
        fn login(&self, _account: &IRAccount) -> IRResponse<IRPlayerData> {
            IRResponse::failure("authentication failed".to_string())
        }

        fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>> {
            IRResponse::failure("not implemented".to_string())
        }
        fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>> {
            IRResponse::failure("not implemented".to_string())
        }
        fn get_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _chart: Option<&IRChartData>,
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
            "FailingLoginIR"
        }
    }

    #[test]
    fn test_initialize_ir_config_empty_config() {
        let player = PlayerConfig::default();
        let result = initialize_ir_config(&player);
        assert!(result.is_empty());
    }

    #[test]
    fn test_initialize_ir_config_with_none_entries() {
        let mut player = PlayerConfig::default();
        player.irconfig = vec![None, None];
        let result = initialize_ir_config(&player);
        assert!(result.is_empty());
    }

    #[test]
    fn test_initialize_ir_config_empty_credentials() {
        use rubato_skin::ir_config::IRConfig;
        let mut player = PlayerConfig::default();
        let mut ir = IRConfig::default();
        ir.irname = "TestIR".to_string();
        // userid and password are empty -> should skip
        player.irconfig = vec![Some(ir)];
        let result = initialize_ir_config(&player);
        // No IR connection registered for "TestIR", so result is empty
        assert!(result.is_empty());
    }

    #[test]
    fn test_initialize_ir_config_login_failure_returns_empty() {
        use rubato_skin::ir_config::IRConfig;

        // Register the mock IR connection in the global registry
        register_ir_connections(vec![IRConnectionEntry {
            name: "FailingLoginIR".to_string(),
            home: None,
            factory: Box::new(|| Box::new(FailingLoginIR)),
        }]);

        let mut player = PlayerConfig::default();
        let mut ir = IRConfig::default();
        ir.irname = "FailingLoginIR".to_string();
        // Set non-empty credentials so the login path is reached
        ir.userid = "testuser".to_string();
        ir.password = "testpass".to_string();
        player.irconfig = vec![Some(ir)];

        let result = initialize_ir_config(&player);
        // login() returns failure, so no IRStatus entries should be added
        assert!(result.is_empty());
    }
}

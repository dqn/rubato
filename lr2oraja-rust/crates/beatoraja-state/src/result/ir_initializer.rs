// IR initialization logic
// Translated from: MainController.initializeIRConfig() (Java)
//
// This module lives in beatoraja-result instead of beatoraja-core because
// beatoraja-core cannot depend on beatoraja-ir (circular dependency).

use std::sync::Arc;

use beatoraja_ir::ir_account::IRAccount;
use beatoraja_ir::ir_connection::IRConnection;
use beatoraja_ir::ir_connection_manager::IRConnectionManager;
use beatoraja_types::player_config::PlayerConfig;

use super::ir_status::IRStatus;

/// Initialize IR connections from player config.
///
/// Translated from: MainController.initializeIRConfig()
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
            IRConnectionManager::get_ir_connection(irconfig.get_irname());
        if let Some(ir) = ir {
            let userid = irconfig.get_userid();
            let password = irconfig.get_password();
            if userid.is_empty() || password.is_empty() {
                // Java: empty block — skip if no credentials
            } else {
                let ir: Arc<dyn IRConnection + Send + Sync> = Arc::from(ir);
                // Java: try { ir.login(new IRAccount(...)) }
                //        catch (IllegalArgumentException) { ir.login(userid, password) }
                // In Rust, the default login() panics like Java's IllegalArgumentException.
                // Use catch_unwind to match the Java try/catch pattern.
                let account = IRAccount::new(userid.clone(), password.clone(), String::new());
                let login_result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ir.login(&account)));
                match login_result {
                    Ok(response) => {
                        if response.is_succeeded() {
                            if let Some(player_data) = response.data {
                                ir_array.push(IRStatus::new(irconfig.clone(), ir, player_data));
                            }
                        } else {
                            log::warn!("IRへのログイン失敗 : {}", response.get_message());
                        }
                    }
                    Err(_) => {
                        // Java: catch (IllegalArgumentException e)
                        log::info!("trying pre-0.8.5 IR login method");
                        let response = ir.login_with_credentials(&userid, &password);
                        if response.is_succeeded() {
                            if let Some(player_data) = response.data {
                                ir_array.push(IRStatus::new(irconfig.clone(), ir, player_data));
                            }
                        } else {
                            log::warn!("IRへのログイン失敗 : {}", response.get_message());
                        }
                    }
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
        use beatoraja_types::ir_config::IRConfig;
        let mut player = PlayerConfig::default();
        let mut ir = IRConfig::default();
        ir.irname = "TestIR".to_string();
        // userid and password are empty -> should skip
        player.irconfig = vec![Some(ir)];
        let result = initialize_ir_config(&player);
        // No IR connection registered for "TestIR", so result is empty
        assert!(result.is_empty());
    }
}

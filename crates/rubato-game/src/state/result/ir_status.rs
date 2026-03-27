use std::sync::Arc;

use crate::core::ir_config::IRConfig;
use crate::ir::ir_connection::IRConnection;
use crate::ir::ir_player_data::IRPlayerData;

/// MainController.IRStatus — IR connection status
///
/// Translated from: MainController.IRStatus (Java inner class)
pub struct IRStatus {
    pub config: IRConfig,
    pub connection: Arc<dyn IRConnection + Send + Sync>,
    pub player: IRPlayerData,
}

impl IRStatus {
    pub fn new(
        config: IRConfig,
        connection: Arc<dyn IRConnection + Send + Sync>,
        player: IRPlayerData,
    ) -> Self {
        Self {
            config,
            connection,
            player,
        }
    }
}

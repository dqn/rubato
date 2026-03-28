use crate::ir::ir_connection::IRConnection;

/// MainController.IRStatus -- uses dyn IRConnection trait
pub struct IRStatus {
    pub connection: Box<dyn IRConnection>,
    pub player: crate::ir::ir_player_data::IRPlayerData,
}

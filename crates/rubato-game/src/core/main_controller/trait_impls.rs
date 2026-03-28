use super::*;

/// Additional inherent methods on MainController.
///
/// These methods were formerly part of the `MainControllerAccess` trait impl.
/// They delegate to fields on `GameContext` and are used by state_factory
/// and target_property code.
impl MainController {
    pub fn rival_count(&self) -> usize {
        self.ctx.db.rivals.rival_count()
    }

    pub fn rival_information(
        &self,
        index: usize,
    ) -> Option<rubato_types::player_information::PlayerInformation> {
        self.ctx.db.rivals.rival_information(index).cloned()
    }

    pub fn is_ipfs_download_alive(&self) -> bool {
        self.ctx
            .integration
            .download
            .as_ref()
            .is_some_and(|dl| dl.is_alive())
    }

    pub fn read_score_data_by_hash(
        &self,
        hash: &str,
        ln: bool,
        lnmode: i32,
    ) -> Option<rubato_types::score_data::ScoreData> {
        self.ctx
            .db
            .playdata
            .as_ref()
            .and_then(|pda| pda.read_score_data_by_hash(hash, ln, lnmode))
    }
}

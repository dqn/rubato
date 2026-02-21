use crate::player_information::PlayerInformation;
use crate::main_controller::MainController;

/// Rival data accessor.
/// Translated from Java: RivalDataAccessor
///
/// Note: Most of the IR (Internet Ranking) functionality requires Phase 5+ types
/// (IRConnectionManager, IRResponse, ScoreDataImporter, etc.) and is stubbed.
pub struct RivalDataAccessor {
    rivals: Vec<PlayerInformation>,
    // rivalcaches would hold ScoreDataCache instances (Phase 5+)
}

impl RivalDataAccessor {
    pub fn new() -> Self {
        Self { rivals: Vec::new() }
    }

    pub fn get_rival_information(&self, index: usize) -> Option<&PlayerInformation> {
        self.rivals.get(index)
    }

    pub fn get_rival_count(&self) -> usize {
        self.rivals.len()
    }

    pub fn update(&mut self, _main: &MainController) {
        // TODO: IR integration requires Phase 5+ types
        // (IRResponse, IRPlayerData, IRScoreData, ScoreDataImporter, ScoreDataCache)
        // Stubbed for now.
        todo!("IR integration not yet translated")
    }
}

impl Default for RivalDataAccessor {
    fn default() -> Self {
        Self::new()
    }
}

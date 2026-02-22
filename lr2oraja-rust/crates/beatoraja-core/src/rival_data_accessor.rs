use crate::main_controller::MainController;
use crate::player_information::PlayerInformation;

/// ScoreDataCache stub for rival score caching (Phase 5+)
pub struct ScoreDataCacheStub;

/// Rival data accessor.
/// Translated from Java: RivalDataAccessor
///
/// Note: Most of the IR (Internet Ranking) functionality requires Phase 5+ types
/// (IRConnectionManager, IRResponse, ScoreDataImporter, etc.) and is stubbed.
pub struct RivalDataAccessor {
    rivals: Vec<PlayerInformation>,
    /// Rival score data caches (Phase 5+: ScoreDataCache instances)
    rivalcaches: Vec<ScoreDataCacheStub>,
}

impl RivalDataAccessor {
    pub fn new() -> Self {
        Self {
            rivals: Vec::new(),
            rivalcaches: Vec::new(),
        }
    }

    pub fn get_rival_information(&self, index: usize) -> Option<&PlayerInformation> {
        self.rivals.get(index)
    }

    /// Get rival score data cache by index.
    ///
    /// Translated from: RivalDataAccessor.getRivalScoreDataCache(int)
    pub fn get_rival_score_data_cache(&self, index: usize) -> Option<&ScoreDataCacheStub> {
        self.rivalcaches.get(index)
    }

    pub fn get_rival_count(&self) -> usize {
        self.rivals.len()
    }

    pub fn update(&mut self, _main: &MainController) {
        // TODO: IR integration requires Phase 5+ types
        // (IRResponse, IRPlayerData, IRScoreData, ScoreDataImporter, ScoreDataCache)
        // Stubbed for now.
        log::warn!("not yet implemented: IR integration for rival data");
    }
}

impl Default for RivalDataAccessor {
    fn default() -> Self {
        Self::new()
    }
}

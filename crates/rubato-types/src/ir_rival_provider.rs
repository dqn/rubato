use crate::player_information::PlayerInformation;
use crate::score_data::ScoreData;

/// IR rival info (lightweight struct for passing across crate boundaries)
#[derive(Clone, Debug)]
pub struct RivalInfo {
    pub id: String,
    pub name: String,
    pub rank: String,
}

impl RivalInfo {
    pub fn to_player_information(&self) -> PlayerInformation {
        PlayerInformation {
            id: Some(self.id.clone()),
            name: Some(self.name.clone()),
            rank: Some(self.rank.clone()),
        }
    }
}

/// Trait bridge for IR rival/score operations.
/// Implemented in beatoraja-ir, consumed by beatoraja-core (RivalDataAccessor).
/// Breaks the core→ir circular dependency.
pub trait IRRivalProvider: Send + Sync {
    /// Whether the user has requested score import from IR
    fn should_import_scores(&self) -> bool;
    /// Clear the import flag after import
    fn clear_import_flag(&mut self);
    /// Fetch own player's scores from IR (already converted to ScoreData)
    fn fetch_own_scores(&self) -> anyhow::Result<Vec<ScoreData>>;
    /// Whether rival import is enabled in config
    fn should_import_rivals(&self) -> bool;
    /// Fetch rival list from IR
    fn fetch_rival_list(&self) -> anyhow::Result<Vec<RivalInfo>>;
    /// Fetch a specific rival's scores from IR (already converted to ScoreData)
    fn fetch_rival_scores(&self, rival: &RivalInfo) -> anyhow::Result<Vec<ScoreData>>;
    /// IR service name (used for file naming: "rival/{irname}{id}.db")
    fn ir_name(&self) -> String;
    /// Score hash identifier for import
    fn score_hash(&self) -> String;
}

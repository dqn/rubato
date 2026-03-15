use crate::score_data::ScoreData;

/// Trait interface for score database access.
///
/// Downstream crates use `Box<dyn ScoreDatabaseAccess>` instead of concrete
/// ScoreDatabaseAccessor stubs. The real implementation in beatoraja-core
/// implements this trait.
///
/// Translated from Java: ScoreDatabaseAccessor (read/write interface)
pub trait ScoreDatabaseAccess: Send {
    /// Create database tables if they don't exist.
    fn create_table(&self) -> anyhow::Result<()>;

    /// Get score data for the given hash and mode.
    fn score_data(&self, sha256: &str, mode: i32) -> Option<ScoreData>;
    /// Write multiple score data entries.
    fn set_score_data_slice(&self, scores: &[ScoreData]);
}

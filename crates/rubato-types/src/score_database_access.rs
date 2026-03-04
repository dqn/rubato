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
    fn create_table(&self);

    /// Get score data for the given hash and mode.
    fn get_score_data(&self, sha256: &str, mode: i32) -> Option<ScoreData>;

    /// Write multiple score data entries.
    fn set_score_data_slice(&self, scores: &[ScoreData]);
}

/// Null implementation that logs warnings and returns defaults.
pub struct NullScoreDatabaseAccess;

impl ScoreDatabaseAccess for NullScoreDatabaseAccess {
    fn create_table(&self) {
        log::warn!("NullScoreDatabaseAccess::create_table called — no-op");
    }

    fn get_score_data(&self, _sha256: &str, _mode: i32) -> Option<ScoreData> {
        log::warn!("NullScoreDatabaseAccess::get_score_data called — returning None");
        None
    }

    fn set_score_data_slice(&self, _scores: &[ScoreData]) {
        log::warn!("NullScoreDatabaseAccess::set_score_data_slice called — no-op");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_score_database_access_create_table() {
        let db = NullScoreDatabaseAccess;
        db.create_table(); // should not panic
    }

    #[test]
    fn test_null_score_database_access_get_score_data() {
        let db = NullScoreDatabaseAccess;
        assert!(db.get_score_data("abc", 0).is_none());
    }

    #[test]
    fn test_null_score_database_access_set_score_data() {
        let db = NullScoreDatabaseAccess;
        db.set_score_data_slice(&[]); // should not panic
    }
}

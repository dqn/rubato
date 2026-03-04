use crate::score_data::ScoreData;

/// Trait interface for AbstractResult data access.
///
/// Downstream crates use `&dyn AbstractResultAccess` instead of concrete AbstractResult stubs.
/// The real implementation in beatoraja-result's AbstractResultData implements this trait.
///
/// Translated from Java: AbstractResult (field access pattern for result screens)
pub trait AbstractResultAccess {
    /// Get the new (current play) score data
    fn get_new_score(&self) -> &ScoreData;

    /// Get the old (previous best) score data
    fn get_old_score(&self) -> &ScoreData;

    /// Get the IR ranking position (0 if not ranked)
    fn get_ir_rank(&self) -> i32;

    /// Get the total number of IR players
    fn get_ir_total_player(&self) -> i32;

    /// Get the previous IR ranking position
    fn get_old_ir_rank(&self) -> i32;
}

/// Null implementation returning defaults.
pub struct NullAbstractResult {
    default_score: ScoreData,
}

impl NullAbstractResult {
    pub fn new() -> Self {
        Self {
            default_score: ScoreData::default(),
        }
    }
}

impl Default for NullAbstractResult {
    fn default() -> Self {
        Self::new()
    }
}

impl AbstractResultAccess for NullAbstractResult {
    fn get_new_score(&self) -> &ScoreData {
        &self.default_score
    }
    fn get_old_score(&self) -> &ScoreData {
        &self.default_score
    }
    fn get_ir_rank(&self) -> i32 {
        0
    }
    fn get_ir_total_player(&self) -> i32 {
        0
    }
    fn get_old_ir_rank(&self) -> i32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_abstract_result() {
        let r = NullAbstractResult::new();
        assert_eq!(r.get_ir_rank(), 0);
        assert_eq!(r.get_ir_total_player(), 0);
        assert_eq!(r.get_old_ir_rank(), 0);
        assert_eq!(r.get_new_score().get_exscore(), 0);
        assert_eq!(r.get_old_score().get_exscore(), 0);
    }
}

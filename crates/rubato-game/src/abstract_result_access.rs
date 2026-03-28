use rubato_types::score_data::ScoreData;

/// Trait interface for AbstractResult data access.
///
/// Downstream crates use `&dyn AbstractResultAccess` instead of concrete AbstractResult stubs.
/// The real implementation in beatoraja-result's AbstractResultData implements this trait.
///
/// Translated from Java: AbstractResult (field access pattern for result screens)
pub trait AbstractResultAccess {
    /// Get the new (current play) score data
    fn new_score(&self) -> &ScoreData;
    /// Get the old (previous best) score data
    fn old_score(&self) -> &ScoreData;
    /// Get the IR ranking position (0 if not ranked)
    fn ir_rank(&self) -> i32;
    /// Get the total number of IR players
    fn ir_total_player(&self) -> i32;
    /// Get the previous IR ranking position
    fn old_ir_rank(&self) -> i32;
}

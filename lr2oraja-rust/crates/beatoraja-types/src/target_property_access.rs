use crate::score_data::ScoreData;

/// Trait interface for TargetProperty access.
///
/// Downstream crates use `&dyn TargetPropertyAccess` instead of concrete
/// TargetProperty from beatoraja-play. This breaks the circular dependency
/// between beatoraja-core and beatoraja-play.
///
/// The concrete implementation lives in beatoraja-play::target_property.
pub trait TargetPropertyAccess: Send + Sync {
    /// Get the target property ID string (e.g., "RATE_AAA", "RIVAL_1", "IR_NEXT_1").
    fn id(&self) -> &str;

    /// Get display name for this target.
    /// May vary by context (e.g., rival name lookup).
    fn get_name_display(&self) -> String;

    /// Compute and return the target score data.
    /// This is called during gameplay to get the target score for comparison.
    fn get_target_score(&mut self) -> ScoreData;
}

/// Pre-computed distribution data for folder lamp/rank display.
///
/// Used by SkinDistributionGraph to render bar graphs without
/// requiring direct access to DirectoryBar (which lives in beatoraja-select).
#[derive(Clone, Debug, Default)]
pub struct DistributionData {
    /// Clear lamp distribution (11 entries: NoPlay..FullCombo)
    pub lamps: [i32; 11],
    /// Score rank distribution (28 entries)
    pub ranks: [i32; 28],
}

// ScoreHandoff - data transferred from BMSPlayer to PlayerResource via outbox pattern.
// Used when transitioning from Play to Result state.

use crate::groove_gauge::GrooveGauge;
use crate::score_data::ScoreData;

/// Data bundle produced by BMSPlayer at end of play for Result state consumption.
///
/// Transferred through the MainState outbox pattern: BMSPlayer populates this,
/// MainController reads it via `take_score_handoff()`, and writes to PlayerResource.
pub struct ScoreHandoff {
    /// Score data (None for autoplay or when no notes were hit)
    pub score_data: Option<ScoreData>,
    /// Course combo count
    pub combo: i32,
    /// Course max combo count
    pub maxcombo: i32,
    /// Gauge log per gauge type (Vec of gauge values sampled every 500ms)
    pub gauge: Vec<Vec<f32>>,
    /// Groove gauge state at end of play
    pub groove_gauge: Option<GrooveGauge>,
    /// Assist flags
    pub assist: i32,
}

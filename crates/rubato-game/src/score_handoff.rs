// ScoreHandoff - data transferred from BMSPlayer to PlayerResource via outbox pattern.
// Used when transitioning from Play to Result state.

use rubato_types::groove_gauge::GrooveGauge;
use rubato_types::replay_data::ReplayData;
use rubato_types::score_data::ScoreData;

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
    /// Whether frequency training is active (blocks score DB updates in result).
    pub freq_on: bool,
    /// Whether IR score submission should be blocked (e.g., frequency training active).
    pub force_no_ir_send: bool,
    /// Replay data populated with key input log and pattern info from the play session.
    /// Applied to PlayerResource.replay on handoff so save_replay_data() writes the live data.
    pub replay_data: Option<ReplayData>,
    /// BMSModel with judge states synced from JudgeManager.
    ///
    /// In Java, JudgeManager modifies Note objects in-place via shared references,
    /// so the model always has current state/play_time values. In Rust, these are
    /// synced explicitly and the updated model is passed through the handoff so that
    /// the result screen can read note states for timing distribution computation.
    pub updated_model: Option<bms::model::bms_model::BMSModel>,
    /// Recent judge timing offsets (milliseconds), 100-element circular buffer.
    /// Transferred so the result screen's SkinTimingVisualizer and SkinHitErrorVisualizer
    /// can display the scrolling judge offset visualization.
    pub recent_judges: Vec<i64>,
    /// Current write index into the recent_judges circular buffer.
    pub recent_judges_index: usize,
}

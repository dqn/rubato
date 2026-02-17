//! Judge windows, gauge calculations, score data, and clear type definitions.
//!
//! Provides [`JudgeProperty`] and [`JudgeAlgorithm`] for hit-timing evaluation,
//! [`GrooveGauge`] with [`GaugeProperty`] for life-bar management, [`ScoreData`]
//! for tracking per-play statistics, and [`ClearType`] for ranking play results.
//! Used by the play state to evaluate player input against chart timing.

mod clear_type;
pub mod gauge_property;
mod groove_gauge;
mod judge_algorithm;
pub mod judge_manager;
mod judge_property;
mod player_rule;
mod score_data;

pub use clear_type::ClearType;
pub use gauge_property::{
    GaugeElementProperty, GaugeModifier, GaugeProperty, GaugeType, GutsEntry,
};
pub use groove_gauge::{Gauge, GrooveGauge};
pub use judge_algorithm::JudgeAlgorithm;
pub use judge_property::{
    JudgeNoteType, JudgeProperty, JudgeWindow, JudgeWindowRule, JudgeWindowTable, MissCondition,
};
pub use player_rule::PlayerRule;
pub use score_data::ScoreData;

/// Number of judge categories: PG, GR, GD, BD, PR, MS.
pub const JUDGE_COUNT: usize = 6;

/// Judge index constants
pub const JUDGE_PG: usize = 0;
pub const JUDGE_GR: usize = 1;
pub const JUDGE_GD: usize = 2;
pub const JUDGE_BD: usize = 3;
pub const JUDGE_PR: usize = 4;
pub const JUDGE_MS: usize = 5;

// Shared render context helpers for music_result and course_result.
// Both result screens share identical logic for most SkinRenderContext methods;
// only state_type, current_play_config_ref, and string_value differ.

use rubato_core::clear_type::ClearType;

use super::abstract_result::AbstractResultData;
use super::stubs::{MainController, PlayerResource};

/// Map event IDs to replay slot indices.
/// Identical for both music and course result screens.
pub fn replay_index_from_event_id(event_id: i32) -> Option<usize> {
    match event_id {
        19 => Some(0),
        316 => Some(1),
        317 => Some(2),
        318 => Some(3),
        _ => None,
    }
}

/// Shared gauge_value computation for result render contexts.
#[inline]
pub fn gauge_value(data: &AbstractResultData) -> f32 {
    data.oldscore.play_option.gauge as f32 / 100.0
}

/// Shared gauge_type accessor.
#[inline]
pub fn gauge_type(data: &AbstractResultData) -> i32 {
    data.gauge_type
}

/// Shared judge_count accessor.
#[inline]
pub fn judge_count(data: &AbstractResultData, judge: i32, fast: bool) -> i32 {
    data.score
        .score
        .as_ref()
        .map_or(0, |s| s.judge_count(judge, fast))
}

/// Shared integer_value accessor for result render contexts.
pub fn integer_value(data: &AbstractResultData, timer_now: i64, id: i32) -> i32 {
    match id {
        // EX score
        71 => data.score.nowscore,
        // Max combo
        75 => data.score.score.as_ref().map_or(0, |s| s.maxcombo),
        // Miss count
        76 => data.score.score.as_ref().map_or(0, |s| s.minbp),
        // Total notes
        350 => data.score.totalnotes,
        // Playtime (hours/minutes/seconds from boot)
        17 => (timer_now / 3_600_000) as i32,
        18 => ((timer_now % 3_600_000) / 60_000) as i32,
        19 => ((timer_now % 60_000) / 1_000) as i32,
        _ => 0,
    }
}

/// Shared float_value accessor for result render contexts.
#[inline]
pub fn float_value(data: &AbstractResultData, id: i32) -> f32 {
    match id {
        // Score rate
        1102 => data.score.rate,
        _ => 0.0,
    }
}

/// Shared boolean_value accessor for result render contexts.
#[inline]
pub fn boolean_value(data: &AbstractResultData, id: i32) -> bool {
    match id {
        // Clear result
        90 => data.oldscore.clear >= ClearType::AssistEasy as i32,
        // Fail result
        91 => data.oldscore.clear < ClearType::AssistEasy as i32,
        _ => false,
    }
}

/// Shared SkinRenderContext property accessors that delegate to resource/main/data.
/// These methods are identical between music and course result render contexts.
pub fn player_config_ref(
    resource: &PlayerResource,
) -> Option<&rubato_types::player_config::PlayerConfig> {
    Some(resource.player_config())
}

pub fn config_ref(main: &MainController) -> Option<&rubato_types::config::Config> {
    Some(main.config())
}

pub fn replay_option_data(
    resource: &PlayerResource,
) -> Option<&rubato_types::replay_data::ReplayData> {
    resource.replay_data()
}

pub fn target_score_data(resource: &PlayerResource) -> Option<&rubato_core::score_data::ScoreData> {
    resource.target_score_data()
}

pub fn score_data_ref(data: &AbstractResultData) -> Option<&rubato_core::score_data::ScoreData> {
    data.score.score.as_ref()
}

pub fn rival_score_data_ref(
    data: &AbstractResultData,
) -> Option<&rubato_core::score_data::ScoreData> {
    Some(&data.oldscore)
}

pub fn song_data_ref(resource: &PlayerResource) -> Option<&rubato_types::song_data::SongData> {
    resource.songdata()
}

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
        // Average duration (ms integer part)
        372 => (data.avgduration / 1000) as i32,
        // Average duration (afterdot: tenths of ms)
        373 => ((data.avgduration / 100) % 10) as i32,
        // Timing average (ms integer part)
        374 => (data.avg / 1000) as i32,
        // Timing average (afterdot: tenths of ms)
        375 => ((data.avg / 100) % 10) as i32,
        // Timing stddev (ms integer part)
        376 => (data.stddev / 1000) as i32,
        // Timing stddev (afterdot: tenths of ms)
        377 => ((data.stddev / 100) % 10) as i32,
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
        90 => data.oldscore.clear >= ClearType::AssistEasy.id(),
        // Fail result
        91 => data.oldscore.clear < ClearType::AssistEasy.id(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_value_timing_stats_typical() {
        let mut data = AbstractResultData::new();
        // 2500 us = 2.5 ms
        data.avgduration = 2500;
        // -1300 us = -1.3 ms
        data.avg = -1300;
        // 4700 us = 4.7 ms
        data.stddev = 4700;

        // 372: duration_average integer part = 2500 / 1000 = 2
        assert_eq!(integer_value(&data, 0, 372), 2);
        // 373: duration_average afterdot = (2500 / 100) % 10 = 25 % 10 = 5
        assert_eq!(integer_value(&data, 0, 373), 5);

        // 374: timing_average integer part = -1300 / 1000 = -1
        assert_eq!(integer_value(&data, 0, 374), -1);
        // 375: timing_average afterdot = (-1300 / 100) % 10 = -13 % 10 = -3
        assert_eq!(integer_value(&data, 0, 375), -3);

        // 376: timing_stddev integer part = 4700 / 1000 = 4
        assert_eq!(integer_value(&data, 0, 376), 4);
        // 377: timing_stddev afterdot = (4700 / 100) % 10 = 47 % 10 = 7
        assert_eq!(integer_value(&data, 0, 377), 7);
    }

    #[test]
    fn test_integer_value_timing_stats_zero() {
        let data = AbstractResultData::new();

        assert_eq!(integer_value(&data, 0, 372), 0);
        assert_eq!(integer_value(&data, 0, 373), 0);
        assert_eq!(integer_value(&data, 0, 374), 0);
        assert_eq!(integer_value(&data, 0, 375), 0);
        assert_eq!(integer_value(&data, 0, 376), 0);
        assert_eq!(integer_value(&data, 0, 377), 0);
    }

    #[test]
    fn test_integer_value_timing_stats_large_values() {
        let mut data = AbstractResultData::new();
        // 12345 us = 12.3 ms (with remainder 45)
        data.avgduration = 12345;
        data.avg = 12345;
        data.stddev = 12345;

        assert_eq!(integer_value(&data, 0, 372), 12);
        assert_eq!(integer_value(&data, 0, 373), 3);
        assert_eq!(integer_value(&data, 0, 374), 12);
        assert_eq!(integer_value(&data, 0, 375), 3);
        assert_eq!(integer_value(&data, 0, 376), 12);
        assert_eq!(integer_value(&data, 0, 377), 3);
    }

    #[test]
    fn test_integer_value_existing_ids_unchanged() {
        let data = AbstractResultData::new();

        // Verify unknown IDs still return 0
        assert_eq!(integer_value(&data, 0, 999), 0);

        // Verify playtime IDs still work
        // 3_661_000 ms = 1h 1m 1s
        assert_eq!(integer_value(&data, 3_661_000, 17), 1);
        assert_eq!(integer_value(&data, 3_661_000, 18), 1);
        assert_eq!(integer_value(&data, 3_661_000, 19), 1);
    }

    #[test]
    fn test_boolean_value_clear_uses_id_not_discriminant() {
        // Regression: boolean_value must use ClearType::AssistEasy.id() (== 2),
        // not `ClearType::AssistEasy as i32` which relies on implicit discriminant
        // ordering and could silently diverge if the enum is reordered.
        let assist_easy_id = ClearType::AssistEasy.id();

        // ID 90: clear result (clear >= AssistEasy)
        // ID 91: fail result  (clear <  AssistEasy)
        let mut data = AbstractResultData::new();

        // Exactly at AssistEasy threshold -> cleared
        data.oldscore.clear = assist_easy_id;
        assert!(boolean_value(&data, 90));
        assert!(!boolean_value(&data, 91));

        // Above threshold -> cleared
        data.oldscore.clear = assist_easy_id + 1;
        assert!(boolean_value(&data, 90));
        assert!(!boolean_value(&data, 91));

        // Below threshold -> failed
        data.oldscore.clear = assist_easy_id - 1;
        assert!(!boolean_value(&data, 90));
        assert!(boolean_value(&data, 91));

        // NoPlay (0) -> failed
        data.oldscore.clear = ClearType::NoPlay.id();
        assert!(!boolean_value(&data, 90));
        assert!(boolean_value(&data, 91));
    }
}

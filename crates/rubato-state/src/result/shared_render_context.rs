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
/// Returns the gauge fill percentage from the GrooveGauge (0.0-100.0),
/// matching the play screen's behavior.
#[inline]
pub fn gauge_value(resource: &PlayerResource) -> f32 {
    resource.groove_gauge().map_or(0.0, |g| g.value())
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
///
/// Java reference (BooleanPropertyFactory):
///   OPTION_RESULT_CLEAR (90): score.getClear() != Failed.id
///   OPTION_RESULT_FAIL  (91): score.getClear() == Failed.id
/// where `score` is getPlayerResource().getScoreData() (the current play's score).
#[inline]
pub fn boolean_value(data: &AbstractResultData, id: i32) -> bool {
    match id {
        // Clear result: current play's clear != Failed
        90 => data
            .score
            .score
            .as_ref()
            .is_some_and(|s| s.clear != ClearType::Failed.id()),
        // Fail result: current play's clear == Failed
        91 => data
            .score
            .score
            .as_ref()
            .is_none_or(|s| s.clear == ClearType::Failed.id()),
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

/// Returns the clear type ID for the ranking score at the given slot
/// (0-based, relative to the ranking offset stored in AbstractResultData).
pub fn ranking_score_clear_type(data: &AbstractResultData, slot: i32) -> i32 {
    if let Some(ref ranking) = data.ranking {
        let index = data.ranking_offset + slot;
        ranking
            .score(index)
            .map(|score| score.clear.id())
            .unwrap_or(-1)
    } else {
        -1
    }
}

/// Returns the current ranking display offset.
pub fn ranking_offset(data: &AbstractResultData) -> i32 {
    data.ranking_offset
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
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
    fn test_boolean_value_uses_current_play_score_not_oldscore() {
        // Regression: boolean_value IDs 90/91 must check the current play's
        // score (data.score.score), not the old best score (data.oldscore).
        // Java: OPTION_RESULT_CLEAR(90) = score.getClear() != Failed.id
        //       OPTION_RESULT_FAIL(91)  = score.getClear() == Failed.id
        let failed_id = ClearType::Failed.id();

        let mut data = AbstractResultData::new();

        // No score data -> ID 90 false (not cleared), ID 91 true (failed)
        assert!(!boolean_value(&data, 90));
        assert!(boolean_value(&data, 91));

        // Current play cleared (AssistEasy) -> clear=true, fail=false
        let mut score = rubato_core::score_data::ScoreData::default();
        score.clear = ClearType::AssistEasy.id();
        data.score.score = Some(score.clone());
        assert!(boolean_value(&data, 90));
        assert!(!boolean_value(&data, 91));

        // Current play cleared (FullCombo) -> clear=true, fail=false
        score.clear = ClearType::FullCombo.id();
        data.score.score = Some(score.clone());
        assert!(boolean_value(&data, 90));
        assert!(!boolean_value(&data, 91));

        // Current play failed -> clear=false, fail=true
        score.clear = failed_id;
        data.score.score = Some(score.clone());
        assert!(!boolean_value(&data, 90));
        assert!(boolean_value(&data, 91));

        // NoPlay (0) is not Failed (1) -> clear=true, fail=false
        score.clear = ClearType::NoPlay.id();
        data.score.score = Some(score);
        assert!(boolean_value(&data, 90));
        assert!(!boolean_value(&data, 91));

        // Verify oldscore does NOT affect the result
        data.oldscore.clear = failed_id;
        assert!(boolean_value(&data, 90), "oldscore must not affect ID 90");
        assert!(!boolean_value(&data, 91), "oldscore must not affect ID 91");
    }

    // ============================================================
    // ranking_score_clear_type tests
    // ============================================================

    fn make_ranking_with_scores() -> rubato_ir::ranking_data::RankingData {
        use rubato_ir::ir_score_data::IRScoreData;
        use rubato_ir::ranking_data::RankingData;

        let mut rd = RankingData::new();
        let scores: Vec<IRScoreData> = vec![
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.judge_counts.epg = 100;
                s.judge_counts.lpg = 100;
                s.clear = ClearType::FullCombo.id(); // 8
                IRScoreData::new(&s)
            },
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.judge_counts.epg = 80;
                s.judge_counts.lpg = 80;
                s.clear = ClearType::Hard.id(); // 6
                IRScoreData::new(&s)
            },
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.judge_counts.epg = 50;
                s.judge_counts.lpg = 50;
                s.clear = ClearType::Easy.id(); // 4
                IRScoreData::new(&s)
            },
        ];
        rd.update_score(&scores, None);
        rd
    }

    #[test]
    fn test_ranking_score_clear_type_returns_clear_for_each_slot() {
        let mut data = AbstractResultData::new();
        data.ranking = Some(make_ranking_with_scores());
        data.ranking_offset = 0;

        assert_eq!(ranking_score_clear_type(&data, 0), 8); // FullCombo
        assert_eq!(ranking_score_clear_type(&data, 1), 6); // Hard
        assert_eq!(ranking_score_clear_type(&data, 2), 4); // Easy
    }

    #[test]
    fn test_ranking_score_clear_type_respects_offset() {
        let mut data = AbstractResultData::new();
        data.ranking = Some(make_ranking_with_scores());
        data.ranking_offset = 1;

        assert_eq!(ranking_score_clear_type(&data, 0), 6); // Hard (offset 1 + slot 0)
        assert_eq!(ranking_score_clear_type(&data, 1), 4); // Easy (offset 1 + slot 1)
        assert_eq!(ranking_score_clear_type(&data, 2), -1); // out of bounds
    }

    #[test]
    fn test_ranking_score_clear_type_returns_minus_one_when_no_ranking() {
        let data = AbstractResultData::new();
        assert!(data.ranking.is_none());

        for slot in 0..10 {
            assert_eq!(
                ranking_score_clear_type(&data, slot),
                -1,
                "slot {} should return -1",
                slot
            );
        }
    }

    #[test]
    fn test_ranking_offset_returns_data_offset() {
        let mut data = AbstractResultData::new();
        assert_eq!(ranking_offset(&data), 0);

        data.ranking_offset = 5;
        assert_eq!(ranking_offset(&data), 5);
    }

    // ============================================================
    // gauge_value tests
    // ============================================================

    /// Mock PlayerResourceAccess that holds an optional GrooveGauge
    /// for testing gauge_value().
    struct GaugeTestResourceAccess {
        config: rubato_types::config::Config,
        player_config: rubato_types::player_config::PlayerConfig,
        groove_gauge: Option<rubato_types::groove_gauge::GrooveGauge>,
        course_gauge: Vec<Vec<Vec<f32>>>,
        course_replay: Vec<rubato_core::replay_data::ReplayData>,
    }

    impl rubato_types::player_resource_access::PlayerResourceAccess for GaugeTestResourceAccess {
        fn into_any_send(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }
        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }
        fn score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }
        fn rival_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }
        fn target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }
        fn course_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }
        fn set_course_score_data(&mut self, _score: rubato_core::score_data::ScoreData) {}
        fn songdata(&self) -> Option<&rubato_types::song_data::SongData> {
            None
        }
        fn songdata_mut(&mut self) -> Option<&mut rubato_types::song_data::SongData> {
            None
        }
        fn set_songdata(&mut self, _data: Option<rubato_types::song_data::SongData>) {}
        fn replay_data(&self) -> Option<&rubato_core::replay_data::ReplayData> {
            None
        }
        fn replay_data_mut(&mut self) -> Option<&mut rubato_core::replay_data::ReplayData> {
            None
        }
        fn course_replay(&self) -> &[rubato_core::replay_data::ReplayData] {
            &self.course_replay
        }
        fn add_course_replay(&mut self, rd: rubato_core::replay_data::ReplayData) {
            self.course_replay.push(rd);
        }
        fn course_data(&self) -> Option<&rubato_types::course_data::CourseData> {
            None
        }
        fn course_index(&self) -> usize {
            0
        }
        fn next_course(&mut self) -> bool {
            false
        }
        fn constraint(&self) -> Vec<rubato_types::course_data::CourseDataConstraint> {
            vec![]
        }
        fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
            None
        }
        fn groove_gauge(&self) -> Option<&rubato_types::groove_gauge::GrooveGauge> {
            self.groove_gauge.as_ref()
        }
        fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
            &self.course_gauge
        }
        fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>) {
            self.course_gauge.push(gauge);
        }
        fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
            &mut self.course_gauge
        }
        fn score_data_mut(&mut self) -> Option<&mut rubato_core::score_data::ScoreData> {
            None
        }
        fn course_replay_mut(&mut self) -> &mut Vec<rubato_core::replay_data::ReplayData> {
            &mut self.course_replay
        }
        fn maxcombo(&self) -> i32 {
            0
        }
        fn org_gauge_option(&self) -> i32 {
            0
        }
        fn set_org_gauge_option(&mut self, _val: i32) {}
        fn assist(&self) -> i32 {
            0
        }
        fn is_update_score(&self) -> bool {
            false
        }
        fn is_update_course_score(&self) -> bool {
            false
        }
        fn is_force_no_ir_send(&self) -> bool {
            false
        }
        fn is_freq_on(&self) -> bool {
            false
        }
        fn reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }
        fn reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }
        fn clear(&mut self) {}
        fn set_bms_file(
            &mut self,
            _path: &std::path::Path,
            _mode_type: i32,
            _mode_id: i32,
        ) -> bool {
            false
        }
        fn set_course_bms_files(&mut self, _files: &[std::path::PathBuf]) -> bool {
            false
        }
        fn set_tablename(&mut self, _name: &str) {}
        fn set_tablelevel(&mut self, _level: &str) {}
        fn set_rival_score_data_option(
            &mut self,
            _score: Option<rubato_core::score_data::ScoreData>,
        ) {
        }
        fn set_chart_option_data(&mut self, _option: Option<rubato_core::replay_data::ReplayData>) {
        }
        fn set_course_data(&mut self, _data: rubato_types::course_data::CourseData) {}
        fn clear_course_data(&mut self) {}
        fn course_song_data(&self) -> Vec<rubato_types::song_data::SongData> {
            vec![]
        }
    }

    fn make_resource_with_gauge(gauge_value: f32) -> PlayerResource {
        use rubato_types::gauge_property::GaugeProperty;

        let model = bms_model::bms_model::BMSModel::new();
        let mut gg = rubato_types::groove_gauge::GrooveGauge::new(
            &model,
            rubato_types::groove_gauge::NORMAL,
            &GaugeProperty::SevenKeys,
        );
        gg.set_value(gauge_value);
        PlayerResource::new(
            Box::new(GaugeTestResourceAccess {
                config: rubato_types::config::Config::default(),
                player_config: rubato_types::player_config::PlayerConfig::default(),
                groove_gauge: Some(gg),
                course_gauge: Vec::new(),
                course_replay: Vec::new(),
            }),
            crate::result::stubs::BMSPlayerMode::new(crate::result::stubs::BMSPlayerModeType::Play),
        )
    }

    #[test]
    fn test_gauge_value_returns_fill_percentage_not_type() {
        // Regression: gauge_value must return the gauge fill percentage
        // from GrooveGauge, not the gauge type from PlayOption.gauge.
        let resource = make_resource_with_gauge(75.0);
        let value = gauge_value(&resource);
        // Should return the fill percentage (75.0), not a gauge type / 100
        assert!(
            (value - 75.0).abs() < 0.01,
            "gauge_value should return fill percentage 75.0, got {}",
            value,
        );
    }

    #[test]
    fn test_gauge_value_zero_when_no_groove_gauge() {
        let resource = PlayerResource::default();
        assert_eq!(gauge_value(&resource), 0.0);
    }

    #[test]
    fn test_gauge_value_full_gauge() {
        let resource = make_resource_with_gauge(100.0);
        let value = gauge_value(&resource);
        assert!(
            (value - 100.0).abs() < 0.01,
            "gauge_value should return 100.0 for full gauge, got {}",
            value,
        );
    }

    #[test]
    fn test_gauge_value_at_minimum() {
        // NORMAL gauge has min=2.0, so set_value(2.0) stays at minimum.
        let resource = make_resource_with_gauge(2.0);
        let value = gauge_value(&resource);
        assert!(
            (value - 2.0).abs() < 0.01,
            "gauge_value should return 2.0 at minimum gauge, got {}",
            value,
        );
    }
}

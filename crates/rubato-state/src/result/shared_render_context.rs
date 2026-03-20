// Shared render context helpers for music_result and course_result.
// Both result screens share identical logic for most SkinRenderContext methods;
// only state_type, current_play_config_ref, and string_value differ.

use rubato_core::clear_type::ClearType;
use rubato_core::score_data::ScoreData;

use super::abstract_result::{AbstractResultData, STATE_OFFLINE};
use super::{MainController, PlayerResource};

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

/// Returns whether the gauge reached max value.
/// Used by skin properties to determine MAX PG judge display on result screens.
#[inline]
pub fn is_gauge_max(resource: &PlayerResource) -> bool {
    resource.groove_gauge().is_some_and(|g| g.gauge().is_max())
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

/// Helper: returns `i32::MIN` when score data is absent, else applies `f` to the score.
#[inline]
fn with_score(data: &AbstractResultData, f: impl FnOnce(&ScoreData) -> i32) -> i32 {
    data.score.score.as_ref().map_or(i32::MIN, f)
}

/// Helper: returns `i32::MIN` when rival (old best) score data's judge info is
/// unavailable, else applies `f`.
#[inline]
fn with_rival_score(data: &AbstractResultData, f: impl FnOnce(&ScoreData) -> i32) -> i32 {
    if let Some(ref rival) = data.score.rival {
        f(rival)
    } else {
        i32::MIN
    }
}

/// Shared integer_value accessor for result render contexts.
///
/// Java reference: IntegerPropertyFactory (getIntegerProperty / getIntegerProperty0 /
/// ValueType enum). IDs are dispatched for `AbstractResult` (both MusicResult and
/// CourseResult).
pub fn integer_value(
    data: &AbstractResultData,
    timer_now: i64,
    cumulative_playtime_seconds: i64,
    id: i32,
) -> i32 {
    match id {
        // ---- EX score (NUMBER_SCORE / SCORE2 / SCORE3) ----
        // Java: AbstractResult -> getNewScore().getExscore()
        71 | 101 | 171 => data.score.score.as_ref().map_or(i32::MIN, |s| s.exscore()),

        // ---- Point / score (NUMBER_POINT) ----
        // Java: getScoreDataProperty().getNowScore()
        100 => data.score.nowpoint,

        // ---- Max score (NUMBER_MAXSCORE) ----
        // Java: score.getNotes() * 2
        72 => with_score(data, |s| s.notes * 2),

        // ---- Max combo (NUMBER_MAXCOMBO / MAXCOMBO2 / MAXCOMBO3) ----
        // Java: AbstractResult -> getNewScore().getCombo()
        75 | 105 | 174 => data.score.score.as_ref().map_or(i32::MIN, |s| s.maxcombo),

        // ---- Miss count (NUMBER_MISSCOUNT / MISSCOUNT2) ----
        // Java: AbstractResult -> getNewScore().getMinbp()
        76 | 177 => data.score.score.as_ref().map_or(i32::MIN, |s| s.minbp),

        // ---- Score rate (NUMBER_SCORE_RATE) ----
        // Java: score != null ? getNowRateInt() : Integer.MIN_VALUE
        102 => with_score(data, |_| data.score.nowrate_int),

        // ---- Score rate afterdot (NUMBER_SCORE_RATE_AFTERDOT) ----
        103 => with_score(data, |_| data.score.nowrate_after_dot),

        // ---- Total rate (NUMBER_TOTAL_RATE / NUMBER_SCORE_RATE2) ----
        // Java: score != null ? getRateInt() : Integer.MIN_VALUE
        115 | 155 => with_score(data, |_| data.score.rate_int),

        // ---- Total rate afterdot (NUMBER_TOTAL_RATE_AFTERDOT / NUMBER_SCORE_RATE_AFTERDOT2) ----
        116 | 156 => with_score(data, |_| data.score.rate_after_dot),

        // ---- Best rate (NUMBER_BEST_RATE) ----
        183 => data.score.bestrate_int,

        // ---- Best rate afterdot (NUMBER_BEST_RATE_AFTERDOT) ----
        184 => data.score.bestrate_after_dot,

        // ---- High score / old best EX score (NUMBER_HIGHSCORE / HIGHSCORE2) ----
        // Java: AbstractResult -> getOldScore().getExscore()
        150 | 170 => data.oldscore.exscore(),

        // ---- Target / rival score (NUMBER_TARGET_SCORE / TARGET_SCORE2 / RIVAL_SCORE) ----
        // Java: getScoreDataProperty().getRivalScore()
        121 | 151 | 271 => data.score.rivalscore,

        // ---- Target / rival score rate (NUMBER_TARGET_SCORE_RATE / TARGET_TOTAL_RATE / TARGET_SCORE_RATE2) ----
        122 | 157 => data.score.rivalrate_int,

        // ---- Target / rival score rate afterdot ----
        123 | 158 => data.score.rivalrate_after_dot,

        // ---- Diff vs target (NUMBER_DIFF_EXSCORE / DIFF_EXSCORE2 / DIFF_TARGETSCORE) ----
        // Java: nowEXScore - nowRivalScore
        108 | 128 | 153 => data.score.nowscore - data.score.nowrivalscore,

        // ---- Diff vs high score (NUMBER_DIFF_HIGHSCORE / DIFF_HIGHSCORE2) ----
        // Java: nowEXScore - nowBestScore
        152 | 172 => data.score.nowscore - data.score.nowbestscore,

        // ---- Diff next rank (NUMBER_DIFF_NEXTRANK) ----
        154 => data.score.nextrank,

        // ---- Clear type (NUMBER_CLEAR) ----
        // Java: AbstractResult -> getNewScore().getClear()
        370 => data.score.score.as_ref().map_or(i32::MIN, |s| s.clear),

        // ---- Target clear (NUMBER_TARGET_CLEAR) ----
        // Java: getOldScore().getClear()
        371 => data.oldscore.clear,

        // ---- IR ranking EX score (ranking_exscore1-10: 380-389) ----
        // Java: RankingData.getScore(offset + slot).getExscore()
        380..=389 => ranking_exscore(data, id - 380),

        // ---- IR ranking order (ranking_index1-10: 390-399) ----
        // Java: RankingData.getScoreRanking(offset + slot)
        // Image-index refs with the same IDs are handled separately by
        // SkinRenderContext::image_index_value() and still map to clear lamps.
        390..=399 => ranking_index(data, id - 390),

        // ---- Target max combo (NUMBER_TARGET_MAXCOMBO) ----
        // Java: oldScore.getCombo() > 0 ? combo : Integer.MIN_VALUE
        173 => {
            let combo = data.oldscore.maxcombo;
            if combo > 0 { combo } else { i32::MIN }
        }

        // ---- Diff max combo (NUMBER_DIFF_MAXCOMBO) ----
        // Java: oldCombo > 0 ? newCombo - oldCombo : Integer.MIN_VALUE
        175 => {
            let old_combo = data.oldscore.maxcombo;
            if old_combo > 0 {
                data.score
                    .score
                    .as_ref()
                    .map_or(i32::MIN, |s| s.maxcombo - old_combo)
            } else {
                i32::MIN
            }
        }

        // ---- Target miss count (NUMBER_TARGET_MISSCOUNT) ----
        // Java: oldScore.getMinbp() != Integer.MAX_VALUE ? minbp : Integer.MIN_VALUE
        176 => {
            let minbp = data.oldscore.minbp;
            if minbp != i32::MAX { minbp } else { i32::MIN }
        }

        // ---- Diff miss count (NUMBER_DIFF_MISSCOUNT) ----
        // Java: oldMinbp != MAX_VALUE ? newMinbp - oldMinbp : Integer.MIN_VALUE
        178 => {
            let old_minbp = data.oldscore.minbp;
            if old_minbp != i32::MAX {
                data.score
                    .score
                    .as_ref()
                    .map_or(i32::MIN, |s| s.minbp - old_minbp)
            } else {
                i32::MIN
            }
        }

        // ---- Judge counts from score data (NUMBER_PERFECT2..NUMBER_POOR2: 80-84) ----
        // Java: score != null ? score.getJudgeCount(index) : Integer.MIN_VALUE
        80..=84 => {
            let index = id - 80;
            with_score(data, |s| s.judge_count_total(index))
        }

        // ---- Judge count rates (NUMBER_PERFECT_RATE..NUMBER_POOR_RATE: 85-89) ----
        // Java: score != null && notes > 0 ? count * 100 / notes : Integer.MIN_VALUE
        85..=89 => {
            let index = id - 85;
            with_score(data, |s| {
                if s.notes > 0 {
                    s.judge_count_total(index) * 100 / s.notes
                } else {
                    i32::MIN
                }
            })
        }

        // ---- Judge counts via state (NUMBER_PERFECT..NUMBER_POOR: 110-114) ----
        // Java: state.getJudgeCount(index, true) + state.getJudgeCount(index, false)
        // On result screens, getJudgeCount delegates to score data.
        110..=114 => {
            let index = id - 110;
            judge_count(data, index, true) + judge_count(data, index, false)
        }

        // ---- Early/late judge counts (NUMBER_EARLY_PERFECT..NUMBER_LATE_POOR: 410-419) ----
        // Java: state.getJudgeCount(index, early)
        // Even IDs (410,412,414,416,418) = early; odd IDs (411,413,415,417,419) = late
        410..=419 => {
            let offset = id - 410;
            let index = offset / 2;
            let early = offset % 2 == 0;
            judge_count(data, index, early)
        }

        // ---- Total early (NUMBER_TOTALEARLY: 423) ----
        // Java: sum of getJudgeCount(i, true) for i in 1..=5
        423 => {
            let mut total = 0;
            for i in 1..6 {
                total += judge_count(data, i, true);
            }
            total
        }

        // ---- Total late (NUMBER_TOTALLATE: 424) ----
        // Java: sum of getJudgeCount(i, false) for i in 1..=5
        424 => {
            let mut total = 0;
            for i in 1..6 {
                total += judge_count(data, i, false);
            }
            total
        }

        // ---- Combo break (NUMBER_COMBOBREAK: 425) ----
        // Java: BD(early+late) + PR(early+late)
        425 => {
            judge_count(data, 3, true)
                + judge_count(data, 3, false)
                + judge_count(data, 4, true)
                + judge_count(data, 4, false)
        }

        // ---- Rival judge counts (NUMBER_RIVAL_PERFECT..NUMBER_RIVAL_POOR: 280-284) ----
        // Java: rivalScoreData != null ? rivalScore.getJudgeCount(index) : Integer.MIN_VALUE
        280..=284 => {
            let index = id - 280;
            with_rival_score(data, |s| s.judge_count_total(index))
        }

        // ---- Rival judge count rates (NUMBER_RIVAL_PERFECT_RATE..NUMBER_RIVAL_POOR_RATE: 285-289) ----
        285..=289 => {
            let index = id - 285;
            with_rival_score(data, |s| {
                if s.notes > 0 {
                    s.judge_count_total(index) * 100 / s.notes
                } else {
                    i32::MIN
                }
            })
        }

        // ---- IR rank (ir_rank: 179) ----
        // Java: state != OFFLINE ? getIRRank() : Integer.MIN_VALUE
        179 => {
            if data.state != STATE_OFFLINE {
                data.ir_rank()
            } else {
                i32::MIN
            }
        }

        // ---- IR previous rank (ir_prevrank: 182) ----
        182 => {
            if data.state != STATE_OFFLINE {
                data.old_ir_rank()
            } else {
                i32::MIN
            }
        }

        // ---- IR total player (NUMBER_IR_TOTALPLAYER / IR_TOTALPLAYER2: 180 / 200) ----
        180 | 200 => {
            if data.state != STATE_OFFLINE {
                data.ir_total_player()
            } else {
                i32::MIN
            }
        }

        // ---- Total notes (NUMBER_TOTALNOTES / TOTALNOTES2: 74 / 106) ----
        // Java: songdata.getNotes() (for non-course). data.score.totalnotes is
        // pre-computed from the model, which matches songdata.notes on result screens.
        74 | 106 => data.score.totalnotes,

        // ---- Chart total notes (existing, from SongData information) ----
        350 => data.score.totalnotes,

        // ---- Cumulative playtime (hours/minutes/seconds from PlayerData, in seconds) ----
        // Java: PlayerData.getPlaytime() / 3600, / 60 % 60, % 60
        17 => (cumulative_playtime_seconds / 3600) as i32,
        18 => ((cumulative_playtime_seconds / 60) % 60) as i32,
        19 => (cumulative_playtime_seconds % 60) as i32,

        // ---- Average duration (ms integer part) ----
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

        // ---- Global IDs 20-26: FPS and system date/time ----
        // Java IntegerPropertyFactory defines these as global lambdas (all screens).
        // Since this is a free function (not a trait method), we inline the logic
        // from SkinRenderContext::default_integer_value().
        20 => rubato_types::fps_counter::current_fps(),
        21 => {
            let now = chrono::Local::now();
            chrono::Datelike::year(&now)
        }
        22 => {
            let now = chrono::Local::now();
            chrono::Datelike::month(&now) as i32
        }
        23 => {
            let now = chrono::Local::now();
            chrono::Datelike::day(&now) as i32
        }
        24 => {
            let now = chrono::Local::now();
            chrono::Timelike::hour(&now) as i32
        }
        25 => {
            let now = chrono::Local::now();
            chrono::Timelike::minute(&now) as i32
        }
        26 => {
            let now = chrono::Local::now();
            chrono::Timelike::second(&now) as i32
        }

        // ---- Boot time (hours/minutes/seconds since application start) ----
        // Java: main.getPlayTime() returns ms since boot
        27 => (timer_now / 3_600_000) as i32,
        28 => ((timer_now % 3_600_000) / 60_000) as i32,
        29 => ((timer_now % 60_000) / 1_000) as i32,

        _ => 0,
    }
}

/// Shared float_value accessor for result render contexts.
///
/// Java reference: FloatPropertyFactory (FloatType / RateType enums).
/// IDs dispatched here are state-independent (only need AbstractResultData).
/// IDs that need PlayerResource (e.g. 1107 / groove gauge) are handled in
/// the individual render contexts (music_result, course_result).
pub fn float_value(data: &AbstractResultData, id: i32) -> Option<f32> {
    match id {
        // ---- Score rate (FLOAT_SCORE_RATE: 1102) ----
        // Java: score != null ? getNowRate() : Float.MIN_VALUE
        1102 => Some({
            if data.score.score.is_some() {
                data.score.nowrate
            } else {
                f32::MIN
            }
        }),

        // ---- Total rate (FLOAT_TOTAL_RATE: 1115) ----
        // Java: score != null ? getRate() : Float.MIN_VALUE
        1115 => Some({
            if data.score.score.is_some() {
                data.score.rate
            } else {
                f32::MIN
            }
        }),

        // ---- Score rate 2 (FLOAT_SCORE_RATE2: 155) ----
        // Java: same as total_rate
        155 => Some({
            if data.score.score.is_some() {
                data.score.rate
            } else {
                f32::MIN
            }
        }),

        // ---- Score rate (RateType scorerate: 110) ----
        // Java: getScoreDataProperty().getRate()
        110 => Some(data.score.rate),

        // ---- Score rate final (RateType scorerate_final: 111) ----
        // Java: getScoreDataProperty().getNowRate()
        111 => Some(data.score.nowrate),

        // ---- Best score rate now (RateType bestscorerate_now: 112) ----
        // Java: getScoreDataProperty().getNowBestScoreRate()
        112 => Some(data.score.nowbestscorerate),

        // ---- Best score rate (RateType bestscorerate: 113 / FloatType best_rate: 183) ----
        // Java: getScoreDataProperty().getBestScoreRate()
        113 | 183 => Some(data.score.bestscorerate),

        // ---- Target score rate now (RateType targetscorerate_now: 114) ----
        // Java: getScoreDataProperty().getNowRivalScoreRate()
        114 => Some(data.score.nowrivalscorerate),

        // ---- Target score rate (RateType targetscorerate: 115 / FloatType rival_rate: 122 / target_rate: 135 / target_rate2: 157) ----
        // Java: getScoreDataProperty().getRivalScoreRate()
        115 | 122 | 135 | 157 => Some(data.score.rivalscorerate),

        // ---- Judge rates from score (FloatType perfect_rate..poor_rate: 85-89) ----
        // Java: score != null && notes > 0 ? judgeCount(j) / notes : Float.MIN_VALUE
        85..=89 => {
            let index = id - 85;
            Some(if let Some(ref s) = data.score.score {
                if s.notes > 0 {
                    s.judge_count_total(index) as f32 / s.notes as f32
                } else {
                    f32::MIN
                }
            } else {
                f32::MIN
            })
        }

        // ---- Rival judge rates (FloatType rival_perfect_rate..rival_poor_rate: 285-289) ----
        // Java: rivalScore != null && notes > 0 ? count / notes : Float.MIN_VALUE
        285..=289 => {
            let index = id - 285;
            Some(if let Some(ref s) = data.score.rival {
                if s.notes > 0 {
                    s.judge_count_total(index) as f32 / s.notes as f32
                } else {
                    f32::MIN
                }
            } else {
                f32::MIN
            })
        }

        _ => None,
    }
}

/// Shared boolean_value accessor for result render contexts.
///
/// Java reference (BooleanPropertyFactory):
///   OPTION_RESULT_CLEAR (90): score.getClear() != Failed.id && (cscore == null || cscore.getClear() != Failed.id)
///   OPTION_RESULT_FAIL  (91): score.getClear() == Failed.id || (cscore != null && cscore.getClear() == Failed.id)
/// where `score` is getPlayerResource().getScoreData() and `cscore` is getCourseScoreData().
#[inline]
pub fn boolean_value(data: &AbstractResultData, course_score: Option<&ScoreData>, id: i32) -> bool {
    match id {
        // Clear result: current play's clear != Failed AND course aggregate (if present) != Failed
        90 => {
            data.score
                .score
                .as_ref()
                .is_some_and(|s| s.clear != ClearType::Failed.id())
                && course_score.is_none_or(|cs| cs.clear != ClearType::Failed.id())
        }
        // Fail result: current play's clear == Failed OR course aggregate (if present) == Failed
        91 => {
            data.score
                .score
                .as_ref()
                .is_none_or(|s| s.clear == ClearType::Failed.id())
                || course_score.is_some_and(|cs| cs.clear == ClearType::Failed.id())
        }
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

/// Returns the player name for the ranking score at the given slot.
pub fn ranking_name(data: &AbstractResultData, slot: i32) -> String {
    if let Some(ref ranking) = data.ranking {
        let index = data.ranking_offset + slot;
        ranking
            .score(index)
            .map(|score| score.player.clone())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Returns the EX score for the ranking score at the given slot.
pub fn ranking_exscore(data: &AbstractResultData, slot: i32) -> i32 {
    if let Some(ref ranking) = data.ranking {
        let index = data.ranking_offset + slot;
        ranking
            .score(index)
            .map(|score| score.exscore())
            .unwrap_or(i32::MIN)
    } else {
        i32::MIN
    }
}

/// Returns the displayed ranking number for the ranking score at the given slot.
pub fn ranking_index(data: &AbstractResultData, slot: i32) -> i32 {
    if let Some(ref ranking) = data.ranking {
        ranking.score_ranking(data.ranking_offset + slot)
    } else {
        i32::MIN
    }
}

/// Returns the current ranking display offset.
pub fn ranking_offset(data: &AbstractResultData) -> i32 {
    data.ranking_offset
}

/// Returns gauge history from the player resource.
/// Used by SkinGaugeGraphObject::prepare() on result screens.
pub fn gauge_history(resource: &PlayerResource) -> Option<&Vec<Vec<f32>>> {
    resource.gauge()
}

/// Returns (border, max) for the current gauge type.
/// Used by SkinGaugeGraphObject::prepare() on result screens.
pub fn gauge_border_max(resource: &PlayerResource, gauge_type: i32) -> Option<(f32, f32)> {
    let gauge = resource.groove_gauge()?;
    let prop = gauge.gauge_by_type(gauge_type).property();
    Some((prop.border, prop.max))
}

/// Returns the minimum gauge value for the current gauge type.
/// Used by SkinGauge for the result-screen fill animation (Java: getProperty().min).
pub fn gauge_min(resource: &PlayerResource, gauge_type: i32) -> f32 {
    resource
        .groove_gauge()
        .map_or(0.0, |g| g.gauge_by_type(gauge_type).property().min)
}

/// Returns the cached rubato_types TimingDistribution for the result screen.
pub fn get_timing_distribution(
    data: &AbstractResultData,
) -> Option<&rubato_types::timing_distribution::TimingDistribution> {
    if data.timing_distribution_cache.distribution.is_empty() {
        None
    } else {
        Some(&data.timing_distribution_cache)
    }
}

/// Returns the ScoreDataProperty for the result screen.
pub fn score_data_property(
    data: &AbstractResultData,
) -> &rubato_types::score_data_property::ScoreDataProperty {
    &data.score
}

/// Returns (border, max) for each gauge type from the GrooveGauge.
/// Used by SkinGauge to adjust parts count so borders divide evenly on result screens.
pub fn gauge_element_borders(resource: &PlayerResource) -> Vec<(f32, f32)> {
    match resource.groove_gauge() {
        Some(g) => (0..g.gauge_type_length())
            .map(|i| {
                let prop = g.gauge_by_type(i as i32).property();
                (prop.border, prop.max)
            })
            .collect(),
        None => Vec::new(),
    }
}

/// Returns the course gauge history from the player resource.
pub fn course_gauge_history(resource: &PlayerResource) -> &[Vec<Vec<f32>>] {
    resource.course_gauge()
}

/// Computes the judge area (timing windows per judge level in milliseconds) from
/// the BMS model and player config.
///
/// Java reference: SkinTimingVisualizer.getJudgeArea(PlayerResource resource)
///   1. Gets JudgeProperty from BMSPlayerRule for the original mode
///   2. Gets judgerank from BMSModel
///   3. Gets custom judge window rates from PlayerConfig (or [100,100,100])
///   4. Applies course constraint NO_GREAT / NO_GOOD overrides
///   5. Returns rule.getNoteJudge(judgerank, judgeWindowRate) as int[][]
pub fn judge_area(resource: &PlayerResource) -> Option<Vec<Vec<i32>>> {
    let model = resource.bms_model();
    let mode = model
        .mode()
        .copied()
        .unwrap_or(bms_model::mode::Mode::BEAT_7K);
    let rule = rubato_play::bms_player_rule::BMSPlayerRule::for_mode(&mode);

    let judgerank = model.judgerank;
    let config = resource.player_config();
    let mut judge_window_rate = if config.judge_settings.custom_judge {
        [
            config.judge_settings.key_judge_window_rate_perfect_great,
            config.judge_settings.key_judge_window_rate_great,
            config.judge_settings.key_judge_window_rate_good,
        ]
    } else {
        [100, 100, 100]
    };

    // Apply course constraints
    for constraint in &resource.constraint() {
        use rubato_core::course_data::CourseDataConstraint;
        match constraint {
            CourseDataConstraint::NoGreat => {
                judge_window_rate[1] = 0;
                judge_window_rate[2] = 0;
            }
            CourseDataConstraint::NoGood => {
                judge_window_rate[2] = 0;
            }
            _ => {}
        }
    }

    Some(rule.judge.note_judge(judgerank, &judge_window_rate))
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
        assert_eq!(integer_value(&data, 0, 0, 372), 2);
        // 373: duration_average afterdot = (2500 / 100) % 10 = 25 % 10 = 5
        assert_eq!(integer_value(&data, 0, 0, 373), 5);

        // 374: timing_average integer part = -1300 / 1000 = -1
        assert_eq!(integer_value(&data, 0, 0, 374), -1);
        // 375: timing_average afterdot = (-1300 / 100) % 10 = -13 % 10 = -3
        assert_eq!(integer_value(&data, 0, 0, 375), -3);

        // 376: timing_stddev integer part = 4700 / 1000 = 4
        assert_eq!(integer_value(&data, 0, 0, 376), 4);
        // 377: timing_stddev afterdot = (4700 / 100) % 10 = 47 % 10 = 7
        assert_eq!(integer_value(&data, 0, 0, 377), 7);
    }

    #[test]
    fn test_integer_value_timing_stats_zero() {
        let data = AbstractResultData::new();

        assert_eq!(integer_value(&data, 0, 0, 372), 0);
        assert_eq!(integer_value(&data, 0, 0, 373), 0);
        assert_eq!(integer_value(&data, 0, 0, 374), 0);
        assert_eq!(integer_value(&data, 0, 0, 375), 0);
        assert_eq!(integer_value(&data, 0, 0, 376), 0);
        assert_eq!(integer_value(&data, 0, 0, 377), 0);
    }

    #[test]
    fn test_integer_value_timing_stats_large_values() {
        let mut data = AbstractResultData::new();
        // 12345 us = 12.3 ms (with remainder 45)
        data.avgduration = 12345;
        data.avg = 12345;
        data.stddev = 12345;

        assert_eq!(integer_value(&data, 0, 0, 372), 12);
        assert_eq!(integer_value(&data, 0, 0, 373), 3);
        assert_eq!(integer_value(&data, 0, 0, 374), 12);
        assert_eq!(integer_value(&data, 0, 0, 375), 3);
        assert_eq!(integer_value(&data, 0, 0, 376), 12);
        assert_eq!(integer_value(&data, 0, 0, 377), 3);
    }

    #[test]
    fn test_integer_value_existing_ids_unchanged() {
        let data = AbstractResultData::new();

        // Verify unknown IDs still return 0
        assert_eq!(integer_value(&data, 0, 0, 999), 0);

        // Verify cumulative playtime IDs (17-19) work
        // cumulative_playtime_seconds = 3661 = 1h 1m 1s
        assert_eq!(integer_value(&data, 0, 3661, 17), 1);
        assert_eq!(integer_value(&data, 0, 3661, 18), 1);
        assert_eq!(integer_value(&data, 0, 3661, 19), 1);

        // Verify boot time IDs (27-29) work
        // 3_661_000 ms = 1h 1m 1s
        assert_eq!(integer_value(&data, 3_661_000, 0, 27), 1);
        assert_eq!(integer_value(&data, 3_661_000, 0, 28), 1);
        assert_eq!(integer_value(&data, 3_661_000, 0, 29), 1);
    }

    #[test]
    fn test_boolean_value_uses_current_play_score_not_oldscore() {
        // Regression: boolean_value IDs 90/91 must check the current play's
        // score (data.score.score), not the old best score (data.oldscore).
        // Java: OPTION_RESULT_CLEAR(90) = score.getClear() != Failed.id && (cscore == null || cscore.getClear() != Failed.id)
        //       OPTION_RESULT_FAIL(91)  = score.getClear() == Failed.id || (cscore != null && cscore.getClear() == Failed.id)
        let failed_id = ClearType::Failed.id();

        let mut data = AbstractResultData::new();

        // No score data, no course score -> ID 90 false (not cleared), ID 91 true (failed)
        assert!(!boolean_value(&data, None, 90));
        assert!(boolean_value(&data, None, 91));

        // Current play cleared (AssistEasy) -> clear=true, fail=false
        let mut score = rubato_core::score_data::ScoreData::default();
        score.clear = ClearType::AssistEasy.id();
        data.score.score = Some(score.clone());
        assert!(boolean_value(&data, None, 90));
        assert!(!boolean_value(&data, None, 91));

        // Current play cleared (FullCombo) -> clear=true, fail=false
        score.clear = ClearType::FullCombo.id();
        data.score.score = Some(score.clone());
        assert!(boolean_value(&data, None, 90));
        assert!(!boolean_value(&data, None, 91));

        // Current play failed -> clear=false, fail=true
        score.clear = failed_id;
        data.score.score = Some(score.clone());
        assert!(!boolean_value(&data, None, 90));
        assert!(boolean_value(&data, None, 91));

        // NoPlay (0) is not Failed (1) -> clear=true, fail=false
        score.clear = ClearType::NoPlay.id();
        data.score.score = Some(score);
        assert!(boolean_value(&data, None, 90));
        assert!(!boolean_value(&data, None, 91));

        // Verify oldscore does NOT affect the result
        data.oldscore.clear = failed_id;
        assert!(
            boolean_value(&data, None, 90),
            "oldscore must not affect ID 90"
        );
        assert!(
            !boolean_value(&data, None, 91),
            "oldscore must not affect ID 91"
        );
    }

    #[test]
    fn test_boolean_value_course_score_affects_clear_and_fail() {
        // When course_score_data is present, it must also be checked:
        // ID 90 (clear): stage clear AND course clear
        // ID 91 (fail):  stage failed OR course failed
        let failed_id = ClearType::Failed.id();

        let mut data = AbstractResultData::new();
        let mut stage_score = rubato_core::score_data::ScoreData::default();
        stage_score.clear = ClearType::Normal.id();
        data.score.score = Some(stage_score);

        // Stage cleared, no course score -> clear
        assert!(boolean_value(&data, None, 90));
        assert!(!boolean_value(&data, None, 91));

        // Stage cleared, course cleared -> still clear
        let mut course_clear = rubato_core::score_data::ScoreData::default();
        course_clear.clear = ClearType::Normal.id();
        assert!(boolean_value(&data, Some(&course_clear), 90));
        assert!(!boolean_value(&data, Some(&course_clear), 91));

        // Stage cleared, but course failed -> NOT clear, IS fail
        let mut course_fail = rubato_core::score_data::ScoreData::default();
        course_fail.clear = failed_id;
        assert!(
            !boolean_value(&data, Some(&course_fail), 90),
            "stage clear + course fail should NOT be clear"
        );
        assert!(
            boolean_value(&data, Some(&course_fail), 91),
            "stage clear + course fail should be fail"
        );

        // Stage failed, course cleared -> NOT clear, IS fail
        let mut stage_fail = rubato_core::score_data::ScoreData::default();
        stage_fail.clear = failed_id;
        data.score.score = Some(stage_fail);
        assert!(
            !boolean_value(&data, Some(&course_clear), 90),
            "stage fail + course clear should NOT be clear"
        );
        assert!(
            boolean_value(&data, Some(&course_clear), 91),
            "stage fail + course clear should be fail"
        );

        // Both failed -> NOT clear, IS fail
        assert!(!boolean_value(&data, Some(&course_fail), 90));
        assert!(boolean_value(&data, Some(&course_fail), 91));
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

    fn make_named_ranking_with_scores() -> rubato_ir::ranking_data::RankingData {
        use rubato_ir::ir_score_data::IRScoreData;
        use rubato_ir::ranking_data::RankingData;

        let mut rd = RankingData::new();
        let scores: Vec<IRScoreData> = vec![
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.player = "ALICE".to_string();
                s.judge_counts.epg = 120;
                IRScoreData::new(&s)
            },
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.player = "YOU".to_string();
                s.judge_counts.epg = 110;
                IRScoreData::new(&s)
            },
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.player = "BOB".to_string();
                s.judge_counts.epg = 90;
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

    #[test]
    fn test_ranking_name_returns_player_names_with_offset() {
        let mut data = AbstractResultData::new();
        data.ranking = Some(make_named_ranking_with_scores());
        data.ranking_offset = 1;

        assert_eq!(ranking_name(&data, 0), "YOU");
        assert_eq!(ranking_name(&data, 1), "BOB");
        assert_eq!(ranking_name(&data, 2), "");
    }

    #[test]
    fn test_integer_value_ranking_exscore_and_index_respect_offset() {
        let mut data = AbstractResultData::new();
        data.ranking = Some(make_named_ranking_with_scores());
        data.ranking_offset = 1;

        assert_eq!(integer_value(&data, 0, 0, 380), 220);
        assert_eq!(integer_value(&data, 0, 0, 381), 180);
        assert_eq!(integer_value(&data, 0, 0, 382), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 390), 2);
        assert_eq!(integer_value(&data, 0, 0, 391), 3);
        assert_eq!(integer_value(&data, 0, 0, 392), i32::MIN);
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

    impl rubato_types::player_resource_access::ConfigAccess for GaugeTestResourceAccess {
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }
        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }
    }

    impl rubato_types::player_resource_access::ScoreAccess for GaugeTestResourceAccess {
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
        fn score_data_mut(&mut self) -> Option<&mut rubato_core::score_data::ScoreData> {
            None
        }
    }

    impl rubato_types::player_resource_access::SongAccess for GaugeTestResourceAccess {
        fn songdata(&self) -> Option<&rubato_types::song_data::SongData> {
            None
        }
        fn songdata_mut(&mut self) -> Option<&mut rubato_types::song_data::SongData> {
            None
        }
        fn set_songdata(&mut self, _data: Option<rubato_types::song_data::SongData>) {}
        fn course_song_data(&self) -> Vec<rubato_types::song_data::SongData> {
            vec![]
        }
    }

    impl rubato_types::player_resource_access::ReplayAccess for GaugeTestResourceAccess {
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
        fn course_replay_mut(&mut self) -> &mut Vec<rubato_core::replay_data::ReplayData> {
            &mut self.course_replay
        }
    }

    impl rubato_types::player_resource_access::CourseAccess for GaugeTestResourceAccess {
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
        fn set_course_data(&mut self, _data: rubato_types::course_data::CourseData) {}
        fn clear_course_data(&mut self) {}
    }

    impl rubato_types::player_resource_access::GaugeAccess for GaugeTestResourceAccess {
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
    }

    impl rubato_types::player_resource_access::PlayerStateAccess for GaugeTestResourceAccess {
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
    }

    impl rubato_types::player_resource_access::SessionMutation for GaugeTestResourceAccess {
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
    }

    impl rubato_types::player_resource_access::MediaAccess for GaugeTestResourceAccess {
        fn reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }
        fn reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }
    }

    impl rubato_types::player_resource_access::PlayerResourceAccess for GaugeTestResourceAccess {
        fn into_any_send(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
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
            crate::result::BMSPlayerMode::new(crate::result::BMSPlayerModeType::Play),
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

    // ============================================================
    // Regression tests: integer_value score-related IDs
    // ============================================================

    /// Helper: creates AbstractResultData with a populated score for testing.
    fn make_data_with_score() -> AbstractResultData {
        let mut data = AbstractResultData::new();

        // Build a new score with known judge counts
        let mut score = rubato_core::score_data::ScoreData::default();
        score.judge_counts.epg = 100; // early PG
        score.judge_counts.lpg = 50; // late PG
        score.judge_counts.egr = 30; // early GR
        score.judge_counts.lgr = 20; // late GR
        score.judge_counts.egd = 10; // early GD
        score.judge_counts.lgd = 5; // late GD
        score.judge_counts.ebd = 3; // early BD
        score.judge_counts.lbd = 2; // late BD
        score.judge_counts.epr = 1; // early PR
        score.judge_counts.lpr = 1; // late PR
        score.judge_counts.ems = 0; // early MS
        score.judge_counts.lms = 1; // late MS
        score.maxcombo = 180;
        score.notes = 223; // total notes
        score.minbp = 8;
        score.clear = ClearType::Hard.id();

        // Build old (best) score
        data.oldscore = rubato_core::score_data::ScoreData::default();
        data.oldscore.judge_counts.epg = 80;
        data.oldscore.judge_counts.lpg = 40;
        data.oldscore.judge_counts.egr = 20;
        data.oldscore.judge_counts.lgr = 15;
        data.oldscore.maxcombo = 160;
        data.oldscore.notes = 223;
        data.oldscore.minbp = 12;
        data.oldscore.clear = ClearType::Normal.id();

        // Set up ScoreDataProperty via update methods
        let old_exscore = data.oldscore.exscore();
        data.score.set_target_score(old_exscore, 0, score.notes);
        data.score.update_score(Some(&score));
        data.score.totalnotes = score.notes;

        data
    }

    #[test]
    fn test_integer_value_score_ids() {
        let data = make_data_with_score();
        let score = data.score.score.as_ref().unwrap();
        let exscore = score.exscore();

        // IDs 71, 101, 171 should all return the new score's exscore
        assert_eq!(integer_value(&data, 0, 0, 71), exscore);
        assert_eq!(integer_value(&data, 0, 0, 101), exscore);
        assert_eq!(integer_value(&data, 0, 0, 171), exscore);
    }

    #[test]
    fn test_integer_value_score_ids_no_score() {
        let data = AbstractResultData::new();
        // No score -> return i32::MIN
        assert_eq!(integer_value(&data, 0, 0, 71), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 101), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 171), i32::MIN);
    }

    #[test]
    fn test_integer_value_point() {
        let data = make_data_with_score();
        // ID 100: nowpoint (mode-dependent scoring)
        assert_eq!(integer_value(&data, 0, 0, 100), data.score.nowpoint);
        assert!(data.score.nowpoint > 0, "nowpoint should be populated");
    }

    #[test]
    fn test_integer_value_maxscore() {
        let data = make_data_with_score();
        // ID 72: notes * 2
        let score = data.score.score.as_ref().unwrap();
        assert_eq!(integer_value(&data, 0, 0, 72), score.notes * 2);
        assert_eq!(integer_value(&data, 0, 0, 72), 223 * 2);
    }

    #[test]
    fn test_integer_value_maxscore_no_score() {
        let data = AbstractResultData::new();
        assert_eq!(integer_value(&data, 0, 0, 72), i32::MIN);
    }

    #[test]
    fn test_integer_value_maxcombo_aliases() {
        let data = make_data_with_score();
        // IDs 75, 105, 174 should all return maxcombo
        assert_eq!(integer_value(&data, 0, 0, 75), 180);
        assert_eq!(integer_value(&data, 0, 0, 105), 180);
        assert_eq!(integer_value(&data, 0, 0, 174), 180);
    }

    #[test]
    fn test_integer_value_misscount_aliases() {
        let data = make_data_with_score();
        // IDs 76, 177 should both return minbp
        assert_eq!(integer_value(&data, 0, 0, 76), 8);
        assert_eq!(integer_value(&data, 0, 0, 177), 8);
    }

    #[test]
    fn test_integer_value_score_rate() {
        let data = make_data_with_score();
        // ID 102: nowrate_int (with score present)
        let rate = integer_value(&data, 0, 0, 102);
        assert_eq!(rate, data.score.nowrate_int);
        assert!(rate > 0, "score rate should be > 0");

        // ID 103: nowrate_after_dot
        let afterdot = integer_value(&data, 0, 0, 103);
        assert_eq!(afterdot, data.score.nowrate_after_dot);
    }

    #[test]
    fn test_integer_value_score_rate_no_score() {
        let data = AbstractResultData::new();
        assert_eq!(integer_value(&data, 0, 0, 102), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 103), i32::MIN);
    }

    #[test]
    fn test_integer_value_total_rate() {
        let data = make_data_with_score();
        // IDs 115, 155: rate_int
        assert_eq!(integer_value(&data, 0, 0, 115), data.score.rate_int);
        assert_eq!(integer_value(&data, 0, 0, 155), data.score.rate_int);
        // IDs 116, 156: rate_after_dot
        assert_eq!(integer_value(&data, 0, 0, 116), data.score.rate_after_dot);
        assert_eq!(integer_value(&data, 0, 0, 156), data.score.rate_after_dot);
    }

    #[test]
    fn test_integer_value_total_rate_no_score() {
        let data = AbstractResultData::new();
        assert_eq!(integer_value(&data, 0, 0, 115), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 155), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 116), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 156), i32::MIN);
    }

    #[test]
    fn test_integer_value_best_rate() {
        let data = make_data_with_score();
        // IDs 183, 184
        assert_eq!(integer_value(&data, 0, 0, 183), data.score.bestrate_int);
        assert_eq!(
            integer_value(&data, 0, 0, 184),
            data.score.bestrate_after_dot
        );
    }

    #[test]
    fn test_integer_value_highscore() {
        let data = make_data_with_score();
        // IDs 150, 170: oldscore.exscore()
        let old_ex = data.oldscore.exscore();
        assert_eq!(integer_value(&data, 0, 0, 150), old_ex);
        assert_eq!(integer_value(&data, 0, 0, 170), old_ex);
        assert!(old_ex > 0, "old exscore should be populated");
    }

    #[test]
    fn test_integer_value_target_rival_score() {
        let data = make_data_with_score();
        // IDs 121, 151, 271
        assert_eq!(integer_value(&data, 0, 0, 121), data.score.rivalscore);
        assert_eq!(integer_value(&data, 0, 0, 151), data.score.rivalscore);
        assert_eq!(integer_value(&data, 0, 0, 271), data.score.rivalscore);
    }

    #[test]
    fn test_integer_value_target_rival_rate() {
        let data = make_data_with_score();
        // IDs 122, 157: rivalrate_int
        assert_eq!(integer_value(&data, 0, 0, 122), data.score.rivalrate_int);
        assert_eq!(integer_value(&data, 0, 0, 157), data.score.rivalrate_int);
        // IDs 123, 158: rivalrate_after_dot
        assert_eq!(
            integer_value(&data, 0, 0, 123),
            data.score.rivalrate_after_dot
        );
        assert_eq!(
            integer_value(&data, 0, 0, 158),
            data.score.rivalrate_after_dot
        );
    }

    #[test]
    fn test_integer_value_diff_exscore() {
        let data = make_data_with_score();
        let expected = data.score.nowscore - data.score.nowrivalscore;
        // IDs 108, 128, 153
        assert_eq!(integer_value(&data, 0, 0, 108), expected);
        assert_eq!(integer_value(&data, 0, 0, 128), expected);
        assert_eq!(integer_value(&data, 0, 0, 153), expected);
    }

    #[test]
    fn test_integer_value_diff_highscore() {
        let data = make_data_with_score();
        let expected = data.score.nowscore - data.score.nowbestscore;
        // IDs 152, 172
        assert_eq!(integer_value(&data, 0, 0, 152), expected);
        assert_eq!(integer_value(&data, 0, 0, 172), expected);
    }

    #[test]
    fn test_integer_value_diff_nextrank() {
        let data = make_data_with_score();
        // ID 154
        assert_eq!(integer_value(&data, 0, 0, 154), data.score.nextrank);
    }

    #[test]
    fn test_integer_value_clear_type() {
        let data = make_data_with_score();
        // ID 370: current play's clear
        assert_eq!(integer_value(&data, 0, 0, 370), ClearType::Hard.id());
        // ID 371: old score's clear
        assert_eq!(integer_value(&data, 0, 0, 371), ClearType::Normal.id());
    }

    #[test]
    fn test_integer_value_clear_type_no_score() {
        let data = AbstractResultData::new();
        assert_eq!(integer_value(&data, 0, 0, 370), i32::MIN);
    }

    #[test]
    fn test_integer_value_target_maxcombo() {
        let data = make_data_with_score();
        // ID 173: oldscore.maxcombo (> 0)
        assert_eq!(integer_value(&data, 0, 0, 173), 160);
    }

    #[test]
    fn test_integer_value_target_maxcombo_zero() {
        let mut data = AbstractResultData::new();
        data.oldscore.maxcombo = 0;
        assert_eq!(integer_value(&data, 0, 0, 173), i32::MIN);
    }

    #[test]
    fn test_integer_value_diff_maxcombo() {
        let data = make_data_with_score();
        // ID 175: newCombo - oldCombo
        assert_eq!(integer_value(&data, 0, 0, 175), 180 - 160);
    }

    #[test]
    fn test_integer_value_diff_maxcombo_old_zero() {
        let mut data = make_data_with_score();
        data.oldscore.maxcombo = 0;
        assert_eq!(integer_value(&data, 0, 0, 175), i32::MIN);
    }

    #[test]
    fn test_integer_value_target_misscount() {
        let data = make_data_with_score();
        // ID 176: oldscore.minbp (not MAX)
        assert_eq!(integer_value(&data, 0, 0, 176), 12);
    }

    #[test]
    fn test_integer_value_target_misscount_max() {
        let data = AbstractResultData::new();
        // Default minbp = i32::MAX -> return i32::MIN
        assert_eq!(integer_value(&data, 0, 0, 176), i32::MIN);
    }

    #[test]
    fn test_integer_value_diff_misscount() {
        let data = make_data_with_score();
        // ID 178: newMinbp - oldMinbp
        assert_eq!(integer_value(&data, 0, 0, 178), 8 - 12);
    }

    #[test]
    fn test_integer_value_diff_misscount_old_max() {
        let data = AbstractResultData::new();
        assert_eq!(integer_value(&data, 0, 0, 178), i32::MIN);
    }

    #[test]
    fn test_integer_value_judge_counts_from_score() {
        let data = make_data_with_score();
        let s = data.score.score.as_ref().unwrap();
        // IDs 80-84: judge_count_total for PG, GR, GD, BD, PR
        assert_eq!(integer_value(&data, 0, 0, 80), s.judge_count_total(0)); // PG = 150
        assert_eq!(integer_value(&data, 0, 0, 81), s.judge_count_total(1)); // GR = 50
        assert_eq!(integer_value(&data, 0, 0, 82), s.judge_count_total(2)); // GD = 15
        assert_eq!(integer_value(&data, 0, 0, 83), s.judge_count_total(3)); // BD = 5
        assert_eq!(integer_value(&data, 0, 0, 84), s.judge_count_total(4)); // PR = 2
    }

    #[test]
    fn test_integer_value_judge_counts_no_score() {
        let data = AbstractResultData::new();
        for id in 80..=84 {
            assert_eq!(
                integer_value(&data, 0, 0, id),
                i32::MIN,
                "ID {} should return i32::MIN when no score",
                id,
            );
        }
    }

    #[test]
    fn test_integer_value_judge_rates() {
        let data = make_data_with_score();
        let s = data.score.score.as_ref().unwrap();
        // IDs 85-89: judgeCountTotal * 100 / notes
        for j in 0..5 {
            let expected = s.judge_count_total(j) * 100 / s.notes;
            assert_eq!(
                integer_value(&data, 0, 0, 85 + j),
                expected,
                "judge rate for index {}",
                j,
            );
        }
    }

    #[test]
    fn test_integer_value_judge_rates_no_score() {
        let data = AbstractResultData::new();
        for id in 85..=89 {
            assert_eq!(integer_value(&data, 0, 0, id), i32::MIN);
        }
    }

    #[test]
    fn test_integer_value_state_judge_counts() {
        let data = make_data_with_score();
        // IDs 110-114: early + late combined via state's judge_count
        let s = data.score.score.as_ref().unwrap();
        for j in 0..5 {
            let expected = s.judge_count(j, true) + s.judge_count(j, false);
            assert_eq!(
                integer_value(&data, 0, 0, 110 + j),
                expected,
                "state judge count for index {}",
                j,
            );
        }
    }

    #[test]
    fn test_integer_value_early_late_judge_counts() {
        let data = make_data_with_score();
        let s = data.score.score.as_ref().unwrap();
        // IDs 410-419: alternating early/late
        for offset in 0..10 {
            let index = offset / 2;
            let early = offset % 2 == 0;
            let expected = s.judge_count(index, early);
            assert_eq!(
                integer_value(&data, 0, 0, 410 + offset),
                expected,
                "early/late judge count for offset {} (index={}, early={})",
                offset,
                index,
                early,
            );
        }
    }

    #[test]
    fn test_integer_value_total_early_late() {
        let data = make_data_with_score();
        let s = data.score.score.as_ref().unwrap();

        // ID 423: total early (judges 1-5 early)
        let mut expected_early = 0;
        for i in 1..6 {
            expected_early += s.judge_count(i, true);
        }
        assert_eq!(integer_value(&data, 0, 0, 423), expected_early);

        // ID 424: total late (judges 1-5 late)
        let mut expected_late = 0;
        for i in 1..6 {
            expected_late += s.judge_count(i, false);
        }
        assert_eq!(integer_value(&data, 0, 0, 424), expected_late);
    }

    #[test]
    fn test_integer_value_combo_break() {
        let data = make_data_with_score();
        let s = data.score.score.as_ref().unwrap();
        // ID 425: BD + PR (early+late for indices 3 and 4)
        let expected = s.judge_count(3, true)
            + s.judge_count(3, false)
            + s.judge_count(4, true)
            + s.judge_count(4, false);
        assert_eq!(integer_value(&data, 0, 0, 425), expected);
        assert_eq!(expected, 3 + 2 + 1 + 1); // ebd + lbd + epr + lpr
    }

    #[test]
    fn test_integer_value_rival_judge_counts() {
        let mut data = make_data_with_score();
        // Set rival score in the ScoreDataProperty
        let mut rival = rubato_core::score_data::ScoreData::default();
        rival.judge_counts.epg = 70;
        rival.judge_counts.lpg = 30;
        rival.judge_counts.egr = 15;
        rival.judge_counts.lgr = 10;
        rival.judge_counts.egd = 5;
        rival.judge_counts.lgd = 3;
        rival.judge_counts.ebd = 2;
        rival.judge_counts.lbd = 1;
        rival.judge_counts.epr = 1;
        rival.judge_counts.lpr = 0;
        rival.notes = 137;
        data.score.rival = Some(rival.clone());

        // IDs 280-284: rival judge_count_total
        assert_eq!(integer_value(&data, 0, 0, 280), rival.judge_count_total(0));
        assert_eq!(integer_value(&data, 0, 0, 281), rival.judge_count_total(1));
        assert_eq!(integer_value(&data, 0, 0, 282), rival.judge_count_total(2));
        assert_eq!(integer_value(&data, 0, 0, 283), rival.judge_count_total(3));
        assert_eq!(integer_value(&data, 0, 0, 284), rival.judge_count_total(4));

        // IDs 285-289: rival judge rates
        for j in 0..5 {
            let expected = rival.judge_count_total(j) * 100 / rival.notes;
            assert_eq!(integer_value(&data, 0, 0, 285 + j), expected);
        }
    }

    #[test]
    fn test_integer_value_rival_judge_counts_no_rival() {
        let data = make_data_with_score();
        // No rival score set -> MIN_VALUE
        for id in 280..=284 {
            assert_eq!(integer_value(&data, 0, 0, id), i32::MIN);
        }
        for id in 285..=289 {
            assert_eq!(integer_value(&data, 0, 0, id), i32::MIN);
        }
    }

    #[test]
    fn test_integer_value_ir_rank_offline() {
        let data = make_data_with_score();
        // IR offline (state == STATE_OFFLINE) -> i32::MIN
        assert_eq!(integer_value(&data, 0, 0, 179), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 182), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 180), i32::MIN);
        assert_eq!(integer_value(&data, 0, 0, 200), i32::MIN);
    }

    #[test]
    fn test_integer_value_ir_rank_with_ranking() {
        use super::super::abstract_result::STATE_IR_FINISHED;
        use rubato_ir::ir_score_data::IRScoreData;

        let mut data = make_data_with_score();
        data.state = STATE_IR_FINISHED;

        // Build ranking via update_score with known scores
        let mut ranking = rubato_ir::ranking_data::RankingData::new();
        let scores: Vec<IRScoreData> = vec![
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.judge_counts.epg = 200;
                s.judge_counts.lpg = 100;
                IRScoreData::new(&s)
            },
            {
                let mut s = rubato_core::score_data::ScoreData::default();
                s.judge_counts.epg = 100;
                s.judge_counts.lpg = 50;
                IRScoreData::new(&s)
            },
        ];
        ranking.update_score(&scores, None);
        data.ranking = Some(ranking);

        // rank/prevrank/total depend on ranking data state
        // total_player should be 2
        assert_eq!(integer_value(&data, 0, 0, 180), 2);
        assert_eq!(integer_value(&data, 0, 0, 200), 2);
        // rank and prevrank depend on player="" matching logic in update_score
        // The important thing is they do NOT return i32::MIN when state != OFFLINE
        assert_ne!(integer_value(&data, 0, 0, 179), i32::MIN);
        assert_ne!(integer_value(&data, 0, 0, 182), i32::MIN);
    }

    #[test]
    fn test_integer_value_totalnotes() {
        let data = make_data_with_score();
        // IDs 74, 106 should return totalnotes
        assert_eq!(integer_value(&data, 0, 0, 74), data.score.totalnotes);
        assert_eq!(integer_value(&data, 0, 0, 106), data.score.totalnotes);
        assert_eq!(data.score.totalnotes, 223);
    }

    // ============================================================
    // Regression tests: float_value IDs
    // ============================================================

    #[test]
    fn test_float_value_score_rate_with_score() {
        let data = make_data_with_score();
        // ID 1102: nowrate when score exists
        let v = float_value(&data, 1102).unwrap();
        assert!((v - data.score.nowrate).abs() < 0.0001);
        assert!(v > 0.0);
    }

    #[test]
    fn test_float_value_score_rate_no_score() {
        let data = AbstractResultData::new();
        // No score -> f32::MIN
        assert_eq!(float_value(&data, 1102), Some(f32::MIN));
    }

    #[test]
    fn test_float_value_total_rate() {
        let data = make_data_with_score();
        // ID 1115: rate when score exists
        let v = float_value(&data, 1115).unwrap();
        assert!((v - data.score.rate).abs() < 0.0001);
    }

    #[test]
    fn test_float_value_total_rate_no_score() {
        let data = AbstractResultData::new();
        assert_eq!(float_value(&data, 1115), Some(f32::MIN));
    }

    #[test]
    fn test_float_value_score_rate2() {
        let data = make_data_with_score();
        // ID 155: same as total rate
        let v = float_value(&data, 155).unwrap();
        assert!((v - data.score.rate).abs() < 0.0001);
    }

    #[test]
    fn test_float_value_score_rate2_no_score() {
        let data = AbstractResultData::new();
        assert_eq!(float_value(&data, 155), Some(f32::MIN));
    }

    #[test]
    fn test_float_value_rate_type_ids() {
        let data = make_data_with_score();
        // ID 110: rate
        assert!((float_value(&data, 110).unwrap() - data.score.rate).abs() < 0.0001);
        // ID 111: nowrate
        assert!((float_value(&data, 111).unwrap() - data.score.nowrate).abs() < 0.0001);
        // ID 112: nowbestscorerate
        assert!((float_value(&data, 112).unwrap() - data.score.nowbestscorerate).abs() < 0.0001);
        // ID 113: bestscorerate
        assert!((float_value(&data, 113).unwrap() - data.score.bestscorerate).abs() < 0.0001);
        // ID 183 (float): bestscorerate
        assert!((float_value(&data, 183).unwrap() - data.score.bestscorerate).abs() < 0.0001);
        // ID 114: nowrivalscorerate
        assert!((float_value(&data, 114).unwrap() - data.score.nowrivalscorerate).abs() < 0.0001);
        // ID 115 (float): rivalscorerate
        assert!((float_value(&data, 115).unwrap() - data.score.rivalscorerate).abs() < 0.0001);
        // ID 122: rivalscorerate
        assert!((float_value(&data, 122).unwrap() - data.score.rivalscorerate).abs() < 0.0001);
        // ID 135: rivalscorerate
        assert!((float_value(&data, 135).unwrap() - data.score.rivalscorerate).abs() < 0.0001);
        // ID 157: rivalscorerate
        assert!((float_value(&data, 157).unwrap() - data.score.rivalscorerate).abs() < 0.0001);
    }

    #[test]
    fn test_float_value_judge_rates() {
        let data = make_data_with_score();
        let s = data.score.score.as_ref().unwrap();
        // IDs 85-89: judge rate as float
        for j in 0..5 {
            let expected = s.judge_count_total(j) as f32 / s.notes as f32;
            let v = float_value(&data, 85 + j).unwrap();
            assert!(
                (v - expected).abs() < 0.0001,
                "float judge rate for index {}: got {}, expected {}",
                j,
                v,
                expected,
            );
        }
    }

    #[test]
    fn test_float_value_judge_rates_no_score() {
        let data = AbstractResultData::new();
        for id in 85..=89 {
            assert_eq!(
                float_value(&data, id),
                Some(f32::MIN),
                "float judge rate ID {} should be f32::MIN without score",
                id,
            );
        }
    }

    #[test]
    fn test_float_value_rival_judge_rates() {
        let mut data = make_data_with_score();
        let mut rival = rubato_core::score_data::ScoreData::default();
        rival.judge_counts.epg = 60;
        rival.judge_counts.lpg = 30;
        rival.judge_counts.egr = 10;
        rival.judge_counts.lgr = 5;
        rival.notes = 110;
        data.score.rival = Some(rival.clone());

        for j in 0..5 {
            let expected = rival.judge_count_total(j) as f32 / rival.notes as f32;
            let v = float_value(&data, 285 + j).unwrap();
            assert!(
                (v - expected).abs() < 0.0001,
                "rival float judge rate for index {}",
                j,
            );
        }
    }

    #[test]
    fn test_float_value_rival_judge_rates_no_rival() {
        let data = make_data_with_score();
        for id in 285..=289 {
            assert_eq!(float_value(&data, id), Some(f32::MIN));
        }
    }

    #[test]
    fn test_float_value_unknown_returns_none() {
        let data = make_data_with_score();
        assert_eq!(float_value(&data, 9999), None);
    }

    #[test]
    fn test_float_value_zero_scorerate_not_confused_with_unmatched() {
        // Regression: when scorerate is legitimately 0.0, float_value must
        // return Some(0.0), not None. The old f32 return used 0.0 as sentinel
        // for both "unmatched ID" and "legitimate zero", causing callers to
        // fall through to default_float_value incorrectly.
        let mut data = AbstractResultData::new();
        data.score.rate = 0.0;
        data.score.score = Some(rubato_core::score_data::ScoreData::default());
        // ID 110 (scorerate) should return Some(0.0)
        assert_eq!(float_value(&data, 110), Some(0.0));
    }

    // ============================================================
    // Tests for newly wired SkinRenderContext methods
    // ============================================================

    #[test]
    fn test_get_timing_distribution_returns_none_when_empty() {
        let data = AbstractResultData::new();
        assert!(get_timing_distribution(&data).is_none());
    }

    #[test]
    fn test_get_timing_distribution_returns_data_after_sync() {
        let mut data = AbstractResultData::new();
        data.timing_distribution.add(5);
        data.timing_distribution.add(-3);
        data.timing_distribution.statistic_value_calculate();
        data.sync_timing_distribution_cache();

        let td = get_timing_distribution(&data);
        assert!(td.is_some());
        let td = td.unwrap();
        assert_eq!(td.array_center(), data.timing_distribution.array_center());
        assert_eq!(
            td.timing_distribution().len(),
            data.timing_distribution.timing_distribution().len()
        );
    }

    #[test]
    fn test_score_data_property_returns_data_score() {
        let mut data = AbstractResultData::new();
        data.score.nowrate = 0.95;
        data.score.nowscore = 1234;
        let prop = score_data_property(&data);
        assert!((prop.nowrate - 0.95).abs() < f32::EPSILON);
        assert_eq!(prop.nowscore, 1234);
    }

    // ============================================================
    // is_gauge_max() tests
    // ============================================================

    #[test]
    fn test_is_gauge_max_true_when_at_max() {
        let resource = make_resource_with_gauge(100.0);
        assert!(
            is_gauge_max(&resource),
            "is_gauge_max should return true when gauge is at max (100.0)"
        );
    }

    #[test]
    fn test_is_gauge_max_false_when_not_at_max() {
        let resource = make_resource_with_gauge(75.0);
        assert!(
            !is_gauge_max(&resource),
            "is_gauge_max should return false when gauge is not at max (75.0)"
        );
    }

    #[test]
    fn test_is_gauge_max_false_when_no_gauge() {
        let resource = PlayerResource::default();
        assert!(
            !is_gauge_max(&resource),
            "is_gauge_max should return false when no groove gauge is present"
        );
    }
}

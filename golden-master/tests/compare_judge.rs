// Golden master comparison tests for JudgeManager.
//
// Loads Java-generated fixtures and runs equivalent Rust simulations,
// comparing ScoreData, maxcombo, ghost, gauge values.
//
// Notes:
// - JudgeManager.prev_time starts at 0, so notes at time_us=0 are skipped on
//   the first frame. We prime the JudgeManager with update(-1) to work around this.
// - LN notes are split into start+end pairs via build_judge_notes() for JudgeManager.
// - Pure LN (LNTYPE_LONGNOTE) end notes are not independently judged — only 1
//   judgment per LN pair, matching Java's behavior.

use std::path::Path;

use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use bms_model::chart_information::ChartInformation;
use bms_model::mode::Mode;
use golden_master::judge_fixtures::{JudgeFixtures, JudgeTestCase};
use rubato_core::score_data::ScoreData;
use rubato_input::key_input_log::KeyInputLog;
use rubato_play::bms_player_rule::BMSPlayerRule;
use rubato_play::judge_algorithm::JudgeAlgorithm;
use rubato_play::judge_manager::{JudgeConfig, JudgeManager};
use rubato_play::lane_property::LaneProperty;
use rubato_types::groove_gauge::GrooveGauge;

#[path = "support/random_seeds.rs"]
mod random_seeds;

/// Sentinel for "not set" timestamps (matches JudgeManager internal).
const NOT_SET: i64 = i64::MIN;

/// Frame step for simulation (1ms = 1000us).
const FRAME_STEP: i64 = 1_000;

/// Extra time after last note to finish simulation (1 second).
const TAIL_TIME: i64 = 1_000_000;

/// Gauge value comparison tolerance (f32 rounding).
const GAUGE_TOLERANCE: f32 = 0.02;

fn test_bms_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-bms")
        .leak()
}

fn load_bms(filename: &str) -> BMSModel {
    let path = test_bms_dir().join(filename);
    let randoms = random_seeds::try_load_selected_randoms(test_bms_dir(), filename);
    let info = ChartInformation::new(Some(path), LNTYPE_LONGNOTE, randoms);
    let mut model = BMSDecoder::new()
        .decode(info)
        .unwrap_or_else(|| panic!("Failed to parse {filename}"));
    BMSPlayerRule::validate(&mut model);
    model
}

fn parse_gauge_type(s: &str) -> i32 {
    match s {
        "ASSIST_EASY" => GrooveGauge::ASSISTEASY,
        "EASY" => GrooveGauge::EASY,
        "NORMAL" => GrooveGauge::NORMAL,
        "HARD" => GrooveGauge::HARD,
        "EXHARD" => GrooveGauge::EXHARD,
        "HAZARD" => GrooveGauge::HAZARD,
        "CLASS" => GrooveGauge::GRADE_NORMAL,
        "EXCLASS" => GrooveGauge::GRADE_HARD,
        "EXHARDCLASS" => GrooveGauge::GRADE_EXHARD,
        _ => panic!("Unknown gauge type: {s}"),
    }
}

struct SimResult {
    score: ScoreData,
    max_combo: i32,
    ghost: Vec<usize>,
    gauge_value: f32,
    gauge_qualified: bool,
    pass_notes: i32,
}

fn run_simulation(model: &BMSModel, tc: &JudgeTestCase) -> SimResult {
    let judge_notes = model.build_judge_notes();
    let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
    let rule = BMSPlayerRule::for_mode(&mode);

    let config = JudgeConfig {
        notes: &judge_notes,
        mode: &mode,
        ln_type: model.lntype(),
        judge_rank: model.judgerank(),
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: tc.autoplay,
        judge_property: &rule.judge,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
    };

    let gauge_type = parse_gauge_type(&tc.gauge_type);
    let mut jm = JudgeManager::from_config(&config);
    let mut gauge = GrooveGauge::new(model, gauge_type, &rule.gauge);

    let lp = LaneProperty::new(&mode);
    let physical_key_count = lp.key_lane_assign().len();

    // Prime JudgeManager: set prev_time to -1 so notes at time_us=0 are not skipped.
    let empty_states = vec![false; physical_key_count];
    let empty_times = vec![NOT_SET; physical_key_count];
    jm.update(-1, &judge_notes, &empty_states, &empty_times, &mut gauge);

    let last_note_time = judge_notes
        .iter()
        .map(|n| n.time_us.max(n.end_time_us))
        .max()
        .unwrap_or(0);
    let end_time = last_note_time + TAIL_TIME;

    if tc.autoplay {
        // Autoplay: run with empty key states
        let key_states = vec![false; physical_key_count];
        let key_times = vec![NOT_SET; physical_key_count];
        let mut time = 0i64;
        while time <= end_time {
            jm.update(time, &judge_notes, &key_states, &key_times, &mut gauge);
            time += FRAME_STEP;
        }
    } else {
        // Manual: convert input_log to per-frame key states
        let log: Vec<KeyInputLog> = tc
            .input_log
            .iter()
            .map(|e| KeyInputLog::with_data(e.presstime, e.keycode, e.pressed))
            .collect();

        let mut sorted_log: Vec<&KeyInputLog> = log.iter().collect();
        sorted_log.sort_by_key(|e| e.time());

        let mut key_states = vec![false; physical_key_count];
        let mut log_cursor = 0;
        let mut time = 0i64;

        while time <= end_time {
            let mut key_changed_times = vec![NOT_SET; physical_key_count];

            // Input log uses lane indices (keycodes); map directly to physical key indices.
            while log_cursor < sorted_log.len() && sorted_log[log_cursor].time() <= time {
                let event = sorted_log[log_cursor];
                let key = event.keycode() as usize;
                if key < physical_key_count {
                    key_states[key] = event.is_pressed();
                    key_changed_times[key] = event.time();
                }
                log_cursor += 1;
            }

            jm.update(
                time,
                &judge_notes,
                &key_states,
                &key_changed_times,
                &mut gauge,
            );
            time += FRAME_STEP;
        }
    }

    SimResult {
        score: jm.score().clone(),
        max_combo: jm.max_combo(),
        ghost: jm.ghost_as_usize(),
        gauge_value: gauge.value(),
        gauge_qualified: gauge.is_qualified(),
        pass_notes: jm.past_notes(),
    }
}

fn compare_score(
    actual: &ScoreData,
    expected: &golden_master::judge_fixtures::ExpectedScore,
) -> Vec<String> {
    let mut diffs = Vec::new();
    let fields = [
        ("epg", actual.epg, expected.epg),
        ("lpg", actual.lpg, expected.lpg),
        ("egr", actual.egr, expected.egr),
        ("lgr", actual.lgr, expected.lgr),
        ("egd", actual.egd, expected.egd),
        ("lgd", actual.lgd, expected.lgd),
        ("ebd", actual.ebd, expected.ebd),
        ("lbd", actual.lbd, expected.lbd),
        ("epr", actual.epr, expected.epr),
        ("lpr", actual.lpr, expected.lpr),
        ("ems", actual.ems, expected.ems),
        ("lms", actual.lms, expected.lms),
        ("score.maxcombo", actual.maxcombo, expected.maxcombo),
        ("score.passnotes", actual.passnotes, expected.passnotes),
    ];
    for (name, actual_val, expected_val) in fields {
        if actual_val != expected_val {
            diffs.push(format!("{name}: rust={actual_val} java={expected_val}"));
        }
    }
    diffs
}

fn run_test_case(tc: &JudgeTestCase) {
    let model = load_bms(&tc.filename);
    let result = run_simulation(&model, tc);

    let mut diffs: Vec<String> = Vec::new();

    // Compare score fields
    diffs.extend(compare_score(&result.score, &tc.expected.score));

    // Compare maxcombo
    if result.max_combo != tc.expected.maxcombo {
        diffs.push(format!(
            "maxcombo: rust={} java={}",
            result.max_combo, tc.expected.maxcombo
        ));
    }

    // Compare passnotes
    if result.pass_notes != tc.expected.passnotes {
        diffs.push(format!(
            "passnotes: rust={} java={}",
            result.pass_notes, tc.expected.passnotes
        ));
    }

    // Compare gauge_value with tolerance
    if (result.gauge_value - tc.expected.gauge_value).abs() > GAUGE_TOLERANCE {
        diffs.push(format!(
            "gauge_value: rust={:.4} java={:.4} (diff={:.4})",
            result.gauge_value,
            tc.expected.gauge_value,
            (result.gauge_value - tc.expected.gauge_value).abs()
        ));
    }

    // Compare gauge_qualified
    if result.gauge_qualified != tc.expected.gauge_qualified {
        diffs.push(format!(
            "gauge_qualified: rust={} java={}",
            result.gauge_qualified, tc.expected.gauge_qualified
        ));
    }

    // Compare ghost
    if result.ghost != tc.expected.ghost {
        let ghost_len = result.ghost.len().min(tc.expected.ghost.len());
        let mut ghost_diff_count = 0;
        for i in 0..ghost_len {
            if result.ghost[i] != tc.expected.ghost[i] {
                if ghost_diff_count < 5 {
                    diffs.push(format!(
                        "ghost[{i}]: rust={} java={}",
                        result.ghost[i], tc.expected.ghost[i]
                    ));
                }
                ghost_diff_count += 1;
            }
        }
        if result.ghost.len() != tc.expected.ghost.len() {
            diffs.push(format!(
                "ghost.len: rust={} java={}",
                result.ghost.len(),
                tc.expected.ghost.len()
            ));
        }
        if ghost_diff_count > 5 {
            diffs.push(format!("... and {} more ghost diffs", ghost_diff_count - 5));
        }
    }

    if !diffs.is_empty() {
        panic!(
            "GM mismatch for [{}] {}:\n  {}",
            tc.group,
            tc.name,
            diffs.join("\n  ")
        );
    }
}

// =========================================================================
// Test functions: one per test case group for clear failure reporting
// =========================================================================

#[test]
fn group_a_autoplay() {
    let fixtures = JudgeFixtures::load().expect("Failed to load judge fixtures");
    for tc in &fixtures.test_cases {
        if tc.group == "A_autoplay" {
            run_test_case(tc);
        }
    }
}

#[test]
fn group_b_manual() {
    let fixtures = JudgeFixtures::load().expect("Failed to load judge fixtures");
    for tc in &fixtures.test_cases {
        if tc.group == "B_manual" {
            run_test_case(tc);
        }
    }
}

#[test]
fn group_c_gauge() {
    let fixtures = JudgeFixtures::load().expect("Failed to load judge fixtures");
    for tc in &fixtures.test_cases {
        if tc.group == "C_gauge" {
            run_test_case(tc);
        }
    }
}

#[test]
fn group_d_gauge_miss() {
    let fixtures = JudgeFixtures::load().expect("Failed to load judge fixtures");
    for tc in &fixtures.test_cases {
        if tc.group == "D_gauge_miss" {
            run_test_case(tc);
        }
    }
}

#[test]
fn group_e_longnote() {
    let fixtures = JudgeFixtures::load().expect("Failed to load judge fixtures");
    for tc in &fixtures.test_cases {
        if tc.group == "E_longnote" {
            run_test_case(tc);
        }
    }
}

#[test]
fn group_f_cross_mode() {
    let fixtures = JudgeFixtures::load().expect("Failed to load judge fixtures");
    for tc in &fixtures.test_cases {
        if tc.group == "F_cross_mode" {
            run_test_case(tc);
        }
    }
}

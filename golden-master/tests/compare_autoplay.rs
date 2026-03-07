// Golden master tests: compare Rust create_autoplay_log() against Java KeyInputLog.createAutoplayLog()

use std::path::Path;

use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use bms_model::bmson_decoder::BMSONDecoder;
use bms_model::chart_information::ChartInformation;
use bms_model::time_line::TimeLine;
use golden_master::autoplay_fixtures::{AutoplayFixture, AutoplayLogEntry, AutoplayTestCase};
use rubato_input::key_input_log::KeyInputLog;

#[path = "support/random_seeds.rs"]
mod random_seeds;

fn fixtures_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .leak()
}

fn test_bms_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-bms")
        .leak()
}

fn load_autoplay_fixture() -> AutoplayFixture {
    let path = fixtures_dir().join("autoplay_log.json");
    assert!(
        path.exists(),
        "Autoplay fixture not found: {}. Run `just golden-master-autoplay-gen` first.",
        path.display()
    );
    let content = std::fs::read_to_string(&path).expect("Failed to read fixture");
    serde_json::from_str(&content).expect("Failed to parse fixture")
}

fn find_test_case<'a>(fixture: &'a AutoplayFixture, filename: &str) -> &'a AutoplayTestCase {
    fixture
        .test_cases
        .iter()
        .find(|tc| tc.filename == filename)
        .unwrap_or_else(|| panic!("Test case not found for {filename}"))
}

fn compare_autoplay_logs(
    rust_log: &[KeyInputLog],
    java_log: &[AutoplayLogEntry],
    filename: &str,
) -> Vec<String> {
    let mut diffs = Vec::new();

    if rust_log.len() != java_log.len() {
        diffs.push(format!(
            "log length: rust={} java={}",
            rust_log.len(),
            java_log.len()
        ));
    }

    let min_len = rust_log.len().min(java_log.len());
    for i in 0..min_len {
        let r = &rust_log[i];
        let j = &java_log[i];

        // Allow +/-2us tolerance for timing
        let time_diff = (r.time() - j.presstime).abs();
        if time_diff > 2 {
            diffs.push(format!(
                "{filename}[{i}] presstime: rust={} java={} (diff={})",
                r.time(),
                j.presstime,
                time_diff
            ));
        }

        if r.keycode() != j.keycode {
            diffs.push(format!(
                "{filename}[{i}] keycode: rust={} java={}",
                r.keycode(),
                j.keycode
            ));
        }

        if r.is_pressed() != j.pressed {
            diffs.push(format!(
                "{filename}[{i}] pressed: rust={} java={}",
                r.is_pressed(),
                j.pressed
            ));
        }
    }

    // Show first few extra entries on either side
    if rust_log.len() > java_log.len() {
        for (i, r) in rust_log.iter().enumerate().skip(min_len).take(5) {
            diffs.push(format!(
                "{filename}[{i}] extra rust: presstime={} keycode={} pressed={}",
                r.time(),
                r.keycode(),
                r.is_pressed()
            ));
        }
    } else if java_log.len() > rust_log.len() {
        for (i, j) in java_log.iter().enumerate().skip(min_len).take(5) {
            diffs.push(format!(
                "{filename}[{i}] extra java: presstime={} keycode={} pressed={}",
                j.presstime, j.keycode, j.pressed
            ));
        }
    }

    diffs
}

/// Ensure the model has timelines at all the Java timeline times.
/// If the Rust model is missing a timeline at a Java time, insert an empty one.
/// This ensures the autoplay algorithm iterates the same set of time points as Java.
fn ensure_timelines_match_fixture(model: &mut BMSModel, timeline_times: &[i64]) {
    use std::collections::HashSet;

    let existing_times: HashSet<i64> = model.timelines.iter().map(|tl| tl.micro_time()).collect();

    let keys = model.mode().map(|m| m.key()).unwrap_or(8);

    // Check if any Java times are missing from Rust model
    let missing: Vec<i64> = timeline_times
        .iter()
        .filter(|t| !existing_times.contains(t))
        .copied()
        .collect();

    if missing.is_empty() {
        return;
    }

    // Take all existing timelines, add empty timelines for missing times, re-sort
    let mut timelines = model.take_all_time_lines();
    for &t in &missing {
        let mut tl = TimeLine::new(0.0, t, keys);
        // Set BPM from nearest existing timeline
        if let Some(nearest) = timelines.iter().rfind(|tl| tl.micro_time() <= t) {
            tl.bpm = nearest.get_bpm();
        } else if let Some(first) = timelines.first() {
            tl.bpm = first.get_bpm();
        }
        timelines.push(tl);
    }
    timelines.sort_by_key(|tl| tl.micro_time());
    model.timelines = timelines;
}

/// Run a single BMS autoplay golden master test
fn run_autoplay_test(bms_name: &str) {
    let fixture = load_autoplay_fixture();
    let test_case = find_test_case(&fixture, bms_name);

    let bms_path = test_bms_dir().join(bms_name);
    assert!(
        bms_path.exists(),
        "BMS file not found: {}",
        bms_path.display()
    );

    let mut model = BMSDecoder::new()
        .decode_path(&bms_path)
        .expect("Failed to parse BMS");
    ensure_timelines_match_fixture(&mut model, &test_case.timeline_times);
    let rust_log = KeyInputLog::create_autoplay_log(&model);

    let diffs = compare_autoplay_logs(&rust_log, &test_case.log, bms_name);
    if !diffs.is_empty() {
        panic!(
            "Autoplay mismatch for {} ({} differences):\n{}",
            bms_name,
            diffs.len(),
            diffs
                .iter()
                .map(|d| format!("  - {d}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Run a BMS test with fixed random selections
fn run_autoplay_test_with_randoms(bms_name: &str, randoms: &[i32]) {
    let fixture = load_autoplay_fixture();
    let test_case = find_test_case(&fixture, bms_name);

    let bms_path = test_bms_dir().join(bms_name);
    assert!(
        bms_path.exists(),
        "BMS file not found: {}",
        bms_path.display()
    );

    let info = ChartInformation::new(Some(bms_path), LNTYPE_LONGNOTE, Some(randoms.to_vec()));
    let mut model = BMSDecoder::new().decode(info).expect("Failed to parse BMS");
    ensure_timelines_match_fixture(&mut model, &test_case.timeline_times);
    let rust_log = KeyInputLog::create_autoplay_log(&model);

    let diffs = compare_autoplay_logs(&rust_log, &test_case.log, bms_name);
    if !diffs.is_empty() {
        panic!(
            "Autoplay mismatch for {} ({} differences):\n{}",
            bms_name,
            diffs.len(),
            diffs
                .iter()
                .map(|d| format!("  - {d}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Run a bmson autoplay golden master test
fn run_autoplay_test_bmson(bmson_name: &str) {
    let fixture = load_autoplay_fixture();
    let test_case = find_test_case(&fixture, bmson_name);

    let bmson_path = test_bms_dir().join(bmson_name);
    assert!(
        bmson_path.exists(),
        "bmson file not found: {}",
        bmson_path.display()
    );

    let mut model = BMSONDecoder::new(LNTYPE_LONGNOTE)
        .decode_path(&bmson_path)
        .expect("Failed to parse bmson");
    ensure_timelines_match_fixture(&mut model, &test_case.timeline_times);
    let rust_log = KeyInputLog::create_autoplay_log(&model);

    let diffs = compare_autoplay_logs(&rust_log, &test_case.log, bmson_name);
    if !diffs.is_empty() {
        panic!(
            "Autoplay mismatch for {} ({} differences):\n{}",
            bmson_name,
            diffs.len(),
            diffs
                .iter()
                .map(|d| format!("  - {d}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

// --- BMS tests ---

#[test]
fn autoplay_minimal_7k() {
    run_autoplay_test("minimal_7k.bms");
}

#[test]
fn autoplay_5key() {
    run_autoplay_test("5key.bms");
}

#[test]
fn autoplay_14key_dp() {
    run_autoplay_test("14key_dp.bms");
}

#[test]
fn autoplay_9key_pms() {
    run_autoplay_test("9key_pms.bms");
}

#[test]
fn autoplay_9key_pms_pms() {
    run_autoplay_test("9key_pms.pms");
}

#[test]
fn autoplay_bpm_change() {
    run_autoplay_test("bpm_change.bms");
}

#[test]
fn autoplay_bpm_stop_combo() {
    run_autoplay_test("bpm_stop_combo.bms");
}

#[test]
fn autoplay_stop_sequence() {
    run_autoplay_test("stop_sequence.bms");
}

#[test]
fn autoplay_longnote_types() {
    run_autoplay_test("longnote_types.bms");
}

#[test]
fn autoplay_mine_notes() {
    run_autoplay_test("mine_notes.bms");
}

#[test]
fn autoplay_scratch_bss() {
    run_autoplay_test("scratch_bss.bms");
}

#[test]
fn autoplay_empty_measures() {
    run_autoplay_test("empty_measures.bms");
}

#[test]
fn autoplay_random_if() {
    let selected_randoms = random_seeds::load_selected_randoms(test_bms_dir(), "random_if.bms");
    run_autoplay_test_with_randoms("random_if.bms", &selected_randoms);
}

#[test]
fn autoplay_random_nested_if() {
    let selected_randoms =
        random_seeds::load_selected_randoms(test_bms_dir(), "random_nested_if.bms");
    run_autoplay_test_with_randoms("random_nested_if.bms", &selected_randoms);
}

#[test]
fn autoplay_encoding_sjis() {
    run_autoplay_test("encoding_sjis.bms");
}

#[test]
fn autoplay_encoding_utf8() {
    run_autoplay_test("encoding_utf8.bms");
}

#[test]
fn autoplay_defexrank() {
    run_autoplay_test("defexrank.bms");
}

#[test]
fn autoplay_timing_extreme() {
    run_autoplay_test("timing_extreme.bms");
}

// --- bmson tests ---

#[test]
fn autoplay_bmson_minimal_7k() {
    run_autoplay_test_bmson("bmson_minimal_7k.bmson");
}

#[test]
fn autoplay_bmson_bpm_change() {
    run_autoplay_test_bmson("bmson_bpm_change.bmson");
}

#[test]
fn autoplay_bmson_longnote() {
    run_autoplay_test_bmson("bmson_longnote.bmson");
}

#[test]
fn autoplay_bmson_stop_sequence() {
    run_autoplay_test_bmson("bmson_stop_sequence.bmson");
}

#[test]
fn autoplay_bmson_mine_invisible() {
    run_autoplay_test_bmson("bmson_mine_invisible.bmson");
}

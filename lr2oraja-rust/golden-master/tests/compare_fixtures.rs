// Golden master tests: compare Rust BMS/bmson parser output against Java fixtures

use std::path::{Path, PathBuf};

use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::LNTYPE_LONGNOTE;
use bms_model::bmson_decoder::BMSONDecoder;
use bms_model::chart_information::ChartInformation;
use golden_master::{
    assert_bmson_model_matches_fixture, assert_model_matches_fixture, load_fixture,
};

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

fn legacy_fixture_name(chart_name: &str) -> String {
    let stem = Path::new(chart_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(chart_name);
    format!("{stem}.json")
}

fn resolve_fixture_path(chart_name: &str) -> PathBuf {
    // New naming keeps extension for uniqueness, e.g. 9key_pms.bms.json.
    let modern = fixtures_dir().join(format!("{chart_name}.json"));
    if modern.exists() {
        return modern;
    }

    // Backward-compatible fallback during migration.
    let legacy = fixtures_dir().join(legacy_fixture_name(chart_name));
    if legacy.exists() {
        return legacy;
    }

    panic!(
        "Fixture not found for {chart_name}. Tried: {} and {}",
        modern.display(),
        legacy.display()
    );
}

/// Test a single BMS file against its Java fixture
fn run_golden_master_test(bms_name: &str) {
    let fixture_path = resolve_fixture_path(bms_name);
    let bms_path = test_bms_dir().join(bms_name);

    assert!(
        bms_path.exists(),
        "BMS file not found: {}",
        bms_path.display()
    );

    let fixture = load_fixture(&fixture_path).expect("Failed to load fixture");
    let model = BMSDecoder::new()
        .decode_path(&bms_path)
        .expect("Failed to parse BMS");

    assert_model_matches_fixture(&model, &fixture);
}

#[test]
fn golden_master_minimal_7k() {
    run_golden_master_test("minimal_7k.bms");
}

#[test]
fn golden_master_5key() {
    run_golden_master_test("5key.bms");
}

#[test]
fn golden_master_longnote_types() {
    run_golden_master_test("longnote_types.bms");
}

#[test]
fn golden_master_bpm_change() {
    run_golden_master_test("bpm_change.bms");
}

#[test]
fn golden_master_bpm_stop_combo() {
    run_golden_master_test("bpm_stop_combo.bms");
}

#[test]
fn golden_master_stop_sequence() {
    run_golden_master_test("stop_sequence.bms");
}

#[test]
fn golden_master_mine_notes() {
    run_golden_master_test("mine_notes.bms");
}

#[test]
fn golden_master_empty_measures() {
    run_golden_master_test("empty_measures.bms");
}

#[test]
fn golden_master_9key_pms() {
    run_golden_master_test("9key_pms.bms");
}

#[test]
fn golden_master_9key_pms_pms() {
    run_golden_master_test("9key_pms.pms");
}

#[test]
fn golden_master_14key_dp() {
    run_golden_master_test("14key_dp.bms");
}

#[test]
fn golden_master_scratch_bss() {
    run_golden_master_test("scratch_bss.bms");
}

#[test]
fn golden_master_encoding_sjis() {
    run_golden_master_test("encoding_sjis.bms");
}

#[test]
fn golden_master_encoding_utf8() {
    run_golden_master_test("encoding_utf8.bms");
}

#[test]
fn golden_master_defexrank() {
    run_golden_master_test("defexrank.bms");
}

#[test]
fn golden_master_timing_extreme() {
    run_golden_master_test("timing_extreme.bms");
}

#[test]
fn golden_master_random_if() {
    // Use the same deterministic random selections as Java fixture generation.
    let selected_randoms = random_seeds::load_selected_randoms(test_bms_dir(), "random_if.bms");

    let fixture_path = resolve_fixture_path("random_if.bms");
    let bms_path = test_bms_dir().join("random_if.bms");
    assert!(
        bms_path.exists(),
        "BMS file not found: {}",
        bms_path.display()
    );

    let fixture = load_fixture(&fixture_path).expect("Failed to load fixture");
    let info = ChartInformation::new(Some(bms_path), LNTYPE_LONGNOTE, Some(selected_randoms));
    let model = BMSDecoder::new().decode(info).expect("Failed to parse BMS");

    assert_model_matches_fixture(&model, &fixture);
}

#[test]
fn golden_master_random_nested_if() {
    // Use the same deterministic random selections as Java fixture generation.
    let selected_randoms =
        random_seeds::load_selected_randoms(test_bms_dir(), "random_nested_if.bms");

    let fixture_path = resolve_fixture_path("random_nested_if.bms");
    let bms_path = test_bms_dir().join("random_nested_if.bms");
    assert!(
        bms_path.exists(),
        "BMS file not found: {}",
        bms_path.display()
    );

    let fixture = load_fixture(&fixture_path).expect("Failed to load fixture");
    let info = ChartInformation::new(Some(bms_path), LNTYPE_LONGNOTE, Some(selected_randoms));
    let model = BMSDecoder::new().decode(info).expect("Failed to parse BMS");

    assert_model_matches_fixture(&model, &fixture);
}

// --- bmson golden master tests ---

/// Test a single bmson file against its Java fixture
fn run_bmson_golden_master_test(bmson_name: &str) {
    let fixture_path = resolve_fixture_path(bmson_name);
    let bmson_path = test_bms_dir().join(bmson_name);
    assert!(
        bmson_path.exists(),
        "bmson file not found: {}",
        bmson_path.display()
    );

    let fixture = load_fixture(&fixture_path).expect("Failed to load fixture");
    let model = BMSONDecoder::new(LNTYPE_LONGNOTE)
        .decode_path(&bmson_path)
        .expect("Failed to parse bmson");

    assert_bmson_model_matches_fixture(&model, &fixture);
}

#[test]
fn golden_master_bmson_minimal_7k() {
    run_bmson_golden_master_test("bmson_minimal_7k.bmson");
}

#[test]
fn golden_master_bmson_bpm_change() {
    run_bmson_golden_master_test("bmson_bpm_change.bmson");
}

#[test]
fn golden_master_bmson_longnote() {
    run_bmson_golden_master_test("bmson_longnote.bmson");
}

#[test]
fn golden_master_bmson_stop_sequence() {
    run_bmson_golden_master_test("bmson_stop_sequence.bmson");
}

#[test]
fn golden_master_bmson_mine_invisible() {
    run_bmson_golden_master_test("bmson_mine_invisible.bmson");
}

// --- Edge case BMS golden master tests ---
// These require Java fixture generation (see Deferred Items in CLAUDE.md).

#[test]
fn golden_master_ln_bpm_cross() {
    run_golden_master_test("ln_bpm_cross.bms");
}

#[test]
fn golden_master_bpm_extreme() {
    run_golden_master_test("bpm_extreme.bms");
}

#[test]
fn golden_master_multi_stop_rapid() {
    run_golden_master_test("multi_stop_rapid.bms");
}

#[test]
fn golden_master_ln_overlap() {
    run_golden_master_test("ln_overlap.bms");
}

#[test]
#[ignore = "known bms-model parser note ordering difference at same time_us"]
fn golden_master_channel_extended() {
    run_golden_master_test("channel_extended.bms");
}

#[test]
fn golden_master_bmson_bpm_ln_cross() {
    run_bmson_golden_master_test("bmson_bpm_ln_cross.bmson");
}

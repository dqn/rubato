// Golden master tests: compare Rust SongInformation::from(&BMSModel) against Java SongInformation fixture export.

use std::path::Path;

use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::LNTYPE_LONGNOTE;
use bms_model::bmson_decoder::BMSONDecoder;
use bms_model::chart_information::ChartInformation;
use golden_master::song_information_fixtures::{SongInformationFixture, SongInformationTestCase};
use rubato_types::song_information::SongInformation;

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

fn load_song_information_fixture() -> SongInformationFixture {
    let path = fixtures_dir().join("song_information.json");
    assert!(
        path.exists(),
        "SongInformation fixture not found: {}. Run `just golden-master-song-info-gen` first.",
        path.display()
    );
    let content = std::fs::read_to_string(&path).expect("Failed to read fixture");
    serde_json::from_str(&content).expect("Failed to parse fixture")
}

fn find_test_case<'a>(
    fixture: &'a SongInformationFixture,
    filename: &str,
) -> &'a SongInformationTestCase {
    fixture
        .test_cases
        .iter()
        .find(|tc| tc.filename == filename)
        .unwrap_or_else(|| panic!("Test case not found for {filename}"))
}

/// Parse speedchange CSV "speed,time,speed,time,..." into [f64; 2] pairs.
fn parse_speedchange(s: &str) -> Vec<[f64; 2]> {
    if s.is_empty() {
        return Vec::new();
    }
    let parts: Vec<&str> = s.split(',').collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i + 1 < parts.len() {
        if let (Ok(speed), Ok(time)) = (parts[i].parse::<f64>(), parts[i + 1].parse::<f64>()) {
            result.push([speed, time]);
        }
        i += 2;
    }
    result
}

/// Compare speedchange values numerically to avoid f64 Display formatting differences
/// (Rust: "120" vs Java: "120.0").
fn compare_speedchange(rust_sc: &str, java_sc: &str, diffs: &mut Vec<String>) {
    let rust_pairs = parse_speedchange(rust_sc);
    let java_pairs = parse_speedchange(java_sc);

    if rust_pairs.len() != java_pairs.len() {
        diffs.push(format!(
            "speedchange entry count: rust={} java={} (rust={:?} java={:?})",
            rust_pairs.len(),
            java_pairs.len(),
            rust_sc,
            java_sc
        ));
        return;
    }

    for (i, (r, j)) in rust_pairs.iter().zip(java_pairs.iter()).enumerate() {
        if (r[0] - j[0]).abs() > 0.001 || (r[1] - j[1]).abs() > 1.0 {
            diffs.push(format!("speedchange[{i}]: rust={:?} java={:?}", r, j));
        }
    }
}

fn compare_song_information(rust: &SongInformation, java: &SongInformationTestCase) -> Vec<String> {
    let mut diffs = Vec::new();

    // sha256: exact match
    if rust.sha256 != java.sha256 {
        diffs.push(format!(
            "sha256: rust={:?} java={:?}",
            rust.sha256, java.sha256
        ));
    }

    // Note counts: exact match
    if rust.n != java.n {
        diffs.push(format!("n: rust={} java={}", rust.n, java.n));
    }
    if rust.ln != java.ln {
        diffs.push(format!("ln: rust={} java={}", rust.ln, java.ln));
    }
    if rust.s != java.s {
        diffs.push(format!("s: rust={} java={}", rust.s, java.s));
    }
    if rust.ls != java.ls {
        diffs.push(format!("ls: rust={} java={}", rust.ls, java.ls));
    }

    // total: +/-0.001
    if (rust.total - java.total).abs() > 0.001 {
        diffs.push(format!("total: rust={} java={}", rust.total, java.total));
    }

    // density/peakdensity/enddensity: +/-0.01 (float precision differences)
    if (rust.density - java.density).abs() > 0.01 {
        diffs.push(format!(
            "density: rust={} java={} (diff={})",
            rust.density,
            java.density,
            (rust.density - java.density).abs()
        ));
    }
    if (rust.peakdensity - java.peakdensity).abs() > 0.01 {
        diffs.push(format!(
            "peakdensity: rust={} java={} (diff={})",
            rust.peakdensity,
            java.peakdensity,
            (rust.peakdensity - java.peakdensity).abs()
        ));
    }
    if (rust.enddensity - java.enddensity).abs() > 0.01 {
        diffs.push(format!(
            "enddensity: rust={} java={} (diff={})",
            rust.enddensity,
            java.enddensity,
            (rust.enddensity - java.enddensity).abs()
        ));
    }

    // mainbpm: +/-0.001
    if (rust.mainbpm - java.mainbpm).abs() > 0.001 {
        diffs.push(format!(
            "mainbpm: rust={} java={}",
            rust.mainbpm, java.mainbpm
        ));
    }

    // distribution, lanenotes: exact string match
    if rust.distribution != java.distribution {
        diffs.push(format!(
            "distribution: rust={:?} java={:?}",
            rust.distribution, java.distribution
        ));
    }
    if rust.lanenotes != java.lanenotes {
        diffs.push(format!(
            "lanenotes: rust={:?} java={:?}",
            rust.lanenotes, java.lanenotes
        ));
    }

    // speedchange: numeric comparison (Rust format!("{}", 120.0) -> "120", Java -> "120.0")
    compare_speedchange(&rust.speedchange, &java.speedchange, &mut diffs);

    diffs
}

fn assert_song_information_matches(
    rust: &SongInformation,
    java: &SongInformationTestCase,
    filename: &str,
) {
    let diffs = compare_song_information(rust, java);
    if !diffs.is_empty() {
        panic!(
            "SongInformation mismatch for {} ({} differences):\n{}",
            filename,
            diffs.len(),
            diffs
                .iter()
                .map(|d| format!("  - {d}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Run a single BMS golden master song information test
fn run_song_information_test(bms_name: &str) {
    let fixture = load_song_information_fixture();
    let test_case = find_test_case(&fixture, bms_name);

    let bms_path = test_bms_dir().join(bms_name);
    assert!(
        bms_path.exists(),
        "BMS file not found: {}",
        bms_path.display()
    );

    let model = BMSDecoder::new()
        .decode_path(&bms_path)
        .expect("Failed to parse BMS");
    let info = SongInformation::from(&model);

    assert_song_information_matches(&info, test_case, bms_name);
}

/// Run a BMS test with fixed random selections
fn run_song_information_test_with_randoms(bms_name: &str, randoms: &[i32]) {
    let fixture = load_song_information_fixture();
    let test_case = find_test_case(&fixture, bms_name);

    let bms_path = test_bms_dir().join(bms_name);
    assert!(
        bms_path.exists(),
        "BMS file not found: {}",
        bms_path.display()
    );

    let info = ChartInformation::new(Some(bms_path), LNTYPE_LONGNOTE, Some(randoms.to_vec()));
    let model = BMSDecoder::new().decode(info).expect("Failed to parse BMS");
    let song_info = SongInformation::from(&model);

    assert_song_information_matches(&song_info, test_case, bms_name);
}

/// Run a bmson golden master song information test
fn run_song_information_test_bmson(bmson_name: &str) {
    let fixture = load_song_information_fixture();
    let test_case = find_test_case(&fixture, bmson_name);

    let bmson_path = test_bms_dir().join(bmson_name);
    assert!(
        bmson_path.exists(),
        "bmson file not found: {}",
        bmson_path.display()
    );

    let model = BMSONDecoder::new(LNTYPE_LONGNOTE)
        .decode_path(&bmson_path)
        .expect("Failed to parse bmson");
    let info = SongInformation::from(&model);

    assert_song_information_matches(&info, test_case, bmson_name);
}

// --- BMS tests ---

#[test]
fn song_info_minimal_7k() {
    run_song_information_test("minimal_7k.bms");
}

#[test]
fn song_info_5key() {
    run_song_information_test("5key.bms");
}

#[test]
fn song_info_14key_dp() {
    run_song_information_test("14key_dp.bms");
}

#[test]
fn song_info_9key_pms() {
    run_song_information_test("9key_pms.bms");
}

#[test]
fn song_info_9key_pms_pms() {
    run_song_information_test("9key_pms.pms");
}

#[test]
fn song_info_bpm_change() {
    run_song_information_test("bpm_change.bms");
}

#[test]
fn song_info_bpm_stop_combo() {
    run_song_information_test("bpm_stop_combo.bms");
}

#[test]
fn song_info_stop_sequence() {
    run_song_information_test("stop_sequence.bms");
}

#[test]
fn song_info_longnote_types() {
    run_song_information_test("longnote_types.bms");
}

#[test]
fn song_info_mine_notes() {
    run_song_information_test("mine_notes.bms");
}

#[test]
fn song_info_scratch_bss() {
    run_song_information_test("scratch_bss.bms");
}

#[test]
fn song_info_empty_measures() {
    run_song_information_test("empty_measures.bms");
}

#[test]
fn song_info_random_if() {
    let selected_randoms = random_seeds::load_selected_randoms(test_bms_dir(), "random_if.bms");
    run_song_information_test_with_randoms("random_if.bms", &selected_randoms);
}

#[test]
fn song_info_random_nested_if() {
    let selected_randoms =
        random_seeds::load_selected_randoms(test_bms_dir(), "random_nested_if.bms");
    run_song_information_test_with_randoms("random_nested_if.bms", &selected_randoms);
}

#[test]
fn song_info_encoding_sjis() {
    run_song_information_test("encoding_sjis.bms");
}

#[test]
fn song_info_encoding_utf8() {
    run_song_information_test("encoding_utf8.bms");
}

#[test]
fn song_info_defexrank() {
    run_song_information_test("defexrank.bms");
}

#[test]
fn song_info_timing_extreme() {
    run_song_information_test("timing_extreme.bms");
}

// --- bmson tests ---
// bmson speedchange times are consistently lower in Rust due to
// timeline time calculation differences in the bmson decoder.

#[test]
fn song_info_bmson_minimal_7k() {
    run_song_information_test_bmson("bmson_minimal_7k.bmson");
}

#[test]
fn song_info_bmson_bpm_change() {
    run_song_information_test_bmson("bmson_bpm_change.bmson");
}

#[test]
fn song_info_bmson_longnote() {
    run_song_information_test_bmson("bmson_longnote.bmson");
}

#[test]
fn song_info_bmson_stop_sequence() {
    run_song_information_test_bmson("bmson_stop_sequence.bmson");
}

#[test]
fn song_info_bmson_mine_invisible() {
    run_song_information_test_bmson("bmson_mine_invisible.bmson");
}

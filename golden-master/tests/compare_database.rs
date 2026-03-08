// Golden master tests: compare Rust SongData::new_from_model() against Java SongData fixture export.

use std::path::Path;

use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::LNTYPE_LONGNOTE;
use bms_model::bmson_decoder::BMSONDecoder;
use bms_model::chart_information::ChartInformation;
use golden_master::database_fixtures::{DatabaseFixture, SongDataFixture};
use rubato_types::song_data::SongData;

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

fn load_database_fixture() -> DatabaseFixture {
    let path = fixtures_dir().join("database_song_data.json");
    assert!(
        path.exists(),
        "Database fixture not found: {}. Run `just golden-master-database-gen` first.",
        path.display()
    );
    let content = std::fs::read_to_string(&path).expect("Failed to read fixture");
    serde_json::from_str(&content).expect("Failed to parse fixture")
}

fn find_test_case<'a>(fixture: &'a DatabaseFixture, filename: &str) -> &'a SongDataFixture {
    fixture
        .test_cases
        .iter()
        .find(|tc| tc.filename == filename)
        .unwrap_or_else(|| panic!("Test case not found for {filename}"))
}

fn compare_song_data(rust: &SongData, java: &SongDataFixture) -> Vec<String> {
    let mut diffs = Vec::new();

    if rust.file.md5 != java.md5 {
        diffs.push(format!("md5: rust={:?} java={:?}", rust.file.md5, java.md5));
    }
    if rust.file.sha256 != java.sha256 {
        diffs.push(format!(
            "sha256: rust={:?} java={:?}",
            rust.file.sha256, java.sha256
        ));
    }
    if rust.metadata.title != java.title {
        diffs.push(format!(
            "title: rust={:?} java={:?}",
            rust.metadata.title, java.title
        ));
    }
    if rust.metadata.subtitle != java.subtitle {
        diffs.push(format!(
            "subtitle: rust={:?} java={:?}",
            rust.metadata.subtitle, java.subtitle
        ));
    }
    if rust.metadata.genre != java.genre {
        diffs.push(format!(
            "genre: rust={:?} java={:?}",
            rust.metadata.genre, java.genre
        ));
    }
    if rust.metadata.artist != java.artist {
        diffs.push(format!(
            "artist: rust={:?} java={:?}",
            rust.metadata.artist, java.artist
        ));
    }
    if rust.metadata.subartist != java.subartist {
        diffs.push(format!(
            "subartist: rust={:?} java={:?}",
            rust.metadata.subartist, java.subartist
        ));
    }
    if rust.metadata.tag != java.tag {
        diffs.push(format!(
            "tag: rust={:?} java={:?}",
            rust.metadata.tag, java.tag
        ));
    }
    // path is set by new_from_model in Rust but not in Java fixture export;
    // skip comparison when Java path is empty (database layer sets it later)
    let rust_path = rust.path().unwrap_or("");
    if !java.path.is_empty() && rust_path != java.path {
        diffs.push(format!("path: rust={:?} java={:?}", rust_path, java.path));
    }
    if rust.folder != java.folder {
        diffs.push(format!(
            "folder: rust={:?} java={:?}",
            rust.folder, java.folder
        ));
    }
    if rust.file.banner != java.banner {
        diffs.push(format!(
            "banner: rust={:?} java={:?}",
            rust.file.banner, java.banner
        ));
    }
    if rust.file.stagefile != java.stagefile {
        diffs.push(format!(
            "stagefile: rust={:?} java={:?}",
            rust.file.stagefile, java.stagefile
        ));
    }
    if rust.file.backbmp != java.backbmp {
        diffs.push(format!(
            "backbmp: rust={:?} java={:?}",
            rust.file.backbmp, java.backbmp
        ));
    }
    if rust.file.preview != java.preview {
        diffs.push(format!(
            "preview: rust={:?} java={:?}",
            rust.file.preview, java.preview
        ));
    }
    if rust.parent != java.parent {
        diffs.push(format!(
            "parent: rust={:?} java={:?}",
            rust.parent, java.parent
        ));
    }
    if rust.chart.level != java.level {
        diffs.push(format!(
            "level: rust={} java={}",
            rust.chart.level, java.level
        ));
    }
    if rust.chart.mode != java.mode {
        diffs.push(format!("mode: rust={} java={}", rust.chart.mode, java.mode));
    }
    if rust.chart.difficulty != java.difficulty {
        diffs.push(format!(
            "difficulty: rust={} java={}",
            rust.chart.difficulty, java.difficulty
        ));
    }
    if rust.chart.judge != java.judge {
        diffs.push(format!(
            "judge: rust={} java={}",
            rust.chart.judge, java.judge
        ));
    }
    if rust.chart.minbpm != java.minbpm {
        diffs.push(format!(
            "minbpm: rust={} java={}",
            rust.chart.minbpm, java.minbpm
        ));
    }
    if rust.chart.maxbpm != java.maxbpm {
        diffs.push(format!(
            "maxbpm: rust={} java={}",
            rust.chart.maxbpm, java.maxbpm
        ));
    }
    // length: allow ±1ms tolerance (Java getLastTime vs Rust total_time_us/1000)
    if (rust.chart.length - java.length).abs() > 1 {
        diffs.push(format!(
            "length: rust={} java={} (diff={})",
            rust.chart.length,
            java.length,
            rust.chart.length - java.length
        ));
    }
    if rust.chart.notes != java.notes {
        diffs.push(format!(
            "notes: rust={} java={}",
            rust.chart.notes, java.notes
        ));
    }
    if rust.chart.feature != java.feature {
        diffs.push(format!(
            "feature: rust={:#010b} java={:#010b} (rust={} java={})",
            rust.chart.feature, java.feature, rust.chart.feature, java.feature
        ));
    }
    if rust.chart.content != java.content {
        diffs.push(format!(
            "content: rust={:#010b} java={:#010b} (rust={} java={})",
            rust.chart.content, java.content, rust.chart.content, java.content
        ));
    }
    if rust.chart.date != java.date {
        diffs.push(format!("date: rust={} java={}", rust.chart.date, java.date));
    }
    if rust.favorite != java.favorite {
        diffs.push(format!(
            "favorite: rust={} java={}",
            rust.favorite, java.favorite
        ));
    }
    if rust.chart.adddate != java.adddate {
        diffs.push(format!(
            "adddate: rust={} java={}",
            rust.chart.adddate, java.adddate
        ));
    }
    // charthash is set by new_from_model in Rust but not in Java fixture export;
    // skip comparison when Java charthash is empty
    let rust_charthash = rust.file.charthash.as_deref().unwrap_or("");
    if !java.charthash.is_empty() && rust_charthash != java.charthash {
        diffs.push(format!(
            "charthash: rust={:?} java={:?}",
            rust_charthash, java.charthash
        ));
    }

    diffs
}

fn assert_song_data_matches(rust: &SongData, java: &SongDataFixture, filename: &str) {
    let diffs = compare_song_data(rust, java);
    if !diffs.is_empty() {
        panic!(
            "SongData mismatch for {} ({} differences):\n{}",
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

/// Run a single BMS golden master database test
fn run_database_test(bms_name: &str) {
    let fixture = load_database_fixture();
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
    let song_data = SongData::new_from_model(model, false);

    assert_song_data_matches(&song_data, test_case, bms_name);
}

/// Run a BMS test with fixed random selections
fn run_database_test_with_randoms(bms_name: &str, randoms: &[i32]) {
    let fixture = load_database_fixture();
    let test_case = find_test_case(&fixture, bms_name);

    let bms_path = test_bms_dir().join(bms_name);
    assert!(
        bms_path.exists(),
        "BMS file not found: {}",
        bms_path.display()
    );

    let info = ChartInformation::new(Some(bms_path), LNTYPE_LONGNOTE, Some(randoms.to_vec()));
    let model = BMSDecoder::new().decode(info).expect("Failed to parse BMS");
    let song_data = SongData::new_from_model(model, false);

    assert_song_data_matches(&song_data, test_case, bms_name);
}

/// Run a bmson golden master database test
fn run_database_test_bmson(bmson_name: &str) {
    let fixture = load_database_fixture();
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
    let song_data = SongData::new_from_model(model, false);

    assert_song_data_matches(&song_data, test_case, bmson_name);
}

// --- BMS tests ---

#[test]
fn database_minimal_7k() {
    run_database_test("minimal_7k.bms");
}

#[test]
fn database_5key() {
    run_database_test("5key.bms");
}

#[test]
fn database_14key_dp() {
    run_database_test("14key_dp.bms");
}

#[test]
fn database_9key_pms() {
    run_database_test("9key_pms.bms");
}

#[test]
fn database_9key_pms_pms() {
    run_database_test("9key_pms.pms");
}

#[test]
fn database_bpm_change() {
    run_database_test("bpm_change.bms");
}

#[test]
fn database_bpm_stop_combo() {
    run_database_test("bpm_stop_combo.bms");
}

#[test]
fn database_stop_sequence() {
    run_database_test("stop_sequence.bms");
}

#[test]
fn database_longnote_types() {
    run_database_test("longnote_types.bms");
}

#[test]
fn database_mine_notes() {
    run_database_test("mine_notes.bms");
}

#[test]
fn database_scratch_bss() {
    run_database_test("scratch_bss.bms");
}

#[test]
fn database_empty_measures() {
    run_database_test("empty_measures.bms");
}

#[test]
fn database_random_if() {
    let selected_randoms = random_seeds::load_selected_randoms(test_bms_dir(), "random_if.bms");
    run_database_test_with_randoms("random_if.bms", &selected_randoms);
}

#[test]
fn database_random_nested_if() {
    let selected_randoms =
        random_seeds::load_selected_randoms(test_bms_dir(), "random_nested_if.bms");
    run_database_test_with_randoms("random_nested_if.bms", &selected_randoms);
}

#[test]
fn database_encoding_sjis() {
    run_database_test("encoding_sjis.bms");
}

#[test]
fn database_encoding_utf8() {
    run_database_test("encoding_utf8.bms");
}

#[test]
fn database_defexrank() {
    run_database_test("defexrank.bms");
}

#[test]
fn database_timing_extreme() {
    run_database_test("timing_extreme.bms");
}

// --- bmson tests ---

#[test]
fn database_bmson_minimal_7k() {
    run_database_test_bmson("bmson_minimal_7k.bmson");
}

#[test]
fn database_bmson_bpm_change() {
    run_database_test_bmson("bmson_bpm_change.bmson");
}

#[test]
fn database_bmson_longnote() {
    run_database_test_bmson("bmson_longnote.bmson");
}

#[test]
fn database_bmson_stop_sequence() {
    run_database_test_bmson("bmson_stop_sequence.bmson");
}

#[test]
fn database_bmson_mine_invisible() {
    run_database_test_bmson("bmson_mine_invisible.bmson");
}

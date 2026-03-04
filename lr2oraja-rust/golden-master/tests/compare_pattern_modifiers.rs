// Golden master tests: compare Rust pattern modifiers against Java fixture export.
//
// Tests deterministic modifiers: AutoplayModifier, PracticeModifier, ScrollSpeedModifier (REMOVE).

use std::path::Path;

use beatoraja_core::pattern::autoplay_modifier::AutoplayModifier;
use beatoraja_core::pattern::pattern_modifier::{AssistLevel, PatternModifier};
use beatoraja_core::pattern::practice_modifier::PracticeModifier;
use beatoraja_core::pattern::scroll_speed_modifier::ScrollSpeedModifier;
use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::BMSModel;
use bms_model::note::{Note, TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_LONGNOTE};
use golden_master::pattern_modifier_detail_fixtures::{
    ModifierNote, PatternModifierDetailFixture, PatternModifierTestCase,
};

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

fn load_fixture() -> PatternModifierDetailFixture {
    let path = fixtures_dir().join("pattern_modifier_detail.json");
    assert!(
        path.exists(),
        "Pattern modifier detail fixture not found: {}. Run the Java exporter first.",
        path.display()
    );
    let content = std::fs::read_to_string(&path).expect("Failed to read fixture");
    serde_json::from_str(&content).expect("Failed to parse fixture")
}

fn find_test_cases<'a>(
    fixture: &'a PatternModifierDetailFixture,
    modifier_type: &str,
    bms_file: &str,
) -> Vec<&'a PatternModifierTestCase> {
    fixture
        .test_cases
        .iter()
        .filter(|tc| tc.modifier_type == modifier_type && tc.bms_file == bms_file)
        .collect()
}

/// Convert a Rust Note variant to the Java fixture string representation.
fn note_type_to_string(note: &Note) -> &'static str {
    match note {
        Note::Normal(_) => "Normal",
        Note::Long { note_type, .. } => match *note_type {
            TYPE_LONGNOTE => "LongNote",
            TYPE_CHARGENOTE => "ChargeNote",
            TYPE_HELLCHARGENOTE => "HellChargeNote",
            _ => "LongNote", // TYPE_UNDEFINED defaults to LongNote in Java
        },
        Note::Mine { .. } => "Mine",
    }
}

/// Capture notes from a BMSModel (timeline-based) in the same format as the Java exporter.
/// Returns (lane, time_ms, note_type, end_time_ms) tuples sorted by (time_ms, lane).
/// Skips LN end notes.
fn capture_notes(model: &BMSModel) -> Vec<ModifierNote> {
    let keys = model.get_mode().map(|m| m.key()).unwrap_or(0);
    let timelines = model.get_all_time_lines();
    let mut notes: Vec<ModifierNote> = Vec::new();

    for (tl_idx, tl) in timelines.iter().enumerate() {
        for lane in 0..keys {
            if let Some(note) = tl.get_note(lane) {
                // Skip LN end notes
                if note.is_end() {
                    continue;
                }

                let time_ms = (tl.get_micro_time() / 1000) as i32;
                let end_time_ms = if note.is_long() {
                    // Find the paired end note by scanning forward
                    let mut end_time = None;
                    for future_tl in &timelines[(tl_idx + 1)..] {
                        if let Some(end_note) = future_tl.get_note(lane)
                            && end_note.is_long()
                            && end_note.is_end()
                        {
                            end_time = Some((future_tl.get_micro_time() / 1000) as i32);
                            break;
                        }
                    }
                    end_time
                } else {
                    None
                };

                notes.push(ModifierNote {
                    lane: lane as usize,
                    time_ms,
                    note_type: note_type_to_string(note).to_string(),
                    end_time_ms,
                });
            }
        }
    }

    // Also capture hidden (invisible) notes
    for tl in timelines.iter() {
        for lane in 0..keys {
            if let Some(_note) = tl.get_hidden_note(lane) {
                let time_ms = (tl.get_micro_time() / 1000) as i32;
                notes.push(ModifierNote {
                    lane: lane as usize,
                    time_ms,
                    note_type: "Invisible".to_string(),
                    end_time_ms: None,
                });
            }
        }
    }

    // Sort by time then lane to match Java output order
    notes.sort_by(|a, b| a.time_ms.cmp(&b.time_ms).then_with(|| a.lane.cmp(&b.lane)));
    notes
}

/// Compare two note lists with +/-2ms time tolerance.
fn compare_notes(
    rust_notes: &[ModifierNote],
    java_notes: &[ModifierNote],
    label: &str,
) -> Vec<String> {
    let mut diffs = Vec::new();

    if rust_notes.len() != java_notes.len() {
        diffs.push(format!(
            "{} note_count: rust={} java={}",
            label,
            rust_notes.len(),
            java_notes.len()
        ));
    }

    let min_len = rust_notes.len().min(java_notes.len());
    for i in 0..min_len {
        let rn = &rust_notes[i];
        let jn = &java_notes[i];

        if rn.lane != jn.lane {
            diffs.push(format!(
                "{} note[{}] lane: rust={} java={}",
                label, i, rn.lane, jn.lane
            ));
        }

        // +/-2ms tolerance
        if (rn.time_ms - jn.time_ms).abs() > 2 {
            diffs.push(format!(
                "{} note[{}] time_ms: rust={} java={} (diff={})",
                label,
                i,
                rn.time_ms,
                jn.time_ms,
                rn.time_ms - jn.time_ms
            ));
        }

        if rn.note_type != jn.note_type {
            diffs.push(format!(
                "{} note[{}] note_type: rust={} java={}",
                label, i, rn.note_type, jn.note_type
            ));
        }

        // LN end time comparison with +/-2ms tolerance
        match (&rn.end_time_ms, &jn.end_time_ms) {
            (Some(r_end), Some(j_end)) => {
                if (r_end - j_end).abs() > 2 {
                    diffs.push(format!(
                        "{} note[{}] end_time_ms: rust={} java={}",
                        label, i, r_end, j_end
                    ));
                }
            }
            (None, Some(j_end)) => {
                diffs.push(format!(
                    "{} note[{}] end_time_ms: rust=None java={}",
                    label, i, j_end
                ));
            }
            (Some(r_end), None) => {
                diffs.push(format!(
                    "{} note[{}] end_time_ms: rust={} java=None",
                    label, i, r_end
                ));
            }
            (None, None) => {}
        }
    }

    diffs
}

fn assert_no_diffs(diffs: &[String], test_name: &str) {
    if !diffs.is_empty() {
        panic!(
            "Pattern modifier mismatch for {} ({} differences):\n{}",
            test_name,
            diffs.len(),
            diffs
                .iter()
                .map(|d| format!("  - {d}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn rust_assist_level_str(level: AssistLevel) -> &'static str {
    match level {
        AssistLevel::None => "NONE",
        AssistLevel::LightAssist => "LIGHT_ASSIST",
        AssistLevel::Assist => "ASSIST",
    }
}

// =========================================================================
// Autoplay modifier tests
// =========================================================================

fn run_autoplay_test(bms_file: &str) {
    let fixture = load_fixture();
    let test_cases = find_test_cases(&fixture, "autoplay", bms_file);
    assert!(
        !test_cases.is_empty(),
        "No autoplay test case for {bms_file}"
    );

    for tc in test_cases {
        let bms_path = test_bms_dir().join(&tc.bms_file);
        assert!(
            bms_path.exists(),
            "BMS file not found: {}",
            bms_path.display()
        );

        let mut model = BMSDecoder::new()
            .decode_path(&bms_path)
            .expect("Failed to parse BMS");

        // Verify notes_before matches
        let notes_before = capture_notes(&model);
        let diffs = compare_notes(&notes_before, &tc.notes_before, "notes_before");
        assert_no_diffs(&diffs, &format!("autoplay/{}/before", tc.bms_file));

        // Extract config
        let lanes: Vec<i32> = tc.config["lanes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap() as i32)
            .collect();

        // Apply modifier
        let mut modifier = AutoplayModifier::new(lanes);
        modifier.modify(&mut model);

        // Compare notes_after
        let notes_after = capture_notes(&model);
        let diffs = compare_notes(&notes_after, &tc.notes_after, "notes_after");
        assert_no_diffs(&diffs, &format!("autoplay/{}/after", tc.bms_file));

        // Compare assist level
        let rust_assist = rust_assist_level_str(modifier.get_assist_level());
        assert_eq!(
            rust_assist, tc.assist_level,
            "autoplay/{}: assist_level mismatch: rust={} java={}",
            tc.bms_file, rust_assist, tc.assist_level
        );
    }
}

#[test]
fn pattern_modifier_autoplay_minimal_7k() {
    run_autoplay_test("minimal_7k.bms");
}

#[test]
fn pattern_modifier_autoplay_longnote_types() {
    run_autoplay_test("longnote_types.bms");
}

// =========================================================================
// Practice modifier tests
// =========================================================================

fn run_practice_test(bms_file: &str) {
    let fixture = load_fixture();
    let test_cases = find_test_cases(&fixture, "practice", bms_file);
    assert!(
        !test_cases.is_empty(),
        "No practice test case for {bms_file}"
    );

    for tc in test_cases {
        let bms_path = test_bms_dir().join(&tc.bms_file);
        assert!(
            bms_path.exists(),
            "BMS file not found: {}",
            bms_path.display()
        );

        let mut model = BMSDecoder::new()
            .decode_path(&bms_path)
            .expect("Failed to parse BMS");

        // Verify notes_before matches
        let notes_before = capture_notes(&model);
        let diffs = compare_notes(&notes_before, &tc.notes_before, "notes_before");
        assert_no_diffs(&diffs, &format!("practice/{}/before", tc.bms_file));

        // Extract config
        let start_ms = tc.config["start_ms"].as_i64().unwrap();
        let end_ms = tc.config["end_ms"].as_i64().unwrap();

        // Apply modifier
        let mut modifier = PracticeModifier::new(start_ms, end_ms);
        modifier.modify(&mut model);

        // Compare notes_after
        let notes_after = capture_notes(&model);
        let diffs = compare_notes(&notes_after, &tc.notes_after, "notes_after");
        assert_no_diffs(&diffs, &format!("practice/{}/after", tc.bms_file));

        // Compare assist level
        let rust_assist = rust_assist_level_str(modifier.get_assist_level());
        assert_eq!(
            rust_assist, tc.assist_level,
            "practice/{}: assist_level mismatch: rust={} java={}",
            tc.bms_file, rust_assist, tc.assist_level
        );
    }
}

#[test]
fn pattern_modifier_practice_minimal_7k() {
    run_practice_test("minimal_7k.bms");
}

// =========================================================================
// ScrollSpeed REMOVE modifier tests
// =========================================================================

fn run_scroll_speed_remove_test(bms_file: &str) {
    let fixture = load_fixture();
    let test_cases = find_test_cases(&fixture, "scroll_speed_remove", bms_file);
    assert!(
        !test_cases.is_empty(),
        "No scroll_speed_remove test case for {bms_file}"
    );

    for tc in test_cases {
        let bms_path = test_bms_dir().join(&tc.bms_file);
        assert!(
            bms_path.exists(),
            "BMS file not found: {}",
            bms_path.display()
        );

        let mut model = BMSDecoder::new()
            .decode_path(&bms_path)
            .expect("Failed to parse BMS");

        // Verify notes_before matches
        let notes_before = capture_notes(&model);
        let diffs = compare_notes(&notes_before, &tc.notes_before, "notes_before");
        assert_no_diffs(
            &diffs,
            &format!("scroll_speed_remove/{}/before", tc.bms_file),
        );

        // Apply modifier (ScrollSpeedModifier::new() defaults to Remove mode)
        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        // Notes should be unchanged (scroll modifier doesn't move notes)
        let notes_after = capture_notes(&model);
        let diffs = compare_notes(&notes_after, &tc.notes_after, "notes_after");
        assert_no_diffs(
            &diffs,
            &format!("scroll_speed_remove/{}/after", tc.bms_file),
        );

        // Compare assist level
        let rust_assist = rust_assist_level_str(modifier.get_assist_level());
        assert_eq!(
            rust_assist, tc.assist_level,
            "scroll_speed_remove/{}: assist_level mismatch: rust={} java={}",
            tc.bms_file, rust_assist, tc.assist_level
        );

        // Verify BPM normalization: all BPM changes should be set to initial_bpm
        let ref_bpm = tc.config["ref_bpm"].as_f64().unwrap();
        for tl in model.get_all_time_lines() {
            assert!(
                (tl.get_bpm() - ref_bpm).abs() < 0.001,
                "scroll_speed_remove/{}: BPM not normalized: expected={} got={}",
                tc.bms_file,
                ref_bpm,
                tl.get_bpm()
            );
        }

        // Verify all stops are cleared
        let stop_count: usize = model
            .get_all_time_lines()
            .iter()
            .filter(|tl| tl.get_stop() != 0)
            .count();
        assert!(
            stop_count == 0,
            "scroll_speed_remove/{}: stop events not cleared: {} remain",
            tc.bms_file,
            stop_count
        );
    }
}

#[test]
fn pattern_modifier_scroll_speed_remove_bpm_change() {
    run_scroll_speed_remove_test("bpm_change.bms");
}

#[test]
fn pattern_modifier_scroll_speed_remove_stop_sequence() {
    run_scroll_speed_remove_test("stop_sequence.bms");
}

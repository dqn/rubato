// Golden master tests: compare Rust rule engine output against Java fixtures
//
// Tests judge window calculation and gauge property/sequence behavior
// against Java-exported golden master fixtures.

use std::path::Path;

use beatoraja_play::judge_property::{self, JudgeProperty, NoteType};
use beatoraja_types::clear_type::ClearType;
use beatoraja_types::gauge_property::GaugeProperty;
use beatoraja_types::groove_gauge::Gauge;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;
use golden_master::rule_fixtures::{
    GaugePropertyFixture, GaugeSequenceFixture, JudgeWindowFixture,
};

fn fixtures_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .leak()
}

fn load_json<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = fixtures_dir().join(name);
    assert!(path.exists(), "Fixture not found: {}", path.display());
    let content = std::fs::read_to_string(&path).expect("Failed to read fixture");
    serde_json::from_str(&content).expect("Failed to parse fixture")
}

fn mode_to_judge_property(name: &str) -> JudgeProperty {
    match name {
        "FIVEKEYS" => judge_property::fivekeys(),
        "SEVENKEYS" => judge_property::sevenkeys(),
        "PMS" => judge_property::pms(),
        "KEYBOARD" => judge_property::keyboard(),
        "LR2" => judge_property::lr2(),
        _ => panic!("Unknown mode: {name}"),
    }
}

fn mode_to_gauge_property(name: &str) -> GaugeProperty {
    match name {
        "FIVEKEYS" => GaugeProperty::FiveKeys,
        "SEVENKEYS" => GaugeProperty::SevenKeys,
        "PMS" => GaugeProperty::Pms,
        "KEYBOARD" => GaugeProperty::Keyboard,
        "LR2" => GaugeProperty::Lr2,
        _ => panic!("Unknown mode: {name}"),
    }
}

fn note_type_to_enum(name: &str) -> NoteType {
    match name {
        "NOTE" => NoteType::Note,
        "LONGNOTE_END" => NoteType::LongnoteEnd,
        "SCRATCH" => NoteType::Scratch,
        "LONGSCRATCH_END" => NoteType::LongscratchEnd,
        _ => panic!("Unknown note type: {name}"),
    }
}

/// Gauge type index to ClearType mapping (matches Java gauge type ordering)
fn gauge_index_to_clear_type(index: usize) -> ClearType {
    ClearType::get_clear_type_by_gauge(index as i32).unwrap_or(ClearType::Failed)
}

/// Create a BMSModel with specified total and total_notes for gauge testing.
/// Adds exactly `total_notes` normal notes to a BEAT_7K model.
fn make_model_for_gauge(total: f64, total_notes: usize) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_total(total);
    model.set_mode(Mode::BEAT_7K);

    let key_count = model.get_mode().unwrap().key();

    // Build timelines with normal notes
    let mut timelines = Vec::with_capacity(total_notes);
    for i in 0..total_notes {
        let section = i as f64;
        let time_us = (i as i64 + 1) * 100_000; // 100ms apart
        let mut tl = TimeLine::new(section, time_us, key_count);
        tl.set_bpm(120.0);
        // Place note on lane 0
        let note = Note::new_normal(1);
        tl.set_note(0, Some(note));
        timelines.push(tl);
    }
    model.set_all_time_line(timelines);

    model
}

// =========================================================================
// Judge Window GM Test
// =========================================================================

#[test]
fn golden_master_judge_windows() {
    let fixture: JudgeWindowFixture = load_json("judge_windows.json");
    let mut failures = Vec::new();

    for (i, tc) in fixture.test_cases.iter().enumerate() {
        let prop = mode_to_judge_property(&tc.mode);
        let note_type = note_type_to_enum(&tc.note_type);
        let jwr: [i32; 3] = [
            tc.judge_window_rate[0],
            tc.judge_window_rate[1],
            tc.judge_window_rate[2],
        ];

        // get_judge returns Vec<[i64; 2]> in microseconds — matches fixture format
        let rust_windows = prop.get_judge(note_type, tc.judgerank, &jwr);

        // Compare window count
        if rust_windows.len() != tc.windows.len() {
            failures.push(format!(
                "[{i}] {}/{}/{}/jwr={:?}: window_count rust={} java={}",
                tc.mode,
                tc.note_type,
                tc.judgerank,
                tc.judge_window_rate,
                rust_windows.len(),
                tc.windows.len()
            ));
            continue;
        }

        // Compare each window pair (exact match for integer arithmetic)
        for (j, (rw, jw)) in rust_windows.iter().zip(tc.windows.iter()).enumerate() {
            if jw.len() >= 2 && (rw[0] != jw[0] || rw[1] != jw[1]) {
                failures.push(format!(
                    "[{i}] {}/{}/{}/jwr={:?} window[{j}]: rust={:?} java={:?}",
                    tc.mode, tc.note_type, tc.judgerank, tc.judge_window_rate, rw, jw
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Judge window GM mismatch ({} failures out of {} cases):\n{}",
            failures.len(),
            fixture.test_cases.len(),
            failures
                .iter()
                .take(20)
                .map(|f| format!("  - {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!(
        "Judge window GM: all {} cases passed",
        fixture.test_cases.len()
    );
}

// =========================================================================
// Gauge Property GM Test
// =========================================================================

#[test]
fn golden_master_gauge_properties() {
    let fixture: GaugePropertyFixture = load_json("gauge_properties.json");
    let mut failures = Vec::new();
    let tol = 1e-4_f32;

    for (i, tc) in fixture.test_cases.iter().enumerate() {
        let prop = mode_to_gauge_property(&tc.mode);
        let elements = prop.get_values();
        let elem = &elements[tc.gauge_type_index];
        let label = format!(
            "[{i}] {}/{}/total={}/notes={}",
            tc.mode, tc.gauge_type, tc.total, tc.total_notes
        );

        // Static properties
        if (elem.min - tc.min).abs() > tol {
            failures.push(format!("{label}: min rust={} java={}", elem.min, tc.min));
        }
        if (elem.max - tc.max).abs() > tol {
            failures.push(format!("{label}: max rust={} java={}", elem.max, tc.max));
        }
        if (elem.init - tc.init).abs() > tol {
            failures.push(format!("{label}: init rust={} java={}", elem.init, tc.init));
        }
        if (elem.border - tc.border).abs() > tol {
            failures.push(format!(
                "{label}: border rust={} java={}",
                elem.border, tc.border
            ));
        }
        if (elem.death - tc.death).abs() > tol {
            failures.push(format!(
                "{label}: death rust={} java={}",
                elem.death, tc.death
            ));
        }

        // Base values (unmodified)
        for (j, (rv, jv)) in elem.value.iter().zip(tc.base_values.iter()).enumerate() {
            if (rv - jv).abs() > tol {
                failures.push(format!("{label}: base_values[{j}] rust={rv} java={jv}"));
            }
        }

        // Modified values: apply modifier to base values using a model
        let model = make_model_for_gauge(tc.total, tc.total_notes);
        for (j, jv) in tc.modified_values.iter().enumerate() {
            let base = elem.value[j];
            let modified = if let Some(ref modifier) = elem.modifier {
                modifier.modify(base, &model)
            } else {
                base
            };
            if (modified - jv).abs() > tol {
                failures.push(format!(
                    "{label}: modified_values[{j}] rust={modified} java={jv}"
                ));
            }
        }

        // Guts table
        if elem.guts.len() != tc.guts.len() {
            failures.push(format!(
                "{label}: guts_count rust={} java={}",
                elem.guts.len(),
                tc.guts.len()
            ));
        } else {
            for (j, (rg, jg)) in elem.guts.iter().zip(tc.guts.iter()).enumerate() {
                let r_threshold = rg.first().copied().unwrap_or(0.0);
                let r_multiplier = rg.get(1).copied().unwrap_or(0.0);
                if (r_threshold - jg.threshold).abs() > tol
                    || (r_multiplier - jg.multiplier).abs() > tol
                {
                    failures.push(format!(
                        "{label}: guts[{j}] rust=({},{}) java=({},{})",
                        r_threshold, r_multiplier, jg.threshold, jg.multiplier
                    ));
                }
            }
        }

        // Verify Gauge initial value matches
        let cleartype = gauge_index_to_clear_type(tc.gauge_type_index);
        let gauge = Gauge::new(&model, elem.clone(), cleartype);
        if (gauge.get_value() - tc.init).abs() > tol {
            failures.push(format!(
                "{label}: gauge_init rust={} java={}",
                gauge.get_value(),
                tc.init
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "Gauge property GM mismatch ({} failures out of {} cases):\n{}",
            failures.len(),
            fixture.test_cases.len(),
            failures
                .iter()
                .take(20)
                .map(|f| format!("  - {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!(
        "Gauge property GM: all {} cases passed",
        fixture.test_cases.len()
    );
}

// =========================================================================
// Gauge Sequence GM Test
// =========================================================================

const GAUGE_NAMES: &[&str] = &[
    "AssistEasy",
    "Easy",
    "Normal",
    "Hard",
    "ExHard",
    "Hazard",
    "Class",
    "ExClass",
    "ExHardClass",
];

#[test]
fn golden_master_gauge_sequences() {
    let fixture: GaugeSequenceFixture = load_json("gauge_sequences.json");
    let mut failures = Vec::new();
    let tol = 1e-3_f32; // cumulative floating-point tolerance

    for (i, tc) in fixture.test_cases.iter().enumerate() {
        let prop = mode_to_gauge_property(&tc.mode);
        let elements = prop.get_values();
        let model = make_model_for_gauge(tc.total, tc.total_notes);
        let label = format!(
            "[{i}] {}/{}/total={}/notes={}",
            tc.mode, tc.sequence_name, tc.total, tc.total_notes
        );

        // Create 9 gauges
        let mut gauges: Vec<Gauge> = elements
            .into_iter()
            .enumerate()
            .map(|(g, elem)| Gauge::new(&model, elem, gauge_index_to_clear_type(g)))
            .collect();

        // Run sequence
        for (step_idx, step) in tc.sequence.iter().enumerate() {
            let rate = step.rate_x100 as f32 / 100.0;
            for gauge in &mut gauges {
                gauge.update(step.judge as i32, rate);
            }

            // Compare all 9 gauge values after this step
            let expected = &tc.values_after_each_step[step_idx];
            for (g, (rust_val, java_val)) in gauges
                .iter()
                .map(|g| g.get_value())
                .zip(expected.iter())
                .enumerate()
            {
                if (rust_val - java_val).abs() > tol {
                    let gauge_name = GAUGE_NAMES.get(g).unwrap_or(&"?");
                    failures.push(format!(
                        "{label} step[{step_idx}] gauge[{gauge_name}]: rust={rust_val} java={java_val} (diff={})",
                        (rust_val - java_val).abs()
                    ));
                }
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Gauge sequence GM mismatch ({} failures out of {} cases):\n{}",
            failures.len(),
            fixture.test_cases.len(),
            failures
                .iter()
                .take(30)
                .map(|f| format!("  - {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!(
        "Gauge sequence GM: all {} cases passed",
        fixture.test_cases.len()
    );
}

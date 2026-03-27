// Golden master tests: compare Rust ScoreDataProperty against Java fixture export.

use std::path::Path;

use bms::model::mode::Mode;
use golden_master::score_data_property_fixtures::{
    ScoreDataPropertyFixture, ScoreDataPropertyTestCase,
};
use rubato_game::core::score_data::ScoreData;
use rubato_game::core::score_data_property::ScoreDataProperty;

fn fixtures_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .leak()
}

fn load_fixture() -> ScoreDataPropertyFixture {
    let path = fixtures_dir().join("score_data_property.json");
    assert!(
        path.exists(),
        "ScoreDataProperty fixture not found: {}. Run the Java exporter first.",
        path.display()
    );
    let content = std::fs::read_to_string(&path).expect("Failed to read fixture");
    serde_json::from_str(&content).expect("Failed to parse fixture")
}

/// Convert Java mode integer to Rust Mode enum.
/// Java mode IDs: 5=BEAT_5K, 7=BEAT_7K, 10=BEAT_10K, 14=BEAT_14K, 9=POPN_9K, 25=KB_24K, 50=KB_24K_DOUBLE
fn mode_from_id(id: i32) -> Mode {
    match id {
        5 => Mode::BEAT_5K,
        7 => Mode::BEAT_7K,
        10 => Mode::BEAT_10K,
        14 => Mode::BEAT_14K,
        9 => Mode::POPN_9K,
        25 => Mode::KEYBOARD_24K,
        50 => Mode::KEYBOARD_24K_DOUBLE,
        _ => Mode::BEAT_7K, // fallback
    }
}

fn compare_score_data_property(
    tc: &ScoreDataPropertyTestCase,
    prop: &ScoreDataProperty,
) -> Vec<String> {
    let mut diffs = Vec::new();

    if prop.now_score() != tc.nowpoint {
        diffs.push(format!(
            "nowpoint: rust={} java={}",
            prop.now_score(),
            tc.nowpoint
        ));
    }

    // Float comparisons with tolerance
    if (prop.rate() - tc.rate).abs() > 0.001 {
        diffs.push(format!("rate: rust={} java={}", prop.rate(), tc.rate));
    }

    if prop.rate_int() != tc.rate_int {
        diffs.push(format!(
            "rate_int: rust={} java={}",
            prop.rate_int(),
            tc.rate_int
        ));
    }

    if prop.rate_after_dot() != tc.rate_after_dot {
        diffs.push(format!(
            "rate_after_dot: rust={} java={}",
            prop.rate_after_dot(),
            tc.rate_after_dot
        ));
    }

    if (prop.now_rate() - tc.nowrate).abs() > 0.001 {
        diffs.push(format!(
            "nowrate: rust={} java={}",
            prop.now_rate(),
            tc.nowrate
        ));
    }

    if prop.now_rate_int() != tc.nowrate_int {
        diffs.push(format!(
            "nowrate_int: rust={} java={}",
            prop.now_rate_int(),
            tc.nowrate_int
        ));
    }

    if prop.now_rate_after_dot() != tc.nowrate_after_dot {
        diffs.push(format!(
            "nowrate_after_dot: rust={} java={}",
            prop.now_rate_after_dot(),
            tc.nowrate_after_dot
        ));
    }

    // Rank array comparison
    for i in 0..27 {
        let rust_rank = prop.qualify_rank(i);
        let java_rank = tc.rank.get(i).copied().unwrap_or(false);
        if rust_rank != java_rank {
            diffs.push(format!(
                "rank[{}]: rust={} java={}",
                i, rust_rank, java_rank
            ));
        }
    }

    if prop.next_rank() != tc.nextrank {
        diffs.push(format!(
            "nextrank: rust={} java={}",
            prop.next_rank(),
            tc.nextrank
        ));
    }

    if (prop.best_score_rate() - tc.bestscorerate).abs() > 0.001 {
        diffs.push(format!(
            "bestscorerate: rust={} java={}",
            prop.best_score_rate(),
            tc.bestscorerate
        ));
    }

    diffs
}

#[test]
fn score_data_property_all_cases() {
    let fixture = load_fixture();

    assert!(!fixture.test_cases.is_empty(), "Fixture has no test cases");

    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    for tc in &fixture.test_cases {
        let playmode = mode_from_id(tc.mode);
        let score = ScoreData {
            mode: tc.mode,
            playmode,
            judge_counts: rubato_types::score_data::JudgeCounts {
                epg: tc.epg,
                lpg: tc.lpg,
                egr: tc.egr,
                lgr: tc.lgr,
                egd: tc.egd,
                lgd: tc.lgd,
                ebd: tc.ebd,
                lbd: tc.lbd,
                epr: tc.epr,
                lpr: tc.lpr,
                ems: tc.ems,
                lms: tc.lms,
            },
            maxcombo: tc.maxcombo,
            notes: tc.totalnotes,
            ..Default::default()
        };

        let mut prop = ScoreDataProperty::new();
        prop.update_score_with_notes(Some(&score), tc.notes);

        let diffs = compare_score_data_property(tc, &prop);
        if diffs.is_empty() {
            passed += 1;
        } else {
            failed += 1;
            failures.push(format!(
                "mode={} pattern={}: {}",
                tc.mode,
                tc.pattern_name,
                diffs.join(", ")
            ));
        }
    }

    if failed > 0 {
        panic!(
            "ScoreDataProperty GM test: {passed} passed, {failed} failed:\n{}",
            failures
                .iter()
                .map(|f| format!("  - {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!(
        "ScoreDataProperty GM test: all {} test cases passed",
        passed
    );
}

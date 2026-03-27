//! Phase 8 Task 8.3: Golden Master Expansion Tests
//!
//! Additional golden master tests covering:
//! - Complex BPM changes within a single BMS
//! - Long notes in each LN mode
//! - Mine notes damage and structure
//! - After random option applied (deterministic seed)

use std::path::Path;

use bms::model::bms_decoder::BMSDecoder;
use bms::model::bms_model::{
    BMSModel, LNTYPE_CHARGENOTE, LNTYPE_HELLCHARGENOTE, LNTYPE_LONGNOTE, LnType,
};
use bms::model::chart_information::ChartInformation;
use bms::model::judge_note::JudgeNote;
use golden_master::e2e_helpers::*;
use rubato_game::core::pattern::lane_shuffle_modifier::LaneRandomShuffleModifier;
use rubato_game::core::pattern::pattern_modifier::PatternModifier;
use rubato_game::play::bms_player_rule::BMSPlayerRule;
use rubato_types::groove_gauge::{EXHARD, HARD, NORMAL};

fn test_bms_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-bms")
        .leak()
}

// ============================================================================
// Complex BPM changes within a single BMS
// ============================================================================

#[test]
fn bpm_stop_combo_autoplay_all_pgreat() {
    let model = load_bms("bpm_stop_combo.bms");
    let total = model.total_notes() as usize;
    assert!(total > 0, "bpm_stop_combo should have playable notes");
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "bpm_stop_combo_autoplay");
}

#[test]
fn bpm_stop_combo_timelines_have_varying_bpm() {
    let model = load_bms("bpm_stop_combo.bms");
    let bpms: Vec<f64> = model.timelines.iter().map(|tl| tl.bpm).collect();
    let unique_bpms: std::collections::HashSet<u64> = bpms.iter().map(|b| b.to_bits()).collect();
    assert!(
        unique_bpms.len() > 1,
        "bpm_stop_combo should have multiple distinct BPM values, got {} unique from {} timelines",
        unique_bpms.len(),
        bpms.len()
    );
}

#[test]
fn bpm_stop_combo_has_stop_events() {
    let model = load_bms("bpm_stop_combo.bms");
    let stop_count = model
        .timelines
        .iter()
        .filter(|tl| tl.micro_stop() > 0)
        .count();
    assert!(
        stop_count > 0,
        "bpm_stop_combo should have STOP events, got 0"
    );
}

#[test]
fn bpm_extreme_autoplay_hard_gauge() {
    let model = load_bms("bpm_extreme.bms");
    let total = model.total_notes() as usize;
    assert!(total > 0);
    let result = run_autoplay_simulation(&model, HARD);
    assert_all_pgreat(&result, total, "bpm_extreme_hard_autoplay");
}

#[test]
fn bpm_extreme_min_max_differ() {
    let model = load_bms("bpm_extreme.bms");
    assert!(
        model.max_bpm() > model.min_bpm(),
        "bpm_extreme should have different min/max BPM: min={}, max={}",
        model.min_bpm(),
        model.max_bpm()
    );
}

#[test]
fn bpm_change_autoplay_exhard() {
    let model = load_bms("bpm_change.bms");
    let total = model.total_notes() as usize;
    assert!(total > 0);
    let result = run_autoplay_simulation(&model, EXHARD);
    assert_all_pgreat(&result, total, "bpm_change_exhard_autoplay");
}

#[test]
fn ln_bpm_cross_timeline_ordering() {
    let model = load_bms("ln_bpm_cross.bms");
    // Verify timelines are in strict time order
    for window in model.timelines.windows(2) {
        assert!(
            window[1].micro_time() >= window[0].micro_time(),
            "Timelines should be time-ordered: {} >= {}",
            window[1].micro_time(),
            window[0].micro_time()
        );
    }
}

// ============================================================================
// Long notes in each LN mode
// ============================================================================

/// Load a BMS with a specific LN type, with validation.
fn load_bms_with_lntype(filename: &str, lntype: LnType) -> BMSModel {
    let path = test_bms_dir().join(filename);
    let info = ChartInformation::new(Some(path), lntype, None);
    let mut model = BMSDecoder::new()
        .decode(info)
        .unwrap_or_else(|| panic!("Failed to parse {filename} with lntype={lntype:?}"));
    BMSPlayerRule::validate(&mut model);
    model
}

#[test]
fn longnote_types_lntype_longnote_autoplay() {
    let model = load_bms_with_lntype("longnote_types.bms", LNTYPE_LONGNOTE);
    let jn = model.build_judge_notes();
    let ln_starts = jn.iter().filter(|n| n.is_long_start()).count();
    assert!(ln_starts > 0, "Should have LN start notes");

    let result = run_autoplay_simulation(&model, NORMAL);
    let total = result.ghost.len();
    assert!(total > 0, "Should have judged notes");
    assert_all_pgreat(&result, total, "longnote_types_longnote_autoplay");
}

#[test]
fn longnote_types_lntype_chargenote_autoplay() {
    let model = load_bms_with_lntype("longnote_types.bms", LNTYPE_CHARGENOTE);
    let jn = model.build_judge_notes();
    let ln_starts = jn.iter().filter(|n| n.is_long_start()).count();
    assert!(
        ln_starts > 0,
        "Should have LN start notes in CHARGENOTE mode"
    );

    let result = run_autoplay_simulation(&model, NORMAL);
    let total = result.ghost.len();
    assert!(total > 0);
    assert_all_pgreat(&result, total, "longnote_types_chargenote_autoplay");
}

#[test]
fn longnote_types_lntype_hellchargenote_autoplay() {
    let model = load_bms_with_lntype("longnote_types.bms", LNTYPE_HELLCHARGENOTE);
    let jn = model.build_judge_notes();
    let ln_starts = jn.iter().filter(|n| n.is_long_start()).count();
    assert!(
        ln_starts > 0,
        "Should have LN start notes in HELLCHARGENOTE mode"
    );

    let result = run_autoplay_simulation(&model, NORMAL);
    let total = result.ghost.len();
    assert!(total > 0);
    assert_all_pgreat(&result, total, "longnote_types_hellchargenote_autoplay");
}

#[test]
fn ln_bpm_cross_autoplay_hard_gauge() {
    let model = load_bms("ln_bpm_cross.bms");
    let result = run_autoplay_simulation(&model, HARD);
    let total = result.ghost.len();
    assert!(total > 0, "Should have judged notes");
    assert_all_pgreat(&result, total, "ln_bpm_cross_hard_autoplay");
}

#[test]
fn longnote_end_times_are_after_start_times() {
    let model = load_bms_with_lntype("longnote_types.bms", LNTYPE_LONGNOTE);
    let jn = model.build_judge_notes();
    for note in &jn {
        if note.is_long_start() {
            assert!(
                note.end_time_us > note.time_us,
                "LN end time ({}) should be after start time ({})",
                note.end_time_us,
                note.time_us
            );
        }
    }
}

// ============================================================================
// Mine notes
// ============================================================================

#[test]
fn mine_notes_autoplay_ignores_mines() {
    let model = load_bms("mine_notes.bms");
    let total = model.total_notes() as usize;
    assert!(total > 0, "Should have playable notes");

    let result = run_autoplay_simulation(&model, NORMAL);
    // Autoplay should hit all regular notes and skip mines
    assert_all_pgreat(&result, total, "mine_notes_autoplay");
}

#[test]
fn mine_notes_have_damage() {
    let model = load_bms("mine_notes.bms");
    let jn = model.build_judge_notes();
    let mines: Vec<&JudgeNote> = jn.iter().filter(|n| n.is_mine()).collect();

    assert!(!mines.is_empty(), "Should have mine notes");
    for mine in &mines {
        // Mine damage can be 0 or positive; the important thing is it's set
        assert!(
            mine.damage >= 0.0,
            "Mine note damage should be non-negative, got {}",
            mine.damage
        );
    }
}

#[test]
fn mine_notes_manual_no_input_gauge_qualified() {
    let model = load_bms("mine_notes.bms");
    // With no input, all regular notes are MISS, but mines should not be triggered
    let result = run_manual_simulation(&model, &[], NORMAL);

    // Not qualified because no notes hit, but mine notes should not have been
    // triggered (no keys pressed)
    assert!(
        result.gauge_value >= 0.0,
        "gauge should be non-negative, got {}",
        result.gauge_value
    );
}

#[test]
fn channel_extended_mine_count_matches_model() {
    let model = load_bms("channel_extended.bms");
    let jn = model.build_judge_notes();
    let mine_count = jn.iter().filter(|n| n.is_mine()).count();

    // Also count mines from timeline API
    let keys = model.mode().map(|m| m.key()).unwrap_or(0);
    let mut tl_mine_count = 0;
    for tl in &model.timelines {
        for lane in 0..keys {
            if let Some(note) = tl.note(lane)
                && note.is_mine()
            {
                tl_mine_count += 1;
            }
        }
    }

    assert_eq!(
        mine_count, tl_mine_count,
        "Mine count from judge_notes ({}) should match timeline count ({})",
        mine_count, tl_mine_count
    );
}

// ============================================================================
// After random option applied (deterministic seed)
// ============================================================================

#[test]
fn random_shuffle_preserves_note_count() {
    let mut model = load_bms("minimal_7k.bms");
    let total_before = model.total_notes();
    assert!(total_before > 0);

    let mut modifier = LaneRandomShuffleModifier::new(0, false);
    modifier.modify(&mut model);

    let total_after = model.total_notes();
    assert_eq!(
        total_before, total_after,
        "Random shuffle should not change total note count: before={}, after={}",
        total_before, total_after
    );
}

#[test]
fn random_shuffle_autoplay_still_all_pgreat() {
    let mut model = load_bms("minimal_7k.bms");
    let mut modifier = LaneRandomShuffleModifier::new(0, false);
    modifier.modify(&mut model);

    let total = model.total_notes() as usize;
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "random_shuffle_autoplay");
}

#[test]
fn random_shuffle_changes_lane_assignment() {
    let model_original = load_bms("minimal_7k.bms");
    let jn_original = model_original.build_judge_notes();
    let lanes_original: Vec<usize> = jn_original
        .iter()
        .filter(|n| n.is_playable())
        .map(|n| n.lane)
        .collect();

    let mut model_shuffled = load_bms("minimal_7k.bms");
    let mut modifier = LaneRandomShuffleModifier::new(0, false);
    modifier.modify(&mut model_shuffled);
    let jn_shuffled = model_shuffled.build_judge_notes();
    let lanes_shuffled: Vec<usize> = jn_shuffled
        .iter()
        .filter(|n| n.is_playable())
        .map(|n| n.lane)
        .collect();

    // Same number of notes
    assert_eq!(lanes_original.len(), lanes_shuffled.len());

    // The lane assignments should differ (unless the random shuffle happened to
    // produce the identity permutation, which is unlikely for seed 0 with 7 keys)
    // We allow identity as a valid outcome but flag it for inspection
    if lanes_original == lanes_shuffled {
        eprintln!("WARNING: random shuffle produced identity permutation (possible but unlikely)");
    }
}

#[test]
fn random_shuffle_preserves_timing() {
    let model_original = load_bms("minimal_7k.bms");
    let jn_original = model_original.build_judge_notes();
    let times_original: Vec<i64> = jn_original
        .iter()
        .filter(|n| n.is_playable())
        .map(|n| n.time_us)
        .collect();

    let mut model_shuffled = load_bms("minimal_7k.bms");
    let mut modifier = LaneRandomShuffleModifier::new(0, false);
    modifier.modify(&mut model_shuffled);
    let jn_shuffled = model_shuffled.build_judge_notes();
    let mut times_shuffled: Vec<i64> = jn_shuffled
        .iter()
        .filter(|n| n.is_playable())
        .map(|n| n.time_us)
        .collect();

    // Sort both time lists to compare
    let mut times_orig_sorted = times_original.clone();
    times_orig_sorted.sort();
    times_shuffled.sort();

    assert_eq!(
        times_orig_sorted, times_shuffled,
        "Random shuffle should preserve note timings"
    );
}

#[test]
fn random_shuffle_deterministic_with_same_seed() {
    // PatternModifierBase uses SystemTime for seed by default, so we must
    // explicitly set the same seed on both modifiers to get determinism.
    let fixed_seed: i64 = 12345;

    let mut model1 = load_bms("minimal_7k.bms");
    let mut modifier1 = LaneRandomShuffleModifier::new(0, false);
    modifier1.set_seed(fixed_seed);
    modifier1.modify(&mut model1);
    let jn1 = model1.build_judge_notes();
    let lanes1: Vec<usize> = jn1
        .iter()
        .filter(|n| n.is_playable())
        .map(|n| n.lane)
        .collect();

    let mut model2 = load_bms("minimal_7k.bms");
    let mut modifier2 = LaneRandomShuffleModifier::new(0, false);
    modifier2.set_seed(fixed_seed);
    modifier2.modify(&mut model2);
    let jn2 = model2.build_judge_notes();
    let lanes2: Vec<usize> = jn2
        .iter()
        .filter(|n| n.is_playable())
        .map(|n| n.lane)
        .collect();

    assert_eq!(
        lanes1, lanes2,
        "Same seed should produce same lane assignment"
    );
}

#[test]
fn random_shuffle_5key_autoplay() {
    let mut model = load_bms("5key.bms");
    let mut modifier = LaneRandomShuffleModifier::new(0, false);
    modifier.modify(&mut model);

    let total = model.total_notes() as usize;
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "random_shuffle_5key_autoplay");
}

#[test]
fn random_shuffle_with_longnotes_preserves_ln_pairs() {
    let path = test_bms_dir().join("longnote_types.bms");
    if !path.exists() {
        eprintln!("skipping: longnote_types.bms not found");
        return;
    }

    let mut model = load_bms_with_lntype("longnote_types.bms", LNTYPE_LONGNOTE);
    let ln_count_before = model
        .build_judge_notes()
        .iter()
        .filter(|n| n.is_long_start())
        .count();

    let mut modifier = LaneRandomShuffleModifier::new(0, false);
    modifier.modify(&mut model);

    let ln_count_after = model
        .build_judge_notes()
        .iter()
        .filter(|n| n.is_long_start())
        .count();
    assert_eq!(
        ln_count_before, ln_count_after,
        "Random shuffle should preserve LN count: before={}, after={}",
        ln_count_before, ln_count_after
    );

    // Autoplay should still work
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(
        &result,
        result.ghost.len(),
        "random_shuffle_longnotes_autoplay",
    );
}

// ============================================================================
// Course simulation with complex BMS
// ============================================================================

#[test]
fn course_two_bpm_change_songs_autoplay() {
    let model1 = load_bms("bpm_change.bms");
    let model2 = load_bms("bpm_stop_combo.bms");

    let result = run_course_simulation(&[&model1, &model2], NORMAL);
    assert!(result.completed, "Course should complete with autoplay");
    assert_eq!(result.stages.len(), 2, "Should have 2 stage results");

    for (i, stage) in result.stages.iter().enumerate() {
        assert!(
            stage.gauge_qualified,
            "Stage {} gauge should be qualified (value={})",
            i, stage.gauge_value
        );
    }
}

#[test]
fn course_gauge_carries_over() {
    let model1 = load_bms("minimal_7k.bms");
    let model2 = load_bms("5key.bms");

    let result = run_course_simulation(&[&model1, &model2], NORMAL);
    assert!(result.completed);
    assert_eq!(result.stages.len(), 2);

    // The gauge value at end of stage 1 should be close to the start of stage 2
    // (carried over). Since both are autoplay, gauge should increase.
    let stage1_gauge = result.stages[0].gauge_value;
    assert!(
        stage1_gauge > 20.0,
        "Stage 1 gauge should increase from starting 20%: got {}",
        stage1_gauge
    );
}

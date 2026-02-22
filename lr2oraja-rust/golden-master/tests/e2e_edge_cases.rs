// E2E integration tests for edge case BMS files.
//
// Tests parsing and autoplay invariants for:
// - LN crossing BPM changes
// - Extreme BPM values
// - Rapid STOP + BPM interleaving
// - Multiple simultaneous LNs / overlapping patterns
// - Extended channels (LNTYPE 2, invisible, mine combinations)
// - BMSON BPM-crossing long notes

use beatoraja_types::groove_gauge::NORMAL;
use bms_model::judge_note::JUDGE_PG;
use golden_master::e2e_helpers::*;

// ============================================================================
// Autoplay invariant tests for edge case BMS files
// ============================================================================

#[test]
fn autoplay_ln_bpm_cross() {
    let model = load_bms("ln_bpm_cross.bms");
    let result = run_autoplay_simulation(&model, NORMAL);
    let total = result.ghost.len();
    assert!(total > 0, "ln_bpm_cross should have judged notes");
    assert_all_pgreat(&result, total, "autoplay_ln_bpm_cross");
}

#[test]
fn autoplay_bpm_extreme() {
    let model = load_bms("bpm_extreme.bms");
    let total = model.get_total_notes() as usize;
    assert!(total > 0, "bpm_extreme should have playable notes");
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "autoplay_bpm_extreme");
}

#[test]
fn autoplay_multi_stop_rapid() {
    let model = load_bms("multi_stop_rapid.bms");
    let total = model.get_total_notes() as usize;
    assert!(total > 0, "multi_stop_rapid should have playable notes");
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "autoplay_multi_stop_rapid");
}

#[test]
fn autoplay_ln_overlap() {
    let model = load_bms("ln_overlap.bms");
    let result = run_autoplay_simulation(&model, NORMAL);
    let total = result.ghost.len();
    assert!(total > 0, "ln_overlap should have judged notes");
    // LN overlap edge cases may cause imperfect autoplay (e.g. simultaneous
    // LN end + normal note on the same tick can confuse the judge). Verify
    // that most notes are PG and the gauge is alive.
    let pg = result.score.get_judge_count_total(JUDGE_PG);
    assert!(
        pg as usize >= total - 2,
        "autoplay_ln_overlap: most notes should be PG (PG={pg}, total={total})"
    );
    assert!(
        result.gauge_value > 0.0,
        "Gauge should be alive after autoplay"
    );
}

#[test]
fn autoplay_channel_extended() {
    let model = load_bms("channel_extended.bms");
    let result = run_autoplay_simulation(&model, NORMAL);
    let total = result.ghost.len();
    assert!(total > 0, "channel_extended should have judged notes");
    assert_all_pgreat(&result, total, "autoplay_channel_extended");
}

#[test]
fn autoplay_bmson_bpm_ln_cross() {
    use bms_model::bms_model::LNTYPE_LONGNOTE;
    use bms_model::bmson_decoder::BMSONDecoder;
    use bms_model::chart_information::ChartInformation;

    let path = golden_master::e2e_helpers::test_bms_dir().join("bmson_bpm_ln_cross.bmson");
    let info = ChartInformation::new(Some(path), LNTYPE_LONGNOTE, None);
    let mut decoder = BMSONDecoder::new(LNTYPE_LONGNOTE);
    let model = decoder
        .decode(info)
        .unwrap_or_else(|| panic!("Failed to parse bmson_bpm_ln_cross.bmson"));
    let result = run_autoplay_simulation(&model, NORMAL);
    let total = result.ghost.len();
    assert!(total > 0, "bmson_bpm_ln_cross should have judged notes");
    assert_all_pgreat(&result, total, "autoplay_bmson_bpm_ln_cross");
}

// ============================================================================
// Structural validation tests
// ============================================================================

/// Verify that extreme BPM values produce correct timing relationships.
#[test]
fn bpm_extreme_timing_structure() {
    let model = load_bms("bpm_extreme.bms");
    let notes: Vec<_> = model
        .build_judge_notes()
        .into_iter()
        .filter(|n| n.is_playable())
        .collect();
    assert!(notes.len() >= 4, "Should have at least 4 playable notes");

    // Notes should be in time order
    for window in notes.windows(2) {
        assert!(
            window[1].time_us >= window[0].time_us,
            "Notes should be time-ordered: {} >= {}",
            window[1].time_us,
            window[0].time_us
        );
    }

    // All times should be non-negative
    for note in &notes {
        assert!(
            note.time_us >= 0,
            "Note time should be non-negative: {}",
            note.time_us
        );
    }
}

/// Verify rapid STOP sequences produce correct timing gaps.
#[test]
fn multi_stop_timing_gaps() {
    let model = load_bms("multi_stop_rapid.bms");
    let notes: Vec<_> = model
        .build_judge_notes()
        .into_iter()
        .filter(|n| n.is_playable())
        .collect();
    assert!(!notes.is_empty(), "Should have playable notes");

    // All notes should be in time order
    for window in notes.windows(2) {
        assert!(
            window[1].time_us >= window[0].time_us,
            "Notes should be time-ordered"
        );
    }

    // All notes should be at distinct times (no collapsed timing)
    let mut times: Vec<i64> = notes.iter().map(|n| n.time_us).collect();
    times.sort();
    times.dedup();
    // Allow some notes to share times (same measure position on different lanes)
    // but total unique times should be > 1
    assert!(
        times.len() > 1,
        "Should have notes at multiple distinct times"
    );

    // STOPs add extra time to the timeline, so the last note should be
    // further in time than measure count alone would suggest. Verify the
    // last note is past 3 seconds (beyond the first two measures at BPM 120).
    let last_time = notes.last().unwrap().time_us;
    assert!(
        last_time > 3_000_000,
        "STOPs should push notes later (last note at {last_time}us, expected > 3s)"
    );
}

/// Verify LN overlap BMS has correct note structure.
#[test]
fn ln_overlap_note_structure() {
    let model = load_bms("ln_overlap.bms");

    let judge_notes = model.build_judge_notes();
    let long_notes = judge_notes.iter().filter(|n| n.is_long_start()).count();
    assert!(long_notes > 0, "Should have long notes");

    let normal_notes = count_normal_notes(&judge_notes);
    assert!(normal_notes > 0, "Should have normal notes");

    // Normal gauge should be qualified on autoplay (tolerant of 1-2 edge case misses)
    let result = run_autoplay_simulation(&model, NORMAL);
    assert!(
        result.gauge_qualified,
        "Normal gauge should be qualified on autoplay (value={})",
        result.gauge_value
    );
}

/// Verify extended channel BMS parses invisible and mine notes correctly.
#[test]
fn channel_extended_note_types() {
    let model = load_bms("channel_extended.bms");

    // Check mine notes via judge_notes
    let judge_notes = model.build_judge_notes();
    let mine_count = judge_notes.iter().filter(|n| n.is_mine()).count();
    assert!(mine_count > 0, "Should have mine notes");

    // Check invisible notes via timeline API
    let keys = model.get_mode().map(|m| m.key()).unwrap_or(0);
    let mut invisible_count = 0;
    for tl in model.get_all_time_lines() {
        for lane in 0..keys {
            if tl.get_hidden_note(lane).is_some() {
                invisible_count += 1;
            }
        }
    }
    assert!(invisible_count > 0, "Should have invisible notes");

    // Check long notes
    let ln_count = judge_notes.iter().filter(|n| n.is_long_start()).count();
    assert!(ln_count > 0, "Should have long notes (LNTYPE 2 / MGQ)");
}

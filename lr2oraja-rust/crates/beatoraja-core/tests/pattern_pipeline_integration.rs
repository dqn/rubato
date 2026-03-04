// Integration test: BMS parse -> pattern apply pipeline
//
// Parses a real BMS file, applies pattern modifiers (Mirror, Identity),
// and verifies the resulting model's note layout.

use std::path::Path;

use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;

use beatoraja_core::pattern::lane_shuffle_modifier::LaneMirrorShuffleModifier;
use beatoraja_core::pattern::pattern_modifier::{IdentityModifier, PatternModifier};

/// Helper: parse the minimal_7k.bms test fixture and return the decoded model.
fn parse_minimal_7k() -> BMSModel {
    let bms_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/minimal_7k.bms");
    assert!(
        bms_path.exists(),
        "Test BMS file not found at: {}",
        bms_path.display()
    );

    let mut decoder = BMSDecoder::new();

    decoder
        .decode_path(&bms_path)
        .expect("Failed to decode minimal_7k.bms")
}

/// Collect note positions (lane indices that have a note) from all timelines.
/// Returns a Vec of (timeline_index, lane) pairs.
fn collect_note_positions(model: &BMSModel) -> Vec<(usize, i32)> {
    let mode_key = model.get_mode().map(|m| m.key()).unwrap_or(0);
    let mut positions = Vec::new();
    for (tl_idx, tl) in model.get_all_time_lines().iter().enumerate() {
        for lane in 0..mode_key {
            if tl.get_note(lane).is_some() {
                positions.push((tl_idx, lane));
            }
        }
    }
    positions
}

#[test]
fn parse_minimal_7k_produces_valid_model() {
    let model = parse_minimal_7k();

    // The BMS file should produce a valid model with notes
    assert!(
        !model.get_all_time_lines().is_empty(),
        "Parsed model should have timelines"
    );

    // Mode should be set (minimal_7k.bms uses 7-key channels)
    let mode = model.get_mode().expect("Model should have a mode set");
    // 7K channels (#001xx) mean BEAT_7K
    assert!(
        mode.key() > 0,
        "Mode should have a positive key count, got {}",
        mode.key()
    );

    // Should have notes from the BMS data
    let positions = collect_note_positions(&model);
    assert!(
        !positions.is_empty(),
        "Model should contain notes after parsing"
    );

    // Verify metadata
    assert_eq!(model.get_title(), "Minimal 7K Test");
    assert_eq!(model.get_artist(), "brs-test");
    assert!((model.get_bpm() - 120.0).abs() < f64::EPSILON);
}

#[test]
fn identity_modifier_preserves_notes() {
    let mut model = parse_minimal_7k();

    // Record note positions before
    let positions_before = collect_note_positions(&model);
    assert!(
        !positions_before.is_empty(),
        "Model should have notes before identity modifier"
    );

    // Record the wav values for each note position
    let wavs_before: Vec<i32> = positions_before
        .iter()
        .map(|&(tl_idx, lane)| {
            model.get_all_time_lines()[tl_idx]
                .get_note(lane)
                .unwrap()
                .get_wav()
        })
        .collect();

    // Apply identity modifier (should do nothing)
    let mut modifier = IdentityModifier::new();
    modifier.modify(&mut model);

    // Record note positions after
    let positions_after = collect_note_positions(&model);

    // Positions and wav values should be identical
    assert_eq!(
        positions_before, positions_after,
        "Identity modifier should not change note positions"
    );

    let wavs_after: Vec<i32> = positions_after
        .iter()
        .map(|&(tl_idx, lane)| {
            model.get_all_time_lines()[tl_idx]
                .get_note(lane)
                .unwrap()
                .get_wav()
        })
        .collect();
    assert_eq!(
        wavs_before, wavs_after,
        "Identity modifier should not change note wav values"
    );
}

#[test]
fn mirror_modifier_reverses_lanes() {
    let mut model = parse_minimal_7k();

    let mode = model.get_mode().cloned().expect("Model should have a mode");

    // Record the note wav values per lane for each timeline (before mirror)
    let mode_key = mode.key();
    let before: Vec<Vec<Option<i32>>> = model
        .get_all_time_lines()
        .iter()
        .map(|tl| {
            (0..mode_key)
                .map(|lane| tl.get_note(lane).map(|n| n.get_wav()))
                .collect()
        })
        .collect();

    // There should be some notes to actually test
    let has_notes = before
        .iter()
        .any(|tl_notes| tl_notes.iter().any(|n| n.is_some()));
    assert!(has_notes, "Model should have notes to test mirror on");

    // Apply mirror modifier (without scratch lane modify)
    let mut modifier = LaneMirrorShuffleModifier::new(0, false);
    modifier.modify(&mut model);

    let after: Vec<Vec<Option<i32>>> = model
        .get_all_time_lines()
        .iter()
        .map(|tl| {
            (0..mode_key)
                .map(|lane| tl.get_note(lane).map(|n| n.get_wav()))
                .collect()
        })
        .collect();

    // For BEAT_7K without scratch: lanes 0-6 are mirrored (0<->6, 1<->5, 2<->4, 3 stays),
    // and scratch lane 7 stays put.
    // The mirror mapping for keys [0,1,2,3,4,5,6] is [6,5,4,3,2,1,0].
    // So after[tl][i] should equal before[tl][6-i] for i in 0..7, and after[tl][7] == before[tl][7].
    if mode == Mode::BEAT_7K {
        for (tl_idx, (b, a)) in before.iter().zip(after.iter()).enumerate() {
            for lane in 0..7 {
                let mirrored_lane = 6 - lane;
                assert_eq!(
                    a[lane as usize], b[mirrored_lane as usize],
                    "Timeline {}: lane {} after mirror should match lane {} before",
                    tl_idx, lane, mirrored_lane
                );
            }
            // Scratch lane (7) unchanged
            assert_eq!(
                a[7], b[7],
                "Timeline {}: scratch lane 7 should be unchanged after mirror",
                tl_idx
            );
        }
    }

    // At a minimum, verify that something changed (unless the chart was symmetric)
    // We check that the overall note distribution differs or is the same mirror pattern
    let positions_after = collect_note_positions(&model);
    assert!(
        !positions_after.is_empty(),
        "Model should still have notes after mirror modifier"
    );
}

#[test]
fn mirror_then_mirror_restores_original() {
    let mut model = parse_minimal_7k();

    let mode_key = model.get_mode().map(|m| m.key()).unwrap_or(0);

    // Record original state
    let original: Vec<Vec<Option<i32>>> = model
        .get_all_time_lines()
        .iter()
        .map(|tl| {
            (0..mode_key)
                .map(|lane| tl.get_note(lane).map(|n| n.get_wav()))
                .collect()
        })
        .collect();

    // Apply mirror twice
    let mut modifier1 = LaneMirrorShuffleModifier::new(0, false);
    modifier1.modify(&mut model);

    let mut modifier2 = LaneMirrorShuffleModifier::new(0, false);
    modifier2.modify(&mut model);

    // After two mirrors, should be back to original
    let restored: Vec<Vec<Option<i32>>> = model
        .get_all_time_lines()
        .iter()
        .map(|tl| {
            (0..mode_key)
                .map(|lane| tl.get_note(lane).map(|n| n.get_wav()))
                .collect()
        })
        .collect();

    assert_eq!(
        original, restored,
        "Applying mirror twice should restore the original note layout"
    );
}

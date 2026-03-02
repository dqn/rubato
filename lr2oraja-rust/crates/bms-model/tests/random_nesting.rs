//! Tests for `#RANDOM`/`#IF`/`#ENDIF`/`#ENDRANDOM` nesting logic in BMSDecoder.
//!
//! These tests verify correct branch selection with `selected_random`,
//! deeply nested random blocks, and graceful handling of mismatched
//! directives (extra `#ENDIF` / `#ENDRANDOM` without matching openers).

use bms_model::bms_decoder::BMSDecoder;

/// Helper: collect all WAV indices found across all timelines and lanes.
fn collect_note_wavs(model: &bms_model::bms_model::BMSModel) -> Vec<i32> {
    let mut wavs = Vec::new();
    for tl in model.get_all_time_lines() {
        for lane in 0..tl.get_lane_count() {
            if let Some(note) = tl.get_note(lane) {
                wavs.push(note.get_wav());
            }
        }
    }
    wavs
}

// ---------------------------------------------------------------------------
// Basic #RANDOM/#IF value matching
// ---------------------------------------------------------------------------

/// `#IF 1` branch is included when `selected_random = [1]`.
#[test]
fn test_random_if_value_match() {
    let data = b"\
#BPM 120
#WAV01 a.wav
#RANDOM 2
#IF 1
#00112:01
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    let model = decoder.decode_bytes(data, false, Some(&[1]));
    let model = model.expect("decode should succeed");
    assert_eq!(
        model.get_total_notes(),
        1,
        "note inside matching #IF 1 branch should be included"
    );
}

/// `#IF 1` branch is skipped when `selected_random = [2]`.
#[test]
fn test_random_if_value_mismatch() {
    let data = b"\
#BPM 120
#WAV01 a.wav
#RANDOM 2
#IF 1
#00112:01
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    let model = decoder.decode_bytes(data, false, Some(&[2]));
    let model = model.expect("decode should succeed");
    assert_eq!(
        model.get_total_notes(),
        0,
        "note inside non-matching #IF 1 branch should be excluded when random=2"
    );
}

// ---------------------------------------------------------------------------
// 2-level nesting
// ---------------------------------------------------------------------------

/// Two-level nesting: outer `#RANDOM 3 / #IF 1` and inner `#RANDOM 2 / #IF 1`.
/// With `selected_random = [1, 1]`, both branches match and both notes appear.
#[test]
fn test_nested_random_if() {
    let data = b"\
#BPM 120
#WAV01 outer.wav
#WAV02 inner.wav
#RANDOM 3
#IF 1
#00112:01
#RANDOM 2
#IF 1
#00114:02
#ENDIF
#ENDRANDOM
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    let model = decoder.decode_bytes(data, false, Some(&[1, 1]));
    let model = model.expect("decode should succeed with nested randoms");
    let wavs = collect_note_wavs(&model);
    assert!(
        wavs.len() >= 2,
        "expected at least 2 notes (outer + inner), got {}",
        wavs.len()
    );
}

/// Two-level nesting: outer matches (#IF 1), inner does NOT match (#IF 2 vs random=1).
/// Only the outer note should appear.
#[test]
fn test_nested_random_inner_mismatch() {
    let data = b"\
#BPM 120
#WAV01 outer.wav
#WAV02 inner.wav
#RANDOM 3
#IF 1
#00112:01
#RANDOM 2
#IF 2
#00114:02
#ENDIF
#ENDRANDOM
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    let model = decoder.decode_bytes(data, false, Some(&[1, 1]));
    let model = model.expect("decode should succeed");
    assert_eq!(
        model.get_total_notes(),
        1,
        "only outer note should appear when inner #IF does not match"
    );
}

/// Two-level nesting: outer does NOT match (#IF 1 vs random=2), but inner
/// DOES match (#IF 1 vs random=1).
///
/// The BMS decoder processes `#RANDOM`/`#IF`/`#ENDIF`/`#ENDRANDOM` directives
/// regardless of the current skip state, and the skip check (`skip.last()`)
/// only examines the top of the stack. This means when the inner `#IF`
/// matches, its `false` on the skip stack overrides the outer `true`,
/// causing lines inside the inner block to be included even though the
/// outer block was skipped. This matches the Java beatoraja behavior.
#[test]
fn test_nested_random_outer_mismatch() {
    let data = b"\
#BPM 120
#WAV01 outer.wav
#WAV02 inner.wav
#RANDOM 3
#IF 1
#00112:01
#RANDOM 2
#IF 1
#00114:02
#ENDIF
#ENDRANDOM
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    let model = decoder.decode_bytes(data, false, Some(&[2, 1]));
    let model = model.expect("decode should succeed");
    // The outer note (#00112:01) is skipped because outer #IF 1 != random 2.
    // However, the inner note (#00114:02) IS included because skip.last()
    // only checks the innermost #IF, which matches (inner #IF 1 == random 1).
    assert_eq!(
        model.get_total_notes(),
        1,
        "inner note should appear because skip stack only checks the top element"
    );
}

// ---------------------------------------------------------------------------
// 3-level deep nesting
// ---------------------------------------------------------------------------

/// Three levels of nesting, all branches match.
#[test]
fn test_deeply_nested_random() {
    let data = b"\
#BPM 120
#WAV01 l1.wav
#WAV02 l2.wav
#WAV03 l3.wav
#RANDOM 2
#IF 1
#00112:01
#RANDOM 3
#IF 2
#00114:02
#RANDOM 4
#IF 3
#00116:03
#ENDIF
#ENDRANDOM
#ENDIF
#ENDRANDOM
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    let model = decoder.decode_bytes(data, false, Some(&[1, 2, 3]));
    let model = model.expect("decode should succeed with 3-level nesting");
    let wavs = collect_note_wavs(&model);
    assert!(
        wavs.len() >= 3,
        "expected at least 3 notes from all 3 matching levels, got {}",
        wavs.len()
    );
}

/// Three levels: level 1 matches, level 2 matches, level 3 does NOT.
#[test]
fn test_deeply_nested_random_partial_match() {
    let data = b"\
#BPM 120
#WAV01 l1.wav
#WAV02 l2.wav
#WAV03 l3.wav
#RANDOM 2
#IF 1
#00112:01
#RANDOM 3
#IF 2
#00114:02
#RANDOM 4
#IF 3
#00116:03
#ENDIF
#ENDRANDOM
#ENDIF
#ENDRANDOM
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    // Level 3 random is 4 but #IF expects 3; selected_random[2]=4 means mismatch
    let model = decoder.decode_bytes(data, false, Some(&[1, 2, 4]));
    let model = model.expect("decode should succeed");
    assert_eq!(
        model.get_total_notes(),
        2,
        "only level 1 and level 2 notes should appear (level 3 mismatched)"
    );
}

// ---------------------------------------------------------------------------
// Graceful handling of mismatched directives
// ---------------------------------------------------------------------------

/// `#ENDIF` without a preceding `#IF` should not panic.
#[test]
fn test_extra_endif_graceful() {
    let data = b"\
#BPM 120
#WAV01 a.wav
#00112:01
#ENDIF
";
    let mut decoder = BMSDecoder::new();
    let model = decoder.decode_bytes(data, false, None);
    assert!(model.is_some(), "decoder should not panic on stray #ENDIF");
    assert!(
        decoder
            .log
            .iter()
            .any(|l| l.message.contains("ENDIF") || l.message.contains("IF")),
        "expected a warning about unmatched #ENDIF"
    );
    // The note before the stray #ENDIF should still be decoded
    let model = model.unwrap();
    assert_eq!(
        model.get_total_notes(),
        1,
        "note before stray #ENDIF should still be included"
    );
}

/// `#ENDRANDOM` without a preceding `#RANDOM` should not panic.
#[test]
fn test_extra_endrandom_graceful() {
    let data = b"\
#BPM 120
#WAV01 a.wav
#00112:01
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    let model = decoder.decode_bytes(data, false, None);
    assert!(
        model.is_some(),
        "decoder should not panic on stray #ENDRANDOM"
    );
    assert!(
        decoder
            .log
            .iter()
            .any(|l| l.message.contains("ENDRANDOM") || l.message.contains("RANDOM")),
        "expected a warning about unmatched #ENDRANDOM"
    );
    let model = model.unwrap();
    assert_eq!(
        model.get_total_notes(),
        1,
        "note before stray #ENDRANDOM should still be included"
    );
}

// ---------------------------------------------------------------------------
// Multiple sequential #RANDOM blocks
// ---------------------------------------------------------------------------

/// Two sequential (non-nested) #RANDOM blocks each select different branches.
#[test]
fn test_sequential_random_blocks() {
    let data = b"\
#BPM 120
#WAV01 first.wav
#WAV02 second.wav
#RANDOM 2
#IF 1
#00112:01
#ENDIF
#IF 2
#00112:02
#ENDIF
#ENDRANDOM
#RANDOM 3
#IF 3
#00214:01
#ENDIF
#IF 1
#00214:02
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    // First #RANDOM selects 1 (matches #IF 1), second #RANDOM selects 3 (matches #IF 3)
    let model = decoder.decode_bytes(data, false, Some(&[1, 3]));
    let model = model.expect("decode should succeed");
    assert_eq!(
        model.get_total_notes(),
        2,
        "one note from each sequential #RANDOM block"
    );
}

// ---------------------------------------------------------------------------
// selected_random shorter than number of #RANDOM blocks
// ---------------------------------------------------------------------------

/// When `selected_random` has fewer entries than `#RANDOM` blocks encountered,
/// the decoder falls back to generating a random value for the excess blocks.
/// We just verify no panic occurs.
#[test]
fn test_selected_random_shorter_than_blocks() {
    let data = b"\
#BPM 120
#WAV01 a.wav
#RANDOM 2
#IF 1
#00112:01
#ENDIF
#ENDRANDOM
#RANDOM 3
#IF 1
#00214:01
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    // Only one entry but two #RANDOM blocks -- second block gets a generated value
    let model = decoder.decode_bytes(data, false, Some(&[1]));
    assert!(
        model.is_some(),
        "decoder should not panic when selected_random is shorter than #RANDOM count"
    );
    let model = model.unwrap();
    // First block: selected_random[0]=1 matches #IF 1, so at least 1 note
    assert!(
        model.get_total_notes() >= 1,
        "at least the first #RANDOM block note should be present"
    );
}

// ---------------------------------------------------------------------------
// No selected_random (fully random mode) -- just verify no panic
// ---------------------------------------------------------------------------

/// Decoding with `None` for selected_random uses generated random values.
/// We just verify no crash and that a valid model is returned.
#[test]
fn test_random_none_no_panic() {
    let data = b"\
#BPM 120
#WAV01 a.wav
#RANDOM 5
#IF 1
#00112:01
#ENDIF
#IF 2
#00112:01
#ENDIF
#IF 3
#00112:01
#ENDIF
#IF 4
#00112:01
#ENDIF
#IF 5
#00112:01
#ENDIF
#ENDRANDOM
";
    let mut decoder = BMSDecoder::new();
    let model = decoder.decode_bytes(data, false, None);
    assert!(
        model.is_some(),
        "decoder should produce a model even without selected_random"
    );
    let model = model.unwrap();
    // Exactly one of the 5 branches should be selected
    assert_eq!(
        model.get_total_notes(),
        1,
        "exactly one #IF branch should be selected by random"
    );
}

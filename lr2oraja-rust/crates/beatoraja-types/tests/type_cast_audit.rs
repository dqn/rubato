// Type cast audit tests for beatoraja-types.
//
// These tests document narrowing / truncation bugs in the encode/decode
// paths of ReplayData and ScoreData.  They are RED-ONLY: the buggy tests
// are marked `#[ignore]` so the suite stays green, while still serving as
// living documentation of the vulnerabilities.

use beatoraja_types::replay_data::ReplayData;
use beatoraja_types::score_data::ScoreData;
use beatoraja_types::stubs::KeyInputLog;
use beatoraja_types::validatable::Validatable;

use bms_model::mode::Mode;

// ---------------------------------------------------------------------------
// Test 1: replay_shrink_keycode_roundtrip (1-2a)
//
// ReplayData::shrink() encodes keycode as:
//   ((keycode + 1) * sign) as i8 as u8
// ReplayData::validate() decodes as:
//   decompressed[pos] as i8  ->  unsigned_abs() - 1
//
// i8 range is -128..=127, so (keycode+1) > 127 overflows.
// ---------------------------------------------------------------------------

/// Helper: shrink a single KeyInputLog and unshrink via validate(), returning
/// the recovered log entry (or None if validate rejected it).
fn shrink_unshrink_one(keycode: i32, pressed: bool) -> Option<KeyInputLog> {
    let mut rd = ReplayData::new();
    rd.keylog = vec![KeyInputLog {
        time: 1000,
        keycode,
        pressed,
    }];
    rd.shrink();
    assert!(rd.keyinput.is_some(), "shrink must produce keyinput");

    if rd.validate() {
        assert_eq!(rd.keylog.len(), 1);
        Some(rd.keylog.remove(0))
    } else {
        None
    }
}

#[test]
fn replay_shrink_keycode_126_roundtrip() {
    // keycode=126: (126+1)*1 = 127 fits in i8 — boundary OK
    let pressed = shrink_unshrink_one(126, true).expect("should roundtrip");
    assert_eq!(pressed.keycode, 126);
    assert!(pressed.pressed);

    let released = shrink_unshrink_one(126, false).expect("should roundtrip");
    assert_eq!(released.keycode, 126);
    assert!(!released.pressed);
}

#[test]
#[ignore] // BUG: (127+1)*1 = 128 overflows i8 to -128; pressed flag is inverted
fn replay_shrink_keycode_127_overflow() {
    // keycode=127: (127+1)*1 = 128, which wraps to -128 as i8.
    // On decode, -128 is negative so pressed reads as false,
    // and unsigned_abs(-128)-1 = 127 — keycode survives but pressed is wrong.
    let recovered = shrink_unshrink_one(127, true).expect("should roundtrip");
    assert_eq!(
        recovered.keycode, 127,
        "keycode should survive (actual: {})",
        recovered.keycode
    );
    assert!(
        recovered.pressed,
        "pressed=true should survive, but i8 overflow inverts it"
    );
}

#[test]
#[ignore] // BUG: (200+1)*1 = 201 as i8 = -55; both keycode AND pressed are corrupted
fn replay_shrink_keycode_200_corrupted() {
    // keycode=200: (200+1)*1 = 201, 201u8 as i8 = -55.
    // On decode: pressed = (-55 >= 0) = false (wrong).
    // keycode = unsigned_abs(-55) - 1 = 54 (wrong, expected 200).
    let recovered = shrink_unshrink_one(200, true).expect("should roundtrip");
    assert_eq!(
        recovered.keycode, 200,
        "keycode should survive (actual: {})",
        recovered.keycode
    );
    assert!(
        recovered.pressed,
        "pressed=true should survive, but truncation corrupts it"
    );
}

// ---------------------------------------------------------------------------
// Test 2: ghost_encode_truncation (1-2b)
//
// ScoreData::encode_ghost() does: v.iter().map(|&j| j as u8).collect()
// ScoreData::decode_ghost() does: decompressed[i] as i32
//
// `as u8` silently truncates values outside 0..=255.
// ---------------------------------------------------------------------------

/// Helper: encode a ghost array and decode it back.
fn ghost_roundtrip(judges: &[i32], notes: i32) -> Vec<i32> {
    let mut sd = ScoreData::new(Mode::BEAT_7K);
    sd.notes = notes;
    sd.encode_ghost(Some(judges));
    assert!(!sd.ghost.is_empty(), "encode_ghost must produce data");
    sd.decode_ghost().expect("decode_ghost must succeed")
}

#[test]
fn ghost_encode_valid_judges_roundtrip() {
    // Judge values 0-5 are the valid range in beatoraja.
    let input = vec![0, 1, 2, 3, 4, 5];
    let decoded = ghost_roundtrip(&input, input.len() as i32);
    assert_eq!(decoded, input);
}

#[test]
#[ignore] // BUG: 256 as u8 = 0 — silent truncation corrupts the ghost data
fn ghost_encode_truncation_256() {
    // 256i32 as u8 = 0.  On decode, decompressed[0] as i32 = 0.
    // The value 256 is silently replaced by 0.
    let input = vec![256];
    let decoded = ghost_roundtrip(&input, 1);
    assert_eq!(
        decoded[0], 256,
        "value 256 should survive roundtrip (actual: {})",
        decoded[0]
    );
}

#[test]
#[ignore] // BUG: -1 as u8 = 255 — negative values wrap around
fn ghost_encode_negative_wrap() {
    // (-1i32) as u8 = 255.  On decode, 255u8 as i32 = 255.
    // The value -1 becomes 255.
    let input = vec![-1];
    let decoded = ghost_roundtrip(&input, 1);
    assert_eq!(
        decoded[0], -1,
        "value -1 should survive roundtrip (actual: {})",
        decoded[0]
    );
}

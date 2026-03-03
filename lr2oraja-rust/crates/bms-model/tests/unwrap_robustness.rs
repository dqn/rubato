//! Unwrap robustness tests for bms-model crate.
//!
//! These tests document panic-prone code paths where `.unwrap()` is used
//! on values that can be absent under certain inputs. The tests use
//! `#[should_panic]` to prove the panic exists without fixing it.

use bms_model::bms_decoder::BMSDecoder;

// ---------------------------------------------------------------------------
// BMSDecoder: #RANDOM with bare keyword (no value after space)
// ---------------------------------------------------------------------------

/// When `#RANDOM` appears as a bare keyword without a trailing number
/// (e.g., `#RANDOM\n` with length exactly 7), the parser now gracefully
/// skips the malformed line instead of panicking on out-of-bounds slice.
#[test]
fn bms_decoder_random_bare_keyword_handled_gracefully() {
    let mut decoder = BMSDecoder::new();
    // "#RANDOM" is exactly 7 bytes. Previously line[8..] panicked;
    // now line.get(8..) returns None and the line is skipped.
    let data = b"#BPM 120\n#RANDOM\n#001011:0101\n";
    let model = decoder.decode_bytes(data, false, None);
    assert!(model.is_some(), "should not panic on bare #RANDOM");
}

/// When `#RANDOM abc` (non-numeric value) is supplied, the parser
/// gracefully logs a warning instead of panicking, because
/// `line[8..].trim().parse::<i32>()` falls through to the Err branch.
/// This test confirms the graceful path works.
#[test]
fn bms_decoder_random_non_numeric_handled_gracefully() {
    let mut decoder = BMSDecoder::new();
    let data = b"#BPM 120\n#RANDOM abc\n#001011:0101\n";
    // Should not panic - the Err branch logs a warning
    let model = decoder.decode_bytes(data, false, None);
    assert!(model.is_some());
    // Verify a warning was logged
    assert!(
        decoder.log.iter().any(|l| l.message.contains("RANDOM")),
        "Expected a warning about malformed #RANDOM"
    );
}

// ---------------------------------------------------------------------------
// BMSDecoder: #IF without preceding #RANDOM
// ---------------------------------------------------------------------------

/// When `#IF` appears without a preceding `#RANDOM`, the parser logs
/// a warning and does not panic. This test confirms the safe handling.
#[test]
fn bms_decoder_if_without_random_handled_gracefully() {
    let mut decoder = BMSDecoder::new();
    let data = b"#BPM 120\n#IF 1\n#TITLE Conditional\n#ENDIF\n";
    let model = decoder.decode_bytes(data, false, None);
    assert!(model.is_some());
    assert!(
        decoder.log.iter().any(|l| l.message.contains("RANDOM")),
        "Expected a warning about #IF without #RANDOM"
    );
}

// ---------------------------------------------------------------------------
// BMSDecoder: #ENDRANDOM without preceding #RANDOM
// ---------------------------------------------------------------------------

/// When `#ENDRANDOM` appears without a preceding `#RANDOM`, the parser
/// logs a warning and does not panic.
#[test]
fn bms_decoder_endrandom_without_random_handled_gracefully() {
    let mut decoder = BMSDecoder::new();
    let data = b"#BPM 120\n#ENDRANDOM\n";
    let model = decoder.decode_bytes(data, false, None);
    assert!(model.is_some());
    assert!(
        decoder
            .log
            .iter()
            .any(|l| l.message.contains("ENDRANDOM") || l.message.contains("RANDOM")),
        "Expected a warning about #ENDRANDOM without #RANDOM"
    );
}

// ---------------------------------------------------------------------------
// BMSDecoder: #ENDIF without preceding #IF
// ---------------------------------------------------------------------------

/// When `#ENDIF` appears without a preceding `#IF`, the parser logs
/// a warning and does not panic.
#[test]
fn bms_decoder_endif_without_if_handled_gracefully() {
    let mut decoder = BMSDecoder::new();
    let data = b"#BPM 120\n#ENDIF\n";
    let model = decoder.decode_bytes(data, false, None);
    assert!(model.is_some());
    assert!(
        decoder
            .log
            .iter()
            .any(|l| l.message.contains("ENDIF") || l.message.contains("IF")),
        "Expected a warning about #ENDIF without #IF"
    );
}

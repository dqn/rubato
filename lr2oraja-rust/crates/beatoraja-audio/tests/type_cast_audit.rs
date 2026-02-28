//! Type cast overflow audit tests for BytePCM 32-bit float→i8 conversion.
//!
//! These tests document the behavior of the conversion path in BytePCM::load_pcm
//! for 32-bit float WAV samples.
//!
//! Source: crates/beatoraja-audio/src/byte_pcm.rs line 77
//! Code:   `s[i] = (f * i8::MAX as f32) as i8 as u8`
//!
//! The conversion chain: f32 → multiply by 127.0 → cast to i8 → cast to u8.
//! Rust's `as i8` saturates to i8::MAX (127) or i8::MIN (-128) for out-of-range
//! values since Rust 1.45. NaN becomes 0. This differs from Java's behavior
//! where float→byte is implementation-defined for out-of-range values.

use beatoraja_audio::byte_pcm::BytePCM;
use beatoraja_audio::pcm::PCMLoader;

/// Helper: create a PCMLoader with 32-bit float samples from raw f32 values.
/// Each f32 is stored as 4 little-endian bytes (IEEE 754).
fn make_float32_loader(samples: &[f32]) -> PCMLoader {
    let mut pcm_data = Vec::with_capacity(samples.len() * 4);
    for &s in samples {
        pcm_data.extend_from_slice(&s.to_le_bytes());
    }
    PCMLoader {
        pcm_data,
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 32,
        block_align: 4,
    }
}

/// f=1.0: 1.0 * 127 = 127.0 → i8 127 → u8 127.
/// This is the maximum valid normalized sample. Should be fine.
#[test]
fn byte_pcm_float_1_0_produces_127() {
    let loader = make_float32_loader(&[1.0]);
    let pcm = BytePCM::load_pcm(&loader).unwrap();
    assert_eq!(pcm.sample[0], 127, "f=1.0 should map to u8 127 (i8::MAX)");
}

/// f=-1.0: -1.0 * 127 = -127.0 → i8 -127 → u8 129.
/// Negative full-scale maps to u8 129 via two's complement reinterpretation.
#[test]
fn byte_pcm_float_neg1_0_produces_129() {
    let loader = make_float32_loader(&[-1.0]);
    let pcm = BytePCM::load_pcm(&loader).unwrap();
    // -127 as i8 has bit pattern 0x81 = 129 as u8
    assert_eq!(
        pcm.sample[0], 129,
        "f=-1.0 should map to u8 129 (-127 as i8 reinterpreted as u8)"
    );
}

/// f=0.0: 0.0 * 127 = 0.0 → i8 0 → u8 0.
/// Silence.
#[test]
fn byte_pcm_float_0_0_produces_0() {
    let loader = make_float32_loader(&[0.0]);
    let pcm = BytePCM::load_pcm(&loader).unwrap();
    assert_eq!(pcm.sample[0], 0, "f=0.0 should map to u8 0 (silence)");
}

/// BUG: f=2.0 (over-range sample, common in unmastered audio).
/// 2.0 * 127 = 254.0. In Rust 1.45+, `254.0 as i8` saturates to i8::MAX (127).
/// So the result is u8 127, same as f=1.0, silently clipping.
///
/// In Java, `(byte)(int)(2.0f * 127)` = `(byte)254` = -2, producing different
/// behavior. The Rust port clips to 127 instead of wrapping to -2.
///
/// A correct implementation should explicitly clamp to [-1.0, 1.0] range
/// before conversion, or use a wider intermediate type.
#[test]
fn byte_pcm_float_overflow() {
    let loader = make_float32_loader(&[2.0]);
    let pcm = BytePCM::load_pcm(&loader).unwrap();

    // In Java: (byte)(int)(2.0 * 127) = (byte)254 = -2 → unsigned 254
    // In Rust: (2.0 * 127.0) as i8 saturates to 127 → u8 127
    //
    // Neither behavior is correct — both silently corrupt the sample.
    // But they produce DIFFERENT wrong values, breaking Java↔Rust compatibility.
    //
    // Java result: 254 (wraps via int→byte truncation)
    // Rust result: 127 (saturates via f32→i8 saturation)
    let java_compatible_result: u8 = 254; // (-2i8 as u8)
    assert_eq!(
        pcm.sample[0], java_compatible_result,
        "f=2.0 should produce Java-compatible result 254 (Rust saturates to 127 instead)"
    );
}

/// BUG: f=NaN produces 0 in Rust (NaN as i8 = 0), which is "silence".
/// In Java, `(byte)(int)(Float.NaN * 127)` = `(byte)0` = 0, so this happens
/// to match. However, this is undefined behavior territory in Java and the
/// match is coincidental. The code should explicitly handle NaN.
#[test]
fn byte_pcm_float_nan_produces_zero() {
    let loader = make_float32_loader(&[f32::NAN]);
    let pcm = BytePCM::load_pcm(&loader).unwrap();
    // Rust: NaN as i8 = 0 → u8 0
    // Java: (byte)(int)(NaN * 127) = (byte)0 = 0
    // Both produce 0, but this is coincidental and fragile.
    assert_eq!(
        pcm.sample[0], 0,
        "NaN should map to 0 (silence), which Rust does by convention"
    );
}

/// BUG: f=-2.0 (negative over-range).
/// -2.0 * 127 = -254.0. In Rust 1.45+, `-254.0 as i8` saturates to i8::MIN (-128).
/// So the result is u8 128, not the Java-compatible value.
///
/// In Java: `(byte)(int)(-2.0f * 127)` = `(byte)(-254)` = 2 → u8 2.
/// Rust produces 128 instead.
#[test]
fn byte_pcm_float_neg_overflow() {
    let loader = make_float32_loader(&[-2.0]);
    let pcm = BytePCM::load_pcm(&loader).unwrap();

    // Java: (byte)(int)(-254.0) = (byte)(-254) = 2
    // Rust: (-254.0 as i8) saturates to -128 → u8 128
    let java_compatible_result: u8 = 2; // (((-254i32) as i8) as u8) in Java semantics
    assert_eq!(
        pcm.sample[0], java_compatible_result,
        "f=-2.0 should produce Java-compatible result 2 (Rust saturates to 128 instead)"
    );
}

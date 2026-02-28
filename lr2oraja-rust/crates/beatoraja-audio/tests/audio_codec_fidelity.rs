//! Audio codec fidelity tests for PCM conversion pipelines.
//!
//! These tests exercise boundary conditions in the PCM type conversion chains:
//! - MS-ADPCM block decoding edge cases
//! - ShortPCM <-> FloatPCM round-trip precision at i16 boundaries
//! - BytePCM silence and amplitude preservation
//! - Cross-format normalization consistency (8-bit via ShortPCM vs FloatPCM)
//!
//! Convention: tests assert actual behavior (GREEN). Where a real bug is found,
//! the test is marked `#[ignore]` with a BUG comment documenting the discrepancy.

use beatoraja_audio::byte_pcm::BytePCM;
use beatoraja_audio::float_pcm::FloatPCM;
use beatoraja_audio::ms_adpcm_decoder::MSADPCMDecoder;
use beatoraja_audio::pcm::PCMLoader;
use beatoraja_audio::short_pcm::ShortPCM;

/// Helper: create a PCMLoader with 16-bit samples from raw i16 values.
fn make_i16_loader(samples: &[i16]) -> PCMLoader {
    let mut pcm_data = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        pcm_data.extend_from_slice(&s.to_le_bytes());
    }
    PCMLoader {
        pcm_data,
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        block_align: 2,
    }
}

/// Helper: create a PCMLoader with 32-bit float samples from raw f32 values.
fn make_f32_loader(samples: &[f32]) -> PCMLoader {
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

/// Helper: create a PCMLoader with 8-bit samples.
fn make_u8_loader(samples: &[u8]) -> PCMLoader {
    PCMLoader {
        pcm_data: samples.to_vec(),
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 8,
        block_align: 1,
    }
}

// ---------------------------------------------------------------------------
// Test 1: MS-ADPCM minimum block (header only, 0 data nibbles)
// ---------------------------------------------------------------------------

/// MS-ADPCM mono block with block_align=7: header only, zero data nibbles.
///
/// Block layout (mono, 7 bytes):
///   byte 0: predictor index (0..6)
///   bytes 1-2: initial delta (i16 LE)
///   bytes 3-4: sample1 (i16 LE)
///   bytes 5-6: sample2 (i16 LE)
///
/// samples_per_block = (block_align - channels * 6) * 2 / channels
///                   = (7 - 1*6) * 2 / 1 = 2
///
/// The decoder should produce exactly 2 samples (sample2 first, then sample1)
/// from the header without any nibble expansion, and not panic.
#[test]
fn ms_adpcm_minimum_block_header_only() {
    let channels = 1;
    let sample_rate = 44100;
    let block_align = 7;

    let mut block = [0u8; 7];
    // predictor index = 0
    block[0] = 0;
    // initial delta = 16 (minimum valid delta, little-endian)
    block[1] = 16;
    block[2] = 0;
    // sample1 = 1000 (0x03E8 LE)
    block[3] = 0xE8;
    block[4] = 0x03;
    // sample2 = 500 (0x01F4 LE)
    block[5] = 0xF4;
    block[6] = 0x01;

    let mut decoder = MSADPCMDecoder::new(channels, sample_rate, block_align);
    let result = decoder.decode(&block);

    assert!(
        result.is_ok(),
        "Decoding a minimum-size MS-ADPCM block should not panic or error"
    );

    let output = result.unwrap();
    // Output is 16-bit LE samples: samples_per_block (2) * channels (1) * 2 bytes = 4 bytes
    assert_eq!(
        output.len(),
        4,
        "Expected 4 bytes (2 samples * 2 bytes each)"
    );

    // First output sample should be sample2 (500), second should be sample1 (1000)
    let out_sample0 = i16::from_le_bytes([output[0], output[1]]);
    let out_sample1 = i16::from_le_bytes([output[2], output[3]]);
    assert_eq!(
        out_sample0, 500,
        "First decoded sample should be sample2 from header"
    );
    assert_eq!(
        out_sample1, 1000,
        "Second decoded sample should be sample1 from header"
    );
}

// ---------------------------------------------------------------------------
// Test 2: ShortPCM -> FloatPCM -> ShortPCM round-trip at boundaries
// ---------------------------------------------------------------------------

/// Round-trip conversion ShortPCM -> FloatPCM -> ShortPCM at i16 boundary values.
///
/// Pipeline:
///   1. Pack i16 values as 16-bit LE bytes into PCMLoader
///   2. Load as FloatPCM (divides each i16 by i16::MAX = 32767.0)
///   3. Take resulting f32 values, pack as 32-bit float bytes into PCMLoader
///   4. Load as ShortPCM (multiplies each f32 by i16::MAX = 32767.0, casts to i16)
///
/// f32 has a 24-bit mantissa, which is more than sufficient for lossless
/// round-trip of all 16-bit integer values through the divide/multiply-by-32767
/// normalization. All i16 values survive the round-trip exactly, including
/// boundary values and the asymmetric i16::MIN case (-32768/32767 = -1.0000305).
#[test]
fn short_pcm_to_float_pcm_roundtrip_boundaries() {
    let boundary_values: &[i16] = &[i16::MIN, -1, 0, 1, i16::MAX];

    // Step 1: Create 16-bit LE PCMLoader and load as FloatPCM
    let i16_loader = make_i16_loader(boundary_values);
    let float_pcm = FloatPCM::load_pcm(&i16_loader).unwrap();

    // Step 2: Extract the float samples and create a 32-bit float PCMLoader
    let float_samples: Vec<f32> = float_pcm.sample.iter().copied().collect();
    let f32_loader = make_f32_loader(&float_samples);

    // Step 3: Load back as ShortPCM
    let short_pcm = ShortPCM::load_pcm(&f32_loader).unwrap();

    // Step 4: Verify round-trip fidelity for each boundary value
    for (i, &original) in boundary_values.iter().enumerate() {
        assert_eq!(
            short_pcm.sample[i], original,
            "Round-trip failed for i16 value {}: got {} (float intermediate: {})",
            original, short_pcm.sample[i], float_samples[i]
        );
    }
}

// ---------------------------------------------------------------------------
// Test 3: BytePCM silence preservation (all-zero 8-bit samples)
// ---------------------------------------------------------------------------

/// 8-bit PCM with all-zero samples should produce all-zero BytePCM output.
///
/// BytePCM::load_pcm with bits_per_sample=8 directly copies the input bytes.
/// Zero input must produce zero output to preserve digital silence.
#[test]
fn byte_pcm_silence_all_zeros() {
    let silence = vec![0u8; 64];
    let loader = make_u8_loader(&silence);
    let pcm = BytePCM::load_pcm(&loader).unwrap();

    assert_eq!(pcm.sample.len(), 64);
    for (i, &s) in pcm.sample.iter().enumerate() {
        assert_eq!(s, 0, "Sample {} should be 0 for silence, got {}", i, s);
    }
}

// ---------------------------------------------------------------------------
// Test 4: BytePCM max amplitude 8-bit values
// ---------------------------------------------------------------------------

/// 8-bit PCM at maximum values (127 = max positive signed, 255 = max unsigned).
///
/// BytePCM::load_pcm for 8-bit directly copies raw bytes without interpretation.
/// Note: WAV 8-bit is unsigned (128 = silence, 0 = negative peak, 255 = positive peak),
/// but BytePCM stores the raw bytes as-is. This test verifies the passthrough
/// behavior for extreme values.
#[test]
fn byte_pcm_max_amplitude_8bit() {
    // Test signed interpretation peak: 127 (0x7F) = max positive in signed view
    let loader_127 = make_u8_loader(&[127]);
    let pcm_127 = BytePCM::load_pcm(&loader_127).unwrap();
    assert_eq!(
        pcm_127.sample[0], 127,
        "8-bit value 127 should pass through unchanged"
    );

    // Test unsigned max: 255 (0xFF) = max unsigned value
    let loader_255 = make_u8_loader(&[255]);
    let pcm_255 = BytePCM::load_pcm(&loader_255).unwrap();
    assert_eq!(
        pcm_255.sample[0], 255,
        "8-bit value 255 should pass through unchanged"
    );

    // Test WAV silence (unsigned midpoint): 128
    let loader_128 = make_u8_loader(&[128]);
    let pcm_128 = BytePCM::load_pcm(&loader_128).unwrap();
    assert_eq!(
        pcm_128.sample[0], 128,
        "8-bit value 128 (WAV silence) should pass through unchanged"
    );

    // Test unsigned zero: 0 (negative peak in unsigned WAV format)
    let loader_0 = make_u8_loader(&[0]);
    let pcm_0 = BytePCM::load_pcm(&loader_0).unwrap();
    assert_eq!(
        pcm_0.sample[0], 0,
        "8-bit value 0 should pass through unchanged"
    );
}

// ---------------------------------------------------------------------------
// Test 5: ShortPCM DC offset preservation through float round-trip
// ---------------------------------------------------------------------------

/// A constant DC offset (all samples = 1000) should survive the
/// ShortPCM -> FloatPCM -> ShortPCM round-trip exactly.
///
/// Pipeline:
///   1000.0 / 32767.0 = 0.030518509... (f32 representable)
///   0.030518509... * 32767.0 = 1000.0 (exact due to f32 24-bit mantissa precision)
///
/// f32 has sufficient precision for lossless round-trip of all i16 values
/// through the divide/multiply-by-32767 normalization.
#[test]
fn short_pcm_dc_offset_preservation() {
    let dc_value: i16 = 1000;
    let num_samples = 16;
    let dc_samples: Vec<i16> = vec![dc_value; num_samples];

    // Step 1: Load as FloatPCM (16-bit LE -> f32 via divide by 32767)
    let i16_loader = make_i16_loader(&dc_samples);
    let float_pcm = FloatPCM::load_pcm(&i16_loader).unwrap();

    // Step 2: Extract float values and reload as ShortPCM (32-bit float -> i16 via multiply by 32767)
    let float_samples: Vec<f32> = float_pcm.sample.iter().copied().collect();
    let f32_loader = make_f32_loader(&float_samples);
    let short_pcm = ShortPCM::load_pcm(&f32_loader).unwrap();

    // Step 3: Verify DC offset is preserved exactly
    for (i, &float_val) in float_samples.iter().enumerate() {
        assert_eq!(
            short_pcm.sample[i], dc_value,
            "DC offset sample {} should be {} after round-trip, got {} (float: {})",
            i, dc_value, short_pcm.sample[i], float_val
        );
    }
}

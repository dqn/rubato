// Golden master tests: Java vs Rust audio processing comparison
//
// Compares WAV decode, resample, and channel conversion results
// between Java (AudioExporter) and Rust (beatoraja-audio) implementations.

use std::path::PathBuf;

use golden_master::audio_fixtures::{AudioTestCase, load_audio_fixture};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/audio_fixtures.json")
}

fn audio_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test-bms/audio")
}

fn get_test_case(name: &str) -> AudioTestCase {
    let fixture = load_audio_fixture(&fixture_path()).expect("Failed to load audio fixture");
    fixture
        .test_cases
        .into_iter()
        .find(|tc| tc.name == name)
        .unwrap_or_else(|| panic!("Test case '{}' not found in fixture", name))
}

/// Decode a WAV file and convert to i16 samples for comparison
fn decode_wav_to_i16(filename: &str) -> (Vec<i16>, u16, u32) {
    let path = audio_dir().join(filename);
    let pcm = rubato_audio::decode::load_audio(&path)
        .unwrap_or_else(|e| panic!("Failed to decode {}: {}", filename, e));
    let i16_samples = rubato_audio::bms_renderer::f32_to_i16(&pcm.samples);
    (i16_samples, pcm.channels, pcm.sample_rate)
}

/// Compare i16 sample arrays with tolerance, reporting mismatches
fn compare_samples(rust: &[i16], java: &[i16], tolerance: i16, test_name: &str) {
    assert_eq!(
        rust.len(),
        java.len(),
        "{}: sample count mismatch: rust={} java={}",
        test_name,
        rust.len(),
        java.len()
    );

    let mut mismatch_count = 0;
    let mut first_mismatch = None;
    let mut max_diff: i16 = 0;

    for (i, (&r, &j)) in rust.iter().zip(java.iter()).enumerate() {
        let diff = (r as i32 - j as i32).abs() as i16;
        if diff > tolerance {
            mismatch_count += 1;
            if first_mismatch.is_none() {
                first_mismatch = Some((i, r, j, diff));
            }
            if diff > max_diff {
                max_diff = diff;
            }
        }
    }

    if mismatch_count > 0 {
        let (idx, r, j, diff) = first_mismatch.unwrap();
        panic!(
            "{}: {} mismatches (tolerance=±{}), max_diff={}, first at index {}: rust={} java={} diff={}",
            test_name, mismatch_count, tolerance, max_diff, idx, r, j, diff
        );
    }
}

// ========== Category 1: WAV PCM Decode ==========

#[test]
fn wav_16bit_mono_decode() {
    let tc = get_test_case("wav_16bit_mono_decode");
    let (rust_samples, channels, sample_rate) = decode_wav_to_i16(&tc.source_file);

    assert_eq!(channels, tc.channels.unwrap());
    assert_eq!(sample_rate, tc.sample_rate.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 1, "wav_16bit_mono_decode");
}

#[test]
fn wav_16bit_stereo_decode() {
    let tc = get_test_case("wav_16bit_stereo_decode");
    let (rust_samples, channels, sample_rate) = decode_wav_to_i16(&tc.source_file);

    assert_eq!(channels, tc.channels.unwrap());
    assert_eq!(sample_rate, tc.sample_rate.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 1, "wav_16bit_stereo_decode");
}

#[test]
fn wav_8bit_mono_decode() {
    let tc = get_test_case("wav_8bit_mono_decode");
    let (rust_samples, channels, sample_rate) = decode_wav_to_i16(&tc.source_file);

    assert_eq!(channels, tc.channels.unwrap());
    assert_eq!(sample_rate, tc.sample_rate.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 1, "wav_8bit_mono_decode");
}

#[test]
fn wav_24bit_mono_decode() {
    let tc = get_test_case("wav_24bit_mono_decode");
    let (rust_samples, channels, sample_rate) = decode_wav_to_i16(&tc.source_file);

    assert_eq!(channels, tc.channels.unwrap());
    assert_eq!(sample_rate, tc.sample_rate.unwrap());
    // ±2 tolerance: Java reads upper 16 bits directly via getShort(i*3+1),
    // Rust normalizes 24-bit to f32 then converts back to i16, causing rounding diffs.
    compare_samples(&rust_samples, &tc.samples_i16, 2, "wav_24bit_mono_decode");
}

#[test]
fn wav_float32_mono_decode() {
    let tc = get_test_case("wav_float32_mono_decode");
    let (rust_samples, channels, sample_rate) = decode_wav_to_i16(&tc.source_file);

    assert_eq!(channels, tc.channels.unwrap());
    assert_eq!(sample_rate, tc.sample_rate.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 1, "wav_float32_mono_decode");
}

#[test]
fn wav_adpcm_mono_decode() {
    let tc = get_test_case("wav_adpcm_mono_decode");
    let (rust_samples, channels, sample_rate) = decode_wav_to_i16(&tc.source_file);

    assert_eq!(channels, tc.channels.unwrap());
    assert_eq!(sample_rate, tc.sample_rate.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 1, "wav_adpcm_mono_decode");
}

// ========== Category 2: Sample Rate Conversion ==========

#[test]
fn resample_44100_to_22050() {
    let tc = get_test_case("resample_44100_to_22050");
    let path = audio_dir().join(&tc.source_file);
    let pcm = rubato_audio::decode::load_audio(&path).unwrap();

    let resampled = pcm.change_sample_rate(tc.target_rate.unwrap());
    let rust_samples = rubato_audio::bms_renderer::f32_to_i16(&resampled.samples);

    assert_eq!(resampled.sample_rate, tc.target_rate.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 1, "resample_44100_to_22050");
}

#[test]
fn resample_44100_to_48000() {
    let tc = get_test_case("resample_44100_to_48000");
    let path = audio_dir().join(&tc.source_file);
    let pcm = rubato_audio::decode::load_audio(&path).unwrap();

    let resampled = pcm.change_sample_rate(tc.target_rate.unwrap());
    let rust_samples = rubato_audio::bms_renderer::f32_to_i16(&resampled.samples);

    assert_eq!(resampled.sample_rate, tc.target_rate.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 1, "resample_44100_to_48000");
}

#[test]
fn resample_48000_to_44100() {
    let tc = get_test_case("resample_48000_to_44100");
    let path = audio_dir().join(&tc.source_file);
    let pcm = rubato_audio::decode::load_audio(&path).unwrap();

    let resampled = pcm.change_sample_rate(tc.target_rate.unwrap());
    let rust_samples = rubato_audio::bms_renderer::f32_to_i16(&resampled.samples);

    assert_eq!(resampled.sample_rate, tc.target_rate.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 2, "resample_48000_to_44100");
}

// ========== Category 3: Channel Conversion ==========

#[test]
fn channel_mono_to_stereo() {
    let tc = get_test_case("channel_mono_to_stereo");
    let path = audio_dir().join(&tc.source_file);
    let pcm = rubato_audio::decode::load_audio(&path).unwrap();

    let converted = pcm.change_channels(tc.target_channels.unwrap());
    let rust_samples = rubato_audio::bms_renderer::f32_to_i16(&converted.samples);

    assert_eq!(converted.channels, tc.target_channels.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 1, "channel_mono_to_stereo");
}

#[test]
fn channel_stereo_to_mono() {
    let tc = get_test_case("channel_stereo_to_mono");
    let path = audio_dir().join(&tc.source_file);
    let pcm = rubato_audio::decode::load_audio(&path).unwrap();

    let converted = pcm.change_channels(tc.target_channels.unwrap());
    let rust_samples = rubato_audio::bms_renderer::f32_to_i16(&converted.samples);

    assert_eq!(converted.channels, tc.target_channels.unwrap());
    compare_samples(&rust_samples, &tc.samples_i16, 1, "channel_stereo_to_mono");
}

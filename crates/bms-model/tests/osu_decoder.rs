//! Integration tests for the OSU decoder.
//!
//! These tests verify the full decode pipeline from .osu file to BMSModel,
//! including timing points, hit object conversion, long note handling,
//! section generation, and edge cases.

use std::io::Write;
use std::path::PathBuf;

use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::osu_decoder::OSUDecoder;
use tempfile::NamedTempFile;

/// Path to the test fixture directory.
fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // lr2oraja-rust/  -- wait, CARGO_MANIFEST_DIR is crates/bms-model
    // So pop twice goes to lr2oraja-rust
    path.push("test-bms");
    path.push(name);
    path
}

/// Helper: write content to a temporary .osu file and return the path.
fn write_temp_osu(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new()
        .suffix(".osu")
        .tempfile()
        .expect("failed to create temp file");
    f.write_all(content.as_bytes())
        .expect("failed to write temp file");
    f.flush().expect("failed to flush temp file");
    f
}

/// Collect all notes (non-background) from all timelines across all lanes.
fn collect_lane_notes(model: &bms_model::bms_model::BMSModel) -> Vec<(i32, &Note)> {
    let mut notes = Vec::new();
    for tl in &model.timelines {
        for lane in 0..tl.lane_count() {
            if let Some(note) = tl.note(lane) {
                notes.push((lane, note));
            }
        }
    }
    notes
}

// ---------------------------------------------------------------------------
// Basic 7K .osu fixture decode
// ---------------------------------------------------------------------------

#[test]
fn decode_7k_fixture_metadata() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    assert_eq!(model.title.as_str(), "Test Song");
    assert_eq!(model.sub_title.as_str(), "[7K Hard]");
    assert_eq!(model.artist(), "Test Artist");
    assert_eq!(model.sub_artist(), "Test Creator");
    assert_eq!(model.genre(), "7K");
    assert_eq!(model.mode(), Some(&Mode::BEAT_7K));
}

#[test]
fn decode_7k_fixture_bpm() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    // beat_length=500ms => BPM = 60000/500 = 120
    let bpm = model.bpm;
    assert!((bpm - 120.0).abs() < 0.01, "expected BPM ~120, got {}", bpm);
}

#[test]
fn decode_7k_fixture_timing_points() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    // Verify timelines contain BPM data from the uninherited timing point
    let timelines = &&model.timelines;
    assert!(!timelines.is_empty(), "decoded model should have timelines");

    // The first timing point is at time=0ms with beat_length=500 (120 BPM).
    // After offset adjustment (+38ms), the timeline at time=38 should have BPM=120.
    let first_bpm_tl = timelines.iter().find(|tl| (tl.bpm - 120.0).abs() < 0.01);
    assert!(
        first_bpm_tl.is_some(),
        "should have at least one timeline with BPM=120"
    );
}

#[test]
fn decode_7k_fixture_scroll_velocity() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    // The inherited timing point at 10000ms has beat_length=-50,
    // so scroll = 100 / -(-50) = 2.0
    let timelines = &&model.timelines;
    let sv2_tl = timelines.iter().find(|tl| (tl.scroll - 2.0).abs() < 0.01);
    assert!(
        sv2_tl.is_some(),
        "should have a timeline with scroll velocity 2.0x from inherited point"
    );
}

#[test]
fn decode_7k_fixture_normal_notes() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    let notes = collect_lane_notes(&model);

    // Count normal (non-LN) notes
    let normal_count = notes
        .iter()
        .filter(|(_, n)| matches!(n, Note::Normal(_)))
        .count();

    // We have 6 normal hit objects + 1 LN (head+tail) in the fixture.
    // Normal notes: lines at t=0 (cols 0,1), t=500 (cols 2,3), t=1000 (col 5), t=1500 (col 6)
    assert!(
        normal_count >= 6,
        "expected at least 6 normal notes, got {}",
        normal_count
    );
}

#[test]
fn decode_7k_fixture_long_note() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    let notes = collect_lane_notes(&model);

    // Count LN parts (head + tail = 2 Long variants)
    let ln_parts: Vec<_> = notes
        .iter()
        .filter(|(_, n)| matches!(n, Note::Long { .. }))
        .collect();

    assert!(
        ln_parts.len() >= 2,
        "expected at least 2 LN parts (head + tail), got {}",
        ln_parts.len()
    );

    // Verify we have both head (end=false) and tail (end=true)
    let has_head = ln_parts
        .iter()
        .any(|(_, n)| matches!(n, Note::Long { end: false, .. }));
    let has_tail = ln_parts
        .iter()
        .any(|(_, n)| matches!(n, Note::Long { end: true, .. }));
    assert!(has_head, "should have LN head (end=false)");
    assert!(has_tail, "should have LN tail (end=true)");
}

// ---------------------------------------------------------------------------
// Section generation
// ---------------------------------------------------------------------------

#[test]
fn decode_7k_fixture_section_lines() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    let timelines = &&model.timelines;
    let section_line_count = timelines.iter().filter(|tl| tl.section_line).count();

    // With beat_length=500ms and notes spanning ~2000ms, we should have
    // multiple section lines (one per measure = 4 beats = 2000ms at 120 BPM).
    assert!(
        section_line_count >= 1,
        "expected at least 1 section line, got {}",
        section_line_count
    );
}

#[test]
fn decode_7k_fixture_sections_are_monotonically_increasing() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    let timelines = &&model.timelines;
    let sections: Vec<f64> = timelines.iter().map(|tl| tl.get_section()).collect();

    for window in sections.windows(2) {
        assert!(
            window[1] >= window[0],
            "sections should be monotonically non-decreasing: {} followed by {}",
            window[0],
            window[1]
        );
    }
}

// ---------------------------------------------------------------------------
// Events: background image, video, and sample sound
// ---------------------------------------------------------------------------

#[test]
fn decode_7k_fixture_events() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    // Event type "0" sets backbmp/stagefile
    assert_eq!(model.backbmp(), "bg.jpg");
    assert_eq!(model.stagefile(), "bg.jpg");

    // Video event adds to bga_list
    let bga_list = &model.bgamap;
    assert!(
        bga_list.iter().any(|s| s == "video.mp4"),
        "bga_list should contain 'video.mp4', got: {:?}",
        bga_list
    );

    // Audio preview set from general.audio_filename
    assert_eq!(model.preview(), "audio.mp3");
}

// ---------------------------------------------------------------------------
// Hash computation (md5 and sha256)
// ---------------------------------------------------------------------------

#[test]
fn decode_7k_fixture_hashes_are_nonempty() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    assert!(!model.md5().is_empty(), "md5 hash should be computed");
    assert!(!model.sha256().is_empty(), "sha256 hash should be computed");
    // MD5 is always 32 hex chars
    assert_eq!(model.md5().len(), 32, "md5 should be 32 hex chars");
    // SHA256 is always 64 hex chars
    assert_eq!(model.sha256().len(), 64, "sha256 should be 64 hex chars");
}

// ---------------------------------------------------------------------------
// Key mode mapping
// ---------------------------------------------------------------------------

#[test]
fn decode_4k_mode() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:4K Test
Artist:Test
Creator:Test
Version:4K

[Difficulty]
CircleSize:4

[TimingPoints]
0,500,4,1,0,70,1,0

[HitObjects]
64,192,100,1,0,0:0:0:0:
192,192,100,1,0,0:0:0:0:
320,192,200,1,0,0:0:0:0:
448,192,200,1,0,0:0:0:0:
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(f.path())
        .expect("should decode 4K .osu");

    assert_eq!(model.mode(), Some(&Mode::BEAT_7K));
    assert_eq!(model.genre(), "4K");
}

#[test]
fn decode_5k_mode() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:5K Test
Artist:Test
Creator:Test
Version:5K

[Difficulty]
CircleSize:5

[TimingPoints]
0,500,4,1,0,70,1,0

[HitObjects]
51,192,100,1,0,0:0:0:0:
153,192,100,1,0,0:0:0:0:
256,192,200,1,0,0:0:0:0:
358,192,200,1,0,0:0:0:0:
460,192,300,1,0,0:0:0:0:
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(f.path())
        .expect("should decode 5K .osu");

    assert_eq!(model.mode(), Some(&Mode::BEAT_5K));
    assert_eq!(model.genre(), "5K");
}

#[test]
fn decode_9k_mode() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:9K Test
Artist:Test
Creator:Test
Version:9K

[Difficulty]
CircleSize:9

[TimingPoints]
0,500,4,1,0,70,1,0

[HitObjects]
28,192,100,1,0,0:0:0:0:
85,192,100,1,0,0:0:0:0:
142,192,200,1,0,0:0:0:0:
199,192,200,1,0,0:0:0:0:
256,192,300,1,0,0:0:0:0:
313,192,300,1,0,0:0:0:0:
370,192,400,1,0,0:0:0:0:
427,192,400,1,0,0:0:0:0:
484,192,500,1,0,0:0:0:0:
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(f.path())
        .expect("should decode 9K .osu");

    assert_eq!(model.mode(), Some(&Mode::POPN_9K));
    assert_eq!(model.genre(), "9K");
}

// ---------------------------------------------------------------------------
// Long note: tail time <= head time degrades to normal note
// ---------------------------------------------------------------------------

#[test]
fn ln_with_tail_before_head_becomes_normal_note() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:LN Edge
Artist:Test
Creator:Test
Version:7K

[Difficulty]
CircleSize:7

[TimingPoints]
0,500,4,1,0,70,1,0

[HitObjects]
36,192,1000,128,0,500:0:0:0:0:
";
    // tail_time=500 + offset(38) = 538, head_time=1000 + offset(38) = 1038
    // tail <= head, so this should degrade to a normal note
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(f.path())
        .expect("should decode LN edge case");

    let notes = collect_lane_notes(&model);
    let ln_count = notes
        .iter()
        .filter(|(_, n)| matches!(n, Note::Long { .. }))
        .count();
    let normal_count = notes
        .iter()
        .filter(|(_, n)| matches!(n, Note::Normal(_)))
        .count();

    // The LN with tail<=head should be converted to a normal note
    assert_eq!(
        ln_count, 0,
        "LN with tail<=head should not produce Long notes"
    );
    assert!(
        normal_count >= 1,
        "LN with tail<=head should produce a normal note"
    );
}

// ---------------------------------------------------------------------------
// Edge case: non-mania mode returns None
// ---------------------------------------------------------------------------

#[test]
fn non_mania_mode_returns_none() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 0

[Metadata]
Title:Standard Mode
Artist:Test
Creator:Test
Version:Normal

[Difficulty]
CircleSize:4

[TimingPoints]
0,500,4,1,0,70,1,0

[HitObjects]
256,192,100,1,0,0:0:0:0:
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let result = decoder.decode_path(f.path());
    assert!(
        result.is_none(),
        "non-mania mode (mode=0) should return None"
    );
}

// ---------------------------------------------------------------------------
// Edge case: unsupported key count returns None
// ---------------------------------------------------------------------------

#[test]
fn unsupported_keycount_returns_none() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:Unsupported
Artist:Test
Creator:Test
Version:3K

[Difficulty]
CircleSize:3

[TimingPoints]
0,500,4,1,0,70,1,0

[HitObjects]
85,192,100,1,0,0:0:0:0:
256,192,100,1,0,0:0:0:0:
426,192,200,1,0,0:0:0:0:
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let result = decoder.decode_path(f.path());
    assert!(
        result.is_none(),
        "unsupported key count (3K) should return None"
    );
}

// ---------------------------------------------------------------------------
// Edge case: empty file returns None
// ---------------------------------------------------------------------------

#[test]
fn empty_file_returns_none() {
    let f = write_temp_osu("");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let result = decoder.decode_path(f.path());
    assert!(result.is_none(), "empty file should return None");
}

// ---------------------------------------------------------------------------
// Edge case: file with only headers (no timing points or hit objects)
// ---------------------------------------------------------------------------

#[test]
fn headers_only_returns_none() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:Headers Only
Artist:Test
Creator:Test
Version:7K

[Difficulty]
CircleSize:7
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let result = decoder.decode_path(f.path());
    assert!(
        result.is_none(),
        "file with no timing points and no hit objects should return None"
    );
}

// ---------------------------------------------------------------------------
// Edge case: timing points but no hit objects returns None
// ---------------------------------------------------------------------------

#[test]
fn timing_points_only_returns_none() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:No Notes
Artist:Test
Creator:Test
Version:7K

[Difficulty]
CircleSize:7

[TimingPoints]
0,500,4,1,0,70,1,0
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let result = decoder.decode_path(f.path());
    assert!(
        result.is_none(),
        "file with timing points but no hit objects should return None"
    );
}

// ---------------------------------------------------------------------------
// Edge case: hit objects but no timing points returns None
// ---------------------------------------------------------------------------

#[test]
fn hit_objects_only_returns_none() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:No Timing
Artist:Test
Creator:Test
Version:7K

[Difficulty]
CircleSize:7

[HitObjects]
36,192,100,1,0,0:0:0:0:
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let result = decoder.decode_path(f.path());
    assert!(
        result.is_none(),
        "file with hit objects but no timing points should return None"
    );
}

// ---------------------------------------------------------------------------
// Edge case: negative time hit object is skipped
// ---------------------------------------------------------------------------

#[test]
fn negative_time_hit_object_skipped() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:Negative Time
Artist:Test
Creator:Test
Version:7K

[Difficulty]
CircleSize:7

[TimingPoints]
0,500,4,1,0,70,1,0

[HitObjects]
36,192,-100,1,0,0:0:0:0:
109,192,100,1,0,0:0:0:0:
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(f.path())
        .expect("should decode despite negative-time object");

    let notes = collect_lane_notes(&model);
    let normal_count = notes
        .iter()
        .filter(|(_, n)| matches!(n, Note::Normal(_)))
        .count();

    // Only the note at t=100 should be present; the one at t=-100 is skipped
    assert_eq!(
        normal_count, 1,
        "negative-time hit object should be skipped, expected 1 note, got {}",
        normal_count
    );
}

// ---------------------------------------------------------------------------
// Multiple timing points (BPM changes)
// ---------------------------------------------------------------------------

#[test]
fn multiple_timing_points_bpm_change() {
    let content = "\
osu file format v14

[General]
AudioFilename: audio.mp3
Mode: 3

[Metadata]
Title:BPM Change
Artist:Test
Creator:Test
Version:7K

[Difficulty]
CircleSize:7

[TimingPoints]
0,500,4,1,0,70,1,0
5000,250,4,1,0,70,1,0

[HitObjects]
36,192,100,1,0,0:0:0:0:
36,192,6000,1,0,0:0:0:0:
";
    let f = write_temp_osu(content);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(f.path())
        .expect("should decode with BPM change");

    let timelines = &&model.timelines;

    // First timing point: beat_length=500 => BPM=120
    let has_120bpm = timelines.iter().any(|tl| (tl.bpm - 120.0).abs() < 0.01);
    assert!(has_120bpm, "should have timelines with BPM=120");

    // Second timing point: beat_length=250 => BPM=240
    let has_240bpm = timelines.iter().any(|tl| (tl.bpm - 240.0).abs() < 0.01);
    assert!(has_240bpm, "should have timelines with BPM=240");
}

// ---------------------------------------------------------------------------
// WAV list includes audio filename
// ---------------------------------------------------------------------------

#[test]
fn wav_list_includes_audio_and_samples() {
    let path = fixture_path("osu_7k_basic.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode_path(&path)
        .expect("should decode 7K .osu fixture");

    let wav_list = model.wav_list();
    // First entry should be the general audio filename
    assert_eq!(
        wav_list[0], "audio.mp3",
        "first wav entry should be the audio filename"
    );
    // Sample event "effect.wav" should also be in the list
    assert!(
        wav_list.iter().any(|s| s == "effect.wav"),
        "wav_list should contain 'effect.wav' from Sample event, got: {:?}",
        wav_list
    );
}

// ---------------------------------------------------------------------------
// Nonexistent file returns None
// ---------------------------------------------------------------------------

#[test]
fn nonexistent_file_returns_none() {
    let path = PathBuf::from("/tmp/does_not_exist_osu_test_12345.osu");
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let result = decoder.decode_path(&path);
    assert!(result.is_none(), "nonexistent file should return None");
}

// ---------------------------------------------------------------------------
// decode() via ChartInformation
// ---------------------------------------------------------------------------

#[test]
fn decode_via_chart_information() {
    use bms_model::chart_information::ChartInformation;

    let path = fixture_path("osu_7k_basic.osu");
    let info = ChartInformation::new(Some(path), bms_model::bms_model::LnType::LongNote, None);
    let mut decoder = OSUDecoder::new(bms_model::bms_model::LnType::LongNote);
    let model = decoder
        .decode(info)
        .expect("should decode via ChartInformation");

    assert_eq!(model.title.as_str(), "Test Song");
}

// Golden master test infrastructure: Java fixture comparison harness

pub mod audio_fixtures;
pub mod autoplay_fixtures;
pub mod bga_timeline_fixtures;
pub mod course_data_fixtures;
pub mod database_fixtures;
// e2e_helpers depends on beatoraja-play APIs (JudgeManager, GrooveGauge, PlayerRule, etc.)
// that have different names/signatures than expected (e.g., BMSPlayerRule vs PlayerRule,
// judge constants as raw i32 vs named constants). Moved to src/pending/.
// pub mod e2e_helpers;
pub mod judge_fixtures;
pub mod pattern_fixtures;
pub mod pattern_modifier_detail_fixtures;
// render_snapshot depends on beatoraja-skin/beatoraja-render internal APIs
// (SkinStateProvider, eval, property_id, etc.). Moved to src/pending/.
// pub mod render_snapshot;
pub mod replay_e2e_fixtures;
pub mod rule_fixtures;
pub mod score_data_property_fixtures;
pub mod skin_fixtures;
pub mod song_information_fixtures;

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;

/// Java fixture root structure
#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub metadata: FixtureMetadata,
    pub hashes: FixtureHashes,
    pub statistics: FixtureStatistics,
    #[serde(default)]
    pub timelines: Vec<FixtureTimeline>,
    pub notes: Vec<FixtureNote>,
    pub bpm_changes: Vec<FixtureBpmChange>,
    pub stop_events: Vec<FixtureStopEvent>,
}

#[derive(Debug, Deserialize)]
pub struct FixtureMetadata {
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub sub_artist: String,
    pub genre: String,
    pub initial_bpm: f64,
    pub judge_rank: i32,
    pub total: f64,
    pub player: i32,
    pub mode: String,
    pub mode_key_count: usize,
    pub ln_type: i32,
    pub banner: String,
    pub stagefile: String,
    pub backbmp: String,
    pub preview: String,
}

#[derive(Debug, Deserialize)]
pub struct FixtureHashes {
    pub md5: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
pub struct FixtureStatistics {
    pub total_notes: usize,
    pub total_notes_mine: usize,
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub timeline_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct FixtureTimeline {
    pub time_us: i64,
    pub bpm: f64,
    pub stop_us: i64,
    #[serde(default)]
    pub notes: Vec<FixtureNote>,
    #[serde(default)]
    pub hidden_notes: Vec<FixtureNote>,
}

#[derive(Debug, Deserialize)]
pub struct FixtureNote {
    pub lane: usize,
    pub time_us: i64,
    /// Java wav_id is a wavlist index (0-based), different from Rust base36 value
    pub wav_id: i32,
    #[serde(rename = "type")]
    pub note_type: String,
    pub end_time_us: Option<i64>,
    /// May be -2 for undefined in Java
    pub end_wav_id: Option<i32>,
    pub damage: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct FixtureBpmChange {
    pub time_us: i64,
    pub bpm: f64,
}

#[derive(Debug, Deserialize)]
pub struct FixtureStopEvent {
    pub time_us: i64,
    pub duration_us: i64,
}

/// Load a fixture JSON file
pub fn load_fixture(path: &Path) -> Result<Fixture> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read fixture: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse fixture: {}", path.display()))
}

/// Map Java mode hint string to Rust Mode
pub fn mode_hint_to_mode(hint: &str) -> Option<Mode> {
    Mode::get_mode(hint)
}

/// Collect all playable notes (non-LN-end) from the model as flat list,
/// sorted by (time_us, lane).
struct FlatNote {
    lane: usize,
    time_us: i64,
    note_type_str: String,
    wav_id: i32,
    end_time_us: i64,
    end_wav_id: i32,
    damage: f64,
}

fn flatten_notes(model: &BMSModel) -> Vec<FlatNote> {
    let keys = model.get_mode().map(|m| m.key()).unwrap_or(0);
    let timelines = model.get_all_time_lines();
    let mut flat = Vec::new();

    // Per-timeline: regular notes for all lanes, then hidden notes for all lanes.
    // This matches the Java iteration order within each timeline.
    for (tl_idx, tl) in timelines.iter().enumerate() {
        // First: regular notes for all lanes
        for lane in 0..keys {
            if let Some(note) = tl.get_note(lane) {
                // Skip LN end notes
                if note.is_end() {
                    continue;
                }
                let note_type_str = match note {
                    Note::Normal(_) => "Normal".to_string(),
                    Note::Long { note_type, .. } => match *note_type {
                        bms_model::note::TYPE_LONGNOTE => "LongNote".to_string(),
                        bms_model::note::TYPE_CHARGENOTE => "ChargeNote".to_string(),
                        bms_model::note::TYPE_HELLCHARGENOTE => "HellChargeNote".to_string(),
                        _ => "LongNoteUndefined".to_string(),
                    },
                    Note::Mine { .. } => "Mine".to_string(),
                };

                // For LN notes, find the paired end note by scanning forward
                let (end_time_us, end_wav_id) = if note.is_long() {
                    find_ln_end_time(timelines, tl_idx, lane)
                } else {
                    (0, note.get_wav())
                };

                flat.push(FlatNote {
                    lane: lane as usize,
                    time_us: note.get_micro_time(),
                    note_type_str,
                    wav_id: note.get_wav(),
                    end_time_us,
                    end_wav_id,
                    damage: note.get_damage(),
                });
            }
        }
        // Then: hidden (invisible) notes for all lanes
        for lane in 0..keys {
            if let Some(note) = tl.get_hidden_note(lane) {
                flat.push(FlatNote {
                    lane: lane as usize,
                    time_us: note.get_micro_time(),
                    note_type_str: "Invisible".to_string(),
                    wav_id: note.get_wav(),
                    end_time_us: 0,
                    end_wav_id: note.get_wav(),
                    damage: 0.0,
                });
            }
        }
    }

    flat
}

/// Find the end time and wav of the paired LN-end note for a LN-start note.
/// Scans forward from the start timeline index on the same lane.
fn find_ln_end_time(
    timelines: &[bms_model::time_line::TimeLine],
    start_tl_idx: usize,
    lane: i32,
) -> (i64, i32) {
    for tl in &timelines[(start_tl_idx + 1)..] {
        if let Some(note) = tl.get_note(lane)
            && note.is_long()
            && note.is_end()
        {
            return (note.get_micro_time(), note.get_wav());
        }
    }
    // No end note found
    (0, -2)
}

/// Count mine notes in the model.
fn count_mines(model: &BMSModel) -> usize {
    let keys = model.get_mode().map(|m| m.key()).unwrap_or(0);
    let mut count = 0;
    for tl in model.get_all_time_lines() {
        for lane in 0..keys {
            if let Some(note) = tl.get_note(lane)
                && note.is_mine()
            {
                count += 1;
            }
        }
    }
    count
}

/// Compare a Rust BMSModel against a Java fixture.
/// Returns a list of differences found.
pub fn compare_model(model: &BMSModel, fixture: &Fixture) -> Vec<String> {
    let mut diffs = Vec::new();

    // Metadata
    if model.get_title() != fixture.metadata.title {
        diffs.push(format!(
            "title: rust={:?} java={:?}",
            model.get_title(),
            fixture.metadata.title
        ));
    }
    if model.get_sub_title() != fixture.metadata.subtitle {
        diffs.push(format!(
            "subtitle: rust={:?} java={:?}",
            model.get_sub_title(),
            fixture.metadata.subtitle
        ));
    }
    if model.get_artist() != fixture.metadata.artist {
        diffs.push(format!(
            "artist: rust={:?} java={:?}",
            model.get_artist(),
            fixture.metadata.artist
        ));
    }
    if model.get_sub_artist() != fixture.metadata.sub_artist {
        diffs.push(format!(
            "sub_artist: rust={:?} java={:?}",
            model.get_sub_artist(),
            fixture.metadata.sub_artist
        ));
    }
    if model.get_genre() != fixture.metadata.genre {
        diffs.push(format!(
            "genre: rust={:?} java={:?}",
            model.get_genre(),
            fixture.metadata.genre
        ));
    }
    if (model.get_bpm() - fixture.metadata.initial_bpm).abs() > 0.001 {
        diffs.push(format!(
            "initial_bpm: rust={} java={}",
            model.get_bpm(),
            fixture.metadata.initial_bpm
        ));
    }
    if model.get_judgerank() != fixture.metadata.judge_rank {
        diffs.push(format!(
            "judge_rank: rust={} java={}",
            model.get_judgerank(),
            fixture.metadata.judge_rank
        ));
    }
    if (model.get_total() - fixture.metadata.total).abs() > 0.001 {
        diffs.push(format!(
            "total: rust={} java={}",
            model.get_total(),
            fixture.metadata.total
        ));
    }
    if fixture.metadata.player > 0 && model.get_player() != fixture.metadata.player {
        diffs.push(format!(
            "player: rust={} java={}",
            model.get_player(),
            fixture.metadata.player
        ));
    }
    if let Some(mode) = model.get_mode()
        && mode.key() as usize != fixture.metadata.mode_key_count
    {
        diffs.push(format!(
            "mode_key_count: rust={} java={}",
            mode.key(),
            fixture.metadata.mode_key_count
        ));
    }
    if fixture.metadata.ln_type > 0 && model.get_lnmode() != fixture.metadata.ln_type {
        diffs.push(format!(
            "ln_type: rust={} java={}",
            model.get_lnmode(),
            fixture.metadata.ln_type
        ));
    }
    if model.get_banner() != fixture.metadata.banner {
        diffs.push(format!(
            "banner: rust={:?} java={:?}",
            model.get_banner(),
            fixture.metadata.banner
        ));
    }
    if model.get_stagefile() != fixture.metadata.stagefile {
        diffs.push(format!(
            "stagefile: rust={:?} java={:?}",
            model.get_stagefile(),
            fixture.metadata.stagefile
        ));
    }
    if model.get_backbmp() != fixture.metadata.backbmp {
        diffs.push(format!(
            "backbmp: rust={:?} java={:?}",
            model.get_backbmp(),
            fixture.metadata.backbmp
        ));
    }
    if model.get_preview() != fixture.metadata.preview {
        diffs.push(format!(
            "preview: rust={:?} java={:?}",
            model.get_preview(),
            fixture.metadata.preview
        ));
    }

    // Play mode
    if let Some(expected_mode) = mode_hint_to_mode(&fixture.metadata.mode) {
        if let Some(actual_mode) = model.get_mode() {
            if actual_mode != &expected_mode {
                diffs.push(format!(
                    "mode: rust={:?} java={:?} ({})",
                    actual_mode, expected_mode, fixture.metadata.mode
                ));
            }
        } else {
            diffs.push(format!(
                "mode: rust=None java={:?} ({})",
                expected_mode, fixture.metadata.mode
            ));
        }
    }

    // Hashes
    if model.get_md5() != fixture.hashes.md5 {
        diffs.push(format!(
            "md5: rust={} java={}",
            model.get_md5(),
            fixture.hashes.md5
        ));
    }
    if model.get_sha256() != fixture.hashes.sha256 {
        diffs.push(format!(
            "sha256: rust={} java={}",
            model.get_sha256(),
            fixture.hashes.sha256
        ));
    }

    // Statistics
    let rust_total_notes = model.get_total_notes() as usize;
    if rust_total_notes != fixture.statistics.total_notes {
        diffs.push(format!(
            "total_notes: rust={} java={}",
            rust_total_notes, fixture.statistics.total_notes
        ));
    }

    let rust_mines = count_mines(model);
    if rust_mines != fixture.statistics.total_notes_mine {
        diffs.push(format!(
            "mine_notes: rust={} java={}",
            rust_mines, fixture.statistics.total_notes_mine
        ));
    }

    if (model.get_min_bpm() - fixture.statistics.min_bpm).abs() > 0.001 {
        diffs.push(format!(
            "min_bpm: rust={} java={}",
            model.get_min_bpm(),
            fixture.statistics.min_bpm
        ));
    }
    if (model.get_max_bpm() - fixture.statistics.max_bpm).abs() > 0.001 {
        diffs.push(format!(
            "max_bpm: rust={} java={}",
            model.get_max_bpm(),
            fixture.statistics.max_bpm
        ));
    }

    // Timeline count consistency check
    if fixture.statistics.timeline_count != fixture.timelines.len() {
        diffs.push(format!(
            "timeline_count(fixture_consistency): statistics={} timelines={}",
            fixture.statistics.timeline_count,
            fixture.timelines.len()
        ));
    }

    // Notes comparison (flat list, excluding LN ends)
    let rust_notes = flatten_notes(model);
    let fixture_notes = &fixture.notes;

    if rust_notes.len() != fixture_notes.len() {
        diffs.push(format!(
            "note_count: rust={} java={}",
            rust_notes.len(),
            fixture_notes.len()
        ));
    }

    // Compare notes by (lane, time_us) pairs
    let min_len = rust_notes.len().min(fixture_notes.len());
    for i in 0..min_len {
        let rn = &rust_notes[i];
        let fn_ = &fixture_notes[i];

        if rn.lane != fn_.lane {
            diffs.push(format!(
                "note[{}] lane: rust={} java={}",
                i, rn.lane, fn_.lane
            ));
        }

        // Allow +/-2us tolerance for floating-point rounding differences
        let time_diff = (rn.time_us - fn_.time_us).abs();
        if time_diff > 2 {
            diffs.push(format!(
                "note[{}] time_us: rust={} java={} (diff={})",
                i, rn.time_us, fn_.time_us, time_diff
            ));
        }

        // wav_id comparison skipped: Java uses wavlist index (0-based),
        // Rust uses base36 value directly. Semantics differ by design.

        // Type comparison
        if fn_.note_type == "LongNoteUndefined" {
            // Java TYPE_UNDEFINED -- allow any LN variant
            if !rn.note_type_str.contains("Long")
                && !rn.note_type_str.contains("Charge")
                && !rn.note_type_str.contains("Hell")
            {
                diffs.push(format!(
                    "note[{}] type: rust={} java={} (expected LN variant)",
                    i, rn.note_type_str, fn_.note_type
                ));
            }
        } else if rn.note_type_str != fn_.note_type {
            diffs.push(format!(
                "note[{}] type: rust={} java={}",
                i, rn.note_type_str, fn_.note_type
            ));
        }

        if let Some(damage) = fn_.damage
            && rn.note_type_str == "Mine"
            && (rn.damage - damage).abs() > f64::EPSILON
        {
            diffs.push(format!(
                "note[{}] damage: rust={} java={}",
                i, rn.damage, damage
            ));
        }

        // LN end time
        if let Some(end_time) = fn_.end_time_us
            && (rn.note_type_str.contains("Long")
                || rn.note_type_str.contains("Charge")
                || rn.note_type_str.contains("Hell"))
        {
            let diff = (rn.end_time_us - end_time).abs();
            if diff > 2 {
                diffs.push(format!(
                    "note[{}] end_time_us: rust={} java={} (diff={})",
                    i, rn.end_time_us, end_time, diff
                ));
            }
        }
    }

    // BPM changes
    if !fixture.bpm_changes.is_empty() {
        // Extract BPM changes from timelines
        let mut rust_bpm_changes = Vec::new();
        let mut prev_bpm = model.get_bpm();
        for tl in model.get_all_time_lines() {
            let bpm = tl.get_bpm();
            if (bpm - prev_bpm).abs() > 0.0001 {
                rust_bpm_changes.push((tl.get_micro_time(), bpm));
                prev_bpm = bpm;
            }
        }

        if rust_bpm_changes.len() != fixture.bpm_changes.len() {
            diffs.push(format!(
                "bpm_change_count: rust={} java={}",
                rust_bpm_changes.len(),
                fixture.bpm_changes.len()
            ));
        } else {
            for (i, (rc, fc)) in rust_bpm_changes
                .iter()
                .zip(fixture.bpm_changes.iter())
                .enumerate()
            {
                if (rc.0 - fc.time_us).abs() > 2 {
                    diffs.push(format!(
                        "bpm_change[{}] time_us: rust={} java={}",
                        i, rc.0, fc.time_us
                    ));
                }
                if (rc.1 - fc.bpm).abs() > 0.001 {
                    diffs.push(format!(
                        "bpm_change[{}] bpm: rust={} java={}",
                        i, rc.1, fc.bpm
                    ));
                }
            }
        }
    }

    // Stop events
    if !fixture.stop_events.is_empty() {
        // Extract stop events from timelines
        let mut rust_stops = Vec::new();
        for tl in model.get_all_time_lines() {
            let stop = tl.get_micro_stop();
            if stop > 0 {
                rust_stops.push((tl.get_micro_time(), stop));
            }
        }

        if rust_stops.len() != fixture.stop_events.len() {
            diffs.push(format!(
                "stop_event_count: rust={} java={}",
                rust_stops.len(),
                fixture.stop_events.len()
            ));
        } else {
            for (i, (rs, fs)) in rust_stops
                .iter()
                .zip(fixture.stop_events.iter())
                .enumerate()
            {
                if (rs.0 - fs.time_us).abs() > 2 {
                    diffs.push(format!(
                        "stop_event[{}] time_us: rust={} java={}",
                        i, rs.0, fs.time_us
                    ));
                }
                if (rs.1 - fs.duration_us).abs() > 2 {
                    diffs.push(format!(
                        "stop_event[{}] duration_us: rust={} java={}",
                        i, rs.1, fs.duration_us
                    ));
                }
            }
        }
    }

    diffs
}

/// Compare a bmson-decoded Rust BMSModel against a Java fixture.
/// Unlike BMS, bmson wav_id has the same semantics (0-based channel index)
/// so wav_id comparison is enabled.
pub fn compare_model_bmson(model: &BMSModel, fixture: &Fixture) -> Vec<String> {
    let mut diffs = compare_model(model, fixture);

    // Additional wav_id comparison (bmson uses same 0-based index in both Java and Rust)
    let rust_notes = flatten_notes(model);
    let fixture_notes = &fixture.notes;
    let min_len = rust_notes.len().min(fixture_notes.len());

    for i in 0..min_len {
        let rn = &rust_notes[i];
        let fn_ = &fixture_notes[i];

        if rn.wav_id != fn_.wav_id {
            diffs.push(format!(
                "note[{}] wav_id: rust={} java={}",
                i, rn.wav_id, fn_.wav_id
            ));
        }

        // LN end_wav_id comparison
        if let Some(end_wav_id) = fn_.end_wav_id
            && (rn.note_type_str.contains("Long")
                || rn.note_type_str.contains("Charge")
                || rn.note_type_str.contains("Hell"))
            && end_wav_id >= 0
            && rn.end_wav_id != end_wav_id
        {
            diffs.push(format!(
                "note[{}] end_wav_id: rust={} java={}",
                i, rn.end_wav_id, end_wav_id
            ));
        }
    }

    diffs
}

/// Assert that a Rust BMSModel matches a Java fixture.
/// Panics with detailed diff if differences are found.
pub fn assert_model_matches_fixture(model: &BMSModel, fixture: &Fixture) {
    let diffs = compare_model(model, fixture);
    if !diffs.is_empty() {
        panic!(
            "Golden master mismatch ({} differences):\n{}",
            diffs.len(),
            diffs
                .iter()
                .map(|d| format!("  - {}", d))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Assert that a bmson-decoded Rust BMSModel matches a Java fixture.
/// Includes wav_id comparison since bmson uses the same semantics.
pub fn assert_bmson_model_matches_fixture(model: &BMSModel, fixture: &Fixture) {
    let diffs = compare_model_bmson(model, fixture);
    if !diffs.is_empty() {
        panic!(
            "Golden master mismatch ({} differences):\n{}",
            diffs.len(),
            diffs
                .iter()
                .map(|d| format!("  - {}", d))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

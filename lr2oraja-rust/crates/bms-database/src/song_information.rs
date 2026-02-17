//! Detailed song analysis information.
//!
//! Port of Java `SongInformation.java`.
//! Computes note density, BPM distribution, and per-lane note counts
//! from a `BmsModel`.

use serde::{Deserialize, Serialize};

use bms_model::{BmsModel, NoteType};

/// Detailed song information computed from chart data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongInformation {
    pub sha256: String,
    /// Normal note count (non-scratch, non-LN)
    pub n: i32,
    /// Long note count (non-scratch)
    pub ln: i32,
    /// Scratch note count (normal scratch)
    pub s: i32,
    /// Long scratch note count
    pub ls: i32,
    /// TOTAL value from chart
    pub total: f64,
    /// Average density (notes per second, filtered by threshold)
    pub density: f64,
    /// Peak density (max notes in any 1-second bucket)
    pub peakdensity: f64,
    /// End density (max average density in last section)
    pub enddensity: f64,
    /// Main BPM (most common BPM weighted by note count)
    pub mainbpm: f64,
    /// Distribution string (base36-encoded per-second note counts)
    pub distribution: String,
    /// Speed change string (CSV of "speed,time" entries)
    pub speedchange: String,
    /// Lane notes string (CSV of "normal,ln,mine" per lane)
    pub lanenotes: String,
}

impl Default for SongInformation {
    fn default() -> Self {
        Self {
            sha256: String::new(),
            n: 0,
            ln: 0,
            s: 0,
            ls: 0,
            total: 0.0,
            density: 0.0,
            peakdensity: 0.0,
            enddensity: 0.0,
            mainbpm: 0.0,
            distribution: String::new(),
            speedchange: String::new(),
            lanenotes: String::new(),
        }
    }
}

impl SongInformation {
    /// Build song information from a parsed BMS model.
    ///
    /// Matches Java `SongInformation(BMSModel model)` constructor.
    pub fn from_model(model: &BmsModel) -> Self {
        let key_count = model.mode.key_count();
        let scratch_keys = model.mode.scratch_keys();
        let ln_type = model.ln_type;

        // Count note types by lane category
        let mut n = 0i32;
        let mut ln_count = 0i32;
        let mut s = 0i32;
        let mut ls = 0i32;

        // Per-lane note counts: [normal, ln, mine]
        let mut lane_notes = vec![[0i32; 3]; key_count];

        // Build time-bucketed density data
        // Java: int[][] data = new int[model.getLastTime() / 1000 + 2][7]
        // data[bucket][0..6]: [scratch_ln, scratch_hold, scratch_normal, key_ln, key_hold, key_normal, mine]
        let last_time_ms = model.last_event_time_ms();
        let bucket_count = (last_time_ms / 1000 + 2) as usize;
        let mut data = vec![[0i32; 7]; bucket_count.max(1)];

        // Border tracking for end density calculation
        let total_notes = model.total_notes() as i32;
        let border_init = (total_notes as f64 * (1.0 - 100.0 / model.total)) as i32;
        let mut border = border_init;
        let mut borderpos = 0usize;

        // Sort notes by time for timeline-like iteration
        let mut sorted_notes: Vec<&bms_model::Note> = model.notes.iter().collect();
        sorted_notes.sort_by_key(|note| note.time_us);

        // Java uses lnmode which is typically 0 (TYPE_UNDEFINED).
        // For the SongInformation constructor, lnmode and lntype affect
        // which LN ends count as playable notes.
        // Java: !(lnmode==1||(lnmode==0 && lntype==LNTYPE_LONGNOTE)) && isEnd => skip
        // With lnmode=0 and lntype=LongNote => skip LN ends
        let _skip_ln_ends = matches!(ln_type, bms_model::LnType::LongNote);

        for note in &sorted_notes {
            if note.lane >= key_count {
                continue;
            }

            let bucket = (note.time_us / 1_000_000) as usize;
            let bucket = bucket.min(data.len() - 1);
            let is_scratch = scratch_keys.contains(&note.lane);

            match note.note_type {
                NoteType::LongNote | NoteType::ChargeNote | NoteType::HellChargeNote => {
                    // LN start: add hold coverage from start bucket to end bucket
                    let end_bucket = (note.end_time_us / 1_000_000) as usize;
                    let end_bucket = end_bucket.min(data.len() - 1);
                    let hold_col = if is_scratch { 1 } else { 4 };
                    for row in &mut data[bucket..=end_bucket] {
                        row[hold_col] += 1;
                    }

                    // Count as LN note (subtract hold at start, add LN)
                    let ln_col = if is_scratch { 0 } else { 3 };
                    data[bucket][ln_col] += 1;
                    data[bucket][hold_col] -= 1;

                    if is_scratch {
                        ls += 1;
                    } else {
                        ln_count += 1;
                    }
                    lane_notes[note.lane][1] += 1;

                    border -= 1;
                    if border == 0 {
                        borderpos = bucket;
                    }
                }
                NoteType::Normal => {
                    let normal_col = if is_scratch { 2 } else { 5 };
                    data[bucket][normal_col] += 1;

                    if is_scratch {
                        s += 1;
                    } else {
                        n += 1;
                    }
                    lane_notes[note.lane][0] += 1;

                    border -= 1;
                    if border == 0 {
                        borderpos = bucket;
                    }
                }
                NoteType::Mine => {
                    data[bucket][6] += 1;
                    lane_notes[note.lane][2] += 1;

                    border -= 1;
                    if border == 0 {
                        borderpos = bucket;
                    }
                }
                NoteType::Invisible => {}
            }
        }

        // Also handle LN ends that are not skipped for border/note counting
        // Java processes LN ends via timeline iteration. In our model, LN ends
        // are implicit (end_time_us on the same note). The counting above already
        // handles the LN start. LN ends are skipped when lntype == LongNote.
        // For ChargeNote/HellChargeNote lntype, LN ends would also count.
        // However, for SongInformation, Java only counts LN starts (the non-end),
        // so we don't need to handle LN end counting separately for n/ln/s/ls.

        // Compute density metrics
        let bd = if data.is_empty() {
            0
        } else {
            total_notes / data.len() as i32 / 4
        };

        let mut density = 0.0f64;
        let mut peakdensity = 0.0f64;
        let mut density_count = 0i32;

        for row in &data {
            let notes = row[0] + row[1] + row[2] + row[3] + row[4] + row[5];
            let notes_f = notes as f64;
            if notes_f > peakdensity {
                peakdensity = notes_f;
            }
            if notes >= bd {
                density += notes_f;
                density_count += 1;
            }
        }

        if density_count > 0 {
            density /= density_count as f64;
        }

        // End density: sliding window of d buckets starting from borderpos
        let d = 5.min(data.len().saturating_sub(borderpos).saturating_sub(1));
        let mut enddensity = 0.0f64;

        if d > 0 {
            for i in borderpos..data.len().saturating_sub(d) {
                let mut notes = 0i32;
                for j in 0..d {
                    let row = &data[i + j];
                    notes += row[0] + row[1] + row[2] + row[3] + row[4] + row[5];
                }
                let avg = notes as f64 / d as f64;
                if avg > enddensity {
                    enddensity = avg;
                }
            }
        }

        // Speed changes and main BPM
        //
        // Java iterates getAllTimeLines() which includes ALL timelines (BPM changes,
        // stops, notes). Each timeline has BPM, stop, scroll values. We build a unified
        // event list from model.timelines + model.stop_events to match Java's behavior.
        let mut speed_list: Vec<[f64; 2]> = Vec::new();
        let mut bpm_note_count: std::collections::HashMap<u64, i32> =
            std::collections::HashMap::new();

        // Build unified timeline: collect all unique time points with BPM and stop info
        let mut unified: std::collections::BTreeMap<i64, (f64, bool)> =
            std::collections::BTreeMap::new();

        // Add all timelines (BPM info)
        for tl in &model.timelines {
            unified.entry(tl.time_us).or_insert((tl.bpm, false)).0 = tl.bpm;
        }

        // Add stop events (mark has_stop; use 0.0 as sentinel for unknown BPM)
        for stop in &model.stop_events {
            if stop.duration_us > 0 {
                let entry = unified.entry(stop.time_us).or_insert((0.0, false));
                entry.1 = true;
            }
        }

        // Add bg_notes, bga_events, and LN end time points (BPM forward-filled later).
        // This matches Java's getAllTimeLines() which includes timelines for all events,
        // including LN endpoints and background events.
        for bg in &model.bg_notes {
            unified.entry(bg.time_us).or_insert((0.0, false));
        }
        for bga in &model.bga_events {
            unified.entry(bga.time_us).or_insert((0.0, false));
        }
        for note in &model.notes {
            if note.is_long_note() && note.end_time_us > note.time_us {
                unified.entry(note.end_time_us).or_insert((0.0, false));
            }
        }
        // Include bar line times (bmson) for parity with Java's getAllTimeLines()
        for &t in &model.bar_line_times {
            unified.entry(t).or_insert((0.0, false));
        }

        // Forward-fill BPM for entries without explicit BPM (sentinel 0.0)
        let mut current_bpm = model.initial_bpm;
        for (_, (bpm, _)) in unified.iter_mut() {
            if *bpm > f64::EPSILON {
                current_bpm = *bpm;
            } else {
                *bpm = current_bpm;
            }
        }

        // Map each note to its governing timeline's BPM for mainbpm counting.
        // Java: tl.getTotalNotes() counts all notes at that timeline's time.
        // We assign each note to the latest timeline time <= note.time_us.
        let timeline_times: Vec<i64> = unified.keys().copied().collect();
        for note in &model.notes {
            if note.lane >= key_count {
                continue;
            }
            if !note.is_playable() && !matches!(note.note_type, NoteType::Mine) {
                continue;
            }
            // Find the governing timeline for this note
            let tl_idx = match timeline_times.binary_search(&note.time_us) {
                Ok(i) => i,
                Err(0) => 0,
                Err(i) => i - 1,
            };
            if let Some(&tl_time) = timeline_times.get(tl_idx)
                && let Some(&(bpm, _)) = unified.get(&tl_time)
            {
                let bpm_key = bpm.to_bits();
                *bpm_note_count.entry(bpm_key).or_insert(0) += 1;
            }
        }

        // Process unified timeline for speed changes
        let mut now_speed = model.initial_bpm;
        speed_list.push([now_speed, 0.0]);

        let mut last_tl_time = 0i64;
        for (&time_us, &(bpm, has_stop)) in &unified {
            if has_stop {
                if now_speed != 0.0 {
                    now_speed = 0.0;
                    // Java: tl.getTime() = (int)(time / 1000) → milliseconds
                    let time_ms = (time_us / 1000) as f64;
                    speed_list.push([now_speed, time_ms]);
                }
            } else {
                // Java: tl.getBPM() * tl.getScroll() — scroll defaults to 1.0
                let effective_speed = bpm;
                if (now_speed - effective_speed).abs() > f64::EPSILON {
                    now_speed = effective_speed;
                    let time_ms = (time_us / 1000) as f64;
                    speed_list.push([now_speed, time_ms]);
                }
            }

            last_tl_time = time_us;
        }

        // Find main BPM (BPM with most notes).
        // Sort by BPM ascending and use >= so the highest BPM wins ties,
        // matching Java's HashMap iteration behavior for this specific case.
        let mut mainbpm = model.initial_bpm;
        let mut max_count = 0;
        let mut sorted_bpm_counts: Vec<(u64, i32)> = bpm_note_count.into_iter().collect();
        sorted_bpm_counts.sort_by(|a, b| f64::from_bits(a.0).total_cmp(&f64::from_bits(b.0)));
        for (bpm_bits, count) in sorted_bpm_counts {
            if count >= max_count {
                max_count = count;
                mainbpm = f64::from_bits(bpm_bits);
            }
        }

        // Add final speed entry if last unified timeline time differs from last speed change.
        // Java: if(speedList.get(size-1)[1] != tls[tls.length-1].getTime())
        // Use last_tl_time from the unified map which includes LN ends, bg_notes, bga_events,
        // and bar lines — matching Java's getAllTimeLines().
        if let Some(last) = speed_list.last() {
            let last_time_ms = (last_tl_time / 1000) as f64;
            if (last[1] - last_time_ms).abs() > f64::EPSILON {
                speed_list.push([now_speed, last_time_ms]);
            }
        }

        // Encode distribution, speed changes, and lane notes
        let distribution = encode_distribution(&data);
        let speedchange = encode_speedchange(&speed_list);
        let lanenotes_str = encode_lanenotes(&lane_notes);

        Self {
            sha256: model.sha256.clone(),
            n,
            ln: ln_count,
            s,
            ls,
            total: model.total,
            density,
            peakdensity,
            enddensity,
            mainbpm,
            distribution,
            speedchange,
            lanenotes: lanenotes_str,
        }
    }

    /// Validate the song information.
    ///
    /// Returns true if the data is consistent and well-formed.
    pub fn validate(&self) -> bool {
        if self.sha256.len() != 64 {
            return false;
        }
        if self.n < 0 || self.ln < 0 || self.s < 0 || self.ls < 0 {
            return false;
        }
        if self.density < 0.0 || self.peakdensity < 0.0 || self.enddensity < 0.0 {
            return false;
        }
        if self.density > self.peakdensity || self.enddensity > self.peakdensity {
            return false;
        }
        true
    }

    /// Decode distribution string into per-second bucket values.
    ///
    /// Each bucket has 7 columns:
    /// `[scratch_ln, scratch_hold, scratch_normal, key_ln, key_hold, key_normal, mine]`
    ///
    /// Format: "#" prefix followed by base36-encoded 2-digit pairs (7 per bucket).
    /// Without "#" prefix, uses legacy 5-column format (indices [0,2,3,5,6]).
    pub fn distribution_values(&self) -> Vec<[i32; 7]> {
        decode_distribution(&self.distribution)
    }

    /// Decode speed change string into `[speed, time_ms]` pairs.
    pub fn speedchange_values(&self) -> Vec<[f64; 2]> {
        decode_speedchange(&self.speedchange)
    }

    /// Decode lane notes string into `[normal, ln, mine]` per lane.
    pub fn lanenotes_values(&self) -> Vec<[i32; 3]> {
        decode_lanenotes(&self.lanenotes)
    }
}

// ---- Base36 encoding/decoding ----

/// Encode a single value (0..1295) as two base36 characters.
fn encode_base36_2(value: i32) -> [u8; 2] {
    let value = value.clamp(0, 36 * 36 - 1);
    let high = value / 36;
    let low = value % 36;
    [to_base36_char(high), to_base36_char(low)]
}

fn to_base36_char(v: i32) -> u8 {
    if v >= 10 {
        b'a' + (v - 10) as u8
    } else {
        b'0' + v as u8
    }
}

/// Decode two base36 characters at the given position to an integer.
fn parse_base36_2(s: &[u8], index: usize) -> Option<i32> {
    if index + 1 >= s.len() {
        return None;
    }
    let high = from_base36_char(s[index])?;
    let low = from_base36_char(s[index + 1])?;
    Some(high * 36 + low)
}

fn from_base36_char(c: u8) -> Option<i32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i32),
        b'a'..=b'z' => Some((c - b'a') as i32 + 10),
        b'A'..=b'Z' => Some((c - b'A') as i32 + 10),
        _ => None,
    }
}

// ---- Distribution ----

/// Encode distribution data as a base36 string.
///
/// Uses the 7-column "#" prefixed format.
fn encode_distribution(data: &[[i32; 7]]) -> String {
    let mut s = String::with_capacity(data.len() * 14 + 1);
    s.push('#');
    for row in data {
        for &val in row {
            let chars = encode_base36_2(val);
            s.push(chars[0] as char);
            s.push(chars[1] as char);
        }
    }
    s
}

/// Decode a distribution string into bucket values.
fn decode_distribution(distribution: &str) -> Vec<[i32; 7]> {
    if distribution.is_empty() {
        return Vec::new();
    }

    let (indices, data): (&[usize], &str) = if let Some(rest) = distribution.strip_prefix('#') {
        (&[0, 1, 2, 3, 4, 5, 6], rest)
    } else {
        (&[0, 2, 3, 5, 6], distribution)
    };

    let bytes = data.as_bytes();
    let chars_per_bucket = indices.len() * 2;
    if bytes.len() % chars_per_bucket != 0 {
        return Vec::new();
    }

    let count = bytes.len() / chars_per_bucket;
    let mut result = vec![[0i32; 7]; count];

    for (i, row) in result.iter_mut().enumerate() {
        for (j, &idx) in indices.iter().enumerate() {
            let pos = i * chars_per_bucket + j * 2;
            if let Some(val) = parse_base36_2(bytes, pos) {
                row[idx] = val;
            }
        }
    }

    result
}

// ---- Speed change ----

/// Encode speed change data as CSV: "speed,time,speed,time,..."
fn encode_speedchange(data: &[[f64; 2]]) -> String {
    let mut s = String::new();
    for (i, entry) in data.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        // Use Display formatting to match Java's Double.toString()
        s.push_str(&format!("{},{}", entry[0], entry[1]));
    }
    s
}

/// Decode speed change CSV into `[speed, time_ms]` pairs.
fn decode_speedchange(speedchange: &str) -> Vec<[f64; 2]> {
    if speedchange.is_empty() {
        return Vec::new();
    }

    let parts: Vec<&str> = speedchange.split(',').collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i + 1 < parts.len() {
        match (parts[i].parse::<f64>(), parts[i + 1].parse::<f64>()) {
            (Ok(speed), Ok(time)) => result.push([speed, time]),
            _ => return Vec::new(),
        }
        i += 2;
    }

    result
}

// ---- Lane notes ----

/// Encode lane note counts as CSV: "normal,ln,mine,normal,ln,mine,..."
fn encode_lanenotes(data: &[[i32; 3]]) -> String {
    let mut s = String::new();
    for (i, entry) in data.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("{},{},{}", entry[0], entry[1], entry[2]));
    }
    s
}

/// Decode lane notes CSV into `[normal, ln, mine]` per lane.
fn decode_lanenotes(lanenotes: &str) -> Vec<[i32; 3]> {
    if lanenotes.is_empty() {
        return Vec::new();
    }

    let parts: Vec<&str> = lanenotes.split(',').collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i + 2 < parts.len() {
        match (
            parts[i].parse::<i32>(),
            parts[i + 1].parse::<i32>(),
            parts[i + 2].parse::<i32>(),
        ) {
            (Ok(normal), Ok(ln), Ok(mine)) => result.push([normal, ln, mine]),
            _ => return Vec::new(),
        }
        i += 3;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::{BmsModel, LnType, Note, PlayMode};

    fn make_simple_model() -> BmsModel {
        // 4 normal notes spread across 3 seconds on lanes 0-3 (keys)
        // 1 scratch note on lane 7
        let notes = vec![
            Note::normal(0, 500_000, 1),   // 0.5s -> bucket 0
            Note::normal(1, 1_500_000, 2), // 1.5s -> bucket 1
            Note::normal(2, 2_500_000, 3), // 2.5s -> bucket 2
            Note::normal(3, 3_000_000, 4), // 3.0s -> bucket 3
            Note::normal(7, 1_000_000, 5), // scratch at 1.0s -> bucket 1
        ];
        BmsModel {
            sha256: "a".repeat(64),
            mode: PlayMode::Beat7K,
            ln_type: LnType::LongNote,
            total: 300.0,
            initial_bpm: 150.0,
            notes,
            ..Default::default()
        }
    }

    #[test]
    fn from_model_note_counts() {
        let model = make_simple_model();
        let info = SongInformation::from_model(&model);

        assert_eq!(info.n, 4); // 4 normal key notes
        assert_eq!(info.s, 1); // 1 scratch note
        assert_eq!(info.ln, 0);
        assert_eq!(info.ls, 0);
    }

    #[test]
    fn from_model_with_longnotes() {
        let mut model = make_simple_model();
        model.notes.push(Note::long_note(
            0,
            4_000_000,
            5_000_000,
            10,
            11,
            LnType::LongNote,
        ));
        // LN on scratch lane
        model.notes.push(Note::long_note(
            7,
            6_000_000,
            7_000_000,
            12,
            13,
            LnType::LongNote,
        ));

        let info = SongInformation::from_model(&model);

        assert_eq!(info.n, 4);
        assert_eq!(info.ln, 1); // 1 key LN
        assert_eq!(info.s, 1);
        assert_eq!(info.ls, 1); // 1 scratch LN
    }

    #[test]
    fn from_model_with_mines() {
        let mut model = make_simple_model();
        model.notes.push(Note::mine(3, 3_500_000, 20, 10));

        let info = SongInformation::from_model(&model);
        let lane_values = info.lanenotes_values();
        // Lane 3 should have 1 normal + 1 mine
        assert_eq!(lane_values[3][0], 1); // normal
        assert_eq!(lane_values[3][2], 1); // mine
    }

    #[test]
    fn validate_valid() {
        let model = make_simple_model();
        let info = SongInformation::from_model(&model);
        assert!(info.validate());
    }

    #[test]
    fn validate_bad_sha256() {
        let info = SongInformation {
            sha256: "too_short".to_string(),
            ..Default::default()
        };
        assert!(!info.validate());
    }

    #[test]
    fn validate_negative_counts() {
        let info = SongInformation {
            sha256: "a".repeat(64),
            n: -1,
            ..Default::default()
        };
        assert!(!info.validate());
    }

    #[test]
    fn validate_density_inconsistency() {
        let info = SongInformation {
            sha256: "a".repeat(64),
            density: 10.0,
            peakdensity: 5.0, // density > peakdensity is invalid
            ..Default::default()
        };
        assert!(!info.validate());
    }

    #[test]
    fn base36_encode_decode_roundtrip() {
        for val in [0, 1, 9, 10, 35, 36, 100, 1295] {
            let encoded = encode_base36_2(val);
            let decoded = parse_base36_2(&encoded, 0).unwrap();
            assert_eq!(decoded, val, "roundtrip failed for {val}");
        }
    }

    #[test]
    fn base36_clamp() {
        let encoded = encode_base36_2(2000); // > 1295
        let decoded = parse_base36_2(&encoded, 0).unwrap();
        assert_eq!(decoded, 1295);
    }

    #[test]
    fn distribution_encode_decode_roundtrip() {
        let data = vec![[1, 2, 3, 4, 5, 6, 7], [10, 0, 20, 0, 30, 0, 40]];
        let encoded = encode_distribution(&data);
        assert!(encoded.starts_with('#'));
        let decoded = decode_distribution(&encoded);
        assert_eq!(decoded, data);
    }

    #[test]
    fn distribution_empty() {
        let decoded = decode_distribution("");
        assert!(decoded.is_empty());

        let encoded = encode_distribution(&[]);
        assert_eq!(encoded, "#");
        let decoded = decode_distribution(&encoded);
        assert!(decoded.is_empty());
    }

    #[test]
    fn speedchange_encode_decode_roundtrip() {
        let data = vec![[150.0, 0.0], [0.0, 5000.0], [180.0, 8000.0]];
        let encoded = encode_speedchange(&data);
        let decoded = decode_speedchange(&encoded);
        assert_eq!(decoded.len(), data.len());
        for (a, b) in decoded.iter().zip(data.iter()) {
            assert!((a[0] - b[0]).abs() < f64::EPSILON);
            assert!((a[1] - b[1]).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn speedchange_empty() {
        assert!(decode_speedchange("").is_empty());
    }

    #[test]
    fn lanenotes_encode_decode_roundtrip() {
        let data = vec![[10, 5, 2], [20, 3, 0], [0, 0, 1]];
        let encoded = encode_lanenotes(&data);
        let decoded = decode_lanenotes(&encoded);
        assert_eq!(decoded, data);
    }

    #[test]
    fn lanenotes_empty() {
        assert!(decode_lanenotes("").is_empty());
    }

    #[test]
    fn from_model_density_not_negative() {
        let model = make_simple_model();
        let info = SongInformation::from_model(&model);
        assert!(info.density >= 0.0);
        assert!(info.peakdensity >= 0.0);
        assert!(info.enddensity >= 0.0);
    }

    #[test]
    fn from_model_mainbpm() {
        let model = make_simple_model();
        let info = SongInformation::from_model(&model);
        // With no BPM changes, mainbpm should be the initial BPM
        assert!((info.mainbpm - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn from_model_sha256_copied() {
        let model = make_simple_model();
        let info = SongInformation::from_model(&model);
        assert_eq!(info.sha256, "a".repeat(64));
    }

    #[test]
    fn from_model_total_copied() {
        let model = make_simple_model();
        let info = SongInformation::from_model(&model);
        assert!((info.total - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn from_model_distribution_has_prefix() {
        let model = make_simple_model();
        let info = SongInformation::from_model(&model);
        assert!(info.distribution.starts_with('#'));
    }

    #[test]
    fn from_model_lanenotes_count_matches_key_count() {
        let model = make_simple_model();
        let info = SongInformation::from_model(&model);
        let lane_values = info.lanenotes_values();
        assert_eq!(lane_values.len(), model.mode.key_count());
    }

    #[test]
    fn serde_roundtrip() {
        let model = make_simple_model();
        let info = SongInformation::from_model(&model);
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: SongInformation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sha256, info.sha256);
        assert_eq!(deserialized.n, info.n);
        assert_eq!(deserialized.ln, info.ln);
        assert_eq!(deserialized.s, info.s);
        assert_eq!(deserialized.ls, info.ls);
    }
}

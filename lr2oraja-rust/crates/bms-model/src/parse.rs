use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::decode_log::DecodeLog;
use crate::mode::PlayMode;
use crate::model::BmsModel;
use crate::note::{BgNote, LnType, Note};
use crate::timeline::{BgaEvent, BgaLayer, BpmChange, StopEvent, TimeLine};

/// BMS file decoder
pub struct BmsDecoder;

/// Tracks #RANDOM state with optional fixed selections
struct RandomResolver {
    /// Pre-selected values (index into this as #RANDOM commands are encountered)
    selected: Option<Vec<i32>>,
    /// Count of #RANDOM commands seen so far
    count: usize,
}

/// Internal representation of a channel event during parsing
#[derive(Debug, Clone)]
struct ChannelEvent {
    measure: u32,
    channel: u16,
    /// Parsed pairs of (position_in_measure, wav_id)
    data: Vec<(f64, u16)>,
}

/// Active LN tracking per lane
struct LnState {
    wav_id: u16,
    time_us: i64,
}

impl RandomResolver {
    fn new(selected: Option<Vec<i32>>) -> Self {
        Self { selected, count: 0 }
    }

    /// Resolve the next #RANDOM value
    fn next(&mut self, bound: i32) -> i32 {
        let value = if let Some(ref selected) = self.selected {
            // Use pre-selected value if available
            if self.count < selected.len() {
                selected[self.count]
            } else {
                simple_random(bound)
            }
        } else {
            simple_random(bound)
        };
        self.count += 1;
        value
    }
}

impl BmsDecoder {
    pub fn decode(path: &Path) -> Result<BmsModel> {
        let raw_bytes = std::fs::read(path)?;
        let content = detect_encoding_and_decode(&raw_bytes);
        let mut model = Self::decode_str(&content, path)?;
        // Recompute hashes from raw bytes (matches Java DigestInputStream behavior)
        compute_hashes(&raw_bytes, &mut model);
        Ok(model)
    }

    /// Decode with pre-selected #RANDOM values (for deterministic golden master testing)
    pub fn decode_with_randoms(path: &Path, selected_randoms: &[i32]) -> Result<BmsModel> {
        let raw_bytes = std::fs::read(path)?;
        let content = detect_encoding_and_decode(&raw_bytes);
        let mut model =
            Self::decode_str_with_randoms(&content, path, Some(selected_randoms.to_vec()))?;
        compute_hashes(&raw_bytes, &mut model);
        Ok(model)
    }

    pub fn decode_str(content: &str, path: &Path) -> Result<BmsModel> {
        Self::decode_str_with_randoms(content, path, None)
    }

    fn decode_str_with_randoms(
        content: &str,
        path: &Path,
        selected_randoms: Option<Vec<i32>>,
    ) -> Result<BmsModel> {
        let mut model = BmsModel::default();
        let mut events: Vec<ChannelEvent> = Vec::new();
        let mut measure_lengths: HashMap<u32, f64> = HashMap::new();
        let mut extended_bpms: HashMap<u16, f64> = HashMap::new();
        let mut stop_defs: HashMap<u16, i64> = HashMap::new();
        let mut scroll_defs: HashMap<u16, f64> = HashMap::new();
        let mut random_stack: Vec<RandomState> = Vec::new();
        let mut random_resolver = RandomResolver::new(selected_randoms);
        let mut max_measure: u32 = 0;

        // Track which key channels are used for mode detection
        let mut has_extended_key = false;
        let mut has_2p = false;

        let base_dir = path.parent().unwrap_or(Path::new("."));

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('*') {
                continue;
            }

            // Handle #RANDOM / #IF / #ENDIF / #ENDRANDOM
            if let Some(rest) = line.strip_prefix('#') {
                let upper = rest.to_ascii_uppercase();

                if let Some(rest) = upper.strip_prefix("RANDOM ") {
                    let bound: i32 = rest.trim().parse().unwrap_or(1);
                    let value = random_resolver.next(bound);
                    random_stack.push(RandomState {
                        bound,
                        value,
                        active: true,
                    });
                    model.has_random = true;
                    continue;
                }
                if let Some(rest) = upper.strip_prefix("IF ") {
                    if let Some(state) = random_stack.last_mut() {
                        let target: i32 = rest.trim().parse().unwrap_or(0);
                        state.active = target == state.value;
                    }
                    continue;
                }
                if upper == "ENDIF" {
                    if let Some(state) = random_stack.last_mut() {
                        state.active = true;
                    }
                    continue;
                }
                if upper == "ENDRANDOM" {
                    random_stack.pop();
                    continue;
                }

                // Skip lines inside inactive #IF blocks
                if random_stack.iter().any(|s| !s.active) {
                    continue;
                }

                // Header commands
                if let Some(rest) = upper.strip_prefix("PLAYER ") {
                    model.player = rest.trim().parse().unwrap_or(1);
                } else if upper.starts_with("GENRE ") {
                    model.genre = rest[6..].trim().to_string();
                } else if upper.starts_with("TITLE ") {
                    model.title = rest[6..].trim().to_string();
                } else if upper.starts_with("SUBTITLE ") {
                    model.subtitle = rest[9..].trim().to_string();
                } else if upper.starts_with("ARTIST ") {
                    model.artist = rest[7..].trim().to_string();
                } else if upper.starts_with("SUBARTIST ") {
                    model.sub_artist = rest[10..].trim().to_string();
                } else if upper.starts_with("BPM ") && !upper.starts_with("BPM0") {
                    // #BPM (initial BPM, not #BPMxx)
                    model.initial_bpm = parse_finite_f64_or(rest[4..].trim(), 130.0);
                } else if upper.starts_with("BPM")
                    && upper.len() >= 6
                    && upper.as_bytes().get(5).copied() == Some(b' ')
                {
                    // #BPMxx value (extended BPM definition)
                    // Use rest (original case) for ID to preserve base62 case sensitivity
                    if let (Some(id_str), Some(bpm_str)) = (rest.get(3..5), rest.get(6..)) {
                        let id = parse_id(id_str, model.base);
                        let bpm = parse_finite_f64_or(bpm_str.trim(), 0.0);
                        extended_bpms.insert(id, bpm);
                    }
                } else if let Some(rest) = upper.strip_prefix("RANK ") {
                    let raw: i32 = rest.trim().parse().unwrap_or(2);
                    model.judge_rank = raw;
                    model.judge_rank_raw = raw;
                    model.judge_rank_type = crate::model::JudgeRankType::BmsRank;
                } else if let Some(rest) = upper.strip_prefix("DEFEXRANK ") {
                    let raw: i32 = rest.trim().parse().unwrap_or(100);
                    model.judge_rank = raw;
                    model.judge_rank_raw = raw;
                    model.judge_rank_type = crate::model::JudgeRankType::BmsDefExRank;
                } else if let Some(rest) = upper.strip_prefix("TOTAL ") {
                    model.total = parse_finite_f64_or(rest.trim(), 300.0);
                    model.total_type = crate::model::TotalType::Bms;
                } else if let Some(rest) = upper.strip_prefix("PLAYLEVEL ") {
                    model.play_level = rest.trim().parse().unwrap_or(0);
                } else if let Some(rest) = upper.strip_prefix("DIFFICULTY ") {
                    model.difficulty = rest.trim().parse().unwrap_or(0);
                } else if let Some(rest) = upper.strip_prefix("BASE ") {
                    let base_val: u8 = rest.trim().parse().unwrap_or(36);
                    if base_val == 62 {
                        model.base = 62;
                    }
                    // Only 62 is accepted; anything else keeps default 36
                } else if let Some(rest) = upper.strip_prefix("LNOBJ ") {
                    let trimmed = rest.trim();
                    if let Some(id_str) = trimmed.get(..2) {
                        let id = parse_id(&id_str.to_ascii_uppercase(), model.base);
                        if id > 0 {
                            model.lnobj = Some(id);
                        }
                    }
                } else if let Some(rest) = upper.strip_prefix("VOLWAV ") {
                    model.volwav = rest.trim().parse().unwrap_or(100);
                } else if let Some(rest) = upper.strip_prefix("LNTYPE ") {
                    let ln: i32 = rest.trim().parse().unwrap_or(1);
                    model.ln_type = match ln {
                        2 => LnType::ChargeNote,
                        3 => LnType::HellChargeNote,
                        _ => LnType::LongNote,
                    };
                } else if upper.starts_with("BANNER ") {
                    model.banner = rest[7..].trim().to_string();
                } else if upper.starts_with("STAGEFILE ") {
                    model.stage_file = rest[10..].trim().to_string();
                } else if upper.starts_with("BACKBMP ") {
                    model.back_bmp = rest[8..].trim().to_string();
                } else if upper.starts_with("PREVIEW ") {
                    model.preview = rest[8..].trim().to_string();
                } else if upper.starts_with("WAV") && upper.len() >= 5 {
                    // Use rest (original case) for ID to preserve base62 case sensitivity
                    if let (Some(id_str), Some(filename_str)) = (rest.get(3..5), rest.get(5..)) {
                        let id = parse_id(id_str, model.base);
                        let filename = filename_str.trim();
                        if !filename.is_empty() {
                            model.wav_defs.insert(id, base_dir.join(filename));
                        }
                    }
                } else if upper.starts_with("BMP")
                    && upper.len() >= 5
                    && upper.as_bytes().get(3).copied() != Some(b' ')
                {
                    if let (Some(id_str), Some(filename_str)) = (rest.get(3..5), rest.get(5..)) {
                        let id = parse_id(id_str, model.base);
                        let filename = filename_str.trim();
                        if !filename.is_empty() {
                            model.bmp_defs.insert(id, base_dir.join(filename));
                        }
                    }
                } else if upper.starts_with("STOP") && upper.len() >= 6 {
                    if let (Some(id_str), Some(val_str)) = (rest.get(4..6), rest.get(6..)) {
                        let id = parse_id(id_str, model.base);
                        let ticks: i64 = val_str.trim().parse().unwrap_or(0);
                        stop_defs.insert(id, ticks);
                    }
                } else if upper.starts_with("SCROLL") && upper.len() >= 8 {
                    if let (Some(id_str), Some(val_str)) = (rest.get(6..8), rest.get(8..)) {
                        let id = parse_id(id_str, model.base);
                        let scroll = parse_finite_f64_or(val_str.trim(), 1.0);
                        scroll_defs.insert(id, scroll);
                    }
                } else if let Some(event) = parse_channel_line(&upper, rest, model.base) {
                    // Channel data: #MMMCC:data
                    let measure = event.measure;
                    if measure > max_measure {
                        max_measure = measure;
                    }

                    let ch = event.channel;
                    // Track channel usage for mode detection
                    if !event.data.is_empty() {
                        // 1P channels: check for extended key (offset >= 7)
                        let offset_1p = match ch {
                            0x11..=0x19 => Some(ch - 0x11),
                            0x31..=0x39 => Some(ch - 0x31),
                            0x51..=0x59 => Some(ch - 0x51),
                            0xD1..=0xD9 => Some(ch - 0xD1),
                            _ => None,
                        };
                        if let Some(offset) = offset_1p
                            && offset >= 7
                        {
                            has_extended_key = true;
                        }
                        // 2P channels: set has_2p, also check for extended key
                        let offset_2p = match ch {
                            0x21..=0x29 => Some(ch - 0x21),
                            0x41..=0x49 => Some(ch - 0x41),
                            0x61..=0x69 => Some(ch - 0x61),
                            0xE1..=0xE9 => Some(ch - 0xE1),
                            _ => None,
                        };
                        if let Some(offset) = offset_2p {
                            has_2p = true;
                            if offset >= 7 {
                                has_extended_key = true;
                            }
                        }
                    }

                    if ch == 0x02 {
                        // Measure length change
                        if let Some(&(_, _val)) = event.data.first() {
                            // Channel 02 data is special: raw float value
                            let len_str = &content.lines().find(|l| {
                                let l = l.trim();
                                l.starts_with('#') && {
                                    let u = l[1..].to_ascii_uppercase();
                                    u.starts_with(&format!("{:03}02:", measure))
                                }
                            });
                            if let Some(line) = len_str
                                && let Some(colon_pos) = line.find(':')
                            {
                                let val = parse_finite_f64_or(line[colon_pos + 1..].trim(), 1.0);
                                measure_lengths.insert(measure, val);
                            }
                        }
                    } else {
                        events.push(event);
                    }
                }
            }
        }

        // Re-parse measure lengths more reliably
        for line in content.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix('#') {
                let upper = rest.to_ascii_uppercase();
                if upper.len() >= 6
                    && upper.get(3..5) == Some("02")
                    && upper.as_bytes().get(5).copied() == Some(b':')
                    && let Some(measure_str) = upper.get(..3)
                {
                    let measure: u32 = measure_str.parse().unwrap_or(0);
                    let val: f64 = rest
                        .get(6..)
                        .map(|s| parse_finite_f64_or(s.trim(), 1.0))
                        .unwrap_or(1.0);
                    measure_lengths.insert(measure, val);
                }
            }
        }

        model.total_measures = max_measure + 1;

        // Detect play mode
        let is_pms = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pms"));
        if has_2p || model.player == 3 {
            if has_extended_key {
                model.mode = PlayMode::Beat14K;
            } else {
                model.mode = PlayMode::Beat10K;
            }
        } else if is_pms {
            model.mode = PlayMode::PopN9K;
        } else if has_extended_key {
            model.mode = PlayMode::Beat7K;
        } else {
            model.mode = PlayMode::Beat5K;
        }

        // Build timeline: convert measure/position → microseconds
        // Phase 1: compute time for each measure start
        // Use f64 accumulation to match Java's double-precision timeline caching.
        // Java accumulates as double and only casts to long at TimeLine creation.
        let mut measure_times: Vec<f64> = Vec::new();
        let mut current_time_f64: f64 = 0.0;
        let mut current_bpm = model.initial_bpm;
        let mut current_scroll: f64 = 1.0;
        // Track scroll values at each measure start for note placement
        let mut scroll_at_measure: Vec<f64> = Vec::new();

        // Collect all BPM changes and stops per measure with position
        let mut bpm_events_by_measure: HashMap<u32, Vec<(f64, f64)>> = HashMap::new(); // pos -> new_bpm
        let mut stop_events_by_measure: HashMap<u32, Vec<(f64, u16)>> = HashMap::new(); // pos -> stop_id
        let mut scroll_events_by_measure: HashMap<u32, Vec<(f64, u16)>> = HashMap::new(); // pos -> scroll_id

        for event in &events {
            if event.channel == 0x03 {
                // Integer BPM change (channel 03 data is hex 00-FF, not base36)
                for &(pos, val) in &event.data {
                    if val > 0 {
                        let bpm = base36_to_hex(val) as f64;
                        bpm_events_by_measure
                            .entry(event.measure)
                            .or_default()
                            .push((pos, bpm));
                    }
                }
            } else if event.channel == 0x08 {
                // Extended BPM change
                for &(pos, id) in &event.data {
                    if id > 0
                        && let Some(&bpm) = extended_bpms.get(&id)
                    {
                        bpm_events_by_measure
                            .entry(event.measure)
                            .or_default()
                            .push((pos, bpm));
                    }
                }
            } else if event.channel == 0x09 {
                // STOP event
                for &(pos, id) in &event.data {
                    if id > 0 {
                        stop_events_by_measure
                            .entry(event.measure)
                            .or_default()
                            .push((pos, id));
                    }
                }
            } else if event.channel == CHANNEL_SCROLL {
                // SCROLL event
                for &(pos, id) in &event.data {
                    if id > 0 {
                        scroll_events_by_measure
                            .entry(event.measure)
                            .or_default()
                            .push((pos, id));
                    }
                }
            }
        }

        // Phase 2: walk through measures, computing time for each position
        for measure in 0..=max_measure {
            measure_times.push(current_time_f64);
            scroll_at_measure.push(current_scroll);
            let measure_len = measure_lengths.get(&measure).copied().unwrap_or(1.0);

            // Collect events in this measure, sorted by position
            let mut timing_events: Vec<(f64, TimingEvent)> = Vec::new();

            if let Some(bpm_evts) = bpm_events_by_measure.get(&measure) {
                for &(pos, bpm) in bpm_evts {
                    timing_events.push((pos, TimingEvent::Bpm(bpm)));
                }
            }
            if let Some(stop_evts) = stop_events_by_measure.get(&measure) {
                for &(pos, id) in stop_evts {
                    if let Some(&ticks) = stop_defs.get(&id) {
                        timing_events.push((pos, TimingEvent::Stop(ticks)));
                    }
                }
            }
            if let Some(scroll_evts) = scroll_events_by_measure.get(&measure) {
                for &(pos, id) in scroll_evts {
                    if let Some(&scroll) = scroll_defs.get(&id) {
                        timing_events.push((pos, TimingEvent::Scroll(scroll)));
                    }
                }
            }

            timing_events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            let mut prev_pos = 0.0;
            for (pos, event) in &timing_events {
                // Advance time from prev_pos to this position
                let delta_beats = (*pos - prev_pos) * 4.0 * measure_len;
                current_time_f64 += beats_to_us(delta_beats, current_bpm);

                match event {
                    TimingEvent::Bpm(new_bpm) => {
                        model.bpm_changes.push(BpmChange {
                            time_us: current_time_f64 as i64,
                            bpm: *new_bpm,
                        });
                        current_bpm = *new_bpm;
                    }
                    TimingEvent::Stop(ticks) => {
                        let stop_us = ticks_to_us(*ticks, current_bpm);
                        model.stop_events.push(StopEvent {
                            time_us: current_time_f64 as i64,
                            duration_ticks: *ticks,
                            duration_us: stop_us as i64,
                        });
                        current_time_f64 += stop_us;
                    }
                    TimingEvent::Scroll(scroll) => {
                        current_scroll = *scroll;
                    }
                }
                prev_pos = *pos;
            }

            // Advance to end of measure
            let remaining_beats = (1.0 - prev_pos) * 4.0 * measure_len;
            current_time_f64 += beats_to_us(remaining_beats, current_bpm);
        }

        model.total_time_us = current_time_f64 as i64;

        // Phase 3: Place notes
        let mut ln_states: HashMap<(u32, usize), LnState> = HashMap::new(); // (channel_group, lane) -> state

        for event in &events {
            let ch = event.channel;

            // Skip timing channels (already processed), but not 0x01 (BGM) or BGA channels
            if matches!(ch, 0x02 | 0x03 | 0x08 | 0x09) || ch == CHANNEL_SCROLL {
                continue;
            }

            // BGA channels: 04 (BGA base), 06 (BGA layer), 07 (BGA poor)
            if matches!(ch, 0x04 | 0x06 | 0x07) {
                let bga_layer = match ch {
                    0x04 => BgaLayer::Bga,
                    0x06 => BgaLayer::Layer,
                    _ => BgaLayer::Poor,
                };

                let measure = event.measure;
                let measure_time_f64 = measure_times.get(measure as usize).copied().unwrap_or(0.0);
                let measure_len = measure_lengths.get(&measure).copied().unwrap_or(1.0);

                for &(pos, bmp_id) in &event.data {
                    if bmp_id == 0 {
                        continue;
                    }
                    let time_us = (measure_time_f64
                        + position_to_us(
                            pos,
                            measure,
                            measure_len,
                            &measure_times,
                            &model,
                            current_bpm,
                            &bpm_events_by_measure,
                            &stop_events_by_measure,
                            &stop_defs,
                            &extended_bpms,
                        )) as i64;
                    model.bga_events.push(BgaEvent {
                        time_us,
                        layer: bga_layer,
                        id: bmp_id as i32,
                    });
                }
                continue;
            }

            let measure = event.measure;
            let measure_time_f64 = measure_times.get(measure as usize).copied().unwrap_or(0.0);
            let measure_len = measure_lengths.get(&measure).copied().unwrap_or(1.0);

            for &(pos, wav_id) in &event.data {
                if wav_id == 0 {
                    continue;
                }

                let time_us = (measure_time_f64
                    + position_to_us(
                        pos,
                        measure,
                        measure_len,
                        &measure_times,
                        &model,
                        current_bpm,
                        &bpm_events_by_measure,
                        &stop_events_by_measure,
                        &stop_defs,
                        &extended_bpms,
                    )) as i64;

                // BGM channel (01): add as background note
                if ch == 0x01 {
                    model.bg_notes.push(BgNote {
                        wav_id,
                        time_us,
                        micro_starttime: 0,
                        micro_duration: 0,
                    });
                    continue;
                }

                let (lane, note_kind) = match ch {
                    // 1P visible (11-19)
                    0x11..=0x19 => {
                        let idx = (ch - 0x11) as usize;
                        let assign = model.mode.channel_assign_1p();
                        let l = assign[idx];
                        if l < 0 {
                            continue;
                        }
                        (l as usize, NoteKind::Normal)
                    }
                    // 2P visible (21-29)
                    0x21..=0x29 => {
                        let idx = (ch - 0x21) as usize;
                        let assign = model.mode.channel_assign_2p();
                        let l = assign[idx];
                        if l < 0 {
                            continue;
                        }
                        (l as usize, NoteKind::Normal)
                    }
                    // 1P invisible (31-39)
                    0x31..=0x39 => {
                        let idx = (ch - 0x31) as usize;
                        let assign = model.mode.channel_assign_1p();
                        let l = assign[idx];
                        if l < 0 {
                            continue;
                        }
                        (l as usize, NoteKind::Invisible)
                    }
                    // 2P invisible (41-49)
                    0x41..=0x49 => {
                        let idx = (ch - 0x41) as usize;
                        let assign = model.mode.channel_assign_2p();
                        let l = assign[idx];
                        if l < 0 {
                            continue;
                        }
                        (l as usize, NoteKind::Invisible)
                    }
                    // 1P LN (51-59)
                    0x51..=0x59 => {
                        let idx = (ch - 0x51) as usize;
                        let assign = model.mode.channel_assign_1p();
                        let l = assign[idx];
                        if l < 0 {
                            continue;
                        }
                        (l as usize, NoteKind::LongNote)
                    }
                    // 2P LN (61-69)
                    0x61..=0x69 => {
                        let idx = (ch - 0x61) as usize;
                        let assign = model.mode.channel_assign_2p();
                        let l = assign[idx];
                        if l < 0 {
                            continue;
                        }
                        (l as usize, NoteKind::LongNote)
                    }
                    // 1P mine (D1-D9)
                    0xD1..=0xD9 => {
                        let idx = (ch - 0xD1) as usize;
                        let assign = model.mode.channel_assign_1p();
                        let l = assign[idx];
                        if l < 0 {
                            continue;
                        }
                        (l as usize, NoteKind::Mine)
                    }
                    // 2P mine (E1-E9)
                    0xE1..=0xE9 => {
                        let idx = (ch - 0xE1) as usize;
                        let assign = model.mode.channel_assign_2p();
                        let l = assign[idx];
                        if l < 0 {
                            continue;
                        }
                        (l as usize, NoteKind::Mine)
                    }
                    _ => continue,
                };

                match note_kind {
                    NoteKind::Normal => {
                        // LNOBJ check: if wav_id matches lnobj, convert previous note to LN
                        if model.lnobj == Some(wav_id) {
                            // Search backwards for the most recent note on this lane
                            let mut found = false;
                            for i in (0..model.notes.len()).rev() {
                                if model.notes[i].lane != lane {
                                    continue;
                                }
                                if model.notes[i].time_us >= time_us {
                                    continue;
                                }
                                let prev = &model.notes[i];
                                if prev.note_type == crate::note::NoteType::Normal {
                                    // Convert NormalNote → LongNote
                                    let start_wav = prev.wav_id;
                                    let start_time = prev.time_us;
                                    model.notes[i] = Note::long_note(
                                        lane,
                                        start_time,
                                        time_us,
                                        start_wav,
                                        0, // LNOBJ end has no wav (Java uses -2)
                                        model.ln_type,
                                    );
                                    found = true;
                                    break;
                                } else if prev.is_long_note() && prev.end_time_us == 0 {
                                    // Unpaired LN started by LN channel, ended by LNOBJ
                                    model.notes[i].end_time_us = time_us;
                                    found = true;
                                    break;
                                } else {
                                    // Conflicting note type
                                    model.decode_logs.push(DecodeLog::warning(format!(
                                        "LNOBJ conflict on lane {} at time {}us",
                                        lane, time_us
                                    )));
                                    break;
                                }
                            }
                            if !found {
                                model.decode_logs.push(DecodeLog::warning(format!(
                                    "LNOBJ on lane {} at time {}us has no matching start note",
                                    lane, time_us
                                )));
                            }
                        } else {
                            model.notes.push(Note::normal(lane, time_us, wav_id));
                        }
                    }
                    NoteKind::Invisible => {
                        model.notes.push(Note::invisible(lane, time_us, wav_id));
                    }
                    NoteKind::Mine => {
                        model
                            .notes
                            .push(Note::mine(lane, time_us, wav_id, wav_id as i32));
                    }
                    NoteKind::LongNote => {
                        let key = ((ch & 0x0F) as u32, lane);
                        if let Some(start_state) = ln_states.remove(&key) {
                            // End of LN
                            let note = Note::long_note(
                                lane,
                                start_state.time_us,
                                time_us,
                                start_state.wav_id,
                                wav_id,
                                model.ln_type,
                            );
                            model.notes.push(note);
                        } else {
                            // Start of LN
                            ln_states.insert(key, LnState { wav_id, time_us });
                        }
                    }
                }
            }
        }

        // Sort notes by time, then by visibility category, then by lane
        // Java stores non-invisible and invisible in separate lists, so
        // non-invisible notes come first within each time group
        model.notes.sort_by(|a, b| {
            a.time_us
                .cmp(&b.time_us)
                .then_with(|| is_invisible(a).cmp(&is_invisible(b)))
                .then_with(|| a.lane.cmp(&b.lane))
        });

        // Sort background notes by time
        model.bg_notes.sort_by_key(|n| n.time_us);

        // Sort BGA events by time
        model.bga_events.sort_by_key(|e| e.time_us);

        // Deduplicate: when same (lane, time_us, visibility), keep highest priority note
        // Invisible and non-invisible notes can coexist on the same lane+time (Java stores them separately)
        model.notes.dedup_by(|b, a| {
            if a.lane == b.lane && a.time_us == b.time_us && is_invisible(a) == is_invisible(b) {
                if note_priority(b) > note_priority(a) {
                    std::mem::swap(a, b);
                }
                true
            } else {
                false
            }
        });

        // Build timelines
        // Convert f64 measure times to i64 for helper functions
        let measure_times_i64: Vec<i64> = measure_times.iter().map(|&t| t as i64).collect();
        let mut seen_times: Vec<i64> = model.notes.iter().map(|n| n.time_us).collect();
        seen_times.sort();
        seen_times.dedup();

        for &t in &seen_times {
            // Find BPM at this time
            let bpm = bpm_at_time(t, model.initial_bpm, &model.bpm_changes);
            let measure = find_measure_for_time(t, &measure_times_i64);
            let scroll = scroll_at_time(
                t,
                measure,
                &measure_times_i64,
                &measure_lengths,
                &scroll_at_measure,
                &scroll_events_by_measure,
                &scroll_defs,
            );
            model.timelines.push(TimeLine {
                time_us: t,
                measure,
                position: 0.0,
                bpm,
                scroll,
            });
        }

        // Compute hashes
        compute_hashes(&raw_content_for_hash(content), &mut model);

        Ok(model)
    }
}

#[derive(Debug)]
enum TimingEvent {
    Bpm(f64),
    Stop(i64),
    Scroll(f64),
}

#[derive(Debug, Clone, Copy)]
enum NoteKind {
    Normal,
    Invisible,
    LongNote,
    Mine,
}

struct RandomState {
    #[allow(dead_code)] // Parsed for completeness (BMS #RANDOM bound)
    bound: i32,
    value: i32,
    active: bool,
}

/// Convert beats to microseconds at given BPM (returns f64 for accumulation precision)
fn beats_to_us(beats: f64, bpm: f64) -> f64 {
    if bpm <= 0.0 {
        return 0.0;
    }
    (beats * 60_000_000.0) / bpm
}

/// Convert STOP ticks to microseconds (192 ticks = 1 measure = 4 beats, returns f64)
fn ticks_to_us(ticks: i64, bpm: f64) -> f64 {
    if bpm <= 0.0 {
        return 0.0;
    }
    let beats = ticks as f64 / 48.0; // 192 ticks / 4 beats = 48 ticks per beat
    (beats * 60_000_000.0) / bpm
}

/// Compute time offset for a position within a measure
/// This is a simplified version that accounts for BPM changes and stops within the measure
#[allow(clippy::too_many_arguments)]
fn position_to_us(
    pos: f64,
    measure: u32,
    measure_len: f64,
    _measure_times: &[f64],
    _model: &BmsModel,
    _current_bpm: f64,
    bpm_events: &HashMap<u32, Vec<(f64, f64)>>,
    stop_events: &HashMap<u32, Vec<(f64, u16)>>,
    stop_defs: &HashMap<u16, i64>,
    _extended_bpms: &HashMap<u16, f64>,
) -> f64 {
    let mut bpm = _model.initial_bpm;

    // Find the BPM at the start of this measure by looking at all previous BPM changes
    for m in 0..measure {
        if let Some(evts) = bpm_events.get(&m) {
            for &(_, new_bpm) in evts {
                bpm = new_bpm;
            }
        }
    }

    // Process events within this measure up to pos
    let mut time_offset: f64 = 0.0;
    let mut prev_pos = 0.0;

    let mut timing: Vec<(f64, TimingEvent)> = Vec::new();
    if let Some(evts) = bpm_events.get(&measure) {
        for &(p, b) in evts {
            if p < pos {
                timing.push((p, TimingEvent::Bpm(b)));
            }
        }
    }
    if let Some(evts) = stop_events.get(&measure) {
        for &(p, id) in evts {
            if p < pos
                && let Some(&ticks) = stop_defs.get(&id)
            {
                timing.push((p, TimingEvent::Stop(ticks)));
            }
        }
    }
    timing.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    for (p, event) in &timing {
        let delta_beats = (*p - prev_pos) * 4.0 * measure_len;
        time_offset += beats_to_us(delta_beats, bpm);
        match event {
            TimingEvent::Bpm(new_bpm) => bpm = *new_bpm,
            TimingEvent::Stop(ticks) => time_offset += ticks_to_us(*ticks, bpm),
            TimingEvent::Scroll(_) => {} // Scroll doesn't affect timing
        }
        prev_pos = *p;
    }

    let remaining_beats = (pos - prev_pos) * 4.0 * measure_len;
    time_offset += beats_to_us(remaining_beats, bpm);

    time_offset
}

/// Convert a base36-parsed value back to hex interpretation.
/// Channel 03 data is hex (00-FF), but parse_channel_data reads it as base36.
/// E.g., "B4" → base36: 11*36+4=400 → hex: 0xB4=180
fn base36_to_hex(val: u16) -> u16 {
    (val / 36) * 16 + (val % 36)
}

fn parse_finite_f64_or(input: &str, default: f64) -> f64 {
    match input.parse::<f64>() {
        Ok(v) if v.is_finite() => v,
        _ => default,
    }
}

/// Parse base-36 two-character string to u16
fn parse_base36(s: &str) -> u16 {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return 0;
    }
    let high = base36_digit(bytes[0]);
    let low = base36_digit(bytes[1]);
    high * 36 + low
}

fn base36_digit(b: u8) -> u16 {
    match b {
        b'0'..=b'9' => (b - b'0') as u16,
        b'A'..=b'Z' => (b - b'A' + 10) as u16,
        b'a'..=b'z' => (b - b'a' + 10) as u16,
        _ => 0,
    }
}

/// Parse base-62 two-character string to u16.
/// 0-9 = 0-9, A-Z = 10-35, a-z = 36-61
/// Max value: 61*62+61 = 3843
fn parse_base62(s: &str) -> u16 {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return 0;
    }
    let high = base62_digit(bytes[0]);
    let low = base62_digit(bytes[1]);
    high * 62 + low
}

fn base62_digit(b: u8) -> u16 {
    match b {
        b'0'..=b'9' => (b - b'0') as u16,
        b'A'..=b'Z' => (b - b'A' + 10) as u16,
        b'a'..=b'z' => (b - b'a' + 36) as u16,
        _ => 0,
    }
}

/// Parse an ID using the current base (36 or 62)
fn parse_id(s: &str, base: u8) -> u16 {
    if base == 62 {
        parse_base62(s)
    } else {
        parse_base36(s)
    }
}

/// Scroll channel number (base36: S=28, C=12 → 28*36+12=1020)
const CHANNEL_SCROLL: u16 = 1020;

/// Parse a channel line: #MMMCC:data
/// `upper` is used for measure/channel parsing, `original` preserves case for base62 data
fn parse_channel_line(upper: &str, original: &str, base: u8) -> Option<ChannelEvent> {
    if upper.len() < 7 {
        return None;
    }

    // Safe slicing: ensure byte indices are on char boundaries (guards against
    // multi-byte UTF-8 characters produced by Shift_JIS decoding).
    let measure_str = upper.get(..3)?;
    let channel_str = upper.get(3..5)?;
    let measure: u32 = measure_str.parse().ok()?;
    // SC channel uses base36, not hex
    let channel = if channel_str == "SC" {
        CHANNEL_SCROLL
    } else {
        parse_hex_channel(channel_str)?
    };

    if upper.as_bytes().get(5).copied() != Some(b':') {
        return None;
    }

    // Use original case for data to preserve base62 case sensitivity
    let data_str = original.get(6..)?;
    let data = parse_channel_data(data_str, base);

    Some(ChannelEvent {
        measure,
        channel,
        data,
    })
}

/// Parse channel identifier (base-16 for some, base-36 for others)
fn parse_hex_channel(s: &str) -> Option<u16> {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    // Channel identifiers use hex-like notation
    let high = hex_or_base36_digit(bytes[0])? as u16;
    let low = hex_or_base36_digit(bytes[1])? as u16;
    Some(high * 16 + low)
}

fn hex_or_base36_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// Parse channel data string into (position, value) pairs
/// Data is a sequence of base-36/62 pairs: "01020300" = [(0.0, 01), (0.333, 02), (0.666, 03), (1.0, 00)]
fn parse_channel_data(data: &str, base: u8) -> Vec<(f64, u16)> {
    let data = data.trim();
    if data.len() < 2 {
        return Vec::new();
    }

    let count = data.len() / 2;
    let mut result = Vec::new();

    for i in 0..count {
        let s = &data[i * 2..i * 2 + 2];
        let val = parse_id(s, base);
        if val > 0 {
            let pos = i as f64 / count as f64;
            result.push((pos, val));
        }
    }

    result
}

/// Whether a note is invisible (Java stores these in a separate list)
fn is_invisible(n: &Note) -> bool {
    n.note_type == crate::note::NoteType::Invisible
}

/// Note priority for deduplication: Normal/Invisible > LN > Mine
fn note_priority(n: &Note) -> u8 {
    match n.note_type {
        crate::note::NoteType::Normal | crate::note::NoteType::Invisible => 2,
        crate::note::NoteType::LongNote
        | crate::note::NoteType::ChargeNote
        | crate::note::NoteType::HellChargeNote => 1,
        crate::note::NoteType::Mine => 0,
    }
}

/// Find BPM at a given time
fn bpm_at_time(time_us: i64, initial_bpm: f64, bpm_changes: &[BpmChange]) -> f64 {
    let mut bpm = initial_bpm;
    for change in bpm_changes {
        if change.time_us <= time_us {
            bpm = change.bpm;
        } else {
            break;
        }
    }
    bpm
}

/// Compute the scroll value at a given time_us.
///
/// Uses the scroll value at measure start (from timing walk) and applies
/// any scroll events within the measure up to the note's position.
fn scroll_at_time(
    time_us: i64,
    measure: u32,
    measure_times: &[i64],
    _measure_lengths: &HashMap<u32, f64>,
    scroll_at_measure: &[f64],
    scroll_events_by_measure: &HashMap<u32, Vec<(f64, u16)>>,
    scroll_defs: &HashMap<u16, f64>,
) -> f64 {
    let mut scroll = scroll_at_measure
        .get(measure as usize)
        .copied()
        .unwrap_or(1.0);

    if let Some(evts) = scroll_events_by_measure.get(&measure) {
        let measure_start = measure_times.get(measure as usize).copied().unwrap_or(0);
        let measure_end = measure_times
            .get(measure as usize + 1)
            .copied()
            .unwrap_or(measure_start);
        let measure_duration = measure_end - measure_start;

        let pos_in_measure = if measure_duration > 0 {
            (time_us - measure_start) as f64 / measure_duration as f64
        } else {
            0.0
        };

        let mut sorted_evts: Vec<(f64, u16)> = evts.clone();
        sorted_evts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        for &(evt_pos, id) in &sorted_evts {
            if evt_pos <= pos_in_measure
                && let Some(&s) = scroll_defs.get(&id)
            {
                scroll = s;
            }
        }
    }

    scroll
}

/// Find the measure number for a given time_us using binary search on measure_times
fn find_measure_for_time(time_us: i64, measure_times: &[i64]) -> u32 {
    match measure_times.binary_search(&time_us) {
        Ok(idx) => idx as u32,
        Err(idx) => {
            if idx > 0 {
                (idx - 1) as u32
            } else {
                0
            }
        }
    }
}

/// Detect encoding and decode bytes to string
fn detect_encoding_and_decode(raw: &[u8]) -> String {
    // Check for UTF-8 BOM
    if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&raw[3..]).into_owned();
    }

    // Try UTF-8 first
    if let Ok(s) = std::str::from_utf8(raw) {
        return s.to_string();
    }

    // Try Shift_JIS
    let (decoded, _, had_errors) = encoding_rs::SHIFT_JIS.decode(raw);
    if !had_errors {
        return decoded.into_owned();
    }

    // Try EUC-JP
    let (decoded, _, had_errors) = encoding_rs::EUC_JP.decode(raw);
    if !had_errors {
        return decoded.into_owned();
    }

    // Fallback to Shift_JIS with lossy conversion
    let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(raw);
    decoded.into_owned()
}

/// Simple PRNG for #RANDOM (not Java LCG, just for basic functionality)
fn simple_random(bound: i32) -> i32 {
    if bound <= 1 {
        return 1;
    }
    // Use a simple random for now; Java LCG reproduction is in bms-pattern
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (seed as i32 % bound) + 1
}

/// Extract raw content for hash computation (strip comments, normalize)
fn raw_content_for_hash(content: &str) -> Vec<u8> {
    content.as_bytes().to_vec()
}

/// Compute MD5 and SHA256 hashes
fn compute_hashes(raw: &[u8], model: &mut BmsModel) {
    model.md5 = format!("{:x}", md5::compute(raw));

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(raw);
    model.sha256 = format!("{:x}", hasher.finalize());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_base36() {
        assert_eq!(parse_base36("00"), 0);
        assert_eq!(parse_base36("01"), 1);
        assert_eq!(parse_base36("0Z"), 35);
        assert_eq!(parse_base36("10"), 36);
        assert_eq!(parse_base36("ZZ"), 35 * 36 + 35);
    }

    #[test]
    fn test_parse_channel_data() {
        let data = parse_channel_data("01020300", 36);
        assert_eq!(data.len(), 3); // 00 is filtered out
        assert_eq!(data[0].1, 1); // 01
        assert_eq!(data[1].1, 2); // 02
        assert_eq!(data[2].1, 3); // 03
    }

    #[test]
    fn test_beats_to_us() {
        // 1 beat at 120 BPM = 500000 us (0.5s)
        assert_eq!(beats_to_us(1.0, 120.0), 500000.0);
        // 4 beats at 120 BPM = 2000000 us (2s)
        assert_eq!(beats_to_us(4.0, 120.0), 2000000.0);
    }

    #[test]
    fn test_ticks_to_us() {
        // 192 ticks = 4 beats at 120 BPM = 2000000 us
        assert_eq!(ticks_to_us(192, 120.0), 2000000.0);
        // 48 ticks = 1 beat at 120 BPM = 500000 us
        assert_eq!(ticks_to_us(48, 120.0), 500000.0);
    }

    #[test]
    fn test_detect_encoding_utf8() {
        let content = "UTF-8テスト".as_bytes();
        let result = detect_encoding_and_decode(content);
        assert!(result.contains("テスト"));
    }

    #[test]
    fn test_decode_minimal_7k() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/minimal_7k.bms");
        if path.exists() {
            let model = BmsDecoder::decode(&path).unwrap();
            assert_eq!(model.title, "Minimal 7K Test");
            assert_eq!(model.artist, "brs-test");
            assert_eq!(model.genre, "Test");
            assert_eq!(model.initial_bpm, 120.0);
            assert_eq!(model.mode, PlayMode::Beat7K);
            assert!(model.total_notes() > 0, "should have notes");
        }
    }

    #[test]
    fn test_decode_bpm_change() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/bpm_change.bms");
        if path.exists() {
            let model = BmsDecoder::decode(&path).unwrap();
            assert_eq!(model.initial_bpm, 120.0);
            assert!(!model.bpm_changes.is_empty(), "should have BPM changes");
        }
    }

    #[test]
    fn test_decode_stop_sequence() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/stop_sequence.bms");
        if path.exists() {
            let model = BmsDecoder::decode(&path).unwrap();
            assert!(!model.stop_events.is_empty(), "should have STOP events");
        }
    }

    #[test]
    fn test_decode_longnote() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/longnote_types.bms");
        if path.exists() {
            let model = BmsDecoder::decode(&path).unwrap();
            assert!(model.total_long_notes() > 0, "should have long notes");
        }
    }

    #[test]
    fn test_decode_mine_notes() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/mine_notes.bms");
        if path.exists() {
            let model = BmsDecoder::decode(&path).unwrap();
            let mines = model
                .notes
                .iter()
                .filter(|n| n.note_type == crate::note::NoteType::Mine)
                .count();
            assert!(mines > 0, "should have mine notes");
        }
    }

    #[test]
    fn test_decode_encoding_sjis() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/encoding_sjis.bms");
        if path.exists() {
            let model = BmsDecoder::decode(&path).unwrap();
            assert!(model.title.contains("Shift_JIS"));
        }
    }

    #[test]
    fn test_decode_14key_dp() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/14key_dp.bms");
        if path.exists() {
            let model = BmsDecoder::decode(&path).unwrap();
            assert_eq!(model.mode, PlayMode::Beat14K);
            assert_eq!(model.player, 3);
        }
    }

    // --- LN pair integrity tests ---

    /// Helper to decode inline BMS content with a .bms dummy path
    fn decode_inline(content: &str) -> BmsModel {
        BmsDecoder::decode_str(content, Path::new("test.bms")).unwrap()
    }

    #[test]
    fn test_ln_pair_basic() {
        let bms = "\
#PLAYER 1
#BPM 120
#WAV01 test.wav
#00151:01000001
";
        let model = decode_inline(bms);
        let lns: Vec<&Note> = model.notes.iter().filter(|n| n.is_long_note()).collect();
        assert_eq!(lns.len(), 1, "should have exactly 1 LN");
        let ln = lns[0];
        assert!(
            ln.end_time_us > ln.time_us,
            "end_time_us must be after time_us"
        );
        assert!(ln.is_long_note());
        assert_eq!(ln.note_type, crate::note::NoteType::LongNote);
    }

    #[test]
    fn test_ln_pair_sequential() {
        // Two consecutive LNs on the same lane (ch51) across two measures
        let bms = "\
#PLAYER 1
#BPM 120
#WAV01 test.wav
#00151:01000001
#00251:01000001
";
        let model = decode_inline(bms);
        let lns: Vec<&Note> = model.notes.iter().filter(|n| n.is_long_note()).collect();
        assert_eq!(lns.len(), 2, "should have 2 LNs");
        // Second LN starts after first LN ends
        assert!(
            lns[1].time_us >= lns[0].end_time_us,
            "second LN should start at or after first LN ends"
        );
    }

    #[test]
    fn test_ln_pair_multi_lane() {
        // Simultaneous LNs on ch51 (lane 0) and ch52 (lane 1)
        let bms = "\
#PLAYER 1
#BPM 120
#WAV01 test.wav
#WAV02 test.wav
#00151:01000001
#00152:02000002
";
        let model = decode_inline(bms);
        let lns: Vec<&Note> = model.notes.iter().filter(|n| n.is_long_note()).collect();
        assert_eq!(lns.len(), 2, "should have 2 LNs on different lanes");
        let lanes: Vec<usize> = lns.iter().map(|n| n.lane).collect();
        assert!(
            lanes.contains(&0) || lanes.contains(&1),
            "should have LNs on distinct lanes"
        );
        assert_ne!(lns[0].lane, lns[1].lane, "LNs should be on different lanes");
    }

    #[test]
    fn test_ln_unclosed_dropped() {
        // Only a start marker with no end → should NOT produce an LN
        let bms = "\
#PLAYER 1
#BPM 120
#WAV01 test.wav
#00151:01
";
        let model = decode_inline(bms);
        let lns: Vec<&Note> = model.notes.iter().filter(|n| n.is_long_note()).collect();
        assert_eq!(lns.len(), 0, "unclosed LN should not produce a note");
    }

    #[test]
    fn test_ln_end_wav_id() {
        // Start wav=01, end wav=02 → end_wav_id should be 2
        let bms = "\
#PLAYER 1
#BPM 120
#WAV01 start.wav
#WAV02 end.wav
#00151:01000002
";
        let model = decode_inline(bms);
        let lns: Vec<&Note> = model.notes.iter().filter(|n| n.is_long_note()).collect();
        assert_eq!(lns.len(), 1, "should have 1 LN");
        assert_eq!(lns[0].wav_id, 1, "start wav_id should be 1");
        assert_eq!(lns[0].end_wav_id, 2, "end wav_id should be 2");
    }

    #[test]
    fn test_ln_type_charge_note() {
        let bms = "\
#PLAYER 1
#BPM 120
#LNTYPE 2
#WAV01 test.wav
#00151:01000001
";
        let model = decode_inline(bms);
        let lns: Vec<&Note> = model.notes.iter().filter(|n| n.is_long_note()).collect();
        assert_eq!(lns.len(), 1, "should have 1 LN");
        assert_eq!(lns[0].note_type, crate::note::NoteType::ChargeNote);
    }

    #[test]
    fn test_ln_type_hell_charge_note() {
        let bms = "\
#PLAYER 1
#BPM 120
#LNTYPE 3
#WAV01 test.wav
#00151:01000001
";
        let model = decode_inline(bms);
        let lns: Vec<&Note> = model.notes.iter().filter(|n| n.is_long_note()).collect();
        assert_eq!(lns.len(), 1, "should have 1 LN");
        assert_eq!(lns[0].note_type, crate::note::NoteType::HellChargeNote);
    }

    #[test]
    fn test_parse_empty_bms() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.bms");
        std::fs::write(&path, "").unwrap();
        let result = BmsDecoder::decode(&path);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.total_notes(), 0);
    }

    #[test]
    fn test_parse_bms_no_notes() {
        let bms = "\
#PLAYER 1
#BPM 130
";
        let model = decode_inline(bms);
        assert_eq!(model.player, 1);
        assert!((model.initial_bpm - 130.0).abs() < f64::EPSILON);
        assert_eq!(model.total_notes(), 0);
        assert!(model.notes.is_empty());
    }

    #[test]
    fn test_parse_extreme_bpm() {
        let bms = "\
#PLAYER 1
#BPM 0.01
#BPM01 999999
#00108:01
";
        let model = decode_inline(bms);
        assert!((model.initial_bpm - 0.01).abs() < 0.001);
        assert!(
            model
                .bpm_changes
                .iter()
                .any(|c| (c.bpm - 999999.0).abs() < f64::EPSILON)
        );
    }

    #[test]
    fn test_parse_duplicate_definitions() {
        let bms = "\
#PLAYER 1
#BPM 120
#WAV01 a.wav
#WAV02 b.wav
#00111:01
#00111:02
";
        let model = decode_inline(bms);
        assert!(!model.notes.is_empty(), "should have at least one note");
    }

    // --- LNOBJ tests ---

    #[test]
    fn test_lnobj_basic() {
        // LNOBJ ZZ: WAV01 at beat 0, WAVZZ at beat 2 → LN
        let bms = "\
#PLAYER 1
#BPM 120
#LNOBJ ZZ
#WAV01 test.wav
#WAVZZ end.wav
#00111:0100ZZ00
";
        let model = decode_inline(bms);
        assert_eq!(model.lnobj, Some(parse_base36("ZZ")));

        let lns: Vec<&Note> = model.notes.iter().filter(|n| n.is_long_note()).collect();
        assert_eq!(lns.len(), 1, "should have 1 LN from LNOBJ");
        assert!(
            lns[0].end_time_us > lns[0].time_us,
            "LN end must be after start"
        );
        assert_eq!(lns[0].wav_id, 1, "start wav_id should be WAV01");
    }

    #[test]
    fn test_lnobj_no_match_stays_normal() {
        // LNOBJ ZZ set but notes use WAV01/WAV02 (not ZZ) → remain normal
        let bms = "\
#PLAYER 1
#BPM 120
#LNOBJ ZZ
#WAV01 test.wav
#WAV02 test2.wav
#00111:01000200
";
        let model = decode_inline(bms);
        let normals: Vec<&Note> = model
            .notes
            .iter()
            .filter(|n| n.note_type == crate::note::NoteType::Normal)
            .collect();
        assert_eq!(normals.len(), 2, "both notes should remain normal");
    }

    #[test]
    fn test_lnobj_multiple_lanes() {
        // LNOBJ on two different lanes
        let bms = "\
#PLAYER 1
#BPM 120
#LNOBJ ZZ
#WAV01 a.wav
#WAV02 b.wav
#WAVZZ end.wav
#00111:0100ZZ00
#00112:0200ZZ00
";
        let model = decode_inline(bms);
        let lns: Vec<&Note> = model.notes.iter().filter(|n| n.is_long_note()).collect();
        assert_eq!(lns.len(), 2, "should have 2 LNs from LNOBJ");
    }

    // --- VOLWAV test ---

    #[test]
    fn test_volwav_parsing() {
        let bms = "\
#PLAYER 1
#BPM 120
#VOLWAV 80
";
        let model = decode_inline(bms);
        assert_eq!(model.volwav, 80);
    }

    #[test]
    fn test_volwav_default() {
        let bms = "\
#PLAYER 1
#BPM 120
";
        let model = decode_inline(bms);
        assert_eq!(model.volwav, 100);
    }

    // --- BASE 62 tests ---

    #[test]
    fn test_parse_base62() {
        // 0-9: 0-9, A-Z: 10-35, a-z: 36-61
        assert_eq!(parse_base62("00"), 0);
        assert_eq!(parse_base62("01"), 1);
        assert_eq!(parse_base62("0Z"), 35);
        assert_eq!(parse_base62("0a"), 36);
        assert_eq!(parse_base62("0z"), 61);
        assert_eq!(parse_base62("10"), 62);
        assert_eq!(parse_base62("zz"), 61 * 62 + 61); // 3843
    }

    #[test]
    fn test_base62_header_parsing() {
        let bms = "\
#PLAYER 1
#BPM 120
#BASE 62
#WAV0a test.wav
#00111:0a
";
        let model = decode_inline(bms);
        assert_eq!(model.base, 62);
        // WAV0a in base62 = 36
        assert!(model.wav_defs.contains_key(&36));
        assert!(!model.notes.is_empty());
        assert_eq!(model.notes[0].wav_id, 36);
    }

    #[test]
    fn test_base62_channel_data() {
        let data = parse_channel_data("0a0b00", 62);
        assert_eq!(data.len(), 2); // 00 is filtered
        assert_eq!(data[0].1, 36); // 0a in base62
        assert_eq!(data[1].1, 37); // 0b in base62
    }

    #[test]
    fn test_base36_default_when_no_base_header() {
        let bms = "\
#PLAYER 1
#BPM 120
";
        let model = decode_inline(bms);
        assert_eq!(model.base, 36);
    }

    // --- Error case tests ---

    #[test]
    fn test_decode_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.bms");
        std::fs::write(&path, b"").unwrap();
        let result = BmsDecoder::decode(&path);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.total_notes(), 0);
        assert!(model.notes.is_empty());
        assert!(model.bg_notes.is_empty());
    }

    #[test]
    fn test_decode_headers_only_no_notes() {
        let bms = "\
#PLAYER 1
#GENRE Test
#TITLE Headers Only
#ARTIST nobody
#BPM 150
#PLAYLEVEL 10
#RANK 2
#TOTAL 400
";
        let model = decode_inline(bms);
        assert_eq!(model.total_notes(), 0);
        assert_eq!(model.title, "Headers Only");
        assert_eq!(model.artist, "nobody");
        assert_eq!(model.genre, "Test");
        assert!((model.initial_bpm - 150.0).abs() < f64::EPSILON);
        assert_eq!(model.play_level, 10);
        assert_eq!(model.judge_rank, 2);
        assert!((model.total - 400.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_decode_zero_bpm() {
        // BPM 0 should not cause division by zero
        let bms = "\
#PLAYER 1
#BPM 0
#WAV01 test.wav
#00111:01
";
        let model = decode_inline(bms);
        assert!((model.initial_bpm - 0.0).abs() < f64::EPSILON);
        // Should not panic, notes should still be parsed
        assert!(!model.notes.is_empty());
    }

    #[test]
    fn test_decode_negative_bpm_string() {
        // Negative BPM parses as-is (unwrap_or defaults to 130.0 on failure,
        // but -50 is a valid parse)
        let bms = "\
#PLAYER 1
#BPM -50
#WAV01 test.wav
#00111:01
";
        let model = decode_inline(bms);
        // -50 parses as f64 successfully
        assert!((model.initial_bpm - (-50.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_decode_extremely_large_bpm() {
        let bms = "\
#PLAYER 1
#BPM 99999999
#WAV01 test.wav
#00111:01
";
        let model = decode_inline(bms);
        assert!((model.initial_bpm - 99999999.0).abs() < 1.0);
        assert!(!model.notes.is_empty());
    }

    #[test]
    fn test_decode_malformed_bpm_value() {
        // Non-numeric BPM should fall back to default (unwrap_or(130.0))
        let bms = "\
#PLAYER 1
#BPM abc
#WAV01 test.wav
#00111:01
";
        let model = decode_inline(bms);
        assert!((model.initial_bpm - 130.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_decode_nan_bpm_falls_back_to_default() {
        let bms = "\
#PLAYER 1
#BPM NaN
#WAV01 test.wav
#00111:01
";
        let model = decode_inline(bms);
        assert!((model.initial_bpm - 130.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_decode_nan_extended_bpm_is_not_nan() {
        let bms = "\
#PLAYER 1
#BPM 120
#BPM01 NaN
#WAV01 test.wav
#00108:01
#00111:01
";
        let model = decode_inline(bms);
        assert!(
            model.bpm_changes.iter().all(|c| c.bpm.is_finite()),
            "extended BPM changes should stay finite"
        );
    }

    #[test]
    fn test_decode_extreme_measure_number() {
        // High measure number (999) should not crash
        let bms = "\
#PLAYER 1
#BPM 120
#WAV01 test.wav
#99911:01
";
        let model = decode_inline(bms);
        assert!(!model.notes.is_empty());
        assert!(model.total_measures >= 999);
    }

    #[test]
    fn test_decode_invalid_base36_characters() {
        // Invalid characters in channel data should be handled gracefully
        let bms = "\
#PLAYER 1
#BPM 120
#WAV01 test.wav
#00111:!!
";
        let model = decode_inline(bms);
        // '!!' maps to 0 via base36_digit fallback, so no notes are placed
        assert_eq!(model.total_notes(), 0);
    }

    #[test]
    fn test_decode_single_char_channel_data() {
        // Channel data with length < 2 should not crash
        let bms = "\
#PLAYER 1
#BPM 120
#00111:0
";
        let model = decode_inline(bms);
        // Too short to parse any notes
        assert_eq!(model.total_notes(), 0);
    }

    #[test]
    fn test_decode_only_comments_and_blank_lines() {
        let bms = "\
* This is a comment
* Another comment


";
        let model = decode_inline(bms);
        assert_eq!(model.total_notes(), 0);
    }

    #[test]
    fn test_decode_random_binary_does_not_panic() {
        // Write some random-ish bytes that aren't valid BMS
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("garbage.bms");
        let garbage: Vec<u8> = (0..256).map(|i| (i % 256) as u8).collect();
        std::fs::write(&path, &garbage).unwrap();
        // Should not panic; may return Ok or Err
        let _ = BmsDecoder::decode(&path);
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn decode_arbitrary_bytes_never_panics(data: Vec<u8>) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("fuzz.bms");
            std::fs::write(&path, &data).unwrap();
            // Should never panic regardless of input
            let _ = BmsDecoder::decode(&path);
        }

        #[test]
        fn decode_str_arbitrary_content_never_panics(content in ".*") {
            let _ = BmsDecoder::decode_str(&content, Path::new("test.bms"));
        }

        #[test]
        fn parse_base36_any_pair_never_panics(
            a in proptest::char::range('0', 'z'),
            b in proptest::char::range('0', 'z'),
        ) {
            let s = format!("{a}{b}");
            // Should never panic
            let _ = parse_base36(&s);
        }

        #[test]
        fn parse_base62_any_pair_never_panics(
            a in proptest::char::range('0', 'z'),
            b in proptest::char::range('0', 'z'),
        ) {
            let s = format!("{a}{b}");
            let _ = parse_base62(&s);
        }

        #[test]
        fn beats_to_us_no_panic(beats in -1e12_f64..1e12, bpm in -1e6_f64..1e6) {
            let _ = beats_to_us(beats, bpm);
        }
    }
}

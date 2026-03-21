use std::collections::BTreeMap;

use crate::bms_model::BMSModel;
use crate::chart_decoder::{self, TimeLineCache};
use crate::decode_log::{DecodeLog, State};
use crate::layer::{Event, EventType, Layer, Sequence};
use crate::mode::Mode;
use crate::note::Note;
use crate::time_line::TimeLine;

pub const ILLEGAL: i32 = -1;
pub const LANE_AUTOPLAY: i32 = 1;
pub const SECTION_RATE: i32 = 2;
pub const BPM_CHANGE: i32 = 3;
pub const BGA_PLAY: i32 = 4;
pub const POOR_PLAY: i32 = 6;
pub const LAYER_PLAY: i32 = 7;
pub const BPM_CHANGE_EXTEND: i32 = 8;
pub const STOP: i32 = 9;

pub const P1_KEY_BASE: i32 = 36 + 1;
pub const P2_KEY_BASE: i32 = 2 * 36 + 1;
pub const P1_INVISIBLE_KEY_BASE: i32 = 3 * 36 + 1;
pub const P2_INVISIBLE_KEY_BASE: i32 = 4 * 36 + 1;
pub const P1_LONG_KEY_BASE: i32 = 5 * 36 + 1;
pub const P2_LONG_KEY_BASE: i32 = 6 * 36 + 1;
pub const P1_MINE_KEY_BASE: i32 = 13 * 36 + 1;
pub const P2_MINE_KEY_BASE: i32 = 14 * 36 + 1;

pub const SCROLL: i32 = 1020;

pub const NOTE_CHANNELS: [i32; 8] = [
    P1_KEY_BASE,
    P2_KEY_BASE,
    P1_INVISIBLE_KEY_BASE,
    P2_INVISIBLE_KEY_BASE,
    P1_LONG_KEY_BASE,
    P2_LONG_KEY_BASE,
    P1_MINE_KEY_BASE,
    P2_MINE_KEY_BASE,
];

const CHANNELASSIGN_BEAT5: [i32; 18] =
    [0, 1, 2, 3, 4, 5, -1, -1, -1, 6, 7, 8, 9, 10, 11, -1, -1, -1];
const CHANNELASSIGN_BEAT7: [i32; 18] =
    [0, 1, 2, 3, 4, 7, -1, 5, 6, 8, 9, 10, 11, 12, 15, -1, 13, 14];
const CHANNELASSIGN_POPN: [i32; 18] = [
    0, 1, 2, 3, 4, -1, -1, -1, -1, -1, 5, 6, 7, 8, -1, -1, -1, -1,
];

// Use u64 bit representation for f64 keys in BTreeMap since f64 doesn't impl Ord
type F64Key = u64;

fn f64_key(f: f64) -> F64Key {
    f64_to_key(f)
}

fn key_f64(k: F64Key) -> f64 {
    f64::from_bits(k)
}

/// Lookup tables for BPM, STOP, and SCROLL definitions.
pub struct SectionLookupTables<'a> {
    pub bpm: &'a BTreeMap<i32, f64>,
    pub stop: &'a BTreeMap<i32, f64>,
    pub scroll: &'a BTreeMap<i32, f64>,
}

/// Wav and BGA mapping arrays used by make_time_lines.
pub struct TimeLineMaps<'a> {
    pub wavmap: &'a [i32],
    pub bgamap: &'a [i32],
}

pub struct Section {
    rate: f64,
    poor: Vec<i32>,
    sectionnum: f64,
    channellines: Vec<String>,
    bpmchange: BTreeMap<F64Key, f64>,
    stop: BTreeMap<F64Key, f64>,
    scroll: BTreeMap<F64Key, f64>,
}

impl Section {
    pub fn new(
        model: &mut BMSModel,
        prev_sectionnum: f64,
        prev_rate: f64,
        is_first: bool,
        lines: &[String],
        tables: &SectionLookupTables<'_>,
        log: &mut Vec<DecodeLog>,
    ) -> Self {
        let bpmtable = tables.bpm;
        let stoptable = tables.stop;
        let scrolltable = tables.scroll;
        let base = model.base();
        let mut rate = 1.0;
        let mut poor: Vec<i32> = Vec::new();
        let mut channellines: Vec<String> = Vec::with_capacity(lines.len());
        let mut bpmchange: BTreeMap<F64Key, f64> = BTreeMap::new();
        let mut stop_map: BTreeMap<F64Key, f64> = BTreeMap::new();
        let mut scroll_map: BTreeMap<F64Key, f64> = BTreeMap::new();

        let sectionnum = if !is_first {
            prev_sectionnum + prev_rate
        } else {
            0.0
        };

        for line in lines {
            let bytes = line.as_bytes();
            if bytes.len() < 6 {
                continue;
            }
            let channel = chart_decoder::parse_int36(bytes[4] as char, bytes[5] as char);
            match channel {
                ILLEGAL => {
                    log.push(DecodeLog::new(
                        State::Warning,
                        format!("チャンネル定義が無効です : {}", line),
                    ));
                }
                LANE_AUTOPLAY | BGA_PLAY | LAYER_PLAY => {
                    channellines.push(line.clone());
                }
                SECTION_RATE => {
                    if let Some(colon_index) = line.find(':') {
                        match line[colon_index + 1..].parse::<f64>() {
                            Ok(r) if r.is_finite() && (0.0..=1000.0).contains(&r) => rate = r,
                            Ok(_) | Err(_) => {
                                log.push(DecodeLog::new(
                                    State::Warning,
                                    format!("小節の拡大率が不正です : {}", line),
                                ));
                            }
                        }
                    }
                }
                BPM_CHANGE => {
                    let results = process_data_collect(line, base, log, &model.title);
                    for (pos, mut data) in results {
                        if base == 62 {
                            // NOTE: This base62->base36 re-parsing is lossy for data values >= 1296
                            // (base-36 two-digit max). This matches beatoraja's behavior. For base-62
                            // charts, BPM changes should use channel 08 (extended BPM) instead of
                            // channel 03 (standard BPM).
                            let s = chart_decoder::to_base62(data);
                            let sb = s.as_bytes();
                            data = chart_decoder::parse_int36(sb[0] as char, sb[1] as char);
                            if data < 0 {
                                data = 0;
                            }
                        }
                        bpmchange
                            .insert(f64_key(pos), (data / 36) as f64 * 16.0 + (data % 36) as f64);
                    }
                }
                POOR_PLAY => {
                    poor = split_data(line, base, log, &model.title);
                    let mut singleid: i32 = 0;
                    for &id in &poor {
                        if id != 0 {
                            if singleid != 0 && singleid != id {
                                singleid = -1;
                                break;
                            } else {
                                singleid = id;
                            }
                        }
                    }
                    if singleid != -1 {
                        poor = vec![singleid];
                    }
                }
                BPM_CHANGE_EXTEND => {
                    let results = process_data_collect(line, base, log, &model.title);
                    for (pos, data) in results {
                        if let Some(&bpm) = bpmtable.get(&data) {
                            bpmchange.insert(f64_key(pos), bpm);
                        } else {
                            log.push(DecodeLog::new(
                                State::Warning,
                                format!("未定義のBPM変化を参照しています : {}", data),
                            ));
                        }
                    }
                }
                STOP => {
                    let results = process_data_collect(line, base, log, &model.title);
                    for (pos, data) in results {
                        if let Some(&st) = stoptable.get(&data) {
                            stop_map.insert(f64_key(pos), st);
                        } else {
                            log.push(DecodeLog::new(
                                State::Warning,
                                format!("未定義のSTOPを参照しています : {}", data),
                            ));
                        }
                    }
                }
                c if c == SCROLL => {
                    let results = process_data_collect(line, base, log, &model.title);
                    for (pos, data) in results {
                        if let Some(&st) = scrolltable.get(&data) {
                            scroll_map.insert(f64_key(pos), st);
                        } else {
                            log.push(DecodeLog::new(
                                State::Warning,
                                format!("未定義のSCROLLを参照しています : {}", data),
                            ));
                        }
                    }
                }
                _ => {}
            }

            let mut basech: i32 = 0;
            let mut ch2: i32 = -1;
            for &ch in &NOTE_CHANNELS {
                if ch <= channel && channel <= ch + 8 {
                    basech = ch;
                    ch2 = channel - ch;
                    channellines.push(line.clone());
                    break;
                }
            }
            // 5/10KEY -> 7/14KEY
            if ch2 == 7 || ch2 == 8 {
                let new_mode = if model.mode() == Some(&Mode::BEAT_5K) {
                    Some(Mode::BEAT_7K)
                } else if model.mode() == Some(&Mode::BEAT_10K) {
                    Some(Mode::BEAT_14K)
                } else {
                    None
                };
                if let Some(mode) = new_mode
                    && has_nonzero_data(line, base)
                {
                    model.set_mode(mode);
                }
            }
            // 5/7KEY -> 10/14KEY
            if basech == P2_KEY_BASE
                || basech == P2_INVISIBLE_KEY_BASE
                || basech == P2_LONG_KEY_BASE
                || basech == P2_MINE_KEY_BASE
            {
                let new_mode = if model.mode() == Some(&Mode::BEAT_5K) {
                    Some(Mode::BEAT_10K)
                } else if model.mode() == Some(&Mode::BEAT_7K) {
                    Some(Mode::BEAT_14K)
                } else {
                    None
                };
                if let Some(mode) = new_mode
                    && has_nonzero_data(line, base)
                {
                    model.set_mode(mode);
                }
            }
        }

        Section {
            rate,
            poor,
            sectionnum,
            channellines,
            bpmchange,
            stop: stop_map,
            scroll: scroll_map,
        }
    }

    pub fn make_time_lines(
        &self,
        model: &mut BMSModel,
        maps: &TimeLineMaps<'_>,
        tlcache: &mut BTreeMap<u64, TimeLineCache>,
        lnlist: &mut Vec<Option<Vec<LnInfo>>>,
        startln: &mut Vec<Option<StartLnInfo>>,
        log: &mut Vec<DecodeLog>,
    ) {
        let wavmap = maps.wavmap;
        let bgamap = maps.bgamap;
        let lnobj = model.lnobj;
        let lnmode = model.lnmode;
        let mode = model.mode().copied();
        let cassign: &[i32; 18] = if mode.as_ref() == Some(&Mode::POPN_9K) {
            &CHANNELASSIGN_POPN
        } else if mode.as_ref() == Some(&Mode::BEAT_7K) || mode.as_ref() == Some(&Mode::BEAT_14K) {
            &CHANNELASSIGN_BEAT7
        } else {
            &CHANNELASSIGN_BEAT5
        };
        let base = model.base();
        let mode_key = mode.as_ref().map(|m| m.key()).unwrap_or(0);

        // section line
        let basetl_section = self.sectionnum;
        ensure_timeline(tlcache, basetl_section, mode_key);
        let basetl_key = f64_to_key(basetl_section);
        tlcache
            .get_mut(&basetl_key)
            .expect("timeline key must exist")
            .timeline
            .section_line = true;

        self.process_poor_layer(bgamap, tlcache, basetl_key, mode_key);
        self.process_timing_events(tlcache, mode_key);

        for line in &self.channellines {
            let bytes = line.as_bytes();
            if bytes.len() < 6 {
                continue;
            }
            let mut channel = chart_decoder::parse_int36(bytes[4] as char, bytes[5] as char);
            let mut tmpkey: i32 = 0;
            if (P1_KEY_BASE..P1_KEY_BASE + 9).contains(&channel) {
                tmpkey = cassign[(channel - P1_KEY_BASE) as usize];
                channel = P1_KEY_BASE;
            } else if (P2_KEY_BASE..P2_KEY_BASE + 9).contains(&channel) {
                tmpkey = cassign[(channel - P2_KEY_BASE + 9) as usize];
                channel = P1_KEY_BASE;
            } else if (P1_INVISIBLE_KEY_BASE..P1_INVISIBLE_KEY_BASE + 9).contains(&channel) {
                tmpkey = cassign[(channel - P1_INVISIBLE_KEY_BASE) as usize];
                channel = P1_INVISIBLE_KEY_BASE;
            } else if (P2_INVISIBLE_KEY_BASE..P2_INVISIBLE_KEY_BASE + 9).contains(&channel) {
                tmpkey = cassign[(channel - P2_INVISIBLE_KEY_BASE + 9) as usize];
                channel = P1_INVISIBLE_KEY_BASE;
            } else if (P1_LONG_KEY_BASE..P1_LONG_KEY_BASE + 9).contains(&channel) {
                tmpkey = cassign[(channel - P1_LONG_KEY_BASE) as usize];
                channel = P1_LONG_KEY_BASE;
            } else if (P2_LONG_KEY_BASE..P2_LONG_KEY_BASE + 9).contains(&channel) {
                tmpkey = cassign[(channel - P2_LONG_KEY_BASE + 9) as usize];
                channel = P1_LONG_KEY_BASE;
            } else if (P1_MINE_KEY_BASE..P1_MINE_KEY_BASE + 9).contains(&channel) {
                tmpkey = cassign[(channel - P1_MINE_KEY_BASE) as usize];
                channel = P1_MINE_KEY_BASE;
            } else if (P2_MINE_KEY_BASE..P2_MINE_KEY_BASE + 9).contains(&channel) {
                tmpkey = cassign[(channel - P2_MINE_KEY_BASE + 9) as usize];
                channel = P1_MINE_KEY_BASE;
            }
            let key = tmpkey;
            if key == -1 {
                continue;
            }

            let ctx = ChannelContext {
                sectionnum: self.sectionnum,
                rate: self.rate,
                base,
                mode_key,
                lnobj,
                lnmode,
            };

            let state = &mut ChannelState {
                tlcache,
                lnlist,
                startln,
                log,
            };

            match channel {
                P1_KEY_BASE => {
                    process_normal_notes(&ctx, line, key, wavmap, state);
                }
                P1_INVISIBLE_KEY_BASE => {
                    process_invisible_notes(&ctx, line, key, wavmap, state.tlcache, state.log);
                }
                P1_LONG_KEY_BASE => {
                    process_long_notes(&ctx, line, key, wavmap, state);
                }
                P1_MINE_KEY_BASE => {
                    process_mine_notes(
                        &ctx,
                        line,
                        key,
                        wavmap,
                        state.tlcache,
                        state.lnlist,
                        state.log,
                    );
                }
                LANE_AUTOPLAY => {
                    process_autoplay_notes(&ctx, line, wavmap, state.tlcache, state.log);
                }
                BGA_PLAY => {
                    process_bga_channel(&ctx, line, bgamap, state.tlcache, state.log);
                }
                LAYER_PLAY => {
                    process_layer_channel(&ctx, line, bgamap, state.tlcache, state.log);
                }
                _ => {}
            }
        }
    }

    /// Process POOR layer (miss BGA animation).
    fn process_poor_layer(
        &self,
        bgamap: &[i32],
        tlcache: &mut BTreeMap<u64, TimeLineCache>,
        basetl_key: u64,
        mode_key: i32,
    ) {
        if self.poor.is_empty() {
            return;
        }
        let mut poors: Vec<Sequence> = Vec::with_capacity(self.poor.len() + 1);
        let poortime: i64 = 500;

        for (i, &poor_idx) in self.poor.iter().enumerate() {
            let time = (i as i64) * poortime / (self.poor.len() as i64);
            if poor_idx >= 0
                && (poor_idx as usize) < bgamap.len()
                && bgamap[poor_idx as usize] != -2
            {
                poors.push(Sequence::new(time, bgamap[poor_idx as usize]));
            } else {
                poors.push(Sequence::new(time, -1));
            }
        }
        poors.push(Sequence::new_end(poortime));
        let layer = Layer::new(Event::new(EventType::Miss, 1), vec![poors]);
        // Ensure timeline exists for the section base (needed when section has poor but no notes)
        ensure_timeline(tlcache, self.sectionnum, mode_key);
        tlcache
            .get_mut(&basetl_key)
            .expect("timeline key must exist")
            .timeline
            .eventlayer = vec![layer];
    }

    /// Process BPM changes, STOP sequences, and SCROLL events by merging them
    /// in position order.
    fn process_timing_events(&self, tlcache: &mut BTreeMap<u64, TimeLineCache>, mode_key: i32) {
        let stops_vec: Vec<(f64, f64)> = self.stop.iter().map(|(&k, &v)| (key_f64(k), v)).collect();
        let bpms_vec: Vec<(f64, f64)> = self
            .bpmchange
            .iter()
            .map(|(&k, &v)| (key_f64(k), v))
            .collect();
        let scrolls_vec: Vec<(f64, f64)> =
            self.scroll.iter().map(|(&k, &v)| (key_f64(k), v)).collect();

        let mut st_idx: usize = 0;
        let mut bc_idx: usize = 0;
        let mut sc_idx: usize = 0;

        loop {
            let ste = stops_vec.get(st_idx).copied();
            let bce = bpms_vec.get(bc_idx).copied();
            let sce = scrolls_vec.get(sc_idx).copied();

            if ste.is_none() && bce.is_none() && sce.is_none() {
                break;
            }

            let bc = bce.map(|(k, _)| k).unwrap_or(2.0);
            let st = ste.map(|(k, _)| k).unwrap_or(2.0);
            let sc = sce.map(|(k, _)| k).unwrap_or(2.0);

            // Guard: all three event types must have positions <= 1.0 (within
            // the section). Positions from process_data_collect are always
            // < 1.0 in practice, but the guard prevents processing phantom
            // events when the sentinel value 2.0 leaks through.
            if sc <= st && sc <= bc && sc <= 1.0 {
                let scroll_val = sce.expect("sce").1;
                let section = self.sectionnum + sc * self.rate;
                ensure_timeline(tlcache, section, mode_key);
                let tl = &mut tlcache
                    .get_mut(&f64_to_key(section))
                    .expect("timeline key must exist")
                    .timeline;
                tl.scroll = scroll_val;
                sc_idx += 1;
            } else if bc <= st && bc <= 1.0 {
                let bpm_val = bce.expect("bce").1;
                let section = self.sectionnum + bc * self.rate;
                ensure_timeline(tlcache, section, mode_key);
                let tl = &mut tlcache
                    .get_mut(&f64_to_key(section))
                    .expect("timeline key must exist")
                    .timeline;
                tl.bpm = bpm_val;
                bc_idx += 1;
            } else if st <= 1.0 {
                let stop_val = ste.expect("ste").1;
                let ste_key = ste.expect("ste").0;
                let section = self.sectionnum + ste_key * self.rate;
                ensure_timeline(tlcache, section, mode_key);
                let key = f64_to_key(section);
                let bpm = tlcache
                    .get(&key)
                    .expect("timeline key must exist")
                    .timeline
                    .bpm;
                let stop_us = if bpm != 0.0 {
                    (1000.0 * 1000.0 * 60.0 * 4.0 * stop_val / bpm) as i64
                } else {
                    0
                };
                tlcache
                    .get_mut(&key)
                    .expect("timeline key must exist")
                    .timeline
                    .stop = stop_us;
                st_idx += 1;
            } else {
                break;
            }
        }
    }

    pub fn sectionnum(&self) -> f64 {
        self.sectionnum
    }

    pub fn rate(&self) -> f64 {
        self.rate
    }
}

// ---------------------------------------------------------------------------
// Channel processing context & helpers
// ---------------------------------------------------------------------------

/// Shared parameters for channel processing functions.
struct ChannelContext {
    sectionnum: f64,
    rate: f64,
    base: i32,
    mode_key: i32,
    lnobj: i32,
    lnmode: i32,
}

/// Mutable state shared across channel processing functions.
struct ChannelState<'a> {
    tlcache: &'a mut BTreeMap<u64, TimeLineCache>,
    lnlist: &'a mut Vec<Option<Vec<LnInfo>>>,
    startln: &'a mut Vec<Option<StartLnInfo>>,
    log: &'a mut Vec<DecodeLog>,
}

/// Resolve a WAV index through the wavmap, returning -2 for out-of-range.
fn resolve_wav(data: i32, wavmap: &[i32]) -> i32 {
    if data >= 0 && (data as usize) < wavmap.len() {
        wavmap[data as usize]
    } else {
        -2
    }
}

/// Resolve a BGA index through the bgamap, returning -2 for out-of-range.
fn resolve_bga(data: i32, bgamap: &[i32]) -> i32 {
    if data >= 0 && (data as usize) < bgamap.len() {
        bgamap[data as usize]
    } else {
        -2
    }
}

/// Ensure `lnlist` and `startln` vectors are large enough for `key`.
fn ensure_ln_vecs(
    key: i32,
    lnlist: &mut Vec<Option<Vec<LnInfo>>>,
    startln: &mut Vec<Option<StartLnInfo>>,
) {
    let key_usize = key as usize;
    while lnlist.len() <= key_usize {
        lnlist.push(None);
    }
    while startln.len() <= key_usize {
        startln.push(None);
    }
}

/// Ensure only `lnlist` is large enough for `key`.
fn ensure_lnlist(key: i32, lnlist: &mut Vec<Option<Vec<LnInfo>>>) {
    let key_usize = key as usize;
    while lnlist.len() <= key_usize {
        lnlist.push(None);
    }
}

/// Check if a section falls inside any existing LN range for this key.
fn is_inside_ln(key: i32, section: f64, lnlist: &[Option<Vec<LnInfo>>]) -> bool {
    let key_usize = key as usize;
    if key_usize >= lnlist.len() {
        return false;
    }
    if let Some(ref list) = lnlist[key_usize] {
        for ln_info in list {
            if ln_info.start_section <= section && section <= ln_info.end_section {
                return true;
            }
        }
    }
    false
}

/// Push a new LnInfo entry for a key.
fn push_ln_info(key: i32, info: LnInfo, lnlist: &mut [Option<Vec<LnInfo>>]) {
    let key_usize = key as usize;
    if lnlist[key_usize].is_none() {
        lnlist[key_usize] = Some(Vec::new());
    }
    lnlist[key_usize]
        .as_mut()
        .expect("initialized above")
        .push(info);
}

// ---------------------------------------------------------------------------
// Channel processors
// ---------------------------------------------------------------------------

/// Process normal (visible) notes including LNOBJ conversion.
fn process_normal_notes(
    ctx: &ChannelContext,
    line: &str,
    key: i32,
    wavmap: &[i32],
    state: &mut ChannelState<'_>,
) {
    let results = process_data_collect(line, ctx.base, state.log, "");
    for (pos, data) in results {
        let section = ctx.sectionnum + ctx.rate * pos;
        ensure_timeline(state.tlcache, section, ctx.mode_key);
        let tl_key = f64_to_key(section);

        if state
            .tlcache
            .get(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .exist_note_at(key)
        {
            let tl_time = state
                .tlcache
                .get(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .time();
            state.log.push(DecodeLog::new(
                State::Warning,
                format!(
                    "通常ノート追加時に衝突が発生しました : {}:{}",
                    key + 1,
                    tl_time
                ),
            ));
        }
        if data == ctx.lnobj {
            process_lnobj_note(ctx, key, tl_key, state);
        } else {
            let wav_val = resolve_wav(data, wavmap);
            let note = Note::new_normal(wav_val);
            state
                .tlcache
                .get_mut(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .set_note(key, Some(note));
        }
    }
}

/// Handle LNOBJ: convert previous normal note into LN start, place LN end here.
fn process_lnobj_note(ctx: &ChannelContext, key: i32, tl_key: u64, state: &mut ChannelState<'_>) {
    let tl_section = state
        .tlcache
        .get(&tl_key)
        .expect("timeline key must exist")
        .timeline
        .section();
    let keys_desc: Vec<u64> = state.tlcache.keys().rev().cloned().collect();
    for &ekey in &keys_desc {
        let e_section = state
            .tlcache
            .get(&ekey)
            .expect("timeline key must exist")
            .timeline
            .section();
        if e_section >= tl_section {
            continue;
        }
        if !state
            .tlcache
            .get(&ekey)
            .expect("timeline key must exist")
            .timeline
            .exist_note_at(key)
        {
            continue;
        }
        let note_is_normal = state
            .tlcache
            .get(&ekey)
            .expect("timeline key must exist")
            .timeline
            .note(key)
            .map(|n| n.is_normal())
            .unwrap_or(false);
        let note_is_long_no_pair = state
            .tlcache
            .get(&ekey)
            .expect("timeline key must exist")
            .timeline
            .note(key)
            .map(|n| n.is_long() && n.pair().is_none())
            .unwrap_or(false);

        if note_is_normal {
            let note_wav = state
                .tlcache
                .get(&ekey)
                .expect("timeline key must exist")
                .timeline
                .note(key)
                .expect("exist_note_at check guarantees note exists")
                .wav();
            let mut ln = Note::new_long(note_wav);
            ln.set_long_note_type(ctx.lnmode);
            state
                .tlcache
                .get_mut(&ekey)
                .expect("timeline key must exist")
                .timeline
                .set_note(key, Some(ln));
            let lnend = Note::new_long(-2);
            state
                .tlcache
                .get_mut(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .set_note(key, Some(lnend));
            let start_section = state
                .tlcache
                .get(&ekey)
                .expect("timeline key must exist")
                .timeline
                .section();
            let end_section = state
                .tlcache
                .get(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .section();
            set_long_note_pair_sections(state.tlcache, ekey, tl_key, key);

            ensure_ln_vecs(key, state.lnlist, state.startln);
            push_ln_info(
                key,
                LnInfo {
                    start_section,
                    end_section,
                },
                state.lnlist,
            );
            break;
        } else if note_is_long_no_pair {
            let tl2_section = state
                .tlcache
                .get(&ekey)
                .expect("timeline key must exist")
                .timeline
                .section();
            let tl_section_display = state
                .tlcache
                .get(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .section();
            state.log.push(DecodeLog::new(
                State::Warning,
                format!(
                    "LNレーンで開始定義し、LNオブジェクトで終端定義しています。レーン: {} - Section : {} - {}",
                    key + 1, tl2_section, tl_section_display
                ),
            ));
            let lnend = Note::new_long(-2);
            state
                .tlcache
                .get_mut(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .set_note(key, Some(lnend));
            set_long_note_pair_sections(state.tlcache, ekey, tl_key, key);

            ensure_ln_vecs(key, state.lnlist, state.startln);
            push_ln_info(
                key,
                LnInfo {
                    start_section: tl2_section,
                    end_section: tl_section_display,
                },
                state.lnlist,
            );
            state.startln[key as usize] = None;
            break;
        } else {
            let tl2_time = state
                .tlcache
                .get(&ekey)
                .expect("timeline key must exist")
                .timeline
                .time();
            state.log.push(DecodeLog::new(
                State::Warning,
                format!(
                    "LNオブジェクトの対応が取れません。レーン: {} - Time(ms):{}",
                    key, tl2_time
                ),
            ));
            break;
        }
    }
}

/// Process invisible (hidden) notes.
fn process_invisible_notes(
    ctx: &ChannelContext,
    line: &str,
    key: i32,
    wavmap: &[i32],
    tlcache: &mut BTreeMap<u64, TimeLineCache>,
    log: &mut Vec<DecodeLog>,
) {
    let results = process_data_collect(line, ctx.base, log, "");
    for (pos, data) in results {
        let section = ctx.sectionnum + ctx.rate * pos;
        ensure_timeline(tlcache, section, ctx.mode_key);
        let tl_key = f64_to_key(section);
        let wav_val = resolve_wav(data, wavmap);
        tlcache
            .get_mut(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .set_hidden_note(key, Some(Note::new_normal(wav_val)));
    }
}

/// Process long note (LN) channel data with start/end pairing.
fn process_long_notes(
    ctx: &ChannelContext,
    line: &str,
    key: i32,
    wavmap: &[i32],
    state: &mut ChannelState<'_>,
) {
    let results = process_data_collect(line, ctx.base, state.log, "");
    for (pos, data) in results {
        let section = ctx.sectionnum + ctx.rate * pos;
        ensure_timeline(state.tlcache, section, ctx.mode_key);
        let tl_key = f64_to_key(section);
        let tl_section = state
            .tlcache
            .get(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .section();

        ensure_ln_vecs(key, state.lnlist, state.startln);
        let insideln = is_inside_ln(key, tl_section, state.lnlist);

        if !insideln {
            process_long_note_outside(ctx, key, data, tl_key, tl_section, wavmap, state);
        } else {
            process_long_note_inside_ln(key, tl_key, tl_section, wavmap, data, state);
        }
    }
}

/// Handle an LN data point that is NOT inside an existing LN range.
fn process_long_note_outside(
    ctx: &ChannelContext,
    key: i32,
    data: i32,
    tl_key: u64,
    tl_section: f64,
    wavmap: &[i32],
    state: &mut ChannelState<'_>,
) {
    let key_usize = key as usize;

    if state.startln[key_usize].is_none() {
        // LN start
        if state
            .tlcache
            .get(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .exist_note_at(key)
        {
            let tl_time = state
                .tlcache
                .get(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .time();
            state.log.push(DecodeLog::new(
                State::Warning,
                format!(
                    "LN開始位置に通常ノートが存在します。レーン: {} - Time(ms):{}",
                    key + 1,
                    tl_time
                ),
            ));
            let existing_is_normal = state
                .tlcache
                .get(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .note(key)
                .map(|n| n.is_normal())
                .unwrap_or(false);
            let wav_val = resolve_wav(data, wavmap);
            let existing_wav = state
                .tlcache
                .get(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .note(key)
                .map(|n| n.wav())
                .unwrap_or(0);
            if existing_is_normal && existing_wav != wav_val {
                let note = state
                    .tlcache
                    .get_mut(&tl_key)
                    .expect("timeline key must exist")
                    .timeline
                    .take_note(key);
                if let Some(n) = note {
                    state
                        .tlcache
                        .get_mut(&tl_key)
                        .expect("timeline key must exist")
                        .timeline
                        .add_back_ground_note(n);
                }
            }
        }
        let wav_val = resolve_wav(data, wavmap);
        let ln = Note::new_long(wav_val);
        let ln_section = state
            .tlcache
            .get(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .section();
        state
            .tlcache
            .get_mut(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .set_note(key, Some(ln));
        state.startln[key_usize] = Some(StartLnInfo {
            section: ln_section,
            wav: wav_val,
        });
    } else if state.startln[key_usize]
        .as_ref()
        .map(|s| s.section == f64::MIN)
        .unwrap_or(false)
    {
        state.startln[key_usize] = None;
    } else {
        // LN end processing
        process_long_note_end(ctx, key, data, tl_key, tl_section, wavmap, state);
    }
}

/// Finalize an LN end: pair with start, record in lnlist.
fn process_long_note_end(
    ctx: &ChannelContext,
    key: i32,
    data: i32,
    tl_key: u64,
    tl_section: f64,
    wavmap: &[i32],
    state: &mut ChannelState<'_>,
) {
    let key_usize = key as usize;
    let start_info = *state.startln[key_usize]
        .as_ref()
        .expect("initialized above");
    let keys_desc: Vec<u64> = state.tlcache.keys().rev().cloned().collect();
    for &ekey in &keys_desc {
        let e_section = state
            .tlcache
            .get(&ekey)
            .expect("timeline key must exist")
            .timeline
            .section();
        if e_section >= tl_section {
            continue;
        }

        if e_section == start_info.section {
            if let Some(note) = state
                .tlcache
                .get_mut(&ekey)
                .expect("timeline key must exist")
                .timeline
                .note_mut(key)
            {
                note.set_long_note_type(ctx.lnmode);
            }
            let wav_val = resolve_wav(data, wavmap);
            let noteend_wav = if start_info.wav != wav_val {
                wav_val
            } else {
                -2
            };
            let noteend = Note::new_long(noteend_wav);
            state
                .tlcache
                .get_mut(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .set_note(key, Some(noteend));
            set_long_note_pair_sections(state.tlcache, ekey, tl_key, key);

            let end_section = state
                .tlcache
                .get(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .section();
            push_ln_info(
                key,
                LnInfo {
                    start_section: start_info.section,
                    end_section,
                },
                state.lnlist,
            );

            state.startln[key_usize] = None;
            break;
        } else if state
            .tlcache
            .get(&ekey)
            .expect("timeline key must exist")
            .timeline
            .exist_note_at(key)
        {
            let tl2_time = state
                .tlcache
                .get(&ekey)
                .expect("timeline key must exist")
                .timeline
                .time();
            let existing_is_normal = state
                .tlcache
                .get(&ekey)
                .expect("timeline key must exist")
                .timeline
                .note(key)
                .map(|n| n.is_normal())
                .unwrap_or(false);
            state.log.push(DecodeLog::new(
                State::Warning,
                format!(
                    "LN内に通常ノートが存在します。レーン: {} - Time(ms):{}",
                    key + 1,
                    tl2_time
                ),
            ));
            let note = state
                .tlcache
                .get_mut(&ekey)
                .expect("timeline key must exist")
                .timeline
                .take_note(key);
            if existing_is_normal && let Some(n) = note {
                state
                    .tlcache
                    .get_mut(&ekey)
                    .expect("timeline key must exist")
                    .timeline
                    .add_back_ground_note(n);
            }
        }
    }
}

/// Handle an LN data point that IS inside an existing LN range.
fn process_long_note_inside_ln(
    key: i32,
    tl_key: u64,
    tl_section: f64,
    wavmap: &[i32],
    data: i32,
    state: &mut ChannelState<'_>,
) {
    let key_usize = key as usize;
    if state.startln[key_usize].is_none() {
        let wav_val = resolve_wav(data, wavmap);
        let tl_time = state
            .tlcache
            .get(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .time();
        state.startln[key_usize] = Some(StartLnInfo {
            section: f64::MIN,
            wav: wav_val,
        });
        state.log.push(DecodeLog::new(
            State::Warning,
            format!(
                "LN内にLN開始ノートを定義しようとしています : {} - Section : {} - Time(ms):{}",
                key + 1,
                tl_section,
                tl_time
            ),
        ));
    } else {
        let start_section = state.startln[key_usize]
            .as_ref()
            .expect("initialized above")
            .section;
        if start_section != f64::MIN {
            let start_key = f64_to_key(start_section);
            if state.tlcache.contains_key(&start_key) {
                state
                    .tlcache
                    .get_mut(&start_key)
                    .expect("timeline key must exist")
                    .timeline
                    .set_note(key, None);
            }
        }
        state.startln[key_usize] = None;
        let tl_time = state
            .tlcache
            .get(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .time();
        state.log.push(DecodeLog::new(
            State::Warning,
            format!(
                "LN内にLN終端ノートを定義しようとしています : {} - Section : {} - Time(ms):{}",
                key + 1,
                tl_section,
                tl_time
            ),
        ));
    }
}

/// Process mine notes.
fn process_mine_notes(
    ctx: &ChannelContext,
    line: &str,
    key: i32,
    wavmap: &[i32],
    tlcache: &mut BTreeMap<u64, TimeLineCache>,
    lnlist: &mut Vec<Option<Vec<LnInfo>>>,
    log: &mut Vec<DecodeLog>,
) {
    let results = process_data_collect(line, ctx.base, log, "");
    for (pos, mut data) in results {
        let section = ctx.sectionnum + ctx.rate * pos;
        ensure_timeline(tlcache, section, ctx.mode_key);
        let tl_key = f64_to_key(section);
        let tl_section = tlcache
            .get(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .section();

        let mut insideln = tlcache
            .get(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .exist_note_at(key);
        ensure_lnlist(key, lnlist);
        if !insideln {
            insideln = is_inside_ln(key, tl_section, lnlist);
        }

        if !insideln {
            if ctx.base == 62 {
                let s = chart_decoder::to_base62(data);
                let sb = s.as_bytes();
                data = chart_decoder::parse_int36(sb[0] as char, sb[1] as char);
                if data < 0 {
                    data = 0;
                }
            }
            let wav_val = if !wavmap.is_empty() { wavmap[0] } else { -2 };
            let note = Note::new_mine(wav_val, data as f64);
            tlcache
                .get_mut(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .set_note(key, Some(note));
        } else {
            let tl_time = tlcache
                .get(&tl_key)
                .expect("timeline key must exist")
                .timeline
                .time();
            log.push(DecodeLog::new(
                State::Warning,
                format!(
                    "地雷ノート追加時に衝突が発生しました : {}:{}",
                    key + 1,
                    tl_time
                ),
            ));
        }
    }
}

/// Process autoplay (background) notes.
fn process_autoplay_notes(
    ctx: &ChannelContext,
    line: &str,
    wavmap: &[i32],
    tlcache: &mut BTreeMap<u64, TimeLineCache>,
    log: &mut Vec<DecodeLog>,
) {
    let results = process_data_collect(line, ctx.base, log, "");
    for (pos, data) in results {
        let section = ctx.sectionnum + ctx.rate * pos;
        ensure_timeline(tlcache, section, ctx.mode_key);
        let tl_key = f64_to_key(section);
        let wav_val = resolve_wav(data, wavmap);
        tlcache
            .get_mut(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .add_back_ground_note(Note::new_normal(wav_val));
    }
}

/// Process BGA play channel.
fn process_bga_channel(
    ctx: &ChannelContext,
    line: &str,
    bgamap: &[i32],
    tlcache: &mut BTreeMap<u64, TimeLineCache>,
    log: &mut Vec<DecodeLog>,
) {
    let results = process_data_collect(line, ctx.base, log, "");
    for (pos, data) in results {
        let section = ctx.sectionnum + ctx.rate * pos;
        ensure_timeline(tlcache, section, ctx.mode_key);
        let tl_key = f64_to_key(section);
        let bga_val = resolve_bga(data, bgamap);
        tlcache
            .get_mut(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .bga = bga_val;
    }
}

/// Process layer play channel.
fn process_layer_channel(
    ctx: &ChannelContext,
    line: &str,
    bgamap: &[i32],
    tlcache: &mut BTreeMap<u64, TimeLineCache>,
    log: &mut Vec<DecodeLog>,
) {
    let results = process_data_collect(line, ctx.base, log, "");
    for (pos, data) in results {
        let section = ctx.sectionnum + ctx.rate * pos;
        ensure_timeline(tlcache, section, ctx.mode_key);
        let tl_key = f64_to_key(section);
        let bga_val = resolve_bga(data, bgamap);
        tlcache
            .get_mut(&tl_key)
            .expect("timeline key must exist")
            .timeline
            .layer = bga_val;
    }
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct LnInfo {
    pub start_section: f64,
    pub end_section: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct StartLnInfo {
    pub section: f64,
    pub wav: i32,
}

// ---------------------------------------------------------------------------
// Public utility functions
// ---------------------------------------------------------------------------

pub fn f64_to_key(f: f64) -> u64 {
    if f == 0.0 { 0u64 } else { f.to_bits() }
}

pub fn key_to_f64(k: u64) -> f64 {
    f64::from_bits(k)
}

// ---------------------------------------------------------------------------
// Internal utility functions
// ---------------------------------------------------------------------------

fn ensure_timeline(tlcache: &mut BTreeMap<u64, TimeLineCache>, section: f64, mode_key: i32) {
    let key = f64_to_key(section);
    if tlcache.contains_key(&key) {
        return;
    }

    let mut scroll = 1.0;
    let mut bpm = 0.0;
    let mut time = 0.0;

    for (&k, v) in tlcache.range(..key).rev().take(1) {
        let le_section = key_to_f64(k);
        scroll = v.timeline.scroll;
        bpm = v.timeline.bpm;
        if bpm != 0.0 {
            time = v.time
                + (v.timeline.micro_stop() as f64)
                + (240000.0 * 1000.0 * (section - le_section)) / bpm;
        } else {
            time = v.time + (v.timeline.micro_stop() as f64);
        }
    }

    let mut tl = TimeLine::new(section, time as i64, mode_key);
    tl.bpm = bpm;
    tl.scroll = scroll;
    tlcache.insert(key, TimeLineCache::new(time, tl));
}

fn set_long_note_pair_sections(
    tlcache: &mut BTreeMap<u64, TimeLineCache>,
    start_key: u64,
    end_key: u64,
    lane: i32,
) {
    let end_section = tlcache
        .get(&end_key)
        .expect("timeline key must exist")
        .timeline
        .section();
    let start_section = tlcache
        .get(&start_key)
        .expect("timeline key must exist")
        .timeline
        .section();

    // Read start note type first
    let start_type = tlcache
        .get(&start_key)
        .expect("timeline key must exist")
        .timeline
        .note(lane)
        .map(|n| n.long_note_type())
        .unwrap_or(0);

    if let Some(note) = tlcache
        .get_mut(&start_key)
        .expect("timeline key must exist")
        .timeline
        .note_mut(lane)
        && note.is_long()
    {
        let is_end = start_section > end_section;
        note.set_end(is_end);
    }
    if let Some(note) = tlcache
        .get_mut(&end_key)
        .expect("timeline key must exist")
        .timeline
        .note_mut(lane)
        && note.is_long()
    {
        let is_end = end_section > start_section;
        note.set_end(is_end);
        note.set_long_note_type(start_type);
    }
}

fn split_data(line: &str, base: i32, log: &mut Vec<DecodeLog>, title: &str) -> Vec<i32> {
    let findex = line.find(':').map(|i| i + 1).unwrap_or(0);
    let lindex = line.len();
    let split = (lindex - findex) / 2;
    let bytes = line.as_bytes();
    let mut result = Vec::with_capacity(split);
    for i in 0..split {
        if findex + i * 2 + 1 >= bytes.len() {
            break;
        }
        let c1 = bytes[findex + i * 2] as char;
        let c2 = bytes[findex + i * 2 + 1] as char;
        let val = if base == 62 {
            chart_decoder::parse_int62(c1, c2)
        } else {
            chart_decoder::parse_int36(c1, c2)
        };
        if val == -1 {
            log.push(DecodeLog::new(
                State::Warning,
                format!("{}:チャンネル定義中の不正な値:{}", title, line),
            ));
            result.push(0);
        } else {
            result.push(val);
        }
    }
    result
}

fn process_data_collect(
    line: &str,
    base: i32,
    log: &mut Vec<DecodeLog>,
    title: &str,
) -> Vec<(f64, i32)> {
    let findex = line.find(':').map(|i| i + 1).unwrap_or(0);
    let lindex = line.len();
    let split = (lindex - findex) / 2;
    let bytes = line.as_bytes();
    let mut results = Vec::new();
    for i in 0..split {
        if findex + i * 2 + 1 >= bytes.len() {
            break;
        }
        let c1 = bytes[findex + i * 2] as char;
        let c2 = bytes[findex + i * 2 + 1] as char;
        let result = if base == 62 {
            chart_decoder::parse_int62(c1, c2)
        } else {
            chart_decoder::parse_int36(c1, c2)
        };
        if result > 0 {
            results.push(((i as f64) / (split as f64), result));
        } else if result == -1 {
            log.push(DecodeLog::new(
                State::Warning,
                format!("{}:チャンネル定義中の不正な値:{}", title, line),
            ));
        }
    }
    results
}

fn has_nonzero_data(line: &str, base: i32) -> bool {
    let findex = line.find(':').map(|i| i + 1).unwrap_or(0);
    let lindex = line.len();
    let split = (lindex - findex) / 2;
    let bytes = line.as_bytes();
    for i in 0..split {
        if findex + i * 2 + 1 >= bytes.len() {
            break;
        }
        let c1 = bytes[findex + i * 2] as char;
        let c2 = bytes[findex + i * 2 + 1] as char;
        let result = if base == 62 {
            chart_decoder::parse_int62(c1, c2)
        } else {
            chart_decoder::parse_int36(c1, c2)
        };
        if result > 0 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_decoder::TimeLineCache;

    /// Helper to build a Section from a SECTION_RATE line and return its rate.
    fn section_rate_from_line(rate_str: &str) -> (f64, Vec<DecodeLog>) {
        let mut model = BMSModel::new();
        let bpm = BTreeMap::new();
        let stop = BTreeMap::new();
        let scroll = BTreeMap::new();
        let tables = SectionLookupTables {
            bpm: &bpm,
            stop: &stop,
            scroll: &scroll,
        };
        let mut log = Vec::new();
        // Channel 02 = SECTION_RATE; line format "#NNN02:<value>"
        let line = format!("#00002:{}", rate_str);
        let section = Section::new(&mut model, 0.0, 1.0, true, &[line], &tables, &mut log);
        (section.rate(), log)
    }

    #[test]
    fn test_section_rate_normal_value() {
        let (rate, log) = section_rate_from_line("0.75");
        assert!((rate - 0.75).abs() < f64::EPSILON);
        assert!(log.is_empty());
    }

    #[test]
    fn test_section_rate_rejects_nan() {
        let (rate, log) = section_rate_from_line("NaN");
        // Should fall back to default 1.0
        assert!((rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_section_rate_rejects_infinity() {
        let (rate, log) = section_rate_from_line("inf");
        assert!((rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_section_rate_rejects_negative_infinity() {
        let (rate, log) = section_rate_from_line("-inf");
        assert!((rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_section_rate_rejects_negative() {
        let (rate, log) = section_rate_from_line("-1.0");
        assert!((rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_section_rate_rejects_above_upper_bound() {
        let (rate, log) = section_rate_from_line("1001.0");
        assert!((rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_section_rate_accepts_zero() {
        let (rate, log) = section_rate_from_line("0.0");
        assert!((rate - 0.0).abs() < f64::EPSILON);
        assert!(log.is_empty());
    }

    #[test]
    fn test_section_rate_accepts_upper_bound() {
        let (rate, log) = section_rate_from_line("1000.0");
        assert!((rate - 1000.0).abs() < f64::EPSILON);
        assert!(log.is_empty());
    }

    #[test]
    fn test_ensure_timeline_bpm_zero_no_inf() {
        let mut tlcache: BTreeMap<u64, TimeLineCache> = BTreeMap::new();
        let mode_key = 8;
        let section0 = 0.0;
        let mut tl0 = TimeLine::new(section0, 0, mode_key);
        // Set BPM to 0 to trigger the division-by-zero path
        tl0.bpm = 0.0;
        tlcache.insert(f64_to_key(section0), TimeLineCache::new(0.0, tl0));

        // This should NOT produce Inf or NaN
        ensure_timeline(&mut tlcache, 1.0, mode_key);

        let key = f64_to_key(1.0);
        let entry = tlcache.get(&key).expect("timeline should be created");
        assert!(
            entry.time.is_finite(),
            "time should be finite when BPM is 0, got {}",
            entry.time
        );
        assert_eq!(
            entry.time, 0.0,
            "time should equal previous entry's time when BPM is 0"
        );
    }

    #[test]
    fn test_ensure_timeline_normal_bpm() {
        let mut tlcache: BTreeMap<u64, TimeLineCache> = BTreeMap::new();
        let mode_key = 8;
        let section0 = 0.0;
        let mut tl0 = TimeLine::new(section0, 0, mode_key);
        tl0.bpm = 120.0;
        tlcache.insert(f64_to_key(section0), TimeLineCache::new(0.0, tl0));

        ensure_timeline(&mut tlcache, 1.0, mode_key);

        let key = f64_to_key(1.0);
        let entry = tlcache.get(&key).expect("timeline should be created");
        assert!(
            entry.time.is_finite(),
            "time should be finite with normal BPM"
        );
        // 240000 * 1000 * (1.0 - 0.0) / 120.0 = 2_000_000
        let expected = 240000.0 * 1000.0 * 1.0 / 120.0;
        assert!(
            (entry.time - expected).abs() < 1.0,
            "time should be ~{}, got {}",
            expected,
            entry.time
        );
    }

    #[test]
    fn test_ensure_timeline_already_exists() {
        let mut tlcache: BTreeMap<u64, TimeLineCache> = BTreeMap::new();
        let mode_key = 8;
        let section0 = 0.0;
        let mut tl0 = TimeLine::new(section0, 0, mode_key);
        tl0.bpm = 120.0;
        tlcache.insert(f64_to_key(section0), TimeLineCache::new(42.0, tl0));

        // Calling ensure_timeline for an existing key should be a no-op
        ensure_timeline(&mut tlcache, section0, mode_key);

        let entry = tlcache
            .get(&f64_to_key(section0))
            .expect("entry should exist");
        assert_eq!(entry.time, 42.0, "existing entry should not be modified");
    }

    // --- split_data / process_data_collect bounds-check tests ---

    #[test]
    fn split_data_odd_length_no_panic() {
        // Data portion "0A1" has 3 chars (odd), so split = 1 but the second
        // pair byte would be out of bounds without the guard.
        let mut log = Vec::new();
        let result = split_data("#000XX:0A1", 36, &mut log, "test");
        // Should parse the one complete pair "0A" = 10, and skip the trailing byte
        assert_eq!(result, vec![10]);
    }

    #[test]
    fn process_data_collect_odd_length_no_panic() {
        let mut log = Vec::new();
        let result = process_data_collect("#000XX:0A1", 36, &mut log, "test");
        // "0A" = 10 > 0, position 0/1 = 0.0
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 10);
    }

    #[test]
    fn split_data_empty_data_portion() {
        let mut log = Vec::new();
        let result = split_data("#000XX:", 36, &mut log, "test");
        assert!(result.is_empty());
    }

    #[test]
    fn process_data_collect_empty_data_portion() {
        let mut log = Vec::new();
        let result = process_data_collect("#000XX:", 36, &mut log, "test");
        assert!(result.is_empty());
    }

    /// Helper to build a Section with specific BPM/STOP/SCROLL maps for
    /// testing process_timing_events directly.
    fn section_with_timing(
        sectionnum: f64,
        rate: f64,
        bpmchange: BTreeMap<F64Key, f64>,
        stop: BTreeMap<F64Key, f64>,
        scroll: BTreeMap<F64Key, f64>,
    ) -> Section {
        Section {
            rate,
            poor: Vec::new(),
            sectionnum,
            channellines: Vec::new(),
            bpmchange,
            stop,
            scroll,
        }
    }

    /// Build a tlcache pre-seeded with section 0.0 at BPM 120.
    fn seeded_tlcache(mode_key: i32) -> BTreeMap<u64, TimeLineCache> {
        let mut tlcache: BTreeMap<u64, TimeLineCache> = BTreeMap::new();
        let mut tl = TimeLine::new(0.0, 0, mode_key);
        tl.bpm = 120.0;
        tlcache.insert(f64_to_key(0.0), TimeLineCache::new(0.0, tl));
        tlcache
    }

    #[test]
    fn process_timing_events_guards_bpm_beyond_section_boundary() {
        // BPM change at position 1.5 (beyond section boundary 1.0) should be
        // skipped. Before the fix, only STOP had the <= 1.0 guard.
        let mut bpmchange = BTreeMap::new();
        bpmchange.insert(f64_key(1.5), 200.0);
        let section = section_with_timing(0.0, 1.0, bpmchange, BTreeMap::new(), BTreeMap::new());

        let mode_key = 8;
        let mut tlcache = seeded_tlcache(mode_key);
        section.process_timing_events(&mut tlcache, mode_key);

        // The out-of-range BPM change should NOT create a timeline entry at
        // section 0.0 + 1.5 * 1.0 = 1.5.
        let key = f64_to_key(1.5);
        assert!(
            tlcache.get(&key).is_none(),
            "BPM change beyond section boundary should be skipped"
        );
    }

    #[test]
    fn process_timing_events_guards_scroll_beyond_section_boundary() {
        // SCROLL at position 1.5 (beyond section boundary 1.0) should be
        // skipped. Before the fix, only STOP had the <= 1.0 guard.
        let mut scroll = BTreeMap::new();
        scroll.insert(f64_key(1.5), 2.0);
        let section = section_with_timing(0.0, 1.0, BTreeMap::new(), BTreeMap::new(), scroll);

        let mode_key = 8;
        let mut tlcache = seeded_tlcache(mode_key);
        section.process_timing_events(&mut tlcache, mode_key);

        // The out-of-range SCROLL should NOT create a timeline entry at
        // section 0.0 + 1.5 * 1.0 = 1.5.
        let key = f64_to_key(1.5);
        assert!(
            tlcache.get(&key).is_none(),
            "SCROLL beyond section boundary should be skipped"
        );
    }

    #[test]
    fn process_timing_events_processes_valid_bpm_and_scroll() {
        // Verify that in-range events (position 0.5) are still processed after
        // adding the guards.
        let mut bpmchange = BTreeMap::new();
        bpmchange.insert(f64_key(0.5), 180.0);
        let mut scroll = BTreeMap::new();
        scroll.insert(f64_key(0.25), 2.5);
        let section = section_with_timing(0.0, 1.0, bpmchange, BTreeMap::new(), scroll);

        let mode_key = 8;
        let mut tlcache = seeded_tlcache(mode_key);
        section.process_timing_events(&mut tlcache, mode_key);

        let bpm_key = f64_to_key(0.5);
        let bpm_entry = tlcache.get(&bpm_key).expect("BPM at 0.5 should exist");
        assert!(
            (bpm_entry.timeline.bpm - 180.0).abs() < f64::EPSILON,
            "BPM should be 180.0, got {}",
            bpm_entry.timeline.bpm
        );

        let scroll_key = f64_to_key(0.25);
        let scroll_entry = tlcache
            .get(&scroll_key)
            .expect("SCROLL at 0.25 should exist");
        assert!(
            (scroll_entry.timeline.scroll - 2.5).abs() < f64::EPSILON,
            "scroll should be 2.5, got {}",
            scroll_entry.timeline.scroll
        );
    }
}

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
    // For BTreeMap ordering to match f64 ordering for positive values,
    // we use to_bits() which works correctly for non-negative f64s.
    // For negative values we need to flip. But in this code, section
    // positions are always >= 0, so to_bits() is fine.
    f.to_bits()
}

fn key_f64(k: F64Key) -> f64 {
    f64::from_bits(k)
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model: &mut BMSModel,
        prev_sectionnum: f64,
        prev_rate: f64,
        is_first: bool,
        lines: &[String],
        bpmtable: &BTreeMap<i32, f64>,
        stoptable: &BTreeMap<i32, f64>,
        scrolltable: &BTreeMap<i32, f64>,
        log: &mut Vec<DecodeLog>,
    ) -> Self {
        let base = model.get_base();
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
                            Ok(r) => rate = r,
                            Err(_) => {
                                log.push(DecodeLog::new(
                                    State::Warning,
                                    format!("小節の拡大率が不正です : {}", line),
                                ));
                            }
                        }
                    }
                }
                BPM_CHANGE => {
                    let results = process_data_collect(line, base, log, model.get_title());
                    for (pos, mut data) in results {
                        if base == 62 {
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
                    poor = split_data(line, base, log, model.get_title());
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
                    let results = process_data_collect(line, base, log, model.get_title());
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
                    let results = process_data_collect(line, base, log, model.get_title());
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
                    let results = process_data_collect(line, base, log, model.get_title());
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

    #[allow(clippy::too_many_arguments)]
    pub fn make_time_lines(
        &self,
        model: &mut BMSModel,
        wavmap: &[i32],
        bgamap: &[i32],
        tlcache: &mut BTreeMap<u64, TimeLineCache>,
        lnlist: &mut Vec<Option<Vec<LnInfo>>>,
        startln: &mut Vec<Option<StartLnInfo>>,
        log: &mut Vec<DecodeLog>,
    ) {
        let lnobj = model.lnobj();
        let lnmode = model.lnmode;
        let mode = model.mode().copied();
        let cassign: &[i32; 18] = if mode.as_ref() == Some(&Mode::POPN_9K) {
            &CHANNELASSIGN_POPN
        } else if mode.as_ref() == Some(&Mode::BEAT_7K) || mode.as_ref() == Some(&Mode::BEAT_14K) {
            &CHANNELASSIGN_BEAT7
        } else {
            &CHANNELASSIGN_BEAT5
        };
        let base = model.get_base();
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

        if !self.poor.is_empty() {
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
            tlcache
                .get_mut(&basetl_key)
                .expect("timeline key must exist")
                .timeline
                .eventlayer = vec![layer];
        }

        // BPM changes, stop sequences, scroll
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
            let ste = if st_idx < stops_vec.len() {
                Some(stops_vec[st_idx])
            } else {
                None
            };
            let bce = if bc_idx < bpms_vec.len() {
                Some(bpms_vec[bc_idx])
            } else {
                None
            };
            let sce = if sc_idx < scrolls_vec.len() {
                Some(scrolls_vec[sc_idx])
            } else {
                None
            };

            if ste.is_none() && bce.is_none() && sce.is_none() {
                break;
            }

            let bc = bce.map(|(k, _)| k).unwrap_or(2.0);
            let st = ste.map(|(k, _)| k).unwrap_or(2.0);
            let sc = sce.map(|(k, _)| k).unwrap_or(2.0);

            if sc <= st && sc <= bc {
                let scroll_val = sce.expect("sce").1;
                let section = self.sectionnum + sc * self.rate;
                ensure_timeline(tlcache, section, mode_key);
                let tl = &mut tlcache
                    .get_mut(&f64_to_key(section))
                    .expect("timeline key must exist")
                    .timeline;
                tl.scroll = scroll_val;
                sc_idx += 1;
            } else if bc <= st {
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

            if channel == P1_KEY_BASE {
                let results = process_data_collect(line, base, log, model.get_title());
                for (pos, data) in results {
                    let section = self.sectionnum + self.rate * pos;
                    ensure_timeline(tlcache, section, mode_key);
                    let tl_key = f64_to_key(section);

                    if tlcache
                        .get(&tl_key)
                        .expect("timeline key must exist")
                        .timeline
                        .exist_note_at(key)
                    {
                        let tl_time = tlcache
                            .get(&tl_key)
                            .expect("timeline key must exist")
                            .timeline
                            .time();
                        log.push(DecodeLog::new(
                            State::Warning,
                            format!(
                                "通常ノート追加時に衝突が発生しました : {}:{}",
                                key + 1,
                                tl_time
                            ),
                        ));
                    }
                    if data == lnobj {
                        let tl_section = tlcache
                            .get(&tl_key)
                            .expect("timeline key must exist")
                            .timeline
                            .get_section();
                        let keys_desc: Vec<u64> = tlcache.keys().rev().cloned().collect();
                        for &ekey in &keys_desc {
                            let e_section = tlcache
                                .get(&ekey)
                                .expect("timeline key must exist")
                                .timeline
                                .get_section();
                            if e_section >= tl_section {
                                continue;
                            }
                            if !tlcache
                                .get(&ekey)
                                .expect("timeline key must exist")
                                .timeline
                                .exist_note_at(key)
                            {
                                continue;
                            }
                            let note_is_normal = tlcache
                                .get(&ekey)
                                .expect("timeline key must exist")
                                .timeline
                                .note(key)
                                .map(|n| n.is_normal())
                                .unwrap_or(false);
                            let note_is_long_no_pair = tlcache
                                .get(&ekey)
                                .expect("timeline key must exist")
                                .timeline
                                .note(key)
                                .map(|n| n.is_long() && n.pair().is_none())
                                .unwrap_or(false);

                            if note_is_normal {
                                let note_wav = tlcache
                                    .get(&ekey)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .note(key)
                                    .expect("exist_note_at check guarantees note exists")
                                    .wav();
                                let mut ln = Note::new_long(note_wav);
                                ln.set_long_note_type(lnmode);
                                tlcache
                                    .get_mut(&ekey)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .set_note(key, Some(ln));
                                let lnend = Note::new_long(-2);
                                tlcache
                                    .get_mut(&tl_key)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .set_note(key, Some(lnend));
                                let start_section = tlcache
                                    .get(&ekey)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .get_section();
                                let end_section = tlcache
                                    .get(&tl_key)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .get_section();
                                set_long_note_pair_sections(tlcache, ekey, tl_key, key);

                                let key_usize = key as usize;
                                while lnlist.len() <= key_usize {
                                    lnlist.push(None);
                                }
                                if lnlist[key_usize].is_none() {
                                    lnlist[key_usize] = Some(Vec::new());
                                }
                                lnlist[key_usize].as_mut().expect("initialized above").push(
                                    LnInfo {
                                        start_section,
                                        end_section,
                                    },
                                );
                                break;
                            } else if note_is_long_no_pair {
                                let tl2_section = tlcache
                                    .get(&ekey)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .get_section();
                                let tl_section_display = tlcache
                                    .get(&tl_key)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .get_section();
                                log.push(DecodeLog::new(
                                    State::Warning,
                                    format!(
                                        "LNレーンで開始定義し、LNオブジェクトで終端定義しています。レーン: {} - Section : {} - {}",
                                        key + 1, tl2_section, tl_section_display
                                    ),
                                ));
                                let lnend = Note::new_long(-2);
                                tlcache
                                    .get_mut(&tl_key)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .set_note(key, Some(lnend));
                                set_long_note_pair_sections(tlcache, ekey, tl_key, key);

                                let key_usize = key as usize;
                                while lnlist.len() <= key_usize {
                                    lnlist.push(None);
                                }
                                if lnlist[key_usize].is_none() {
                                    lnlist[key_usize] = Some(Vec::new());
                                }
                                lnlist[key_usize].as_mut().expect("initialized above").push(
                                    LnInfo {
                                        start_section: tl2_section,
                                        end_section: tl_section_display,
                                    },
                                );
                                while startln.len() <= key_usize {
                                    startln.push(None);
                                }
                                startln[key_usize] = None;
                                break;
                            } else {
                                let tl2_time = tlcache
                                    .get(&ekey)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .time();
                                log.push(DecodeLog::new(
                                    State::Warning,
                                    format!(
                                        "LNオブジェクトの対応が取れません。レーン: {} - Time(ms):{}",
                                        key, tl2_time
                                    ),
                                ));
                                break;
                            }
                        }
                    } else {
                        let wav_val = if data >= 0 && (data as usize) < wavmap.len() {
                            wavmap[data as usize]
                        } else {
                            -2
                        };
                        let note = Note::new_normal(wav_val);
                        tlcache
                            .get_mut(&tl_key)
                            .expect("timeline key must exist")
                            .timeline
                            .set_note(key, Some(note));
                    }
                }
            } else if channel == P1_INVISIBLE_KEY_BASE {
                let results = process_data_collect(line, base, log, model.get_title());
                for (pos, data) in results {
                    let section = self.sectionnum + self.rate * pos;
                    ensure_timeline(tlcache, section, mode_key);
                    let tl_key = f64_to_key(section);
                    let wav_val = if data >= 0 && (data as usize) < wavmap.len() {
                        wavmap[data as usize]
                    } else {
                        -2
                    };
                    tlcache
                        .get_mut(&tl_key)
                        .expect("timeline key must exist")
                        .timeline
                        .set_hidden_note(key, Some(Note::new_normal(wav_val)));
                }
            } else if channel == P1_LONG_KEY_BASE {
                let results = process_data_collect(line, base, log, model.get_title());
                for (pos, data) in results {
                    let section = self.sectionnum + self.rate * pos;
                    ensure_timeline(tlcache, section, mode_key);
                    let tl_key = f64_to_key(section);
                    let tl_section = tlcache
                        .get(&tl_key)
                        .expect("timeline key must exist")
                        .timeline
                        .get_section();
                    let key_usize = key as usize;

                    let mut insideln = false;
                    while lnlist.len() <= key_usize {
                        lnlist.push(None);
                    }
                    if let Some(ref list) = lnlist[key_usize] {
                        for ln_info in list {
                            if ln_info.start_section <= tl_section
                                && tl_section <= ln_info.end_section
                            {
                                insideln = true;
                                break;
                            }
                        }
                    }

                    while startln.len() <= key_usize {
                        startln.push(None);
                    }

                    if !insideln {
                        if startln[key_usize].is_none() {
                            if tlcache
                                .get(&tl_key)
                                .expect("timeline key must exist")
                                .timeline
                                .exist_note_at(key)
                            {
                                let tl_time = tlcache
                                    .get(&tl_key)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .time();
                                log.push(DecodeLog::new(
                                    State::Warning,
                                    format!(
                                        "LN開始位置に通常ノートが存在します。レーン: {} - Time(ms):{}",
                                        key + 1, tl_time
                                    ),
                                ));
                                let existing_is_normal = tlcache
                                    .get(&tl_key)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .note(key)
                                    .map(|n| n.is_normal())
                                    .unwrap_or(false);
                                let wav_val = if data >= 0 && (data as usize) < wavmap.len() {
                                    wavmap[data as usize]
                                } else {
                                    -2
                                };
                                let existing_wav = tlcache
                                    .get(&tl_key)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .note(key)
                                    .map(|n| n.wav())
                                    .unwrap_or(0);
                                if existing_is_normal && existing_wav != wav_val {
                                    let note = tlcache
                                        .get_mut(&tl_key)
                                        .expect("timeline key must exist")
                                        .timeline
                                        .take_note(key);
                                    if let Some(n) = note {
                                        tlcache
                                            .get_mut(&tl_key)
                                            .expect("timeline key must exist")
                                            .timeline
                                            .add_back_ground_note(n);
                                    }
                                }
                            }
                            let wav_val = if data >= 0 && (data as usize) < wavmap.len() {
                                wavmap[data as usize]
                            } else {
                                -2
                            };
                            let ln = Note::new_long(wav_val);
                            let ln_section = tlcache
                                .get(&tl_key)
                                .expect("timeline key must exist")
                                .timeline
                                .get_section();
                            tlcache
                                .get_mut(&tl_key)
                                .expect("timeline key must exist")
                                .timeline
                                .set_note(key, Some(ln));
                            startln[key_usize] = Some(StartLnInfo {
                                section: ln_section,
                                wav: wav_val,
                            });
                        } else if startln[key_usize]
                            .as_ref()
                            .map(|s| s.section == f64::MIN)
                            .unwrap_or(false)
                        {
                            startln[key_usize] = None;
                        } else {
                            // LN end processing
                            let start_info =
                                *startln[key_usize].as_ref().expect("initialized above");
                            let keys_desc: Vec<u64> = tlcache.keys().rev().cloned().collect();
                            for &ekey in &keys_desc {
                                let e_section = tlcache
                                    .get(&ekey)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .get_section();
                                if e_section >= tl_section {
                                    continue;
                                }

                                if e_section == start_info.section {
                                    if let Some(note) = tlcache
                                        .get_mut(&ekey)
                                        .expect("timeline key must exist")
                                        .timeline
                                        .note_mut(key)
                                    {
                                        note.set_long_note_type(lnmode);
                                    }
                                    let wav_val = if data >= 0 && (data as usize) < wavmap.len() {
                                        wavmap[data as usize]
                                    } else {
                                        -2
                                    };
                                    let noteend_wav = if start_info.wav != wav_val {
                                        wav_val
                                    } else {
                                        -2
                                    };
                                    let noteend = Note::new_long(noteend_wav);
                                    tlcache
                                        .get_mut(&tl_key)
                                        .expect("timeline key must exist")
                                        .timeline
                                        .set_note(key, Some(noteend));
                                    set_long_note_pair_sections(tlcache, ekey, tl_key, key);

                                    let end_section = tlcache
                                        .get(&tl_key)
                                        .expect("timeline key must exist")
                                        .timeline
                                        .get_section();
                                    if lnlist[key_usize].is_none() {
                                        lnlist[key_usize] = Some(Vec::new());
                                    }
                                    lnlist[key_usize].as_mut().expect("initialized above").push(
                                        LnInfo {
                                            start_section: start_info.section,
                                            end_section,
                                        },
                                    );

                                    startln[key_usize] = None;
                                    break;
                                } else if tlcache
                                    .get(&ekey)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .exist_note_at(key)
                                {
                                    let tl2_time = tlcache
                                        .get(&ekey)
                                        .expect("timeline key must exist")
                                        .timeline
                                        .time();
                                    let existing_is_normal = tlcache
                                        .get(&ekey)
                                        .expect("timeline key must exist")
                                        .timeline
                                        .note(key)
                                        .map(|n| n.is_normal())
                                        .unwrap_or(false);
                                    log.push(DecodeLog::new(
                                        State::Warning,
                                        format!(
                                            "LN内に通常ノートが存在します。レーン: {} - Time(ms):{}",
                                            key + 1, tl2_time
                                        ),
                                    ));
                                    let note = tlcache
                                        .get_mut(&ekey)
                                        .expect("timeline key must exist")
                                        .timeline
                                        .take_note(key);
                                    if existing_is_normal && let Some(n) = note {
                                        tlcache
                                            .get_mut(&ekey)
                                            .expect("timeline key must exist")
                                            .timeline
                                            .add_back_ground_note(n);
                                    }
                                }
                            }
                        }
                    } else if startln[key_usize].is_none() {
                        let wav_val = if data >= 0 && (data as usize) < wavmap.len() {
                            wavmap[data as usize]
                        } else {
                            -2
                        };
                        let tl_time = tlcache
                            .get(&tl_key)
                            .expect("timeline key must exist")
                            .timeline
                            .time();
                        startln[key_usize] = Some(StartLnInfo {
                            section: f64::MIN,
                            wav: wav_val,
                        });
                        log.push(DecodeLog::new(
                            State::Warning,
                            format!(
                                "LN内にLN開始ノートを定義しようとしています : {} - Section : {} - Time(ms):{}",
                                key + 1, tl_section, tl_time
                            ),
                        ));
                    } else {
                        let start_section = startln[key_usize]
                            .as_ref()
                            .expect("initialized above")
                            .section;
                        if start_section != f64::MIN {
                            let start_key = f64_to_key(start_section);
                            if tlcache.contains_key(&start_key) {
                                tlcache
                                    .get_mut(&start_key)
                                    .expect("timeline key must exist")
                                    .timeline
                                    .set_note(key, None);
                            }
                        }
                        startln[key_usize] = None;
                        let tl_time = tlcache
                            .get(&tl_key)
                            .expect("timeline key must exist")
                            .timeline
                            .time();
                        log.push(DecodeLog::new(
                            State::Warning,
                            format!(
                                "LN内にLN終端ノートを定義しようとしています : {} - Section : {} - Time(ms):{}",
                                key + 1, tl_section, tl_time
                            ),
                        ));
                    }
                }
            } else if channel == P1_MINE_KEY_BASE {
                let results = process_data_collect(line, base, log, model.get_title());
                for (pos, mut data) in results {
                    let section = self.sectionnum + self.rate * pos;
                    ensure_timeline(tlcache, section, mode_key);
                    let tl_key = f64_to_key(section);
                    let tl_section = tlcache
                        .get(&tl_key)
                        .expect("timeline key must exist")
                        .timeline
                        .get_section();
                    let key_usize = key as usize;

                    let mut insideln = tlcache
                        .get(&tl_key)
                        .expect("timeline key must exist")
                        .timeline
                        .exist_note_at(key);
                    while lnlist.len() <= key_usize {
                        lnlist.push(None);
                    }
                    if !insideln && let Some(ref list) = lnlist[key_usize] {
                        for ln_info in list {
                            if ln_info.start_section <= tl_section
                                && tl_section <= ln_info.end_section
                            {
                                insideln = true;
                                break;
                            }
                        }
                    }

                    if !insideln {
                        if base == 62 {
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
            } else if channel == LANE_AUTOPLAY {
                let results = process_data_collect(line, base, log, model.get_title());
                for (pos, data) in results {
                    let section = self.sectionnum + self.rate * pos;
                    ensure_timeline(tlcache, section, mode_key);
                    let tl_key = f64_to_key(section);
                    let wav_val = if data >= 0 && (data as usize) < wavmap.len() {
                        wavmap[data as usize]
                    } else {
                        -2
                    };
                    tlcache
                        .get_mut(&tl_key)
                        .expect("timeline key must exist")
                        .timeline
                        .add_back_ground_note(Note::new_normal(wav_val));
                }
            } else if channel == BGA_PLAY {
                let results = process_data_collect(line, base, log, model.get_title());
                for (pos, data) in results {
                    let section = self.sectionnum + self.rate * pos;
                    ensure_timeline(tlcache, section, mode_key);
                    let tl_key = f64_to_key(section);
                    let bga_val = if data >= 0 && (data as usize) < bgamap.len() {
                        bgamap[data as usize]
                    } else {
                        -2
                    };
                    tlcache
                        .get_mut(&tl_key)
                        .expect("timeline key must exist")
                        .timeline
                        .bga = bga_val;
                }
            } else if channel == LAYER_PLAY {
                let results = process_data_collect(line, base, log, model.get_title());
                for (pos, data) in results {
                    let section = self.sectionnum + self.rate * pos;
                    ensure_timeline(tlcache, section, mode_key);
                    let tl_key = f64_to_key(section);
                    let bga_val = if data >= 0 && (data as usize) < bgamap.len() {
                        bgamap[data as usize]
                    } else {
                        -2
                    };
                    tlcache
                        .get_mut(&tl_key)
                        .expect("timeline key must exist")
                        .timeline
                        .layer = bga_val;
                }
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

pub fn f64_to_key(f: f64) -> u64 {
    f.to_bits()
}

pub fn key_to_f64(k: u64) -> f64 {
    f64::from_bits(k)
}

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
        .get_section();
    let start_section = tlcache
        .get(&start_key)
        .expect("timeline key must exist")
        .timeline
        .get_section();

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
}

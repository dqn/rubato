use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;

use md5::Md5;
use sha2::{Digest, Sha256};

use crate::bms_model::{BMSModel, JudgeRankType, LNTYPE_LONGNOTE, TotalType};
use crate::chart_decoder::{self, TimeLineCache};
use crate::chart_information::ChartInformation;
use crate::decode_log::{DecodeLog, State};
use crate::mode::Mode;
use crate::section::{self, Section, f64_to_key};
use crate::time_line::TimeLine;

pub struct BMSDecoder {
    pub lntype: i32,
    pub log: Vec<DecodeLog>,
    wavlist: Vec<String>,
    wm: Vec<i32>,
    bgalist: Vec<String>,
    bm: Vec<i32>,
    lines: Vec<Option<Vec<String>>>,
    scrolltable: BTreeMap<i32, f64>,
    stoptable: BTreeMap<i32, f64>,
    bpmtable: BTreeMap<i32, f64>,
}

impl Default for BMSDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl BMSDecoder {
    pub fn new() -> Self {
        Self::new_with_lntype(LNTYPE_LONGNOTE)
    }

    pub fn new_with_lntype(lntype: i32) -> Self {
        BMSDecoder {
            lntype,
            log: Vec::new(),
            wavlist: Vec::with_capacity(62 * 62),
            wm: vec![-2; 62 * 62],
            bgalist: Vec::with_capacity(62 * 62),
            bm: vec![-2; 62 * 62],
            lines: Vec::new(),
            scrolltable: BTreeMap::new(),
            stoptable: BTreeMap::new(),
            bpmtable: BTreeMap::new(),
        }
    }

    pub fn decode_path(&mut self, f: &Path) -> Option<BMSModel> {
        log::debug!("BMSファイル解析開始 :{}", f.display());
        match std::fs::read(f) {
            Ok(data) => {
                let ispms = f.to_string_lossy().to_lowercase().ends_with(".pms");
                let model = self.decode_internal(Some(f), &data, ispms, None);
                if let Some(ref model) = model {
                    log::debug!(
                        "BMSファイル解析完了 :{} - TimeLine数:{}",
                        f.display(),
                        model.get_all_times().len()
                    );
                }
                model
            }
            Err(_) => {
                self.log
                    .push(DecodeLog::new(State::Error, "BMSファイルが見つかりません"));
                None
            }
        }
    }

    pub fn decode(&mut self, info: ChartInformation) -> Option<BMSModel> {
        self.lntype = info.lntype;
        let path = info.path.clone();
        let selected_randoms = info.selected_randoms.clone();
        match path {
            Some(ref p) => match std::fs::read(p) {
                Ok(data) => {
                    let ispms = p.to_string_lossy().to_lowercase().ends_with(".pms");
                    self.decode_internal(Some(p), &data, ispms, selected_randoms.as_deref())
                }
                Err(_) => {
                    self.log
                        .push(DecodeLog::new(State::Error, "BMSファイルが見つかりません"));
                    None
                }
            },
            None => None,
        }
    }

    pub fn decode_bytes(
        &mut self,
        data: &[u8],
        ispms: bool,
        random: Option<&[i32]>,
    ) -> Option<BMSModel> {
        self.decode_internal(None, data, ispms, random)
    }

    fn decode_internal(
        &mut self,
        path: Option<&Path>,
        data: &[u8],
        ispms: bool,
        selected_random: Option<&[i32]>,
    ) -> Option<BMSModel> {
        self.log.clear();
        let mut model = BMSModel::new();
        self.scrolltable.clear();
        self.stoptable.clear();
        self.bpmtable.clear();

        // Compute MD5 and SHA256
        let mut md5_hasher = Md5::new();
        let mut sha256_hasher = Sha256::new();
        md5_hasher.update(data);
        sha256_hasher.update(data);

        let mut maxsec: usize = 0;

        // Decode MS932 (Shift_JIS) to string (Cow::Borrowed when pure ASCII)
        let (text, _, _) = encoding_rs::SHIFT_JIS.decode(data);

        model.set_mode(if ispms { Mode::POPN_9K } else { Mode::BEAT_5K });

        self.wavlist.clear();
        for v in self.wm.iter_mut() {
            *v = -2;
        }
        self.bgalist.clear();
        for v in self.bm.iter_mut() {
            *v = -2;
        }

        // Ensure lines has 1000 slots
        self.lines.clear();
        self.lines.resize_with(1000, || None);

        let mut randoms: Vec<i32> = Vec::with_capacity(8);
        let mut srandoms: Vec<i32> = Vec::with_capacity(8);
        let mut crandom: Vec<i32> = Vec::with_capacity(8);
        let mut skip: Vec<bool> = Vec::with_capacity(8);

        for line in text.lines() {
            if line.len() < 2 {
                continue;
            }

            let first_char = line.as_bytes()[0] as char;
            if first_char == '#' {
                if matches_reserve_word(line, "RANDOM") {
                    let Some(arg) = line.get(8..) else {
                        continue;
                    };
                    match arg.trim().parse::<i32>() {
                        Ok(r) => {
                            randoms.push(r);
                            if let Some(sr) = selected_random {
                                if randoms.len() - 1 < sr.len() {
                                    crandom.push(sr[randoms.len() - 1]);
                                } else {
                                    crandom.push((rand_f64() * (r as f64)) as i32 + 1);
                                    srandoms.push(*crandom.last().unwrap());
                                }
                            } else {
                                crandom.push((rand_f64() * (r as f64)) as i32 + 1);
                                srandoms.push(*crandom.last().unwrap());
                            }
                        }
                        Err(_) => {
                            self.log.push(DecodeLog::new(
                                State::Warning,
                                "#RANDOMに数字が定義されていません",
                            ));
                        }
                    }
                } else if matches_reserve_word(line, "IF") {
                    if !crandom.is_empty() {
                        let Some(arg) = line.get(4..) else {
                            continue;
                        };
                        match arg.trim().parse::<i32>() {
                            Ok(val) => {
                                skip.push(*crandom.last().unwrap() != val);
                            }
                            Err(_) => {
                                self.log.push(DecodeLog::new(
                                    State::Warning,
                                    "#IFに数字が定義されていません",
                                ));
                            }
                        }
                    } else {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            "#IFに対応する#RANDOMが定義されていません",
                        ));
                    }
                } else if matches_reserve_word(line, "ENDIF") {
                    if !skip.is_empty() {
                        skip.pop();
                    } else {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            format!("ENDIFに対応するIFが存在しません: {}", line),
                        ));
                    }
                } else if matches_reserve_word(line, "ENDRANDOM") {
                    if !crandom.is_empty() {
                        crandom.pop();
                    } else {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            format!("ENDRANDOMに対応するRANDOMが存在しません: {}", line),
                        ));
                    }
                } else if skip.is_empty() || !*skip.last().unwrap() {
                    let c = line.as_bytes()[1] as char;
                    let base = model.get_base();
                    if ('0'..='9').contains(&c) && line.len() > 6 {
                        let c2 = line.as_bytes()[2] as char;
                        let c3 = line.as_bytes()[3] as char;
                        if ('0'..='9').contains(&c2) && ('0'..='9').contains(&c3) {
                            let bar_index = ((c as usize) - ('0' as usize)) * 100
                                + ((c2 as usize) - ('0' as usize)) * 10
                                + ((c3 as usize) - ('0' as usize));
                            if bar_index < 1000 {
                                if self.lines[bar_index].is_none() {
                                    self.lines[bar_index] = Some(Vec::new());
                                }
                                self.lines[bar_index]
                                    .as_mut()
                                    .unwrap()
                                    .push(line.to_owned());
                                maxsec = if maxsec > bar_index {
                                    maxsec
                                } else {
                                    bar_index
                                };
                            }
                        } else {
                            self.log.push(DecodeLog::new(
                                State::Warning,
                                format!("小節に数字が定義されていません : {}", line),
                            ));
                        }
                    } else if matches_reserve_word(line, "BPM") {
                        if line.len() > 4 && line.as_bytes()[4] == b' ' {
                            match line[5..].trim().parse::<f64>() {
                                Ok(bpm) => {
                                    if bpm > 0.0 {
                                        model.set_bpm(bpm);
                                    } else {
                                        self.log.push(DecodeLog::new(
                                            State::Warning,
                                            format!(
                                                "#negative BPMはサポートされていません : {}",
                                                line
                                            ),
                                        ));
                                    }
                                }
                                Err(_) => {
                                    self.log.push(DecodeLog::new(
                                        State::Warning,
                                        format!("#BPMに数字が定義されていません : {}", line),
                                    ));
                                }
                            }
                        } else if line.len() > 7 {
                            let Some(bpm_arg) = line.get(7..) else {
                                continue;
                            };
                            match bpm_arg.trim().parse::<f64>() {
                                Ok(bpm) => {
                                    if bpm > 0.0 {
                                        if base == 62 {
                                            match chart_decoder::parse_int62_str(line, 4) {
                                                Ok(idx) => {
                                                    self.bpmtable.insert(idx, bpm);
                                                }
                                                Err(_) => {
                                                    self.log.push(DecodeLog::new(
                                                        State::Warning,
                                                        format!(
                                                            "#BPMxxに数字が定義されていません : {}",
                                                            line
                                                        ),
                                                    ));
                                                }
                                            }
                                        } else {
                                            match chart_decoder::parse_int36_str(line, 4) {
                                                Ok(idx) => {
                                                    self.bpmtable.insert(idx, bpm);
                                                }
                                                Err(_) => {
                                                    self.log.push(DecodeLog::new(
                                                        State::Warning,
                                                        format!(
                                                            "#BPMxxに数字が定義されていません : {}",
                                                            line
                                                        ),
                                                    ));
                                                }
                                            }
                                        }
                                    } else {
                                        self.log.push(DecodeLog::new(
                                            State::Warning,
                                            format!(
                                                "#negative BPMはサポートされていません : {}",
                                                line
                                            ),
                                        ));
                                    }
                                }
                                Err(_) => {
                                    self.log.push(DecodeLog::new(
                                        State::Warning,
                                        format!("#BPMxxに数字が定義されていません : {}", line),
                                    ));
                                }
                            }
                        }
                    } else if matches_reserve_word(line, "WAV") {
                        if line.len() >= 8 {
                            let parse_result = if base == 62 {
                                chart_decoder::parse_int62_str(line, 4)
                            } else {
                                chart_decoder::parse_int36_str(line, 4)
                            };
                            match parse_result {
                                Ok(idx) => {
                                    let raw = line.get(7..).unwrap_or("").trim();
                                    let file_name = normalize_path_separators(raw);
                                    if (idx as usize) < self.wm.len() {
                                        self.wm[idx as usize] = self.wavlist.len() as i32;
                                    } else {
                                        log::warn!(
                                            "WAV index {} out of bounds (max {})",
                                            idx,
                                            self.wm.len() - 1
                                        );
                                    }
                                    self.wavlist.push(file_name.into_owned());
                                }
                                Err(_) => {
                                    self.log.push(DecodeLog::new(
                                        State::Warning,
                                        format!("#WAVxxは不十分な定義です : {}", line),
                                    ));
                                }
                            }
                        } else {
                            self.log.push(DecodeLog::new(
                                State::Warning,
                                format!("#WAVxxは不十分な定義です : {}", line),
                            ));
                        }
                    } else if matches_reserve_word(line, "BMP") {
                        if line.len() >= 8 {
                            let parse_result = if base == 62 {
                                chart_decoder::parse_int62_str(line, 4)
                            } else {
                                chart_decoder::parse_int36_str(line, 4)
                            };
                            match parse_result {
                                Ok(idx) => {
                                    let raw = line.get(7..).unwrap_or("").trim();
                                    let file_name = normalize_path_separators(raw);
                                    if (idx as usize) < self.bm.len() {
                                        self.bm[idx as usize] = self.bgalist.len() as i32;
                                    } else {
                                        log::warn!(
                                            "BMP index {} out of bounds (max {})",
                                            idx,
                                            self.bm.len() - 1
                                        );
                                    }
                                    self.bgalist.push(file_name.into_owned());
                                }
                                Err(_) => {
                                    self.log.push(DecodeLog::new(
                                        State::Warning,
                                        format!("#BMPxxは不十分な定義です : {}", line),
                                    ));
                                }
                            }
                        } else {
                            self.log.push(DecodeLog::new(
                                State::Warning,
                                format!("#BMPxxは不十分な定義です : {}", line),
                            ));
                        }
                    } else if matches_reserve_word(line, "STOP") {
                        if line.len() >= 9 {
                            let parse_result = if base == 62 {
                                chart_decoder::parse_int62_str(line, 5)
                            } else {
                                chart_decoder::parse_int36_str(line, 5)
                            };
                            match parse_result {
                                Ok(idx) => {
                                    match line.get(8..).unwrap_or("").trim().parse::<f64>() {
                                        Ok(mut stop) => {
                                            stop /= 192.0;
                                            if stop < 0.0 {
                                                stop = stop.abs();
                                                self.log.push(DecodeLog::new(
                                                State::Warning,
                                                format!(
                                                    "#negative STOPはサポートされていません : {}",
                                                    line
                                                ),
                                            ));
                                            }
                                            self.stoptable.insert(idx, stop);
                                        }
                                        Err(_) => {
                                            self.log.push(DecodeLog::new(
                                                State::Warning,
                                                format!(
                                                    "#STOPxxに数字が定義されていません : {}",
                                                    line
                                                ),
                                            ));
                                        }
                                    }
                                }
                                Err(_) => {
                                    self.log.push(DecodeLog::new(
                                        State::Warning,
                                        format!("#STOPxxに数字が定義されていません : {}", line),
                                    ));
                                }
                            }
                        } else {
                            self.log.push(DecodeLog::new(
                                State::Warning,
                                format!("#STOPxxは不十分な定義です : {}", line),
                            ));
                        }
                    } else if matches_reserve_word(line, "SCROLL") {
                        if line.len() >= 11 {
                            let parse_result = if base == 62 {
                                chart_decoder::parse_int62_str(line, 7)
                            } else {
                                chart_decoder::parse_int36_str(line, 7)
                            };
                            match parse_result {
                                Ok(idx) => match line.get(10..).unwrap_or("").trim().parse::<f64>()
                                {
                                    Ok(scroll) => {
                                        self.scrolltable.insert(idx, scroll);
                                    }
                                    Err(_) => {
                                        self.log.push(DecodeLog::new(
                                            State::Warning,
                                            format!(
                                                "#SCROLLxxに数字が定義されていません : {}",
                                                line
                                            ),
                                        ));
                                    }
                                },
                                Err(_) => {
                                    self.log.push(DecodeLog::new(
                                        State::Warning,
                                        format!("#SCROLLxxに数字が定義されていません : {}", line),
                                    ));
                                }
                            }
                        } else {
                            self.log.push(DecodeLog::new(
                                State::Warning,
                                format!("#SCROLLxxは不十分な定義です : {}", line),
                            ));
                        }
                    } else {
                        // Command words
                        let handled = process_command_word(line, &mut model, &mut self.log);
                        let _ = handled;
                    }
                }
            } else if first_char == '%' {
                if let Some(index) = line.find(' ')
                    && line.len() > index + 1
                {
                    model
                        .get_values_mut()
                        .insert(line[1..index].to_string(), line[index + 1..].to_string());
                }
            } else if first_char == '@'
                && let Some(index) = line.find(' ')
                && line.len() > index + 1
            {
                model
                    .get_values_mut()
                    .insert(line[1..index].to_string(), line[index + 1..].to_string());
            }
        }

        model.set_wav_list(std::mem::take(&mut self.wavlist));
        model.set_bga_list(std::mem::take(&mut self.bgalist));

        let mut sections: Vec<Section> = Vec::with_capacity(maxsec + 1);
        let mut prev_sectionnum: f64 = 0.0;
        let mut prev_rate: f64 = 1.0;
        for i in 0..=maxsec {
            let empty_lines: Vec<String> = Vec::new();
            let lines_ref = self.lines[i].as_deref().unwrap_or(&empty_lines);
            let is_first = i == 0;
            let section = Section::new(
                &mut model,
                prev_sectionnum,
                prev_rate,
                is_first,
                lines_ref,
                &self.bpmtable,
                &self.stoptable,
                &self.scrolltable,
                &mut self.log,
            );
            prev_sectionnum = section.get_sectionnum();
            prev_rate = section.get_rate();
            sections.push(section);
        }

        let mode_key = model.get_mode().map(|m| m.key()).unwrap_or(0);
        let mut tlcache: BTreeMap<u64, TimeLineCache> = BTreeMap::new();
        let mut lnlist: Vec<Option<Vec<section::LnInfo>>> = vec![None; mode_key as usize];
        let mut lnendstatus: Vec<Option<section::StartLnInfo>> = vec![None; mode_key as usize];
        let basetl = TimeLine::new(0.0, 0, mode_key);
        let mut basetl = basetl;
        basetl.set_bpm(model.get_bpm());
        tlcache.insert(f64_to_key(0.0), TimeLineCache::new(0.0, basetl));

        for section in &sections {
            section.make_time_lines(
                &mut model,
                &self.wm,
                &self.bm,
                &mut tlcache,
                &mut lnlist,
                &mut lnendstatus,
                &mut self.log,
            );
        }

        let tl_vec: Vec<TimeLine> = tlcache.into_values().map(|tlc| tlc.timeline).collect();
        model.set_all_time_line(tl_vec);

        let all_tl = model.get_all_time_lines();
        if !all_tl.is_empty() && all_tl[0].get_bpm() == 0.0 {
            self.log.push(DecodeLog::new(
                State::Error,
                "開始BPMが定義されていないため、BMS解析に失敗しました",
            ));
            return None;
        }

        for i in 0..lnendstatus.len() {
            if let Some(ref status) = lnendstatus[i] {
                self.log.push(DecodeLog::new(
                    State::Warning,
                    format!(
                        "曲の終端までにLN終端定義されていないLNがあります。lane:{}",
                        i + 1
                    ),
                ));
                if status.section != f64::MIN {
                    // Find the timeline in model's timelines and clear the note
                    for tl in model.get_all_time_lines_mut() {
                        if tl.get_section() == status.section {
                            tl.set_note(i as i32, None);
                            break;
                        }
                    }
                }
            }
        }

        if *model.get_total_type() != TotalType::Bms {
            self.log
                .push(DecodeLog::new(State::Warning, "TOTALが未定義です"));
        }
        if model.get_total() <= 60.0 {
            self.log
                .push(DecodeLog::new(State::Warning, "TOTAL値が少なすぎます"));
        }
        let all_tl = model.get_all_time_lines();
        if !all_tl.is_empty()
            && all_tl[all_tl.len() - 1].get_time() >= model.get_last_time() + 30000
        {
            self.log.push(DecodeLog::new(
                State::Warning,
                "最後のノート定義から30秒以上の余白があります",
            ));
        }
        if model.get_player() > 1
            && (model.get_mode() == Some(&Mode::BEAT_5K)
                || model.get_mode() == Some(&Mode::BEAT_7K))
        {
            self.log.push(DecodeLog::new(
                State::Warning,
                "#PLAYER定義が2以上にもかかわらず2P側のノーツ定義が一切ありません",
            ));
        }
        if model.get_player() == 1
            && (model.get_mode() == Some(&Mode::BEAT_10K)
                || model.get_mode() == Some(&Mode::BEAT_14K))
        {
            self.log.push(DecodeLog::new(
                State::Warning,
                "#PLAYER定義が1にもかかわらず2P側のノーツ定義が存在します",
            ));
        }

        let md5_result = md5_hasher.finalize();
        let sha256_result = sha256_hasher.finalize();
        model.set_md5(convert_hex_string(&md5_result));
        model.set_sha256(convert_hex_string(&sha256_result));

        self.log.push(DecodeLog::new(
            State::Info,
            "#PLAYER定義が1にもかかわらず2P側のノーツ定義が存在します",
        ));

        let final_selected_random = if let Some(sr) = selected_random {
            sr.to_vec()
        } else {
            srandoms.clone()
        };

        model.set_chart_information(ChartInformation::new(
            path.map(|p| p.to_path_buf()),
            self.lntype,
            Some(final_selected_random),
        ));

        if let Some(p) = path {
            self.print_log(p);
        }

        Some(model)
    }

    fn print_log(&self, path: &Path) {
        for l in &self.log {
            match l.state {
                State::Info => {
                    log::info!("{} : {}", path.display(), l.message);
                }
                State::Warning => {
                    log::warn!("{} : {}", path.display(), l.message);
                }
                State::Error => {
                    log::error!("{} : {}", path.display(), l.message);
                }
            }
        }
    }
}

/// Replace backslashes with forward slashes, returning `Cow::Borrowed` when no
/// replacement is needed (the common case on non-Windows paths).
fn normalize_path_separators(s: &str) -> Cow<'_, str> {
    if s.contains('\\') {
        Cow::Owned(s.replace('\\', "/"))
    } else {
        Cow::Borrowed(s)
    }
}

fn matches_reserve_word(line: &str, s: &str) -> bool {
    let len = s.len();
    if line.len() <= len {
        return false;
    }
    let line_bytes = line.as_bytes();
    let s_bytes = s.as_bytes();
    for i in 0..len {
        let c = line_bytes[i + 1];
        let c2 = s_bytes[i];
        if c != c2 && c != c2 + 32 {
            return false;
        }
    }
    true
}

pub fn convert_hex_string(data: &[u8]) -> String {
    let mut sb = String::with_capacity(data.len() * 2);
    for &b in data {
        sb.push(char::from_digit(((b >> 4) & 0xf) as u32, 16).unwrap());
        sb.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    sb
}

fn rand_f64() -> f64 {
    // Simple random - use system time as seed
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos as f64) / 1_000_000_000.0
}

fn process_command_word(line: &str, model: &mut BMSModel, log: &mut Vec<DecodeLog>) -> bool {
    struct CmdDef {
        name: &'static str,
        handler: fn(&mut BMSModel, &str) -> Option<DecodeLog>,
    }

    let commands: &[CmdDef] = &[
        CmdDef {
            name: "PLAYER",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(player) => {
                        if (1..3).contains(&player) {
                            model.set_player(player);
                        } else {
                            return Some(DecodeLog::new(
                                State::Warning,
                                format!("#PLAYERに規定外の数字が定義されています : {}", player),
                            ));
                        }
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#PLAYERに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "GENRE",
            handler: |model, arg| {
                model.set_genre(arg);
                None
            },
        },
        CmdDef {
            name: "TITLE",
            handler: |model, arg| {
                model.set_title(arg);
                None
            },
        },
        CmdDef {
            name: "SUBTITLE",
            handler: |model, arg| {
                model.set_sub_title(arg);
                None
            },
        },
        CmdDef {
            name: "ARTIST",
            handler: |model, arg| {
                model.set_artist(arg);
                None
            },
        },
        CmdDef {
            name: "SUBARTIST",
            handler: |model, arg| {
                model.set_sub_artist(arg);
                None
            },
        },
        CmdDef {
            name: "PLAYLEVEL",
            handler: |model, arg| {
                model.set_playlevel(arg);
                None
            },
        },
        CmdDef {
            name: "RANK",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(rank) => {
                        if (0..5).contains(&rank) {
                            model.set_judgerank(rank);
                            model.set_judgerank_type(JudgeRankType::BmsRank);
                        } else {
                            return Some(DecodeLog::new(
                                State::Warning,
                                format!("#RANKに規定外の数字が定義されています : {}", rank),
                            ));
                        }
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#RANKに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "DEFEXRANK",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(rank) => {
                        if rank >= 1 {
                            model.set_judgerank(rank);
                            model.set_judgerank_type(JudgeRankType::BmsDefexrank);
                        } else {
                            return Some(DecodeLog::new(
                                State::Warning,
                                format!("#DEFEXRANK 1以下はサポートしていません{}", rank),
                            ));
                        }
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#DEFEXRANKに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "TOTAL",
            handler: |model, arg| {
                match arg.parse::<f64>() {
                    Ok(total) => {
                        if total > 0.0 {
                            model.set_total(total);
                            model.set_total_type(TotalType::Bms);
                        } else {
                            return Some(DecodeLog::new(State::Warning, "#TOTALが0以下です"));
                        }
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#TOTALに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "VOLWAV",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(v) => {
                        model.set_volwav(v);
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#VOLWAVに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "STAGEFILE",
            handler: |model, arg| {
                model.set_stagefile(normalize_path_separators(arg).into_owned());
                None
            },
        },
        CmdDef {
            name: "BACKBMP",
            handler: |model, arg| {
                model.set_backbmp(normalize_path_separators(arg).into_owned());
                None
            },
        },
        CmdDef {
            name: "PREVIEW",
            handler: |model, arg| {
                model.set_preview(normalize_path_separators(arg).into_owned());
                None
            },
        },
        CmdDef {
            name: "LNOBJ",
            handler: |model, arg| {
                if model.get_base() == 62 {
                    match chart_decoder::parse_int62_str(arg, 0) {
                        Ok(v) => model.set_lnobj(v),
                        Err(_) => {
                            return Some(DecodeLog::new(
                                State::Warning,
                                "#LNOBJに数字が定義されていません",
                            ));
                        }
                    }
                } else {
                    match i32::from_str_radix(&arg.to_uppercase(), 36) {
                        Ok(v) => model.set_lnobj(v),
                        Err(_) => {
                            return Some(DecodeLog::new(
                                State::Warning,
                                "#LNOBJに数字が定義されていません",
                            ));
                        }
                    }
                }
                None
            },
        },
        CmdDef {
            name: "LNMODE",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(mut lnmode) => {
                        if !(0..=3).contains(&lnmode) {
                            return Some(DecodeLog::new(
                                State::Warning,
                                "#LNMODEに無効な数字が定義されています",
                            ));
                        }
                        // LR2oraja Endless Dream: LR2 does not support LNMODE, suppress modes 1 or 2
                        lnmode = 0;
                        model.set_lnmode(lnmode);
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#LNMODEに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "DIFFICULTY",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(v) => model.set_difficulty(v),
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#DIFFICULTYに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "BANNER",
            handler: |model, arg| {
                model.set_banner(normalize_path_separators(arg).into_owned());
                None
            },
        },
        CmdDef {
            name: "COMMENT",
            handler: |_model, _arg| {
                // #COMMENT: metadata-only, no behavioral effect
                None
            },
        },
        CmdDef {
            name: "BASE",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(base) => {
                        if base != 62 {
                            return Some(DecodeLog::new(
                                State::Warning,
                                "#BASEに無効な数字が定義されています",
                            ));
                        }
                        model.set_base(base);
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#BASEに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
    ];

    for cmd in commands {
        if line.len() > cmd.name.len() + 2 && matches_reserve_word(line, cmd.name) {
            let Some(arg) = line.get(cmd.name.len() + 2..) else {
                continue;
            };
            let arg = arg.trim();
            let result = (cmd.handler)(model, arg);
            if let Some(dl) = result {
                log.push(dl);
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- convert_hex_string tests ---

    #[test]
    fn convert_hex_string_empty() {
        assert_eq!(convert_hex_string(&[]), "");
    }

    #[test]
    fn convert_hex_string_single_byte() {
        assert_eq!(convert_hex_string(&[0x00]), "00");
        assert_eq!(convert_hex_string(&[0xff]), "ff");
        assert_eq!(convert_hex_string(&[0x0a]), "0a");
        assert_eq!(convert_hex_string(&[0xa0]), "a0");
        assert_eq!(convert_hex_string(&[0x42]), "42");
    }

    #[test]
    fn convert_hex_string_multiple_bytes() {
        assert_eq!(convert_hex_string(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
        assert_eq!(convert_hex_string(&[0x01, 0x23, 0x45, 0x67]), "01234567");
    }

    #[test]
    fn convert_hex_string_all_digits() {
        // Test that 0-9 and a-f are produced correctly
        assert_eq!(
            convert_hex_string(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
            "0123456789abcdef"
        );
    }

    // --- matches_reserve_word tests ---

    #[test]
    fn matches_reserve_word_exact_case() {
        assert!(matches_reserve_word("#TITLE test", "TITLE"));
        assert!(matches_reserve_word("#BPM 120", "BPM"));
        assert!(matches_reserve_word("#ARTIST someone", "ARTIST"));
    }

    #[test]
    fn matches_reserve_word_case_insensitive() {
        // The function matches uppercase word against lowercase line content
        assert!(matches_reserve_word("#title test", "TITLE"));
        assert!(matches_reserve_word("#bpm 120", "BPM"));
    }

    #[test]
    fn matches_reserve_word_no_match() {
        assert!(!matches_reserve_word("#GENRE rock", "TITLE"));
        assert!(!matches_reserve_word("#BPM 120", "TITLE"));
    }

    #[test]
    fn matches_reserve_word_too_short() {
        assert!(!matches_reserve_word("#BP", "BPM"));
        assert!(!matches_reserve_word("#", "BPM"));
    }

    // --- BMSDecoder construction tests ---

    #[test]
    fn decoder_new_defaults() {
        let decoder = BMSDecoder::new();
        assert_eq!(decoder.lntype, LNTYPE_LONGNOTE);
        assert!(decoder.log.is_empty());
    }

    #[test]
    fn decoder_new_with_lntype() {
        let decoder = BMSDecoder::new_with_lntype(2);
        assert_eq!(decoder.lntype, 2);
    }

    // --- Header parsing via decode_bytes tests ---

    fn make_bms_bytes(lines: &[&str]) -> Vec<u8> {
        let mut content = String::new();
        for line in lines {
            content.push_str(line);
            content.push('\n');
        }
        content.into_bytes()
    }

    #[test]
    fn decode_title() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#TITLE My Song"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.get_title(), "My Song");
    }

    #[test]
    fn decode_artist() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#ARTIST DJ Test"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_artist(), "DJ Test");
    }

    #[test]
    fn decode_bpm() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 150"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert!((model.get_bpm() - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn decode_playlevel() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#PLAYLEVEL 12"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_playlevel(), "12");
    }

    #[test]
    fn decode_genre() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#GENRE Hardcore"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_genre(), "Hardcore");
    }

    #[test]
    fn decode_subtitle() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SUBTITLE [SPA]"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_sub_title(), "[SPA]");
    }

    #[test]
    fn decode_subartist() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SUBARTIST feat. Vocalist"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_sub_artist(), "feat. Vocalist");
    }

    #[test]
    fn decode_total() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#TOTAL 300"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert!((model.get_total() - 300.0).abs() < f64::EPSILON);
        assert_eq!(model.get_total_type(), &TotalType::Bms);
    }

    #[test]
    fn decode_difficulty() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#DIFFICULTY 4"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_difficulty(), 4);
    }

    #[test]
    fn decode_rank() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#RANK 3"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.get_judgerank(), 3);
        assert_eq!(model.get_judgerank_type(), &JudgeRankType::BmsRank);
    }

    #[test]
    fn decode_stagefile() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#STAGEFILE bg\\stage.bmp"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        // Backslash should be converted to forward slash
        assert_eq!(model.unwrap().get_stagefile(), "bg/stage.bmp");
    }

    #[test]
    fn decode_banner() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#BANNER banner.png"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_banner(), "banner.png");
    }

    #[test]
    fn decode_preview() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#PREVIEW preview.ogg"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_preview(), "preview.ogg");
    }

    #[test]
    fn decode_no_bpm_returns_none() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#TITLE No BPM"]);
        let model = decoder.decode_bytes(&data, false, None);
        // Without BPM defined, the first timeline has BPM 0.0 and decode should fail
        assert!(model.is_none());
    }

    #[test]
    fn decode_pms_sets_popn_mode() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120"]);
        let model = decoder.decode_bytes(&data, true, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_mode(), Some(&Mode::POPN_9K));
    }

    #[test]
    fn decode_bms_sets_beat5k_mode() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_mode(), Some(&Mode::BEAT_5K));
    }

    #[test]
    fn decode_generates_md5_and_sha256() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        // MD5 hash should be 32 hex characters
        assert_eq!(model.get_md5().len(), 32);
        // SHA256 hash should be 64 hex characters
        assert_eq!(model.get_sha256().len(), 64);
        // Should only contain hex digits
        assert!(model.get_md5().chars().all(|c| c.is_ascii_hexdigit()));
        assert!(model.get_sha256().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn decode_player() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#PLAYER 1"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_player(), 1);
    }

    #[test]
    fn decode_volwav() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#VOLWAV 80"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_volwav(), 80);
    }

    #[test]
    fn decode_defexrank() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#DEFEXRANK 100"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.get_judgerank(), 100);
        assert_eq!(model.get_judgerank_type(), &JudgeRankType::BmsDefexrank);
    }

    #[test]
    fn decode_base_62() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#BASE 62"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_base(), 62);
    }

    #[test]
    fn decode_backbmp() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#BACKBMP img\\back.bmp"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_backbmp(), "img/back.bmp");
    }

    #[test]
    fn decode_percent_values() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "%URL http://example.com"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(
            model.get_values().get("URL"),
            Some(&"http://example.com".to_string())
        );
    }

    #[test]
    fn decode_wav_definition() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#WAV01 sound\\kick.wav"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        let wav_list = model.get_wav_list();
        assert!(!wav_list.is_empty());
        // Backslash should be converted to forward slash
        assert!(wav_list.iter().any(|w| w == "sound/kick.wav"));
    }

    #[test]
    fn decode_multiple_headers() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#TITLE Combined Test",
            "#ARTIST Multi Artist",
            "#BPM 180",
            "#PLAYLEVEL 7",
            "#GENRE Trance",
            "#TOTAL 350",
            "#RANK 2",
        ]);
        let model = decoder.decode_bytes(&data, false, None).unwrap();

        assert_eq!(model.get_title(), "Combined Test");
        assert_eq!(model.get_artist(), "Multi Artist");
        assert!((model.get_bpm() - 180.0).abs() < f64::EPSILON);
        assert_eq!(model.get_playlevel(), "7");
        assert_eq!(model.get_genre(), "Trance");
        assert!((model.get_total() - 350.0).abs() < f64::EPSILON);
        assert_eq!(model.get_judgerank(), 2);
    }

    // --- process_command_word tests ---

    #[test]
    fn process_command_word_title() {
        let mut model = BMSModel::new();
        let mut log = Vec::new();
        let handled = process_command_word("#TITLE Hello World", &mut model, &mut log);
        assert!(handled);
        assert_eq!(model.get_title(), "Hello World");
        assert!(log.is_empty());
    }

    #[test]
    fn process_command_word_artist() {
        let mut model = BMSModel::new();
        let mut log = Vec::new();
        let handled = process_command_word("#ARTIST Test Artist", &mut model, &mut log);
        assert!(handled);
        assert_eq!(model.get_artist(), "Test Artist");
    }

    #[test]
    fn process_command_word_unknown() {
        let mut model = BMSModel::new();
        let mut log = Vec::new();
        let handled = process_command_word("#UNKNOWN something", &mut model, &mut log);
        assert!(!handled);
    }

    #[test]
    fn process_command_word_player_valid() {
        let mut model = BMSModel::new();
        let mut log = Vec::new();
        let handled = process_command_word("#PLAYER 1", &mut model, &mut log);
        assert!(handled);
        assert_eq!(model.get_player(), 1);
        assert!(log.is_empty());
    }

    #[test]
    fn process_command_word_player_invalid() {
        let mut model = BMSModel::new();
        let mut log = Vec::new();
        let handled = process_command_word("#PLAYER 5", &mut model, &mut log);
        assert!(handled);
        // Invalid player value should produce a warning
        assert!(!log.is_empty());
    }

    // --- Multi-byte char boundary safety regression tests ---
    //
    // These tests verify that string slicing does not panic when multi-byte
    // UTF-8 characters (e.g., from Shift_JIS decoded text) appear at positions
    // where the old byte-index slicing would land mid-character.

    /// Helper: encode a UTF-8 string as Shift_JIS bytes (mimicking real BMS files).
    fn make_bms_bytes_sjis(lines: &[&str]) -> Vec<u8> {
        let mut content = String::new();
        for line in lines {
            content.push_str(line);
            content.push('\n');
        }
        let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(&content);
        encoded.into_owned()
    }

    #[test]
    fn multibyte_title_no_panic() {
        // #TITLE followed by multi-byte Japanese text
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&[
            "#BPM 120",
            "#TITLE \u{8868}\u{793a}\u{30c6}\u{30b9}\u{30c8}",
        ]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(
            model.unwrap().get_title(),
            "\u{8868}\u{793a}\u{30c6}\u{30b9}\u{30c8}"
        );
    }

    #[test]
    fn multibyte_artist_no_panic() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&["#BPM 120", "#ARTIST \u{97f3}\u{697d}\u{5bb6}"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().get_artist(), "\u{97f3}\u{697d}\u{5bb6}");
    }

    #[test]
    fn multibyte_genre_no_panic() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&[
            "#BPM 120",
            "#GENRE \u{30cf}\u{30fc}\u{30c9}\u{30b3}\u{30a2}",
        ]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(
            model.unwrap().get_genre(),
            "\u{30cf}\u{30fc}\u{30c9}\u{30b3}\u{30a2}"
        );
    }

    #[test]
    fn multibyte_wav_filename_no_panic() {
        // #WAV01 with a Japanese filename
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&["#BPM 120", "#WAV01 \u{97f3}\u{58f0}.wav"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        let wav_list = model.get_wav_list();
        assert!(wav_list.iter().any(|w| w.contains(".wav")));
    }

    #[test]
    fn multibyte_bmp_filename_no_panic() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&["#BPM 120", "#BMP01 \u{80cc}\u{666f}.bmp"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        let bga_list = model.get_bga_list();
        assert!(bga_list.iter().any(|b| b.contains(".bmp")));
    }

    #[test]
    fn multibyte_stagefile_no_panic() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&["#BPM 120", "#STAGEFILE \u{753b}\u{50cf}/stage.bmp"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert!(model.unwrap().get_stagefile().contains("stage.bmp"));
    }

    #[test]
    fn malformed_random_with_multibyte_no_panic() {
        // #RANDOM followed directly by a multi-byte char (no space, no valid number).
        // This used to panic when slicing at byte index 8 mid-character.
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&["#BPM 120", "#RANDOM\u{8868}"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
    }

    #[test]
    fn malformed_if_with_multibyte_no_panic() {
        // #IF followed by multi-byte char
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&[
            "#BPM 120",
            "#RANDOM 2",
            "#IF \u{8868}",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
    }

    #[test]
    fn malformed_bpmxx_with_multibyte_no_panic() {
        // #BPM with multi-byte chars in the index position
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&["#BPM 120", "#BPM\u{8868}\u{793a} 180"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
    }

    #[test]
    fn process_command_word_multibyte_value_no_panic() {
        let mut model = BMSModel::new();
        let mut log = Vec::new();
        let handled = process_command_word(
            "#TITLE \u{8868}\u{793a}\u{30c6}\u{30b9}\u{30c8}",
            &mut model,
            &mut log,
        );
        assert!(handled);
        assert_eq!(
            model.get_title(),
            "\u{8868}\u{793a}\u{30c6}\u{30b9}\u{30c8}"
        );
    }

    #[test]
    fn process_command_word_multibyte_genre_no_panic() {
        let mut model = BMSModel::new();
        let mut log = Vec::new();
        let handled = process_command_word(
            "#GENRE \u{30cf}\u{30fc}\u{30c9}\u{30b3}\u{30a2}",
            &mut model,
            &mut log,
        );
        assert!(handled);
        assert_eq!(
            model.get_genre(),
            "\u{30cf}\u{30fc}\u{30c9}\u{30b3}\u{30a2}"
        );
    }

    #[test]
    fn full_bms_with_multibyte_metadata_no_panic() {
        // Integration test: a complete BMS with all multi-byte metadata fields
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes_sjis(&[
            "#TITLE \u{661f}\u{306e}\u{5668}",
            "#ARTIST \u{4f5c}\u{66f2}\u{8005}",
            "#GENRE \u{30c8}\u{30e9}\u{30f3}\u{30b9}",
            "#SUBTITLE [\u{5225}\u{540d}]",
            "#SUBARTIST feat.\u{6b4c}\u{624b}",
            "#STAGEFILE \u{753b}\u{50cf}\\bg.bmp",
            "#BPM 140",
            "#PLAYLEVEL 12",
            "#RANK 2",
            "#TOTAL 300",
            "#WAV01 \u{97f3}\u{58f0}/kick.wav",
            "#BMP01 \u{80cc}\u{666f}/bg.bmp",
        ]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.get_title(), "\u{661f}\u{306e}\u{5668}");
        assert_eq!(model.get_artist(), "\u{4f5c}\u{66f2}\u{8005}");
        assert_eq!(model.get_genre(), "\u{30c8}\u{30e9}\u{30f3}\u{30b9}");
        assert!((model.get_bpm() - 140.0).abs() < f64::EPSILON);
    }
}

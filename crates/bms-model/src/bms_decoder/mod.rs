use std::collections::BTreeMap;
use std::path::Path;

use md5::Md5;
use sha2::{Digest, Sha256};

#[cfg(test)]
use crate::bms_model::JudgeRankType;
use crate::bms_model::{BMSModel, LNTYPE_LONGNOTE, LnType, TotalType};
use crate::chart_decoder::{self, TimeLineCache};
use crate::chart_information::ChartInformation;
use crate::decode_log::{DecodeLog, State};
use crate::mode::Mode;
use crate::section::{self, Section, f64_to_key};
use crate::time_line::TimeLine;

pub struct BMSDecoder {
    pub lntype: LnType,
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

    pub fn new_with_lntype(lntype: LnType) -> Self {
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
                        model.all_times().len()
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
        let path = info.path;
        let selected_randoms = info.selected_randoms;
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

        // Decode MS932 (Shift_JIS) to string (Cow::Borrowed when pure ASCII)
        let (text, _, _) = encoding_rs::SHIFT_JIS.decode(data);

        model.set_mode(if ispms { Mode::POPN_9K } else { Mode::BEAT_5K });

        self.reset_resource_tables();

        let (maxsec, srandoms) = self.parse_lines(&text, &mut model, selected_random);

        model.wavmap = std::mem::take(&mut self.wavlist);
        model.bgamap = std::mem::take(&mut self.bgalist);

        let sections = self.build_sections(&mut model, maxsec);
        self.build_timelines(&mut model, &sections);
        model.resolve_long_note_pairs();

        // Validate start BPM
        let all_tl = &model.timelines;
        if !all_tl.is_empty() && all_tl[0].bpm == 0.0 {
            self.log.push(DecodeLog::new(
                State::Error,
                "開始BPMが定義されていないため、BMS解析に失敗しました",
            ));
            return None;
        }

        self.validate_model(&model);

        let md5_result = md5_hasher.finalize();
        let sha256_result = sha256_hasher.finalize();
        model.md5 = convert_hex_string(&md5_result);
        model.sha256 = convert_hex_string(&sha256_result);

        let final_selected_random = if let Some(sr) = selected_random {
            sr.to_vec()
        } else {
            srandoms
        };

        model.info = Some(ChartInformation::new(
            path.map(|p| p.to_path_buf()),
            self.lntype,
            Some(final_selected_random),
        ));

        if let Some(p) = path {
            self.print_log(p);
        }

        Some(model)
    }

    // -----------------------------------------------------------------------
    // Phase 1: Reset resource tables
    // -----------------------------------------------------------------------

    fn reset_resource_tables(&mut self) {
        self.wavlist.clear();
        for v in self.wm.iter_mut() {
            *v = -2;
        }
        self.bgalist.clear();
        for v in self.bm.iter_mut() {
            *v = -2;
        }
        self.lines.clear();
        self.lines.resize_with(1000, || None);
    }

    // -----------------------------------------------------------------------
    // Phase 2: Parse all lines (RANDOM/IF, headers, bar data, resources)
    // -----------------------------------------------------------------------

    /// Parse all BMS lines. Returns (max_bar_index, selected_randoms).
    fn parse_lines(
        &mut self,
        text: &str,
        model: &mut BMSModel,
        selected_random: Option<&[i32]>,
    ) -> (usize, Vec<i32>) {
        let mut maxsec: usize = 0;
        let mut random_state = RandomDirectiveState::new();

        for line in text.lines() {
            if line.len() < 2 {
                continue;
            }

            let first_char = line.as_bytes()[0] as char;
            if first_char == '#' {
                if let Some(directive) = RandomDirectiveState::try_parse(line) {
                    random_state.handle_directive(directive, line, selected_random, &mut self.log);
                } else if !random_state.should_skip() {
                    self.parse_header_line(line, model, &mut maxsec);
                }
            } else if !random_state.should_skip() {
                if first_char == '%' {
                    if let Some(index) = line.find(' ')
                        && line.len() > index + 1
                    {
                        model
                            .values
                            .insert(line[1..index].to_string(), line[index + 1..].to_string());
                    }
                } else if first_char == '@'
                    && let Some(index) = line.find(' ')
                    && line.len() > index + 1
                {
                    model
                        .values
                        .insert(line[1..index].to_string(), line[index + 1..].to_string());
                }
            }
        }

        (maxsec, random_state.into_srandoms())
    }

    /// Parse a single non-conditional header line (after # prefix, not RANDOM/IF/ENDIF/ENDRANDOM).
    fn parse_header_line(&mut self, line: &str, model: &mut BMSModel, maxsec: &mut usize) {
        let c = line.as_bytes()[1] as char;
        let base = model.base();

        // Bar data lines: #NNNcc:data
        if c.is_ascii_digit() && line.len() > 6 {
            self.try_collect_bar_data(line, c, maxsec);
            return;
        }

        // Resource/timing table entries
        if self.try_parse_resource_entry(line, base, model) {
            return;
        }

        // Command words (TITLE, ARTIST, RANK, etc.)
        process_command_word(line, model, &mut self.log);
    }

    /// Try to collect a bar data line (#NNNcc:data). Returns true if handled.
    fn try_collect_bar_data(&mut self, line: &str, c: char, maxsec: &mut usize) {
        let c2 = line.as_bytes()[2] as char;
        let c3 = line.as_bytes()[3] as char;
        if c2.is_ascii_digit() && c3.is_ascii_digit() {
            let bar_index = ((c as usize) - ('0' as usize)) * 100
                + ((c2 as usize) - ('0' as usize)) * 10
                + ((c3 as usize) - ('0' as usize));
            if bar_index < 1000 {
                if self.lines[bar_index].is_none() {
                    self.lines[bar_index] = Some(Vec::new());
                }
                self.lines[bar_index]
                    .as_mut()
                    .expect("initialized above")
                    .push(line.to_owned());
                if bar_index > *maxsec {
                    *maxsec = bar_index;
                }
            }
        } else {
            self.log.push(DecodeLog::new(
                State::Warning,
                format!("小節に数字が定義されていません : {}", line),
            ));
        }
    }

    /// Try to parse a resource/timing table entry (#BPM, #WAV, #BMP, #STOP, #SCROLL).
    /// Returns true if the line was handled.
    fn try_parse_resource_entry(&mut self, line: &str, base: i32, model: &mut BMSModel) -> bool {
        if matches_reserve_word(line, "BPM") {
            self.parse_bpm_entry(line, base, model);
            true
        } else if matches_reserve_word(line, "WAV") {
            self.parse_wav_entry(line, base);
            true
        } else if matches_reserve_word(line, "BMP") {
            self.parse_bmp_entry(line, base);
            true
        } else if matches_reserve_word(line, "STOP") {
            self.parse_stop_entry(line, base);
            true
        } else if matches_reserve_word(line, "SCROLL") {
            self.parse_scroll_entry(line, base);
            true
        } else {
            false
        }
    }

    fn parse_bpm_entry(&mut self, line: &str, base: i32, model: &mut BMSModel) {
        if line.len() > 4 && line.as_bytes()[4] == b' ' {
            // #BPM N (initial BPM)
            match line[5..].trim().parse::<f64>() {
                Ok(bpm) => {
                    if bpm > 0.0 {
                        model.bpm = bpm;
                    } else {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            format!("#negative BPMはサポートされていません : {}", line),
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
            // #BPMxx value (extended BPM table)
            let Some(bpm_arg) = line.get(7..) else {
                return;
            };
            match bpm_arg.trim().parse::<f64>() {
                Ok(bpm) => {
                    if bpm > 0.0 {
                        self.parse_indexed_entry(
                            line,
                            base,
                            4,
                            |idx, this| {
                                this.bpmtable.insert(idx, bpm);
                            },
                            "#BPMxxに数字が定義されていません",
                        );
                    } else {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            format!("#negative BPMはサポートされていません : {}", line),
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
    }

    fn parse_wav_entry(&mut self, line: &str, base: i32) {
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
    }

    fn parse_bmp_entry(&mut self, line: &str, base: i32) {
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
    }

    fn parse_stop_entry(&mut self, line: &str, base: i32) {
        if line.len() >= 9 {
            let parse_result = if base == 62 {
                chart_decoder::parse_int62_str(line, 5)
            } else {
                chart_decoder::parse_int36_str(line, 5)
            };
            match parse_result {
                Ok(idx) => match line.get(8..).unwrap_or("").trim().parse::<f64>() {
                    Ok(mut stop) => {
                        stop /= 192.0;
                        if stop < 0.0 {
                            stop = stop.abs();
                            self.log.push(DecodeLog::new(
                                State::Warning,
                                format!("#negative STOPはサポートされていません : {}", line),
                            ));
                        }
                        self.stoptable.insert(idx, stop);
                    }
                    Err(_) => {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            format!("#STOPxxに数字が定義されていません : {}", line),
                        ));
                    }
                },
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
    }

    fn parse_scroll_entry(&mut self, line: &str, base: i32) {
        if line.len() >= 11 {
            let parse_result = if base == 62 {
                chart_decoder::parse_int62_str(line, 7)
            } else {
                chart_decoder::parse_int36_str(line, 7)
            };
            match parse_result {
                Ok(idx) => match line.get(10..).unwrap_or("").trim().parse::<f64>() {
                    Ok(scroll) => {
                        self.scrolltable.insert(idx, scroll);
                    }
                    Err(_) => {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            format!("#SCROLLxxに数字が定義されていません : {}", line),
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
    }

    /// Helper to parse an indexed entry (#XXXyy) with base-36 or base-62 index.
    fn parse_indexed_entry(
        &mut self,
        line: &str,
        base: i32,
        offset: usize,
        on_success: impl FnOnce(i32, &mut Self),
        error_msg: &str,
    ) {
        let parse_result = if base == 62 {
            chart_decoder::parse_int62_str(line, offset)
        } else {
            chart_decoder::parse_int36_str(line, offset)
        };
        match parse_result {
            Ok(idx) => on_success(idx, self),
            Err(_) => {
                self.log.push(DecodeLog::new(
                    State::Warning,
                    format!("{} : {}", error_msg, line),
                ));
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 3: Build sections from collected bar data
    // -----------------------------------------------------------------------

    fn build_sections(&mut self, model: &mut BMSModel, maxsec: usize) -> Vec<Section> {
        let mut sections: Vec<Section> = Vec::with_capacity(maxsec + 1);
        let mut prev_sectionnum: f64 = 0.0;
        let mut prev_rate: f64 = 1.0;
        for i in 0..=maxsec {
            let empty_lines: Vec<String> = Vec::new();
            let lines_ref = self.lines[i].as_deref().unwrap_or(&empty_lines);
            let is_first = i == 0;
            let tables = section::SectionLookupTables {
                bpm: &self.bpmtable,
                stop: &self.stoptable,
                scroll: &self.scrolltable,
            };
            let section = Section::new(
                model,
                prev_sectionnum,
                prev_rate,
                is_first,
                lines_ref,
                &tables,
                &mut self.log,
            );
            prev_sectionnum = section.sectionnum();
            prev_rate = section.rate();
            sections.push(section);
        }
        sections
    }

    // -----------------------------------------------------------------------
    // Phase 4: Build timelines from sections
    // -----------------------------------------------------------------------

    fn build_timelines(&mut self, model: &mut BMSModel, sections: &[Section]) {
        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);
        let mut tlcache: BTreeMap<u64, TimeLineCache> = BTreeMap::new();
        let mut lnlist: Vec<Option<Vec<section::LnInfo>>> = vec![None; mode_key as usize];
        let mut lnendstatus: Vec<Option<section::StartLnInfo>> = vec![None; mode_key as usize];
        let mut basetl = TimeLine::new(0.0, 0, mode_key);
        basetl.bpm = model.bpm;
        tlcache.insert(f64_to_key(0.0), TimeLineCache::new(0.0, basetl));

        let tl_maps = section::TimeLineMaps {
            wavmap: &self.wm,
            bgamap: &self.bm,
        };
        for section in sections {
            section.make_time_lines(
                model,
                &tl_maps,
                &mut tlcache,
                &mut lnlist,
                &mut lnendstatus,
                &mut self.log,
            );
        }

        let tl_vec: Vec<TimeLine> = tlcache.into_values().map(|tlc| tlc.timeline).collect();
        model.timelines = tl_vec;

        // Clean up unterminated LNs
        for (i, lnend) in lnendstatus.iter().enumerate() {
            if let Some(status) = lnend {
                self.log.push(DecodeLog::new(
                    State::Warning,
                    format!(
                        "曲の終端までにLN終端定義されていないLNがあります。lane:{}",
                        i + 1
                    ),
                ));
                if status.section != f64::MIN {
                    for tl in &mut model.timelines {
                        if tl.section() == status.section {
                            tl.set_note(i as i32, None);
                            break;
                        }
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 5: Validate model and produce warnings
    // -----------------------------------------------------------------------

    fn validate_model(&mut self, model: &BMSModel) {
        if model.total_type != TotalType::Bms {
            self.log
                .push(DecodeLog::new(State::Warning, "TOTALが未定義です"));
        }
        if model.total <= 60.0 {
            self.log
                .push(DecodeLog::new(State::Warning, "TOTAL値が少なすぎます"));
        }
        let all_tl = &model.timelines;
        if !all_tl.is_empty() && all_tl[all_tl.len() - 1].time() >= model.last_time() + 30000 {
            self.log.push(DecodeLog::new(
                State::Warning,
                "最後のノート定義から30秒以上の余白があります",
            ));
        }
        if model.player > 1
            && (model.mode() == Some(&Mode::BEAT_5K) || model.mode() == Some(&Mode::BEAT_7K))
        {
            self.log.push(DecodeLog::new(
                State::Warning,
                "#PLAYER定義が2以上にもかかわらず2P側のノーツ定義が一切ありません",
            ));
        }
        if model.player == 1
            && (model.mode() == Some(&Mode::BEAT_10K) || model.mode() == Some(&Mode::BEAT_14K))
        {
            self.log.push(DecodeLog::new(
                State::Warning,
                "#PLAYER定義が1にもかかわらず2P側のノーツ定義が存在します",
            ));
        }
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

// ---------------------------------------------------------------------------
// RANDOM/IF/ENDIF/ENDRANDOM directive state machine
// ---------------------------------------------------------------------------

/// Recognized conditional directives.
enum RandomDirective {
    Random,
    SetRandom,
    If,
    EndIf,
    EndRandom,
    Switch,
    SetSwitch,
    Case,
    Skip,
    Def,
    EndSw,
}

/// Manages the RANDOM/IF/ENDIF/ENDRANDOM and SWITCH/CASE/SKIP/DEF/ENDSW control-flow stack.
struct RandomDirectiveState {
    randoms: Vec<i32>,
    srandoms: Vec<i32>,
    crandom: Vec<i32>,
    skip: Vec<bool>,
    /// Per nesting level: true if this level is a SWITCH block (vs RANDOM block).
    is_switch: Vec<bool>,
    /// Per SWITCH nesting level: true if any #CASE has matched (for fall-through).
    case_matched: Vec<bool>,
    /// Per SWITCH nesting level: true if #SKIP has been hit (skip rest until #ENDSW).
    case_skipped: Vec<bool>,
}

impl RandomDirectiveState {
    fn new() -> Self {
        Self {
            randoms: Vec::with_capacity(8),
            srandoms: Vec::with_capacity(8),
            crandom: Vec::with_capacity(8),
            skip: Vec::with_capacity(8),
            is_switch: Vec::with_capacity(8),
            case_matched: Vec::with_capacity(8),
            case_skipped: Vec::with_capacity(8),
        }
    }

    /// Try to identify which directive a line represents.
    fn try_parse(line: &str) -> Option<RandomDirective> {
        // Check longer keywords before shorter ones that share a prefix:
        // SETRANDOM before RANDOM, SETSWITCH before SWITCH, ENDRANDOM before ENDIF, ENDSW before ENDIF
        if matches_reserve_word(line, "SETRANDOM") {
            Some(RandomDirective::SetRandom)
        } else if matches_reserve_word(line, "RANDOM") {
            Some(RandomDirective::Random)
        } else if matches_reserve_word(line, "IF") {
            Some(RandomDirective::If)
        } else if matches_reserve_word(line, "ENDRANDOM") {
            Some(RandomDirective::EndRandom)
        } else if matches_reserve_word(line, "ENDIF") {
            Some(RandomDirective::EndIf)
        } else if matches_reserve_word(line, "SETSWITCH") {
            Some(RandomDirective::SetSwitch)
        } else if matches_reserve_word(line, "SWITCH") {
            Some(RandomDirective::Switch)
        } else if matches_reserve_word(line, "CASE") {
            Some(RandomDirective::Case)
        } else if matches_reserve_word(line, "SKIP") {
            Some(RandomDirective::Skip)
        } else if matches_reserve_word(line, "DEF")
            && !line
                .as_bytes()
                .get(4)
                .is_some_and(|b| b.is_ascii_alphabetic())
        {
            Some(RandomDirective::Def)
        } else if matches_reserve_word(line, "ENDSW") {
            Some(RandomDirective::EndSw)
        } else {
            None
        }
    }

    fn should_skip(&self) -> bool {
        // Check RANDOM/IF skip stack
        if self.skip.last().copied().unwrap_or(false) {
            return true;
        }
        // Check ALL SWITCH nesting levels, not just the innermost.
        // `is_switch` has one entry per RANDOM or SWITCH block, while
        // `case_matched`/`case_skipped` only have entries for SWITCH blocks.
        // Use a separate counter to index into the SWITCH-only vectors.
        let mut switch_idx = 0;
        for &is_sw in &self.is_switch {
            if is_sw {
                let matched = self.case_matched.get(switch_idx).copied().unwrap_or(false);
                let skipped = self.case_skipped.get(switch_idx).copied().unwrap_or(false);
                if !matched || skipped {
                    return true;
                }
                switch_idx += 1;
            }
        }
        false
    }

    fn handle_directive(
        &mut self,
        directive: RandomDirective,
        line: &str,
        selected_random: Option<&[i32]>,
        log: &mut Vec<DecodeLog>,
    ) {
        match directive {
            RandomDirective::Random => self.handle_random(line, selected_random, log),
            RandomDirective::SetRandom => self.handle_setrandom(line, selected_random, log),
            RandomDirective::If => self.handle_if(line, log),
            RandomDirective::EndIf => self.handle_endif(line, log),
            RandomDirective::EndRandom => self.handle_endrandom(line, log),
            RandomDirective::Switch => self.handle_switch(line, selected_random, log),
            RandomDirective::SetSwitch => self.handle_setswitch(line, selected_random, log),
            RandomDirective::Case => self.handle_case(line, log),
            RandomDirective::Skip => self.handle_skip(log),
            RandomDirective::Def => self.handle_def(log),
            RandomDirective::EndSw => self.handle_endsw(line, log),
        }
    }

    /// Push a random value onto the crandom stack. Used by both #RANDOM and #SWITCH.
    fn push_random_value(&mut self, r: i32, selected_random: Option<&[i32]>) {
        self.randoms.push(r);
        if let Some(sr) = selected_random {
            if self.randoms.len() - 1 < sr.len() {
                self.crandom.push(sr[self.randoms.len() - 1]);
            } else {
                let val = (rand_f64() * (r as f64)) as i32 + 1;
                self.crandom.push(val);
                self.srandoms.push(val);
            }
        } else {
            let val = (rand_f64() * (r as f64)) as i32 + 1;
            self.crandom.push(val);
            self.srandoms.push(val);
        }
    }

    /// Push a deterministic value onto the crandom stack. Used by both #SETRANDOM and #SETSWITCH.
    fn push_set_value(&mut self, n: i32, selected_random: Option<&[i32]>) {
        self.randoms.push(n);
        if let Some(sr) = selected_random {
            if self.randoms.len() - 1 < sr.len() {
                self.crandom.push(sr[self.randoms.len() - 1]);
            } else {
                self.crandom.push(n);
                self.srandoms.push(n);
            }
        } else {
            self.crandom.push(n);
            self.srandoms.push(n);
        }
    }

    fn handle_random(
        &mut self,
        line: &str,
        selected_random: Option<&[i32]>,
        log: &mut Vec<DecodeLog>,
    ) {
        let Some(arg) = line.get(8..) else { return };
        match arg.trim().parse::<i32>() {
            Ok(r) if r >= 1 => {
                self.push_random_value(r, selected_random);
                self.is_switch.push(false);
            }
            Ok(_) => {
                log.push(DecodeLog::new(
                    State::Warning,
                    "#RANDOMの値は1以上である必要があります",
                ));
            }
            Err(_) => {
                log.push(DecodeLog::new(
                    State::Warning,
                    "#RANDOMに数字が定義されていません",
                ));
            }
        }
    }

    fn handle_setrandom(
        &mut self,
        line: &str,
        selected_random: Option<&[i32]>,
        log: &mut Vec<DecodeLog>,
    ) {
        // #SETRANDOM has 10 chars: "#SETRANDOM"
        let Some(arg) = line.get(11..) else { return };
        match arg.trim().parse::<i32>() {
            Ok(n) if n >= 1 => {
                self.push_set_value(n, selected_random);
                self.is_switch.push(false);
            }
            Ok(_) => {
                log.push(DecodeLog::new(
                    State::Warning,
                    "#SETRANDOMの値は1以上である必要があります",
                ));
            }
            Err(_) => {
                log.push(DecodeLog::new(
                    State::Warning,
                    "#SETRANDOMに数字が定義されていません",
                ));
            }
        }
    }

    fn handle_if(&mut self, line: &str, log: &mut Vec<DecodeLog>) {
        if !self.crandom.is_empty() {
            let Some(arg) = line.get(4..) else { return };
            match arg.trim().parse::<i32>() {
                Ok(val) => {
                    self.skip.push(
                        *self
                            .crandom
                            .last()
                            .expect("crandom non-empty checked above")
                            != val,
                    );
                }
                Err(_) => {
                    log.push(DecodeLog::new(
                        State::Warning,
                        "#IFに数字が定義されていません",
                    ));
                }
            }
        } else {
            log.push(DecodeLog::new(
                State::Warning,
                "#IFに対応する#RANDOMが定義されていません",
            ));
            self.skip.push(true);
        }
    }

    fn handle_endif(&mut self, line: &str, log: &mut Vec<DecodeLog>) {
        if !self.skip.is_empty() {
            self.skip.pop();
        } else {
            log.push(DecodeLog::new(
                State::Warning,
                format!("ENDIFに対応するIFが存在しません: {}", line),
            ));
        }
    }

    fn handle_endrandom(&mut self, line: &str, log: &mut Vec<DecodeLog>) {
        if !self.crandom.is_empty() {
            if self.is_switch.last().copied().unwrap_or(false) {
                // Top entry is a SWITCH block, not a RANDOM block.
                // Log warning but do not pop to avoid corrupting SWITCH state.
                log.push(DecodeLog::new(
                    State::Warning,
                    format!("ENDRANDOMに対応するRANDOMが存在しません: {}", line),
                ));
            } else {
                self.crandom.pop();
                self.is_switch.pop();
            }
        } else {
            log.push(DecodeLog::new(
                State::Warning,
                format!("ENDRANDOMに対応するRANDOMが存在しません: {}", line),
            ));
        }
    }

    fn handle_switch(
        &mut self,
        line: &str,
        selected_random: Option<&[i32]>,
        log: &mut Vec<DecodeLog>,
    ) {
        // #SWITCH has 7 chars: "#SWITCH"
        let Some(arg) = line.get(8..) else { return };
        match arg.trim().parse::<i32>() {
            Ok(r) if r >= 1 => {
                self.push_random_value(r, selected_random);
                self.is_switch.push(true);
                self.case_matched.push(false);
                self.case_skipped.push(false);
            }
            Ok(_) => {
                log.push(DecodeLog::new(
                    State::Warning,
                    "#SWITCHの値は1以上である必要があります",
                ));
            }
            Err(_) => {
                log.push(DecodeLog::new(
                    State::Warning,
                    "#SWITCHに数字が定義されていません",
                ));
            }
        }
    }

    fn handle_setswitch(
        &mut self,
        line: &str,
        selected_random: Option<&[i32]>,
        log: &mut Vec<DecodeLog>,
    ) {
        // #SETSWITCH has 10 chars: "#SETSWITCH"
        let Some(arg) = line.get(11..) else { return };
        match arg.trim().parse::<i32>() {
            Ok(n) if n >= 1 => {
                self.push_set_value(n, selected_random);
                self.is_switch.push(true);
                self.case_matched.push(false);
                self.case_skipped.push(false);
            }
            Ok(_) => {
                log.push(DecodeLog::new(
                    State::Warning,
                    "#SETSWITCHの値は1以上である必要があります",
                ));
            }
            Err(_) => {
                log.push(DecodeLog::new(
                    State::Warning,
                    "#SETSWITCHに数字が定義されていません",
                ));
            }
        }
    }

    fn handle_case(&mut self, line: &str, log: &mut Vec<DecodeLog>) {
        if self.crandom.is_empty() {
            log.push(DecodeLog::new(
                State::Warning,
                "#CASEに対応する#SWITCHが定義されていません",
            ));
            return;
        }
        let Some(arg) = line.get(6..) else { return };
        match arg.trim().parse::<i32>() {
            Ok(val) => {
                let crandom_val = *self
                    .crandom
                    .last()
                    .expect("crandom non-empty checked above");
                let already_matched = self.case_matched.last().copied().unwrap_or(false);
                let already_skipped = self.case_skipped.last().copied().unwrap_or(false);

                if already_matched && already_skipped {
                    // A previous case matched and #SKIP was hit.
                    // This case and all subsequent content stay skipped until #ENDSW.
                } else if already_matched {
                    // A previous case matched without #SKIP (fall-through).
                    // Continue including content.
                } else if crandom_val == val {
                    // This case matches -- mark it and start including content.
                    if let Some(matched) = self.case_matched.last_mut() {
                        *matched = true;
                    }
                }
                // If not matched and no fall-through, content is skipped
                // (handled by should_skip checking case_matched)
            }
            Err(_) => {
                log.push(DecodeLog::new(
                    State::Warning,
                    "#CASEに数字が定義されていません",
                ));
            }
        }
    }

    fn handle_skip(&mut self, log: &mut Vec<DecodeLog>) {
        if self.case_skipped.is_empty() {
            log.push(DecodeLog::new(
                State::Warning,
                "#SKIPに対応する#SWITCHが定義されていません",
            ));
            return;
        }
        // #SKIP only takes effect when a case has been matched.
        // When no case matched, content is already skipped via should_skip(),
        // and we must NOT set case_skipped because that would prevent
        // subsequent #CASE directives from matching.
        let matched = self.case_matched.last().copied().unwrap_or(false);
        if matched && let Some(skipped) = self.case_skipped.last_mut() {
            *skipped = true;
        }
    }

    fn handle_def(&mut self, log: &mut Vec<DecodeLog>) {
        if self.crandom.is_empty() {
            log.push(DecodeLog::new(
                State::Warning,
                "#DEFに対応する#SWITCHが定義されていません",
            ));
            return;
        }
        let already_matched = self.case_matched.last().copied().unwrap_or(false);
        let already_skipped = self.case_skipped.last().copied().unwrap_or(false);

        if already_matched && already_skipped {
            // A case already matched and was ended with #SKIP.
            // Default case should NOT activate.
        } else if already_matched {
            // Fall-through from a previous case continues.
            // Content keeps being included.
        } else {
            // No case matched yet - default case activates.
            if let Some(matched) = self.case_matched.last_mut() {
                *matched = true;
            }
        }
    }

    fn handle_endsw(&mut self, line: &str, log: &mut Vec<DecodeLog>) {
        if !self.crandom.is_empty() {
            if self.is_switch.last().copied().unwrap_or(false) {
                self.crandom.pop();
                self.is_switch.pop();
                self.case_matched.pop();
                self.case_skipped.pop();
            } else {
                // Top entry is a RANDOM block, not a SWITCH block.
                // Log warning but do not pop to avoid corrupting RANDOM state.
                log.push(DecodeLog::new(
                    State::Warning,
                    format!("ENDSWに対応するSWITCHが存在しません: {}", line),
                ));
            }
        } else {
            log.push(DecodeLog::new(
                State::Warning,
                format!("ENDSWに対応するSWITCHが存在しません: {}", line),
            ));
        }
    }

    fn into_srandoms(self) -> Vec<i32> {
        self.srandoms
    }
}

mod helpers;
pub use helpers::convert_hex_string;
use helpers::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms_model::LNTYPE_HELLCHARGENOTE;

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

    #[test]
    fn matches_reserve_word_non_letter_no_false_positive() {
        // '@' (64) + 32 = '`' (96). The old manual ASCII shift would falsely
        // match '`' against '@'. eq_ignore_ascii_case must reject this.
        assert!(!matches_reserve_word("#`", "@"));
        // Also verify that non-letter uppercase chars don't match their +32 counterpart
        assert!(!matches_reserve_word("#[", ";"));
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
        let decoder = BMSDecoder::new_with_lntype(LNTYPE_HELLCHARGENOTE);
        assert_eq!(decoder.lntype, LNTYPE_HELLCHARGENOTE);
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
        assert_eq!(model.title, "My Song");
    }

    #[test]
    fn decode_artist() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#ARTIST DJ Test"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().artist, "DJ Test");
    }

    #[test]
    fn decode_bpm() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 150"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert!((model.bpm - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn decode_bpm_case_insensitive() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#bpm 200"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert!((model.unwrap().bpm - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn decode_negative_bpm_rejected() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM -50", "#BPM 100"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        // Negative BPM is rejected but fallback to last valid
        assert!((model.unwrap().bpm - 100.0).abs() < f64::EPSILON);
        assert!(decoder.log.iter().any(|l| l.message.contains("negative")));
    }

    #[test]
    fn decode_player_valid() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#PLAYER 1"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().player, 1);
    }

    #[test]
    fn decode_player_3_double_play_accepted() {
        // Regression: #PLAYER 3 (double play) was rejected because (1..3) excludes 3.
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#PLAYER 3"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().player, 3);
        assert!(
            !decoder
                .log
                .iter()
                .any(|l| l.message.contains("#PLAYERに規定外の数字")),
            "#PLAYER 3 should be accepted without warning"
        );
    }

    #[test]
    fn decode_player_invalid() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#PLAYER 5"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#PLAYERに規定外の数字"))
        );
    }

    #[test]
    fn decode_rank() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#RANK 2"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.judgerank, 2);
        assert_eq!(model.judgerank_type, JudgeRankType::BmsRank);
    }

    #[test]
    fn decode_total() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#TOTAL 300"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert!((model.total - 300.0).abs() < f64::EPSILON);
        assert_eq!(model.total_type, TotalType::Bms);
    }

    #[test]
    fn decode_playlevel() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#PLAYLEVEL 12"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().playlevel, "12");
    }

    #[test]
    fn decode_lnobj() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#LNOBJ ZZ"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        // ZZ in base-36 = 35*36+35 = 1295
        assert_eq!(model.unwrap().lnobj, 1295);
    }

    #[test]
    fn decode_stagefile() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#STAGEFILE bg\\image.bmp"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().stagefile, "bg/image.bmp");
    }

    #[test]
    fn decode_wav_entry() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#WAV01 kick.wav",
            "#WAV02 snare.wav",
            "#00111:0102",
        ]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert!(model.wavmap.len() >= 2);
        assert_eq!(model.wavmap[0], "kick.wav");
        assert_eq!(model.wavmap[1], "snare.wav");
    }

    #[test]
    fn decode_bmp_entry() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#BMP01 bg.bmp"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert!(!model.bgamap.is_empty());
        assert_eq!(model.bgamap[0], "bg.bmp");
    }

    #[test]
    fn decode_random_if() {
        let mut decoder = BMSDecoder::new();
        // With selected_random = [1], #IF 1 body should execute
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#RANDOM 2",
            "#IF 1",
            "#TITLE RandomTitle",
            "#ENDIF",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1]));
        assert!(model.is_some());
        assert_eq!(model.unwrap().title, "RandomTitle");
    }

    #[test]
    fn decode_random_if_skip() {
        let mut decoder = BMSDecoder::new();
        // With selected_random = [2], #IF 1 body should be skipped
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#RANDOM 2",
            "#IF 1",
            "#TITLE SkippedTitle",
            "#ENDIF",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[2]));
        assert!(model.is_some());
        // Title should remain default (empty)
        assert_eq!(model.unwrap().title, "");
    }

    #[test]
    fn decode_percent_value() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "%URL http://example.com"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(
            model.values.get("URL").map(|s| s.as_str()),
            Some("http://example.com")
        );
    }

    #[test]
    fn decode_at_value() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "@KEY somevalue"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(
            model.values.get("KEY").map(|s| s.as_str()),
            Some("somevalue")
        );
    }

    #[test]
    fn decode_pms_mode() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120"]);
        let model = decoder.decode_bytes(&data, true, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().mode(), Some(&Mode::POPN_9K));
    }

    #[test]
    fn decode_no_bpm_returns_none() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#TITLE NoBPM"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_none());
    }

    #[test]
    fn decode_stop_entry() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#STOP01 192"]);
        let _ = decoder.decode_bytes(&data, false, None);
        // STOP value = 192 / 192 = 1.0
        assert!((decoder.stoptable.get(&1).copied().unwrap_or(0.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn decode_md5_sha256_populated() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert!(!model.md5.is_empty());
        assert!(!model.sha256.is_empty());
        assert_eq!(model.md5.len(), 32); // MD5 hex = 32 chars
        assert_eq!(model.sha256.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn decode_defexrank() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#DEFEXRANK 100"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.judgerank, 100);
        assert_eq!(model.judgerank_type, JudgeRankType::BmsDefexrank);
    }

    #[test]
    fn decode_genre() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#GENRE Techno"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().genre, "Techno");
    }

    #[test]
    fn decode_subtitle() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SUBTITLE -remix-"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().sub_title, "-remix-");
    }

    #[test]
    fn decode_subartist() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SUBARTIST feat. Someone"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().subartist, "feat. Someone");
    }

    #[test]
    fn decode_difficulty() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#DIFFICULTY 3"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().difficulty, 3);
    }

    #[test]
    fn decode_volwav() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#VOLWAV 80"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().volwav, 80);
    }

    #[test]
    fn decode_base62() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#BASE 62"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().base(), 62);
    }

    #[test]
    fn decode_banner() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#BANNER img\\banner.png"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().banner, "img/banner.png");
    }

    #[test]
    fn decode_preview() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#PREVIEW preview.ogg"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().preview, "preview.ogg");
    }

    #[test]
    fn decode_backbmp() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#BACKBMP bg\\back.bmp"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().backbmp, "bg/back.bmp");
    }

    #[test]
    fn decode_total_low_warning() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#TOTAL 50"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("TOTAL値が少なすぎます"))
        );
    }

    #[test]
    fn decode_nested_random_if() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#RANDOM 3",
            "#IF 2",
            "#RANDOM 2",
            "#IF 1",
            "#TITLE NestedMatch",
            "#ENDIF",
            "#ENDRANDOM",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[2, 1]));
        assert!(model.is_some());
        assert_eq!(model.unwrap().title, "NestedMatch");
    }

    #[test]
    fn decode_endrandom_without_random_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#ENDRANDOM"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(decoder.log.iter().any(|l| {
            l.message
                .contains("ENDRANDOMに対応するRANDOMが存在しません")
        }));
    }

    #[test]
    fn decode_endif_without_if_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#ENDIF"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("ENDIFに対応するIFが存在しません"))
        );
    }

    #[test]
    fn decode_if_without_random_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#IF 1", "#ENDIF"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(decoder.log.iter().any(|l| {
            l.message
                .contains("#IFに対応する#RANDOMが定義されていません")
        }));
    }

    #[test]
    fn decode_random_zero_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#RANDOM 0"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#RANDOMの値は1以上"))
        );
    }

    // -- Edge case: empty input --

    #[test]
    fn decode_empty_bytes_returns_none() {
        let mut decoder = BMSDecoder::new();
        let result = decoder.decode_bytes(&[], false, None);
        // Empty file has no BPM defined, so first timeline BPM is 0 => None
        assert!(result.is_none());
    }

    #[test]
    fn decode_only_whitespace_returns_none() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["   ", "\t", ""]);
        let result = decoder.decode_bytes(&data, false, None);
        assert!(result.is_none());
    }

    // -- Edge case: BPM edge values --

    #[test]
    fn decode_bpm_negative_warns_and_uses_zero() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM -120"]);
        let result = decoder.decode_bytes(&data, false, None);
        // Negative BPM is rejected, BPM stays 0 => returns None
        assert!(result.is_none());
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("negative BPM"))
        );
    }

    #[test]
    fn decode_bpm_very_large() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 999999"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().bpm, 999999.0);
    }

    #[test]
    fn decode_bpm_fractional() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 133.33"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert!((model.unwrap().bpm - 133.33).abs() < 0.01);
    }

    #[test]
    fn decode_bpm_non_numeric_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM abc"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("数字が定義されていません"))
        );
    }

    // -- Edge case: title with special characters --

    #[test]
    fn decode_title_with_spaces_and_symbols() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#TITLE [HARD] Song ~remix~ (ver.2)"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().title, "[HARD] Song ~remix~ (ver.2)");
    }

    // -- Edge case: TOTAL --

    #[test]
    fn decode_total_zero_warns_and_keeps_default() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#TOTAL 0"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        // TOTAL 0 is rejected (not > 0), so model.total stays at default (100.0)
        assert_eq!(model.unwrap().total, 100.0);
        assert!(decoder.log.iter().any(|l| l.message.contains("TOTAL")));
    }

    #[test]
    fn decode_total_negative_warns_and_keeps_default() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#TOTAL -100"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        // Negative total is rejected (not > 0), model.total stays at default
        assert_eq!(model.unwrap().total, 100.0);
        assert!(decoder.log.iter().any(|l| l.message.contains("TOTAL")));
    }

    // -- Edge case: multiple headers override each other --

    #[test]
    fn decode_duplicate_title_uses_last() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#TITLE First Title", "#TITLE Second Title"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().title, "Second Title");
    }

    #[test]
    fn decode_duplicate_bpm_uses_last() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 100", "#BPM 200"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().bpm, 200.0);
    }

    // -- Edge case: difficulty boundary values --

    #[test]
    fn decode_difficulty_zero() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#DIFFICULTY 0"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().difficulty, 0);
    }

    #[test]
    fn decode_difficulty_non_numeric_stays_default() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#DIFFICULTY abc"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        assert_eq!(model.unwrap().difficulty, 0); // default
    }

    // -- Edge case: playlevel --

    #[test]
    fn decode_playlevel_with_star_prefix_preserved() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#PLAYLEVEL *12"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        // Star prefix is preserved as-is (playlevel is a string field)
        let model = model.unwrap();
        assert_eq!(model.playlevel, "*12");
    }

    // -- Edge case: PMS mode --

    #[test]
    fn decode_pms_mode_sets_popn_9k() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120"]);
        let model = decoder.decode_bytes(&data, true, None).unwrap();
        use crate::mode::Mode;
        assert_eq!(*model.mode().unwrap(), Mode::POPN_9K);
    }

    #[test]
    fn decode_non_pms_defaults_to_beat_5k() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        use crate::mode::Mode;
        assert_eq!(*model.unwrap().mode().unwrap(), Mode::BEAT_5K);
    }

    // -- Edge case: LNOBJ values --

    #[test]
    fn decode_lnobj_lowercase() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#LNOBJ zz"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        // zz in base-36 = 35*36+35 = 1295
        assert_eq!(model.unwrap().lnobj, 1295);
    }

    // -- Edge case: % and @ value lines --

    #[test]
    fn decode_percent_and_at_values() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "%URL http://example.com", "@MAIL test@test.com"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(
            model.values.get("URL"),
            Some(&"http://example.com".to_string())
        );
        assert_eq!(model.values.get("MAIL"), Some(&"test@test.com".to_string()));
    }

    // -- Edge case: bar index boundary --

    #[test]
    fn decode_bar_index_999_is_valid() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#WAV01 kick.wav", "#99911:01"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
    }

    // -- Edge case: MD5/SHA256 hashing --

    #[test]
    fn decode_produces_consistent_hashes() {
        let data = make_bms_bytes(&["#BPM 120", "#TITLE Hash Test"]);
        let mut decoder1 = BMSDecoder::new();
        let model1 = decoder1.decode_bytes(&data, false, None).unwrap();

        let mut decoder2 = BMSDecoder::new();
        let model2 = decoder2.decode_bytes(&data, false, None).unwrap();

        assert_eq!(model1.md5, model2.md5);
        assert_eq!(model1.sha256, model2.sha256);
        assert!(!model1.md5.is_empty());
        assert!(!model1.sha256.is_empty());
    }

    #[test]
    fn decode_different_data_produces_different_hashes() {
        let data1 = make_bms_bytes(&["#BPM 120", "#TITLE Song A"]);
        let data2 = make_bms_bytes(&["#BPM 120", "#TITLE Song B"]);

        let mut decoder1 = BMSDecoder::new();
        let model1 = decoder1.decode_bytes(&data1, false, None).unwrap();

        let mut decoder2 = BMSDecoder::new();
        let model2 = decoder2.decode_bytes(&data2, false, None).unwrap();

        assert_ne!(model1.md5, model2.md5);
        assert_ne!(model1.sha256, model2.sha256);
    }

    // -- Edge case: decoder reuse --

    #[test]
    fn decoder_reuse_clears_previous_state() {
        let mut decoder = BMSDecoder::new();

        let data1 = make_bms_bytes(&["#BPM 120", "#TITLE First"]);
        let model1 = decoder.decode_bytes(&data1, false, None);
        assert_eq!(model1.unwrap().title, "First");

        let data2 = make_bms_bytes(&["#BPM 150", "#TITLE Second"]);
        let model2 = decoder.decode_bytes(&data2, false, None);
        assert_eq!(model2.unwrap().title, "Second");
    }

    // -- Edge case: short lines --

    #[test]
    fn decode_single_char_line_is_ignored() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#", "X"]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
    }

    // -- RANDOM/IF/ENDIF directive tests --

    #[test]
    fn orphaned_if_without_random_skips_content() {
        // An orphaned #IF (no preceding #RANDOM) should skip its body.
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#TITLE Correct",
            "#IF 1",
            "#TITLE Wrong",
            "#ENDIF",
        ]);
        let model = decoder.decode_bytes(&data, false, None);
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.title, "Correct");
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#IFに対応する#RANDOM")),
            "Expected warning about orphaned #IF"
        );
    }

    // --- #SETRANDOM tests ---

    #[test]
    fn decode_setrandom_sets_deterministic_value() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SETRANDOM 2",
            "#IF 2",
            "#TITLE SetRandomMatch",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        let model = decoder.decode_bytes(&data, false, None).unwrap();
        assert_eq!(model.title, "SetRandomMatch");
    }

    #[test]
    fn decode_setrandom_non_matching_skips() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SETRANDOM 2",
            "#IF 1",
            "#TITLE SkippedBySetRandom",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        let model = decoder.decode_bytes(&data, false, None).unwrap();
        assert_eq!(model.title, "");
    }

    #[test]
    fn decode_setrandom_selected_random_overrides() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SETRANDOM 2",
            "#IF 3",
            "#TITLE OverriddenBySelectedRandom",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[3])).unwrap();
        assert_eq!(model.title, "OverriddenBySelectedRandom");
    }

    #[test]
    fn decode_setrandom_invalid_value_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SETRANDOM 0"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#SETRANDOMの値は1以上"))
        );
    }

    #[test]
    fn decode_setrandom_non_numeric_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SETRANDOM abc"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#SETRANDOMに数字が定義されていません"))
        );
    }

    // --- #SWITCH / #CASE / #ENDSW tests ---

    #[test]
    fn decode_switch_case_matching() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 3",
            "#CASE 1",
            "#TITLE Case1",
            "#CASE 2",
            "#TITLE Case2",
            "#CASE 3",
            "#TITLE Case3",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[2])).unwrap();
        assert_eq!(model.title, "Case3");
    }

    #[test]
    fn decode_switch_case_with_skip() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 3",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#CASE 2",
            "#TITLE Case2",
            "#SKIP",
            "#CASE 3",
            "#TITLE Case3",
            "#SKIP",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[2])).unwrap();
        assert_eq!(model.title, "Case2");
    }

    #[test]
    fn decode_switch_case_first_match() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 3",
            "#CASE 1",
            "#TITLE First",
            "#SKIP",
            "#CASE 2",
            "#TITLE Second",
            "#SKIP",
            "#CASE 3",
            "#TITLE Third",
            "#SKIP",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(model.title, "First");
    }

    #[test]
    fn decode_switch_case_last_match() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 3",
            "#CASE 1",
            "#TITLE First",
            "#SKIP",
            "#CASE 2",
            "#TITLE Second",
            "#SKIP",
            "#CASE 3",
            "#TITLE Third",
            "#SKIP",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[3])).unwrap();
        assert_eq!(model.title, "Third");
    }

    #[test]
    fn decode_switch_case_no_match_skips_all() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 5",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#CASE 2",
            "#TITLE Case2",
            "#SKIP",
            "#CASE 3",
            "#TITLE Case3",
            "#SKIP",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[4])).unwrap();
        assert_eq!(model.title, "");
    }

    #[test]
    fn decode_switch_case_fall_through_without_skip() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 3",
            "#CASE 1",
            "#TITLE FromCase1",
            "#CASE 2",
            "#TITLE FromCase2",
            "#CASE 3",
            "#TITLE FromCase3",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(model.title, "FromCase3");
    }

    #[test]
    fn decode_switch_case_partial_fall_through() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 3",
            "#CASE 1",
            "#TITLE FromCase1",
            "#CASE 2",
            "#TITLE FromCase2",
            "#SKIP",
            "#CASE 3",
            "#TITLE FromCase3",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(model.title, "FromCase2");
    }

    // --- #DEF (default case) tests ---

    #[test]
    fn decode_switch_def_when_no_case_matches() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 5",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#CASE 2",
            "#TITLE Case2",
            "#SKIP",
            "#DEF",
            "#TITLE DefaultCase",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[4])).unwrap();
        assert_eq!(model.title, "DefaultCase");
    }

    #[test]
    fn decode_switch_def_skipped_when_case_matches_with_skip() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 3",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#DEF",
            "#TITLE DefaultCase",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(model.title, "Case1");
    }

    #[test]
    fn decode_switch_def_fall_through_from_case() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 3",
            "#CASE 1",
            "#TITLE Case1",
            "#DEF",
            "#TITLE DefaultCase",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(model.title, "DefaultCase");
    }

    // --- #SETSWITCH tests ---

    #[test]
    fn decode_setswitch_deterministic() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SETSWITCH 2",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#CASE 2",
            "#TITLE Case2",
            "#SKIP",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, None).unwrap();
        assert_eq!(model.title, "Case2");
    }

    #[test]
    fn decode_setswitch_selected_random_overrides() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SETSWITCH 2",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#CASE 2",
            "#TITLE Case2",
            "#SKIP",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(model.title, "Case1");
    }

    // --- Error/warning tests for SWITCH directives ---

    #[test]
    fn decode_switch_invalid_value_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SWITCH 0"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#SWITCHの値は1以上"))
        );
    }

    #[test]
    fn decode_switch_non_numeric_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SWITCH abc"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#SWITCHに数字が定義されていません"))
        );
    }

    #[test]
    fn decode_case_without_switch_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#CASE 1"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(decoder.log.iter().any(|l| {
            l.message
                .contains("#CASEに対応する#SWITCHが定義されていません")
        }));
    }

    #[test]
    fn decode_skip_without_switch_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SKIP"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(decoder.log.iter().any(|l| {
            l.message
                .contains("#SKIPに対応する#SWITCHが定義されていません")
        }));
    }

    #[test]
    fn decode_def_without_switch_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#DEF"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(decoder.log.iter().any(|l| {
            l.message
                .contains("#DEFに対応する#SWITCHが定義されていません")
        }));
    }

    #[test]
    fn decode_endsw_without_switch_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#ENDSW"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("ENDSWに対応するSWITCHが存在しません"))
        );
    }

    #[test]
    fn decode_case_non_numeric_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SWITCH 3", "#CASE abc", "#ENDSW"]);
        let _ = decoder.decode_bytes(&data, false, Some(&[1]));
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#CASEに数字が定義されていません"))
        );
    }

    // --- Interaction: SWITCH does not interfere with DEFEXRANK ---

    #[test]
    fn decode_defexrank_not_confused_with_def() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#DEFEXRANK 100"]);
        let model = decoder.decode_bytes(&data, false, None).unwrap();
        assert_eq!(model.judgerank, 100);
        assert_eq!(model.judgerank_type, JudgeRankType::BmsDefexrank);
    }

    // --- Nested SWITCH inside RANDOM ---

    #[test]
    fn decode_switch_nested_in_random() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#RANDOM 2",
            "#IF 1",
            "#SWITCH 2",
            "#CASE 1",
            "#TITLE NestedCase1",
            "#SKIP",
            "#CASE 2",
            "#TITLE NestedCase2",
            "#SKIP",
            "#ENDSW",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1, 2])).unwrap();
        assert_eq!(model.title, "NestedCase2");
    }

    #[test]
    fn decode_switch_nested_in_random_skipped_branch() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#RANDOM 2",
            "#IF 2",
            "#SWITCH 2",
            "#CASE 1",
            "#TITLE ShouldNotAppear",
            "#SKIP",
            "#ENDSW",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(model.title, "");
    }

    // --- Content outside switch cases is skipped ---

    #[test]
    fn decode_switch_content_before_first_case_skipped() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 2",
            "#TITLE BeforeCase",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(model.title, "Case1");
    }

    // --- SETRANDOM + SWITCH interleaved with selected_randoms ---

    #[test]
    fn decode_setrandom_and_switch_share_random_counter() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SETRANDOM 1",
            "#IF 1",
            "#ARTIST SetRandomArtist",
            "#ENDIF",
            "#ENDRANDOM",
            "#SWITCH 3",
            "#CASE 2",
            "#TITLE SwitchCase2",
            "#SKIP",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1, 2])).unwrap();
        assert_eq!(model.artist, "SetRandomArtist");
        assert_eq!(model.title, "SwitchCase2");
    }

    // --- Case-insensitive directive parsing ---

    #[test]
    fn decode_switch_case_insensitive() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#switch 2",
            "#case 1",
            "#TITLE Case1",
            "#skip",
            "#case 2",
            "#TITLE Case2",
            "#skip",
            "#endsw",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[2])).unwrap();
        assert_eq!(model.title, "Case2");
    }

    #[test]
    fn decode_setrandom_case_insensitive() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#setrandom 1",
            "#IF 1",
            "#TITLE Match",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        let model = decoder.decode_bytes(&data, false, None).unwrap();
        assert_eq!(model.title, "Match");
    }

    // --- SETSWITCH error tests ---

    #[test]
    fn decode_setswitch_invalid_value_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SETSWITCH 0"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#SETSWITCHの値は1以上"))
        );
    }

    #[test]
    fn decode_setswitch_non_numeric_warns() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&["#BPM 120", "#SETSWITCH abc"]);
        let _ = decoder.decode_bytes(&data, false, None);
        assert!(
            decoder
                .log
                .iter()
                .any(|l| l.message.contains("#SETSWITCHに数字が定義されていません"))
        );
    }

    // --- RANDOM nested inside unentered SWITCH case ---

    #[test]
    fn decode_random_nested_in_switch_unentered_case() {
        // Outer SWITCH matches case 1 (crandom=1). After #SKIP, case 2 is unentered.
        // Inner RANDOM with #IF 1 is inside the unentered case 2,
        // so its body must be skipped and must NOT overwrite the title.
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 2",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#CASE 2",
            "#RANDOM 2",
            "#IF 1",
            "#TITLE ShouldNotAppear",
            "#ENDIF",
            "#ENDRANDOM",
            "#SKIP",
            "#ENDSW",
        ]);
        // selected_random[0]=1 → SWITCH picks 1 (case 1 matches),
        // selected_random[1]=1 → inner RANDOM picks 1 (IF 1 matches inside unentered case)
        let model = decoder.decode_bytes(&data, false, Some(&[1, 1])).unwrap();
        // Case 1 matched and set title. Case 2 is unentered so RANDOM inside it should be skipped.
        assert_eq!(model.title, "Case1");
    }

    #[test]
    fn decode_random_nested_in_switch_after_skip() {
        // SWITCH matches case 1, then #SKIP is hit.
        // Inner RANDOM with #IF 1 is after the #SKIP, so its body should be skipped.
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 2",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#RANDOM 2",
            "#IF 1",
            "#TITLE ShouldNotOverwrite",
            "#ENDIF",
            "#ENDRANDOM",
            "#ENDSW",
        ]);
        // selected_random[0]=1 → SWITCH picks 1, selected_random[1]=1 → RANDOM picks 1
        let model = decoder.decode_bytes(&data, false, Some(&[1, 1])).unwrap();
        // CASE 1 matched and set title to "Case1", then #SKIP was hit.
        // The RANDOM block after SKIP should not overwrite.
        assert_eq!(model.title, "Case1");
    }

    #[test]
    fn decode_switch_random_switch_nested() {
        // Outer SWITCH matches case 1, then #SKIP. Case 2 is unentered.
        // Inside unentered case 2: RANDOM → IF → innermost SWITCH → CASE with title.
        // All content inside the unentered case 2 must be skipped.
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 3",
            "#CASE 1",
            "#TITLE Case1",
            "#SKIP",
            "#CASE 2",
            "#RANDOM 2",
            "#IF 1",
            "#SWITCH 2",
            "#CASE 1",
            "#TITLE DeepNested",
            "#SKIP",
            "#ENDSW",
            "#ENDIF",
            "#ENDRANDOM",
            "#SKIP",
            "#ENDSW",
        ]);
        // selected_random[0]=1 → outer SWITCH picks 1 (case 1 matches)
        // selected_random[1]=1 → inner RANDOM picks 1
        // selected_random[2]=1 → innermost SWITCH picks 1
        let model = decoder
            .decode_bytes(&data, false, Some(&[1, 1, 1]))
            .unwrap();
        // Case 1 matched. Case 2 is unentered so nested content should be skipped.
        assert_eq!(model.title, "Case1");
    }

    // -- Finding 1: % and @ lines must be skipped inside conditional blocks --

    #[test]
    fn percent_value_skipped_in_unmatched_if_branch() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#RANDOM 2",
            "#IF 1",
            "%URL http://included.com",
            "#ENDIF",
            "#IF 2",
            "%URL http://skipped.com",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        // selected_random[0]=1 → IF 1 matches, IF 2 is skipped
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(
            model.values.get("URL"),
            Some(&"http://included.com".to_string())
        );
    }

    #[test]
    fn at_value_skipped_in_unmatched_if_branch() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#RANDOM 2",
            "#IF 1",
            "@KEY included_val",
            "#ENDIF",
            "#IF 2",
            "@KEY skipped_val",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        // selected_random[0]=1 → IF 1 matches, IF 2 is skipped
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(model.values.get("KEY"), Some(&"included_val".to_string()));
    }

    #[test]
    fn percent_value_skipped_in_unmatched_switch_case() {
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 2",
            "#CASE 1",
            "%URL http://case1.com",
            "#SKIP",
            "#CASE 2",
            "%URL http://case2.com",
            "#SKIP",
            "#ENDSW",
        ]);
        // selected_random[0]=1 → CASE 1 matches
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        assert_eq!(
            model.values.get("URL"),
            Some(&"http://case1.com".to_string())
        );
    }

    // -- Finding 2: ENDSW/ENDRANDOM must only close their own block type --

    #[test]
    fn endsw_does_not_pop_random_block() {
        // Malformed BMS: #ENDSW when the top block is RANDOM.
        // The ENDSW should be ignored (with warning), preserving the RANDOM state.
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#RANDOM 2",
            "#IF 1",
            "#TITLE CorrectTitle",
            "#ENDSW",
            "#ENDIF",
            "#ENDRANDOM",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        // The title should be set because ENDSW should NOT have popped the RANDOM state.
        assert_eq!(model.title, "CorrectTitle");
    }

    #[test]
    fn endrandom_does_not_pop_switch_block() {
        // Malformed BMS: #ENDRANDOM when the top block is SWITCH.
        // The ENDRANDOM should be ignored (with warning), preserving the SWITCH state.
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 2",
            "#CASE 1",
            "#TITLE CorrectTitle",
            "#ENDRANDOM",
            "#SKIP",
            "#ENDSW",
        ]);
        let model = decoder.decode_bytes(&data, false, Some(&[1])).unwrap();
        // The title should be set because ENDRANDOM should NOT have popped the SWITCH state.
        assert_eq!(model.title, "CorrectTitle");
    }

    #[test]
    fn endsw_inside_random_does_not_corrupt_outer_switch() {
        // Outer SWITCH with matched case wraps an inner RANDOM block.
        // A stray #ENDSW inside the RANDOM should not pop the outer SWITCH's
        // case_matched/case_skipped, which would corrupt skip state.
        let mut decoder = BMSDecoder::new();
        let data = make_bms_bytes(&[
            "#BPM 120",
            "#SWITCH 2",
            "#CASE 1",
            "#RANDOM 2",
            "#IF 1",
            "#TITLE InsideRandom",
            "#ENDSW",
            "#ENDIF",
            "#ENDRANDOM",
            "#TITLE AfterRandom",
            "#SKIP",
            "#ENDSW",
        ]);
        // selected_random[0]=1 → SWITCH picks 1 (CASE 1 matches)
        // selected_random[1]=1 → RANDOM picks 1 (IF 1 matches)
        let model = decoder.decode_bytes(&data, false, Some(&[1, 1])).unwrap();
        // Without the fix, the stray #ENDSW would pop the outer SWITCH's case state,
        // causing "AfterRandom" to be skipped. With the fix, it's correctly set.
        assert_eq!(model.title, "AfterRandom");
    }
}

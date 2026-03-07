use std::collections::HashMap;

use crate::chart_information::ChartInformation;
use crate::event_lane::EventLane;
use crate::judge_note::JudgeNote;
use crate::lane::Lane;
use crate::mode::Mode;
use crate::note::Note;
use crate::time_line::TimeLine;

/// Long note type for BMS charts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(i32)]
pub enum LnType {
    #[default]
    LongNote = 0,
    ChargeNote = 1,
    HellChargeNote = 2,
}

impl LnType {
    /// Convert from i32 (for deserialization and legacy compatibility).
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => LnType::LongNote,
            1 => LnType::ChargeNote,
            2 => LnType::HellChargeNote,
            _ => LnType::LongNote,
        }
    }

    /// Convert to i32 (for serialization and legacy compatibility).
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

// Backward-compatible constants for migration period
pub const LNTYPE_LONGNOTE: LnType = LnType::LongNote;
pub const LNTYPE_CHARGENOTE: LnType = LnType::ChargeNote;
pub const LNTYPE_HELLCHARGENOTE: LnType = LnType::HellChargeNote;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JudgeRankType {
    BmsRank,
    BmsDefexrank,
    BmsonJudgerank,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TotalType {
    Bms,
    Bmson,
}

#[derive(Clone)]
pub struct BMSModel {
    pub player: i32,
    mode: Option<Mode>,
    title: String,
    sub_title: String,
    genre: String,
    artist: String,
    subartist: String,
    banner: String,
    stagefile: String,
    backbmp: String,
    preview: String,
    pub bpm: f64,
    playlevel: String,
    pub difficulty: i32,
    pub judgerank: i32,
    pub judgerank_type: JudgeRankType,
    pub total: f64,
    pub total_type: TotalType,
    pub volwav: i32,
    md5: String,
    sha256: String,
    pub wavmap: Vec<String>,
    pub bgamap: Vec<String>,
    base: i32,
    pub lnmode: i32,
    pub lnobj: i32,
    pub from_osu: bool,
    pub timelines: Vec<TimeLine>,
    info: Option<ChartInformation>,
    values: HashMap<String, String>,
}

impl Default for BMSModel {
    fn default() -> Self {
        Self::new()
    }
}

impl BMSModel {
    pub fn new() -> Self {
        BMSModel {
            player: 0,
            mode: None,
            title: String::new(),
            sub_title: String::new(),
            genre: String::new(),
            artist: String::new(),
            subartist: String::new(),
            banner: String::new(),
            stagefile: String::new(),
            backbmp: String::new(),
            preview: String::new(),
            bpm: 0.0,
            playlevel: String::new(),
            difficulty: 0,
            judgerank: 2,
            judgerank_type: JudgeRankType::BmsRank,
            total: 100.0,
            total_type: TotalType::Bmson,
            volwav: 0,
            md5: String::new(),
            sha256: String::new(),
            wavmap: Vec::new(),
            bgamap: Vec::new(),
            base: 36,
            lnmode: crate::note::TYPE_UNDEFINED,
            lnobj: -1,
            from_osu: false,
            timelines: Vec::new(),
            info: None,
            values: HashMap::new(),
        }
    }

    pub fn player(&self) -> i32 {
        self.player
    }
    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        let t: String = title.into();
        self.title = t;
    }

    pub fn sub_title(&self) -> &str {
        &self.sub_title
    }

    pub fn set_sub_title(&mut self, sub_title: impl Into<String>) {
        let t: String = sub_title.into();
        self.sub_title = t;
    }

    pub fn genre(&self) -> &str {
        &self.genre
    }

    pub fn set_genre(&mut self, genre: impl Into<String>) {
        let t: String = genre.into();
        self.genre = t;
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn set_artist(&mut self, artist: impl Into<String>) {
        let t: String = artist.into();
        self.artist = t;
    }

    pub fn sub_artist(&self) -> &str {
        &self.subartist
    }

    pub fn set_sub_artist(&mut self, artist: impl Into<String>) {
        let t: String = artist.into();
        self.subartist = t;
    }

    pub fn set_banner(&mut self, banner: impl Into<String>) {
        let t: String = banner.into();
        self.banner = t;
    }

    pub fn banner(&self) -> &str {
        &self.banner
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }
    pub fn get_playlevel(&self) -> &str {
        &self.playlevel
    }

    pub fn set_playlevel(&mut self, playlevel: impl Into<String>) {
        self.playlevel = playlevel.into();
    }

    pub fn judgerank(&self) -> i32 {
        self.judgerank
    }
    pub fn get_total(&self) -> f64 {
        self.total
    }
    pub fn get_volwav(&self) -> i32 {
        self.volwav
    }
    pub fn get_min_bpm(&self) -> f64 {
        let mut bpm = self.bpm;
        for time in &self.timelines {
            let d = time.bpm;
            bpm = if bpm <= d { bpm } else { d };
        }
        bpm
    }

    pub fn max_bpm(&self) -> f64 {
        let mut bpm = self.bpm;
        for time in &self.timelines {
            let d = time.bpm;
            bpm = if bpm >= d { bpm } else { d };
        }
        bpm
    }
    pub fn get_all_time_lines(&self) -> &[TimeLine] {
        &self.timelines
    }

    pub fn all_time_lines_mut(&mut self) -> &mut [TimeLine] {
        &mut self.timelines
    }

    pub fn take_all_time_lines(&mut self) -> Vec<TimeLine> {
        std::mem::take(&mut self.timelines)
    }

    pub fn all_times(&self) -> Vec<i64> {
        let times = &self.timelines;
        let mut result = Vec::with_capacity(times.len());
        for tl in times {
            result.push(tl.time() as i64);
        }
        result
    }

    pub fn last_time(&self) -> i32 {
        self.last_milli_time() as i32
    }

    pub fn last_milli_time(&self) -> i64 {
        let keys = self.mode.as_ref().map(|m| m.key()).unwrap_or(0);
        for i in (0..self.timelines.len()).rev() {
            let tl = &self.timelines[i];
            for lane in 0..keys {
                if tl.exist_note_at(lane)
                    || tl.hidden_note(lane).is_some()
                    || !tl.back_ground_notes().is_empty()
                    || tl.bga() != -1
                    || tl.get_layer() != -1
                {
                    return tl.milli_time();
                }
            }
        }
        0
    }

    pub fn last_note_time(&self) -> i32 {
        self.last_note_milli_time() as i32
    }

    pub fn last_note_milli_time(&self) -> i64 {
        let keys = self.mode.as_ref().map(|m| m.key()).unwrap_or(0);
        for i in (0..self.timelines.len()).rev() {
            let tl = &self.timelines[i];
            for lane in 0..keys {
                if tl.exist_note_at(lane) {
                    return tl.milli_time();
                }
            }
        }
        0
    }

    pub fn difficulty(&self) -> i32 {
        self.difficulty
    }
    pub fn get_full_title(&self) -> String {
        let mut s = self.title.clone();
        if !self.sub_title.is_empty() {
            s.push(' ');
            s.push_str(&self.sub_title);
        }
        s
    }

    pub fn full_artist(&self) -> String {
        let mut s = self.artist.clone();
        if !self.subartist.is_empty() {
            s.push(' ');
            s.push_str(&self.subartist);
        }
        s
    }

    pub fn set_md5(&mut self, hash: impl Into<String>) {
        self.md5 = hash.into();
    }

    pub fn md5(&self) -> &str {
        &self.md5
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn set_sha256(&mut self, sha256: impl Into<String>) {
        self.sha256 = sha256.into();
    }

    pub fn set_mode(&mut self, mode: Mode) {
        let key = mode.key();
        self.mode = Some(mode);
        for tl in &mut self.timelines {
            tl.set_lane_count(key);
        }
    }

    pub fn mode(&self) -> Option<&Mode> {
        self.mode.as_ref()
    }

    pub fn wav_list(&self) -> &[String] {
        &self.wavmap
    }
    pub fn get_bga_list(&self) -> &[String] {
        &self.bgamap
    }
    pub fn get_chart_information(&self) -> Option<&ChartInformation> {
        self.info.as_ref()
    }

    pub fn set_chart_information(&mut self, info: ChartInformation) {
        self.info = Some(info);
    }

    pub fn random(&self) -> Option<&[i32]> {
        self.info
            .as_ref()
            .and_then(|i| i.selected_randoms.as_deref())
    }

    pub fn path(&self) -> Option<String> {
        self.info
            .as_ref()
            .and_then(|i| i.path.as_ref())
            .map(|p| p.to_string_lossy().to_string())
    }

    pub fn lntype(&self) -> LnType {
        self.info
            .as_ref()
            .map(|i| i.lntype)
            .unwrap_or(LnType::LongNote)
    }

    pub fn stagefile(&self) -> &str {
        &self.stagefile
    }

    pub fn set_stagefile(&mut self, stagefile: impl Into<String>) {
        let t: String = stagefile.into();
        self.stagefile = t;
    }

    pub fn backbmp(&self) -> &str {
        &self.backbmp
    }

    pub fn set_backbmp(&mut self, backbmp: impl Into<String>) {
        let t: String = backbmp.into();
        self.backbmp = t;
    }

    pub fn total_notes(&self) -> i32 {
        crate::bms_model_utils::total_notes(self)
    }

    pub fn build_judge_notes(&self) -> Vec<JudgeNote> {
        crate::judge_note::build_judge_notes(self)
    }

    pub fn is_from_osu(&self) -> bool {
        self.from_osu
    }
    pub fn contains_undefined_long_note(&self) -> bool {
        let keys = self.mode.as_ref().map(|m| m.key()).unwrap_or(0);
        for tl in &self.timelines {
            for i in 0..keys {
                if let Some(note) = tl.note(i)
                    && note.is_long()
                    && note.long_note_type() == crate::note::TYPE_UNDEFINED
                {
                    return true;
                }
            }
        }
        false
    }

    pub fn contains_long_note(&self) -> bool {
        let keys = self.mode.as_ref().map(|m| m.key()).unwrap_or(0);
        for tl in &self.timelines {
            for i in 0..keys {
                if let Some(note) = tl.note(i)
                    && note.is_long()
                {
                    return true;
                }
            }
        }
        false
    }

    pub fn contains_mine_note(&self) -> bool {
        let keys = self.mode.as_ref().map(|m| m.key()).unwrap_or(0);
        for tl in &self.timelines {
            for i in 0..keys {
                if let Some(note) = tl.note(i)
                    && note.is_mine()
                {
                    return true;
                }
            }
        }
        false
    }

    pub fn preview(&self) -> &str {
        &self.preview
    }

    pub fn set_preview(&mut self, preview: impl Into<String>) {
        self.preview = preview.into();
    }

    pub fn lnobj(&self) -> i32 {
        self.lnobj
    }
    pub fn get_lnmode(&self) -> i32 {
        self.lnmode
    }
    pub fn get_values(&self) -> &HashMap<String, String> {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.values
    }

    pub fn to_chart_string(&self) -> String {
        let mode = match &self.mode {
            Some(m) => m,
            None => return String::new(),
        };
        let key = mode.key();
        let mut sb = String::new();
        sb.push_str(&format!("JUDGERANK:{}\n", self.judgerank));
        sb.push_str(&format!("TOTAL:{}\n", self.total));
        if self.lnmode != 0 {
            sb.push_str(&format!("LNMODE:{}\n", self.lnmode));
        }
        let mut nowbpm = -f64::MIN_POSITIVE;
        for tl in &self.timelines {
            let mut tlsb = String::new();
            tlsb.push_str(&format!("{}:", tl.time()));
            let mut write = false;
            if nowbpm != tl.bpm {
                nowbpm = tl.bpm;
                tlsb.push_str(&format!("B({})", nowbpm));
                write = true;
            }
            if tl.stop() != 0 {
                tlsb.push_str(&format!("S({})", tl.stop()));
                write = true;
            }
            if tl.get_section_line() {
                tlsb.push('L');
                write = true;
            }

            tlsb.push('[');
            for lane in 0..key {
                if let Some(n) = tl.note(lane) {
                    match n {
                        Note::Normal(_) => {
                            tlsb.push('1');
                            write = true;
                        }
                        Note::Long { end, note_type, .. } => {
                            if !end {
                                let lnchars = ['l', 'L', 'C', 'H'];
                                tlsb.push(lnchars[*note_type as usize]);
                                tlsb.push_str(&format!("{}", n.milli_duration()));
                                write = true;
                            }
                        }
                        Note::Mine { damage, .. } => {
                            tlsb.push_str(&format!("m{}", damage));
                            write = true;
                        }
                    }
                } else {
                    tlsb.push('0');
                }
                if lane < key - 1 {
                    tlsb.push(',');
                }
            }
            tlsb.push_str("]\n");

            if write {
                sb.push_str(&tlsb);
            }
        }
        sb
    }

    pub fn judgerank_type(&self) -> &JudgeRankType {
        &self.judgerank_type
    }
    pub fn get_total_type(&self) -> &TotalType {
        &self.total_type
    }
    pub fn get_base(&self) -> i32 {
        self.base
    }

    pub fn set_base(&mut self, base: i32) {
        if base == 62 {
            self.base = base;
        } else {
            self.base = 36;
        }
    }

    pub fn event_lane(&self) -> EventLane {
        EventLane::new(self)
    }

    pub fn lanes(&self) -> Vec<Lane> {
        let key = self.mode().map(|m| m.key()).unwrap_or(0);
        (0..key).map(|i| Lane::new(self, i)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let model = BMSModel::new();
        assert_eq!(model.player(), 0);
        assert!(model.mode().is_none());
        assert_eq!(model.get_title(), "");
        assert_eq!(model.sub_title(), "");
        assert_eq!(model.genre(), "");
        assert_eq!(model.artist(), "");
        assert_eq!(model.sub_artist(), "");
        assert_eq!(model.banner(), "");
        assert_eq!(model.stagefile(), "");
        assert_eq!(model.backbmp(), "");
        assert_eq!(model.preview(), "");
        assert!((model.bpm).abs() < f64::EPSILON);
        assert_eq!(model.get_playlevel(), "");
        assert_eq!(model.difficulty(), 0);
        assert_eq!(model.judgerank(), 2);
        assert_eq!(model.judgerank_type(), &JudgeRankType::BmsRank);
        assert!((model.total - 100.0).abs() < f64::EPSILON);
        assert_eq!(model.total_type, TotalType::Bmson);
        assert_eq!(model.volwav, 0);
        assert_eq!(model.md5(), "");
        assert_eq!(model.sha256(), "");
        assert!(model.wav_list().is_empty());
        assert!(model.bgamap.is_empty());
        assert_eq!(model.get_base(), 36);
        assert_eq!(model.lnmode, crate::note::TYPE_UNDEFINED);
        assert_eq!(model.lnobj(), -1);
        assert!(!model.is_from_osu());
        assert!(model.timelines.is_empty());
    }

    #[test]
    fn default_matches_new() {
        let from_new = BMSModel::new();
        let from_default = BMSModel::default();
        assert_eq!(from_new.get_title(), from_default.get_title());
        assert_eq!(from_new.player(), from_default.player());
        assert_eq!(from_new.get_base(), from_default.get_base());
    }

    #[test]
    fn mode_set_and_get() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        assert_eq!(model.mode(), Some(&Mode::BEAT_7K));
    }

    #[test]
    fn set_mode_adjusts_timeline_lane_count() {
        let mut model = BMSModel::new();
        let tl = TimeLine::new(0.0, 0, 6);
        model.timelines = vec![tl];

        model.set_mode(Mode::BEAT_7K);
        // BEAT_7K has key=8, so timelines should be resized to 8 lanes
        assert_eq!(model.timelines[0].lane_count(), 8);
    }

    #[test]
    fn title_set_and_get() {
        let mut model = BMSModel::new();
        model.set_title("Test Song");
        assert_eq!(model.get_title(), "Test Song");
    }

    #[test]
    fn sub_title_set_and_get() {
        let mut model = BMSModel::new();
        model.set_sub_title("[SPA]");
        assert_eq!(model.sub_title(), "[SPA]");
    }

    #[test]
    fn full_title_without_subtitle() {
        let mut model = BMSModel::new();
        model.set_title("Main Title");
        assert_eq!(model.get_full_title(), "Main Title");
    }

    #[test]
    fn full_title_with_subtitle() {
        let mut model = BMSModel::new();
        model.set_title("Main Title");
        model.set_sub_title("[ANOTHER]");
        assert_eq!(model.get_full_title(), "Main Title [ANOTHER]");
    }

    #[test]
    fn artist_set_and_get() {
        let mut model = BMSModel::new();
        model.set_artist("Artist Name");
        assert_eq!(model.artist(), "Artist Name");
    }

    #[test]
    fn sub_artist_set_and_get() {
        let mut model = BMSModel::new();
        model.set_sub_artist("feat. Someone");
        assert_eq!(model.sub_artist(), "feat. Someone");
    }

    #[test]
    fn full_artist_without_subartist() {
        let mut model = BMSModel::new();
        model.set_artist("DJ Test");
        assert_eq!(model.full_artist(), "DJ Test");
    }

    #[test]
    fn full_artist_with_subartist() {
        let mut model = BMSModel::new();
        model.set_artist("DJ Test");
        model.set_sub_artist("feat. Vocal");
        assert_eq!(model.full_artist(), "DJ Test feat. Vocal");
    }

    #[test]
    fn genre_set_and_get() {
        let mut model = BMSModel::new();
        model.set_genre("Techno");
        assert_eq!(model.genre(), "Techno");
    }

    #[test]
    fn bpm_set_and_get() {
        let mut model = BMSModel::new();
        model.bpm = 150.0;
        assert!((model.bpm - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn playlevel_set_and_get() {
        let mut model = BMSModel::new();
        model.set_playlevel("12");
        assert_eq!(model.get_playlevel(), "12");
    }

    #[test]
    fn difficulty_set_and_get() {
        let mut model = BMSModel::new();
        model.difficulty = 5;
        assert_eq!(model.difficulty, 5);
    }

    #[test]
    fn judgerank_set_and_get() {
        let mut model = BMSModel::new();
        model.judgerank = 3;
        assert_eq!(model.judgerank, 3);
    }

    #[test]
    fn judgerank_type_set_and_get() {
        let mut model = BMSModel::new();
        model.judgerank_type = JudgeRankType::BmsDefexrank;
        assert_eq!(model.judgerank_type, JudgeRankType::BmsDefexrank);

        model.judgerank_type = JudgeRankType::BmsonJudgerank;
        assert_eq!(model.judgerank_type, JudgeRankType::BmsonJudgerank);
    }

    #[test]
    fn total_set_and_get() {
        let mut model = BMSModel::new();
        model.total = 300.0;
        assert!((model.get_total() - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn total_type_set_and_get() {
        let mut model = BMSModel::new();
        model.total_type = TotalType::Bms;
        assert_eq!(model.get_total_type(), &TotalType::Bms);
    }

    #[test]
    fn volwav_set_and_get() {
        let mut model = BMSModel::new();
        model.volwav = 100;
        assert_eq!(model.get_volwav(), 100);
    }

    #[test]
    fn md5_set_and_get() {
        let mut model = BMSModel::new();
        model.set_md5("abc123");
        assert_eq!(model.md5(), "abc123");
    }

    #[test]
    fn sha256_set_and_get() {
        let mut model = BMSModel::new();
        model.set_sha256("deadbeef");
        assert_eq!(model.sha256(), "deadbeef");
    }

    #[test]
    fn banner_set_and_get() {
        let mut model = BMSModel::new();
        model.set_banner("banner.png");
        assert_eq!(model.banner(), "banner.png");
    }

    #[test]
    fn stagefile_set_and_get() {
        let mut model = BMSModel::new();
        model.set_stagefile("stage.bmp");
        assert_eq!(model.stagefile(), "stage.bmp");
    }

    #[test]
    fn backbmp_set_and_get() {
        let mut model = BMSModel::new();
        model.set_backbmp("back.bmp");
        assert_eq!(model.backbmp(), "back.bmp");
    }

    #[test]
    fn preview_set_and_get() {
        let mut model = BMSModel::new();
        model.set_preview("preview.ogg");
        assert_eq!(model.preview(), "preview.ogg");
    }

    #[test]
    fn player_set_and_get() {
        let mut model = BMSModel::new();
        model.player = 2;
        assert_eq!(model.player, 2);
    }

    #[test]
    fn from_osu_set_and_get() {
        let mut model = BMSModel::new();
        model.from_osu = true;
        assert!(model.is_from_osu());
    }

    #[test]
    fn wav_list_set_and_get() {
        let mut model = BMSModel::new();
        let wavs = vec!["sound1.wav".to_string(), "sound2.ogg".to_string()];
        model.wavmap = wavs.clone();
        assert_eq!(model.wavmap, wavs);
    }

    #[test]
    fn bga_list_set_and_get() {
        let mut model = BMSModel::new();
        let bgas = vec!["video.mpg".to_string()];
        model.bgamap = bgas.clone();
        assert_eq!(model.get_bga_list(), &bgas);
    }

    #[test]
    fn base_set_62() {
        let mut model = BMSModel::new();
        model.set_base(62);
        assert_eq!(model.get_base(), 62);
    }

    #[test]
    fn base_set_non62_defaults_to_36() {
        let mut model = BMSModel::new();
        model.set_base(16);
        assert_eq!(model.get_base(), 36);

        model.set_base(100);
        assert_eq!(model.get_base(), 36);
    }

    #[test]
    fn lnobj_set_and_get() {
        let mut model = BMSModel::new();
        model.lnobj = 5;
        assert_eq!(model.lnobj, 5);
    }

    #[test]
    fn lnmode_set_and_get() {
        let mut model = BMSModel::new();
        model.lnmode = 2;
        assert_eq!(model.get_lnmode(), 2);
    }

    #[test]
    fn timeline_management() {
        let mut model = BMSModel::new();
        assert!(model.timelines.is_empty());

        let tl1 = TimeLine::new(0.0, 0, 8);
        let tl2 = TimeLine::new(1.0, 1000, 8);
        model.timelines = vec![tl1, tl2];

        assert_eq!(model.timelines.len(), 2);
        assert_eq!(model.timelines[0].micro_time(), 0);
        assert_eq!(model.timelines[1].micro_time(), 1000);
    }

    #[test]
    fn take_all_timelines_empties_model() {
        let mut model = BMSModel::new();
        model.timelines = vec![TimeLine::new(0.0, 0, 8)];
        assert_eq!(model.get_all_time_lines().len(), 1);

        let taken = model.take_all_time_lines();
        assert_eq!(taken.len(), 1);
        assert!(model.timelines.is_empty());
    }

    #[test]
    fn get_all_times() {
        let mut model = BMSModel::new();
        let mut tl1 = TimeLine::new(0.0, 0, 8);
        tl1.set_micro_time(0);
        let mut tl2 = TimeLine::new(1.0, 5000000, 8);
        tl2.set_micro_time(5000000); // 5000 ms = 5000 time
        model.timelines = vec![tl1, tl2];

        let times = model.all_times();
        assert_eq!(times.len(), 2);
        assert_eq!(times[0], 0);
        assert_eq!(times[1], 5000); // get_time() returns time/1000
    }

    #[test]
    fn min_and_max_bpm() {
        let mut model = BMSModel::new();
        model.bpm = 120.0;

        let mut tl1 = TimeLine::new(0.0, 0, 8);
        tl1.bpm = 100.0;
        let mut tl2 = TimeLine::new(1.0, 1000, 8);
        tl2.bpm = 200.0;
        let mut tl3 = TimeLine::new(2.0, 2000, 8);
        tl3.bpm = 150.0;
        model.timelines = vec![tl1, tl2, tl3];

        assert!((model.get_min_bpm() - 100.0).abs() < f64::EPSILON);
        assert!((model.max_bpm() - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn min_and_max_bpm_no_timelines() {
        let mut model = BMSModel::new();
        model.bpm = 130.0;

        assert!((model.get_min_bpm() - 130.0).abs() < f64::EPSILON);
        assert!((model.max_bpm() - 130.0).abs() < f64::EPSILON);
    }

    #[test]
    fn total_notes_with_notes() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_normal(2)));
        tl.set_note(2, Some(Note::new_mine(3, 0.5)));
        model.timelines = vec![tl];

        // Only normal notes are counted, not mines
        assert_eq!(model.total_notes(), 2);
    }

    #[test]
    fn total_notes_empty_model() {
        let model = BMSModel::new();
        assert_eq!(model.total_notes(), 0);
    }

    #[test]
    fn contains_long_note() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        model.timelines = vec![tl];
        assert!(!model.contains_long_note());

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_long(1)));
        model.timelines = vec![tl];
        assert!(model.contains_long_note());
    }

    #[test]
    fn contains_undefined_long_note() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut tl = TimeLine::new(0.0, 0, 8);
        let mut ln = Note::new_long(1);
        ln.set_long_note_type(crate::note::TYPE_CHARGENOTE);
        tl.set_note(0, Some(ln));
        model.timelines = vec![tl];
        assert!(!model.contains_undefined_long_note());

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_long(1))); // TYPE_UNDEFINED by default
        model.timelines = vec![tl];
        assert!(model.contains_undefined_long_note());
    }

    #[test]
    fn contains_mine_note() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        model.timelines = vec![tl];
        assert!(!model.contains_mine_note());

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_mine(1, 0.5)));
        model.timelines = vec![tl];
        assert!(model.contains_mine_note());
    }

    #[test]
    fn lntype_without_chart_information() {
        let model = BMSModel::new();
        assert_eq!(model.lntype(), LNTYPE_LONGNOTE);
    }

    #[test]
    fn chart_information_set_and_get() {
        let mut model = BMSModel::new();
        assert!(model.get_chart_information().is_none());

        let info = ChartInformation::new(None, LnType::ChargeNote, Some(vec![3, 5]));
        model.set_chart_information(info);
        assert!(model.get_chart_information().is_some());
        assert_eq!(model.lntype(), LnType::ChargeNote);
        assert_eq!(model.random(), Some(&[3, 5][..]));
    }

    #[test]
    fn get_path_without_chart_information() {
        let model = BMSModel::new();
        assert!(model.path().is_none());
    }

    #[test]
    fn values_map() {
        let mut model = BMSModel::new();
        assert!(model.get_values().is_empty());

        model
            .values_mut()
            .insert("key1".to_string(), "val1".to_string());
        assert_eq!(model.get_values().get("key1").unwrap(), "val1");
    }

    #[test]
    fn get_last_time_empty() {
        let model = BMSModel::new();
        assert_eq!(model.last_time(), 0);
        assert_eq!(model.last_milli_time(), 0);
    }

    #[test]
    fn get_last_note_time_empty() {
        let model = BMSModel::new();
        assert_eq!(model.last_note_time(), 0);
        assert_eq!(model.last_note_milli_time(), 0);
    }

    #[test]
    fn get_last_note_time_with_notes() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let tl1 = TimeLine::new(0.0, 0, 8);
        let mut tl2 = TimeLine::new(1.0, 5_000_000, 8);
        tl2.set_note(0, Some(Note::new_normal(1)));
        let tl3 = TimeLine::new(2.0, 10_000_000, 8);
        model.timelines = vec![tl1, tl2, tl3];

        // Last timeline with a note is tl2 at 5_000_000 microseconds = 5000 ms
        assert_eq!(model.last_note_milli_time(), 5000);
    }

    #[test]
    fn lntype_constants() {
        assert_eq!(LNTYPE_LONGNOTE.as_i32(), 0);
        assert_eq!(LNTYPE_CHARGENOTE.as_i32(), 1);
        assert_eq!(LNTYPE_HELLCHARGENOTE.as_i32(), 2);
    }

    #[test]
    fn get_event_lane_returns_event_lane() {
        let mut model = BMSModel::new();
        model.bpm = 120.0;
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.bpm = 150.0;
        model.timelines = vec![tl];

        let event_lane = model.event_lane();
        // The BPM changed from 120 to 150, so there should be 1 BPM change event
        assert_eq!(event_lane.bpm_changes().len(), 1);
    }

    #[test]
    fn get_lanes_returns_correct_count_for_beat_7k() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let lanes = model.lanes();
        // BEAT_7K has key() == 8
        assert_eq!(lanes.len(), 8);
    }

    #[test]
    fn get_lanes_returns_empty_when_no_mode() {
        let model = BMSModel::new();

        let lanes = model.lanes();
        assert!(lanes.is_empty());
    }
}

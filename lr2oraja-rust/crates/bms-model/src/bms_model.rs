use std::collections::HashMap;

use crate::chart_information::ChartInformation;
use crate::event_lane::EventLane;
use crate::judge_note::JudgeNote;
use crate::lane::Lane;
use crate::mode::Mode;
use crate::note::Note;
use crate::time_line::TimeLine;

pub const LNTYPE_LONGNOTE: i32 = 0;
pub const LNTYPE_CHARGENOTE: i32 = 1;
pub const LNTYPE_HELLCHARGENOTE: i32 = 2;

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
    player: i32,
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
    bpm: f64,
    playlevel: String,
    difficulty: i32,
    judgerank: i32,
    judgerank_type: JudgeRankType,
    total: f64,
    total_type: TotalType,
    volwav: i32,
    md5: String,
    sha256: String,
    wavmap: Vec<String>,
    bgamap: Vec<String>,
    base: i32,
    lnmode: i32,
    lnobj: i32,
    from_osu: bool,
    timelines: Vec<TimeLine>,
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

    pub fn get_player(&self) -> i32 {
        self.player
    }

    pub fn set_player(&mut self, player: i32) {
        self.player = player;
    }

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        let t: String = title.into();
        self.title = t;
    }

    pub fn get_sub_title(&self) -> &str {
        &self.sub_title
    }

    pub fn set_sub_title(&mut self, sub_title: impl Into<String>) {
        let t: String = sub_title.into();
        self.sub_title = t;
    }

    pub fn get_genre(&self) -> &str {
        &self.genre
    }

    pub fn set_genre(&mut self, genre: impl Into<String>) {
        let t: String = genre.into();
        self.genre = t;
    }

    pub fn get_artist(&self) -> &str {
        &self.artist
    }

    pub fn set_artist(&mut self, artist: impl Into<String>) {
        let t: String = artist.into();
        self.artist = t;
    }

    pub fn get_sub_artist(&self) -> &str {
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

    pub fn get_banner(&self) -> &str {
        &self.banner
    }

    pub fn get_bpm(&self) -> f64 {
        self.bpm
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm;
    }

    pub fn get_playlevel(&self) -> &str {
        &self.playlevel
    }

    pub fn set_playlevel(&mut self, playlevel: impl Into<String>) {
        self.playlevel = playlevel.into();
    }

    pub fn get_judgerank(&self) -> i32 {
        self.judgerank
    }

    pub fn set_judgerank(&mut self, judgerank: i32) {
        self.judgerank = judgerank;
    }

    pub fn get_total(&self) -> f64 {
        self.total
    }

    pub fn set_total(&mut self, total: f64) {
        self.total = total;
    }

    pub fn get_volwav(&self) -> i32 {
        self.volwav
    }

    pub fn set_volwav(&mut self, volwav: i32) {
        self.volwav = volwav;
    }

    pub fn get_min_bpm(&self) -> f64 {
        let mut bpm = self.get_bpm();
        for time in &self.timelines {
            let d = time.get_bpm();
            bpm = if bpm <= d { bpm } else { d };
        }
        bpm
    }

    pub fn get_max_bpm(&self) -> f64 {
        let mut bpm = self.get_bpm();
        for time in &self.timelines {
            let d = time.get_bpm();
            bpm = if bpm >= d { bpm } else { d };
        }
        bpm
    }

    pub fn set_all_time_line(&mut self, timelines: Vec<TimeLine>) {
        self.timelines = timelines;
    }

    pub fn get_all_time_lines(&self) -> &[TimeLine] {
        &self.timelines
    }

    pub fn get_all_time_lines_mut(&mut self) -> &mut [TimeLine] {
        &mut self.timelines
    }

    pub fn take_all_time_lines(&mut self) -> Vec<TimeLine> {
        std::mem::take(&mut self.timelines)
    }

    pub fn get_all_times(&self) -> Vec<i64> {
        let times = self.get_all_time_lines();
        let mut result = Vec::with_capacity(times.len());
        for tl in times {
            result.push(tl.get_time() as i64);
        }
        result
    }

    pub fn get_last_time(&self) -> i32 {
        self.get_last_milli_time() as i32
    }

    pub fn get_last_milli_time(&self) -> i64 {
        let keys = self.mode.as_ref().map(|m| m.key()).unwrap_or(0);
        for i in (0..self.timelines.len()).rev() {
            let tl = &self.timelines[i];
            for lane in 0..keys {
                if tl.exist_note_at(lane)
                    || tl.get_hidden_note(lane).is_some()
                    || !tl.get_back_ground_notes().is_empty()
                    || tl.get_bga() != -1
                    || tl.get_layer() != -1
                {
                    return tl.get_milli_time();
                }
            }
        }
        0
    }

    pub fn get_last_note_time(&self) -> i32 {
        self.get_last_note_milli_time() as i32
    }

    pub fn get_last_note_milli_time(&self) -> i64 {
        let keys = self.mode.as_ref().map(|m| m.key()).unwrap_or(0);
        for i in (0..self.timelines.len()).rev() {
            let tl = &self.timelines[i];
            for lane in 0..keys {
                if tl.exist_note_at(lane) {
                    return tl.get_milli_time();
                }
            }
        }
        0
    }

    pub fn get_difficulty(&self) -> i32 {
        self.difficulty
    }

    pub fn set_difficulty(&mut self, difficulty: i32) {
        self.difficulty = difficulty;
    }

    pub fn get_full_title(&self) -> String {
        let mut s = self.title.clone();
        if !self.sub_title.is_empty() {
            s.push(' ');
            s.push_str(&self.sub_title);
        }
        s
    }

    pub fn get_full_artist(&self) -> String {
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

    pub fn get_md5(&self) -> &str {
        &self.md5
    }

    pub fn get_sha256(&self) -> &str {
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

    pub fn get_mode(&self) -> Option<&Mode> {
        self.mode.as_ref()
    }

    pub fn get_wav_list(&self) -> &[String] {
        &self.wavmap
    }

    pub fn set_wav_list(&mut self, wavmap: Vec<String>) {
        self.wavmap = wavmap;
    }

    pub fn get_bga_list(&self) -> &[String] {
        &self.bgamap
    }

    pub fn set_bga_list(&mut self, bgamap: Vec<String>) {
        self.bgamap = bgamap;
    }

    pub fn get_chart_information(&self) -> Option<&ChartInformation> {
        self.info.as_ref()
    }

    pub fn set_chart_information(&mut self, info: ChartInformation) {
        self.info = Some(info);
    }

    pub fn get_random(&self) -> Option<&[i32]> {
        self.info
            .as_ref()
            .and_then(|i| i.selected_randoms.as_deref())
    }

    pub fn get_path(&self) -> Option<String> {
        self.info
            .as_ref()
            .and_then(|i| i.path.as_ref())
            .map(|p| p.to_string_lossy().to_string())
    }

    pub fn get_lntype(&self) -> i32 {
        self.info
            .as_ref()
            .map(|i| i.lntype)
            .unwrap_or(LNTYPE_LONGNOTE)
    }

    pub fn get_stagefile(&self) -> &str {
        &self.stagefile
    }

    pub fn set_stagefile(&mut self, stagefile: impl Into<String>) {
        let t: String = stagefile.into();
        self.stagefile = t;
    }

    pub fn get_backbmp(&self) -> &str {
        &self.backbmp
    }

    pub fn set_backbmp(&mut self, backbmp: impl Into<String>) {
        let t: String = backbmp.into();
        self.backbmp = t;
    }

    pub fn get_total_notes(&self) -> i32 {
        crate::bms_model_utils::get_total_notes(self)
    }

    pub fn build_judge_notes(&self) -> Vec<JudgeNote> {
        crate::judge_note::build_judge_notes(self)
    }

    pub fn is_from_osu(&self) -> bool {
        self.from_osu
    }

    pub fn set_from_osu(&mut self, from_osu: bool) {
        self.from_osu = from_osu;
    }

    pub fn contains_undefined_long_note(&self) -> bool {
        let keys = self.mode.as_ref().map(|m| m.key()).unwrap_or(0);
        for tl in &self.timelines {
            for i in 0..keys {
                if let Some(note) = tl.get_note(i)
                    && note.is_long()
                    && note.get_long_note_type() == crate::note::TYPE_UNDEFINED
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
                if let Some(note) = tl.get_note(i)
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
                if let Some(note) = tl.get_note(i)
                    && note.is_mine()
                {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_preview(&self) -> &str {
        &self.preview
    }

    pub fn set_preview(&mut self, preview: impl Into<String>) {
        self.preview = preview.into();
    }

    pub fn get_lnobj(&self) -> i32 {
        self.lnobj
    }

    pub fn set_lnobj(&mut self, lnobj: i32) {
        self.lnobj = lnobj;
    }

    pub fn get_lnmode(&self) -> i32 {
        self.lnmode
    }

    pub fn set_lnmode(&mut self, lnmode: i32) {
        self.lnmode = lnmode;
    }

    pub fn get_values(&self) -> &HashMap<String, String> {
        &self.values
    }

    pub fn get_values_mut(&mut self) -> &mut HashMap<String, String> {
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
            tlsb.push_str(&format!("{}:", tl.get_time()));
            let mut write = false;
            if nowbpm != tl.get_bpm() {
                nowbpm = tl.get_bpm();
                tlsb.push_str(&format!("B({})", nowbpm));
                write = true;
            }
            if tl.get_stop() != 0 {
                tlsb.push_str(&format!("S({})", tl.get_stop()));
                write = true;
            }
            if tl.get_section_line() {
                tlsb.push('L');
                write = true;
            }

            tlsb.push('[');
            for lane in 0..key {
                if let Some(n) = tl.get_note(lane) {
                    match n {
                        Note::Normal(_) => {
                            tlsb.push('1');
                            write = true;
                        }
                        Note::Long { end, note_type, .. } => {
                            if !end {
                                let lnchars = ['l', 'L', 'C', 'H'];
                                tlsb.push(lnchars[*note_type as usize]);
                                tlsb.push_str(&format!("{}", n.get_milli_duration()));
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

    pub fn get_judgerank_type(&self) -> &JudgeRankType {
        &self.judgerank_type
    }

    pub fn set_judgerank_type(&mut self, judgerank_type: JudgeRankType) {
        self.judgerank_type = judgerank_type;
    }

    pub fn get_total_type(&self) -> &TotalType {
        &self.total_type
    }

    pub fn set_total_type(&mut self, total_type: TotalType) {
        self.total_type = total_type;
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

    pub fn get_event_lane(&self) -> EventLane {
        EventLane::new(self)
    }

    pub fn get_lanes(&self) -> Vec<Lane> {
        let key = self.get_mode().map(|m| m.key()).unwrap_or(0);
        (0..key).map(|i| Lane::new(self, i)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let model = BMSModel::new();
        assert_eq!(model.get_player(), 0);
        assert!(model.get_mode().is_none());
        assert_eq!(model.get_title(), "");
        assert_eq!(model.get_sub_title(), "");
        assert_eq!(model.get_genre(), "");
        assert_eq!(model.get_artist(), "");
        assert_eq!(model.get_sub_artist(), "");
        assert_eq!(model.get_banner(), "");
        assert_eq!(model.get_stagefile(), "");
        assert_eq!(model.get_backbmp(), "");
        assert_eq!(model.get_preview(), "");
        assert!((model.get_bpm()).abs() < f64::EPSILON);
        assert_eq!(model.get_playlevel(), "");
        assert_eq!(model.get_difficulty(), 0);
        assert_eq!(model.get_judgerank(), 2);
        assert_eq!(model.get_judgerank_type(), &JudgeRankType::BmsRank);
        assert!((model.get_total() - 100.0).abs() < f64::EPSILON);
        assert_eq!(model.get_total_type(), &TotalType::Bmson);
        assert_eq!(model.get_volwav(), 0);
        assert_eq!(model.get_md5(), "");
        assert_eq!(model.get_sha256(), "");
        assert!(model.get_wav_list().is_empty());
        assert!(model.get_bga_list().is_empty());
        assert_eq!(model.get_base(), 36);
        assert_eq!(model.get_lnmode(), crate::note::TYPE_UNDEFINED);
        assert_eq!(model.get_lnobj(), -1);
        assert!(!model.is_from_osu());
        assert!(model.get_all_time_lines().is_empty());
    }

    #[test]
    fn default_matches_new() {
        let from_new = BMSModel::new();
        let from_default = BMSModel::default();
        assert_eq!(from_new.get_title(), from_default.get_title());
        assert_eq!(from_new.get_player(), from_default.get_player());
        assert_eq!(from_new.get_base(), from_default.get_base());
    }

    #[test]
    fn mode_set_and_get() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        assert_eq!(model.get_mode(), Some(&Mode::BEAT_7K));
    }

    #[test]
    fn set_mode_adjusts_timeline_lane_count() {
        let mut model = BMSModel::new();
        let tl = TimeLine::new(0.0, 0, 6);
        model.set_all_time_line(vec![tl]);

        model.set_mode(Mode::BEAT_7K);
        // BEAT_7K has key=8, so timelines should be resized to 8 lanes
        assert_eq!(model.get_all_time_lines()[0].get_lane_count(), 8);
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
        assert_eq!(model.get_sub_title(), "[SPA]");
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
        assert_eq!(model.get_artist(), "Artist Name");
    }

    #[test]
    fn sub_artist_set_and_get() {
        let mut model = BMSModel::new();
        model.set_sub_artist("feat. Someone");
        assert_eq!(model.get_sub_artist(), "feat. Someone");
    }

    #[test]
    fn full_artist_without_subartist() {
        let mut model = BMSModel::new();
        model.set_artist("DJ Test");
        assert_eq!(model.get_full_artist(), "DJ Test");
    }

    #[test]
    fn full_artist_with_subartist() {
        let mut model = BMSModel::new();
        model.set_artist("DJ Test");
        model.set_sub_artist("feat. Vocal");
        assert_eq!(model.get_full_artist(), "DJ Test feat. Vocal");
    }

    #[test]
    fn genre_set_and_get() {
        let mut model = BMSModel::new();
        model.set_genre("Techno");
        assert_eq!(model.get_genre(), "Techno");
    }

    #[test]
    fn bpm_set_and_get() {
        let mut model = BMSModel::new();
        model.set_bpm(150.0);
        assert!((model.get_bpm() - 150.0).abs() < f64::EPSILON);
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
        model.set_difficulty(5);
        assert_eq!(model.get_difficulty(), 5);
    }

    #[test]
    fn judgerank_set_and_get() {
        let mut model = BMSModel::new();
        model.set_judgerank(3);
        assert_eq!(model.get_judgerank(), 3);
    }

    #[test]
    fn judgerank_type_set_and_get() {
        let mut model = BMSModel::new();
        model.set_judgerank_type(JudgeRankType::BmsDefexrank);
        assert_eq!(model.get_judgerank_type(), &JudgeRankType::BmsDefexrank);

        model.set_judgerank_type(JudgeRankType::BmsonJudgerank);
        assert_eq!(model.get_judgerank_type(), &JudgeRankType::BmsonJudgerank);
    }

    #[test]
    fn total_set_and_get() {
        let mut model = BMSModel::new();
        model.set_total(300.0);
        assert!((model.get_total() - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn total_type_set_and_get() {
        let mut model = BMSModel::new();
        model.set_total_type(TotalType::Bms);
        assert_eq!(model.get_total_type(), &TotalType::Bms);
    }

    #[test]
    fn volwav_set_and_get() {
        let mut model = BMSModel::new();
        model.set_volwav(100);
        assert_eq!(model.get_volwav(), 100);
    }

    #[test]
    fn md5_set_and_get() {
        let mut model = BMSModel::new();
        model.set_md5("abc123");
        assert_eq!(model.get_md5(), "abc123");
    }

    #[test]
    fn sha256_set_and_get() {
        let mut model = BMSModel::new();
        model.set_sha256("deadbeef");
        assert_eq!(model.get_sha256(), "deadbeef");
    }

    #[test]
    fn banner_set_and_get() {
        let mut model = BMSModel::new();
        model.set_banner("banner.png");
        assert_eq!(model.get_banner(), "banner.png");
    }

    #[test]
    fn stagefile_set_and_get() {
        let mut model = BMSModel::new();
        model.set_stagefile("stage.bmp");
        assert_eq!(model.get_stagefile(), "stage.bmp");
    }

    #[test]
    fn backbmp_set_and_get() {
        let mut model = BMSModel::new();
        model.set_backbmp("back.bmp");
        assert_eq!(model.get_backbmp(), "back.bmp");
    }

    #[test]
    fn preview_set_and_get() {
        let mut model = BMSModel::new();
        model.set_preview("preview.ogg");
        assert_eq!(model.get_preview(), "preview.ogg");
    }

    #[test]
    fn player_set_and_get() {
        let mut model = BMSModel::new();
        model.set_player(2);
        assert_eq!(model.get_player(), 2);
    }

    #[test]
    fn from_osu_set_and_get() {
        let mut model = BMSModel::new();
        model.set_from_osu(true);
        assert!(model.is_from_osu());
    }

    #[test]
    fn wav_list_set_and_get() {
        let mut model = BMSModel::new();
        let wavs = vec!["sound1.wav".to_string(), "sound2.ogg".to_string()];
        model.set_wav_list(wavs.clone());
        assert_eq!(model.get_wav_list(), &wavs);
    }

    #[test]
    fn bga_list_set_and_get() {
        let mut model = BMSModel::new();
        let bgas = vec!["video.mpg".to_string()];
        model.set_bga_list(bgas.clone());
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
        model.set_lnobj(5);
        assert_eq!(model.get_lnobj(), 5);
    }

    #[test]
    fn lnmode_set_and_get() {
        let mut model = BMSModel::new();
        model.set_lnmode(2);
        assert_eq!(model.get_lnmode(), 2);
    }

    #[test]
    fn timeline_management() {
        let mut model = BMSModel::new();
        assert!(model.get_all_time_lines().is_empty());

        let tl1 = TimeLine::new(0.0, 0, 8);
        let tl2 = TimeLine::new(1.0, 1000, 8);
        model.set_all_time_line(vec![tl1, tl2]);

        assert_eq!(model.get_all_time_lines().len(), 2);
        assert_eq!(model.get_all_time_lines()[0].get_micro_time(), 0);
        assert_eq!(model.get_all_time_lines()[1].get_micro_time(), 1000);
    }

    #[test]
    fn take_all_timelines_empties_model() {
        let mut model = BMSModel::new();
        model.set_all_time_line(vec![TimeLine::new(0.0, 0, 8)]);
        assert_eq!(model.get_all_time_lines().len(), 1);

        let taken = model.take_all_time_lines();
        assert_eq!(taken.len(), 1);
        assert!(model.get_all_time_lines().is_empty());
    }

    #[test]
    fn get_all_times() {
        let mut model = BMSModel::new();
        let mut tl1 = TimeLine::new(0.0, 0, 8);
        tl1.set_micro_time(0);
        let mut tl2 = TimeLine::new(1.0, 5000000, 8);
        tl2.set_micro_time(5000000); // 5000 ms = 5000 time
        model.set_all_time_line(vec![tl1, tl2]);

        let times = model.get_all_times();
        assert_eq!(times.len(), 2);
        assert_eq!(times[0], 0);
        assert_eq!(times[1], 5000); // get_time() returns time/1000
    }

    #[test]
    fn min_and_max_bpm() {
        let mut model = BMSModel::new();
        model.set_bpm(120.0);

        let mut tl1 = TimeLine::new(0.0, 0, 8);
        tl1.set_bpm(100.0);
        let mut tl2 = TimeLine::new(1.0, 1000, 8);
        tl2.set_bpm(200.0);
        let mut tl3 = TimeLine::new(2.0, 2000, 8);
        tl3.set_bpm(150.0);
        model.set_all_time_line(vec![tl1, tl2, tl3]);

        assert!((model.get_min_bpm() - 100.0).abs() < f64::EPSILON);
        assert!((model.get_max_bpm() - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn min_and_max_bpm_no_timelines() {
        let mut model = BMSModel::new();
        model.set_bpm(130.0);

        assert!((model.get_min_bpm() - 130.0).abs() < f64::EPSILON);
        assert!((model.get_max_bpm() - 130.0).abs() < f64::EPSILON);
    }

    #[test]
    fn total_notes_with_notes() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_normal(2)));
        tl.set_note(2, Some(Note::new_mine(3, 0.5)));
        model.set_all_time_line(vec![tl]);

        // Only normal notes are counted, not mines
        assert_eq!(model.get_total_notes(), 2);
    }

    #[test]
    fn total_notes_empty_model() {
        let model = BMSModel::new();
        assert_eq!(model.get_total_notes(), 0);
    }

    #[test]
    fn contains_long_note() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        model.set_all_time_line(vec![tl]);
        assert!(!model.contains_long_note());

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_long(1)));
        model.set_all_time_line(vec![tl]);
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
        model.set_all_time_line(vec![tl]);
        assert!(!model.contains_undefined_long_note());

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_long(1))); // TYPE_UNDEFINED by default
        model.set_all_time_line(vec![tl]);
        assert!(model.contains_undefined_long_note());
    }

    #[test]
    fn contains_mine_note() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        model.set_all_time_line(vec![tl]);
        assert!(!model.contains_mine_note());

        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_mine(1, 0.5)));
        model.set_all_time_line(vec![tl]);
        assert!(model.contains_mine_note());
    }

    #[test]
    fn lntype_without_chart_information() {
        let model = BMSModel::new();
        assert_eq!(model.get_lntype(), LNTYPE_LONGNOTE);
    }

    #[test]
    fn chart_information_set_and_get() {
        let mut model = BMSModel::new();
        assert!(model.get_chart_information().is_none());

        let info = ChartInformation::new(None, 1, Some(vec![3, 5]));
        model.set_chart_information(info);
        assert!(model.get_chart_information().is_some());
        assert_eq!(model.get_lntype(), 1);
        assert_eq!(model.get_random(), Some(&[3, 5][..]));
    }

    #[test]
    fn get_path_without_chart_information() {
        let model = BMSModel::new();
        assert!(model.get_path().is_none());
    }

    #[test]
    fn values_map() {
        let mut model = BMSModel::new();
        assert!(model.get_values().is_empty());

        model
            .get_values_mut()
            .insert("key1".to_string(), "val1".to_string());
        assert_eq!(model.get_values().get("key1").unwrap(), "val1");
    }

    #[test]
    fn get_last_time_empty() {
        let model = BMSModel::new();
        assert_eq!(model.get_last_time(), 0);
        assert_eq!(model.get_last_milli_time(), 0);
    }

    #[test]
    fn get_last_note_time_empty() {
        let model = BMSModel::new();
        assert_eq!(model.get_last_note_time(), 0);
        assert_eq!(model.get_last_note_milli_time(), 0);
    }

    #[test]
    fn get_last_note_time_with_notes() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let tl1 = TimeLine::new(0.0, 0, 8);
        let mut tl2 = TimeLine::new(1.0, 5_000_000, 8);
        tl2.set_note(0, Some(Note::new_normal(1)));
        let tl3 = TimeLine::new(2.0, 10_000_000, 8);
        model.set_all_time_line(vec![tl1, tl2, tl3]);

        // Last timeline with a note is tl2 at 5_000_000 microseconds = 5000 ms
        assert_eq!(model.get_last_note_milli_time(), 5000);
    }

    #[test]
    fn lntype_constants() {
        assert_eq!(LNTYPE_LONGNOTE, 0);
        assert_eq!(LNTYPE_CHARGENOTE, 1);
        assert_eq!(LNTYPE_HELLCHARGENOTE, 2);
    }

    #[test]
    fn get_event_lane_returns_event_lane() {
        let mut model = BMSModel::new();
        model.set_bpm(120.0);
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_bpm(150.0);
        model.set_all_time_line(vec![tl]);

        let event_lane = model.get_event_lane();
        // The BPM changed from 120 to 150, so there should be 1 BPM change event
        assert_eq!(event_lane.get_bpm_changes().len(), 1);
    }

    #[test]
    fn get_lanes_returns_correct_count_for_beat_7k() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);

        let lanes = model.get_lanes();
        // BEAT_7K has key() == 8
        assert_eq!(lanes.len(), 8);
    }

    #[test]
    fn get_lanes_returns_empty_when_no_mode() {
        let model = BMSModel::new();

        let lanes = model.get_lanes();
        assert!(lanes.is_empty());
    }
}

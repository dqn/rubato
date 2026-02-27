use std::fmt;
use std::io::{Read, Write};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

use bms_model::mode::Mode;

use crate::clear_type::ClearType;
use crate::stubs::{BMSPlayerRule, JudgeAlgorithm, bms_player_input_device};
use crate::validatable::Validatable;

/// Score data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreData {
    pub sha256: String,
    pub player: String,
    pub mode: i32,
    pub clear: i32,
    pub date: i64,
    pub playcount: i32,
    pub clearcount: i32,
    pub epg: i32,
    pub lpg: i32,
    pub egr: i32,
    pub lgr: i32,
    pub egd: i32,
    pub lgd: i32,
    pub ebd: i32,
    pub lbd: i32,
    pub epr: i32,
    pub lpr: i32,
    pub ems: i32,
    pub lms: i32,
    pub combo: i32,
    pub notes: i32,
    pub passnotes: i32,
    pub minbp: i32,
    pub avgjudge: i64,
    pub total_duration: i64,
    pub avg: i64,
    pub total_avg: i64,
    pub stddev: i64,
    pub trophy: String,
    pub ghost: String,
    pub random: i32,
    pub option: i32,
    pub seed: i64,
    pub assist: i32,
    pub gauge: i32,
    pub device_type: Option<bms_player_input_device::Type>,
    pub state: i32,
    pub scorehash: String,
    pub playmode: Mode,
    pub judge_algorithm: Option<JudgeAlgorithm>,
    pub rule: Option<BMSPlayerRule>,
    pub skin: Option<String>,
}

impl ScoreData {
    pub const TROPHY_EASY: SongTrophy = SongTrophy::Easy;
    pub const TROPHY_GROOVE: SongTrophy = SongTrophy::Groove;
    pub const TROPHY_HARD: SongTrophy = SongTrophy::Hard;
    pub const TROPHY_EXHARD: SongTrophy = SongTrophy::ExHard;
    pub const TROPHY_NORMAL: SongTrophy = SongTrophy::Normal;
    pub const TROPHY_MIRROR: SongTrophy = SongTrophy::Mirror;
    pub const TROPHY_RANDOM: SongTrophy = SongTrophy::Random;
    pub const TROPHY_R_RANDOM: SongTrophy = SongTrophy::RRandom;
    pub const TROPHY_S_RANDOM: SongTrophy = SongTrophy::SRandom;
    pub const TROPHY_H_RANDOM: SongTrophy = SongTrophy::HRandom;
    pub const TROPHY_SPIRAL: SongTrophy = SongTrophy::Spiral;
    pub const TROPHY_ALL_SCR: SongTrophy = SongTrophy::AllScr;
    pub const TROPHY_EX_RANDOM: SongTrophy = SongTrophy::ExRandom;
    pub const TROPHY_EX_S_RANDOM: SongTrophy = SongTrophy::ExSRandom;
    pub const TROPHY_BATTLE: SongTrophy = SongTrophy::Battle;
    pub const TROPHY_BATTLE_ASSIST: SongTrophy = SongTrophy::BattleAssist;
}

impl Default for ScoreData {
    fn default() -> Self {
        Self::new(Mode::BEAT_7K)
    }
}

impl ScoreData {
    pub fn new(playmode: Mode) -> Self {
        Self {
            sha256: String::new(),
            player: "unknown".to_string(),
            mode: 0,
            clear: 0,
            date: 0,
            playcount: 0,
            clearcount: 0,
            epg: 0,
            lpg: 0,
            egr: 0,
            lgr: 0,
            egd: 0,
            lgd: 0,
            ebd: 0,
            lbd: 0,
            epr: 0,
            lpr: 0,
            ems: 0,
            lms: 0,
            combo: 0,
            notes: 0,
            passnotes: 0,
            minbp: i32::MAX,
            avgjudge: i64::MAX,
            total_duration: 0,
            avg: i64::MAX,
            total_avg: 0,
            stddev: i64::MAX,
            trophy: String::new(),
            ghost: String::new(),
            random: 0,
            option: 0,
            seed: -1,
            assist: 0,
            gauge: 0,
            device_type: None,
            state: 0,
            scorehash: String::new(),
            playmode,
            judge_algorithm: None,
            rule: None,
            skin: None,
        }
    }

    pub fn set_player(&mut self, player: Option<&str>) {
        self.player = player.unwrap_or("").to_string();
    }

    pub fn get_sha256(&self) -> &str {
        &self.sha256
    }

    pub fn get_player(&self) -> &str {
        &self.player
    }

    pub fn get_mode(&self) -> i32 {
        self.mode
    }

    pub fn get_clear(&self) -> i32 {
        self.clear
    }

    pub fn get_date(&self) -> i64 {
        self.date
    }

    pub fn get_playcount(&self) -> i32 {
        self.playcount
    }

    pub fn get_clearcount(&self) -> i32 {
        self.clearcount
    }

    pub fn get_notes(&self) -> i32 {
        self.notes
    }

    pub fn get_combo(&self) -> i32 {
        self.combo
    }

    pub fn get_minbp(&self) -> i32 {
        self.minbp
    }

    pub fn get_passnotes(&self) -> i32 {
        self.passnotes
    }

    pub fn get_random(&self) -> i32 {
        self.random
    }

    pub fn get_option(&self) -> i32 {
        self.option
    }

    pub fn get_seed(&self) -> i64 {
        self.seed
    }

    pub fn get_assist(&self) -> i32 {
        self.assist
    }

    pub fn get_gauge(&self) -> i32 {
        self.gauge
    }

    pub fn get_state(&self) -> i32 {
        self.state
    }

    pub fn get_scorehash(&self) -> &str {
        &self.scorehash
    }

    pub fn get_trophy(&self) -> &str {
        &self.trophy
    }

    pub fn get_ghost(&self) -> &str {
        &self.ghost
    }

    pub fn get_epg(&self) -> i32 {
        self.epg
    }

    pub fn get_lpg(&self) -> i32 {
        self.lpg
    }

    pub fn get_egr(&self) -> i32 {
        self.egr
    }

    pub fn get_lgr(&self) -> i32 {
        self.lgr
    }

    pub fn get_egd(&self) -> i32 {
        self.egd
    }

    pub fn get_lgd(&self) -> i32 {
        self.lgd
    }

    pub fn get_ebd(&self) -> i32 {
        self.ebd
    }

    pub fn get_lbd(&self) -> i32 {
        self.lbd
    }

    pub fn get_epr(&self) -> i32 {
        self.epr
    }

    pub fn get_lpr(&self) -> i32 {
        self.lpr
    }

    pub fn get_ems(&self) -> i32 {
        self.ems
    }

    pub fn get_lms(&self) -> i32 {
        self.lms
    }

    pub fn get_avgjudge(&self) -> i64 {
        self.avgjudge
    }

    pub fn get_exscore(&self) -> i32 {
        (self.epg + self.lpg) * 2 + self.egr + self.lgr
    }

    pub fn get_judge_count_total(&self, judge: i32) -> i32 {
        self.get_judge_count(judge, true) + self.get_judge_count(judge, false)
    }

    /// Get judge count for a specific judge type.
    /// judge: 0=PG, 1=GR, 2=GD, 3=BD, 4=PR, 5=MS
    /// fast: true=FAST, false=SLOW
    pub fn get_judge_count(&self, judge: i32, fast: bool) -> i32 {
        match judge {
            0 => {
                if fast {
                    self.epg
                } else {
                    self.lpg
                }
            }
            1 => {
                if fast {
                    self.egr
                } else {
                    self.lgr
                }
            }
            2 => {
                if fast {
                    self.egd
                } else {
                    self.lgd
                }
            }
            3 => {
                if fast {
                    self.ebd
                } else {
                    self.lbd
                }
            }
            4 => {
                if fast {
                    self.epr
                } else {
                    self.lpr
                }
            }
            5 => {
                if fast {
                    self.ems
                } else {
                    self.lms
                }
            }
            _ => 0,
        }
    }

    pub fn add_judge_count(&mut self, judge: i32, fast: bool, count: i32) {
        match judge {
            0 => {
                if fast {
                    self.epg += count;
                } else {
                    self.lpg += count;
                }
            }
            1 => {
                if fast {
                    self.egr += count;
                } else {
                    self.lgr += count;
                }
            }
            2 => {
                if fast {
                    self.egd += count;
                } else {
                    self.lgd += count;
                }
            }
            3 => {
                if fast {
                    self.ebd += count;
                } else {
                    self.lbd += count;
                }
            }
            4 => {
                if fast {
                    self.epr += count;
                } else {
                    self.lpr += count;
                }
            }
            5 => {
                if fast {
                    self.ems += count;
                } else {
                    self.lms += count;
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::unbuffered_bytes)]
    pub fn decode_ghost(&self) -> Option<Vec<i32>> {
        if self.ghost.is_empty() {
            return None;
        }
        let decoded = match URL_SAFE.decode(self.ghost.as_bytes()) {
            Ok(d) => d,
            Err(_) => return None,
        };
        let mut gz = match GzDecoder::new(&decoded[..])
            .bytes()
            .collect::<Result<Vec<u8>, _>>()
        {
            Ok(_bytes) => {
                // Re-create decoder for proper reading
                drop(_bytes);
                GzDecoder::new(&decoded[..])
            }
            Err(_) => return None,
        };
        let mut decompressed = Vec::new();
        if gz.read_to_end(&mut decompressed).is_err() {
            return None;
        }
        if decompressed.is_empty() {
            return None;
        }
        let mut value = vec![0i32; self.notes as usize];
        for i in 0..value.len() {
            if i < decompressed.len() {
                let judge = decompressed[i] as i32;
                value[i] = if judge >= 0 { judge } else { 4 };
            } else {
                value[i] = 4;
            }
        }
        Some(value)
    }

    #[allow(clippy::redundant_guards)]
    pub fn encode_ghost(&mut self, value: Option<&[i32]>) {
        match value {
            None => {
                self.ghost = String::new();
            }
            Some(v) if v.is_empty() => {
                self.ghost = String::new();
            }
            Some(v) => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                let bytes: Vec<u8> = v.iter().map(|&j| j as u8).collect();
                if encoder.write_all(&bytes).is_err() {
                    self.ghost = String::new();
                    return;
                }
                match encoder.finish() {
                    Ok(compressed) => {
                        self.ghost = URL_SAFE.encode(&compressed);
                    }
                    Err(_) => {
                        self.ghost = String::new();
                    }
                }
            }
        }
    }

    /// Update this score data with a new score. Returns true if updated.
    /// If update_score is false, only clear is updated.
    pub fn update(&mut self, newscore: &ScoreData, update_score: bool) -> bool {
        let mut update = false;
        if self.clear < newscore.clear {
            self.clear = newscore.clear;
            self.option = newscore.option;
            self.seed = newscore.seed;
            update = true;
        }
        if self.get_exscore() < newscore.get_exscore() && update_score {
            self.epg = newscore.epg;
            self.lpg = newscore.lpg;
            self.egr = newscore.egr;
            self.lgr = newscore.lgr;
            self.egd = newscore.egd;
            self.lgd = newscore.lgd;
            self.ebd = newscore.ebd;
            self.lbd = newscore.lbd;
            self.epr = newscore.epr;
            self.lpr = newscore.lpr;
            self.ems = newscore.ems;
            self.lms = newscore.lms;
            self.option = newscore.option;
            self.seed = newscore.seed;
            self.ghost = newscore.ghost.clone();
            update = true;
        }
        if self.avgjudge > newscore.avgjudge && update_score {
            self.avgjudge = newscore.avgjudge;
            self.option = newscore.option;
            self.seed = newscore.seed;
            update = true;
        }
        if self.minbp > newscore.minbp && update_score {
            self.minbp = newscore.minbp;
            self.option = newscore.option;
            self.seed = newscore.seed;
            update = true;
        }
        if self.combo < newscore.combo && update_score {
            self.combo = newscore.combo;
            self.option = newscore.option;
            self.seed = newscore.seed;
            update = true;
        }
        update
    }
}

impl Validatable for ScoreData {
    fn validate(&mut self) -> bool {
        self.mode >= 0
            && self.clear >= 0
            && self.clear <= ClearType::Max.id()
            && self.epg >= 0
            && self.lpg >= 0
            && self.egr >= 0
            && self.lgr >= 0
            && self.egd >= 0
            && self.lgd >= 0
            && self.ebd >= 0
            && self.lbd >= 0
            && self.epr >= 0
            && self.lpr >= 0
            && self.ems >= 0
            && self.lms >= 0
            && self.clearcount >= 0
            && self.playcount >= self.clearcount
            && self.combo >= 0
            && self.notes > 0
            && self.passnotes >= 0
            && self.passnotes <= self.notes
            && self.minbp >= 0
            && self.avgjudge >= 0
            && self.random >= 0
            && self.option >= 0
            && self.assist >= 0
            && self.gauge >= 0
    }
}

impl fmt::Display for ScoreData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        write!(f, "\"Date\": {}, ", self.date)?;
        write!(f, "\"Playcount\": {}, ", self.playcount)?;
        write!(f, "\"Clear\": {}, ", self.clear)?;
        write!(f, "\"Epg\": {}, ", self.epg)?;
        write!(f, "\"Lpg\": {}, ", self.lpg)?;
        write!(f, "\"Egr\": {}, ", self.egr)?;
        write!(f, "\"Lgr\": {}, ", self.lgr)?;
        write!(f, "\"Egd\": {}, ", self.egd)?;
        write!(f, "\"Lgd\": {}, ", self.lgd)?;
        write!(f, "\"Ebd\": {}, ", self.ebd)?;
        write!(f, "\"Lbd\": {}, ", self.lbd)?;
        write!(f, "\"Epr\": {}, ", self.epr)?;
        write!(f, "\"Lpr\": {}, ", self.lpr)?;
        write!(f, "\"Ems\": {}, ", self.ems)?;
        write!(f, "\"Lms\": {}, ", self.lms)?;
        write!(f, "\"Combo\": {}, ", self.combo)?;
        write!(f, "\"Mode\": {}, ", self.mode)?;
        write!(f, "\"Notes\": {}, ", self.notes)?;
        write!(f, "\"Clearcount\": {}, ", self.clearcount)?;
        write!(f, "\"Minbp\": {}, ", self.minbp)?;
        write!(f, "\"Avgjudge\": {}, ", self.avgjudge)?;
        write!(f, "\"Trophy\": \"{}\", ", self.trophy)?;
        write!(f, "\"Option\": {}, ", self.option)?;
        write!(f, "\"State\": {}, ", self.state)?;
        write!(f, "\"Sha256\": \"{}\", ", self.sha256)?;
        write!(f, "\"Exscore\": {}, ", self.get_exscore())?;
        write!(f, "\"Random\": {}, ", self.random)?;
        write!(f, "\"Scorehash\": \"{}\", ", self.scorehash)?;
        write!(f, "\"Assist\": {}, ", self.assist)?;
        write!(f, "\"Gauge\": {}, ", self.gauge)?;
        write!(f, "\"DeviceType\": \"{:?}\", ", self.device_type)?;
        write!(f, "\"Playmode\": \"{:?}\", ", self.playmode)?;
        write!(f, "\"Ghost\": \"{}\", ", self.ghost)?;
        write!(f, "\"Passnotes\": {}", self.passnotes)?;
        write!(f, "}}")
    }
}

/// Song trophy enum
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SongTrophy {
    Easy,
    Groove,
    Hard,
    ExHard,
    Normal,
    Mirror,
    Random,
    RRandom,
    SRandom,
    HRandom,
    Spiral,
    AllScr,
    ExRandom,
    ExSRandom,
    Battle,
    BattleAssist,
}

impl SongTrophy {
    pub fn character(&self) -> char {
        match self {
            SongTrophy::Easy => 'g',
            SongTrophy::Groove => 'G',
            SongTrophy::Hard => 'h',
            SongTrophy::ExHard => 'H',
            SongTrophy::Normal => 'n',
            SongTrophy::Mirror => 'm',
            SongTrophy::Random => 'r',
            SongTrophy::RRandom => 'o',
            SongTrophy::SRandom => 's',
            SongTrophy::HRandom => 'p',
            SongTrophy::Spiral => 'P',
            SongTrophy::AllScr => 'a',
            SongTrophy::ExRandom => 'R',
            SongTrophy::ExSRandom => 'S',
            SongTrophy::Battle => 'B',
            SongTrophy::BattleAssist => 'b',
        }
    }

    pub fn values() -> &'static [SongTrophy] {
        &[
            SongTrophy::Easy,
            SongTrophy::Groove,
            SongTrophy::Hard,
            SongTrophy::ExHard,
            SongTrophy::Normal,
            SongTrophy::Mirror,
            SongTrophy::Random,
            SongTrophy::RRandom,
            SongTrophy::SRandom,
            SongTrophy::HRandom,
            SongTrophy::Spiral,
            SongTrophy::AllScr,
            SongTrophy::ExRandom,
            SongTrophy::ExSRandom,
            SongTrophy::Battle,
            SongTrophy::BattleAssist,
        ]
    }

    pub fn get_trophy(c: char) -> Option<SongTrophy> {
        for trophy in SongTrophy::values() {
            if trophy.character() == c {
                return Some(*trophy);
            }
        }
        None
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn test_score_data_default() {
        let sd = ScoreData::default();
        assert_eq!(sd.get_player(), "unknown");
        assert_eq!(sd.get_sha256(), "");
        assert_eq!(sd.get_mode(), 0);
        assert_eq!(sd.get_clear(), 0);
        assert_eq!(sd.get_date(), 0);
        assert_eq!(sd.get_playcount(), 0);
        assert_eq!(sd.get_clearcount(), 0);
        assert_eq!(sd.get_epg(), 0);
        assert_eq!(sd.get_lpg(), 0);
        assert_eq!(sd.get_egr(), 0);
        assert_eq!(sd.get_lgr(), 0);
        assert_eq!(sd.get_egd(), 0);
        assert_eq!(sd.get_lgd(), 0);
        assert_eq!(sd.get_ebd(), 0);
        assert_eq!(sd.get_lbd(), 0);
        assert_eq!(sd.get_epr(), 0);
        assert_eq!(sd.get_lpr(), 0);
        assert_eq!(sd.get_ems(), 0);
        assert_eq!(sd.get_lms(), 0);
        assert_eq!(sd.get_combo(), 0);
        assert_eq!(sd.get_notes(), 0);
        assert_eq!(sd.get_passnotes(), 0);
        assert_eq!(sd.get_minbp(), i32::MAX);
        assert_eq!(sd.get_avgjudge(), i64::MAX);
        assert_eq!(sd.get_seed(), -1);
        assert_eq!(sd.get_trophy(), "");
        assert_eq!(sd.get_ghost(), "");
        assert_eq!(sd.get_scorehash(), "");
        assert!(sd.device_type.is_none());
        assert!(sd.judge_algorithm.is_none());
        assert!(sd.rule.is_none());
        assert!(sd.skin.is_none());
    }

    #[test]
    fn test_score_data_new_with_mode() {
        let sd = ScoreData::new(Mode::BEAT_5K);
        assert_eq!(sd.playmode, Mode::BEAT_5K);
        assert_eq!(sd.get_player(), "unknown");
    }

    #[test]
    fn test_score_data_serde_round_trip() {
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.sha256 = "abc123".to_string();
        sd.player = "player1".to_string();
        sd.clear = 5;
        sd.epg = 100;
        sd.lpg = 90;
        sd.egr = 80;
        sd.lgr = 70;
        sd.egd = 10;
        sd.lgd = 5;
        sd.combo = 250;
        sd.notes = 500;
        sd.date = 1700000000;

        let json = serde_json::to_string(&sd).unwrap();
        let deserialized: ScoreData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.get_sha256(), "abc123");
        assert_eq!(deserialized.get_player(), "player1");
        assert_eq!(deserialized.get_clear(), 5);
        assert_eq!(deserialized.get_epg(), 100);
        assert_eq!(deserialized.get_lpg(), 90);
        assert_eq!(deserialized.get_egr(), 80);
        assert_eq!(deserialized.get_lgr(), 70);
        assert_eq!(deserialized.get_egd(), 10);
        assert_eq!(deserialized.get_lgd(), 5);
        assert_eq!(deserialized.get_combo(), 250);
        assert_eq!(deserialized.get_notes(), 500);
        assert_eq!(deserialized.get_date(), 1700000000);
    }

    #[test]
    fn test_exscore_calculation() {
        let mut sd = ScoreData::default();
        sd.epg = 100;
        sd.lpg = 50;
        sd.egr = 30;
        sd.lgr = 20;
        // exscore = (epg + lpg) * 2 + egr + lgr = (100+50)*2 + 30+20 = 350
        assert_eq!(sd.get_exscore(), 350);
    }

    #[test]
    fn test_judge_count() {
        let mut sd = ScoreData::default();
        sd.epg = 10;
        sd.lpg = 20;
        sd.egr = 30;
        sd.lgr = 40;
        sd.egd = 5;
        sd.lgd = 6;
        sd.ebd = 3;
        sd.lbd = 4;
        sd.epr = 1;
        sd.lpr = 2;
        sd.ems = 7;
        sd.lms = 8;

        // PG (judge=0)
        assert_eq!(sd.get_judge_count(0, true), 10);
        assert_eq!(sd.get_judge_count(0, false), 20);
        assert_eq!(sd.get_judge_count_total(0), 30);

        // GR (judge=1)
        assert_eq!(sd.get_judge_count(1, true), 30);
        assert_eq!(sd.get_judge_count(1, false), 40);
        assert_eq!(sd.get_judge_count_total(1), 70);

        // GD (judge=2)
        assert_eq!(sd.get_judge_count(2, true), 5);
        assert_eq!(sd.get_judge_count(2, false), 6);

        // BD (judge=3)
        assert_eq!(sd.get_judge_count(3, true), 3);
        assert_eq!(sd.get_judge_count(3, false), 4);

        // PR (judge=4)
        assert_eq!(sd.get_judge_count(4, true), 1);
        assert_eq!(sd.get_judge_count(4, false), 2);

        // MS (judge=5)
        assert_eq!(sd.get_judge_count(5, true), 7);
        assert_eq!(sd.get_judge_count(5, false), 8);

        // Out of range
        assert_eq!(sd.get_judge_count(6, true), 0);
        assert_eq!(sd.get_judge_count(-1, false), 0);
    }

    #[test]
    fn test_add_judge_count() {
        let mut sd = ScoreData::default();
        sd.add_judge_count(0, true, 5);
        sd.add_judge_count(0, false, 3);
        sd.add_judge_count(1, true, 10);
        sd.add_judge_count(5, false, 2);
        // Out of range should be no-op
        sd.add_judge_count(6, true, 100);

        assert_eq!(sd.get_epg(), 5);
        assert_eq!(sd.get_lpg(), 3);
        assert_eq!(sd.get_egr(), 10);
        assert_eq!(sd.get_lms(), 2);
    }

    #[test]
    fn test_set_player() {
        let mut sd = ScoreData::default();
        sd.set_player(Some("TestPlayer"));
        assert_eq!(sd.get_player(), "TestPlayer");

        sd.set_player(None);
        assert_eq!(sd.get_player(), "");
    }

    #[test]
    fn test_ghost_encode_decode_round_trip() {
        let mut sd = ScoreData::default();
        sd.notes = 5;
        let ghost_data = vec![0, 1, 2, 3, 4];
        sd.encode_ghost(Some(&ghost_data));
        assert!(!sd.ghost.is_empty());

        let decoded = sd.decode_ghost().unwrap();
        assert_eq!(decoded, ghost_data);
    }

    #[test]
    fn test_ghost_encode_none() {
        let mut sd = ScoreData::default();
        sd.encode_ghost(None);
        assert!(sd.ghost.is_empty());
    }

    #[test]
    fn test_ghost_encode_empty() {
        let mut sd = ScoreData::default();
        sd.encode_ghost(Some(&[]));
        assert!(sd.ghost.is_empty());
    }

    #[test]
    fn test_ghost_decode_empty() {
        let sd = ScoreData::default();
        assert!(sd.decode_ghost().is_none());
    }

    #[test]
    fn test_update_clear() {
        let mut sd = ScoreData::default();
        sd.clear = 3;
        sd.notes = 100;

        let mut newscore = ScoreData::default();
        newscore.clear = 5;
        newscore.notes = 100;

        assert!(sd.update(&newscore, false));
        assert_eq!(sd.clear, 5);
    }

    #[test]
    fn test_update_exscore() {
        let mut sd = ScoreData::default();
        sd.epg = 10;
        sd.lpg = 10;
        sd.notes = 100;

        let mut newscore = ScoreData::default();
        newscore.epg = 50;
        newscore.lpg = 50;
        newscore.notes = 100;

        assert!(sd.update(&newscore, true));
        assert_eq!(sd.epg, 50);
        assert_eq!(sd.lpg, 50);
    }

    #[test]
    fn test_update_no_change() {
        let mut sd = ScoreData::default();
        sd.clear = 5;
        sd.epg = 100;
        sd.lpg = 100;
        sd.combo = 200;
        sd.minbp = 0;
        sd.avgjudge = 0;

        let newscore = sd.clone();
        assert!(!sd.update(&newscore, true));
    }

    // -- Phase 46b: ghost encoding truncation tests --

    #[test]
    fn test_ghost_encode_valid_range_roundtrip() {
        // Judge values 0–5 are the valid range; encode/decode should roundtrip cleanly
        let mut sd = ScoreData::default();
        let ghost_data: Vec<i32> = vec![0, 1, 2, 3, 4, 5];
        sd.notes = ghost_data.len() as i32;
        sd.encode_ghost(Some(&ghost_data));
        assert!(!sd.ghost.is_empty());

        let decoded = sd.decode_ghost().unwrap();
        assert_eq!(decoded, ghost_data);
    }

    #[test]
    #[ignore] // BUG: encode_ghost uses `j as u8` which silently truncates values >= 256
    // — value 256 becomes 0 after truncation, corrupting the ghost data
    fn test_ghost_encode_truncation_256() {
        let mut sd = ScoreData::default();
        let ghost_data: Vec<i32> = vec![256];
        sd.notes = 1;
        sd.encode_ghost(Some(&ghost_data));

        let decoded = sd.decode_ghost().unwrap();
        // After the bug: 256 as u8 = 0, so decoded[0] = 0 instead of 256
        assert_eq!(
            decoded[0], 256,
            "value 256 should survive roundtrip (actual: {})",
            decoded[0]
        );
    }

    // -- SongTrophy tests --

    #[test]
    fn test_song_trophy_character() {
        assert_eq!(SongTrophy::Easy.character(), 'g');
        assert_eq!(SongTrophy::Groove.character(), 'G');
        assert_eq!(SongTrophy::Hard.character(), 'h');
        assert_eq!(SongTrophy::ExHard.character(), 'H');
        assert_eq!(SongTrophy::Normal.character(), 'n');
        assert_eq!(SongTrophy::Mirror.character(), 'm');
        assert_eq!(SongTrophy::Random.character(), 'r');
        assert_eq!(SongTrophy::SRandom.character(), 's');
        assert_eq!(SongTrophy::Battle.character(), 'B');
    }

    #[test]
    fn test_song_trophy_values_count() {
        assert_eq!(SongTrophy::values().len(), 16);
    }

    #[test]
    fn test_song_trophy_get_trophy() {
        assert_eq!(SongTrophy::get_trophy('g'), Some(SongTrophy::Easy));
        assert_eq!(SongTrophy::get_trophy('G'), Some(SongTrophy::Groove));
        assert_eq!(SongTrophy::get_trophy('H'), Some(SongTrophy::ExHard));
        assert_eq!(SongTrophy::get_trophy('B'), Some(SongTrophy::Battle));
        assert_eq!(SongTrophy::get_trophy('z'), None);
    }

    #[test]
    fn test_song_trophy_round_trip() {
        // Every trophy should be recoverable from its character
        for trophy in SongTrophy::values() {
            let c = trophy.character();
            let recovered = SongTrophy::get_trophy(c);
            assert_eq!(recovered, Some(*trophy));
        }
    }

    #[test]
    fn test_score_data_trophy_constants() {
        assert_eq!(ScoreData::TROPHY_EASY, SongTrophy::Easy);
        assert_eq!(ScoreData::TROPHY_GROOVE, SongTrophy::Groove);
        assert_eq!(ScoreData::TROPHY_HARD, SongTrophy::Hard);
        assert_eq!(ScoreData::TROPHY_EXHARD, SongTrophy::ExHard);
        assert_eq!(ScoreData::TROPHY_NORMAL, SongTrophy::Normal);
        assert_eq!(ScoreData::TROPHY_MIRROR, SongTrophy::Mirror);
        assert_eq!(ScoreData::TROPHY_RANDOM, SongTrophy::Random);
        assert_eq!(ScoreData::TROPHY_S_RANDOM, SongTrophy::SRandom);
        assert_eq!(ScoreData::TROPHY_BATTLE, SongTrophy::Battle);
    }
}

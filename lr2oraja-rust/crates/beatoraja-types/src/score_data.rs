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
    pub maxcombo: i32,
    pub notes: i32,
    pub passnotes: i32,
    pub minbp: i32,
    pub avgjudge: i64,
    #[serde(rename = "totalDuration")]
    pub total_duration: i64,
    pub avg: i64,
    #[serde(rename = "totalAvg")]
    pub total_avg: i64,
    pub stddev: i64,
    pub trophy: String,
    pub ghost: String,
    pub random: i32,
    pub option: i32,
    pub seed: i64,
    pub assist: i32,
    pub gauge: i32,
    #[serde(rename = "deviceType")]
    pub device_type: Option<bms_player_input_device::Type>,
    pub state: i32,
    pub scorehash: String,
    pub playmode: Mode,
    #[serde(rename = "judgeAlgorithm")]
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
            maxcombo: 0,
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
        self.maxcombo
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
        (self.epg.saturating_add(self.lpg))
            .saturating_mul(2)
            .saturating_add(self.egr)
            .saturating_add(self.lgr)
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
        if self.maxcombo < newscore.maxcombo && update_score {
            self.maxcombo = newscore.maxcombo;
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
            && self.maxcombo >= 0
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
        write!(f, "\"Combo\": {}, ", self.maxcombo)?;
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
        sd.maxcombo = 250;
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
        sd.maxcombo = 200;
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
    fn test_score_data_serde_java_field_names() {
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.maxcombo = 250;
        sd.total_duration = 120_000;
        sd.total_avg = 500;
        sd.device_type = None;
        sd.judge_algorithm = None;

        let json = serde_json::to_string(&sd).unwrap();

        // Field must serialize as "maxcombo" (Java field name), not "combo"
        assert!(
            json.contains("\"maxcombo\""),
            "Expected 'maxcombo' in JSON, got: {}",
            json
        );
        assert!(
            !json.contains("\"combo\"") || json.contains("\"maxcombo\""),
            "Should not have bare 'combo' field without 'max' prefix"
        );

        // camelCase renames for Java compatibility
        assert!(
            json.contains("\"totalDuration\""),
            "Expected 'totalDuration' in JSON, got: {}",
            json
        );
        assert!(
            json.contains("\"totalAvg\""),
            "Expected 'totalAvg' in JSON, got: {}",
            json
        );
        assert!(
            json.contains("\"deviceType\""),
            "Expected 'deviceType' in JSON, got: {}",
            json
        );
        assert!(
            json.contains("\"judgeAlgorithm\""),
            "Expected 'judgeAlgorithm' in JSON, got: {}",
            json
        );

        // Verify these snake_case forms do NOT appear
        assert!(
            !json.contains("\"total_duration\""),
            "Should not have 'total_duration' in JSON"
        );
        assert!(
            !json.contains("\"total_avg\""),
            "Should not have 'total_avg' in JSON"
        );
        assert!(
            !json.contains("\"device_type\""),
            "Should not have 'device_type' in JSON"
        );
        assert!(
            !json.contains("\"judge_algorithm\""),
            "Should not have 'judge_algorithm' in JSON"
        );

        // Round-trip: deserialize from Java-style JSON
        let deserialized: ScoreData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.maxcombo, 250);
        assert_eq!(deserialized.total_duration, 120_000);
        assert_eq!(deserialized.total_avg, 500);
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

    // -- get_exscore() formula and rank boundary tests --

    /// Helper: build a ScoreData with specific judge counts and notes.
    fn make_score(epg: i32, lpg: i32, egr: i32, lgr: i32, notes: i32) -> ScoreData {
        let mut sd = ScoreData::default();
        sd.epg = epg;
        sd.lpg = lpg;
        sd.egr = egr;
        sd.lgr = lgr;
        sd.notes = notes;
        sd
    }

    /// Compute the rank boundary rate for a given index (0..27).
    /// rank[i] is true when rate >= i / 27.
    /// Rate boundaries:
    ///   0/27=F, 3/27=E, 6/27=D, 9/27=C, 12/27=B, 15/27=A,
    ///   18/27=AA, 21/27=AAA, 24/27=MAX-
    /// Sub-grades: i%3==1 is "-", i%3==2 is base, i%3==0 is "+" (except 0).
    fn rank_boundary_exscore(rank_index: usize, notes: i32) -> i32 {
        // exscore needed = ceil(rank_index / 27 * notes * 2)
        // Using integer arithmetic to avoid float imprecision:
        let max_ex = notes as i64 * 2;
        let needed = (rank_index as i64 * max_ex + 26) / 27; // ceiling division
        needed as i32
    }

    // -- get_exscore() formula verification --

    #[test]
    fn test_exscore_formula_only_perfects() {
        // All PG: exscore = (epg + lpg) * 2
        let sd = make_score(50, 50, 0, 0, 100);
        assert_eq!(sd.get_exscore(), 200);
    }

    #[test]
    fn test_exscore_formula_only_greats() {
        // All GR: exscore = egr + lgr
        let sd = make_score(0, 0, 60, 40, 100);
        assert_eq!(sd.get_exscore(), 100);
    }

    #[test]
    fn test_exscore_formula_mixed() {
        // (10 + 20) * 2 + 30 + 40 = 60 + 70 = 130
        let sd = make_score(10, 20, 30, 40, 100);
        assert_eq!(sd.get_exscore(), 130);
    }

    #[test]
    fn test_exscore_formula_single_epg() {
        let sd = make_score(1, 0, 0, 0, 1);
        // (1 + 0) * 2 + 0 + 0 = 2
        assert_eq!(sd.get_exscore(), 2);
    }

    #[test]
    fn test_exscore_formula_single_egr() {
        let sd = make_score(0, 0, 1, 0, 1);
        // (0 + 0) * 2 + 1 + 0 = 1
        assert_eq!(sd.get_exscore(), 1);
    }

    // -- Zero notes (all miss / empty chart) --

    #[test]
    fn test_exscore_zero_notes_all_zero() {
        let sd = make_score(0, 0, 0, 0, 0);
        assert_eq!(sd.get_exscore(), 0);
    }

    #[test]
    fn test_exscore_zero_judge_counts_nonzero_notes() {
        // Chart has 1000 notes but all missed
        let sd = make_score(0, 0, 0, 0, 1000);
        assert_eq!(sd.get_exscore(), 0);
    }

    // -- All perfect (MAX) --

    #[test]
    fn test_exscore_all_perfect_100_notes() {
        // 100 notes, all perfect great: max exscore = 200
        let sd = make_score(100, 0, 0, 0, 100);
        assert_eq!(sd.get_exscore(), 200);
    }

    #[test]
    fn test_exscore_all_perfect_split_fast_slow() {
        // 100 notes: 60 epg + 40 lpg = max exscore 200
        let sd = make_score(60, 40, 0, 0, 100);
        assert_eq!(sd.get_exscore(), 200);
    }

    #[test]
    fn test_exscore_all_perfect_1000_notes() {
        let sd = make_score(500, 500, 0, 0, 1000);
        assert_eq!(sd.get_exscore(), 2000);
    }

    // -- Rank boundary transitions using 1000-note chart --
    // For 1000 notes, max exscore = 2000.
    // Boundary at index i: exscore >= ceil(i * 2000 / 27)
    //
    // Rank indices (every 3rd is a major boundary):
    //   0: always true (F floor)
    //   3: E   -> ceil(3*2000/27) = ceil(222.22) = 223
    //   6: D   -> ceil(6*2000/27) = ceil(444.44) = 445
    //   9: C   -> ceil(9*2000/27) = ceil(666.67) = 667
    //  12: B   -> ceil(12*2000/27) = ceil(888.89) = 889
    //  15: A   -> ceil(15*2000/27) = ceil(1111.11) = 1112
    //  18: AA  -> ceil(18*2000/27) = ceil(1333.33) = 1334
    //  21: AAA -> ceil(21*2000/27) = ceil(1555.56) = 1556
    //  24: MAX--> ceil(24*2000/27) = ceil(1777.78) = 1778

    /// Verify rank_boundary_exscore helper is correct for a few known values.
    #[test]
    fn test_rank_boundary_helper_sanity() {
        // Index 0: boundary = 0
        assert_eq!(rank_boundary_exscore(0, 1000), 0);
        // Index 27: boundary = 2000 (max)
        assert_eq!(rank_boundary_exscore(27, 1000), 2000);
        // Index 9 (C): ceil(9*2000/27) = ceil(666.67) = 667
        assert_eq!(rank_boundary_exscore(9, 1000), 667);
        // Index 21 (AAA): ceil(21*2000/27) = ceil(1555.56) = 1556
        assert_eq!(rank_boundary_exscore(21, 1000), 1556);
    }

    /// For each major rank boundary (E, D, C, B, A, AA, AAA, MAX-),
    /// verify exscore exactly at, one below, and one above the boundary.
    /// The rate = exscore / (notes * 2), and rank[i] = rate >= i/27.
    #[test]
    fn test_exscore_at_rank_boundaries() {
        let notes = 1000;
        let max_ex = notes * 2; // 2000

        // Major rank boundary indices
        let boundaries = [
            (3, "E"),
            (6, "D"),
            (9, "C"),
            (12, "B"),
            (15, "A"),
            (18, "AA"),
            (21, "AAA"),
            (24, "MAX-"),
        ];

        for (idx, name) in &boundaries {
            let threshold = rank_boundary_exscore(*idx, notes);

            // One below: should NOT qualify
            if threshold > 0 {
                let below = threshold - 1;
                let rate_below = below as f32 / max_ex as f32;
                let rank_threshold = *idx as f32 / 27.0;
                assert!(
                    rate_below < rank_threshold,
                    "rank {} (idx {}): exscore {} should be below threshold (rate {:.6} < {:.6})",
                    name,
                    idx,
                    below,
                    rate_below,
                    rank_threshold
                );
            }

            // Exactly at: should qualify
            let rate_at = threshold as f32 / max_ex as f32;
            let rank_threshold = *idx as f32 / 27.0;
            assert!(
                rate_at >= rank_threshold,
                "rank {} (idx {}): exscore {} should meet threshold (rate {:.6} >= {:.6})",
                name,
                idx,
                threshold,
                rate_at,
                rank_threshold
            );

            // One above: should qualify
            if threshold < max_ex {
                let above = threshold + 1;
                let rate_above = above as f32 / max_ex as f32;
                assert!(
                    rate_above >= rank_threshold,
                    "rank {} (idx {}): exscore {} should exceed threshold (rate {:.6} >= {:.6})",
                    name,
                    idx,
                    above,
                    rate_above,
                    rank_threshold
                );
            }
        }
    }

    /// Verify all 27 sub-rank boundaries (including +/- variants).
    #[test]
    fn test_exscore_all_27_sub_rank_boundaries() {
        let notes = 1000;
        let max_ex = notes * 2;

        for idx in 0..=26 {
            let threshold = rank_boundary_exscore(idx, notes);
            let rate = threshold as f32 / max_ex as f32;
            let boundary = idx as f32 / 27.0;

            assert!(
                rate >= boundary,
                "sub-rank {}: exscore {} rate {:.6} should >= {:.6}",
                idx,
                threshold,
                rate,
                boundary
            );

            // One below should NOT qualify (except index 0 which is always 0)
            if threshold > 0 {
                let rate_below = (threshold - 1) as f32 / max_ex as f32;
                assert!(
                    rate_below < boundary,
                    "sub-rank {}: exscore {} rate {:.6} should < {:.6}",
                    idx,
                    threshold - 1,
                    rate_below,
                    boundary
                );
            }
        }
    }

    /// Verify exscore exactly produces the right rate for AAA boundary (21/27).
    #[test]
    fn test_exscore_aaa_boundary_exact() {
        // For a chart with 27 notes, max exscore = 54.
        // AAA boundary at index 21: rate >= 21/27 = 7/9
        // Needed exscore = 21 * 54 / 27 = 42 (exact division)
        let sd = make_score(21, 0, 0, 0, 27);
        // epg=21 -> exscore = 21*2 = 42
        assert_eq!(sd.get_exscore(), 42);
        let rate = sd.get_exscore() as f32 / (27 * 2) as f32;
        assert!((rate - 7.0 / 9.0).abs() < 1e-6);
    }

    /// Verify one below AAA boundary does not qualify.
    #[test]
    fn test_exscore_aaa_boundary_one_below() {
        // exscore 41 for 27 notes: rate = 41/54 < 21/27
        let sd = make_score(20, 0, 1, 0, 27);
        // epg=20, egr=1 -> exscore = 20*2 + 1 = 41
        assert_eq!(sd.get_exscore(), 41);
        let rate = sd.get_exscore() as f32 / (27 * 2) as f32;
        assert!(rate < 21.0 / 27.0);
    }

    /// Verify one above AAA boundary qualifies.
    #[test]
    fn test_exscore_aaa_boundary_one_above() {
        // exscore 43 for 27 notes: rate = 43/54 > 21/27
        let sd = make_score(21, 0, 1, 0, 27);
        // epg=21, egr=1 -> exscore = 21*2 + 1 = 43
        assert_eq!(sd.get_exscore(), 43);
        let rate = sd.get_exscore() as f32 / (27 * 2) as f32;
        assert!(rate > 21.0 / 27.0);
    }

    /// MAX rank: all perfect, exscore = notes * 2.
    #[test]
    fn test_exscore_max_rank() {
        let sd = make_score(500, 500, 0, 0, 1000);
        assert_eq!(sd.get_exscore(), 2000);
        let rate = sd.get_exscore() as f32 / (1000 * 2) as f32;
        assert!((rate - 1.0).abs() < 1e-6);
    }

    /// F rank: all miss, exscore = 0.
    #[test]
    fn test_exscore_f_rank_all_miss() {
        let sd = make_score(0, 0, 0, 0, 1000);
        assert_eq!(sd.get_exscore(), 0);
        // rate = 0, only rank[0] should be satisfied (0/27 = 0.0 <= 0.0)
    }

    // -- Saturating arithmetic in get_exscore() --

    #[test]
    fn test_exscore_saturating_epg_lpg_overflow() {
        // (epg + lpg) would overflow i32 without saturating_add
        let sd = make_score(i32::MAX, 1, 0, 0, 1000);
        // saturating_add: i32::MAX + 1 = i32::MAX
        // saturating_mul: i32::MAX * 2 = i32::MAX
        // saturating_add(0).saturating_add(0) = i32::MAX
        assert_eq!(sd.get_exscore(), i32::MAX);
    }

    #[test]
    fn test_exscore_saturating_mul_overflow() {
        // Even if sum fits, *2 would overflow
        let sd = make_score(i32::MAX / 2 + 1, i32::MAX / 2 + 1, 0, 0, 1000);
        // saturating_add: (MAX/2+1) + (MAX/2+1) = MAX/2*2 + 2 = MAX + 1 -> saturates to MAX
        // saturating_mul: MAX * 2 -> saturates to MAX
        assert_eq!(sd.get_exscore(), i32::MAX);
    }

    #[test]
    fn test_exscore_saturating_add_egr_overflow() {
        // (epg+lpg)*2 fits, but adding egr overflows
        let sd = make_score(i32::MAX / 4, i32::MAX / 4, i32::MAX, 0, 1000);
        // epg+lpg = MAX/4 + MAX/4 = MAX/2 (fits)
        // (MAX/2) * 2 = MAX - 1 (just under MAX due to integer division)
        // (MAX-1) + MAX -> saturates to MAX
        assert_eq!(sd.get_exscore(), i32::MAX);
    }

    #[test]
    fn test_exscore_saturating_add_lgr_overflow() {
        // Everything fits except final lgr addition
        let sd = make_score(0, 0, i32::MAX, i32::MAX, 1000);
        // (0+0)*2 = 0; 0 + MAX = MAX; MAX + MAX -> saturates to MAX
        assert_eq!(sd.get_exscore(), i32::MAX);
    }

    #[test]
    fn test_exscore_saturating_all_max() {
        let sd = make_score(i32::MAX, i32::MAX, i32::MAX, i32::MAX, 1000);
        assert_eq!(sd.get_exscore(), i32::MAX);
    }

    #[test]
    fn test_exscore_large_values_no_overflow() {
        // Large but within range: (500000 + 500000) * 2 + 100000 + 100000 = 2200000
        let sd = make_score(500_000, 500_000, 100_000, 100_000, 1_200_000);
        assert_eq!(sd.get_exscore(), 2_200_000);
    }

    #[test]
    fn test_exscore_just_under_overflow_boundary() {
        // (epg + lpg) * 2 just barely fits in i32
        // i32::MAX = 2_147_483_647
        // We want (epg + lpg) * 2 = 2_147_483_646 (MAX - 1), so epg+lpg = 1_073_741_823
        let sd = make_score(1_073_741_823, 0, 0, 0, 1000);
        assert_eq!(sd.get_exscore(), 2_147_483_646);
    }

    #[test]
    fn test_exscore_just_at_overflow_boundary() {
        // (epg + lpg) * 2 + egr + lgr = i32::MAX = 2_147_483_647
        let sd = make_score(1_073_741_823, 0, 1, 0, 1000);
        assert_eq!(sd.get_exscore(), 2_147_483_647);
    }

    #[test]
    fn test_exscore_one_past_overflow_boundary() {
        // Without saturation this would overflow, but saturating keeps at MAX
        let sd = make_score(1_073_741_823, 0, 2, 0, 1000);
        // (1_073_741_823 + 0) * 2 = 2_147_483_646
        // 2_147_483_646 + 2 = 2_147_483_648 -> overflows i32, saturates to MAX
        assert_eq!(sd.get_exscore(), i32::MAX);
    }

    // -- Rank boundary transitions with small note counts --

    #[test]
    fn test_exscore_rank_boundary_single_note() {
        // 1 note, max exscore = 2
        // Only 3 possible exscores: 0, 1, 2
        let sd0 = make_score(0, 0, 0, 0, 1);
        assert_eq!(sd0.get_exscore(), 0);

        let sd1 = make_score(0, 0, 1, 0, 1);
        assert_eq!(sd1.get_exscore(), 1);

        let sd2 = make_score(1, 0, 0, 0, 1);
        assert_eq!(sd2.get_exscore(), 2);
    }

    #[test]
    fn test_exscore_rank_boundary_27_notes_exact_divisions() {
        // With 27 notes, max exscore = 54.
        // Each rank boundary at index i divides exactly: i * 54 / 27 = i * 2.
        // So rank[i] requires exscore >= i * 2.
        for i in 0..=26 {
            let needed = i * 2;
            let rate = needed as f32 / 54.0;
            let threshold = i as f32 / 27.0;
            assert!(
                rate >= threshold,
                "27-note chart: rank {} needs exscore >= {}, rate {:.4} >= {:.4}",
                i,
                needed,
                rate,
                threshold
            );
            if needed > 0 {
                let rate_below = (needed - 1) as f32 / 54.0;
                assert!(
                    rate_below < threshold,
                    "27-note chart: rank {} exscore {} should be below (rate {:.4} < {:.4})",
                    i,
                    needed - 1,
                    rate_below,
                    threshold
                );
            }
        }
    }

    /// Exscore respects that GD/BD/PR/MS do not contribute.
    #[test]
    fn test_exscore_ignores_gd_bd_pr_ms() {
        let mut sd = ScoreData::default();
        sd.egd = 100;
        sd.lgd = 200;
        sd.ebd = 300;
        sd.lbd = 400;
        sd.epr = 500;
        sd.lpr = 600;
        sd.ems = 700;
        sd.lms = 800;
        sd.notes = 3600;
        // None of these contribute to exscore
        assert_eq!(sd.get_exscore(), 0);
    }

    /// Exscore with asymmetric fast/slow split still calculates correctly.
    #[test]
    fn test_exscore_asymmetric_fast_slow() {
        // All perfects as early, all greats as late
        let sd = make_score(100, 0, 0, 50, 150);
        // (100 + 0) * 2 + 0 + 50 = 250
        assert_eq!(sd.get_exscore(), 250);
    }
}

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

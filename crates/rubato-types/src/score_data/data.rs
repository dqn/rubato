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
use crate::validatable::Validatable;

use super::{JudgeCounts, PlayOption, SongTrophy, TimingStats};

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
    #[serde(flatten)]
    pub judge_counts: JudgeCounts,
    pub maxcombo: i32,
    pub notes: i32,
    pub passnotes: i32,
    pub minbp: i32,
    #[serde(flatten)]
    pub timing_stats: TimingStats,
    pub trophy: String,
    pub ghost: String,
    #[serde(flatten)]
    pub play_option: PlayOption,
    pub state: i32,
    pub scorehash: String,
    pub playmode: Mode,
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
            judge_counts: JudgeCounts::default(),
            maxcombo: 0,
            notes: 0,
            passnotes: 0,
            minbp: i32::MAX,
            timing_stats: TimingStats::default(),
            trophy: String::new(),
            ghost: String::new(),
            play_option: PlayOption {
                seed: -1,
                ..Default::default()
            },
            state: 0,
            scorehash: String::new(),
            playmode,
        }
    }

    pub fn set_player(&mut self, player: Option<&str>) {
        self.player = player.unwrap_or("").to_string();
    }

    pub fn exscore(&self) -> i32 {
        let jc = &self.judge_counts;
        (jc.epg.saturating_add(jc.lpg))
            .saturating_mul(2)
            .saturating_add(jc.egr)
            .saturating_add(jc.lgr)
    }

    pub fn judge_count_total(&self, judge: i32) -> i32 {
        self.judge_count(judge, true) + self.judge_count(judge, false)
    }

    /// Get judge count for a specific judge type.
    /// judge: 0=PG, 1=GR, 2=GD, 3=BD, 4=PR, 5=MS
    /// fast: true=FAST, false=SLOW
    pub fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        let jc = &self.judge_counts;
        match judge {
            0 => {
                if fast {
                    jc.epg
                } else {
                    jc.lpg
                }
            }
            1 => {
                if fast {
                    jc.egr
                } else {
                    jc.lgr
                }
            }
            2 => {
                if fast {
                    jc.egd
                } else {
                    jc.lgd
                }
            }
            3 => {
                if fast {
                    jc.ebd
                } else {
                    jc.lbd
                }
            }
            4 => {
                if fast {
                    jc.epr
                } else {
                    jc.lpr
                }
            }
            5 => {
                if fast {
                    jc.ems
                } else {
                    jc.lms
                }
            }
            _ => 0,
        }
    }

    pub fn add_judge_count(&mut self, judge: i32, fast: bool, count: i32) {
        let jc = &mut self.judge_counts;
        match judge {
            0 => {
                if fast {
                    jc.epg += count;
                } else {
                    jc.lpg += count;
                }
            }
            1 => {
                if fast {
                    jc.egr += count;
                } else {
                    jc.lgr += count;
                }
            }
            2 => {
                if fast {
                    jc.egd += count;
                } else {
                    jc.lgd += count;
                }
            }
            3 => {
                if fast {
                    jc.ebd += count;
                } else {
                    jc.lbd += count;
                }
            }
            4 => {
                if fast {
                    jc.epr += count;
                } else {
                    jc.lpr += count;
                }
            }
            5 => {
                if fast {
                    jc.ems += count;
                } else {
                    jc.lms += count;
                }
            }
            _ => {}
        }
    }

    pub fn decode_ghost(&self) -> Option<Vec<i32>> {
        if self.ghost.is_empty() {
            return None;
        }
        if self.notes <= 0 {
            return None;
        }
        let decoded = match URL_SAFE.decode(self.ghost.as_bytes()) {
            Ok(d) => d,
            Err(_) => return None,
        };
        // Limit decompression to prevent unbounded memory allocation from
        // malicious/corrupted ghost data.  Java reads exactly `notes` bytes;
        // add a small margin for gzip framing overhead.
        let limit = (self.notes as u64).saturating_mul(4).saturating_add(1024);
        let mut gz = GzDecoder::new(&decoded[..]).take(limit);
        let mut decompressed = Vec::new();
        if gz.read_to_end(&mut decompressed).is_err() {
            return None;
        }
        if decompressed.is_empty() {
            return None;
        }
        let value: Vec<i32> = (0..self.notes as usize)
            .map(|i| {
                if i < decompressed.len() {
                    // Sign-extend u8 to match Java's signed byte semantics:
                    // Java byte is -128..127, values > 127 map to negative (POOR=4).
                    let judge = decompressed[i] as i8 as i32;
                    if judge >= 0 { judge } else { 4 }
                } else {
                    4
                }
            })
            .collect();
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
                // Clamp to 0..=255 to avoid silent truncation of out-of-range judge values.
                let bytes: Vec<u8> = v.iter().map(|&j| j.clamp(0, 255) as u8).collect();
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
            self.play_option.option = newscore.play_option.option;
            self.play_option.seed = newscore.play_option.seed;
            update = true;
        }
        if self.exscore() < newscore.exscore() && update_score {
            self.judge_counts = newscore.judge_counts.clone();
            self.play_option.option = newscore.play_option.option;
            self.play_option.seed = newscore.play_option.seed;
            self.ghost = newscore.ghost.clone();
            update = true;
        }
        if self.timing_stats.avgjudge > newscore.timing_stats.avgjudge && update_score {
            self.timing_stats = newscore.timing_stats.clone();
            self.play_option.option = newscore.play_option.option;
            self.play_option.seed = newscore.play_option.seed;
            update = true;
        }
        if self.minbp > newscore.minbp && update_score {
            self.minbp = newscore.minbp;
            self.play_option.option = newscore.play_option.option;
            self.play_option.seed = newscore.play_option.seed;
            update = true;
        }
        if self.maxcombo < newscore.maxcombo && update_score {
            self.maxcombo = newscore.maxcombo;
            self.play_option.option = newscore.play_option.option;
            self.play_option.seed = newscore.play_option.seed;
            update = true;
        }
        update
    }
}

impl Validatable for ScoreData {
    fn validate(&mut self) -> bool {
        let jc = &self.judge_counts;
        let po = &self.play_option;
        self.mode >= 0
            && self.clear >= 0
            && self.clear <= ClearType::Max.id()
            && jc.epg >= 0
            && jc.lpg >= 0
            && jc.egr >= 0
            && jc.lgr >= 0
            && jc.egd >= 0
            && jc.lgd >= 0
            && jc.ebd >= 0
            && jc.lbd >= 0
            && jc.epr >= 0
            && jc.lpr >= 0
            && jc.ems >= 0
            && jc.lms >= 0
            && self.clearcount >= 0
            && self.playcount >= self.clearcount
            && self.maxcombo >= 0
            && self.notes > 0
            && self.passnotes >= 0
            && self.passnotes <= self.notes
            && self.minbp >= 0
            // NOTE: i64::MAX sentinel (no timing data) intentionally passes this check.
            // Consumers must guard against i64::MAX separately (e.g., bar_sorter compare_duration).
            && self.timing_stats.avgjudge >= 0
            && po.random >= 0
            && po.option >= 0
            && po.assist >= 0
            && po.gauge >= 0
    }
}

impl fmt::Display for ScoreData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let jc = &self.judge_counts;
        let ts = &self.timing_stats;
        let po = &self.play_option;
        write!(f, "{{")?;
        write!(f, "\"Date\": {}, ", self.date)?;
        write!(f, "\"Playcount\": {}, ", self.playcount)?;
        write!(f, "\"Clear\": {}, ", self.clear)?;
        write!(f, "\"Epg\": {}, ", jc.epg)?;
        write!(f, "\"Lpg\": {}, ", jc.lpg)?;
        write!(f, "\"Egr\": {}, ", jc.egr)?;
        write!(f, "\"Lgr\": {}, ", jc.lgr)?;
        write!(f, "\"Egd\": {}, ", jc.egd)?;
        write!(f, "\"Lgd\": {}, ", jc.lgd)?;
        write!(f, "\"Ebd\": {}, ", jc.ebd)?;
        write!(f, "\"Lbd\": {}, ", jc.lbd)?;
        write!(f, "\"Epr\": {}, ", jc.epr)?;
        write!(f, "\"Lpr\": {}, ", jc.lpr)?;
        write!(f, "\"Ems\": {}, ", jc.ems)?;
        write!(f, "\"Lms\": {}, ", jc.lms)?;
        write!(f, "\"Combo\": {}, ", self.maxcombo)?;
        write!(f, "\"Mode\": {}, ", self.mode)?;
        write!(f, "\"Notes\": {}, ", self.notes)?;
        write!(f, "\"Clearcount\": {}, ", self.clearcount)?;
        write!(f, "\"Minbp\": {}, ", self.minbp)?;
        write!(f, "\"Avgjudge\": {}, ", ts.avgjudge)?;
        write!(f, "\"Trophy\": \"{}\", ", self.trophy)?;
        write!(f, "\"Option\": {}, ", po.option)?;
        write!(f, "\"State\": {}, ", self.state)?;
        write!(f, "\"Sha256\": \"{}\", ", self.sha256)?;
        write!(f, "\"Exscore\": {}, ", self.exscore())?;
        write!(f, "\"Random\": {}, ", po.random)?;
        write!(f, "\"Scorehash\": \"{}\", ", self.scorehash)?;
        write!(f, "\"Assist\": {}, ", po.assist)?;
        write!(f, "\"Gauge\": {}, ", po.gauge)?;
        write!(f, "\"DeviceType\": \"{:?}\", ", po.device_type)?;
        write!(f, "\"Playmode\": \"{:?}\", ", self.playmode)?;
        write!(f, "\"Ghost\": \"{}\", ", self.ghost)?;
        write!(f, "\"Passnotes\": {}", self.passnotes)?;
        write!(f, "}}")
    }
}

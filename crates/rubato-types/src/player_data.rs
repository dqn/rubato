use serde::{Deserialize, Serialize};

use crate::validatable::Validatable;

/// Player data
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PlayerData {
    pub date: i64,
    pub playcount: i64,
    pub clear: i64,
    pub epg: i64,
    pub lpg: i64,
    pub egr: i64,
    pub lgr: i64,
    pub egd: i64,
    pub lgd: i64,
    pub ebd: i64,
    pub lbd: i64,
    pub epr: i64,
    pub lpr: i64,
    pub ems: i64,
    pub lms: i64,
    pub playtime: i64,
    pub maxcombo: i64,
}

impl PlayerData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn judge_count(&self, judge: i32) -> i64 {
        self.judge_count_fast(judge, true) + self.judge_count_fast(judge, false)
    }

    /// Get judge count for a specific judge type.
    /// judge: 0=PG, 1=GR, 2=GD, 3=BD, 4=PR, 5=MS
    /// fast: true=FAST, false=SLOW
    pub fn judge_count_fast(&self, judge: i32, fast: bool) -> i64 {
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
}

impl Validatable for PlayerData {
    fn validate(&mut self) -> bool {
        self.clear >= 0
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
            && self.playcount >= self.clear
            && self.maxcombo >= 0
            && self.playtime >= 0
            && self.date > 0
    }
}

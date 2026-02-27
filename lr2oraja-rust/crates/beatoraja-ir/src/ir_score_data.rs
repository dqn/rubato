use beatoraja_core::clear_type::ClearType;
use beatoraja_core::score_data::ScoreData;
use beatoraja_core::stubs::{BMSPlayerRule, JudgeAlgorithm, bms_player_input_device};

/// IR score data
///
/// Translated from: IRScoreData.java
#[derive(Clone, Debug)]
pub struct IRScoreData {
    /// Chart SHA256 hash
    pub sha256: String,
    /// LN TYPE (0: LN, 1: CN, 2: HCN)
    pub lntype: i32,
    /// Player name. Empty string for own score.
    pub player: String,
    /// Clear type
    pub clear: ClearType,
    /// Score last obtained date (unixtime, seconds)
    pub date: i64,
    /// Total PGREAT early count
    pub epg: i32,
    pub lpg: i32,
    /// Total GREAT early count
    pub egr: i32,
    pub lgr: i32,
    /// Total GOOD early count
    pub egd: i32,
    pub lgd: i32,
    /// Total BAD early count
    pub ebd: i32,
    pub lbd: i32,
    /// Total POOR early count
    pub epr: i32,
    pub lpr: i32,
    /// Total MISS early count
    pub ems: i32,
    pub lms: i32,
    /// Average judge
    pub avgjudge: i64,
    /// Maximum combo
    pub maxcombo: i32,
    /// Total notes
    pub notes: i32,
    /// Processed notes
    pub passnotes: i32,
    /// Minimum miss count
    pub minbp: i32,
    /// Option at update time
    pub option: i32,
    /// Seed
    pub seed: i64,
    /// Assist option
    pub assist: i32,
    /// Play gauge
    pub gauge: i32,
    /// Input device
    pub device_type: Option<bms_player_input_device::Type>,
    /// Judge algorithm
    pub judge_algorithm: Option<JudgeAlgorithm>,
    /// Rule
    pub rule: Option<BMSPlayerRule>,
    /// Skin
    pub skin: Option<String>,
}

impl IRScoreData {
    pub fn new(score: &ScoreData) -> Self {
        Self {
            sha256: score.sha256.clone(),
            lntype: score.mode,
            player: score.player.clone(),
            clear: ClearType::get_clear_type_by_id(score.clear),
            date: score.date,
            epg: score.epg,
            lpg: score.lpg,
            egr: score.egr,
            lgr: score.lgr,
            egd: score.egd,
            lgd: score.lgd,
            ebd: score.ebd,
            lbd: score.lbd,
            epr: score.epr,
            lpr: score.lpr,
            ems: score.ems,
            lms: score.lms,
            avgjudge: score.avgjudge,
            maxcombo: score.combo,
            notes: score.notes,
            passnotes: score.passnotes,
            minbp: score.minbp,
            option: score.option,
            seed: score.seed,
            assist: score.assist,
            gauge: score.gauge,
            device_type: score.device_type.clone(),
            judge_algorithm: score.judge_algorithm.clone(),
            rule: score.rule.clone(),
            skin: score.skin.clone(),
        }
    }

    pub fn get_exscore(&self) -> i32 {
        (self.epg + self.lpg) * 2 + self.egr + self.lgr
    }

    #[allow(clippy::field_reassign_with_default)]
    pub fn convert_to_score_data(&self) -> ScoreData {
        let mut score = ScoreData::default();
        score.sha256 = self.sha256.clone();
        score.mode = self.lntype;
        score.player = self.player.clone();
        score.clear = self.clear.id();
        score.date = self.date;
        score.epg = self.epg;
        score.lpg = self.lpg;
        score.egr = self.egr;
        score.lgr = self.lgr;
        score.egd = self.egd;
        score.lgd = self.lgd;
        score.ebd = self.ebd;
        score.lbd = self.lbd;
        score.epr = self.epr;
        score.lpr = self.lpr;
        score.ems = self.ems;
        score.lms = self.lms;
        score.combo = self.maxcombo;
        score.notes = self.notes;
        // Java: score.setPassnotes(this.passnotes != 0 ? this.notes : this.passnotes);
        score.passnotes = if self.passnotes != 0 {
            self.notes
        } else {
            self.passnotes
        };
        score.minbp = self.minbp;
        score.avgjudge = self.avgjudge;
        score.option = self.option;
        score.seed = self.seed;
        score.assist = self.assist;
        score.gauge = self.gauge;
        score.device_type = self.device_type.clone();
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_from_default_score_data() {
        let sd = ScoreData::default();
        let ir = IRScoreData::new(&sd);
        assert_eq!(ir.sha256, "");
        assert_eq!(ir.epg, 0);
        assert_eq!(ir.maxcombo, 0);
        assert_eq!(ir.notes, 0);
    }

    #[test]
    fn exscore_calculation() {
        let mut sd = ScoreData::default();
        sd.epg = 10;
        sd.lpg = 5;
        sd.egr = 3;
        sd.lgr = 2;
        let ir = IRScoreData::new(&sd);
        // exscore = (epg + lpg) * 2 + egr + lgr = (10+5)*2 + 3+2 = 35
        assert_eq!(ir.get_exscore(), 35);
    }

    #[test]
    fn convert_to_score_data_roundtrip() {
        let mut sd = ScoreData::default();
        sd.sha256 = "abc123".to_string();
        sd.epg = 100;
        sd.lpg = 50;
        sd.combo = 42;
        sd.minbp = 3;
        sd.clear = 7;

        let ir = IRScoreData::new(&sd);
        let converted = ir.convert_to_score_data();

        assert_eq!(converted.sha256, "abc123");
        assert_eq!(converted.epg, 100);
        assert_eq!(converted.lpg, 50);
        assert_eq!(converted.combo, 42);
        assert_eq!(converted.minbp, 3);
    }

    #[test]
    fn clone_preserves_all_fields() {
        let mut sd = ScoreData::default();
        sd.sha256 = "test".to_string();
        sd.epg = 7;
        let ir = IRScoreData::new(&sd);
        let cloned = ir.clone();
        assert_eq!(cloned.sha256, "test");
        assert_eq!(cloned.epg, 7);
    }
}

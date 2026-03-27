use crate::core::clear_type::ClearType;
use crate::core::score_data::ScoreData;
use rubato_types::{BMSPlayerRule, JudgeAlgorithm, bms_player_input_device};

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
            clear: ClearType::clear_type_by_id(score.clear),
            date: score.date,
            epg: score.judge_counts.epg,
            lpg: score.judge_counts.lpg,
            egr: score.judge_counts.egr,
            lgr: score.judge_counts.lgr,
            egd: score.judge_counts.egd,
            lgd: score.judge_counts.lgd,
            ebd: score.judge_counts.ebd,
            lbd: score.judge_counts.lbd,
            epr: score.judge_counts.epr,
            lpr: score.judge_counts.lpr,
            ems: score.judge_counts.ems,
            lms: score.judge_counts.lms,
            avgjudge: score.timing_stats.avgjudge,
            maxcombo: score.maxcombo,
            notes: score.notes,
            passnotes: score.passnotes,
            minbp: score.minbp,
            option: score.play_option.option,
            seed: score.play_option.seed,
            assist: score.play_option.assist,
            gauge: score.play_option.gauge,
            device_type: score.play_option.device_type,
            judge_algorithm: score.play_option.judge_algorithm,
            rule: score.play_option.rule,
            skin: score.play_option.skin.clone(),
        }
    }

    pub fn exscore(&self) -> i32 {
        (self.epg + self.lpg) * 2 + self.egr + self.lgr
    }

    pub fn convert_to_score_data(&self) -> ScoreData {
        use rubato_types::score_data::{JudgeCounts, PlayOption, TimingStats};
        ScoreData {
            sha256: self.sha256.clone(),
            mode: self.lntype,
            player: self.player.clone(),
            clear: self.clear.id(),
            date: self.date,
            judge_counts: JudgeCounts {
                epg: self.epg,
                lpg: self.lpg,
                egr: self.egr,
                lgr: self.lgr,
                egd: self.egd,
                lgd: self.lgd,
                ebd: self.ebd,
                lbd: self.lbd,
                epr: self.epr,
                lpr: self.lpr,
                ems: self.ems,
                lms: self.lms,
            },
            maxcombo: self.maxcombo,
            notes: self.notes,
            // Java: score.setPassnotes(this.passnotes != 0 ? this.notes : this.passnotes);
            passnotes: if self.passnotes != 0 {
                self.notes
            } else {
                self.passnotes
            },
            minbp: self.minbp,
            timing_stats: TimingStats {
                avgjudge: self.avgjudge,
                ..Default::default()
            },
            play_option: PlayOption {
                option: self.option,
                seed: self.seed,
                assist: self.assist,
                gauge: self.gauge,
                device_type: self.device_type,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use bms::model::mode::Mode;

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
        sd.judge_counts.epg = 10;
        sd.judge_counts.lpg = 5;
        sd.judge_counts.egr = 3;
        sd.judge_counts.lgr = 2;
        let ir = IRScoreData::new(&sd);
        // exscore = (epg + lpg) * 2 + egr + lgr = (10+5)*2 + 3+2 = 35
        assert_eq!(ir.exscore(), 35);
    }

    #[test]
    fn convert_to_score_data_roundtrip() {
        let mut sd = ScoreData::default();
        sd.sha256 = "abc123".to_string();
        sd.judge_counts.epg = 100;
        sd.judge_counts.lpg = 50;
        sd.maxcombo = 42;
        sd.minbp = 3;
        sd.clear = 7;

        let ir = IRScoreData::new(&sd);
        let converted = ir.convert_to_score_data();

        assert_eq!(converted.sha256, "abc123");
        assert_eq!(converted.judge_counts.epg, 100);
        assert_eq!(converted.judge_counts.lpg, 50);
        assert_eq!(converted.maxcombo, 42);
        assert_eq!(converted.minbp, 3);
    }

    #[test]
    fn clone_preserves_all_fields() {
        let mut sd = ScoreData::default();
        sd.sha256 = "test".to_string();
        sd.judge_counts.epg = 7;
        let ir = IRScoreData::new(&sd);
        let cloned = ir.clone();
        assert_eq!(cloned.sha256, "test");
        assert_eq!(cloned.epg, 7);
    }

    fn make_score_data() -> ScoreData {
        let mut s = ScoreData::new(Mode::BEAT_7K);
        s.sha256 = "abc123def456".to_string();
        s.mode = 1;
        s.player = "TestPlayer".to_string();
        s.clear = 5; // Normal
        s.date = 1700000000;
        s.judge_counts.epg = 100;
        s.judge_counts.lpg = 90;
        s.judge_counts.egr = 50;
        s.judge_counts.lgr = 40;
        s.judge_counts.egd = 10;
        s.judge_counts.lgd = 8;
        s.judge_counts.ebd = 3;
        s.judge_counts.lbd = 2;
        s.judge_counts.epr = 1;
        s.judge_counts.lpr = 0;
        s.judge_counts.ems = 0;
        s.judge_counts.lms = 1;
        s.timing_stats.avgjudge = 500;
        s.maxcombo = 280;
        s.notes = 305;
        s.passnotes = 300;
        s.minbp = 7;
        s.play_option.option = 2;
        s.play_option.seed = 42;
        s.play_option.assist = 0;
        s.play_option.gauge = 3;
        s
    }

    #[test]
    fn test_ir_score_data_new_copies_all_fields() {
        let score = make_score_data();
        let ir = IRScoreData::new(&score);

        assert_eq!(ir.sha256, "abc123def456");
        assert_eq!(ir.lntype, 1);
        assert_eq!(ir.player, "TestPlayer");
        assert_eq!(ir.clear, ClearType::Normal);
        assert_eq!(ir.date, 1700000000);
        assert_eq!(ir.epg, 100);
        assert_eq!(ir.lpg, 90);
        assert_eq!(ir.egr, 50);
        assert_eq!(ir.lgr, 40);
        assert_eq!(ir.egd, 10);
        assert_eq!(ir.lgd, 8);
        assert_eq!(ir.ebd, 3);
        assert_eq!(ir.lbd, 2);
        assert_eq!(ir.epr, 1);
        assert_eq!(ir.lpr, 0);
        assert_eq!(ir.ems, 0);
        assert_eq!(ir.lms, 1);
        assert_eq!(ir.avgjudge, 500);
        assert_eq!(ir.maxcombo, 280);
        assert_eq!(ir.notes, 305);
        assert_eq!(ir.passnotes, 300);
        assert_eq!(ir.minbp, 7);
        assert_eq!(ir.option, 2);
        assert_eq!(ir.seed, 42);
        assert_eq!(ir.assist, 0);
        assert_eq!(ir.gauge, 3);
    }

    #[test]
    fn test_get_exscore_calculation() {
        let score = make_score_data();
        let ir = IRScoreData::new(&score);
        // exscore = (epg + lpg) * 2 + egr + lgr
        // = (100 + 90) * 2 + 50 + 40 = 380 + 90 = 470
        assert_eq!(ir.exscore(), 470);
    }

    #[test]
    fn test_get_exscore_zero_when_all_zero() {
        let s = ScoreData::default();
        let ir = IRScoreData::new(&s);
        assert_eq!(ir.exscore(), 0);
    }

    #[test]
    fn test_convert_to_score_data_roundtrip() {
        let original = make_score_data();
        let ir = IRScoreData::new(&original);
        let converted = ir.convert_to_score_data();

        assert_eq!(converted.sha256, original.sha256);
        assert_eq!(converted.mode, original.mode);
        assert_eq!(converted.player, original.player);
        assert_eq!(converted.clear, original.clear);
        assert_eq!(converted.date, original.date);
        assert_eq!(converted.judge_counts.epg, original.judge_counts.epg);
        assert_eq!(converted.judge_counts.lpg, original.judge_counts.lpg);
        assert_eq!(converted.judge_counts.egr, original.judge_counts.egr);
        assert_eq!(converted.judge_counts.lgr, original.judge_counts.lgr);
        assert_eq!(converted.judge_counts.egd, original.judge_counts.egd);
        assert_eq!(converted.judge_counts.lgd, original.judge_counts.lgd);
        assert_eq!(converted.judge_counts.ebd, original.judge_counts.ebd);
        assert_eq!(converted.judge_counts.lbd, original.judge_counts.lbd);
        assert_eq!(converted.judge_counts.epr, original.judge_counts.epr);
        assert_eq!(converted.judge_counts.lpr, original.judge_counts.lpr);
        assert_eq!(converted.judge_counts.ems, original.judge_counts.ems);
        assert_eq!(converted.judge_counts.lms, original.judge_counts.lms);
        assert_eq!(converted.maxcombo, original.maxcombo);
        assert_eq!(converted.notes, original.notes);
        assert_eq!(converted.minbp, original.minbp);
        assert_eq!(
            converted.timing_stats.avgjudge,
            original.timing_stats.avgjudge
        );
        assert_eq!(converted.play_option.option, original.play_option.option);
        assert_eq!(converted.play_option.seed, original.play_option.seed);
        assert_eq!(converted.play_option.assist, original.play_option.assist);
        assert_eq!(converted.play_option.gauge, original.play_option.gauge);
    }

    #[test]
    fn test_convert_to_score_data_passnotes_nonzero_uses_notes() {
        // When passnotes != 0, converted score.passnotes should be self.notes
        let s = ScoreData {
            notes: 500,
            passnotes: 100,
            ..Default::default()
        };
        let ir = IRScoreData::new(&s);
        let converted = ir.convert_to_score_data();
        assert_eq!(converted.passnotes, 500); // uses notes, not passnotes
    }

    #[test]
    fn test_convert_to_score_data_passnotes_zero_uses_passnotes() {
        // When passnotes == 0, converted score.passnotes should be self.passnotes (0)
        let s = ScoreData {
            notes: 500,
            passnotes: 0,
            ..Default::default()
        };
        let ir = IRScoreData::new(&s);
        let converted = ir.convert_to_score_data();
        assert_eq!(converted.passnotes, 0);
    }
}

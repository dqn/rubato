use crate::ir::ir_score_data::IRScoreData;

/// IR type enum
///
/// Translated from: LeaderboardEntry.IRType
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IRType {
    Primary,
    LR2,
}

/// Leaderboard entry
///
/// Translated from: LeaderboardEntry.java
#[derive(Clone, Debug)]
pub struct LeaderboardEntry {
    ir_score: IRScoreData,
    ir_type: IRType,
    lr2_id: i64,
}

impl LeaderboardEntry {
    fn new(ir_score: IRScoreData, ir_type: IRType) -> Self {
        Self {
            ir_score,
            ir_type,
            lr2_id: 0,
        }
    }

    pub fn new_entry_primary_ir(ir_score: IRScoreData) -> Self {
        Self::new(ir_score, IRType::Primary)
    }

    pub fn new_entry_lr2_ir(ir_score: IRScoreData, lr2_id: i64) -> Self {
        let mut entry = Self::new(ir_score, IRType::LR2);
        entry.lr2_id = lr2_id;
        entry
    }

    pub fn ir_score(&self) -> &IRScoreData {
        &self.ir_score
    }

    pub fn into_ir_score(self) -> IRScoreData {
        self.ir_score
    }

    pub fn is_primary_ir(&self) -> bool {
        self.ir_type == IRType::Primary
    }

    pub fn is_lr2_ir(&self) -> bool {
        self.ir_type == IRType::LR2
    }

    pub fn lr2_id(&self) -> i64 {
        self.lr2_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::score_data::ScoreData;

    fn make_ir_score() -> IRScoreData {
        IRScoreData::new(&ScoreData::default())
    }

    #[test]
    fn test_new_entry_primary_ir() {
        let entry = LeaderboardEntry::new_entry_primary_ir(make_ir_score());
        assert!(entry.is_primary_ir());
        assert!(!entry.is_lr2_ir());
        assert_eq!(entry.lr2_id(), 0);
    }

    #[test]
    fn test_new_entry_lr2_ir() {
        let entry = LeaderboardEntry::new_entry_lr2_ir(make_ir_score(), 12345);
        assert!(entry.is_lr2_ir());
        assert!(!entry.is_primary_ir());
        assert_eq!(entry.lr2_id(), 12345);
    }

    #[test]
    fn test_get_ir_score_returns_reference() {
        let mut s = ScoreData::default();
        s.judge_counts.epg = 100;
        s.judge_counts.lpg = 50;
        let ir = IRScoreData::new(&s);
        let entry = LeaderboardEntry::new_entry_primary_ir(ir);
        assert_eq!(entry.ir_score().epg, 100);
        assert_eq!(entry.ir_score().lpg, 50);
    }
}

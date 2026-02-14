// Target property — score target for play screen comparison.
//
// Ported from Java `TargetProperty.java`.
// Computes a target EX score for real-time score difference display.

use bms_ir::RankingData;
use bms_ir::RankingState;

/// Rival target sub-type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RivalTarget {
    /// Specific rival by registration index.
    Index,
    /// Next N ranks above player among all rivals + self.
    Next,
    /// Nth rank among all rivals + self.
    Rank,
}

/// IR target sub-type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IRTarget {
    /// N ranks above current player in IR.
    Next,
    /// Nth rank in IR.
    Rank,
    /// Top N% in IR.
    RankRate,
}

/// A rival score entry: (player_name, exscore).
#[derive(Debug, Clone)]
pub struct RivalScore {
    pub name: String,
    pub exscore: i32,
}

/// Context for target score computation.
///
/// Provides access to rival scores and IR ranking data needed by
/// Rival/IR target variants.
pub struct TargetContext<'a> {
    pub total_notes: i32,
    pub current_exscore: i32,
    /// Rival scores sorted by exscore descending. Self is included with name = "".
    pub rival_scores: Option<&'a [RivalScore]>,
    /// IR ranking data (None if IR is not available).
    pub ranking_data: Option<&'a RankingData>,
}

/// Score target for play screen comparison.
///
/// Four categories exist (matches Java):
/// 1. StaticRate — fixed percentage targets (AAA, AA, MAX, etc.)
/// 2. NextRank — the next rank threshold above the player's old score
/// 3. Rival — rival player score targets
/// 4. InternetRanking — IR ranking score targets
pub enum TargetProperty {
    /// Fixed score rate target (e.g., AAA = 24/27).
    StaticRate { name: String, rate: f32 },
    /// Next rank above the player's current best score.
    NextRank,
    /// Rival player target.
    Rival { target: RivalTarget, index: usize },
    /// Internet ranking target.
    InternetRanking { target: IRTarget, value: usize },
}

impl TargetProperty {
    /// Resolve a target ID string into a TargetProperty.
    ///
    /// Matches the Java `TargetProperty.getTargetProperty()` dispatch:
    /// StaticTargetProperty -> RivalTargetProperty -> IRTargetProperty -> NextRank -> MAX fallback
    pub fn resolve(id: &str) -> Self {
        // Static rate targets
        match id {
            "RATE_A-" => {
                return Self::static_rate("RANK A-", 100.0 * 17.0 / 27.0);
            }
            "RATE_A" => {
                return Self::static_rate("RANK A", 100.0 * 18.0 / 27.0);
            }
            "RATE_A+" => {
                return Self::static_rate("RANK A+", 100.0 * 19.0 / 27.0);
            }
            "RATE_AA-" => {
                return Self::static_rate("RANK AA-", 100.0 * 20.0 / 27.0);
            }
            "RATE_AA" => {
                return Self::static_rate("RANK AA", 100.0 * 21.0 / 27.0);
            }
            "RATE_AA+" => {
                return Self::static_rate("RANK AA+", 100.0 * 22.0 / 27.0);
            }
            "RATE_AAA-" => {
                return Self::static_rate("RANK AAA-", 100.0 * 23.0 / 27.0);
            }
            "RATE_AAA" => {
                return Self::static_rate("RANK AAA", 100.0 * 24.0 / 27.0);
            }
            "RATE_AAA+" => {
                return Self::static_rate("RANK AAA+", 100.0 * 25.0 / 27.0);
            }
            "RATE_MAX-" => {
                return Self::static_rate("RANK MAX-", 100.0 * 26.0 / 27.0);
            }
            "MAX" => {
                return Self::static_rate("MAX", 100.0);
            }
            "RANK_NEXT" => {
                return Self::NextRank;
            }
            _ => {}
        }

        // Custom rate: RATE_<float>
        if let Some(suffix) = id.strip_prefix("RATE_")
            && let Ok(rate) = suffix.parse::<f32>()
            && (0.0..=100.0).contains(&rate)
        {
            return Self::StaticRate {
                name: format!("SCORE RATE {rate}%"),
                rate,
            };
        }

        // Rival targets: RIVAL_NEXT_N, RIVAL_RANK_N, RIVAL_N
        if let Some(tp) = Self::parse_rival(id) {
            return tp;
        }

        // IR targets: IR_NEXT_N, IR_RANK_N, IR_RANKRATE_N
        if let Some(tp) = Self::parse_ir(id) {
            return tp;
        }

        // Default fallback: MAX
        Self::static_rate("MAX", 100.0)
    }

    /// Parse rival target IDs.
    ///
    /// Java `RivalTargetProperty.getTargetProperty()` (L264-293):
    /// - RIVAL_NEXT_N -> Next, index = N-1
    /// - RIVAL_RANK_N -> Rank, index = N-1
    /// - RIVAL_N -> Index, index = N-1
    fn parse_rival(id: &str) -> Option<Self> {
        if let Some(suffix) = id.strip_prefix("RIVAL_NEXT_")
            && let Ok(n) = suffix.parse::<usize>()
            && n > 0
        {
            return Some(Self::Rival {
                target: RivalTarget::Next,
                index: n - 1,
            });
        }
        if let Some(suffix) = id.strip_prefix("RIVAL_RANK_")
            && let Ok(n) = suffix.parse::<usize>()
            && n > 0
        {
            return Some(Self::Rival {
                target: RivalTarget::Rank,
                index: n - 1,
            });
        }
        if let Some(suffix) = id.strip_prefix("RIVAL_")
            && let Ok(n) = suffix.parse::<usize>()
            && n > 0
        {
            return Some(Self::Rival {
                target: RivalTarget::Index,
                index: n - 1,
            });
        }
        None
    }

    /// Parse IR target IDs.
    ///
    /// Java `InternetRankingTargetProperty.getTargetProperty()` (L441-473):
    /// - IR_NEXT_N -> Next, value = N (N > 0)
    /// - IR_RANK_N -> Rank, value = N (N > 0)
    /// - IR_RANKRATE_N -> RankRate, value = N (0 < N < 100)
    fn parse_ir(id: &str) -> Option<Self> {
        if let Some(suffix) = id.strip_prefix("IR_NEXT_")
            && let Ok(n) = suffix.parse::<usize>()
            && n > 0
        {
            return Some(Self::InternetRanking {
                target: IRTarget::Next,
                value: n,
            });
        }
        if let Some(suffix) = id.strip_prefix("IR_RANK_")
            && let Ok(n) = suffix.parse::<usize>()
            && n > 0
        {
            return Some(Self::InternetRanking {
                target: IRTarget::Rank,
                value: n,
            });
        }
        if let Some(suffix) = id.strip_prefix("IR_RANKRATE_")
            && let Ok(n) = suffix.parse::<usize>()
            && n > 0
            && n < 100
        {
            return Some(Self::InternetRanking {
                target: IRTarget::RankRate,
                value: n,
            });
        }
        None
    }

    /// Compute the target EX score.
    ///
    /// For `StaticRate`: `ceil(total_notes * 2 * rate / 100)`
    /// For `NextRank`: scans rank thresholds 15/27..26/27 to find the first
    /// one exceeding `current_exscore`, falling back to MAX.
    /// For `Rival`: uses rival score data from context.
    /// For `InternetRanking`: uses IR ranking data from context.
    ///
    /// Returns `(target_exscore, target_name)`.
    pub fn compute_target(&self, ctx: &TargetContext) -> (i32, String) {
        match self {
            Self::StaticRate { name, rate } => {
                let score = (ctx.total_notes as f32 * 2.0 * rate / 100.0).ceil() as i32;
                (score, name.clone())
            }
            Self::NextRank => {
                let max = ctx.total_notes * 2;
                let mut target_score = max;
                for i in 15..27 {
                    let target = (max as f32 * i as f32 / 27.0).ceil() as i32;
                    if ctx.current_exscore < target {
                        target_score = target;
                        break;
                    }
                }
                (target_score, "NEXT RANK".to_string())
            }
            Self::Rival { target, index } => self.compute_rival(ctx, *target, *index),
            Self::InternetRanking { target, value } => self.compute_ir(ctx, *target, *value),
        }
    }

    /// Compute rival target score.
    ///
    /// Java `RivalTargetProperty.getTarget()` (L188-243).
    fn compute_rival(
        &self,
        ctx: &TargetContext,
        target: RivalTarget,
        index: usize,
    ) -> (i32, String) {
        let Some(scores) = ctx.rival_scores else {
            return self.max_fallback(ctx.total_notes);
        };
        if scores.is_empty() {
            return self.max_fallback(ctx.total_notes);
        }

        match target {
            RivalTarget::Index => {
                if let Some(rival) = scores.iter().filter(|s| !s.name.is_empty()).nth(index) {
                    (rival.exscore, format!("RIVAL {}", rival.name))
                } else {
                    (0, "NO DATA".to_string())
                }
            }
            RivalTarget::Rank => {
                let clamped = if index < scores.len() {
                    index
                } else {
                    scores.len() - 1
                };
                let s = &scores[clamped];
                let name = if index == 0 {
                    "RIVAL TOP".to_string()
                } else {
                    format!("RIVAL RANK {}", index + 1)
                };
                (s.exscore, name)
            }
            RivalTarget::Next => {
                // Find self (player = "") position, then go `index` ranks up
                let self_pos = scores
                    .iter()
                    .position(|s| s.name.is_empty())
                    .unwrap_or(scores.len().saturating_sub(1));
                let target_pos = self_pos.saturating_sub(index);
                // Java: Math.max(scores.length - 1 - index, 0) as default
                // then overrides with self_pos - index if self found
                let s = &scores[target_pos];
                (s.exscore, format!("RIVAL NEXT {}", index + 1))
            }
        }
    }

    /// Compute IR target score.
    ///
    /// Java `InternetRankingTargetProperty.getTarget()` (L364-415) and
    /// `getTargetRank()` (L417-438).
    fn compute_ir(&self, ctx: &TargetContext, target: IRTarget, value: usize) -> (i32, String) {
        let Some(ranking) = ctx.ranking_data else {
            return self.max_fallback(ctx.total_notes);
        };
        if ranking.state() != RankingState::Finish || ranking.total_player() <= 0 {
            return self.max_fallback(ctx.total_notes);
        }

        let total = ranking.total_player() as usize;
        let rank_index = match target {
            IRTarget::Next => {
                // Find first score with exscore <= nowscore, then go `value` above
                let mut idx = 0usize;
                for i in 0..total {
                    if let Some(s) = ranking.score(i)
                        && s.exscore() <= ctx.current_exscore
                    {
                        idx = i.saturating_sub(value);
                        break;
                    }
                }
                idx
            }
            IRTarget::Rank => {
                // Java: Math.min(totalPlayer, value) - 1
                total.min(value) - 1
            }
            IRTarget::RankRate => {
                // Java: totalPlayer * value / 100
                total * value / 100
            }
        };

        if let Some(ir_score) = ranking.score(rank_index) {
            let exscore = ir_score.exscore();
            let player_name = if ir_score.player.is_empty() {
                "YOU".to_string()
            } else {
                ir_score.player.clone()
            };
            let name = match target {
                IRTarget::Next => format!("IR NEXT {value}RANK"),
                IRTarget::Rank => format!("IR RANK {value}"),
                IRTarget::RankRate => format!("IR RANK TOP {value}%"),
            };
            (exscore, format!("{name} ({player_name})"))
        } else {
            self.max_fallback(ctx.total_notes)
        }
    }

    /// Fallback to MAX score.
    fn max_fallback(&self, total_notes: i32) -> (i32, String) {
        let score = total_notes * 2;
        (score, "MAX".to_string())
    }

    fn static_rate(name: &str, rate: f32) -> Self {
        Self::StaticRate {
            name: name.to_string(),
            rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_ctx(total_notes: i32, current_exscore: i32) -> TargetContext<'static> {
        TargetContext {
            total_notes,
            current_exscore,
            rival_scores: None,
            ranking_data: None,
        }
    }

    // --- Parsing tests ---

    #[test]
    fn resolve_known_ids() {
        assert!(matches!(
            TargetProperty::resolve("MAX"),
            TargetProperty::StaticRate { rate, .. } if (rate - 100.0).abs() < f32::EPSILON
        ));
        assert!(matches!(
            TargetProperty::resolve("RATE_AAA"),
            TargetProperty::StaticRate { rate, .. } if (rate - 100.0 * 24.0 / 27.0).abs() < 0.01
        ));
        assert!(matches!(
            TargetProperty::resolve("RANK_NEXT"),
            TargetProperty::NextRank
        ));
    }

    #[test]
    fn resolve_custom_rate() {
        let tp = TargetProperty::resolve("RATE_55.5");
        assert!(matches!(
            tp,
            TargetProperty::StaticRate { rate, .. } if (rate - 55.5).abs() < 0.01
        ));
    }

    #[test]
    fn resolve_out_of_range_rate_falls_back_to_max() {
        let tp = TargetProperty::resolve("RATE_150.0");
        assert!(matches!(
            tp,
            TargetProperty::StaticRate { rate, .. } if (rate - 100.0).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn resolve_rival_index() {
        let tp = TargetProperty::resolve("RIVAL_1");
        assert!(matches!(
            tp,
            TargetProperty::Rival {
                target: RivalTarget::Index,
                index: 0
            }
        ));
        let tp = TargetProperty::resolve("RIVAL_3");
        assert!(matches!(
            tp,
            TargetProperty::Rival {
                target: RivalTarget::Index,
                index: 2
            }
        ));
    }

    #[test]
    fn resolve_rival_next() {
        let tp = TargetProperty::resolve("RIVAL_NEXT_1");
        assert!(matches!(
            tp,
            TargetProperty::Rival {
                target: RivalTarget::Next,
                index: 0
            }
        ));
        let tp = TargetProperty::resolve("RIVAL_NEXT_3");
        assert!(matches!(
            tp,
            TargetProperty::Rival {
                target: RivalTarget::Next,
                index: 2
            }
        ));
    }

    #[test]
    fn resolve_rival_rank() {
        let tp = TargetProperty::resolve("RIVAL_RANK_1");
        assert!(matches!(
            tp,
            TargetProperty::Rival {
                target: RivalTarget::Rank,
                index: 0
            }
        ));
        let tp = TargetProperty::resolve("RIVAL_RANK_2");
        assert!(matches!(
            tp,
            TargetProperty::Rival {
                target: RivalTarget::Rank,
                index: 1
            }
        ));
    }

    #[test]
    fn resolve_rival_zero_falls_back() {
        // RIVAL_0 should not parse (Java: index > 0 required)
        let tp = TargetProperty::resolve("RIVAL_0");
        assert!(matches!(tp, TargetProperty::StaticRate { .. }));
    }

    #[test]
    fn resolve_ir_next() {
        let tp = TargetProperty::resolve("IR_NEXT_5");
        assert!(matches!(
            tp,
            TargetProperty::InternetRanking {
                target: IRTarget::Next,
                value: 5
            }
        ));
    }

    #[test]
    fn resolve_ir_rank() {
        let tp = TargetProperty::resolve("IR_RANK_1");
        assert!(matches!(
            tp,
            TargetProperty::InternetRanking {
                target: IRTarget::Rank,
                value: 1
            }
        ));
    }

    #[test]
    fn resolve_ir_rankrate() {
        let tp = TargetProperty::resolve("IR_RANKRATE_10");
        assert!(matches!(
            tp,
            TargetProperty::InternetRanking {
                target: IRTarget::RankRate,
                value: 10
            }
        ));
    }

    #[test]
    fn resolve_ir_rankrate_boundary() {
        // IR_RANKRATE_0 invalid (must be > 0)
        let tp = TargetProperty::resolve("IR_RANKRATE_0");
        assert!(matches!(tp, TargetProperty::StaticRate { .. }));
        // IR_RANKRATE_100 invalid (must be < 100)
        let tp = TargetProperty::resolve("IR_RANKRATE_100");
        assert!(matches!(tp, TargetProperty::StaticRate { .. }));
        // IR_RANKRATE_99 valid
        let tp = TargetProperty::resolve("IR_RANKRATE_99");
        assert!(matches!(
            tp,
            TargetProperty::InternetRanking {
                target: IRTarget::RankRate,
                value: 99
            }
        ));
    }

    #[test]
    fn resolve_unknown_falls_back_to_max() {
        let tp = TargetProperty::resolve("SOMETHING_UNKNOWN");
        assert!(matches!(
            tp,
            TargetProperty::StaticRate { rate, .. } if (rate - 100.0).abs() < f32::EPSILON
        ));
    }

    // --- Static/NextRank computation tests ---

    #[test]
    fn compute_static_max() {
        let tp = TargetProperty::resolve("MAX");
        let ctx = simple_ctx(500, 0);
        let (score, name) = tp.compute_target(&ctx);
        assert_eq!(score, 1000);
        assert_eq!(name, "MAX");
    }

    #[test]
    fn compute_static_aaa() {
        let tp = TargetProperty::resolve("RATE_AAA");
        let ctx = simple_ctx(500, 0);
        let (score, _) = tp.compute_target(&ctx);
        let expected = (500.0f64 * 2.0 * 100.0 * 24.0 / 27.0 / 100.0).ceil() as i32;
        assert_eq!(score, expected);
    }

    #[test]
    fn compute_next_rank_below_b() {
        let tp = TargetProperty::resolve("RANK_NEXT");
        let ctx = simple_ctx(500, 400);
        let (score, name) = tp.compute_target(&ctx);
        let expected = (1000.0f32 * 15.0 / 27.0).ceil() as i32;
        assert_eq!(score, expected);
        assert_eq!(name, "NEXT RANK");
    }

    #[test]
    fn compute_next_rank_above_all() {
        let tp = TargetProperty::resolve("RANK_NEXT");
        let ctx = simple_ctx(500, 999);
        let (score, _) = tp.compute_target(&ctx);
        assert_eq!(score, 1000);
    }

    #[test]
    fn compute_next_rank_at_threshold() {
        let tp = TargetProperty::resolve("RANK_NEXT");
        let b_threshold = (1000.0f32 * 15.0 / 27.0).ceil() as i32;
        let ctx = simple_ctx(500, b_threshold);
        let (score, _) = tp.compute_target(&ctx);
        let expected = (1000.0f32 * 16.0 / 27.0).ceil() as i32;
        assert_eq!(score, expected);
    }

    // --- Rival computation tests ---

    fn make_rival_scores() -> Vec<RivalScore> {
        // Sorted by exscore descending
        vec![
            RivalScore {
                name: "Alice".to_string(),
                exscore: 800,
            },
            RivalScore {
                name: "Bob".to_string(),
                exscore: 600,
            },
            RivalScore {
                name: String::new(),
                exscore: 500,
            }, // self
            RivalScore {
                name: "Carol".to_string(),
                exscore: 400,
            },
        ]
    }

    #[test]
    fn compute_rival_index() {
        let tp = TargetProperty::resolve("RIVAL_1");
        let scores = make_rival_scores();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, name) = tp.compute_target(&ctx);
        // First non-empty rival: Alice (800)
        assert_eq!(score, 800);
        assert_eq!(name, "RIVAL Alice");
    }

    #[test]
    fn compute_rival_index_second() {
        let tp = TargetProperty::resolve("RIVAL_2");
        let scores = make_rival_scores();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, name) = tp.compute_target(&ctx);
        // Second non-empty rival: Bob (600)
        assert_eq!(score, 600);
        assert_eq!(name, "RIVAL Bob");
    }

    #[test]
    fn compute_rival_index_out_of_range() {
        let tp = TargetProperty::resolve("RIVAL_10");
        let scores = make_rival_scores();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, name) = tp.compute_target(&ctx);
        assert_eq!(score, 0);
        assert_eq!(name, "NO DATA");
    }

    #[test]
    fn compute_rival_rank() {
        let tp = TargetProperty::resolve("RIVAL_RANK_1");
        let scores = make_rival_scores();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, name) = tp.compute_target(&ctx);
        // Rank 1 = index 0 = Alice (800)
        assert_eq!(score, 800);
        assert_eq!(name, "RIVAL TOP");
    }

    #[test]
    fn compute_rival_rank_second() {
        let tp = TargetProperty::resolve("RIVAL_RANK_2");
        let scores = make_rival_scores();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, name) = tp.compute_target(&ctx);
        // Rank 2 = index 1 = Bob (600)
        assert_eq!(score, 600);
        assert_eq!(name, "RIVAL RANK 2");
    }

    #[test]
    fn compute_rival_rank_out_of_range() {
        let tp = TargetProperty::resolve("RIVAL_RANK_10");
        let scores = make_rival_scores();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, _) = tp.compute_target(&ctx);
        // Clamped to last: Carol (400)
        assert_eq!(score, 400);
    }

    #[test]
    fn compute_rival_next() {
        // RIVAL_NEXT_1 -> index=0: Java rank = max(self_pos - 0, 0) = self
        let tp = TargetProperty::resolve("RIVAL_NEXT_1");
        let scores = make_rival_scores();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, name) = tp.compute_target(&ctx);
        // Self is at index 2, index=0 -> scores[2] = self (500)
        assert_eq!(score, 500);
        assert_eq!(name, "RIVAL NEXT 1");
    }

    #[test]
    fn compute_rival_next_one_above() {
        // RIVAL_NEXT_2 -> index=1: Java rank = max(self_pos - 1, 0) = 1 rank above
        let tp = TargetProperty::resolve("RIVAL_NEXT_2");
        let scores = make_rival_scores();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, name) = tp.compute_target(&ctx);
        // Self at index 2, index=1 -> scores[1] = Bob (600)
        assert_eq!(score, 600);
        assert_eq!(name, "RIVAL NEXT 2");
    }

    #[test]
    fn compute_rival_next_above_top() {
        // RIVAL_NEXT_4 -> index=3: saturates to 0 = Alice (800)
        let tp = TargetProperty::resolve("RIVAL_NEXT_4");
        let scores = make_rival_scores();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, _) = tp.compute_target(&ctx);
        assert_eq!(score, 800);
    }

    #[test]
    fn compute_rival_no_data_fallback() {
        let tp = TargetProperty::resolve("RIVAL_1");
        let ctx = simple_ctx(500, 500);
        let (score, name) = tp.compute_target(&ctx);
        // No rival_scores -> MAX fallback
        assert_eq!(score, 1000);
        assert_eq!(name, "MAX");
    }

    #[test]
    fn compute_rival_empty_scores_fallback() {
        let tp = TargetProperty::resolve("RIVAL_1");
        let scores: Vec<RivalScore> = vec![];
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 500,
            rival_scores: Some(&scores),
            ranking_data: None,
        };
        let (score, name) = tp.compute_target(&ctx);
        assert_eq!(score, 1000);
        assert_eq!(name, "MAX");
    }

    // --- IR computation tests ---

    fn make_ranking_data() -> RankingData {
        use bms_ir::IRScoreData;
        use bms_rule::ClearType;

        let mut rd = RankingData::new();
        let scores = vec![
            {
                let mut sd = bms_rule::ScoreData::default();
                sd.player = "Top".to_string();
                sd.epg = 400;
                sd.clear = ClearType::Hard;
                IRScoreData::from(&sd)
            }, // exscore: 800
            {
                let mut sd = bms_rule::ScoreData::default();
                sd.player = String::new(); // self
                sd.epg = 300;
                sd.clear = ClearType::Normal;
                IRScoreData::from(&sd)
            }, // exscore: 600
            {
                let mut sd = bms_rule::ScoreData::default();
                sd.player = "Mid".to_string();
                sd.epg = 200;
                sd.clear = ClearType::Easy;
                IRScoreData::from(&sd)
            }, // exscore: 400
            {
                let mut sd = bms_rule::ScoreData::default();
                sd.player = "Low".to_string();
                sd.epg = 100;
                sd.clear = ClearType::Failed;
                IRScoreData::from(&sd)
            }, // exscore: 200
        ];
        rd.update_score(&scores, None);
        rd
    }

    #[test]
    fn compute_ir_next() {
        let tp = TargetProperty::resolve("IR_NEXT_1");
        let ranking = make_ranking_data();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 600,
            rival_scores: None,
            ranking_data: Some(&ranking),
        };
        let (score, name) = tp.compute_target(&ctx);
        // Self (600) is at index 1. First score with exscore <= 600 is index 1.
        // target = max(1 - 1, 0) = 0 -> Top (800)
        assert_eq!(score, 800);
        assert!(name.contains("IR NEXT 1RANK"));
    }

    #[test]
    fn compute_ir_rank() {
        let tp = TargetProperty::resolve("IR_RANK_1");
        let ranking = make_ranking_data();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 0,
            rival_scores: None,
            ranking_data: Some(&ranking),
        };
        let (score, name) = tp.compute_target(&ctx);
        // Rank 1 = index 0 = Top (800)
        assert_eq!(score, 800);
        assert!(name.contains("IR RANK 1"));
    }

    #[test]
    fn compute_ir_rank_beyond_total() {
        let tp = TargetProperty::resolve("IR_RANK_10");
        let ranking = make_ranking_data();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 0,
            rival_scores: None,
            ranking_data: Some(&ranking),
        };
        let (score, _) = tp.compute_target(&ctx);
        // min(4, 10) - 1 = 3 -> Low (200)
        assert_eq!(score, 200);
    }

    #[test]
    fn compute_ir_rankrate() {
        let tp = TargetProperty::resolve("IR_RANKRATE_50");
        let ranking = make_ranking_data();
        let ctx = TargetContext {
            total_notes: 500,
            current_exscore: 0,
            rival_scores: None,
            ranking_data: Some(&ranking),
        };
        let (score, name) = tp.compute_target(&ctx);
        // 4 * 50 / 100 = 2 -> index 2 = Mid (400)
        assert_eq!(score, 400);
        assert!(name.contains("IR RANK TOP 50%"));
    }

    #[test]
    fn compute_ir_no_ranking_fallback() {
        let tp = TargetProperty::resolve("IR_NEXT_1");
        let ctx = simple_ctx(500, 600);
        let (score, name) = tp.compute_target(&ctx);
        // No ranking data -> MAX fallback
        assert_eq!(score, 1000);
        assert_eq!(name, "MAX");
    }
}

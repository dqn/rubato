use crate::MainController;
use beatoraja_core::score_data::ScoreData;

/// Score target
pub enum TargetProperty {
    Static(StaticTargetProperty),
    Rival(RivalTargetProperty),
    InternetRanking(InternetRankingTargetProperty),
    NextRank(NextRankTargetProperty),
}

impl TargetProperty {
    pub fn id(&self) -> &str {
        match self {
            TargetProperty::Static(p) => &p.id,
            TargetProperty::Rival(p) => &p.id,
            TargetProperty::InternetRanking(p) => &p.id,
            TargetProperty::NextRank(p) => &p.id,
        }
    }

    pub fn get_targets() -> Vec<String> {
        beatoraja_types::target_list::get_targets()
    }

    pub fn get_target_name(target: &str) -> String {
        beatoraja_types::target_list::get_target_name(target)
    }

    pub fn get_target_property(id: &str) -> Option<TargetProperty> {
        if let Some(target) = StaticTargetProperty::get_target_property(id) {
            return Some(target);
        }
        if let Some(target) = RivalTargetProperty::get_target_property(id) {
            return Some(target);
        }
        if let Some(target) = InternetRankingTargetProperty::get_target_property(id) {
            return Some(target);
        }
        if id == "RANK_NEXT" {
            return Some(TargetProperty::NextRank(NextRankTargetProperty::new()));
        }
        // fallback to MAX
        StaticTargetProperty::get_target_property("MAX")
    }

    /// Translated from: Java TargetProperty.getName(MainController)
    pub fn get_name(&self, main: &MainController) -> String {
        match self {
            TargetProperty::Static(p) => p.name.clone(),
            TargetProperty::Rival(p) => {
                let info = main
                    .get_rival_data_accessor()
                    .get_rival_information(p.index as usize);
                match p.target {
                    RivalTarget::Index => match info {
                        Some(info) => format!("RIVAL {}", info.get_name()),
                        None => "NO RIVAL".to_string(),
                    },
                    RivalTarget::Rank => {
                        if p.index > 0 {
                            format!("RIVAL RANK {}", p.index + 1)
                        } else {
                            "RIVAL TOP".to_string()
                        }
                    }
                    RivalTarget::Next => {
                        format!("RIVAL NEXT {}", p.index + 1)
                    }
                }
            }
            TargetProperty::InternetRanking(p) => match p.target {
                IRTarget::Next => format!("IR NEXT {}RANK", p.value),
                IRTarget::Rank => format!("IR RANK {}", p.value),
                IRTarget::RankRate => format!("IR RANK TOP {}%", p.value),
            },
            TargetProperty::NextRank(_) => "NEXT RANK".to_string(),
        }
    }

    /// Translated from: Java TargetProperty.getTarget(MainController)
    pub fn get_target(&mut self, main: &mut MainController) -> ScoreData {
        match self {
            TargetProperty::Static(p) => p.get_target(main),
            TargetProperty::Rival(p) => p.get_target(main),
            TargetProperty::InternetRanking(p) => p.get_target(main),
            TargetProperty::NextRank(p) => p.get_target(main),
        }
    }
}

/// Static target (fixed rate)
pub struct StaticTargetProperty {
    pub id: String,
    pub name: String,
    pub rate: f32,
    pub target_score: ScoreData,
}

impl StaticTargetProperty {
    pub fn new(id: &str, name: &str, rate: f32) -> Self {
        StaticTargetProperty {
            id: id.to_string(),
            name: name.to_string(),
            rate,
            target_score: ScoreData::default(),
        }
    }

    /// Translated from: Java StaticTargetProperty.getTarget(MainController)
    fn get_target(&mut self, main: &MainController) -> ScoreData {
        let total_notes = main
            .get_player_resource()
            .and_then(|r| r.get_bms_model())
            .map(|m| m.get_total_notes())
            .unwrap_or(0);
        let rivalscore = (total_notes as f64 * 2.0 * self.rate as f64 / 100.0).ceil() as i32;
        self.target_score.player = self.name.clone();
        self.target_score.epg = rivalscore / 2;
        self.target_score.egr = rivalscore % 2;
        self.target_score.clone()
    }

    pub fn get_target_property(id: &str) -> Option<TargetProperty> {
        match id {
            "RATE_A-" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_A-",
                "RANK A-",
                100.0 * 17.0 / 27.0,
            ))),
            "RATE_A" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_A",
                "RANK A",
                100.0 * 18.0 / 27.0,
            ))),
            "RATE_A+" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_A+",
                "RANK A+",
                100.0 * 19.0 / 27.0,
            ))),
            "RATE_AA-" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_AA-",
                "RANK AA-",
                100.0 * 20.0 / 27.0,
            ))),
            "RATE_AA" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_AA",
                "RANK AA",
                100.0 * 21.0 / 27.0,
            ))),
            "RATE_AA+" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_AA+",
                "RANK AA+",
                100.0 * 22.0 / 27.0,
            ))),
            "RATE_AAA-" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_AAA-",
                "RANK AAA-",
                100.0 * 23.0 / 27.0,
            ))),
            "RATE_AAA" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_AAA",
                "RANK AAA",
                100.0 * 24.0 / 27.0,
            ))),
            "RATE_AAA+" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_AAA+",
                "RANK AAA+",
                100.0 * 25.0 / 27.0,
            ))),
            "RATE_MAX-" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "RATE_MAX-",
                "RANK MAX-",
                100.0 * 26.0 / 27.0,
            ))),
            "MAX" => Some(TargetProperty::Static(StaticTargetProperty::new(
                "MAX", "MAX", 100.0,
            ))),
            _ => {
                if id.starts_with("RATE_")
                    && let Ok(index) = id[5..].parse::<f32>()
                    && (0.0..=100.0).contains(&index)
                {
                    return Some(TargetProperty::Static(StaticTargetProperty::new(
                        &format!("RATE_{}", index),
                        &format!("SCORE RATE {}%", index),
                        index,
                    )));
                }
                None
            }
        }
    }
}

/// Rival target
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RivalTarget {
    Index,
    Next,
    Rank,
}

pub struct RivalTargetProperty {
    pub id: String,
    pub target: RivalTarget,
    pub index: i32,
    pub target_score: ScoreData,
}

impl RivalTargetProperty {
    pub fn new(target: RivalTarget, index: i32) -> Self {
        RivalTargetProperty {
            id: format!("RIVAL_{}", index + 1),
            target,
            index,
            target_score: ScoreData::default(),
        }
    }

    /// Translated from: Java RivalTargetProperty.getTarget(MainController)
    fn get_target(&mut self, main: &mut MainController) -> ScoreData {
        // Extract read-only values before mutable borrows
        let songdata = main
            .get_player_resource()
            .and_then(|r| r.get_songdata())
            .cloned();
        let songdata = match songdata {
            Some(sd) => sd,
            None => {
                self.target_score.player = "NO RIVAL".to_string();
                self.target_score.option = 0;
                return self.target_score.clone();
            }
        };
        let lnmode = main.get_player_config().get_lnmode();
        let index = self.index as usize;

        let mut name: Option<String> = None;
        let mut score: Option<ScoreData> = None;

        match self.target {
            RivalTarget::Index => {
                name = main
                    .get_rival_data_accessor()
                    .get_rival_information(index)
                    .map(|info| info.get_name().to_string());
                score = main
                    .get_rival_data_accessor_mut()
                    .get_rival_score_data_cache_mut(index)
                    .and_then(|cache| cache.read_score_data(&songdata, lnmode).cloned());
            }
            RivalTarget::Rank => {
                let mut scores = Self::create_score_array_impl(main, &songdata, lnmode);
                if !scores.is_empty() {
                    scores.sort_by_key(|b| std::cmp::Reverse(b.get_exscore()));
                    let pick = if index < scores.len() {
                        index
                    } else {
                        scores.len() - 1
                    };
                    let s = &scores[pick];
                    name = Some(s.player.clone());
                    score = Some(s.clone());
                }
            }
            RivalTarget::Next => {
                let mut scores = Self::create_score_array_impl(main, &songdata, lnmode);
                if !scores.is_empty() {
                    scores.sort_by_key(|b| std::cmp::Reverse(b.get_exscore()));
                    // Find own score position (empty player name)
                    let mut rank = scores.len().saturating_sub(1).saturating_sub(index);
                    for (i, s) in scores.iter().enumerate() {
                        if s.player.is_empty() {
                            rank = i.saturating_sub(index);
                            break;
                        }
                    }
                    let rank = rank.min(scores.len() - 1);
                    let s = &scores[rank];
                    name = Some(s.player.clone());
                    score = Some(s.clone());
                }
            }
        }

        if let Some(s) = score {
            self.target_score.player = name.unwrap_or_default();
            self.target_score.epg = s.epg;
            self.target_score.lpg = s.lpg;
            self.target_score.egr = s.egr;
            self.target_score.lgr = s.lgr;
            self.target_score.option = s.option;
        } else if name.is_some() {
            self.target_score.player = "NO DATA".to_string();
            self.target_score.option = 0;
        } else {
            self.target_score.player = "NO RIVAL".to_string();
            self.target_score.option = 0;
        }

        self.target_score.clone()
    }

    /// Create score array from rivals + own score.
    /// Translated from: Java RivalTargetProperty.createScoreArray(MainController)
    fn create_score_array_impl(
        main: &mut MainController,
        songdata: &beatoraja_types::song_data::SongData,
        lnmode: i32,
    ) -> Vec<ScoreData> {
        let rival_count = main.get_rival_data_accessor().get_rival_count();

        // Collect rival names first (immutable borrow)
        let rival_names: Vec<Option<String>> = (0..rival_count)
            .map(|i| {
                main.get_rival_data_accessor()
                    .get_rival_information(i)
                    .map(|info| info.get_name().to_string())
            })
            .collect();

        // Read rival scores (mutable borrow for cache)
        let mut scorearray = Vec::new();
        for i in 0..rival_count {
            let score = main
                .get_rival_data_accessor_mut()
                .get_rival_score_data_cache_mut(i)
                .and_then(|cache| cache.read_score_data(songdata, lnmode).cloned());

            if let Some(mut sd) = score {
                if let Some(ref name) = rival_names[i] {
                    sd.player = name.clone();
                }
                scorearray.push(sd);
            }
        }

        // Add own score with empty player name
        let own_score = main
            .get_player_resource()
            .and_then(|r| r.get_bms_model())
            .and_then(|model| {
                main.get_play_data_accessor()
                    .and_then(|pda| pda.read_score_data_model(model, lnmode))
            });

        if let Some(mut myscore) = own_score {
            myscore.player = String::new();
            scorearray.push(myscore);
        }

        scorearray
    }

    pub fn get_target_property(id: &str) -> Option<TargetProperty> {
        if let Some(suffix) = id.strip_prefix("RIVAL_NEXT_") {
            if let Ok(index) = suffix.parse::<i32>()
                && index > 0
            {
                return Some(TargetProperty::Rival(RivalTargetProperty::new(
                    RivalTarget::Next,
                    index - 1,
                )));
            }
        } else if let Some(suffix) = id.strip_prefix("RIVAL_RANK_") {
            if let Ok(index) = suffix.parse::<i32>()
                && index > 0
            {
                return Some(TargetProperty::Rival(RivalTargetProperty::new(
                    RivalTarget::Rank,
                    index - 1,
                )));
            }
        } else if let Some(suffix) = id.strip_prefix("RIVAL_")
            && let Ok(index) = suffix.parse::<i32>()
            && index > 0
        {
            return Some(TargetProperty::Rival(RivalTargetProperty::new(
                RivalTarget::Index,
                index - 1,
            )));
        }
        None
    }
}

/// Internet ranking target
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IRTarget {
    Next,
    Rank,
    RankRate,
}

pub struct InternetRankingTargetProperty {
    pub id: String,
    pub target: IRTarget,
    pub value: i32,
    pub target_score: ScoreData,
}

impl InternetRankingTargetProperty {
    pub fn new(target: IRTarget, value: i32) -> Self {
        InternetRankingTargetProperty {
            id: format!("IR_{:?}_{}", target, value),
            target,
            value,
            target_score: ScoreData::default(),
        }
    }

    /// Translated from: Java InternetRankingTargetProperty.getTarget(MainController)
    ///
    /// Simplified: ranking data retrieval requires async IR access which is not
    /// yet fully wired. Returns "NO DATA" until RankingData integration is complete.
    fn get_target(&mut self, _main: &MainController) -> ScoreData {
        self.target_score.player = "NO DATA".to_string();
        self.target_score.option = 0;
        self.target_score.clone()
    }

    pub fn get_target_property(id: &str) -> Option<TargetProperty> {
        if id.starts_with("IR_NEXT_")
            && let Ok(index) = id[8..].parse::<i32>()
            && index > 0
        {
            return Some(TargetProperty::InternetRanking(
                InternetRankingTargetProperty::new(IRTarget::Next, index),
            ));
        }
        if id.starts_with("IR_RANK_")
            && let Ok(index) = id[8..].parse::<i32>()
            && index > 0
        {
            return Some(TargetProperty::InternetRanking(
                InternetRankingTargetProperty::new(IRTarget::Rank, index),
            ));
        }
        if id.starts_with("IR_RANKRATE_")
            && let Ok(index) = id[12..].parse::<i32>()
            && index > 0
            && index < 100
        {
            return Some(TargetProperty::InternetRanking(
                InternetRankingTargetProperty::new(IRTarget::RankRate, index),
            ));
        }
        None
    }
}

/// Next rank target
pub struct NextRankTargetProperty {
    pub id: String,
    pub target_score: ScoreData,
}

impl Default for NextRankTargetProperty {
    fn default() -> Self {
        Self::new()
    }
}

impl NextRankTargetProperty {
    pub fn new() -> Self {
        NextRankTargetProperty {
            id: "RANK_NEXT".to_string(),
            target_score: ScoreData::default(),
        }
    }

    /// Translated from: Java NextRankTargetProperty.getTarget(MainController)
    fn get_target(&mut self, main: &MainController) -> ScoreData {
        let lnmode = main.get_player_config().get_lnmode();
        let model = main.get_player_resource().and_then(|r| r.get_bms_model());

        let nowscore = model
            .and_then(|m| {
                main.get_play_data_accessor()
                    .and_then(|pda| pda.read_score_data_model(m, lnmode))
            })
            .map(|s| s.get_exscore())
            .unwrap_or(0);

        let max = model.map(|m| m.get_total_notes() * 2).unwrap_or(0);

        // Find next rank threshold: iterate from 15/27 to 26/27
        let mut targetscore = max;
        for i in 15..27 {
            let target = (max as f64 * i as f64 / 27.0).ceil() as i32;
            if nowscore < target {
                targetscore = target;
                break;
            }
        }

        self.target_score.player = "NEXT RANK".to_string();
        self.target_score.epg = targetscore / 2;
        self.target_score.egr = targetscore % 2;
        self.target_score.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_core::config::Config;
    use beatoraja_types::player_config::PlayerConfig;

    fn make_main() -> MainController {
        MainController::new(
            None,
            Config::default(),
            PlayerConfig::default(),
            None,
            false,
        )
    }

    #[test]
    fn test_static_target_property_lookup() {
        let target = StaticTargetProperty::get_target_property("MAX");
        assert!(target.is_some());
        let target = target.unwrap();
        assert_eq!(target.id(), "MAX");
    }

    #[test]
    fn test_static_target_property_rate() {
        let target = StaticTargetProperty::get_target_property("RATE_50").unwrap();
        if let TargetProperty::Static(p) = target {
            assert_eq!(p.rate, 50.0);
            assert_eq!(p.name, "SCORE RATE 50%");
        } else {
            panic!("Expected Static variant");
        }
    }

    #[test]
    fn test_rival_target_property_index() {
        let target = RivalTargetProperty::get_target_property("RIVAL_1").unwrap();
        if let TargetProperty::Rival(p) = target {
            assert_eq!(p.target, RivalTarget::Index);
            assert_eq!(p.index, 0);
        } else {
            panic!("Expected Rival variant");
        }
    }

    #[test]
    fn test_rival_target_property_rank() {
        let target = RivalTargetProperty::get_target_property("RIVAL_RANK_3").unwrap();
        if let TargetProperty::Rival(p) = target {
            assert_eq!(p.target, RivalTarget::Rank);
            assert_eq!(p.index, 2);
        } else {
            panic!("Expected Rival variant");
        }
    }

    #[test]
    fn test_rival_target_property_next() {
        let target = RivalTargetProperty::get_target_property("RIVAL_NEXT_2").unwrap();
        if let TargetProperty::Rival(p) = target {
            assert_eq!(p.target, RivalTarget::Next);
            assert_eq!(p.index, 1);
        } else {
            panic!("Expected Rival variant");
        }
    }

    #[test]
    fn test_next_rank_target_property() {
        let target = TargetProperty::get_target_property("RANK_NEXT").unwrap();
        assert_eq!(target.id(), "RANK_NEXT");
    }

    #[test]
    fn test_ir_target_property_next() {
        let target = InternetRankingTargetProperty::get_target_property("IR_NEXT_5").unwrap();
        if let TargetProperty::InternetRanking(p) = target {
            assert_eq!(p.target, IRTarget::Next);
            assert_eq!(p.value, 5);
        } else {
            panic!("Expected InternetRanking variant");
        }
    }

    #[test]
    fn test_ir_target_property_rank() {
        let target = InternetRankingTargetProperty::get_target_property("IR_RANK_10").unwrap();
        if let TargetProperty::InternetRanking(p) = target {
            assert_eq!(p.target, IRTarget::Rank);
            assert_eq!(p.value, 10);
        } else {
            panic!("Expected InternetRanking variant");
        }
    }

    #[test]
    fn test_ir_target_property_rankrate() {
        let target = InternetRankingTargetProperty::get_target_property("IR_RANKRATE_50").unwrap();
        if let TargetProperty::InternetRanking(p) = target {
            assert_eq!(p.target, IRTarget::RankRate);
            assert_eq!(p.value, 50);
        } else {
            panic!("Expected InternetRanking variant");
        }
    }

    #[test]
    fn test_fallback_to_max() {
        let target = TargetProperty::get_target_property("UNKNOWN").unwrap();
        assert_eq!(target.id(), "MAX");
    }

    #[test]
    fn test_get_name_static() {
        let target = StaticTargetProperty::get_target_property("MAX").unwrap();
        let main = make_main();
        assert_eq!(target.get_name(&main), "MAX");
    }

    #[test]
    fn test_get_name_rival_no_rival() {
        let target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Index, 0));
        let main = make_main();
        // No rivals loaded → "NO RIVAL"
        assert_eq!(target.get_name(&main), "NO RIVAL");
    }

    #[test]
    fn test_get_name_rival_rank() {
        let target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Rank, 0));
        let main = make_main();
        assert_eq!(target.get_name(&main), "RIVAL TOP");
    }

    #[test]
    fn test_get_name_rival_rank_nonzero() {
        let target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Rank, 2));
        let main = make_main();
        assert_eq!(target.get_name(&main), "RIVAL RANK 3");
    }

    #[test]
    fn test_get_name_rival_next() {
        let target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Next, 1));
        let main = make_main();
        assert_eq!(target.get_name(&main), "RIVAL NEXT 2");
    }

    #[test]
    fn test_get_name_ir() {
        let target =
            TargetProperty::InternetRanking(InternetRankingTargetProperty::new(IRTarget::Next, 3));
        let main = make_main();
        assert_eq!(target.get_name(&main), "IR NEXT 3RANK");
    }

    #[test]
    fn test_get_name_next_rank() {
        let target = TargetProperty::NextRank(NextRankTargetProperty::new());
        let main = make_main();
        assert_eq!(target.get_name(&main), "NEXT RANK");
    }

    #[test]
    fn test_static_get_target_no_model() {
        let mut target = TargetProperty::Static(StaticTargetProperty::new("MAX", "MAX", 100.0));
        let mut main = make_main();
        let score = target.get_target(&mut main);
        // No PlayerResource → total_notes=0 → rivalscore=0
        assert_eq!(score.epg, 0);
        assert_eq!(score.egr, 0);
        assert_eq!(score.player, "MAX");
    }

    #[test]
    fn test_ir_get_target_returns_no_data() {
        let mut target =
            TargetProperty::InternetRanking(InternetRankingTargetProperty::new(IRTarget::Next, 1));
        let mut main = make_main();
        let score = target.get_target(&mut main);
        assert_eq!(score.player, "NO DATA");
    }

    #[test]
    fn test_rival_get_target_no_resource() {
        let mut target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Index, 0));
        let mut main = make_main();
        let score = target.get_target(&mut main);
        // No PlayerResource → no songdata → "NO RIVAL"
        assert_eq!(score.player, "NO RIVAL");
    }

    #[test]
    fn test_next_rank_get_target_no_model() {
        let mut target = TargetProperty::NextRank(NextRankTargetProperty::new());
        let mut main = make_main();
        let score = target.get_target(&mut main);
        assert_eq!(score.player, "NEXT RANK");
        // No model → max=0, nowscore=0, targetscore=0
        assert_eq!(score.epg, 0);
        assert_eq!(score.egr, 0);
    }

    #[test]
    fn test_static_get_target_score_calculation() {
        // Test the score calculation directly on StaticTargetProperty
        let mut p = StaticTargetProperty::new("AAA", "RANK AAA", 100.0 * 24.0 / 27.0);
        let main = make_main();
        // No model so total_notes=0, but verify formula doesn't panic
        let score = p.get_target(&main);
        assert_eq!(score.player, "RANK AAA");
    }

    #[test]
    fn test_ir_target_returns_no_data_for_all_types() {
        let mut main = make_main();
        for ir_type in [IRTarget::Next, IRTarget::Rank, IRTarget::RankRate] {
            let mut target =
                TargetProperty::InternetRanking(InternetRankingTargetProperty::new(ir_type, 1));
            let score = target.get_target(&mut main);
            assert_eq!(score.player, "NO DATA");
        }
    }
}

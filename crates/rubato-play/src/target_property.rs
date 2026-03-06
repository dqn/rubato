use crate::MainController;
use rubato_core::score_data::ScoreData;

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

    pub fn targets() -> Vec<String> {
        rubato_types::target_list::targets()
    }

    pub fn target_name(target: &str) -> String {
        rubato_types::target_list::target_name(target)
    }

    pub fn from_id(id: &str) -> Option<TargetProperty> {
        if let Some(target) = StaticTargetProperty::from_id(id) {
            return Some(target);
        }
        if let Some(target) = RivalTargetProperty::from_id(id) {
            return Some(target);
        }
        if let Some(target) = InternetRankingTargetProperty::from_id(id) {
            return Some(target);
        }
        if id == "RANK_NEXT" {
            return Some(TargetProperty::NextRank(NextRankTargetProperty::new()));
        }
        // fallback to MAX
        StaticTargetProperty::from_id("MAX")
    }

    /// Translated from: Java TargetProperty.getName(MainController)
    pub fn name(&self, main: &MainController) -> String {
        match self {
            TargetProperty::Static(p) => p.name.clone(),
            TargetProperty::Rival(p) => {
                let info = main
                    .rival_data_accessor()
                    .rival_information(p.index as usize);
                match p.target {
                    RivalTarget::Index => match info {
                        Some(info) => format!("RIVAL {}", info.name()),
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
    pub fn target(&mut self, main: &mut MainController) -> ScoreData {
        match self {
            TargetProperty::Static(p) => p.target(main),
            TargetProperty::Rival(p) => p.target(main),
            TargetProperty::InternetRanking(p) => p.target(main),
            TargetProperty::NextRank(p) => p.target(main),
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
    fn target(&mut self, main: &MainController) -> ScoreData {
        let total_notes = main
            .player_resource()
            .and_then(|r| r.bms_model())
            .map(|m| m.total_notes())
            .unwrap_or(0);
        let rivalscore = (total_notes as f64 * 2.0 * self.rate as f64 / 100.0).ceil() as i32;
        self.target_score.player = self.name.clone();
        self.target_score.epg = rivalscore / 2;
        self.target_score.egr = rivalscore % 2;
        self.target_score.clone()
    }

    pub fn from_id(id: &str) -> Option<TargetProperty> {
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
    fn target(&mut self, main: &mut MainController) -> ScoreData {
        // Extract read-only values before mutable borrows
        let songdata = main.player_resource().and_then(|r| r.songdata()).cloned();
        let songdata = match songdata {
            Some(sd) => sd,
            None => {
                self.target_score.player = "NO RIVAL".to_string();
                self.target_score.option = 0;
                return self.target_score.clone();
            }
        };
        let lnmode = main.player_config().lnmode;
        let index = self.index as usize;

        let mut name: Option<String> = None;
        let mut score: Option<ScoreData> = None;

        match self.target {
            RivalTarget::Index => {
                name = main
                    .rival_data_accessor()
                    .rival_information(index)
                    .map(|info| info.name().to_string());
                score = main
                    .rival_data_accessor_mut()
                    .rival_score_data_cache_mut(index)
                    .and_then(|cache| cache.read_score_data(&songdata, lnmode).cloned());
            }
            RivalTarget::Rank => {
                let mut scores = Self::create_score_array_impl(main, &songdata, lnmode);
                if !scores.is_empty() {
                    scores.sort_by_key(|b| std::cmp::Reverse(b.exscore()));
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
                    scores.sort_by_key(|b| std::cmp::Reverse(b.exscore()));
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
        songdata: &rubato_types::song_data::SongData,
        lnmode: i32,
    ) -> Vec<ScoreData> {
        let rival_count = main.rival_data_accessor().rival_count();

        // Collect rival names first (immutable borrow)
        let rival_names: Vec<Option<String>> = (0..rival_count)
            .map(|i| {
                main.rival_data_accessor()
                    .rival_information(i)
                    .map(|info| info.name().to_string())
            })
            .collect();

        // Read rival scores (mutable borrow for cache)
        let mut scorearray = Vec::new();
        #[allow(clippy::needless_range_loop)]
        for i in 0..rival_count {
            let score = main
                .rival_data_accessor_mut()
                .rival_score_data_cache_mut(i)
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
            .player_resource()
            .and_then(|r| r.bms_model())
            .and_then(|model| {
                main.play_data_accessor()
                    .and_then(|pda| pda.read_score_data_model(model, lnmode))
            });

        if let Some(mut myscore) = own_score {
            myscore.player = String::new();
            scorearray.push(myscore);
        }

        scorearray
    }

    pub fn from_id(id: &str) -> Option<TargetProperty> {
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
    /// Receiver for async IR load results.
    /// When a background thread finishes loading, it sends the result here.
    ir_result_rx: Option<std::sync::mpsc::Receiver<ScoreData>>,
    /// Whether async IR loading has been initiated for the current song.
    loading_initiated: bool,
}

impl InternetRankingTargetProperty {
    pub fn new(target: IRTarget, value: i32) -> Self {
        InternetRankingTargetProperty {
            id: format!("IR_{:?}_{}", target, value),
            target,
            value,
            target_score: ScoreData::default(),
            ir_result_rx: None,
            loading_initiated: false,
        }
    }

    /// Initiate async IR data loading on a background thread.
    ///
    /// The background thread calls `connection.get_play_data()`, processes the ranking
    /// data, and sends the resulting target score via channel. `get_target()` polls
    /// the channel on each call and updates `target_score` when the result arrives.
    ///
    /// This mirrors the Java pattern where `InternetRankingTargetProperty.getTarget()`
    /// spawns a background thread to load IR data.
    pub fn initiate_load(
        &mut self,
        connection: std::sync::Arc<dyn rubato_ir::ir_connection::IRConnection + Send + Sync>,
        chart: rubato_ir::ir_chart_data::IRChartData,
        local_score: Option<ScoreData>,
        target: IRTarget,
        value: i32,
    ) {
        if self.loading_initiated {
            return;
        }
        self.loading_initiated = true;

        let (tx, rx) = std::sync::mpsc::channel();
        self.ir_result_rx = Some(rx);

        std::thread::spawn(move || {
            let mut ranking = rubato_ir::ranking_data::RankingData::new();
            ranking.load_song(&*connection, &chart, local_score.as_ref());

            if ranking.state() == rubato_ir::ranking_data::FINISH {
                let mut score = ScoreData::default();
                if ranking.total_player() > 0 {
                    let total = ranking.total_player();
                    let target_index = match target {
                        IRTarget::Next => {
                            // In the async path, nowscore is 0 (game just started)
                            (total - value).max(0)
                        }
                        IRTarget::Rank => (value.min(total) - 1).max(0),
                        IRTarget::RankRate => total * value / 100,
                    };
                    if let Some(ir_score) = ranking.score(target_index) {
                        let exscore = ir_score.exscore();
                        score.player = if ir_score.player.is_empty() {
                            "YOU".to_string()
                        } else {
                            ir_score.player.clone()
                        };
                        score.epg = exscore / 2;
                        score.egr = exscore % 2;
                        score.option = ir_score.option;
                    } else {
                        score.player = "NO DATA".to_string();
                    }
                } else {
                    score.player = "NO DATA".to_string();
                }
                let _ = tx.send(score);
            }
        });
    }

    /// Reset loading state (e.g., when switching to a new song).
    pub fn reset_loading(&mut self) {
        self.ir_result_rx = None;
        self.loading_initiated = false;
    }

    /// Translated from: Java InternetRankingTargetProperty.getTarget(MainController)
    fn target(&mut self, main: &MainController) -> ScoreData {
        // Poll for async IR load result
        if let Some(rx) = self.ir_result_rx.take() {
            match rx.try_recv() {
                Ok(result) => {
                    self.target_score = result;
                    // Don't put receiver back - loading complete
                    return self.target_score.clone();
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still loading, put receiver back
                    self.ir_result_rx = Some(rx);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Thread finished without sending (load failed)
                }
            }
        }

        // If async load already produced a result, return it
        if self.loading_initiated
            && self.ir_result_rx.is_none()
            && self.target_score.player != "NO DATA"
        {
            return self.target_score.clone();
        }

        // Get ranking data from cache via dyn Any downcast
        let ranking_data = (|| -> Option<rubato_ir::ranking_data::RankingData> {
            let resource = main.player_resource()?;
            let songdata = resource.songdata()?;
            let lnmode = resource.player_config().lnmode;
            let cache = main.ranking_data_cache()?;
            let any = cache.song_any(songdata, lnmode)?;
            any.downcast::<rubato_ir::ranking_data::RankingData>()
                .ok()
                .map(|ranking| *ranking)
        })();

        match ranking_data {
            Some(ref ranking) if ranking.state() == rubato_ir::ranking_data::FINISH => {
                if ranking.total_player() > 0 {
                    let index = self.target_rank(main, ranking);
                    if let Some(ir_score) = ranking.score(index) {
                        let exscore = ir_score.exscore();
                        self.target_score.player = if ir_score.player.is_empty() {
                            "YOU".to_string()
                        } else {
                            ir_score.player.clone()
                        };
                        self.target_score.epg = exscore / 2;
                        self.target_score.egr = exscore % 2;
                        self.target_score.option = ir_score.option;
                    } else {
                        self.target_score.player = "NO DATA".to_string();
                        self.target_score.option = 0;
                    }
                } else {
                    self.target_score.player = "NO DATA".to_string();
                    self.target_score.option = 0;
                }
            }
            _ => {
                // Not yet loaded or no ranking data available
                self.target_score.player = "NO DATA".to_string();
                self.target_score.option = 0;
            }
        }
        self.target_score.clone()
    }

    /// Get the target rank index based on the IR target type.
    fn target_rank(
        &self,
        main: &MainController,
        ranking: &rubato_ir::ranking_data::RankingData,
    ) -> i32 {
        let total = ranking.total_player();
        // Get the player's current exscore
        let nowscore = main
            .player_resource()
            .and_then(|r| r.score_data())
            .map(|s| s.exscore())
            .unwrap_or(0);

        match self.target {
            IRTarget::Next => {
                // Find the rank of the first score <= nowscore, then go 'value' ranks above
                let mut target_index = 0;
                for i in 0..total {
                    if let Some(score) = ranking.score(i)
                        && score.exscore() <= nowscore
                    {
                        target_index = (i - self.value).max(0);
                        break;
                    }
                }
                target_index
            }
            IRTarget::Rank => {
                // value-th place (1-indexed, capped to totalPlayer)
                (self.value.min(total) - 1).max(0)
            }
            IRTarget::RankRate => {
                // top value% rank index (matches Java: totalPlayer * value / 100)
                total * self.value / 100
            }
        }
    }

    pub fn from_id(id: &str) -> Option<TargetProperty> {
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
    fn target(&mut self, main: &MainController) -> ScoreData {
        let lnmode = main.player_config().lnmode;
        let model = main.player_resource().and_then(|r| r.bms_model());

        let nowscore = model
            .and_then(|m| {
                main.play_data_accessor()
                    .and_then(|pda| pda.read_score_data_model(m, lnmode))
            })
            .map(|s| s.exscore())
            .unwrap_or(0);

        let max = model.map(|m| m.total_notes() * 2).unwrap_or(0);

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
    use rubato_core::config::Config;
    use rubato_types::player_config::PlayerConfig;

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
        let target = StaticTargetProperty::from_id("MAX");
        assert!(target.is_some());
        let target = target.unwrap();
        assert_eq!(target.id(), "MAX");
    }

    #[test]
    fn test_static_target_property_rate() {
        let target = StaticTargetProperty::from_id("RATE_50").unwrap();
        if let TargetProperty::Static(p) = target {
            assert_eq!(p.rate, 50.0);
            assert_eq!(p.name, "SCORE RATE 50%");
        } else {
            panic!("Expected Static variant");
        }
    }

    #[test]
    fn test_rival_target_property_index() {
        let target = RivalTargetProperty::from_id("RIVAL_1").unwrap();
        if let TargetProperty::Rival(p) = target {
            assert_eq!(p.target, RivalTarget::Index);
            assert_eq!(p.index, 0);
        } else {
            panic!("Expected Rival variant");
        }
    }

    #[test]
    fn test_rival_target_property_rank() {
        let target = RivalTargetProperty::from_id("RIVAL_RANK_3").unwrap();
        if let TargetProperty::Rival(p) = target {
            assert_eq!(p.target, RivalTarget::Rank);
            assert_eq!(p.index, 2);
        } else {
            panic!("Expected Rival variant");
        }
    }

    #[test]
    fn test_rival_target_property_next() {
        let target = RivalTargetProperty::from_id("RIVAL_NEXT_2").unwrap();
        if let TargetProperty::Rival(p) = target {
            assert_eq!(p.target, RivalTarget::Next);
            assert_eq!(p.index, 1);
        } else {
            panic!("Expected Rival variant");
        }
    }

    #[test]
    fn test_next_rank_target_property() {
        let target = TargetProperty::from_id("RANK_NEXT").unwrap();
        assert_eq!(target.id(), "RANK_NEXT");
    }

    #[test]
    fn test_ir_target_property_next() {
        let target = InternetRankingTargetProperty::from_id("IR_NEXT_5").unwrap();
        if let TargetProperty::InternetRanking(p) = target {
            assert_eq!(p.target, IRTarget::Next);
            assert_eq!(p.value, 5);
        } else {
            panic!("Expected InternetRanking variant");
        }
    }

    #[test]
    fn test_ir_target_property_rank() {
        let target = InternetRankingTargetProperty::from_id("IR_RANK_10").unwrap();
        if let TargetProperty::InternetRanking(p) = target {
            assert_eq!(p.target, IRTarget::Rank);
            assert_eq!(p.value, 10);
        } else {
            panic!("Expected InternetRanking variant");
        }
    }

    #[test]
    fn test_ir_target_property_rankrate() {
        let target = InternetRankingTargetProperty::from_id("IR_RANKRATE_50").unwrap();
        if let TargetProperty::InternetRanking(p) = target {
            assert_eq!(p.target, IRTarget::RankRate);
            assert_eq!(p.value, 50);
        } else {
            panic!("Expected InternetRanking variant");
        }
    }

    #[test]
    fn test_fallback_to_max() {
        let target = TargetProperty::from_id("UNKNOWN").unwrap();
        assert_eq!(target.id(), "MAX");
    }

    #[test]
    fn test_get_name_static() {
        let target = StaticTargetProperty::from_id("MAX").unwrap();
        let main = make_main();
        assert_eq!(target.name(&main), "MAX");
    }

    #[test]
    fn test_get_name_rival_no_rival() {
        let target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Index, 0));
        let main = make_main();
        // No rivals loaded → "NO RIVAL"
        assert_eq!(target.name(&main), "NO RIVAL");
    }

    #[test]
    fn test_get_name_rival_rank() {
        let target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Rank, 0));
        let main = make_main();
        assert_eq!(target.name(&main), "RIVAL TOP");
    }

    #[test]
    fn test_get_name_rival_rank_nonzero() {
        let target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Rank, 2));
        let main = make_main();
        assert_eq!(target.name(&main), "RIVAL RANK 3");
    }

    #[test]
    fn test_get_name_rival_next() {
        let target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Next, 1));
        let main = make_main();
        assert_eq!(target.name(&main), "RIVAL NEXT 2");
    }

    #[test]
    fn test_get_name_ir() {
        let target =
            TargetProperty::InternetRanking(InternetRankingTargetProperty::new(IRTarget::Next, 3));
        let main = make_main();
        assert_eq!(target.name(&main), "IR NEXT 3RANK");
    }

    #[test]
    fn test_get_name_next_rank() {
        let target = TargetProperty::NextRank(NextRankTargetProperty::new());
        let main = make_main();
        assert_eq!(target.name(&main), "NEXT RANK");
    }

    #[test]
    fn test_static_get_target_no_model() {
        let mut target = TargetProperty::Static(StaticTargetProperty::new("MAX", "MAX", 100.0));
        let mut main = make_main();
        let score = target.target(&mut main);
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
        let score = target.target(&mut main);
        assert_eq!(score.player, "NO DATA");
    }

    #[test]
    fn test_rival_get_target_no_resource() {
        let mut target = TargetProperty::Rival(RivalTargetProperty::new(RivalTarget::Index, 0));
        let mut main = make_main();
        let score = target.target(&mut main);
        // No PlayerResource → no songdata → "NO RIVAL"
        assert_eq!(score.player, "NO RIVAL");
    }

    #[test]
    fn test_next_rank_get_target_no_model() {
        let mut target = TargetProperty::NextRank(NextRankTargetProperty::new());
        let mut main = make_main();
        let score = target.target(&mut main);
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
        let score = p.target(&main);
        assert_eq!(score.player, "RANK AAA");
    }

    #[test]
    fn test_ir_target_returns_no_data_for_all_types() {
        let mut main = make_main();
        for ir_type in [IRTarget::Next, IRTarget::Rank, IRTarget::RankRate] {
            let mut target =
                TargetProperty::InternetRanking(InternetRankingTargetProperty::new(ir_type, 1));
            let score = target.target(&mut main);
            assert_eq!(score.player, "NO DATA");
        }
    }

    // ============================================================
    // IR async loading tests
    // ============================================================

    /// Mock IR connection for async loading tests
    struct MockIRConnection {
        scores: Vec<rubato_ir::ir_score_data::IRScoreData>,
    }

    impl rubato_ir::ir_connection::IRConnection for MockIRConnection {
        fn get_rivals(
            &self,
        ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_player_data::IRPlayerData>>
        {
            rubato_ir::ir_response::IRResponse::success("OK".to_string(), vec![])
        }
        fn get_table_datas(
            &self,
        ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_table_data::IRTableData>>
        {
            rubato_ir::ir_response::IRResponse::success("OK".to_string(), vec![])
        }
        fn get_play_data(
            &self,
            _player: Option<&rubato_ir::ir_player_data::IRPlayerData>,
            _chart: &rubato_ir::ir_chart_data::IRChartData,
        ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_score_data::IRScoreData>>
        {
            rubato_ir::ir_response::IRResponse::success("OK".to_string(), self.scores.clone())
        }
        fn get_course_play_data(
            &self,
            _player: Option<&rubato_ir::ir_player_data::IRPlayerData>,
            _course: &rubato_ir::ir_course_data::IRCourseData,
        ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_score_data::IRScoreData>>
        {
            rubato_ir::ir_response::IRResponse::success("OK".to_string(), vec![])
        }
        fn send_play_data(
            &self,
            _model: &rubato_ir::ir_chart_data::IRChartData,
            _score: &rubato_ir::ir_score_data::IRScoreData,
        ) -> rubato_ir::ir_response::IRResponse<()> {
            rubato_ir::ir_response::IRResponse::success("OK".to_string(), ())
        }
        fn send_course_play_data(
            &self,
            _course: &rubato_ir::ir_course_data::IRCourseData,
            _score: &rubato_ir::ir_score_data::IRScoreData,
        ) -> rubato_ir::ir_response::IRResponse<()> {
            rubato_ir::ir_response::IRResponse::success("OK".to_string(), ())
        }
        fn get_song_url(&self, _chart: &rubato_ir::ir_chart_data::IRChartData) -> Option<String> {
            None
        }
        fn get_course_url(
            &self,
            _course: &rubato_ir::ir_course_data::IRCourseData,
        ) -> Option<String> {
            None
        }
        fn get_player_url(
            &self,
            _player: &rubato_ir::ir_player_data::IRPlayerData,
        ) -> Option<String> {
            None
        }
        fn name(&self) -> &str {
            "MockIR"
        }
    }

    #[test]
    fn test_ir_async_load_success() {
        use std::sync::Arc;

        let score_data = ScoreData {
            player: "TestPlayer".to_string(),
            epg: 500,
            egr: 200,
            option: 42,
            ..Default::default()
        };
        let ir_score = rubato_ir::ir_score_data::IRScoreData::new(&score_data);

        let conn: Arc<dyn rubato_ir::ir_connection::IRConnection + Send + Sync> =
            Arc::new(MockIRConnection {
                scores: vec![ir_score],
            });
        let chart = rubato_ir::ir_chart_data::IRChartData::default();

        let mut prop = InternetRankingTargetProperty::new(IRTarget::Rank, 1);
        prop.initiate_load(conn, chart, None, IRTarget::Rank, 1);

        // Wait for the background thread to finish
        let mut received = false;
        for _ in 0..100 {
            if let Some(ref rx) = prop.ir_result_rx {
                match rx.try_recv() {
                    Ok(result) => {
                        prop.target_score = result;
                        prop.ir_result_rx = None;
                        received = true;
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }

        assert!(received, "should have received async IR load result");
        assert_eq!(prop.target_score.player, "TestPlayer");
        assert_eq!(prop.target_score.option, 42);
    }

    #[test]
    fn test_ir_async_load_no_double_initiate() {
        use std::sync::Arc;

        let conn: Arc<dyn rubato_ir::ir_connection::IRConnection + Send + Sync> =
            Arc::new(MockIRConnection { scores: vec![] });
        let chart = rubato_ir::ir_chart_data::IRChartData::default();

        let mut prop = InternetRankingTargetProperty::new(IRTarget::Rank, 1);
        prop.initiate_load(conn.clone(), chart.clone(), None, IRTarget::Rank, 1);
        assert!(prop.loading_initiated);

        // Second call should be a no-op
        let rx_addr_before = prop.ir_result_rx.as_ref().map(|rx| rx as *const _);
        prop.initiate_load(conn, chart, None, IRTarget::Rank, 1);
        let rx_addr_after = prop.ir_result_rx.as_ref().map(|rx| rx as *const _);
        assert_eq!(
            rx_addr_before, rx_addr_after,
            "receiver should not change on double initiate"
        );
    }

    #[test]
    fn test_ir_async_load_reset() {
        let mut prop = InternetRankingTargetProperty::new(IRTarget::Next, 1);
        prop.loading_initiated = true;
        prop.reset_loading();
        assert!(!prop.loading_initiated, "loading_initiated should be reset");
        assert!(prop.ir_result_rx.is_none(), "receiver should be cleared");
    }

    #[test]
    fn test_ir_get_target_polls_channel() {
        use std::sync::mpsc;

        let mut prop = InternetRankingTargetProperty::new(IRTarget::Rank, 1);

        // Simulate a completed async load by injecting a pre-loaded channel
        let (tx, rx) = mpsc::channel();
        let score = ScoreData {
            player: "AsyncPlayer".to_string(),
            epg: 100,
            egr: 1,
            option: 7,
            ..Default::default()
        };
        tx.send(score).unwrap();
        prop.ir_result_rx = Some(rx);
        prop.loading_initiated = true;

        let main = make_main();
        let result = prop.target(&main);
        assert_eq!(result.player, "AsyncPlayer");
        assert_eq!(result.epg, 100);
        assert_eq!(result.egr, 1);
        assert_eq!(result.option, 7);
        assert!(
            prop.ir_result_rx.is_none(),
            "receiver should be consumed after receiving"
        );
    }
}

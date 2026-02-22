use crate::stubs::MainController;
use beatoraja_core::score_data::ScoreData;

use std::sync::Mutex;

static TARGETS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static TARGET_NAMES: Mutex<Vec<String>> = Mutex::new(Vec::new());

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
        TARGETS.lock().unwrap().clone()
    }

    pub fn get_target_name(target: &str) -> String {
        let targets = TARGETS.lock().unwrap();
        let names = TARGET_NAMES.lock().unwrap();
        for i in 0..targets.len() {
            if targets[i] == target {
                return names[i].clone();
            }
        }
        String::new()
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

    pub fn get_name(&self, _main: &MainController) -> String {
        match self {
            TargetProperty::Static(p) => p.name.clone(),
            TargetProperty::Rival(p) => {
                // TODO: Phase 7+ dependency - requires MainController.getRivalDataAccessor()
                // In Java, getName accesses RivalDataAccessor for player info
                match p.target {
                    RivalTarget::Index => {
                        // In Java: info != null ? "RIVAL " + info.getName() : "NO RIVAL"
                        format!("RIVAL {}", p.index + 1)
                    }
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

    pub fn get_target(&self, _main: &MainController) -> ScoreData {
        // Stub - actual implementation requires MainController to be fully implemented
        ScoreData::default()
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

    /// Create score array from rivals + own score.
    /// Corresponds to Java createScoreArray(MainController).
    fn create_score_array(&self, _main: &MainController) -> Vec<ScoreData> {
        // TODO: Phase 7+ dependency - requires MainController.getRivalDataAccessor(),
        // getPlayDataAccessor(), getSongdata(), getPlayerConfig()
        // In Java, this collects rival scores and own score into an array.
        Vec::new()
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
}

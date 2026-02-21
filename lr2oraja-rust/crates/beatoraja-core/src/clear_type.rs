use serde::{Deserialize, Serialize};

/// Clear type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClearType {
    NoPlay,
    Failed,
    AssistEasy,
    LightAssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    FullCombo,
    Perfect,
    Max,
}

impl ClearType {
    pub fn id(&self) -> i32 {
        match self {
            ClearType::NoPlay => 0,
            ClearType::Failed => 1,
            ClearType::AssistEasy => 2,
            ClearType::LightAssistEasy => 3,
            ClearType::Easy => 4,
            ClearType::Normal => 5,
            ClearType::Hard => 6,
            ClearType::ExHard => 7,
            ClearType::FullCombo => 8,
            ClearType::Perfect => 9,
            ClearType::Max => 10,
        }
    }

    pub fn gaugetype(&self) -> &'static [i32] {
        match self {
            ClearType::NoPlay => &[],
            ClearType::Failed => &[],
            ClearType::AssistEasy => &[],
            ClearType::LightAssistEasy => &[0],
            ClearType::Easy => &[1],
            ClearType::Normal => &[2, 6],
            ClearType::Hard => &[3, 7],
            ClearType::ExHard => &[4, 8],
            ClearType::FullCombo => &[5],
            ClearType::Perfect => &[],
            ClearType::Max => &[],
        }
    }

    pub fn values() -> &'static [ClearType] {
        &[
            ClearType::NoPlay,
            ClearType::Failed,
            ClearType::AssistEasy,
            ClearType::LightAssistEasy,
            ClearType::Easy,
            ClearType::Normal,
            ClearType::Hard,
            ClearType::ExHard,
            ClearType::FullCombo,
            ClearType::Perfect,
            ClearType::Max,
        ]
    }

    /// Get ClearType by ID. Returns NoPlay if not found.
    pub fn get_clear_type_by_id(id: i32) -> ClearType {
        for clear in ClearType::values() {
            if clear.id() == id {
                return *clear;
            }
        }
        ClearType::NoPlay
    }

    /// Get ClearType by gauge type. Returns None if not found.
    pub fn get_clear_type_by_gauge(gaugetype: i32) -> Option<ClearType> {
        for clear in ClearType::values() {
            for &t in clear.gaugetype() {
                if gaugetype == t {
                    return Some(*clear);
                }
            }
        }
        None
    }
}

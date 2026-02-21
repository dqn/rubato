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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_variants_exist() {
        let values = ClearType::values();
        assert_eq!(values.len(), 11);
    }

    #[test]
    fn test_id_mapping() {
        assert_eq!(ClearType::NoPlay.id(), 0);
        assert_eq!(ClearType::Failed.id(), 1);
        assert_eq!(ClearType::AssistEasy.id(), 2);
        assert_eq!(ClearType::LightAssistEasy.id(), 3);
        assert_eq!(ClearType::Easy.id(), 4);
        assert_eq!(ClearType::Normal.id(), 5);
        assert_eq!(ClearType::Hard.id(), 6);
        assert_eq!(ClearType::ExHard.id(), 7);
        assert_eq!(ClearType::FullCombo.id(), 8);
        assert_eq!(ClearType::Perfect.id(), 9);
        assert_eq!(ClearType::Max.id(), 10);
    }

    #[test]
    fn test_from_id_all_valid() {
        for i in 0..=10 {
            let ct = ClearType::get_clear_type_by_id(i);
            assert_eq!(ct.id(), i);
        }
    }

    #[test]
    fn test_from_id_invalid_returns_noplay() {
        assert_eq!(ClearType::get_clear_type_by_id(-1), ClearType::NoPlay);
        assert_eq!(ClearType::get_clear_type_by_id(11), ClearType::NoPlay);
        assert_eq!(ClearType::get_clear_type_by_id(999), ClearType::NoPlay);
    }

    #[test]
    fn test_from_id_round_trip() {
        for clear in ClearType::values() {
            let id = clear.id();
            let recovered = ClearType::get_clear_type_by_id(id);
            assert_eq!(recovered, *clear);
        }
    }

    #[test]
    fn test_gaugetype_mapping() {
        assert!(ClearType::NoPlay.gaugetype().is_empty());
        assert!(ClearType::Failed.gaugetype().is_empty());
        assert!(ClearType::AssistEasy.gaugetype().is_empty());
        assert_eq!(ClearType::LightAssistEasy.gaugetype(), &[0]);
        assert_eq!(ClearType::Easy.gaugetype(), &[1]);
        assert_eq!(ClearType::Normal.gaugetype(), &[2, 6]);
        assert_eq!(ClearType::Hard.gaugetype(), &[3, 7]);
        assert_eq!(ClearType::ExHard.gaugetype(), &[4, 8]);
        assert_eq!(ClearType::FullCombo.gaugetype(), &[5]);
        assert!(ClearType::Perfect.gaugetype().is_empty());
        assert!(ClearType::Max.gaugetype().is_empty());
    }

    #[test]
    fn test_get_clear_type_by_gauge() {
        assert_eq!(
            ClearType::get_clear_type_by_gauge(0),
            Some(ClearType::LightAssistEasy)
        );
        assert_eq!(ClearType::get_clear_type_by_gauge(1), Some(ClearType::Easy));
        assert_eq!(
            ClearType::get_clear_type_by_gauge(2),
            Some(ClearType::Normal)
        );
        assert_eq!(ClearType::get_clear_type_by_gauge(3), Some(ClearType::Hard));
        assert_eq!(
            ClearType::get_clear_type_by_gauge(4),
            Some(ClearType::ExHard)
        );
        assert_eq!(
            ClearType::get_clear_type_by_gauge(5),
            Some(ClearType::FullCombo)
        );
        // Gauge type 6 = Normal (class), 7 = Hard (class), 8 = ExHard (class)
        assert_eq!(
            ClearType::get_clear_type_by_gauge(6),
            Some(ClearType::Normal)
        );
        assert_eq!(ClearType::get_clear_type_by_gauge(7), Some(ClearType::Hard));
        assert_eq!(
            ClearType::get_clear_type_by_gauge(8),
            Some(ClearType::ExHard)
        );
        // Invalid gauge type
        assert_eq!(ClearType::get_clear_type_by_gauge(9), None);
        assert_eq!(ClearType::get_clear_type_by_gauge(-1), None);
    }

    #[test]
    fn test_serde_round_trip() {
        for clear in ClearType::values() {
            let json = serde_json::to_string(clear).unwrap();
            let deserialized: ClearType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, *clear);
        }
    }
}

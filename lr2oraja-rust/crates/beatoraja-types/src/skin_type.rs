use bms_model::mode::Mode;

/// Skin type enum
///
/// Translated from SkinType.java
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SkinType {
    #[default]
    Play7Keys,
    Play5Keys,
    Play14Keys,
    Play10Keys,
    Play9Keys,
    MusicSelect,
    Decide,
    Result,
    KeyConfig,
    SkinSelect,
    SoundSet,
    Theme,
    Play7KeysBattle,
    Play5KeysBattle,
    Play9KeysBattle,
    CourseResult,
    Play24Keys,
    Play24KeysDouble,
    Play24KeysBattle,
}

impl SkinType {
    pub fn get_id(&self) -> i32 {
        self.id()
    }

    pub fn get_name(&self) -> &'static str {
        self.name()
    }

    pub fn id(&self) -> i32 {
        match self {
            SkinType::Play7Keys => 0,
            SkinType::Play5Keys => 1,
            SkinType::Play14Keys => 2,
            SkinType::Play10Keys => 3,
            SkinType::Play9Keys => 4,
            SkinType::MusicSelect => 5,
            SkinType::Decide => 6,
            SkinType::Result => 7,
            SkinType::KeyConfig => 8,
            SkinType::SkinSelect => 9,
            SkinType::SoundSet => 10,
            SkinType::Theme => 11,
            SkinType::Play7KeysBattle => 12,
            SkinType::Play5KeysBattle => 13,
            SkinType::Play9KeysBattle => 14,
            SkinType::CourseResult => 15,
            SkinType::Play24Keys => 16,
            SkinType::Play24KeysDouble => 17,
            SkinType::Play24KeysBattle => 18,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            SkinType::Play7Keys => "7KEYS",
            SkinType::Play5Keys => "5KEYS",
            SkinType::Play14Keys => "14KEYS",
            SkinType::Play10Keys => "10KEYS",
            SkinType::Play9Keys => "9KEYS",
            SkinType::MusicSelect => "MUSIC SELECT",
            SkinType::Decide => "DECIDE",
            SkinType::Result => "RESULT",
            SkinType::KeyConfig => "KEY CONFIG",
            SkinType::SkinSelect => "SKIN SELECT",
            SkinType::SoundSet => "SOUND SET",
            SkinType::Theme => "THEME",
            SkinType::Play7KeysBattle => "7KEYS BATTLE",
            SkinType::Play5KeysBattle => "5KEYS BATTLE",
            SkinType::Play9KeysBattle => "9KEYS BATTLE",
            SkinType::CourseResult => "COURSE RESULT",
            SkinType::Play24Keys => "24KEYS",
            SkinType::Play24KeysDouble => "24KEYS DOUBLE",
            SkinType::Play24KeysBattle => "24KEYS BATTLE",
        }
    }

    pub fn is_play(&self) -> bool {
        match self {
            SkinType::Play7Keys => true,
            SkinType::Play5Keys => true,
            SkinType::Play14Keys => true,
            SkinType::Play10Keys => true,
            SkinType::Play9Keys => true,
            SkinType::MusicSelect => false,
            SkinType::Decide => false,
            SkinType::Result => false,
            SkinType::KeyConfig => false,
            SkinType::SkinSelect => false,
            SkinType::SoundSet => false,
            SkinType::Theme => false,
            SkinType::Play7KeysBattle => true,
            SkinType::Play5KeysBattle => true,
            SkinType::Play9KeysBattle => true,
            SkinType::CourseResult => false,
            SkinType::Play24Keys => true,
            SkinType::Play24KeysDouble => true,
            SkinType::Play24KeysBattle => true,
        }
    }

    pub fn mode(&self) -> Option<Mode> {
        match self {
            SkinType::Play7Keys => Some(Mode::BEAT_7K),
            SkinType::Play5Keys => Some(Mode::BEAT_5K),
            SkinType::Play14Keys => Some(Mode::BEAT_14K),
            SkinType::Play10Keys => Some(Mode::BEAT_10K),
            SkinType::Play9Keys => Some(Mode::POPN_9K),
            SkinType::MusicSelect => None,
            SkinType::Decide => None,
            SkinType::Result => None,
            SkinType::KeyConfig => None,
            SkinType::SkinSelect => None,
            SkinType::SoundSet => None,
            SkinType::Theme => None,
            SkinType::Play7KeysBattle => Some(Mode::BEAT_7K),
            SkinType::Play5KeysBattle => Some(Mode::BEAT_5K),
            SkinType::Play9KeysBattle => Some(Mode::POPN_9K),
            SkinType::CourseResult => None,
            SkinType::Play24Keys => Some(Mode::KEYBOARD_24K),
            SkinType::Play24KeysDouble => Some(Mode::KEYBOARD_24K_DOUBLE),
            SkinType::Play24KeysBattle => Some(Mode::KEYBOARD_24K),
        }
    }

    pub fn get_mode(&self) -> Option<Mode> {
        self.mode()
    }

    pub fn is_battle(&self) -> bool {
        matches!(
            self,
            SkinType::Play7KeysBattle
                | SkinType::Play5KeysBattle
                | SkinType::Play9KeysBattle
                | SkinType::Play24KeysBattle
        )
    }

    pub fn get_skin_type_by_id(id: i32) -> Option<SkinType> {
        Self::values()
            .into_iter()
            .find(|&skin_type| skin_type.id() == id)
    }

    pub fn get_max_skin_type_id() -> i32 {
        let mut max = -1_i32;
        for skin_type in Self::values() {
            max = max.max(skin_type.id());
        }
        max
    }

    pub fn values() -> Vec<SkinType> {
        vec![
            SkinType::Play7Keys,
            SkinType::Play5Keys,
            SkinType::Play14Keys,
            SkinType::Play10Keys,
            SkinType::Play9Keys,
            SkinType::MusicSelect,
            SkinType::Decide,
            SkinType::Result,
            SkinType::KeyConfig,
            SkinType::SkinSelect,
            SkinType::SoundSet,
            SkinType::Theme,
            SkinType::Play7KeysBattle,
            SkinType::Play5KeysBattle,
            SkinType::Play9KeysBattle,
            SkinType::CourseResult,
            SkinType::Play24Keys,
            SkinType::Play24KeysDouble,
            SkinType::Play24KeysBattle,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_play7keys() {
        let st = SkinType::default();
        assert_eq!(st, SkinType::Play7Keys);
    }

    #[test]
    fn test_values_count() {
        assert_eq!(SkinType::values().len(), 19);
    }

    #[test]
    fn test_id_mapping() {
        assert_eq!(SkinType::Play7Keys.id(), 0);
        assert_eq!(SkinType::Play5Keys.id(), 1);
        assert_eq!(SkinType::Play14Keys.id(), 2);
        assert_eq!(SkinType::Play10Keys.id(), 3);
        assert_eq!(SkinType::Play9Keys.id(), 4);
        assert_eq!(SkinType::MusicSelect.id(), 5);
        assert_eq!(SkinType::Decide.id(), 6);
        assert_eq!(SkinType::Result.id(), 7);
        assert_eq!(SkinType::KeyConfig.id(), 8);
        assert_eq!(SkinType::SkinSelect.id(), 9);
        assert_eq!(SkinType::SoundSet.id(), 10);
        assert_eq!(SkinType::Theme.id(), 11);
        assert_eq!(SkinType::Play7KeysBattle.id(), 12);
        assert_eq!(SkinType::Play5KeysBattle.id(), 13);
        assert_eq!(SkinType::Play9KeysBattle.id(), 14);
        assert_eq!(SkinType::CourseResult.id(), 15);
        assert_eq!(SkinType::Play24Keys.id(), 16);
        assert_eq!(SkinType::Play24KeysDouble.id(), 17);
        assert_eq!(SkinType::Play24KeysBattle.id(), 18);
    }

    #[test]
    fn test_get_id_equals_id() {
        for st in SkinType::values() {
            assert_eq!(st.get_id(), st.id());
        }
    }

    #[test]
    fn test_name_mapping() {
        assert_eq!(SkinType::Play7Keys.name(), "7KEYS");
        assert_eq!(SkinType::Play5Keys.name(), "5KEYS");
        assert_eq!(SkinType::Play14Keys.name(), "14KEYS");
        assert_eq!(SkinType::MusicSelect.name(), "MUSIC SELECT");
        assert_eq!(SkinType::Decide.name(), "DECIDE");
        assert_eq!(SkinType::Result.name(), "RESULT");
        assert_eq!(SkinType::Play7KeysBattle.name(), "7KEYS BATTLE");
        assert_eq!(SkinType::Play24Keys.name(), "24KEYS");
        assert_eq!(SkinType::Play24KeysDouble.name(), "24KEYS DOUBLE");
    }

    #[test]
    fn test_get_name_equals_name() {
        for st in SkinType::values() {
            assert_eq!(st.get_name(), st.name());
        }
    }

    #[test]
    fn test_is_play() {
        let play_types = [
            SkinType::Play7Keys,
            SkinType::Play5Keys,
            SkinType::Play14Keys,
            SkinType::Play10Keys,
            SkinType::Play9Keys,
            SkinType::Play7KeysBattle,
            SkinType::Play5KeysBattle,
            SkinType::Play9KeysBattle,
            SkinType::Play24Keys,
            SkinType::Play24KeysDouble,
            SkinType::Play24KeysBattle,
        ];
        let non_play_types = [
            SkinType::MusicSelect,
            SkinType::Decide,
            SkinType::Result,
            SkinType::KeyConfig,
            SkinType::SkinSelect,
            SkinType::SoundSet,
            SkinType::Theme,
            SkinType::CourseResult,
        ];

        for st in &play_types {
            assert!(st.is_play(), "{:?} should be play", st);
        }
        for st in &non_play_types {
            assert!(!st.is_play(), "{:?} should not be play", st);
        }
    }

    #[test]
    fn test_is_battle() {
        let battle_types = [
            SkinType::Play7KeysBattle,
            SkinType::Play5KeysBattle,
            SkinType::Play9KeysBattle,
            SkinType::Play24KeysBattle,
        ];
        for st in &battle_types {
            assert!(st.is_battle(), "{:?} should be battle", st);
        }

        // Non-battle types
        assert!(!SkinType::Play7Keys.is_battle());
        assert!(!SkinType::MusicSelect.is_battle());
        assert!(!SkinType::Play24Keys.is_battle());
        assert!(!SkinType::Play24KeysDouble.is_battle());
    }

    #[test]
    fn test_mode_mapping() {
        assert_eq!(SkinType::Play7Keys.mode(), Some(Mode::BEAT_7K));
        assert_eq!(SkinType::Play5Keys.mode(), Some(Mode::BEAT_5K));
        assert_eq!(SkinType::Play14Keys.mode(), Some(Mode::BEAT_14K));
        assert_eq!(SkinType::Play10Keys.mode(), Some(Mode::BEAT_10K));
        assert_eq!(SkinType::Play9Keys.mode(), Some(Mode::POPN_9K));
        assert_eq!(SkinType::MusicSelect.mode(), None);
        assert_eq!(SkinType::Decide.mode(), None);
        assert_eq!(SkinType::Result.mode(), None);
        assert_eq!(SkinType::Play7KeysBattle.mode(), Some(Mode::BEAT_7K));
        assert_eq!(SkinType::Play24Keys.mode(), Some(Mode::KEYBOARD_24K));
        assert_eq!(
            SkinType::Play24KeysDouble.mode(),
            Some(Mode::KEYBOARD_24K_DOUBLE)
        );
    }

    #[test]
    fn test_get_mode_equals_mode() {
        for st in SkinType::values() {
            assert_eq!(st.get_mode(), st.mode());
        }
    }

    #[test]
    fn test_get_skin_type_by_id() {
        for st in SkinType::values() {
            let found = SkinType::get_skin_type_by_id(st.id());
            assert_eq!(found, Some(st));
        }
    }

    #[test]
    fn test_get_skin_type_by_id_invalid() {
        assert_eq!(SkinType::get_skin_type_by_id(-1), None);
        assert_eq!(SkinType::get_skin_type_by_id(19), None);
        assert_eq!(SkinType::get_skin_type_by_id(100), None);
    }

    #[test]
    fn test_get_max_skin_type_id() {
        assert_eq!(SkinType::get_max_skin_type_id(), 18);
    }

    #[test]
    fn test_unique_ids() {
        let values = SkinType::values();
        let mut ids: Vec<i32> = values.iter().map(|st| st.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(
            ids.len(),
            values.len(),
            "All skin type IDs should be unique"
        );
    }
}

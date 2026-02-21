use bms_model::mode::Mode;

/// Skin type enum
///
/// Translated from SkinType.java
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkinType {
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
        for skin_type in Self::values() {
            if skin_type.id() == id {
                return Some(skin_type);
            }
        }
        None
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

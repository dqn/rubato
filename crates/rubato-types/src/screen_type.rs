use crate::main_state_type::MainStateType;

/// Enum to represent the current screen state type.
///
/// This is the external-facing version of MainStateType, used by listeners
/// and exporters that need to know which screen is active. Includes `Other`
/// for unknown/unsupported screen types.
///
/// Translated from: Java instanceof checks in ScreenShotExporter, DiscordListener, etc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenType {
    MusicSelector,
    MusicDecide,
    BMSPlayer,
    MusicResult,
    CourseResult,
    KeyConfiguration,
    Other,
}

impl ScreenType {
    /// Convert from MainStateType
    pub fn from_state_type(state: MainStateType) -> Self {
        match state {
            MainStateType::MusicSelect => ScreenType::MusicSelector,
            MainStateType::Decide => ScreenType::MusicDecide,
            MainStateType::Play => ScreenType::BMSPlayer,
            MainStateType::Result => ScreenType::MusicResult,
            MainStateType::CourseResult => ScreenType::CourseResult,
            MainStateType::Config => ScreenType::KeyConfiguration,
            MainStateType::SkinConfig => ScreenType::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_type_from_state_type() {
        assert_eq!(
            ScreenType::from_state_type(MainStateType::MusicSelect),
            ScreenType::MusicSelector
        );
        assert_eq!(
            ScreenType::from_state_type(MainStateType::Decide),
            ScreenType::MusicDecide
        );
        assert_eq!(
            ScreenType::from_state_type(MainStateType::Play),
            ScreenType::BMSPlayer
        );
        assert_eq!(
            ScreenType::from_state_type(MainStateType::Result),
            ScreenType::MusicResult
        );
        assert_eq!(
            ScreenType::from_state_type(MainStateType::CourseResult),
            ScreenType::CourseResult
        );
        assert_eq!(
            ScreenType::from_state_type(MainStateType::Config),
            ScreenType::KeyConfiguration
        );
        assert_eq!(
            ScreenType::from_state_type(MainStateType::SkinConfig),
            ScreenType::Other
        );
    }

    #[test]
    fn test_screen_type_eq() {
        assert_eq!(ScreenType::MusicSelector, ScreenType::MusicSelector);
        assert_ne!(ScreenType::MusicSelector, ScreenType::BMSPlayer);
    }
}

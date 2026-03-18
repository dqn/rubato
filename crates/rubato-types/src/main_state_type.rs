/// MainStateType - enum for each state in the application
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainStateType {
    MusicSelect,
    Decide,
    Play,
    Result,
    CourseResult,
    Config,
    SkinConfig,
}

impl MainStateType {
    /// Returns the OBS config key for this state type.
    ///
    /// Must match the SCREAMING_SNAKE_CASE keys used by `ObsListener::trigger_state_change_by_type`.
    pub fn obs_key(self) -> &'static str {
        match self {
            MainStateType::MusicSelect => "MUSICSELECT",
            MainStateType::Decide => "DECIDE",
            MainStateType::Play => "PLAY",
            MainStateType::Result => "RESULT",
            MainStateType::CourseResult => "COURSERESULT",
            MainStateType::Config => "CONFIG",
            MainStateType::SkinConfig => "SKINCONFIG",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obs_key_returns_screaming_case_for_all_variants() {
        // Verify every variant produces an all-uppercase key with no underscores
        // (except COURSERESULT/SKINCONFIG which are concatenated).
        let expected = [
            (MainStateType::MusicSelect, "MUSICSELECT"),
            (MainStateType::Decide, "DECIDE"),
            (MainStateType::Play, "PLAY"),
            (MainStateType::Result, "RESULT"),
            (MainStateType::CourseResult, "COURSERESULT"),
            (MainStateType::Config, "CONFIG"),
            (MainStateType::SkinConfig, "SKINCONFIG"),
        ];
        for (variant, key) in &expected {
            assert_eq!(variant.obs_key(), *key);
            // Keys must be all-uppercase ASCII
            assert!(
                key.chars().all(|c| c.is_ascii_uppercase()),
                "obs_key for {:?} contains non-uppercase chars: {}",
                variant,
                key
            );
        }
    }
}

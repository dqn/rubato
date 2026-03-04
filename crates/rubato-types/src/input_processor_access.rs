/// ControlKeys enum — keyboard control keys for BMS player input.
///
/// Translated from: KeyBoardInputProcesseor.ControlKeys
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKeys {
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Insert,
    Del,
    Escape,
    KeyC,
}

/// KeyCommand enum — high-level keyboard commands.
///
/// Translated from: bms.player.beatoraja.input.KeyCommand
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCommand {
    ShowFps,
    UpdateFolder,
    OpenExplorer,
    CopySongMd5Hash,
    CopySongSha256Hash,
    SwitchScreenMode,
    SaveScreenshot,
    PostTwitter,
    AddFavoriteSong,
    AddFavoriteChart,
    AutoplayFolder,
    OpenIr,
    OpenSkinConfiguration,
    ToggleModMenu,
    CopyHighlightedMenuText,
}

/// Trait interface for input processor access.
///
/// Downstream crates use `&dyn InputProcessorAccess` instead of concrete
/// BMSPlayerInputProcessor references.
pub trait InputProcessorAccess {
    /// Get the state of a control key (true = pressed).
    fn get_control_key_state(&self, key: ControlKeys) -> bool;

    /// Check if a key command has been activated this frame.
    fn is_activated(&self, cmd: KeyCommand) -> bool;

    /// Get the start time of the input processor.
    fn get_start_time(&self) -> i64 {
        0
    }

    /// Get accumulated scroll value.
    fn get_scroll(&self) -> i32 {
        0
    }

    /// Reset scroll accumulator to zero.
    fn reset_scroll(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestInput;
    impl InputProcessorAccess for TestInput {
        fn get_control_key_state(&self, _key: ControlKeys) -> bool {
            false
        }
        fn is_activated(&self, _cmd: KeyCommand) -> bool {
            false
        }
    }

    #[test]
    fn test_input_processor_access_trait() {
        let input = TestInput;
        assert!(!input.get_control_key_state(ControlKeys::Num1));
        assert!(!input.is_activated(KeyCommand::ShowFps));
        assert_eq!(input.get_start_time(), 0);
    }
}

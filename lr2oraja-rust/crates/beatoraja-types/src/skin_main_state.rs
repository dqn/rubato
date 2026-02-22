use crate::skin_offset::SkinOffset;
use crate::timer_access::TimerAccess;

/// Trait for skin rendering — provides the subset of MainState needed by skin objects.
///
/// This is the skin-specific MainState interface. Each concrete state (MusicSelector,
/// BMSPlayer, etc.) implements this trait so that SkinObject/SkinImage/etc. can
/// query timer state, offset values, and resource data during rendering.
///
/// Translated from Java: bms.player.beatoraja.MainState (rendering-related methods)
pub trait SkinMainState {
    /// Get the timer manager (read-only)
    fn get_timer(&self) -> &dyn TimerAccess;

    /// Get a skin offset value by ID
    fn get_offset_value(&self, id: i32) -> Option<&SkinOffset>;

    /// Get the mouse X position (from InputProcessor)
    fn get_mouse_x(&self) -> f32;

    /// Get the mouse Y position (from InputProcessor)
    fn get_mouse_y(&self) -> f32;

    /// Whether debug mode is active
    fn is_debug(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer_access::NullTimer;

    struct TestSkinState {
        timer: NullTimer,
    }

    impl SkinMainState for TestSkinState {
        fn get_timer(&self) -> &dyn TimerAccess {
            &self.timer
        }
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
        fn get_mouse_x(&self) -> f32 {
            0.0
        }
        fn get_mouse_y(&self) -> f32 {
            0.0
        }
        fn is_debug(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_skin_main_state_trait() {
        let state = TestSkinState { timer: NullTimer };
        assert_eq!(state.get_timer().get_now_time(), 0);
        assert!(state.get_offset_value(0).is_none());
        assert_eq!(state.get_mouse_x(), 0.0);
        assert!(!state.is_debug());
    }
}

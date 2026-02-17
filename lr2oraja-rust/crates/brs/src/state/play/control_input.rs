// Control input handling during play.
//
// Handles hi-speed changes, lane cover adjustments, and abort detection.
// Ported from Java BMSPlayer key handling logic.

/// Hi-speed change step (percentage points per press).
#[allow(dead_code)] // Used in tests
const HISPEED_STEP: i32 = 25;

/// Minimum hi-speed value.
#[allow(dead_code)] // Used in tests
const HISPEED_MIN: i32 = 25;

/// Maximum hi-speed value.
#[allow(dead_code)] // Used in tests
const HISPEED_MAX: i32 = 2000;

/// Actions resulting from control input processing.
#[allow(dead_code)] // Used in tests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    /// Request to abort play (Start+Select simultaneous press).
    Abort,
    /// Hi-speed increase.
    HiSpeedUp,
    /// Hi-speed decrease.
    HiSpeedDown,
}

/// Adjust hi-speed value by one step in the given direction.
///
/// Returns the new hi-speed value, clamped to [HISPEED_MIN, HISPEED_MAX].
#[allow(dead_code)] // Used in tests
pub fn adjust_hispeed(current: i32, increase: bool) -> i32 {
    let delta = if increase {
        HISPEED_STEP
    } else {
        -HISPEED_STEP
    };
    (current + delta).clamp(HISPEED_MIN, HISPEED_MAX)
}

/// Detect abort condition: both start and select pressed simultaneously.
#[allow(dead_code)] // Used in tests
pub fn detect_abort(start_pressed: bool, select_pressed: bool) -> bool {
    start_pressed && select_pressed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hispeed_increase() {
        assert_eq!(adjust_hispeed(100, true), 125);
    }

    #[test]
    fn hispeed_decrease() {
        assert_eq!(adjust_hispeed(100, false), 75);
    }

    #[test]
    fn hispeed_clamp_min() {
        assert_eq!(adjust_hispeed(25, false), 25);
    }

    #[test]
    fn hispeed_clamp_max() {
        assert_eq!(adjust_hispeed(2000, true), 2000);
    }

    #[test]
    fn abort_detected() {
        assert!(detect_abort(true, true));
    }

    #[test]
    fn no_abort_start_only() {
        assert!(!detect_abort(true, false));
    }

    #[test]
    fn no_abort_select_only() {
        assert!(!detect_abort(false, true));
    }

    #[test]
    fn no_abort_neither() {
        assert!(!detect_abort(false, false));
    }
}

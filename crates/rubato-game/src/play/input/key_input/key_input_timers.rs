use rubato_types::timer_id::TimerId;

// SkinProperty timer constants for key beam on/off
// Translated from SkinPropertyMapper.keyOnTimerId / keyOffTimerId
pub(super) const TIMER_KEYON_1P_SCRATCH: i32 = 100;
pub(super) const TIMER_KEYON_1P_KEY10: i32 = 1410;
pub(super) const TIMER_KEYOFF_1P_SCRATCH: i32 = 120;
pub(super) const TIMER_KEYOFF_1P_KEY10: i32 = 1610;

/// Compute the timer ID for key-on (key beam start).
///
/// Translated from: SkinPropertyMapper.keyOnTimerId(player, key)
pub(super) fn key_on_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_KEYON_1P_SCRATCH + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_KEYON_1P_KEY10 + key - 10 + player * 100);
        }
    }
    TimerId::new(-1)
}

/// Compute the timer ID for key-off (key beam end).
///
/// Translated from: SkinPropertyMapper.keyOffTimerId(player, key)
pub(super) fn key_off_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_KEYOFF_1P_SCRATCH + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_KEYOFF_1P_KEY10 + key - 10 + player * 100);
        }
    }
    TimerId::new(-1)
}

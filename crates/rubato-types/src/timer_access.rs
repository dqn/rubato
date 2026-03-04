/// Read-only access trait for TimerManager.
///
/// Used by skin rendering code to query timer state without mutable access.
/// The real TimerManager in beatoraja-core implements this trait.
///
/// Translated from Java: bms.player.beatoraja.TimerManager (read-only subset)
pub trait TimerAccess {
    /// Get current time in milliseconds (nowmicrotime / 1000)
    fn get_now_time(&self) -> i64;

    /// Get current time in microseconds
    fn get_now_micro_time(&self) -> i64;

    /// Get micro timer value for the given timer ID
    fn get_micro_timer(&self, timer_id: i32) -> i64;

    /// Get timer value in milliseconds for the given timer ID (getMicroTimer / 1000)
    fn get_timer(&self, timer_id: i32) -> i64;

    /// Get elapsed time since timer was set (millis), or 0 if timer is off.
    /// Java: getNowTime(int id)
    fn get_now_time_for(&self, timer_id: i32) -> i64;

    /// Check if the given timer is active (not i64::MIN)
    fn is_timer_on(&self, timer_id: i32) -> bool;
}

/// Default (null) implementation that returns zero/false for all queries.
#[derive(Clone, Debug, Default)]
pub struct NullTimer;

impl TimerAccess for NullTimer {
    fn get_now_time(&self) -> i64 {
        0
    }

    fn get_now_micro_time(&self) -> i64 {
        0
    }

    fn get_micro_timer(&self, _timer_id: i32) -> i64 {
        i64::MIN
    }

    fn get_timer(&self, _timer_id: i32) -> i64 {
        i64::MIN / 1000
    }

    fn get_now_time_for(&self, _timer_id: i32) -> i64 {
        0
    }

    fn is_timer_on(&self, _timer_id: i32) -> bool {
        false
    }
}

impl crate::skin_render_context::SkinRenderContext for NullTimer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_timer_defaults() {
        let timer = NullTimer;
        assert_eq!(timer.get_now_time(), 0);
        assert_eq!(timer.get_now_micro_time(), 0);
        assert_eq!(timer.get_micro_timer(0), i64::MIN);
        assert!(!timer.is_timer_on(0));
        assert_eq!(timer.get_now_time_for(0), 0);
    }

    #[test]
    fn test_timer_access_trait_object() {
        let timer: Box<dyn TimerAccess> = Box::new(NullTimer);
        assert_eq!(timer.get_now_time(), 0);
        assert!(!timer.is_timer_on(42));
    }
}

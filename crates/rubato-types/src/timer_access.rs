use crate::timer_id::TimerId;

/// Read-only access trait for TimerManager.
///
/// Used by skin rendering code to query timer state without mutable access.
/// The real TimerManager in beatoraja-core implements this trait.
///
/// Translated from Java: bms.player.beatoraja.TimerManager (read-only subset)
pub trait TimerAccess {
    /// Get current time in milliseconds (nowmicrotime / 1000)
    fn now_time(&self) -> i64;
    /// Get current time in microseconds
    fn now_micro_time(&self) -> i64;
    /// Get micro timer value for the given timer ID
    fn micro_timer(&self, timer_id: TimerId) -> i64;
    /// Get timer value in milliseconds for the given timer ID (getMicroTimer / 1000)
    fn timer(&self, timer_id: TimerId) -> i64;
    /// Get elapsed time since timer was set (millis), or 0 if timer is off.
    /// Java: getNowTime(int id)
    fn now_time_for(&self, timer_id: TimerId) -> i64;
    /// Check if the given timer is active (not i64::MIN)
    fn is_timer_on(&self, timer_id: TimerId) -> bool;
}

/// Default (null) implementation that returns zero/false for all queries.
#[derive(Clone, Debug, Default)]
pub struct NullTimer;

impl TimerAccess for NullTimer {
    fn now_time(&self) -> i64 {
        0
    }

    fn now_micro_time(&self) -> i64 {
        0
    }

    fn micro_timer(&self, _timer_id: TimerId) -> i64 {
        i64::MIN
    }

    fn timer(&self, _timer_id: TimerId) -> i64 {
        i64::MIN / 1000
    }

    fn now_time_for(&self, _timer_id: TimerId) -> i64 {
        0
    }

    fn is_timer_on(&self, _timer_id: TimerId) -> bool {
        false
    }
}

impl crate::skin_render_context::SkinEventHandler for NullTimer {}
impl crate::skin_render_context::SkinAudioControl for NullTimer {}
impl crate::skin_render_context::SkinPropertyProvider for NullTimer {}
impl crate::skin_render_context::SkinStateQuery for NullTimer {}
impl crate::skin_render_context::SkinConfigAccess for NullTimer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_timer_defaults() {
        let timer = NullTimer;
        assert_eq!(timer.now_time(), 0);
        assert_eq!(timer.now_micro_time(), 0);
        assert_eq!(timer.micro_timer(TimerId::new(0)), i64::MIN);
        assert!(!timer.is_timer_on(TimerId::new(0)));
        assert_eq!(timer.now_time_for(TimerId::new(0)), 0);
    }

    #[test]
    fn test_timer_access_trait_object() {
        let timer: Box<dyn TimerAccess> = Box::new(NullTimer);
        assert_eq!(timer.now_time(), 0);
        assert!(!timer.is_timer_on(TimerId::new(42)));
    }
}

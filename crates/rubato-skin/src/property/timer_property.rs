use crate::lua::skin_lua_accessor::LuaTimerProperty;
use crate::property::timer_property_factory::TimerPropertyImpl;
use crate::stubs::MainState;

pub trait TimerProperty: Send + Sync {
    fn get_micro(&self, state: &dyn MainState) -> i64;

    fn get(&self, state: &dyn MainState) -> i64 {
        self.get_micro(state) / 1000
    }

    fn now_time(&self, state: &dyn MainState) -> i64 {
        let time = self.get_micro(state);
        if time == i64::MIN {
            0
        } else {
            MainState::timer(state).now_time() - time / 1000
        }
    }

    fn is_on(&self, state: &dyn MainState) -> bool {
        self.get_micro(state) != i64::MIN
    }

    fn is_off(&self, state: &dyn MainState) -> bool {
        self.get_micro(state) == i64::MIN
    }

    /// Returns the timer ID.
    /// For script-defined timers, returns `i32::MIN`.
    fn get_timer_id(&self) -> i32 {
        i32::MIN
    }
}

/// Enum dispatch for TimerProperty, replacing `Box<dyn TimerProperty>`.
#[derive(Clone)]
pub enum TimerPropertyEnum {
    Impl(TimerPropertyImpl),
    Lua(LuaTimerProperty),
}

impl TimerProperty for TimerPropertyEnum {
    fn get_micro(&self, state: &dyn MainState) -> i64 {
        match self {
            Self::Impl(inner) => inner.get_micro(state),
            Self::Lua(inner) => inner.get_micro(state),
        }
    }

    fn get(&self, state: &dyn MainState) -> i64 {
        match self {
            Self::Impl(inner) => inner.get(state),
            Self::Lua(inner) => inner.get(state),
        }
    }

    fn now_time(&self, state: &dyn MainState) -> i64 {
        match self {
            Self::Impl(inner) => inner.now_time(state),
            Self::Lua(inner) => inner.now_time(state),
        }
    }

    fn is_on(&self, state: &dyn MainState) -> bool {
        match self {
            Self::Impl(inner) => inner.is_on(state),
            Self::Lua(inner) => inner.is_on(state),
        }
    }

    fn is_off(&self, state: &dyn MainState) -> bool {
        match self {
            Self::Impl(inner) => inner.is_off(state),
            Self::Lua(inner) => inner.is_off(state),
        }
    }

    fn get_timer_id(&self) -> i32 {
        match self {
            Self::Impl(inner) => inner.get_timer_id(),
            Self::Lua(inner) => inner.get_timer_id(),
        }
    }
}

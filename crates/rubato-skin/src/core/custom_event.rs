use crate::property::boolean_property::BooleanProperty;
use crate::property::event::Event;
use crate::stubs::MainState;

/// Custom event definition
///
/// Translated from CustomEvent.java
pub struct CustomEvent {
    pub id: i32,
    action: Box<dyn Event>,
    condition: Option<Box<dyn BooleanProperty>>,
    min_interval: i32,
    last_execute_time: i64,
}

impl CustomEvent {
    pub fn new(
        id: i32,
        action: Box<dyn Event>,
        condition: Option<Box<dyn BooleanProperty>>,
        min_interval: i32,
    ) -> Self {
        Self {
            id,
            action,
            condition,
            min_interval,
            last_execute_time: i64::MIN,
        }
    }

    pub fn execute(&mut self, state: &mut dyn MainState, arg1: i32, arg2: i32) {
        self.action.exec(state, arg1, arg2);
        self.last_execute_time = state.timer().now_micro_time();
    }

    pub fn update(&mut self, state: &mut dyn MainState) {
        if self.condition.is_none() {
            return;
        }

        let condition = self.condition.as_ref().expect("condition is Some");
        if condition.get(state)
            && (self.last_execute_time == i64::MIN
                || (state.timer().now_micro_time() - self.last_execute_time) / 1000
                    >= self.min_interval as i64)
        {
            self.action.exec_no_args(state);
            self.last_execute_time = state.timer().now_micro_time();
        }
    }
}

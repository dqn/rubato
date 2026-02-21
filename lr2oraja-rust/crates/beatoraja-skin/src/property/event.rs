use crate::stubs::MainState;

/// Events can be specified for reaction to buttons, and can be defined by users in a skin.
pub trait Event: Send + Sync {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, arg2: i32);

    fn exec_no_args(&self, state: &mut dyn MainState) {
        self.exec(state, 0, 0);
    }

    fn exec_one_arg(&self, state: &mut dyn MainState, arg1: i32) {
        self.exec(state, arg1, 0);
    }

    /// Returns the event ID.
    /// For script-defined timers, returns `i32::MIN`.
    fn get_event_id(&self) -> i32 {
        i32::MIN
    }
}

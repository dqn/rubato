use crate::main_state::MainState;

/// MainStateListener - interface for listening to main state changes
pub trait MainStateListener {
    fn update(&mut self, state: &dyn MainState, status: i32);
}

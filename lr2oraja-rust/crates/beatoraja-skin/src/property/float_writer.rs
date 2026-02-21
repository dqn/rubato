use crate::stubs::MainState;

/// Interface for writing float values back to state.
pub trait FloatWriter: Send + Sync {
    fn set(&self, state: &mut dyn MainState, value: f32);
}

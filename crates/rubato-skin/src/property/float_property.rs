use crate::stubs::MainState;

pub trait FloatProperty: Send + Sync {
    fn get(&self, state: &dyn MainState) -> f32;

    /// Returns the property ID, or `i32::MIN` if unknown.
    fn get_id(&self) -> i32 {
        i32::MIN
    }
}

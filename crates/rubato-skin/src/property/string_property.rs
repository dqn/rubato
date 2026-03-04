use crate::stubs::MainState;

pub trait StringProperty: Send + Sync {
    fn get(&self, state: &dyn MainState) -> String;

    /// Returns the property ID, or `i32::MIN` if unknown.
    fn get_id(&self) -> i32 {
        i32::MIN
    }
}

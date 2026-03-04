use crate::stubs::MainState;

pub trait BooleanProperty: Send + Sync {
    fn is_static(&self, state: &dyn MainState) -> bool;
    fn get(&self, state: &dyn MainState) -> bool;

    /// Returns the property ID. Negative IDs indicate negation.
    /// Returns `i32::MIN` if the ID is unknown (e.g. script-defined).
    fn get_id(&self) -> i32 {
        i32::MIN
    }
}

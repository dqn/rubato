use crate::stubs::MainState;

pub trait BooleanProperty: Send + Sync {
    fn is_static(&self, state: &dyn MainState) -> bool;
    fn get(&self, state: &dyn MainState) -> bool;
}

use crate::stubs::MainState;

pub trait IntegerProperty: Send + Sync {
    fn get(&self, state: &dyn MainState) -> i32;
}

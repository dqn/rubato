use crate::stubs::MainState;

pub trait StringProperty: Send + Sync {
    fn get(&self, state: &dyn MainState) -> String;
}

use crate::stubs::MainState;

pub trait FloatProperty: Send + Sync {
    fn get(&self, state: &dyn MainState) -> f32;
}

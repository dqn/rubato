use crate::reexports::{MainState, TextureRegion};

/// Skin source image set trait (abstract base class in Java)
pub trait SkinSourceSet: Send + Sync {
    fn get_images(&self, time: i64, state: &dyn MainState) -> Option<Vec<TextureRegion>>;
    fn validate(&self) -> bool;
    fn dispose(&mut self);
    fn is_disposed(&self) -> bool;

    /// Downcast to concrete type for test assertions.
    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any;
}

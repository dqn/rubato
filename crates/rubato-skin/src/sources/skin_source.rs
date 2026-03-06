use crate::stubs::{MainState, TextureRegion};

/// Skin source image trait (abstract base class in Java)
pub trait SkinSource: Send + Sync {
    fn get_image(&self, time: i64, state: &dyn MainState) -> Option<TextureRegion>;
    fn validate(&self) -> bool;
    fn dispose(&mut self);
    fn is_disposed(&self) -> bool;
}

/// Helper to dispose multiple SkinSource objects
pub fn dispose_all(sources: &mut [Option<Box<dyn SkinSource>>]) {
    for source in sources.iter_mut() {
        if let Some(s) = source
            && !s.is_disposed()
        {
            s.dispose();
        }
    }
}

/// Helper to dispose a single SkinSource
pub fn dispose_one(source: &mut dyn SkinSource) {
    if !source.is_disposed() {
        source.dispose();
    }
}

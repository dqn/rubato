use crate::skin_source::SkinSource;
use crate::stubs::{MainState, TextureRegion};

/// Skin source image (system reference) (SkinSourceReference.java)
pub struct SkinSourceReference {
    id: i32,
    disposed: bool,
}

impl SkinSourceReference {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            disposed: false,
        }
    }
}

impl SkinSource for SkinSourceReference {
    fn get_image(&self, _time: i64, state: &dyn MainState) -> Option<TextureRegion> {
        state.get_image(self.id)
    }

    fn validate(&self) -> bool {
        true
    }

    fn dispose(&mut self) {
        self.disposed = true;
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}

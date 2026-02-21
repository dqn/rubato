use crate::skin_source::SkinSource;
use crate::stubs::{MainState, TextureRegion};

/// Skin source movie (SkinSourceMovie.java)
pub struct SkinSourceMovie {
    _path: String,
    _timer: i32,
    _playing: bool,
    disposed: bool,
    region: TextureRegion,
}

impl SkinSourceMovie {
    pub fn new(path: &str) -> Self {
        Self::new_with_timer(path, 0)
    }

    pub fn new_with_timer(path: &str, timer: i32) -> Self {
        Self {
            _path: path.to_string(),
            _timer: timer,
            _playing: false,
            disposed: false,
            region: TextureRegion::new(),
        }
    }
}

impl SkinSource for SkinSourceMovie {
    fn get_image(&self, _time: i64, _state: &dyn MainState) -> Option<TextureRegion> {
        todo!("FFmpeg video processing not yet available")
    }

    fn validate(&self) -> bool {
        true
    }

    fn dispose(&mut self) {
        if !self.disposed {
            self.disposed = true;
        }
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}

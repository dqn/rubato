// SkinBar wrapper for SkinObject enum (Phase 32b)
// Minimal SkinBar type in beatoraja-skin to avoid circular dependency with beatoraja-select.
// The full SkinBar implementation lives in beatoraja-select::skin_bar.

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::stubs::MainState;

/// SkinBar skin object — minimal wrapper with SkinObjectData for the skin pipeline.
/// The full bar rendering logic lives in beatoraja-select::skin_bar::SkinBar.
pub struct SkinBarObject {
    pub data: SkinObjectData,
    /// Position mode (0 = normal, 1 = reverse)
    pub position: i32,
}

impl SkinBarObject {
    pub fn new(position: i32) -> Self {
        Self {
            data: SkinObjectData::new(),
            position,
        }
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);
    }

    pub fn draw(&self, _sprite: &mut SkinObjectRenderer) {
        // Stub: bar drawing is handled by BarRenderer in beatoraja-select
    }

    pub fn dispose(&mut self) {
        self.data.set_disposed();
    }
}

// SkinNote wrapper for SkinObject enum (Phase 32a)
// Wraps beatoraja_play::SkinNote with SkinObjectData for the skin pipeline.

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::stubs::MainState;

/// SkinNote skin object — wraps play-side SkinNote with SkinObjectData.
pub struct SkinNoteObject {
    pub data: SkinObjectData,
    pub inner: beatoraja_play::skin_note::SkinNote,
}

impl SkinNoteObject {
    pub fn new(lane_count: usize) -> Self {
        Self {
            data: SkinObjectData::new(),
            inner: beatoraja_play::skin_note::SkinNote::new(lane_count),
        }
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);
        self.inner.prepare(time);
    }

    pub fn draw(&mut self, _sprite: &mut SkinObjectRenderer) {
        // Stub: note drawing is handled by LaneRenderer, not the skin object itself
    }

    pub fn dispose(&mut self) {
        self.inner.dispose();
        self.data.set_disposed();
    }
}

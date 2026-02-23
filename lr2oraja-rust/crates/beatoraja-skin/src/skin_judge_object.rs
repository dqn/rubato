// SkinJudge wrapper for SkinObject enum (Phase 32a)
// Wraps beatoraja_play::SkinJudge with SkinObjectData for the skin pipeline.

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::stubs::MainState;

/// SkinJudge skin object — wraps play-side SkinJudge with SkinObjectData.
pub struct SkinJudgeObject {
    pub data: SkinObjectData,
    pub inner: beatoraja_play::skin_judge::SkinJudge,
}

impl SkinJudgeObject {
    pub fn new(player: i32, shift: bool) -> Self {
        Self {
            data: SkinObjectData::new(),
            inner: beatoraja_play::skin_judge::SkinJudge::new(player, shift),
        }
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);
        self.inner.prepare(time);
    }

    pub fn draw(&self, _sprite: &mut SkinObjectRenderer) {
        // Stub: judge drawing requires SkinImage/SkinNumber integration
    }

    pub fn dispose(&mut self) {
        self.inner.dispose();
        self.data.set_disposed();
    }
}

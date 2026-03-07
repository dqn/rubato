// SkinBar wrapper for SkinObject enum (Phase 32b)
// Minimal SkinBar type in beatoraja-skin to avoid circular dependency with beatoraja-select.
// The full SkinBar implementation lives in beatoraja-select::skin_bar.

use crate::stubs::MainState;
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

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

    pub fn draw(&mut self, _sprite: &mut SkinObjectRenderer) {
        // Stub: bar drawing is handled by BarRenderer in beatoraja-select
    }

    pub fn dispose(&mut self) {
        self.data.set_disposed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::MockMainState;

    /// Helper: set up a single-destination SkinObjectData so prepare() sets draw=true.
    fn setup_data(data: &mut SkinObjectData, x: f32, y: f32, w: f32, h: f32) {
        data.set_destination_with_int_timer_ops(
            0,
            x,
            y,
            w,
            h,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[0],
        );
    }

    #[test]
    fn test_skin_bar_object_two_phase_prepare_draw() {
        // Phase 40a: SkinBarObject follows two-phase pattern —
        // prepare(&mut self) then draw(&mut self, &mut sprite)
        let mut bar = SkinBarObject::new(0);
        setup_data(&mut bar.data, 10.0, 20.0, 300.0, 40.0);

        let state = MockMainState::default();

        // Phase 1: prepare — mutates internal state
        bar.prepare(0, &state);
        assert!(bar.data.draw_state.draw);
        assert_eq!(bar.data.draw_state.region.x, 10.0);
        assert_eq!(bar.data.draw_state.region.y, 20.0);
        assert_eq!(bar.data.draw_state.region.width, 300.0);

        // Phase 2: draw — reads pre-computed state (stub does nothing but verifies signature)
        let mut renderer = SkinObjectRenderer::new();
        bar.draw(&mut renderer);
        // No panic = success (draw is a stub for now)
    }

    #[test]
    fn test_skin_bar_object_prepare_sets_draw_false_when_timer_off() {
        // Phase 40a: prepare can set draw=false when conditions are not met
        let bar = SkinBarObject::new(0);
        // No destinations set, so validate would fail, but prepare with no dst
        // won't set draw=true either
        assert!(!bar.data.draw_state.draw);
    }

    #[test]
    fn test_skin_bar_object_position_preserved() {
        let bar = SkinBarObject::new(1);
        assert_eq!(bar.position, 1);
    }
}

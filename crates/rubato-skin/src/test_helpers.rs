// Test helpers for skin draw tests.
// Provides a mock MainState that satisfies the trait for testing.

use crate::reexports::{MainState, SkinOffset, Timer};

/// A minimal MainState implementation for testing.
pub struct MockMainState {
    pub timer: Timer,
    pub offsets: std::collections::HashMap<i32, SkinOffset>,
    pub mouse_x: f32,
    pub mouse_y: f32,
}

impl Default for MockMainState {
    fn default() -> Self {
        Self {
            timer: Timer::default(),
            offsets: std::collections::HashMap::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }
}

impl rubato_types::timer_access::TimerAccess for MockMainState {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }
    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for(timer_id)
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for MockMainState {
    fn get_offset_value(&self, id: i32) -> Option<&SkinOffset> {
        self.offsets.get(&id)
    }

    fn mouse_x(&self) -> f32 {
        self.mouse_x
    }

    fn mouse_y(&self) -> f32 {
        self.mouse_y
    }
}

impl MainState for MockMainState {}

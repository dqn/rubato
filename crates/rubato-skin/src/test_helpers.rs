// Test helpers for skin draw tests.
// Provides a mock MainState that satisfies the trait for testing.

use crate::stubs::{MainController, MainState, PlayerResource, SkinOffset, TextureRegion, Timer};

/// A minimal MainState implementation for testing.
pub struct MockMainState {
    pub timer: Timer,
    pub main: MainController,
    pub resource: PlayerResource,
    pub offsets: std::collections::HashMap<i32, SkinOffset>,
}

impl Default for MockMainState {
    fn default() -> Self {
        Self {
            timer: Timer::default(),
            main: MainController { debug: false },
            resource: PlayerResource,
            offsets: std::collections::HashMap::new(),
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
}

impl MainState for MockMainState {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
        &self.timer
    }

    fn get_main(&self) -> &MainController {
        &self.main
    }

    fn get_image(&self, _id: i32) -> Option<TextureRegion> {
        None
    }

    fn get_resource(&self) -> &PlayerResource {
        &self.resource
    }
}

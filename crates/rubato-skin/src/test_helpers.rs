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

impl MainState for MockMainState {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
        &self.timer
    }

    fn get_offset_value(&self, id: i32) -> Option<&SkinOffset> {
        self.offsets.get(&id)
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

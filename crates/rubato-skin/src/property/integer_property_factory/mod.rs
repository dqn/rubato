use rubato_types::value_id::ValueId;

use super::integer_property::IntegerProperty;
use super::property_lookup::{find_by_id, find_by_name};
use crate::stubs::MainState;

mod index_types;
mod value_type_data;
mod value_types;
use index_types::INDEX_TYPES;
use value_types::VALUE_TYPES;

const ID_LENGTH: usize = 65536;

/// Returns an IntegerProperty for the given option ID.
pub fn integer_property_by_id(optionid: i32) -> Option<Box<dyn IntegerProperty>> {
    let id = ValueId::new(optionid);
    if optionid < 0 || optionid as usize >= ID_LENGTH {
        return None;
    }

    // Check ValueType enum
    find_by_id!(VALUE_TYPES, id, DelegateIntegerProperty);

    // Check various range-based properties and switch-based properties
    // All reference BMSPlayer, MusicSelector, AbstractResult etc.
    Some(Box::new(DelegateIntegerProperty { id }))
}

/// Returns an IntegerProperty for the given ValueType name.
pub fn integer_property_by_name(name: &str) -> Option<Box<dyn IntegerProperty>> {
    find_by_name!(VALUE_TYPES, name, DelegateIntegerProperty);
    None
}

/// Returns an IntegerProperty for image index usage.
pub fn image_index_property_by_id(optionid: i32) -> Option<Box<dyn IntegerProperty>> {
    let id = ValueId::new(optionid);
    if optionid < 0 || optionid as usize >= ID_LENGTH {
        return None;
    }

    // Check IndexType enum
    find_by_id!(INDEX_TYPES, id, DelegateImageIndexProperty);

    // Judge properties (VALUE_JUDGE_1P_SCRATCH to VALUE_JUDGE_2P_KEY99)
    // SkinSelectType properties
    // All require Phase 7+ dependencies

    Some(Box::new(DelegateImageIndexProperty { id }))
}

/// Returns an IntegerProperty for the given IndexType name.
pub fn image_index_property_by_name(name: &str) -> Option<Box<dyn IntegerProperty>> {
    find_by_name!(INDEX_TYPES, name, DelegateImageIndexProperty);
    None
}

/// Delegate IntegerProperty that reads values from MainState::integer_value().
/// This enables both StaticStateProvider (golden-master) and real game states
/// to provide integer values through the same interface.
struct DelegateIntegerProperty {
    id: ValueId,
}

impl IntegerProperty for DelegateIntegerProperty {
    fn get(&self, state: &dyn MainState) -> i32 {
        state.integer_value(self.id.as_i32())
    }

    fn get_id(&self) -> i32 {
        self.id.as_i32()
    }
}

struct DelegateImageIndexProperty {
    id: ValueId,
}

impl IntegerProperty for DelegateImageIndexProperty {
    fn get(&self, state: &dyn MainState) -> i32 {
        state.image_index_value(self.id.as_i32())
    }

    fn get_id(&self) -> i32 {
        self.id.as_i32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering_stubs::TextureRegion;
    use crate::stubs::{MainController, PlayerResource, SkinOffset, Timer};

    struct TestState {
        timer: Timer,
        integer_value: i32,
        image_index_value: i32,
    }

    impl TestState {
        fn new(integer_value: i32, image_index_value: i32) -> Self {
            Self {
                timer: Timer::default(),
                integer_value,
                image_index_value,
            }
        }
    }

    impl rubato_types::timer_access::TimerAccess for TestState {
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

    impl rubato_types::skin_render_context::SkinRenderContext for TestState {
        fn integer_value(&self, _id: i32) -> i32 {
            self.integer_value
        }

        fn image_index_value(&self, _id: i32) -> i32 {
            self.image_index_value
        }
    }

    impl MainState for TestState {
        fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
            &self.timer
        }

        fn get_main(&self) -> &MainController {
            static MAIN: MainController = MainController { debug: false };
            &MAIN
        }

        fn get_image(&self, _id: i32) -> Option<TextureRegion> {
            None
        }

        fn get_resource(&self) -> &PlayerResource {
            static RESOURCE: PlayerResource = PlayerResource;
            &RESOURCE
        }
    }

    #[test]
    fn value_refs_read_integer_value() {
        let state = TestState::new(7, 9);
        let property = integer_property_by_id(42).expect("value property should exist");

        assert_eq!(property.get(&state), 7);
    }

    #[test]
    fn image_index_refs_read_image_index_value() {
        let state = TestState::new(7, 9);
        let property = image_index_property_by_id(42).expect("image index property should exist");

        assert_eq!(property.get(&state), 9);
    }
}

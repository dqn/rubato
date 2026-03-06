// Phase 60c: Verify MainControllerRef stores input processor directly (no more Box::leak).
//
// Previously input_processor() used Box::leak to return references.
// Now it returns a reference to an owned field, so repeated calls return the same address.

use rubato_state::decide::stubs::{MainControllerRef, NullMainController};

/// input_processor() returns the same stored instance on repeated calls.
#[test]
fn get_input_processor_returns_same_instance() {
    let mut mc = MainControllerRef::new(Box::new(NullMainController));

    let ptr1 = mc.input_processor() as *const _ as usize;
    let ptr2 = mc.input_processor() as *const _ as usize;

    assert_eq!(ptr1, ptr2, "should return same stored instance");
}

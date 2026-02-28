// Phase 6: Memory Safety — Document Box::leak in beatoraja-decide MainControllerRef stubs
//
// MainControllerRef::get_input_processor() uses Box::leak to return &'static mut references.
// Each call allocates new heap memory that is never freed.
// These tests document the leak by verifying that consecutive calls return different addresses.

use beatoraja_decide::stubs::{MainControllerRef, NullMainController};

/// Calling get_input_processor() twice returns different &mut references,
/// proving each call leaks a new Box allocation. This documents the intentional
/// memory leak in the stub — every caller pays heap allocation cost that is
/// never reclaimed.
#[test]
fn memory_safety_box_leak_input_processor_allocates_each_call() {
    let mut mc = MainControllerRef::new(Box::new(NullMainController));

    let ptr1 = mc.get_input_processor() as *const _ as usize;
    let ptr2 = mc.get_input_processor() as *const _ as usize;

    // Each call to get_input_processor() creates a new Box::leak allocation,
    // so the pointers must differ. This documents the intentional leak behavior.
    assert_ne!(
        ptr1, ptr2,
        "Box::leak should allocate new memory each call (documenting leak)"
    );
}

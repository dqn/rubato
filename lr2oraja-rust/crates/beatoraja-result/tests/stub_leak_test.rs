// Phase 50a: Document Box::leak memory leak in MainController stubs
//
// MainController::get_input_processor() uses Box::leak to return &'static mut references.
// Each call allocates new heap memory that is never freed.
// This test documents the leak by verifying that two calls return different addresses.

use beatoraja_result::stubs::{MainController, NullMainController};

/// Calling get_input_processor() twice returns different &mut references,
/// proving each call leaks a new Box allocation.
#[test]
fn get_input_processor_leaks_new_allocation_each_call() {
    let mut mc = MainController::new(Box::new(NullMainController));

    let ptr1 = mc.get_input_processor() as *const _ as usize;
    let ptr2 = mc.get_input_processor() as *const _ as usize;

    // Each call to get_input_processor() creates a new Box::leak allocation,
    // so the pointers must differ. This documents the intentional leak behavior.
    assert_ne!(ptr1, ptr2, "Box::leak should allocate new memory each call");
}

/// ir_send_status() also uses Box::leak and leaks on every call.
#[test]
fn ir_send_status_leaks_new_allocation_each_call() {
    let mc = MainController::new(Box::new(NullMainController));

    let ptr1 = mc.ir_send_status() as *const _ as usize;
    let ptr2 = mc.ir_send_status() as *const _ as usize;

    assert_ne!(ptr1, ptr2, "Box::leak should allocate new memory each call");
}

/// get_play_data_accessor() also uses Box::leak and leaks on every call.
#[test]
fn get_play_data_accessor_leaks_new_allocation_each_call() {
    let mc = MainController::new(Box::new(NullMainController));

    let ptr1 = mc.get_play_data_accessor() as *const _ as usize;
    let ptr2 = mc.get_play_data_accessor() as *const _ as usize;

    assert_ne!(ptr1, ptr2, "Box::leak should allocate new memory each call");
}

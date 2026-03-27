// Phase 60c: Verify MainController stores components directly (no more Box::leak).
//
// Previously these methods used Box::leak to return references.
// Now they return references to owned fields, so repeated calls return the same address.
// input_processor was removed during InputSnapshot migration.

use rubato_game::state::result::{MainController, NullMainController};

/// ir_send_status() returns the same shared Vec (backed by Arc<Mutex<...>>).
#[test]
fn ir_send_status_returns_same_instance() {
    let mc = MainController::new(Box::new(NullMainController));

    // With Arc<Mutex<Vec<...>>>, each call returns a MutexGuard to the same Vec.
    // Verify by checking the underlying data pointer.
    let guard1 = mc.ir_send_status();
    let ptr1 = guard1.as_ptr() as usize;
    drop(guard1);

    let guard2 = mc.ir_send_status();
    let ptr2 = guard2.as_ptr() as usize;
    drop(guard2);

    assert_eq!(ptr1, ptr2, "should return same stored Vec");
}

/// play_data_accessor() returns the same stored instance on repeated calls.
#[test]
fn get_play_data_accessor_returns_same_instance() {
    let mc = MainController::new(Box::new(NullMainController));

    let ptr1 = mc.play_data_accessor() as *const _ as usize;
    let ptr2 = mc.play_data_accessor() as *const _ as usize;

    assert_eq!(ptr1, ptr2, "should return same stored instance");
}

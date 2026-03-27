// Phase 6: Memory Safety — Document unsafe Send+Sync impls on raw pointer wrappers
//
// Several types in the Lua integration layer contain raw pointers (*const dyn MainState
// or *const Lua) and manually implement Send+Sync. While the SAFETY comments claim
// single-threaded access, the compiler cannot verify this invariant.
//
// These tests statically assert that the types claim Send+Sync, documenting the
// unsafe contract. If the unsafe impls were ever removed, these tests would fail
// to compile, serving as a reminder that the safety invariant needs to be maintained
// or the types must be redesigned.

use rubato_skin::lua::event_utility::EventUtility;
use rubato_skin::lua::main_state_accessor::{MainStateAccessor, SnapshotAccessor};
use rubato_skin::lua::timer_utility::TimerUtility;

/// Static assertion helper: compiles only if T: Send + Sync.
fn assert_send_sync<T: Send + Sync>() {}

/// MainStateAccessor wraps a raw *const dyn MainState pointer with
/// unsafe impl Send + Sync. This test documents that the type claims
/// thread safety despite containing a raw pointer.
#[test]
fn memory_safety_main_state_accessor_is_send_sync() {
    // This compiles — documenting that MainStateAccessor claims Send+Sync
    // despite containing a raw pointer to dyn MainState.
    assert_send_sync::<MainStateAccessor>();
}

/// TimerUtility wraps a raw *const dyn MainState pointer with
/// unsafe impl Send + Sync.
#[test]
fn memory_safety_timer_utility_is_send_sync() {
    assert_send_sync::<TimerUtility>();
}

/// EventUtility wraps a raw *const dyn MainState pointer with
/// unsafe impl Send + Sync.
#[test]
fn memory_safety_event_utility_is_send_sync() {
    assert_send_sync::<EventUtility>();
}

/// SnapshotAccessor wraps a raw *mut PropertySnapshot pointer with
/// unsafe impl Send + Sync. This test documents that the type claims
/// thread safety despite containing a raw pointer to PropertySnapshot.
#[test]
fn memory_safety_snapshot_accessor_is_send_sync() {
    assert_send_sync::<SnapshotAccessor>();
}

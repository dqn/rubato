//! Phase 5f: Timer and animation E2E tests.
//!
//! Tests deterministic timer control and time accumulation
//! via the E2eHarness frozen timer infrastructure.

use rubato_e2e::{E2eHarness, FRAME_DURATION_US};

// ============================================================
// 1. Frozen timer stays at zero without explicit steps
// ============================================================

#[test]
fn test_frozen_timer_stays_at_zero() {
    let mut harness = E2eHarness::new();

    // Timer is frozen at 0 by harness construction.
    // Calling timer update() should not advance time.
    harness.controller_mut().timer_mut().update();
    assert_eq!(
        harness.current_time_us(),
        0,
        "frozen timer should remain at 0 after update()"
    );

    // Render frames via controller.render() only (without step_frame)
    // to verify the frozen timer does not drift from wall clock.
    harness.controller_mut().render();
    harness.controller_mut().render();
    assert_eq!(
        harness.current_time_us(),
        0,
        "frozen timer should remain at 0 after controller render calls"
    );
}

// ============================================================
// 2. Timer advances to the value set by set_time()
// ============================================================

#[test]
fn test_timer_advances_with_step() {
    let mut harness = E2eHarness::new();

    harness.set_time(500_000);
    assert_eq!(
        harness.current_time_us(),
        500_000,
        "timer should report the value set by set_time()"
    );

    harness.set_time(1_000_000);
    assert_eq!(
        harness.current_time_us(),
        1_000_000,
        "timer should update to the new value after second set_time()"
    );
}

// ============================================================
// 3. step_frame() advances by exactly one 60fps frame (16667us)
// ============================================================

#[test]
fn test_frame_step_advances_by_16667us() {
    let mut harness = E2eHarness::new();

    assert_eq!(harness.current_time_us(), 0, "should start at 0");

    harness.step_frame();
    assert_eq!(
        harness.current_time_us(),
        16_667,
        "one step_frame() should advance to 16667us (60fps frame duration)"
    );
    assert_eq!(
        FRAME_DURATION_US, 16_667,
        "FRAME_DURATION_US constant should be 16667"
    );
}

// ============================================================
// 4. Multiple steps accumulate correctly
// ============================================================

#[test]
fn test_multiple_steps_accumulate() {
    let mut harness = E2eHarness::new();

    harness.step_frames(10);
    assert_eq!(
        harness.current_time_us(),
        10 * FRAME_DURATION_US,
        "10 steps should accumulate to 10 * FRAME_DURATION_US = {}",
        10 * FRAME_DURATION_US
    );
}

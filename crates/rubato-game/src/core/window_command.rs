// Window commands signaled from MainController to the application shell (beatoraja-bin).
// Uses atomic flags for lock-free cross-layer communication.

use std::sync::atomic::{AtomicBool, Ordering};

static FULLSCREEN_TOGGLE: AtomicBool = AtomicBool::new(false);
static SCREENSHOT_REQUEST: AtomicBool = AtomicBool::new(false);

/// Request a fullscreen toggle (called by MainController on F4 press).
pub fn request_fullscreen_toggle() {
    FULLSCREEN_TOGGLE.store(true, Ordering::Release);
}

/// Consume the fullscreen toggle request (called by the app shell).
/// Returns true if a toggle was requested since the last call.
pub fn take_fullscreen_toggle() -> bool {
    FULLSCREEN_TOGGLE.swap(false, Ordering::AcqRel)
}

/// Request a screenshot capture (called by MainController on hotkey press).
pub fn request_screenshot() {
    SCREENSHOT_REQUEST.store(true, Ordering::Release);
}

/// Consume the screenshot request (called by the app shell).
/// Returns true if a screenshot was requested since the last call.
pub fn take_screenshot_request() -> bool {
    SCREENSHOT_REQUEST.swap(false, Ordering::AcqRel)
}

//! Global focus state for SkinWidgetManager.
//!
//! Extracted to beatoraja-types to break circular dependency:
//! beatoraja-core -> beatoraja-input -> beatoraja-modmenu -> beatoraja-core.
//!
//! Both beatoraja-input (consumer) and beatoraja-modmenu (setter) import from here.

use std::sync::Mutex;

static FOCUS: Mutex<bool> = Mutex::new(false);

/// Returns the current focus state of the skin widget manager.
pub fn get_focus() -> bool {
    *FOCUS.lock().unwrap()
}

/// Sets the focus state of the skin widget manager.
pub fn set_focus(focus: bool) {
    *FOCUS.lock().unwrap() = focus;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_focus_default_is_false() {
        set_focus(false);
        assert!(!get_focus());
    }

    #[test]
    fn test_get_focus_returns_true_after_set_true() {
        set_focus(true);
        assert!(get_focus());
        // Clean up
        set_focus(false);
    }

    #[test]
    fn test_get_focus_returns_false_after_set_false() {
        set_focus(true);
        set_focus(false);
        assert!(!get_focus());
    }
}

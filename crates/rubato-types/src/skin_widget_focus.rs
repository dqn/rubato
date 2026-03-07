//! Global focus state for SkinWidgetManager.
//!
//! Extracted to beatoraja-types to break circular dependency:
//! beatoraja-core -> beatoraja-input -> beatoraja-modmenu -> beatoraja-core.
//!
//! Both beatoraja-input (consumer) and beatoraja-modmenu (setter) import from here.

use std::sync::Mutex;

static FOCUS: Mutex<bool> = Mutex::new(false);

fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Returns the current focus state of the skin widget manager.
pub fn focus() -> bool {
    *lock_or_recover(&FOCUS)
}

/// Sets the focus state of the skin widget manager.
pub fn set_focus(focus: bool) {
    *lock_or_recover(&FOCUS) = focus;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_focus_default_is_false() {
        set_focus(false);
        assert!(!focus());
    }

    #[test]
    fn test_get_focus_returns_true_after_set_true() {
        set_focus(true);
        assert!(focus());
        // Clean up
        set_focus(false);
    }

    #[test]
    fn test_get_focus_returns_false_after_set_false() {
        set_focus(true);
        set_focus(false);
        assert!(!focus());
    }

    #[test]
    fn test_focus_recovers_after_poison() {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = FOCUS.lock().expect("mutex poisoned");
            panic!("poison focus");
        }));

        set_focus(true);
        assert!(focus());
        set_focus(false);
        assert!(!focus());
    }
}

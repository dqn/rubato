use std::sync::Mutex;

static LAST_PLAYED_SORT: Mutex<bool> = Mutex::new(false);

fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn is_enabled() -> bool {
    *lock_or_recover(&LAST_PLAYED_SORT)
}

pub fn force_disable() {
    *lock_or_recover(&LAST_PLAYED_SORT) = false;
}

pub fn set(value: bool) {
    *lock_or_recover(&LAST_PLAYED_SORT) = value;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_played_sort_recovers_after_poison() {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = LAST_PLAYED_SORT.lock().expect("mutex poisoned");
            panic!("poison last played sort");
        }));

        set(true);
        assert!(is_enabled());
        force_disable();
        assert!(!is_enabled());
    }
}

use std::sync::Mutex;

static LAST_PLAYED_SORT: Mutex<bool> = Mutex::new(false);

pub fn is_enabled() -> bool {
    *LAST_PLAYED_SORT.lock().unwrap()
}

pub fn force_disable() {
    *LAST_PLAYED_SORT.lock().unwrap() = false;
}

pub fn set(value: bool) {
    *LAST_PLAYED_SORT.lock().unwrap() = value;
}

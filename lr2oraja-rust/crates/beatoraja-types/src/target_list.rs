use std::sync::Mutex;

static TARGETS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static TARGET_NAMES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Set the target ID list.
pub fn set_target_ids(targets: Vec<String>) {
    *TARGETS.lock().unwrap() = targets;
}

/// Set the display names for the current target list.
/// Must be called after `set_target_ids` with a names vec of the same length.
pub fn set_target_names(names: Vec<String>) {
    *TARGET_NAMES.lock().unwrap() = names;
}

/// Get the current target ID list.
pub fn get_targets() -> Vec<String> {
    TARGETS.lock().unwrap().clone()
}

/// Look up the display name for a target ID.
pub fn get_target_name(target: &str) -> String {
    let targets = TARGETS.lock().unwrap();
    let names = TARGET_NAMES.lock().unwrap();
    for i in 0..targets.len() {
        if targets[i] == target && i < names.len() {
            return names[i].clone();
        }
    }
    String::new()
}

use std::borrow::Cow;
use std::sync::Mutex;

use crate::player_information::PlayerInformation;

static TARGETS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static TARGET_NAMES: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Set the target ID list.
pub fn set_target_ids(targets: Vec<String>) {
    *lock_or_recover(&TARGETS) = targets;
}

/// Set the display names for the current target list.
/// Must be called after `set_target_ids` with a names vec of the same length.
pub fn set_target_names(names: Vec<String>) {
    *lock_or_recover(&TARGET_NAMES) = names;
}

/// Get the current target ID list.
pub fn targets() -> Vec<String> {
    lock_or_recover(&TARGETS).clone()
}

/// Look up the display name for a target ID.
pub fn target_name(target: &str) -> String {
    let targets = lock_or_recover(&TARGETS);
    let names = lock_or_recover(&TARGET_NAMES);
    for (t, n) in targets.iter().zip(names.iter()) {
        if *t == target {
            return n.clone();
        }
    }
    String::new()
}

/// Resolve a target ID to a display name using static mappings and rival info.
///
/// Static mappings: RANK_AAA→"RANK AAA-", RANK_AA→"RANK AA-", RANK_A→"RANK A-",
/// RANK_MAX→"MAX-", MYBEST→"MY BEST", RANK_NEXT→"NEXT RANK".
/// Rival mappings: RIVAL_1..RIVAL_4 → rivals[n-1].name().
///
/// Java: TargetProperty.getTargetName()
pub fn resolve_target_name<'a>(id: &'a str, rivals: &[PlayerInformation]) -> Cow<'a, str> {
    match id {
        "RANK_AAA" => Cow::Borrowed("RANK AAA-"),
        "RANK_AA" => Cow::Borrowed("RANK AA-"),
        "RANK_A" => Cow::Borrowed("RANK A-"),
        "RANK_MAX" => Cow::Borrowed("MAX-"),
        "MYBEST" => Cow::Borrowed("MY BEST"),
        "RANK_NEXT" => Cow::Borrowed("NEXT RANK"),
        _ => {
            if let Some(suffix) = id.strip_prefix("RIVAL_")
                && let Ok(n) = suffix.parse::<usize>()
                && n >= 1
                && n <= rivals.len()
            {
                return Cow::Owned(rivals[n - 1].name().to_string());
            }
            Cow::Borrowed(id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_target_name_static_mappings() {
        let rivals = [];
        assert_eq!(resolve_target_name("RANK_AAA", &rivals), "RANK AAA-");
        assert_eq!(resolve_target_name("RANK_AA", &rivals), "RANK AA-");
        assert_eq!(resolve_target_name("RANK_A", &rivals), "RANK A-");
        assert_eq!(resolve_target_name("RANK_MAX", &rivals), "MAX-");
        assert_eq!(resolve_target_name("MYBEST", &rivals), "MY BEST");
        assert_eq!(resolve_target_name("RANK_NEXT", &rivals), "NEXT RANK");
    }

    #[test]
    fn test_resolve_target_name_rival_lookup() {
        let rivals = vec![
            PlayerInformation {
                name: Some("Alice".to_string()),
                ..Default::default()
            },
            PlayerInformation {
                name: Some("Bob".to_string()),
                ..Default::default()
            },
        ];
        assert_eq!(resolve_target_name("RIVAL_1", &rivals), "Alice");
        assert_eq!(resolve_target_name("RIVAL_2", &rivals), "Bob");
        // Out of range returns the ID as-is
        assert_eq!(resolve_target_name("RIVAL_3", &rivals), "RIVAL_3");
    }

    #[test]
    fn test_resolve_target_name_unknown() {
        assert_eq!(resolve_target_name("UNKNOWN", &[]), "UNKNOWN");
    }

    #[test]
    fn target_list_recovers_after_poison() {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = TARGETS.lock().expect("mutex poisoned");
            panic!("poison targets");
        }));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = TARGET_NAMES.lock().expect("mutex poisoned");
            panic!("poison names");
        }));

        set_target_ids(vec!["MYBEST".to_string()]);
        set_target_names(vec!["MY BEST".to_string()]);

        assert_eq!(targets(), vec!["MYBEST".to_string()]);
        assert_eq!(target_name("MYBEST"), "MY BEST");
    }
}

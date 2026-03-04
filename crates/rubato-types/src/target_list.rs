use std::sync::Mutex;

use crate::player_information::PlayerInformation;

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

/// Resolve a target ID to a display name using static mappings and rival info.
///
/// Static mappings: RANK_AAA→"RANK AAA-", RANK_AA→"RANK AA-", RANK_A→"RANK A-",
/// RANK_MAX→"MAX-", MYBEST→"MY BEST", RANK_NEXT→"NEXT RANK".
/// Rival mappings: RIVAL_1..RIVAL_4 → rivals[n-1].get_name().
///
/// Java: TargetProperty.getTargetName()
pub fn resolve_target_name(id: &str, rivals: &[PlayerInformation]) -> String {
    match id {
        "RANK_AAA" => "RANK AAA-".to_string(),
        "RANK_AA" => "RANK AA-".to_string(),
        "RANK_A" => "RANK A-".to_string(),
        "RANK_MAX" => "MAX-".to_string(),
        "MYBEST" => "MY BEST".to_string(),
        "RANK_NEXT" => "NEXT RANK".to_string(),
        _ => {
            if let Some(suffix) = id.strip_prefix("RIVAL_")
                && let Ok(n) = suffix.parse::<usize>()
                && n >= 1
                && n <= rivals.len()
            {
                return rivals[n - 1].get_name().to_string();
            }
            id.to_string()
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
}

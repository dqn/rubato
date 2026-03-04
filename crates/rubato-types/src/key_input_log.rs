// KeyInputLog DTO - moved from stubs.rs (Phase 30b)
// NOTE: This is the DTO version with pub fields for JSON serde.
// The runtime version exists in beatoraja-input with private fields.

/// Stub for beatoraja.input.KeyInputLog (pub fields; beatoraja-input uses private fields)
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KeyInputLog {
    pub time: i64,
    pub keycode: i32,
    pub pressed: bool,
}

impl KeyInputLog {
    pub fn validate(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_input_log_default() {
        let log = KeyInputLog::default();
        assert_eq!(log.time, 0);
        assert_eq!(log.keycode, 0);
        assert!(!log.pressed);
    }

    #[test]
    fn test_key_input_log_validate() {
        let log = KeyInputLog {
            time: 1000,
            keycode: 5,
            pressed: true,
        };
        assert!(log.validate());
    }

    #[test]
    fn test_key_input_log_serde_round_trip() {
        let log = KeyInputLog {
            time: 12345,
            keycode: 3,
            pressed: true,
        };
        let json = serde_json::to_string(&log).unwrap();
        let deserialized: KeyInputLog = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.time, 12345);
        assert_eq!(deserialized.keycode, 3);
        assert!(deserialized.pressed);
    }

    #[test]
    fn test_key_input_log_clone() {
        let log = KeyInputLog {
            time: 500,
            keycode: 1,
            pressed: false,
        };
        let cloned = log.clone();
        assert_eq!(cloned.time, log.time);
        assert_eq!(cloned.keycode, log.keycode);
        assert_eq!(cloned.pressed, log.pressed);
    }
}

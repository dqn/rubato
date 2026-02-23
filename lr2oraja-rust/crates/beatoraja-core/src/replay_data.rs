pub use beatoraja_types::replay_data::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::KeyInputLog;
    use crate::validatable::Validatable;

    #[test]
    fn test_replay_data_new() {
        let rd = ReplayData::new();
        assert_eq!(rd.randomoptionseed, -1);
        assert_eq!(rd.randomoption2seed, -1);
        assert_eq!(rd.mode, 0);
        assert_eq!(rd.gauge, 0);
        assert!(rd.keylog.is_empty());
        assert!(rd.keyinput.is_none());
        assert!(rd.pattern.is_none());
        assert!(rd.rand.is_empty());
        assert_eq!(rd.date, 0);
    }

    #[test]
    fn test_replay_data_default() {
        let rd = ReplayData::default();
        assert_eq!(rd.randomoptionseed, 0);
        assert_eq!(rd.randomoption2seed, 0);
        assert!(rd.keylog.is_empty());
    }

    #[test]
    fn test_replay_data_shrink_empty_keylog() {
        let mut rd = ReplayData::new();
        rd.shrink();
        // Should not create keyinput if keylog is empty
        assert!(rd.keyinput.is_none());
    }

    #[test]
    fn test_replay_data_shrink_encodes_keylog() {
        let mut rd = ReplayData::new();
        rd.keylog = vec![
            KeyInputLog {
                time: 1000,
                keycode: 0,
                pressed: true,
            },
            KeyInputLog {
                time: 2000,
                keycode: 0,
                pressed: false,
            },
            KeyInputLog {
                time: 3000,
                keycode: 1,
                pressed: true,
            },
        ];

        rd.shrink();
        assert!(rd.keyinput.is_some(), "keyinput should be set after shrink");
        assert!(
            rd.keylog.is_empty(),
            "keylog should be emptied after shrink"
        );
    }

    #[test]
    fn test_replay_data_shrink_validate_roundtrip() {
        let mut rd = ReplayData::new();
        let original_keylogs = vec![
            KeyInputLog {
                time: 1000,
                keycode: 0,
                pressed: true,
            },
            KeyInputLog {
                time: 2000,
                keycode: 0,
                pressed: false,
            },
            KeyInputLog {
                time: 3000,
                keycode: 2,
                pressed: true,
            },
            KeyInputLog {
                time: 4000,
                keycode: 2,
                pressed: false,
            },
        ];
        rd.keylog = original_keylogs.clone();

        // Shrink encodes keylog into keyinput
        rd.shrink();
        assert!(rd.keyinput.is_some());
        assert!(rd.keylog.is_empty());

        // Validate decodes keyinput back into keylog
        let valid = rd.validate();
        assert!(valid, "validate should return true for valid replay data");
        assert_eq!(rd.keylog.len(), original_keylogs.len());

        for (i, (decoded, original)) in rd.keylog.iter().zip(original_keylogs.iter()).enumerate() {
            assert_eq!(decoded.time, original.time, "time mismatch at index {}", i);
            assert_eq!(
                decoded.keycode, original.keycode,
                "keycode mismatch at index {}",
                i
            );
            assert_eq!(
                decoded.pressed, original.pressed,
                "pressed mismatch at index {}",
                i
            );
        }
    }

    #[test]
    fn test_replay_data_validate_empty_returns_false() {
        let mut rd = ReplayData::new();
        // No keylog and no keyinput
        let valid = rd.validate();
        assert!(!valid, "validate should return false for empty replay data");
    }

    #[test]
    fn test_replay_data_field_accessors() {
        let mut rd = ReplayData::new();
        rd.player = Some("player1".to_string());
        rd.sha256 = Some("hash123".to_string());
        rd.mode = 7;
        rd.gauge = 3;
        rd.date = 1234567890;
        rd.randomoption = 1;
        rd.randomoption2 = 2;
        rd.doubleoption = 3;

        assert_eq!(rd.player.as_deref(), Some("player1"));
        assert_eq!(rd.sha256.as_deref(), Some("hash123"));
        assert_eq!(rd.mode, 7);
        assert_eq!(rd.gauge, 3);
        assert_eq!(rd.date, 1234567890);
        assert_eq!(rd.randomoption, 1);
        assert_eq!(rd.randomoption2, 2);
        assert_eq!(rd.doubleoption, 3);
    }

    #[test]
    fn test_replay_data_serde_roundtrip() {
        let mut rd = ReplayData::new();
        rd.player = Some("testplayer".to_string());
        rd.sha256 = Some("abc".to_string());
        rd.mode = 7;
        rd.gauge = 2;
        rd.rand = vec![1, 2, 3];
        rd.seven_to_nine_pattern = 1;

        let json = serde_json::to_string(&rd).unwrap();
        let deserialized: ReplayData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.player, Some("testplayer".to_string()));
        assert_eq!(deserialized.sha256, Some("abc".to_string()));
        assert_eq!(deserialized.mode, 7);
        assert_eq!(deserialized.gauge, 2);
        assert_eq!(deserialized.rand, vec![1, 2, 3]);
        assert_eq!(deserialized.seven_to_nine_pattern, 1);
        assert_eq!(deserialized.randomoptionseed, -1);
        assert_eq!(deserialized.randomoption2seed, -1);
    }

    #[test]
    fn test_replay_data_with_pattern() {
        use crate::stubs::PatternModifyLog;

        let mut rd = ReplayData::new();
        rd.pattern = Some(vec![
            PatternModifyLog {
                old_lane: 2,
                new_lane: 0,
            },
            PatternModifyLog {
                old_lane: 3,
                new_lane: 1,
            },
        ]);

        assert_eq!(rd.pattern.as_ref().unwrap().len(), 2);
        assert_eq!(rd.pattern.as_ref().unwrap()[0].old_lane, 2);
        assert_eq!(rd.pattern.as_ref().unwrap()[0].new_lane, 0);
    }

    #[test]
    fn test_replay_data_lane_shuffle_pattern() {
        let mut rd = ReplayData::new();
        rd.lane_shuffle_pattern = Some(vec![vec![0, 1, 2], vec![2, 0, 1]]);

        assert_eq!(rd.lane_shuffle_pattern.as_ref().unwrap().len(), 2);
        assert_eq!(rd.lane_shuffle_pattern.as_ref().unwrap()[0], vec![0, 1, 2]);
    }
}

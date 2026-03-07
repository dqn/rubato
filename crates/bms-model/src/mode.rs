use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum Mode {
    BEAT_5K,
    BEAT_7K,
    BEAT_10K,
    BEAT_14K,
    POPN_5K,
    POPN_9K,
    KEYBOARD_24K,
    KEYBOARD_24K_DOUBLE,
}

impl Mode {
    pub fn id(&self) -> i32 {
        match self {
            Mode::BEAT_5K => 5,
            Mode::BEAT_7K => 7,
            Mode::BEAT_10K => 10,
            Mode::BEAT_14K => 14,
            Mode::POPN_5K => 9,
            Mode::POPN_9K => 9,
            Mode::KEYBOARD_24K => 25,
            Mode::KEYBOARD_24K_DOUBLE => 50,
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Mode::BEAT_5K => "beat-5k",
            Mode::BEAT_7K => "beat-7k",
            Mode::BEAT_10K => "beat-10k",
            Mode::BEAT_14K => "beat-14k",
            Mode::POPN_5K => "popn-5k",
            Mode::POPN_9K => "popn-9k",
            Mode::KEYBOARD_24K => "keyboard-24k",
            Mode::KEYBOARD_24K_DOUBLE => "keyboard-24k-double",
        }
    }

    pub fn player(&self) -> i32 {
        match self {
            Mode::BEAT_5K => 1,
            Mode::BEAT_7K => 1,
            Mode::BEAT_10K => 2,
            Mode::BEAT_14K => 2,
            Mode::POPN_5K => 1,
            Mode::POPN_9K => 1,
            Mode::KEYBOARD_24K => 1,
            Mode::KEYBOARD_24K_DOUBLE => 2,
        }
    }

    pub fn key(&self) -> i32 {
        match self {
            Mode::BEAT_5K => 6,
            Mode::BEAT_7K => 8,
            Mode::BEAT_10K => 12,
            Mode::BEAT_14K => 16,
            Mode::POPN_5K => 5,
            Mode::POPN_9K => 9,
            Mode::KEYBOARD_24K => 26,
            Mode::KEYBOARD_24K_DOUBLE => 52,
        }
    }

    pub fn scratch_key(&self) -> &'static [i32] {
        match self {
            Mode::BEAT_5K => &[5],
            Mode::BEAT_7K => &[7],
            Mode::BEAT_10K => &[5, 11],
            Mode::BEAT_14K => &[7, 15],
            Mode::POPN_5K => &[],
            Mode::POPN_9K => &[],
            Mode::KEYBOARD_24K => &[24, 25],
            Mode::KEYBOARD_24K_DOUBLE => &[24, 25, 50, 51],
        }
    }

    pub fn is_scratch_key(&self, key: i32) -> bool {
        for sc in self.scratch_key() {
            if key == *sc {
                return true;
            }
        }
        false
    }

    pub fn from_hint(hint: &str) -> Option<Mode> {
        let modes = [
            Mode::BEAT_5K,
            Mode::BEAT_7K,
            Mode::BEAT_10K,
            Mode::BEAT_14K,
            Mode::POPN_5K,
            Mode::POPN_9K,
            Mode::KEYBOARD_24K,
            Mode::KEYBOARD_24K_DOUBLE,
        ];
        modes.into_iter().find(|mode| mode.hint() == hint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_exist() {
        let variants = [
            Mode::BEAT_5K,
            Mode::BEAT_7K,
            Mode::BEAT_10K,
            Mode::BEAT_14K,
            Mode::POPN_5K,
            Mode::POPN_9K,
            Mode::KEYBOARD_24K,
            Mode::KEYBOARD_24K_DOUBLE,
        ];
        assert_eq!(variants.len(), 8);
    }

    #[test]
    fn key_values() {
        assert_eq!(Mode::BEAT_5K.key(), 6);
        assert_eq!(Mode::BEAT_7K.key(), 8);
        assert_eq!(Mode::BEAT_10K.key(), 12);
        assert_eq!(Mode::BEAT_14K.key(), 16);
        assert_eq!(Mode::POPN_5K.key(), 5);
        assert_eq!(Mode::POPN_9K.key(), 9);
        assert_eq!(Mode::KEYBOARD_24K.key(), 26);
        assert_eq!(Mode::KEYBOARD_24K_DOUBLE.key(), 52);
    }

    #[test]
    fn player_values() {
        // Single-player modes
        assert_eq!(Mode::BEAT_5K.player(), 1);
        assert_eq!(Mode::BEAT_7K.player(), 1);
        assert_eq!(Mode::POPN_5K.player(), 1);
        assert_eq!(Mode::POPN_9K.player(), 1);
        assert_eq!(Mode::KEYBOARD_24K.player(), 1);

        // Double-player modes
        assert_eq!(Mode::BEAT_10K.player(), 2);
        assert_eq!(Mode::BEAT_14K.player(), 2);
        assert_eq!(Mode::KEYBOARD_24K_DOUBLE.player(), 2);
    }

    #[test]
    fn id_values() {
        assert_eq!(Mode::BEAT_5K.id(), 5);
        assert_eq!(Mode::BEAT_7K.id(), 7);
        assert_eq!(Mode::BEAT_10K.id(), 10);
        assert_eq!(Mode::BEAT_14K.id(), 14);
        assert_eq!(Mode::POPN_5K.id(), 9);
        assert_eq!(Mode::POPN_9K.id(), 9);
        assert_eq!(Mode::KEYBOARD_24K.id(), 25);
        assert_eq!(Mode::KEYBOARD_24K_DOUBLE.id(), 50);
    }

    #[test]
    fn scratch_key_values() {
        assert_eq!(Mode::BEAT_5K.scratch_key(), &[5]);
        assert_eq!(Mode::BEAT_7K.scratch_key(), &[7]);
        assert_eq!(Mode::BEAT_10K.scratch_key(), &[5, 11]);
        assert_eq!(Mode::BEAT_14K.scratch_key(), &[7, 15]);
        assert_eq!(Mode::POPN_5K.scratch_key(), &[] as &[i32]);
        assert_eq!(Mode::POPN_9K.scratch_key(), &[] as &[i32]);
        assert_eq!(Mode::KEYBOARD_24K.scratch_key(), &[24, 25]);
        assert_eq!(Mode::KEYBOARD_24K_DOUBLE.scratch_key(), &[24, 25, 50, 51]);
    }

    #[test]
    fn is_scratch_key_true() {
        assert!(Mode::BEAT_5K.is_scratch_key(5));
        assert!(Mode::BEAT_7K.is_scratch_key(7));
        assert!(Mode::BEAT_10K.is_scratch_key(5));
        assert!(Mode::BEAT_10K.is_scratch_key(11));
        assert!(Mode::BEAT_14K.is_scratch_key(7));
        assert!(Mode::BEAT_14K.is_scratch_key(15));
    }

    #[test]
    fn is_scratch_key_false() {
        assert!(!Mode::BEAT_5K.is_scratch_key(0));
        assert!(!Mode::BEAT_7K.is_scratch_key(0));
        assert!(!Mode::POPN_5K.is_scratch_key(0));
        assert!(!Mode::POPN_9K.is_scratch_key(0));
    }

    #[test]
    fn get_mode_from_hint() {
        assert_eq!(Mode::from_hint("beat-5k"), Some(Mode::BEAT_5K));
        assert_eq!(Mode::from_hint("beat-7k"), Some(Mode::BEAT_7K));
        assert_eq!(Mode::from_hint("beat-10k"), Some(Mode::BEAT_10K));
        assert_eq!(Mode::from_hint("beat-14k"), Some(Mode::BEAT_14K));
        assert_eq!(Mode::from_hint("popn-5k"), Some(Mode::POPN_5K));
        assert_eq!(Mode::from_hint("popn-9k"), Some(Mode::POPN_9K));
        assert_eq!(Mode::from_hint("keyboard-24k"), Some(Mode::KEYBOARD_24K));
        assert_eq!(
            Mode::from_hint("keyboard-24k-double"),
            Some(Mode::KEYBOARD_24K_DOUBLE)
        );
    }

    #[test]
    fn get_mode_invalid_hint_returns_none() {
        assert_eq!(Mode::from_hint("invalid"), None);
        assert_eq!(Mode::from_hint(""), None);
        assert_eq!(Mode::from_hint("beat-3k"), None);
    }

    #[test]
    fn hint_values() {
        assert_eq!(Mode::BEAT_5K.hint(), "beat-5k");
        assert_eq!(Mode::BEAT_7K.hint(), "beat-7k");
        assert_eq!(Mode::BEAT_10K.hint(), "beat-10k");
        assert_eq!(Mode::BEAT_14K.hint(), "beat-14k");
        assert_eq!(Mode::POPN_5K.hint(), "popn-5k");
        assert_eq!(Mode::POPN_9K.hint(), "popn-9k");
        assert_eq!(Mode::KEYBOARD_24K.hint(), "keyboard-24k");
        assert_eq!(Mode::KEYBOARD_24K_DOUBLE.hint(), "keyboard-24k-double");
    }

    #[test]
    fn hint_roundtrips_through_get_mode() {
        let modes = [
            Mode::BEAT_5K,
            Mode::BEAT_7K,
            Mode::BEAT_10K,
            Mode::BEAT_14K,
            Mode::POPN_5K,
            Mode::POPN_9K,
            Mode::KEYBOARD_24K,
            Mode::KEYBOARD_24K_DOUBLE,
        ];
        for mode in &modes {
            let hint = mode.hint();
            let recovered = Mode::from_hint(hint).expect("should find mode by hint");
            assert_eq!(&recovered, mode);
        }
    }
}

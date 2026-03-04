// long_note_modifier::Mode - moved from stubs.rs (Phase 30a)

#[derive(Clone, Debug)]
pub enum Mode {
    Off,
    Add,
    Remove,
}

impl Mode {
    pub fn values() -> &'static [Mode] {
        &[Mode::Off, Mode::Add, Mode::Remove]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_note_modifier_mode_values() {
        let values = Mode::values();
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_long_note_modifier_mode_clone_debug() {
        let m = Mode::Add;
        let cloned = m.clone();
        assert_eq!(format!("{:?}", cloned), "Add");
    }
}

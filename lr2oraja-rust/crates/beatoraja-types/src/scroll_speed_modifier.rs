// scroll_speed_modifier::Mode - moved from stubs.rs (Phase 30a)

#[derive(Clone, Debug)]
pub enum Mode {
    Off,
    Variable,
    Fixed,
}

impl Mode {
    pub fn values() -> &'static [Mode] {
        &[Mode::Off, Mode::Variable, Mode::Fixed]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_speed_modifier_mode_values() {
        let values = Mode::values();
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_scroll_speed_modifier_mode_clone_debug() {
        let m = Mode::Variable;
        let cloned = m.clone();
        assert_eq!(format!("{:?}", cloned), "Variable");
    }
}

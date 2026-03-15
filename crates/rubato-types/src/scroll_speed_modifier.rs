// scroll_speed_modifier::Mode

#[derive(Clone, Copy, Debug)]
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
        let copied = m;
        assert_eq!(format!("{:?}", copied), "Variable");
    }
}

// ResultKeyProperty.java -> result_key_property.rs
// Mechanical line-by-line translation.

use bms_model::mode::Mode;

/// Result key assignment for each key
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultKey {
    Ok,
    ReplayDifferent,
    ReplaySame,
    ChangeGraph,
}

/// Result key property - maps input keys for result screen navigation
#[derive(Clone, Debug)]
pub struct ResultKeyProperty {
    assign: Vec<Option<ResultKey>>,
}

impl ResultKeyProperty {
    pub fn new(keys: Vec<Option<ResultKey>>) -> Self {
        Self { assign: keys }
    }

    pub fn get_assign(&self, index: i32) -> Option<ResultKey> {
        if index < 0 || index as usize >= self.assign.len() {
            return None;
        }
        self.assign[index as usize]
    }

    pub fn get_assign_length(&self) -> i32 {
        self.assign.len() as i32
    }

    /// Get ResultKeyProperty for a given Mode. Corresponds to Java's ResultKeyProperty.get(Mode)
    pub fn get(mode: &Mode) -> Option<ResultKeyProperty> {
        match mode {
            Mode::BEAT_5K => Some(ResultKeyProperty::beat_5k()),
            Mode::BEAT_7K => Some(ResultKeyProperty::beat_7k()),
            Mode::BEAT_10K => Some(ResultKeyProperty::beat_10k()),
            Mode::BEAT_14K => Some(ResultKeyProperty::beat_14k()),
            Mode::POPN_9K => Some(ResultKeyProperty::popn_9k()),
            Mode::KEYBOARD_24K => Some(ResultKeyProperty::keyboard_24k()),
            Mode::KEYBOARD_24K_DOUBLE => Some(ResultKeyProperty::keyboard_24k_double()),
            _ => Some(ResultKeyProperty::beat_7k()),
        }
    }

    pub fn beat_5k() -> Self {
        Self::new(vec![
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            None,
            None,
        ])
    }

    pub fn beat_7k() -> Self {
        Self::new(vec![
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            None,
            None,
        ])
    }

    pub fn beat_10k() -> Self {
        Self::new(vec![
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            None,
            None,
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            None,
            None,
        ])
    }

    pub fn beat_14k() -> Self {
        Self::new(vec![
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            None,
            None,
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            None,
            None,
        ])
    }

    pub fn popn_9k() -> Self {
        Self::new(vec![
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
        ])
    }

    pub fn keyboard_24k() -> Self {
        Self::new(vec![
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            None,
            None,
        ])
    }

    pub fn keyboard_24k_double() -> Self {
        Self::new(vec![
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            None,
            None,
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::ReplayDifferent),
            Some(ResultKey::ChangeGraph),
            Some(ResultKey::ReplaySame),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            Some(ResultKey::Ok),
            None,
            None,
        ])
    }
}

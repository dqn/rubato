// SkinStateProvider: lightweight state interface for skin snapshot evaluation.
//
// Decouples skin evaluation from the full MainState trait, providing only
// the values needed for golden-master snapshot comparison.

use std::collections::HashMap;

use rubato_skin::stubs::{
    MainController, MainState, PlayerResource, SkinOffset, TextureRegion, Timer,
};
use serde::{Deserialize, Serialize};

/// Lightweight state provider for skin evaluation.
/// Decoupled from MainState to allow pure-function snapshot capture.
pub trait SkinStateProvider {
    /// Current time in milliseconds (replaces Timer.getNowTime()).
    fn now_time_ms(&self) -> i64;

    /// Whether a timer is active. An inactive timer causes objects to hide.
    fn is_timer_on(&self, timer_id: i32) -> bool;

    /// Timer value in milliseconds. Time is relative to timer start.
    fn timer_value_ms(&self, timer_id: i32) -> i64;

    /// Boolean property value. Negative IDs indicate negation.
    fn boolean_value(&self, id: i32) -> bool;

    /// Whether a boolean value has been explicitly set.
    fn has_boolean_value(&self, id: i32) -> bool;

    /// Integer property value.
    fn integer_value(&self, id: i32) -> i32;

    /// Whether an integer value has been explicitly set.
    fn has_integer_value(&self, id: i32) -> bool;

    /// Float property value.
    fn float_value(&self, id: i32) -> f32;

    /// Whether a float value has been explicitly set.
    fn has_float_value(&self, id: i32) -> bool;

    /// String property value.
    fn string_value(&self, id: i32) -> Option<String>;

    /// Skin offset value.
    fn offset_value(&self, id: i32) -> Option<SkinOffset>;
}

/// Static state provider backed by HashMaps. For golden-master tests.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StaticStateProvider {
    pub time_ms: i64,
    #[serde(default)]
    pub timers: HashMap<i32, i64>,
    #[serde(default)]
    pub booleans: HashMap<i32, bool>,
    #[serde(default)]
    pub integers: HashMap<i32, i32>,
    #[serde(default)]
    pub floats: HashMap<i32, f32>,
    #[serde(default)]
    pub strings: HashMap<i32, String>,
    #[serde(default)]
    pub offsets: HashMap<i32, SkinOffset>,
}

impl SkinStateProvider for StaticStateProvider {
    fn now_time_ms(&self) -> i64 {
        self.time_ms
    }

    fn is_timer_on(&self, timer_id: i32) -> bool {
        // Timer 0 (no timer) is always on.
        // Explicitly set timers are on.
        // Unset timers are off.
        timer_id == 0 || self.timers.contains_key(&timer_id)
    }

    fn timer_value_ms(&self, timer_id: i32) -> i64 {
        self.timers.get(&timer_id).copied().unwrap_or(0)
    }

    fn boolean_value(&self, id: i32) -> bool {
        let abs_id = id.abs();
        let value = self.booleans.get(&abs_id).copied().unwrap_or(false);
        if id < 0 { !value } else { value }
    }

    fn has_boolean_value(&self, id: i32) -> bool {
        self.booleans.contains_key(&id.abs())
    }

    fn integer_value(&self, id: i32) -> i32 {
        self.integers.get(&id).copied().unwrap_or(0)
    }

    fn has_integer_value(&self, id: i32) -> bool {
        self.integers.contains_key(&id)
    }

    fn float_value(&self, id: i32) -> f32 {
        self.floats.get(&id).copied().unwrap_or(0.0)
    }

    fn has_float_value(&self, id: i32) -> bool {
        self.floats.contains_key(&id)
    }

    fn string_value(&self, id: i32) -> Option<String> {
        self.strings.get(&id).cloned()
    }

    fn offset_value(&self, id: i32) -> Option<SkinOffset> {
        self.offsets.get(&id).cloned()
    }
}

/// MainState adapter that wraps a StaticStateProvider.
///
/// Implements the skin crate's MainState trait so that Lua skins can call
/// `main_state.number(id)` and `main_state.text(id)` during skin loading.
/// The integer/string values are looked up from the provider's HashMaps.
pub struct StaticMainStateAdapter<'a> {
    provider: &'a StaticStateProvider,
    timer: Timer,
    main: MainController,
    resource: PlayerResource,
}

impl<'a> StaticMainStateAdapter<'a> {
    pub fn new(provider: &'a StaticStateProvider) -> Self {
        Self {
            provider,
            timer: Timer::with_timers(provider.time_ms, provider.time_ms * 1000, Vec::new()),
            main: MainController { debug: false },
            resource: PlayerResource,
        }
    }
}

impl MainState for StaticMainStateAdapter<'_> {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
        &self.timer
    }

    fn get_offset_value(&self, id: i32) -> Option<&SkinOffset> {
        self.provider.offsets.get(&id)
    }

    fn get_main(&self) -> &MainController {
        &self.main
    }

    fn get_image(&self, _id: i32) -> Option<TextureRegion> {
        None
    }

    fn get_resource(&self) -> &PlayerResource {
        &self.resource
    }

    fn integer_value(&self, id: i32) -> i32 {
        self.provider.integers.get(&id).copied().unwrap_or(0)
    }

    fn string_value(&self, id: i32) -> String {
        self.provider.strings.get(&id).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_returns_zeroes() {
        let p = StaticStateProvider::default();
        assert_eq!(p.now_time_ms(), 0);
        assert!(!p.is_timer_on(1));
        assert!(p.is_timer_on(0));
        assert_eq!(p.timer_value_ms(1), 0);
        assert!(!p.boolean_value(1));
        assert!(!p.has_boolean_value(1));
        assert_eq!(p.integer_value(1), 0);
        assert!(!p.has_integer_value(1));
        assert_eq!(p.float_value(1), 0.0);
        assert!(!p.has_float_value(1));
        assert!(p.string_value(1).is_none());
        assert!(p.offset_value(1).is_none());
    }

    #[test]
    fn boolean_negation() {
        let mut p = StaticStateProvider::default();
        p.booleans.insert(5, true);
        assert!(p.boolean_value(5));
        assert!(!p.boolean_value(-5));
        p.booleans.insert(5, false);
        assert!(!p.boolean_value(5));
        assert!(p.boolean_value(-5));
    }

    #[test]
    fn timer_on_off() {
        let mut p = StaticStateProvider::default();
        assert!(!p.is_timer_on(42));
        p.timers.insert(42, 100);
        assert!(p.is_timer_on(42));
        assert_eq!(p.timer_value_ms(42), 100);
    }

    #[test]
    fn serde_round_trip() {
        let mut p = StaticStateProvider::default();
        p.time_ms = 50;
        p.booleans.insert(1, true);
        p.integers.insert(100, 42);
        p.floats.insert(4, 0.5);
        p.strings.insert(30, "hello".to_string());

        let json = serde_json::to_string(&p).unwrap();
        let back: StaticStateProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(back.time_ms, 50);
        assert!(back.boolean_value(1));
        assert_eq!(back.integer_value(100), 42);
        assert!((back.float_value(4) - 0.5).abs() < f32::EPSILON);
        assert_eq!(back.string_value(30), Some("hello".to_string()));
    }
}

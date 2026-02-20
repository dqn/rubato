// Lua state provider trait for runtime game state access.
//
// Defines the interface that Lua skin scripts use to query and mutate
// game state (timers, numbers, options, volumes, etc.).
//
// `StubLuaStateProvider` returns default values for all methods,
// matching the existing Lua stub behavior used during header-only loading.
//
// Ported from SkinLuaAccessor.exportMainStateAccessor(MainState) in Java.

use crate::skin_object::SkinOffset;

/// Timer OFF sentinel value (matches Java Long.MIN_VALUE semantics).
pub const TIMER_OFF: i64 = i64::MIN;

/// Trait for providing runtime game state to Lua skin scripts.
pub trait LuaStateProvider {
    // Read-only state queries
    fn option(&self, id: i32) -> bool;
    fn number(&self, id: i32) -> i32;
    fn float_number(&self, id: i32) -> f64;
    fn text(&self, id: i32) -> String;
    fn timer(&self, id: i32) -> i64;
    fn time(&self) -> i64;
    fn slider(&self, id: i32) -> f64;
    fn offset(&self, id: i32) -> SkinOffset;

    // Concrete accessors
    fn rate(&self) -> f64;
    fn exscore(&self) -> i32;
    fn rate_best(&self) -> f64;
    fn exscore_best(&self) -> i32;
    fn rate_rival(&self) -> f64;
    fn exscore_rival(&self) -> i32;
    fn volume_sys(&self) -> f32;
    fn volume_key(&self) -> f32;
    fn volume_bg(&self) -> f32;
    fn judge(&self, id: i32) -> i32;
    fn gauge(&self) -> f64;
    fn gauge_type(&self) -> i32;
    fn event_index(&self, id: i32) -> i32;

    // Writable state mutations
    fn set_timer(&mut self, id: i32, value: i64);
    fn set_volume_sys(&mut self, value: f32);
    fn set_volume_key(&mut self, value: f32);
    fn set_volume_bg(&mut self, value: f32);
    fn event_exec(&mut self, id: i32, args: &[i32]);

    // Audio control (matches Java MainStateAccessor)
    fn audio_play(&mut self, path: &str, volume: f32);
    fn audio_loop(&mut self, path: &str, volume: f32);
    fn audio_stop(&mut self, path: &str);
}

/// Stub provider that returns default values for all methods.
///
/// Matches the behavior of the existing Lua `main_state` stub:
/// - `number()` -> 0
/// - `option()` -> false
/// - `text()` -> ""
/// - `timer()` -> TIMER_OFF
/// - `float_number()` -> 0.0
/// - `slider()` -> 0.0
/// - All other numeric accessors -> 0 / 0.0
/// - Write operations are no-ops.
pub struct StubLuaStateProvider;

impl LuaStateProvider for StubLuaStateProvider {
    fn option(&self, _id: i32) -> bool {
        false
    }

    fn number(&self, _id: i32) -> i32 {
        0
    }

    fn float_number(&self, _id: i32) -> f64 {
        0.0
    }

    fn text(&self, _id: i32) -> String {
        String::new()
    }

    fn timer(&self, _id: i32) -> i64 {
        TIMER_OFF
    }

    fn time(&self) -> i64 {
        0
    }

    fn slider(&self, _id: i32) -> f64 {
        0.0
    }

    fn offset(&self, _id: i32) -> SkinOffset {
        SkinOffset::default()
    }

    fn rate(&self) -> f64 {
        0.0
    }

    fn exscore(&self) -> i32 {
        0
    }

    fn rate_best(&self) -> f64 {
        0.0
    }

    fn exscore_best(&self) -> i32 {
        0
    }

    fn rate_rival(&self) -> f64 {
        0.0
    }

    fn exscore_rival(&self) -> i32 {
        0
    }

    fn volume_sys(&self) -> f32 {
        0.0
    }

    fn volume_key(&self) -> f32 {
        0.0
    }

    fn volume_bg(&self) -> f32 {
        0.0
    }

    fn judge(&self, _id: i32) -> i32 {
        0
    }

    fn gauge(&self) -> f64 {
        0.0
    }

    fn gauge_type(&self) -> i32 {
        0
    }

    fn event_index(&self, _id: i32) -> i32 {
        0
    }

    fn set_timer(&mut self, _id: i32, _value: i64) {}

    fn set_volume_sys(&mut self, _value: f32) {}

    fn set_volume_key(&mut self, _value: f32) {}

    fn set_volume_bg(&mut self, _value: f32) {}

    fn event_exec(&mut self, _id: i32, _args: &[i32]) {}

    fn audio_play(&mut self, _path: &str, _volume: f32) {}

    fn audio_loop(&mut self, _path: &str, _volume: f32) {}

    fn audio_stop(&mut self, _path: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_returns_default_values() {
        let stub = StubLuaStateProvider;
        assert!(!stub.option(0));
        assert_eq!(stub.number(0), 0);
        assert_eq!(stub.float_number(0), 0.0);
        assert_eq!(stub.text(0), "");
        assert_eq!(stub.timer(0), TIMER_OFF);
        assert_eq!(stub.time(), 0);
        assert_eq!(stub.slider(0), 0.0);
        assert_eq!(stub.rate(), 0.0);
        assert_eq!(stub.exscore(), 0);
        assert_eq!(stub.volume_sys(), 0.0);
        assert_eq!(stub.judge(0), 0);
        assert_eq!(stub.gauge(), 0.0);
        assert_eq!(stub.gauge_type(), 0);
        assert_eq!(stub.event_index(0), 0);

        let off = stub.offset(0);
        assert_eq!(off.x, 0.0);
        assert_eq!(off.y, 0.0);
        assert_eq!(off.w, 0.0);
        assert_eq!(off.h, 0.0);
        assert_eq!(off.r, 0.0);
        assert_eq!(off.a, 0.0);
    }

    #[test]
    fn stub_write_operations_are_noop() {
        let mut stub = StubLuaStateProvider;
        // These should not panic
        stub.set_timer(0, 1000);
        stub.set_volume_sys(0.5);
        stub.set_volume_key(0.5);
        stub.set_volume_bg(0.5);
        stub.event_exec(0, &[1, 2]);
        stub.audio_play("test.wav", 0.8);
        stub.audio_loop("test.wav", 0.5);
        stub.audio_stop("test.wav");
    }
}

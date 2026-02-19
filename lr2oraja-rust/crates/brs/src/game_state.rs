// SharedGameState + GameStateProvider — bridges TimerManager to SkinStateProvider.
//
// SharedGameState holds a snapshot of the current game state that
// the SkinStateProvider reads from. A sync system updates it each frame.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use bevy::prelude::{Handle, Image};
use bms_config::Config;
use bms_render::draw::bar::BarScrollState;
use bms_render::state_provider::SkinStateProvider;
use bms_skin::property_id::{BooleanId, FloatId, IntegerId, StringId, TIMER_MAX, TimerId};
use bms_skin::skin_object::SkinOffset;

use crate::timer_manager::{TIMER_OFF, TimerManager};

/// Snapshot of game state readable by the skin renderer.
#[derive(Debug, Clone, Default)]
pub struct SharedGameState {
    /// Active timers: timer_id -> elapsed milliseconds.
    /// Absent entries mean the timer is OFF.
    pub timers: HashMap<i32, i64>,
    pub integers: HashMap<i32, i32>,
    pub floats: HashMap<i32, f32>,
    pub strings: HashMap<i32, String>,
    pub booleans: HashMap<i32, bool>,
    pub offsets: HashMap<i32, SkinOffset>,
    pub now_time_ms: i64,
    /// Bar scroll state for music select screen rendering.
    pub bar_scroll_state: Option<BarScrollState>,
    /// BPM change events as (time_us, bpm) pairs for BPM graph rendering.
    pub bpm_events: Vec<(i64, f64)>,
    /// Note distribution counts per time bucket for note distribution graph rendering.
    pub note_distribution: Vec<u32>,
    /// Current BGA base layer image handle.
    pub bga_image: Option<Handle<Image>>,
    /// Current BGA overlay layer image handle.
    pub layer_image: Option<Handle<Image>>,
    /// Current poor/miss layer image handle.
    pub poor_image: Option<Handle<Image>>,
    /// Whether the poor layer is currently active.
    pub poor_active: bool,
}

/// SkinStateProvider implementation backed by SharedGameState.
pub struct GameStateProvider {
    state: Arc<RwLock<SharedGameState>>,
}

impl GameStateProvider {
    pub fn new(state: Arc<RwLock<SharedGameState>>) -> Self {
        Self { state }
    }
}

impl SkinStateProvider for GameStateProvider {
    fn timer_value(&self, timer: TimerId) -> Option<i64> {
        let state = self.state.read();
        state.timers.get(&timer.0).copied()
    }

    fn integer_value(&self, id: IntegerId) -> i32 {
        let state = self.state.read();
        state.integers.get(&id.0).copied().unwrap_or(0)
    }

    fn has_integer_value(&self, id: IntegerId) -> bool {
        let state = self.state.read();
        state.integers.contains_key(&id.0)
    }

    fn float_value(&self, id: FloatId) -> f32 {
        let state = self.state.read();
        state.floats.get(&id.0).copied().unwrap_or(0.0)
    }

    fn has_float_value(&self, id: FloatId) -> bool {
        let state = self.state.read();
        state.floats.contains_key(&id.0)
    }

    fn string_value(&self, id: StringId) -> Option<String> {
        let state = self.state.read();
        state.strings.get(&id.0).cloned()
    }

    fn boolean_value(&self, id: BooleanId) -> bool {
        let state = self.state.read();
        let raw = state.booleans.get(&id.abs_id()).copied().unwrap_or(false);
        if id.is_negated() { !raw } else { raw }
    }

    fn has_boolean_value(&self, id: BooleanId) -> bool {
        let state = self.state.read();
        state.booleans.contains_key(&id.abs_id())
    }

    fn now_time_ms(&self) -> i64 {
        let state = self.state.read();
        state.now_time_ms
    }

    fn offset_value(&self, id: i32) -> SkinOffset {
        let state = self.state.read();
        state.offsets.get(&id).copied().unwrap_or_default()
    }

    fn bga_image(&self) -> Option<Handle<Image>> {
        let state = self.state.read();
        state.bga_image.clone()
    }

    fn layer_image(&self) -> Option<Handle<Image>> {
        let state = self.state.read();
        state.layer_image.clone()
    }

    fn poor_image(&self) -> Option<Handle<Image>> {
        let state = self.state.read();
        state.poor_image.clone()
    }

    fn is_poor_active(&self) -> bool {
        let state = self.state.read();
        state.poor_active
    }
}

/// Synchronizes TimerManager state into SharedGameState.
///
/// Called once per frame to update the shared state snapshot
/// that the renderer reads from.
pub fn sync_timer_state(timer: &TimerManager, state: &Arc<RwLock<SharedGameState>>) {
    let mut shared = state.write();
    shared.now_time_ms = timer.now_time();

    // Sync all standard timers
    shared.timers.clear();
    for id in 0..=TIMER_MAX {
        let val = timer.micro_timer(id);
        if val != TIMER_OFF {
            // Convert absolute microsecond time to elapsed milliseconds
            let elapsed_ms = (timer.now_micro_time() - val) / 1000;
            shared.timers.insert(id, elapsed_ms);
        }
    }
}

/// Synchronize common system properties (time, volume).
///
/// Called once per frame from state_sync_system.
pub fn sync_common_state(state: &mut SharedGameState, config: &Config) {
    use bms_skin::property_id::{
        NUMBER_BGM_VOLUME, NUMBER_CURRENT_FPS, NUMBER_KEY_VOLUME, NUMBER_MASTER_VOLUME,
        NUMBER_TIME_DAY, NUMBER_TIME_HOUR, NUMBER_TIME_MINUTE, NUMBER_TIME_MONTH,
        NUMBER_TIME_SECOND, NUMBER_TIME_YEAR,
    };

    // Current time (local)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simplified UTC calendar (adequate for skin display)
    let (year, month, day, hour, minute, second) = unix_to_calendar(now);
    state.integers.insert(NUMBER_TIME_YEAR, year);
    state.integers.insert(NUMBER_TIME_MONTH, month);
    state.integers.insert(NUMBER_TIME_DAY, day);
    state.integers.insert(NUMBER_TIME_HOUR, hour);
    state.integers.insert(NUMBER_TIME_MINUTE, minute);
    state.integers.insert(NUMBER_TIME_SECOND, second);

    // FPS (approximate; exact value requires frame timing)
    state.integers.insert(NUMBER_CURRENT_FPS, 60);

    // Volume settings from config (0-100 scale)
    let master = (config.audio.systemvolume * 100.0) as i32;
    let key = (config.audio.keyvolume * 100.0) as i32;
    let bgm = (config.audio.bgvolume * 100.0) as i32;
    state.integers.insert(NUMBER_MASTER_VOLUME, master);
    state.integers.insert(NUMBER_KEY_VOLUME, key);
    state.integers.insert(NUMBER_BGM_VOLUME, bgm);

    // Volume as float rates (0.0-1.0)
    state.floats.insert(
        bms_skin::property_id::RATE_MASTERVOLUME,
        config.audio.systemvolume,
    );
    state.floats.insert(
        bms_skin::property_id::RATE_KEYVOLUME,
        config.audio.keyvolume,
    );
    state
        .floats
        .insert(bms_skin::property_id::RATE_BGMVOLUME, config.audio.bgvolume);
}

/// Convert UNIX timestamp (seconds) to UTC calendar components.
fn unix_to_calendar(secs: u64) -> (i32, i32, i32, i32, i32, i32) {
    let days = (secs / 86400) as i64;
    let time_of_day = secs % 86400;
    let hour = (time_of_day / 3600) as i32;
    let minute = ((time_of_day % 3600) / 60) as i32;
    let second = (time_of_day % 60) as i32;

    // Days since 1970-01-01
    let mut y = 1970i64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days: [i64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    for md in &month_days {
        if remaining < *md {
            break;
        }
        remaining -= *md;
        m += 1;
    }
    (y as i32, m + 1, remaining as i32 + 1, hour, minute, second)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> Arc<RwLock<SharedGameState>> {
        Arc::new(RwLock::new(SharedGameState::default()))
    }

    #[test]
    fn timer_value_from_shared_state() {
        let state = make_state();
        state.write().timers.insert(1, 500);

        let provider = GameStateProvider::new(state);
        assert_eq!(provider.timer_value(TimerId(1)), Some(500));
        assert_eq!(provider.timer_value(TimerId(999)), None);
    }

    #[test]
    fn integer_value_from_shared_state() {
        let state = make_state();
        state.write().integers.insert(42, 123);

        let provider = GameStateProvider::new(state);
        assert_eq!(provider.integer_value(IntegerId(42)), 123);
        assert_eq!(provider.integer_value(IntegerId(99)), 0);
    }

    #[test]
    fn boolean_negation() {
        let state = make_state();
        state.write().booleans.insert(5, true);

        let provider = GameStateProvider::new(state);
        assert!(provider.boolean_value(BooleanId(5)));
        assert!(!provider.boolean_value(BooleanId(-5)));
    }

    #[test]
    fn sync_timer_state_populates_shared() {
        let mut tm = TimerManager::new();
        tm.set_now_micro_time(10_000);
        tm.set_timer_on(1); // TIMER_STARTINPUT = now_micro_time = 10_000

        tm.set_now_micro_time(15_000);

        let state = make_state();
        sync_timer_state(&tm, &state);

        let shared = state.read();
        assert_eq!(shared.now_time_ms, 15); // 15_000 / 1000
        // Timer 1 was set at 10_000, now is 15_000, elapsed = 5_000 us = 5 ms
        assert_eq!(shared.timers.get(&1), Some(&5));
        // Inactive timers should not be present
        assert!(!shared.timers.contains_key(&2));
    }

    #[test]
    fn offset_returns_default_when_missing() {
        let state = make_state();
        let provider = GameStateProvider::new(state);
        let offset = provider.offset_value(999);
        assert_eq!(offset.x, 0.0);
        assert_eq!(offset.y, 0.0);
    }

    #[test]
    fn sync_common_state_populates_time() {
        let state_arc = make_state();
        let config = bms_config::Config::default();
        let mut shared = state_arc.write();

        sync_common_state(&mut shared, &config);

        // Time values should be populated (non-zero year)
        assert!(
            *shared
                .integers
                .get(&bms_skin::property_id::NUMBER_TIME_YEAR)
                .unwrap()
                >= 2024
        );
        assert!(
            shared
                .integers
                .contains_key(&bms_skin::property_id::NUMBER_TIME_MONTH)
        );
        assert!(
            shared
                .integers
                .contains_key(&bms_skin::property_id::NUMBER_TIME_SECOND)
        );
    }

    #[test]
    fn sync_common_state_populates_volume() {
        let state_arc = make_state();
        let config = bms_config::Config::default();
        let mut shared = state_arc.write();

        sync_common_state(&mut shared, &config);

        assert!(
            shared
                .integers
                .contains_key(&bms_skin::property_id::NUMBER_MASTER_VOLUME)
        );
        assert!(
            shared
                .floats
                .contains_key(&bms_skin::property_id::RATE_MASTERVOLUME)
        );
    }

    #[test]
    fn bga_fields_default_to_none_and_false() {
        let shared = SharedGameState::default();
        assert!(shared.bga_image.is_none());
        assert!(shared.layer_image.is_none());
        assert!(shared.poor_image.is_none());
        assert!(!shared.poor_active);
    }

    #[test]
    fn bga_image_from_shared_state() {
        let state = make_state();
        let provider = GameStateProvider::new(state.clone());

        // Default: None
        assert!(provider.bga_image().is_none());
        assert!(provider.layer_image().is_none());
        assert!(provider.poor_image().is_none());
        assert!(!provider.is_poor_active());

        // Set poor_active
        state.write().poor_active = true;
        assert!(provider.is_poor_active());
    }

    #[test]
    fn unix_to_calendar_epoch() {
        let (y, m, d, h, min, s) = super::unix_to_calendar(0);
        assert_eq!((y, m, d, h, min, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn unix_to_calendar_known_date() {
        // 2024-01-15 11:30:45 UTC = 1705318245
        let (y, m, d, h, min, s) = super::unix_to_calendar(1705318245);
        assert_eq!((y, m, d, h, min, s), (2024, 1, 15, 11, 30, 45));
    }
}

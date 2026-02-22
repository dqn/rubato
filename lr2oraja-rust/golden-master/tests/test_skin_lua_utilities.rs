// Integration tests for Lua utility wiring in beatoraja-skin.
//
// Tests that MainStateAccessor, TimerUtility, and EventUtility
// correctly export functions to Lua and produce expected results.

use beatoraja_skin::lua::event_utility::{
    EventMinIntervalState, EventObserveTimerOffState, EventObserveTimerOnState,
    EventObserveTimerState, EventObserveTurnTrueState, TIMER_OFF_VALUE as EVENT_TIMER_OFF,
};
use beatoraja_skin::lua::main_state_accessor::TIMER_OFF_VALUE as MSA_TIMER_OFF;
use beatoraja_skin::lua::timer_utility::{
    PassiveTimerState, TIMER_OFF_VALUE as TIMER_OFF, TimerObserveBooleanState, is_timer_off,
    is_timer_on, now_timer,
};

// ===========================================================================
// Timer utility pure functions
// ===========================================================================

#[test]
fn timer_off_value_is_i64_min() {
    assert_eq!(TIMER_OFF, i64::MIN);
    assert_eq!(MSA_TIMER_OFF, i64::MIN);
    assert_eq!(EVENT_TIMER_OFF, i64::MIN);
}

#[test]
fn now_timer_returns_elapsed() {
    // Timer ON at 1000, current time 5000 -> elapsed 4000
    assert_eq!(now_timer(1000, 5000), 4000);
}

#[test]
fn now_timer_returns_zero_when_off() {
    assert_eq!(now_timer(TIMER_OFF, 5000), 0);
}

#[test]
fn is_timer_on_checks_value() {
    assert!(is_timer_on(0));
    assert!(is_timer_on(1000));
    assert!(is_timer_on(-1));
    assert!(!is_timer_on(TIMER_OFF));
}

#[test]
fn is_timer_off_checks_value() {
    assert!(is_timer_off(TIMER_OFF));
    assert!(!is_timer_off(0));
    assert!(!is_timer_off(1000));
}

// ===========================================================================
// TimerObserveBooleanState
// ===========================================================================

#[test]
fn timer_observe_boolean_starts_off() {
    let state = TimerObserveBooleanState::new();
    assert_eq!(state.timer_value, TIMER_OFF);
}

#[test]
fn timer_observe_boolean_turns_on() {
    let mut state = TimerObserveBooleanState::new();
    let result = state.update(true, 1000);
    assert_eq!(result, 1000); // Timer set to current time
}

#[test]
fn timer_observe_boolean_stays_on() {
    let mut state = TimerObserveBooleanState::new();
    state.update(true, 1000);
    let result = state.update(true, 2000);
    assert_eq!(result, 1000); // Timer stays at original time
}

#[test]
fn timer_observe_boolean_turns_off() {
    let mut state = TimerObserveBooleanState::new();
    state.update(true, 1000);
    let result = state.update(false, 2000);
    assert_eq!(result, TIMER_OFF);
}

// ===========================================================================
// PassiveTimerState
// ===========================================================================

#[test]
fn passive_timer_starts_off() {
    let state = PassiveTimerState::new();
    assert_eq!(state.get_timer(), TIMER_OFF);
}

#[test]
fn passive_timer_turn_on() {
    let mut state = PassiveTimerState::new();
    state.turn_on(5000);
    assert_eq!(state.get_timer(), 5000);
}

#[test]
fn passive_timer_turn_on_idempotent() {
    let mut state = PassiveTimerState::new();
    state.turn_on(5000);
    state.turn_on(8000); // Should not change
    assert_eq!(state.get_timer(), 5000);
}

#[test]
fn passive_timer_turn_on_reset() {
    let mut state = PassiveTimerState::new();
    state.turn_on(5000);
    state.turn_on_reset(8000); // Should reset
    assert_eq!(state.get_timer(), 8000);
}

#[test]
fn passive_timer_turn_off() {
    let mut state = PassiveTimerState::new();
    state.turn_on(5000);
    state.turn_off();
    assert_eq!(state.get_timer(), TIMER_OFF);
}

// ===========================================================================
// EventObserveTurnTrueState
// ===========================================================================

#[test]
fn observe_turn_true_starts_off() {
    let state = EventObserveTurnTrueState::new();
    assert!(!state.is_on);
}

#[test]
fn observe_turn_true_fires_on_transition() {
    let mut state = EventObserveTurnTrueState::new();
    assert!(state.update(true)); // false -> true: fire
}

#[test]
fn observe_turn_true_no_fire_when_already_on() {
    let mut state = EventObserveTurnTrueState::new();
    state.update(true);
    assert!(!state.update(true)); // true -> true: no fire
}

#[test]
fn observe_turn_true_no_fire_on_turn_off() {
    let mut state = EventObserveTurnTrueState::new();
    state.update(true);
    assert!(!state.update(false)); // true -> false: no fire
}

#[test]
fn observe_turn_true_fires_again_after_off_on() {
    let mut state = EventObserveTurnTrueState::new();
    state.update(true);
    state.update(false);
    assert!(state.update(true)); // off -> on: fire again
}

// ===========================================================================
// EventObserveTimerState
// ===========================================================================

#[test]
fn observe_timer_fires_on_new_value() {
    let mut state = EventObserveTimerState::new();
    assert!(state.update(1000)); // OFF -> 1000: fire
}

#[test]
fn observe_timer_no_fire_when_same_value() {
    let mut state = EventObserveTimerState::new();
    state.update(1000);
    assert!(!state.update(1000)); // 1000 -> 1000: no fire
}

#[test]
fn observe_timer_fires_on_changed_value() {
    let mut state = EventObserveTimerState::new();
    state.update(1000);
    assert!(state.update(2000)); // 1000 -> 2000: fire
}

#[test]
fn observe_timer_no_fire_when_off() {
    let mut state = EventObserveTimerState::new();
    assert!(!state.update(EVENT_TIMER_OFF)); // OFF -> OFF: no fire
}

// ===========================================================================
// EventObserveTimerOnState
// ===========================================================================

#[test]
fn observe_timer_on_fires_when_turns_on() {
    let mut state = EventObserveTimerOnState::new();
    assert!(state.update(1000)); // OFF -> ON: fire
}

#[test]
fn observe_timer_on_no_fire_when_stays_on() {
    let mut state = EventObserveTimerOnState::new();
    state.update(1000);
    assert!(!state.update(2000)); // ON -> ON: no fire (still on)
}

#[test]
fn observe_timer_on_no_fire_when_turns_off() {
    let mut state = EventObserveTimerOnState::new();
    state.update(1000);
    assert!(!state.update(EVENT_TIMER_OFF)); // ON -> OFF: no fire (wrong direction)
}

// ===========================================================================
// EventObserveTimerOffState
// ===========================================================================

#[test]
fn observe_timer_off_fires_on_initial_off() {
    let mut state = EventObserveTimerOffState::new();
    // Initial is_off=false, timer is OFF -> transition detected -> fire
    assert!(state.update(EVENT_TIMER_OFF));
}

#[test]
fn observe_timer_off_fires_when_turns_off() {
    let mut state = EventObserveTimerOffState::new();
    state.update(1000); // ON
    assert!(state.update(EVENT_TIMER_OFF)); // ON -> OFF: fire
}

#[test]
fn observe_timer_off_no_fire_when_stays_off() {
    let mut state = EventObserveTimerOffState::new();
    state.update(1000);
    state.update(EVENT_TIMER_OFF);
    assert!(!state.update(EVENT_TIMER_OFF)); // OFF -> OFF: no fire
}

// ===========================================================================
// EventMinIntervalState
// ===========================================================================

#[test]
fn min_interval_fires_first_time() {
    let mut state = EventMinIntervalState::new();
    assert!(state.update(100, 0)); // First call always fires
}

#[test]
fn min_interval_throttles() {
    let mut state = EventMinIntervalState::new();
    state.update(100, 0); // fires at t=0
    assert!(!state.update(100, 50_000)); // 50ms < 100ms: throttled
}

#[test]
fn min_interval_fires_after_interval() {
    let mut state = EventMinIntervalState::new();
    state.update(100, 0); // fires at t=0
    assert!(state.update(100, 100_000)); // 100ms >= 100ms: fires
}

// ===========================================================================
// SkinObject enum type names
// ===========================================================================

#[test]
fn skin_object_type_names() {
    use beatoraja_skin::skin::SkinObject;
    use beatoraja_skin::skin_image::SkinImage;

    let obj = SkinObject::Image(SkinImage::new_with_image_id(0));
    assert_eq!(obj.get_type_name(), "Image");
}

// ===========================================================================
// SkinObject is_slider
// ===========================================================================

#[test]
fn skin_object_is_slider() {
    use beatoraja_skin::skin::SkinObject;
    use beatoraja_skin::skin_image::SkinImage;

    let img = SkinObject::Image(SkinImage::new_with_image_id(0));
    assert!(!img.is_slider());
}

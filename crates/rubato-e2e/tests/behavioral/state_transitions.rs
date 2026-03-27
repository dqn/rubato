use rubato_e2e::E2eHarness;

#[test]
fn initial_state_is_none_without_factory() {
    // Without a StateCreator, no state is created during construction.
    // MainController starts with current = None.
    let harness = E2eHarness::new();
    assert!(
        harness.controller().current_state().is_none(),
        "current state should be None without a state factory"
    );
    assert!(
        harness.controller().current_state_type().is_none(),
        "current state type should be None without a state factory"
    );
}

#[test]
fn controller_has_timer_manager() {
    let harness = E2eHarness::new();
    let timer = harness.controller().timer();
    // Timer should be frozen at 0 by the harness
    assert!(timer.frozen, "timer should be frozen by the harness");
    assert_eq!(
        timer.now_micro_time(),
        0,
        "timer should start at 0 microseconds"
    );
}

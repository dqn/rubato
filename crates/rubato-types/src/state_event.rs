use crate::main_state_type::MainStateType;

/// Observable state machine events for E2E testing.
///
/// Emitted by MainController during state transitions, lifecycle events,
/// and data handoffs. Collected via an optional `Arc<Mutex<Vec<StateEvent>>>`
/// event log that test harnesses can inject.
#[derive(Debug, Clone, PartialEq)]
pub enum StateEvent {
    /// Emitted at the start of a state transition, before create().
    TransitionStart {
        from: Option<MainStateType>,
        to: MainStateType,
    },
    /// Emitted after the new state is fully prepared and set as current.
    TransitionComplete { state: MainStateType },
    /// Emitted after new_state.create() completes.
    StateCreated { state: MainStateType },
    /// Emitted when the old state's shutdown() is called.
    StateShutdown { state: MainStateType },
    /// Emitted when a ScoreHandoff is applied to PlayerResource.
    ScoreHandoffApplied {
        exscore: i32,
        max_combo: i32,
        gauge: f64,
    },
    /// Emitted after outbox sounds and state changes are drained in render().
    OutboxDrained { sounds: usize, state_change: bool },
}

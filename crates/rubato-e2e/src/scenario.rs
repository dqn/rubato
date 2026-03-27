//! Fluent scenario builder for E2E tests.
//!
//! Allows writing concise, declarative test scripts using a builder pattern:
//!
//! ```ignore
//! E2eScenario::new()
//!     .start_state(MainStateType::Play)
//!     .render_frames(30)
//!     .assert_state(MainStateType::Play)
//!     .run();
//! ```

use crate::E2eHarness;
use rubato_game::core::main_controller::StateCreator;
use rubato_game::state_factory::LauncherStateFactory;
use rubato_types::main_state_type::MainStateType;

/// A step in the scenario execution pipeline.
enum ScenarioStep {
    RenderFrames(usize),
    ChangeState(MainStateType),
    AssertState(MainStateType),
    AssertGaugeBetween(f32, f32),
    AssertAudioNonEmpty,
    AssertNoPanicsAfterFrames(usize),
    Custom(Box<dyn FnOnce(&mut E2eHarness)>),
}

/// Fluent builder for E2E test scenarios.
///
/// Build a sequence of steps (state changes, rendering, assertions) and
/// execute them against an `E2eHarness`.
pub struct E2eScenario {
    initial_state: Option<MainStateType>,
    state_factory: Option<StateCreator>,
    steps: Vec<ScenarioStep>,
}

impl E2eScenario {
    /// Create a new empty scenario. By default, uses `LauncherStateFactory`.
    pub fn new() -> Self {
        Self {
            initial_state: None,
            state_factory: None,
            steps: Vec::new(),
        }
    }

    /// Set the initial state to transition to at the start.
    pub fn start_state(mut self, state: MainStateType) -> Self {
        self.initial_state = Some(state);
        self
    }

    /// Override the state factory (default: `LauncherStateFactory`).
    pub fn with_state_factory(mut self, factory: StateCreator) -> Self {
        self.state_factory = Some(factory);
        self
    }

    /// Add a step to render `n` frames.
    pub fn render_frames(mut self, n: usize) -> Self {
        self.steps.push(ScenarioStep::RenderFrames(n));
        self
    }

    /// Add a step to change state.
    pub fn change_state(mut self, state: MainStateType) -> Self {
        self.steps.push(ScenarioStep::ChangeState(state));
        self
    }

    /// Add a step to assert the current state type.
    pub fn assert_state(mut self, state: MainStateType) -> Self {
        self.steps.push(ScenarioStep::AssertState(state));
        self
    }

    /// Add a step to assert the gauge value is within `[min, max]`.
    pub fn assert_gauge_between(mut self, min: f32, max: f32) -> Self {
        self.steps.push(ScenarioStep::AssertGaugeBetween(min, max));
        self
    }

    /// Add a step to assert that at least one audio event was recorded.
    pub fn assert_audio_non_empty(mut self) -> Self {
        self.steps.push(ScenarioStep::AssertAudioNonEmpty);
        self
    }

    /// Add a step to render `n` frames asserting no panics.
    pub fn assert_no_panics_after_frames(mut self, n: usize) -> Self {
        self.steps.push(ScenarioStep::AssertNoPanicsAfterFrames(n));
        self
    }

    /// Add a custom step with arbitrary access to the harness.
    pub fn then(mut self, f: impl FnOnce(&mut E2eHarness) + 'static) -> Self {
        self.steps.push(ScenarioStep::Custom(Box::new(f)));
        self
    }

    /// Build the harness and execute all steps in order.
    pub fn run(self) {
        let factory: StateCreator = self
            .state_factory
            .unwrap_or_else(|| LauncherStateFactory::new().into_creator());

        let mut harness = E2eHarness::new().with_state_factory(factory);

        if let Some(state) = self.initial_state {
            harness.change_state(state);
        }

        for step in self.steps {
            match step {
                ScenarioStep::RenderFrames(n) => {
                    harness.render_frames(n);
                }
                ScenarioStep::ChangeState(state) => {
                    // Decide/Result/CourseResult require a PlayerResource
                    if matches!(
                        state,
                        MainStateType::Decide | MainStateType::Result | MainStateType::CourseResult
                    ) {
                        harness.ensure_player_resource();
                    }
                    harness.change_state(state);
                }
                ScenarioStep::AssertState(state) => {
                    harness.assert_state(state);
                }
                ScenarioStep::AssertGaugeBetween(min, max) => {
                    harness.assert_gauge_between(min, max);
                }
                ScenarioStep::AssertAudioNonEmpty => {
                    harness.assert_audio_event_count_at_least(1);
                }
                ScenarioStep::AssertNoPanicsAfterFrames(n) => {
                    harness.assert_no_panics_after_frames(n);
                }
                ScenarioStep::Custom(f) => {
                    f(&mut harness);
                }
            }
        }
    }
}

impl Default for E2eScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_default_runs_empty() {
        // An empty scenario should not panic.
        E2eScenario::new().run();
    }

    #[test]
    fn scenario_with_start_state() {
        E2eScenario::new()
            .start_state(MainStateType::Play)
            .assert_state(MainStateType::Play)
            .run();
    }

    #[test]
    fn scenario_then_custom_step() {
        E2eScenario::new()
            .start_state(MainStateType::MusicSelect)
            .then(|h| {
                assert_eq!(h.current_state_type(), Some(MainStateType::MusicSelect));
            })
            .run();
    }
}

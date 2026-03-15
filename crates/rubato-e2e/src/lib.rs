//! E2E test harness for behavioral testing of the rubato application.
//!
//! Provides `E2eHarness` which wires up a `MainController` with a
//! `RecordingAudioDriver` and deterministic (frozen) timing, suitable
//! for integration tests that exercise the full state machine without
//! requiring GPU or audio hardware.

pub mod harness;

pub use harness::{E2eHarness, FRAME_DURATION_US};

// Re-export commonly needed types for E2E tests
pub use rubato_audio::recording_audio_driver::AudioEvent;
pub use rubato_core::main_controller::StateFactory;
pub use rubato_types::main_state_type::MainStateType;

//! Input device abstraction for keyboard, controller, and MIDI.

pub mod bm_controller_input_processor;
pub mod bms_player_input_device;
pub mod bms_player_input_processor;
pub mod controller;
pub mod gdx_compat;
pub mod key_command;
pub mod key_input_log;
pub mod keyboard_input_processor;
pub mod keys;
pub mod midi_input_processor;
pub mod mouse_scratch_input;
pub mod winit_input_bridge;

// Re-exports from rubato_types (previously in stubs.rs)
pub use rubato_types::config::Config;
pub use rubato_types::play_mode_config::{
    ANALOG_SCRATCH_VER_1, ANALOG_SCRATCH_VER_2, ControllerConfig, KeyboardConfig,
    MOUSE_SCRATCH_VER_1, MOUSE_SCRATCH_VER_2, MidiConfig, MidiInput, MidiInputType,
    MouseScratchConfig, PlayModeConfig,
};
pub use rubato_types::player_config::PlayerConfig;
pub use rubato_types::resolution::Resolution;

// Re-exports from gdx_compat (previously in stubs.rs)
pub use gdx_compat::{GdxGraphics, GdxInput};

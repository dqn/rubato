//! Input device abstraction for keyboard, game controllers, and MIDI.
//!
//! Provides [`input_processor::InputProcessor`] for unified input handling,
//! [`keyboard::KeyboardBackend`] and [`controller::ControllerBackend`] for device
//! polling, [`autoplay::AutoplayProcessor`] for automated input generation,
//! and [`key_command::KeyCommand`] / [`key_state::KeyState`] for mapping raw
//! inputs to game actions. Supports analog scratch via [`analog_scratch`].

pub mod analog_scratch;
pub mod autoplay;
pub mod control_keys;
pub mod controller;
pub mod controller_keys;
pub mod device;
pub mod input_processor;
pub mod key_command;
pub mod key_state;
pub mod keyboard;
pub mod midi;
pub mod mouse_scratch;

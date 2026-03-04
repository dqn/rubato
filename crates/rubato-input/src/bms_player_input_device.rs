//! BMSPlayerInputDevice - input device abstract class / trait
//!
//! Translated from: bms.player.beatoraja.input.BMSPlayerInputDevice

/// Input device type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceType {
    Keyboard,
    BmController,
    Midi,
}

/// Input device trait
///
/// Java abstract class BMSPlayerInputDevice -> Rust trait
pub trait BMSPlayerInputDevice {
    /// Get the device type
    fn device_type(&self) -> DeviceType;

    /// Clear the input state of the device
    fn clear(&mut self);
}

/// Rust equivalent of beatoraja.input.BMSPlayerInputDevice.Type
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Type {
    BM_CONTROLLER,
    KEYBOARD,
    MIDI,
    MOUSE,
}

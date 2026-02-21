// Stubs for external dependencies not yet available in the beatoraja-ir crate

pub use beatoraja_song::song_data::SongData;

/// Stub for beatoraja.modmenu.ImGuiNotify
pub struct ImGuiNotify;

impl ImGuiNotify {
    pub fn error(msg: &str) {
        log::error!("ImGuiNotify: {}", msg);
    }

    pub fn warning(msg: &str) {
        log::warn!("ImGuiNotify: {}", msg);
    }
}

/// Stub for beatoraja.pattern.Random (enum for random option types)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Random {
    IDENTITY,
    MIRROR,
    RANDOM,
}

/// Stub for beatoraja.pattern.LR2Random
pub struct LR2Random {
    state: u32,
}

impl LR2Random {
    pub fn new(seed: i32) -> Self {
        Self { state: seed as u32 }
    }

    pub fn next_int(&mut self, bound: i32) -> i32 {
        // Simplified stub - real MT implementation is in beatoraja-pattern
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.state >> 16) as i32).abs() % bound
    }
}

pub use bms_model::bms_decoder::convert_hex_string;

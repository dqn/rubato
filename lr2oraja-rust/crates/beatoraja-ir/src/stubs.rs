// Stubs for external dependencies not yet available in the beatoraja-ir crate

pub use beatoraja_song::song_data::SongData;

/// Stub for MainController
pub struct MainController;

impl MainController {
    pub fn get_ir_status(&self) -> &[IRStatusStub] {
        &[]
    }

    pub fn get_player_config(&self) -> &PlayerConfigStub {
        todo!("MainController.get_player_config stub")
    }
}

/// Stub for IRStatus
pub struct IRStatusStub {
    pub connection: Box<dyn super::ir_connection::IRConnection>,
}

/// Stub for PlayerConfig (subset)
pub struct PlayerConfigStub {
    pub lnmode: i32,
}

impl PlayerConfigStub {
    pub fn get_lnmode(&self) -> i32 {
        self.lnmode
    }
}

/// Stub for MainState trait (subset needed by RankingData)
pub trait MainStateAccessor {
    fn get_main_controller(&self) -> &MainController;
    fn get_score_data_property(&self) -> &ScoreDataPropertyStub;
}

/// Stub for ScoreDataProperty (subset)
pub struct ScoreDataPropertyStub {
    pub score: Option<beatoraja_core::score_data::ScoreData>,
}

impl ScoreDataPropertyStub {
    pub fn get_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
        self.score.as_ref()
    }
}

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
        // LR2-specific MT19937 seeding
        let _state = seed as u32;
        // Simple LCG-based stub; real implementation in beatoraja-pattern
        Self { state: seed as u32 }
    }

    pub fn next_int(&mut self, bound: i32) -> i32 {
        // Simplified stub - real MT implementation is in beatoraja-pattern
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.state >> 16) as i32).abs() % bound
    }
}

/// Stub for BMSDecoder.convertHexString
pub fn convert_hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

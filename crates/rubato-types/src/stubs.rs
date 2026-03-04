// Phase 5+ stubs — types from downstream crates that beatoraja-core cannot import
// due to circular dependency constraints. These will be replaced if/when the
// dependency graph is restructured (e.g., extracting shared types into a common crate).

// ---------------------------------------------------------------------------
// beatoraja-play stubs
// ---------------------------------------------------------------------------

// GrooveGauge moved to beatoraja-types/src/groove_gauge.rs (Phase 15b)
pub use crate::groove_gauge::GrooveGauge;

// JudgeAlgorithm moved to beatoraja-types/src/judge_algorithm.rs (Phase 30a)
pub use crate::judge_algorithm::JudgeAlgorithm;

// BMSPlayerRule moved to beatoraja-types/src/bms_player_rule.rs (Phase 30a)
pub use crate::bms_player_rule::BMSPlayerRule;

// ---------------------------------------------------------------------------
// beatoraja-skin: SkinType moved to beatoraja-types/src/skin_type.rs
// ---------------------------------------------------------------------------

pub use crate::skin_type::SkinType;

// ---------------------------------------------------------------------------
// beatoraja-select stubs
// ---------------------------------------------------------------------------

// BarSorter/BarSorterEntry moved to beatoraja-types/src/bar_sorter.rs (Phase 30a)
pub use crate::bar_sorter::{BarSorter, BarSorterEntry};

// ---------------------------------------------------------------------------
// beatoraja-pattern stubs
// ---------------------------------------------------------------------------

// Modifier Mode enums moved to dedicated files (Phase 30a)
pub use crate::long_note_modifier;
pub use crate::mine_note_modifier;
pub use crate::scroll_speed_modifier;

// ---------------------------------------------------------------------------
// beatoraja-ir stubs
// ---------------------------------------------------------------------------

/// Stub for beatoraja.ir.IRConnectionManager
pub struct IRConnectionManager;

impl IRConnectionManager {
    pub fn get_all_available_ir_connection_name() -> Vec<String> {
        vec![]
    }

    pub fn get_ir_connection_class(_name: &str) -> Option<()> {
        Some(())
    }
}

// ---------------------------------------------------------------------------
// beatoraja-input stubs (incompatible field layout with beatoraja-input crate)
// ---------------------------------------------------------------------------

/// Stub for beatoraja.input.BMSPlayerInputDevice.Type
pub mod bms_player_input_device {
    #[allow(non_camel_case_types)]
    #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum Type {
        BM_CONTROLLER,
        KEYBOARD,
        MIDI,
        MOUSE,
    }
}

// KeyInputLog moved to beatoraja-types/src/key_input_log.rs (Phase 30b)
pub use crate::key_input_log::KeyInputLog;

// ---------------------------------------------------------------------------
// beatoraja-pattern stubs (incompatible field layout with beatoraja-pattern crate)
// ---------------------------------------------------------------------------

// PatternModifyLog moved to beatoraja-types/src/pattern_modify_log.rs (Phase 30b)
pub use crate::pattern_modify_log::PatternModifyLog;

// ---------------------------------------------------------------------------
// beatoraja-song stubs — SongData moved to beatoraja-types/src/song_data.rs
// ---------------------------------------------------------------------------

pub use crate::song_data::SongData;

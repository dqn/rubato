pub(super) static GAUGE: &[&str] = &[
    "ASSIST EASY",
    "EASY",
    "NORMAL",
    "HARD",
    "EX-HARD",
    "HAZARD",
    "GRADE",
    "EX GRADE",
    "EXHARD GRADE",
];
pub(super) static RANDOM: &[&str] = &[
    "NORMAL",
    "MIRROR",
    "RANDOM",
    "R-RANDOM",
    "S-RANDOM",
    "SPIRAL",
    "H-RANDOM",
    "ALL-SCR",
    "RANDOM-EX",
    "S-RANDOM-EX",
];
pub(super) static DPRANDOM: &[&str] = &["NORMAL", "FLIP"];
pub(super) static GRAPHTYPESTR: &[&str] = &["NOTETYPE", "JUDGE", "EARLYLATE"];

// Re-export shared practice draw command types from rubato-types (canonical location).
pub use rubato_types::practice_draw_command::{PracticeColor, PracticeDrawCommand};

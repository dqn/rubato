//! Replay recording, ghost data, and key input logging for BMS plays.
//!
//! Provides [`ReplayData`] for serializing and deserializing full play replays,
//! [`LR2GhostData`] for LR2-compatible ghost score curves, [`KeyInputLog`] for
//! per-frame key state recording, and [`LR2Random`] for LR2 random seed handling.
//! Used by the play and result states to save, load, and compare play sessions.

pub mod key_input_log;
pub mod lr2_ghost_data;
pub mod lr2_random;
pub mod replay_data;

pub use key_input_log::KeyInputLog;
pub use lr2_ghost_data::LR2GhostData;
pub use lr2_random::LR2Random;
pub use replay_data::ReplayData;

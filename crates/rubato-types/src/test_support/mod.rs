//! Test support utilities for rubato crates.
//!
//! Available when `cfg(test)` or `feature = "test-support"` is enabled.
//! Provides shared test doubles (TestSongDb, TestPlayerResource) and
//! data builders to replace per-crate MockSongDb / MockPlayerResource stubs.

pub mod builders;
pub mod current_dir_guard;
pub mod test_player_resource;

pub use builders::*;
pub use current_dir_guard::CurrentDirGuard;
pub use test_player_resource::{TestPlayerResource, TestPlayerResourceLog};

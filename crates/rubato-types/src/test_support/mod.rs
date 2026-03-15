//! Test support utilities for rubato crates.
//!
//! Available when `cfg(test)` or `feature = "test-support"` is enabled.
//! Provides shared test doubles (TestSongDb, TestPlayerResource) and
//! data builders to replace per-crate MockSongDb / MockPlayerResource stubs.

pub mod builders;
pub mod test_player_resource;
pub mod test_song_db;

pub use builders::*;
pub use test_player_resource::{TestPlayerResource, TestPlayerResourceLog};
pub use test_song_db::TestSongDb;

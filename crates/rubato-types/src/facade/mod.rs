//! Facade modules that group related types by domain.
//!
//! These provide convenience re-exports so downstream crates can import
//! grouped types via e.g. `rubato_types::facade::score::*` instead of
//! importing from individual modules. The original module paths remain
//! fully supported for backward compatibility.

pub mod config;
pub mod ids;
pub mod lifecycle;
pub mod play;
pub mod score;
pub mod skin;
pub mod song;
pub mod state;

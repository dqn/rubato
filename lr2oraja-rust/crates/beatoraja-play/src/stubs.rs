// Phase 11: stubs replaced with real imports where possible.
// Remaining stubs are for types from crates that beatoraja-play cannot depend on
// (beatoraja-skin circular dep, LibGDX rendering types).

// Re-export from beatoraja-core
pub use beatoraja_core::main_controller::MainController;

/// Stub for Texture (LibGDX) - cannot import from beatoraja-skin (circular dep)
pub struct Texture;

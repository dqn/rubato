// MainController wrapper
pub mod main_controller_ref;

// Re-exports
pub use rubato_input::keyboard_input_processor::ControlKeys;
pub use rubato_types::main_controller_access::{MainControllerAccess, NullMainController};
pub use rubato_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};

// Decide screen modules
pub mod music_decide;
pub mod music_decide_skin;

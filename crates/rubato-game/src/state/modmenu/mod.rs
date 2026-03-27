// beatoraja-modmenu: In-game mod menu (egui)
// Translated from Java: bms.player.beatoraja.modmenu package (15 files)

// Types
pub mod clipboard_wrapper;
pub mod imgui_surrogates;
pub mod modmenu_skin;

// Re-exports
pub use crate::core::config::Config;
pub use crate::core::play_config::PlayConfig;
pub use crate::core::score_data::ScoreData;
pub use crate::core::version::{self, Version};
pub use rubato_skin::reexports::Rectangle;
pub use rubato_skin::skin_header::{
    CustomCategory, CustomFile, CustomItemEnum as CustomCategoryItem, CustomOffset, CustomOption,
    SkinHeader, TYPE_LR2SKIN,
};
pub use rubato_skin::skin_property::OPTION_RANDOM_VALUE;
pub use rubato_types::main_controller_access::{MainControllerAccess, NullMainController};
pub use rubato_types::player_config::PlayerConfig;
pub use rubato_types::player_config::read_all_player_id;
pub use rubato_types::skin_config::{
    SkinConfig, SkinFilePath, SkinOffset, SkinOption, SkinProperty,
};
pub use rubato_types::skin_type::SkinType;
pub use rubato_types::song_data::SongData;
pub use rubato_types::song_selection_access::SongSelectionAccess;
pub use rubato_types::validatable::Validatable;

// Skin loaders
pub use rubato_skin::json::json_skin_loader::JSONSkinLoader;
pub use rubato_skin::lr2::lr2_skin_header_loader::LR2SkinHeaderLoader;
pub use rubato_skin::lua::lua_skin_loader::LuaSkinLoader;

/// Type alias for backward compatibility -- callers use `MainController`.
/// Accepted trade-off: the modmenu (egui launcher/settings) uses NullMainController,
/// so controller-backed skin menu actions (e.g., freeze timers) are no-ops.
/// Wiring a real MainController here requires restructuring the egui integration.
pub type MainController = NullMainController;

// Re-exports for moved types
pub use clipboard_wrapper::Clipboard;
pub use imgui_surrogates::{ImBoolean, ImFloat};
pub use modmenu_skin::{Skin, SkinObject, SkinObjectDestination};

pub mod download_task_menu;
pub mod download_task_state;
pub mod font_awesome_icons;
pub mod freq_trainer_menu;
pub mod imgui_notify;
pub mod imgui_renderer;
pub mod judge_trainer;
pub mod judge_trainer_menu;
pub mod misc_setting_menu;
pub mod performance_monitor;
pub mod random_trainer;
pub mod random_trainer_menu;
pub mod skin_menu;
pub mod skin_widget_manager;
pub mod song_manager_menu;

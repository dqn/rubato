// Stubs for external dependencies not yet available in the Rust port.
// These will be replaced with real implementations in future phases.

// =========================================================================
// Real type re-exports (replaced from stubs)
// =========================================================================

pub use beatoraja_core::config::Config;
pub use beatoraja_core::play_config::PlayConfig;
pub use beatoraja_core::score_data::ScoreData;
pub use beatoraja_core::version::{self, Version};
pub use beatoraja_types::player_config::PlayerConfig;
pub use beatoraja_types::player_config::read_all_player_id;
pub use beatoraja_types::skin_config::{
    SkinConfig, SkinFilePath, SkinOffset, SkinOption, SkinProperty,
};
pub use beatoraja_types::validatable::Validatable;

// =========================================================================
// MainController — replaced with NullMainController from beatoraja-types
// =========================================================================

pub use beatoraja_types::main_controller_access::{MainControllerAccess, NullMainController};

/// Type alias for backward compatibility — callers use `MainController`.
pub type MainController = NullMainController;

// MainState trait: removed (unused in modmenu — Phase 25d-2)

// Version is re-exported from beatoraja_core at the top of this file.

// =========================================================================
// Skin types — real type re-exports from beatoraja-skin
// =========================================================================

// SkinType moved to beatoraja-types (Phase 15b)
pub use beatoraja_types::skin_type::SkinType;

pub use beatoraja_skin::skin_header::{
    CustomCategory, CustomFile, CustomItemEnum as CustomCategoryItem, CustomOffset, CustomOption,
    SkinHeader, TYPE_LR2SKIN,
};
pub use beatoraja_skin::skin_property::OPTION_RANDOM_VALUE;

// Skin loaders — real type re-exports
pub use beatoraja_skin::json::json_skin_loader::JSONSkinLoader;
pub use beatoraja_skin::lr2::lr2_skin_header_loader::LR2SkinHeaderLoader;
pub use beatoraja_skin::lua::lua_skin_loader::LuaSkinLoader;

// =========================================================================
// Skin stub
// =========================================================================

#[derive(Clone, Default)]
pub struct Skin {
    pub header: SkinHeader,
    objects: Vec<SkinObject>,
}

impl Skin {
    pub fn get_all_skin_objects(&self) -> &[SkinObject] {
        &self.objects
    }
}

// =========================================================================
// SkinObject stub
// =========================================================================

#[derive(Clone, Debug, Default)]
pub struct SkinObject {
    pub name: Option<String>,
    pub draw: bool,
    pub visible: bool,
    pub destinations: Vec<SkinObjectDestination>,
}

impl SkinObject {
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn get_all_destination(&self) -> &[SkinObjectDestination] {
        &self.destinations
    }
}

#[derive(Clone, Debug, Default)]
pub struct SkinObjectDestination {
    pub time: i32,
    pub region: Rectangle,
    pub color: Option<[f32; 4]>,
    pub angle: f32,
    pub alpha: f32,
}

// Rectangle — re-exported from beatoraja-skin (via beatoraja-render, Phase 25d-2)
pub use beatoraja_skin::stubs::Rectangle;

// =========================================================================
// MusicSelector — replaced with SongSelectionAccess trait
// =========================================================================

pub use beatoraja_types::song_selection_access::SongSelectionAccess;

// =========================================================================
// SongData — real type from beatoraja-types
// =========================================================================

// SongData — re-exported from beatoraja-types (Phase 25d-2)
pub use beatoraja_types::song_data::SongData;

// ScoreData is re-exported from beatoraja_core at the top of this file.

// =========================================================================
// ImGui surrogate types used by SkinWidgetManager
// =========================================================================

/// Surrogate for ImGui ImFloat — a plain f32 wrapper used in static Mutex statics.
pub struct ImFloat {
    pub value: f32,
}

/// Surrogate for ImGui ImBoolean — a plain bool wrapper used in static Mutex statics.
pub struct ImBoolean {
    pub value: bool,
}

// =========================================================================
// Clipboard stub — wraps arboard
// =========================================================================

/// Thin wrapper around arboard::Clipboard that silences errors in non-critical paths.
pub struct Clipboard {
    inner: Option<arboard::Clipboard>,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            inner: arboard::Clipboard::new().ok(),
        }
    }

    pub fn set_contents(&self, text: &str) {
        if let Some(ref mut cb) = self
            .inner
            .as_ref()
            .and_then(|_| arboard::Clipboard::new().ok())
        {
            if let Err(e) = cb.set_text(text) {
                log::warn!("Clipboard::set_contents failed: {}", e);
            }
        } else {
            log::warn!("Clipboard::set_contents: clipboard unavailable");
        }
    }
}

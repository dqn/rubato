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

// =========================================================================
// MainState trait stub
// =========================================================================

pub trait MainState {
    fn get_skin(&self) -> &Skin;
    fn set_skin(&mut self, skin: Skin);
    fn as_any(&self) -> &dyn std::any::Any;
}

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

    pub fn prepare(&self, _state: &dyn MainState) {
        log::warn!("not yet implemented: Skin::prepare - rendering dependency");
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

#[derive(Clone, Debug, Default)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

// =========================================================================
// MusicSelector stub
// =========================================================================

pub struct MusicSelector;

impl MusicSelector {
    pub fn get_selected_bar(&self) -> &dyn Bar {
        log::warn!("not yet implemented: MusicSelector::get_selected_bar - select dependency");
        static DEFAULT_BAR: DefaultBar = DefaultBar;
        &DEFAULT_BAR
    }

    pub fn get_reverse_lookup_data(&self) -> Vec<String> {
        log::warn!(
            "not yet implemented: MusicSelector::get_reverse_lookup_data - select dependency"
        );
        Vec::new()
    }
}

pub trait Bar {
    fn as_any(&self) -> &dyn std::any::Any;
}

struct DefaultBar;
impl Bar for DefaultBar {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct SongBar {
    pub song_data: Option<SongData>,
    pub score: Option<ScoreData>,
}

impl SongBar {
    pub fn get_song_data(&self) -> Option<&SongData> {
        self.song_data.as_ref()
    }

    pub fn get_score(&self) -> Option<&ScoreData> {
        self.score.as_ref()
    }
}

// =========================================================================
// SongData — real type from beatoraja-types
// =========================================================================

pub use beatoraja_core::stubs::SongData;

// ScoreData is re-exported from beatoraja_core at the top of this file.

// ImBoolean/ImInt/ImFloat: removed (replaced with plain bool/i32/f32 in Mutex — Phase 18e-6)
// LWJGL3/LibGDX stubs: InputProcessor, Lwjgl3ControllerManager, Controller removed (unused — Phase 18e-5)

// Clipboard: removed (replaced with direct arboard calls — Phase 18e-6)

// External dependency stubs for beatoraja-external crate

use beatoraja_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};

// Real implementations moved to dedicated modules (Phase 25a):
//   Pixmap, GdxGraphics, BufferUtils, PixmapIO → pixmap_io.rs
//   ClipboardHelper → clipboard_helper.rs
pub use crate::clipboard_helper::ClipboardHelper;
pub use crate::pixmap_io::{BufferUtils, GdxGraphics, Pixmap, PixmapIO};

//
// Stubs replaced with real types:
//   Config → pub use beatoraja_core::config::Config
//   PlayerConfig → pub use beatoraja_core::player_config::PlayerConfig
//   ScoreData → pub use beatoraja_core::score_data::ScoreData
//   SongData → pub use beatoraja_song::song_data::SongData
//   ReplayData → pub use beatoraja_core::replay_data::ReplayData
//
// Remaining stubs — Why they cannot be replaced:
//
// MainController:
//   Replaced with NullMainController from beatoraja-types (Phase 18e-2).
//
// PlayerResource:
//   Replaced with Box<dyn PlayerResourceAccess> wrapper (Phase 18e-2).
//   get_original_mode() is crate-local (Mode from bms-model, not on trait).
//
// MainState:
//   Real type is a trait (beatoraja_core::main_state::MainState), but external
//   code uses it as a struct with `state.resource` field access. Replacing
//   requires a wrapper struct or reworking all callers.
//
// MainStateListener:
//   Takes &MainState (struct) in stub vs &dyn MainState (trait) in real type.
//
// SongDatabaseAccessor:
//   Real type is a trait in beatoraja-song, stub is a struct. Callers instantiate
//   it as a concrete type.
//
// ScoreDatabaseAccessor:
//   Real type requires path in constructor (new(path) -> Result), stub is unit
//   struct. set_score_data signature differs (&ScoreData vs &[ScoreData]).
//
// TableData, TableFolder, TableDataAccessor, TableAccessor:
//   Replaced with pub use from beatoraja-core (Phase 18e-11).
//
// Mode:
//   Replaced with real bms_model::mode::Mode enum (Phase 18e-2).
//
// IntegerProperty / BooleanProperty / StringProperty traits + factories:
//   Real traits in beatoraja-skin reference beatoraja-skin's own MainState stub
//   trait, not this crate's MainState struct. Type mismatch.
//
// Twitter4j:
//   No real Rust equivalent exists (twitter4j has no Rust port).
//
// ImGuiNotify:
//   From beatoraja-modmenu (cannot depend on it).
//
// AbstractResult, ScreenType:
//   From beatoraja-result/beatoraja-play (cannot depend on them).

// ============================================================
// MainController — replaced with NullMainController from beatoraja-types (Phase 18e-2)
// ============================================================

pub use beatoraja_types::main_controller_access::NullMainController;

// ============================================================
// PlayerResource — replaced with Box<dyn PlayerResourceAccess> wrapper (Phase 18e-2)
// ============================================================

/// Wrapper for bms.player.beatoraja.PlayerResource.
/// Delegates to `Box<dyn PlayerResourceAccess>` for trait methods.
/// `get_original_mode()` is crate-local (not on trait, since Mode lives in bms-model).
pub struct PlayerResource {
    pub(crate) inner: Box<dyn PlayerResourceAccess>,
    original_mode: Mode,
}

impl PlayerResource {
    pub fn new(inner: Box<dyn PlayerResourceAccess>, original_mode: Mode) -> Self {
        Self {
            inner,
            original_mode,
        }
    }

    pub fn get_config(&self) -> &Config {
        self.inner.get_config()
    }

    pub fn get_songdata(&self) -> Option<&SongData> {
        self.inner.get_songdata()
    }

    pub fn get_replay_data(&self) -> Option<&ReplayData> {
        self.inner.get_replay_data()
    }

    pub fn get_reverse_lookup_levels(&self) -> Vec<String> {
        self.inner.get_reverse_lookup_levels()
    }

    pub fn get_original_mode(&self) -> &Mode {
        &self.original_mode
    }
}

impl Default for PlayerResource {
    fn default() -> Self {
        Self {
            inner: Box::new(NullPlayerResource::new()),
            original_mode: Mode::BEAT_7K,
        }
    }
}

// ============================================================
// Config — replaced with real type from beatoraja-core
// ============================================================

pub use beatoraja_core::config::Config;

// ============================================================
// PlayerConfig — replaced with real type from beatoraja-core
// ============================================================

pub use beatoraja_core::player_config::PlayerConfig;

// ============================================================
// SongData — replaced with real type from beatoraja-song
// ============================================================

pub use beatoraja_song::song_data::SongData;

// ============================================================
// SongDatabaseAccessor — replaced with real trait from beatoraja-types
// ============================================================

pub use beatoraja_types::song_database_accessor::SongDatabaseAccessor;

// ============================================================
// ScoreData — replaced with real type from beatoraja-core
// ============================================================

pub use beatoraja_core::score_data::ScoreData;

// ============================================================
// ScoreDatabaseAccessor — replaced with real type from beatoraja-core
// ============================================================

pub use beatoraja_core::score_database_accessor::ScoreDatabaseAccessor;

// ============================================================
// MainState — replaced with MainStateAccess trait from beatoraja-types
// ============================================================

/// Legacy MainState wrapper for external code that accesses `state.resource`.
/// Implements MainStateAccess and provides direct field access for compatibility.
pub struct MainState {
    pub main: NullMainController,
    pub resource: PlayerResource,
    pub screen_type: ScreenType,
}

impl beatoraja_types::main_state_access::MainStateAccess for MainState {
    fn get_screen_type(&self) -> ScreenType {
        self.screen_type.clone()
    }

    fn get_resource(
        &self,
    ) -> Option<&dyn beatoraja_types::player_resource_access::PlayerResourceAccess> {
        Some(&*self.resource.inner)
    }

    fn get_config(&self) -> &Config {
        self.resource.get_config()
    }
}

impl Default for MainState {
    fn default() -> Self {
        Self {
            main: NullMainController,
            resource: PlayerResource::default(),
            screen_type: ScreenType::Other,
        }
    }
}

// ============================================================
// Screen type — replaced with real type from beatoraja-types
// ============================================================

pub use beatoraja_types::screen_type::ScreenType;

// ============================================================
// AbstractResult — replaced with AbstractResultAccess trait from beatoraja-types
// ============================================================

pub use beatoraja_types::abstract_result_access::AbstractResultAccess;

/// Legacy type alias for backward compatibility in this crate.
/// Functions that return `Option<&AbstractResult>` should use
/// `Option<&dyn AbstractResultAccess>` instead.
pub type AbstractResult = dyn AbstractResultAccess;

// ============================================================
// ReplayData — replaced with real type from beatoraja-core
// ============================================================

pub use beatoraja_core::replay_data::ReplayData;

// ============================================================
// Mode — replaced with real type from bms-model
// ============================================================

pub use bms_model::mode::Mode;

// ============================================================
// TableData and related types — replaced with real types from beatoraja-core (Phase 18e-11)
// ============================================================

pub use beatoraja_core::table_data::{TableData, TableFolder};
pub use beatoraja_core::table_data_accessor::{TableAccessor, TableDataAccessor};

// ============================================================
// ImGuiNotify — real type re-export (replaced from stubs)
// ============================================================

pub use beatoraja_types::imgui_notify::ImGuiNotify;

// ============================================================
// skin::MainState trait impl — bridges external's concrete MainState
// to skin's property system (resolves type mismatch, not a circular dep)
// ============================================================

impl beatoraja_skin::stubs::MainState for MainState {
    fn get_timer(&self) -> &dyn beatoraja_types::timer_access::TimerAccess {
        static TIMER: std::sync::OnceLock<beatoraja_skin::stubs::Timer> =
            std::sync::OnceLock::new();
        TIMER.get_or_init(beatoraja_skin::stubs::Timer::default)
    }

    fn get_offset_value(&self, _id: i32) -> Option<&beatoraja_types::skin_offset::SkinOffset> {
        None
    }

    fn get_main(&self) -> &beatoraja_skin::stubs::MainController {
        static MAIN: beatoraja_skin::stubs::MainController =
            beatoraja_skin::stubs::MainController { debug: false };
        &MAIN
    }

    fn get_image(&self, _id: i32) -> Option<beatoraja_skin::rendering_stubs::TextureRegion> {
        None
    }

    fn get_resource(&self) -> &beatoraja_skin::stubs::PlayerResource {
        static RES: beatoraja_skin::stubs::PlayerResource = beatoraja_skin::stubs::PlayerResource;
        &RES
    }
}

// ============================================================
// IntegerProperty / BooleanProperty / StringProperty
// Delegate to skin's real property factories via MainState trait bridge
// ============================================================

/// Property trait wrapping skin's IntegerProperty for external's &MainState callers.
pub trait IntegerProperty {
    fn get(&self, state: &MainState) -> i32;
}

/// Property trait wrapping skin's BooleanProperty for external's &MainState callers.
pub trait BooleanProperty {
    fn get(&self, state: &MainState) -> bool;
}

/// Property trait wrapping skin's StringProperty for external's &MainState callers.
pub trait StringProperty {
    fn get(&self, state: &MainState) -> String;
}

// --- Wrapper adapters that delegate to skin's real property traits ---

struct SkinIntegerPropertyAdapter(
    Box<dyn beatoraja_skin::property::integer_property::IntegerProperty>,
);
impl IntegerProperty for SkinIntegerPropertyAdapter {
    fn get(&self, state: &MainState) -> i32 {
        self.0.get(state)
    }
}

struct SkinBooleanPropertyAdapter(
    Box<dyn beatoraja_skin::property::boolean_property::BooleanProperty>,
);
impl BooleanProperty for SkinBooleanPropertyAdapter {
    fn get(&self, state: &MainState) -> bool {
        self.0.get(state)
    }
}

struct SkinStringPropertyAdapter(
    Box<dyn beatoraja_skin::property::string_property::StringProperty>,
);
impl StringProperty for SkinStringPropertyAdapter {
    fn get(&self, state: &MainState) -> String {
        self.0.get(state)
    }
}

// --- Default fallbacks for IDs not found in skin's factory ---

struct DefaultIntegerProperty;
impl IntegerProperty for DefaultIntegerProperty {
    fn get(&self, _state: &MainState) -> i32 {
        0
    }
}

struct DefaultBooleanProperty;
impl BooleanProperty for DefaultBooleanProperty {
    fn get(&self, _state: &MainState) -> bool {
        false
    }
}

struct DefaultStringProperty;
impl StringProperty for DefaultStringProperty {
    fn get(&self, _state: &MainState) -> String {
        String::new()
    }
}

// --- Factory facades matching original API ---

pub struct IntegerPropertyFactory;
impl IntegerPropertyFactory {
    pub fn get_integer_property(id: i32) -> Box<dyn IntegerProperty> {
        match beatoraja_skin::property::integer_property_factory::get_integer_property_by_id(id) {
            Some(prop) => Box::new(SkinIntegerPropertyAdapter(prop)),
            None => Box::new(DefaultIntegerProperty),
        }
    }
}

pub struct BooleanPropertyFactory;
impl BooleanPropertyFactory {
    pub fn get_boolean_property(id: i32) -> Box<dyn BooleanProperty> {
        match beatoraja_skin::property::boolean_property_factory::get_boolean_property(id) {
            Some(prop) => Box::new(SkinBooleanPropertyAdapter(prop)),
            None => Box::new(DefaultBooleanProperty),
        }
    }
}

pub struct StringPropertyFactory;
impl StringPropertyFactory {
    pub fn get_string_property(id: i32) -> Box<dyn StringProperty> {
        match beatoraja_skin::property::string_property_factory::get_string_property_by_id(id) {
            Some(prop) => Box::new(SkinStringPropertyAdapter(prop)),
            None => Box::new(DefaultStringProperty),
        }
    }
}

// ============================================================
// SkinProperty constants (re-exported from beatoraja-skin)
// ============================================================

pub use beatoraja_skin::skin_property::{
    NUMBER_CLEAR, NUMBER_MAXSCORE, NUMBER_PLAYLEVEL, OPTION_RESULT_A_1P, OPTION_RESULT_AA_1P,
    OPTION_RESULT_AAA_1P, OPTION_RESULT_B_1P, OPTION_RESULT_C_1P, OPTION_RESULT_D_1P,
    OPTION_RESULT_E_1P, OPTION_RESULT_F_1P, STRING_FULLTITLE, STRING_TABLE_LEVEL,
    STRING_TABLE_NAME,
};

// ============================================================
// Twitter4j stubs (entirely stubbed - no Rust equivalent)
// ============================================================

/// Stub for twitter4j.Twitter — Twitter API not supported in Rust port
pub struct Twitter;

impl Twitter {
    pub fn upload_media(&self, _name: &str, _input: &[u8]) -> anyhow::Result<UploadedMedia> {
        anyhow::bail!(
            "Twitter API is not supported in Rust port (twitter4j has no Rust equivalent)"
        )
    }

    pub fn update_status(&self, _update: &StatusUpdate) -> anyhow::Result<Status> {
        anyhow::bail!(
            "Twitter API is not supported in Rust port (twitter4j has no Rust equivalent)"
        )
    }
}

/// Stub for twitter4j.TwitterFactory
pub struct TwitterFactory;

impl TwitterFactory {
    pub fn new(_config: TwitterConfiguration) -> Self {
        Self
    }

    pub fn get_instance(&self) -> Twitter {
        Twitter
    }
}

/// Stub for twitter4j.conf.ConfigurationBuilder
pub struct TwitterConfigurationBuilder {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
}

impl Default for TwitterConfigurationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitterConfigurationBuilder {
    pub fn new() -> Self {
        Self {
            consumer_key: String::new(),
            consumer_secret: String::new(),
            access_token: String::new(),
            access_token_secret: String::new(),
        }
    }

    pub fn set_o_auth_consumer_key(mut self, key: &str) -> Self {
        self.consumer_key = key.to_string();
        self
    }

    pub fn set_o_auth_consumer_secret(mut self, secret: &str) -> Self {
        self.consumer_secret = secret.to_string();
        self
    }

    pub fn set_o_auth_access_token(mut self, token: &str) -> Self {
        self.access_token = token.to_string();
        self
    }

    pub fn set_o_auth_access_token_secret(mut self, secret: &str) -> Self {
        self.access_token_secret = secret.to_string();
        self
    }

    pub fn build(self) -> TwitterConfiguration {
        TwitterConfiguration
    }
}

/// Stub for twitter4j.conf.Configuration
pub struct TwitterConfiguration;

/// Stub for twitter4j.UploadedMedia
pub struct UploadedMedia {
    pub media_id: i64,
}

impl std::fmt::Display for UploadedMedia {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UploadedMedia(id={})", self.media_id)
    }
}

impl UploadedMedia {
    pub fn get_media_id(&self) -> i64 {
        self.media_id
    }
}

/// Stub for twitter4j.StatusUpdate
pub struct StatusUpdate {
    pub text: String,
    pub media_ids: Vec<i64>,
}

impl StatusUpdate {
    pub fn new(text: String) -> Self {
        Self {
            text,
            media_ids: Vec::new(),
        }
    }

    pub fn set_media_ids(&mut self, ids: Vec<i64>) {
        self.media_ids = ids;
    }
}

/// Stub for twitter4j.Status
pub struct Status;

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status")
    }
}

// ============================================================
// MainStateListener — replaced with trait from beatoraja-types
// ============================================================

// Re-export the trait from beatoraja-types.
// Note: the real trait uses `&dyn MainStateAccess` instead of `&MainState`.
// External code still uses the legacy `MainState` struct which implements `MainStateAccess`.
pub use beatoraja_types::main_state_access::MainStateListener;

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_types::main_state_access::MainStateAccess;

    #[test]
    fn main_state_default_screen_type_is_other() {
        let state = MainState::default();
        assert_eq!(state.get_screen_type(), ScreenType::Other);
    }

    #[test]
    fn main_state_with_screen_type_returns_correct_type() {
        let state = MainState {
            main: NullMainController,
            resource: PlayerResource::default(),
            screen_type: ScreenType::MusicSelector,
        };
        assert_eq!(state.get_screen_type(), ScreenType::MusicSelector);
    }

    #[test]
    fn main_state_with_each_screen_type_variant() {
        let variants = vec![
            ScreenType::MusicSelector,
            ScreenType::MusicDecide,
            ScreenType::BMSPlayer,
            ScreenType::MusicResult,
            ScreenType::CourseResult,
            ScreenType::KeyConfiguration,
            ScreenType::Other,
        ];
        for variant in variants {
            let state = MainState {
                main: NullMainController,
                resource: PlayerResource::default(),
                screen_type: variant.clone(),
            };
            assert_eq!(state.get_screen_type(), variant);
        }
    }
}

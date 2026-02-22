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
}

impl beatoraja_types::main_state_access::MainStateAccess for MainState {
    fn get_screen_type(&self) -> ScreenType {
        ScreenType::Other
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
// IntegerProperty / BooleanProperty / StringProperty stubs
// ============================================================

/// Stub for bms.player.beatoraja.skin.property.IntegerProperty
pub trait IntegerProperty {
    fn get(&self, state: &MainState) -> i32;
}

/// Stub for bms.player.beatoraja.skin.property.BooleanProperty
pub trait BooleanProperty {
    fn get(&self, state: &MainState) -> bool;
}

/// Stub for bms.player.beatoraja.skin.property.StringProperty
pub trait StringProperty {
    fn get(&self, state: &MainState) -> String;
}

/// Stub for IntegerPropertyFactory
pub struct IntegerPropertyFactory;

/// Default integer property returning 0
struct DefaultIntegerProperty;
impl IntegerProperty for DefaultIntegerProperty {
    fn get(&self, _state: &MainState) -> i32 {
        0
    }
}

impl IntegerPropertyFactory {
    pub fn get_integer_property(_id: i32) -> Box<dyn IntegerProperty> {
        log::warn!("not yet implemented: IntegerPropertyFactory.getIntegerProperty");
        Box::new(DefaultIntegerProperty)
    }
}

/// Stub for BooleanPropertyFactory
pub struct BooleanPropertyFactory;

/// Default boolean property returning false
struct DefaultBooleanProperty;
impl BooleanProperty for DefaultBooleanProperty {
    fn get(&self, _state: &MainState) -> bool {
        false
    }
}

impl BooleanPropertyFactory {
    pub fn get_boolean_property(_id: i32) -> Box<dyn BooleanProperty> {
        log::warn!("not yet implemented: BooleanPropertyFactory.getBooleanProperty");
        Box::new(DefaultBooleanProperty)
    }
}

/// Stub for StringPropertyFactory
pub struct StringPropertyFactory;

/// Default string property returning empty string
struct DefaultStringProperty;
impl StringProperty for DefaultStringProperty {
    fn get(&self, _state: &MainState) -> String {
        String::new()
    }
}

impl StringPropertyFactory {
    pub fn get_string_property(_id: i32) -> Box<dyn StringProperty> {
        log::warn!("not yet implemented: StringPropertyFactory.getStringProperty");
        Box::new(DefaultStringProperty)
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

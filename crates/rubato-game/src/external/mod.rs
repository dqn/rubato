//! External service integrations: score import, screenshots, webhooks.

// beatoraja-external: External integrations (screenshot, webhook, BMS search, Discord, score import)

// Discord RPC (merged from discord-rpc crate)
pub mod discord_rpc;

// OBS WebSocket (merged from beatoraja-obs crate)
pub mod obs;

// Real implementations
pub mod clipboard_helper;
pub mod pixmap_io;

// Adapter modules
pub mod main_state_adapter;
pub mod player_resource_adapter;
pub mod property_adapters;

// BMS Search API accessor
pub mod bms_search_accessor;

// Discord Rich Presence listener
pub mod discord_listener;

// Score data import from LR2
pub mod score_data_importer;

// Screenshot export interface (trait)
pub mod screen_shot_exporter;

// Screenshot file exporter
pub mod screen_shot_file_exporter;

// Webhook handler for Discord webhooks
pub mod webhook_handler;

// ============================================================
// Type re-exports
// ============================================================

pub use crate::core::config::Config;
pub use crate::core::player_config::PlayerConfig;
pub use crate::core::replay_data::ReplayData;
pub use crate::core::score_data::ScoreData;
pub use crate::core::score_database_accessor::ScoreDatabaseAccessor;
pub use crate::core::table_data::{TableData, TableFolder};
pub use crate::core::table_data_accessor::{TableAccessor, TableDataAccessor};
pub use crate::song::song_data::SongData;
pub use bms::model::mode::Mode;
pub use rubato_types::abstract_result_access::AbstractResultAccess;
pub use rubato_types::imgui_notify::ImGuiNotify;
pub use rubato_types::main_controller_access::NullMainController;
pub use rubato_types::screen_type::ScreenType;
pub use rubato_types::song_database_accessor::SongDatabaseAccessor;

pub use crate::external::clipboard_helper::ClipboardHelper;
pub use crate::external::main_state_adapter::MainState;
pub use crate::external::pixmap_io::{BufferUtils, GdxGraphics, Pixmap, PixmapIO};
pub use crate::external::player_resource_adapter::PlayerResource;
pub use crate::external::property_adapters::{
    BooleanProperty, BooleanPropertyFactory, IntegerProperty, IntegerPropertyFactory,
    StringProperty, StringPropertyFactory,
};

pub use rubato_skin::skin_property::{
    NUMBER_CLEAR, NUMBER_MAXSCORE, NUMBER_PLAYLEVEL, OPTION_RESULT_A_1P, OPTION_RESULT_AA_1P,
    OPTION_RESULT_AAA_1P, OPTION_RESULT_B_1P, OPTION_RESULT_C_1P, OPTION_RESULT_D_1P,
    OPTION_RESULT_E_1P, OPTION_RESULT_F_1P, STRING_FULLTITLE, STRING_TABLE_LEVEL,
    STRING_TABLE_NAME,
};

/// Legacy type alias for backward compatibility in this crate.
/// Functions that return `Option<&AbstractResult>` should use
/// `Option<&dyn AbstractResultAccess>` instead.
pub type AbstractResult = dyn AbstractResultAccess;

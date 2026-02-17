//! Configuration data structures for system, player, play, skin, audio, and IR settings.
//!
//! Provides [`Config`] for global system settings (window, display, audio driver),
//! [`PlayerConfig`] for per-player preferences and key bindings,
//! [`PlayConfig`] / [`PlayModeConfig`] for gameplay options, [`SkinConfig`] for skin
//! customization, [`AudioConfig`] for audio driver selection, and [`IRConfig`] for
//! internet ranking credentials. All configs are serialized as JSON via serde.

pub mod audio_config;
pub mod config;
pub mod ir_config;
pub mod play_config;
pub mod play_mode_config;
pub mod player_config;
pub mod practice_config;
pub mod resolution;
pub mod skin_config;
pub mod skin_type;

pub use audio_config::{AudioConfig, DriverType, FrequencyType};
pub use config::{Config, DisplayMode, SongPreview};
pub use ir_config::IRConfig;
pub use play_config::PlayConfig;
pub use play_mode_config::{
    ControllerConfig, KeyboardConfig, MidiConfig, MidiInput, MidiInputType, MouseScratchConfig,
    PlayModeConfig,
};
pub use player_config::PlayerConfig;
pub use practice_config::PracticeProperty;
pub use resolution::Resolution;
pub use skin_config::{FilePath, Offset, Property, SkinConfig, SkinOption};
pub use skin_type::SkinType;

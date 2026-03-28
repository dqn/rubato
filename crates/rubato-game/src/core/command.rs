use bms::model::mode::Mode;
use rubato_types::play_config::PlayConfig;
use rubato_types::skin_config::SkinConfig;

use crate::core::player_config::PlayerConfig;

/// Commands queued by states during render/input and drained by MainController.
///
/// Replaces the scattered outbox fields on DatabaseState, GameContext, and
/// ModmenuOutbox. The queue is `Arc<Mutex<Vec<Command>>>` so that egui
/// callbacks (which cannot carry `&mut GameContext`) can push commands
/// through a cloned `Arc`.
pub enum Command {
    /// Request a song database update. None = update all, Some(path) = specific.
    UpdateSong(Option<String>),
    /// Request a table data update.
    UpdateTable(Box<dyn crate::table_update_source::TableUpdateSource>),
    /// Load a new player profile.
    LoadNewProfile(Box<PlayerConfig>),
    /// Update play config for a specific mode (from modmenu).
    UpdatePlayConfig { mode: Mode, config: Box<PlayConfig> },
    /// Save config and player config to disk.
    SaveConfig,
    /// Update skin config at the given slot index.
    UpdateSkinConfig {
        id: usize,
        config: Option<Box<SkinConfig>>,
    },
    /// Update skin history entry for the given path.
    UpdateSkinHistory {
        path: String,
        config: Box<SkinConfig>,
    },
}

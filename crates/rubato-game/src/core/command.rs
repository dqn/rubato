use crate::core::player_config::PlayerConfig;

/// Commands queued by states during render/input and drained by MainController.
///
/// Replaces the scattered outbox fields on DatabaseState and GameContext.
pub enum Command {
    /// Request a song database update. None = update all, Some(path) = specific.
    UpdateSong(Option<String>),
    /// Request a table data update.
    UpdateTable(Box<dyn rubato_types::table_update_source::TableUpdateSource>),
    /// Load a new player profile.
    LoadNewProfile(Box<PlayerConfig>),
}

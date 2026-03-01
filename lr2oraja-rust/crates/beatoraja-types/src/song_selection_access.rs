use crate::score_data::ScoreData;
use crate::song_data::SongData;

/// Trait for accessing currently selected song information from MusicSelector.
///
/// Used by modmenu and other crates that need selected bar data without
/// depending on the full MusicSelector type.
pub trait SongSelectionAccess: Send {
    /// Returns the song data of the currently selected bar, if it's a SongBar.
    fn get_selected_song_data(&self) -> Option<SongData>;

    /// Returns the score data of the currently selected bar, if it's a SongBar.
    fn get_selected_score_data(&self) -> Option<ScoreData>;

    /// Returns reverse lookup data for the currently selected song.
    fn get_reverse_lookup_data(&self) -> Vec<String>;
}

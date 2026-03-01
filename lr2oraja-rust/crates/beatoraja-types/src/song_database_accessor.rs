use crate::folder_data::FolderData;
use crate::song_data::SongData;

/// Song database accessor interface (core query methods).
///
/// Update methods that depend on SongDatabaseUpdateListener/SongInformationAccessor
/// remain as inherent methods on the concrete implementation in beatoraja-song.
pub trait SongDatabaseAccessor: Send {
    /// Get song data by key-value pair
    fn get_song_datas(&self, key: &str, value: &str) -> Vec<SongData>;

    /// Get song data by MD5/SHA256 hashes
    fn get_song_datas_by_hashes(&self, hashes: &[String]) -> Vec<SongData>;

    /// Query song data using SQL across score, scorelog, and info databases
    fn get_song_datas_by_sql(
        &self,
        sql: &str,
        score: &str,
        scorelog: &str,
        info: Option<&str>,
    ) -> Vec<SongData>;

    /// Set song data
    fn set_song_datas(&self, songs: &[SongData]);

    /// Search song data by text
    fn get_song_datas_by_text(&self, text: &str) -> Vec<SongData>;

    /// Get folder data by key-value pair
    fn get_folder_datas(&self, key: &str, value: &str) -> Vec<FolderData>;

    /// Update song database for the given path and BMS root directories.
    fn update_song_datas(
        &self,
        _update_path: Option<&str>,
        _bmsroot: &[String],
        _update_all: bool,
        _update_parent_when_missing: bool,
    ) {
        // default no-op
    }
}

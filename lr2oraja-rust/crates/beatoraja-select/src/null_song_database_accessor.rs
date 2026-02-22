use beatoraja_types::folder_data::FolderData;
use beatoraja_types::song_data::SongData;
use beatoraja_types::song_database_accessor::SongDatabaseAccessor;

/// Null implementation of SongDatabaseAccessor.
/// Returns empty results for all queries.
/// Used as default in MusicSelector when no real database is connected.
pub struct NullSongDatabaseAccessor;

impl SongDatabaseAccessor for NullSongDatabaseAccessor {
    fn get_song_datas(&self, _key: &str, _value: &str) -> Vec<SongData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_song_datas");
        Vec::new()
    }
    fn get_song_datas_by_hashes(&self, _hashes: &[String]) -> Vec<SongData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_song_datas_by_hashes");
        Vec::new()
    }
    fn get_song_datas_by_sql(
        &self,
        _sql: &str,
        _score: &str,
        _scorelog: &str,
        _info: Option<&str>,
    ) -> Vec<SongData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_song_datas_by_sql");
        Vec::new()
    }
    fn set_song_datas(&self, _songs: &[SongData]) {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.set_song_datas");
    }
    fn get_song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_song_datas_by_text");
        Vec::new()
    }
    fn get_folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_folder_datas");
        Vec::new()
    }
}

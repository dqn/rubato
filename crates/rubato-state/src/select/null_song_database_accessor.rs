use rubato_types::folder_data::FolderData;
use rubato_types::song_data::SongData;
use rubato_types::song_database_accessor::SongDatabaseAccessor;

/// Null object pattern — returns empty results for all queries.
/// Used as default in MusicSelector when no real database is connected.
pub struct NullSongDatabaseAccessor;

impl SongDatabaseAccessor for NullSongDatabaseAccessor {
    fn song_datas(&self, _key: &str, _value: &str) -> Vec<SongData> {
        log::warn!("NullSongDatabaseAccessor.song_datas: returning empty result");
        Vec::new()
    }
    fn song_datas_by_hashes(&self, _hashes: &[String]) -> Vec<SongData> {
        log::warn!("NullSongDatabaseAccessor.get_song_datas_by_hashes: returning empty result");
        Vec::new()
    }
    fn song_datas_by_sql(
        &self,
        _sql: &str,
        _score: &str,
        _scorelog: &str,
        _info: Option<&str>,
    ) -> Vec<SongData> {
        log::warn!("NullSongDatabaseAccessor.get_song_datas_by_sql: returning empty result");
        Vec::new()
    }
    fn set_song_datas(&self, _songs: &[SongData]) -> anyhow::Result<()> {
        log::warn!("NullSongDatabaseAccessor.set_song_datas: no-op");
        Ok(())
    }
    fn song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
        log::warn!("NullSongDatabaseAccessor.get_song_datas_by_text: returning empty result");
        Vec::new()
    }
    fn folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
        log::warn!("NullSongDatabaseAccessor.get_folder_datas: returning empty result");
        Vec::new()
    }
}

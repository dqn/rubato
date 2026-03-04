/// Corresponds to MusicDatabaseAccessor interface in Java
/// Interface for music database access
pub trait MusicDatabaseAccessor: Send + Sync {
    /// Get music paths by SHA256/md5
    fn get_music_paths(&self, md5: &[String]) -> Vec<String>;
}

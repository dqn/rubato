/// Corresponds to MusicDatabaseAccessor interface in Java
/// Interface for music database access
pub trait MusicDatabaseAccessor: Send + Sync {
    /// Get music paths by SHA256/md5
    fn get_music_paths(&self, md5: &[String]) -> Vec<String>;

    /// Trigger a song DB refresh for the given directory path.
    /// Called after IPFS download completes to rebuild songdata.db.
    fn update_song(&self, path: &str) {
        let _ = path;
    }
}

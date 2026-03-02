/// Trait for IPFS music download processor access across crate boundaries.
///
/// The concrete implementation lives in md-processor where the IPFS download
/// daemon is implemented. MainController holds this as a trait object to
/// avoid depending on md-processor.
///
/// Translated from: bms.player.beatoraja.md.MusicDownloadProcessor
pub trait MusicDownloadAccess: Send + Sync {
    /// Start the download daemon, optionally queueing a song for download.
    fn start_download(&self, song: &crate::song_data::SongData);

    /// Dispose of the download daemon.
    fn dispose(&self);

    /// Whether the download daemon thread is alive.
    /// Java: MusicDownloadProcessor.isAlive() checks Thread.isAlive().
    fn is_alive(&self) -> bool;

    /// Whether a download is currently in progress.
    fn is_download(&self) -> bool;

    /// Get the current download status message.
    fn get_message(&self) -> String;
}

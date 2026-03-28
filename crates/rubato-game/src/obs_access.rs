/// Trait interface for OBS WebSocket client access.
///
/// Downstream crates use `Box<dyn ObsAccess>` instead of concrete ObsWsClient.
/// The real implementation is in beatoraja-obs.
pub trait ObsAccess: Send + Sync {
    /// Save the last OBS recording with the given reason tag.
    fn save_last_recording(&self, reason: &str);

    /// Check if OBS is connected.
    fn is_connected(&self) -> bool {
        false
    }

    /// Check if OBS is recording.
    fn is_recording(&self) -> bool {
        false
    }
}

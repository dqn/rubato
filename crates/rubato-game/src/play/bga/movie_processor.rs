use crate::play::Texture;

/// Movie processor interface for video playback
pub trait MovieProcessor: Send {
    /// Get the current video frame
    fn frame(&mut self, time: i64) -> Option<Texture>;

    /// Start video playback
    fn play(&mut self, time: i64, loop_play: bool);

    /// Stop video playback
    fn stop(&mut self);

    /// Release resources
    fn dispose(&mut self);
}

// Movie processor trait and stub implementation.
//
// Ported from Java MovieProcessor interface (~29 lines).
// Video decoding (FFmpegProcessor) is deferred; only the trait and a
// no-op stub are provided for now.

use std::path::Path;

use anyhow::Result;
use bevy::prelude::*;

/// Trait for video/movie playback processors.
///
/// Implementations decode video files frame-by-frame and provide
/// textures for BGA rendering.
pub trait MovieProcessor: Send + Sync {
    /// Load and prepare a video file for playback.
    fn prepare(&mut self, path: &Path) -> Result<()>;

    /// Start or seek playback to the given time.
    ///
    /// - `time_us`: playback position in microseconds
    /// - `looping`: whether to loop the video
    fn play(&mut self, time_us: i64, looping: bool);

    /// Get the video frame at the given time as a Bevy image handle.
    fn get_frame(&self, time_us: i64) -> Option<Handle<Image>>;

    /// Stop playback.
    fn stop(&mut self);

    /// Whether the processor is ready for playback.
    fn is_ready(&self) -> bool;

    /// Upload the latest decoded frame to the Bevy image asset.
    ///
    /// Called every frame from the main thread. Returns the image handle
    /// if a new frame was uploaded.
    fn update_frame(&mut self, images: &mut Assets<Image>) -> Option<Handle<Image>>;

    /// Release all resources.
    fn dispose(&mut self);
}

/// No-op movie processor stub. Returns no frames and ignores all operations.
///
/// Used as a placeholder until a real video decoder (e.g., FFmpeg) is integrated.
#[derive(Debug, Default)]
pub struct StubMovieProcessor;

impl MovieProcessor for StubMovieProcessor {
    fn prepare(&mut self, _path: &Path) -> Result<()> {
        Ok(())
    }

    fn play(&mut self, _time_us: i64, _looping: bool) {}

    fn get_frame(&self, _time_us: i64) -> Option<Handle<Image>> {
        None
    }

    fn stop(&mut self) {}

    fn update_frame(&mut self, _images: &mut Assets<Image>) -> Option<Handle<Image>> {
        None
    }

    fn is_ready(&self) -> bool {
        false
    }

    fn dispose(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_is_not_ready() {
        let stub = StubMovieProcessor;
        assert!(!stub.is_ready());
    }

    #[test]
    fn stub_returns_no_frame() {
        let stub = StubMovieProcessor;
        assert!(stub.get_frame(0).is_none());
        assert!(stub.get_frame(1_000_000).is_none());
    }

    #[test]
    fn stub_prepare_succeeds() {
        let mut stub = StubMovieProcessor;
        assert!(stub.prepare(Path::new("nonexistent.mp4")).is_ok());
    }
}

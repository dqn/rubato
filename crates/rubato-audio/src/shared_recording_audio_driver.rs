//! A thread-safe shared wrapper around `RecordingAudioDriver`.
//!
//! Stores the real `RecordingAudioDriver` inside an `Arc<Mutex<>>` so that
//! the E2E harness can retain a handle to inspect recorded events even
//! though `MainController` owns the driver as `Box<dyn AudioDriver>`.

use std::sync::{Arc, Mutex};

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

use crate::audio_driver::AudioDriver;
use crate::recording_audio_driver::{AudioEvent, RecordingAudioDriver};

/// A shared wrapper around `RecordingAudioDriver` that implements `AudioDriver`.
///
/// All trait methods lock the inner `Mutex` and delegate to the wrapped driver.
/// The harness keeps a clone of the `Arc<Mutex<RecordingAudioDriver>>` so it
/// can query events without downcasting through the trait object.
pub struct SharedRecordingAudioDriver {
    inner: Arc<Mutex<RecordingAudioDriver>>,
}

impl SharedRecordingAudioDriver {
    /// Create a new shared driver wrapping a fresh `RecordingAudioDriver`.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RecordingAudioDriver::new())),
        }
    }

    /// Returns a clone of the inner `Arc` for external event inspection.
    pub fn inner(&self) -> Arc<Mutex<RecordingAudioDriver>> {
        Arc::clone(&self.inner)
    }

    /// Returns a snapshot of all recorded events.
    pub fn events(&self) -> Vec<AudioEvent> {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .events()
            .to_vec()
    }

    /// Clears the event log.
    pub fn clear_events(&self) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .clear_events();
    }
}

impl Default for SharedRecordingAudioDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioDriver for SharedRecordingAudioDriver {
    fn play_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .play_path(path, volume, loop_play);
    }

    fn set_volume_path(&mut self, path: &str, volume: f32) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .set_volume_path(path, volume);
    }

    fn is_playing_path(&self, path: &str) -> bool {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .is_playing_path(path)
    }

    fn stop_path(&mut self, path: &str) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .stop_path(path);
    }

    fn dispose_path(&mut self, path: &str) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .dispose_path(path);
    }

    fn set_model(&mut self, model: &BMSModel) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .set_model(model);
    }

    fn set_additional_key_sound(&mut self, judge: i32, fast: bool, path: Option<&str>) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .set_additional_key_sound(judge, fast, path);
    }

    fn abort(&mut self) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .abort();
    }

    fn get_progress(&self) -> f32 {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .get_progress()
    }

    fn preload_path(&mut self, path: &str) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .preload_path(path);
    }

    fn play_note(&mut self, n: &Note, volume: f32, pitch: i32) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .play_note(n, volume, pitch);
    }

    fn play_judge(&mut self, judge: i32, fast: bool) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .play_judge(judge, fast);
    }

    fn stop_note(&mut self, n: Option<&Note>) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .stop_note(n);
    }

    fn set_volume_note(&mut self, n: &Note, volume: f32) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .set_volume_note(n, volume);
    }

    fn set_global_pitch(&mut self, pitch: f32) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .set_global_pitch(pitch);
    }

    fn get_global_pitch(&self) -> f32 {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .get_global_pitch()
    }

    fn dispose_old(&mut self) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .dispose_old();
    }

    fn dispose(&mut self) {
        self.inner
            .lock()
            .expect("recording audio driver mutex poisoned")
            .dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_driver_records_events() {
        let shared = SharedRecordingAudioDriver::new();
        let handle = shared.inner();

        // We need mutable access through the trait, so use the inner directly
        handle
            .lock()
            .expect("recording audio driver mutex poisoned")
            .play_path("bgm.ogg", 0.8, false);

        let events = shared.events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            AudioEvent::PlayPath {
                path: "bgm.ogg".to_string(),
                volume: 0.8,
                loop_play: false,
            }
        );
    }

    #[test]
    fn shared_driver_trait_object_delegates() {
        let mut shared = SharedRecordingAudioDriver::new();
        let handle = shared.inner();

        // Use through AudioDriver trait
        shared.play_path("test.wav", 1.0, true);
        shared.stop_path("test.wav");

        let inner = handle
            .lock()
            .expect("recording audio driver mutex poisoned");
        assert_eq!(inner.events().len(), 2);
        assert!(!inner.is_playing_path("test.wav"));
    }

    #[test]
    fn clear_events_works() {
        let mut shared = SharedRecordingAudioDriver::new();

        shared.play_path("a.wav", 1.0, false);
        assert_eq!(shared.events().len(), 1);

        shared.clear_events();
        assert!(shared.events().is_empty());
    }

    #[test]
    fn inner_handle_survives_driver_moves() {
        let shared = SharedRecordingAudioDriver::new();
        let handle = shared.inner();

        // Simulate what E2eHarness does: keep handle, move driver into Box
        let mut boxed: Box<dyn AudioDriver> = Box::new(shared);
        boxed.play_path("moved.ogg", 0.5, false);

        // Handle still works
        let inner = handle
            .lock()
            .expect("recording audio driver mutex poisoned");
        assert_eq!(inner.events().len(), 1);
        assert!(inner.is_playing_path("moved.ogg"));
    }
}

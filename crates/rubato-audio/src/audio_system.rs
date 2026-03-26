//! Concrete enum replacing `Box<dyn AudioDriver>` for audio driver dispatch.
//!
//! Each variant wraps one of the audio driver implementations. Method calls
//! are delegated via `match` instead of dynamic dispatch.

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

use crate::audio_driver::AudioDriver;
use crate::gdx_audio_device_driver::GdxAudioDeviceDriver;
use crate::gdx_sound_driver::GdxSoundDriver;
use crate::port_audio_driver::PortAudioDriver;
use crate::recording_audio_driver::RecordingAudioDriver;
use crate::shared_recording_audio_driver::SharedRecordingAudioDriver;

/// Concrete audio system enum replacing `Box<dyn AudioDriver>`.
///
/// The `Noop` variant is used when no real audio driver is needed (e.g. in
/// queued command proxies where audio commands are forwarded via a command
/// queue rather than executed directly).
///
/// The `Boxed` variant provides backward compatibility for test-only or
/// one-off `AudioDriver` trait implementations that are not part of the
/// standard driver set.
pub enum AudioSystem {
    GdxSound(GdxSoundDriver),
    GdxAudioDevice(GdxAudioDeviceDriver),
    PortAudio(PortAudioDriver),
    Recording(RecordingAudioDriver),
    SharedRecording(SharedRecordingAudioDriver),
    Boxed(Box<dyn AudioDriver>),
    Noop,
}

macro_rules! delegate {
    ($self:expr, $method:ident ( $($arg:expr),* )) => {
        match $self {
            AudioSystem::GdxSound(d) => d.$method($($arg),*),
            AudioSystem::GdxAudioDevice(d) => d.$method($($arg),*),
            AudioSystem::PortAudio(d) => d.$method($($arg),*),
            AudioSystem::Recording(d) => d.$method($($arg),*),
            AudioSystem::SharedRecording(d) => d.$method($($arg),*),
            AudioSystem::Boxed(d) => d.$method($($arg),*),
            AudioSystem::Noop => Default::default(),
        }
    };
    // For methods that need a non-Default noop return value
    ($self:expr, $method:ident ( $($arg:expr),* ), noop: $noop:expr) => {
        match $self {
            AudioSystem::GdxSound(d) => d.$method($($arg),*),
            AudioSystem::GdxAudioDevice(d) => d.$method($($arg),*),
            AudioSystem::PortAudio(d) => d.$method($($arg),*),
            AudioSystem::Recording(d) => d.$method($($arg),*),
            AudioSystem::SharedRecording(d) => d.$method($($arg),*),
            AudioSystem::Boxed(d) => d.$method($($arg),*),
            AudioSystem::Noop => $noop,
        }
    };
}

impl AudioSystem {
    /// Play audio at the specified path.
    pub fn play_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        delegate!(self, play_path(path, volume, loop_play));
    }

    /// Set volume for the audio at the specified path.
    pub fn set_volume_path(&mut self, path: &str, volume: f32) {
        delegate!(self, set_volume_path(path, volume));
    }

    /// Returns true if the audio at the specified path is playing.
    pub fn is_playing_path(&self, path: &str) -> bool {
        delegate!(self, is_playing_path(path))
    }

    /// Stop the audio at the specified path.
    pub fn stop_path(&mut self, path: &str) {
        delegate!(self, stop_path(path));
    }

    /// Dispose the audio at the specified path.
    pub fn dispose_path(&mut self, path: &str) {
        delegate!(self, dispose_path(path));
    }

    /// Load BMS audio data.
    pub fn set_model(&mut self, model: &BMSModel) {
        delegate!(self, set_model(model));
    }

    /// Define additional key sounds for judgement.
    pub fn set_additional_key_sound(&mut self, judge: i32, fast: bool, path: Option<&str>) {
        delegate!(self, set_additional_key_sound(judge, fast, path));
    }

    /// Abort loading BMS audio data.
    pub fn abort(&mut self) {
        delegate!(self, abort());
    }

    /// Returns audio loading progress (0.0 - 1.0).
    pub fn get_progress(&self) -> f32 {
        delegate!(self, get_progress(), noop: 1.0)
    }

    /// Poll background loading completion. Returns true when loading is finished.
    pub fn poll_loading(&mut self) -> bool {
        delegate!(self, poll_loading(), noop: true)
    }

    /// Preload a sound file into the path sound cache without playing it.
    pub fn preload_path(&mut self, path: &str) {
        delegate!(self, preload_path(path));
    }

    /// Play the sound for the specified Note.
    pub fn play_note(&mut self, n: &Note, volume: f32, pitch: i32) {
        delegate!(self, play_note(n, volume, pitch));
    }

    /// Play additional key sound for judgement.
    pub fn play_judge(&mut self, judge: i32, fast: bool) {
        delegate!(self, play_judge(judge, fast));
    }

    /// Stop the sound for the specified Note. If None, stop all sounds.
    pub fn stop_note(&mut self, n: Option<&Note>) {
        delegate!(self, stop_note(n));
    }

    /// Set volume for the specified Note.
    pub fn set_volume_note(&mut self, n: &Note, volume: f32) {
        delegate!(self, set_volume_note(n, volume));
    }

    /// Set global pitch (0.5 - 2.0).
    pub fn set_global_pitch(&mut self, pitch: f32) {
        delegate!(self, set_global_pitch(pitch));
    }

    /// Get global pitch (0.5 - 2.0).
    pub fn get_global_pitch(&self) -> f32 {
        delegate!(self, get_global_pitch(), noop: 1.0)
    }

    /// Dispose old audio resources.
    pub fn dispose_old(&mut self) {
        delegate!(self, dispose_old());
    }

    /// Dispose all resources.
    pub fn dispose(&mut self) {
        delegate!(self, dispose());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_returns_defaults() {
        let mut noop = AudioSystem::Noop;
        assert!(!noop.is_playing_path("anything"));
        assert_eq!(noop.get_progress(), 1.0);
        assert!(noop.poll_loading());
        assert_eq!(noop.get_global_pitch(), 1.0);
        // These should not panic
        noop.play_path("test", 1.0, false);
        noop.stop_path("test");
        noop.dispose();
    }

    #[test]
    fn recording_variant_delegates() {
        let mut sys = AudioSystem::Recording(RecordingAudioDriver::new());
        sys.play_path("bgm.ogg", 0.8, false);
        assert!(sys.is_playing_path("bgm.ogg"));
        sys.stop_path("bgm.ogg");
        assert!(!sys.is_playing_path("bgm.ogg"));
    }

    #[test]
    fn recording_variant_pitch() {
        let mut sys = AudioSystem::Recording(RecordingAudioDriver::new());
        assert_eq!(sys.get_global_pitch(), 1.0);
        sys.set_global_pitch(1.5);
        assert_eq!(sys.get_global_pitch(), 1.5);
    }

    #[test]
    fn shared_recording_variant_delegates() {
        let shared = SharedRecordingAudioDriver::new();
        let handle = shared.inner();
        let mut sys = AudioSystem::SharedRecording(shared);
        sys.play_path("test.wav", 1.0, false);
        assert!(sys.is_playing_path("test.wav"));
        // Verify through the shared handle
        let inner = handle.lock().expect("mutex poisoned");
        assert!(inner.is_playing_path("test.wav"));
    }
}

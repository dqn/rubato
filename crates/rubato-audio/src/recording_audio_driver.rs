//! A recording audio driver for testing.
//!
//! Records all method calls as `AudioEvent` entries, tracks playback state,
//! and provides query helpers. This is a real implementation of `AudioDriver`,
//! not a mock -- it maintains correct internal state for `is_playing_path()`,
//! `get_global_pitch()`, and `get_progress()`.

use std::collections::HashSet;

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

use crate::audio_driver::AudioDriver;

/// An event recorded by `RecordingAudioDriver`.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioEvent {
    PlayPath {
        path: String,
        volume: f32,
        loop_play: bool,
    },
    SetVolumePath {
        path: String,
        volume: f32,
    },
    StopPath {
        path: String,
    },
    DisposePath {
        path: String,
    },
    SetModel,
    SetAdditionalKeySound {
        judge: i32,
        fast: bool,
        path: Option<String>,
    },
    Abort,
    PlayNote {
        wav_id: i32,
        volume: f32,
        pitch: i32,
    },
    PlayJudge {
        judge: i32,
        fast: bool,
    },
    StopNote {
        /// None means stop all notes.
        wav_id: Option<i32>,
    },
    SetVolumeNote {
        wav_id: i32,
        volume: f32,
    },
    SetGlobalPitch {
        pitch: f32,
    },
    DisposeOld,
    Dispose,
    PreloadPath {
        path: String,
    },
}

/// A test audio driver that records every call and tracks playback state.
///
/// Use this in integration and E2E tests instead of per-test `MockAudioDriver`
/// definitions. All calls are appended to an internal event log that can be
/// queried after the test completes.
pub struct RecordingAudioDriver {
    events: Vec<AudioEvent>,
    playing_paths: HashSet<String>,
    global_pitch: f32,
    progress: f32,
}

impl RecordingAudioDriver {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            playing_paths: HashSet::new(),
            global_pitch: 1.0,
            progress: 1.0,
        }
    }

    /// Returns a slice of all recorded events.
    pub fn events(&self) -> &[AudioEvent] {
        &self.events
    }

    /// Clears the event log.
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    /// Returns the number of `PlayPath` events recorded.
    pub fn play_path_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, AudioEvent::PlayPath { .. }))
            .count()
    }

    /// Returns the number of `StopPath` events recorded.
    pub fn stop_path_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, AudioEvent::StopPath { .. }))
            .count()
    }

    /// Returns the number of `StopNote` events recorded.
    pub fn stop_note_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, AudioEvent::StopNote { .. }))
            .count()
    }

    /// Returns all paths that were played (from `PlayPath` events).
    pub fn played_paths(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                AudioEvent::PlayPath { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// Returns all paths that were stopped (from `StopPath` events).
    pub fn stopped_paths(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                AudioEvent::StopPath { path } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// Returns all paths that were disposed (from `DisposePath` events).
    pub fn disposed_paths(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                AudioEvent::DisposePath { path } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// Sets the progress value returned by `get_progress()`.
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress;
    }
}

impl Default for RecordingAudioDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioDriver for RecordingAudioDriver {
    fn play_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        self.events.push(AudioEvent::PlayPath {
            path: path.to_string(),
            volume,
            loop_play,
        });
        self.playing_paths.insert(path.to_string());
    }

    fn set_volume_path(&mut self, path: &str, volume: f32) {
        self.events.push(AudioEvent::SetVolumePath {
            path: path.to_string(),
            volume,
        });
    }

    fn is_playing_path(&self, path: &str) -> bool {
        self.playing_paths.contains(path)
    }

    fn stop_path(&mut self, path: &str) {
        self.events.push(AudioEvent::StopPath {
            path: path.to_string(),
        });
        self.playing_paths.remove(path);
    }

    fn dispose_path(&mut self, path: &str) {
        self.events.push(AudioEvent::DisposePath {
            path: path.to_string(),
        });
        self.playing_paths.remove(path);
    }

    fn set_model(&mut self, _model: &BMSModel) {
        self.events.push(AudioEvent::SetModel);
    }

    fn set_additional_key_sound(&mut self, judge: i32, fast: bool, path: Option<&str>) {
        self.events.push(AudioEvent::SetAdditionalKeySound {
            judge,
            fast,
            path: path.map(|s| s.to_string()),
        });
    }

    fn abort(&mut self) {
        self.events.push(AudioEvent::Abort);
    }

    fn get_progress(&self) -> f32 {
        self.progress
    }

    fn preload_path(&mut self, path: &str) {
        self.events.push(AudioEvent::PreloadPath {
            path: path.to_string(),
        });
    }

    fn play_note(&mut self, n: &Note, volume: f32, pitch: i32) {
        self.events.push(AudioEvent::PlayNote {
            wav_id: n.wav(),
            volume,
            pitch,
        });
    }

    fn play_judge(&mut self, judge: i32, fast: bool) {
        self.events.push(AudioEvent::PlayJudge { judge, fast });
    }

    fn stop_note(&mut self, n: Option<&Note>) {
        self.events.push(AudioEvent::StopNote {
            wav_id: n.map(|n| n.wav()),
        });
    }

    fn set_volume_note(&mut self, n: &Note, volume: f32) {
        self.events.push(AudioEvent::SetVolumeNote {
            wav_id: n.wav(),
            volume,
        });
    }

    fn set_global_pitch(&mut self, pitch: f32) {
        self.events.push(AudioEvent::SetGlobalPitch { pitch });
        self.global_pitch = pitch;
    }

    fn get_global_pitch(&self) -> f32 {
        self.global_pitch
    }

    fn dispose_old(&mut self) {
        self.events.push(AudioEvent::DisposeOld);
    }

    fn dispose(&mut self) {
        self.events.push(AudioEvent::Dispose);
        self.playing_paths.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_path_records_event_and_tracks_playing() {
        let mut driver = RecordingAudioDriver::new();

        driver.play_path("bgm.ogg", 0.8, false);

        assert_eq!(driver.events().len(), 1);
        assert_eq!(
            driver.events()[0],
            AudioEvent::PlayPath {
                path: "bgm.ogg".to_string(),
                volume: 0.8,
                loop_play: false,
            }
        );
        assert!(driver.is_playing_path("bgm.ogg"));
        assert!(!driver.is_playing_path("other.ogg"));
    }

    #[test]
    fn stop_path_records_event_and_removes_from_playing() {
        let mut driver = RecordingAudioDriver::new();

        driver.play_path("bgm.ogg", 1.0, false);
        assert!(driver.is_playing_path("bgm.ogg"));

        driver.stop_path("bgm.ogg");
        assert!(!driver.is_playing_path("bgm.ogg"));
        assert_eq!(driver.stop_path_count(), 1);
    }

    #[test]
    fn dispose_path_records_event_and_removes_from_playing() {
        let mut driver = RecordingAudioDriver::new();

        driver.play_path("se.wav", 1.0, false);
        driver.dispose_path("se.wav");

        assert!(!driver.is_playing_path("se.wav"));
        assert_eq!(driver.disposed_paths(), vec!["se.wav".to_string()]);
    }

    #[test]
    fn global_pitch_get_set() {
        let mut driver = RecordingAudioDriver::new();

        assert_eq!(driver.get_global_pitch(), 1.0);
        driver.set_global_pitch(1.5);
        assert_eq!(driver.get_global_pitch(), 1.5);
        assert_eq!(
            driver.events()[0],
            AudioEvent::SetGlobalPitch { pitch: 1.5 }
        );
    }

    #[test]
    fn play_and_stop_note() {
        let mut driver = RecordingAudioDriver::new();
        let note = Note::new_normal(42);

        driver.play_note(&note, 0.9, 0);
        driver.stop_note(Some(&note));

        assert_eq!(driver.events().len(), 2);
        assert_eq!(
            driver.events()[0],
            AudioEvent::PlayNote {
                wav_id: 42,
                volume: 0.9,
                pitch: 0,
            }
        );
        assert_eq!(
            driver.events()[1],
            AudioEvent::StopNote { wav_id: Some(42) }
        );
    }

    #[test]
    fn stop_all_notes() {
        let mut driver = RecordingAudioDriver::new();

        driver.stop_note(None);

        assert_eq!(driver.events()[0], AudioEvent::StopNote { wav_id: None });
    }

    #[test]
    fn progress_defaults_to_complete() {
        let driver = RecordingAudioDriver::new();
        assert_eq!(driver.get_progress(), 1.0);
    }

    #[test]
    fn set_progress() {
        let mut driver = RecordingAudioDriver::new();
        driver.set_progress(0.5);
        assert_eq!(driver.get_progress(), 0.5);
    }

    #[test]
    fn clear_events() {
        let mut driver = RecordingAudioDriver::new();
        driver.play_path("a.wav", 1.0, false);
        driver.play_path("b.wav", 1.0, false);
        assert_eq!(driver.events().len(), 2);

        driver.clear_events();
        assert!(driver.events().is_empty());
        // Playing state is preserved even after clearing events
        assert!(driver.is_playing_path("a.wav"));
    }

    #[test]
    fn count_helpers() {
        let mut driver = RecordingAudioDriver::new();

        driver.play_path("a.wav", 1.0, false);
        driver.play_path("b.wav", 1.0, true);
        driver.stop_path("a.wav");
        let note = Note::new_normal(1);
        driver.stop_note(Some(&note));

        assert_eq!(driver.play_path_count(), 2);
        assert_eq!(driver.stop_path_count(), 1);
        assert_eq!(driver.stop_note_count(), 1);
        assert_eq!(
            driver.played_paths(),
            vec!["a.wav".to_string(), "b.wav".to_string()]
        );
        assert_eq!(driver.stopped_paths(), vec!["a.wav".to_string()]);
    }

    #[test]
    fn dispose_clears_all_playing() {
        let mut driver = RecordingAudioDriver::new();
        driver.play_path("a.wav", 1.0, false);
        driver.play_path("b.wav", 1.0, false);

        driver.dispose();

        assert!(!driver.is_playing_path("a.wav"));
        assert!(!driver.is_playing_path("b.wav"));
    }

    #[test]
    fn set_volume_path_records_event() {
        let mut driver = RecordingAudioDriver::new();
        driver.set_volume_path("bgm.ogg", 0.5);
        assert_eq!(
            driver.events()[0],
            AudioEvent::SetVolumePath {
                path: "bgm.ogg".to_string(),
                volume: 0.5,
            }
        );
    }

    #[test]
    fn play_judge_records_event() {
        let mut driver = RecordingAudioDriver::new();
        driver.play_judge(2, true);
        assert_eq!(
            driver.events()[0],
            AudioEvent::PlayJudge {
                judge: 2,
                fast: true,
            }
        );
    }

    #[test]
    fn set_volume_note_records_event() {
        let mut driver = RecordingAudioDriver::new();
        let note = Note::new_normal(10);
        driver.set_volume_note(&note, 0.7);
        assert_eq!(
            driver.events()[0],
            AudioEvent::SetVolumeNote {
                wav_id: 10,
                volume: 0.7,
            }
        );
    }

    #[test]
    fn loop_play_flag_is_recorded() {
        let mut driver = RecordingAudioDriver::new();
        driver.play_path("loop.ogg", 1.0, true);
        assert_eq!(
            driver.events()[0],
            AudioEvent::PlayPath {
                path: "loop.ogg".to_string(),
                volume: 1.0,
                loop_play: true,
            }
        );
    }
}

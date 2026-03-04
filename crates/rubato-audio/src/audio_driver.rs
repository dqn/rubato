use std::path::{Path, PathBuf};

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;

/// Audio driver interface for playing various audio sources.
///
/// Translated from: AudioDriver.java
pub trait AudioDriver {
    /// Play audio at the specified path
    fn play_path(&mut self, path: &str, volume: f32, loop_play: bool);

    /// Set volume for the audio at the specified path
    fn set_volume_path(&mut self, path: &str, volume: f32);

    /// Returns true if the audio at the specified path is playing
    fn is_playing_path(&self, path: &str) -> bool;

    /// Stop the audio at the specified path
    fn stop_path(&mut self, path: &str);

    /// Dispose the audio at the specified path
    fn dispose_path(&mut self, path: &str);

    /// Load BMS audio data
    fn set_model(&mut self, model: &BMSModel);

    /// Define additional key sounds for judgement
    fn set_additional_key_sound(&mut self, judge: i32, fast: bool, path: Option<&str>);

    /// Abort loading BMS audio data
    fn abort(&mut self);

    /// Returns audio loading progress (0.0 - 1.0)
    fn get_progress(&self) -> f32;

    /// Play the sound for the specified Note
    fn play_note(&mut self, n: &Note, volume: f32, pitch: i32);

    /// Play additional key sound for judgement
    fn play_judge(&mut self, judge: i32, fast: bool);

    /// Stop the sound for the specified Note. If None, stop all sounds
    fn stop_note(&mut self, n: Option<&Note>);

    /// Set volume for the specified Note
    fn set_volume_note(&mut self, n: &Note, volume: f32);

    /// Set global pitch (0.5 - 2.0). Changes pitch of currently playing sounds if possible
    fn set_global_pitch(&mut self, pitch: f32);

    /// Get global pitch (0.5 - 2.0)
    fn get_global_pitch(&self) -> f32;

    /// Dispose old audio resources
    fn dispose_old(&mut self);

    /// Dispose all resources
    fn dispose(&mut self);
}

/// Get all supported audio file paths for the given path.
///
/// Tries the original path and alternate extensions (.wav, .flac, .ogg, .mp3).
pub fn get_paths(path: &str) -> Vec<PathBuf> {
    let exts = [".wav", ".flac", ".ogg", ".mp3"];

    let mut result: Vec<PathBuf> = Vec::new();
    let index = path.rfind('.');
    let name = &path[0..index.unwrap_or(path.len())];
    let ext = if let Some(idx) = index {
        &path[idx..path.len()]
    } else {
        ""
    };

    let p = Path::new(path);
    if p.exists() {
        result.push(p.to_path_buf());
    }

    for _ext in &exts {
        if *_ext != ext {
            let p2 = PathBuf::from(format!("{}{}", name, _ext));
            if p2.exists() {
                result.push(p2);
            }
        }
    }

    result
}

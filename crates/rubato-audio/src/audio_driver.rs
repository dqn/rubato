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

    /// Load BMS audio data. This initiates background loading of keysound files.
    /// Call `poll_loading()` each frame to check for completion and finalize the
    /// wav_sounds/slicesound maps.
    fn set_model(&mut self, model: &BMSModel);

    /// Define additional key sounds for judgement
    fn set_additional_key_sound(&mut self, judge: i32, fast: bool, path: Option<&str>);

    /// Abort loading BMS audio data
    fn abort(&mut self);

    /// Returns audio loading progress (0.0 - 1.0)
    fn get_progress(&self) -> f32;

    /// Poll background loading completion. Returns true when loading is finished
    /// (or if no loading is in progress). Should be called each frame from the
    /// render loop.
    fn poll_loading(&mut self) -> bool {
        true
    }

    /// Preload a sound file into the path sound cache without playing it.
    /// Call during state `create()` for known system sounds to avoid blocking
    /// I/O on first `play_path()`.
    fn preload_path(&mut self, _path: &str) {}

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
pub fn paths(path: &str) -> Vec<PathBuf> {
    let exts = [".wav", ".flac", ".ogg", ".mp3"];

    let mut result: Vec<PathBuf> = Vec::new();
    let p_path = Path::new(path);
    let ext = p_path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let name = {
        let stem = p_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Some(parent) = p_path.parent() {
            parent.join(&stem).to_string_lossy().to_string()
        } else {
            stem
        }
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

/// Check whether a WAV/BMP resource path from a BMS file is safe (no directory traversal).
///
/// Returns `true` if the path does not contain `..` components that would escape
/// the BMS file's directory. BMS resource paths are relative filenames (e.g., "kick.wav",
/// "sfx/hit.ogg") and should never traverse upward.
///
/// This is a security measure to prevent malicious BMS files from reading arbitrary
/// files via paths like `../../../../etc/passwd`.
pub fn is_bms_resource_path_safe(resource_name: &str) -> bool {
    let path = Path::new(resource_name);
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return false;
        }
        // Reject absolute paths on any platform
        if matches!(
            component,
            std::path::Component::RootDir | std::path::Component::Prefix(_)
        ) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bms_resource_path_allows_simple_filename() {
        assert!(is_bms_resource_path_safe("kick.wav"));
    }

    #[test]
    fn bms_resource_path_allows_subdirectory() {
        assert!(is_bms_resource_path_safe("sfx/hit.ogg"));
    }

    #[test]
    fn bms_resource_path_rejects_parent_traversal() {
        assert!(!is_bms_resource_path_safe("../../../etc/passwd"));
    }

    #[test]
    fn bms_resource_path_rejects_mid_traversal() {
        assert!(!is_bms_resource_path_safe("sfx/../../secret.wav"));
    }

    #[test]
    fn bms_resource_path_rejects_absolute() {
        assert!(!is_bms_resource_path_safe("/etc/passwd"));
    }

    #[test]
    fn bms_resource_path_allows_dotfile() {
        // ".hidden.wav" is a valid filename, not a traversal
        assert!(is_bms_resource_path_safe(".hidden.wav"));
    }

    #[test]
    fn bms_resource_path_allows_current_dir() {
        // "./kick.wav" is fine
        assert!(is_bms_resource_path_safe("./kick.wav"));
    }
}

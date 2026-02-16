/// Unified audio loader.
///
/// Dispatches to the appropriate decoder based on file extension.
/// Provides fallback search across formats (`.wav` → `.flac` → `.ogg` → `.mp3`).
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::flac;
use crate::mp3;
use crate::ogg;
use crate::pcm::Pcm;
use crate::wav;

/// Supported audio file extensions, in search priority order.
const AUDIO_EXTENSIONS: &[&str] = &[".wav", ".flac", ".ogg", ".mp3"];

/// Load audio from a file path, dispatching to the correct decoder.
pub fn load_audio(path: &Path) -> Result<Pcm> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "wav" => {
            let mut file = std::fs::File::open(path)?;
            wav::decode(&mut file)
        }
        "ogg" => ogg::decode_file(path),
        "flac" => flac::decode_file(path),
        "mp3" => mp3::decode_file(path),
        _ => bail!("Unsupported audio format: .{ext}"),
    }
}

/// Resolve the audio file path by trying alternative extensions.
///
/// Given a base directory and a filename (e.g., "sound.wav"), searches for
/// the file with the original extension first, then tries `.wav`, `.flac`,
/// `.ogg`, `.mp3` in order.
///
/// Ports Java `AudioDriver.getPaths()`.
pub fn resolve_audio_path(base: &Path, name: &str) -> Option<PathBuf> {
    // Try the original path first
    let original = base.join(name);
    if original.exists() {
        return Some(original);
    }

    // Extract stem (without extension)
    let stem = match name.rfind('.') {
        Some(idx) => &name[..idx],
        None => name,
    };

    let original_ext = name.rfind('.').map(|idx| &name[idx..]);

    // Try each supported extension
    for &ext in AUDIO_EXTENSIONS {
        // Skip the original extension (already tried)
        if Some(ext) == original_ext {
            continue;
        }
        let candidate = base.join(format!("{stem}{ext}"));
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_resolve_audio_path_not_found() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("bms_audio_test_resolve_{}", id));
        let _ = fs::create_dir_all(&dir);
        assert!(resolve_audio_path(&dir, "nonexistent.wav").is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_audio_path_exact_match() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("bms_audio_test_resolve_exact_{}", id));
        let _ = fs::create_dir_all(&dir);

        let file_path = dir.join("test.wav");
        fs::write(&file_path, b"dummy").unwrap();

        let result = resolve_audio_path(&dir, "test.wav");
        assert_eq!(result, Some(file_path));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_audio_path_fallback() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("bms_audio_test_resolve_fallback_{}", id));
        let _ = fs::create_dir_all(&dir);

        // Only .ogg exists, but we search for .wav
        let ogg_path = dir.join("test.ogg");
        fs::write(&ogg_path, b"dummy").unwrap();

        let result = resolve_audio_path(&dir, "test.wav");
        // Should skip .wav (not found), skip .flac (not found), find .ogg
        assert_eq!(result, Some(ogg_path));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_audio_unsupported() {
        let result = load_audio(Path::new("/tmp/test.xyz"));
        assert!(result.is_err());
    }
}

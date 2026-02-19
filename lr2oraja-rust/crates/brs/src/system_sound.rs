// System sound playback manager with Kira audio integration.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use kira::manager::{AudioManager, AudioManagerSettings, backend::DefaultBackend};
use kira::sound::static_sound::StaticSoundData;
use tracing::warn;

use bms_audio::pcm::Pcm;

/// Convert Pcm (f32 interleaved) to WAV bytes in memory.
fn pcm_to_wav_bytes(pcm: &Pcm) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: pcm.channels,
        sample_rate: pcm.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::new(&mut cursor, spec).expect("WAV writer creation");
    for &sample in &pcm.samples {
        writer.write_sample(sample).expect("WAV sample write");
    }
    writer.finalize().expect("WAV finalize");
    cursor.into_inner()
}

/// System sound types for state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemSound {
    Decide,
    ResultClear,
    ResultFail,
    Select,
    #[allow(dead_code)] // Parsed for completeness (Java SystemSound enum)
    Scratch,
    Folder,
    OptionChange,
}

/// Manages system sound playback with Kira audio integration.
pub struct SystemSoundManager {
    /// Queue of sounds to play this frame.
    queue: Vec<SystemSound>,
    /// Paths to system sound files, loaded from config.
    sound_paths: HashMap<SystemSound, PathBuf>,
    /// Kira audio manager for playback.
    audio_manager: Option<AudioManager>,
    /// Preloaded sound data keyed by sound type.
    loaded_sounds: HashMap<SystemSound, StaticSoundData>,
    /// Playback volume (0.0-1.0).
    volume: f64,
}

impl Default for SystemSoundManager {
    fn default() -> Self {
        let audio_manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| warn!("Failed to create system sound audio manager: {e}"))
            .ok();
        Self {
            queue: Vec::new(),
            sound_paths: HashMap::new(),
            audio_manager,
            loaded_sounds: HashMap::new(),
            volume: 1.0,
        }
    }
}

impl SystemSoundManager {
    #[allow(dead_code)] // Used in tests
    pub fn new() -> Self {
        Self::default()
    }

    /// Load system sound file paths from the given base directory and preload audio data.
    pub fn load_sounds(&mut self, base_dir: &Path) {
        let sound_files = [
            (SystemSound::Decide, "decide.wav"),
            (SystemSound::Select, "select.wav"),
            (SystemSound::Folder, "folder.wav"),
            (SystemSound::ResultClear, "clear.wav"),
            (SystemSound::ResultFail, "fail.wav"),
            (SystemSound::Scratch, "scratch.wav"),
            (SystemSound::OptionChange, "option.wav"),
        ];
        for (sound, filename) in &sound_files {
            let path = base_dir.join(filename);
            if path.exists() {
                self.sound_paths.insert(*sound, path.clone());
                // Preload audio data via bms_audio decode -> PCM -> WAV -> StaticSoundData
                match bms_audio::decode::load_audio(&path) {
                    Ok(pcm) => {
                        let wav_bytes = pcm_to_wav_bytes(&pcm);
                        match StaticSoundData::from_cursor(Cursor::new(wav_bytes)) {
                            Ok(sound_data) => {
                                self.loaded_sounds.insert(*sound, sound_data);
                            }
                            Err(e) => {
                                warn!(
                                    path = %path.display(),
                                    "Failed to create StaticSoundData for system sound: {e}"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            path = %path.display(),
                            "Failed to load system sound audio: {e}"
                        );
                    }
                }
            }
        }
    }

    /// Get the file path for a sound type (if loaded).
    #[allow(dead_code)] // Used in tests
    pub fn sound_path(&self, sound: SystemSound) -> Option<&Path> {
        self.sound_paths.get(&sound).map(|p| p.as_path())
    }

    /// Set playback volume (0.0-1.0).
    pub fn set_volume(&mut self, volume: f64) {
        self.volume = volume;
    }

    /// Queue a sound for playback and immediately play via Kira.
    pub fn play(&mut self, sound: SystemSound) {
        self.queue.push(sound);
        // Kira immediate playback
        if let Some(manager) = &mut self.audio_manager
            && let Some(data) = self.loaded_sounds.get(&sound)
            && let Err(e) = manager.play(data.clone().volume(self.volume))
        {
            warn!(?sound, "Failed to play system sound: {e}");
        }
    }

    /// Drain the queue (consumed by audio system each frame).
    #[allow(dead_code)] // Used in tests
    pub fn drain(&mut self) -> Vec<SystemSound> {
        std::mem::take(&mut self.queue)
    }

    /// Check if queue is empty.
    #[allow(dead_code)] // Used in tests
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let mgr = SystemSoundManager::new();
        assert!(mgr.is_empty());
    }

    #[test]
    fn play_adds_to_queue() {
        let mut mgr = SystemSoundManager::new();
        mgr.play(SystemSound::Decide);
        mgr.play(SystemSound::Select);
        assert!(!mgr.is_empty());
    }

    #[test]
    fn drain_returns_and_clears_queue() {
        let mut mgr = SystemSoundManager::new();
        mgr.play(SystemSound::ResultClear);
        mgr.play(SystemSound::Folder);
        let drained = mgr.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0], SystemSound::ResultClear);
        assert_eq!(drained[1], SystemSound::Folder);
        assert!(mgr.is_empty());
    }

    #[test]
    fn set_volume_updates_volume() {
        let mut mgr = SystemSoundManager::new();
        mgr.set_volume(0.5);
        assert!((mgr.volume - 0.5).abs() < f64::EPSILON);
    }
}

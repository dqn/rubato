use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::info;

// SoundType moved to beatoraja-types (Phase 59a)
pub use beatoraja_types::sound_type::SoundType;

/// SystemSoundManager - manages BGM and sound effect sets
pub struct SystemSoundManager {
    /// Detected BGM set directory paths
    bgms: Vec<PathBuf>,
    /// Current BGM set directory path
    current_bgm_path: Option<PathBuf>,
    /// Detected sound effect set directory paths
    sounds: Vec<PathBuf>,
    /// Current sound effect set directory path
    current_sound_path: Option<PathBuf>,
    /// Sound path map
    soundmap: HashMap<SoundType, String>,
}

impl SystemSoundManager {
    pub fn new(bgmpath: Option<&str>, soundpath: Option<&str>) -> Self {
        let mut bgms = Vec::new();
        let mut sounds = Vec::new();

        if let Some(bp) = bgmpath
            && !bp.is_empty()
        {
            let abs = Path::new(bp)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(bp));
            Self::scan(&abs, &mut bgms, "select.wav");
        }

        if let Some(sp) = soundpath
            && !sp.is_empty()
        {
            let abs = Path::new(sp)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(sp));
            Self::scan(&abs, &mut sounds, "clear.wav");
        }

        info!(
            "Detected BGM Set: {} Sound Set: {}",
            bgms.len(),
            sounds.len()
        );

        Self {
            bgms,
            current_bgm_path: None,
            sounds,
            current_sound_path: None,
            soundmap: HashMap::new(),
        }
    }

    pub fn shuffle(&mut self) {
        if !self.bgms.is_empty() {
            let idx = (rand_f64() * self.bgms.len() as f64) as usize;
            self.current_bgm_path = Some(self.bgms[idx.min(self.bgms.len() - 1)].clone());
        }
        if !self.sounds.is_empty() {
            let idx = (rand_f64() * self.sounds.len() as f64) as usize;
            self.current_sound_path = Some(self.sounds[idx.min(self.sounds.len() - 1)].clone());
        }
        info!(
            "BGM Set: {:?} Sound Set: {:?}",
            self.current_bgm_path, self.current_sound_path
        );

        for sound in SoundType::values() {
            let paths = self.get_sound_paths(sound);
            if let Some(first_path) = paths.first() {
                let newpath = first_path.to_string_lossy().to_string();
                let oldpath = self.soundmap.get(sound).cloned();
                if Some(&newpath) == oldpath.as_ref() && *sound != SoundType::Select {
                    continue;
                }
                // In Java: main.getAudioProcessor().dispose(oldpath)
                // Phase 5+ dependency
                self.soundmap.insert(sound.clone(), newpath);
            }
        }
    }

    pub fn get_bgm_path(&self) -> Option<&Path> {
        self.current_bgm_path.as_deref()
    }

    pub fn get_sound_path(&self) -> Option<&Path> {
        self.current_sound_path.as_deref()
    }

    fn scan(p: &Path, paths: &mut Vec<PathBuf>, name: &str) {
        if p.is_dir() {
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    Self::scan(&entry.path(), paths, name);
                }
            }
            // Check if the sound file exists in this directory
            let sound_path = p.join(name);
            if Self::get_audio_paths(&sound_path.to_string_lossy()).is_some() {
                paths.push(p.to_path_buf());
            }
        }
    }

    pub fn get_sound_paths(&self, sound_type: &SoundType) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let p = if sound_type.is_bgm() {
            &self.current_bgm_path
        } else {
            &self.current_sound_path
        };

        if let Some(base) = p {
            let resolved = base.join(sound_type.path());
            if let Some(audio_paths) = Self::get_audio_paths(&resolved.to_string_lossy()) {
                paths.extend(audio_paths);
            }
        }

        // Default sound path
        let default_path = PathBuf::from("defaultsound").join(sound_type.path());
        if let Some(audio_paths) = Self::get_audio_paths(&default_path.to_string_lossy()) {
            paths.extend(audio_paths);
        }

        paths
    }

    /// Get audio file paths (AudioDriver.getPaths equivalent)
    fn get_audio_paths(path: &str) -> Option<Vec<PathBuf>> {
        let p = Path::new(path);
        if p.exists() {
            Some(vec![p.to_path_buf()])
        } else {
            // Try common audio extensions
            let base = p.with_extension("");
            let extensions = ["wav", "ogg", "mp3", "flac"];
            let mut found = Vec::new();
            for ext in &extensions {
                let candidate = base.with_extension(ext);
                if candidate.exists() {
                    found.push(candidate);
                }
            }
            if found.is_empty() { None } else { Some(found) }
        }
    }

    pub fn get_sound(&self, sound: &SoundType) -> Option<&String> {
        self.soundmap.get(sound)
    }

    /// Play a system sound effect or BGM.
    ///
    /// Translated from: SystemSoundManager.play() (Java lines 119-121)
    ///
    /// When an audio driver is provided, plays the sound at the given system volume.
    /// Without an audio driver, this is a no-op (useful for testing).
    pub fn play(
        &self,
        sound: &SoundType,
        loop_sound: bool,
        audio: Option<&mut dyn beatoraja_audio::audio_driver::AudioDriver>,
        system_volume: f32,
    ) {
        if let Some(path) = self.soundmap.get(sound)
            && let Some(audio) = audio
        {
            audio.play_path(path, system_volume, loop_sound);
        }
    }

    /// Stop a system sound effect or BGM.
    ///
    /// Translated from: SystemSoundManager.stop() (Java lines 126-128)
    pub fn stop(
        &self,
        sound: &SoundType,
        audio: Option<&mut dyn beatoraja_audio::audio_driver::AudioDriver>,
    ) {
        if let Some(path) = self.soundmap.get(sound)
            && let Some(audio) = audio
        {
            audio.stop_path(path);
        }
    }

    /// Dispose a sound (called when sound set changes).
    ///
    /// Translated from: SystemSoundManager.shuffle() dispose call (Java line 73)
    pub fn dispose_sound(
        &self,
        path: &str,
        audio: Option<&mut dyn beatoraja_audio::audio_driver::AudioDriver>,
    ) {
        if let Some(audio) = audio {
            audio.dispose_path(path);
        }
    }
}

/// Simple random f64 in [0, 1) - equivalent to Math.random()
fn rand_f64() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos as f64) / (u32::MAX as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock AudioDriver for testing play/stop/dispose calls.
    struct MockAudioDriver {
        played: Vec<(String, f32, bool)>,
        stopped: Vec<String>,
        disposed: Vec<String>,
    }

    impl MockAudioDriver {
        fn new() -> Self {
            Self {
                played: Vec::new(),
                stopped: Vec::new(),
                disposed: Vec::new(),
            }
        }
    }

    impl beatoraja_audio::audio_driver::AudioDriver for MockAudioDriver {
        fn play_path(&mut self, path: &str, volume: f32, loop_play: bool) {
            self.played.push((path.to_string(), volume, loop_play));
        }
        fn set_volume_path(&mut self, _path: &str, _volume: f32) {}
        fn is_playing_path(&self, _path: &str) -> bool {
            false
        }
        fn stop_path(&mut self, path: &str) {
            self.stopped.push(path.to_string());
        }
        fn dispose_path(&mut self, path: &str) {
            self.disposed.push(path.to_string());
        }
        fn set_model(&mut self, _model: &bms_model::bms_model::BMSModel) {}
        fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}
        fn abort(&mut self) {}
        fn get_progress(&self) -> f32 {
            0.0
        }
        fn play_note(&mut self, _n: &bms_model::note::Note, _volume: f32, _pitch: i32) {}
        fn play_judge(&mut self, _judge: i32, _fast: bool) {}
        fn stop_note(&mut self, _n: Option<&bms_model::note::Note>) {}
        fn set_volume_note(&mut self, _n: &bms_model::note::Note, _volume: f32) {}
        fn set_global_pitch(&mut self, _pitch: f32) {}
        fn get_global_pitch(&self) -> f32 {
            1.0
        }
        fn dispose_old(&mut self) {}
        fn dispose(&mut self) {}
    }

    #[test]
    fn play_calls_audio_driver() {
        let mut sm = SystemSoundManager::new(None, None);
        // Manually insert a sound path
        sm.soundmap
            .insert(SoundType::PlayReady, "test/ready.wav".to_string());

        let mut audio = MockAudioDriver::new();
        sm.play(&SoundType::PlayReady, false, Some(&mut audio), 0.8);
        assert_eq!(audio.played.len(), 1);
        assert_eq!(audio.played[0].0, "test/ready.wav");
        assert!((audio.played[0].1 - 0.8).abs() < f32::EPSILON);
        assert!(!audio.played[0].2);
    }

    #[test]
    fn play_loop_passes_loop_flag() {
        let mut sm = SystemSoundManager::new(None, None);
        sm.soundmap
            .insert(SoundType::Select, "test/select.wav".to_string());

        let mut audio = MockAudioDriver::new();
        sm.play(&SoundType::Select, true, Some(&mut audio), 1.0);
        assert_eq!(audio.played.len(), 1);
        assert!(audio.played[0].2); // loop = true
    }

    #[test]
    fn stop_calls_audio_driver() {
        let mut sm = SystemSoundManager::new(None, None);
        sm.soundmap
            .insert(SoundType::PlayStop, "test/stop.wav".to_string());

        let mut audio = MockAudioDriver::new();
        sm.stop(&SoundType::PlayStop, Some(&mut audio));
        assert_eq!(audio.stopped.len(), 1);
        assert_eq!(audio.stopped[0], "test/stop.wav");
    }

    #[test]
    fn play_without_audio_driver_is_noop() {
        let mut sm = SystemSoundManager::new(None, None);
        sm.soundmap
            .insert(SoundType::PlayReady, "test/ready.wav".to_string());
        // Should not panic
        sm.play(&SoundType::PlayReady, false, None, 0.5);
    }

    #[test]
    fn stop_without_audio_driver_is_noop() {
        let mut sm = SystemSoundManager::new(None, None);
        sm.soundmap
            .insert(SoundType::PlayStop, "test/stop.wav".to_string());
        // Should not panic
        sm.stop(&SoundType::PlayStop, None);
    }

    #[test]
    fn play_missing_sound_is_noop() {
        let sm = SystemSoundManager::new(None, None);
        let mut audio = MockAudioDriver::new();
        // No sound in the map
        sm.play(&SoundType::PlayReady, false, Some(&mut audio), 0.5);
        assert!(audio.played.is_empty());
    }

    #[test]
    fn dispose_sound_calls_audio_driver() {
        let sm = SystemSoundManager::new(None, None);
        let mut audio = MockAudioDriver::new();
        sm.dispose_sound("old/path.wav", Some(&mut audio));
        assert_eq!(audio.disposed.len(), 1);
        assert_eq!(audio.disposed[0], "old/path.wav");
    }
}

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::info;

/// SoundType - system sound types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SoundType {
    Scratch,
    FolderOpen,
    FolderClose,
    OptionChange,
    OptionOpen,
    OptionClose,
    PlayReady,
    PlayStop,
    ResultClear,
    ResultFail,
    ResultClose,
    CourseClear,
    CourseFail,
    CourseClose,
    GuidesePg,
    GuideseGr,
    GuideseGd,
    GuideseBd,
    GuidesePr,
    GuideseMs,
    Select,
    Decide,
}

impl SoundType {
    pub fn is_bgm(&self) -> bool {
        matches!(self, SoundType::Select | SoundType::Decide)
    }

    pub fn path(&self) -> &str {
        match self {
            SoundType::Scratch => "scratch.wav",
            SoundType::FolderOpen => "f-open.wav",
            SoundType::FolderClose => "f-close.wav",
            SoundType::OptionChange => "o-change.wav",
            SoundType::OptionOpen => "o-open.wav",
            SoundType::OptionClose => "o-close.wav",
            SoundType::PlayReady => "playready.wav",
            SoundType::PlayStop => "playstop.wav",
            SoundType::ResultClear => "clear.wav",
            SoundType::ResultFail => "fail.wav",
            SoundType::ResultClose => "resultclose.wav",
            SoundType::CourseClear => "course_clear.wav",
            SoundType::CourseFail => "course_fail.wav",
            SoundType::CourseClose => "course_close.wav",
            SoundType::GuidesePg => "guide-pg.wav",
            SoundType::GuideseGr => "guide-gr.wav",
            SoundType::GuideseGd => "guide-gd.wav",
            SoundType::GuideseBd => "guide-bd.wav",
            SoundType::GuidesePr => "guide-pr.wav",
            SoundType::GuideseMs => "guide-ms.wav",
            SoundType::Select => "select.wav",
            SoundType::Decide => "decide.wav",
        }
    }

    pub fn values() -> &'static [SoundType] {
        &[
            SoundType::Scratch,
            SoundType::FolderOpen,
            SoundType::FolderClose,
            SoundType::OptionChange,
            SoundType::OptionOpen,
            SoundType::OptionClose,
            SoundType::PlayReady,
            SoundType::PlayStop,
            SoundType::ResultClear,
            SoundType::ResultFail,
            SoundType::ResultClose,
            SoundType::CourseClear,
            SoundType::CourseFail,
            SoundType::CourseClose,
            SoundType::GuidesePg,
            SoundType::GuideseGr,
            SoundType::GuideseGd,
            SoundType::GuideseBd,
            SoundType::GuidesePr,
            SoundType::GuideseMs,
            SoundType::Select,
            SoundType::Decide,
        ]
    }
}

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

    pub fn play(&self, sound: &SoundType, _loop_sound: bool) {
        if let Some(_path) = self.soundmap.get(sound) {
            // main.getAudioProcessor().play(path, systemvolume, loop)
            // Phase 5+ dependency
        }
    }

    pub fn stop(&self, sound: &SoundType) {
        if let Some(_path) = self.soundmap.get(sound) {
            // main.getAudioProcessor().stop(path)
            // Phase 5+ dependency
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

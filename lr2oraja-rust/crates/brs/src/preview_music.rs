// PreviewMusicProcessor — preview music playback for the select screen.
//
// Ports Java PreviewMusicProcessor: plays a preview of the selected song
// after a short delay, falling back to the default select screen BGM.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::Result;
use kira::manager::{AudioManager, AudioManagerSettings, backend::DefaultBackend};
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::tween::Tween;
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

/// Delay in milliseconds before starting preview playback after cursor change.
pub const PREVIEW_DELAY_MS: i64 = 400;

/// Manages preview music playback for the select screen.
///
/// Plays a default select screen BGM, switching to song previews when
/// the cursor rests on a song bar for longer than `PREVIEW_DELAY_MS`.
pub struct PreviewMusicProcessor {
    manager: AudioManager,
    /// Currently playing preview sound handle.
    current_handle: Option<StaticSoundHandle>,
    /// Default BGM handle (select screen BGM).
    default_handle: Option<StaticSoundHandle>,
    /// Default BGM sound data (cached for replay after preview ends).
    default_sound: Option<StaticSoundData>,
    /// Path of the currently playing preview.
    current_path: Option<PathBuf>,
    /// Whether the default BGM is currently playing (vs a preview).
    is_default_playing: bool,
    /// System volume (0.0-1.0).
    volume: f64,
}

impl PreviewMusicProcessor {
    /// Create a new PreviewMusicProcessor with its own AudioManager.
    pub fn new() -> Result<Self> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| anyhow::anyhow!("Failed to create preview audio manager: {e}"))?;
        Ok(Self {
            manager,
            current_handle: None,
            default_handle: None,
            default_sound: None,
            current_path: None,
            is_default_playing: false,
            volume: 1.0,
        })
    }

    /// Load and start looping the default select screen BGM.
    pub fn set_default(&mut self, path: &Path) {
        match bms_audio::decode::load_audio(path) {
            Ok(pcm) => {
                let wav_bytes = pcm_to_wav_bytes(&pcm);
                match StaticSoundData::from_cursor(Cursor::new(wav_bytes)) {
                    Ok(sound_data) => {
                        let looped = sound_data.loop_region(..);
                        self.default_sound = Some(looped);
                        self.play_default();
                    }
                    Err(e) => warn!("Failed to create default BGM data: {e}"),
                }
            }
            Err(e) => warn!(path = %path.display(), "Failed to load default BGM: {e}"),
        }
    }

    /// Start preview playback. If `preview_path` is `None`, fall back to default BGM.
    pub fn start_preview(&mut self, preview_path: Option<&Path>, loop_play: bool) {
        match preview_path {
            Some(path) => {
                // Already playing this preview — skip
                if self.current_path.as_deref() == Some(path) {
                    return;
                }

                self.stop_all_handles();

                match bms_audio::decode::load_audio(path) {
                    Ok(pcm) => {
                        let wav_bytes = pcm_to_wav_bytes(&pcm);
                        match StaticSoundData::from_cursor(Cursor::new(wav_bytes)) {
                            Ok(sound_data) => {
                                let data = if loop_play {
                                    sound_data.volume(self.volume).loop_region(..)
                                } else {
                                    sound_data.volume(self.volume)
                                };
                                match self.manager.play(data) {
                                    Ok(handle) => {
                                        self.current_handle = Some(handle);
                                        self.current_path = Some(path.to_path_buf());
                                        self.is_default_playing = false;
                                    }
                                    Err(e) => warn!("Failed to play preview: {e}"),
                                }
                            }
                            Err(e) => warn!("Failed to create preview sound data: {e}"),
                        }
                    }
                    Err(e) => {
                        warn!(path = %path.display(), "Failed to load preview audio: {e}");
                        self.play_default();
                    }
                }
            }
            None => {
                if !self.is_default_playing {
                    self.stop_all_handles();
                    self.play_default();
                }
            }
        }
    }

    /// Called every frame. Detects when a non-looping preview finishes
    /// and switches back to the default BGM.
    pub fn update(&mut self) {
        if !self.is_default_playing
            && let Some(handle) = &self.current_handle
            && handle.state() == PlaybackState::Stopped
        {
            self.current_handle = None;
            self.current_path = None;
            self.play_default();
        }
    }

    /// Set playback volume (0.0-1.0).
    pub fn set_volume(&mut self, volume: f64) {
        self.volume = volume;
        if let Some(handle) = &mut self.current_handle {
            handle.set_volume(volume, Tween::default());
        }
        if let Some(handle) = &mut self.default_handle {
            handle.set_volume(volume, Tween::default());
        }
    }

    /// Stop all playback and release handles.
    pub fn stop(&mut self) {
        self.stop_all_handles();
    }

    /// Whether a song preview (not default BGM) is currently playing.
    #[allow(dead_code)] // TODO: integrate with select screen skin state
    pub fn is_playing_preview(&self) -> bool {
        !self.is_default_playing && self.current_handle.is_some()
    }

    /// Stop both preview and default BGM handles.
    fn stop_all_handles(&mut self) {
        if let Some(mut handle) = self.current_handle.take() {
            handle.stop(Tween::default());
        }
        if let Some(mut handle) = self.default_handle.take() {
            handle.stop(Tween::default());
        }
        self.current_path = None;
        self.is_default_playing = false;
    }

    /// Start playing the default BGM (cached sound data).
    fn play_default(&mut self) {
        if let Some(sound_data) = &self.default_sound {
            let data = sound_data.clone().volume(self.volume);
            match self.manager.play(data) {
                Ok(handle) => {
                    self.default_handle = Some(handle);
                    self.is_default_playing = true;
                }
                Err(e) => warn!("Failed to play default BGM: {e}"),
            }
        }
    }
}

/// Audio driver trait and implementation.
///
/// Ports Java `AudioDriver.java` / `AbstractAudioDriver.java`.
/// Manages key sounds, sliced sounds, and audio playback.
use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use bms_model::{BgNote, BmsModel, Note};

use crate::decode;
use crate::pcm::Pcm;

/// Identifies a sound slice for caching.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SliceKey {
    pub wav_id: u16,
    pub micro_starttime: i64,
    pub micro_duration: i64,
}

/// A loaded audio slice ready for playback.
#[derive(Debug, Clone)]
pub struct SliceWav {
    pub micro_starttime: i64,
    pub micro_duration: i64,
    pub pcm: Pcm,
}

/// Audio driver interface for BMS key sound playback.
pub trait AudioDriver: Send + Sync {
    /// Load all WAV files referenced by the model.
    fn set_model(&mut self, model: &BmsModel, base_path: &Path) -> Result<()>;

    /// Play a note's key sound with volume and pitch shift.
    fn play_note(&mut self, note: &Note, volume: f32, pitch_shift: i32);

    /// Play a BG note.
    fn play_bg_note(&mut self, bg_note: &BgNote, volume: f32);

    /// Stop a note's key sound.
    fn stop_note(&mut self, note: &Note);

    /// Stop all currently playing sounds.
    fn stop_all(&mut self);

    /// Set the volume for a note's key sound.
    fn set_note_volume(&mut self, note: &Note, volume: f32);

    /// Set the global pitch multiplier (0.5 - 2.0).
    fn set_global_pitch(&mut self, pitch: f32);

    /// Get the global pitch multiplier.
    fn global_pitch(&self) -> f32;

    /// Get the loading progress (0.0 - 1.0).
    fn progress(&self) -> f32;

    /// Check if the audio driver needs recovery from accumulated errors.
    fn needs_recovery(&self) -> bool {
        false
    }

    /// Attempt to recover from audio failures (e.g., recreate the audio backend).
    fn try_recover(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Compute the channel ID for a note + pitch combination.
///
/// Java: `id * 256 + pitch + 128`
pub fn channel_id(wav_id: u16, pitch_shift: i32) -> i32 {
    wav_id as i32 * 256 + pitch_shift + 128
}

/// Compute the pitch multiplier from a semitone shift.
///
/// Java: `(float)Math.pow(2.0, pitchShift / 12.0)`
pub fn pitch_from_shift(pitch_shift: i32) -> f32 {
    if pitch_shift == 0 {
        1.0
    } else {
        2.0f32.powf(pitch_shift as f32 / 12.0)
    }
}

/// Offline audio driver that loads PCM data but doesn't play anything.
///
/// Used by BmsRenderer and LoudnessAnalyzer.
pub struct OfflineAudioDriver {
    /// Full (un-sliced) WAV data indexed by wav_id.
    pub wav_map: HashMap<u16, Pcm>,
    /// Sliced WAV data indexed by wav_id.
    pub slice_map: HashMap<u16, Vec<SliceWav>>,
    /// Base volume multiplier.
    pub volume: f32,
    /// Global pitch multiplier.
    pub global_pitch: f32,
    /// Target sample rate for all loaded audio.
    pub target_sample_rate: u32,
    /// Target channel count for all loaded audio.
    pub target_channels: u16,
}

impl OfflineAudioDriver {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            wav_map: HashMap::new(),
            slice_map: HashMap::new(),
            volume: 1.0,
            global_pitch: 1.0,
            target_sample_rate: sample_rate,
            target_channels: channels,
        }
    }

    /// Look up the PCM for a note, handling slicing.
    pub fn get_pcm_for_note(
        &self,
        wav_id: u16,
        micro_starttime: i64,
        micro_duration: i64,
    ) -> Option<&Pcm> {
        if micro_starttime == 0 && micro_duration == 0 {
            self.wav_map.get(&wav_id)
        } else if let Some(slices) = self.slice_map.get(&wav_id) {
            slices
                .iter()
                .find(|s| {
                    s.micro_starttime == micro_starttime && s.micro_duration == micro_duration
                })
                .map(|s| &s.pcm)
        } else {
            None
        }
    }
}

impl AudioDriver for OfflineAudioDriver {
    fn set_model(&mut self, model: &BmsModel, base_path: &Path) -> Result<()> {
        self.wav_map.clear();
        self.slice_map.clear();

        // Collect all unique (wav_id, starttime, duration) tuples needed
        let mut needed: HashMap<u16, Vec<(i64, i64)>> = HashMap::new();

        for note in &model.notes {
            let entry = needed.entry(note.wav_id).or_default();
            let key = (note.micro_starttime, note.micro_duration);
            if !entry.contains(&key) {
                entry.push(key);
            }
        }

        for bg in &model.bg_notes {
            let entry = needed.entry(bg.wav_id).or_default();
            let key = (bg.micro_starttime, bg.micro_duration);
            if !entry.contains(&key) {
                entry.push(key);
            }
        }

        // Load each WAV file
        for (&wav_id, slice_keys) in &needed {
            let wav_name = match model.wav_defs.get(&wav_id) {
                Some(path) => path,
                None => continue,
            };

            let resolved = decode::resolve_audio_path(base_path, &wav_name.to_string_lossy());
            let audio_path = match resolved {
                Some(p) => p,
                None => continue,
            };

            let mut pcm = match decode::load_audio(&audio_path) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Convert to target format
            if self.target_channels != 0 && pcm.channels != self.target_channels {
                pcm = pcm.change_channels(self.target_channels);
            }
            if self.target_sample_rate != 0 && pcm.sample_rate != self.target_sample_rate {
                pcm = pcm.change_sample_rate(self.target_sample_rate);
            }

            // Create slices
            for &(starttime, duration) in slice_keys {
                if starttime == 0 && duration == 0 {
                    self.wav_map.insert(wav_id, pcm.clone());
                } else {
                    let sliced = pcm.slice(starttime, duration);
                    if let Some(sliced) = sliced {
                        let slices = self.slice_map.entry(wav_id).or_default();
                        slices.push(SliceWav {
                            micro_starttime: starttime,
                            micro_duration: duration,
                            pcm: sliced,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn play_note(&mut self, _note: &Note, _volume: f32, _pitch_shift: i32) {
        // Offline driver doesn't play audio
    }

    fn play_bg_note(&mut self, _bg_note: &BgNote, _volume: f32) {
        // Offline driver doesn't play audio
    }

    fn stop_note(&mut self, _note: &Note) {}
    fn stop_all(&mut self) {}
    fn set_note_volume(&mut self, _note: &Note, _volume: f32) {}

    fn set_global_pitch(&mut self, pitch: f32) {
        self.global_pitch = pitch;
    }

    fn global_pitch(&self) -> f32 {
        self.global_pitch
    }

    fn progress(&self) -> f32 {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_id() {
        assert_eq!(channel_id(0, 0), 128);
        assert_eq!(channel_id(1, 0), 384);
        assert_eq!(channel_id(1, 12), 396);
        assert_eq!(channel_id(1, -12), 372);
    }

    #[test]
    fn test_pitch_from_shift() {
        assert!((pitch_from_shift(0) - 1.0).abs() < 1e-6);
        assert!((pitch_from_shift(12) - 2.0).abs() < 1e-4);
        assert!((pitch_from_shift(-12) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_offline_driver_new() {
        let driver = OfflineAudioDriver::new(44100, 2);
        assert_eq!(driver.target_sample_rate, 44100);
        assert_eq!(driver.target_channels, 2);
        assert_eq!(driver.global_pitch(), 1.0);
    }

    #[test]
    fn test_offline_driver_default_recovery() {
        let mut driver = OfflineAudioDriver::new(44100, 2);
        assert!(!driver.needs_recovery());
        assert!(driver.try_recover().is_ok());
    }
}

use std::path::Path;

use anyhow::Result;

use crate::float_pcm::FloatPCM;
use crate::pcm::PCMLoader;

/// Decoded audio data with f32 samples normalized to [-1.0, 1.0].
///
/// Provides a simplified public API for golden master tests and external consumers.
/// Internally delegates to FloatPCM for resampling and channel conversion.
#[derive(Clone, Debug)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
}

impl AudioData {
    fn from_float_pcm(pcm: &FloatPCM) -> Self {
        let start = pcm.start as usize;
        let len = pcm.len as usize;
        AudioData {
            samples: pcm.sample[start..start + len].to_vec(),
            channels: pcm.channels as u16,
            sample_rate: pcm.sample_rate as u32,
        }
    }

    fn to_float_pcm(&self) -> FloatPCM {
        FloatPCM::new(
            self.channels as i32,
            self.sample_rate as i32,
            0,
            self.samples.len() as i32,
            self.samples.clone(),
        )
    }

    /// Change sample rate with linear interpolation resampling.
    pub fn change_sample_rate(&self, rate: u32) -> AudioData {
        let pcm = self.to_float_pcm().change_sample_rate(rate as i32);
        AudioData::from_float_pcm(&pcm)
    }

    /// Change channel count (mono/stereo conversion).
    pub fn change_channels(&self, channels: u16) -> AudioData {
        let pcm = self.to_float_pcm().change_channels(channels as i32);
        AudioData::from_float_pcm(&pcm)
    }
}

/// Load an audio file and decode to f32 samples normalized to [-1.0, 1.0].
///
/// Supports WAV (PCM 8/16/24/32-bit, IEEE float, MS-ADPCM), OGG, MP3, FLAC.
/// No driver-specific channel/sample rate conversion is applied.
pub fn load_audio(path: &Path) -> Result<AudioData> {
    let mut loader = PCMLoader::new();
    loader.load_pcm(path)?;
    let float_pcm = FloatPCM::load_pcm(&loader)?;
    Ok(AudioData::from_float_pcm(&float_pcm))
}

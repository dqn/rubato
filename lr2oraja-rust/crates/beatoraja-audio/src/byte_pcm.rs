use anyhow::{Result, bail};
use std::sync::Arc;

/// 8-bit byte PCM audio data.
///
/// Translated from: BytePCM.java
#[derive(Clone, Debug)]
pub struct BytePCM {
    pub channels: i32,
    pub sample_rate: i32,
    pub start: i32,
    pub len: i32,
    pub sample: Arc<Vec<u8>>,
}

impl BytePCM {
    pub fn new(channels: i32, sample_rate: i32, start: i32, len: i32, sample: Vec<u8>) -> Self {
        BytePCM {
            channels,
            sample_rate,
            start,
            len,
            sample: Arc::new(sample),
        }
    }

    fn new_shared(
        channels: i32,
        sample_rate: i32,
        start: i32,
        len: i32,
        sample: Arc<Vec<u8>>,
    ) -> Self {
        BytePCM {
            channels,
            sample_rate,
            start,
            len,
            sample,
        }
    }

    pub fn load_pcm(loader: &crate::pcm::PCMLoader) -> Result<BytePCM> {
        let sample: Vec<u8>;
        let bytes = loader.pcm_data.len();
        let pcm = &loader.pcm_data;

        match loader.bits_per_sample {
            8 => {
                sample = pcm.to_vec();
            }
            16 => {
                let mut s = vec![0u8; bytes / 2];
                for i in 0..s.len() {
                    // Java: pcm.get(i * 2 + 1) -- takes high byte of each 16-bit sample
                    s[i] = pcm[i * 2 + 1];
                }
                sample = s;
            }
            24 => {
                let mut s = vec![0u8; bytes / 3];
                for i in 0..s.len() {
                    // Java: pcm.get(i * 3 + 2) -- takes highest byte of each 24-bit sample
                    s[i] = pcm[i * 3 + 2];
                }
                sample = s;
            }
            32 => {
                let mut s = vec![0u8; bytes / 4];
                for i in 0..s.len() {
                    let f = f32::from_le_bytes([
                        pcm[i * 4],
                        pcm[i * 4 + 1],
                        pcm[i * 4 + 2],
                        pcm[i * 4 + 3],
                    ]);
                    s[i] = (f * i8::MAX as f32) as i8 as u8;
                }
                sample = s;
            }
            _ => {
                bail!(
                    "{} bits per samples isn't supported",
                    loader.bits_per_sample
                );
            }
        }

        Ok(BytePCM::new(
            loader.channels,
            loader.sample_rate,
            0,
            sample.len() as i32,
            sample,
        ))
    }

    /// Change sample rate with linear interpolation resampling.
    ///
    /// Translated from: BytePCM.changeSampleRate
    pub fn change_sample_rate(&self, sample: i32) -> BytePCM {
        let samples = self.get_sample(sample);
        let start = ((((self.start as i64) * (sample as i64) / (self.sample_rate as i64)) as i32)
            .min(samples.len() as i32 - 1)
            / self.channels)
            * self.channels;
        let len = ((((self.len as i64) * (sample as i64) / (self.sample_rate as i64)) as i32)
            .min(samples.len() as i32 - start)
            / self.channels)
            * self.channels;
        BytePCM::new(self.channels, sample, start, len, samples)
    }

    /// Change playback speed.
    ///
    /// Translated from: BytePCM.changeFrequency
    pub fn change_frequency(&self, rate: f32) -> BytePCM {
        let samples = self.get_sample((self.sample_rate as f32 / rate) as i32);
        let start = ((((self.start as i64) as f32 / rate / self.sample_rate as f32) as i32)
            .min(samples.len() as i32 - 1)
            / self.channels)
            * self.channels;
        let len = ((((self.len as i64) as f32 / rate / self.sample_rate as f32) as i32)
            .min(samples.len() as i32 - start)
            / self.channels)
            * self.channels;
        BytePCM::new(self.channels, self.sample_rate, start, len, samples)
    }

    /// Linear interpolation resampling.
    ///
    /// Translated from: BytePCM.getSample
    fn get_sample(&self, sample: i32) -> Vec<u8> {
        let new_len = (((self.sample.len() as i64 / self.channels as i64) * sample as i64
            / self.sample_rate as i64)
            * self.channels as i64) as usize;
        let mut samples = vec![0u8; new_len];

        for i in 0i64..(samples.len() as i64 / self.channels as i64) {
            let position = i * self.sample_rate as i64 / sample as i64;
            let modv = (i * self.sample_rate as i64) % sample as i64;
            for j in 0..self.channels {
                if modv != 0
                    && (((position + 1) * self.channels as i64 + j as i64) as usize)
                        < self.sample.len()
                {
                    // Java uses short for interpolation of byte values
                    let sample1 =
                        self.sample[(position * self.channels as i64 + j as i64) as usize] as i16;
                    let sample2 = self.sample
                        [((position + 1) * self.channels as i64 + j as i64) as usize]
                        as i16;
                    samples[(i * self.channels as i64 + j as i64) as usize] =
                        ((sample1 as i64 * (sample as i64 - modv) + sample2 as i64 * modv)
                            / sample as i64) as u8;
                } else {
                    samples[(i * self.channels as i64 + j as i64) as usize] =
                        self.sample[(position * self.channels as i64 + j as i64) as usize];
                }
            }
        }

        samples
    }

    /// Change channel count (mono/stereo conversion).
    ///
    /// Translated from: BytePCM.changeChannels
    pub fn change_channels(&self, channels: i32) -> BytePCM {
        let mut samples = vec![0u8; self.sample.len() * channels as usize / self.channels as usize];

        for i in 0i64..(samples.len() as i64 / channels as i64) {
            for j in 0..channels {
                samples[(i * channels as i64 + j as i64) as usize] =
                    self.sample[(i * self.channels as i64) as usize];
            }
        }
        BytePCM::new(
            channels,
            self.sample_rate,
            self.start * channels / self.channels,
            self.len * channels / self.channels,
            samples,
        )
    }

    /// Trim PCM with silent-end removal.
    ///
    /// Translated from: BytePCM.slice
    pub fn slice(&self, starttime: i64, duration: i64) -> Option<BytePCM> {
        let mut duration = duration;
        if duration == 0
            || starttime + duration
                > (self.len as i64) * 1000000 / (self.sample_rate as i64 * self.channels as i64)
        {
            duration = ((self.len as i64) * 1000000
                / (self.sample_rate as i64 * self.channels as i64)
                - starttime)
                .max(0);
        }

        let start = ((starttime * self.sample_rate as i64 / 1000000) * self.channels as i64) as i32;
        let mut length =
            ((duration * self.sample_rate as i64 / 1000000) * self.channels as i64) as i32;
        while length > self.channels {
            let mut zero = true;
            for i in 0..self.channels {
                zero &= self.sample[(self.start + start + length - i - 1) as usize] == 0;
            }
            if zero {
                length -= self.channels;
            } else {
                break;
            }
        }
        if length > 0 {
            Some(BytePCM::new_shared(
                self.channels,
                self.sample_rate,
                self.start + start,
                length,
                Arc::clone(&self.sample),
            ))
        } else {
            None
        }
    }

    pub fn validate(&self) -> bool {
        !self.sample.is_empty()
    }
}

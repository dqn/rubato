use anyhow::{Result, bail};
use std::sync::Arc;

/// 16-bit short PCM audio data.
///
/// Translated from: ShortPCM.java
/// Also incorporates ShortDirectPCM.java since Rust doesn't distinguish direct/heap buffers.
#[derive(Clone, Debug)]
pub struct ShortPCM {
    pub channels: i32,
    pub sample_rate: i32,
    pub start: i32,
    pub len: i32,
    pub sample: Arc<Vec<i16>>,
}

impl ShortPCM {
    pub fn new(channels: i32, sample_rate: i32, start: i32, len: i32, sample: Vec<i16>) -> Self {
        ShortPCM {
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
        sample: Arc<Vec<i16>>,
    ) -> Self {
        ShortPCM {
            channels,
            sample_rate,
            start,
            len,
            sample,
        }
    }

    pub fn load_pcm(loader: &crate::pcm::PCMLoader) -> Result<ShortPCM> {
        let sample: Vec<i16>;
        let bytes = loader.pcm_data.len();
        let pcm = &loader.pcm_data;

        match loader.bits_per_sample {
            8 => {
                let mut s = vec![0i16; bytes];
                for i in 0..s.len() {
                    s[i] = (((pcm[i] as i16) - 128) * 256);
                }
                sample = s;
            }
            16 => {
                let mut s = vec![0i16; bytes / 2];
                for i in 0..s.len() {
                    s[i] = i16::from_le_bytes([pcm[i * 2], pcm[i * 2 + 1]]);
                }
                sample = s;
            }
            24 => {
                let mut s = vec![0i16; bytes / 3];
                for i in 0..s.len() {
                    // Java: pcm.getShort(i * 3 + 1) -- reads 2 bytes at offset i*3+1
                    s[i] = i16::from_le_bytes([pcm[i * 3 + 1], pcm[i * 3 + 2]]);
                }
                sample = s;
            }
            32 => {
                let mut s = vec![0i16; bytes / 4];
                for i in 0..s.len() {
                    let f = f32::from_le_bytes([
                        pcm[i * 4],
                        pcm[i * 4 + 1],
                        pcm[i * 4 + 2],
                        pcm[i * 4 + 3],
                    ]);
                    s[i] = (f * i16::MAX as f32) as i16;
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

        Ok(ShortPCM::new(
            loader.channels,
            loader.sample_rate,
            0,
            sample.len() as i32,
            sample,
        ))
    }

    /// Change sample rate with linear interpolation resampling.
    ///
    /// Translated from: ShortPCM.changeSampleRate
    pub fn change_sample_rate(&self, sample: i32) -> ShortPCM {
        let samples = self.get_sample(sample);
        let start = ((((self.start as i64) * (sample as i64) / (self.sample_rate as i64)) as i32)
            .min(samples.len() as i32 - 1)
            / self.channels)
            * self.channels;
        let len = ((((self.len as i64) * (sample as i64) / (self.sample_rate as i64)) as i32)
            .min(samples.len() as i32 - start)
            / self.channels)
            * self.channels;
        ShortPCM::new(self.channels, sample, start, len, samples)
    }

    /// Change playback speed.
    ///
    /// Translated from: ShortPCM.changeFrequency
    pub fn change_frequency(&self, rate: f32) -> ShortPCM {
        let samples = self.get_sample((self.sample_rate as f32 / rate) as i32);
        let start = ((((self.start as i64) as f32 / rate / self.sample_rate as f32) as i32)
            .min(samples.len() as i32 - 1)
            / self.channels)
            * self.channels;
        let len = ((((self.len as i64) as f32 / rate / self.sample_rate as f32) as i32)
            .min(samples.len() as i32 - start)
            / self.channels)
            * self.channels;
        ShortPCM::new(self.channels, self.sample_rate, start, len, samples)
    }

    /// Linear interpolation resampling.
    ///
    /// Translated from: ShortPCM.getSample
    fn get_sample(&self, sample: i32) -> Vec<i16> {
        let new_len = (((self.sample.len() as i64 / self.channels as i64) * sample as i64
            / self.sample_rate as i64)
            * self.channels as i64) as usize;
        let mut samples = vec![0i16; new_len];

        for i in 0i64..(samples.len() as i64 / self.channels as i64) {
            let position = i * self.sample_rate as i64 / sample as i64;
            let modv = (i * self.sample_rate as i64) % sample as i64;
            for j in 0..self.channels {
                if modv != 0
                    && (((position + 1) * self.channels as i64 + j as i64) as usize)
                        < self.sample.len()
                {
                    let sample1 =
                        self.sample[(position * self.channels as i64 + j as i64) as usize];
                    let sample2 =
                        self.sample[((position + 1) * self.channels as i64 + j as i64) as usize];
                    samples[(i * self.channels as i64 + j as i64) as usize] =
                        ((sample1 as i64 * (sample as i64 - modv) + sample2 as i64 * modv)
                            / sample as i64) as i16;
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
    /// Translated from: ShortPCM.changeChannels
    pub fn change_channels(&self, channels: i32) -> ShortPCM {
        let mut samples =
            vec![0i16; self.sample.len() * channels as usize / self.channels as usize];

        for i in 0i64..(samples.len() as i64 / channels as i64) {
            for j in 0..channels {
                samples[(i * channels as i64 + j as i64) as usize] =
                    self.sample[(i * self.channels as i64) as usize];
            }
        }
        ShortPCM::new(
            channels,
            self.sample_rate,
            self.start * channels / self.channels,
            self.len * channels / self.channels,
            samples,
        )
    }

    /// Trim PCM with silent-end removal.
    ///
    /// Translated from: ShortPCM.slice
    pub fn slice(&self, starttime: i64, duration: i64) -> Option<ShortPCM> {
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
            Some(ShortPCM::new_shared(
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

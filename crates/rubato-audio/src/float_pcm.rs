use anyhow::{Result, bail};
use std::sync::Arc;

/// 32-bit float PCM audio data.
///
/// Translated from: FloatPCM.java
#[derive(Clone, Debug)]
pub struct FloatPCM {
    pub channels: i32,
    pub sample_rate: i32,
    pub start: i32,
    pub len: i32,
    pub sample: Arc<Vec<f32>>,
}

impl FloatPCM {
    pub fn new(channels: i32, sample_rate: i32, start: i32, len: i32, sample: Vec<f32>) -> Self {
        FloatPCM {
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
        sample: Arc<Vec<f32>>,
    ) -> Self {
        FloatPCM {
            channels,
            sample_rate,
            start,
            len,
            sample,
        }
    }

    pub fn load_pcm(loader: &crate::pcm::PCMLoader) -> Result<FloatPCM> {
        let sample: Vec<f32>;
        let bytes = loader.pcm_data.len();
        let pcm = &loader.pcm_data;

        match loader.bits_per_sample {
            8 => {
                let mut s = vec![0f32; bytes];
                for i in 0..s.len() {
                    s[i] = (pcm[i] as f32 - 128.0) / 128.0;
                }
                sample = s;
            }
            16 => {
                let mut s = vec![0f32; bytes / 2];
                for i in 0..s.len() {
                    let short_val = i16::from_le_bytes([pcm[i * 2], pcm[i * 2 + 1]]);
                    s[i] = (short_val as f32) / i16::MAX as f32;
                }
                sample = s;
            }
            24 => {
                let mut s = vec![0f32; bytes / 3];
                for i in 0..s.len() {
                    // Java: (((pcm.get(i*3) & 0xff) << 8) | ((pcm.get(i*3+1) & 0xff) << 16) | ((pcm.get(i*3+2) & 0xff) << 24)) / Integer.MAX_VALUE
                    let val = ((pcm[i * 3] as i32 & 0xff) << 8)
                        | ((pcm[i * 3 + 1] as i32 & 0xff) << 16)
                        | ((pcm[i * 3 + 2] as i32 & 0xff) << 24);
                    s[i] = (val as f32) / i32::MAX as f32;
                }
                sample = s;
            }
            32 => {
                let mut s = vec![0f32; bytes / 4];
                for i in 0..s.len() {
                    s[i] = f32::from_le_bytes([
                        pcm[i * 4],
                        pcm[i * 4 + 1],
                        pcm[i * 4 + 2],
                        pcm[i * 4 + 3],
                    ]);
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

        Ok(FloatPCM::new(
            loader.channels,
            loader.sample_rate,
            0,
            sample.len() as i32,
            sample,
        ))
    }

    /// Change sample rate with linear interpolation resampling.
    ///
    /// Translated from: FloatPCM.changeSampleRate
    pub fn change_sample_rate(&self, sample: i32) -> FloatPCM {
        if self.sample_rate == 0 || self.channels == 0 || sample == 0 {
            return FloatPCM::new(self.channels, sample, 0, 0, Vec::new());
        }
        let samples = self.get_sample(sample);
        let start = ((((self.start as i64) * (sample as i64) / (self.sample_rate as i64)) as i32)
            .min(samples.len() as i32 - 1)
            / self.channels)
            * self.channels;
        let len = ((((self.len as i64) * (sample as i64) / (self.sample_rate as i64)) as i32)
            .min(samples.len() as i32 - start)
            / self.channels)
            * self.channels;
        FloatPCM::new(self.channels, sample, start, len, samples)
    }

    /// Change playback speed.
    ///
    /// Translated from: FloatPCM.changeFrequency
    pub fn change_frequency(&self, rate: f32) -> FloatPCM {
        let samples = self.get_sample((self.sample_rate as f32 / rate) as i32);
        let start = ((((self.start as i64) as f32 / rate / self.sample_rate as f32) as i32)
            .min(samples.len() as i32 - 1)
            / self.channels)
            * self.channels;
        let len = ((((self.len as i64) as f32 / rate / self.sample_rate as f32) as i32)
            .min(samples.len() as i32 - start)
            / self.channels)
            * self.channels;
        FloatPCM::new(self.channels, self.sample_rate, start, len, samples)
    }

    /// Linear interpolation resampling.
    ///
    /// Translated from: FloatPCM.getSample
    fn get_sample(&self, sample: i32) -> Vec<f32> {
        if self.channels == 0 || self.sample_rate == 0 || sample == 0 {
            return Vec::new();
        }
        let new_len = (((self.sample.len() as i64 / self.channels as i64) * sample as i64
            / self.sample_rate as i64)
            * self.channels as i64) as usize;
        let mut samples = vec![0f32; new_len];

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
                        (sample1 * (sample as i64 - modv) as f32 + sample2 * modv as f32)
                            / sample as f32;
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
    /// Translated from: FloatPCM.changeChannels
    pub fn change_channels(&self, channels: i32) -> FloatPCM {
        let mut samples =
            vec![0f32; self.sample.len() * channels as usize / self.channels as usize];

        for i in 0i64..(samples.len() as i64 / channels as i64) {
            for j in 0..channels {
                samples[(i * channels as i64 + j as i64) as usize] =
                    self.sample[(i * self.channels as i64) as usize];
            }
        }
        FloatPCM::new(
            channels,
            self.sample_rate,
            self.start * channels / self.channels,
            self.len * channels / self.channels,
            samples,
        )
    }

    /// Trim PCM with silent-end removal.
    ///
    /// Translated from: FloatPCM.slice
    pub fn slice(&self, starttime: i64, duration: i64) -> Option<FloatPCM> {
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
            let frame_start = (self.start + start + length - self.channels) as usize;
            let frame_end = (self.start + start + length) as usize;
            let zero = self.sample[frame_start..frame_end]
                .iter()
                .all(|&s| s == 0.0);
            if zero {
                length -= self.channels;
            } else {
                break;
            }
        }
        if length > 0 {
            Some(FloatPCM::new_shared(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_pcm_change_sample_rate_zero_sample_rate() {
        let pcm = FloatPCM::new(1, 0, 0, 4, vec![0.1, 0.2, 0.3, 0.4]);
        let result = pcm.change_sample_rate(44100);
        assert_eq!(result.sample.len(), 0);
    }

    #[test]
    fn test_float_pcm_change_sample_rate_zero_channels() {
        let pcm = FloatPCM::new(0, 44100, 0, 4, vec![0.1, 0.2, 0.3, 0.4]);
        let result = pcm.change_sample_rate(48000);
        assert_eq!(result.sample.len(), 0);
    }

    #[test]
    fn test_float_pcm_change_sample_rate_zero_sample() {
        let pcm = FloatPCM::new(1, 44100, 0, 4, vec![0.1, 0.2, 0.3, 0.4]);
        let result = pcm.change_sample_rate(0);
        assert_eq!(result.sample.len(), 0);
    }

    // --- Construction tests ---

    #[test]
    fn new_stores_fields_correctly() {
        let pcm = FloatPCM::new(2, 48000, 5, 100, vec![0.1, 0.2, 0.3, 0.4]);
        assert_eq!(pcm.channels, 2);
        assert_eq!(pcm.sample_rate, 48000);
        assert_eq!(pcm.start, 5);
        assert_eq!(pcm.len, 100);
        assert_eq!(*pcm.sample, vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn new_empty_data() {
        let pcm = FloatPCM::new(1, 44100, 0, 0, Vec::new());
        assert!(pcm.sample.is_empty());
        assert_eq!(pcm.len, 0);
    }

    #[test]
    fn new_single_sample() {
        let pcm = FloatPCM::new(1, 44100, 0, 1, vec![0.5]);
        assert_eq!(pcm.sample.len(), 1);
        assert!((pcm.sample[0] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn new_extreme_values() {
        let pcm = FloatPCM::new(1, 44100, 0, 3, vec![-1.0, 0.0, 1.0]);
        assert!((pcm.sample[0] - (-1.0)).abs() < f32::EPSILON);
        assert!((pcm.sample[1] - 0.0).abs() < f32::EPSILON);
        assert!((pcm.sample[2] - 1.0).abs() < f32::EPSILON);
    }

    // --- Validate tests ---

    #[test]
    fn validate_non_empty() {
        let pcm = FloatPCM::new(1, 44100, 0, 1, vec![0.5]);
        assert!(pcm.validate());
    }

    #[test]
    fn validate_empty() {
        let pcm = FloatPCM::new(1, 44100, 0, 0, Vec::new());
        assert!(!pcm.validate());
    }

    // --- Clone / Arc sharing tests ---

    #[test]
    fn clone_shares_sample_arc() {
        let pcm = FloatPCM::new(1, 44100, 0, 2, vec![0.1, 0.2]);
        let cloned = pcm.clone();
        assert_eq!(Arc::strong_count(&pcm.sample), 2);
        assert_eq!(*cloned.sample, vec![0.1, 0.2]);
    }

    // --- load_pcm tests ---

    #[test]
    fn load_pcm_8bit_converts_to_float() {
        // 8-bit: (byte - 128) / 128
        // 128 -> 0.0, 0 -> -1.0, 255 -> ~0.992
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![128, 0, 255],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 8,
            block_align: 1,
        };
        let pcm = FloatPCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 3);
        assert!((pcm.sample[0] - 0.0).abs() < 0.01); // 128 -> 0
        assert!((pcm.sample[1] - (-1.0)).abs() < 0.01); // 0 -> -1
        assert!((pcm.sample[2] - (127.0 / 128.0)).abs() < 0.01); // 255 -> ~0.992
    }

    #[test]
    fn load_pcm_16bit_converts_to_float() {
        // 16-bit LE: i16 / 32767
        // 0x7FFF (32767) -> 1.0, 0x0000 -> 0.0, 0x8001 (-32767) -> -1.0
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![0xFF, 0x7F, 0x00, 0x00, 0x01, 0x80],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            block_align: 2,
        };
        let pcm = FloatPCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 3);
        assert!((pcm.sample[0] - 1.0).abs() < 0.001);
        assert!((pcm.sample[1] - 0.0).abs() < 0.001);
        assert!((pcm.sample[2] - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn load_pcm_32bit_passthrough() {
        // 32-bit float: direct passthrough
        let val = 0.75f32;
        let bytes = val.to_le_bytes();
        let loader = crate::pcm::PCMLoader {
            pcm_data: bytes.to_vec(),
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 32,
            block_align: 4,
        };
        let pcm = FloatPCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 1);
        assert!((pcm.sample[0] - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn load_pcm_unsupported_bits() {
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![0; 8],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 12,
            block_align: 2,
        };
        let result = FloatPCM::load_pcm(&loader);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("12 bits"));
    }

    // --- change_sample_rate tests ---

    #[test]
    fn change_sample_rate_same_rate_preserves_data() {
        let pcm = FloatPCM::new(1, 44100, 0, 4, vec![0.1, 0.2, 0.3, 0.4]);
        let result = pcm.change_sample_rate(44100);
        assert_eq!(result.sample_rate, 44100);
        assert_eq!(result.len, 4);
        assert_eq!(*result.sample, vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn change_sample_rate_double_upsamples() {
        let pcm = FloatPCM::new(1, 44100, 0, 4, vec![0.0, 0.5, 1.0, 0.25]);
        let result = pcm.change_sample_rate(88200);
        assert_eq!(result.sample_rate, 88200);
        assert_eq!(result.sample.len(), 8);
        // First sample preserved
        assert!((result.sample[0] - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn change_sample_rate_interpolation_midpoint() {
        // 2 samples [0.0, 1.0] at 1Hz -> upsample to 2Hz -> 4 samples
        // At i=1: position=0, modv=1, sample=2
        // interpolation: (0.0*(2-1) + 1.0*1) / 2 = 0.5
        let pcm = FloatPCM::new(1, 1, 0, 2, vec![0.0, 1.0]);
        let result = pcm.change_sample_rate(2);
        assert_eq!(result.sample.len(), 4);
        assert!((result.sample[0] - 0.0).abs() < f32::EPSILON); // original
        assert!((result.sample[1] - 0.5).abs() < 0.01); // interpolated midpoint
    }

    // --- change_channels tests ---

    #[test]
    fn change_channels_mono_to_stereo() {
        let pcm = FloatPCM::new(1, 44100, 0, 3, vec![0.1, 0.5, 0.9]);
        let result = pcm.change_channels(2);
        assert_eq!(result.channels, 2);
        assert_eq!(result.sample.len(), 6);
        // Each mono sample duplicated
        assert!((result.sample[0] - 0.1).abs() < f32::EPSILON);
        assert!((result.sample[1] - 0.1).abs() < f32::EPSILON);
        assert!((result.sample[2] - 0.5).abs() < f32::EPSILON);
        assert!((result.sample[3] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn change_channels_stereo_to_mono() {
        let pcm = FloatPCM::new(2, 44100, 0, 4, vec![0.1, 0.9, 0.5, 0.3]);
        let result = pcm.change_channels(1);
        assert_eq!(result.channels, 1);
        assert_eq!(result.sample.len(), 2);
        // Takes first channel of each frame
        assert!((result.sample[0] - 0.1).abs() < f32::EPSILON);
        assert!((result.sample[1] - 0.5).abs() < f32::EPSILON);
    }

    // --- slice tests ---

    #[test]
    fn slice_all_silent_mono_returns_some_with_one_frame() {
        // The while loop trims trailing silence but stops when length <= channels.
        // So all-zero mono data results in length=1 (one frame), which is > 0 -> Some.
        let pcm = FloatPCM::new(1, 44100, 0, 4, vec![0.0, 0.0, 0.0, 0.0]);
        let result = pcm.slice(0, 0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len, 1);
    }

    #[test]
    fn slice_returns_some_for_non_silent_data() {
        let pcm = FloatPCM::new(1, 44100, 0, 4, vec![0.5, 0.25, 0.1, 0.0]);
        let result = pcm.slice(0, 0);
        assert!(result.is_some());
        let sliced = result.unwrap();
        assert_eq!(Arc::strong_count(&sliced.sample), 2);
    }

    #[test]
    fn slice_trims_trailing_silence() {
        let pcm = FloatPCM::new(1, 44100, 0, 4, vec![0.5, 0.25, 0.0, 0.0]);
        let result = pcm.slice(0, 0).unwrap();
        assert_eq!(result.len, 2);
    }

    // --- change_frequency tests ---

    #[test]
    fn change_frequency_rate_one_preserves() {
        let pcm = FloatPCM::new(1, 44100, 0, 4, vec![0.1, 0.2, 0.3, 0.4]);
        let result = pcm.change_frequency(1.0);
        assert_eq!(result.sample_rate, 44100);
        assert_eq!(*result.sample, vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn change_frequency_double_speed() {
        let pcm = FloatPCM::new(1, 44100, 0, 4, vec![0.1, 0.2, 0.3, 0.4]);
        let result = pcm.change_frequency(2.0);
        // 2x speed -> half the samples
        assert_eq!(result.sample.len(), 2);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// For any Vec<u8> bytes, 8-bit float PCM conversion produces samples in [-1.0, 1.0].
        /// The formula is (byte as f32 - 128.0) / 128.0.
        /// Min: (0 - 128) / 128 = -1.0, Max: (255 - 128) / 128 = 127/128 ~= 0.992.
        #[test]
        fn float_pcm_8bit_range(data in proptest::collection::vec(any::<u8>(), 1..=256)) {
            let loader = crate::pcm::PCMLoader {
                pcm_data: data,
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: 8,
                block_align: 1,
            };
            let pcm = FloatPCM::load_pcm(&loader).unwrap();
            for &s in pcm.sample.iter() {
                prop_assert!(
                    (-1.0..=1.0).contains(&s),
                    "sample {} out of range [-1.0, 1.0]",
                    s
                );
            }
        }

        /// For any Vec<i16> (converted to LE bytes), 16-bit float PCM produces samples in [-1.0, 1.0].
        /// The formula is (i16 as f32) / 32767.0.
        /// Min: -32768 / 32767 ~= -1.00003, but that is still within [-1.001, 1.0] due to asymmetry.
        /// We check [-1.01, 1.0] to account for -32768 / 32767.
        #[test]
        fn float_pcm_16bit_range(values in proptest::collection::vec(any::<i16>(), 1..=128)) {
            let mut bytes = Vec::with_capacity(values.len() * 2);
            for &v in &values {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
            let loader = crate::pcm::PCMLoader {
                pcm_data: bytes,
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: 16,
                block_align: 2,
            };
            let pcm = FloatPCM::load_pcm(&loader).unwrap();
            for &s in pcm.sample.iter() {
                // -32768 / 32767 = -1.0000305... so we allow a small margin below -1.0
                prop_assert!(
                    (-1.01..=1.0).contains(&s),
                    "sample {} out of expected range [-1.01, 1.0]",
                    s
                );
            }
        }
    }
}

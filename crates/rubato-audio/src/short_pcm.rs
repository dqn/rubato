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
                    s[i] = ((pcm[i] as i16) - 128) * 256;
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
        if self.sample_rate == 0 || self.channels == 0 || sample == 0 {
            return ShortPCM::new(self.channels, sample, 0, 0, Vec::new());
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
        if self.channels == 0 || self.sample_rate == 0 || sample == 0 {
            return Vec::new();
        }
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
            let frame_start = (self.start + start + length - self.channels) as usize;
            let frame_end = (self.start + start + length) as usize;
            let zero = self.sample[frame_start..frame_end].iter().all(|&s| s == 0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_pcm_change_sample_rate_zero_sample_rate() {
        let pcm = ShortPCM::new(1, 0, 0, 4, vec![100, 200, 300, 400]);
        let result = pcm.change_sample_rate(44100);
        assert_eq!(result.sample.len(), 0);
    }

    #[test]
    fn test_short_pcm_change_sample_rate_zero_channels() {
        let pcm = ShortPCM::new(0, 44100, 0, 4, vec![100, 200, 300, 400]);
        let result = pcm.change_sample_rate(48000);
        assert_eq!(result.sample.len(), 0);
    }

    #[test]
    fn test_short_pcm_change_sample_rate_zero_sample() {
        let pcm = ShortPCM::new(1, 44100, 0, 4, vec![100, 200, 300, 400]);
        let result = pcm.change_sample_rate(0);
        assert_eq!(result.sample.len(), 0);
    }

    // --- Construction tests ---

    #[test]
    fn new_stores_fields_correctly() {
        let pcm = ShortPCM::new(2, 48000, 10, 50, vec![1000, -2000, 3000, -4000]);
        assert_eq!(pcm.channels, 2);
        assert_eq!(pcm.sample_rate, 48000);
        assert_eq!(pcm.start, 10);
        assert_eq!(pcm.len, 50);
        assert_eq!(*pcm.sample, vec![1000, -2000, 3000, -4000]);
    }

    #[test]
    fn new_empty_data() {
        let pcm = ShortPCM::new(1, 44100, 0, 0, Vec::new());
        assert!(pcm.sample.is_empty());
        assert_eq!(pcm.len, 0);
    }

    #[test]
    fn new_single_sample() {
        let pcm = ShortPCM::new(1, 44100, 0, 1, vec![12345]);
        assert_eq!(pcm.sample.len(), 1);
        assert_eq!(pcm.sample[0], 12345);
    }

    #[test]
    fn new_max_min_values() {
        let pcm = ShortPCM::new(1, 44100, 0, 2, vec![i16::MIN, i16::MAX]);
        assert_eq!(pcm.sample[0], i16::MIN);
        assert_eq!(pcm.sample[1], i16::MAX);
    }

    // --- Validate tests ---

    #[test]
    fn validate_non_empty() {
        let pcm = ShortPCM::new(1, 44100, 0, 1, vec![42]);
        assert!(pcm.validate());
    }

    #[test]
    fn validate_empty() {
        let pcm = ShortPCM::new(1, 44100, 0, 0, Vec::new());
        assert!(!pcm.validate());
    }

    // --- Clone / Arc sharing tests ---

    #[test]
    fn clone_shares_sample_arc() {
        let pcm = ShortPCM::new(1, 44100, 0, 2, vec![100, 200]);
        let cloned = pcm.clone();
        assert_eq!(Arc::strong_count(&pcm.sample), 2);
        assert_eq!(*cloned.sample, vec![100, 200]);
    }

    // --- load_pcm tests ---

    #[test]
    fn load_pcm_8bit_converts_to_short() {
        // 8-bit: (byte - 128) * 256
        // 128 -> 0, 0 -> -32768, 255 -> 32512
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![128, 0, 255],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 8,
            block_align: 1,
        };
        let pcm = ShortPCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 3);
        assert_eq!(pcm.sample[0], 0); // (128 - 128) * 256
        assert_eq!(pcm.sample[1], -32768); // (0 - 128) * 256
        assert_eq!(pcm.sample[2], 32512); // (255 - 128) * 256
    }

    #[test]
    fn load_pcm_16bit_passthrough() {
        // 16-bit LE: direct little-endian i16
        // 0x0100 = 256, 0xFF7F = 32767
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![0x00, 0x01, 0xFF, 0x7F],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            block_align: 2,
        };
        let pcm = ShortPCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 2);
        assert_eq!(pcm.sample[0], 256);
        assert_eq!(pcm.sample[1], 32767);
    }

    #[test]
    fn load_pcm_24bit_takes_upper_two_bytes() {
        // 24-bit: reads i16 from bytes at i*3+1, i*3+2
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![0x00, 0xAB, 0xCD, 0x00, 0x12, 0x34],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 24,
            block_align: 3,
        };
        let pcm = ShortPCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 2);
        assert_eq!(pcm.sample[0], i16::from_le_bytes([0xAB, 0xCD]));
        assert_eq!(pcm.sample[1], i16::from_le_bytes([0x12, 0x34]));
    }

    #[test]
    fn load_pcm_32bit_float_to_short() {
        // 32-bit float: f * 32767
        let one = 1.0f32.to_le_bytes();
        let zero = 0.0f32.to_le_bytes();
        let neg = (-1.0f32).to_le_bytes();
        let mut data = Vec::new();
        data.extend_from_slice(&one);
        data.extend_from_slice(&zero);
        data.extend_from_slice(&neg);

        let loader = crate::pcm::PCMLoader {
            pcm_data: data,
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 32,
            block_align: 4,
        };
        let pcm = ShortPCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 3);
        assert_eq!(pcm.sample[0], 32767); // 1.0 * 32767
        assert_eq!(pcm.sample[1], 0); // 0.0 * 32767
        assert_eq!(pcm.sample[2], -32767); // -1.0 * 32767
    }

    #[test]
    fn load_pcm_unsupported_bits() {
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![0; 8],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 6,
            block_align: 1,
        };
        let result = ShortPCM::load_pcm(&loader);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("6 bits"));
    }

    // --- change_sample_rate tests ---

    #[test]
    fn change_sample_rate_same_rate_preserves() {
        let pcm = ShortPCM::new(1, 44100, 0, 4, vec![100, 200, 300, 400]);
        let result = pcm.change_sample_rate(44100);
        assert_eq!(result.sample_rate, 44100);
        assert_eq!(result.len, 4);
        assert_eq!(*result.sample, vec![100, 200, 300, 400]);
    }

    #[test]
    fn change_sample_rate_double_upsamples() {
        let pcm = ShortPCM::new(1, 44100, 0, 4, vec![0, 10000, 20000, 5000]);
        let result = pcm.change_sample_rate(88200);
        assert_eq!(result.sample_rate, 88200);
        assert_eq!(result.sample.len(), 8);
        assert_eq!(result.sample[0], 0);
    }

    #[test]
    fn change_sample_rate_interpolation_midpoint() {
        // 2 samples [0, 10000] at 1Hz -> 2Hz -> 4 output samples
        // At i=1: interpolation between 0 and 10000 -> 5000
        let pcm = ShortPCM::new(1, 1, 0, 2, vec![0, 10000]);
        let result = pcm.change_sample_rate(2);
        assert_eq!(result.sample.len(), 4);
        assert_eq!(result.sample[0], 0);
        assert_eq!(result.sample[1], 5000);
    }

    // --- change_channels tests ---

    #[test]
    fn change_channels_mono_to_stereo() {
        let pcm = ShortPCM::new(1, 44100, 0, 3, vec![100, 500, 900]);
        let result = pcm.change_channels(2);
        assert_eq!(result.channels, 2);
        assert_eq!(result.sample.len(), 6);
        assert_eq!(result.sample[0], 100);
        assert_eq!(result.sample[1], 100);
        assert_eq!(result.sample[2], 500);
        assert_eq!(result.sample[3], 500);
        assert_eq!(result.sample[4], 900);
        assert_eq!(result.sample[5], 900);
    }

    #[test]
    fn change_channels_stereo_to_mono() {
        let pcm = ShortPCM::new(2, 44100, 0, 4, vec![100, 900, 500, 300]);
        let result = pcm.change_channels(1);
        assert_eq!(result.channels, 1);
        assert_eq!(result.sample.len(), 2);
        assert_eq!(result.sample[0], 100);
        assert_eq!(result.sample[1], 500);
    }

    // --- slice tests ---

    #[test]
    fn slice_all_silent_mono_returns_some_with_one_frame() {
        // The while loop trims trailing silence but stops when length <= channels.
        // So all-zero mono data results in length=1 (one frame), which is > 0 -> Some.
        let pcm = ShortPCM::new(1, 44100, 0, 4, vec![0, 0, 0, 0]);
        let result = pcm.slice(0, 0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len, 1);
    }

    #[test]
    fn slice_returns_some_for_non_silent() {
        let pcm = ShortPCM::new(1, 44100, 0, 4, vec![500, 250, 100, 0]);
        let result = pcm.slice(0, 0);
        assert!(result.is_some());
        let sliced = result.unwrap();
        assert_eq!(Arc::strong_count(&sliced.sample), 2);
    }

    #[test]
    fn slice_trims_trailing_silence() {
        let pcm = ShortPCM::new(1, 44100, 0, 4, vec![500, 250, 0, 0]);
        let result = pcm.slice(0, 0).unwrap();
        assert_eq!(result.len, 2);
    }

    #[test]
    fn slice_stereo_trims_per_frame() {
        // Stereo: trailing frame [0, 0] should be trimmed
        // [L1, R1, L2, R2] = [100, 200, 0, 0]
        let pcm = ShortPCM::new(2, 44100, 0, 4, vec![100, 200, 0, 0]);
        let result = pcm.slice(0, 0).unwrap();
        // Trailing frame [0, 0] trimmed, remaining length = 2
        assert_eq!(result.len, 2);
    }

    // --- change_frequency tests ---

    #[test]
    fn change_frequency_rate_one_preserves() {
        let pcm = ShortPCM::new(1, 44100, 0, 4, vec![100, 200, 300, 400]);
        let result = pcm.change_frequency(1.0);
        assert_eq!(result.sample_rate, 44100);
        assert_eq!(*result.sample, vec![100, 200, 300, 400]);
    }

    #[test]
    fn change_frequency_double_speed() {
        let pcm = ShortPCM::new(1, 44100, 0, 4, vec![100, 200, 300, 400]);
        let result = pcm.change_frequency(2.0);
        assert_eq!(result.sample.len(), 2);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// For any Vec<u8> (1..=256 bytes), 8-bit PCM conversion produces samples in [-32768, 32512].
        /// The formula is ((byte as i16) - 128) * 256.
        #[test]
        fn load_pcm_8bit_range(data in proptest::collection::vec(any::<u8>(), 1..=256)) {
            let loader = crate::pcm::PCMLoader {
                pcm_data: data,
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: 8,
                block_align: 1,
            };
            let pcm = ShortPCM::load_pcm(&loader).unwrap();
            for &s in pcm.sample.iter() {
                prop_assert!(s >= -32768 && s <= 32512, "sample {} out of range", s);
            }
        }

        /// All-128 byte input produces all-zero samples: (128 - 128) * 256 = 0.
        #[test]
        fn load_pcm_8bit_128_is_zero(len in 1usize..=256) {
            let data = vec![128u8; len];
            let loader = crate::pcm::PCMLoader {
                pcm_data: data,
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: 8,
                block_align: 1,
            };
            let pcm = ShortPCM::load_pcm(&loader).unwrap();
            for &s in pcm.sample.iter() {
                prop_assert_eq!(s, 0);
            }
        }

        /// Converting Vec<i16> to LE bytes and loading as 16-bit PCM recovers the original values.
        #[test]
        fn load_pcm_16bit_roundtrip(values in proptest::collection::vec(any::<i16>(), 1..=128)) {
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
            let pcm = ShortPCM::load_pcm(&loader).unwrap();
            prop_assert_eq!(pcm.sample.len(), values.len());
            for (i, &v) in values.iter().enumerate() {
                prop_assert_eq!(pcm.sample[i], v, "mismatch at index {}", i);
            }
        }

        /// Same input always produces same output length (deterministic).
        #[test]
        fn load_pcm_output_length_deterministic(
            data in proptest::collection::vec(any::<u8>(), 2..=256),
            bits in prop_oneof![Just(8i32), Just(16), Just(24), Just(32)],
        ) {
            // Ensure data length is a multiple of the byte width for the bit depth.
            let byte_width = (bits / 8) as usize;
            let trimmed_len = (data.len() / byte_width) * byte_width;
            if trimmed_len == 0 {
                return Ok(());
            }
            let trimmed = data[..trimmed_len].to_vec();

            let loader = crate::pcm::PCMLoader {
                pcm_data: trimmed.clone(),
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: bits,
                block_align: byte_width as i32,
            };
            let pcm1 = ShortPCM::load_pcm(&loader).unwrap();

            let loader2 = crate::pcm::PCMLoader {
                pcm_data: trimmed,
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: bits,
                block_align: byte_width as i32,
            };
            let pcm2 = ShortPCM::load_pcm(&loader2).unwrap();

            prop_assert_eq!(pcm1.sample.len(), pcm2.sample.len());
        }
    }
}

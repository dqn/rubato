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
        let pcm = &loader.pcm_data;

        let sample: Vec<u8> = match loader.bits_per_sample {
            8 => pcm.to_vec(),
            16 => {
                // Java: pcm.get(i * 2 + 1) -- takes high byte of each 16-bit sample
                pcm.chunks_exact(2).map(|chunk| chunk[1]).collect()
            }
            24 => {
                // Java: pcm.get(i * 3 + 2) -- takes highest byte of each 24-bit sample
                pcm.chunks_exact(3).map(|chunk| chunk[2]).collect()
            }
            32 => {
                pcm.chunks_exact(4)
                    .map(|chunk| {
                        let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        // Java float->i8 truncation semantics: (byte)(pcm.getFloat() * Byte.MAX_VALUE)
                        // float->int truncates toward zero, int->byte truncates to low 8 bits.
                        // SAFETY: lossy narrowing matches Java behavior -- Rust `as i8` saturates
                        // (since 1.45), so go via i32 first to get truncation.
                        ((f * i8::MAX as f32) as i32 as i8) as u8
                    })
                    .collect()
            }
            _ => {
                bail!(
                    "{} bits per samples isn't supported",
                    loader.bits_per_sample
                );
            }
        };

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
        if self.sample_rate == 0 || self.channels == 0 || sample == 0 {
            return BytePCM::new(self.channels, sample, 0, 0, Vec::new());
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
        if self.channels == 0 || self.sample_rate == 0 || sample == 0 {
            return Vec::new();
        }
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
                    // Java float->i8 truncation semantics: Java's byte is signed;
                    // assigning byte->short does sign extension.
                    // SAFETY: lossy narrowing matches Java behavior -- u8->i8->i16
                    // replicates Java's sign-extended promotion.
                    let sample1 = self.sample[(position * self.channels as i64 + j as i64) as usize]
                        as i8 as i16;
                    let sample2 = self.sample
                        [((position + 1) * self.channels as i64 + j as i64) as usize]
                        as i8 as i16;
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
            let frame_start = (self.start + start + length - self.channels) as usize;
            let frame_end = (self.start + start + length) as usize;
            let zero = self.sample[frame_start..frame_end].iter().all(|&b| b == 0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_pcm_change_sample_rate_zero_sample_rate() {
        let pcm = BytePCM::new(1, 0, 0, 4, vec![1, 2, 3, 4]);
        let result = pcm.change_sample_rate(44100);
        assert_eq!(result.sample.len(), 0);
    }

    #[test]
    fn test_byte_pcm_change_sample_rate_zero_channels() {
        let pcm = BytePCM::new(0, 44100, 0, 4, vec![1, 2, 3, 4]);
        let result = pcm.change_sample_rate(48000);
        assert_eq!(result.sample.len(), 0);
    }

    #[test]
    fn test_byte_pcm_change_sample_rate_zero_sample() {
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![1, 2, 3, 4]);
        let result = pcm.change_sample_rate(0);
        assert_eq!(result.sample.len(), 0);
    }

    #[test]
    fn get_sample_sign_extension_interpolation() {
        // Java's byte is signed: 0xFF = -1, 0x00 = 0
        // When resampling with interpolation, Java sign-extends byte→short,
        // so 0xFF becomes -1 (not 255). This affects interpolation results.
        //
        // Samples: [0xFF, 0x00] at 44100Hz mono
        // Resample to 88200Hz → 4 output samples
        // At i=1: interpolate between 0xFF(-1) and 0x00(0)
        //   Java (signed): (-1 * 44100 + 0 * 44100) / 88200 = 0 → 0x00
        //   Bug (unsigned): (255 * 44100 + 0 * 44100) / 88200 = 127 → 0x7F
        let pcm = BytePCM::new(1, 44100, 0, 2, vec![0xFF, 0x00]);
        let resampled = pcm.change_sample_rate(88200);

        // With correct sign extension: [0xFF, 0x00, 0x00, 0x00]
        // Without (bug):               [0xFF, 0x7F, 0x00, 0x00]
        assert_eq!(resampled.sample[0], 0xFF);
        assert_eq!(
            resampled.sample[1], 0x00,
            "Interpolation should use signed byte arithmetic (Java sign extension)"
        );
        assert_eq!(resampled.sample[2], 0x00);
    }

    // --- Construction tests ---

    #[test]
    fn new_stores_fields_correctly() {
        let pcm = BytePCM::new(2, 44100, 10, 100, vec![1, 2, 3, 4]);
        assert_eq!(pcm.channels, 2);
        assert_eq!(pcm.sample_rate, 44100);
        assert_eq!(pcm.start, 10);
        assert_eq!(pcm.len, 100);
        assert_eq!(*pcm.sample, vec![1, 2, 3, 4]);
    }

    #[test]
    fn new_empty_data() {
        let pcm = BytePCM::new(1, 44100, 0, 0, Vec::new());
        assert_eq!(pcm.channels, 1);
        assert_eq!(pcm.sample_rate, 44100);
        assert_eq!(pcm.start, 0);
        assert_eq!(pcm.len, 0);
        assert!(pcm.sample.is_empty());
    }

    #[test]
    fn new_single_sample() {
        let pcm = BytePCM::new(1, 44100, 0, 1, vec![128]);
        assert_eq!(pcm.sample.len(), 1);
        assert_eq!(pcm.sample[0], 128);
    }

    #[test]
    fn new_max_min_values() {
        let pcm = BytePCM::new(1, 44100, 0, 2, vec![0x00, 0xFF]);
        assert_eq!(pcm.sample[0], 0);
        assert_eq!(pcm.sample[1], 255);
    }

    // --- Validate tests ---

    #[test]
    fn validate_non_empty() {
        let pcm = BytePCM::new(1, 44100, 0, 1, vec![42]);
        assert!(pcm.validate());
    }

    #[test]
    fn validate_empty() {
        let pcm = BytePCM::new(1, 44100, 0, 0, Vec::new());
        assert!(!pcm.validate());
    }

    // --- Clone / Arc sharing tests ---

    #[test]
    fn clone_shares_sample_arc() {
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![1, 2, 3, 4]);
        let cloned = pcm.clone();
        // Arc::strong_count should be 2 after clone
        assert_eq!(Arc::strong_count(&pcm.sample), 2);
        assert_eq!(*cloned.sample, vec![1, 2, 3, 4]);
    }

    // --- load_pcm tests ---

    #[test]
    fn load_pcm_8bit() {
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![100, 200, 50, 255],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 8,
            block_align: 1,
        };
        let pcm = BytePCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.channels, 1);
        assert_eq!(pcm.sample_rate, 44100);
        assert_eq!(pcm.start, 0);
        assert_eq!(pcm.len, 4);
        assert_eq!(*pcm.sample, vec![100, 200, 50, 255]);
    }

    #[test]
    fn load_pcm_16bit_takes_high_byte() {
        // 16-bit LE: [low, high, low, high]
        // Each pair's high byte is taken
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![0x12, 0xAB, 0x34, 0xCD],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            block_align: 2,
        };
        let pcm = BytePCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 2);
        assert_eq!(pcm.sample[0], 0xAB);
        assert_eq!(pcm.sample[1], 0xCD);
    }

    #[test]
    fn load_pcm_24bit_takes_highest_byte() {
        // 24-bit: [b0, b1, b2, b0, b1, b2]
        // Takes byte at index i*3+2 (highest)
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 24,
            block_align: 3,
        };
        let pcm = BytePCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 2);
        assert_eq!(pcm.sample[0], 0x33);
        assert_eq!(pcm.sample[1], 0x66);
    }

    #[test]
    fn load_pcm_32bit_float_conversion() {
        // 1.0f32 * 127 = 127 -> as i32 as i8 = 127 -> as u8 = 127
        let one_f32 = 1.0f32.to_le_bytes();
        // 0.0f32 * 127 = 0 -> as i32 as i8 = 0 -> as u8 = 0
        let zero_f32 = 0.0f32.to_le_bytes();
        // -1.0f32 * 127 = -127 -> as i32 = -127 -> as i8 = -127 -> as u8 = 129
        let neg_one_f32 = (-1.0f32).to_le_bytes();

        let mut data = Vec::new();
        data.extend_from_slice(&one_f32);
        data.extend_from_slice(&zero_f32);
        data.extend_from_slice(&neg_one_f32);

        let loader = crate::pcm::PCMLoader {
            pcm_data: data,
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 32,
            block_align: 4,
        };
        let pcm = BytePCM::load_pcm(&loader).unwrap();
        assert_eq!(pcm.len, 3);
        assert_eq!(pcm.sample[0], 127u8); // 1.0 * 127
        assert_eq!(pcm.sample[1], 0u8); // 0.0 * 127
        assert_eq!(pcm.sample[2], (-127i8) as u8); // -1.0 * 127
    }

    #[test]
    fn load_pcm_unsupported_bits() {
        let loader = crate::pcm::PCMLoader {
            pcm_data: vec![0; 8],
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 4,
            block_align: 1,
        };
        let result = BytePCM::load_pcm(&loader);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("4 bits"));
    }

    // --- change_sample_rate tests ---

    #[test]
    fn change_sample_rate_same_rate_preserves_data() {
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![10, 20, 30, 40]);
        let result = pcm.change_sample_rate(44100);
        assert_eq!(result.sample_rate, 44100);
        assert_eq!(result.len, 4);
        assert_eq!(*result.sample, vec![10, 20, 30, 40]);
    }

    #[test]
    fn change_sample_rate_double_upsamples() {
        // Mono, 4 samples at 44100 -> 88200 should produce ~8 samples
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![0, 100, 200, 50]);
        let result = pcm.change_sample_rate(88200);
        assert_eq!(result.sample_rate, 88200);
        assert_eq!(result.sample.len(), 8);
        // First sample should match original
        assert_eq!(result.sample[0], 0);
    }

    // --- change_channels tests ---

    #[test]
    fn change_channels_mono_to_stereo() {
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![10, 20, 30, 40]);
        let result = pcm.change_channels(2);
        assert_eq!(result.channels, 2);
        // Each mono sample duplicated to both channels
        assert_eq!(result.sample.len(), 8);
        assert_eq!(result.sample[0], 10); // L
        assert_eq!(result.sample[1], 10); // R
        assert_eq!(result.sample[2], 20); // L
        assert_eq!(result.sample[3], 20); // R
    }

    #[test]
    fn change_channels_stereo_to_mono() {
        // Stereo: [L1, R1, L2, R2] -> Mono: takes first channel of each frame
        let pcm = BytePCM::new(2, 44100, 0, 4, vec![10, 20, 30, 40]);
        let result = pcm.change_channels(1);
        assert_eq!(result.channels, 1);
        assert_eq!(result.sample.len(), 2);
        assert_eq!(result.sample[0], 10);
        assert_eq!(result.sample[1], 30);
    }

    // --- slice tests ---

    #[test]
    fn slice_all_silent_mono_returns_some_with_one_frame() {
        // The while loop trims trailing silence but stops when length <= channels.
        // So all-zero mono data results in length=1 (one frame), which is > 0 -> Some.
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![0, 0, 0, 0]);
        let result = pcm.slice(0, 0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len, 1);
    }

    #[test]
    fn slice_returns_some_for_non_silent_data() {
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![100, 50, 25, 0]);
        let result = pcm.slice(0, 0);
        assert!(result.is_some());
        let sliced = result.unwrap();
        // Should share the same Arc
        assert_eq!(Arc::strong_count(&sliced.sample), 2);
        assert_eq!(sliced.channels, 1);
        assert_eq!(sliced.sample_rate, 44100);
    }

    #[test]
    fn slice_trims_trailing_silence() {
        // 4 samples at 44100 mono: [100, 50, 0, 0]
        // Duration 0 means use full length. Trailing zeros trimmed.
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![100, 50, 0, 0]);
        let result = pcm.slice(0, 0).unwrap();
        // After trimming trailing zeros: length should be 2
        assert_eq!(result.len, 2);
    }

    #[test]
    fn slice_with_start_offset() {
        // 8 samples at 8 Hz mono -> 1 sample per 125000us
        // starttime = 125000us -> skip first sample
        let pcm = BytePCM::new(1, 8, 0, 8, vec![0, 100, 50, 25, 10, 5, 0, 0]);
        let result = pcm.slice(125000, 0);
        assert!(result.is_some());
        let sliced = result.unwrap();
        assert_eq!(sliced.start, 1); // starts at sample index 1
    }

    // --- change_frequency tests ---

    #[test]
    fn change_frequency_rate_one_preserves_data() {
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![10, 20, 30, 40]);
        let result = pcm.change_frequency(1.0);
        assert_eq!(result.sample_rate, 44100);
        assert_eq!(*result.sample, vec![10, 20, 30, 40]);
    }

    #[test]
    fn change_frequency_double_speed_halves_samples() {
        // 2x speed: target rate = 44100/2 = 22050
        // 4 samples at 44100 -> ~2 samples
        let pcm = BytePCM::new(1, 44100, 0, 4, vec![10, 20, 30, 40]);
        let result = pcm.change_frequency(2.0);
        assert_eq!(result.sample.len(), 2);
    }
}

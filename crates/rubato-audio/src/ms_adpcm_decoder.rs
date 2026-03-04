use anyhow::{Result, bail};
use log::warn;

/// MS-ADPCM (WAV format 0x0002) decoder.
///
/// Translated from: MSADPCMDecoder.java
pub struct MSADPCMDecoder {
    adapt_coeff1: Vec<i32>,
    adapt_coeff2: Vec<i32>,
    initial_delta: Vec<i32>,
    sample1: Vec<i32>,
    sample2: Vec<i32>,
    channel_samples: Vec<Vec<i16>>,

    samples_per_block: i32,
    channels: i32,
    block_size: i32,
    #[allow(dead_code)]
    sample_rate: i32,
}

#[allow(clippy::upper_case_acronyms)]
static ADAPTION_TABLE: [i32; 16] = [
    230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230,
];

static INITIALIZATION_COEFF1: [i32; 7] = [64, 128, 0, 48, 60, 115, 98];

static INITIALIZATION_COEFF2: [i32; 7] = [0, -64, 0, 16, 0, -52, -58];

impl MSADPCMDecoder {
    pub fn new(channels: i32, sample_rate: i32, block_align: i32) -> Result<Self> {
        if channels == 0 {
            bail!("MSADPCMDecoder: channels must be non-zero");
        }
        let block_size = block_align;
        // sizeof(header) = 7
        // each header contains two samples
        // channels * 2 + (blockSize - channels * sizeof(header)) * 2 ==> (blockSize - channels * 6) * 2
        let samples_per_block = (block_size - channels * 6) * 2 / channels;
        if samples_per_block <= 0 {
            bail!(
                "MSADPCMDecoder: invalid block_align {} for {} channels (samples_per_block={})",
                block_align,
                channels,
                samples_per_block
            );
        }
        Ok(MSADPCMDecoder {
            adapt_coeff1: vec![0; channels as usize],
            adapt_coeff2: vec![0; channels as usize],
            initial_delta: vec![0; channels as usize],
            sample1: vec![0; channels as usize],
            sample2: vec![0; channels as usize],
            channel_samples: Vec::new(),
            samples_per_block,
            channels,
            block_size,
            sample_rate,
        })
    }

    pub fn decode(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        // init decode context
        self.adapt_coeff1 = vec![0; self.channels as usize];
        self.adapt_coeff2 = vec![0; self.channels as usize];
        self.initial_delta = vec![0; self.channels as usize];
        self.sample1 = vec![0; self.channels as usize];
        self.sample2 = vec![0; self.channels as usize];
        if self.channels > 2 {
            self.channel_samples =
                vec![vec![0i16; self.samples_per_block as usize]; self.channels as usize];
        }

        let block_size = self.block_size as usize;
        if !input.len().is_multiple_of(block_size) {
            log::error!("Malformed MS ADPCM block");
            bail!("too few elements left in input buffer");
            // Note: ffmpeg doesn't process incomplete blocks.
        }
        let block_count = input.len() / block_size;
        let block_sample_size = self.samples_per_block as usize * self.channels as usize * 2;
        let mut out = vec![0u8; block_count * block_sample_size];

        let mut in_pos = 0usize;
        let mut out_pos = 0usize;
        while in_pos < input.len() {
            let block = &input[in_pos..in_pos + block_size];
            self.decode_block(&mut out[out_pos..out_pos + block_sample_size], block)?;
            in_pos += block_size;
            out_pos += block_sample_size;
        }

        Ok(out)
    }

    fn decode_block(&mut self, out: &mut [u8], block_data: &[u8]) -> Result<()> {
        let mut block_pos = 0usize;
        let mut out_short_pos = 0usize;

        if self.channels > 2 {
            // When channels > 2, channels are NOT interleaved.
            for ch in 0..self.channels as usize {
                let predictor = block_data[block_pos] as u32 as i32;
                block_pos += 1;
                if predictor > 6 {
                    warn!("Malformed block header");
                    bail!(
                        "Malformed block header. Expected range for predictor 0..6, found {}",
                        predictor
                    );
                }

                // Initialize the Adaption coefficients for each channel by indexing
                // into the coeff. table with the predictor value (range 0..6)
                self.adapt_coeff1[ch] = INITIALIZATION_COEFF1[predictor as usize];
                self.adapt_coeff2[ch] = INITIALIZATION_COEFF2[predictor as usize];

                self.initial_delta[ch] = read_i16_le(block_data, block_pos) as i32;
                block_pos += 2;

                // Acquire initial uncompressed signed 16 bit PCM samples for initialization
                self.sample1[ch] = read_i16_le(block_data, block_pos) as i32;
                block_pos += 2;

                self.sample2[ch] = read_i16_le(block_data, block_pos) as i32;
                block_pos += 2;

                let mut sample_ptr = 0usize;
                self.channel_samples[ch][sample_ptr] = self.sample2[ch] as i16;
                sample_ptr += 1;
                self.channel_samples[ch][sample_ptr] = self.sample1[ch] as i16;
                sample_ptr += 1;

                let n_count = (self.samples_per_block - 2) >> 1;
                for _n in 0..n_count {
                    let current_byte = block_data[block_pos] as u32 as i32;
                    block_pos += 1;

                    self.channel_samples[ch][sample_ptr] =
                        self.expand_nibble((current_byte & 0xFF) >> 4, ch);
                    sample_ptr += 1;
                    self.channel_samples[ch][sample_ptr] =
                        self.expand_nibble((current_byte & 0xFF) & 0xf, ch);
                    sample_ptr += 1;
                }
            }
            // interleave samples
            for i in 0..self.samples_per_block as usize {
                for j in 0..self.channels as usize {
                    write_i16_le(out, out_short_pos, self.channel_samples[j][i]);
                    out_short_pos += 2;
                }
            }
        } else {
            // Channels <= 2: interleaved preamble
            for ch in 0..self.channels as usize {
                let predictor = block_data[block_pos] as u32 as i32;
                block_pos += 1;
                if predictor > 6 {
                    warn!("Malformed block header");
                    bail!(
                        "Malformed block header. Expected range for predictor 0..6, found {}",
                        predictor
                    );
                }

                self.adapt_coeff1[ch] = INITIALIZATION_COEFF1[predictor as usize];
                self.adapt_coeff2[ch] = INITIALIZATION_COEFF2[predictor as usize];
            }

            for ch in 0..self.channels as usize {
                self.initial_delta[ch] = read_i16_le(block_data, block_pos) as i32;
                block_pos += 2;
            }

            // Acquire initial uncompressed signed 16 bit PCM samples for initialization
            for ch in 0..self.channels as usize {
                self.sample1[ch] = read_i16_le(block_data, block_pos) as i32;
                block_pos += 2;
            }

            for ch in 0..self.channels as usize {
                self.sample2[ch] = read_i16_le(block_data, block_pos) as i32;
                block_pos += 2;
            }

            for ch in 0..self.channels as usize {
                write_i16_le(out, out_short_pos, self.sample2[ch] as i16);
                out_short_pos += 2;
            }

            for ch in 0..self.channels as usize {
                write_i16_le(out, out_short_pos, self.sample1[ch] as i16);
                out_short_pos += 2;
            }

            let mut ch: usize = 0;

            // while blockData.hasRemaining()
            while block_pos < block_data.len() {
                let current_byte = block_data[block_pos] as u32 as i32;
                block_pos += 1;

                let s1 = self.expand_nibble((current_byte & 0xFF) >> 4, ch);
                write_i16_le(out, out_short_pos, s1);
                out_short_pos += 2;
                ch = (ch + 1) % self.channels as usize;

                let s2 = self.expand_nibble((current_byte & 0xFF) & 0xf, ch);
                write_i16_le(out, out_short_pos, s2);
                out_short_pos += 2;
                ch = (ch + 1) % self.channels as usize;
            }
        }

        Ok(())
    }

    fn expand_nibble(&mut self, nibble: i32, channel: usize) -> i16 {
        let signed: i32 = if nibble >= 8 { nibble - 16 } else { nibble };

        let result = (self.sample1[channel] * self.adapt_coeff1[channel])
            + (self.sample2[channel] * self.adapt_coeff2[channel]);
        let predictor: i16 = Self::clamp((result >> 6) + (signed * self.initial_delta[channel]));

        self.sample2[channel] = self.sample1[channel];
        self.sample1[channel] = predictor as i32;

        self.initial_delta[channel] =
            (ADAPTION_TABLE[nibble as usize] * self.initial_delta[channel]) >> 8;
        if self.initial_delta[channel] < 16 {
            self.initial_delta[channel] = 16;
        }
        if self.initial_delta[channel] > i32::MAX / 768 {
            warn!("idelta overflow");
            self.initial_delta[channel] = i32::MAX / 768;
        }
        predictor
    }

    fn clamp(value: i32) -> i16 {
        value.max(i16::MIN as i32).min(i16::MAX as i32) as i16
    }
}

/// Read a little-endian i16 from a byte slice at the given offset.
fn read_i16_le(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Write a little-endian i16 to a byte slice at the given offset.
fn write_i16_le(data: &mut [u8], offset: usize, value: i16) {
    let bytes = value.to_le_bytes();
    data[offset] = bytes[0];
    data[offset + 1] = bytes[1];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp() {
        assert_eq!(MSADPCMDecoder::clamp(0), 0);
        assert_eq!(MSADPCMDecoder::clamp(32767), 32767);
        assert_eq!(MSADPCMDecoder::clamp(-32768), -32768);
        assert_eq!(MSADPCMDecoder::clamp(40000), 32767);
        assert_eq!(MSADPCMDecoder::clamp(-40000), -32768);
    }

    #[test]
    fn test_ms_adpcm_decoder_zero_channels() {
        let result = MSADPCMDecoder::new(0, 44100, 256);
        assert!(
            result.is_err(),
            "MSADPCMDecoder::new with channels=0 should return Err"
        );
    }

    #[test]
    fn test_ms_adpcm_decoder_negative_samples_per_block() {
        // block_align=1, channels=1: (1 - 6) * 2 / 1 = -10
        let result = MSADPCMDecoder::new(1, 44100, 1);
        assert!(result.is_err(), "negative samples_per_block should fail");
    }

    #[test]
    fn test_ms_adpcm_decoder_zero_samples_per_block() {
        // block_align=6, channels=1: (6 - 6) * 2 / 1 = 0
        let result = MSADPCMDecoder::new(1, 44100, 6);
        assert!(result.is_err(), "zero samples_per_block should fail");
    }

    #[test]
    fn test_ms_adpcm_decoder_valid_block_align() {
        // block_align=256, channels=1: (256 - 6) * 2 / 1 = 500
        let result = MSADPCMDecoder::new(1, 44100, 256);
        assert!(result.is_ok(), "valid block_align should succeed");
    }

    #[test]
    fn test_read_write_i16_le() {
        let mut buf = [0u8; 4];
        write_i16_le(&mut buf, 0, 0x0102);
        assert_eq!(buf[0], 0x02);
        assert_eq!(buf[1], 0x01);
        assert_eq!(read_i16_le(&buf, 0), 0x0102);

        write_i16_le(&mut buf, 2, -1);
        assert_eq!(buf[2], 0xFF);
        assert_eq!(buf[3], 0xFF);
        assert_eq!(read_i16_le(&buf, 2), -1);
    }
}

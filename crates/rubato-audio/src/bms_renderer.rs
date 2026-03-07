use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bms_model::bms_model::BMSModel;
use bms_model::note::Note;
use log::{info, trace, warn};

use crate::audio_driver;
use crate::pcm::PCM;

/// Convert f32 samples (normalized [-1.0, 1.0]) to i16 samples.
///
/// Translated from: ShortPCM.loadPCM 32-bit case: `(short)(pcm.getFloat() * Short.MAX_VALUE)`
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

/// Renders BMS to PCM buffer.
///
/// Translated from: BMSRenderer.java
pub struct BMSRenderer {
    sample_rate: i32,
    channels: i32,
}

/// Render result containing the PCM data and metadata.
///
/// Translated from: BMSRenderer.RenderResult
pub struct RenderResult {
    pub pcm_data: Vec<u8>,
    pub sample_rate: i32,
    pub channels: i32,
    pub duration_ms: i64,
}

impl BMSRenderer {
    pub fn new(sample_rate: i32, channels: i32) -> Self {
        BMSRenderer {
            sample_rate,
            channels,
        }
    }

    pub fn new_default() -> Self {
        BMSRenderer::new(44100, 2)
    }

    pub fn render_bms(&self, model: &BMSModel) -> Option<RenderResult> {
        self.render_bms_with_limit(model, 0)
    }

    pub fn render_bms_with_limit(
        &self,
        model: &BMSModel,
        max_duration_ms: i64,
    ) -> Option<RenderResult> {
        let wav_cache = self.load_wav_files(model);

        // Calculate output buffer size
        // (number of samples = sampling rate * seconds)
        let mut end_time = model.last_milli_time();

        // Apply time limit if specified (0 = no limit)
        if max_duration_ms > 0 && end_time > max_duration_ms {
            info!(
                "Limiting render duration from {}ms to {}ms",
                end_time, max_duration_ms
            );
            end_time = max_duration_ms;
        }

        let total_samples = end_time * self.sample_rate as i64 / 1000;
        let bytes_per_sample: i32 = 2; // 16-bit
        let buffer_size = (total_samples * self.channels as i64 * bytes_per_sample as i64) as usize;

        info!(
            "Rendering chart: 0ms - {}ms (total {} samples, {} bytes)",
            end_time, total_samples, buffer_size
        );

        // Create mix buffer (float)
        let mix_len = (total_samples * self.channels as i64) as usize;
        let mut mix_buffer = vec![0.0f32; mix_len];

        // Process all timelines
        let timelines = &model.timelines;

        for tl in timelines {
            let time = tl.milli_time();
            if time >= end_time {
                break;
            }
            for note in tl.back_ground_notes() {
                self.render_note(note, time, &wav_cache, &mut mix_buffer);
            }
            let lanes = model.mode().map(|m| m.key()).unwrap_or(0);
            for i in 0..lanes {
                if let Some(note) = tl.note(i) {
                    self.render_note(note, time, &wav_cache, &mut mix_buffer);
                    for layered in note.layered_notes() {
                        self.render_note(layered, time, &wav_cache, &mut mix_buffer);
                    }
                }
            }
        }

        // Float -> Int16 with -6dB headroom
        let mut output_buffer = Vec::with_capacity(buffer_size);
        for sample in &mix_buffer {
            let mut s = *sample;
            // -6dB headroom to try to alleviate clipping
            s *= 0.5f32;

            s = s.clamp(-1.0f32, 1.0f32);

            let short_val = (s * 32767.0f32) as i16;
            output_buffer.extend_from_slice(&short_val.to_le_bytes());
        }

        Some(RenderResult {
            pcm_data: output_buffer,
            sample_rate: self.sample_rate,
            channels: self.channels,
            duration_ms: end_time,
        })
    }

    fn render_note(
        &self,
        note: &Note,
        note_time: i64,
        wav_cache: &HashMap<i32, PCM>,
        mix_buffer: &mut [f32],
    ) -> bool {
        let wav_id = note.wav();
        if wav_id < 0 {
            return false;
        }
        let pcm = match wav_cache.get(&wav_id) {
            Some(p) => p,
            None => return false,
        };

        let start_sample = note_time * self.sample_rate as i64 / 1000;
        let micro_start_time = note.micro_starttime();
        let micro_duration = note.micro_duration();

        let render_pcm: PCM;
        if micro_start_time > 0 || micro_duration > 0 {
            match pcm.slice(micro_start_time, micro_duration) {
                Some(sliced) => render_pcm = sliced,
                None => return false,
            }
        } else {
            render_pcm = pcm.clone();
        }

        // Mix PCM data
        self.mix_pcm(&render_pcm, start_sample as i32, mix_buffer);
        true
    }

    fn mix_pcm(&self, pcm: &PCM, start_sample: i32, mix_buffer: &mut [f32]) {
        let mut pcm_owned: PCM;
        let pcm = if pcm.sample_rate() != self.sample_rate {
            pcm_owned = pcm.change_sample_rate(self.sample_rate);
            if pcm_owned.channels() != self.channels {
                pcm_owned = pcm_owned.change_channels(self.channels);
            }
            &pcm_owned
        } else if pcm.channels() != self.channels {
            pcm_owned = pcm.change_channels(self.channels);
            &pcm_owned
        } else {
            pcm
        };

        match pcm {
            PCM::Short(short_pcm) => {
                self.mix_short_pcm(short_pcm, start_sample, mix_buffer);
            }
            PCM::Float(float_pcm) => {
                self.mix_float_pcm(float_pcm, start_sample, mix_buffer);
            }
            PCM::Byte(byte_pcm) => {
                self.mix_byte_pcm(byte_pcm, start_sample, mix_buffer);
            }
        }
    }

    fn mix_short_pcm(
        &self,
        pcm: &crate::short_pcm::ShortPCM,
        start_sample: i32,
        mix_buffer: &mut [f32],
    ) {
        let samples = &pcm.sample;
        let mut src_index = pcm.start as usize;
        let mut dst_index = (start_sample * self.channels) as usize;
        let len = pcm.len as usize;

        for _i in 0..len {
            if dst_index >= mix_buffer.len() || src_index >= samples.len() {
                break;
            }
            mix_buffer[dst_index] += samples[src_index] as f32 / 32768.0;
            src_index += 1;
            dst_index += 1;
        }
    }

    fn mix_float_pcm(
        &self,
        pcm: &crate::float_pcm::FloatPCM,
        start_sample: i32,
        mix_buffer: &mut [f32],
    ) {
        let samples = &pcm.sample;
        let mut src_index = pcm.start as usize;
        let mut dst_index = (start_sample * self.channels) as usize;
        let len = pcm.len as usize;

        for _i in 0..len {
            if dst_index >= mix_buffer.len() || src_index >= samples.len() {
                break;
            }
            mix_buffer[dst_index] += samples[src_index];
            src_index += 1;
            dst_index += 1;
        }
    }

    fn mix_byte_pcm(
        &self,
        pcm: &crate::byte_pcm::BytePCM,
        start_sample: i32,
        mix_buffer: &mut [f32],
    ) {
        let samples = &pcm.sample;
        let mut src_index = pcm.start as usize;
        let mut dst_index = (start_sample * self.channels) as usize;
        let len = pcm.len as usize;

        for _i in 0..len {
            if dst_index >= mix_buffer.len() || src_index >= samples.len() {
                break;
            }
            mix_buffer[dst_index] += samples[src_index] as f32 / 128.0;
            src_index += 1;
            dst_index += 1;
        }
    }

    fn load_wav_files(&self, model: &BMSModel) -> HashMap<i32, PCM> {
        info!("Loading audio files...");

        let mut result: HashMap<i32, PCM> = HashMap::new();
        let wav_list = &model.wavmap;
        let model_path = match model.path() {
            Some(p) => p,
            None => return result,
        };
        let base_path = Path::new(&model_path).parent().unwrap_or(Path::new(""));

        let mut loaded = 0;
        for (i, wav_name) in wav_list.iter().enumerate() {
            if wav_name.is_empty() {
                continue;
            }

            // Resolve audio file path
            let resolved_path = base_path.join(wav_name);
            let resolved_str = resolved_path.to_string_lossy().to_string();
            let candidates = audio_driver::paths(&resolved_str);

            let mut wav_path: Option<PathBuf> = None;
            for candidate in &candidates {
                if candidate.exists() {
                    wav_path = Some(candidate.clone());
                    break;
                }
            }

            let wav_path = match wav_path {
                Some(p) => p,
                None => {
                    warn!("Audio file not found: {}", wav_name);
                    continue;
                }
            };

            // Load as PCM
            if let Some(pcm) = PCM::load(&wav_path, self.channels, self.sample_rate) {
                result.insert(i as i32, pcm);
                loaded += 1;
            } else {
                trace!("Failed to load audio file: {:?}", wav_path);
            }
        }

        info!("Audio files loaded: {} / {}", loaded, wav_list.len());
        result
    }
}

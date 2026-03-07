use std::path::PathBuf;

use bms_model::bms_model::BMSModel;

use crate::bms_renderer::BMSRenderer;

/// BMS loudness analyzer using EBU R128 integrated loudness measurement.
///
/// Translated from: BMSLoudnessAnalyzer.java
pub struct BMSLoudnessAnalyzer {
    available: bool,
    cache_dir: PathBuf,
}

/// Analysis result containing loudness measurement.
///
/// Translated from: BMSLoudnessAnalyzer.AnalysisResult
pub struct AnalysisResult {
    pub loudness_lufs: f64,
    pub success: bool,
    pub error_message: Option<String>,
}

impl AnalysisResult {
    pub fn new_success(loudness_lufs: f64) -> Self {
        AnalysisResult {
            loudness_lufs,
            success: true,
            error_message: None,
        }
    }

    pub fn new_error(error_message: String) -> Self {
        AnalysisResult {
            loudness_lufs: f64::NAN,
            success: false,
            error_message: Some(error_message),
        }
    }

    pub fn calculate_adjusted_volume(&self, base_volume: f32) -> f32 {
        if !self.success || self.loudness_lufs.is_nan() {
            return base_volume;
        }

        // Average loudness level (50% volume)
        let average_lufs: f64 = -12.00;

        let loudness_diff = self.loudness_lufs - average_lufs;
        let gain_adjustment = 10.0f64.powf(-loudness_diff / 20.0);

        let adjusted_volume = (0.5f64 * gain_adjustment) as f32;
        adjusted_volume.clamp(0.0, 1.0)
    }
}

impl BMSLoudnessAnalyzer {
    pub fn new() -> Self {
        let cache_dir = PathBuf::from("cache/normalize");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            log::warn!("Failed to create cache directory: {}", e);
        }
        BMSLoudnessAnalyzer {
            available: true,
            cache_dir,
        }
    }

    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Analyze the loudness of a BMS chart.
    ///
    /// Checks cache first, then renders the chart to PCM and measures
    /// EBU R128 integrated loudness. Results are cached by SHA256 hash.
    ///
    /// Translated from: BMSLoudnessAnalyzer.analyze(BMSModel)
    pub fn analyze(&self, model: &BMSModel) -> AnalysisResult {
        // Check cache first
        let hash = &model.sha256;
        if !hash.is_empty()
            && let Some(cached) = Self::read_from_cache(&self.cache_dir, hash)
        {
            return AnalysisResult::new_success(cached);
        }

        match self.analyze_inner(model) {
            Ok(result) => {
                // Write to cache on success
                if result.success && !hash.is_empty() {
                    Self::write_to_cache(&self.cache_dir, hash, result.loudness_lufs);
                }
                result
            }
            Err(e) => AnalysisResult::new_error(format!("Analysis failed: {}", e)),
        }
    }

    /// Render BMS to PCM and measure EBU R128 integrated loudness.
    ///
    /// Translated from: BMSLoudnessAnalyzer.analyze + analyzeLoudness
    fn analyze_inner(&self, model: &BMSModel) -> anyhow::Result<AnalysisResult> {
        log::info!("BMSLoudnessAnalyzer: rendering chart to PCM");

        let renderer = BMSRenderer::new_default();
        let result = renderer
            .render_bms_with_limit(model, 10 * 60 * 1000) // 10 minute limit
            .ok_or_else(|| anyhow::anyhow!("Failed to render BMS file"))?;

        log::info!("BMSLoudnessAnalyzer: analyzing loudness");
        let loudness = Self::analyze_loudness(&result)?;
        Ok(AnalysisResult::new_success(loudness))
    }

    /// Measure EBU R128 integrated loudness from rendered PCM data.
    ///
    /// Translated from: BMSLoudnessAnalyzer.analyzeLoudness(RenderResult)
    fn analyze_loudness(result: &crate::bms_renderer::RenderResult) -> anyhow::Result<f64> {
        let channels = result.channels as u32;
        let sample_rate = result.sample_rate as u32;

        let mut state = ebur128::EbuR128::new(channels, sample_rate, ebur128::Mode::I)
            .map_err(|e| anyhow::anyhow!("Failed to create EbuR128 state: {}", e))?;

        if channels == 2 {
            state
                .set_channel(0, ebur128::Channel::Left)
                .map_err(|e| anyhow::anyhow!("Failed to set channel 0: {}", e))?;
            state
                .set_channel(1, ebur128::Channel::Right)
                .map_err(|e| anyhow::anyhow!("Failed to set channel 1: {}", e))?;
        }

        // Convert PCM bytes (little-endian i16) to i16 slice
        let pcm_data = &result.pcm_data;
        let samples: Vec<i16> = pcm_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        state
            .add_frames_i16(&samples)
            .map_err(|e| anyhow::anyhow!("Failed to add frames: {}", e))?;

        let loudness = state
            .loudness_global()
            .map_err(|e| anyhow::anyhow!("Failed to get integrated loudness: {}", e))?;

        if loudness.is_infinite() && loudness < 0.0 {
            anyhow::bail!("Failed to get integrated loudness (silence)");
        }

        Ok(loudness)
    }

    pub fn shutdown(&self) {
        // No-op
    }

    /// Reads cached loudness value for the given hash.
    ///
    /// Translated from: BMSLoudnessAnalyzer.readFromCache(String)
    fn read_from_cache(cache_dir: &std::path::Path, hash: &str) -> Option<f64> {
        let cache_file = cache_dir.join(format!("{}.lufs", hash));
        match std::fs::read_to_string(&cache_file) {
            Ok(content) => match content.trim().parse::<f64>() {
                Ok(value) => Some(value),
                Err(e) => {
                    log::warn!("Failed to parse cache for {}: {}", hash, e);
                    None
                }
            },
            Err(_) => None,
        }
    }

    /// Writes loudness value to cache for the given hash.
    ///
    /// Translated from: BMSLoudnessAnalyzer.writeToCache(String, double)
    fn write_to_cache(cache_dir: &std::path::Path, hash: &str, loudness: f64) {
        let cache_file = cache_dir.join(format!("{}.lufs", hash));
        match std::fs::write(&cache_file, loudness.to_string().as_bytes()) {
            Ok(()) => {
                log::info!("Cached loudness for {}: {} LUFS", hash, loudness);
            }
            Err(e) => {
                log::warn!("Failed to write cache for {}: {}", hash, e);
            }
        }
    }
}

impl Default for BMSLoudnessAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_result_success() {
        let result = AnalysisResult::new_success(-14.0);
        assert!(result.success);
        assert_eq!(result.loudness_lufs, -14.0);
        assert!(result.error_message.is_none());
    }

    #[test]
    fn analysis_result_error() {
        let result = AnalysisResult::new_error("test error".to_string());
        assert!(!result.success);
        assert!(result.loudness_lufs.is_nan());
        assert_eq!(result.error_message.as_deref(), Some("test error"));
    }

    #[test]
    fn adjusted_volume_on_error_returns_base() {
        let result = AnalysisResult::new_error("err".to_string());
        assert_eq!(result.calculate_adjusted_volume(0.8), 0.8);
    }

    #[test]
    fn adjusted_volume_at_average_lufs() {
        // At -12 LUFS (average), gain_adjustment = 10^0 = 1.0, adjusted = 0.5
        let result = AnalysisResult::new_success(-12.0);
        let vol = result.calculate_adjusted_volume(1.0);
        assert!((vol - 0.5).abs() < 0.01);
    }

    #[test]
    fn adjusted_volume_clamped_to_range() {
        // Very quiet track: large negative LUFS -> high gain -> clamped to 1.0
        let result = AnalysisResult::new_success(-60.0);
        let vol = result.calculate_adjusted_volume(1.0);
        assert_eq!(vol, 1.0);
    }

    #[test]
    fn cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path();

        BMSLoudnessAnalyzer::write_to_cache(cache_dir, "testhash", -14.5);
        let cached = BMSLoudnessAnalyzer::read_from_cache(cache_dir, "testhash");
        assert_eq!(cached, Some(-14.5));
    }

    #[test]
    fn cache_miss_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = BMSLoudnessAnalyzer::read_from_cache(dir.path(), "nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn analyze_loudness_sine_wave() {
        // Create a simple stereo 44100Hz sine wave buffer as le i16 bytes
        let sample_rate = 44100;
        let channels = 2;
        let duration_secs = 2;
        let total_frames = sample_rate * duration_secs;
        let freq = 1000.0f64;

        let mut pcm_data = Vec::with_capacity(total_frames * channels * 2);
        for i in 0..total_frames {
            let t = i as f64 / sample_rate as f64;
            let sample = (2.0 * std::f64::consts::PI * freq * t).sin();
            // -6dB headroom like BMSRenderer applies
            let val = (sample * 0.5 * 32767.0) as i16;
            // stereo: same value for L and R
            pcm_data.extend_from_slice(&val.to_le_bytes());
            pcm_data.extend_from_slice(&val.to_le_bytes());
        }

        let render_result = crate::bms_renderer::RenderResult {
            pcm_data,
            sample_rate: sample_rate as i32,
            channels: channels as i32,
            duration_ms: (duration_secs * 1000) as i64,
        };

        let loudness = BMSLoudnessAnalyzer::analyze_loudness(&render_result).unwrap();
        // A 1kHz sine at -6dB should measure around -9 to -10 LUFS
        // The exact value depends on the K-weighting filter but should be finite and negative
        assert!(loudness.is_finite());
        assert!(loudness < 0.0);
        // Reasonable range for a -6dB sine wave
        assert!(loudness > -20.0 && loudness < -5.0);
    }
}

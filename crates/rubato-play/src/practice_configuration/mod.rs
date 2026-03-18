mod constants;
mod ui;

#[cfg(test)]
mod tests;

use constants::{DPRANDOM, GAUGE, GRAPHTYPESTR, RANDOM};
pub use constants::{PracticeColor, PracticeDrawCommand};

use crate::bms_player_rule::BMSPlayerRule;
use crate::gauge_property::GaugeProperty;
use crate::groove_gauge::{GrooveGauge, create_groove_gauge};
use bms_model::bms_model::BMSModel;
use bms_model::bms_model_utils;
use bms_model::mode::Mode;
use rubato_core::pattern::lane_shuffle_modifier::PlayerFlipModifier;
use rubato_core::pattern::pattern_modifier::{PatternModifier, create_pattern_modifier};
use rubato_core::pattern::practice_modifier::PracticeModifier;
use rubato_types::player_config::PlayerConfig;
use serde::{Deserialize, Serialize};

/// Practice mode settings
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PracticeProperty {
    /// Play start time
    pub starttime: i32,
    /// Play end time
    pub endtime: i32,
    /// Selected gauge category
    #[serde(skip)]
    pub gaugecategory: Option<GaugeProperty>,
    /// Selected gauge type
    pub gaugetype: i32,
    /// Start gauge value
    pub startgauge: i32,
    /// 1P option
    pub random: i32,
    /// 2P option
    pub random2: i32,
    /// DP option
    pub doubleop: i32,
    /// Judge window
    pub judgerank: i32,
    /// Playback speed ratio
    pub freq: i32,
    /// TOTAL value
    pub total: f64,
    /// Graph type
    pub graphtype: i32,
}

impl PracticeProperty {
    pub fn new() -> Self {
        PracticeProperty {
            starttime: 0,
            endtime: 10000,
            gaugecategory: None,
            gaugetype: 2,
            startgauge: 20,
            random: 0,
            random2: 0,
            doubleop: 0,
            judgerank: 100,
            freq: 100,
            total: 0.0,
            graphtype: 0,
        }
    }

    /// Clamp all index fields to valid array bounds.
    /// Called after deserialization to prevent out-of-bounds panics.
    pub(super) fn sanitize(&mut self) {
        self.gaugetype = self.gaugetype.rem_euclid(GAUGE.len() as i32);
        self.random = self.random.rem_euclid(RANDOM.len() as i32);
        self.random2 = self.random2.rem_euclid(RANDOM.len() as i32);
        self.doubleop = self.doubleop.rem_euclid(DPRANDOM.len() as i32);
        self.graphtype = self.graphtype.rem_euclid(GRAPHTYPESTR.len() as i32);
        self.freq = self.freq.clamp(50, 200);
    }
}

/// Result of applying practice configuration to a model.
/// Returned by `PracticeConfiguration::apply_to_model`.
pub struct PracticeApplyResult {
    /// Groove gauge initialized with practice start gauge
    pub gauge: Option<GrooveGauge>,
    /// Frequency ratio if freq != 100 (caller should set global audio pitch)
    pub freq_ratio: Option<f32>,
    /// Adjusted start time offset in milliseconds
    pub starttimeoffset: i64,
    /// Adjusted play time limit in milliseconds
    pub playtime: i64,
}

/// Cached model data for practice mode (extracted from BMSModel to avoid Clone)
pub(super) struct PracticeModelData {
    pub(super) mode: Mode,
    /// Time values from all timelines (used for start/end time bounds)
    pub(super) timeline_times: Vec<i64>,
}

/// Practice mode configuration display/edit
pub struct PracticeConfiguration {
    pub(super) cursorpos: usize,
    pub(super) presscount: i64,
    pub(super) model_data: Option<PracticeModelData>,
    pub(super) property: PracticeProperty,
    /// SHA256 of the current model (for save/load path)
    sha256: String,
}

impl Default for PracticeConfiguration {
    fn default() -> Self {
        Self::new()
    }
}

impl PracticeConfiguration {
    pub fn new() -> Self {
        PracticeConfiguration {
            cursorpos: 0,
            presscount: 0,
            model_data: None,
            property: PracticeProperty::new(),
            sha256: String::new(),
        }
    }

    pub fn create(&mut self, model: &BMSModel) {
        self.sha256 = model.sha256.clone();
        self.property.judgerank = model.judgerank;
        self.property.endtime =
            (model.last_time() + 1000).clamp(i32::MIN as i64, i32::MAX as i64) as i32;

        // Load saved practice property from practice/<sha256>.json if exists
        if !self.sha256.is_empty() {
            let path = format!("practice/{}.json", self.sha256);
            if let Ok(data) = std::fs::read_to_string(&path)
                && let Ok(mut saved) = serde_json::from_str::<PracticeProperty>(&data)
            {
                saved.sanitize();
                self.property = saved;
                // Re-initialize gaugecategory (skipped by serde) from current mode
                let mode = model.mode().copied().unwrap_or(Mode::BEAT_7K);
                self.property.gaugecategory = Some(BMSPlayerRule::for_mode(&mode).gauge);
                // Restore total from model if saved config omitted it (older version / manual edit)
                if self.property.total == 0.0 {
                    self.property.total = model.total;
                }
                let timeline_times: Vec<i64> = model.timelines.iter().map(|tl| tl.time()).collect();
                self.model_data = Some(PracticeModelData {
                    mode,
                    timeline_times,
                });
                return;
            }
        }

        if self.property.gaugecategory.is_none() {
            let mode = model.mode().copied().unwrap_or(Mode::BEAT_7K);
            self.property.gaugecategory = Some(BMSPlayerRule::for_mode(&mode).gauge);
        }
        let mode = model.mode().copied().unwrap_or(Mode::BEAT_7K);
        let timeline_times: Vec<i64> = model.timelines.iter().map(|tl| tl.time()).collect();
        self.model_data = Some(PracticeModelData {
            mode,
            timeline_times,
        });
        if self.property.total == 0.0 {
            self.property.total = model.total;
        }
    }

    /// Save practice property to practice/<sha256>.json.
    /// Translates: PracticeConfiguration.saveProperty()
    pub fn save_property(&self) {
        if self.sha256.is_empty() {
            return;
        }
        let path = format!("practice/{}.json", self.sha256);
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&self.property) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    log::warn!("Failed to save practice property: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to serialize practice property: {}", e);
            }
        }
    }

    pub fn practice_property(&self) -> &PracticeProperty {
        &self.property
    }

    pub fn practice_property_mut(&mut self) -> &mut PracticeProperty {
        &mut self.property
    }

    pub fn gauge(&self, model: &BMSModel) -> Option<GrooveGauge> {
        let gauge_category = self
            .property
            .gaugecategory
            .unwrap_or(GaugeProperty::SevenKeys);
        let mut gauge =
            create_groove_gauge(model, self.property.gaugetype, 0, Some(gauge_category))?;
        gauge.set_value(self.property.startgauge as f32);
        Some(gauge)
    }

    pub fn dispose(&mut self) {
        // cleanup rendering resources (stub - no GPU resources in Rust translation)
    }

    /// Apply practice settings to the model.
    ///
    /// Translates Java BMSPlayer lines 684-723 (practice mode initialization).
    /// Modifies the model in-place (frequency, total, time range, pattern, judgerank).
    /// Returns timing offsets and gauge for the caller.
    pub fn apply_to_model(
        &self,
        model: &mut BMSModel,
        config: &PlayerConfig,
    ) -> PracticeApplyResult {
        let property = &self.property;

        // Frequency change
        let freq_ratio = if property.freq != 100 {
            let ratio = property.freq as f32 / 100.0;
            bms_model_utils::change_frequency(model, ratio);
            Some(ratio)
        } else {
            None
        };

        // Set total
        model.total = property.total;

        // PracticeModifier: filter notes outside the time range (scaled by freq)
        // freq is clamped to [50, 200] by update_config(), so division by zero cannot occur.
        let pm_start = (property.starttime as i64) * 100 / (property.freq as i64);
        let pm_end = (property.endtime as i64) * 100 / (property.freq as i64);
        let mut pm = PracticeModifier::new(pm_start, pm_end);
        pm.modify(model);

        // DP modifiers
        let mode = model.mode().copied().unwrap_or(Mode::BEAT_7K);
        if mode.player() == 2 {
            if property.doubleop == 1 {
                PlayerFlipModifier::new().modify(model);
            }
            create_pattern_modifier(property.random2, 1, &mode, config).modify(model);
        }

        // 1P random modifier
        create_pattern_modifier(property.random, 0, &mode, config).modify(model);

        // Second PracticeModifier application (preserves Java behavior)
        pm.modify(model);

        // Gauge
        let gauge = self.gauge(model);

        // Judge rank
        model.judgerank = property.judgerank;

        // Timing calculations
        let starttimeoffset = if property.starttime > 1000 {
            (property.starttime as i64 - 1000) * 100 / property.freq as i64
        } else {
            0
        };
        let playtime = (property.endtime as i64 + 1000) * 100 / property.freq as i64;

        PracticeApplyResult {
            gauge,
            freq_ratio,
            starttimeoffset,
            playtime,
        }
    }
}

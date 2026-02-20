// Scroll speed modifier — normalizes or randomizes scroll speed.
//
// Ported from Java: ScrollSpeedModifier.java

use bms_model::BmsModel;
use rand::Rng;

use crate::modifier::{AssistLevel, PatternModifier};

/// Mode for scroll speed modification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollSpeedMode {
    /// Remove BPM changes and stop events, normalize to initial BPM
    Remove,
    /// Add random scroll speed changes every N sections
    Add,
}

/// Normalizes or randomizes scroll speed.
///
/// Remove mode: normalizes all BPM to initial, removes stop events.
/// Add mode: randomizes scroll speed per section.
///
/// Java: `ScrollSpeedModifier`
pub struct ScrollSpeedModifier {
    pub mode: ScrollSpeedMode,
    /// Section interval for scroll changes (Add mode)
    pub section: u32,
    /// Scroll rate variance (Add mode)
    pub rate: f64,
    /// Track assist level
    assist: AssistLevel,
}

impl ScrollSpeedModifier {
    pub fn new(mode: ScrollSpeedMode) -> Self {
        Self {
            mode,
            section: 4,
            rate: 0.5,
            assist: AssistLevel::None,
        }
    }

    pub fn with_section(mut self, section: u32) -> Self {
        self.section = section;
        self
    }

    pub fn with_rate(mut self, rate: f64) -> Self {
        self.rate = rate;
        self
    }
}

impl PatternModifier for ScrollSpeedModifier {
    fn modify(&mut self, model: &mut BmsModel) {
        match self.mode {
            ScrollSpeedMode::Remove => self.remove_scroll_changes(model),
            ScrollSpeedMode::Add => self.add_scroll_changes(model),
        }
    }

    fn assist_level(&self) -> AssistLevel {
        self.assist
    }
}

impl ScrollSpeedModifier {
    fn remove_scroll_changes(&mut self, model: &mut BmsModel) {
        let initial_bpm = model.initial_bpm;
        let initial_scroll = model.timelines.first().map(|tl| tl.scroll).unwrap_or(1.0);

        // Check if there are any changes to remove
        let has_bpm_changes = model.bpm_changes.iter().any(|c| c.bpm != initial_bpm);
        let has_stops = !model.stop_events.is_empty();
        let has_scroll_changes = model.timelines.iter().any(|tl| tl.scroll != initial_scroll);

        if has_bpm_changes || has_stops || has_scroll_changes {
            self.assist = AssistLevel::LightAssist;
        }

        // Normalize all BPM changes to initial BPM
        for change in &mut model.bpm_changes {
            change.bpm = initial_bpm;
        }

        // Remove all stop events
        model.stop_events.clear();

        // Reset scroll to initial value
        for tl in &mut model.timelines {
            tl.scroll = initial_scroll;
        }
    }

    /// Add random scroll speed changes every N sections.
    ///
    /// Java: ScrollSpeedModifier.java L53-65
    fn add_scroll_changes(&mut self, model: &mut BmsModel) {
        let base = model.timelines.first().map(|tl| tl.scroll).unwrap_or(1.0);
        let mut current = base;
        let mut section_count = 0u32;
        let mut prev_measure: Option<u32> = None;
        let mut rng = rand::rng();

        for tl in &mut model.timelines {
            // Detect section (measure) boundary
            if prev_measure != Some(tl.measure) {
                section_count += 1;
                if section_count == self.section {
                    current = base * (1.0 + rng.random::<f64>() * self.rate * 2.0 - self.rate);
                    section_count = 0;
                }
            }
            tl.scroll = current;
            prev_measure = Some(tl.measure);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::{BpmChange, Note, PlayMode, StopEvent, TimeLine};

    fn make_model_with_bpm(bpm_changes: Vec<BpmChange>, stop_events: Vec<StopEvent>) -> BmsModel {
        BmsModel {
            mode: PlayMode::Beat7K,
            initial_bpm: 150.0,
            bpm_changes,
            stop_events,
            notes: vec![Note::normal(0, 1000, 1)],
            ..Default::default()
        }
    }

    /// Create a model with timelines spanning multiple measures.
    fn make_model_with_timelines(num_measures: u32) -> BmsModel {
        let mut timelines = Vec::new();
        for m in 0..num_measures {
            // 2 timelines per measure
            timelines.push(TimeLine {
                time_us: (m as i64) * 2_000_000,
                measure: m,
                position: 0.0,
                bpm: 150.0,
                scroll: 1.0,
            });
            timelines.push(TimeLine {
                time_us: (m as i64) * 2_000_000 + 1_000_000,
                measure: m,
                position: 0.5,
                bpm: 150.0,
                scroll: 1.0,
            });
        }
        BmsModel {
            mode: PlayMode::Beat7K,
            initial_bpm: 150.0,
            timelines,
            notes: vec![Note::normal(0, 1000, 1)],
            ..Default::default()
        }
    }

    #[test]
    fn test_remove_normalizes_bpm() {
        let bpm_changes = vec![
            BpmChange {
                time_us: 0,
                bpm: 150.0,
            },
            BpmChange {
                time_us: 1000000,
                bpm: 200.0,
            },
            BpmChange {
                time_us: 2000000,
                bpm: 100.0,
            },
        ];
        let mut model = make_model_with_bpm(bpm_changes, Vec::new());
        let mut modifier = ScrollSpeedModifier::new(ScrollSpeedMode::Remove);
        modifier.modify(&mut model);

        assert!(model.bpm_changes.iter().all(|c| c.bpm == 150.0));
        assert_eq!(modifier.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn test_remove_clears_stops() {
        let stop_events = vec![
            StopEvent {
                time_us: 500000,
                duration_ticks: 48,
                duration_us: 100000,
            },
            StopEvent {
                time_us: 1500000,
                duration_ticks: 96,
                duration_us: 200000,
            },
        ];
        let mut model = make_model_with_bpm(Vec::new(), stop_events);
        let mut modifier = ScrollSpeedModifier::new(ScrollSpeedMode::Remove);
        modifier.modify(&mut model);

        assert!(model.stop_events.is_empty());
        assert_eq!(modifier.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn test_remove_resets_scroll() {
        let mut model = make_model_with_timelines(4);
        // Set varying scroll values
        model.timelines[2].scroll = 2.0;
        model.timelines[3].scroll = 2.0;
        model.timelines[4].scroll = 0.5;
        model.timelines[5].scroll = 0.5;

        let mut modifier = ScrollSpeedModifier::new(ScrollSpeedMode::Remove);
        modifier.modify(&mut model);

        // All scroll values should be reset to initial (1.0)
        assert!(model.timelines.iter().all(|tl| tl.scroll == 1.0));
        assert_eq!(modifier.assist_level(), AssistLevel::LightAssist);
    }

    #[test]
    fn test_remove_no_changes_no_assist() {
        let bpm_changes = vec![BpmChange {
            time_us: 0,
            bpm: 150.0, // same as initial
        }];
        let mut model = make_model_with_bpm(bpm_changes, Vec::new());
        let mut modifier = ScrollSpeedModifier::new(ScrollSpeedMode::Remove);
        modifier.modify(&mut model);

        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn test_add_mode_changes_scroll() {
        // 8 measures, section interval = 4
        let mut model = make_model_with_timelines(8);
        let mut modifier = ScrollSpeedModifier::new(ScrollSpeedMode::Add)
            .with_section(4)
            .with_rate(0.5);
        modifier.modify(&mut model);

        // After 4 measure boundaries, scroll should change
        // First 4 measures: scroll = base (1.0) since section_count hasn't reached 4 yet
        // At measure 4 boundary: section_count reaches 4, scroll changes
        let scrolls: Vec<f64> = model.timelines.iter().map(|tl| tl.scroll).collect();

        // All timelines should have a scroll value set (not necessarily 1.0)
        assert!(scrolls.iter().all(|&s| s > 0.0));

        // Add mode does not change assist level (Java behavior)
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }

    #[test]
    fn test_add_mode_rate_zero_keeps_base() {
        let mut model = make_model_with_timelines(8);
        let mut modifier = ScrollSpeedModifier::new(ScrollSpeedMode::Add)
            .with_section(2)
            .with_rate(0.0);
        modifier.modify(&mut model);

        // With rate=0, scroll = base * (1.0 + random * 0 - 0) = base
        assert!(model.timelines.iter().all(|tl| tl.scroll == 1.0));
    }

    #[test]
    fn test_add_mode_empty_timelines() {
        let mut model = BmsModel {
            mode: PlayMode::Beat7K,
            initial_bpm: 150.0,
            ..Default::default()
        };
        let mut modifier = ScrollSpeedModifier::new(ScrollSpeedMode::Add);
        // Should not panic
        modifier.modify(&mut model);
        assert_eq!(modifier.assist_level(), AssistLevel::None);
    }
}

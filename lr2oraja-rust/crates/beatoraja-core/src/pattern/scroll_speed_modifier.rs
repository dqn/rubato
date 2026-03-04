use bms_model::bms_model::BMSModel;

use crate::pattern::pattern_modifier::{AssistLevel, PatternModifier, PatternModifierBase};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Remove,
    Add,
}

impl Mode {
    pub fn values() -> &'static [Mode] {
        &[Mode::Remove, Mode::Add]
    }

    pub fn from_index(index: i32) -> Mode {
        let values = Self::values();
        if index >= 0 && (index as usize) < values.len() {
            values[index as usize]
        } else {
            Mode::Remove
        }
    }
}

pub struct ScrollSpeedModifier {
    pub base: PatternModifierBase,
    mode: Mode,
    section: i32,
    rate: f64,
}

impl Default for ScrollSpeedModifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollSpeedModifier {
    pub fn new() -> Self {
        ScrollSpeedModifier {
            base: PatternModifierBase::new(),
            mode: Mode::Remove,
            section: 4,
            rate: 0.5,
        }
    }

    pub fn with_params(mode: i32, section: i32, scrollrate: f64) -> Self {
        ScrollSpeedModifier {
            base: PatternModifierBase::new(),
            mode: Mode::from_index(mode),
            section,
            rate: scrollrate,
        }
    }
}

impl PatternModifier for ScrollSpeedModifier {
    fn modify(&mut self, model: &mut BMSModel) {
        if self.mode == Mode::Remove {
            let mut assist = AssistLevel::None;
            let timelines = model.get_all_time_lines_mut();

            let start_bpm = timelines[0].get_bpm();
            let start_scroll = timelines[0].get_scroll();

            for tl in timelines.iter_mut() {
                if tl.get_bpm() != start_bpm
                    || tl.get_scroll() != start_scroll
                    || tl.get_stop() != 0
                {
                    assist = AssistLevel::LightAssist;
                }
                tl.set_section(start_bpm * tl.get_micro_time() as f64 / 240000000.0);
                tl.set_stop(0);
                tl.set_bpm(start_bpm);
                tl.set_scroll(start_scroll);
            }
            self.base.assist = assist;
        } else {
            let timelines = model.get_all_time_lines_mut();
            let base = timelines[0].get_scroll();
            let mut current = base;
            let mut sectioncount = 0;
            for tl in timelines.iter_mut() {
                if tl.get_section_line() {
                    sectioncount += 1;
                    if self.section == sectioncount {
                        current =
                            base * (1.0 + rand::random::<f64>() * self.rate * 2.0 - self.rate);
                        sectioncount = 0;
                    }
                }
                tl.set_scroll(current);
            }
        }
    }

    fn get_assist_level(&self) -> AssistLevel {
        self.base.assist
    }

    fn set_assist_level(&mut self, assist: AssistLevel) {
        self.base.assist = assist;
    }

    fn get_seed(&self) -> i64 {
        self.base.seed
    }

    fn set_seed(&mut self, seed: i64) {
        if seed >= 0 {
            self.base.seed = seed;
        }
    }

    fn get_player(&self) -> i32 {
        self.base.player
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::pattern_modifier::{PatternModifier, make_test_model};
    use bms_model::mode::Mode as BmsMode;
    use bms_model::time_line::TimeLine;

    // -- Mode enum --

    #[test]
    fn mode_values_has_2_elements() {
        assert_eq!(Mode::values().len(), 2);
    }

    #[test]
    fn mode_from_index_valid() {
        assert_eq!(Mode::from_index(0), Mode::Remove);
        assert_eq!(Mode::from_index(1), Mode::Add);
    }

    #[test]
    fn mode_from_index_negative_returns_remove() {
        assert_eq!(Mode::from_index(-1), Mode::Remove);
    }

    #[test]
    fn mode_from_index_out_of_range_returns_remove() {
        assert_eq!(Mode::from_index(2), Mode::Remove);
        assert_eq!(Mode::from_index(100), Mode::Remove);
    }

    // -- Construction --

    #[test]
    fn new_defaults() {
        let m = ScrollSpeedModifier::new();
        assert_eq!(m.mode, Mode::Remove);
        assert_eq!(m.section, 4);
        assert!((m.rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(m.get_assist_level(), AssistLevel::None);
    }

    #[test]
    fn default_trait() {
        let m = ScrollSpeedModifier::default();
        assert_eq!(m.mode, Mode::Remove);
    }

    #[test]
    fn with_params_remove() {
        let m = ScrollSpeedModifier::with_params(0, 8, 0.3);
        assert_eq!(m.mode, Mode::Remove);
        assert_eq!(m.section, 8);
        assert!((m.rate - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn with_params_add() {
        let m = ScrollSpeedModifier::with_params(1, 2, 0.7);
        assert_eq!(m.mode, Mode::Add);
        assert_eq!(m.section, 2);
        assert!((m.rate - 0.7).abs() < f64::EPSILON);
    }

    // -- PatternModifier trait methods --

    #[test]
    fn set_seed_positive() {
        let mut m = ScrollSpeedModifier::new();
        m.set_seed(42);
        assert_eq!(m.get_seed(), 42);
    }

    #[test]
    fn set_seed_negative_ignored() {
        let mut m = ScrollSpeedModifier::new();
        let original = m.get_seed();
        m.set_seed(-1);
        assert_eq!(m.get_seed(), original);
    }

    #[test]
    fn set_seed_zero() {
        let mut m = ScrollSpeedModifier::new();
        m.set_seed(0);
        assert_eq!(m.get_seed(), 0);
    }

    #[test]
    fn set_assist_level() {
        let mut m = ScrollSpeedModifier::new();
        m.set_assist_level(AssistLevel::Assist);
        assert_eq!(m.get_assist_level(), AssistLevel::Assist);
    }

    #[test]
    fn get_player_default() {
        let m = ScrollSpeedModifier::new();
        assert_eq!(m.get_player(), 0);
    }

    // -- Remove mode: all timelines uniform -> AssistLevel::None --

    #[test]
    fn remove_mode_uniform_timelines_keeps_none_assist() {
        // All timelines have the same BPM, scroll, and stop=0
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_bpm(120.0);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = ScrollSpeedModifier::new(); // Remove mode
        modifier.modify(&mut model);

        assert_eq!(modifier.get_assist_level(), AssistLevel::None);
    }

    // -- Remove mode: different BPM -> LightAssist --

    #[test]
    fn remove_mode_different_bpm_sets_light_assist() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_bpm(180.0); // different BPM

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        assert_eq!(modifier.get_assist_level(), AssistLevel::LightAssist);

        // After modification, all timelines should have start_bpm
        let tls = model.get_all_time_lines();
        assert!((tls[0].get_bpm() - 120.0).abs() < f64::EPSILON);
        assert!((tls[1].get_bpm() - 120.0).abs() < f64::EPSILON);
    }

    // -- Remove mode: different scroll -> LightAssist --

    #[test]
    fn remove_mode_different_scroll_sets_light_assist() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);
        tl0.set_scroll(1.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_bpm(120.0);
        tl1.set_scroll(2.0); // different scroll

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        assert_eq!(modifier.get_assist_level(), AssistLevel::LightAssist);

        let tls = model.get_all_time_lines();
        assert!((tls[0].get_scroll() - 1.0).abs() < f64::EPSILON);
        assert!((tls[1].get_scroll() - 1.0).abs() < f64::EPSILON);
    }

    // -- Remove mode: non-zero stop -> LightAssist --

    #[test]
    fn remove_mode_nonzero_stop_sets_light_assist() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_bpm(120.0);
        tl1.set_stop(5_000_000); // non-zero stop (get_stop() = 5000)

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        assert_eq!(modifier.get_assist_level(), AssistLevel::LightAssist);

        // Stop should be zeroed after modification
        let tls = model.get_all_time_lines();
        assert_eq!(tls[1].get_stop(), 0);
    }

    // -- Remove mode: BPM is normalized to timeline[0] --

    #[test]
    fn remove_mode_normalizes_bpm_to_first_timeline() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(150.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_bpm(200.0);

        let mut tl2 = TimeLine::new(2.0, 2_000_000, 8);
        tl2.set_bpm(100.0);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        for tl in tls {
            assert!((tl.get_bpm() - 150.0).abs() < f64::EPSILON);
        }
    }

    // -- Remove mode: scroll is normalized to timeline[0] --

    #[test]
    fn remove_mode_normalizes_scroll_to_first_timeline() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);
        tl0.set_scroll(1.5);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_bpm(120.0);
        tl1.set_scroll(0.5);

        let mut tl2 = TimeLine::new(2.0, 2_000_000, 8);
        tl2.set_bpm(120.0);
        tl2.set_scroll(3.0);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        for tl in tls {
            assert!((tl.get_scroll() - 1.5).abs() < f64::EPSILON);
        }
    }

    // -- Remove mode: stops zeroed on all timelines --

    #[test]
    fn remove_mode_zeroes_all_stops() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);
        tl0.set_stop(2_000_000);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_bpm(120.0);
        tl1.set_stop(3_000_000);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        assert_eq!(tls[0].get_stop(), 0);
        assert_eq!(tls[1].get_stop(), 0);
    }

    // -- Remove mode: section recalculated as start_bpm * micro_time / 240000000 --

    #[test]
    fn remove_mode_recalculates_section() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);

        // micro_time = 2_000_000 -> section = 120.0 * 2_000_000 / 240_000_000 = 1.0
        let mut tl1 = TimeLine::new(5.0, 2_000_000, 8);
        tl1.set_bpm(120.0);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // tl[0]: 120.0 * 0 / 240_000_000 = 0.0
        assert!((tls[0].get_section() - 0.0).abs() < f64::EPSILON);
        // tl[1]: 120.0 * 2_000_000 / 240_000_000 = 1.0
        assert!((tls[1].get_section() - 1.0).abs() < f64::EPSILON);
    }

    // -- Add mode: section count advances and scroll changes --

    #[test]
    fn add_mode_changes_scroll_at_section_boundary() {
        // Create timelines with section_line markers. section=2, so after 2 section lines
        // the scroll should be re-randomized.
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);
        tl0.set_scroll(1.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_bpm(120.0);
        tl1.set_section_line(true);

        let mut tl2 = TimeLine::new(2.0, 2_000_000, 8);
        tl2.set_bpm(120.0);
        tl2.set_section_line(true); // sectioncount reaches 2 -> reset

        let mut tl3 = TimeLine::new(3.0, 3_000_000, 8);
        tl3.set_bpm(120.0);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1, tl2, tl3]);

        let mut modifier = ScrollSpeedModifier::with_params(1, 2, 0.5); // Add, section=2
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // tl[0] has no section_line, so scroll starts at base (1.0)
        assert!((tls[0].get_scroll() - 1.0).abs() < f64::EPSILON);
        // tl[1] is the first section_line, sectioncount=1 (not yet = section=2), scroll stays 1.0
        assert!((tls[1].get_scroll() - 1.0).abs() < f64::EPSILON);
        // tl[2] is the second section_line, sectioncount=2 == section=2, scroll randomized
        // The new scroll is base * (1.0 + rand * rate * 2 - rate) where base=1.0, rate=0.5
        // So scroll is in range [1.0 * (1.0 - 0.5), 1.0 * (1.0 + 0.5)] = [0.5, 1.5]
        let scroll2 = tls[2].get_scroll();
        assert!(
            scroll2 >= 0.5 && scroll2 <= 1.5,
            "scroll at tl[2] should be in [0.5, 1.5], got {}",
            scroll2
        );
        // tl[3] carries the same scroll as tl[2] (no new section_line)
        assert!((tls[3].get_scroll() - scroll2).abs() < f64::EPSILON);
    }

    // -- Add mode: no section lines -> all timelines keep base scroll --

    #[test]
    fn add_mode_no_section_lines_keeps_base_scroll() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);
        tl0.set_scroll(2.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_bpm(120.0);
        // section_line defaults to false

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = ScrollSpeedModifier::with_params(1, 4, 0.5); // Add
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        assert!((tls[0].get_scroll() - 2.0).abs() < f64::EPSILON);
        assert!((tls[1].get_scroll() - 2.0).abs() < f64::EPSILON);
    }

    // -- Add mode: section count resets after reaching threshold --

    #[test]
    fn add_mode_section_count_resets() {
        // section=1: every section_line triggers a scroll change
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);
        tl0.set_scroll(1.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_section_line(true); // sectioncount=1 == section=1, randomize

        let mut tl2 = TimeLine::new(2.0, 2_000_000, 8);
        tl2.set_section_line(true); // sectioncount=1 again (reset), randomize again

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1, tl2]);

        let mut modifier = ScrollSpeedModifier::with_params(1, 1, 0.5); // Add, section=1
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // tl[0] keeps the base scroll
        assert!((tls[0].get_scroll() - 1.0).abs() < f64::EPSILON);
        // tl[1] and tl[2] should each have randomized scroll in [0.5, 1.5]
        let s1 = tls[1].get_scroll();
        let s2 = tls[2].get_scroll();
        assert!(
            s1 >= 0.5 && s1 <= 1.5,
            "tl[1] scroll should be in [0.5, 1.5], got {}",
            s1
        );
        assert!(
            s2 >= 0.5 && s2 <= 1.5,
            "tl[2] scroll should be in [0.5, 1.5], got {}",
            s2
        );
    }

    // -- Add mode: rate=0 -> scroll stays at base --

    #[test]
    fn add_mode_rate_zero_keeps_base_scroll() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_scroll(1.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_section_line(true);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        // rate=0.0 means: base * (1.0 + rand * 0.0 - 0.0) = base * 1.0 = base
        let mut modifier = ScrollSpeedModifier::with_params(1, 1, 0.0);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        assert!((tls[0].get_scroll() - 1.0).abs() < f64::EPSILON);
        assert!((tls[1].get_scroll() - 1.0).abs() < f64::EPSILON);
    }

    // -- Edge case: single timeline, Remove mode --

    #[test]
    fn remove_mode_single_timeline() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(130.0);
        tl0.set_scroll(1.0);
        tl0.set_stop(0);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0]);

        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        // Single timeline with same values as itself -> no assist needed
        assert_eq!(modifier.get_assist_level(), AssistLevel::None);

        let tls = model.get_all_time_lines();
        assert!((tls[0].get_bpm() - 130.0).abs() < f64::EPSILON);
        assert!((tls[0].get_scroll() - 1.0).abs() < f64::EPSILON);
        assert_eq!(tls[0].get_stop(), 0);
    }

    // -- Edge case: single timeline, Add mode --

    #[test]
    fn add_mode_single_timeline() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_scroll(1.0);
        // No section_line, so sectioncount never advances

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0]);

        let mut modifier = ScrollSpeedModifier::with_params(1, 1, 0.5);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // Single timeline with no section_line -> scroll stays at base
        assert!((tls[0].get_scroll() - 1.0).abs() < f64::EPSILON);
    }

    // -- Edge case: single timeline with section_line, Add mode --

    #[test]
    fn add_mode_single_timeline_with_section_line() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_scroll(1.0);
        tl0.set_section_line(true);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0]);

        let mut modifier = ScrollSpeedModifier::with_params(1, 1, 0.5); // section=1
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // section_line is true, sectioncount=1 == section=1, so scroll is randomized
        let s = tls[0].get_scroll();
        assert!(
            s >= 0.5 && s <= 1.5,
            "scroll should be in [0.5, 1.5], got {}",
            s
        );
    }

    // -- Remove mode: first timeline has stop -> stop still gets zeroed, assist is LightAssist --

    #[test]
    fn remove_mode_first_timeline_with_stop() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_bpm(120.0);
        tl0.set_stop(1_000_000); // get_stop() = 1000

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0]);

        let mut modifier = ScrollSpeedModifier::new();
        modifier.modify(&mut model);

        // Even the first timeline triggers LightAssist because get_stop() != 0
        assert_eq!(modifier.get_assist_level(), AssistLevel::LightAssist);
        assert_eq!(model.get_all_time_lines()[0].get_stop(), 0);
    }

    // -- Add mode: base scroll propagates from first timeline --

    #[test]
    fn add_mode_uses_first_timeline_scroll_as_base() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_scroll(3.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_scroll(1.0); // different initial scroll, but base comes from tl[0]

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = ScrollSpeedModifier::with_params(1, 4, 0.5);
        modifier.modify(&mut model);

        let tls = model.get_all_time_lines();
        // No section lines -> all get base scroll (3.0)
        assert!((tls[0].get_scroll() - 3.0).abs() < f64::EPSILON);
        assert!((tls[1].get_scroll() - 3.0).abs() < f64::EPSILON);
    }

    // -- Add mode does not set assist level --

    #[test]
    fn add_mode_does_not_change_assist_level() {
        let mut tl0 = TimeLine::new(0.0, 0, 8);
        tl0.set_scroll(1.0);

        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_section_line(true);

        let mut model = make_test_model(&BmsMode::BEAT_7K, vec![tl0, tl1]);

        let mut modifier = ScrollSpeedModifier::with_params(1, 1, 0.5);
        modifier.modify(&mut model);

        // Add mode never modifies self.base.assist
        assert_eq!(modifier.get_assist_level(), AssistLevel::None);
    }
}

use crate::clear_type::ClearType;
use crate::gauge_property::{GaugeElementProperty, GaugeProperty};
use bms_model::bms_model::BMSModel;

pub const ASSISTEASY: i32 = 0;
pub const EASY: i32 = 1;
pub const NORMAL: i32 = 2;
pub const HARD: i32 = 3;
pub const EXHARD: i32 = 4;
pub const HAZARD: i32 = 5;
pub const CLASS: i32 = 6;
pub const EXCLASS: i32 = 7;
pub const EXHARDCLASS: i32 = 8;

/// Gauge modifier type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GaugeModifier {
    /// Use TOTAL for recovery
    Total,
    /// Limit increment by TOTAL
    LimitIncrement,
    /// Modify damage by TOTAL and total notes
    ModifyDamage,
}

impl GaugeModifier {
    pub fn modify(&self, f: f32, model: &BMSModel) -> f32 {
        match self {
            GaugeModifier::Total => {
                if f > 0.0 {
                    f * model.total() as f32 / model.total_notes() as f32
                } else {
                    f
                }
            }
            GaugeModifier::LimitIncrement => {
                let pg = (0.15f32)
                    .min(((2.0 * model.total() - 320.0) / model.total_notes() as f64) as f32)
                    .max(0.0);
                if f > 0.0 { f * pg / 0.15 } else { f }
            }
            GaugeModifier::ModifyDamage => {
                if f < 0.0 {
                    let fix2: f32;

                    // TOTAL correction (<240)
                    let fix1: f32 = (10.0
                        / (10.0f64).min((model.total() / 16.0).floor() - 5.0).max(1.0))
                        as f32;

                    // Notes count correction (<1000)
                    let total_notes = model.total_notes();
                    if total_notes <= 20 {
                        fix2 = 10.0;
                    } else if total_notes < 30 {
                        fix2 = 8.0 + 0.2 * (30 - total_notes) as f32;
                    } else if total_notes < 60 {
                        fix2 = 5.0 + 0.2 * (60 - total_notes) as f32 / 3.0;
                    } else if total_notes < 125 {
                        fix2 = 4.0 + (125 - total_notes) as f32 / 65.0;
                    } else if total_notes < 250 {
                        fix2 = 3.0 + 0.008 * (250 - total_notes) as f32;
                    } else if total_notes < 500 {
                        fix2 = 2.0 + 0.004 * (500 - total_notes) as f32;
                    } else if total_notes < 1000 {
                        fix2 = 1.0 + 0.002 * (1000 - total_notes) as f32;
                    } else {
                        fix2 = 1.0;
                    }

                    f * fix1.max(fix2)
                } else {
                    f
                }
            }
        }
    }
}

/// Individual gauge state
#[derive(Clone, Debug)]
pub struct Gauge {
    /// Current gauge value
    value: f32,
    /// Gauge element property
    element: GaugeElementProperty,
    /// Judge-specific gauge changes
    gauge: Vec<f32>,
    /// Clear type for this gauge
    pub cleartype: ClearType,
}

impl Gauge {
    pub fn new(model: &BMSModel, element: GaugeElementProperty, cleartype: ClearType) -> Self {
        let value = element.init;
        let mut gauge = element.value.clone();
        if let Some(ref modifier) = element.modifier {
            for g in &mut gauge {
                *g = modifier.modify(*g, model);
            }
        }
        Gauge {
            value,
            element,
            gauge,
            cleartype,
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&mut self, value: f32) {
        if self.value > 0.0 {
            self.value = value.clamp(self.element.min, self.element.max);
            if self.value < self.element.death {
                self.value = 0.0;
            }
        }
    }

    pub fn update(&mut self, judge: i32, rate: f32) {
        let Some(judge_index) = usize::try_from(judge).ok() else {
            return;
        };
        let Some(mut inc) = self
            .gauge
            .get(judge_index)
            .copied()
            .map(|value| value * rate)
        else {
            return;
        };
        if inc < 0.0 {
            for gut in &self.element.guts {
                if self.value < gut[0] {
                    inc *= gut[1];
                    break;
                }
            }
        }
        let new_value = self.value + inc;
        self.set_value(new_value);
    }

    pub fn property(&self) -> &GaugeElementProperty {
        &self.element
    }

    pub fn is_qualified(&self) -> bool {
        self.value > 0.0 && self.value >= self.element.border
    }

    pub fn is_max(&self) -> bool {
        self.value == self.element.max
    }
}

/// Groove gauge
#[derive(Clone)]
pub struct GrooveGauge {
    typeorg: i32,
    gauge_type: i32,
    gauges: Vec<Gauge>,
}

impl GrooveGauge {
    pub const ASSISTEASY: i32 = ASSISTEASY;
    pub const EASY: i32 = EASY;
    pub const NORMAL: i32 = NORMAL;
    pub const HARD: i32 = HARD;
    pub const EXHARD: i32 = EXHARD;
    pub const HAZARD: i32 = HAZARD;
    pub const GRADE_NORMAL: i32 = CLASS;
    pub const GRADE_HARD: i32 = EXCLASS;
    pub const GRADE_EXHARD: i32 = EXHARDCLASS;

    pub fn new(model: &BMSModel, gauge_type: i32, property: &GaugeProperty) -> Self {
        let values = property.element_values();
        let mut gauges = Vec::with_capacity(values.len());
        for (i, element) in values.into_iter().enumerate() {
            gauges.push(Gauge::new(
                model,
                element,
                ClearType::clear_type_by_gauge(i as i32).unwrap_or(ClearType::Failed),
            ));
        }
        GrooveGauge {
            typeorg: gauge_type,
            gauge_type,
            gauges,
        }
    }

    pub fn update(&mut self, judge: i32) {
        self.update_with_rate(judge, 1.0);
    }

    pub fn update_with_rate(&mut self, judge: i32, rate: f32) {
        for gauge in &mut self.gauges {
            gauge.update(judge, rate);
        }
    }

    pub fn add_value(&mut self, value: f32) {
        for gauge in &mut self.gauges {
            let new_val = gauge.value() + value;
            gauge.set_value(new_val);
        }
    }

    fn gauge_at(&self, gauge_type: i32) -> Option<&Gauge> {
        usize::try_from(gauge_type)
            .ok()
            .and_then(|i| self.gauges.get(i))
    }

    fn gauge_at_mut(&mut self, gauge_type: i32) -> Option<&mut Gauge> {
        usize::try_from(gauge_type)
            .ok()
            .and_then(|i| self.gauges.get_mut(i))
    }

    pub fn value(&self) -> f32 {
        self.gauge_at(self.gauge_type)
            .map(|g| g.value())
            .unwrap_or(0.0)
    }

    pub fn value_by_type(&self, gauge_type: i32) -> f32 {
        self.gauge_at(gauge_type).map(|g| g.value()).unwrap_or(0.0)
    }

    pub fn set_value(&mut self, value: f32) {
        for gauge in &mut self.gauges {
            gauge.set_value(value);
        }
    }

    pub fn set_value_by_type(&mut self, gauge_type: i32, value: f32) {
        if let Some(gauge) = self.gauge_at_mut(gauge_type) {
            gauge.set_value(value);
        }
    }

    pub fn is_qualified(&self) -> bool {
        self.gauge_at(self.gauge_type)
            .map(|g| g.is_qualified())
            .unwrap_or(false)
    }

    pub fn is_qualified_by_type(&self, gauge_type: i32) -> bool {
        if (gauge_type as usize) < self.gauges.len() {
            self.gauges[gauge_type as usize].is_qualified()
        } else {
            false
        }
    }

    pub fn gauge_type(&self) -> i32 {
        self.gauge_type
    }

    pub fn set_type(&mut self, gauge_type: i32) {
        if let Ok(i) = usize::try_from(gauge_type)
            && i < self.gauges.len()
        {
            self.gauge_type = gauge_type;
        }
    }

    pub fn is_type_changed(&self) -> bool {
        self.typeorg != self.gauge_type
    }

    pub fn is_course_gauge(&self) -> bool {
        self.gauge_type >= CLASS && self.gauge_type <= EXHARDCLASS
    }

    pub fn gauge_type_length(&self) -> usize {
        self.gauges.len()
    }

    pub fn clear_type(&self) -> ClearType {
        self.gauge_at(self.gauge_type)
            .map(|g| g.cleartype)
            .unwrap_or(ClearType::Failed)
    }

    pub fn gauge(&self) -> &Gauge {
        self.gauge_at(self.gauge_type).unwrap_or(&self.gauges[0])
    }

    pub fn gauge_by_type(&self, gauge_type: i32) -> &Gauge {
        self.gauge_at(gauge_type).unwrap_or(&self.gauges[0])
    }

    pub fn gauge_by_type_mut(&mut self, gauge_type: i32) -> &mut Gauge {
        let idx = usize::try_from(gauge_type)
            .ok()
            .filter(|&i| i < self.gauges.len())
            .unwrap_or(0);
        &mut self.gauges[idx]
    }

    pub fn create_with_id(model: &BMSModel, id: i32, gauge: &GaugeProperty) -> Self {
        GrooveGauge::new(model, id, gauge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model() -> BMSModel {
        let mut model = BMSModel::new();
        model.set_total(300.0);
        model
    }

    // -- GaugeModifier tests --

    #[test]
    fn test_gauge_modifier_total_positive() {
        let model = make_model();
        let result = GaugeModifier::Total.modify(1.0, &model);
        // f * total / total_notes; total_notes = 0 for empty model
        // 1.0 * 300.0 / 0 = inf (or NaN), but let us check with notes > 0
        // With 0 notes, this would be inf; that's the Java behavior too
        assert!(result.is_infinite() || result.is_nan() || result > 0.0);
    }

    #[test]
    fn test_gauge_modifier_total_negative_unchanged() {
        let model = make_model();
        let result = GaugeModifier::Total.modify(-5.0, &model);
        assert_eq!(result, -5.0);
    }

    #[test]
    fn test_gauge_modifier_limit_increment_negative_unchanged() {
        let model = make_model();
        let result = GaugeModifier::LimitIncrement.modify(-3.0, &model);
        assert_eq!(result, -3.0);
    }

    #[test]
    fn test_gauge_modifier_modify_damage_positive_unchanged() {
        let model = make_model();
        let result = GaugeModifier::ModifyDamage.modify(1.0, &model);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_gauge_modifier_equality() {
        assert_eq!(GaugeModifier::Total, GaugeModifier::Total);
        assert_ne!(GaugeModifier::Total, GaugeModifier::LimitIncrement);
        assert_ne!(GaugeModifier::LimitIncrement, GaugeModifier::ModifyDamage);
    }

    // -- Gauge tests --

    #[test]
    fn test_gauge_initial_value() {
        let model = make_model();
        let element = GaugeElementProperty {
            modifier: None,
            value: vec![0.15, 0.12, 0.03, -5.0, -10.0, -5.0],
            min: 0.0,
            max: 100.0,
            init: 100.0,
            border: 0.0,
            death: 0.0,
            guts: vec![],
        };
        let gauge = Gauge::new(&model, element, ClearType::Hard);
        assert_eq!(gauge.value(), 100.0);
    }

    #[test]
    fn test_gauge_set_value_clamped() {
        let model = make_model();
        let element = GaugeElementProperty {
            modifier: None,
            value: vec![0.15, 0.12, 0.03, -5.0, -10.0, -5.0],
            min: 0.0,
            max: 100.0,
            init: 50.0,
            border: 0.0,
            death: 0.0,
            guts: vec![],
        };
        let mut gauge = Gauge::new(&model, element, ClearType::Hard);
        assert_eq!(gauge.value(), 50.0);

        // Set above max
        gauge.set_value(150.0);
        assert_eq!(gauge.value(), 100.0);

        // Set to min
        gauge.set_value(0.0);
        assert_eq!(gauge.value(), 0.0);
    }

    #[test]
    fn test_gauge_death_border() {
        let model = make_model();
        let element = GaugeElementProperty {
            modifier: None,
            value: vec![0.1, 0.1, 0.05, -6.0, -10.0, -2.0],
            min: 0.0,
            max: 100.0,
            init: 100.0,
            border: 0.0,
            death: 2.0,
            guts: vec![],
        };
        let mut gauge = Gauge::new(&model, element, ClearType::Hard);
        // Setting below death border kills the gauge
        gauge.set_value(1.5);
        assert_eq!(gauge.value(), 0.0);
    }

    #[test]
    fn test_gauge_is_qualified() {
        let model = make_model();
        let element = GaugeElementProperty {
            modifier: None,
            value: vec![1.0, 1.0, 0.5, -3.0, -6.0, -2.0],
            min: 2.0,
            max: 100.0,
            init: 20.0,
            border: 80.0,
            death: 0.0,
            guts: vec![],
        };
        let mut gauge = Gauge::new(&model, element, ClearType::Normal);
        // 20 < 80, not qualified
        assert!(!gauge.is_qualified());

        gauge.set_value(80.0);
        assert!(gauge.is_qualified());

        gauge.set_value(90.0);
        assert!(gauge.is_qualified());
    }

    #[test]
    fn test_gauge_is_max() {
        let model = make_model();
        let element = GaugeElementProperty {
            modifier: None,
            value: vec![1.0, 1.0, 0.5, -3.0, -6.0, -2.0],
            min: 0.0,
            max: 100.0,
            init: 100.0,
            border: 80.0,
            death: 0.0,
            guts: vec![],
        };
        let gauge = Gauge::new(&model, element, ClearType::Normal);
        assert!(gauge.is_max());
    }

    #[test]
    fn test_gauge_update() {
        let model = make_model();
        let element = GaugeElementProperty {
            modifier: None,
            value: vec![0.15, 0.12, 0.03, -5.0, -10.0, -5.0],
            min: 0.0,
            max: 100.0,
            init: 50.0,
            border: 0.0,
            death: 0.0,
            guts: vec![],
        };
        let mut gauge = Gauge::new(&model, element, ClearType::Hard);
        assert_eq!(gauge.value(), 50.0);

        // Update with PG (judge=0), rate=1.0 => +0.15
        gauge.update(0, 1.0);
        let expected = (50.0 + 0.15_f32).clamp(0.0, 100.0);
        assert!((gauge.value() - expected).abs() < 1e-6);
    }

    // -- GrooveGauge tests --

    #[test]
    fn test_groove_gauge_construction() {
        let model = make_model();
        let gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);
        assert_eq!(gg.gauge_type(), NORMAL);
        assert!(!gg.is_type_changed());
        assert_eq!(gg.gauge_type_length(), 9);
    }

    #[test]
    fn test_groove_gauge_type_change() {
        let model = make_model();
        let mut gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);
        assert!(!gg.is_type_changed());

        gg.set_type(HARD);
        assert_eq!(gg.gauge_type(), HARD);
        assert!(gg.is_type_changed());
    }

    #[test]
    fn test_groove_gauge_is_course_gauge() {
        let model = make_model();
        let gg_normal = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);
        assert!(!gg_normal.is_course_gauge());

        let gg_class = GrooveGauge::new(&model, CLASS, &GaugeProperty::SevenKeys);
        assert!(gg_class.is_course_gauge());

        let gg_exclass = GrooveGauge::new(&model, EXCLASS, &GaugeProperty::SevenKeys);
        assert!(gg_exclass.is_course_gauge());

        let gg_exhardclass = GrooveGauge::new(&model, EXHARDCLASS, &GaugeProperty::SevenKeys);
        assert!(gg_exhardclass.is_course_gauge());
    }

    #[test]
    fn test_groove_gauge_get_value() {
        let model = make_model();
        let gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);
        // NORMAL init = 20.0
        assert_eq!(gg.value(), 20.0);
    }

    #[test]
    fn test_groove_gauge_set_value() {
        let model = make_model();
        let mut gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);
        gg.set_value(50.0);
        // All gauges should be clamped to their respective min/max
        // NORMAL gauge value should be set to 50
        assert_eq!(gg.value(), 50.0);
    }

    #[test]
    fn test_groove_gauge_get_clear_type() {
        let model = make_model();
        let gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);
        let ct = gg.clear_type();
        // Gauge type 2 (NORMAL) maps to ClearType::Normal via get_clear_type_by_gauge
        assert_eq!(ct, ClearType::Normal);
    }

    #[test]
    fn test_groove_gauge_constants() {
        assert_eq!(GrooveGauge::ASSISTEASY, 0);
        assert_eq!(GrooveGauge::EASY, 1);
        assert_eq!(GrooveGauge::NORMAL, 2);
        assert_eq!(GrooveGauge::HARD, 3);
        assert_eq!(GrooveGauge::EXHARD, 4);
        assert_eq!(GrooveGauge::HAZARD, 5);
        assert_eq!(GrooveGauge::GRADE_NORMAL, 6);
        assert_eq!(GrooveGauge::GRADE_HARD, 7);
        assert_eq!(GrooveGauge::GRADE_EXHARD, 8);
    }

    #[test]
    fn test_groove_gauge_create_with_id() {
        let model = make_model();
        let gg = GrooveGauge::create_with_id(&model, EASY, &GaugeProperty::FiveKeys);
        assert_eq!(gg.gauge_type(), EASY);
    }

    #[test]
    fn test_groove_gauge_oob_negative_gauge_type() {
        let model = make_model();
        let mut gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);

        // set_type with negative should be no-op
        gg.set_type(-1);
        assert_eq!(gg.gauge_type(), NORMAL);

        // by_type accessors with negative should not panic
        assert_eq!(gg.value_by_type(-1), 0.0);
        assert!(!gg.is_qualified_by_type(-1));
        gg.set_value_by_type(-1, 50.0); // no-op

        // get_gauge_by_type with negative falls back to gauges[0]
        let gauge = gg.gauge_by_type(-1);
        assert_eq!(gauge.cleartype, ClearType::clear_type_by_gauge(0).unwrap());
    }

    #[test]
    fn test_groove_gauge_oob_large_gauge_type() {
        let model = make_model();
        let mut gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);

        // set_type with too-large value should be no-op
        gg.set_type(100);
        assert_eq!(gg.gauge_type(), NORMAL);

        // by_type accessors with too-large value should not panic
        assert_eq!(gg.value_by_type(100), 0.0);
        assert!(!gg.is_qualified_by_type(100));
        gg.set_value_by_type(100, 50.0); // no-op

        // get_clear_type / gauge fallbacks
        assert_eq!(gg.clear_type(), ClearType::Normal);
        let gauge = gg.gauge_by_type(100);
        assert_eq!(gauge.cleartype, ClearType::clear_type_by_gauge(0).unwrap());
    }

    #[test]
    fn test_groove_gauge_oob_get_gauge_by_type_mut() {
        let model = make_model();
        let mut gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);

        // Should not panic, falls back to gauges[0]
        let gauge = gg.gauge_by_type_mut(-1);
        gauge.set_value(42.0);
        assert_eq!(gg.value_by_type(0), 42.0);

        let gauge = gg.gauge_by_type_mut(100);
        gauge.set_value(55.0);
        assert_eq!(gg.value_by_type(0), 55.0);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::gauge_property::GaugeElementProperty;
    use bms_model::bms_model::BMSModel;
    use proptest::prelude::*;

    fn make_model() -> BMSModel {
        let mut model = BMSModel::new();
        model.set_total(300.0);
        model
    }

    proptest! {
        /// For any positive f, ModifyDamage returns f unchanged.
        #[test]
        fn modify_damage_positive_unchanged(f in 0.001f32..1e6) {
            let model = make_model();
            let result = GaugeModifier::ModifyDamage.modify(f, &model);
            prop_assert_eq!(result, f);
        }

        /// For any negative f, ModifyDamage amplifies damage (result <= f, since both are negative).
        #[test]
        fn modify_damage_negative_amplified(f in -1e6f32..-0.001) {
            let model = make_model();
            let result = GaugeModifier::ModifyDamage.modify(f, &model);
            prop_assert!(result <= f, "expected result ({}) <= f ({})", result, f);
        }

        /// For any negative f, Total modifier returns f unchanged.
        #[test]
        fn total_negative_unchanged(f in -1e6f32..-0.001) {
            let model = make_model();
            let result = GaugeModifier::Total.modify(f, &model);
            prop_assert_eq!(result, f);
        }

        /// For any negative f, LimitIncrement returns f unchanged.
        #[test]
        fn limit_increment_negative_unchanged(f in -1e6f32..-0.001) {
            let model = make_model();
            let result = GaugeModifier::LimitIncrement.modify(f, &model);
            prop_assert_eq!(result, f);
        }

        /// After set_value(v) for any v, the gauge value is either 0 (dead) or in [min, max].
        #[test]
        fn gauge_set_value_always_clamped(v in -200.0f32..200.0) {
            let model = make_model();
            let element = GaugeElementProperty {
                modifier: None,
                value: vec![0.15, 0.12, 0.03, -5.0, -10.0, -5.0],
                min: 2.0,
                max: 100.0,
                init: 50.0,
                border: 80.0,
                death: 2.0,
                guts: vec![],
            };
            let mut gauge = Gauge::new(&model, element, ClearType::Hard);
            // Gauge starts at init=50.0 which is > 0, so set_value will execute
            gauge.set_value(v);
            let result = gauge.value();
            // set_value clamps to [min, max], then sets to 0 if below death
            // So result is either 0.0 (dead) or in [min, max]
            prop_assert!(
                result == 0.0 || (2.0..=100.0).contains(&result),
                "expected 0.0 or [2.0, 100.0], got {}",
                result
            );
        }
    }
}

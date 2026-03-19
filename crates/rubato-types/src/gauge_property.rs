use crate::groove_gauge::GaugeModifier;

/// Gauge specification
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GaugeProperty {
    FiveKeys,
    SevenKeys,
    Pms,
    Keyboard,
    Lr2,
}

impl GaugeProperty {
    pub fn values() -> &'static [GaugeProperty] {
        &[
            GaugeProperty::FiveKeys,
            GaugeProperty::SevenKeys,
            GaugeProperty::Pms,
            GaugeProperty::Keyboard,
            GaugeProperty::Lr2,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            GaugeProperty::FiveKeys => "FIVEKEYS",
            GaugeProperty::SevenKeys => "SEVENKEYS",
            GaugeProperty::Pms => "PMS",
            GaugeProperty::Keyboard => "KEYBOARD",
            GaugeProperty::Lr2 => "LR2",
        }
    }

    pub fn element_values(&self) -> Vec<GaugeElementProperty> {
        match self {
            GaugeProperty::FiveKeys => vec![
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 50.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -1.5, -3.0, -0.5],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 75.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -1.5, -4.5, -1.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 75.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -3.0, -6.0, -2.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::LimitIncrement),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.0, 0.0, 0.0, -5.0, -10.0, -5.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::ModifyDamage),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.0, 0.0, 0.0, -10.0, -20.0, -10.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.0, 0.0, 0.0, -100.0, -100.0, -100.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.01, 0.01, 0.0, -0.5, -1.0, -0.5],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.01, 0.01, 0.0, -1.0, -2.0, -1.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.01, 0.01, 0.0, -2.5, -5.0, -2.5],
                    guts: &[],
                }),
            ],
            GaugeProperty::SevenKeys => vec![
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 60.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -1.5, -3.0, -0.5],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 80.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -1.5, -4.5, -1.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 80.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -3.0, -6.0, -2.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::LimitIncrement),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.12, 0.03, -5.0, -10.0, -5.0],
                    guts: &[
                        &[10.0, 0.4],
                        &[20.0, 0.5],
                        &[30.0, 0.6],
                        &[40.0, 0.7],
                        &[50.0, 0.8],
                    ],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::LimitIncrement),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.06, 0.0, -8.0, -16.0, -8.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.06, 0.0, -100.0, -100.0, -10.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.12, 0.06, -1.5, -3.0, -1.5],
                    guts: &[
                        &[5.0, 0.4],
                        &[10.0, 0.5],
                        &[15.0, 0.6],
                        &[20.0, 0.7],
                        &[25.0, 0.8],
                    ],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.12, 0.03, -3.0, -6.0, -3.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.06, 0.0, -5.0, -10.0, -5.0],
                    guts: &[],
                }),
            ],
            GaugeProperty::Pms => vec![
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 120.0,
                    init: 30.0,
                    border: 65.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -1.0, -2.0, -2.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 120.0,
                    init: 30.0,
                    border: 85.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -1.0, -3.0, -3.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 120.0,
                    init: 30.0,
                    border: 85.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -2.0, -6.0, -6.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::LimitIncrement),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.12, 0.03, -5.0, -10.0, -10.0],
                    guts: &[
                        &[10.0, 0.4],
                        &[20.0, 0.5],
                        &[30.0, 0.6],
                        &[40.0, 0.7],
                        &[50.0, 0.8],
                    ],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::LimitIncrement),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.06, 0.0, -10.0, -15.0, -15.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.06, 0.0, -100.0, -100.0, -100.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.12, 0.06, -1.5, -3.0, -3.0],
                    guts: &[
                        &[5.0, 0.4],
                        &[10.0, 0.5],
                        &[15.0, 0.6],
                        &[20.0, 0.7],
                        &[25.0, 0.8],
                    ],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.12, 0.03, -3.0, -6.0, -6.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.15, 0.06, 0.0, -5.0, -10.0, -10.0],
                    guts: &[],
                }),
            ],
            GaugeProperty::Keyboard => vec![
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 30.0,
                    border: 50.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -1.0, -2.0, -1.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 70.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -1.0, -3.0, -1.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 70.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -2.0, -4.0, -2.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::LimitIncrement),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.2, 0.2, 0.1, -4.0, -8.0, -4.0],
                    guts: &[
                        &[10.0, 0.4],
                        &[20.0, 0.5],
                        &[30.0, 0.6],
                        &[40.0, 0.7],
                        &[50.0, 0.8],
                    ],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::LimitIncrement),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.2, 0.1, 0.0, -6.0, -12.0, -6.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.2, 0.1, 0.0, -100.0, -100.0, -100.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.2, 0.2, 0.1, -1.5, -3.0, -1.5],
                    guts: &[
                        &[5.0, 0.4],
                        &[10.0, 0.5],
                        &[15.0, 0.6],
                        &[20.0, 0.7],
                        &[25.0, 0.8],
                    ],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.2, 0.2, 0.1, -3.0, -6.0, -3.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 0.0,
                    value: &[0.2, 0.1, 0.0, -5.0, -10.0, -5.0],
                    guts: &[],
                }),
            ],
            GaugeProperty::Lr2 => vec![
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 60.0,
                    death: 0.0,
                    value: &[1.2, 1.2, 0.6, -3.2, -4.8, -1.6],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 80.0,
                    death: 0.0,
                    value: &[1.2, 1.2, 0.6, -3.2, -4.8, -1.6],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::Total),
                    min: 2.0,
                    max: 100.0,
                    init: 20.0,
                    border: 80.0,
                    death: 0.0,
                    value: &[1.0, 1.0, 0.5, -4.0, -6.0, -2.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::ModifyDamage),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 2.0,
                    value: &[0.1, 0.1, 0.05, -6.0, -10.0, -2.0],
                    guts: &[&[32.0, 0.6]],
                }),
                gep(GaugeElementParams {
                    modifier: Some(GaugeModifier::ModifyDamage),
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 2.0,
                    value: &[0.1, 0.1, 0.05, -12.0, -20.0, -2.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 2.0,
                    value: &[0.15, 0.06, 0.0, -100.0, -100.0, -10.0],
                    guts: &[],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 2.0,
                    value: &[0.10, 0.10, 0.05, -2.0, -3.0, -2.0],
                    guts: &[&[32.0, 0.6]],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 2.0,
                    value: &[0.10, 0.10, 0.05, -6.0, -10.0, -2.0],
                    guts: &[&[32.0, 0.6]],
                }),
                gep(GaugeElementParams {
                    modifier: None,
                    min: 0.0,
                    max: 100.0,
                    init: 100.0,
                    border: 0.0,
                    death: 2.0,
                    value: &[0.10, 0.10, 0.05, -12.0, -20.0, -2.0],
                    guts: &[],
                }),
            ],
        }
    }
}

/// Gauge element property for each gauge type
#[derive(Clone, Debug)]
pub struct GaugeElementProperty {
    /// Gauge modifier type
    pub modifier: Option<GaugeModifier>,
    /// Gauge change values per judge: PG, GR, GD, BD, PR, MS
    pub value: Vec<f32>,
    /// Minimum gauge value
    pub min: f32,
    /// Maximum gauge value
    pub max: f32,
    /// Initial gauge value
    pub init: f32,
    /// Border value for clearing
    pub border: f32,
    /// Death border
    pub death: f32,
    /// Guts correction table
    pub guts: Vec<Vec<f32>>,
}

/// Parameters for constructing a `GaugeElementProperty` from borrowed slices.
struct GaugeElementParams<'a> {
    pub modifier: Option<GaugeModifier>,
    pub min: f32,
    pub max: f32,
    pub init: f32,
    pub border: f32,
    pub death: f32,
    pub value: &'a [f32],
    pub guts: &'a [&'a [f32]],
}

fn gep(params: GaugeElementParams<'_>) -> GaugeElementProperty {
    GaugeElementProperty {
        modifier: params.modifier,
        min: params.min,
        max: params.max,
        init: params.init,
        border: params.border,
        death: params.death,
        value: params.value.to_vec(),
        guts: params.guts.iter().map(|g| g.to_vec()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gauge_property_values_count() {
        assert_eq!(GaugeProperty::values().len(), 5);
    }

    #[test]
    fn test_gauge_property_names() {
        assert_eq!(GaugeProperty::FiveKeys.name(), "FIVEKEYS");
        assert_eq!(GaugeProperty::SevenKeys.name(), "SEVENKEYS");
        assert_eq!(GaugeProperty::Pms.name(), "PMS");
        assert_eq!(GaugeProperty::Keyboard.name(), "KEYBOARD");
        assert_eq!(GaugeProperty::Lr2.name(), "LR2");
    }

    #[test]
    fn test_gauge_property_equality() {
        assert_eq!(GaugeProperty::FiveKeys, GaugeProperty::FiveKeys);
        assert_ne!(GaugeProperty::FiveKeys, GaugeProperty::SevenKeys);
    }

    #[test]
    fn test_gauge_property_copy() {
        let gp = GaugeProperty::SevenKeys;
        let gp2 = gp;
        assert_eq!(gp, gp2);
    }

    #[test]
    fn test_get_values_returns_9_elements() {
        // Each GaugeProperty variant should produce exactly 9 gauge element properties
        for gp in GaugeProperty::values() {
            let values = gp.element_values();
            assert_eq!(
                values.len(),
                9,
                "{} should have 9 gauge elements",
                gp.name()
            );
        }
    }

    #[test]
    fn test_five_keys_first_element() {
        let values = GaugeProperty::FiveKeys.element_values();
        let first = &values[0];
        assert!(first.modifier.is_some());
        assert_eq!(first.min, 2.0);
        assert_eq!(first.max, 100.0);
        assert_eq!(first.init, 20.0);
        assert_eq!(first.border, 50.0);
        assert_eq!(first.death, 0.0);
        assert_eq!(first.value.len(), 6);
        assert_eq!(first.value[0], 1.0); // PG
        assert_eq!(first.value[4], -3.0); // PR
    }

    #[test]
    fn test_seven_keys_hard_gauge_has_guts() {
        let values = GaugeProperty::SevenKeys.element_values();
        // Index 3 is HARD gauge
        let hard = &values[3];
        assert!(!hard.guts.is_empty());
        assert_eq!(hard.guts.len(), 5);
        // First guts entry: [10.0, 0.4]
        assert_eq!(hard.guts[0], vec![10.0, 0.4]);
        assert_eq!(hard.guts[4], vec![50.0, 0.8]);
    }

    #[test]
    fn test_seven_keys_assist_easy_modifier() {
        let values = GaugeProperty::SevenKeys.element_values();
        let assist_easy = &values[0];
        assert_eq!(assist_easy.modifier, Some(GaugeModifier::Total));
    }

    #[test]
    fn test_seven_keys_hard_modifier() {
        let values = GaugeProperty::SevenKeys.element_values();
        let hard = &values[3];
        assert_eq!(hard.modifier, Some(GaugeModifier::LimitIncrement));
    }

    #[test]
    fn test_seven_keys_hazard_modifier() {
        let values = GaugeProperty::SevenKeys.element_values();
        let hazard = &values[5];
        assert_eq!(hazard.modifier, None);
    }

    #[test]
    fn test_lr2_hard_has_death_border() {
        let values = GaugeProperty::Lr2.element_values();
        // LR2 HARD gauge (index 3) has death=2.0, matching the golden master fixture.
        let hard = &values[3];
        assert_eq!(hard.death, 2.0);
    }

    #[test]
    fn test_gauge_element_value_len() {
        // All gauge elements should have exactly 6 values (PG, GR, GD, BD, PR, MS)
        for gp in GaugeProperty::values() {
            for (i, element) in gp.element_values().iter().enumerate() {
                assert_eq!(
                    element.value.len(),
                    6,
                    "{}[{}] should have 6 judge values",
                    gp.name(),
                    i
                );
            }
        }
    }

    #[test]
    fn test_gauge_element_property_clone() {
        let values = GaugeProperty::SevenKeys.element_values();
        let original = &values[0];
        let cloned = original.clone();
        assert_eq!(cloned.min, original.min);
        assert_eq!(cloned.max, original.max);
        assert_eq!(cloned.init, original.init);
        assert_eq!(cloned.border, original.border);
        assert_eq!(cloned.value, original.value);
    }

    #[test]
    fn test_assist_easy_border_values() {
        // Verify the border values differ across gauge types
        let fk = GaugeProperty::FiveKeys.element_values();
        let sk = GaugeProperty::SevenKeys.element_values();
        let pms = GaugeProperty::Pms.element_values();

        // Assist Easy (index 0) borders
        assert_eq!(fk[0].border, 50.0);
        assert_eq!(sk[0].border, 60.0);
        assert_eq!(pms[0].border, 65.0);
    }
}

// Pure evaluation functions for skin objects.
//
// These functions resolve draw conditions, timer state, interpolation,
// offsets, and text content without any Bevy dependency. They can be
// used both by the Bevy renderer (skin_renderer.rs) and by the
// golden-master test harness for RenderSnapshot capture.

use bms_skin::property_id::BooleanId;
use bms_skin::skin::Skin;
use bms_skin::skin_object::{Color, Rect, SkinObjectBase, SkinOffset};
use bms_skin::skin_text::SkinText;

use crate::state_provider::SkinStateProvider;

/// Checks whether all draw conditions are met.
pub fn check_draw_conditions(base: &SkinObjectBase, provider: &dyn SkinStateProvider) -> bool {
    for &cond in &base.draw_conditions {
        if !provider.boolean_value(cond) {
            return false;
        }
    }
    true
}

/// Checks whether all option conditions (JSON "op" field) are met.
///
/// Option conditions contain two types of values:
/// - **Draw condition IDs** (known ranges like 1..=84, 160..=207, etc.)
///   → evaluated as `BooleanId` via `provider.boolean_value()`.
/// - **Skin option IDs** (outside known ranges) → checked against
///   `skin.options` map where `selected == 1` means enabled.
pub fn check_option_conditions(
    base: &SkinObjectBase,
    skin: &Skin,
    provider: &dyn SkinStateProvider,
) -> bool {
    base.option_conditions.iter().all(|&op| {
        if op == 0 {
            return true;
        }
        let abs = op.abs();
        if is_known_draw_condition_id(abs) {
            return provider.boolean_value(BooleanId(op));
        }
        // Skin option: check against skin.options
        if let Some(selected) = skin.options.get(&abs).copied() {
            if op > 0 { selected == 1 } else { selected == 0 }
        } else {
            // Unknown option IDs default to visible (Java parity: unregistered options show).
            true
        }
    })
}

/// Returns true if the given ID is a known draw condition ID.
///
/// These ranges match the Java BooleanProperty constants that are
/// registered in SkinPropertyMapper as draw conditions. IDs outside
/// these ranges are treated as skin-level option selections.
fn is_known_draw_condition_id(id: i32) -> bool {
    matches!(
        id,
        1..=84
            | 90..=105
            | 118..=207
            | 210..=227
            | 230..=246
            | 261..=263
            | 270..=273
            | 280..=293
            | 300..=318
            | 320..=336
            | 340..=354
            | 400
            | 601..=608
            | 624..=625
            | 1002..=1017
            | 1030..=1031
            | 1046..=1047
            | 1080
            | 1100..=1104
            | 1128..=1131
            | 1160..=1161
            | 1177
            | 1196..=1208
            | 1240
            | 1242..=1243
            | 1262..=1263
            | 1330..=1336
            | 1362..=1363
            | 2241..=2246
    )
}

/// Resolves the animation time from the base timer.
/// Returns None if the timer is required but inactive.
pub fn resolve_timer_time(base: &SkinObjectBase, provider: &dyn SkinStateProvider) -> Option<i64> {
    match base.timer {
        Some(timer_id) => provider.timer_value(timer_id),
        None => Some(provider.now_time_ms()),
    }
}

/// Common resolution: checks draw conditions, resolves timer, interpolates,
/// applies offsets. Returns (rect, color, final_angle, final_alpha) or None
/// if the object should be hidden.
pub fn resolve_common(
    base: &SkinObjectBase,
    provider: &dyn SkinStateProvider,
) -> Option<(Rect, Color, i32, f32)> {
    if !check_draw_conditions(base, provider) {
        return None;
    }

    let time = resolve_timer_time(base, provider)?;
    let (mut rect, color, angle) = base.interpolate(time)?;

    let mut angle_offset = 0.0_f32;
    let mut alpha_offset = 0.0_f32;
    for &oid in &base.offset_ids {
        let off = provider.offset_value(oid);
        apply_offset(&mut rect, &off, &mut angle_offset, &mut alpha_offset);
    }

    let final_angle = angle + angle_offset as i32;
    let final_alpha = (color.a + alpha_offset / 255.0).clamp(0.0, 1.0);

    Some((rect, color, final_angle, final_alpha))
}

/// Applies a SkinOffset to the current rect and accumulates angle/alpha offsets.
pub fn apply_offset(
    rect: &mut Rect,
    offset: &SkinOffset,
    angle_offset: &mut f32,
    alpha_offset: &mut f32,
) {
    rect.x += offset.x;
    rect.y += offset.y;
    rect.w += offset.w;
    rect.h += offset.h;
    *angle_offset += offset.r;
    *alpha_offset += offset.a;
}

/// Resolves text content from a SkinText's ref_id or constant_text.
pub fn resolve_text_content(text: &SkinText, provider: &dyn SkinStateProvider) -> String {
    if let Some(ref_id) = text.ref_id
        && let Some(s) = provider.string_value(ref_id)
    {
        return s;
    }
    text.constant_text.clone().unwrap_or_default()
}

/// Computes shadow color from main color: RGB halved, alpha preserved.
pub fn shadow_color_from_main(r: f32, g: f32, b: f32, a: f32) -> (f32, f32, f32, f32) {
    (r / 2.0, g / 2.0, b / 2.0, a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_provider::StaticStateProvider;
    use bms_skin::property_id::{BooleanId, StringId, TimerId};
    use bms_skin::skin_object::Destination;

    fn make_base_with_dst(time: i64, x: f32, y: f32, w: f32, h: f32) -> SkinObjectBase {
        let mut base = SkinObjectBase::default();
        base.add_destination(Destination {
            time,
            region: Rect::new(x, y, w, h),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });
        base
    }

    #[test]
    fn check_draw_conditions_all_true() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.draw_conditions = vec![BooleanId(1), BooleanId(2)];
        let mut p = StaticStateProvider::default();
        p.booleans.insert(1, true);
        p.booleans.insert(2, true);
        assert!(check_draw_conditions(&base, &p));
    }

    #[test]
    fn check_draw_conditions_one_false() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.draw_conditions = vec![BooleanId(1), BooleanId(2)];
        let mut p = StaticStateProvider::default();
        p.booleans.insert(1, true);
        p.booleans.insert(2, false);
        assert!(!check_draw_conditions(&base, &p));
    }

    #[test]
    fn check_draw_conditions_empty() {
        let base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        let p = StaticStateProvider::default();
        assert!(check_draw_conditions(&base, &p));
    }

    #[test]
    fn check_draw_conditions_negated() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.draw_conditions = vec![BooleanId(-5)]; // NOT id=5
        let mut p = StaticStateProvider::default();
        p.booleans.insert(5, false);
        // NOT false = true
        assert!(check_draw_conditions(&base, &p));
    }

    #[test]
    fn resolve_timer_time_no_timer() {
        let base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        let mut p = StaticStateProvider::default();
        p.time_ms = 5000;
        assert_eq!(resolve_timer_time(&base, &p), Some(5000));
    }

    #[test]
    fn resolve_timer_time_active_timer() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.timer = Some(TimerId(10));
        let mut p = StaticStateProvider::default();
        p.timers.insert(10, 3000);
        assert_eq!(resolve_timer_time(&base, &p), Some(3000));
    }

    #[test]
    fn resolve_timer_time_inactive_timer() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.timer = Some(TimerId(10));
        let p = StaticStateProvider::default(); // timer 10 not set
        assert_eq!(resolve_timer_time(&base, &p), None);
    }

    #[test]
    fn apply_offset_accumulates() {
        let mut rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let offset = SkinOffset {
            x: 5.0,
            y: -3.0,
            w: 10.0,
            h: -5.0,
            r: 15.0,
            a: 32.0,
        };
        let mut angle_off = 0.0_f32;
        let mut alpha_off = 0.0_f32;
        apply_offset(&mut rect, &offset, &mut angle_off, &mut alpha_off);

        assert!((rect.x - 15.0).abs() < 0.001);
        assert!((rect.y - 17.0).abs() < 0.001);
        assert!((rect.w - 110.0).abs() < 0.001);
        assert!((rect.h - 45.0).abs() < 0.001);
        assert!((angle_off - 15.0).abs() < 0.001);
        assert!((alpha_off - 32.0).abs() < 0.001);
    }

    #[test]
    fn apply_multiple_offsets() {
        let mut rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let off1 = SkinOffset {
            x: 10.0,
            y: 20.0,
            w: 0.0,
            h: 0.0,
            r: 5.0,
            a: 10.0,
        };
        let off2 = SkinOffset {
            x: -5.0,
            y: -10.0,
            w: 0.0,
            h: 0.0,
            r: -3.0,
            a: 20.0,
        };
        let mut angle_off = 0.0_f32;
        let mut alpha_off = 0.0_f32;
        apply_offset(&mut rect, &off1, &mut angle_off, &mut alpha_off);
        apply_offset(&mut rect, &off2, &mut angle_off, &mut alpha_off);

        assert!((rect.x - 5.0).abs() < 0.001);
        assert!((rect.y - 10.0).abs() < 0.001);
        assert!((angle_off - 2.0).abs() < 0.001);
        assert!((alpha_off - 30.0).abs() < 0.001);
    }

    #[test]
    fn interpolation_with_offset() {
        let mut base = SkinObjectBase::default();
        base.add_destination(Destination {
            time: 0,
            region: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });
        base.add_destination(Destination {
            time: 100,
            region: Rect::new(100.0, 0.0, 100.0, 100.0),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });
        base.set_offset_ids(&[1]);

        let (mut rect, _color, _angle) = base.interpolate(50).unwrap();
        // At t=50: x should be 50
        assert!((rect.x - 50.0).abs() < 0.001);

        let offset = SkinOffset {
            x: 10.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            r: 0.0,
            a: 0.0,
        };
        let mut ao = 0.0_f32;
        let mut aao = 0.0_f32;
        apply_offset(&mut rect, &offset, &mut ao, &mut aao);
        // After offset: x = 60
        assert!((rect.x - 60.0).abs() < 0.001);
    }

    #[test]
    fn resolve_text_content_from_provider() {
        let text = SkinText {
            ref_id: Some(StringId(42)),
            constant_text: Some("fallback".to_string()),
            ..Default::default()
        };
        let mut p = StaticStateProvider::default();
        p.strings.insert(42, "dynamic text".to_string());
        assert_eq!(resolve_text_content(&text, &p), "dynamic text");
    }

    #[test]
    fn resolve_text_content_fallback_constant() {
        let text = SkinText {
            ref_id: Some(StringId(42)),
            constant_text: Some("fallback".to_string()),
            ..Default::default()
        };
        let p = StaticStateProvider::default(); // no string 42
        assert_eq!(resolve_text_content(&text, &p), "fallback");
    }

    #[test]
    fn resolve_text_content_no_ref_no_constant() {
        let text = SkinText::default();
        let p = StaticStateProvider::default();
        assert_eq!(resolve_text_content(&text, &p), "");
    }

    #[test]
    fn resolve_common_returns_none_when_hidden() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.draw_conditions = vec![BooleanId(1)];
        let p = StaticStateProvider::default(); // bool 1 = false
        assert!(resolve_common(&base, &p).is_none());
    }

    #[test]
    fn resolve_common_returns_values() {
        let base = make_base_with_dst(0, 10.0, 20.0, 100.0, 50.0);
        let p = StaticStateProvider::default();
        let (rect, color, angle, alpha) = resolve_common(&base, &p).unwrap();
        assert!((rect.x - 10.0).abs() < 0.001);
        assert!((rect.y - 20.0).abs() < 0.001);
        assert!((color.a - 1.0).abs() < 0.001);
        assert_eq!(angle, 0);
        assert!((alpha - 1.0).abs() < 0.001);
    }

    #[test]
    fn shadow_color_halves_rgb() {
        let (r, g, b, a) = shadow_color_from_main(1.0, 0.8, 0.6, 0.9);
        assert!((r - 0.5).abs() < 0.001);
        assert!((g - 0.4).abs() < 0.001);
        assert!((b - 0.3).abs() < 0.001);
        assert!((a - 0.9).abs() < 0.001);
    }

    #[test]
    fn shadow_color_zero_input() {
        let (r, g, b, a) = shadow_color_from_main(0.0, 0.0, 0.0, 1.0);
        assert!(r.abs() < 0.001);
        assert!(g.abs() < 0.001);
        assert!(b.abs() < 0.001);
        assert!((a - 1.0).abs() < 0.001);
    }

    #[test]
    fn triple_offset_accumulation() {
        let mut base = SkinObjectBase::default();
        base.add_destination(Destination {
            time: 0,
            region: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });
        base.set_offset_ids(&[1, 2, 3]);

        let mut p = StaticStateProvider::default();
        p.offsets.insert(
            1,
            bms_skin::skin_object::SkinOffset {
                x: 10.0,
                y: 5.0,
                w: 0.0,
                h: 0.0,
                r: 10.0,
                a: 20.0,
            },
        );
        p.offsets.insert(
            2,
            bms_skin::skin_object::SkinOffset {
                x: 20.0,
                y: 10.0,
                w: 0.0,
                h: 0.0,
                r: 15.0,
                a: 30.0,
            },
        );
        p.offsets.insert(
            3,
            bms_skin::skin_object::SkinOffset {
                x: -5.0,
                y: -3.0,
                w: 0.0,
                h: 0.0,
                r: 5.0,
                a: 10.0,
            },
        );

        let (rect, _color, angle, alpha) = resolve_common(&base, &p).unwrap();
        // x: 0 + 10 + 20 - 5 = 25
        assert!((rect.x - 25.0).abs() < 0.001);
        // y: 0 + 5 + 10 - 3 = 12
        assert!((rect.y - 12.0).abs() < 0.001);
        // angle: 0 + (10 + 15 + 5) = 30
        assert_eq!(angle, 30);
        // alpha: 1.0 + (20 + 30 + 10) / 255.0 = 1.0 + 0.2353 = 1.2353 → clamped to 1.0
        assert!((alpha - 1.0).abs() < 0.001);
    }

    #[test]
    fn alpha_clamp_upper() {
        let mut base = SkinObjectBase::default();
        base.add_destination(Destination {
            time: 0,
            region: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });
        base.set_offset_ids(&[1]);

        let mut p = StaticStateProvider::default();
        p.offsets.insert(
            1,
            bms_skin::skin_object::SkinOffset {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                r: 0.0,
                a: 300.0,
            },
        );

        let (_rect, _color, _angle, alpha) = resolve_common(&base, &p).unwrap();
        // 1.0 + 300/255 = 2.176 → clamped to 1.0
        assert!((alpha - 1.0).abs() < 0.001);
    }

    #[test]
    fn alpha_clamp_lower() {
        let mut base = SkinObjectBase::default();
        base.add_destination(Destination {
            time: 0,
            region: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });
        base.set_offset_ids(&[1]);

        let mut p = StaticStateProvider::default();
        p.offsets.insert(
            1,
            bms_skin::skin_object::SkinOffset {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                r: 0.0,
                a: -300.0,
            },
        );

        let (_rect, _color, _angle, alpha) = resolve_common(&base, &p).unwrap();
        // 1.0 + (-300)/255 = -0.176 → clamped to 0.0
        assert!(alpha.abs() < 0.001);
    }

    #[test]
    fn resolve_common_with_timer() {
        let mut base = SkinObjectBase::default();
        base.timer = Some(TimerId(10));
        base.add_destination(Destination {
            time: 0,
            region: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });
        base.add_destination(Destination {
            time: 100,
            region: Rect::new(100.0, 0.0, 100.0, 100.0),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });

        let mut p = StaticStateProvider::default();
        p.timers.insert(10, 50); // timer at 50ms

        let (rect, _, _, _) = resolve_common(&base, &p).unwrap();
        // Timer value 50 → midpoint → x=50
        assert!((rect.x - 50.0).abs() < 0.001);
    }

    #[test]
    fn resolve_common_timer_inactive_returns_none() {
        let mut base = SkinObjectBase::default();
        base.timer = Some(TimerId(10));
        base.add_destination(Destination {
            time: 0,
            region: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });

        let p = StaticStateProvider::default(); // timer 10 not set
        assert!(resolve_common(&base, &p).is_none());
    }

    fn make_skin() -> bms_skin::skin::Skin {
        bms_skin::skin::Skin::new(bms_skin::skin_header::SkinHeader::default())
    }

    #[test]
    fn option_conditions_draw_condition_all_true() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        // 160 and 161 are in the known draw condition ID range (118..=207)
        base.option_conditions = vec![160, 161];
        let mut p = StaticStateProvider::default();
        p.booleans.insert(160, true);
        p.booleans.insert(161, true);
        let skin = make_skin();
        assert!(check_option_conditions(&base, &skin, &p));
    }

    #[test]
    fn option_conditions_draw_condition_one_false() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.option_conditions = vec![160, 162];
        let mut p = StaticStateProvider::default();
        p.booleans.insert(160, true);
        p.booleans.insert(162, false);
        let skin = make_skin();
        assert!(!check_option_conditions(&base, &skin, &p));
    }

    #[test]
    fn option_conditions_skip_zero() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.option_conditions = vec![0, 160];
        let mut p = StaticStateProvider::default();
        p.booleans.insert(160, true);
        let skin = make_skin();
        assert!(check_option_conditions(&base, &skin, &p));
    }

    #[test]
    fn option_conditions_skin_option_selected() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        // ID 5000 is outside known draw condition ranges → skin option
        base.option_conditions = vec![5000];
        let p = StaticStateProvider::default();
        let mut skin = make_skin();
        skin.options.insert(5000, 1); // selected = 1 → enabled
        assert!(check_option_conditions(&base, &skin, &p));
    }

    #[test]
    fn option_conditions_skin_option_not_selected() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.option_conditions = vec![5000];
        let p = StaticStateProvider::default();
        let mut skin = make_skin();
        skin.options.insert(5000, 0); // selected = 0 → disabled
        assert!(!check_option_conditions(&base, &skin, &p));
    }

    #[test]
    fn option_conditions_negated_skin_option() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        // Negative op → negated: show when selected == 0
        base.option_conditions = vec![-5000];
        let p = StaticStateProvider::default();
        let mut skin = make_skin();
        skin.options.insert(5000, 0); // selected = 0, negated → show
        assert!(check_option_conditions(&base, &skin, &p));
    }

    #[test]
    fn option_conditions_unknown_skin_option_defaults_visible() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        base.option_conditions = vec![9999]; // not in known ranges, not in skin.options
        let p = StaticStateProvider::default();
        let skin = make_skin();
        // Unknown option IDs default to visible (Java parity)
        assert!(check_option_conditions(&base, &skin, &p));
    }

    #[test]
    fn draw_conditions_mixed_negation() {
        let mut base = make_base_with_dst(0, 0.0, 0.0, 100.0, 100.0);
        // Positive BooleanId(1) and negative BooleanId(-2) → NOT id=2
        base.draw_conditions = vec![BooleanId(1), BooleanId(-2)];

        let mut p = StaticStateProvider::default();
        p.booleans.insert(1, true);
        p.booleans.insert(2, false); // NOT false = true

        // Both conditions should be true
        assert!(check_draw_conditions(&base, &p));

        // Now set id=2 to true → NOT true = false → overall false
        p.booleans.insert(2, true);
        assert!(!check_draw_conditions(&base, &p));
    }
}

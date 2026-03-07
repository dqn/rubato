// Pure-function keyframe evaluation for golden-master snapshot comparison.
//
// Replicates SkinObjectData::prepare_region/prepare_color/prepare_angle as pure
// functions that take &SkinObjectData + &dyn SkinStateProvider and return computed
// values without mutation.

use rubato_skin::skin_object::{SkinObjectData, SkinObjectDestination};
use rubato_skin::skin_text::SkinTextData;
use rubato_skin::stubs::SkinOffset;

use crate::state_provider::SkinStateProvider;

/// Evaluated rectangle (skin coordinates).
pub struct EvalRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Evaluated color (RGB, alpha separate).
pub struct EvalColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// Resolves the common keyframe evaluation for a SkinObjectData.
/// Returns (rect, color, angle, alpha) or None if the object should not be drawn.
pub fn resolve_common(
    data: &SkinObjectData,
    provider: &dyn SkinStateProvider,
) -> Option<(EvalRect, EvalColor, i32, f32)> {
    if data.dst.is_empty() {
        return None;
    }

    let mut time = provider.now_time_ms();

    // Timer check: extract timer_id, look up in provider.
    let timer_id = data
        .dsttimer
        .as_ref()
        .map(|t| t.get_timer_id())
        .unwrap_or(0);
    if timer_id != 0 {
        if !provider.is_timer_on(timer_id) {
            return None;
        }
        time -= provider.timer_value_ms(timer_id);
    }

    // Loop handling (replicates SkinObjectData::prepare_region loop logic).
    let lasttime = data.endtime;
    if data.dstloop == -1 {
        if time > data.endtime {
            time = -1;
        }
    } else if lasttime > 0 && time > data.dstloop as i64 {
        if lasttime == data.dstloop as i64 {
            time = data.dstloop as i64;
        } else {
            time = (time - data.dstloop as i64) % (lasttime - data.dstloop as i64)
                + data.dstloop as i64;
        }
    }
    if data.starttime > time {
        return None;
    }

    // Resolve offsets from provider.
    let offsets: Vec<Option<SkinOffset>> = data
        .offset
        .iter()
        .map(|&id| provider.offset_value(id))
        .collect();

    // Compute interpolation rate and keyframe index.
    let (rate, index) = compute_rate(&data.dst, time, data.acc);

    // Region interpolation.
    let rect = compute_region(data, rate, index, &offsets);

    // Color interpolation.
    let (color, alpha) = compute_color(data, rate, index, &offsets);

    // Angle interpolation.
    let angle = compute_angle(data, rate, index, &offsets);

    Some((rect, color, angle, alpha))
}

/// Compute interpolation rate and keyframe index.
/// Replicates SkinObjectData::rate() as a pure function.
fn compute_rate(dst: &[SkinObjectDestination], nowtime: i64, acc: i32) -> (f32, usize) {
    let last = dst.len() - 1;
    let mut time2 = dst[last].time;
    if nowtime == time2 {
        return (0.0, last);
    }
    for i in (0..last).rev() {
        let time1 = dst[i].time;
        if time1 <= nowtime && time2 > nowtime {
            let mut rate = (nowtime - time1) as f32 / (time2 - time1) as f32;
            match acc {
                1 => rate = rate * rate,
                2 => rate = 1.0 - (rate - 1.0) * (rate - 1.0),
                _ => {}
            }
            return (rate, i);
        }
        time2 = time1;
    }
    (0.0, 0)
}

/// Compute interpolated region.
/// Replicates the region part of SkinObjectData::prepare_region().
fn compute_region(
    data: &SkinObjectData,
    rate: f32,
    index: usize,
    offsets: &[Option<SkinOffset>],
) -> EvalRect {
    let mut rect = if let Some(ref fixr) = data.fixr {
        // Fixed region: use directly, apply offsets if any.
        EvalRect {
            x: fixr.x,
            y: fixr.y,
            w: fixr.width,
            h: fixr.height,
        }
    } else if rate == 0.0 {
        let r = &data.dst[index].region;
        EvalRect {
            x: r.x,
            y: r.y,
            w: r.width,
            h: r.height,
        }
    } else if data.acc == 3 {
        // Discrete mode: snap to current keyframe.
        let r = &data.dst[index].region;
        EvalRect {
            x: r.x,
            y: r.y,
            w: r.width,
            h: r.height,
        }
    } else {
        // Interpolate between keyframes.
        let r1 = &data.dst[index].region;
        let r2 = &data.dst[index + 1].region;
        EvalRect {
            x: r1.x + (r2.x - r1.x) * rate,
            y: r1.y + (r2.y - r1.y) * rate,
            w: r1.width + (r2.width - r1.width) * rate,
            h: r1.height + (r2.height - r1.height) * rate,
        }
    };

    // Apply offsets.
    for off in offsets.iter().flatten() {
        if !data.relative {
            rect.x += off.x - off.w / 2.0;
            rect.y += off.y - off.h / 2.0;
        }
        rect.w += off.w;
        rect.h += off.h;
    }

    rect
}

/// Compute interpolated color and alpha.
/// Replicates SkinObjectData::prepare_color() as a pure function.
/// Note: alpha offset is only applied for fixc and rate==0 cases
/// (matches Java early-return behavior for acc==3 and interpolated cases).
fn compute_color(
    data: &SkinObjectData,
    rate: f32,
    index: usize,
    offsets: &[Option<SkinOffset>],
) -> (EvalColor, f32) {
    if let Some(ref fixc) = data.fixc {
        let mut alpha = fixc.a;
        for off in offsets.iter().flatten() {
            alpha += off.a / 255.0;
            alpha = alpha.clamp(0.0, 1.0);
        }
        return (
            EvalColor {
                r: fixc.r,
                g: fixc.g,
                b: fixc.b,
            },
            alpha,
        );
    }

    if rate == 0.0 {
        let c = &data.dst[index].color;
        let mut alpha = c.a;
        // Alpha offset applied only for rate==0 (matches Java).
        for off in offsets.iter().flatten() {
            alpha += off.a / 255.0;
            alpha = alpha.clamp(0.0, 1.0);
        }
        (
            EvalColor {
                r: c.r,
                g: c.g,
                b: c.b,
            },
            alpha,
        )
    } else if data.acc == 3 {
        // Discrete: no offset applied (matches Java early return).
        let c = &data.dst[index].color;
        (
            EvalColor {
                r: c.r,
                g: c.g,
                b: c.b,
            },
            c.a,
        )
    } else {
        // Interpolated: no offset applied (matches Java early return).
        let c1 = &data.dst[index].color;
        let c2 = &data.dst[index + 1].color;
        (
            EvalColor {
                r: c1.r + (c2.r - c1.r) * rate,
                g: c1.g + (c2.g - c1.g) * rate,
                b: c1.b + (c2.b - c1.b) * rate,
            },
            c1.a + (c2.a - c1.a) * rate,
        )
    }
}

/// Compute interpolated angle.
/// Replicates SkinObjectData::prepare_angle() as a pure function.
fn compute_angle(
    data: &SkinObjectData,
    rate: f32,
    index: usize,
    offsets: &[Option<SkinOffset>],
) -> i32 {
    let mut angle = if data.fixa != i32::MIN {
        data.fixa
    } else if rate == 0.0 || data.acc == 3 {
        data.dst[index].angle
    } else {
        (data.dst[index].angle as f32
            + (data.dst[index + 1].angle - data.dst[index].angle) as f32 * rate) as i32
    };

    for off in offsets.iter().flatten() {
        angle += off.r as i32;
    }

    angle
}

/// Resolve text content from a SkinTextData using the provider.
pub fn resolve_text_content(text_data: &SkinTextData, provider: &dyn SkinStateProvider) -> String {
    if let Some(ref ref_prop) = text_data.ref_prop {
        let id = ref_prop.get_id();
        if id != i32::MIN {
            if let Some(content) = provider.string_value(id) {
                return content;
            }
            if id == rubato_skin::skin_property::STRING_TABLE_FULL {
                // Java GM mocks allocate PlayerResource via Unsafe without running field
                // initializers, and tablefull is computed from null + "". This yields
                // "null" and keeps tablefull text visible in decide skin snapshots.
                return "null".to_string();
            }
        }
    }
    text_data.constant_text.clone().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_provider::StaticStateProvider;
    use rubato_skin::skin_object::SkinObjectData;

    fn make_data_with_single_dst() -> SkinObjectData {
        let mut data = SkinObjectData::new();
        data.set_destination_with_int_timer_ops(
            0,
            10.0,
            20.0,
            100.0,
            50.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[],
        );
        data
    }

    fn make_data_with_two_dst() -> SkinObjectData {
        let mut data = SkinObjectData::new();
        data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            100.0,
            100.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[],
        );
        data.set_destination_with_int_timer_ops(
            1000,
            200.0,
            200.0,
            300.0,
            300.0,
            0,
            128,
            128,
            128,
            128,
            0,
            0,
            0,
            0,
            0,
            0,
            &[],
        );
        data
    }

    #[test]
    fn resolve_single_keyframe() {
        let data = make_data_with_single_dst();
        let provider = StaticStateProvider::default();
        let result = resolve_common(&data, &provider);
        let (rect, _color, angle, alpha) = result.unwrap();
        assert!((rect.x - 10.0).abs() < 0.001);
        assert!((rect.y - 20.0).abs() < 0.001);
        assert!((rect.w - 100.0).abs() < 0.001);
        assert!((rect.h - 50.0).abs() < 0.001);
        assert_eq!(angle, 0);
        assert!((alpha - 1.0).abs() < 0.001);
    }

    #[test]
    fn resolve_interpolated_midpoint() {
        let data = make_data_with_two_dst();
        let mut provider = StaticStateProvider::default();
        provider.time_ms = 500;
        let result = resolve_common(&data, &provider);
        let (rect, _color, _angle, _alpha) = result.unwrap();
        // At t=500, rate=0.5: x = 0 + (200-0)*0.5 = 100
        assert!((rect.x - 100.0).abs() < 1.0);
        assert!((rect.y - 100.0).abs() < 1.0);
    }

    #[test]
    fn timer_off_returns_none() {
        // Set timer via destination (timer=42)
        let mut data = SkinObjectData::new();
        data.set_destination_with_int_timer_ops(
            0,
            10.0,
            20.0,
            100.0,
            50.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            42,
            &[],
        );
        let provider = StaticStateProvider::default();
        // Timer 42 is not set, so is_timer_on(42) returns false.
        let result = resolve_common(&data, &provider);
        assert!(result.is_none());
    }

    #[test]
    fn timer_on_shifts_time() {
        let mut data = SkinObjectData::new();
        data.set_destination_with_int_timer_ops(
            0,
            10.0,
            20.0,
            100.0,
            50.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            42,
            &[],
        );
        let mut provider = StaticStateProvider::default();
        provider.time_ms = 100;
        provider.timers.insert(42, 50);
        // effective_time = 100 - 50 = 50, which is > starttime=0, so visible.
        let result = resolve_common(&data, &provider);
        assert!(result.is_some());
    }

    #[test]
    fn empty_dst_returns_none() {
        let data = SkinObjectData::new();
        let provider = StaticStateProvider::default();
        let result = resolve_common(&data, &provider);
        assert!(result.is_none());
    }

    #[test]
    fn before_start_time_returns_none() {
        let mut data = SkinObjectData::new();
        data.set_destination_with_int_timer_ops(
            100,
            10.0,
            20.0,
            100.0,
            50.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[],
        );
        let mut provider = StaticStateProvider::default();
        provider.time_ms = 50; // Before starttime=100
        let result = resolve_common(&data, &provider);
        assert!(result.is_none());
    }

    #[test]
    fn ease_in_acceleration() {
        let mut data = SkinObjectData::new();
        // acc=1 (ease-in: rate = rate^2)
        data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            0.0,
            0.0,
            1,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[],
        );
        data.set_destination_with_int_timer_ops(
            1000,
            100.0,
            0.0,
            0.0,
            0.0,
            1,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[],
        );
        let mut provider = StaticStateProvider::default();
        provider.time_ms = 500;
        let result = resolve_common(&data, &provider);
        let (rect, _, _, _) = result.unwrap();
        // At t=500, linear_rate=0.5, ease_in_rate=0.25, x = 0 + 100*0.25 = 25
        assert!((rect.x - 25.0).abs() < 1.0);
    }
}

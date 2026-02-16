// Skin object base types ported from SkinObject.java.
//
// Contains destination animation, offset handling, and draw conditions.
// Rendering logic (draw calls) is deferred to Phase 10.

use serde::{Deserialize, Serialize};

use crate::property_id::{BooleanId, EventId, OFFSET_MAX, TimerId};
use crate::stretch_type::StretchType;

// ---------------------------------------------------------------------------
// Destination keyframe
// ---------------------------------------------------------------------------

/// A single destination keyframe for animation.
#[derive(Debug, Clone, PartialEq)]
pub struct Destination {
    /// Time in milliseconds from timer start.
    pub time: i64,
    /// Destination rectangle (x, y, width, height).
    pub region: Rect,
    /// RGBA color (0.0-1.0).
    pub color: Color,
    /// Rotation angle in degrees.
    pub angle: i32,
    /// Acceleration type: 0=linear, 1=ease-in, 2=ease-out, 3=discrete.
    pub acc: i32,
}

/// Axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

/// RGBA color with components in 0.0-1.0 range.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn from_rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    pub fn white() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

/// Offset values applied on top of destination animation.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct SkinOffset {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Rotation offset in degrees.
    pub r: f32,
    /// Alpha offset (0-255 scale, added to color alpha).
    pub a: f32,
}

// ---------------------------------------------------------------------------
// Rotation center
// ---------------------------------------------------------------------------

/// Rotation center lookup tables (Java CENTERX/CENTERY arrays).
const CENTER_X: [f32; 10] = [0.5, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0];
const CENTER_Y: [f32; 10] = [0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 1.0, 1.0, 1.0];

/// Returns the rotation center (cx, cy) for the given center index (0-9).
/// Values are in the range 0.0-1.0 relative to the object's region.
pub fn rotation_center(center: i32) -> (f32, f32) {
    let idx = center.clamp(0, 9) as usize;
    (CENTER_X[idx], CENTER_Y[idx])
}

// ---------------------------------------------------------------------------
// SkinObjectBase
// ---------------------------------------------------------------------------

/// Base properties shared by all skin objects.
#[derive(Debug, Clone)]
pub struct SkinObjectBase {
    /// Destination keyframes sorted by time.
    pub destinations: Vec<Destination>,
    /// Timer that drives the animation. None = always active.
    pub timer: Option<TimerId>,
    /// Loop point in milliseconds. 0 = no loop, -1 = play once.
    pub loop_time: i32,
    /// Blend mode: 0=normal, 2=additive, 9=invert.
    pub blend: i32,
    /// Texture filter: 0=nearest, 1=linear.
    pub filter: i32,
    /// Rotation center index (0-9).
    pub center: i32,
    /// Computed center X (0.0-1.0).
    pub center_x: f32,
    /// Computed center Y (0.0-1.0).
    pub center_y: f32,
    /// Acceleration type.
    pub acc: i32,
    /// Stretch type for image rendering.
    pub stretch: StretchType,
    /// Offset IDs (refer to custom offsets defined in skin).
    pub offset_ids: Vec<i32>,
    /// Draw conditions (boolean property IDs). All must be true for drawing.
    pub draw_conditions: Vec<BooleanId>,
    /// Option conditions (legacy integer-based conditions).
    pub option_conditions: Vec<i32>,
    /// Click event ID.
    pub click_event: Option<EventId>,
    /// Click event type: 0=plus-only, 1=minus-only, 2=left-right split, 3=up-down split.
    pub click_event_type: i32,
    /// Mouse rectangle constraint for draw condition.
    pub mouse_rect: Option<Rect>,
    /// Whether offsets are applied in relative mode.
    pub relative: bool,
    /// Whether a Lua draw function is present (unresolvable in test harness).
    pub has_script_draw: bool,
    /// Optional debug name.
    pub name: Option<String>,
}

impl Default for SkinObjectBase {
    fn default() -> Self {
        Self {
            destinations: Vec::new(),
            timer: None,
            loop_time: 0,
            blend: 0,
            filter: 0,
            center: 0,
            center_x: 0.5,
            center_y: 0.5,
            acc: 0,
            stretch: StretchType::default(),
            offset_ids: Vec::new(),
            draw_conditions: Vec::new(),
            option_conditions: Vec::new(),
            click_event: None,
            click_event_type: 0,
            mouse_rect: None,
            relative: false,
            has_script_draw: false,
            name: None,
        }
    }
}

impl SkinObjectBase {
    /// Adds a destination keyframe, maintaining time-sorted order.
    /// First-keyframe values for blend, filter, center, acc, timer, and loop
    /// are captured on the first call (matching Java behavior).
    pub fn add_destination(&mut self, dst: Destination) {
        if self.destinations.is_empty() {
            self.acc = dst.acc;
        }

        // Insert in time-sorted order
        let pos = self
            .destinations
            .iter()
            .position(|d| d.time > dst.time)
            .unwrap_or(self.destinations.len());
        self.destinations.insert(pos, dst);
    }

    /// Sets the offset IDs, filtering to valid range (1..=OFFSET_MAX).
    /// Only sets if no offsets have been set yet (matching Java behavior).
    pub fn set_offset_ids(&mut self, ids: &[i32]) {
        if !self.offset_ids.is_empty() {
            return;
        }
        let mut unique = Vec::new();
        for &id in ids {
            if id > 0 && id <= OFFSET_MAX && !unique.contains(&id) {
                unique.push(id);
            }
        }
        self.offset_ids = unique;
    }

    /// Sets the rotation center from an index (0-9).
    pub fn set_center(&mut self, center: i32) {
        if (0..10).contains(&center) {
            self.center = center;
            let (cx, cy) = rotation_center(center);
            self.center_x = cx;
            self.center_y = cy;
        }
    }

    /// Returns true if this object has any destination keyframes.
    pub fn is_valid(&self) -> bool {
        !self.destinations.is_empty()
    }

    /// Computes the interpolated region, color, and angle for the given time.
    /// Returns None if the object should not be drawn at this time.
    pub fn interpolate(&self, time: i64) -> Option<(Rect, Color, i32)> {
        if self.destinations.is_empty() {
            return None;
        }

        let start = self.destinations[0].time;
        let end = self.destinations.last().unwrap().time;

        // Apply loop
        let time = if self.loop_time == -1 {
            if time > end {
                return None;
            }
            time
        } else if end > 0 && time > self.loop_time as i64 {
            if end == self.loop_time as i64 {
                self.loop_time as i64
            } else {
                let loop_start = self.loop_time as i64;
                (time - loop_start) % (end - loop_start) + loop_start
            }
        } else {
            time
        };

        if time < start {
            return None;
        }

        // Find the keyframe pair
        let dsts = &self.destinations;
        if time >= dsts.last().unwrap().time {
            let last = dsts.last().unwrap();
            return Some((last.region, last.color, last.angle));
        }

        for i in (0..dsts.len() - 1).rev() {
            let t1 = dsts[i].time;
            let t2 = dsts[i + 1].time;
            if t1 <= time && t2 > time {
                let raw_rate = (time - t1) as f32 / (t2 - t1) as f32;
                let rate = match dsts[i].acc {
                    1 => raw_rate * raw_rate,                                         // ease-in
                    2 => 1.0 - (raw_rate - 1.0) * (raw_rate - 1.0),                   // ease-out
                    3 => return Some((dsts[i].region, dsts[i].color, dsts[i].angle)), // discrete
                    _ => raw_rate,                                                    // linear
                };

                let r1 = &dsts[i].region;
                let r2 = &dsts[i + 1].region;
                let region = Rect {
                    x: r1.x + (r2.x - r1.x) * rate,
                    y: r1.y + (r2.y - r1.y) * rate,
                    w: r1.w + (r2.w - r1.w) * rate,
                    h: r1.h + (r2.h - r1.h) * rate,
                };

                let c1 = &dsts[i].color;
                let c2 = &dsts[i + 1].color;
                let color = Color {
                    r: c1.r + (c2.r - c1.r) * rate,
                    g: c1.g + (c2.g - c1.g) * rate,
                    b: c1.b + (c2.b - c1.b) * rate,
                    a: c1.a + (c2.a - c1.a) * rate,
                };

                let a1 = dsts[i].angle;
                let a2 = dsts[i + 1].angle;
                let angle = a1 + ((a2 - a1) as f32 * rate) as i32;

                return Some((region, color, angle));
            }
        }

        let first = &dsts[0];
        Some((first.region, first.color, first.angle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dst(time: i64, x: f32, y: f32, w: f32, h: f32, a: u8) -> Destination {
        Destination {
            time,
            region: Rect::new(x, y, w, h),
            color: Color::from_rgba_u8(255, 255, 255, a),
            angle: 0,
            acc: 0,
        }
    }

    #[test]
    fn test_single_destination() {
        let mut base = SkinObjectBase::default();
        base.add_destination(make_dst(0, 10.0, 20.0, 100.0, 50.0, 255));

        let (r, c, a) = base.interpolate(0).unwrap();
        assert_eq!(r, Rect::new(10.0, 20.0, 100.0, 50.0));
        assert!((c.a - 1.0).abs() < 0.001);
        assert_eq!(a, 0);

        // After the only keyframe -> still returns last
        let (r, _, _) = base.interpolate(1000).unwrap();
        assert_eq!(r, Rect::new(10.0, 20.0, 100.0, 50.0));
    }

    #[test]
    fn test_linear_interpolation() {
        let mut base = SkinObjectBase::default();
        base.add_destination(make_dst(0, 0.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 0));

        // At t=50, halfway
        let (r, c, _) = base.interpolate(50).unwrap();
        assert!((r.x - 50.0).abs() < 0.001);
        assert!((c.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_before_start() {
        let mut base = SkinObjectBase::default();
        base.add_destination(make_dst(100, 0.0, 0.0, 100.0, 100.0, 255));
        assert!(base.interpolate(50).is_none());
    }

    #[test]
    fn test_loop() {
        let mut base = SkinObjectBase::default();
        base.loop_time = 0;
        base.add_destination(make_dst(0, 0.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 255));

        // t=150 with loop_time=0, end=100: (150 - 0) % (100 - 0) + 0 = 50
        let (r, _, _) = base.interpolate(150).unwrap();
        assert!((r.x - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_play_once() {
        let mut base = SkinObjectBase::default();
        base.loop_time = -1;
        base.add_destination(make_dst(0, 0.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 255));

        assert!(base.interpolate(50).is_some());
        assert!(base.interpolate(101).is_none()); // past end -> None
    }

    #[test]
    fn test_ease_in() {
        let mut base = SkinObjectBase::default();
        let mut d1 = make_dst(0, 0.0, 0.0, 100.0, 100.0, 255);
        d1.acc = 1; // ease-in
        base.add_destination(d1);
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 255));

        let (r, _, _) = base.interpolate(50).unwrap();
        // Ease-in: rate = 0.5^2 = 0.25, so x = 25
        assert!((r.x - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_discrete() {
        let mut base = SkinObjectBase::default();
        let mut d1 = make_dst(0, 0.0, 0.0, 100.0, 100.0, 255);
        d1.acc = 3; // discrete
        base.add_destination(d1);
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 255));

        let (r, _, _) = base.interpolate(50).unwrap();
        // Discrete: stays at first keyframe
        assert!((r.x - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_offset_ids() {
        let mut base = SkinObjectBase::default();
        base.set_offset_ids(&[0, 10, 200, 50, 10]); // 0 and 200 out of range, 10 duplicate
        assert_eq!(base.offset_ids, vec![10, 50]);

        // Second call is ignored (Java behavior)
        base.set_offset_ids(&[1, 2]);
        assert_eq!(base.offset_ids, vec![10, 50]);
    }

    #[test]
    fn test_rotation_center() {
        let (cx, cy) = rotation_center(0);
        assert_eq!((cx, cy), (0.5, 0.5)); // center
        let (cx, cy) = rotation_center(1);
        assert_eq!((cx, cy), (0.0, 0.0)); // top-left
        let (cx, cy) = rotation_center(9);
        assert_eq!((cx, cy), (1.0, 1.0)); // bottom-right
    }

    #[test]
    fn test_rect_contains() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(r.contains(50.0, 40.0));
        assert!(!r.contains(5.0, 40.0));
        assert!(!r.contains(50.0, 100.0));
    }

    #[test]
    fn test_color_from_rgba() {
        let c = Color::from_rgba_u8(255, 128, 0, 64);
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.g - 0.502).abs() < 0.01);
        assert!((c.b - 0.0).abs() < 0.01);
        assert!((c.a - 0.251).abs() < 0.01);
    }

    #[test]
    fn test_sorted_insertion() {
        let mut base = SkinObjectBase::default();
        base.add_destination(make_dst(100, 0.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(0, 0.0, 0.0, 50.0, 50.0, 255));
        base.add_destination(make_dst(50, 0.0, 0.0, 75.0, 75.0, 255));

        assert_eq!(base.destinations[0].time, 0);
        assert_eq!(base.destinations[1].time, 50);
        assert_eq!(base.destinations[2].time, 100);
    }

    #[test]
    fn test_ease_out() {
        let mut base = SkinObjectBase::default();
        let mut d1 = make_dst(0, 0.0, 0.0, 100.0, 100.0, 255);
        d1.acc = 2; // ease-out
        base.add_destination(d1);
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 255));

        let (r, _, _) = base.interpolate(50).unwrap();
        // Ease-out: rate = 1 - (0.5 - 1)^2 = 1 - 0.25 = 0.75, so x = 75
        assert!((r.x - 75.0).abs() < 0.001);
    }

    #[test]
    fn test_multi_keyframe() {
        let mut base = SkinObjectBase::default();
        base.add_destination(make_dst(0, 0.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(200, 200.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(300, 300.0, 0.0, 100.0, 100.0, 255));

        // t=50: between keyframe 0 and 1, halfway → x=50
        let (r, _, _) = base.interpolate(50).unwrap();
        assert!((r.x - 50.0).abs() < 0.001);

        // t=150: between keyframe 1 and 2, halfway → x=150
        let (r, _, _) = base.interpolate(150).unwrap();
        assert!((r.x - 150.0).abs() < 0.001);

        // t=250: between keyframe 2 and 3, halfway → x=250
        let (r, _, _) = base.interpolate(250).unwrap();
        assert!((r.x - 250.0).abs() < 0.001);
    }

    #[test]
    fn test_loop_boundary() {
        let mut base = SkinObjectBase::default();
        base.loop_time = 50;
        base.add_destination(make_dst(0, 0.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(50, 50.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 255));

        // t=150 with loop_time=50, end=100: (150-50) % (100-50) + 50 = 100%50+50 = 50
        let (r, _, _) = base.interpolate(150).unwrap();
        assert!((r.x - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_loop_equal_end() {
        let mut base = SkinObjectBase::default();
        base.loop_time = 100;
        base.add_destination(make_dst(0, 0.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 255));

        // When loop_time == end, should stay at loop_time value
        let (r, _, _) = base.interpolate(200).unwrap();
        assert!((r.x - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_play_once_exact_end() {
        let mut base = SkinObjectBase::default();
        base.loop_time = -1;
        base.add_destination(make_dst(0, 0.0, 0.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(100, 100.0, 0.0, 100.0, 100.0, 255));

        // At exact end time → Some (returns last keyframe)
        let result = base.interpolate(100);
        assert!(result.is_some());
        let (r, _, _) = result.unwrap();
        assert!((r.x - 100.0).abs() < 0.001);

        // Past end → None
        assert!(base.interpolate(101).is_none());
    }

    #[test]
    fn test_color_interpolation() {
        let mut base = SkinObjectBase::default();
        base.add_destination(Destination {
            time: 0,
            region: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 0.0,
            },
            angle: 0,
            acc: 0,
        });
        base.add_destination(Destination {
            time: 100,
            region: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color {
                r: 1.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
            angle: 0,
            acc: 0,
        });

        let (_, c, _) = base.interpolate(50).unwrap();
        assert!((c.r - 0.5).abs() < 0.001);
        assert!((c.g - 0.5).abs() < 0.001);
        assert!((c.b - 0.5).abs() < 0.001);
        assert!((c.a - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_angle_interpolation() {
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
            region: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::white(),
            angle: 360,
            acc: 0,
        });

        let (_, _, angle) = base.interpolate(50).unwrap();
        assert_eq!(angle, 180);
    }

    #[test]
    fn test_zero_duration_keyframe() {
        let mut base = SkinObjectBase::default();
        base.loop_time = -1; // play-once to avoid modulo wrap
        base.add_destination(make_dst(0, 10.0, 20.0, 100.0, 100.0, 255));
        base.add_destination(make_dst(0, 50.0, 60.0, 100.0, 100.0, 255));

        // Both keyframes at t=0 → should not panic (zero division)
        // Returns last keyframe since time >= last.time
        let result = base.interpolate(0);
        assert!(result.is_some());
        let (r, _, _) = result.unwrap();
        assert!((r.x - 50.0).abs() < 0.001);
    }
}

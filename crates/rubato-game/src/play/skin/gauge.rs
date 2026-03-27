use crate::play::groove_gauge::GrooveGauge;

/// Animation types
pub const ANIMATION_RANDOM: i32 = 0;
pub const ANIMATION_INCLEASE: i32 = 1;
pub const ANIMATION_DECLEASE: i32 = 2;
pub const ANIMATION_FLICKERING: i32 = 3;

/// Gauge skin object
pub struct SkinGauge {
    /// Animation type
    pub animation_type: i32,
    /// Animation range
    pub animation_range: i32,
    /// Animation interval (ms)
    pub duration: i64,
    /// Number of gauge parts
    pub parts: i32,
    /// Current animation frame
    animation: i32,
    /// Animation time
    atime: i64,
    /// Current gauge value
    value: f32,
    /// Current gauge type
    gauge_type: i32,
    /// Max value
    max: f32,
    /// Border value
    border: f32,
    /// Result mode start time (ms)
    pub starttime: i32,
    /// Result mode end time (ms)
    pub endtime: i32,
    /// Whether 7to9 border check is done.
    /// Note: the actual adjustment logic lives in rubato_skin::objects::skin_gauge::SkinGauge,
    /// which is the rendering-side gauge that corresponds to Java's SkinGauge.
    _is_checked_seven_to_nine: bool,
}

impl SkinGauge {
    pub fn new(parts: i32, animation_type: i32, animation_range: i32, duration: i64) -> Self {
        SkinGauge {
            animation_type,
            animation_range,
            duration,
            parts,
            animation: 0,
            atime: 0,
            value: 0.0,
            gauge_type: 0,
            max: 0.0,
            border: 0.0,
            starttime: 0,
            endtime: 500,
            _is_checked_seven_to_nine: false,
        }
    }

    pub fn prepare(&mut self, time: i64, gauge: Option<&GrooveGauge>) {
        let gauge = match gauge {
            Some(g) => g,
            None => return,
        };

        if self.animation_range < 0 || self.animation_range == i32::MAX || self.duration <= 0 {
            self.animation = 0;
        } else {
            match self.animation_type {
                ANIMATION_RANDOM => {
                    if self.atime < time {
                        self.animation = (rand_int() % (self.animation_range + 1) as u32) as i32;
                        self.atime = time + self.duration;
                    }
                }
                ANIMATION_INCLEASE => {
                    if self.atime < time {
                        self.animation =
                            (self.animation + self.animation_range) % (self.animation_range + 1);
                        self.atime = time + self.duration;
                    }
                }
                ANIMATION_DECLEASE => {
                    if self.atime < time {
                        self.animation = (self.animation + 1) % (self.animation_range + 1);
                        self.atime = time + self.duration;
                    }
                }
                ANIMATION_FLICKERING => {
                    self.animation = (time.rem_euclid(self.duration)) as i32;
                }
                _ => {}
            }
        }

        self.value = gauge.value();
        self.gauge_type = gauge.gauge_type();
        let g = gauge.gauge_by_type(self.gauge_type);
        let prop = g.property();
        self.max = prop.max;
        self.border = prop.border;
    }

    pub fn draw(&self) {
        // Drawing is handled by rubato_skin::skin_gauge::SkinGauge.
        // The skin-level SkinGauge holds SkinSourceImageSet and implements
        // the full gauge rendering logic (segmented bar with animation).
        // This play-side struct exists for standalone gauge state only.
    }

    pub fn animation_type(&self) -> i32 {
        self.animation_type
    }

    pub fn animation_range(&self) -> i32 {
        self.animation_range
    }

    pub fn duration(&self) -> i64 {
        self.duration
    }

    pub fn parts(&self) -> i32 {
        self.parts
    }

    pub fn dispose(&mut self) {
        // no GPU resources in Rust translation
    }
}

fn rand_int() -> u32 {
    // Simple pseudo-random for animation
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gauge(animation_type: i32, animation_range: i32, duration: i64) -> SkinGauge {
        SkinGauge::new(50, animation_type, animation_range, duration)
    }

    fn make_groove_gauge() -> GrooveGauge {
        use crate::play::groove_gauge::create_groove_gauge;
        use bms::model::bms_model::BMSModel;
        use bms::model::mode::Mode;
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        create_groove_gauge(&model, 2, 0, None).unwrap()
    }

    #[test]
    fn test_flickering_zero_duration_no_panic() {
        let gauge = make_groove_gauge();
        let mut g = make_gauge(ANIMATION_FLICKERING, 4, 0);
        g.prepare(1000, Some(&gauge));
        // duration <= 0 triggers the guard at line 64, setting animation to 0
        assert_eq!(g.animation, 0);
    }

    #[test]
    fn test_random_negative_animation_range_no_panic() {
        let gauge = make_groove_gauge();
        let mut g = make_gauge(ANIMATION_RANDOM, -1, 33);
        g.prepare(1000, Some(&gauge));
        // animation_range < 0 triggers the guard at line 64, setting animation to 0
        assert_eq!(g.animation, 0);
    }

    #[test]
    fn test_increase_negative_animation_range_no_panic() {
        let gauge = make_groove_gauge();
        let mut g = make_gauge(ANIMATION_INCLEASE, -1, 33);
        g.prepare(1000, Some(&gauge));
        // animation_range < 0 triggers the guard at line 64, setting animation to 0
        assert_eq!(g.animation, 0);
    }

    #[test]
    fn test_decrease_negative_animation_range_no_panic() {
        let gauge = make_groove_gauge();
        let mut g = make_gauge(ANIMATION_DECLEASE, -1, 33);
        g.prepare(1000, Some(&gauge));
        // animation_range < 0 triggers the guard at line 64, setting animation to 0
        assert_eq!(g.animation, 0);
    }

    #[test]
    fn test_flickering_negative_duration_no_panic() {
        let gauge = make_groove_gauge();
        let mut g = make_gauge(ANIMATION_FLICKERING, 4, -10);
        g.prepare(1000, Some(&gauge));
        // duration <= 0 triggers the guard at line 64, setting animation to 0
        assert_eq!(g.animation, 0);
    }

    #[test]
    fn test_normal_animation_none_gauge_no_update() {
        // When gauge is None, prepare() returns early without updating animation.
        // This matches the Java behavior: animation is only updated when gauge != null.
        let mut g = make_gauge(ANIMATION_FLICKERING, 4, 100);
        g.prepare(250, None);
        assert_eq!(g.animation, 0);
    }

    #[test]
    fn test_flickering_animation_with_gauge() {
        use crate::play::groove_gauge::create_groove_gauge;
        use bms::model::bms_model::BMSModel;
        use bms::model::mode::Mode;
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        let gauge = create_groove_gauge(&model, 2, 0, None).unwrap();

        let mut g = make_gauge(ANIMATION_FLICKERING, 4, 100);
        g.prepare(250, Some(&gauge));
        // 250 % 100 = 50
        assert_eq!(g.animation, 50);
    }

    /// Regression: ANIMATION_FLICKERING with negative time (practice mode time
    /// rewinding) must produce a non-negative animation value. Rust's `%`
    /// follows the dividend sign, so `(-7) % 100 == -7`. Using `rem_euclid`
    /// ensures the result is always in [0, duration).
    #[test]
    fn test_flickering_negative_time_produces_non_negative() {
        let gauge = make_groove_gauge();
        let mut g = make_gauge(ANIMATION_FLICKERING, 4, 100);

        for &t in &[-1i64, -7, -99, -100, -101, -500, -999] {
            g.prepare(t, Some(&gauge));
            assert!(
                g.animation >= 0,
                "animation must be non-negative for time={t}, got {}",
                g.animation
            );
            assert!(
                (g.animation as i64) < g.duration,
                "animation must be < duration for time={t}, got {}",
                g.animation
            );
        }
    }
}

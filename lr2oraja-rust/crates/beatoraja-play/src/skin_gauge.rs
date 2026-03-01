use crate::groove_gauge::GrooveGauge;

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
    /// Whether 7to9 border check is done
    is_checked_seven_to_nine: bool,
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
            is_checked_seven_to_nine: false,
        }
    }

    pub fn prepare(&mut self, time: i64, gauge: Option<&GrooveGauge>) {
        let gauge = match gauge {
            Some(g) => g,
            None => return,
        };

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
                self.animation = (time % self.duration) as i32;
            }
            _ => {}
        }

        self.value = gauge.get_value();
        self.gauge_type = gauge.get_type();
        let g = gauge.get_gauge_by_type(self.gauge_type);
        let prop = g.get_property();
        self.max = prop.max;
        self.border = prop.border;
    }

    pub fn draw(&self) {
        // Drawing is handled by beatoraja_skin::skin_gauge::SkinGauge.
        // The skin-level SkinGauge holds SkinSourceImageSet and implements
        // the full gauge rendering logic (segmented bar with animation).
        // This play-side struct exists for standalone gauge state only.
    }

    pub fn get_animation_type(&self) -> i32 {
        self.animation_type
    }

    pub fn set_animation_type(&mut self, animation_type: i32) {
        self.animation_type = animation_type;
    }

    pub fn get_animation_range(&self) -> i32 {
        self.animation_range
    }

    pub fn set_animation_range(&mut self, animation_range: i32) {
        self.animation_range = animation_range;
    }

    pub fn get_duration(&self) -> i64 {
        self.duration
    }

    pub fn set_duration(&mut self, duration: i64) {
        self.duration = duration;
    }

    pub fn get_parts(&self) -> i32 {
        self.parts
    }

    pub fn set_parts(&mut self, parts: i32) {
        self.parts = parts;
    }

    pub fn set_starttime(&mut self, starttime: i32) {
        self.starttime = starttime;
    }

    pub fn set_endtime(&mut self, endtime: i32) {
        self.endtime = endtime;
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

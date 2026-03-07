/// Hidden cover skin object (play-side state)
///
/// Tracks hidden/lift cover state for game logic (note visibility).
/// The skin-side SkinHidden in beatoraja-skin handles full rendering
/// with MainState and OFFSET_LIFT integration.
pub struct SkinHidden {
    /// Disappear line y-coordinate (skin setting value)
    /// Trim below this coordinate. Negative means no trimming.
    disapear_line: f32,
    /// Disappear line y-coordinate (computed, with lift)
    disapear_line_added_lift: f32,
    /// Whether disappear line is linked to lift
    pub is_disapear_line_link_lift: bool,
    _previous_y: f32,
    previous_lift: f32,
    _timer: i32,
    cycle: i32,
    image_index: usize,
    image_count: usize,
}

impl SkinHidden {
    pub fn new(image_count: usize, timer: i32, cycle: i32) -> Self {
        SkinHidden {
            disapear_line: -1.0,
            disapear_line_added_lift: -1.0,
            is_disapear_line_link_lift: true,
            _previous_y: f32::MIN,
            previous_lift: f32::MIN,
            _timer: timer,
            cycle,
            image_index: 0,
            image_count,
        }
    }

    /// Translated from: Java SkinHidden.prepare(long time, MainState state)
    ///
    /// The `lift_y` parameter corresponds to `state.getOffsetValue(OFFSET_LIFT).y`
    /// in Java. The caller should extract this from the skin offset system and
    /// pass it here.
    pub fn prepare(&mut self, time: i64, lift_y: Option<f32>) {
        // Update disappear line with lift offset
        if self.is_disapear_line_link_lift
            && self.disapear_line >= 0.0
            && let Some(y) = lift_y
            && self.previous_lift != y
        {
            self.disapear_line_added_lift = self.disapear_line + y;
            self.previous_lift = y;
        }

        self.image_index = self.image_index(self.image_count, time);
    }

    pub fn draw(&self) {
        // Drawing is handled by rubato_skin::skin_hidden::SkinHidden.
        // The skin-level SkinHidden holds TextureRegion arrays and implements
        // the full hidden cover rendering logic (trimming at disappear line).
        // This play-side struct exists for standalone hidden state only.
    }

    fn image_index(&self, length: usize, time: i64) -> usize {
        if self.cycle <= 0 {
            return 0;
        }
        if length == 0 {
            return 0;
        }
        if time < 0 {
            return 0;
        }
        // Reduce magnitude first to avoid overflow in time * length
        let t = (time % self.cycle as i64) as usize;
        (t * length / self.cycle as usize) % length
    }

    pub fn disapear_line(&self) -> f32 {
        self.disapear_line
    }

    pub fn disapear_line_added_lift(&self) -> f32 {
        self.disapear_line_added_lift
    }

    pub fn set_disapear_line(&mut self, disapear_line: f32) {
        self.disapear_line = disapear_line;
        self.disapear_line_added_lift = disapear_line;
    }

    pub fn is_disapear_line_link_lift(&self) -> bool {
        self.is_disapear_line_link_lift
    }

    pub fn image_index_value(&self) -> usize {
        self.image_index
    }

    pub fn dispose(&mut self) {
        // no GPU resources in Rust translation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let h = SkinHidden::new(3, 0, 100);
        assert_eq!(h.image_count, 3);
        assert_eq!(h.disapear_line, -1.0);
        assert!(h.is_disapear_line_link_lift);
        assert_eq!(h.image_index, 0);
    }

    #[test]
    fn test_prepare_no_lift() {
        let mut h = SkinHidden::new(4, 0, 1000);
        h.prepare(250, None);
        // 250 * 4 / 1000 = 1
        assert_eq!(h.image_index, 1);
    }

    #[test]
    fn test_prepare_with_lift() {
        let mut h = SkinHidden::new(2, 0, 100);
        h.set_disapear_line(300.0);
        h.prepare(50, Some(10.0));
        assert_eq!(h.disapear_line_added_lift, 310.0);
        assert_eq!(h.previous_lift, 10.0);
    }

    #[test]
    fn test_prepare_lift_no_change() {
        let mut h = SkinHidden::new(2, 0, 100);
        h.set_disapear_line(300.0);
        h.prepare(50, Some(10.0));
        assert_eq!(h.disapear_line_added_lift, 310.0);
        // Same lift value → no update
        h.prepare(60, Some(10.0));
        assert_eq!(h.disapear_line_added_lift, 310.0);
    }

    #[test]
    fn test_prepare_lift_disabled() {
        let mut h = SkinHidden::new(2, 0, 100);
        h.set_disapear_line(300.0);
        h.is_disapear_line_link_lift = false;
        h.prepare(50, Some(10.0));
        // Lift disabled → disapear_line_added_lift stays at initial value
        assert_eq!(h.disapear_line_added_lift, 300.0);
    }

    #[test]
    fn test_prepare_negative_disapear_line() {
        let mut h = SkinHidden::new(2, 0, 100);
        // disapear_line defaults to -1.0 (no trimming)
        h.prepare(50, Some(10.0));
        // Negative disapear_line → lift not applied
        assert_eq!(h.disapear_line_added_lift, -1.0);
    }

    #[test]
    fn test_image_index_zero_cycle() {
        let mut h = SkinHidden::new(4, 0, 0);
        h.prepare(500, None);
        assert_eq!(h.image_index, 0);
    }

    #[test]
    fn test_image_index_zero_count() {
        let mut h = SkinHidden::new(0, 0, 100);
        h.prepare(500, None);
        assert_eq!(h.image_index, 0);
    }

    #[test]
    fn test_image_index_negative_time() {
        let mut h = SkinHidden::new(4, 0, 1000);
        h.prepare(-100, None);
        assert_eq!(h.image_index, 0);
    }

    #[test]
    fn test_image_index_large_time_no_overflow() {
        let mut h = SkinHidden::new(4, 0, 1000);
        // Large time value that would overflow if multiplied by length directly
        h.prepare(i64::MAX / 2, None);
        // Should not panic; result should be valid index
        assert!(h.image_index < 4);
    }

    #[test]
    fn test_image_index_negative_cycle() {
        let mut h = SkinHidden::new(4, 0, -100);
        h.prepare(500, None);
        assert_eq!(h.image_index, 0);
    }

    #[test]
    fn test_set_disapear_line() {
        let mut h = SkinHidden::new(2, 0, 100);
        h.set_disapear_line(500.0);
        assert_eq!(h.disapear_line(), 500.0);
        // set_disapear_line also sets disapear_line_added_lift
        assert_eq!(h.disapear_line_added_lift(), 500.0);
    }

    #[test]
    fn test_dispose() {
        let mut h = SkinHidden::new(2, 0, 100);
        h.dispose(); // no-op, should not panic
    }
}

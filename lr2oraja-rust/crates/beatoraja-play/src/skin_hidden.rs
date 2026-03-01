/// Hidden cover skin object
pub struct SkinHidden {
    /// Disappear line y-coordinate (skin setting value)
    /// Trim below this coordinate. Negative means no trimming.
    disapear_line: f32,
    /// Disappear line y-coordinate (computed, with lift)
    disapear_line_added_lift: f32,
    /// Whether disappear line is linked to lift
    is_disapear_line_link_lift: bool,
    previous_y: f32,
    previous_lift: f32,
    timer: i32,
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
            previous_y: f32::MIN,
            previous_lift: f32::MIN,
            timer,
            cycle,
            image_index: 0,
            image_count,
        }
    }

    pub fn prepare(&mut self, time: i64) {
        // TODO: Phase 7+ dependency - requires MainState, OFFSET_LIFT
        self.image_index = self.get_image_index(self.image_count, time);
    }

    pub fn draw(&self) {
        // Drawing is handled by beatoraja_skin::skin_hidden::SkinHidden.
        // The skin-level SkinHidden holds TextureRegion arrays and implements
        // the full hidden cover rendering logic (trimming at disappear line).
        // This play-side struct exists for standalone hidden state only.
    }

    fn get_image_index(&self, length: usize, time: i64) -> usize {
        if self.cycle == 0 {
            return 0;
        }
        if length == 0 {
            return 0;
        }
        // TODO: Phase 7+ dependency - timer property offset
        if time < 0 {
            return 0;
        }
        ((time as usize * length / self.cycle as usize) % length)
    }

    pub fn get_disapear_line(&self) -> f32 {
        self.disapear_line
    }

    pub fn set_disapear_line(&mut self, disapear_line: f32) {
        self.disapear_line = disapear_line;
    }

    pub fn is_disapear_line_link_lift(&self) -> bool {
        self.is_disapear_line_link_lift
    }

    pub fn set_disapear_line_link_lift(&mut self, value: bool) {
        self.is_disapear_line_link_lift = value;
    }

    pub fn dispose(&mut self) {
        // no GPU resources in Rust translation
    }
}

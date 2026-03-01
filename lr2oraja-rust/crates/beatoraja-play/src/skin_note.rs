/// Note skin object
pub struct SkinNote {
    lanes: Vec<SkinLane>,
    time: i64,
}

/// Per-lane note rendering data
pub struct SkinLane {
    /// Note image present
    pub has_note: bool,
    /// Long note types (10 types)
    pub has_longnote: [bool; 10],
    /// Mine note image present
    pub has_minenote: bool,
    /// Hidden note image present
    pub has_hiddennote: bool,
    /// Processed note image present
    pub has_processednote: bool,
    /// Lane scale
    pub scale: f32,
    /// dstnote2 value for PMS miss POOR rendering
    pub dstnote2: i32,
    /// Region
    pub region_x: f32,
    pub region_y: f32,
    pub region_width: f32,
    pub region_height: f32,
}

impl Default for SkinLane {
    fn default() -> Self {
        Self::new()
    }
}

impl SkinLane {
    pub fn new() -> Self {
        SkinLane {
            has_note: true,
            has_longnote: [true; 10],
            has_minenote: true,
            has_hiddennote: true,
            has_processednote: true,
            scale: 1.0,
            dstnote2: i32::MIN,
            region_x: 0.0,
            region_y: 0.0,
            region_width: 0.0,
            region_height: 0.0,
        }
    }

    pub fn prepare(&mut self, _time: i64) {
        // In Java, SkinLane.prepare() fetches current images from SkinSource
        // for each note type (note, longnote, mine, hidden, processed).
        // In Rust, image fetching is handled by the skin-side SkinNoteObject
        // which assembles DrawCommand vectors via LaneRenderer.draw_lane().
        // The play-side SkinLane only tracks boolean flags and region data.
    }

    pub fn draw(&self) {
        // stub - drawing is handled by LaneRenderer.drawLane()
    }

    pub fn dispose(&mut self) {
        // no GPU resources in Rust translation
    }
}

impl SkinNote {
    pub fn new(lane_count: usize) -> Self {
        let mut lanes = Vec::with_capacity(lane_count);
        for _ in 0..lane_count {
            lanes.push(SkinLane::new());
        }
        SkinNote { lanes, time: 0 }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_lane_region(
        &mut self,
        index: usize,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        scale: f32,
        dstnote2: i32,
    ) {
        if index < self.lanes.len() {
            self.lanes[index].region_x = x;
            self.lanes[index].region_y = y;
            self.lanes[index].region_width = width;
            self.lanes[index].region_height = height;
            self.lanes[index].scale = scale;
            self.lanes[index].dstnote2 = dstnote2;
        }
    }

    pub fn get_lanes(&self) -> &[SkinLane] {
        &self.lanes
    }

    pub fn get_lanes_mut(&mut self) -> &mut [SkinLane] {
        &mut self.lanes
    }

    pub fn prepare(&mut self, time: i64) {
        self.time = time;
        for lane in &mut self.lanes {
            lane.prepare(time);
        }
    }

    pub fn draw(&self) {
        // Drawing is handled by SkinNoteObject in beatoraja-skin.
        // In Java, draw() calls renderer.drawLane(sprite, time, lanes, offsets).
        // In Rust, LaneRenderer.draw_lane() returns DrawCommand vectors,
        // which are set on SkinNoteObject and executed during its draw().
    }

    pub fn dispose(&mut self) {
        for lane in &mut self.lanes {
            lane.dispose();
        }
    }
}

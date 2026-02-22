use beatoraja_types::play_config::PlayConfig;
use bms_model::bms_model::BMSModel;
use std::collections::HashMap;

/// Lane renderer
pub struct LaneRenderer {
    basehispeed: f32,
    hispeedmargin: f32,
    /// Filtered timeline indices (indexes into BMSModel.get_all_time_lines())
    timeline_indices: Vec<usize>,
    pos: usize,
    currentduration: i32,
    basebpm: f64,
    nowbpm: f64,
    mainbpm: f64,
    minbpm: f64,
    maxbpm: f64,
    /// PMS rhythm-based note expansion time (quarter note to max)
    note_expansion_time: f32,
    /// PMS rhythm-based note contraction time (max to normal)
    note_contraction_time: f32,
    // PlayConfig fields (stubbed inline)
    hispeed: f32,
    duration: i32,
    lanecover: f32,
    lift: f32,
    hidden: f32,
    enable_lanecover: bool,
    enable_lift: bool,
    enable_hidden: bool,
    enable_constant: bool,
    constant_fadein_time: f32,
    fixhispeed: i32,
}

/// Fix hispeed modes
pub const FIX_HISPEED_OFF: i32 = 0;
pub const FIX_HISPEED_STARTBPM: i32 = 1;
pub const FIX_HISPEED_MINBPM: i32 = 2;
pub const FIX_HISPEED_MAXBPM: i32 = 3;
pub const FIX_HISPEED_MAINBPM: i32 = 4;

impl LaneRenderer {
    pub fn new(model: &BMSModel) -> Self {
        let mut renderer = LaneRenderer {
            basehispeed: 1.0,
            hispeedmargin: 0.25,
            timeline_indices: Vec::new(),
            pos: 0,
            currentduration: 0,
            basebpm: 0.0,
            nowbpm: 0.0,
            mainbpm: 0.0,
            minbpm: 0.0,
            maxbpm: 0.0,
            note_expansion_time: 9.0,
            note_contraction_time: 150.0,
            hispeed: 1.0,
            duration: 500,
            lanecover: 0.0,
            lift: 0.0,
            hidden: 0.0,
            enable_lanecover: false,
            enable_lift: false,
            enable_hidden: false,
            enable_constant: false,
            constant_fadein_time: 0.0,
            fixhispeed: FIX_HISPEED_OFF,
        };
        renderer.init(model);
        renderer
    }

    pub fn init(&mut self, model: &BMSModel) {
        self.pos = 0;
        let all_tls = model.get_all_time_lines();
        let mut indices: Vec<usize> = Vec::new();
        let mut cbpm = model.get_bpm();
        let mut cscr = 1.0;
        for (i, tl) in all_tls.iter().enumerate() {
            if cbpm != tl.get_bpm()
                || tl.get_stop() > 0
                || cscr != tl.get_scroll()
                || tl.get_section_line()
                || tl.exist_note()
                || tl.exist_hidden_note()
            {
                indices.push(i);
            }
            cbpm = tl.get_bpm();
            cscr = tl.get_scroll();
        }
        self.timeline_indices = indices;

        self.minbpm = model.get_min_bpm();
        self.maxbpm = model.get_max_bpm();

        // Find main BPM (BPM with most notes)
        let mut bpm_counts: HashMap<u64, (f64, i32)> = HashMap::new();
        for tl in all_tls {
            let key = tl.get_bpm().to_bits();
            let entry = bpm_counts.entry(key).or_insert((tl.get_bpm(), 0));
            entry.1 += tl.get_total_notes();
        }
        let mut maxcount = 0;
        for (bpm, count) in bpm_counts.values() {
            if *count > maxcount {
                maxcount = *count;
                self.mainbpm = *bpm;
            }
        }

        self.basebpm = match self.fixhispeed {
            FIX_HISPEED_OFF => self.basebpm,
            FIX_HISPEED_STARTBPM => model.get_bpm(),
            FIX_HISPEED_MINBPM => self.minbpm,
            FIX_HISPEED_MAXBPM => self.maxbpm,
            FIX_HISPEED_MAINBPM => self.mainbpm,
            _ => self.basebpm,
        };

        self.set_lanecover(self.lanecover);
        if self.fixhispeed != FIX_HISPEED_OFF {
            self.basehispeed = self.hispeed;
        }
    }

    pub fn get_hispeed(&self) -> f32 {
        self.hispeed
    }

    pub fn get_duration(&self) -> i32 {
        self.duration
    }

    pub fn set_duration(&mut self, gvalue: i32) {
        self.duration = if gvalue < 1 { 1 } else { gvalue };
        self.set_lanecover(self.lanecover);
    }

    pub fn get_current_duration(&self) -> i32 {
        self.currentduration
    }

    pub fn get_hispeedmargin(&self) -> f32 {
        self.hispeedmargin
    }

    pub fn set_hispeedmargin(&mut self, hispeedmargin: f32) {
        self.hispeedmargin = hispeedmargin;
    }

    pub fn is_enable_lift(&self) -> bool {
        self.enable_lift
    }

    pub fn get_lift_region(&self) -> f32 {
        self.lift
    }

    pub fn set_lift_region(&mut self, lift_region: f32) {
        self.lift = lift_region.clamp(0.0, 1.0);
    }

    pub fn get_lanecover(&self) -> f32 {
        self.lanecover
    }

    pub fn reset_hispeed(&mut self, target_bpm: f64) {
        if self.fixhispeed != FIX_HISPEED_OFF {
            let lc = if self.enable_lanecover {
                self.lanecover
            } else {
                0.0
            };
            self.hispeed =
                ((2400.0 / (target_bpm / 100.0) / self.duration as f64) * (1.0 - lc as f64)) as f32;
        }
    }

    pub fn set_lanecover(&mut self, lanecover: f32) {
        self.lanecover = lanecover.clamp(0.0, 1.0);
        let basebpm = self.basebpm;
        self.reset_hispeed(basebpm);
    }

    pub fn set_enable_lanecover(&mut self, b: bool) {
        self.enable_lanecover = b;
    }

    pub fn is_enable_lanecover(&self) -> bool {
        self.enable_lanecover
    }

    pub fn get_hidden_cover(&self) -> f32 {
        self.hidden
    }

    pub fn set_hidden_cover(&mut self, hidden_cover: f32) {
        self.hidden = hidden_cover.clamp(0.0, 1.0);
    }

    pub fn set_enable_hidden(&mut self, b: bool) {
        self.enable_hidden = b;
    }

    pub fn is_enable_hidden(&self) -> bool {
        self.enable_hidden
    }

    pub fn change_hispeed(&mut self, b: bool) {
        let f = if self.fixhispeed != FIX_HISPEED_OFF {
            self.basehispeed * self.hispeedmargin * if b { 1.0 } else { -1.0 }
        } else {
            self.hispeedmargin * if b { 1.0 } else { -1.0 }
        };
        if self.hispeed + f > 0.0 && self.hispeed + f < 20.0 {
            self.hispeed += f;
        }
    }

    pub fn draw_lane(&mut self) {
        // TODO: Phase 7+ dependency - requires SkinObjectRenderer, SkinLane, SkinOffset
        // This is the main lane drawing method (713 lines in Java)
        // It handles: section lines, note rendering, LN rendering, PMS miss POOR,
        // note expansion, constant mode, judge area display, etc.
    }

    pub fn get_now_bpm(&self) -> f64 {
        self.nowbpm
    }

    pub fn get_min_bpm(&self) -> f64 {
        self.minbpm
    }

    pub fn get_max_bpm(&self) -> f64 {
        self.maxbpm
    }

    pub fn get_main_bpm(&self) -> f64 {
        self.mainbpm
    }

    pub fn get_play_config(&self) -> PlayConfig {
        // Return a PlayConfig snapshot reflecting current renderer state.
        // In Java, LaneRenderer holds a PlayConfig reference and delegates to it.
        PlayConfig {
            hispeed: self.hispeed,
            duration: self.duration,
            enable_constant: self.enable_constant,
            constant_fadein_time: self.constant_fadein_time as i32,
            fixhispeed: self.fixhispeed,
            hispeedmargin: self.hispeedmargin,
            lanecover: self.lanecover,
            enablelanecover: self.enable_lanecover,
            lift: self.lift,
            enablelift: self.enable_lift,
            hidden: self.hidden,
            enablehidden: self.enable_hidden,
            ..PlayConfig::default()
        }
    }

    /// Draw long note (CN/HCN/LN).
    /// Corresponds to Java drawLongNote() private method.
    fn draw_long_note(&self) {
        // TODO: Phase 7+ dependency - requires SkinObjectRenderer, SkinLane, TextureRegion
        // In Java, this is a private 100+ line method that handles HCN/CN/LN drawing
        // with texture regions, scroll calculation, and note expansion.
    }

    pub fn dispose(&mut self) {
        // no GPU resources in Rust translation
    }
}

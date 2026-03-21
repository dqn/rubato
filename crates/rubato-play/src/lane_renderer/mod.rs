use bms_model::bms_model::{BMSModel, LNTYPE_CHARGENOTE, LNTYPE_HELLCHARGENOTE, LNTYPE_LONGNOTE};
use bms_model::note::{Note, TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_LONGNOTE, TYPE_UNDEFINED};
use bms_model::time_line::TimeLine;
use rubato_types::play_config::{
    FIX_HISPEED_MAINBPM, FIX_HISPEED_MAXBPM, FIX_HISPEED_MINBPM, FIX_HISPEED_OFF,
    FIX_HISPEED_STARTBPM, HISPEED_MAX, HISPEED_MIN, PlayConfig,
};
use std::collections::HashMap;

use crate::skin::note::SkinLane;

/// Draw command types emitted by draw_lane().
/// These represent the rendering operations that the caller must execute
/// using whatever rendering backend is available (SkinObjectRenderer, etc.).
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    /// Set color (RGBA)
    SetColor { r: f32, g: f32, b: f32, a: f32 },
    /// Set blend mode
    SetBlend(i32),
    /// Set renderer type
    SetType(i32),
    /// Draw a note image at position
    DrawNote {
        lane: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        /// Which image to draw: Normal, Processed, Mine, Hidden
        image_type: NoteImageType,
    },
    /// Draw a long note body/start/end
    DrawLongNote {
        lane: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        /// Index into the longImage array (0-9)
        image_index: usize,
    },
    /// Draw section line (delegates to skin line images)
    DrawSectionLine { y_offset: i32 },
    /// Draw timeline display (practice mode)
    DrawTimeLine { y_offset: i32 },
    /// Draw BPM change line
    DrawBpmLine { y_offset: i32, bpm: f64 },
    /// Draw stop line
    DrawStopLine { y_offset: i32, stop_ms: i64 },
    /// Draw timeline text (time display in practice mode)
    DrawTimeText { text: String, x: f32, y: f32 },
    /// Draw BPM text
    DrawBpmText { text: String, x: f32, y: f32 },
    /// Draw stop text
    DrawStopText { text: String, x: f32, y: f32 },
    /// Draw judge area (colored rectangles)
    DrawJudgeArea {
        lane: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color_index: usize,
    },
}

/// Note image types for DrawNote command
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteImageType {
    Normal,
    Processed,
    Mine,
    Hidden,
}

/// A borrowed slice of `TimeLine` values stored as a raw pointer + length.
///
/// This exists so that `DrawLaneContext` can be `'static` (required by
/// `Box<dyn Any>`) without an `unsafe transmute` at every call site.
/// The single unsafe reconstruction is confined to [`TimelinesRef::as_slice`].
///
/// # Safety contract
/// The caller that creates a `TimelinesRef` must guarantee the source slice
/// outlives the `DrawLaneContext` that contains it.  In practice the slice
/// comes from `BMSPlayer.model.timelines` and the context is consumed
/// synchronously within the same `render_skin_impl` call.
#[derive(Clone, Copy)]
pub struct TimelinesRef {
    ptr: *const TimeLine,
    len: usize,
}

// Safety: TimeLine is Send, and the owner guarantees the pointee outlives this handle.
unsafe impl Send for TimelinesRef {}
// Safety: TimeLine is Sync, and we only hand out shared references.
unsafe impl Sync for TimelinesRef {}

impl TimelinesRef {
    /// Create a new `TimelinesRef` from a slice.
    ///
    /// # Safety
    /// The caller must ensure the slice outlives every use of the returned handle.
    pub unsafe fn from_slice(slice: &[TimeLine]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
        }
    }

    /// Reconstruct the shared slice.
    ///
    /// # Safety
    /// Only valid while the original slice is alive (guaranteed by the contract
    /// on [`TimelinesRef::from_slice`]).
    pub unsafe fn as_slice(&self) -> &[TimeLine] {
        if self.len == 0 {
            &[]
        } else {
            // Safety: caller guarantees the source slice is still alive.
            unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
        }
    }
}

/// External state required by draw_lane() that comes from BMSPlayer and other subsystems.
/// This avoids coupling LaneRenderer to BMSPlayer directly.
pub struct DrawLaneContext {
    /// Current global time (ms)
    pub time: i64,
    /// Whether TIMER_PLAY is active and its value
    pub timer_play: Option<i64>,
    /// Whether timer 141 is active and its value
    pub timer_141: Option<i64>,
    /// Judge timing offset from config
    pub judge_timing: i64,
    /// Current player state (STATE_PRACTICE, etc.)
    pub is_practice: bool,
    /// Practice mode start time (only used when is_practice)
    pub practice_start_time: i64,
    /// Current time from timer.getNowTime()
    pub now_time: i64,
    /// Quarter note timing for PMS expansion
    pub now_quarter_note_time: i64,
    /// Note expansion rates [width%, height%] from PlaySkin
    pub note_expansion_rate: [i32; 2],
    /// Lane group regions (for font rendering positions)
    pub lane_group_regions: Vec<LaneGroupRegion>,
    /// Whether to show BPM guide lines
    pub show_bpmguide: bool,
    /// Whether to show past notes
    pub show_pastnote: bool,
    /// Whether to mark processed notes
    pub mark_processednote: bool,
    /// Whether to show hidden notes
    pub show_hiddennote: bool,
    /// Whether to show judge area
    pub show_judgearea: bool,
    /// LN type from model
    pub lntype: bms_model::bms_model::LnType,
    /// Judge time regions per lane (5 judge levels, [start, end])
    pub judge_time_regions: Vec<Vec<[i64; 2]>>,
    /// Processing long note per lane (timeline Vec index of the LN end note, if actively pressing).
    /// Converted from JudgeNote index to timeline index at construction time.
    pub processing_long_notes: Vec<Option<usize>>,
    /// Passing long note per lane (timeline Vec index of the HCN start note, if passing through).
    /// Converted from JudgeNote index to timeline index at construction time.
    pub passing_long_notes: Vec<Option<usize>>,
    /// Hell charge judge per lane
    pub hell_charge_judges: Vec<bool>,
    /// Judge table (for PMS miss POOR bad time)
    pub bad_judge_time: i64,
    /// Model's initial BPM
    pub model_bpm: f64,
    /// All timelines from the model (the full array, not filtered).
    /// Stored as a raw-pointer handle to avoid `'static` transmute at call sites.
    /// Safety: the source slice must outlive this context (see [`TimelinesRef`]).
    pub all_timelines: TimelinesRef,
    /// Whether to force CN endings display
    pub forced_cn_endings: bool,
}

/// Simplified lane group region for text positioning
#[derive(Clone, Debug)]
pub struct LaneGroupRegion {
    pub x: f32,
    pub width: f32,
}

/// Lane renderer
pub struct LaneRenderer {
    basehispeed: f32,
    pub hispeedmargin: f32,
    /// Filtered timeline indices (indexes into BMSModel.timelines)
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
    pub enable_lanecover: bool,
    pub enable_lift: bool,
    pub enable_hidden: bool,
    enable_constant: bool,
    constant_fadein_time: f32,
    fixhispeed: i32,
}

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
        let all_tls = &model.timelines;
        let mut indices: Vec<usize> = Vec::new();
        let mut cbpm = model.bpm;
        let mut cscr = 1.0;
        for (i, tl) in all_tls.iter().enumerate() {
            if cbpm != tl.bpm
                || tl.stop() > 0
                || cscr != tl.scroll
                || tl.section_line
                || tl.exist_note()
                || tl.exist_hidden_note()
            {
                indices.push(i);
            }
            cbpm = tl.bpm;
            cscr = tl.scroll;
        }
        self.timeline_indices = indices;

        self.minbpm = model.min_bpm();
        self.maxbpm = model.max_bpm();

        // Fall back to the chart's declared BPM when no timelines carry notes,
        // so FIX_HISPEED_MAINBPM never operates on the 0.0 default.
        self.mainbpm = model.bpm;

        // Find main BPM (BPM with most notes)
        let mut bpm_counts: HashMap<u64, (f64, i32)> = HashMap::new();
        for tl in all_tls {
            let key = tl.bpm.to_bits();
            let entry = bpm_counts.entry(key).or_insert((tl.bpm, 0));
            entry.1 += tl.total_notes();
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
            FIX_HISPEED_STARTBPM => model.bpm,
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

    pub fn hispeed(&self) -> f32 {
        self.hispeed
    }

    pub fn duration(&self) -> i32 {
        self.duration
    }

    pub fn set_duration(&mut self, gvalue: i32) {
        self.duration = if gvalue < 1 { 1 } else { gvalue };
        self.set_lanecover(self.lanecover);
    }

    pub fn current_duration(&self) -> i32 {
        self.currentduration
    }

    pub fn hispeedmargin(&self) -> f32 {
        self.hispeedmargin
    }

    pub fn is_enable_lift(&self) -> bool {
        self.enable_lift
    }

    pub fn lift_region(&self) -> f32 {
        self.lift
    }

    pub fn set_lift_region(&mut self, lift_region: f32) {
        self.lift = lift_region.clamp(0.0, 1.0);
    }

    pub fn lanecover(&self) -> f32 {
        self.lanecover
    }

    pub fn reset_hispeed(&mut self, target_bpm: f64) {
        if self.duration == 0 {
            return;
        }
        if self.fixhispeed != FIX_HISPEED_OFF && target_bpm != 0.0 {
            let lc = if self.enable_lanecover {
                self.lanecover
            } else {
                0.0
            };
            self.hispeed =
                ((2400.0 / (target_bpm / 100.0) / self.duration as f64) * (1.0 - lc as f64)) as f32;
            self.hispeed = self.hispeed.clamp(HISPEED_MIN, HISPEED_MAX);
        }
    }

    /// Apply fields from an external PlayConfig to this renderer.
    /// This is the reverse of `play_config()` which extracts a snapshot.
    /// Used to propagate initial config during create().
    pub fn apply_play_config(&mut self, pc: &PlayConfig) {
        self.hispeed = pc.hispeed;
        self.duration = pc.duration;
        self.lanecover = pc.lanecover;
        self.lift = pc.lift;
        self.hidden = pc.hidden;
        self.enable_lanecover = pc.enablelanecover;
        self.enable_lift = pc.enablelift;
        self.enable_hidden = pc.enablehidden;
        self.enable_constant = pc.enable_constant;
        self.constant_fadein_time = pc.constant_fadein_time as f32;
        self.fixhispeed = pc.fixhispeed;
        self.hispeedmargin = pc.hispeedmargin;
    }

    /// Apply only modmenu-managed fields from an external PlayConfig.
    /// Preserves hispeed, duration, fixhispeed, and hispeedmargin which may
    /// have been changed during gameplay via scroll keys / ControlInputProcessor.
    pub fn apply_modmenu_fields(&mut self, pc: &PlayConfig) {
        self.lanecover = pc.lanecover;
        self.lift = pc.lift;
        self.hidden = pc.hidden;
        self.enable_lanecover = pc.enablelanecover;
        self.enable_lift = pc.enablelift;
        self.enable_hidden = pc.enablehidden;
        self.enable_constant = pc.enable_constant;
        self.constant_fadein_time = pc.constant_fadein_time as f32;
    }

    pub fn set_lanecover(&mut self, lanecover: f32) {
        self.lanecover = lanecover.clamp(0.0, 1.0);
        let basebpm = self.basebpm;
        self.reset_hispeed(basebpm);
    }

    pub fn is_enable_lanecover(&self) -> bool {
        self.enable_lanecover
    }

    pub fn hidden_cover(&self) -> f32 {
        self.hidden
    }

    pub fn set_hidden_cover(&mut self, hidden_cover: f32) {
        self.hidden = hidden_cover.clamp(0.0, 1.0);
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
        // Java parity: uses strict inequality (> 0.0, < 20.0) unlike reset_hispeed's
        // clamp(HISPEED_MIN, HISPEED_MAX). In Java, changeHispeed() guards with
        // `hispeed + f > 0 && hispeed + f < 20` while setHispeed() (called from
        // resetHispeed) clamps to [HISPEED_MIN, HISPEED_MAX].
        if self.hispeed + f > 0.0 && self.hispeed + f < 20.0 {
            self.hispeed += f;
        }
    }

    /// Calculate the y-position offset for a timeline relative to a previous timeline.
    /// This is the core scroll position calculation used throughout draw_lane().
    ///
    /// Java: y += (tl.getSection() - prevtl.getSection()) * prevtl.getScroll() * ...
    pub fn calc_y_offset(tl: &TimeLine, prev_tl: &TimeLine, microtime: i64, rxhs: f64) -> f64 {
        if prev_tl.micro_time() + prev_tl.micro_stop() > microtime {
            // During a stop: full section distance
            (tl.section() - prev_tl.section()) * prev_tl.scroll * rxhs
        } else {
            // Normal scrolling: proportional to time remaining
            let time_diff = tl.micro_time() - microtime;
            let total_time = tl.micro_time() - prev_tl.micro_time() - prev_tl.micro_stop();
            if total_time == 0 {
                0.0
            } else {
                (tl.section() - prev_tl.section())
                    * prev_tl.scroll
                    * (time_diff as f64 / total_time as f64)
                    * rxhs
            }
        }
    }

    /// Calculate the y-position for the first timeline (no previous timeline).
    pub fn calc_y_offset_first(tl: &TimeLine, microtime: i64, rxhs: f64) -> f64 {
        if tl.micro_time() == 0 {
            0.0
        } else {
            tl.section() * (tl.micro_time() - microtime) as f64 / tl.micro_time() as f64 * rxhs
        }
    }

    /// Calculate constant mode alpha for a timeline.
    /// Returns None if the note should be hidden, Some(alpha) otherwise.
    pub fn calc_constant_alpha(
        tl_microtime: i64,
        microtime: i64,
        base_duration: i32,
        alpha_limit: f32,
    ) -> Option<f32> {
        let target_time = microtime + (base_duration as i64 * 1000);
        let time_difference = tl_microtime - target_time;

        if alpha_limit == 0.0 {
            // Instantaneous transition: notes at or past target are hidden, others fully visible
            if time_difference >= 0 {
                return None;
            } else {
                return Some(1.0);
            }
        }

        if alpha_limit >= 0.0 {
            if tl_microtime >= target_time {
                if time_difference < alpha_limit as i64 {
                    // Fade-in
                    Some((alpha_limit - time_difference as f32) / alpha_limit)
                } else {
                    // Hidden
                    None
                }
            } else {
                // Fully visible
                Some(1.0)
            }
        } else {
            // Negative alpha_limit
            if tl_microtime >= target_time {
                // Hidden
                None
            } else if time_difference > alpha_limit as i64 {
                // Fade-in
                Some(1.0 - (alpha_limit - time_difference as f32) / alpha_limit)
            } else {
                // Fully visible
                Some(1.0)
            }
        }
    }

    /// Calculate PMS note expansion scale factors.
    /// Returns (width_scale, height_scale) multipliers.
    pub fn calc_note_expansion(
        now: i64,
        quarter_note_time: i64,
        expansion_rate_w: i32,
        expansion_rate_h: i32,
        expansion_time: f32,
        contraction_time: f32,
    ) -> (f32, f32) {
        if expansion_rate_w == 100 && expansion_rate_h == 100 {
            return (1.0, 1.0);
        }

        let elapsed = (now - quarter_note_time) as f32;

        if elapsed < expansion_time {
            // Expansion phase
            let w_scale = 1.0 + (expansion_rate_w as f32 / 100.0 - 1.0) * elapsed / expansion_time;
            let h_scale = 1.0 + (expansion_rate_h as f32 / 100.0 - 1.0) * elapsed / expansion_time;
            (w_scale, h_scale)
        } else if elapsed <= expansion_time + contraction_time {
            // Contraction phase
            let contraction_elapsed = elapsed - expansion_time;
            let w_scale = 1.0
                + (expansion_rate_w as f32 / 100.0 - 1.0)
                    * (contraction_time - contraction_elapsed)
                    / contraction_time;
            let h_scale = 1.0
                + (expansion_rate_h as f32 / 100.0 - 1.0)
                    * (contraction_time - contraction_elapsed)
                    / contraction_time;
            (w_scale, h_scale)
        } else {
            // Normal size
            (1.0, 1.0)
        }
    }

    /// Calculate the visible scroll region (duration) based on BPM, hispeed, and scroll.
    /// Java: (240000 / nbpm / hispeed) / nscroll
    pub fn calc_region(bpm: f64, hispeed: f32, scroll: f64) -> f64 {
        if scroll > 0.0 && bpm > 0.0 && hispeed != 0.0 {
            (240000.0 / bpm / hispeed as f64) / scroll
        } else {
            0.0
        }
    }
}

include!("draw.rs");

/// Offset values passed to draw_lane (simplified from SkinOffset)
#[derive(Clone, Copy, Debug, Default)]
pub struct DrawLaneOffset {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Result of draw_lane(), containing draw commands and offset calculations
#[derive(Clone, Debug, Default)]
pub struct DrawLaneResult {
    /// Draw commands to execute on the rendering backend
    pub commands: Vec<DrawCommand>,
    /// LIFT offset Y value (hl - lanes[0].region.y)
    pub lift_offset_y: f32,
    /// LANECOVER offset Y value
    pub lanecover_offset_y: f32,
    /// Hidden cover result
    pub hidden_cover: HiddenCoverResult,
}

/// Hidden cover state calculated by draw_lane()
#[derive(Clone, Copy, Debug, Default)]
pub struct HiddenCoverResult {
    /// Whether hidden cover is visible (false = alpha -255 in Java)
    pub visible: bool,
    /// Y offset for hidden cover
    pub y: f32,
}

#[cfg(test)]
mod tests;

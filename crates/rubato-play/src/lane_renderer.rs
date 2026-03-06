use bms_model::bms_model::{BMSModel, LNTYPE_CHARGENOTE, LNTYPE_HELLCHARGENOTE, LNTYPE_LONGNOTE};
use bms_model::note::{Note, TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_LONGNOTE, TYPE_UNDEFINED};
use bms_model::time_line::TimeLine;
use rubato_types::play_config::PlayConfig;
use std::collections::HashMap;

use crate::skin_note::SkinLane;

/// Fix hispeed modes
pub const FIX_HISPEED_OFF: i32 = 0;
pub const FIX_HISPEED_STARTBPM: i32 = 1;
pub const FIX_HISPEED_MINBPM: i32 = 2;
pub const FIX_HISPEED_MAXBPM: i32 = 3;
pub const FIX_HISPEED_MAINBPM: i32 = 4;

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
    DrawStopLine { y_offset: i32, stop_ms: i32 },
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteImageType {
    Normal,
    Processed,
    Mine,
    Hidden,
}

/// External state required by draw_lane() that comes from BMSPlayer and other subsystems.
/// This avoids coupling LaneRenderer to BMSPlayer directly.
pub struct DrawLaneContext<'a> {
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
    pub lntype: i32,
    /// Judge time regions per lane (5 judge levels, [start, end])
    pub judge_time_regions: Vec<Vec<[i64; 2]>>,
    /// Processing long note per lane (timeline index of the LN pair, if actively pressing)
    pub processing_long_notes: Vec<Option<usize>>,
    /// Passing long note per lane (timeline index, if LN is passing through)
    pub passing_long_notes: Vec<Option<usize>>,
    /// Hell charge judge per lane
    pub hell_charge_judges: Vec<bool>,
    /// Judge table (for PMS miss POOR bad time)
    pub bad_judge_time: i64,
    /// Model's initial BPM
    pub model_bpm: f64,
    /// All timelines from the model (the full array, not filtered)
    pub all_timelines: &'a [TimeLine],
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
    hispeedmargin: f32,
    /// Filtered timeline indices (indexes into BMSModel.all_time_lines())
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
        let all_tls = model.all_time_lines();
        let mut indices: Vec<usize> = Vec::new();
        let mut cbpm = model.bpm();
        let mut cscr = 1.0;
        for (i, tl) in all_tls.iter().enumerate() {
            if cbpm != tl.bpm()
                || tl.stop() > 0
                || cscr != tl.scroll()
                || tl.section_line()
                || tl.exist_note()
                || tl.exist_hidden_note()
            {
                indices.push(i);
            }
            cbpm = tl.bpm();
            cscr = tl.scroll();
        }
        self.timeline_indices = indices;

        self.minbpm = model.min_bpm();
        self.maxbpm = model.max_bpm();

        // Find main BPM (BPM with most notes)
        let mut bpm_counts: HashMap<u64, (f64, i32)> = HashMap::new();
        for tl in all_tls {
            let key = tl.bpm().to_bits();
            let entry = bpm_counts.entry(key).or_insert((tl.bpm(), 0));
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
            FIX_HISPEED_STARTBPM => model.bpm(),
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

    pub fn set_hispeedmargin(&mut self, hispeedmargin: f32) {
        self.hispeedmargin = hispeedmargin;
    }

    pub fn is_enable_lift(&self) -> bool {
        self.enable_lift
    }

    pub fn set_enable_lift(&mut self, b: bool) {
        self.enable_lift = b;
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

    pub fn hidden_cover(&self) -> f32 {
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

    /// Calculate the y-position offset for a timeline relative to a previous timeline.
    /// This is the core scroll position calculation used throughout draw_lane().
    ///
    /// Java: y += (tl.getSection() - prevtl.getSection()) * prevtl.getScroll() * ...
    pub fn calc_y_offset(tl: &TimeLine, prev_tl: &TimeLine, microtime: i64, rxhs: f64) -> f64 {
        if prev_tl.micro_time() + prev_tl.micro_stop() > microtime {
            // During a stop: full section distance
            (tl.section() - prev_tl.section()) * prev_tl.scroll() * rxhs
        } else {
            // Normal scrolling: proportional to time remaining
            let time_diff = tl.micro_time() - microtime;
            let total_time = tl.micro_time() - prev_tl.micro_time() - prev_tl.micro_stop();
            if total_time == 0 {
                0.0
            } else {
                (tl.section() - prev_tl.section())
                    * prev_tl.scroll()
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

    /// Main lane drawing method. Ported from Java LaneRenderer.drawLane() (713 lines).
    ///
    /// Handles:
    /// - Section line drawing
    /// - Per-timeline note rendering via SkinLane objects
    /// - Long note rendering (CN/HCN/LN)
    /// - PMS rhythm-based note expansion/contraction
    /// - Constant mode scrolling with fade-in
    /// - Judge area display
    /// - Lane cover, hidden cover, lift region calculations
    /// - PMS miss POOR note fallthrough rendering
    ///
    /// Returns a Vec of DrawCommands that the caller should execute on the rendering backend.
    /// Also returns offset values for LIFT, LANECOVER, and HIDDEN_COVER positioning.
    #[allow(clippy::too_many_lines)]
    pub fn draw_lane(
        &mut self,
        ctx: &DrawLaneContext,
        lanes: &[SkinLane],
        offsets: &[DrawLaneOffset],
    ) -> DrawLaneResult {
        let mut commands: Vec<DrawCommand> = Vec::new();

        if lanes.is_empty() {
            return DrawLaneResult::default();
        }

        // Accumulate offsets
        let mut offset_x: f32 = 0.0;
        let mut offset_y: f32 = 0.0;
        let mut offset_w: f32 = 0.0;
        let mut offset_h: f32 = 0.0;
        for offset in offsets {
            offset_x += offset.x;
            offset_y += offset.y;
            offset_w += offset.w;
            offset_h += offset.h;
        }

        // Calculate time
        // Java: time = (main.timer.isTimerOn(TIMER_PLAY) ? time - main.timer.getTimer(TIMER_PLAY) :
        //     (main.timer.isTimerOn(141) ? time - main.timer.getTimer(141) : 0)) + config.getJudgetiming();
        let time = if let Some(timer_play) = ctx.timer_play {
            ctx.time - timer_play
        } else if let Some(timer_141) = ctx.timer_141 {
            ctx.time - timer_141
        } else {
            0
        } + ctx.judge_timing;

        let time = if ctx.is_practice {
            self.pos = 0;
            ctx.practice_start_time
        } else {
            time
        };

        let microtime = time * 1000;
        let show_timeline = ctx.is_practice;

        let hispeed = if !ctx.is_practice { self.hispeed } else { 1.0 };

        // Get the filtered timelines (indices into all_timelines)
        let timelines = &self.timeline_indices;
        let all_tl = ctx.all_timelines;

        // Resolve timelines: for each index, get the actual TimeLine reference
        // Build a local vec of references for the filtered timelines
        let tl_count = timelines.len();

        // Find current BPM and scroll
        let mut nbpm = ctx.model_bpm;
        let mut nscroll = 1.0;
        let start_idx = self.pos.saturating_sub(5);
        for i in start_idx..tl_count {
            let tl = &all_tl[timelines[i]];
            if tl.micro_time() > microtime {
                break;
            }
            nbpm = tl.bpm();
            nscroll = tl.scroll();
        }
        self.nowbpm = nbpm;

        let region = Self::calc_region(nbpm, hispeed, nscroll);

        // Y-down coordinate system (wgpu): region_y is at the top of the lane
        // (small y = top of screen), region_y + region_height is at the bottom (judge line).
        // hu = top of visible lane area (small y), hl = judge line (large y).
        let hu = lanes[0].region_y;
        let hl = if self.enable_lift {
            // Lift moves judge line upward from the bottom
            lanes[0].region_y + lanes[0].region_height * (1.0 - self.lift)
        } else {
            lanes[0].region_y + lanes[0].region_height
        };
        // rxhs is negative in Y-down: future notes have decreasing y (move upward on screen).
        let rxhs = (hu - hl) as f64 * hispeed as f64;
        let mut y = hl as f64;

        let lanecover = if self.enable_lanecover {
            self.lanecover
        } else {
            0.0
        };
        self.currentduration = (region * (1.0 - lanecover as f64)).round() as i32;

        // Calculate offset results for LIFT, LANECOVER, HIDDEN
        // In Y-down: default judge line is at region_y + region_height (bottom).
        // Lift moves it upward (smaller y). Lift offset = how far it moved upward.
        let default_hl = lanes[0].region_y + lanes[0].region_height;
        let lift_offset_y = default_hl - hl;
        // Lanecover: (hl - hu) is the visible lane height (positive in Y-down).
        // In Java this was negative (hl < hu in Y-up); skin offsets expect the same sign.
        let lanecover_offset_y = (hu - hl) as f64 * lanecover as f64;

        let hidden_result = if self.enable_hidden {
            let hidden_y = if self.enable_lift {
                (1.0 - self.lift) * self.hidden * lanes[0].region_height
            } else {
                self.hidden * lanes[0].region_height
            };
            HiddenCoverResult {
                visible: true,
                y: hidden_y,
            }
        } else {
            HiddenCoverResult {
                visible: false,
                y: 0.0,
            }
        };

        // Judge area display
        if ctx.show_judgearea {
            let judge_colors: [(f32, f32, f32, f32); 5] = [
                (0.0, 0.0, 1.0, 32.0 / 255.0), // blue
                (0.0, 1.0, 0.0, 32.0 / 255.0), // green
                (1.0, 1.0, 0.0, 32.0 / 255.0), // yellow
                (1.0, 0.5, 0.0, 32.0 / 255.0), // orange
                (1.0, 0.0, 0.0, 32.0 / 255.0), // red
            ];

            #[allow(clippy::needless_range_loop)]
            for lane in 0..lanes.len() {
                if lane >= ctx.judge_time_regions.len() {
                    break;
                }
                let judgetime = &ctx.judge_time_regions[lane];
                for i in self.pos..tl_count {
                    let tl = &all_tl[timelines[i]];
                    if tl.micro_time() >= microtime {
                        let prev_section = if i > 0 {
                            all_tl[timelines[i - 1]].section()
                        } else {
                            0.0
                        };
                        let prev_scroll = if i > 0 {
                            all_tl[timelines[i - 1]].scroll()
                        } else {
                            1.0
                        };
                        let prev_microtime = if i > 0 {
                            all_tl[timelines[i - 1]].micro_time()
                                + all_tl[timelines[i - 1]].micro_stop()
                        } else {
                            0
                        };

                        let denom = tl.micro_time() - prev_microtime;
                        let rate = if denom != 0 {
                            (tl.section() - prev_section) * prev_scroll * rxhs / denom as f64
                        } else {
                            0.0
                        };

                        for j in (0..judge_colors.len()).rev() {
                            let (r, g, b, a) = judge_colors[j];
                            commands.push(DrawCommand::SetColor { r, g, b, a });

                            let nj = if j > 0 && j - 1 < judgetime.len() {
                                judgetime[j - 1][1]
                            } else {
                                0
                            };
                            let judge_end = if j < judgetime.len() {
                                judgetime[j][1]
                            } else {
                                0
                            };

                            commands.push(DrawCommand::DrawJudgeArea {
                                lane,
                                x: lanes[lane].region_x,
                                y: (hl as f64 + nj as f64 * rate) as f32,
                                w: lanes[lane].region_width,
                                h: ((judge_end - nj) as f64 * rate) as f32,
                                color_index: j,
                            });
                        }
                        break;
                    }
                }
            }
        }

        // Draw section lines and markers (first pass)
        let orgy = y;
        let enable_constant = self.enable_constant && !ctx.is_practice;
        let baseduration = self.duration;
        let alpha_limit = self.constant_fadein_time * 1000.0;

        for i in self.pos..tl_count {
            if y < hu as f64 {
                break;
            }
            let tl = &all_tl[timelines[i]];
            if tl.micro_time() >= microtime {
                // Constant mode alpha
                if enable_constant {
                    match Self::calc_constant_alpha(
                        tl.micro_time(),
                        microtime,
                        baseduration,
                        alpha_limit,
                    ) {
                        None => continue, // hidden
                        Some(alpha) => {
                            if (alpha - 1.0).abs() > f32::EPSILON {
                                commands.push(DrawCommand::SetColor {
                                    r: 1.0,
                                    g: 1.0,
                                    b: 1.0,
                                    a: alpha,
                                });
                            } else {
                                commands.push(DrawCommand::SetColor {
                                    r: 1.0,
                                    g: 1.0,
                                    b: 1.0,
                                    a: 1.0,
                                });
                            }
                        }
                    }
                }

                // Calculate y position
                if i > 0 {
                    let prev_tl = &all_tl[timelines[i - 1]];
                    y += Self::calc_y_offset(tl, prev_tl, microtime, rxhs);
                } else {
                    y += Self::calc_y_offset_first(tl, microtime, rxhs);
                }

                // Timeline display (practice mode)
                if show_timeline
                    && i > 0
                    && (tl.time() / 1000) > (all_tl[timelines[i - 1]].time() / 1000)
                {
                    commands.push(DrawCommand::DrawTimeLine {
                        y_offset: (y - hl as f64) as i32,
                    });
                    for r in &ctx.lane_group_regions {
                        commands.push(DrawCommand::DrawTimeText {
                            text: format!(
                                "{:2}:{:02}.{:1}",
                                tl.time() / 60000,
                                (tl.time() / 1000) % 60,
                                (tl.time() / 100) % 10
                            ),
                            x: r.x + 4.0,
                            y: y as f32 + 20.0,
                        });
                    }
                }

                // BPM guide / Stop lines
                if ctx.show_bpmguide || show_timeline {
                    if tl.bpm() != nbpm {
                        commands.push(DrawCommand::DrawBpmLine {
                            y_offset: (y - hl as f64) as i32,
                            bpm: tl.bpm(),
                        });
                        for r in &ctx.lane_group_regions {
                            commands.push(DrawCommand::DrawBpmText {
                                text: format!("BPM{}", tl.bpm() as i32),
                                x: r.x + r.width / 2.0,
                                y: y as f32 + 20.0,
                            });
                        }
                    }
                    if tl.stop() > 0 {
                        commands.push(DrawCommand::DrawStopLine {
                            y_offset: (y - hl as f64) as i32,
                            stop_ms: tl.stop(),
                        });
                        for r in &ctx.lane_group_regions {
                            commands.push(DrawCommand::DrawStopText {
                                text: format!("STOP {}ms", tl.stop()),
                                x: r.x + r.width / 2.0,
                                y: y as f32 + 20.0,
                            });
                        }
                    }
                }

                // Section line
                if tl.section_line() {
                    commands.push(DrawCommand::DrawSectionLine {
                        y_offset: (y - hl as f64) as i32,
                    });
                }

                nbpm = tl.bpm();
            } else if self.pos == i.wrapping_sub(1) {
                // Advance pos: check if all notes in this timeline are past
                let mut can_advance = true;
                for lane in 0..lanes.len() {
                    let note = tl.note(lane as i32);
                    if let Some(note) = note {
                        match note {
                            Note::Long { end, pair, .. } => {
                                // For LN: check if the end is still visible
                                let pair_idx = if *end {
                                    // This is the end note; check if pair (start) is still active
                                    *pair
                                } else {
                                    // This is the start note; check pair (end) time
                                    *pair
                                };
                                if let Some(pair_tl_idx) = pair_idx {
                                    let pair_tl = &all_tl[pair_tl_idx];
                                    let pair_time = pair_tl.micro_time();
                                    if pair_time >= microtime {
                                        can_advance = false;
                                        break;
                                    }
                                }
                            }
                            Note::Normal(_) => {
                                if ctx.show_pastnote && note.state() == 0 {
                                    can_advance = false;
                                    break;
                                }
                            }
                            Note::Mine { .. } => {}
                        }
                    }
                }
                if can_advance {
                    self.pos = i;
                }
            }
        }

        // Reset color and blend for note rendering (second pass)
        commands.push(DrawCommand::SetColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        commands.push(DrawCommand::SetBlend(0));
        commands.push(DrawCommand::SetType(0)); // TYPE_NORMAL

        y = orgy;
        let now = ctx.now_time;

        // Note rendering pass
        for i in self.pos..tl_count {
            if y < hu as f64 {
                break;
            }
            let tl = &all_tl[timelines[i]];

            // Constant mode alpha for notes
            if enable_constant {
                match Self::calc_constant_alpha(
                    tl.micro_time(),
                    microtime,
                    baseduration,
                    alpha_limit,
                ) {
                    None => continue, // hidden
                    Some(alpha) => {
                        if (alpha - 1.0).abs() > f32::EPSILON {
                            commands.push(DrawCommand::SetColor {
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
                                a: alpha,
                            });
                        } else {
                            commands.push(DrawCommand::SetColor {
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
                                a: 1.0,
                            });
                        }
                    }
                }
            }

            // Calculate y position
            if tl.micro_time() >= microtime {
                if i > 0 {
                    let prev_tl = &all_tl[timelines[i - 1]];
                    y += Self::calc_y_offset(tl, prev_tl, microtime, rxhs);
                } else {
                    y += Self::calc_y_offset_first(tl, microtime, rxhs);
                }
            }

            // Per-lane note rendering
            #[allow(clippy::needless_range_loop)]
            for lane in 0..lanes.len() {
                let scale = lanes[lane].scale;
                let note = tl.note(lane as i32);
                if let Some(note) = note {
                    // PMS note expansion
                    let (exp_w, exp_h) = Self::calc_note_expansion(
                        now,
                        ctx.now_quarter_note_time,
                        ctx.note_expansion_rate[0],
                        ctx.note_expansion_rate[1],
                        self.note_expansion_time,
                        self.note_contraction_time,
                    );

                    let mut dstx = lanes[lane].region_x + offset_x;
                    let mut dsty = y as f32 + offset_y - offset_h / 2.0;
                    let mut dstw = lanes[lane].region_width + offset_w;
                    let mut dsth = scale + offset_h;

                    if exp_w != 1.0 || exp_h != 1.0 {
                        dstw *= exp_w;
                        dsth *= exp_h;
                        dstx -= (dstw - lanes[lane].region_width) / 2.0;
                        dsty -= (dsth - scale) / 2.0;
                    }

                    match note {
                        Note::Normal(_) => {
                            // Draw normal note
                            if lanes[lane].dstnote2 != i32::MIN {
                                // PMS mode: only draw if future and unjudged or state >= 4
                                if tl.micro_time() >= microtime
                                    && (note.state() == 0 || note.state() >= 4)
                                {
                                    let image_type = if ctx.mark_processednote && note.state() != 0
                                    {
                                        NoteImageType::Processed
                                    } else {
                                        NoteImageType::Normal
                                    };
                                    commands.push(DrawCommand::DrawNote {
                                        lane,
                                        x: dstx,
                                        y: dsty,
                                        w: dstw,
                                        h: dsth,
                                        image_type,
                                    });
                                }
                            } else if tl.micro_time() >= microtime
                                || (ctx.show_pastnote && note.state() == 0)
                            {
                                let image_type = if ctx.mark_processednote && note.state() != 0 {
                                    NoteImageType::Processed
                                } else {
                                    NoteImageType::Normal
                                };
                                commands.push(DrawCommand::DrawNote {
                                    lane,
                                    x: dstx,
                                    y: dsty,
                                    w: dstw,
                                    h: dsth,
                                    image_type,
                                });
                            }
                        }
                        Note::Long { end, pair, .. } => {
                            if !end {
                                // Only draw from start note
                                if let Some(pair_tl_idx) = pair {
                                    let pair_tl = &all_tl[*pair_tl_idx];
                                    if pair_tl.micro_time() >= microtime {
                                        // Calculate long note body height
                                        let mut dy: f64 = 0.0;
                                        let mut prev_tl_ref = tl;
                                        let pair_section = pair_tl.section();

                                        for j in (i + 1)..tl_count {
                                            let now_tl = &all_tl[timelines[j]];
                                            if prev_tl_ref.section() == pair_section {
                                                break;
                                            }
                                            if now_tl.micro_time() >= microtime {
                                                if prev_tl_ref.micro_time()
                                                    + prev_tl_ref.micro_stop()
                                                    > microtime
                                                {
                                                    dy += (now_tl.section()
                                                        - prev_tl_ref.section())
                                                        * prev_tl_ref.scroll()
                                                        * rxhs;
                                                } else {
                                                    let time_diff = now_tl.micro_time() - microtime;
                                                    let total_time = now_tl.micro_time()
                                                        - prev_tl_ref.micro_time()
                                                        - prev_tl_ref.micro_stop();
                                                    if total_time != 0 {
                                                        dy += (now_tl.section()
                                                            - prev_tl_ref.section())
                                                            * prev_tl_ref.scroll()
                                                            * (time_diff as f64
                                                                / total_time as f64)
                                                            * rxhs;
                                                    }
                                                }
                                            }
                                            prev_tl_ref = now_tl;
                                        }

                                        // In Y-down, dy is negative (end note is above
                                        // start note). Use absolute value for height.
                                        let dy_abs = dy.abs();
                                        if dy_abs > 0.0 {
                                            let dscale = if dsth > scale {
                                                (dsth - scale) / 2.0
                                            } else {
                                                0.0
                                            };
                                            // ln_y = end note position (above start in Y-down)
                                            let ln_y = dsty - dy_abs as f32;
                                            let ln_height = if dsty
                                                > (lanes[lane].region_y
                                                    + lanes[lane].region_height
                                                    + dscale)
                                            {
                                                (lanes[lane].region_y
                                                    + lanes[lane].region_height
                                                    + dscale)
                                                    - dsty
                                            } else {
                                                dy_abs as f32
                                            };
                                            self.draw_long_note_commands(
                                                &mut commands,
                                                ctx,
                                                lane,
                                                dstx,
                                                ln_y,
                                                dstw,
                                                ln_height,
                                                dsth,
                                                note,
                                                *pair_tl_idx,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Note::Mine { .. } => {
                            // Draw mine note
                            if tl.micro_time() >= microtime {
                                commands.push(DrawCommand::DrawNote {
                                    lane,
                                    x: dstx,
                                    y: dsty,
                                    w: dstw,
                                    h: dsth,
                                    image_type: NoteImageType::Mine,
                                });
                            }
                        }
                    }
                }

                // Hidden note rendering
                if ctx.show_hiddennote && tl.micro_time() >= microtime {
                    let hnote = tl.hidden_note(lane as i32);
                    if hnote.is_some() {
                        commands.push(DrawCommand::DrawNote {
                            lane,
                            x: lanes[lane].region_x,
                            y: y as f32,
                            w: lanes[lane].region_width,
                            h: scale,
                            image_type: NoteImageType::Hidden,
                        });
                    }
                }
            }
        }

        // PMS miss POOR rendering
        if lanes[0].dstnote2 != i32::MIN {
            let bad_time = ctx.bad_judge_time.unsigned_abs() as i64;
            let mut orgy2 = lanes[0].dstnote2 as f64;
            if orgy2 < -(lanes[0].region_height as f64) {
                orgy2 = -(lanes[0].region_height as f64);
            }
            if orgy2 > orgy {
                orgy2 = orgy;
            }
            let rxhs2 = (hu - hl) as f64;

            // Find current position in timelines
            let mut now_pos = tl_count.saturating_sub(1);
            for i in self.pos..tl_count {
                let tl = &all_tl[timelines[i]];
                if tl.micro_time() >= microtime {
                    now_pos = i;
                    break;
                }
            }

            // Iterate backwards for miss POOR falling notes
            y = orgy;
            let mut ii = now_pos as i64;
            while ii >= 0 && y >= orgy2 {
                let i = ii as usize;
                let tl = &all_tl[timelines[i]];
                y = orgy;

                if i + 1 < tl_count {
                    let mut j = i;
                    while j + 1 < tl_count && all_tl[timelines[j + 1]].micro_time() < microtime {
                        if all_tl[timelines[j + 1]].micro_time()
                            > tl.micro_time() + tl.micro_stop() + bad_time
                        {
                            let stop_time = 0i64.max(
                                tl.micro_time() + tl.micro_stop() + bad_time
                                    - all_tl[timelines[j]].micro_time()
                                    - all_tl[timelines[j]].micro_stop(),
                            );
                            y -= (all_tl[timelines[j + 1]].micro_time()
                                - all_tl[timelines[j]].micro_time()
                                - all_tl[timelines[j]].micro_stop()
                                - stop_time) as f64
                                * rxhs2
                                * all_tl[timelines[j]].bpm()
                                / 240000000.0;
                        }
                        j += 1;
                    }
                    if all_tl[timelines[j]].micro_time() + all_tl[timelines[j]].micro_stop()
                        < microtime
                        && microtime > tl.micro_time() + tl.micro_stop() + bad_time
                    {
                        let stop_time = 0i64.max(
                            tl.micro_time() + tl.micro_stop() + bad_time
                                - all_tl[timelines[j]].micro_time()
                                - all_tl[timelines[j]].micro_stop(),
                        );
                        y -= (microtime
                            - all_tl[timelines[j]].micro_time()
                            - all_tl[timelines[j]].micro_stop()
                            - stop_time) as f64
                            * rxhs2
                            * all_tl[timelines[j]].bpm()
                            / 240000000.0;
                    }
                } else if tl.micro_time() + tl.micro_stop() < microtime
                    && microtime > tl.micro_time() + tl.micro_stop() + bad_time
                {
                    let stop_time = 0i64.max(
                        tl.micro_time() + tl.micro_stop() + bad_time
                            - tl.micro_time()
                            - tl.micro_stop(),
                    );
                    y -= (microtime - tl.micro_time() - tl.micro_stop() - stop_time) as f64
                        * rxhs2
                        * tl.bpm()
                        / 240000000.0;
                }

                // Per-lane miss POOR note rendering
                #[allow(clippy::needless_range_loop)]
                for lane in 0..lanes.len() {
                    let scale = lanes[lane].scale;
                    if let Some(note) = tl.note(lane as i32).filter(|n| n.is_normal()) {
                        let (exp_w, exp_h) = Self::calc_note_expansion(
                            now,
                            ctx.now_quarter_note_time,
                            ctx.note_expansion_rate[0],
                            ctx.note_expansion_rate[1],
                            self.note_expansion_time,
                            self.note_contraction_time,
                        );

                        let mut dstx = lanes[lane].region_x;
                        let mut dsty = y as f32;
                        let mut dstw = lanes[lane].region_width;
                        let mut dsth = scale;

                        if exp_w != 1.0 || exp_h != 1.0 {
                            dstw *= exp_w;
                            dsth *= exp_h;
                            dstx -= (dstw - lanes[lane].region_width) / 2.0;
                            dsty -= (dsth - scale) / 2.0;
                        }

                        if (note.state() == 0 || note.state() >= 4)
                            && tl.micro_time() <= microtime
                            && y >= orgy2
                        {
                            if y > orgy {
                                commands.push(DrawCommand::DrawNote {
                                    lane,
                                    x: dstx,
                                    y: (orgy as f32) - (dsth - scale) / 2.0,
                                    w: dstw,
                                    h: dsth,
                                    image_type: NoteImageType::Normal,
                                });
                            } else {
                                commands.push(DrawCommand::DrawNote {
                                    lane,
                                    x: dstx,
                                    y: dsty,
                                    w: dstw,
                                    h: dsth,
                                    image_type: NoteImageType::Normal,
                                });
                            }
                        }
                    }
                }

                ii -= 1;
            }
        }

        DrawLaneResult {
            commands,
            lift_offset_y,
            lanecover_offset_y: lanecover_offset_y as f32,
            hidden_cover: hidden_result,
        }
    }

    /// Draw long note (CN/HCN/LN).
    /// Corresponds to Java drawLongNote() private method.
    ///
    /// Emits DrawCommand entries for the long note body, start, and end images.
    /// The long note image array indices are:
    ///   CN/LN: 0=start, 1=end, 2=active_body, 3=inactive_body
    ///   HCN:   4=start, 5=end, 6=active_body, 7=inactive_body,
    ///          8=hell_ok_body, 9=hell_ng_body
    #[allow(clippy::too_many_arguments)]
    fn draw_long_note_commands(
        &self,
        commands: &mut Vec<DrawCommand>,
        ctx: &DrawLaneContext,
        lane: usize,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        scale: f32,
        note: &Note,
        pair_tl_idx: usize,
    ) {
        let Note::Long { note_type, .. } = note else {
            return;
        };

        let is_processing =
            ctx.processing_long_notes.get(lane).copied().flatten() == Some(pair_tl_idx);
        let is_passing = ctx.passing_long_notes.get(lane).copied().flatten()
            == Some(
                self.timeline_indices
                    .iter()
                    .position(|&idx| {
                        ctx.all_timelines.get(idx).is_some_and(|tl| {
                            tl.note(lane as i32).is_some_and(|n| std::ptr::eq(n, note))
                        })
                    })
                    .unwrap_or(usize::MAX),
            );
        let hell_charge_ok = ctx.hell_charge_judges.get(lane).copied().unwrap_or(false);

        if (ctx.lntype == LNTYPE_HELLCHARGENOTE && *note_type == TYPE_UNDEFINED)
            || *note_type == TYPE_HELLCHARGENOTE
        {
            // HCN
            let body_idx = if is_processing {
                6 // active body
            } else if is_passing && note.state() != 0 {
                if hell_charge_ok { 8 } else { 9 } // hell charge ok/ng
            } else {
                7 // inactive body
            };
            commands.push(DrawCommand::DrawLongNote {
                lane,
                x,
                y: y - height + scale,
                w: width,
                h: height - scale,
                image_index: body_idx,
            });
            commands.push(DrawCommand::DrawLongNote {
                lane,
                x,
                y,
                w: width,
                h: scale,
                image_index: 4, // HCN start
            });
            commands.push(DrawCommand::DrawLongNote {
                lane,
                x,
                y: y - height,
                w: width,
                h: scale,
                image_index: 5, // HCN end
            });
        } else if (ctx.lntype == LNTYPE_CHARGENOTE && *note_type == TYPE_UNDEFINED)
            || *note_type == TYPE_CHARGENOTE
        {
            // CN
            let body_idx = if is_processing { 2 } else { 3 };
            commands.push(DrawCommand::DrawLongNote {
                lane,
                x,
                y: y - height + scale,
                w: width,
                h: height - scale,
                image_index: body_idx,
            });
            commands.push(DrawCommand::DrawLongNote {
                lane,
                x,
                y,
                w: width,
                h: scale,
                image_index: 0, // CN start
            });
            commands.push(DrawCommand::DrawLongNote {
                lane,
                x,
                y: y - height,
                w: width,
                h: scale,
                image_index: 1, // CN end
            });
        } else if (ctx.lntype == LNTYPE_LONGNOTE && *note_type == TYPE_UNDEFINED)
            || *note_type == TYPE_LONGNOTE
        {
            // LN
            let body_idx = if is_processing { 2 } else { 3 };
            commands.push(DrawCommand::DrawLongNote {
                lane,
                x,
                y: y - height + scale,
                w: width,
                h: height - scale,
                image_index: body_idx,
            });
            if ctx.forced_cn_endings {
                commands.push(DrawCommand::DrawLongNote {
                    lane,
                    x,
                    y,
                    w: width,
                    h: scale,
                    image_index: 0, // LN start (only when forced CN endings)
                });
            }
            commands.push(DrawCommand::DrawLongNote {
                lane,
                x,
                y: y - height,
                w: width,
                h: scale,
                image_index: 1, // LN end
            });
        }
    }

    pub fn now_bpm(&self) -> f64 {
        self.nowbpm
    }

    pub fn min_bpm(&self) -> f64 {
        self.minbpm
    }

    pub fn max_bpm(&self) -> f64 {
        self.maxbpm
    }

    pub fn main_bpm(&self) -> f64 {
        self.mainbpm
    }

    pub fn play_config(&self) -> PlayConfig {
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

    pub fn dispose(&mut self) {
        // no GPU resources in Rust translation
    }
}

/// Offset values passed to draw_lane (simplified from SkinOffset)
#[derive(Clone, Debug, Default)]
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
#[derive(Clone, Debug, Default)]
pub struct HiddenCoverResult {
    /// Whether hidden cover is visible (false = alpha -255 in Java)
    pub visible: bool,
    /// Y offset for hidden cover
    pub y: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::bms_model::BMSModel;
    use bms_model::note::Note;
    use bms_model::time_line::TimeLine;

    // --- Helper to create a minimal BMSModel with timelines ---

    fn make_model_with_timelines(timelines: Vec<TimeLine>, bpm: f64) -> BMSModel {
        let mut model = BMSModel::new();
        model.set_bpm(bpm);
        model.set_all_time_line(timelines);
        model
    }

    fn make_timeline(section: f64, time_us: i64, bpm: f64, notesize: i32) -> TimeLine {
        let mut tl = TimeLine::new(section, time_us, notesize);
        tl.set_bpm(bpm);
        tl
    }

    fn default_ctx(all_timelines: &[TimeLine]) -> DrawLaneContext<'_> {
        DrawLaneContext {
            time: 0,
            timer_play: Some(0),
            timer_141: None,
            judge_timing: 0,
            is_practice: false,
            practice_start_time: 0,
            now_time: 0,
            now_quarter_note_time: 0,
            note_expansion_rate: [100, 100],
            lane_group_regions: vec![],
            show_bpmguide: false,
            show_pastnote: false,
            mark_processednote: false,
            show_hiddennote: false,
            show_judgearea: false,
            lntype: LNTYPE_LONGNOTE,
            judge_time_regions: vec![],
            processing_long_notes: vec![None; 8],
            passing_long_notes: vec![None; 8],
            hell_charge_judges: vec![false; 8],
            bad_judge_time: 0,
            model_bpm: 120.0,
            all_timelines,
            forced_cn_endings: false,
        }
    }

    fn make_lanes(count: usize) -> Vec<SkinLane> {
        let mut lanes = Vec::new();
        for _ in 0..count {
            let mut lane = SkinLane::new();
            lane.region_x = 0.0;
            lane.region_y = 0.0;
            lane.region_width = 30.0;
            lane.region_height = 500.0;
            lane.scale = 10.0;
            lanes.push(lane);
        }
        lanes
    }

    // =========================================================================
    // calc_region tests
    // =========================================================================

    #[test]
    fn calc_region_normal() {
        // At 120 BPM, hispeed 1.0, scroll 1.0: 240000/120/1 = 2000
        let region = LaneRenderer::calc_region(120.0, 1.0, 1.0);
        assert!((region - 2000.0).abs() < 0.001);
    }

    #[test]
    fn calc_region_with_hispeed() {
        // At 120 BPM, hispeed 2.0: 240000/120/2 = 1000
        let region = LaneRenderer::calc_region(120.0, 2.0, 1.0);
        assert!((region - 1000.0).abs() < 0.001);
    }

    #[test]
    fn calc_region_with_scroll() {
        // At 120 BPM, hispeed 1.0, scroll 2.0: 2000/2 = 1000
        let region = LaneRenderer::calc_region(120.0, 1.0, 2.0);
        assert!((region - 1000.0).abs() < 0.001);
    }

    #[test]
    fn calc_region_zero_scroll_returns_zero() {
        let region = LaneRenderer::calc_region(120.0, 1.0, 0.0);
        assert!((region).abs() < 0.001);
    }

    // =========================================================================
    // calc_constant_alpha tests
    // =========================================================================

    #[test]
    fn constant_alpha_fully_visible() {
        // Timeline is before target time -> fully visible
        let alpha = LaneRenderer::calc_constant_alpha(500_000, 1_000_000, 500, 100_000.0);
        assert_eq!(alpha, Some(1.0));
    }

    #[test]
    fn constant_alpha_hidden() {
        // Timeline is far beyond target time + alpha limit -> hidden
        let alpha = LaneRenderer::calc_constant_alpha(2_000_000, 500_000, 500, 100_000.0);
        assert_eq!(alpha, None);
    }

    #[test]
    fn constant_alpha_fadein() {
        // Timeline is just past target time, within alpha limit -> fade-in
        // target_time = 500_000 + 500*1000 = 1_000_000
        // tl time = 1_050_000, diff = 50_000
        // alpha = (100_000 - 50_000) / 100_000 = 0.5
        let alpha = LaneRenderer::calc_constant_alpha(1_050_000, 500_000, 500, 100_000.0);
        assert!(alpha.is_some());
        assert!((alpha.unwrap() - 0.5).abs() < 0.001);
    }

    #[test]
    fn constant_alpha_negative_limit_hidden() {
        // Negative alpha limit: hidden when past target
        let alpha = LaneRenderer::calc_constant_alpha(2_000_000, 500_000, 500, -100_000.0);
        assert_eq!(alpha, None);
    }

    #[test]
    fn constant_alpha_negative_limit_fadein() {
        // Negative alpha limit: fade-in when within negative range before target
        // target_time = 500_000 + 500*1000 = 1_000_000
        // tl time = 950_000 (before target), diff = 950_000 - 1_000_000 = -50_000
        // alpha_limit = -100_000
        // diff (-50_000) > alpha_limit (-100_000) -> fade-in
        // alpha = 1.0 - (alpha_limit - diff) / alpha_limit
        //       = 1.0 - (-100_000 - (-50_000)) / (-100_000)
        //       = 1.0 - (-50_000) / (-100_000) = 1.0 - 0.5 = 0.5
        let alpha = LaneRenderer::calc_constant_alpha(950_000, 500_000, 500, -100_000.0);
        assert!(alpha.is_some());
        assert!((alpha.unwrap() - 0.5).abs() < 0.001);
    }

    // =========================================================================
    // calc_note_expansion tests
    // =========================================================================

    #[test]
    fn note_expansion_disabled() {
        let (w, h) = LaneRenderer::calc_note_expansion(100, 0, 100, 100, 9.0, 150.0);
        assert_eq!(w, 1.0);
        assert_eq!(h, 1.0);
    }

    #[test]
    fn note_expansion_during_expand_phase() {
        // expansion_rate = 200%, elapsed = 4.5 (half of expansion_time 9.0)
        // scale = 1.0 + (200/100 - 1) * 4.5/9 = 1.0 + 1.0 * 0.5 = 1.5
        let (w, h) = LaneRenderer::calc_note_expansion(
            50, // now
            46, // quarter_note_time (elapsed = 50 - 46 = 4)
            200, 200, 9.0, 150.0,
        );
        // elapsed = 50 - 46 = 4
        // scale = 1.0 + 1.0 * 4/9 = 1.444...
        assert!((w - 1.4444).abs() < 0.01);
        assert!((h - 1.4444).abs() < 0.01);
    }

    #[test]
    fn note_expansion_during_contraction_phase() {
        // expansion_time = 9, contraction_time = 150
        // elapsed = 50 (past expansion_time, in contraction phase)
        // contraction_elapsed = 50 - 9 = 41
        // scale = 1.0 + (200/100 - 1) * (150 - 41) / 150 = 1.0 + 1.0 * 109/150 = 1.7267
        let (w, _h) = LaneRenderer::calc_note_expansion(100, 50, 200, 200, 9.0, 150.0);
        assert!((w - 1.7267).abs() < 0.01);
    }

    #[test]
    fn note_expansion_after_contraction() {
        // elapsed > expansion_time + contraction_time -> normal size
        let (w, h) = LaneRenderer::calc_note_expansion(200, 0, 200, 200, 9.0, 150.0);
        assert_eq!(w, 1.0);
        assert_eq!(h, 1.0);
    }

    // =========================================================================
    // calc_y_offset tests
    // =========================================================================

    #[test]
    fn y_offset_during_stop() {
        // When prev timeline is in a stop, full section distance is used
        let mut prev = make_timeline(0.0, 0, 120.0, 8);
        prev.set_stop(2_000_000); // 2 second stop
        let tl = make_timeline(1.0, 1_000_000, 120.0, 8);

        let offset = LaneRenderer::calc_y_offset(&tl, &prev, 500_000, 100.0);
        // During stop: full section * scroll * rxhs = 1.0 * 1.0 * 100.0 = 100.0
        assert!((offset - 100.0).abs() < 0.001);
    }

    #[test]
    fn y_offset_normal_scroll() {
        let prev = make_timeline(0.0, 0, 120.0, 8);
        let tl = make_timeline(1.0, 1_000_000, 120.0, 8);

        // At time 500000 (halfway): offset should be proportional
        let offset = LaneRenderer::calc_y_offset(&tl, &prev, 500_000, 100.0);
        // time_diff = 1_000_000 - 500_000 = 500_000
        // total_time = 1_000_000 - 0 - 0 = 1_000_000
        // offset = 1.0 * 1.0 * (500_000/1_000_000) * 100.0 = 50.0
        assert!((offset - 50.0).abs() < 0.001);
    }

    #[test]
    fn y_offset_with_scroll() {
        let mut prev = make_timeline(0.0, 0, 120.0, 8);
        prev.set_scroll(2.0);
        let tl = make_timeline(1.0, 1_000_000, 120.0, 8);

        let offset = LaneRenderer::calc_y_offset(&tl, &prev, 500_000, 100.0);
        // 1.0 * 2.0 * (500_000/1_000_000) * 100.0 = 100.0
        assert!((offset - 100.0).abs() < 0.001);
    }

    #[test]
    fn y_offset_first_timeline() {
        let tl = make_timeline(1.0, 1_000_000, 120.0, 8);

        let offset = LaneRenderer::calc_y_offset_first(&tl, 500_000, 100.0);
        // section * (tl_time - microtime) / tl_time * rxhs
        // = 1.0 * 500_000 / 1_000_000 * 100.0 = 50.0
        assert!((offset - 50.0).abs() < 0.001);
    }

    #[test]
    fn y_offset_first_at_time_zero() {
        let tl = make_timeline(1.0, 0, 120.0, 8);
        let offset = LaneRenderer::calc_y_offset_first(&tl, 0, 100.0);
        assert!((offset).abs() < 0.001);
    }

    // =========================================================================
    // draw_lane integration tests
    // =========================================================================

    #[test]
    fn draw_lane_empty_lanes_returns_empty() {
        let tl = make_timeline(0.0, 0, 120.0, 8);
        let model = make_model_with_timelines(vec![tl], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let result = renderer.draw_lane(&ctx, &[], &[]);

        assert!(result.commands.is_empty());
    }

    #[test]
    fn draw_lane_updates_now_bpm() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_section_line(true);
        let mut tl1 = make_timeline(1.0, 500_000, 150.0, 8);
        tl1.set_section_line(true);
        let mut tl2 = make_timeline(2.0, 1_000_000, 180.0, 8);
        tl2.set_section_line(true);
        let model = make_model_with_timelines(vec![tl0, tl1, tl2], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let mut ctx = default_ctx(all_tls);
        // Set time to 750ms (past tl1 at 500ms but before tl2 at 1000ms)
        ctx.time = 750;
        ctx.timer_play = Some(0);

        let lanes = make_lanes(8);
        renderer.draw_lane(&ctx, &lanes, &[]);

        // nowbpm should be 150 (from tl1 which is the last timeline before current time)
        assert!((renderer.now_bpm() - 150.0).abs() < 0.001);
    }

    #[test]
    fn draw_lane_calculates_current_duration() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_section_line(true);
        let model = make_model_with_timelines(vec![tl0], 120.0);
        let mut renderer = LaneRenderer::new(&model);
        renderer.duration = 1000;

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        renderer.draw_lane(&ctx, &lanes, &[]);

        // region = 240000/120/1.0 / 1.0 = 2000
        // currentduration = 2000 * (1 - 0) = 2000
        assert_eq!(renderer.current_duration(), 2000);
    }

    #[test]
    fn draw_lane_current_duration_with_lanecover() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_section_line(true);
        let model = make_model_with_timelines(vec![tl0], 120.0);
        let mut renderer = LaneRenderer::new(&model);
        renderer.enable_lanecover = true;
        renderer.lanecover = 0.5;

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        renderer.draw_lane(&ctx, &lanes, &[]);

        // region = 2000, currentduration = 2000 * (1 - 0.5) = 1000
        assert_eq!(renderer.current_duration(), 1000);
    }

    #[test]
    fn draw_lane_lift_offset() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_section_line(true);
        let model = make_model_with_timelines(vec![tl0], 120.0);
        let mut renderer = LaneRenderer::new(&model);
        renderer.enable_lift = true;
        renderer.lift = 0.2;

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        // Y-down: hl = region_y + region_height * (1 - lift) = 0 + 500 * 0.8 = 400
        // lift_offset_y = (region_y + region_height) - hl = 500 - 400 = 100
        assert!((result.lift_offset_y - 100.0).abs() < 0.001);
    }

    #[test]
    fn draw_lane_hidden_cover_enabled() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_section_line(true);
        let model = make_model_with_timelines(vec![tl0], 120.0);
        let mut renderer = LaneRenderer::new(&model);
        renderer.enable_hidden = true;
        renderer.hidden = 0.3;

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        assert!(result.hidden_cover.visible);
        // hidden_y = hidden * region_height = 0.3 * 500 = 150
        assert!((result.hidden_cover.y - 150.0).abs() < 0.001);
    }

    #[test]
    fn draw_lane_hidden_cover_disabled() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_section_line(true);
        let model = make_model_with_timelines(vec![tl0], 120.0);
        let mut renderer = LaneRenderer::new(&model);
        renderer.enable_hidden = false;

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        assert!(!result.hidden_cover.visible);
    }

    #[test]
    fn draw_lane_section_line_emitted() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
        tl1.set_section_line(true);
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        // Should contain at least one DrawSectionLine command
        let has_section_line = result
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::DrawSectionLine { .. }));
        assert!(has_section_line, "Expected DrawSectionLine command");
    }

    #[test]
    fn draw_lane_normal_note_emitted() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        // Should contain a DrawNote for lane 0
        let has_note = result.commands.iter().any(|c| {
            matches!(
                c,
                DrawCommand::DrawNote {
                    lane: 0,
                    image_type: NoteImageType::Normal,
                    ..
                }
            )
        });
        assert!(has_note, "Expected DrawNote command for lane 0");
    }

    #[test]
    fn draw_lane_mine_note_emitted() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
        tl1.set_note(2, Some(Note::new_mine(1, 0.5)));
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        let has_mine = result.commands.iter().any(|c| {
            matches!(
                c,
                DrawCommand::DrawNote {
                    lane: 2,
                    image_type: NoteImageType::Mine,
                    ..
                }
            )
        });
        assert!(has_mine, "Expected mine note DrawNote command for lane 2");
    }

    #[test]
    fn draw_lane_past_note_not_shown_without_flag() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        // Note at time 0 is already past when time > 0
        let mut tl1 = make_timeline(0.5, 500_000, 120.0, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let mut ctx = default_ctx(all_tls);
        ctx.time = 1000; // well past the note
        ctx.show_pastnote = false;

        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        // No note commands expected since note is past and show_pastnote is false
        let has_note = result.commands.iter().any(|c| {
            matches!(
                c,
                DrawCommand::DrawNote {
                    image_type: NoteImageType::Normal,
                    ..
                }
            )
        });
        assert!(
            !has_note,
            "Should not draw past notes when show_pastnote is false"
        );
    }

    #[test]
    fn draw_lane_constant_mode_hides_far_notes() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        // Note far in the future
        let mut tl1 = make_timeline(10.0, 10_000_000, 120.0, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);
        renderer.enable_constant = true;
        renderer.duration = 500; // target = 500_000 us
        renderer.constant_fadein_time = 0.0; // alpha_limit = 0

        let all_tls = model.all_time_lines();
        let mut ctx = default_ctx(all_tls);
        ctx.time = 0;

        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        // Far future note (10s) should be hidden in constant mode with duration 500ms
        let has_note = result.commands.iter().any(|c| {
            matches!(
                c,
                DrawCommand::DrawNote {
                    image_type: NoteImageType::Normal,
                    ..
                }
            )
        });
        assert!(
            !has_note,
            "Far future note should be hidden in constant mode"
        );
    }

    #[test]
    fn draw_lane_offsets_accumulated() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        let offsets = vec![
            DrawLaneOffset {
                x: 5.0,
                y: 10.0,
                w: 2.0,
                h: 3.0,
            },
            DrawLaneOffset {
                x: 1.0,
                y: 2.0,
                w: 1.0,
                h: 1.0,
            },
        ];
        let result = renderer.draw_lane(&ctx, &lanes, &offsets);

        // Verify note commands reflect accumulated offsets
        let note_cmd = result.commands.iter().find(|c| {
            matches!(
                c,
                DrawCommand::DrawNote {
                    image_type: NoteImageType::Normal,
                    ..
                }
            )
        });
        assert!(note_cmd.is_some(), "Expected a DrawNote command");
        if let Some(DrawCommand::DrawNote { x, w, .. }) = note_cmd {
            // offset_x = 5+1 = 6, offset_w = 2+1 = 3
            // x = region_x + offset_x = 0 + 6 = 6
            // w = region_width + offset_w = 30 + 3 = 33
            assert!(
                (*x - 6.0).abs() < 0.001,
                "x should include offset, got {}",
                x
            );
            assert!(
                (*w - 33.0).abs() < 0.001,
                "w should include offset, got {}",
                w
            );
        }
    }

    #[test]
    fn draw_lane_practice_mode_uses_start_time() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        tl0.set_section_line(true);
        let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
        tl1.set_note(0, Some(Note::new_normal(1)));
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let mut ctx = default_ctx(all_tls);
        ctx.is_practice = true;
        ctx.practice_start_time = 500; // 500ms

        let lanes = make_lanes(8);
        let _result = renderer.draw_lane(&ctx, &lanes, &[]);

        // In practice mode, hispeed should be 1.0 and pos should be reset to 0
        // The test primarily verifies no panics and correct practice mode behavior
        assert_eq!(renderer.pos, 0);
    }

    #[test]
    fn draw_lane_long_note_emits_body_and_caps() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);

        // Create LN start at tl1, end at tl2
        let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
        let mut tl2 = make_timeline(2.0, 2_000_000, 120.0, 8);

        // We need to set pair indices referencing each other via timeline indices
        // In the model, timelines are stored in order, so tl1 is index 1, tl2 is index 2
        let mut start_note = Note::new_long(1);
        start_note.set_pair_index(Some(2)); // points to tl2's index in all_timelines
        start_note.set_end(false);

        let mut end_note = Note::new_long(1);
        end_note.set_pair_index(Some(1)); // points to tl1's index
        end_note.set_end(true);

        tl1.set_note(0, Some(start_note));
        tl2.set_note(0, Some(end_note));

        let model = make_model_with_timelines(vec![tl0, tl1, tl2], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        // Should have DrawLongNote commands (body + end, and optionally start)
        let ln_count = result
            .commands
            .iter()
            .filter(|c| matches!(c, DrawCommand::DrawLongNote { .. }))
            .count();
        assert!(
            ln_count >= 2,
            "Expected at least 2 DrawLongNote commands (body + end), got {}",
            ln_count
        );
    }

    #[test]
    fn draw_lane_hidden_note_shown_when_enabled() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
        tl1.set_hidden_note(0, Some(Note::new_normal(1)));
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let mut ctx = default_ctx(all_tls);
        ctx.show_hiddennote = true;

        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        let has_hidden = result.commands.iter().any(|c| {
            matches!(
                c,
                DrawCommand::DrawNote {
                    image_type: NoteImageType::Hidden,
                    ..
                }
            )
        });
        assert!(has_hidden, "Expected hidden note DrawNote command");
    }

    #[test]
    fn draw_lane_hidden_note_not_shown_when_disabled() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
        tl1.set_hidden_note(0, Some(Note::new_normal(1)));
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let mut ctx = default_ctx(all_tls);
        ctx.show_hiddennote = false;

        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        let has_hidden = result.commands.iter().any(|c| {
            matches!(
                c,
                DrawCommand::DrawNote {
                    image_type: NoteImageType::Hidden,
                    ..
                }
            )
        });
        assert!(
            !has_hidden,
            "Hidden notes should not be drawn when disabled"
        );
    }

    #[test]
    fn draw_lane_bpm_guide_emitted() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_bpm(120.0);
        tl0.set_section_line(true);
        // tl1 has different BPM
        let mut tl1 = make_timeline(1.0, 1_000_000, 180.0, 8);
        tl1.set_section_line(true);
        let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
        let mut renderer = LaneRenderer::new(&model);

        let all_tls = model.all_time_lines();
        let mut ctx = default_ctx(all_tls);
        ctx.show_bpmguide = true;
        ctx.lane_group_regions = vec![LaneGroupRegion {
            x: 0.0,
            width: 100.0,
        }];

        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        let has_bpm_line = result
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::DrawBpmLine { .. }));
        assert!(has_bpm_line, "Expected BPM line when BPM changes");

        let has_bpm_text = result
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::DrawBpmText { .. }));
        assert!(has_bpm_text, "Expected BPM text when BPM changes");
    }

    #[test]
    fn draw_lane_hidden_cover_with_lift() {
        let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
        tl0.set_section_line(true);
        let model = make_model_with_timelines(vec![tl0], 120.0);
        let mut renderer = LaneRenderer::new(&model);
        renderer.enable_hidden = true;
        renderer.hidden = 0.4;
        renderer.enable_lift = true;
        renderer.lift = 0.2;

        let all_tls = model.all_time_lines();
        let ctx = default_ctx(all_tls);
        let lanes = make_lanes(8);
        let result = renderer.draw_lane(&ctx, &lanes, &[]);

        // hidden_y = (1 - lift) * hidden * region_height = 0.8 * 0.4 * 500 = 160
        assert!(result.hidden_cover.visible);
        assert!(
            (result.hidden_cover.y - 160.0).abs() < 0.001,
            "hidden_y = {}, expected 160",
            result.hidden_cover.y
        );
    }
}

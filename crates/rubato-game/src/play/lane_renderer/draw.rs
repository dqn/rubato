/// Parameters for drawing a long note (CN/HCN/LN).
struct DrawLongNoteParams<'a> {
    pub lane: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub note: &'a Note,
    /// Timeline Vec index of the LN end (pair) note.
    pub pair_tl_idx: usize,
    /// Timeline Vec index of the current (start) note.
    pub note_tl_idx: usize,
}

impl LaneRenderer {
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
    #[allow(clippy::too_many_lines, clippy::needless_range_loop)] // Java-ported lane iteration with parallel arrays
    pub fn draw_lane(
        &mut self,
        ctx: &DrawLaneContext,
        lanes: &[SkinLane],
        offsets: &[DrawLaneOffset],
    ) -> DrawLaneResult {
        let mut commands: Vec<DrawCommand> = Vec::new();

        if lanes.is_empty() {
            log::debug!("draw_lane: lanes is empty, returning default");
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
        // Safety: the source slice (BMSPlayer.model.timelines) outlives this
        // synchronous draw_lane() call.
        let all_tl = unsafe { ctx.all_timelines.as_slice() };

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
            nbpm = tl.bpm;
            nscroll = tl.scroll;
        }
        self.nowbpm = nbpm;

        let region = Self::calc_region(nbpm, hispeed, nscroll);

        // Java/original beatoraja coordinates are Y-up.
        // region_y is the judge-line baseline and region_y + region_height is the top.
        let hu = lanes[0].region_y + lanes[0].region_height;
        let hl = if self.enable_lift {
            lanes[0].region_y + lanes[0].region_height * self.lift
        } else {
            lanes[0].region_y
        };
        let rxhs = (hu - hl) as f64 * hispeed as f64;
        let mut y = hl as f64;

        let lanecover = if self.enable_lanecover {
            self.lanecover
        } else {
            0.0
        };
        self.currentduration = (region * (1.0 - lanecover as f64))
            .round()
            .clamp(i32::MIN as f64, i32::MAX as f64) as i32;

        // Calculate offset results for LIFT, LANECOVER, HIDDEN.
        let lift_offset_y = hl - lanes[0].region_y;
        let lanecover_offset_y = (hl - hu) as f64 * lanecover as f64;

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
                            all_tl[timelines[i - 1]].scroll
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
            if y > hu as f64 {
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
                    && (tl.milli_time() / 1000) > (all_tl[timelines[i - 1]].milli_time() / 1000)
                {
                    commands.push(DrawCommand::DrawTimeLine {
                        y_offset: (y - hl as f64) as i32,
                    });
                    for r in &ctx.lane_group_regions {
                        commands.push(DrawCommand::DrawTimeText {
                            text: format!(
                                "{:2}:{:02}.{:1}",
                                tl.milli_time() / 60000,
                                (tl.milli_time() / 1000) % 60,
                                (tl.milli_time() / 100) % 10
                            ),
                            x: r.x + 4.0,
                            y: y as f32 + 20.0,
                        });
                    }
                }

                // BPM guide / Stop lines
                if ctx.show_bpmguide || show_timeline {
                    if tl.bpm != nbpm {
                        commands.push(DrawCommand::DrawBpmLine {
                            y_offset: (y - hl as f64) as i32,
                            bpm: tl.bpm,
                        });
                        for r in &ctx.lane_group_regions {
                            commands.push(DrawCommand::DrawBpmText {
                                text: format!("BPM{}", tl.bpm as i32),
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
                if tl.section_line {
                    commands.push(DrawCommand::DrawSectionLine {
                        y_offset: (y - hl as f64) as i32,
                    });
                }

                nbpm = tl.bpm;
            } else if self.pos == i.wrapping_sub(1) {
                // Advance pos: check if all notes in this timeline are past
                let mut can_advance = true;
                for lane in 0..lanes.len() {
                    let note = tl.note(lane as i32);
                    if let Some(note) = note {
                        match note {
                            Note::Long { end, pair, .. } => {
                                // For LN: check if the end note's time is still visible.
                                // Java: (ln.isEnd() ? ln : ln.getPair()).getMicroTime()
                                // always uses the end note's time.
                                let end_time = if *end {
                                    // This IS the end note; use its own time
                                    tl.micro_time()
                                } else {
                                    // This is the start note; use pair (end) time
                                    if let Some(pair_tl_idx) = pair {
                                        all_tl[*pair_tl_idx].micro_time()
                                    } else {
                                        continue;
                                    }
                                };
                                if end_time >= microtime {
                                    can_advance = false;
                                    break;
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
            if y > hu as f64 {
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
                                        let mut prev_tl_actual_idx = timelines[i];

                                        for j in (i + 1)..tl_count {
                                            let now_tl = &all_tl[timelines[j]];
                                            if prev_tl_actual_idx >= *pair_tl_idx {
                                                break;
                                            }
                                            if now_tl.micro_time() >= microtime {
                                                if prev_tl_ref.micro_time()
                                                    + prev_tl_ref.micro_stop()
                                                    > microtime
                                                {
                                                    dy += (now_tl.section()
                                                        - prev_tl_ref.section())
                                                        * prev_tl_ref.scroll
                                                        * rxhs;
                                                } else {
                                                    let time_diff = now_tl.micro_time() - microtime;
                                                    let total_time = now_tl.micro_time()
                                                        - prev_tl_ref.micro_time()
                                                        - prev_tl_ref.micro_stop();
                                                    if total_time != 0 {
                                                        dy += (now_tl.section()
                                                            - prev_tl_ref.section())
                                                            * prev_tl_ref.scroll
                                                            * (time_diff as f64
                                                                / total_time as f64)
                                                            * rxhs;
                                                    }
                                                }
                                            }
                                            prev_tl_ref = now_tl;
                                            prev_tl_actual_idx = timelines[j];
                                        }

                                        if dy > 0.0 {
                                            let dscale = if dsth > scale {
                                                (dsth - scale) / 2.0
                                            } else {
                                                0.0
                                            };
                                            // Smoothly reduce body height as the start
                                            // note scrolls past the judge line, avoiding
                                            // single-frame flicker from float jitter at
                                            // the boundary. The visible portion equals the
                                            // full body minus the distance below the line.
                                            let ln_height = (dy as f32 + dsty
                                                - (lanes[lane].region_y - dscale))
                                                .clamp(0.0, dy as f32);
                                            self.draw_long_note_commands(
                                                &mut commands,
                                                ctx,
                                                &DrawLongNoteParams {
                                                    lane,
                                                    x: dstx,
                                                    y: dsty + dy as f32,
                                                    width: dstw,
                                                    height: ln_height,
                                                    scale: dsth,
                                                    note,
                                                    pair_tl_idx: *pair_tl_idx,
                                                    note_tl_idx: timelines[i],
                                                },
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
            let bad_time = ctx.bad_judge_time.saturating_abs();
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
                            > tl.micro_time().saturating_add(tl.micro_stop()).saturating_add(bad_time)
                        {
                            let bad_deadline = tl.micro_time().saturating_add(tl.micro_stop()).saturating_add(bad_time);
                            let stop_time = 0i64.max(
                                bad_deadline
                                    - all_tl[timelines[j]].micro_time()
                                    - all_tl[timelines[j]].micro_stop(),
                            );
                            y -= (all_tl[timelines[j + 1]].micro_time()
                                - all_tl[timelines[j]].micro_time()
                                - all_tl[timelines[j]].micro_stop()
                                - stop_time) as f64
                                * rxhs2
                                * all_tl[timelines[j]].bpm
                                / 240000000.0;
                        }
                        j += 1;
                    }
                    if all_tl[timelines[j]].micro_time() + all_tl[timelines[j]].micro_stop()
                        < microtime
                        && microtime > tl.micro_time().saturating_add(tl.micro_stop()).saturating_add(bad_time)
                    {
                        let bad_deadline = tl.micro_time().saturating_add(tl.micro_stop()).saturating_add(bad_time);
                        let stop_time = 0i64.max(
                            bad_deadline
                                - all_tl[timelines[j]].micro_time()
                                - all_tl[timelines[j]].micro_stop(),
                        );
                        y -= (microtime
                            - all_tl[timelines[j]].micro_time()
                            - all_tl[timelines[j]].micro_stop()
                            - stop_time) as f64
                            * rxhs2
                            * all_tl[timelines[j]].bpm
                            / 240000000.0;
                    }
                } else if tl.micro_time() + tl.micro_stop() < microtime
                    && microtime > tl.micro_time().saturating_add(tl.micro_stop()).saturating_add(bad_time)
                {
                    // Algebraically: max(0, micro_time + micro_stop + bad_time - micro_time - micro_stop)
                    // simplifies to max(0, bad_time). Since bad_time is already non-negative, this is just bad_time.
                    let stop_time = bad_time;
                    y -= (microtime - tl.micro_time() - tl.micro_stop() - stop_time) as f64
                        * rxhs2
                        * tl.bpm
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
    fn draw_long_note_commands(
        &self,
        commands: &mut Vec<DrawCommand>,
        ctx: &DrawLaneContext,
        params: &DrawLongNoteParams<'_>,
    ) {
        let lane = params.lane;
        let x = params.x;
        let y = params.y;
        let width = params.width;
        let height = params.height;
        let scale = params.scale;
        let note = params.note;
        let pair_tl_idx = params.pair_tl_idx;
        let Note::Long { note_type, .. } = note else {
            return;
        };

        let note_tl_idx = params.note_tl_idx;
        let is_processing =
            ctx.processing_long_notes.get(lane).copied().flatten() == Some(pair_tl_idx);
        let is_passing =
            ctx.passing_long_notes.get(lane).copied().flatten() == Some(note_tl_idx);
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
            // Clamp body height: when the LN span is shorter than one note image (height < scale),
            // the body shrinks to zero. Without clamping, h = height - scale goes negative,
            // producing an inverted/flipped quad at the lane-cover boundary.
            let body_h = (height - scale).max(0.0);
            if body_h > 0.0 {
                commands.push(DrawCommand::DrawLongNote {
                    lane,
                    x,
                    y: y - height + scale,
                    w: width,
                    h: body_h,
                    image_index: body_idx,
                });
            }
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
            // Clamp body height: see HCN branch for rationale.
            let body_h = (height - scale).max(0.0);
            if body_h > 0.0 {
                commands.push(DrawCommand::DrawLongNote {
                    lane,
                    x,
                    y: y - height + scale,
                    w: width,
                    h: body_h,
                    image_index: body_idx,
                });
            }
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
            // Clamp body height: see HCN branch for rationale.
            let body_h = (height - scale).max(0.0);
            if body_h > 0.0 {
                commands.push(DrawCommand::DrawLongNote {
                    lane,
                    x,
                    y: y - height + scale,
                    w: width,
                    h: body_h,
                    image_index: body_idx,
                });
            }
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

    #[cfg(test)]
    pub(crate) fn base_bpm(&self) -> f64 {
        self.basebpm
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

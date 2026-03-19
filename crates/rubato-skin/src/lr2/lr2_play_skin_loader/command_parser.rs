use crate::lr2::lr2_skin_csv_loader::LR2SkinCSVLoaderState;
use crate::lr2::lr2_skin_loader;
use crate::reexports::{Rectangle, Texture, TextureRegion};
use crate::safe_div_f32;
use crate::skin_image::SkinImage;
use crate::skin_object::DestinationParams;

use super::{LR2PlaySkinLoaderState, PmCharaEntry, SkinSourceData};

/// Note target types for add_note dispatch
enum NoteTarget {
    Note,
    LnStart,
    LnEnd,
    LnBody,
    LnBodyA,
    HcnStart,
    HcnEnd,
    HcnBody,
    HcnBodyA,
    HcnBodyD,
    HcnBodyR,
    Mine,
}

impl LR2PlaySkinLoaderState {
    pub fn process_play_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            // PlayCommand enum entries
            "CLOSE" => {
                if str_parts.len() > 1 {
                    self.play_close = str_parts[1].trim().parse().ok();
                }
            }
            "PLAYSTART" => {
                if str_parts.len() > 1 {
                    self.play_playstart = str_parts[1].trim().parse().ok();
                }
            }
            "LOADSTART" => {
                if str_parts.len() > 1 {
                    self.play_loadstart = str_parts[1].trim().parse().ok();
                }
            }
            "LOADEND" => {
                if str_parts.len() > 1 {
                    self.play_loadend = str_parts[1].trim().parse().ok();
                }
            }
            "FINISHMARGIN" => {
                if str_parts.len() > 1 {
                    self.play_finish_margin = str_parts[1].trim().parse().ok();
                }
            }
            "JUDGETIMER" => {
                if str_parts.len() > 1 {
                    self.play_judgetimer = str_parts[1].trim().parse().ok();
                }
            }
            "SRC_BGA" => {
                // In Java: bga = new SkinBGA(c.getBgaExpand()); skin.add(bga)
                // SkinBgaObject is created and added to the skin by the caller.
                // Here we just signal that BGA was requested.
                self.bga = true;
            }
            "DST_BGA" => {
                // DST_BGA sets the destination for the BGA object.
                // In Java: skin.setDestination(bga, 0, values[3], srch - values[4] - values[6], values[5], values[6], ...)
                // The destination is applied by the caller using the skin's setDestination.
                // We store the raw values for the caller.
                if self.bga {
                    let _values = lr2_skin_loader::parse_int(str_parts);
                    // BGA destination will be applied by caller when connecting to Skin
                }
            }
            "SRC_LINE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if let Some(images) = self.csv.source_image(&values) {
                    let idx = values[1] as usize;
                    if idx < self.line_images.len() {
                        let skin_image =
                            SkinImage::new_with_int_timer(images, values[10], values[9]);
                        self.line_images[idx] = Some(skin_image);
                    }
                }
            }
            "DST_LINE" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                let idx = values[1] as usize;
                if idx < self.line_images.len() && self.line_images[idx].is_some() {
                    if values[5] < 0 {
                        values[3] += values[5];
                        values[5] = -values[5];
                    }
                    if values[6] < 0 {
                        values[4] += values[6];
                        values[6] = -values[6];
                    }
                    let offset = LR2SkinCSVLoaderState::read_offset_with_base(
                        str_parts,
                        21,
                        &[crate::skin_property::OFFSET_LIFT],
                    );
                    if let Some(ref mut li) = self.line_images[idx] {
                        li.data.set_destination_with_int_timer_and_offsets(
                            &DestinationParams {
                                time: values[2] as i64,
                                x: values[3] as f32 * safe_div_f32(self.dstw, self.srcw),
                                y: self.dsth
                                    - (values[4] + values[6]) as f32
                                        * safe_div_f32(self.dsth, self.srch),
                                w: values[5] as f32 * safe_div_f32(self.dstw, self.srcw),
                                h: values[6] as f32 * safe_div_f32(self.dsth, self.srch),
                                acc: values[7],
                                a: values[8],
                                r: values[9],
                                g: values[10],
                                b: values[11],
                                blend: values[12],
                                filter: values[13],
                                angle: values[14],
                                center: values[15],
                                loop_val: values[16],
                            },
                            values[17],
                            values[18],
                            values[19],
                            values[20],
                            &offset,
                        );
                    }
                    // Update player rectangle
                    let player_idx = idx % 2;
                    if player_idx < self.playerr.len() && self.playerr[player_idx].is_some() {
                        self.playerr[player_idx] = Some(Rectangle::new(
                            values[3] as f32 * safe_div_f32(self.dstw, self.srcw),
                            self.dsth
                                - (values[4] + values[6]) as f32
                                    * safe_div_f32(self.dsth, self.srch),
                            values[5] as f32 * safe_div_f32(self.dstw, self.srcw),
                            (values[4] + values[6]) as f32 * safe_div_f32(self.dsth, self.srch),
                        ));
                    }
                    self.linevalues[idx % 2] = Some(str_parts.to_vec());
                }
            }
            "SRC_NOTE" => {
                self.add_note(str_parts, NoteTarget::Note, true);
            }
            "SRC_LN_END" => {
                self.add_note(str_parts, NoteTarget::LnEnd, true);
            }
            "SRC_LN_START" => {
                self.add_note(str_parts, NoteTarget::LnStart, true);
            }
            "SRC_LN_BODY" => {
                self.add_note(str_parts, NoteTarget::LnBody, false);
                self.add_note(str_parts, NoteTarget::LnBodyA, true);
            }
            "SRC_LN_BODY_INACTIVE" => {
                self.add_note(str_parts, NoteTarget::LnBody, true);
            }
            "SRC_LN_BODY_ACTIVE" => {
                self.add_note(str_parts, NoteTarget::LnBodyA, true);
            }
            "SRC_HCN_END" => {
                self.add_note(str_parts, NoteTarget::HcnEnd, true);
            }
            "SRC_HCN_START" => {
                self.add_note(str_parts, NoteTarget::HcnStart, true);
            }
            "SRC_HCN_BODY" => {
                self.add_note(str_parts, NoteTarget::HcnBody, false);
                self.add_note(str_parts, NoteTarget::HcnBodyA, true);
            }
            "SRC_HCN_BODY_INACTIVE" => {
                self.add_note(str_parts, NoteTarget::HcnBody, true);
            }
            "SRC_HCN_BODY_ACTIVE" => {
                self.add_note(str_parts, NoteTarget::HcnBodyA, true);
            }
            "SRC_HCN_DAMAGE" => {
                self.add_note(str_parts, NoteTarget::HcnBodyD, true);
            }
            "SRC_HCN_REACTIVE" => {
                self.add_note(str_parts, NoteTarget::HcnBodyR, true);
            }
            "SRC_MINE" => {
                self.add_note(str_parts, NoteTarget::Mine, true);
            }
            "DST_NOTE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if let Some(ref mode) = self.mode {
                    let mut lane = values[1];
                    if lane % 10 == 0 {
                        lane = if mode.scratch_key().len() > (lane / 10) as usize {
                            mode.scratch_key()[(lane / 10) as usize]
                        } else {
                            -1
                        };
                    } else {
                        // .max(1) guards against division by zero when playerr is
                        // empty (e.g. before DST_PLAYER is parsed).
                        let player_count = self.playerr.len().max(1) as i32;
                        let offset = (lane / 10) * (self.laner.len() as i32 / player_count);
                        lane = if lane > 10 { lane - 11 } else { lane - 1 };
                        if lane
                            >= (self.laner.len() as i32 - mode.scratch_key().len() as i32)
                                / player_count
                        {
                            lane = -1;
                        } else {
                            lane += offset;
                        }
                    }
                    if lane < 0 {
                        return;
                    }
                    let lane = lane as usize;
                    if lane < self.laner.len() && self.laner[lane].is_none() {
                        self.laner[lane] = Some(Rectangle::new(
                            values[3] as f32 * safe_div_f32(self.dstw, self.srcw),
                            self.dsth
                                - (values[4] + values[6]) as f32
                                    * safe_div_f32(self.dsth, self.srch),
                            values[5] as f32 * safe_div_f32(self.dstw, self.srcw),
                            (values[4] + values[6]) as f32 * safe_div_f32(self.dsth, self.srch),
                        ));
                        self.scale[lane] = values[6] as f32 * safe_div_f32(self.dsth, self.srch);
                    }
                    if !self.lanerender {
                        // Fill in missing HCN sources from LN sources
                        for (hcn, ln) in self.hcnend.iter_mut().zip(self.lnend.iter()) {
                            if hcn.is_none() {
                                *hcn = ln.clone();
                            }
                        }
                        for (hcn, ln) in self.hcnstart.iter_mut().zip(self.lnstart.iter()) {
                            if hcn.is_none() {
                                *hcn = ln.clone();
                            }
                        }
                        for (hcn, ln) in self.hcnbody.iter_mut().zip(self.lnbody.iter()) {
                            if hcn.is_none() {
                                *hcn = ln.clone();
                            }
                        }
                        for (hcn, ln) in self.hcnbodya.iter_mut().zip(self.lnbodya.iter()) {
                            if hcn.is_none() {
                                *hcn = ln.clone();
                            }
                        }
                        // hcnbodyd falls back to hcnbody
                        for (hcnd, hcn) in self.hcnbodyd.iter_mut().zip(self.hcnbody.iter()) {
                            if hcnd.is_none() {
                                *hcnd = hcn.clone();
                            }
                        }
                        // hcnbodyr falls back to hcnbodya
                        for (hcnr, hcna) in self.hcnbodyr.iter_mut().zip(self.hcnbodya.iter()) {
                            if hcnr.is_none() {
                                *hcnr = hcna.clone();
                            }
                        }
                        // lanerender = new SkinNote(note, lnss, mine)
                        self.lanerender = true;
                        // lanerender.setOffsetID(readOffset(str, 21, new int[]{OFFSET_NOTES_1P}))
                        // skin.add(lanerender)
                    }
                }
            }
            "DST_NOTE2" => {
                if str_parts.len() > 1 {
                    let y: i32 = str_parts[1].trim().parse().unwrap_or(0);
                    let val = (self.dsth
                        - (y as f32 * safe_div_f32(self.dsth, self.srch) + self.scale[0]))
                        as i32;
                    self.dstnote2.fill(val);
                }
            }
            "DST_NOTE_EXPANSION_RATE" => {
                if str_parts.len() > 2 {
                    let w: i32 = str_parts[1].trim().parse().unwrap_or(100);
                    let h: i32 = str_parts[2].trim().parse().unwrap_or(100);
                    self.play_note_expansion_rate = Some([w, h]);
                }
            }
            "SRC_NOWJUDGE_1P" | "SRC_NOWJUDGE_2P" | "SRC_NOWJUDGE_3P" => {
                let player = match cmd {
                    "SRC_NOWJUDGE_1P" => 0usize,
                    "SRC_NOWJUDGE_2P" => 1,
                    _ => 2,
                };
                let values = lr2_skin_loader::parse_int(str_parts);
                if let Some(images) = self.csv.source_image(&values) {
                    if self.judge_objects[player].is_none() {
                        let shift = values[11] != 1;
                        self.judge_objects[player] = Some(
                            crate::skin_judge_object::SkinJudgeObject::new(player as i32, shift),
                        );
                    }
                    // Map LR2 judge index: values[1] <= 5 -> (5 - values[1]), else values[1]
                    let judge_idx = if values[1] <= 5 {
                        (5 - values[1]) as usize
                    } else {
                        values[1] as usize
                    };
                    if let Some(ref mut judge_obj) = self.judge_objects[player] {
                        let judge_image =
                            SkinImage::new_with_int_timer(images, values[10], values[9]);
                        judge_obj.inner.set_judge(judge_idx);
                        judge_obj.set_judge_image(judge_idx, judge_image);
                    }
                }
            }
            "DST_NOWJUDGE_1P" | "DST_NOWJUDGE_2P" | "DST_NOWJUDGE_3P" => {
                let player = match cmd {
                    "DST_NOWJUDGE_1P" => 0usize,
                    "DST_NOWJUDGE_2P" => 1,
                    _ => 2,
                };
                let judge_idx_raw: i32 = str_parts
                    .get(1)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let judge_idx = if judge_idx_raw <= 5 {
                    (5 - judge_idx_raw) as usize
                } else {
                    judge_idx_raw as usize
                };
                if self.judge_objects[player]
                    .as_ref()
                    .is_some_and(|j| j.inner.judge(judge_idx))
                {
                    let mut values = lr2_skin_loader::parse_int(str_parts);
                    if values[5] < 0 {
                        values[3] += values[5];
                        values[5] = -values[5];
                    }
                    if values[6] < 0 {
                        values[4] += values[6];
                        values[6] = -values[6];
                    }
                    if let Some(ref mut judge_obj) = self.judge_objects[player]
                        && let Some(img) = judge_obj.judge_image_mut(judge_idx)
                    {
                        let offset_judge = match player {
                            0 => crate::skin_property::OFFSET_JUDGE_1P,
                            1 => crate::skin_property::OFFSET_JUDGE_2P,
                            _ => crate::skin_property::OFFSET_JUDGE_3P,
                        };
                        img.data.set_destination_with_int_timer_and_offsets(
                            &DestinationParams {
                                time: values[2] as i64,
                                x: values[3] as f32 * safe_div_f32(self.dstw, self.srcw),
                                y: self.dsth
                                    - (values[4] + values[6]) as f32
                                        * safe_div_f32(self.dsth, self.srch),
                                w: values[5] as f32 * safe_div_f32(self.dstw, self.srcw),
                                h: values[6] as f32 * safe_div_f32(self.dsth, self.srch),
                                acc: values[7],
                                a: values[8],
                                r: values[9],
                                g: values[10],
                                b: values[11],
                                blend: values[12],
                                filter: values[13],
                                angle: values[14],
                                center: values[15],
                                loop_val: values[16],
                            },
                            values[17],
                            values[18],
                            values[19],
                            values[20],
                            &[offset_judge, crate::skin_property::OFFSET_LIFT],
                        );
                    }
                }
            }
            "SRC_NOWCOMBO_1P" | "SRC_NOWCOMBO_2P" | "SRC_NOWCOMBO_3P" => {
                let player = match cmd {
                    "SRC_NOWCOMBO_1P" => 0usize,
                    "SRC_NOWCOMBO_2P" => 1,
                    _ => 2,
                };
                let values = lr2_skin_loader::parse_int(str_parts);
                let divx = if values[7] > 0 { values[7] } else { 1 };
                let divy = if values[8] > 0 { values[8] } else { 1 };
                if let Some(simages) = self.csv.source_image(&values) {
                    let required = (divx * divy) as usize;
                    if simages.len() < required {
                        log::warn!(
                            "SRC_NOWCOMBO: image count {} < divx*divy {}; skipping",
                            simages.len(),
                            required,
                        );
                        return;
                    }
                    // Rearrange flat images into [divy][divx] grid
                    let images_2d: Vec<Vec<TextureRegion>> = (0..divy)
                        .map(|j| {
                            (0..divx)
                                .map(|i| simages[(j * divx + i) as usize].clone())
                                .collect()
                        })
                        .collect();
                    let judge_idx = if values[1] <= 5 {
                        (5 - values[1]) as usize
                    } else {
                        values[1] as usize
                    };
                    if let Some(ref mut judge_obj) = self.judge_objects[player] {
                        let number = crate::objects::skin_number::SkinNumber::new_with_int_timer(
                            images_2d,
                            None,
                            values[10],
                            values[9],
                            crate::objects::skin_number::NumberDisplayConfig {
                                keta: 0,
                                zeropadding: 0,
                                space: 0,
                                align: 2,
                            },
                            0,
                        );
                        judge_obj.inner.set_judge_count(judge_idx);
                        judge_obj.set_judge_count(judge_idx, number);
                    }
                }
            }
            "DST_NOWCOMBO_1P" | "DST_NOWCOMBO_2P" | "DST_NOWCOMBO_3P" => {
                let player = match cmd {
                    "DST_NOWCOMBO_1P" => 0usize,
                    "DST_NOWCOMBO_2P" => 1,
                    _ => 2,
                };
                let judge_idx_raw: i32 = str_parts
                    .get(1)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let judge_idx = if judge_idx_raw <= 5 {
                    (5 - judge_idx_raw) as usize
                } else {
                    judge_idx_raw as usize
                };
                if let Some(ref mut judge_obj) = self.judge_objects[player]
                    && judge_obj.inner.judge_count(judge_idx)
                {
                    let mut values = lr2_skin_loader::parse_int(str_parts);
                    if values[5] < 0 {
                        values[3] += values[5];
                        values[5] = -values[5];
                    }
                    if values[6] < 0 {
                        values[4] += values[6];
                        values[6] = -values[6];
                    }
                    if let Some(num) = judge_obj.judge_count_mut(judge_idx) {
                        let offset_judge = match player {
                            0 => crate::skin_property::OFFSET_JUDGE_1P,
                            1 => crate::skin_property::OFFSET_JUDGE_2P,
                            _ => crate::skin_property::OFFSET_JUDGE_3P,
                        };
                        // Java: setRelative(true) + negative y for combo count
                        // positioned relative to judge image
                        num.data.relative = true;
                        num.data.set_destination_with_int_timer_and_offsets(
                            &DestinationParams {
                                time: values[2] as i64,
                                x: values[3] as f32 * safe_div_f32(self.dstw, self.srcw),
                                y: -(values[4] as f32) * safe_div_f32(self.dsth, self.srch),
                                w: values[5] as f32 * safe_div_f32(self.dstw, self.srcw),
                                h: values[6] as f32 * safe_div_f32(self.dsth, self.srch),
                                acc: values[7],
                                a: values[8],
                                r: values[9],
                                g: values[10],
                                b: values[11],
                                blend: values[12],
                                filter: values[13],
                                angle: values[14],
                                center: values[15],
                                loop_val: values[16],
                            },
                            values[17],
                            values[18],
                            values[19],
                            values[20],
                            &[offset_judge, crate::skin_property::OFFSET_LIFT],
                        );
                    }
                }
            }
            "SRC_JUDGELINE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if let Some(images) = self.csv.source_image(&values) {
                    self.judgeline =
                        Some(SkinImage::new_with_int_timer(images, values[10], values[9]));
                }
            }
            "DST_JUDGELINE" => {
                if self.judgeline.is_some() {
                    let mut values = lr2_skin_loader::parse_int(str_parts);
                    if values[5] < 0 {
                        values[3] += values[5];
                        values[5] = -values[5];
                    }
                    if values[6] < 0 {
                        values[4] += values[6];
                        values[6] = -values[6];
                    }
                    let offset = LR2SkinCSVLoaderState::read_offset_with_base(
                        str_parts,
                        21,
                        &[crate::skin_property::OFFSET_LIFT],
                    );
                    if let Some(ref mut jl) = self.judgeline {
                        jl.data.set_destination_with_int_timer_and_offsets(
                            &DestinationParams {
                                time: values[2] as i64,
                                x: values[3] as f32 * safe_div_f32(self.dstw, self.srcw),
                                y: self.dsth
                                    - (values[4] + values[6]) as f32
                                        * safe_div_f32(self.dsth, self.srch),
                                w: values[5] as f32 * safe_div_f32(self.dstw, self.srcw),
                                h: values[6] as f32 * safe_div_f32(self.dsth, self.srch),
                                acc: values[7],
                                a: values[8],
                                r: values[9],
                                g: values[10],
                                b: values[11],
                                blend: values[12],
                                filter: values[13],
                                angle: values[14],
                                center: values[15],
                                loop_val: values[16],
                            },
                            values[17],
                            values[18],
                            values[19],
                            values[20],
                            &offset,
                        );
                    }
                }
            }
            "SRC_NOTECHART_1P" => {
                lr2_skin_loader::process_src_notechart(
                    str_parts,
                    &mut self.gauge,
                    &mut self.noteobj,
                );
            }
            "DST_NOTECHART_1P" => {
                lr2_skin_loader::process_dst_notechart(
                    str_parts,
                    self.csv.src.height,
                    self.csv.dst.width,
                    self.csv.dst.height,
                    self.csv.src.width,
                    &mut self.gauge,
                    &mut self.noteobj,
                );
            }
            "SRC_BPMCHART" => {
                lr2_skin_loader::process_src_bpmchart(
                    str_parts,
                    &mut self.gauge,
                    &mut self.bpmgraphobj,
                );
            }
            "DST_BPMCHART" => {
                lr2_skin_loader::process_dst_bpmchart(
                    str_parts,
                    self.csv.src.height,
                    self.csv.dst.width,
                    self.csv.dst.height,
                    self.csv.src.width,
                    &mut self.gauge,
                    &mut self.bpmgraphobj,
                );
            }
            "SRC_TIMING_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                // timingobj = new SkinTimingVisualizer(...)
                self.gauge = Rectangle::new(0.0, 0.0, values[4] as f32, values[5] as f32);
                // skin.add(timingobj)
            }
            "DST_TIMING_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(timingobj, ...)
            }
            "SRC_HIDDEN" | "SRC_LIFT" => {
                self.hidden = None;
                let values = lr2_skin_loader::parse_int(str_parts);
                if let Some(images) = self.csv.source_image(&values) {
                    let mut h = crate::objects::skin_hidden::SkinHidden::new_with_int_timer(
                        images, values[10], values[9],
                    );
                    // Java: if(str[11].length() > 0 && values[11] > 0) hidden.setDisapearLine(dsth - values[11] * dsth / srch)
                    let str11 = str_parts.get(11).map(|s| s.trim()).unwrap_or("");
                    if !str11.is_empty() && values[11] > 0 {
                        let dsth = safe_div_f32(self.dsth, self.srch);
                        h.set_disapear_line(self.dsth - values[11] as f32 * dsth);
                    }
                    // Java: hidden.setDisapearLineLinkLift(str[12].length() == 0 || values[12] != 0)
                    let str12 = str_parts.get(12).map(|s| s.trim()).unwrap_or("");
                    h.is_disapear_line_link_lift = str12.is_empty() || values[12] != 0;
                    self.hidden = Some(h);
                }
            }
            "DST_HIDDEN" => {
                if let Some(ref mut hidden) = self.hidden {
                    let mut values = lr2_skin_loader::parse_int(str_parts);
                    if values[5] < 0 {
                        values[3] += values[5];
                        values[5] = -values[5];
                    }
                    if values[6] < 0 {
                        values[4] += values[6];
                        values[6] = -values[6];
                    }
                    let offset = LR2SkinCSVLoaderState::read_offset_with_base(
                        str_parts,
                        21,
                        &[
                            crate::skin_property::OFFSET_LIFT,
                            crate::skin_property::OFFSET_HIDDEN_COVER,
                        ],
                    );
                    hidden.data.set_destination_with_int_timer_and_offsets(
                        &DestinationParams {
                            time: values[2] as i64,
                            x: values[3] as f32 * safe_div_f32(self.dstw, self.srcw),
                            y: self.dsth
                                - (values[4] + values[6]) as f32
                                    * safe_div_f32(self.dsth, self.srch),
                            w: values[5] as f32 * safe_div_f32(self.dstw, self.srcw),
                            h: values[6] as f32 * safe_div_f32(self.dsth, self.srch),
                            acc: values[7],
                            a: values[8],
                            r: values[9],
                            g: values[10],
                            b: values[11],
                            blend: values[12],
                            filter: values[13],
                            angle: values[14],
                            center: values[15],
                            loop_val: values[16],
                        },
                        values[17],
                        values[18],
                        values[19],
                        values[20],
                        &offset,
                    );
                    // Store the Y coordinate for lane cover position calculation.
                    // Java: getLaneCoverPosition() returns last destination's y.
                    let dsth = safe_div_f32(self.csv.dst.height, self.csv.src.height);
                    self.lane_cover_dst_y =
                        self.csv.dst.height - (values[4] + values[6]) as f32 * dsth;
                }
            }
            "DST_LIFT" => {
                if let Some(ref mut hidden) = self.hidden {
                    let mut values = lr2_skin_loader::parse_int(str_parts);
                    if values[5] < 0 {
                        values[3] += values[5];
                        values[5] = -values[5];
                    }
                    if values[6] < 0 {
                        values[4] += values[6];
                        values[6] = -values[6];
                    }
                    let offset = LR2SkinCSVLoaderState::read_offset_with_base(
                        str_parts,
                        21,
                        &[crate::skin_property::OFFSET_LIFT],
                    );
                    hidden.data.set_destination_with_int_timer_and_offsets(
                        &DestinationParams {
                            time: values[2] as i64,
                            x: values[3] as f32 * safe_div_f32(self.dstw, self.srcw),
                            y: self.dsth
                                - (values[4] + values[6]) as f32
                                    * safe_div_f32(self.dsth, self.srch),
                            w: values[5] as f32 * safe_div_f32(self.dstw, self.srcw),
                            h: values[6] as f32 * safe_div_f32(self.dsth, self.srch),
                            acc: values[7],
                            a: values[8],
                            r: values[9],
                            g: values[10],
                            b: values[11],
                            blend: values[12],
                            filter: values[13],
                            angle: values[14],
                            center: values[15],
                            loop_val: values[16],
                        },
                        values[17],
                        values[18],
                        values[19],
                        values[20],
                        &offset,
                    );
                }
            }
            "DST_PM_CHARA_1P" | "DST_PM_CHARA_2P" => {
                // Play: judge-linked character display
                // x,y,w,h,color,offset,folderpath
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[3] < 0 {
                    values[1] += values[3];
                    values[3] = -values[3];
                }
                if values[4] < 0 {
                    values[2] += values[4];
                    values[4] = -values[4];
                }
                let _imagefile = lr2_skin_loader::lr2_path(
                    &self.csv.skinpath,
                    str_parts.get(7).map_or("", |s| s.as_str()),
                    &self.csv.filemap,
                );
                let _side: i32 = if cmd == "DST_PM_CHARA_1P" { 1 } else { 2 };
                let _color = if values[5] == 1 || values[5] == 2 {
                    values[5]
                } else {
                    1
                };
                let _dstx = values[1] as f32 * safe_div_f32(self.dstw, self.srcw);
                let _dsty =
                    self.dsth - (values[2] + values[4]) as f32 * safe_div_f32(self.dsth, self.srch);
                let _dstw = values[3] as f32 * safe_div_f32(self.dstw, self.srcw);
                let _dsth_val = values[4] as f32 * safe_div_f32(self.dsth, self.srch);
                self.pmchara_entries.push(PmCharaEntry::Chara {
                    side: _side,
                    imagefile: _imagefile,
                    color: _color,
                    dst: Rectangle {
                        x: _dstx,
                        y: _dsty,
                        width: _dstw,
                        height: _dsth_val,
                    },
                });
            }
            "DST_PM_CHARA_ANIMATION" => {
                // Non-play: not judge-linked
                // x,y,w,h,color,animationtype,timer,op1,op2,op3,offset,folderpath
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[6] >= 0 && values[6] <= 9 {
                    if values[3] < 0 {
                        values[1] += values[3];
                        values[3] = -values[3];
                    }
                    if values[4] < 0 {
                        values[2] += values[4];
                        values[4] = -values[4];
                    }
                    let _imagefile = lr2_skin_loader::lr2_path(
                        &self.csv.skinpath,
                        str_parts.get(12).map_or("", |s| s.as_str()),
                        &self.csv.filemap,
                    );
                    let _load_type = values[6] + 6;
                    let _color = if values[5] == 1 || values[5] == 2 {
                        values[5]
                    } else {
                        1
                    };
                    let _dstx = values[1] as f32 * safe_div_f32(self.dstw, self.srcw);
                    let _dsty = self.dsth
                        - (values[2] + values[4]) as f32 * safe_div_f32(self.dsth, self.srch);
                    let _dstw = values[3] as f32 * safe_div_f32(self.dstw, self.srcw);
                    let _dsth_val = values[4] as f32 * safe_div_f32(self.dsth, self.srch);
                    self.pmchara_entries.push(PmCharaEntry::Animation {
                        load_type: _load_type,
                        imagefile: _imagefile,
                        color: _color,
                        dst: Rectangle {
                            x: _dstx,
                            y: _dsty,
                            width: _dstw,
                            height: _dsth_val,
                        },
                        timer: values[7],
                    });
                }
            }
            "SRC_PM_CHARA_IMAGE" => {
                // color,type,folderpath
                // type 0:background 1:name 2:face upper 3:face all 4:icon
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[2] >= 0 && values[2] <= 4 {
                    let _imagefile = lr2_skin_loader::lr2_path(
                        &self.csv.skinpath,
                        str_parts.get(3).map_or("", |s| s.as_str()),
                        &self.csv.filemap,
                    );
                    let _load_type = values[2] + 1;
                    let _color = if values[1] == 1 || values[1] == 2 {
                        values[1]
                    } else {
                        1
                    };
                    self.pmchara_entries.push(PmCharaEntry::SrcImage {
                        load_type: _load_type,
                        imagefile: _imagefile,
                        color: _color,
                    });
                }
            }
            "DST_PM_CHARA_IMAGE" => {
                // Same as DST_IMAGE
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let dstx = values[3] as f32 * safe_div_f32(self.dstw, self.srcw);
                let dsty =
                    self.dsth - (values[4] + values[6]) as f32 * safe_div_f32(self.dsth, self.srch);
                let dstw = values[5] as f32 * safe_div_f32(self.dstw, self.srcw);
                let dsth_val = values[6] as f32 * safe_div_f32(self.dsth, self.srch);
                self.pmchara_entries.push(PmCharaEntry::DstImage {
                    dst: Rectangle {
                        x: dstx,
                        y: dsty,
                        width: dstw,
                        height: dsth_val,
                    },
                });
            }
            _ => {
                // Delegate to CSV loader
                self.csv.process_csv_command(cmd, str_parts, None);
            }
        }
    }

    /// Add note source image to the specified note array
    fn add_note(&mut self, str_parts: &[String], target: NoteTarget, animation: bool) {
        let values = lr2_skin_loader::parse_int(str_parts);
        if let Some(ref mode) = self.mode {
            let mut lane = values[1];
            if lane % 10 == 0 {
                lane = if mode.scratch_key().len() > (lane / 10) as usize {
                    mode.scratch_key()[(lane / 10) as usize]
                } else {
                    -1
                };
            } else {
                let offset =
                    (lane / 10) * (self.laner.len() as i32 / self.playerr.len().max(1) as i32);
                lane = if lane > 10 { lane - 11 } else { lane - 1 };
                if lane
                    >= (self.laner.len() as i32 - mode.scratch_key().len() as i32)
                        / self.playerr.len().max(1) as i32
                {
                    lane = -1;
                } else {
                    lane += offset;
                }
            }
            if lane < 0 {
                return;
            }
            let lane = lane as usize;
            let note_array = match target {
                NoteTarget::Note => &mut self.note,
                NoteTarget::LnStart => &mut self.lnstart,
                NoteTarget::LnEnd => &mut self.lnend,
                NoteTarget::LnBody => &mut self.lnbody,
                NoteTarget::LnBodyA => &mut self.lnbodya,
                NoteTarget::HcnStart => &mut self.hcnstart,
                NoteTarget::HcnEnd => &mut self.hcnend,
                NoteTarget::HcnBody => &mut self.hcnbody,
                NoteTarget::HcnBodyA => &mut self.hcnbodya,
                NoteTarget::HcnBodyD => &mut self.hcnbodyd,
                NoteTarget::HcnBodyR => &mut self.hcnbodyr,
                NoteTarget::Mine => &mut self.mine,
            };
            if lane < note_array.len()
                && note_array[lane].is_none()
                && let Some(images) = self.csv.source_image(&values)
            {
                note_array[lane] = Some(SkinSourceData {
                    images: Some(images),
                    timer: if animation { values[10] } else { 0 },
                    cycle: if animation { values[9] } else { 0 },
                });
            }
        }
    }

    /// Create a default line image at `index` from "skin/default/system.png".
    ///
    /// Corresponds to Java LR2PlaySkinLoader.makeDefaultLines().
    /// Uses the DST_LINE values from the associated judge line (linevalues[index % 2])
    /// but overrides alpha to 255 and RGB to the specified color. The height is
    /// multiplied by `h` (1 for time lines, 2 for BPM/stop lines).
    pub(super) fn make_default_line(&mut self, index: usize, h: i32, r: i32, g: i32, b: i32) {
        let linevalue_idx = index % 2;
        let linevalues = match self.linevalues[linevalue_idx] {
            Some(ref lv) => lv.clone(),
            None => return, // No DST_LINE values available; skip (Java would NPE)
        };

        // Create a 1x1 texture region from system.png
        let tex = Texture::new("skin/default/system.png");
        let region = TextureRegion::from_texture_region(tex, 0, 0, 1, 1);
        let skin_image = SkinImage::new_with_single(region);
        self.line_images[index] = Some(skin_image);

        // Parse the original DST_LINE values to extract position/timing parameters
        let values = lr2_skin_loader::parse_int(&linevalues);

        // Compute destination coordinates with resolution scaling
        let time = values[2] as i64;
        let x = values[3] as f32 * safe_div_f32(self.dstw, self.srcw);
        let y = self.dsth - (values[4] + values[6]) as f32 * safe_div_f32(self.dsth, self.srch);
        let w = values[5] as f32 * safe_div_f32(self.dstw, self.srcw);
        let dst_h = values[6] as f32 * safe_div_f32(self.dsth, self.srch) * h as f32;

        let offset = LR2SkinCSVLoaderState::read_offset_with_base(
            &linevalues,
            21,
            &[crate::skin_property::OFFSET_LIFT],
        );

        if let Some(ref mut li) = self.line_images[index] {
            li.data.set_destination_with_int_timer_and_offsets(
                &DestinationParams {
                    time,
                    x,
                    y,
                    w,
                    h: dst_h,
                    acc: values[7],
                    a: 255,
                    r,
                    g,
                    b,
                    blend: values[12],
                    filter: values[13],
                    angle: values[14],
                    center: values[15],
                    loop_val: values[16],
                },
                values[17],
                values[18],
                values[19],
                values[20],
                &offset,
            );
        }
    }
}

use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::reexports::{MainState, Rectangle};

use super::LR2PlaySkinLoaderState;

impl LR2PlaySkinLoaderState {
    pub fn load_skin(
        &mut self,
        path: &std::path::Path,
        state: Option<&dyn MainState>,
    ) -> anyhow::Result<()> {
        // 1. Initialize note/lane arrays based on mode key count
        self.mode = self.skin_type.mode();
        if let Some(ref mode) = self.mode {
            let key = mode.key() as usize;
            self.note = vec![None; key];
            self.lnstart = vec![None; key];
            self.lnend = vec![None; key];
            self.lnbody = vec![None; key];
            self.lnbodya = vec![None; key];
            self.hcnstart = vec![None; key];
            self.hcnend = vec![None; key];
            self.hcnbody = vec![None; key];
            self.hcnbodya = vec![None; key];
            self.hcnbodyd = vec![None; key];
            self.hcnbodyr = vec![None; key];
            self.mine = vec![None; key];
            self.laner = vec![None; key];
            self.scale = vec![0.0; key];
            self.dstnote2 = vec![i32::MIN; key];

            let player_count = mode.player() as usize;
            self.playerr = vec![Some(Rectangle::default()); player_count];
        }

        // 2. Read and parse CSV file, routing commands through process_play_command
        //    which handles play-specific commands and delegates the rest to the CSV loader.
        let raw_bytes = std::fs::read(path)?;
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
        let content = decoded.into_owned();

        for line in content.lines() {
            self.csv.line = Some(line.to_string());
            if let Some((cmd, str_parts)) = self.csv.base.process_line_directives(line, state) {
                self.process_play_command(&cmd, &str_parts);
            }
        }

        // 3. Flush any remaining active objects in the CSV loader
        self.csv.finalize_active_objects();

        // 4. Post-processing: lane cover Y position adjustment
        // When white number (lane cover position) is 0, reduce lane height by (dsth - laneCoverPosition).
        let lane_cover_position = self.lane_cover_position();
        if lane_cover_position > 0.0 {
            for rect in self.laner.iter_mut().flatten() {
                rect.height -= self.dsth - lane_cover_position;
            }
        }

        // 5. Wire lane rendering: lanerender.setLaneRegion(laner, scale, dstnote2, skin)
        // This is handled in assemble_objects() where SkinNoteObject is created with lane regions.

        // 6. Count line images for judge/BPM/stop/time lines
        // Java: skinline = lines[0..n] where n = count of non-null leading lines
        let line_count = if self.line_images[0].is_some() {
            if self.line_images[1].is_some() { 2 } else { 1 }
        } else {
            0
        };

        // Create default time/BPM/stop line images when missing but judge line exists.
        // Java: makeDefaultLines() creates a SkinImage from "skin/default/system.png" (1x1 white pixel)
        // with the judge line's destination but overridden color and optional height multiplier.
        for i in 0..line_count {
            // Time line at index i+6: h=1, cyan (64, 192, 192)
            if self.line_images[i + 6].is_none() && self.line_images[i].is_some() {
                self.make_default_line(i + 6, 1, 64, 192, 192);
            }
            // BPM line at index i+2: h=2, green (0, 192, 0)
            if self.line_images[i + 2].is_none() && self.line_images[i].is_some() {
                self.make_default_line(i + 2, 2, 0, 192, 0);
            }
            // Stop line at index i+4: h=2, yellow (192, 192, 0)
            if self.line_images[i + 4].is_none() && self.line_images[i].is_some() {
                self.make_default_line(i + 4, 2, 192, 192, 0);
            }
        }

        // 7. Count judge regions
        // Java: judge_reg starts at 1, increments for consecutive non-null judge entries
        let mut judge_reg = 1i32;
        for i in 1..self.judge_objects.len() {
            if self.judge_objects[i].is_some() {
                judge_reg += 1;
            } else {
                break;
            }
        }

        // Store computed values for apply_to_play_skin to use
        self.computed_judge_reg = Some(judge_reg);
        self.computed_line_count = Some(line_count);

        Ok(())
    }

    /// Apply accumulated play skin properties to a PlaySkin.
    /// Call this after load_skin() to transfer CLOSE, PLAYSTART,
    /// LOADSTART, LOADEND, FINISHMARGIN, JUDGETIMER, DST_NOTE_EXPANSION_RATE,
    /// judge region count, lane regions, and lane group regions to the PlaySkin.
    pub fn apply_to_play_skin(&self, play_skin: &mut rubato_play::play_skin::PlaySkin) {
        if let Some(close) = self.play_close {
            play_skin.close = close;
        }
        if let Some(playstart) = self.play_playstart {
            play_skin.playstart = playstart;
        }
        if let Some(loadstart) = self.play_loadstart {
            play_skin.loadstart = loadstart;
        }
        if let Some(loadend) = self.play_loadend {
            play_skin.loadend = loadend;
        }
        if let Some(finish_margin) = self.play_finish_margin {
            play_skin.finish_margin = finish_margin;
        }
        if let Some(judgetimer) = self.play_judgetimer {
            play_skin.judgetimer = judgetimer;
        }
        if let Some(rate) = self.play_note_expansion_rate {
            play_skin.note_expansion_rate = rate;
        }

        // Apply computed judge region count
        if let Some(judge_reg) = self.computed_judge_reg {
            play_skin.judgeregion = judge_reg;
        }

        // Apply lane regions (convert Vec<Option<Rectangle>> -> Vec<Rectangle>)
        let lane_rects: Vec<Rectangle> = self
            .laner
            .iter()
            .map(|opt| opt.unwrap_or_default())
            .collect();
        if !lane_rects.is_empty() {
            play_skin.laneregion = Some(lane_rects);
        }

        // Apply lane group regions (player regions)
        let group_rects: Vec<Rectangle> = self
            .playerr
            .iter()
            .map(|opt| opt.unwrap_or_default())
            .collect();
        if !group_rects.is_empty() {
            play_skin.lanegroupregion = Some(group_rects);
        }

        // Apply line/time/BPM/stop line counts as placeholder Vecs
        let line_count = self.computed_line_count.unwrap_or(0);
        play_skin.line = vec![(); line_count];
        play_skin.time = vec![(); line_count];
        play_skin.bpm = vec![(); line_count];
        play_skin.stop = vec![(); line_count];
    }

    /// Get lane cover position (y coordinate when white number is 0).
    /// Returns the Y from the last DST_HIDDEN destination, or -1.0 if none was defined.
    pub fn lane_cover_position(&self) -> f32 {
        self.lane_cover_dst_y
    }
}

impl LR2SkinLoaderAccess for LR2PlaySkinLoaderState {
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState {
        &mut self.csv
    }

    fn load_skin_data(
        &mut self,
        path: &std::path::Path,
        state: Option<&dyn crate::reexports::MainState>,
    ) -> anyhow::Result<()> {
        self.load_skin(path, state)
    }

    fn assemble_objects(&mut self, skin: &mut crate::skin::Skin) {
        use crate::skin::SkinObject;

        // Transfer generic objects (SRC_IMAGE, SRC_BUTTON, SRC_ONMOUSE, SRC_GROOVEGAUGE)
        // accumulated by the base CSV parser into the skin.
        for obj in self.csv.collected_objects.drain(..) {
            skin.add(obj);
        }

        // 1. Add SkinJudgeObject instances (already created during SRC_NOWJUDGE parsing)
        for judge_opt in &mut self.judge_objects {
            if let Some(judge) = judge_opt.take() {
                skin.add(SkinObject::Judge(judge));
            }
        }

        // 2. Extract line image data for SkinNoteObject before consuming them.
        // Line images are rendered dynamically at y_offsets from DrawCommands,
        // NOT as standalone skin objects.
        let mut extracted_lines: [Option<crate::skin_note_object::LineImage>; 8] =
            Default::default();
        for (i, line_opt) in self.line_images.iter().enumerate() {
            if let Some(line_img) = line_opt
                && let Some(region) = line_img.first_frame_region()
            {
                let (dst_w, dst_h) = line_img
                    .data
                    .dst
                    .first()
                    .map(|d| (d.region.width, d.region.height))
                    .unwrap_or((0.0, 0.0));
                extracted_lines[i] = Some(crate::skin_note_object::LineImage {
                    region,
                    dst_width: dst_w,
                    dst_height: dst_h,
                });
            }
        }
        // Consume line images (they're not standalone skin objects)
        for line_opt in &mut self.line_images {
            line_opt.take();
        }

        // 3. Add judgeline SkinImage (already created during SRC_JUDGELINE parsing)
        if let Some(judgeline) = self.judgeline.take() {
            skin.add(SkinObject::Image(judgeline));
        }

        // 4. Create SkinNoteObject from accumulated note source data
        if self.lanerender {
            let lane_count = self.note.len();
            let mut note_obj = crate::skin_note_object::SkinNoteObject::new(lane_count);
            note_obj.line_images = extracted_lines;
            for (i, lane_rect) in self.laner.iter().enumerate() {
                if let Some(rect) = lane_rect {
                    note_obj.inner.set_lane_region(
                        i,
                        &rubato_play::skin::note::LaneRegion {
                            x: rect.x,
                            y: rect.y,
                            width: rect.width,
                            height: rect.height,
                            scale: *self.scale.get(i).unwrap_or(&1.0),
                            dstnote2: *self.dstnote2.get(i).unwrap_or(&0),
                        },
                    );
                }
            }
            // Wire note textures (first frame of each lane's animation)
            for (i, src) in self.note.iter().enumerate() {
                if let Some(data) = src
                    && let Some(images) = &data.images
                    && let Some(first) = images.first()
                    && i < note_obj.note_images.len()
                {
                    note_obj.note_images[i] = Some(first.clone());
                }
            }
            for (i, src) in self.mine.iter().enumerate() {
                if let Some(data) = src
                    && let Some(images) = &data.images
                    && let Some(first) = images.first()
                    && i < note_obj.mine_images.len()
                {
                    note_obj.mine_images[i] = Some(first.clone());
                }
            }
            // Wire LN body textures (first frame)
            let ln_sources: [&Vec<Option<super::SkinSourceData>>; 10] = [
                &self.lnstart,
                &self.lnbodya,
                &self.lnbody,
                &self.lnend,
                &self.hcnstart,
                &self.hcnbodya,
                &self.hcnbody,
                &self.hcnend,
                &self.hcnbodyd,
                &self.hcnbodyr,
            ];
            for (i, arr) in note_obj.ln_body_images.iter_mut().enumerate() {
                for (idx, src_vec) in ln_sources.iter().enumerate() {
                    if let Some(Some(data)) = src_vec.get(i)
                        && let Some(images) = &data.images
                        && let Some(first) = images.first()
                    {
                        arr[idx] = Some(first.clone());
                    }
                }
            }
            skin.add(SkinObject::Note(note_obj));
        }

        // 5a. Add SkinHidden (lane cover) object if created during SRC_HIDDEN/SRC_LIFT parsing
        if let Some(hidden) = self.hidden.take() {
            skin.add(SkinObject::Hidden(hidden));
        }

        // 5. Create SkinBgaObject if BGA was requested
        if self.bga {
            let bga_obj = crate::skin_bga_object::SkinBgaObject::new(0);
            skin.add(SkinObject::Bga(bga_obj));
        }

        // 6. Add graph objects
        if let Some(obj) = self.noteobj.take() {
            skin.add(SkinObject::NoteDistributionGraph(obj));
        }
        if let Some(obj) = self.bpmgraphobj.take() {
            skin.add(SkinObject::BpmGraph(obj));
        }

        // 7. Apply play-specific timing to skin
        if let Some(close) = self.play_close {
            skin.scene = close;
        }
        if let Some(margin) = self.play_finish_margin {
            skin.fadeout = margin;
        }

        log::debug!("LR2PlaySkinLoader: assembled objects into skin");
    }
}

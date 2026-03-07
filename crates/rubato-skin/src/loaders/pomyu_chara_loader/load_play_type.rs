impl<'a> PomyuCharaLoader<'a> {

    #[allow(clippy::too_many_arguments)]
    fn load_play_type(
        &mut self,
        _usecim: bool,
        _chp: &str,
        char_bmp: &mut [Option<Texture>; 8],
        transparent_flag: &mut [bool; 8],
        xywh: &[[i32; 4]],
        frame: &mut [i32; 20],
        anime: i32,
        size: &[i32; 2],
        loop_val: &mut [i32; 20],
        set_color: i32,
        increase_rate_threshold: i32,
        pattern_data: &[Vec<String>],
        char_bmp_index: usize,
        char_tex_index: usize,
        set_motion: i32,
        dsttimer: i32,
        dst_op1: i32,
        dst_op2: i32,
        dst_op3: i32,
        dst_offset: i32,
        side: i32,
        dstx: f32,
        dsty: f32,
        dstw: f32,
        dsth: f32,
    ) {
        // Initialize frame values
        for f in frame.iter_mut() {
            if *f == i32::MIN {
                *f = anime;
            }
            if *f < 1 {
                *f = 100;
            }
        }

        // Dummy transparent 1x1 texture
        let pixmap = Pixmap::new(1, 1, PixmapFormat::RGBA8888);
        let transparent_tex = Texture::from_pixmap(&pixmap);

        // #Pattern, #Texture, #Layer render order
        let set_bmp_index = [char_bmp_index, char_tex_index, char_bmp_index];
        for pattern_index in 0..3 {
            for pattern_data_entry in &pattern_data[pattern_index] {
                let str_parts: Vec<&str> = pattern_data_entry.split('\t').collect();
                if str_parts.len() <= 1 {
                    continue;
                }
                if set_color < 1 {
                    continue;
                }
                let set_index = set_bmp_index[pattern_index] + set_color as usize - 1;
                if set_index >= char_bmp.len() {
                    continue;
                }
                let taken = char_bmp.get_mut(set_index).and_then(|s| s.take());
                if let Some(slot) = char_bmp.get_mut(set_index) {
                    *slot = transparent_processing(taken, set_index, transparent_flag);
                }
                let set_bmp = match char_bmp.get(set_index).and_then(|t| t.as_ref()) {
                    Some(t) => t.clone(),
                    None => continue,
                };

                let mut motion = i32::MIN;
                let mut dst = [String::new(), String::new(), String::new(), String::new()];
                let data = pm_parse_str(&str_parts);
                if data.len() > 1 {
                    motion = pm_parse_int(&data[1]);
                }
                for i in 0..dst.len() {
                    if data.len() > i + 2 {
                        // replaceAll("[^0-9a-zA-Z-]", "")
                        dst[i] = data[i + 2]
                            .chars()
                            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                            .collect();
                    }
                }

                let mut timer = i32::MIN;
                let mut op = [0_i32; 3];
                if set_motion != i32::MIN && set_motion == motion {
                    timer = dsttimer;
                    op[0] = dst_op1;
                    op[1] = dst_op2;
                    op[2] = dst_op3;
                } else if set_motion == i32::MIN {
                    if side != 2 {
                        if motion == 1 {
                            timer = TIMER_PM_CHARA_1P_NEUTRAL.as_i32();
                        } else if motion == 6 {
                            timer = TIMER_PM_CHARA_1P_FEVER.as_i32();
                        } else if motion == 7 {
                            timer = TIMER_PM_CHARA_1P_GREAT.as_i32();
                        } else if motion == 8 {
                            timer = TIMER_PM_CHARA_1P_GOOD.as_i32();
                        } else if motion == 10 {
                            timer = TIMER_PM_CHARA_1P_BAD.as_i32();
                        } else if (15..=17).contains(&motion) {
                            timer = TIMER_MUSIC_END.as_i32();
                            if motion == 15 {
                                // WIN
                                op[0] = OPTION_1P_BORDER_OR_MORE;
                                op[1] = -OPTION_1P_100;
                            } else if motion == 16 {
                                // LOSE
                                op[0] = -OPTION_1P_BORDER_OR_MORE;
                            } else if motion == 17 {
                                // FEVERWIN
                                op[0] = OPTION_1P_100;
                            }
                        }
                    } else if motion == 1 {
                        timer = TIMER_PM_CHARA_2P_NEUTRAL.as_i32();
                    } else if motion == 7 {
                        timer = TIMER_PM_CHARA_2P_GREAT.as_i32();
                    } else if motion == 10 {
                        timer = TIMER_PM_CHARA_2P_BAD.as_i32();
                    } else if motion == 15 || motion == 16 {
                        timer = TIMER_MUSIC_END.as_i32();
                        if motion == 15 {
                            // WIN (2P side: reversed)
                            op[0] = -OPTION_1P_BORDER_OR_MORE;
                        } else if motion == 16 {
                            // LOSE (2P side: reversed)
                            op[0] = OPTION_1P_BORDER_OR_MORE;
                        }
                    }
                }

                if timer != i32::MIN
                    && !dst[0].is_empty()
                    && dst[0].len().is_multiple_of(2)
                    && (dst[1].is_empty() || dst[1].len() == dst[0].len())
                    && (dst[2].is_empty() || dst[2].len() == dst[0].len())
                    && (dst[3].is_empty() || dst[3].len() == dst[0].len())
                {
                    // Clamp loop values
                    if loop_val[motion as usize] >= (dst[0].len() / 2 - 1) as i32 {
                        loop_val[motion as usize] = (dst[0].len() / 2 - 2) as i32;
                    } else if loop_val[motion as usize] < -1 {
                        loop_val[motion as usize] = -1;
                    }

                    let cycle = frame[motion as usize] * (dst[0].len() / 2) as i32;
                    let loop_time = frame[motion as usize] * (loop_val[motion as usize] + 1);

                    if set_motion == i32::MIN
                        && (TIMER_PM_CHARA_1P_NEUTRAL.as_i32()..TIMER_MUSIC_END.as_i32())
                            .contains(&timer)
                    {
                        self.skin
                            .pomyu
                            .set_pm_chara_time(timer - TIMER_PM_CHARA_1P_NEUTRAL.as_i32(), cycle);
                    }

                    // Check for hyphen interpolation flag
                    let mut hyphen_flag = false;
                    for d in &dst[1..] {
                        if d.contains('-') {
                            hyphen_flag = true;
                            break;
                        }
                    }

                    // Frame interpolation when hyphen exists, 60FPS 17ms threshold
                    let mut increase_rate = 1;
                    if hyphen_flag && frame[motion as usize] >= increase_rate_threshold {
                        for i in 1..=frame[motion as usize] {
                            if frame[motion as usize] / i < increase_rate_threshold
                                && frame[motion as usize] % i == 0
                            {
                                increase_rate = i;
                                break;
                            }
                        }
                        // Expand dst[1..] by increase_rate
                        for d in dst[1..].iter_mut() {
                            let mut chars = Vec::new();
                            let bytes = d.as_bytes();
                            let mut j = 0;
                            while j + 1 < bytes.len() {
                                for _k in 0..increase_rate {
                                    chars.push(bytes[j] as char);
                                    chars.push(bytes[j + 1] as char);
                                }
                                j += 2;
                            }
                            *d = chars.into_iter().collect();
                        }
                    }

                    // DST loading
                    let frame_time = frame[motion as usize] as f64 / increase_rate as f64;
                    let loop_frame = loop_val[motion as usize] * increase_rate;
                    let dstxywh_len = if !dst[1].is_empty() {
                        dst[1].len() / 2
                    } else {
                        dst[0].len() / 2
                    };
                    let mut dstxywh = vec![[0, 0, size[0], size[1]]; dstxywh_len];

                    // Parse dst[1] position data with interpolation
                    let mut start_xywh = [0, 0, size[0], size[1]];
                    let mut end_xywh = [0, 0, size[0], size[1]];
                    {
                        let mut i = 0;
                        while i < dst[1].len() {
                            if i + 2 <= dst[1].len() {
                                if &dst[1][i..i + 2] == "--" {
                                    let mut count = 0;
                                    let mut j = i;
                                    while j < dst[1].len()
                                        && j + 2 <= dst[1].len()
                                        && &dst[1][j..j + 2] == "--"
                                    {
                                        count += 1;
                                        j += 2;
                                    }
                                    // Read end value
                                    if i + count * 2 + 2 <= dst[1].len() {
                                        let end_str = &dst[1][i + count * 2..i + count * 2 + 2];
                                        let parsed = pm_parse_int_radix(end_str, 36);
                                        if parsed >= 0 && (parsed as usize) < xywh.len() {
                                            end_xywh = xywh[parsed as usize];
                                        }
                                    }
                                    // Interpolate
                                    j = i;
                                    while j < dst[1].len()
                                        && j + 2 <= dst[1].len()
                                        && &dst[1][j..j + 2] == "--"
                                    {
                                        for k in 0..4 {
                                            dstxywh[j / 2][k] = start_xywh[k]
                                                + (end_xywh[k] - start_xywh[k])
                                                    * (((j - i) / 2 + 1) as i32)
                                                    / (count as i32 + 1);
                                        }
                                        j += 2;
                                    }
                                    i += (count - 1) * 2;
                                } else {
                                    let substr = &dst[1][i..i + 2];
                                    let parsed = pm_parse_int_radix(substr, 36);
                                    if parsed >= 0 && (parsed as usize) < xywh.len() {
                                        start_xywh = xywh[parsed as usize];
                                        dstxywh[i / 2] = start_xywh;
                                    }
                                }
                            }
                            i += 2;
                        }
                    }

                    // Alpha and angle loading
                    let mut alpha_angle = vec![[255_i32, 0_i32]; dstxywh_len];
                    for index in 2..dst.len() {
                        let mut start_value = 0;
                        let mut end_value;
                        let mut i = 0;
                        while i < dst[index].len() {
                            if i + 2 <= dst[index].len() {
                                if &dst[index][i..i + 2] == "--" {
                                    let mut count = 0;
                                    let mut j = i;
                                    while j < dst[index].len()
                                        && j + 2 <= dst[index].len()
                                        && &dst[index][j..j + 2] == "--"
                                    {
                                        count += 1;
                                        j += 2;
                                    }
                                    end_value = 0;
                                    if i + count * 2 + 2 <= dst[index].len() {
                                        let end_str = &dst[index][i + count * 2..i + count * 2 + 2];
                                        let parsed = pm_parse_int_radix(end_str, 16);
                                        if (0..=255).contains(&parsed) {
                                            end_value = parsed;
                                            if index == 3 {
                                                end_value = (end_value as f32 * 360.0 / 256.0)
                                                    .round()
                                                    as i32;
                                            }
                                        }
                                    }
                                    j = i;
                                    while j < dst[index].len()
                                        && j + 2 <= dst[index].len()
                                        && &dst[index][j..j + 2] == "--"
                                    {
                                        alpha_angle[j / 2][index - 2] = start_value
                                            + (end_value - start_value)
                                                * (((j - i) / 2 + 1) as i32)
                                                / (count as i32 + 1);
                                        j += 2;
                                    }
                                    i += (count - 1) * 2;
                                } else {
                                    let substr = &dst[index][i..i + 2];
                                    let parsed = pm_parse_int_radix(substr, 16);
                                    if (0..=255).contains(&parsed) {
                                        start_value = parsed;
                                        if index == 3 {
                                            start_value =
                                                (start_value as f32 * 360.0 / 256.0).round() as i32;
                                        }
                                        alpha_angle[i / 2][index - 2] = start_value;
                                    }
                                }
                            }
                            i += 2;
                        }
                    }

                    // Guard against size[0] or size[1] being zero (division)
                    if size[0] == 0 || size[1] == 0 {
                        continue;
                    }

                    // Pre-loop frames (up to loop start)
                    if (loop_frame + increase_rate) != 0 {
                        let mut images =
                            Vec::with_capacity((loop_val[motion as usize] + 1) as usize);
                        let mut i = 0;
                        while i < (loop_val[motion as usize] + 1) * 2 {
                            if i + 2 <= dst[0].len() as i32 {
                                let idx =
                                    pm_parse_int_radix(&dst[0][i as usize..(i + 2) as usize], 36);
                                if idx >= 0
                                    && (idx as usize) < xywh.len()
                                    && xywh[idx as usize][2] > 0
                                    && xywh[idx as usize][3] > 0
                                {
                                    images.push(TextureRegion::from_texture_region(
                                        set_bmp.clone(),
                                        xywh[idx as usize][0],
                                        xywh[idx as usize][1],
                                        xywh[idx as usize][2],
                                        xywh[idx as usize][3],
                                    ));
                                } else {
                                    images.push(TextureRegion::from_texture_region(
                                        transparent_tex.clone(),
                                        0,
                                        0,
                                        1,
                                        1,
                                    ));
                                }
                            }
                            i += 2;
                        }

                        let mut part = SkinImage::new_with_int_timer(images, timer, loop_time);

                        for i in 0..(loop_frame + increase_rate) as usize {
                            part.data.set_destination_with_int_timer_and_single_offset(
                                (frame_time * i as f64) as i64,
                                dstx + dstxywh[i][0] as f32 * dstw / size[0] as f32,
                                dsty + dsth
                                    - (dstxywh[i][1] + dstxywh[i][3]) as f32 * dsth
                                        / size[1] as f32,
                                dstxywh[i][2] as f32 * dstw / size[0] as f32,
                                dstxywh[i][3] as f32 * dsth / size[1] as f32,
                                3,
                                alpha_angle[i][0],
                                255,
                                255,
                                255,
                                1,
                                0,
                                alpha_angle[i][1],
                                0,
                                -1,
                                timer,
                                op[0],
                                op[1],
                                op[2],
                                0,
                            );
                        }
                        let last_pre = (loop_frame + increase_rate - 1) as usize;
                        part.data.set_destination_with_int_timer_and_single_offset(
                            (loop_time - 1) as i64,
                            dstx + dstxywh[last_pre][0] as f32 * dstw / size[0] as f32,
                            dsty + dsth
                                - (dstxywh[last_pre][1] + dstxywh[last_pre][3]) as f32 * dsth
                                    / size[1] as f32,
                            dstxywh[last_pre][2] as f32 * dstw / size[0] as f32,
                            dstxywh[last_pre][3] as f32 * dsth / size[1] as f32,
                            3,
                            alpha_angle[last_pre][0],
                            255,
                            255,
                            255,
                            1,
                            0,
                            alpha_angle[last_pre][1],
                            0,
                            -1,
                            timer,
                            op[0],
                            op[1],
                            op[2],
                            dst_offset,
                        );
                        self.skin.add(part);
                    }

                    // Loop frames (from loop start to end)
                    let loop_start = (loop_val[motion as usize] + 1) as usize;
                    let total_frames = dst[0].len() / 2;
                    let loop_image_count = total_frames - loop_start;
                    let mut images = Vec::with_capacity(loop_image_count);
                    let mut i = loop_start * 2;
                    while i < dst[0].len() {
                        if i + 2 <= dst[0].len() {
                            let idx = pm_parse_int_radix(&dst[0][i..i + 2], 36);
                            if idx >= 0
                                && (idx as usize) < xywh.len()
                                && xywh[idx as usize][2] > 0
                                && xywh[idx as usize][3] > 0
                            {
                                images.push(TextureRegion::from_texture_region(
                                    set_bmp.clone(),
                                    xywh[idx as usize][0],
                                    xywh[idx as usize][1],
                                    xywh[idx as usize][2],
                                    xywh[idx as usize][3],
                                ));
                            } else {
                                images.push(TextureRegion::from_texture_region(
                                    transparent_tex.clone(),
                                    0,
                                    0,
                                    1,
                                    1,
                                ));
                            }
                        }
                        i += 2;
                    }

                    let mut part = SkinImage::new_with_int_timer(images, timer, cycle - loop_time);

                    for i in (loop_frame + increase_rate) as usize..dstxywh.len() {
                        part.data.set_destination_with_int_timer_and_single_offset(
                            (frame_time * i as f64) as i64,
                            dstx + dstxywh[i][0] as f32 * dstw / size[0] as f32,
                            dsty + dsth
                                - (dstxywh[i][1] + dstxywh[i][3]) as f32 * dsth / size[1] as f32,
                            dstxywh[i][2] as f32 * dstw / size[0] as f32,
                            dstxywh[i][3] as f32 * dsth / size[1] as f32,
                            3,
                            alpha_angle[i][0],
                            255,
                            255,
                            255,
                            1,
                            0,
                            alpha_angle[i][1],
                            0,
                            loop_time,
                            timer,
                            op[0],
                            op[1],
                            op[2],
                            0,
                        );
                    }
                    let last = dstxywh.len() - 1;
                    part.data.set_destination_with_int_timer_and_single_offset(
                        cycle as i64,
                        dstx + dstxywh[last][0] as f32 * dstw / size[0] as f32,
                        dsty + dsth
                            - (dstxywh[last][1] + dstxywh[last][3]) as f32 * dsth / size[1] as f32,
                        dstxywh[last][2] as f32 * dstw / size[0] as f32,
                        dstxywh[last][3] as f32 * dsth / size[1] as f32,
                        3,
                        alpha_angle[last][0],
                        255,
                        255,
                        255,
                        1,
                        0,
                        alpha_angle[last][1],
                        0,
                        loop_time,
                        timer,
                        op[0],
                        op[1],
                        op[2],
                        dst_offset,
                    );
                    self.skin.add(part);
                }
            }
        }
    }
}

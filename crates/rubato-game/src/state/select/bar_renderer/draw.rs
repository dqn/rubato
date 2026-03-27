use super::super::skin_bar::SkinBar;
use super::BarRenderer;
use super::types::RenderContext;
use crate::state::select::*;
use rubato_types::sync_utils::lock_or_recover;

impl BarRenderer {
    /// Refresh the bar text character set for font preparation when songs change.
    pub(super) fn update_bar_text_charset(&mut self, baro: &mut SkinBar, ctx: &RenderContext) {
        if self.bartextupdate {
            self.bartextupdate = false;

            self.bartextcharset.clear();
            for song in ctx.currentsongs {
                for c in song.title().chars() {
                    self.bartextcharset.insert(c);
                }
            }
            let chars: String = self.bartextcharset.iter().collect();

            for index in 0..SkinBar::BARTEXT_COUNT {
                if let Some(text) = baro.text.get_mut(index).and_then(|o| o.as_mut()) {
                    text.prepare_font(&chars);
                }
            }
        }
    }

    /// Draw bar background images for each bar slot.
    pub(super) fn draw_bar_images(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        baro: &mut SkinBar,
        ctx: &RenderContext,
    ) {
        let position = baro.position();
        for i in 0..self.barlength {
            let on = i as i32 == ctx.center_bar;

            let images = if on {
                &mut baro.barimageon
            } else {
                &mut baro.barimageoff
            };
            let si = match images.get_mut(i).and_then(|o| o.as_mut()) {
                Some(si) => si,
                None => continue,
            };

            if si.data.draw {
                // Read the static region from fixr (available for single-DST or
                // uniform-DST objects) or fall back to the first DST entry's region.
                // Avoids reading from si.data.region which holds a stale value from
                // the skin-wide prepare pass; draw_with_value below runs a second
                // prepare that overwrites region with the bar-specific offsets.
                let base_region = si
                    .data
                    .fixr
                    .or_else(|| si.data.dst.first().map(|d| d.region))
                    .unwrap_or(si.data.region);
                let position_offset = if position == 1 {
                    base_region.height
                } else {
                    0.0
                };
                let ba = &self.bararea[i];
                si.draw_with_value(
                    sprite,
                    self.time,
                    ctx.state,
                    ba.value,
                    ba.x - base_region.x,
                    ba.y - base_region.y - position_offset,
                );
            } else {
                self.bararea[i].value = -1;
            }
        }
    }

    /// Draw distribution graphs for directory and function bars.
    pub(super) fn draw_distribution_graphs(
        &self,
        sprite: &mut SkinObjectRenderer,
        baro: &mut SkinBar,
        ctx: &RenderContext,
    ) {
        for i in 0..self.barlength {
            let ba = &self.bararea[i];
            if ba.value == -1 {
                continue;
            }
            if let Some(idx) = ba.sd {
                let sd = &ctx.currentsongs[idx];
                if let Some(dir_data) = sd.as_directory_bar()
                    && let Some(graph) = baro.graph()
                    && graph.draw
                {
                    graph.draw_directory(sprite, dir_data, ba.x, ba.y);
                } else if let Some(fb) = sd.as_function_bar()
                    && let Some(graph) = baro.graph()
                    && graph.draw
                {
                    graph.draw_function_bar(sprite, fb, ba.x, ba.y);
                }
            }
        }
    }

    /// Draw download progress bars for songs being downloaded.
    pub(super) fn draw_download_progress(
        &self,
        sprite: &mut SkinObjectRenderer,
        baro: &mut SkinBar,
        ctx: &RenderContext,
    ) {
        let download_tasks =
            crate::song::md_processor::download_task_state::DownloadTaskState::get_running_download_tasks();
        if download_tasks.is_empty() {
            return;
        }
        for i in 0..self.barlength {
            let ba = &self.bararea[i];
            if ba.value == -1 {
                continue;
            }
            if let Some(idx) = ba.sd {
                let sd = &ctx.currentsongs[idx];
                if let Some(song_bar) = sd.as_song_bar() {
                    let song_md5 = &song_bar.song_data().file.md5;
                    for task_arc in download_tasks.values() {
                        let task = lock_or_recover(task_arc);
                        if task.hash() != song_md5 {
                            continue;
                        }
                        if let Some(graph) = baro.graph()
                            && graph.draw
                        {
                            graph.draw_song_bar_download(sprite, song_bar, &task, ba.x, ba.y);
                        }
                    }
                }
            }
        }
    }

    /// Draw song title text for each bar slot.
    pub(super) fn draw_bar_text(
        &self,
        sprite: &mut SkinObjectRenderer,
        baro: &mut SkinBar,
        ctx: &RenderContext,
    ) {
        for i in 0..self.barlength {
            let ba = &self.bararea[i];
            if ba.value == -1 {
                continue;
            }
            if let Some(idx) = ba.sd {
                let sd = &ctx.currentsongs[idx];
                if let Some(text) = baro.text.get_mut(ba.text).and_then(|o| o.as_mut()) {
                    text.get_text_data_mut().set_text(sd.title().to_string());
                    // Font glyphs are prepared via update_bar_text_charset() at the top of
                    // render(), triggered by update_bar_text() after each load_bar_contents() call.
                    text.draw_with_offset(sprite, ba.x, ba.y);
                }
            }
        }
    }

    /// Draw trophy icons for grade bars.
    pub(super) fn draw_trophies(
        &self,
        sprite: &mut SkinObjectRenderer,
        baro: &mut SkinBar,
        ctx: &RenderContext,
    ) {
        for i in 0..self.barlength {
            let ba = &self.bararea[i];
            if ba.value == -1 {
                continue;
            }
            if let Some(idx) = ba.sd
                && let Some(gb) = ctx.currentsongs[idx].as_grade_bar()
                && let Some(trophy) = gb.trophy()
            {
                for (j, trophy_name) in self.trophy.iter().enumerate() {
                    if *trophy_name == trophy.name() {
                        if let Some(trophy_image) = baro.trophy.get_mut(j).and_then(|o| o.as_mut())
                        {
                            trophy_image.draw_with_offset(sprite, ba.x, ba.y);
                        }
                        break;
                    }
                }
            }
        }
    }

    /// Draw clear lamp indicators for each bar.
    pub(super) fn draw_lamps(
        &self,
        sprite: &mut SkinObjectRenderer,
        baro: &mut SkinBar,
        ctx: &RenderContext,
    ) {
        for i in 0..self.barlength {
            let ba = &self.bararea[i];
            if ba.value == -1 {
                continue;
            }
            if let Some(idx) = ba.sd {
                let sd = &ctx.currentsongs[idx];
                if ctx.rival {
                    let player_lamp_id = sd.lamp(true);
                    if player_lamp_id >= 0
                        && (player_lamp_id as usize) < baro.mylamp.len()
                        && let Some(lamp) = baro.mylamp[player_lamp_id as usize].as_mut()
                    {
                        lamp.draw_with_offset(sprite, ba.x, ba.y);
                    }
                    let rival_lamp_id = sd.lamp(false);
                    if rival_lamp_id >= 0
                        && (rival_lamp_id as usize) < baro.rivallamp.len()
                        && let Some(lamp) = baro.rivallamp[rival_lamp_id as usize].as_mut()
                    {
                        lamp.draw_with_offset(sprite, ba.x, ba.y);
                    }
                } else {
                    let lamp_id = sd.lamp(true);
                    if lamp_id >= 0
                        && (lamp_id as usize) < baro.lamp.len()
                        && let Some(lamp) = baro.lamp[lamp_id as usize].as_mut()
                    {
                        lamp.draw_with_offset(sprite, ba.x, ba.y);
                    }
                }
            }
        }
    }

    /// Draw difficulty level numbers for song and function bars.
    pub(super) fn draw_levels(
        &self,
        sprite: &mut SkinObjectRenderer,
        baro: &mut SkinBar,
        ctx: &RenderContext,
    ) {
        for i in 0..self.barlength {
            let ba = &self.bararea[i];
            if ba.value == -1 {
                continue;
            }
            if let Some(idx) = ba.sd {
                let sd = &ctx.currentsongs[idx];
                if let Some(sb) = sd.as_song_bar() {
                    if sb.exists_song() {
                        let song = sb.song_data();
                        let difficulty = song.chart.difficulty;
                        let level_idx = if (0..7).contains(&difficulty) {
                            difficulty
                        } else {
                            0
                        };
                        if level_idx >= 0
                            && (level_idx as usize) < baro.barlevel.len()
                            && let Some(leveln) = baro.barlevel[level_idx as usize].as_mut()
                        {
                            leveln.draw_with_value(
                                sprite,
                                self.time,
                                song.chart.level,
                                ctx.state,
                                ba.x,
                                ba.y,
                            );
                        }
                    }
                } else if let Some(fb) = sd.as_function_bar()
                    && let Some(level) = fb.level()
                    && let Some(leveln) = baro.barlevel.first_mut().and_then(|o| o.as_mut())
                {
                    leveln.draw_with_value(sprite, self.time, level, ctx.state, ba.x, ba.y);
                }
            }
        }
    }

    /// Draw feature labels (LN/MINE/RANDOM) for songs with special note types.
    pub(super) fn draw_feature_labels(
        &self,
        sprite: &mut SkinObjectRenderer,
        baro: &mut SkinBar,
        ctx: &RenderContext,
    ) {
        for i in 0..self.barlength {
            let ba = &self.bararea[i];
            if ba.value == -1 {
                continue;
            }
            if let Some(idx) = ba.sd {
                let sd = &ctx.currentsongs[idx];
                let mut flag: i32 = 0;

                if let Some(sb) = sd.as_song_bar()
                    && sb.exists_song()
                {
                    flag |= sb.song_data().chart.feature;
                }

                if let Some(gb) = sd.as_grade_bar()
                    && gb.exists_all_songs()
                {
                    for song in gb.song_datas() {
                        flag |= song.chart.feature;
                    }
                }

                // LN
                let mut ln: i32 = -1;
                if (flag & FEATURE_UNDEFINEDLN) != 0 {
                    ln = ctx.lnmode;
                }
                if (flag & FEATURE_LONGNOTE) != 0 {
                    ln = if ln > 0 { ln } else { 0 };
                }
                if (flag & FEATURE_CHARGENOTE) != 0 {
                    ln = if ln > 1 { ln } else { 1 };
                }
                if (flag & FEATURE_HELLCHARGENOTE) != 0 {
                    ln = if ln > 2 { ln } else { 2 };
                }

                if ln >= 0 {
                    // LN label drawing branch
                    let lnindex = [0i32, 3, 4];
                    let ln_idx = ln as usize;
                    if ln_idx < lnindex.len() {
                        let label_idx = lnindex[ln_idx] as usize;
                        let drawn = if label_idx < baro.label.len() {
                            if let Some(label) = baro.label[label_idx].as_mut() {
                                label.draw_with_offset(sprite, ba.x, ba.y);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        if !drawn
                            && let Some(label) = baro.label.first_mut().and_then(|o| o.as_mut())
                        {
                            label.draw_with_offset(sprite, ba.x, ba.y);
                        }
                    }
                }

                // MINE
                if (flag & FEATURE_MINENOTE) != 0
                    && let Some(label) = baro.label.get_mut(2).and_then(|o| o.as_mut())
                {
                    label.draw_with_offset(sprite, ba.x, ba.y);
                }

                // RANDOM
                if (flag & FEATURE_RANDOM) != 0
                    && let Some(label) = baro.label.get_mut(1).and_then(|o| o.as_mut())
                {
                    label.draw_with_offset(sprite, ba.x, ba.y);
                }
            }
        }
    }
}

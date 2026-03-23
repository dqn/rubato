mod draw;
mod types;

pub use types::{
    BarInputContext, MousePressedAction, MousePressedContext, PrepareContext, RenderContext,
};

use std::collections::HashSet;

use super::bar::bar::Bar;
use super::music_select_key_property::MusicSelectKey;
use super::skin_bar::SkinBar;
use super::*;

use types::BarArea;

/// Bar renderer for song bar display
/// Translates: bms.player.beatoraja.select.BarRenderer
pub struct BarRenderer {
    pub trophy: [&'static str; 3],

    pub durationlow: i32,
    pub durationhigh: i32,
    /// Bar movement counter
    pub duration: i64,
    /// Bar movement direction
    pub angle: i32,
    pub keyinput: bool,

    /// Analog scroll buffer
    pub analog_scroll_buffer: i32,
    pub analog_ticks_per_scroll: i32,

    pub barlength: usize,
    bararea: Vec<BarArea>,

    pub bartextupdate: bool,
    bartextcharset: HashSet<char>,

    time: i64,
}

impl BarRenderer {
    pub fn new(durationlow: i32, durationhigh: i32, analog_ticks_per_scroll: i32) -> Self {
        let barlength = 60;
        let bararea = (0..barlength).map(|_| BarArea::new()).collect();

        Self {
            trophy: ["bronzemedal", "silvermedal", "goldmedal"],
            durationlow,
            durationhigh,
            duration: 0,
            angle: 0,
            keyinput: false,
            analog_scroll_buffer: 0,
            analog_ticks_per_scroll,
            barlength,
            bararea,
            bartextupdate: true,
            bartextcharset: HashSet::with_capacity(1024),
            time: 0,
        }
    }

    /// Check if a bar click was detected and return the action.
    /// Translates: Java BarRenderer.mousePressed(SkinBar, int, int, int)
    pub fn mouse_pressed(
        &self,
        baro: &SkinBar,
        button: i32,
        x: i32,
        y: i32,
        ctx: &MousePressedContext,
    ) -> MousePressedAction {
        if ctx.currentsongs.is_empty() {
            return MousePressedAction::None;
        }
        for &i in ctx.clickable_bar {
            let i = i as usize;
            let on = i as i32 == ctx.center_bar;
            let si = match baro.bar_images(on, i) {
                Some(si) => si,
                None => continue,
            };

            // Use i64 arithmetic to avoid usize wrapping when center_bar is negative.
            let index =
                ((ctx.selectedindex as i64 + ctx.currentsongs.len() as i64 * 100 + i as i64
                    - ctx.center_bar as i64)
                    .rem_euclid(ctx.currentsongs.len() as i64)) as usize;

            // After prepare(), data.region contains the interpolated destination rectangle
            if si.data.draw {
                let r = &si.data.region;
                if r.x <= x as f32
                    && r.x + r.width >= x as f32
                    && r.y <= y as f32
                    && r.y + r.height >= y as f32
                {
                    if button == 0 {
                        return MousePressedAction::Select(index);
                    } else {
                        return MousePressedAction::Close;
                    }
                }
            }
        }
        MousePressedAction::None
    }

    /// Calculate bar positions and bar types.
    /// Translates: Java BarRenderer.prepare(SkinBar, long)
    pub fn prepare(&mut self, baro: &SkinBar, time: i64, ctx: &PrepareContext) {
        self.time = time;
        if ctx.currentsongs.is_empty() {
            return;
        }

        let time_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let apply_movement = self.duration != 0 && self.duration > time_millis;
        let mut angle_lerp: f32 = 0.0;
        if apply_movement && self.angle != 0 {
            if self.angle < 0 {
                angle_lerp = (time_millis - self.duration) as f32 / self.angle as f32;
            } else {
                angle_lerp = (self.duration - time_millis) as f32 / self.angle as f32;
            }
        }

        let now_secs = time_millis / 1000;

        for i in 0..self.barlength {
            // calculate song bar position
            let ba = &mut self.bararea[i];
            let on = i as i32 == ctx.center_bar;
            let si1 = match baro.bar_images(on, i) {
                Some(si) => si,
                None => continue,
            };

            if si1.data.draw {
                let mut dx: f32 = 0.0;
                let mut dy: f32 = 0.0;

                if apply_movement {
                    let next_index = if self.angle >= 0 {
                        i as i32 + 1
                    } else {
                        i as i32 - 1
                    };
                    if next_index >= 0 {
                        let si2 =
                            baro.bar_images(next_index == ctx.center_bar, next_index as usize);
                        if let Some(si2) = si2
                            && si2.data.draw
                        {
                            dx = (si2.data.region.x - si1.data.region.x)
                                * angle_lerp.clamp(-1.0, 1.0);
                            dy = (si2.data.region.y - si1.data.region.y) * angle_lerp;
                        }
                    }
                }

                ba.x = (si1.data.region.x + dx) as i32 as f32;
                ba.y = ((si1.data.region.y + dy)
                    + if baro.position() == 1 {
                        si1.data.region.height
                    } else {
                        0.0
                    }) as i32 as f32;

                // set song bar type
                // Use i64 arithmetic to avoid usize wrapping when center_bar is negative.
                let index =
                    ((ctx.selectedindex as i64 + ctx.currentsongs.len() as i64 * 100 + i as i64
                        - ctx.center_bar as i64)
                        .rem_euclid(ctx.currentsongs.len() as i64)) as usize;
                let sd = &ctx.currentsongs[index];
                ba.sd = Some(index);

                match sd {
                    Bar::Table(_) | Bar::Hash(_) | Bar::Executable(_) => {
                        ba.value = 2;
                    }
                    Bar::Grade(gb) => {
                        ba.value = if gb.exists_all_songs() { 3 } else { 4 };
                    }
                    Bar::RandomCourse(rcb) => {
                        ba.value = if rcb.exists_all_songs() { 2 } else { 4 };
                    }
                    Bar::Folder(_) => {
                        ba.value = 1;
                    }
                    Bar::Song(sb) => {
                        ba.value = if sb.exists_song() { 0 } else { 4 };
                    }
                    Bar::SearchWord(_) => {
                        ba.value = 6;
                    }
                    Bar::Command(_) | Bar::Container(_) => {
                        ba.value = 5;
                    }
                    Bar::Function(fb) => {
                        ba.value = fb.display_bar_type();
                        ba.text = fb.display_text_type() as usize;
                    }
                    _ => {
                        ba.value = -1;
                    }
                }
            } else {
                ba.value = -1;
            }

            if ba.value != -1
                && !matches!(
                    ctx.currentsongs.get(ba.sd.unwrap_or(0)),
                    Some(Bar::Function(_))
                )
            {
                // Determine text type based on bar type
                // songstatus values:
                // 0:normal 1:new 2:SongBar(normal) 3:SongBar(new) 4:FolderBar(normal) 5:FolderBar(new)
                // 6:TableBar or HashBar 7:GradeBar(songs exist) 8:(SongBar or GradeBar)(songs missing)
                // 9:CommandBar or ContainerBar 10:SearchWordBar
                // If 3+ is not defined, use 0 or 1
                let mut songstatus = ba.value;
                if songstatus >= 2 {
                    songstatus += 4;
                    // If not defined, use 0:normal
                    if baro.text(songstatus as usize).is_none() {
                        songstatus = 0;
                    }
                } else if songstatus == 0 {
                    // SongBar — check if new (added within 24 hours)
                    if let Some(idx) = ba.sd
                        && let Some(Bar::Song(sb)) = ctx.currentsongs.get(idx)
                    {
                        let song = sb.song_data();
                        songstatus = if now_secs > song.chart.adddate + 3600 * 24 {
                            2 // SongBar(normal)
                        } else {
                            3 // SongBar(new)
                        };
                        // If not defined, fallback to 0:normal or 1:new
                        if baro.text(songstatus as usize).is_none() {
                            songstatus = if songstatus == 3 { 1 } else { 0 };
                        }
                    }
                } else {
                    // songstatus == 1 → FolderBar
                    if let Some(idx) = ba.sd
                        && let Some(Bar::Folder(fb)) = ctx.currentsongs.get(idx)
                    {
                        let data = fb.folder_data();
                        songstatus = if data
                            .as_ref()
                            .is_none_or(|d| now_secs > d.adddate() + 3600 * 24)
                        {
                            4 // FolderBar(normal)
                        } else {
                            5 // FolderBar(new)
                        };
                        // If not defined, fallback to 0:normal or 1:new
                        if baro.text(songstatus as usize).is_none() {
                            songstatus = if songstatus == 5 { 1 } else { 0 };
                        }
                    }
                }
                ba.text = songstatus as usize;
            }
        }
    }

    /// Draw all bar elements.
    /// Translates: Java BarRenderer.render(SkinObjectRenderer, SkinBar)
    pub fn render(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        baro: &mut SkinBar,
        ctx: &RenderContext,
    ) {
        if ctx.currentsongs.is_empty() {
            return;
        }

        self.update_bar_text_charset(baro, ctx);
        self.draw_bar_images(sprite, baro, ctx);
        self.draw_distribution_graphs(sprite, baro, ctx);
        self.draw_download_progress(sprite, baro, ctx);
        self.draw_bar_text(sprite, baro, ctx);
        self.draw_trophies(sprite, baro, ctx);
        self.draw_lamps(sprite, baro, ctx);
        self.draw_levels(sprite, baro, ctx);
        self.draw_feature_labels(sprite, baro, ctx);
    }

    /// Handle scroll input via keyboard/analog/mouse wheel.
    /// Translates: Java BarRenderer.input()
    pub fn input(&mut self, ctx: &mut BarInputContext) {
        // song bar scroll on mouse wheel
        let mut mov = -ctx.input.scroll();
        ctx.input.reset_scroll();

        // analog scroll
        let analog_up = ctx.property.analog_change(ctx.input, MusicSelectKey::Up);
        let analog_down = ctx.property.analog_change(ctx.input, MusicSelectKey::Down);
        self.analog_scroll_buffer += analog_up - analog_down;
        mov += self.analog_scroll_buffer / self.analog_ticks_per_scroll;
        self.analog_scroll_buffer %= self.analog_ticks_per_scroll;

        if mov != 0 {
            // set duration and angle for smooth song bar scroll animation
            let l = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            let mut remaining_scroll = if self.angle == 0 {
                0
            } else {
                (0i64.max(self.duration - l) / self.angle as i64) as i32
            };
            remaining_scroll = (remaining_scroll + mov).clamp(-2, 2);
            if remaining_scroll == 0 {
                self.angle = 0;
                self.duration = l;
            } else {
                let scroll_duration = 120 / remaining_scroll / remaining_scroll;
                self.angle = scroll_duration / remaining_scroll;
                self.duration = l + scroll_duration as i64;
            }
        }

        // song bar scroll by key
        if ctx
            .property
            .is_non_analog_pressed(ctx.input, MusicSelectKey::Up, false)
            || ctx.input.control_key_state(ControlKeys::Down)
        {
            let l = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            if self.duration == 0 {
                self.keyinput = true;
                mov = 1;
                self.duration = l + self.durationlow as i64;
                self.angle = self.durationlow;
            }
            if l > self.duration && self.keyinput {
                self.duration = l + self.durationhigh as i64;
                mov = 1;
                self.angle = self.durationhigh;
            }
        } else if ctx
            .property
            .is_non_analog_pressed(ctx.input, MusicSelectKey::Down, false)
            || ctx.input.control_key_state(ControlKeys::Up)
        {
            let l = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            if self.duration == 0 {
                self.keyinput = true;
                mov = -1;
                self.duration = l + self.durationlow as i64;
                self.angle = -self.durationlow;
            }
            if l > self.duration && self.keyinput {
                self.duration = l + self.durationhigh as i64;
                mov = -1;
                self.angle = -self.durationhigh;
            }
        } else {
            self.keyinput = false;
        }

        let l = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        if l > self.duration && !self.keyinput {
            self.duration = 0;
        }

        while mov > 0 {
            ctx.manager.mov(true);
            (ctx.stop_scratch)();
            (ctx.play_scratch)();
            mov -= 1;
        }
        while mov < 0 {
            ctx.manager.mov(false);
            (ctx.stop_scratch)();
            (ctx.play_scratch)();
            mov += 1;
        }
    }

    pub fn reset_input(&mut self) {
        let l = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        if l > self.duration {
            self.duration = 0;
        }
    }

    pub fn update_bar_text(&mut self) {
        self.bartextupdate = true;
    }

    pub fn dispose(&self) {
        // In Java: no-op (commented out favorite writing)
    }
}

#[cfg(test)]
mod tests;

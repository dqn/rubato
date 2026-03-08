use std::collections::HashSet;

use super::bar::bar::Bar;
use super::bar_manager::BarManager;
use super::music_select_key_property::{MusicSelectKey, MusicSelectKeyProperty};
use super::skin_bar::SkinBar;
use super::stubs::*;

/// Bar area data for rendering
struct BarArea {
    pub sd: Option<usize>, // index into currentsongs
    pub x: f32,
    pub y: f32,
    pub value: i32,
    pub text: usize,
}

impl BarArea {
    fn new() -> Self {
        Self {
            sd: None,
            x: 0.0,
            y: 0.0,
            value: -1,
            text: 0,
        }
    }
}

/// Context for BarRenderer::prepare()
/// Provides the data from MusicSelectSkin and BarManager needed for bar layout.
pub struct PrepareContext<'a> {
    pub center_bar: i32,
    pub currentsongs: &'a [Bar],
    pub selectedindex: usize,
}

/// Context for BarRenderer::render()
/// Provides the data from MusicSelector needed for bar drawing.
pub struct RenderContext<'a> {
    pub center_bar: i32,
    pub currentsongs: &'a [Bar],
    pub rival: bool,
    pub state: &'a dyn MainState,
    pub lnmode: i32,
    /// True if the loader thread has terminated and images should be reloaded.
    pub loader_finished: bool,
}

/// Context for BarRenderer::input()
/// Provides the data from MusicSelector needed for scroll input handling.
pub struct BarInputContext<'a> {
    pub input: &'a mut BMSPlayerInputProcessor,
    pub property: &'a MusicSelectKeyProperty,
    pub manager: &'a mut BarManager,
    /// Callback to play SCRATCH sound
    pub play_scratch: &'a mut dyn FnMut(),
    /// Callback to stop SCRATCH sound
    pub stop_scratch: &'a mut dyn FnMut(),
}

/// Context for BarRenderer::mouse_pressed()
/// Provides the data from MusicSelector needed for click detection.
pub struct MousePressedContext<'a> {
    pub clickable_bar: &'a [i32],
    pub center_bar: i32,
    pub currentsongs: &'a [Bar],
    pub selectedindex: usize,
    pub state: &'a dyn MainState,
    pub timer_now_time: i64,
}

/// Result of mouse_pressed indicating what action to take
pub enum MousePressedAction {
    /// No bar was clicked
    None,
    /// A bar was selected (left click) — index into currentsongs
    Select(usize),
    /// Close the current directory (right click)
    Close,
}

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
            bartextupdate: false,
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

            let index = (ctx.selectedindex + ctx.currentsongs.len() * 100 + i
                - ctx.center_bar as usize)
                % ctx.currentsongs.len();

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
                let index = (ctx.selectedindex + ctx.currentsongs.len() * 100 + i
                    - ctx.center_bar as usize)
                    % ctx.currentsongs.len();
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
                        songstatus = if now_secs > song.chart.adddate as i64 + 3600 * 24 {
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
                        songstatus = if data.is_none()
                            || now_secs > data.expect("data").adddate() as i64 + 3600 * 24
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

        // Update bar text character set for font preparation
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

        // Check terminated loader thread and load song images.
        // In Java: if(manager.loader != null && manager.loader.getState() == TERMINATED) { ... }
        // The caller sets loader_finished=true when the bar contents loader thread terminates,
        // then calls MusicSelector.load_selected_song_images() after this render pass.
        if ctx.loader_finished {
            self.bartextupdate = true;
        }

        // draw song bar
        let position = baro.position();
        for i in 0..self.barlength {
            let ba = &self.bararea[i];
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
                let position_offset = if position == 1 {
                    si.data.region.height
                } else {
                    0.0
                };
                si.draw_with_value(
                    sprite,
                    self.time,
                    ctx.state,
                    ba.value,
                    ba.x - si.data.region.x,
                    ba.y - si.data.region.y - position_offset,
                );
            }
        }

        // draw distribution graphs
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

        // download progress bars
        let download_tasks =
            rubato_song::md_processor::download_task_state::DownloadTaskState::get_running_download_tasks();
        if !download_tasks.is_empty() {
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
                            let task = task_arc.lock().expect("task_arc lock poisoned");
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

        // draw bar text
        for i in 0..self.barlength {
            let ba = &self.bararea[i];
            if ba.value == -1 {
                continue;
            }
            if let Some(idx) = ba.sd {
                let sd = &ctx.currentsongs[idx];
                if let Some(text) = baro.text.get_mut(ba.text).and_then(|o| o.as_mut()) {
                    text.get_text_data_mut().set_text(sd.title().to_string());
                    text.draw_with_offset(sprite, ba.x, ba.y);
                }
            }
        }

        // draw trophies
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

        // draw lamps
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

        // draw levels
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

        // draw feature labels (LN/MINE/RANDOM)
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

    /// Handle scroll input via keyboard/analog/mouse wheel.
    /// Translates: Java BarRenderer.input()
    pub fn input(&mut self, ctx: &mut BarInputContext) {
        // song bar scroll on mouse wheel
        let mut mov = -ctx.input.get_scroll();
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
mod tests {
    use super::*;
    use crate::select::bar::folder_bar::FolderBar;
    use crate::select::bar::song_bar::SongBar;
    use rubato_skin::stubs::{MainController, PlayerResource, SkinOffset, Timer};

    /// Create a test SkinImage with draw=true and specified region.
    /// Uses a default TextureRegion (no real texture, but valid for layout tests).
    fn make_test_image(x: f32, y: f32, w: f32, h: f32) -> SkinImage {
        let mut img = SkinImage::new_with_single(TextureRegion::default());
        img.data.draw = true;
        img.data.region = Rectangle::new(x, y, w, h);
        img
    }

    /// Mock MainState for testing (implements rubato_skin::stubs::MainState)
    struct MockMainState {
        timer: Timer,
        main: MainController,
        resource: PlayerResource,
    }

    impl Default for MockMainState {
        fn default() -> Self {
            Self {
                timer: Timer::default(),
                main: MainController { debug: false },
                resource: PlayerResource,
            }
        }
    }

    impl MainState for MockMainState {
        fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
            &self.timer
        }
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
        fn get_main(&self) -> &MainController {
            &self.main
        }
        fn get_image(&self, _id: i32) -> Option<rubato_skin::stubs::TextureRegion> {
            None
        }
        fn get_resource(&self) -> &PlayerResource {
            &self.resource
        }
    }

    fn make_song_data(sha256: &str, path: Option<&str>) -> SongData {
        let mut sd = SongData::default();
        sd.file.sha256 = sha256.to_string();
        if let Some(p) = path {
            sd.set_path(p.to_string());
        }
        sd
    }

    fn make_song_bar_bar(sha256: &str, path: Option<&str>) -> Bar {
        Bar::Song(Box::new(SongBar::new(make_song_data(sha256, path))))
    }

    #[test]
    fn test_bar_renderer_new() {
        let renderer = BarRenderer::new(300, 100, 5);
        assert_eq!(renderer.durationlow, 300);
        assert_eq!(renderer.durationhigh, 100);
        assert_eq!(renderer.analog_ticks_per_scroll, 5);
        assert_eq!(renderer.barlength, 60);
        assert_eq!(renderer.duration, 0);
        assert_eq!(renderer.angle, 0);
        assert!(!renderer.keyinput);
        assert!(!renderer.bartextupdate);
    }

    #[test]
    fn test_bar_renderer_two_phase_prepare_render() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        let mut bar = SkinBar::new(0);

        let songs: Vec<Bar> = (0..60)
            .map(|i| make_song_bar_bar(&format!("song{}", i), Some("/path.bms")))
            .collect();

        let prep_ctx = PrepareContext {
            center_bar: 0,
            currentsongs: &songs,
            selectedindex: 0,
        };

        // Phase 1: prepare
        renderer.prepare(&bar, 1000, &prep_ctx);
        assert_eq!(renderer.time, 1000);

        // Phase 2: render
        let mut sprite = SkinObjectRenderer::new();
        let state = MockMainState::default();
        let render_ctx = RenderContext {
            center_bar: 0,
            currentsongs: &songs,
            rival: false,
            state: &state,
            lnmode: 0,
            loader_finished: false,
        };
        renderer.render(&mut sprite, &mut bar, &render_ctx);
    }

    #[test]
    fn test_bar_renderer_prepare_stores_time() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        let bar = SkinBar::new(0);
        let songs = vec![make_song_bar_bar("a", Some("/a.bms"))];

        let ctx = PrepareContext {
            center_bar: 0,
            currentsongs: &songs,
            selectedindex: 0,
        };

        renderer.prepare(&bar, 5000, &ctx);
        assert_eq!(renderer.time, 5000);

        renderer.prepare(&bar, 10000, &ctx);
        assert_eq!(renderer.time, 10000);
    }

    #[test]
    fn test_bar_renderer_prepare_empty_songs() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        let bar = SkinBar::new(0);
        let songs: Vec<Bar> = Vec::new();

        let ctx = PrepareContext {
            center_bar: 0,
            currentsongs: &songs,
            selectedindex: 0,
        };

        renderer.prepare(&bar, 1000, &ctx);
        assert_eq!(renderer.time, 1000);
    }

    #[test]
    fn test_bar_renderer_prepare_bar_type_classification() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        let mut bar = SkinBar::new(0);

        // Set a bar image on index 0 with draw=true so prepare processes it
        bar.barimageon[0] = Some(make_test_image(10.0, 20.0, 100.0, 30.0));
        bar.barimageoff[0] = Some(make_test_image(10.0, 20.0, 100.0, 30.0));

        // Create a song bar (exists)
        let songs = vec![make_song_bar_bar("abc", Some("/path.bms"))];
        let ctx = PrepareContext {
            center_bar: 0,
            currentsongs: &songs,
            selectedindex: 0,
        };

        renderer.prepare(&bar, 1000, &ctx);

        // Bar area 0 should have value 0 (SongBar exists)
        assert_eq!(renderer.bararea[0].value, 0);
        assert!(renderer.bararea[0].sd.is_some());
    }

    #[test]
    fn test_bar_renderer_prepare_folder_bar_type() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        let mut bar = SkinBar::new(0);

        bar.barimageoff[0] = Some(make_test_image(0.0, 0.0, 100.0, 30.0));

        let songs = vec![Bar::Folder(Box::new(FolderBar::new(
            None,
            "test".to_string(),
        )))];
        let ctx = PrepareContext {
            center_bar: 1, // center is 1, so bar 0 uses off image
            currentsongs: &songs,
            selectedindex: 0,
        };

        renderer.prepare(&bar, 1000, &ctx);
        // FolderBar -> value 1
        assert_eq!(renderer.bararea[0].value, 1);
    }

    #[test]
    fn test_bar_renderer_prepare_song_bar_missing() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        let mut bar = SkinBar::new(0);

        bar.barimageoff[0] = Some(make_test_image(0.0, 0.0, 0.0, 0.0));

        // SongBar with no path = missing
        let songs = vec![make_song_bar_bar("abc", None)];
        let ctx = PrepareContext {
            center_bar: 1,
            currentsongs: &songs,
            selectedindex: 0,
        };

        renderer.prepare(&bar, 1000, &ctx);
        // Missing SongBar -> value 4
        assert_eq!(renderer.bararea[0].value, 4);
    }

    #[test]
    fn test_bar_renderer_update_bar_text() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        assert!(!renderer.bartextupdate);
        renderer.update_bar_text();
        assert!(renderer.bartextupdate);
    }

    #[test]
    fn test_bar_renderer_render_bartextupdate_collects_chars() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        renderer.bartextupdate = true;

        let mut bar = SkinBar::new(0);

        // Create a song bar with a non-empty title
        let mut sd = SongData::default();
        sd.file.sha256 = "abc".to_string();
        sd.set_path("/path.bms".to_string());
        sd.metadata.title = "Test Song Title".to_string();
        let songs = vec![Bar::Song(Box::new(SongBar::new(sd)))];

        let state = MockMainState::default();
        let render_ctx = RenderContext {
            center_bar: 0,
            currentsongs: &songs,
            rival: false,
            state: &state,
            lnmode: 0,
            loader_finished: false,
        };

        let mut sprite = SkinObjectRenderer::new();
        renderer.render(&mut sprite, &mut bar, &render_ctx);

        // bartextupdate should be reset after render
        assert!(!renderer.bartextupdate);
        // bartextcharset should contain characters from the song title
        assert!(!renderer.bartextcharset.is_empty());
        assert!(renderer.bartextcharset.contains(&'T'));
        assert!(renderer.bartextcharset.contains(&'e'));
    }

    #[test]
    fn test_bar_renderer_prepare_angle_zero_no_nan() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        let mut bar = SkinBar::new(0);

        // Set up two adjacent bar images so the lerp path is exercised
        bar.barimageoff[0] = Some(make_test_image(10.0, 20.0, 100.0, 30.0));
        bar.barimageoff[1] = Some(make_test_image(10.0, 60.0, 100.0, 30.0));

        let songs = vec![
            make_song_bar_bar("a", Some("/a.bms")),
            make_song_bar_bar("b", Some("/b.bms")),
        ];

        // Simulate: angle=0 but duration far in the future => apply_movement=true
        // This triggers the division-by-zero path: angle_lerp = ... / self.angle as f32
        renderer.angle = 0;
        renderer.duration = i64::MAX;

        let ctx = PrepareContext {
            center_bar: 2,
            currentsongs: &songs,
            selectedindex: 0,
        };

        renderer.prepare(&bar, 1000, &ctx);

        // When angle=0, no movement should be applied. Bar positions should match
        // the skin-defined positions (the bar image region coordinates).
        // Before the fix, division by zero produced Infinity which corrupted positions.
        assert_eq!(
            renderer.bararea[0].x, 10.0,
            "bararea[0].x should match skin position"
        );
        assert_eq!(
            renderer.bararea[0].y, 20.0,
            "bararea[0].y should match skin position"
        );
        assert_eq!(
            renderer.bararea[1].x, 10.0,
            "bararea[1].x should match skin position"
        );
        assert_eq!(
            renderer.bararea[1].y, 60.0,
            "bararea[1].y should match skin position"
        );
    }

    #[test]
    fn test_bar_renderer_mouse_pressed_no_songs() {
        let renderer = BarRenderer::new(300, 100, 5);
        let bar = SkinBar::new(0);
        let state = MockMainState::default();

        let ctx = MousePressedContext {
            clickable_bar: &[0, 1, 2],
            center_bar: 1,
            currentsongs: &[],
            selectedindex: 0,
            state: &state,
            timer_now_time: 0,
        };

        let result = renderer.mouse_pressed(&bar, 0, 100, 200, &ctx);
        assert!(matches!(result, MousePressedAction::None));
    }

    #[test]
    fn test_bar_renderer_mouse_pressed_no_hit() {
        let renderer = BarRenderer::new(300, 100, 5);
        let bar = SkinBar::new(0);
        let songs = vec![make_song_bar_bar("abc", Some("/path.bms"))];
        let state = MockMainState::default();

        let ctx = MousePressedContext {
            clickable_bar: &[0],
            center_bar: 0,
            currentsongs: &songs,
            selectedindex: 0,
            state: &state,
            timer_now_time: 0,
        };

        // No bar images set, so bar_images returns None -> no hit
        let result = renderer.mouse_pressed(&bar, 0, 100, 200, &ctx);
        assert!(matches!(result, MousePressedAction::None));
    }
}

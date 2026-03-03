// SkinBPMGraph.java -> skin_bpm_graph.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;

use bms_model::bms_model::BMSModel;

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::stubs::{
    Color, MainState, Pixmap, PixmapFormat, SongData, SongInformation, Texture, TextureRegion,
};

/// BPM transition graph
pub struct SkinBPMGraph {
    pub data: SkinObjectData,

    /// Graph texture
    shapetex: Option<TextureRegion>,
    time: i64,
    state_ref: bool, // flag indicating state was set
    model_set: bool,
    current: Option<SongData>,

    /// Time to complete gauge drawing (ms)
    delay: i32,
    /// Graph line width
    line_width: i32,

    main_line_color: Color,
    min_line_color: Color,
    max_line_color: Color,
    other_line_color: Color,
    stop_line_color: Color,
    transition_line_color: Color,

    bpm_data: Vec<[f64; 2]>,
    mainbpm: f64,
    minbpm: f64,
    maxbpm: f64,

    min_value: f64,
    max_value: f64,
    min_value_log: f64,
    max_value_log: f64,
}

impl SkinBPMGraph {
    pub fn new(
        delay: i32,
        line_width: i32,
        main_bpm_color: &str,
        min_bpm_color: &str,
        max_bpm_color: &str,
        other_bpm_color: &str,
        stop_line_color: &str,
        transition_line_color: &str,
    ) -> Self {
        let min_value = 1.0_f64 / 8.0;
        let max_value = 8.0_f64;

        let mut s = Self {
            data: SkinObjectData::new(),
            shapetex: None,
            time: 0,
            state_ref: false,
            model_set: false,
            current: None,
            delay: if delay > 0 { delay } else { 0 },
            line_width: if line_width > 0 { line_width } else { 2 },
            main_line_color: Color::value_of("00ff00"),
            min_line_color: Color::value_of("0000ff"),
            max_line_color: Color::value_of("ff0000"),
            other_line_color: Color::value_of("ffff00"),
            stop_line_color: Color::value_of("ff00ff"),
            transition_line_color: Color::value_of("7f7f7f"),
            bpm_data: Vec::new(),
            mainbpm: 0.0,
            minbpm: 0.0,
            maxbpm: 0.0,
            min_value,
            max_value,
            min_value_log: min_value.log10(),
            max_value_log: max_value.log10(),
        };

        let main_bpm_color_string = sanitize_hex_color(main_bpm_color);
        let min_bpm_color_string = sanitize_hex_color(min_bpm_color);
        let max_bpm_color_string = sanitize_hex_color(max_bpm_color);
        let other_bpm_color_string = sanitize_hex_color(other_bpm_color);
        let stop_line_color_string = sanitize_hex_color(stop_line_color);
        let transition_line_color_string = sanitize_hex_color(transition_line_color);

        if !main_bpm_color_string.is_empty() {
            s.main_line_color = Color::value_of(&main_bpm_color_string);
        }
        if !min_bpm_color_string.is_empty() {
            s.min_line_color = Color::value_of(&min_bpm_color_string);
        }
        if !max_bpm_color_string.is_empty() {
            s.max_line_color = Color::value_of(&max_bpm_color_string);
        }
        if !other_bpm_color_string.is_empty() {
            s.other_line_color = Color::value_of(&other_bpm_color_string);
        }
        if !stop_line_color_string.is_empty() {
            s.stop_line_color = Color::value_of(&stop_line_color_string);
        }
        if !transition_line_color_string.is_empty() {
            s.transition_line_color = Color::value_of(&transition_line_color_string);
        }

        s
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.time = time;
        self.state_ref = true;
        self.data.prepare(time, state);
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer, state: &dyn MainState) {
        let song = state.get_resource().get_songdata();
        let model = song.and_then(|s| s.get_bms_model());

        let song_changed = match (&self.current, song) {
            (None, None) => false,
            (Some(_), None) | (None, Some(_)) => true,
            (Some(_), Some(_)) => true, // simplified: always treat as changed
        };

        if self.shapetex.is_none() || song_changed || (!self.model_set && model.is_some()) {
            self.current = song.cloned();
            self.model_set = model.is_some();
            if let Some(s) = song {
                if let Some(info) = s.get_song_information() {
                    self.update_graph_from_info(info);
                } else {
                    self.update_graph_from_model(model);
                }
            } else {
                self.update_graph_from_model(None);
            }
        }

        if let Some(ref mut shapetex) = self.shapetex {
            let render = if self.time >= self.delay as i64 {
                1.0_f32
            } else {
                self.time as f32 / self.delay as f32
            };
            let tex_width = shapetex.get_texture().map(|t| t.get_width()).unwrap_or(0);
            shapetex.set_region_width((tex_width as f32 * render) as i32);
            let region = self.data.region.clone();
            let shapetex_clone = shapetex.clone();
            self.data.draw_image_at(
                sprite,
                &shapetex_clone,
                region.x,
                region.y + region.height,
                (region.width * render),
                -region.height,
            );
        }
    }

    fn update_graph_from_info(&mut self, info: &SongInformation) {
        let raw_data = info.get_speedchange_values();
        self.bpm_data = raw_data.to_vec();
        self.minbpm = f64::MAX;
        self.maxbpm = f64::MIN;
        for d in &self.bpm_data {
            if d[0] > 0.0 {
                self.minbpm = self.minbpm.min(d[0]);
            }
            self.maxbpm = self.maxbpm.min(d[0]); // Note: Java code has Math.min here too (likely a bug)
        }
        self.mainbpm = info.mainbpm;

        self.update_texture();
    }

    fn update_graph_from_model(&mut self, model: Option<&BMSModel>) {
        if let Some(model) = model {
            let mut speed_list: Vec<[f64; 2]> = Vec::new();
            let mut bpm_note_count_map: HashMap<u64, i32> = HashMap::new();
            let mut now_speed = model.get_bpm();
            speed_list.push([now_speed, 0.0]);
            let tls = model.get_all_time_lines();
            for tl in tls {
                let bpm_key = tl.get_bpm().to_bits();
                let notecount = bpm_note_count_map.get(&bpm_key).copied().unwrap_or(0);
                bpm_note_count_map.insert(bpm_key, notecount + tl.get_total_notes());

                if tl.get_stop() > 0 {
                    if now_speed != 0.0 {
                        now_speed = 0.0;
                        speed_list.push([now_speed, tl.get_time() as f64]);
                    }
                } else if now_speed != tl.get_bpm() * tl.get_scroll() {
                    now_speed = tl.get_bpm() * tl.get_scroll();
                    speed_list.push([now_speed, tl.get_time() as f64]);
                }
            }

            let mut maxcount = 0;
            for (bpm_key, count) in &bpm_note_count_map {
                if *count > maxcount {
                    maxcount = *count;
                    self.mainbpm = f64::from_bits(*bpm_key);
                }
            }
            if !speed_list.is_empty()
                && !tls.is_empty()
                && speed_list[speed_list.len() - 1][1] != tls[tls.len() - 1].get_time() as f64
            {
                speed_list.push([now_speed, tls[tls.len() - 1].get_time() as f64]);
            }

            self.bpm_data = speed_list;
            self.minbpm = model.get_min_bpm();
            self.maxbpm = model.get_max_bpm();
        } else {
            self.bpm_data = Vec::new();
        }
        self.update_texture();
    }

    fn update_texture(&mut self) {
        let shape: Pixmap = if self.bpm_data.len() < 2 {
            Pixmap::new(1, 1, PixmapFormat::RGBA8888)
        } else {
            let width = self.data.region.width.abs() as i32;
            let height = self.data.region.height.abs() as i32;
            let mut shape_pixmap = Pixmap::new(width, height, PixmapFormat::RGBA8888);

            let mut last_time = self.bpm_data[self.bpm_data.len() - 1][1] as i32;
            // In Java: song = state.main.getPlayerResource().getSongdata()
            // Stubbed: we skip the song length check
            last_time += 1000;

            // Graph drawing
            for i in 1..self.bpm_data.len() {
                // Vertical line
                let x1 = (width as f64 * self.bpm_data[i][1] / last_time as f64) as i32;
                let y1 = ((((self.bpm_data[i - 1][0] / self.mainbpm)
                    .max(self.min_value)
                    .min(self.max_value))
                .log10()
                    - self.min_value_log)
                    / (self.max_value_log - self.min_value_log)
                    * (height - self.line_width) as f64) as i32;
                let _x2 = x1;
                let y2 = ((((self.bpm_data[i][0] / self.mainbpm)
                    .max(self.min_value)
                    .min(self.max_value))
                .log10()
                    - self.min_value_log)
                    / (self.max_value_log - self.min_value_log)
                    * (height - self.line_width) as f64) as i32;
                if (y2 - y1).abs() - self.line_width > 0 {
                    shape_pixmap.set_color(&self.transition_line_color);
                    shape_pixmap.fill_rectangle(
                        x1,
                        y1.min(y2) + self.line_width,
                        self.line_width,
                        (y2 - y1).abs() - self.line_width,
                    );
                }
                // Horizontal line
                let x1 = (width as f64 * self.bpm_data[i - 1][1] / last_time as f64) as i32;
                let y1 = ((((self.bpm_data[i - 1][0] / self.mainbpm)
                    .max(self.min_value)
                    .min(self.max_value))
                .log10()
                    - self.min_value_log)
                    / (self.max_value_log - self.min_value_log)
                    * (height - self.line_width) as f64) as i32;
                let x2 = (width as f64 * self.bpm_data[i][1] / last_time as f64) as i32;
                let y2 = y1;
                let line_color = if self.bpm_data[i - 1][0] == self.mainbpm {
                    &self.main_line_color
                } else if self.bpm_data[i - 1][0] == self.minbpm {
                    &self.min_line_color
                } else if self.bpm_data[i - 1][0] == self.maxbpm {
                    &self.max_line_color
                } else if self.bpm_data[i - 1][0] <= 0.0 {
                    &self.stop_line_color
                } else {
                    &self.other_line_color
                };
                shape_pixmap.set_color(line_color);
                shape_pixmap.fill_rectangle(x1, y2, x2 - x1 + self.line_width, self.line_width);
            }
            // Last horizontal line
            let last_idx = self.bpm_data.len() - 1;
            let x1 = (width as f64 * self.bpm_data[last_idx][1] / last_time as f64) as i32;
            let y1 = ((((self.bpm_data[last_idx][0] / self.mainbpm)
                .max(self.min_value)
                .min(self.max_value))
            .log10()
                - self.min_value_log)
                / (self.max_value_log - self.min_value_log)
                * (height - self.line_width) as f64) as i32;
            let x2 = width;
            let y2 = y1;
            let line_color = if self.bpm_data[last_idx][0] == self.mainbpm {
                &self.main_line_color
            } else if self.bpm_data[last_idx][0] == self.minbpm {
                &self.min_line_color
            } else if self.bpm_data[last_idx][0] == self.maxbpm {
                &self.max_line_color
            } else if self.bpm_data[last_idx][0] <= 0.0 {
                &self.stop_line_color
            } else {
                &self.other_line_color
            };
            shape_pixmap.set_color(line_color);
            shape_pixmap.fill_rectangle(x1, y2, x2 - x1 + self.line_width, self.line_width);

            shape_pixmap
        };

        if let Some(ref mut shapetex) = self.shapetex
            && let Some(tex) = shapetex.texture.as_mut()
        {
            tex.dispose();
        }
        self.shapetex = Some(TextureRegion::from_texture(Texture::from_pixmap(&shape)));
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut shapetex) = self.shapetex
            && let Some(tex) = shapetex.texture.as_mut()
        {
            tex.dispose();
        }
        self.shapetex = None;
    }
}

/// Sanitize hex color string: remove non-hex chars, take first 6 chars max
fn sanitize_hex_color(s: &str) -> String {
    let cleaned: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let len = cleaned.len().min(6);
    cleaned[..len].to_string()
}

// SkinBPMGraph.java -> skin_bpm_graph.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;

use bms_model::bms_model::BMSModel;

use crate::reexports::{
    Color, MainState, Pixmap, PixmapFormat, SongData, SongInformation, Texture, TextureRegion,
};
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

/// Configuration for constructing a `SkinBPMGraph`.
pub struct BpmGraphConfig<'a> {
    pub delay: i32,
    pub line_width: i32,
    pub main_bpm_color: &'a str,
    pub min_bpm_color: &'a str,
    pub max_bpm_color: &'a str,
    pub other_bpm_color: &'a str,
    pub stop_line_color: &'a str,
    pub transition_line_color: &'a str,
}

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
    pub fn new(config: BpmGraphConfig<'_>) -> Self {
        let min_value = 1.0_f64 / 8.0;
        let max_value = 8.0_f64;

        let mut s = Self {
            data: SkinObjectData::new(),
            shapetex: None,
            time: 0,
            state_ref: false,
            model_set: false,
            current: None,
            delay: if config.delay > 0 { config.delay } else { 0 },
            line_width: if config.line_width > 0 {
                config.line_width
            } else {
                2
            },
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

        let main_bpm_color_string = sanitize_hex_color(config.main_bpm_color);
        let min_bpm_color_string = sanitize_hex_color(config.min_bpm_color);
        let max_bpm_color_string = sanitize_hex_color(config.max_bpm_color);
        let other_bpm_color_string = sanitize_hex_color(config.other_bpm_color);
        let stop_line_color_string = sanitize_hex_color(config.stop_line_color);
        let transition_line_color_string = sanitize_hex_color(config.transition_line_color);

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
        let song = state.song_data_ref();
        let model = song.and_then(|s| s.bms_model());

        let song_changed = match (&self.current, song) {
            (None, None) => false,
            (Some(_), None) | (None, Some(_)) => true,
            (Some(prev), Some(cur)) => prev.file.sha256 != cur.file.sha256,
        };

        if self.shapetex.is_none() || song_changed || (!self.model_set && model.is_some()) {
            self.current = song.cloned();
            self.model_set = model.is_some();
            if let Some(s) = song {
                if let Some(info) = s.info.as_ref() {
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
            let tex_width = shapetex.texture.as_ref().map(|t| t.width).unwrap_or(0);
            shapetex.region_width = (tex_width as f32 * render) as i32;
            // Java's TextureRegion.setRegionWidth() internally recalculates u2
            // from the texture dimensions. Mirror that here.
            if tex_width > 0 {
                shapetex.u2 = shapetex.region_width as f32 / tex_width as f32;
            }
            let region = self.data.region;
            let shapetex_clone = shapetex.clone();
            self.data.draw_image_at(
                sprite,
                &shapetex_clone,
                region.x,
                region.y + region.height,
                region.width * render,
                -region.height,
            );
        }
    }

    fn update_graph_from_info(&mut self, info: &SongInformation) {
        let raw_data = info.speedchange_values();
        self.bpm_data = raw_data.to_vec();
        self.minbpm = f64::MAX;
        self.maxbpm = f64::MIN;
        for d in &self.bpm_data {
            if d[0] > 0.0 {
                self.minbpm = self.minbpm.min(d[0]);
            }
            self.maxbpm = self.maxbpm.max(d[0]);
        }
        self.mainbpm = info.mainbpm;

        self.update_texture();
    }

    fn update_graph_from_model(&mut self, model: Option<&BMSModel>) {
        if let Some(model) = model {
            let mut speed_list: Vec<[f64; 2]> = Vec::new();
            let mut bpm_note_count_map: HashMap<u64, i32> = HashMap::new();
            let mut now_speed = model.bpm;
            speed_list.push([now_speed, 0.0]);
            let tls = &model.timelines;
            for tl in tls {
                let bpm_key = tl.bpm.to_bits();
                let notecount = bpm_note_count_map.get(&bpm_key).copied().unwrap_or(0);
                bpm_note_count_map.insert(bpm_key, notecount + tl.total_notes());

                if tl.stop() > 0 {
                    if now_speed != 0.0 {
                        now_speed = 0.0;
                        speed_list.push([now_speed, tl.time() as f64]);
                    }
                } else if now_speed != tl.bpm * tl.scroll {
                    now_speed = tl.bpm * tl.scroll;
                    speed_list.push([now_speed, tl.time() as f64]);
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
                && speed_list[speed_list.len() - 1][1] != tls[tls.len() - 1].time() as f64
            {
                speed_list.push([now_speed, tls[tls.len() - 1].time() as f64]);
            }

            self.bpm_data = speed_list;
            self.minbpm = model.min_bpm();
            self.maxbpm = model.max_bpm();
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
            if width <= 0 || height <= 0 {
                Pixmap::new(1, 1, PixmapFormat::RGBA8888)
            } else {
                let mut shape_pixmap = Pixmap::new(width, height, PixmapFormat::RGBA8888);

                let last_time = self.bpm_data[self.bpm_data.len() - 1][1] + 1000.0;
                // In Java: last_time = max(last_time, song.getInformation().getLastNoteTime() + 1000).
                // SkinBpmGraph does not have access to SongData here (it only receives bpm_data
                // via set_bpm_data). The +1000 padding provides a reasonable default end margin.

                if last_time > 0.0 {
                    let safe_mainbpm = if self.mainbpm == 0.0 {
                        1.0
                    } else {
                        self.mainbpm
                    };

                    // Graph drawing
                    for i in 1..self.bpm_data.len() {
                        // Vertical line
                        let x1 = (width as f64 * self.bpm_data[i][1] / last_time) as i32;
                        let y1 = ((((self.bpm_data[i - 1][0] / safe_mainbpm)
                            .max(self.min_value)
                            .min(self.max_value))
                        .log10()
                            - self.min_value_log)
                            / (self.max_value_log - self.min_value_log)
                            * (height - self.line_width) as f64)
                            as i32;
                        let _x2 = x1;
                        let y2 = ((((self.bpm_data[i][0] / safe_mainbpm)
                            .max(self.min_value)
                            .min(self.max_value))
                        .log10()
                            - self.min_value_log)
                            / (self.max_value_log - self.min_value_log)
                            * (height - self.line_width) as f64)
                            as i32;
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
                        let x1 = (width as f64 * self.bpm_data[i - 1][1] / last_time) as i32;
                        let y1 = ((((self.bpm_data[i - 1][0] / safe_mainbpm)
                            .max(self.min_value)
                            .min(self.max_value))
                        .log10()
                            - self.min_value_log)
                            / (self.max_value_log - self.min_value_log)
                            * (height - self.line_width) as f64)
                            as i32;
                        let x2 = (width as f64 * self.bpm_data[i][1] / last_time) as i32;
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
                        shape_pixmap.fill_rectangle(
                            x1,
                            y2,
                            x2 - x1 + self.line_width,
                            self.line_width,
                        );
                    }
                    // Last horizontal line
                    let last_idx = self.bpm_data.len() - 1;
                    let x1 = (width as f64 * self.bpm_data[last_idx][1] / last_time) as i32;
                    let y1 = ((((self.bpm_data[last_idx][0] / safe_mainbpm)
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
                }

                shape_pixmap
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::{Rectangle, SkinOffset, Timer};

    struct MockBpmState {
        timer: Timer,
    }

    impl Default for MockBpmState {
        fn default() -> Self {
            Self {
                timer: Timer::default(),
            }
        }
    }

    impl rubato_types::timer_access::TimerAccess for MockBpmState {
        fn now_time(&self) -> i64 {
            self.timer.now_time()
        }
        fn now_micro_time(&self) -> i64 {
            self.timer.now_micro_time()
        }
        fn micro_timer(&self, id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.micro_timer(id)
        }
        fn timer(&self, id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.timer(id)
        }
        fn now_time_for(&self, id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.now_time_for(id)
        }
        fn is_timer_on(&self, id: rubato_types::timer_id::TimerId) -> bool {
            self.timer.is_timer_on(id)
        }
    }

    impl rubato_types::skin_render_context::SkinRenderContext for MockBpmState {
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
    }

    impl MainState for MockBpmState {}

    /// Regression: when region_width is set for progressive reveal, u2 must also
    /// be updated. Java's TextureRegion.setRegionWidth() recalculates u2
    /// internally. Without this, the UV coordinate stays at 1.0 and the entire
    /// texture is sampled into a narrower rectangle, causing incorrect clipping.
    #[test]
    fn draw_updates_u2_on_progressive_reveal() {
        let config = BpmGraphConfig {
            delay: 1000,
            line_width: 2,
            main_bpm_color: "",
            min_bpm_color: "",
            max_bpm_color: "",
            other_bpm_color: "",
            stop_line_color: "",
            transition_line_color: "",
        };
        let mut graph = SkinBPMGraph::new(config);

        // Pre-set shapetex with a 200-wide texture so we skip the recreation path
        let tex = Texture {
            width: 200,
            height: 100,
            ..Default::default()
        };
        graph.shapetex = Some(TextureRegion::from_texture(tex));
        graph.data.region = Rectangle::new(0.0, 0.0, 200.0, 100.0);
        graph.time = 500; // half of delay=1000 => render=0.5
        graph.delay = 1000;

        let state = MockBpmState::default();
        let mut renderer = SkinObjectRenderer::new();
        graph.draw(&mut renderer, &state);

        let shapetex = graph.shapetex.as_ref().unwrap();
        // render = 0.5, tex_width = 200 => region_width = 100
        assert_eq!(shapetex.region_width, 100);
        // u2 should be 100/200 = 0.5
        assert!(
            (shapetex.u2 - 0.5).abs() < 1e-5,
            "u2 should be updated to 0.5 for progressive reveal, got {}",
            shapetex.u2
        );
    }

    /// Verify that at full render (time >= delay), u2 is 1.0.
    #[test]
    fn draw_u2_is_1_at_full_render() {
        let config = BpmGraphConfig {
            delay: 1000,
            line_width: 2,
            main_bpm_color: "",
            min_bpm_color: "",
            max_bpm_color: "",
            other_bpm_color: "",
            stop_line_color: "",
            transition_line_color: "",
        };
        let mut graph = SkinBPMGraph::new(config);

        let tex = Texture {
            width: 200,
            height: 100,
            ..Default::default()
        };
        graph.shapetex = Some(TextureRegion::from_texture(tex));
        graph.data.region = Rectangle::new(0.0, 0.0, 200.0, 100.0);
        graph.time = 2000; // past delay => render=1.0
        graph.delay = 1000;

        let state = MockBpmState::default();
        let mut renderer = SkinObjectRenderer::new();
        graph.draw(&mut renderer, &state);

        let shapetex = graph.shapetex.as_ref().unwrap();
        assert_eq!(shapetex.region_width, 200);
        assert!(
            (shapetex.u2 - 1.0).abs() < 1e-5,
            "u2 should be 1.0 at full render, got {}",
            shapetex.u2
        );
    }

    /// Regression: when the last bpm_data timestamp is exactly -1000.0,
    /// last_time becomes 0.0 and all x-coordinate divisions would produce
    /// infinity/NaN. update_texture must skip graph drawing for degenerate data.
    #[test]
    fn update_texture_skips_drawing_when_last_time_is_zero() {
        let config = BpmGraphConfig {
            delay: 0,
            line_width: 2,
            main_bpm_color: "",
            min_bpm_color: "",
            max_bpm_color: "",
            other_bpm_color: "",
            stop_line_color: "",
            transition_line_color: "",
        };
        let mut graph = SkinBPMGraph::new(config);
        graph.data.region = Rectangle::new(0.0, 0.0, 100.0, 50.0);
        // Two entries so bpm_data.len() >= 2 triggers the drawing path.
        // Second entry's timestamp is -1000.0, so last_time = -1000.0 + 1000.0 = 0.0.
        graph.bpm_data = vec![[120.0, 0.0], [120.0, -1000.0]];
        graph.mainbpm = 120.0;

        // Before the fix, this would divide by zero and produce NaN/infinity
        // in the pixel coordinate calculations.
        graph.update_texture();

        // The texture should still be created (just with no graph lines drawn).
        assert!(graph.shapetex.is_some());
    }

    /// Verify that negative last_time (timestamp < -1000) is also guarded.
    #[test]
    fn update_texture_skips_drawing_when_last_time_is_negative() {
        let config = BpmGraphConfig {
            delay: 0,
            line_width: 2,
            main_bpm_color: "",
            min_bpm_color: "",
            max_bpm_color: "",
            other_bpm_color: "",
            stop_line_color: "",
            transition_line_color: "",
        };
        let mut graph = SkinBPMGraph::new(config);
        graph.data.region = Rectangle::new(0.0, 0.0, 100.0, 50.0);
        // last_time = -2000.0 + 1000.0 = -1000.0 (negative)
        graph.bpm_data = vec![[120.0, 0.0], [120.0, -2000.0]];
        graph.mainbpm = 120.0;

        graph.update_texture();

        assert!(graph.shapetex.is_some());
    }

    /// Regression: when the region has zero width or height, update_texture
    /// should return a 1x1 placeholder pixmap instead of creating a 0x0 pixmap
    /// and running the graph drawing loop with degenerate coordinates.
    #[test]
    fn update_texture_returns_placeholder_for_zero_dimension_region() {
        let config = BpmGraphConfig {
            delay: 0,
            line_width: 2,
            main_bpm_color: "",
            min_bpm_color: "",
            max_bpm_color: "",
            other_bpm_color: "",
            stop_line_color: "",
            transition_line_color: "",
        };
        let mut graph = SkinBPMGraph::new(config);
        // Set region to zero width
        graph.data.region = Rectangle::new(0.0, 0.0, 0.0, 50.0);
        graph.bpm_data = vec![[120.0, 0.0], [180.0, 5000.0]];
        graph.mainbpm = 120.0;

        graph.update_texture();
        assert!(graph.shapetex.is_some());

        // Also test zero height
        graph.shapetex = None;
        graph.data.region = Rectangle::new(0.0, 0.0, 100.0, 0.0);
        graph.update_texture();
        assert!(graph.shapetex.is_some());
    }
}

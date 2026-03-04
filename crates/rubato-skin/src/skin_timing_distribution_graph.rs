// SkinTimingDistributionGraph.java -> skin_timing_distribution_graph.rs
// Mechanical line-by-line translation.

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::skin_timing_visualizer::{color_string_validation, get_judge_area};
use crate::stubs::{Color, MainState, MusicResult, Pixmap, PixmapFormat, Texture, TextureRegion};

/// Judge timing distribution graph
///
/// Translated from SkinTimingDistributionGraph.java
pub struct SkinTimingDistributionGraph {
    pub data: SkinObjectData,

    tex: Option<TextureRegion>,
    shape: Option<Pixmap>,

    gx: i32,
    c: i32,
    draw_average: bool,
    draw_dev: bool,
    max: i32,
    j_color: Vec<Color>,
    graph_color: Color,
    average_color: Color,
    dev_color: Color,

    // state is set during prepare, used in draw
    // In Java this holds a MusicResult reference. We use Option here.
    state_set: bool,
}

impl SkinTimingDistributionGraph {
    pub fn new(
        width: i32,
        line_width: i32,
        graph_color: &str,
        average_color: &str,
        dev_color: &str,
        pg_color: &str,
        gr_color: &str,
        gd_color: &str,
        bd_color: &str,
        pr_color: &str,
        draw_average: i32,
        draw_dev: i32,
    ) -> Self {
        let w = if 1 < width { width } else { 1 };
        let lw = line_width.clamp(1, width);
        let gx = w / lw;
        let c = gx / 2;
        let graph_color_val = Color::value_of(&color_string_validation(graph_color));
        let average_color_val = Color::value_of(&color_string_validation(average_color));
        let dev_color_val = Color::value_of(&color_string_validation(dev_color));
        let j_color = vec![
            Color::value_of(&color_string_validation(pg_color)),
            Color::value_of(&color_string_validation(gr_color)),
            Color::value_of(&color_string_validation(gd_color)),
            Color::value_of(&color_string_validation(bd_color)),
            Color::value_of(&color_string_validation(pr_color)),
        ];
        let draw_average = draw_average == 1;
        let draw_dev = draw_dev == 1;

        Self {
            data: SkinObjectData::new(),
            tex: None,
            shape: None,
            gx,
            c,
            draw_average,
            draw_dev,
            max: 10,
            j_color,
            graph_color: graph_color_val,
            average_color: average_color_val,
            dev_color: dev_color_val,
            state_set: false,
        }
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        // if(!(state instanceof MusicResult)) { draw = false; return; }
        // In Rust, the actual instanceof check would require downcasting.
        // The caller must ensure the state is MusicResult.
        // self.state = (MusicResult) state;
        self.state_set = true;
        self.data.prepare(time, state);
    }

    /// Draw the timing distribution graph.
    /// This requires MusicResult data to build the texture on first call.
    /// Since MusicResult is a stub, the actual rendering logic is preserved
    /// but will only work when the full runtime is available.
    pub fn draw_with_music_result(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        music_result: &MusicResult,
    ) {
        // Texture generation happens once
        if self.tex.is_none() {
            let td = music_result.get_timing_distribution();
            let dist = td.get_timing_distribution();
            let center = td.get_array_center();
            let judge_area = get_judge_area(&music_result.resource);

            let mut max = self.max;
            for &d in dist {
                if max < d {
                    max = (d / 10) * 10 + 10;
                }
            }
            self.max = max;

            let mut shape = Pixmap::new(self.gx, max, PixmapFormat::RGBA8888);
            // Graph area rendering
            shape.set_color(&self.j_color[0]);
            shape.fill_rectangle(self.c, 0, 1, max); // Just

            let mut beforex1 = self.c;
            let mut beforex2 = self.c + 1;
            for i in 0..self.j_color.len() {
                shape.set_color(&self.j_color[i]);
                let x1 = if judge_area.len() > i {
                    self.c + judge_area[i][0].clamp(-self.c, self.c)
                } else {
                    self.c
                };
                let x2 = if judge_area.len() > i {
                    self.c + judge_area[i][1].clamp(-self.c, self.c) + 1
                } else {
                    self.c + 1
                };

                if beforex1 > x1 {
                    shape.fill_rectangle(x1, 0, (x1 - beforex1).abs(), max);
                    beforex1 = x1;
                }

                if x2 > beforex2 {
                    shape.fill_rectangle(beforex2, 0, (x2 - beforex2).abs(), max);
                    beforex2 = x2;
                }
            }

            shape.set_color_rgba(0.0, 0.0, 0.0, 0.25);
            let mut x = self.c % 10;
            while x < self.c * 2 + 1 {
                shape.draw_line(x, 0, x, 1);
                x += 10;
            }

            // Average rendering
            if self.draw_average && td.get_average() != f32::MAX {
                let avg = td.get_average().round() as i32;
                shape.set_color(&self.average_color);
                shape.draw_line(self.c + avg, 0, self.c + avg, max);
            }

            // Deviation area rendering
            if self.draw_dev && td.get_std_dev() != -1.0 {
                let avg = td.get_average().round() as i32;
                let dev = td.get_std_dev().round() as i32;
                shape.set_color(&self.dev_color);
                shape.draw_line(self.c + avg + dev, 0, self.c + avg + dev, max);
                shape.draw_line(self.c + avg - dev, 0, self.c + avg - dev, max);
            }

            // Graph rendering
            shape.set_color(&self.graph_color);
            let mut i = -self.c;
            while i < self.gx - self.c {
                if -center < i && i < center {
                    let idx = (center + i) as usize;
                    if idx < dist.len() {
                        shape.fill_rectangle(self.c + i, max - dist[idx], 1, dist[idx]);
                    }
                }
                i += 1;
            }

            self.tex = Some(TextureRegion::from_texture(Texture::from_pixmap(&shape)));
            shape.dispose();
        }

        if let Some(ref tex) = self.tex.clone() {
            self.data.draw_image(sprite, tex);
        }
    }

    pub fn draw(&mut self, _sprite: &mut SkinObjectRenderer) {
        // In Java, draw() accesses this.state (MusicResult) directly.
        // In Rust, we need the MusicResult passed explicitly via draw_with_music_result().
        log::warn!("SkinTimingDistributionGraph::draw() called without MusicResult reference");
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut tex) = self.tex
            && let Some(t) = tex.texture.as_mut()
        {
            t.dispose();
        }
        if let Some(ref mut shape) = self.shape {
            shape.dispose();
        }
    }
}

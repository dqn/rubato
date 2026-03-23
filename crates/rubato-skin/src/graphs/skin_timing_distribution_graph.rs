// SkinTimingDistributionGraph.java -> skin_timing_distribution_graph.rs
// Mechanical line-by-line translation.

use crate::graphs::skin_timing_visualizer::color_string_validation;
use crate::reexports::{Color, MainState, Pixmap, PixmapFormat, Texture, TextureRegion};
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

/// Configuration for constructing a `SkinTimingDistributionGraph`.
pub struct TimingDistributionGraphConfig<'a> {
    pub width: i32,
    pub line_width: i32,
    pub graph_color: &'a str,
    pub average_color: &'a str,
    pub dev_color: &'a str,
    pub pg_color: &'a str,
    pub gr_color: &'a str,
    pub gd_color: &'a str,
    pub bd_color: &'a str,
    pub pr_color: &'a str,
    pub draw_average: i32,
    pub draw_dev: i32,
}

/// Judge timing distribution graph
///
/// Translated from SkinTimingDistributionGraph.java
pub struct SkinTimingDistributionGraph {
    pub data: SkinObjectData,

    tex: Option<TextureRegion>,

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
    pub fn new(config: TimingDistributionGraphConfig<'_>) -> Self {
        let w = config.width.max(1);
        let lw = config.line_width.clamp(1, w);
        let gx = (w / lw).max(1);
        let c = gx / 2;
        let graph_color_val = Color::value_of(&color_string_validation(config.graph_color));
        let average_color_val = Color::value_of(&color_string_validation(config.average_color));
        let dev_color_val = Color::value_of(&color_string_validation(config.dev_color));
        let j_color = vec![
            Color::value_of(&color_string_validation(config.pg_color)),
            Color::value_of(&color_string_validation(config.gr_color)),
            Color::value_of(&color_string_validation(config.gd_color)),
            Color::value_of(&color_string_validation(config.bd_color)),
            Color::value_of(&color_string_validation(config.pr_color)),
        ];
        let draw_average = config.draw_average == 1;
        let draw_dev = config.draw_dev == 1;

        Self {
            data: SkinObjectData::new(),
            tex: None,
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

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer, state: &dyn MainState) {
        // In Java, draw() accesses this.state (MusicResult) directly.
        // In Rust, we get timing distribution and judge area from MainState.
        if !self.state_set {
            return;
        }

        if self.tex.is_none() {
            let td = match state.get_timing_distribution() {
                Some(td) => td,
                None => return,
            };
            let dist = td.timing_distribution();
            let center = td.array_center();
            let judge_area = state.judge_area().unwrap_or_default();

            let mut max = self.max;
            for &d in dist {
                if max < d {
                    max = (d / 10) * 10 + 10;
                }
            }
            self.max = max;
            let max = max.min(4096);

            let mut shape = Pixmap::new(self.gx, max, PixmapFormat::RGBA8888);
            shape.set_color(&self.j_color[0]);
            shape.fill_rectangle(self.c, 0, 1, max);

            let mut beforex1 = self.c;
            let mut beforex2 = self.c + 1;
            for (i, color) in self.j_color.iter().enumerate() {
                shape.set_color(color);
                let x1 = if let Some(area) = judge_area.get(i) {
                    self.c + area[0].clamp(-self.c, self.c)
                } else {
                    self.c
                };
                let x2 = if let Some(area) = judge_area.get(i) {
                    self.c + area[1].clamp(-self.c, self.c) + 1
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

            if self.draw_average && td.average() != f32::MAX {
                let avg = td.average().round() as i32;
                shape.set_color(&self.average_color);
                shape.draw_line(self.c + avg, 0, self.c + avg, max);
            }

            if self.draw_dev && td.std_dev() != -1.0 {
                let avg = td.average().round() as i32;
                let dev = td.std_dev().round() as i32;
                shape.set_color(&self.dev_color);
                shape.draw_line(self.c + avg + dev, 0, self.c + avg + dev, max);
                shape.draw_line(self.c + avg - dev, 0, self.c + avg - dev, max);
            }

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

        if let Some(ref tex) = self.tex {
            self.data.draw_image(sprite, tex);
        }
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut tex) = self.tex
            && let Some(t) = tex.texture.as_mut()
        {
            t.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> TimingDistributionGraphConfig<'static> {
        TimingDistributionGraphConfig {
            width: 200,
            line_width: 2,
            graph_color: "FFFFFFFF",
            average_color: "FF0000FF",
            dev_color: "00FF00FF",
            pg_color: "00FF00FF",
            gr_color: "0000FFFF",
            gd_color: "FFFF00FF",
            bd_color: "FF00FFFF",
            pr_color: "888888FF",
            draw_average: 1,
            draw_dev: 1,
        }
    }

    #[test]
    fn gx_is_at_least_one_when_line_width_exceeds_width() {
        // Regression: when line_width > width, gx = width / line_width = 0,
        // causing a 0-width Pixmap and invisible graph.
        let mut config = default_config();
        config.width = 1;
        config.line_width = 100;
        let graph = SkinTimingDistributionGraph::new(config);
        assert!(
            graph.gx >= 1,
            "gx must be at least 1 to avoid 0-width Pixmap"
        );
    }

    #[test]
    fn gx_normal_case() {
        let mut config = default_config();
        config.width = 200;
        config.line_width = 2;
        let graph = SkinTimingDistributionGraph::new(config);
        assert_eq!(graph.gx, 100);
    }
}

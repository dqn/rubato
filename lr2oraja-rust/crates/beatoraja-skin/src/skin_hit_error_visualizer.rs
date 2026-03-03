// SkinHitErrorVisualizer.java -> skin_hit_error_visualizer.rs
// Mechanical line-by-line translation.

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::skin_timing_visualizer::color_string_validation;
use crate::stubs::{
    Color, MainState, Pixmap, PixmapFormat, PlayerResource, Texture, TextureRegion,
};

/// Early/Late HitError Visualization with EMA
///
/// Translated from SkinHitErrorVisualizer.java
pub struct SkinHitErrorVisualizer {
    pub data: SkinObjectData,

    shapetex: Option<TextureRegion>,
    shape: Option<Pixmap>,

    j_color: Vec<Color>,

    line_color: Color,
    center_color: Color,
    ema_color: Color,

    line_width: i32,
    width: i32,
    center: i32,
    window_length: i32,
    ema_mode: i32,
    judge_width_rate: f32,
    hiterror_mode: bool,
    color_mode: bool,
    draw_decay: bool,

    model_set: bool,
    judge_area: Vec<Vec<i32>>,

    current_index: i32,

    index: usize,
    recent: Vec<i64>,
    ema: Option<i64>,
    alpha: f32,
}

impl SkinHitErrorVisualizer {
    pub fn new(
        width: i32,
        judge_width_millis: i32,
        line_width: i32,
        color_mode: i32,
        hiterror_mode: i32,
        ema_mode: i32,
        line_color: &str,
        center_color: &str,
        pg_color: &str,
        gr_color: &str,
        gd_color: &str,
        bd_color: &str,
        pr_color: &str,
        ema_color: &str,
        alpha: f32,
        window_length: i32,
        transparent: i32,
        draw_decay: i32,
    ) -> Self {
        let line_width = line_width.clamp(1, 4);
        let center = judge_width_millis;
        let judge_width_rate = width as f32 / (judge_width_millis as f32 * 2.0 + 1.0);
        let line_color_val = Color::value_of(&color_string_validation(line_color));
        let center_color_val = Color::value_of(&color_string_validation(center_color));
        let ema_color_val = Color::value_of(&color_string_validation(ema_color));
        let j_color = vec![
            Color::value_of(&color_string_validation(pg_color)),
            Color::value_of(&color_string_validation(gr_color)),
            Color::value_of(&color_string_validation(gd_color)),
            Color::value_of(&color_string_validation(bd_color)),
            if transparent == 1 {
                Color::CLEAR
            } else {
                Color::value_of(pr_color)
            },
        ];
        let hiterror_mode = hiterror_mode == 1;
        let color_mode = color_mode == 1;
        let draw_decay = draw_decay == 1;
        let window_length = if window_length < 100 {
            window_length
        } else {
            100
        };

        Self {
            data: SkinObjectData::new(),
            shapetex: None,
            shape: None,
            j_color,
            line_color: line_color_val,
            center_color: center_color_val,
            ema_color: ema_color_val,
            line_width,
            width,
            center,
            window_length,
            ema_mode,
            judge_width_rate,
            hiterror_mode,
            color_mode,
            draw_decay,
            model_set: false,
            judge_area: Vec::new(),
            current_index: -1,
            index: 0,
            recent: Vec::new(),
            ema: Some(0),
            alpha,
        }
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        if !state.is_bms_player() {
            return;
        }
        self.data.prepare(time, state);

        self.index = state.get_recent_judges_index();
        self.recent = state.get_recent_judges().to_vec();
    }

    fn update_ema(&mut self, value: i64) {
        if let Some(ema) = self.ema {
            self.ema = Some(ema + (self.alpha * (value - ema) as f32) as i64);
        }
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if self.shape.is_none() {
            self.shape = Some(Pixmap::new(
                self.width,
                self.window_length * 2,
                PixmapFormat::RGBA8888,
            ));
        }

        let Some(shape) = self.shape.as_mut() else {
            return;
        };

        // Clear canvas
        shape.set_color(&Color::CLEAR);
        shape.fill();

        // Individual note hiterror
        if self.hiterror_mode {
            let mut i = self.window_length;
            while i > 0 {
                let j = -self.window_length + i + self.index as i32;
                let recent_len = self.recent.len() as i32;
                let cycle = if recent_len > 0 {
                    ((j % recent_len + recent_len) % recent_len) as usize
                } else {
                    i -= 1;
                    continue;
                };

                if cycle >= self.recent.len() || self.recent[cycle] == i64::MIN {
                    i -= 1;
                    continue;
                }

                // Judge color or single color
                if self.color_mode {
                    let judge = self.recent[cycle];
                    if !self.judge_area.is_empty()
                        && judge > self.judge_area[0][0] as i64
                        && judge < self.judge_area[0][1] as i64
                    {
                        shape.set_color(&self.j_color[0]);
                    } else if self.judge_area.len() > 1
                        && judge > self.judge_area[1][0] as i64
                        && judge < self.judge_area[1][1] as i64
                    {
                        shape.set_color(&self.j_color[1]);
                    } else if self.judge_area.len() > 2
                        && judge > self.judge_area[2][0] as i64
                        && judge < self.judge_area[2][1] as i64
                    {
                        shape.set_color(&self.j_color[2]);
                    } else if self.judge_area.len() > 3
                        && judge > self.judge_area[3][0] as i64
                        && judge < self.judge_area[3][1] as i64
                    {
                        shape.set_color(&self.j_color[3]);
                    } else {
                        shape.set_color(&self.j_color[4]);
                    }
                } else {
                    let alpha_val =
                        self.line_color.a * i as f32 / (1.0 * self.window_length as f32 / 2.0);
                    // Color.rgba8888 packs into int, then setColor from packed int
                    // For the stub Pixmap, we just use set_color_rgba
                    shape.set_color_rgba(
                        self.line_color.r,
                        self.line_color.g,
                        self.line_color.b,
                        alpha_val,
                    );
                }
                let clamped = (self.recent[cycle]).clamp(-(self.center as i64), self.center as i64);
                let x = (self.width - self.line_width) / 2
                    + (clamped as f32 * -self.judge_width_rate) as i32;
                // Draw decay shortens older hiterror lines
                if self.draw_decay {
                    shape.fill_rectangle(x, self.window_length - i, self.line_width, i * 2);
                } else {
                    shape.fill_rectangle(x, 0, self.line_width, self.recent.len() as i32 * 2);
                }

                i -= 1;
            }
        }

        // Centre line
        shape.set_color(&self.center_color);
        shape.fill_rectangle(
            (self.width - self.line_width) / 2,
            0,
            self.line_width,
            self.window_length * 2,
        );

        if self.ema_mode != 0 {
            if self.index < self.recent.len() {
                let last = self.recent[self.index];
                // Ignore misses
                if last != i64::MIN
                    && self.judge_area.len() > 3
                    && (last > self.judge_area[3][0] as i64 && last < self.judge_area[3][1] as i64)
                {
                    // Inline update_ema to avoid borrow conflict with shape
                    if let Some(ema) = self.ema {
                        self.ema = Some(ema + (self.alpha * (last - ema) as f32) as i64);
                    }
                }
            }
            let ema_val = self.ema.unwrap_or(0) as i32;
            let clamped_ema = ema_val.clamp(-self.center, self.center);
            let x = (self.width - self.line_width) / 2
                + (clamped_ema as f32 * -self.judge_width_rate) as i32;
            let mut w = (self.width as f32 * 0.01) as i32;

            // Line and/or Triangle style
            shape.set_color(&self.ema_color);
            if self.ema_mode == 1 || self.ema_mode == 3 {
                shape.fill_rectangle(x, 0, self.line_width, self.window_length * 2);
            }
            if self.ema_mode == 2 || self.ema_mode == 3 {
                let x = x + (self.line_width / 2);
                if w % 2 != 0 {
                    w += 1;
                }
                shape.fill_triangle(x, (self.window_length * 2) / 3, x + w, 0, x - w, 0);
            }
        }

        if self.shapetex.is_none() {
            if let Some(ref shape_ref) = self.shape {
                self.shapetex = Some(TextureRegion::from_texture(Texture::from_pixmap(shape_ref)));
            }
        } else {
            // shapetex.getTexture().draw(shape, 0, 0)
            if let Some(ref mut tex) = self.shapetex
                && let Some(ref mut t) = tex.texture
                && let Some(ref shape_ref) = self.shape
            {
                t.draw_pixmap(shape_ref, 0, 0);
            }
        }

        if let Some(ref shapetex) = self.shapetex.clone() {
            self.data.draw_image(sprite, shapetex);
        }
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut tex) = self.shapetex
            && let Some(t) = tex.texture.as_mut()
        {
            t.dispose();
        }
        if let Some(ref mut shape) = self.shape {
            shape.dispose();
        }
    }

    pub fn set_recent_judges(&mut self, index: usize, recent: Vec<i64>) {
        self.index = index;
        self.recent = recent;
    }

    pub fn set_judge_area(&mut self, judge_area: Vec<Vec<i32>>) {
        self.judge_area = judge_area;
    }
}

/// Gets judge area from player resource.
/// Same as SkinTimingVisualizer.getJudgeArea but with PlayerResource.
pub fn get_judge_area(resource: &PlayerResource) -> Vec<Vec<i32>> {
    crate::skin_timing_visualizer::get_judge_area_from_player_resource(resource)
}

/// Validates color string - delegates to SkinTimingVisualizer.
pub fn color_string_validation_hev(cs: &str) -> String {
    color_string_validation(cs)
}

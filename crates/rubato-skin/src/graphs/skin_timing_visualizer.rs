// SkinTimingVisualizer.java -> skin_timing_visualizer.rs
// Mechanical line-by-line translation.

use crate::stubs::{
    Color, MainState, MusicResultResource, Pixmap, PixmapFormat, PlayerResource, Texture,
    TextureRegion,
};
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

/// Judge timing visualizer
///
/// Translated from SkinTimingVisualizer.java
pub struct SkinTimingVisualizer {
    pub data: SkinObjectData,

    backtex: Option<TextureRegion>,
    line: Option<TextureRegion>,
    line_colors: Vec<Color>,

    j_color: Vec<Color>,
    /// Line color for judge history display
    line_color: Color,
    /// Center line color
    center_color: Color,

    /// Line width for judge history display
    line_width: i32,
    center: i32,
    judge_width_rate: f32,
    draw_decay: bool,

    model_set: bool,
    pub judge_area: Vec<Vec<i32>>,

    index: usize,
    recent: Vec<i64>,
}

impl SkinTimingVisualizer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        width: i32,
        judge_width_millis: i32,
        line_width: i32,
        line_color: &str,
        center_color: &str,
        pg_color: &str,
        gr_color: &str,
        gd_color: &str,
        bd_color: &str,
        pr_color: &str,
        transparent: i32,
        draw_decay: i32,
    ) -> Self {
        let line_width = line_width.clamp(1, 4);
        let center = judge_width_millis;
        let judge_width_rate = width as f32 / (judge_width_millis as f32 * 2.0 + 1.0);
        let line_color_val = Color::value_of(&color_string_validation(line_color));
        let center_color_val = Color::value_of(&color_string_validation(center_color));
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
        let draw_decay = draw_decay == 1;

        Self {
            data: SkinObjectData::new(),
            backtex: None,
            line: None,
            line_colors: Vec::new(),
            j_color,
            line_color: line_color_val,
            center_color: center_color_val,
            line_width,
            center,
            judge_width_rate,
            draw_decay,
            model_set: false,
            judge_area: Vec::new(),
            index: 0,
            recent: Vec::new(),
        }
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        if !state.is_bms_player() {
            return;
        }
        self.data.prepare(time, state);

        self.index = state.recent_judges_index();
        self.recent = state.recent_judges().to_vec();

        // if(resource.getBMSModel() != model) { ... }
        if !self.model_set {
            self.model_set = true;
            // judgeArea = getJudgeArea(resource)
            // self.judge_area = judge_area(resource);

            // BMSModel -> background texture generation
            let pwidth = self.center * 2 + 1;
            let mut shape = Pixmap::new(pwidth, 1, PixmapFormat::RGBA8888);

            let mut beforex1 = self.center;
            let mut beforex2 = self.center + 1;
            shape.set_color(&self.center_color);
            shape.fill_rectangle(self.center, 0, 1, 1);
            for (i, color) in self.j_color.iter().enumerate() {
                shape.set_color(color);
                let x1 = if let Some(area) = self.judge_area.get(i) {
                    self.center + area[0].clamp(-self.center, self.center)
                } else {
                    self.center
                };
                let x2 = if let Some(area) = self.judge_area.get(i) {
                    self.center + area[1].clamp(-self.center, self.center) + 1
                } else {
                    self.center + 1
                };

                if beforex1 > x1 {
                    shape.fill_rectangle(x1, 0, (x1 - beforex1).abs(), 1);
                    beforex1 = x1;
                }

                if x2 > beforex2 {
                    shape.fill_rectangle(beforex2, 0, (x2 - beforex2).abs(), 1);
                    beforex2 = x2;
                }
            }

            shape.set_color_rgba(0.0, 0.0, 0.0, 0.25);
            let mut x = self.center % 10;
            while x < pwidth {
                shape.draw_line(x, 0, x, 1);
                x += 10;
            }

            self.backtex = Some(TextureRegion::from_texture(Texture::from_pixmap(&shape)));
            shape.dispose();
        }

        if self.line.is_none() {
            let mut pix = Pixmap::new(self.line_width, 1, PixmapFormat::RGBA8888);
            pix.set_color(&Color::WHITE);
            pix.fill();
            self.line = Some(TextureRegion::from_texture(Texture::from_pixmap(&pix)));
            self.line_colors = (0..self.recent.len())
                .map(|i| {
                    Color::new(
                        self.line_color.r,
                        self.line_color.g,
                        self.line_color.b,
                        self.line_color.a / 100.0 * (i as f32 + 1.0),
                    )
                })
                .collect();
            pix.dispose();
        }
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if let Some(ref backtex) = self.backtex.clone() {
            self.data.draw_image(sprite, backtex);
        }
        let recent_len = self.recent.len();
        if recent_len == 0 {
            return;
        }
        for i in 0..recent_len {
            let j = (self.index + i + 1) % recent_len;
            if -(self.center as i64) <= self.recent[j] && self.recent[j] <= self.center as i64 {
                let line = match self.line.clone() {
                    Some(l) => l,
                    None => continue,
                };
                let region = &self.data.region;
                if self.draw_decay {
                    let x = region.x
                        + (region.width - self.line_width as f32) / 2.0
                        + self.recent[j] as f32 * self.judge_width_rate;
                    let y = region.y
                        + region.height * (recent_len - i) as f32 / recent_len as f32 / 2.0;
                    let h = region.height * i as f32 / recent_len as f32;
                    let color = if i < self.line_colors.len() {
                        self.line_colors[i]
                    } else {
                        self.line_color
                    };
                    self.data.draw_image_at_with_color(
                        sprite,
                        &line,
                        x,
                        y,
                        self.line_width as f32,
                        h,
                        &color,
                        0,
                    );
                } else {
                    let x = region.x
                        + (region.width - self.line_width as f32) / 2.0
                        + self.recent[j] as f32 * self.judge_width_rate;
                    let y = region.y;
                    let h = region.height;
                    let color = if i < self.line_colors.len() {
                        self.line_colors[i]
                    } else {
                        self.line_color
                    };
                    self.data.draw_image_at_with_color(
                        sprite,
                        &line,
                        x,
                        y,
                        self.line_width as f32,
                        h,
                        &color,
                        0,
                    );
                }
            }
        }
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut tex) = self.backtex
            && let Some(t) = tex.texture.as_mut()
        {
            t.dispose();
        }
        if let Some(ref mut tex) = self.line
            && let Some(t) = tex.texture.as_mut()
        {
            t.dispose();
        }
    }

    pub fn set_recent_judges(&mut self, index: usize, recent: Vec<i64>) {
        self.index = index;
        self.recent = recent;
    }
}

/// Validates that a color string contains only hex characters and is at least 6 chars.
/// Returns opaque red ("FF0000FF") if invalid.
pub fn color_string_validation(cs: &str) -> String {
    let all_hex = cs.chars().all(|c| c.is_ascii_hexdigit());
    if !all_hex || cs.len() < 6 {
        "FF0000FF".to_string()
    } else {
        cs.to_string()
    }
}

/// Gets judge area from player resource.
/// Returns judge windows as Vec<Vec<i32>> (5 judge levels x [early, late]).
pub fn judge_area(resource: &MusicResultResource) -> Vec<Vec<i32>> {
    let model = resource.bms_model();
    let mode = resource.original_mode();
    let rule = rubato_play::bms_player_rule::BMSPlayerRule::for_mode(&mode);

    let judgerank = model.judgerank();
    let config = resource.player_config();
    let mut judge_window_rate = if config.judge_settings.custom_judge {
        vec![
            config.judge_settings.key_judge_window_rate_perfect_great,
            config.judge_settings.key_judge_window_rate_great,
            config.judge_settings.key_judge_window_rate_good,
        ]
    } else {
        vec![100, 100, 100]
    };

    for constraint in resource.constraint() {
        match constraint {
            rubato_core::course_data::CourseDataConstraint::NoGreat => {
                judge_window_rate[1] = 0;
                judge_window_rate[2] = 0;
            }
            rubato_core::course_data::CourseDataConstraint::NoGood => {
                judge_window_rate[2] = 0;
            }
            _ => {}
        }
    }

    rule.judge.note_judge(judgerank, &judge_window_rate)
}

/// Gets judge area from player resource (using the PlayerResource stub).
pub fn judge_area_from_player_resource(resource: &PlayerResource) -> Vec<Vec<i32>> {
    let model = resource.bms_model();
    let mode = resource.original_mode();
    let rule = rubato_play::bms_player_rule::BMSPlayerRule::for_mode(&mode);

    let judgerank = model.judgerank();
    let config = resource.player_config();
    let judge_window_rate = if config.judge_settings.custom_judge {
        vec![
            config.judge_settings.key_judge_window_rate_perfect_great,
            config.judge_settings.key_judge_window_rate_great,
            config.judge_settings.key_judge_window_rate_good,
        ]
    } else {
        vec![100, 100, 100]
    };

    // Constraint handling would mutate judge_window_rate
    // Deferred to runtime when PlayerResource is fully available

    rule.judge.note_judge(judgerank, &judge_window_rate)
}

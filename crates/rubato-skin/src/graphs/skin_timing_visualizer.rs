// SkinTimingVisualizer.java -> skin_timing_visualizer.rs
// Mechanical line-by-line translation.

use crate::reexports::{Color, MainState, Pixmap, PixmapFormat, Texture, TextureRegion};
use crate::types::skin_object::{DrawImageAtParams, SkinObjectData, SkinObjectRenderer};

/// Configuration for constructing a `SkinTimingVisualizer`.
pub struct TimingVisualizerConfig<'a> {
    pub width: i32,
    pub judge_width_millis: i32,
    pub line_width: i32,
    pub line_color: &'a str,
    pub center_color: &'a str,
    pub pg_color: &'a str,
    pub gr_color: &'a str,
    pub gd_color: &'a str,
    pub bd_color: &'a str,
    pub pr_color: &'a str,
    pub transparent: i32,
    pub draw_decay: i32,
}

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
    pub fn new(config: TimingVisualizerConfig<'_>) -> Self {
        let line_width = config.line_width.clamp(1, 4);
        let center = config.judge_width_millis.clamp(1, 5000);
        let judge_width_rate = config.width as f32 / (center as f32 * 2.0 + 1.0);
        let line_color_val = Color::value_of(&color_string_validation(config.line_color));
        let center_color_val = Color::value_of(&color_string_validation(config.center_color));
        let j_color = vec![
            Color::value_of(&color_string_validation(config.pg_color)),
            Color::value_of(&color_string_validation(config.gr_color)),
            Color::value_of(&color_string_validation(config.gd_color)),
            Color::value_of(&color_string_validation(config.bd_color)),
            if config.transparent == 1 {
                Color::CLEAR
            } else {
                Color::value_of(config.pr_color)
            },
        ];
        let draw_decay = config.draw_decay == 1;

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
            if let Some(ja) = state.judge_area() {
                self.judge_area = ja;
            }

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
            pix.dispose();
        }

        if !self.recent.is_empty() && self.line_colors.len() != self.recent.len() {
            let recent_len = self.recent.len() as f32;
            self.line_colors = (0..self.recent.len())
                .map(|i| {
                    // NOTE: Java parity divergence. Java uses hardcoded divisor 100 which
                    // causes alpha > 1.0 (clamped) for indices >= 100 when the recent
                    // buffer is larger than 100 entries. We use the actual buffer length
                    // so alpha scales correctly across the full range.
                    Color::new(
                        self.line_color.r,
                        self.line_color.g,
                        self.line_color.b,
                        self.line_color.a * (i as f32 + 1.0) / recent_len,
                    )
                })
                .collect();
        }
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if let Some(ref backtex) = self.backtex {
            self.data.draw_image(sprite, backtex);
        }
        let recent_len = self.recent.len();
        if recent_len == 0 {
            return;
        }
        for i in 0..recent_len {
            let j = (self.index + i + 1) % recent_len;
            if -(self.center as i64) <= self.recent[j] && self.recent[j] <= self.center as i64 {
                let line = match self.line {
                    Some(ref l) => l,
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
                        &DrawImageAtParams {
                            image: line,
                            x,
                            y,
                            width: self.line_width as f32,
                            height: h,
                            color: &color,
                            angle: 0,
                        },
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
                        &DrawImageAtParams {
                            image: line,
                            x,
                            y,
                            width: self.line_width as f32,
                            height: h,
                            color: &color,
                            angle: 0,
                        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::{SkinOffset, Timer};

    /// Mock MainState that provides judge_area and is_bms_player.
    struct MockPlayState {
        timer: Timer,
        judge_area: Option<Vec<Vec<i32>>>,
        recent_judges: Vec<i64>,
        recent_judges_index: usize,
    }

    impl MockPlayState {
        fn with_judge_area(judge_area: Vec<Vec<i32>>) -> Self {
            Self {
                timer: Timer::default(),
                judge_area: Some(judge_area),
                recent_judges: Vec::new(),
                recent_judges_index: 0,
            }
        }
    }

    impl rubato_types::timer_access::TimerAccess for MockPlayState {
        fn now_time(&self) -> i64 {
            self.timer.now_time()
        }
        fn now_micro_time(&self) -> i64 {
            self.timer.now_micro_time()
        }
        fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.micro_timer(timer_id)
        }
        fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.timer(timer_id)
        }
        fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.now_time_for(timer_id)
        }
        fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
            self.timer.is_timer_on(timer_id)
        }
    }

    impl rubato_types::skin_render_context::SkinRenderContext for MockPlayState {
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
        fn is_bms_player(&self) -> bool {
            true
        }
        fn judge_area(&self) -> Option<Vec<Vec<i32>>> {
            self.judge_area.clone()
        }
        fn recent_judges(&self) -> &[i64] {
            &self.recent_judges
        }
        fn recent_judges_index(&self) -> usize {
            self.recent_judges_index
        }
    }

    impl crate::reexports::MainState for MockPlayState {}

    fn default_config() -> TimingVisualizerConfig<'static> {
        TimingVisualizerConfig {
            width: 200,
            judge_width_millis: 100,
            line_width: 2,
            line_color: "FFFFFFFF",
            center_color: "FF0000FF",
            pg_color: "00FF00FF",
            gr_color: "0000FFFF",
            gd_color: "FFFF00FF",
            bd_color: "FF00FFFF",
            pr_color: "888888FF",
            transparent: 0,
            draw_decay: 0,
        }
    }

    #[test]
    fn prepare_populates_judge_area_from_state() {
        let ja = vec![
            vec![-20, 20],
            vec![-40, 40],
            vec![-80, 80],
            vec![-150, 150],
            vec![-1000, 1000],
        ];
        let state = MockPlayState::with_judge_area(ja.clone());
        let mut viz = SkinTimingVisualizer::new(default_config());

        assert!(viz.judge_area.is_empty(), "judge_area should start empty");

        viz.prepare(0, &state);

        assert_eq!(
            viz.judge_area, ja,
            "judge_area must be populated from state after prepare()"
        );
    }

    #[test]
    fn prepare_sets_model_set_only_once() {
        let ja1 = vec![
            vec![-20, 20],
            vec![-40, 40],
            vec![-80, 80],
            vec![-150, 150],
            vec![-1000, 1000],
        ];
        let state = MockPlayState::with_judge_area(ja1.clone());
        let mut viz = SkinTimingVisualizer::new(default_config());

        viz.prepare(0, &state);
        assert_eq!(viz.judge_area, ja1);

        // Second prepare should not overwrite (model_set is true)
        let ja2 = vec![vec![-10, 10]];
        let state2 = MockPlayState::with_judge_area(ja2);
        viz.prepare(100, &state2);
        assert_eq!(
            viz.judge_area, ja1,
            "judge_area should not change after model_set"
        );
    }

    #[test]
    fn line_colors_alpha_scales_correctly_for_large_buffers() {
        // Regression: with the old formula (alpha / 100.0 * (i+1)), indices >= 100
        // produced alpha > 1.0 (clamped). With the fix, alpha scales to the actual
        // buffer length, so the last entry has exactly line_color.a.
        let ja = vec![
            vec![-20, 20],
            vec![-40, 40],
            vec![-80, 80],
            vec![-150, 150],
            vec![-1000, 1000],
        ];
        let mut state = MockPlayState::with_judge_area(ja);
        // Use a 500-entry recent buffer (larger than the old hardcoded 100).
        state.recent_judges = vec![0i64; 500];

        let mut viz = SkinTimingVisualizer::new(default_config());
        viz.prepare(0, &state);

        assert_eq!(viz.line_colors.len(), 500);

        // First entry: alpha = line_color.a * 1.0 / 500.0
        let first_alpha = viz.line_colors[0].a;
        let expected_first = viz.line_color.a / 500.0;
        assert!(
            (first_alpha - expected_first).abs() < 1e-5,
            "first entry alpha {first_alpha} should be ~{expected_first}"
        );

        // Last entry: alpha = line_color.a * 500.0 / 500.0 = line_color.a
        let last_alpha = viz.line_colors[499].a;
        assert!(
            (last_alpha - viz.line_color.a).abs() < 1e-5,
            "last entry alpha {last_alpha} should equal line_color.a {}",
            viz.line_color.a
        );

        // No entry should exceed line_color.a
        for (i, color) in viz.line_colors.iter().enumerate() {
            assert!(
                color.a <= viz.line_color.a + 1e-5,
                "entry {i} alpha {} exceeds line_color.a {}",
                color.a,
                viz.line_color.a
            );
        }
    }
}

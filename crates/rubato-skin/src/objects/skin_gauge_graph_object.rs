// SkinGaugeGraphObject.java -> skin_gauge_graph_object.rs
// Mechanical line-by-line translation.
// Gauge transition graph object (result screen).

use crate::json::json_skin_object_loader::parse_hex_color;
use crate::reexports::{MainState, Pixmap, PixmapFormat, Texture, TextureRegion};
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};
use rubato_render::color::Color;

/// Type-to-color-index mapping table (Java: typetable)
const TYPE_TABLE: [usize; 10] = [0, 1, 2, 3, 4, 5, 3, 4, 5, 3];

/// Color strings for constructing a gauge graph from hex color values.
pub struct GaugeGraphColorStrings<'a> {
    pub assist_clear_bg: &'a str,
    pub assist_easy_fail_bg: &'a str,
    pub groove_fail_bg: &'a str,
    pub groove_clear_hard_bg: &'a str,
    pub ex_hard_bg: &'a str,
    pub hazard_bg: &'a str,
    pub assist_clear_line: &'a str,
    pub assist_easy_fail_line: &'a str,
    pub groove_fail_line: &'a str,
    pub groove_clear_hard_line: &'a str,
    pub ex_hard_line: &'a str,
    pub hazard_line: &'a str,
    pub borderline_color: &'a str,
    pub border_color: &'a str,
}

/// Gauge transition graph rendering object.
///
/// Corresponds to Java `SkinGaugeGraphObject`.
/// Renders a gauge history graph on the result screen showing gauge value over time.
pub struct SkinGaugeGraphObject {
    pub data: SkinObjectData,
    /// Delay before graph is fully drawn (ms)
    pub delay: i32,
    /// Line width for the graph
    pub line_width: i32,
    /// Background colors per gauge type (below border)
    graph_color: [Color; 6],
    /// Graph line colors per gauge type (below border)
    graph_line: [Color; 6],
    /// Background colors per gauge type (above border)
    border_color: [Color; 6],
    /// Graph line colors per gauge type (above border)
    border_line: [Color; 6],

    // Runtime state for gauge graph rendering
    /// Sentinel -1 means "not yet assigned". All index usages are guarded by bounds
    /// checks (TYPE_TABLE.len(), Vec::get()), so -1 as usize safely falls through to defaults.
    current_type: i32,
    color: usize,
    gaugehistory: Vec<f32>,
    section: Vec<i32>,
    border: f32,
    max: f32,
    render: f32,
    redraw: bool,
    backtex: Option<TextureRegion>,
    shapetex: Option<TextureRegion>,
}

impl SkinGaugeGraphObject {
    /// Creates a SkinGaugeGraphObject with default colors.
    pub fn new_default() -> Self {
        let default_colors: [[Color; 4]; 6] = [
            [
                Color::value_of("ff0000"),
                Color::value_of("440000"),
                Color::value_of("ff00ff"),
                Color::value_of("440044"),
            ],
            [
                Color::value_of("ff0000"),
                Color::value_of("440000"),
                Color::value_of("00ffff"),
                Color::value_of("004444"),
            ],
            [
                Color::value_of("ff0000"),
                Color::value_of("440000"),
                Color::value_of("00ff00"),
                Color::value_of("004400"),
            ],
            [
                Color::value_of("ff0000"),
                Color::value_of("440000"),
                Color::BLACK,
                Color::BLACK,
            ],
            [
                Color::value_of("ffff00"),
                Color::value_of("444400"),
                Color::BLACK,
                Color::BLACK,
            ],
            [
                Color::value_of("cccccc"),
                Color::value_of("444444"),
                Color::BLACK,
                Color::BLACK,
            ],
        ];

        let mut graph_color = [Color::BLACK; 6];
        let mut graph_line = [Color::BLACK; 6];
        let mut border_color = [Color::BLACK; 6];
        let mut border_line = [Color::BLACK; 6];

        for i in 0..6 {
            border_line[i] = default_colors[i][0];
            border_color[i] = default_colors[i][1];
            graph_line[i] = if i < 3 {
                default_colors[i][2]
            } else {
                default_colors[i][0]
            };
            graph_color[i] = if i < 3 {
                default_colors[i][3]
            } else {
                default_colors[i][1]
            };
        }

        Self {
            data: SkinObjectData::new(),
            delay: 1500,
            line_width: 2,
            graph_color,
            graph_line,
            border_color,
            border_line,
            current_type: -1,
            color: 0,
            gaugehistory: Vec::new(),
            section: Vec::new(),
            border: 80.0,
            max: 100.0,
            render: 0.0,
            redraw: false,
            backtex: None,
            shapetex: None,
        }
    }

    /// Creates a SkinGaugeGraphObject from a flat color array.
    ///
    /// Corresponds to Java constructor `SkinGaugeGraphObject(Color[][] colors)`.
    /// The color strings are laid out as 6 groups of 4:
    ///   [borderline, bordercolor, graphline, graphcolor] for each gauge type.
    /// If fewer than 24 strings are provided, missing entries default to black.
    pub fn new_from_colors(color_strings: &[String]) -> Self {
        let fallback = Color::BLACK;

        let mut graph_color = [Color::BLACK; 6];
        let mut graph_line = [Color::BLACK; 6];
        let mut border_color = [Color::BLACK; 6];
        let mut border_line = [Color::BLACK; 6];

        // Java: colors[i/4][i%4] where layout is [borderline, bordercolor, graphline, graphcolor]
        for i in 0..6 {
            let base = i * 4;
            border_line[i] = if base < color_strings.len() {
                parse_hex_color(&color_strings[base], fallback)
            } else {
                fallback
            };
            border_color[i] = if base + 1 < color_strings.len() {
                parse_hex_color(&color_strings[base + 1], fallback)
            } else {
                fallback
            };
            graph_line[i] = if base + 2 < color_strings.len() {
                parse_hex_color(&color_strings[base + 2], fallback)
            } else {
                fallback
            };
            graph_color[i] = if base + 3 < color_strings.len() {
                parse_hex_color(&color_strings[base + 3], fallback)
            } else {
                fallback
            };
        }

        Self {
            data: SkinObjectData::new(),
            delay: 1500,
            line_width: 2,
            graph_color,
            graph_line,
            border_color,
            border_line,
            current_type: -1,
            color: 0,
            gaugehistory: Vec::new(),
            section: Vec::new(),
            border: 80.0,
            max: 100.0,
            render: 0.0,
            redraw: false,
            backtex: None,
            shapetex: None,
        }
    }

    /// Creates a SkinGaugeGraphObject from JSON color strings.
    ///
    /// Corresponds to Java constructor with 14 string parameters.
    pub fn new_from_color_strings(colors: &GaugeGraphColorStrings<'_>) -> Self {
        let fallback = Color::BLACK;

        let mut graph_color = [Color::BLACK; 6];
        let mut graph_line = [Color::BLACK; 6];
        let mut border_color = [Color::BLACK; 6];
        let mut border_line = [Color::BLACK; 6];

        // Below-border background colors
        graph_color[0] = parse_hex_color(colors.assist_clear_bg, fallback);
        graph_color[1] = parse_hex_color(colors.assist_easy_fail_bg, fallback);
        graph_color[2] = parse_hex_color(colors.groove_fail_bg, fallback);

        // Above-border background colors
        border_color[3] = parse_hex_color(colors.groove_clear_hard_bg, fallback);
        border_color[4] = parse_hex_color(colors.ex_hard_bg, fallback);
        border_color[5] = parse_hex_color(colors.hazard_bg, fallback);

        // Below-border line colors
        graph_line[0] = parse_hex_color(colors.assist_clear_line, fallback);
        graph_line[1] = parse_hex_color(colors.assist_easy_fail_line, fallback);
        graph_line[2] = parse_hex_color(colors.groove_fail_line, fallback);

        // Above-border line colors
        border_line[3] = parse_hex_color(colors.groove_clear_hard_line, fallback);
        border_line[4] = parse_hex_color(colors.ex_hard_line, fallback);
        border_line[5] = parse_hex_color(colors.hazard_line, fallback);

        // Shared border colors for types 0-2
        let bl = parse_hex_color(colors.borderline_color, fallback);
        let bc = parse_hex_color(colors.border_color, fallback);
        for i in 0..3 {
            border_line[i] = bl;
            border_color[i] = bc;
        }
        // For types 3-5, graph colors = border colors (Java lines 107-110)
        graph_line[3..6].copy_from_slice(&border_line[3..6]);
        graph_color[3..6].copy_from_slice(&border_color[3..6]);

        Self {
            data: SkinObjectData::new(),
            delay: 1500,
            line_width: 2,
            graph_color,
            graph_line,
            border_color,
            border_line,
            current_type: -1,
            color: 0,
            gaugehistory: Vec::new(),
            section: Vec::new(),
            border: 80.0,
            max: 100.0,
            render: 0.0,
            redraw: false,
            backtex: None,
            shapetex: None,
        }
    }

    pub fn delay(&self) -> i32 {
        self.delay
    }

    pub fn line_width(&self) -> i32 {
        self.line_width
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);

        self.render = if time >= self.delay as i64 {
            1.0
        } else if self.delay > 0 {
            time as f32 / self.delay as f32
        } else {
            1.0
        };

        let current_type = state.result_gauge_type();
        if self.current_type != current_type {
            self.current_type = current_type;
            self.redraw = true;

            if current_type < 0 {
                self.gaugehistory.clear();
                return;
            }

            self.gaugehistory = state
                .gauge_history()
                .and_then(|gh| gh.get(self.current_type as usize))
                .cloned()
                .unwrap_or_default();

            self.section = Vec::new();
            let course_history = state.course_gauge_history();
            if !course_history.is_empty() {
                self.gaugehistory = Vec::new();
                for stage in course_history {
                    if let Some(type_history) = stage.get(self.current_type as usize) {
                        self.gaugehistory.extend_from_slice(type_history);
                        let prev = self.section.last().copied().unwrap_or(0);
                        self.section.push(prev + type_history.len() as i32);
                    }
                }
            }

            if let Some((border, max)) = state.gauge_border_max() {
                self.border = border;
                self.max = max;
            }
        }
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if self.gaugehistory.is_empty() {
            return;
        }

        let region_x = self.data.region.x;
        let region_y = self.data.region.y;
        let region_width = self.data.region.width;
        let region_height = self.data.region.height;

        // Check if texture needs recreation
        let needs_dispose = if let Some(ref shapetex) = self.shapetex {
            if !self.redraw {
                if let Some(tex) = shapetex.texture.as_ref() {
                    tex.width != region_width as i32 || tex.height != region_height as i32
                } else {
                    false
                }
            } else {
                true
            }
        } else {
            false
        };
        if needs_dispose {
            self.dispose_textures();
        }

        if self.shapetex.is_none() {
            // Guard against zero/negative dimensions from deserialized skin data.
            if region_width <= 0.0 || region_height <= 0.0 {
                return;
            }
            self.redraw = false;
            let width = region_width as i32;
            let height = region_height as i32;

            let type_idx = self.current_type as usize;
            self.color = if type_idx < TYPE_TABLE.len() {
                TYPE_TABLE[type_idx]
            } else {
                0
            };

            // Create background pixmap
            let mut shape = Pixmap::new(width, height, PixmapFormat::RGBA8888);
            shape.set_color(&self.graph_color[self.color]);
            shape.fill();

            let border = self.border.clamp(0.0, self.max);
            let max = self.max;
            if max > 0.0 {
                shape.set_color(&self.border_color[self.color]);
                shape.fill_rectangle(
                    0,
                    (height as f32 * border / max) as i32,
                    width,
                    (height as f32 * (max - border) / max) as i32,
                );
            }

            self.backtex = Some(TextureRegion::from_texture(Texture::from_pixmap(&shape)));
            shape.dispose();

            // Create graph pixmap
            let mut shape = Pixmap::new(width, height, PixmapFormat::RGBA8888);
            let mut f1: Option<f32> = None;
            let mut last_gauge: f32 = -1.0;
            let mut last_x: i32 = -1;
            let mut last_y: i32 = -1;
            let line_width = self.line_width;

            let gauge_len = self.gaugehistory.len() as f32;
            if gauge_len > 0.0 && max > 0.0 {
                for (i, &f2) in self.gaugehistory.iter().enumerate() {
                    // Section boundaries drawn at x = (i-1)/gauge_len. First entry is always
                    // > 0 (populated from cumulative type_history lengths), so i-1 >= 0 in practice.
                    if self.section.contains(&(i as i32)) {
                        shape.set_color(&Color::value_of("ffffff"));
                        let boundary_x =
                            (width as f32 * (i as f32 - 1.0).max(0.0) / gauge_len) as i32;
                        shape.draw_line(boundary_x, 0, boundary_x, height);
                    }
                    if let Some(f1_val) = f1 {
                        let x1 = (width as f32 * (i as f32 - 1.0).max(0.0) / gauge_len) as i32;
                        let y1 = ((f1_val / max) * (height - line_width) as f32) as i32;
                        let x2 = (width as f32 * i as f32 / gauge_len) as i32;
                        let y2 = ((f2 / max) * (height - line_width) as f32) as i32;
                        let yb = ((border / max) * (height - line_width) as f32) as i32;
                        last_gauge = f2;
                        last_x = x2;
                        last_y = y2;
                        if f1_val < border {
                            if f2 < border {
                                shape.set_color(&self.graph_line[self.color]);
                                shape.fill_rectangle(
                                    x1,
                                    y1.min(y2),
                                    line_width,
                                    (y2 - y1).abs() + line_width,
                                );
                                shape.fill_rectangle(x1, y2, x2 - x1, line_width);
                            } else {
                                shape.set_color(&self.graph_line[self.color]);
                                shape.fill_rectangle(x1, y1, line_width, yb - y1);
                                shape.set_color(&self.border_line[self.color]);
                                shape.fill_rectangle(x1, yb, line_width, y2 - yb + line_width);
                                shape.fill_rectangle(x1, y2, x2 - x1, line_width);
                            }
                        } else if f2 >= border {
                            shape.set_color(&self.border_line[self.color]);
                            shape.fill_rectangle(
                                x1,
                                y1.min(y2),
                                line_width,
                                (y2 - y1).abs() + line_width,
                            );
                            shape.fill_rectangle(x1, y2, x2 - x1, line_width);
                        } else {
                            shape.set_color(&self.border_line[self.color]);
                            shape.fill_rectangle(x1, yb, line_width, y1 - yb + line_width);
                            shape.set_color(&self.graph_line[self.color]);
                            shape.fill_rectangle(x1, y2, line_width, yb - y2);
                            shape.fill_rectangle(x1, y2, x2 - x1, line_width);
                        }
                    }
                    f1 = Some(f2);
                }

                if last_gauge != -1.0 {
                    if last_gauge < border {
                        shape.set_color(&self.graph_line[self.color]);
                    } else {
                        shape.set_color(&self.border_line[self.color]);
                    }
                    shape.fill_rectangle(last_x, last_y, width - last_x, line_width);
                }
            }

            self.shapetex = Some(TextureRegion::from_texture(Texture::from_pixmap(&shape)));
            shape.dispose();
        }

        // Draw background
        if let Some(ref backtex) = self.backtex {
            sprite.draw(
                backtex,
                region_x,
                region_y + region_height,
                region_width,
                -region_height,
            );
        }
        // Draw graph with render progress
        if let Some(ref mut shapetex) = self.shapetex {
            shapetex.set_region_from(
                0,
                0,
                (region_width * self.render) as i32,
                region_height as i32,
            );
            sprite.draw(
                shapetex,
                region_x,
                region_y + region_height,
                region_width * self.render,
                -region_height,
            );
        }
    }

    fn dispose_textures(&mut self) {
        if let Some(ref mut tex) = self.shapetex
            && let Some(t) = tex.texture.as_mut()
        {
            t.dispose();
        }
        self.shapetex = None;
        if let Some(ref mut tex) = self.backtex
            && let Some(t) = tex.texture.as_mut()
        {
            t.dispose();
        }
        self.backtex = None;
    }

    pub fn dispose(&mut self) {
        self.dispose_textures();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::SkinOffset;

    /// Mock MainState that returns a configurable result_gauge_type.
    struct MockGaugeState {
        timer: crate::reexports::Timer,
        gauge_type: i32,
        gauge_history: Option<Vec<Vec<f32>>>,
    }

    impl MockGaugeState {
        fn new(gauge_type: i32) -> Self {
            Self {
                timer: crate::reexports::Timer::default(),
                gauge_type,
                gauge_history: None,
            }
        }

        fn with_gauge_history(mut self, history: Vec<Vec<f32>>) -> Self {
            self.gauge_history = Some(history);
            self
        }
    }

    impl rubato_types::timer_access::TimerAccess for MockGaugeState {
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

    impl rubato_types::skin_render_context::SkinRenderContext for MockGaugeState {
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
        fn result_gauge_type(&self) -> i32 {
            self.gauge_type
        }
        fn gauge_history(&self) -> Option<&Vec<Vec<f32>>> {
            self.gauge_history.as_ref()
        }
    }

    impl crate::reexports::MainState for MockGaugeState {}

    #[test]
    fn test_new_default() {
        let obj = SkinGaugeGraphObject::new_default();
        assert_eq!(obj.delay(), 1500);
        assert_eq!(obj.line_width(), 2);
    }

    #[test]
    fn test_new_from_color_strings() {
        let obj = SkinGaugeGraphObject::new_from_color_strings(&GaugeGraphColorStrings {
            assist_clear_bg: "440044",
            assist_easy_fail_bg: "004444",
            groove_fail_bg: "004400",
            groove_clear_hard_bg: "440000",
            ex_hard_bg: "444400",
            hazard_bg: "444444",
            assist_clear_line: "ff00ff",
            assist_easy_fail_line: "00ffff",
            groove_fail_line: "00ff00",
            groove_clear_hard_line: "ff0000",
            ex_hard_line: "ffff00",
            hazard_line: "cccccc",
            borderline_color: "ff0000",
            border_color: "440000",
        });
        assert_eq!(obj.delay(), 1500);
        assert_eq!(obj.line_width(), 2);
    }

    #[test]
    fn test_set_delay_and_line_width() {
        let mut obj = SkinGaugeGraphObject::new_default();
        obj.delay = 2000;
        obj.line_width = 3;
        assert_eq!(obj.delay(), 2000);
        assert_eq!(obj.line_width(), 3);
    }

    #[test]
    fn test_border_exceeding_max_is_clamped() {
        // Regression: when border > max, fill_rectangle received out-of-bounds
        // y-coordinate (y > height) and negative height. Clamping border to
        // [0, max] prevents this.
        use crate::reexports::Rectangle;

        let mut obj = SkinGaugeGraphObject::new_default();
        // Set border > max to trigger the bug
        obj.border = 120.0;
        obj.max = 100.0;
        obj.current_type = 0;
        obj.gaugehistory = vec![50.0, 60.0, 70.0];
        obj.data.region = Rectangle::new(0.0, 0.0, 200.0, 100.0);
        obj.data.draw = true;

        let mut renderer = SkinObjectRenderer::new();
        // Before the fix, this would produce a fill_rectangle with y > height
        // and negative height for the border region. With the fix, border is
        // clamped to max (100.0), so y = height and rect height = 0.
        obj.draw(&mut renderer);

        // Verify that draw completed without panic and that shapetex was created
        assert!(obj.shapetex.is_some());
        assert!(obj.backtex.is_some());

        // Also verify negative border is clamped to 0
        obj.dispose();
        obj.border = -50.0;
        obj.max = 100.0;
        obj.draw(&mut renderer);
        assert!(obj.shapetex.is_some());
        assert!(obj.backtex.is_some());
    }

    #[test]
    fn test_prepare_negative_current_type_no_wrap() {
        // Regression: when result_gauge_type() returns -1,
        // `current_type as usize` wrapped to usize::MAX causing out-of-bounds
        // indexing into gauge_history. The fix adds an early return guard for
        // negative current_type.
        let mut obj = SkinGaugeGraphObject::new_default();
        // Set to a valid type first so the transition to -1 actually triggers.
        obj.current_type = 0;
        // Pre-populate gaugehistory to verify it gets cleared.
        obj.gaugehistory = vec![10.0, 20.0, 30.0];

        let state = MockGaugeState::new(-1);
        obj.prepare(0, &state);

        assert_eq!(obj.current_type, -1);
        assert!(
            obj.gaugehistory.is_empty(),
            "gaugehistory should be cleared for negative type"
        );
        assert!(obj.redraw, "redraw should be set when current_type changes");
    }

    #[test]
    fn test_prepare_valid_type_indexes_gauge_history() {
        // Verify that a valid (non-negative) current_type properly indexes
        // into gauge_history.
        let mut obj = SkinGaugeGraphObject::new_default();
        let history = vec![
            vec![1.0, 2.0, 3.0], // type 0
            vec![4.0, 5.0, 6.0], // type 1
        ];
        let state = MockGaugeState::new(1).with_gauge_history(history);
        obj.prepare(0, &state);

        assert_eq!(obj.current_type, 1);
        assert_eq!(obj.gaugehistory, vec![4.0, 5.0, 6.0]);
    }
}

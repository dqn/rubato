// SkinGaugeGraphObject.java -> skin_gauge_graph_object.rs
// Mechanical line-by-line translation.
// Gauge transition graph object (result screen).

use crate::json::json_skin_object_loader::parse_hex_color;
use crate::stubs::MainState;
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};
use rubato_render::color::Color;

/// Type-to-color-index mapping table (Java: typetable)
const _TYPE_TABLE: [usize; 10] = [0, 1, 2, 3, 4, 5, 3, 4, 5, 3];

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
    _graph_color: [Color; 6],
    /// Graph line colors per gauge type (below border)
    _graph_line: [Color; 6],
    /// Background colors per gauge type (above border)
    _border_color: [Color; 6],
    /// Graph line colors per gauge type (above border)
    _border_line: [Color; 6],
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
            _graph_color: graph_color,
            _graph_line: graph_line,
            _border_color: border_color,
            _border_line: border_line,
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
            _graph_color: graph_color,
            _graph_line: graph_line,
            _border_color: border_color,
            _border_line: border_line,
        }
    }

    /// Creates a SkinGaugeGraphObject from JSON color strings.
    ///
    /// Corresponds to Java constructor with 14 string parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn new_from_color_strings(
        assist_clear_bg: &str,
        assist_easy_fail_bg: &str,
        groove_fail_bg: &str,
        groove_clear_hard_bg: &str,
        ex_hard_bg: &str,
        hazard_bg: &str,
        assist_clear_line: &str,
        assist_easy_fail_line: &str,
        groove_fail_line: &str,
        groove_clear_hard_line: &str,
        ex_hard_line: &str,
        hazard_line: &str,
        borderline_color: &str,
        border_color_str: &str,
    ) -> Self {
        let fallback = Color::BLACK;

        let mut graph_color = [Color::BLACK; 6];
        let mut graph_line = [Color::BLACK; 6];
        let mut border_color = [Color::BLACK; 6];
        let mut border_line = [Color::BLACK; 6];

        // Below-border background colors
        graph_color[0] = parse_hex_color(assist_clear_bg, fallback);
        graph_color[1] = parse_hex_color(assist_easy_fail_bg, fallback);
        graph_color[2] = parse_hex_color(groove_fail_bg, fallback);

        // Above-border background colors
        border_color[3] = parse_hex_color(groove_clear_hard_bg, fallback);
        border_color[4] = parse_hex_color(ex_hard_bg, fallback);
        border_color[5] = parse_hex_color(hazard_bg, fallback);

        // Below-border line colors
        graph_line[0] = parse_hex_color(assist_clear_line, fallback);
        graph_line[1] = parse_hex_color(assist_easy_fail_line, fallback);
        graph_line[2] = parse_hex_color(groove_fail_line, fallback);

        // Above-border line colors
        border_line[3] = parse_hex_color(groove_clear_hard_line, fallback);
        border_line[4] = parse_hex_color(ex_hard_line, fallback);
        border_line[5] = parse_hex_color(hazard_line, fallback);

        // Shared border colors for types 0-2
        let bl = parse_hex_color(borderline_color, fallback);
        let bc = parse_hex_color(border_color_str, fallback);
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
            _graph_color: graph_color,
            _graph_line: graph_line,
            _border_color: border_color,
            _border_line: border_line,
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
        // Gauge history rendering is deferred to wgpu pixel drawing pipeline.
        // The prepare step in Java reads gauge history from PlayerResource,
        // which is not yet accessible from the skin layer.
    }

    pub fn draw(&mut self, _sprite: &mut SkinObjectRenderer) {
        // Gauge graph drawing requires pixel-level Pixmap operations (Java: Pixmap.fillRectangle).
        // In Rust, this will be implemented via wgpu compute/render pass when the
        // rendering pipeline is ready. For now, this is a no-op.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default() {
        let obj = SkinGaugeGraphObject::new_default();
        assert_eq!(obj.delay(), 1500);
        assert_eq!(obj.line_width(), 2);
    }

    #[test]
    fn test_new_from_color_strings() {
        let obj = SkinGaugeGraphObject::new_from_color_strings(
            "440044", "004444", "004400", "440000", "444400", "444444", "ff00ff", "00ffff",
            "00ff00", "ff0000", "ffff00", "cccccc", "ff0000", "440000",
        );
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
}

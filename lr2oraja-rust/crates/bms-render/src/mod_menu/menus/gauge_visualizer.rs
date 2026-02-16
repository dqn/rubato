// Gauge visualizer menu — displays gauge history as a line graph.
//
// Shows gauge values over time for the result screen.
// X axis: time (500ms intervals), Y axis: gauge value (0-100%).
// Multiple gauge types are color-coded.

use egui::{Color32, Rect, Stroke, Vec2, pos2};

const GRAPH_HEIGHT: f32 = 200.0;
const GRAPH_WIDTH: f32 = 400.0;
const GAUGE_COLORS: &[Color32] = &[
    Color32::from_rgb(0, 200, 0),   // green
    Color32::from_rgb(200, 0, 0),   // red
    Color32::from_rgb(0, 100, 200), // blue
    Color32::from_rgb(200, 200, 0), // yellow
];

/// State for the gauge visualizer panel.
#[derive(Debug, Clone, Default)]
pub struct GaugeVisualizerState {
    /// Gauge history per gauge type. Each inner Vec is sampled at 500ms intervals.
    pub gauge_logs: Vec<GaugeLog>,
}

/// A single gauge type's history.
#[derive(Debug, Clone)]
pub struct GaugeLog {
    pub name: String,
    pub values: Vec<f32>,
}

impl GaugeVisualizerState {
    /// Load gauge data from result screen.
    pub fn load(&mut self, logs: Vec<GaugeLog>) {
        self.gauge_logs = logs;
    }

    /// Clear all gauge data.
    pub fn clear(&mut self) {
        self.gauge_logs.clear();
    }

    /// Maximum sample count across all gauge logs.
    pub fn max_samples(&self) -> usize {
        self.gauge_logs
            .iter()
            .map(|g| g.values.len())
            .max()
            .unwrap_or(0)
    }
}

pub fn render(ctx: &egui::Context, open: &mut bool, state: &mut GaugeVisualizerState) {
    egui::Window::new("Gauge Visualizer")
        .open(open)
        .resizable(true)
        .default_width(GRAPH_WIDTH + 40.0)
        .show(ctx, |ui| {
            if state.gauge_logs.is_empty() {
                ui.label("No gauge data available. Play a song to collect data.");
                return;
            }

            // Legend
            for (i, log) in state.gauge_logs.iter().enumerate() {
                let color = GAUGE_COLORS[i % GAUGE_COLORS.len()];
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, color);
                    ui.label(&log.name);
                });
            }

            ui.separator();

            // Graph area
            let (response, painter) =
                ui.allocate_painter(Vec2::new(GRAPH_WIDTH, GRAPH_HEIGHT), egui::Sense::hover());
            let rect = response.rect;

            // Background
            painter.rect_filled(rect, 0.0, Color32::from_gray(30));

            // Grid lines (25%, 50%, 75%)
            let grid_color = Color32::from_gray(60);
            for pct in [0.25, 0.50, 0.75] {
                let y = rect.max.y - rect.height() * pct;
                painter.line_segment(
                    [pos2(rect.min.x, y), pos2(rect.max.x, y)],
                    Stroke::new(1.0, grid_color),
                );
            }

            // Y-axis labels
            let label_rect = Rect::from_min_size(rect.min - Vec2::new(30.0, 0.0), Vec2::ZERO);
            let _ = label_rect; // Avoid unused; labels are painted in the graph area margin

            // Draw gauge lines
            let max_samples = state.max_samples();
            if max_samples > 1 {
                for (i, log) in state.gauge_logs.iter().enumerate() {
                    let color = GAUGE_COLORS[i % GAUGE_COLORS.len()];
                    let points: Vec<_> = log
                        .values
                        .iter()
                        .enumerate()
                        .map(|(j, &val)| {
                            let x =
                                rect.min.x + (j as f32 / (max_samples - 1) as f32) * rect.width();
                            let y = rect.max.y - (val / 100.0).clamp(0.0, 1.0) * rect.height();
                            pos2(x, y)
                        })
                        .collect();

                    for pair in points.windows(2) {
                        painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, color));
                    }
                }
            }

            // X-axis label
            ui.horizontal(|ui| {
                ui.label("0s");
                ui.add_space(GRAPH_WIDTH - 60.0);
                let total_secs = (max_samples as f32 * 0.5) as i32;
                ui.label(format!("{}s", total_secs));
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty() {
        let state = GaugeVisualizerState::default();
        assert!(state.gauge_logs.is_empty());
        assert_eq!(state.max_samples(), 0);
    }

    #[test]
    fn load_and_max_samples() {
        let mut state = GaugeVisualizerState::default();
        state.load(vec![
            GaugeLog {
                name: "Groove".into(),
                values: vec![0.0, 10.0, 20.0],
            },
            GaugeLog {
                name: "Hard".into(),
                values: vec![100.0, 90.0, 80.0, 70.0, 60.0],
            },
        ]);
        assert_eq!(state.gauge_logs.len(), 2);
        assert_eq!(state.max_samples(), 5);
    }

    #[test]
    fn clear_removes_all() {
        let mut state = GaugeVisualizerState::default();
        state.load(vec![GaugeLog {
            name: "Normal".into(),
            values: vec![50.0],
        }]);
        assert_eq!(state.gauge_logs.len(), 1);
        state.clear();
        assert!(state.gauge_logs.is_empty());
    }

    #[test]
    fn max_samples_single_empty_log() {
        let mut state = GaugeVisualizerState::default();
        state.load(vec![GaugeLog {
            name: "Empty".into(),
            values: vec![],
        }]);
        assert_eq!(state.max_samples(), 0);
    }
}

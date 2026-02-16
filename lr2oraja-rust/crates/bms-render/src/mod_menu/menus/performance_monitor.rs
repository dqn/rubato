// PerformanceMonitor menu — displays performance statistics and event tree.
//
// Corresponds to Java `PerformanceMonitor.java`.
// Watch statistics show average/standard deviation of named timing measurements.
// Event tree shows hierarchical performance events with duration and thread info.
// Frame time tracking shows min/max/avg over recent frames with a simple bar graph.
// Actual measurement system integration is deferred to a future phase;
// data is injected via `load_events()`, `load_watch_data()`, and `record_frame_time()`.

use std::collections::HashMap;

use egui::{Color32, Stroke, Vec2, pos2};

const FRAME_TIME_HISTORY_LEN: usize = 60;

#[derive(Debug, Clone)]
pub struct EventResult {
    pub name: String,
    pub id: i32,
    pub parent: i32,
    pub duration_ns: i64,
    pub thread: String,
}

#[derive(Debug, Clone, Default)]
pub struct WatchStats {
    pub avg_us: f32,
    pub std_us: f32,
}

impl WatchStats {
    pub fn from_durations_ns(durations: &[i64]) -> Self {
        if durations.is_empty() {
            return Self::default();
        }

        let count = durations.len() as f64;
        let sum: f64 = durations.iter().map(|&d| d as f64).sum();
        let avg = sum / count;

        let variance = if durations.len() > 1 {
            durations
                .iter()
                .map(|&d| (d as f64 - avg).powi(2))
                .sum::<f64>()
                / count
        } else {
            0.0
        };

        // Convert from nanoseconds to microseconds
        Self {
            avg_us: (avg / 1000.0) as f32,
            std_us: (variance.sqrt() / 1000.0) as f32,
        }
    }
}

/// Frame time statistics computed from the rolling history.
#[derive(Debug, Clone, Default)]
pub struct FrameTimeStats {
    pub min_ms: f32,
    pub max_ms: f32,
    pub avg_ms: f32,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct PerformanceMonitorState {
    pub event_tree: HashMap<i32, Vec<EventResult>>,
    pub watch_data: Vec<(String, WatchStats)>,
    pub filter_threshold_ms: f32,
    /// Rolling frame time history (milliseconds), most recent at the end.
    pub frame_times_ms: Vec<f32>,
}

impl Default for PerformanceMonitorState {
    fn default() -> Self {
        Self {
            event_tree: HashMap::new(),
            watch_data: Vec::new(),
            filter_threshold_ms: 1.0,
            frame_times_ms: Vec::with_capacity(FRAME_TIME_HISTORY_LEN),
        }
    }
}

impl PerformanceMonitorState {
    /// Load events and build the parent-to-children tree.
    pub fn load_events(&mut self, events: Vec<EventResult>) {
        self.event_tree.clear();
        for event in events {
            self.event_tree.entry(event.parent).or_default().push(event);
        }
    }

    /// Load watch statistics data.
    pub fn load_watch_data(&mut self, data: Vec<(String, WatchStats)>) {
        self.watch_data = data;
    }

    /// Record a frame time sample in milliseconds.
    ///
    /// Maintains a rolling buffer of the last [`FRAME_TIME_HISTORY_LEN`] samples.
    pub fn record_frame_time(&mut self, dt_ms: f32) {
        if self.frame_times_ms.len() >= FRAME_TIME_HISTORY_LEN {
            self.frame_times_ms.remove(0);
        }
        self.frame_times_ms.push(dt_ms);
    }

    /// Compute min/max/avg frame time from the rolling history.
    pub fn frame_time_stats(&self) -> FrameTimeStats {
        if self.frame_times_ms.is_empty() {
            return FrameTimeStats::default();
        }
        let count = self.frame_times_ms.len();
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        let mut sum = 0.0f32;
        for &t in &self.frame_times_ms {
            min = min.min(t);
            max = max.max(t);
            sum += t;
        }
        FrameTimeStats {
            min_ms: min,
            max_ms: max,
            avg_ms: sum / count as f32,
            count,
        }
    }
}

const GRAPH_WIDTH: f32 = 400.0;
const GRAPH_HEIGHT: f32 = 80.0;
const TARGET_60FPS_MS: f32 = 16.67;

pub fn render(ctx: &egui::Context, open: &mut bool, state: &mut PerformanceMonitorState) {
    egui::Window::new("Performance Monitor")
        .open(open)
        .resizable(true)
        .default_width(500.0)
        .show(ctx, |ui| {
            // Frame Time section
            egui::CollapsingHeader::new("Frame Time")
                .default_open(true)
                .show(ui, |ui| {
                    let stats = state.frame_time_stats();
                    if stats.count == 0 {
                        ui.label("No frame time data yet.");
                    } else {
                        ui.label(format!(
                            "FPS: {:.0}  |  avg: {:.2}ms  |  min: {:.2}ms  |  max: {:.2}ms  ({} frames)",
                            if stats.avg_ms > 0.0 { 1000.0 / stats.avg_ms } else { 0.0 },
                            stats.avg_ms,
                            stats.min_ms,
                            stats.max_ms,
                            stats.count,
                        ));

                        ui.separator();

                        // Simple bar graph of frame times
                        render_frame_time_graph(ui, &state.frame_times_ms);
                    }
                });

            // Watch section
            egui::CollapsingHeader::new("Watch")
                .default_open(false)
                .show(ui, |ui| {
                    if state.watch_data.is_empty() {
                        ui.label("No watch data available.");
                    } else {
                        for (name, stats) in &state.watch_data {
                            ui.label(format!(
                                "{}: avg = {:.1}us, std = {:.1}us",
                                name, stats.avg_us, stats.std_us,
                            ));
                        }
                    }
                });

            // Events section
            egui::CollapsingHeader::new("Events")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut state.filter_threshold_ms, 0.0..=4.0)
                            .text("Filter short events (ms)"),
                    );

                    ui.separator();

                    egui::Grid::new("perf_events_grid")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            // Header
                            ui.strong("Event");
                            ui.strong("Time");
                            ui.strong("Thread");
                            ui.end_row();

                            // Render root events (parent == -1)
                            render_event_children(ui, state, -1, 0);
                        });
                });
        });
}

/// Render a simple bar graph of frame times using egui painter.
fn render_frame_time_graph(ui: &mut egui::Ui, frame_times: &[f32]) {
    if frame_times.is_empty() {
        return;
    }

    let (response, painter) =
        ui.allocate_painter(Vec2::new(GRAPH_WIDTH, GRAPH_HEIGHT), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 0.0, Color32::from_gray(30));

    // 16.67ms target line (60 FPS)
    let max_display_ms = frame_times
        .iter()
        .copied()
        .fold(TARGET_60FPS_MS * 2.0, f32::max);
    let target_y = rect.max.y - (TARGET_60FPS_MS / max_display_ms) * rect.height();
    painter.line_segment(
        [pos2(rect.min.x, target_y), pos2(rect.max.x, target_y)],
        Stroke::new(1.0, Color32::from_rgb(100, 100, 100)),
    );

    // Bars
    let bar_count = frame_times.len();
    let bar_width = rect.width() / FRAME_TIME_HISTORY_LEN as f32;

    for (i, &dt) in frame_times.iter().enumerate() {
        let ratio = (dt / max_display_ms).clamp(0.0, 1.0);
        let bar_height = ratio * rect.height();
        let x = rect.min.x + i as f32 * bar_width;
        let y = rect.max.y - bar_height;

        // Color: green under 16ms, yellow under 33ms, red above
        let color = if dt <= TARGET_60FPS_MS {
            Color32::from_rgb(0, 200, 0)
        } else if dt <= TARGET_60FPS_MS * 2.0 {
            Color32::from_rgb(200, 200, 0)
        } else {
            Color32::from_rgb(200, 0, 0)
        };

        painter.rect_filled(
            egui::Rect::from_min_max(pos2(x, y), pos2(x + bar_width - 1.0, rect.max.y)),
            0.0,
            color,
        );
    }

    // Labels
    ui.horizontal(|ui| {
        ui.label(format!("{} frames", bar_count));
        ui.add_space(GRAPH_WIDTH - 200.0);
        ui.label(format!("(max scale: {:.1}ms)", max_display_ms));
    });
}

fn render_event_children(
    ui: &mut egui::Ui,
    state: &PerformanceMonitorState,
    parent_id: i32,
    depth: usize,
) {
    let Some(children) = state.event_tree.get(&parent_id) else {
        return;
    };

    let threshold_ns = (state.filter_threshold_ms * 1_000_000.0) as i64;

    for event in children {
        if event.duration_ns < threshold_ns {
            continue;
        }

        let has_children = state.event_tree.contains_key(&event.id);
        let duration_ms = event.duration_ns as f64 / 1_000_000.0;
        let indent = "  ".repeat(depth);

        if has_children {
            let id = ui.make_persistent_id(format!("perf_event_{}", event.id));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    ui.label(format!("{}{}", indent, event.name));
                })
                .body(|ui| {
                    // Time and thread are shown in the parent row via the grid,
                    // but collapsing header doesn't align well with Grid columns.
                    // Render children as a nested grid.
                    render_event_children(ui, state, event.id, depth + 1);
                });
            // Show time and thread in same row
            ui.label(format!("{:.3}ms", duration_ms));
            ui.label(&event.thread);
            ui.end_row();
        } else {
            ui.label(format!("{}{}", indent, event.name));
            ui.label(format!("{:.3}ms", duration_ms));
            ui.label(&event.thread);
            ui.end_row();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let state = PerformanceMonitorState::default();
        assert!(state.event_tree.is_empty());
        assert!(state.watch_data.is_empty());
        assert!((state.filter_threshold_ms - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn watch_stats_from_empty() {
        let stats = WatchStats::from_durations_ns(&[]);
        assert_eq!(stats.avg_us, 0.0);
        assert_eq!(stats.std_us, 0.0);
    }

    #[test]
    fn watch_stats_from_single() {
        let stats = WatchStats::from_durations_ns(&[2000]); // 2000ns = 2.0us
        assert!((stats.avg_us - 2.0).abs() < 0.01);
        assert_eq!(stats.std_us, 0.0);
    }

    #[test]
    fn watch_stats_from_multiple() {
        // 1000ns, 2000ns, 3000ns -> avg = 2000ns = 2.0us
        // variance = ((1000-2000)^2 + (2000-2000)^2 + (3000-2000)^2) / 3
        //          = (1_000_000 + 0 + 1_000_000) / 3 = 666_666.67
        // std = sqrt(666_666.67) = 816.5ns = 0.8165us
        let stats = WatchStats::from_durations_ns(&[1000, 2000, 3000]);
        assert!((stats.avg_us - 2.0).abs() < 0.01);
        assert!((stats.std_us - 0.8165).abs() < 0.01);
    }

    #[test]
    fn load_events_builds_tree() {
        let mut state = PerformanceMonitorState::default();
        state.load_events(vec![
            EventResult {
                name: "root".into(),
                id: 0,
                parent: -1,
                duration_ns: 10_000_000,
                thread: "main".into(),
            },
            EventResult {
                name: "child1".into(),
                id: 1,
                parent: 0,
                duration_ns: 5_000_000,
                thread: "main".into(),
            },
            EventResult {
                name: "child2".into(),
                id: 2,
                parent: 0,
                duration_ns: 3_000_000,
                thread: "worker".into(),
            },
        ]);

        // Root events (parent == -1)
        assert_eq!(state.event_tree[&-1].len(), 1);
        assert_eq!(state.event_tree[&-1][0].name, "root");

        // Children of root (parent == 0)
        assert_eq!(state.event_tree[&0].len(), 2);
        assert_eq!(state.event_tree[&0][0].name, "child1");
        assert_eq!(state.event_tree[&0][1].name, "child2");
    }

    #[test]
    fn load_watch_data() {
        let mut state = PerformanceMonitorState::default();
        state.load_watch_data(vec![
            (
                "render".into(),
                WatchStats {
                    avg_us: 16.0,
                    std_us: 2.0,
                },
            ),
            (
                "update".into(),
                WatchStats {
                    avg_us: 8.0,
                    std_us: 1.0,
                },
            ),
        ]);
        assert_eq!(state.watch_data.len(), 2);
        assert_eq!(state.watch_data[0].0, "render");
        assert!((state.watch_data[0].1.avg_us - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    fn frame_time_stats_empty() {
        let state = PerformanceMonitorState::default();
        let stats = state.frame_time_stats();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.avg_ms, 0.0);
    }

    #[test]
    fn record_and_stats() {
        let mut state = PerformanceMonitorState::default();
        state.record_frame_time(10.0);
        state.record_frame_time(20.0);
        state.record_frame_time(30.0);

        let stats = state.frame_time_stats();
        assert_eq!(stats.count, 3);
        assert!((stats.min_ms - 10.0).abs() < f32::EPSILON);
        assert!((stats.max_ms - 30.0).abs() < f32::EPSILON);
        assert!((stats.avg_ms - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn frame_time_rolling_buffer() {
        let mut state = PerformanceMonitorState::default();
        // Fill beyond capacity
        for i in 0..(FRAME_TIME_HISTORY_LEN + 10) {
            state.record_frame_time(i as f32);
        }
        assert_eq!(state.frame_times_ms.len(), FRAME_TIME_HISTORY_LEN);
        // Oldest should be index 10 (first 10 evicted)
        assert!((state.frame_times_ms[0] - 10.0).abs() < f32::EPSILON);
    }
}

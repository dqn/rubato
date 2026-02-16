// Profiler menu — FPS, frame time, and CPU usage display.
//
// Extends the existing PerformanceMonitor with real-time metrics.
// Data is injected from the game loop via `update_frame_time()`.

const HISTORY_SIZE: usize = 120;

/// State for the profiler panel.
#[derive(Debug, Clone)]
pub struct ProfilerState {
    /// Frame time history in milliseconds (most recent last).
    pub frame_times_ms: Vec<f32>,
    /// Current FPS (computed from frame times).
    pub fps: f32,
    /// Average frame time (ms).
    pub avg_frame_time_ms: f32,
    /// Min frame time (ms) in the history window.
    pub min_frame_time_ms: f32,
    /// Max frame time (ms) in the history window.
    pub max_frame_time_ms: f32,
}

impl Default for ProfilerState {
    fn default() -> Self {
        Self {
            frame_times_ms: Vec::with_capacity(HISTORY_SIZE),
            fps: 0.0,
            avg_frame_time_ms: 0.0,
            min_frame_time_ms: 0.0,
            max_frame_time_ms: 0.0,
        }
    }
}

impl ProfilerState {
    /// Record a new frame time and update computed statistics.
    pub fn update_frame_time(&mut self, dt_ms: f32) {
        if self.frame_times_ms.len() >= HISTORY_SIZE {
            self.frame_times_ms.remove(0);
        }
        self.frame_times_ms.push(dt_ms);

        self.recompute();
    }

    fn recompute(&mut self) {
        if self.frame_times_ms.is_empty() {
            self.fps = 0.0;
            self.avg_frame_time_ms = 0.0;
            self.min_frame_time_ms = 0.0;
            self.max_frame_time_ms = 0.0;
            return;
        }

        let sum: f32 = self.frame_times_ms.iter().sum();
        let count = self.frame_times_ms.len() as f32;
        self.avg_frame_time_ms = sum / count;
        self.fps = if self.avg_frame_time_ms > 0.0 {
            1000.0 / self.avg_frame_time_ms
        } else {
            0.0
        };
        self.min_frame_time_ms = self
            .frame_times_ms
            .iter()
            .copied()
            .reduce(f32::min)
            .unwrap_or(0.0);
        self.max_frame_time_ms = self
            .frame_times_ms
            .iter()
            .copied()
            .reduce(f32::max)
            .unwrap_or(0.0);
    }
}

pub fn render(ctx: &egui::Context, open: &mut bool, state: &mut ProfilerState) {
    egui::Window::new("Profiler")
        .open(open)
        .resizable(true)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Frame Statistics");
            ui.separator();

            egui::Grid::new("profiler_stats_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.strong("FPS:");
                    ui.label(format!("{:.1}", state.fps));
                    ui.end_row();

                    ui.strong("Avg frame:");
                    ui.label(format!("{:.2} ms", state.avg_frame_time_ms));
                    ui.end_row();

                    ui.strong("Min frame:");
                    ui.label(format!("{:.2} ms", state.min_frame_time_ms));
                    ui.end_row();

                    ui.strong("Max frame:");
                    ui.label(format!("{:.2} ms", state.max_frame_time_ms));
                    ui.end_row();

                    ui.strong("Samples:");
                    ui.label(format!("{} / {}", state.frame_times_ms.len(), HISTORY_SIZE));
                    ui.end_row();
                });

            // Simple frame time bar chart
            if !state.frame_times_ms.is_empty() {
                ui.separator();
                ui.label("Frame time history:");

                let available_width = ui.available_width();
                let bar_width = (available_width / HISTORY_SIZE as f32).max(1.0);
                let chart_height = 60.0;

                let (response, painter) = ui.allocate_painter(
                    egui::Vec2::new(available_width, chart_height),
                    egui::Sense::hover(),
                );
                let rect = response.rect;

                // Background
                painter.rect_filled(rect, 0.0, egui::Color32::from_gray(30));

                // 16.67ms line (60fps target)
                let target_y = rect.max.y - (16.67 / 33.33) * rect.height();
                painter.line_segment(
                    [
                        egui::pos2(rect.min.x, target_y),
                        egui::pos2(rect.max.x, target_y),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 0)),
                );

                // Bars
                for (i, &dt) in state.frame_times_ms.iter().enumerate() {
                    let normalized = (dt / 33.33).clamp(0.0, 1.0);
                    let x = rect.min.x + i as f32 * bar_width;
                    let bar_h = normalized * rect.height();
                    let color = if dt > 16.67 {
                        egui::Color32::from_rgb(200, 80, 80)
                    } else {
                        egui::Color32::from_rgb(80, 200, 80)
                    };
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(x, rect.max.y - bar_h),
                        egui::Vec2::new(bar_width.max(1.0), bar_h),
                    );
                    painter.rect_filled(bar_rect, 0.0, color);
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let state = ProfilerState::default();
        assert_eq!(state.fps, 0.0);
        assert!(state.frame_times_ms.is_empty());
    }

    #[test]
    fn update_frame_time_single() {
        let mut state = ProfilerState::default();
        state.update_frame_time(16.67);
        assert_eq!(state.frame_times_ms.len(), 1);
        assert!((state.fps - 59.988).abs() < 0.1);
        assert!((state.avg_frame_time_ms - 16.67).abs() < 0.01);
    }

    #[test]
    fn update_frame_time_multiple() {
        let mut state = ProfilerState::default();
        state.update_frame_time(10.0);
        state.update_frame_time(20.0);
        assert_eq!(state.frame_times_ms.len(), 2);
        assert!((state.avg_frame_time_ms - 15.0).abs() < 0.01);
        assert!((state.min_frame_time_ms - 10.0).abs() < 0.01);
        assert!((state.max_frame_time_ms - 20.0).abs() < 0.01);
    }

    #[test]
    fn history_size_limit() {
        let mut state = ProfilerState::default();
        for i in 0..HISTORY_SIZE + 10 {
            state.update_frame_time(i as f32);
        }
        assert_eq!(state.frame_times_ms.len(), HISTORY_SIZE);
        // Oldest should be 10 (first 10 evicted)
        assert!((state.frame_times_ms[0] - 10.0).abs() < 0.01);
    }
}

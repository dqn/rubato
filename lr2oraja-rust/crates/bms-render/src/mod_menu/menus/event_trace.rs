// Event trace menu — real-time event log with category filtering.
//
// Displays recent events in a scrollable list with timestamp,
// category, and message. Uses a ring buffer (VecDeque) for memory efficiency.

use std::collections::VecDeque;

const MAX_EVENTS: usize = 1000;

/// State for the event trace panel.
#[derive(Debug, Clone)]
pub struct EventTraceState {
    /// Ring buffer of recent events.
    pub events: VecDeque<TraceEvent>,
    /// Category filter (empty = show all).
    pub filter: String,
    /// Auto-scroll to bottom.
    pub auto_scroll: bool,
}

impl Default for EventTraceState {
    fn default() -> Self {
        Self {
            events: VecDeque::with_capacity(MAX_EVENTS),
            filter: String::new(),
            auto_scroll: true,
        }
    }
}

/// A single trace event.
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub timestamp_us: i64,
    pub category: String,
    pub message: String,
}

impl EventTraceState {
    /// Push a new event into the ring buffer.
    pub fn push(&mut self, event: TraceEvent) {
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Number of events currently stored.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Iterate events matching the current filter.
    pub fn filtered_events(&self) -> impl Iterator<Item = &TraceEvent> {
        let filter = self.filter.clone();
        self.events
            .iter()
            .filter(move |e| filter.is_empty() || e.category.contains(&filter))
    }
}

/// Format microseconds to a compact timestamp string.
fn format_timestamp(us: i64) -> String {
    let ms = us / 1000;
    let secs = ms / 1000;
    let mins = secs / 60;
    format!("{:02}:{:02}.{:03}", mins, secs % 60, ms % 1000)
}

pub fn render(ctx: &egui::Context, open: &mut bool, state: &mut EventTraceState) {
    egui::Window::new("Event Trace")
        .open(open)
        .resizable(true)
        .default_width(500.0)
        .default_height(300.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut state.filter);
                if ui.button("Clear").clicked() {
                    state.clear();
                }
                ui.checkbox(&mut state.auto_scroll, "Auto-scroll");
            });

            ui.separator();

            let filtered: Vec<&TraceEvent> = state.filtered_events().collect();
            ui.label(format!(
                "{} / {} events",
                filtered.len(),
                state.events.len()
            ));

            ui.separator();

            let scroll_area = egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(state.auto_scroll);

            scroll_area.show(ui, |ui| {
                egui::Grid::new("event_trace_grid")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Time");
                        ui.strong("Category");
                        ui.strong("Message");
                        ui.end_row();

                        for event in &filtered {
                            ui.label(format_timestamp(event.timestamp_us));
                            ui.label(&event.category);
                            ui.label(&event.message);
                            ui.end_row();
                        }
                    });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty() {
        let state = EventTraceState::default();
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
        assert!(state.filter.is_empty());
        assert!(state.auto_scroll);
    }

    #[test]
    fn push_and_len() {
        let mut state = EventTraceState::default();
        state.push(TraceEvent {
            timestamp_us: 1_000_000,
            category: "input".into(),
            message: "key pressed".into(),
        });
        assert_eq!(state.len(), 1);
        assert!(!state.is_empty());
    }

    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut state = EventTraceState::default();
        for i in 0..MAX_EVENTS + 10 {
            state.push(TraceEvent {
                timestamp_us: i as i64 * 1000,
                category: "test".into(),
                message: format!("event {}", i),
            });
        }
        assert_eq!(state.len(), MAX_EVENTS);
        // Oldest should be event 10 (first 10 evicted)
        assert_eq!(state.events.front().unwrap().message, "event 10");
    }

    #[test]
    fn clear() {
        let mut state = EventTraceState::default();
        state.push(TraceEvent {
            timestamp_us: 0,
            category: "test".into(),
            message: "msg".into(),
        });
        state.clear();
        assert!(state.is_empty());
    }

    #[test]
    fn filtered_events_no_filter() {
        let mut state = EventTraceState::default();
        state.push(TraceEvent {
            timestamp_us: 0,
            category: "input".into(),
            message: "a".into(),
        });
        state.push(TraceEvent {
            timestamp_us: 1000,
            category: "audio".into(),
            message: "b".into(),
        });
        assert_eq!(state.filtered_events().count(), 2);
    }

    #[test]
    fn filtered_events_with_filter() {
        let mut state = EventTraceState::default();
        state.push(TraceEvent {
            timestamp_us: 0,
            category: "input".into(),
            message: "a".into(),
        });
        state.push(TraceEvent {
            timestamp_us: 1000,
            category: "audio".into(),
            message: "b".into(),
        });
        state.push(TraceEvent {
            timestamp_us: 2000,
            category: "input".into(),
            message: "c".into(),
        });
        state.filter = "input".to_string();
        assert_eq!(state.filtered_events().count(), 2);
    }

    #[test]
    fn format_timestamp_zero() {
        assert_eq!(format_timestamp(0), "00:00.000");
    }

    #[test]
    fn format_timestamp_one_minute() {
        assert_eq!(format_timestamp(60_000_000), "01:00.000");
    }

    #[test]
    fn format_timestamp_mixed() {
        // 1 min 23.456s = 83_456_000 us
        assert_eq!(format_timestamp(83_456_000), "01:23.456");
    }
}

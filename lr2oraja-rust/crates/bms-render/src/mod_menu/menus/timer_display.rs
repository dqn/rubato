// Timer display menu — shows active timers from TimerManager.
//
// Displays a table of all active timers with their ID, current value, and state.
// Data is injected from the game loop via `load_timers()`.

/// State for the timer display panel.
#[derive(Debug, Clone, Default)]
pub struct TimerDisplayState {
    /// Active timers snapshot.
    pub timers: Vec<TimerInfo>,
}

/// Snapshot of a single timer's state.
#[derive(Debug, Clone)]
pub struct TimerInfo {
    pub id: i32,
    pub name: String,
    pub value_us: i64,
    pub state: TimerState,
}

/// Timer running state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Running,
    Paused,
    Stopped,
}

impl std::fmt::Display for TimerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimerState::Running => write!(f, "Running"),
            TimerState::Paused => write!(f, "Paused"),
            TimerState::Stopped => write!(f, "Stopped"),
        }
    }
}

impl TimerDisplayState {
    /// Load timer snapshot from game loop.
    pub fn load_timers(&mut self, timers: Vec<TimerInfo>) {
        self.timers = timers;
    }
}

/// Format microseconds to a human-readable string.
pub fn format_timer_value(us: i64) -> String {
    if us < 0 {
        return format!("-{}", format_timer_value(-us));
    }
    let ms = us / 1000;
    let secs = ms / 1000;
    let mins = secs / 60;
    if mins > 0 {
        format!("{}:{:02}.{:03}", mins, secs % 60, ms % 1000)
    } else {
        format!("{}.{:03}s", secs, ms % 1000)
    }
}

pub fn render(ctx: &egui::Context, open: &mut bool, state: &mut TimerDisplayState) {
    egui::Window::new("Timer Display")
        .open(open)
        .resizable(true)
        .default_width(400.0)
        .show(ctx, |ui| {
            if state.timers.is_empty() {
                ui.label("No active timers.");
                return;
            }

            ui.label(format!("{} active timers", state.timers.len()));
            ui.separator();

            egui::Grid::new("timer_display_grid")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("ID");
                    ui.strong("Name");
                    ui.strong("Value");
                    ui.strong("State");
                    ui.end_row();

                    for timer in &state.timers {
                        ui.label(format!("{}", timer.id));
                        ui.label(&timer.name);
                        ui.label(format_timer_value(timer.value_us));
                        ui.label(format!("{}", timer.state));
                        ui.end_row();
                    }
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty() {
        let state = TimerDisplayState::default();
        assert!(state.timers.is_empty());
    }

    #[test]
    fn load_timers() {
        let mut state = TimerDisplayState::default();
        state.load_timers(vec![
            TimerInfo {
                id: 0,
                name: "Main".into(),
                value_us: 5_000_000,
                state: TimerState::Running,
            },
            TimerInfo {
                id: 1,
                name: "Fade".into(),
                value_us: 1_500_000,
                state: TimerState::Paused,
            },
        ]);
        assert_eq!(state.timers.len(), 2);
    }

    #[test]
    fn format_timer_zero() {
        assert_eq!(format_timer_value(0), "0.000s");
    }

    #[test]
    fn format_timer_milliseconds() {
        assert_eq!(format_timer_value(500_000), "0.500s");
    }

    #[test]
    fn format_timer_seconds() {
        assert_eq!(format_timer_value(5_123_000), "5.123s");
    }

    #[test]
    fn format_timer_minutes() {
        // 1 min 30.456s = 90_456_000 us
        assert_eq!(format_timer_value(90_456_000), "1:30.456");
    }

    #[test]
    fn format_timer_negative() {
        assert_eq!(format_timer_value(-1_234_000), "-1.234s");
    }

    #[test]
    fn timer_state_display() {
        assert_eq!(format!("{}", TimerState::Running), "Running");
        assert_eq!(format!("{}", TimerState::Paused), "Paused");
        assert_eq!(format!("{}", TimerState::Stopped), "Stopped");
    }
}

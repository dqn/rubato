use crate::core::performance_metrics::{EventResult, PerformanceMetrics};

use rubato_types::sync_utils::lock_or_recover;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

static EVENT_TREE: Mutex<Option<HashMap<u64, Vec<EventResult>>>> = Mutex::new(None);
static LAST_EVENT_UPDATE: Mutex<Option<Instant>> = Mutex::new(None);

struct WatchStats {
    avg: f32,
    std: f32,
}

static WATCH_DATA: Mutex<Vec<(String, WatchStats)>> = Mutex::new(Vec::new());

pub static FILTER_SHORT_THRESHOLD: Mutex<f32> = Mutex::new(1.0);
static SORT_BY_DURATION: Mutex<bool> = Mutex::new(false);

pub struct PerformanceMonitor;

impl PerformanceMonitor {
    /// Render performance monitor using egui.
    pub fn show_ui(ctx: &egui::Context) {
        let now = Instant::now();
        {
            let last_update = lock_or_recover(&LAST_EVENT_UPDATE);
            let should_reload = match &*last_update {
                None => true,
                Some(t) => now.duration_since(*t).as_nanos() > 500_000_000,
            };
            if should_reload {
                drop(last_update);
                *lock_or_recover(&LAST_EVENT_UPDATE) = Some(now);
                Self::reload_event_tree();
            }
        }

        let mut open = true;
        egui::Window::new("Performance Monitor")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.collapsing("Watch", |ui| {
                    let watch_data = lock_or_recover(&WATCH_DATA);
                    if watch_data.is_empty() {
                        ui.label("No watch data");
                    } else {
                        egui::Grid::new("watch_grid").show(ui, |ui| {
                            ui.label("Name");
                            ui.label("Avg (ms)");
                            ui.label("Std (ms)");
                            ui.end_row();
                            for (name, stats) in watch_data.iter() {
                                ui.label(name);
                                ui.label(format!("{:.2}", stats.avg));
                                ui.label(format!("{:.2}", stats.std));
                                ui.end_row();
                            }
                        });
                    }
                });

                ui.collapsing("Events", |ui| {
                    let tree = lock_or_recover(&EVENT_TREE);
                    if let Some(ref tree) = *tree {
                        let threshold = *lock_or_recover(&FILTER_SHORT_THRESHOLD);
                        ui.horizontal(|ui| {
                            ui.label("Filter threshold (ms):");
                            let mut t = threshold;
                            ui.add(egui::DragValue::new(&mut t).speed(0.1));
                            *lock_or_recover(&FILTER_SHORT_THRESHOLD) = t;
                        });
                        ui.horizontal(|ui| {
                            let mut sort = *lock_or_recover(&SORT_BY_DURATION);
                            ui.checkbox(&mut sort, "Sort by duration");
                            *lock_or_recover(&SORT_BY_DURATION) = sort;
                        });
                        // Render event tree recursively
                        render_event_tree_ui(ui, tree, 0, threshold);
                    } else {
                        ui.label("No event data");
                    }
                });
            });
    }

    pub fn reload_event_tree() {
        // copy the vector to avoid constantly reading the events while other threads might be writing
        let mut new_tree: HashMap<u64, Vec<EventResult>> = HashMap::new();
        let metrics = PerformanceMetrics::get();
        let events = {
            let results = lock_or_recover(&metrics.event_results);
            results.clone()
        };
        for event in &events {
            new_tree
                .entry(event.parent)
                .or_default()
                .push(event.clone());
        }
        *lock_or_recover(&EVENT_TREE) = Some(new_tree);
    }
}

/// Render events for the given parent group as a recursive collapsible tree.
fn render_event_tree_ui(
    ui: &mut egui::Ui,
    tree: &HashMap<u64, Vec<EventResult>>,
    group_id: u64,
    threshold: f32,
) {
    let Some(group) = tree.get(&group_id) else {
        return;
    };
    let sort_by_duration = *lock_or_recover(&SORT_BY_DURATION);
    let mut events: Vec<_> = group.iter().collect();
    if sort_by_duration {
        events.sort_unstable_by(|a, b| b.duration.cmp(&a.duration));
    }
    for event in &events {
        let duration_ms = event.duration as f64 / 1_000_000.0;
        if (duration_ms as f32) < threshold {
            continue;
        }
        let label = format!("{}: {:.2}ms [{}]", event.name, duration_ms, event.thread);
        let leaf = !tree.contains_key(&event.id);
        if leaf {
            ui.label(&label);
        } else {
            let id = egui::Id::new("perf_event").with(event.id);
            egui::CollapsingHeader::new(&label)
                .id_salt(id)
                .show(ui, |ui| {
                    render_event_tree_ui(ui, tree, event.id, threshold);
                });
        }
    }
}

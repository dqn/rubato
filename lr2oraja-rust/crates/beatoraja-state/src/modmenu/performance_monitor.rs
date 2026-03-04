use beatoraja_core::performance_metrics::{EventResult, PerformanceMetrics};

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

static EVENT_TREE: Mutex<Option<HashMap<i32, Vec<EventResult>>>> = Mutex::new(None);
static LAST_EVENT_UPDATE: Mutex<Option<Instant>> = Mutex::new(None);

struct WatchStats {
    avg: f32,
    std: f32,
}

static WATCH_DATA: Mutex<Vec<(String, WatchStats)>> = Mutex::new(Vec::new());
static LAST_STAT_UPDATE: Mutex<Option<Instant>> = Mutex::new(None);

pub static FILTER_SHORT_THRESHOLD: Mutex<f32> = Mutex::new(1.0);
static SORT_BY_DURATION: Mutex<bool> = Mutex::new(false);

pub struct PerformanceMonitor;

impl PerformanceMonitor {
    /// Render performance monitor using egui.
    pub fn show_ui(ctx: &egui::Context) {
        let now = Instant::now();
        {
            let last_update = LAST_EVENT_UPDATE.lock().unwrap();
            let should_reload = match &*last_update {
                None => true,
                Some(t) => now.duration_since(*t).as_nanos() > 500_000_000,
            };
            if should_reload {
                drop(last_update);
                *LAST_EVENT_UPDATE.lock().unwrap() = Some(now);
                Self::reload_event_tree();
            }
        }

        let mut open = true;
        egui::Window::new("Performance Monitor")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.collapsing("Watch", |ui| {
                    let watch_data = WATCH_DATA.lock().unwrap();
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
                    let tree = EVENT_TREE.lock().unwrap();
                    if let Some(ref tree) = *tree {
                        let threshold = *FILTER_SHORT_THRESHOLD.lock().unwrap();
                        ui.horizontal(|ui| {
                            ui.label("Filter threshold (ms):");
                            let mut t = threshold;
                            ui.add(egui::DragValue::new(&mut t).speed(0.1));
                            *FILTER_SHORT_THRESHOLD.lock().unwrap() = t;
                        });
                        ui.horizontal(|ui| {
                            let mut sort = *SORT_BY_DURATION.lock().unwrap();
                            ui.checkbox(&mut sort, "Sort by duration");
                            *SORT_BY_DURATION.lock().unwrap() = sort;
                        });
                        // Render root events
                        if let Some(roots) = tree.get(&-1) {
                            let sort_by_duration = *SORT_BY_DURATION.lock().unwrap();
                            let mut events: Vec<_> = roots.iter().collect();
                            if sort_by_duration {
                                events.sort_unstable_by(|a, b| b.duration.cmp(&a.duration));
                            }
                            for event in &events {
                                ui.label(format!(
                                    "{}: {:.2}ms",
                                    event.name,
                                    event.duration as f64 / 1_000_000.0
                                ));
                            }
                        }
                    } else {
                        ui.label("No event data");
                    }
                });
            });
    }

    pub fn reload_event_tree() {
        // copy the vector to avoid constantly reading the events while other threads might be writing
        let mut new_tree: HashMap<i32, Vec<EventResult>> = HashMap::new();
        let metrics = PerformanceMetrics::get();
        let events = {
            let results = metrics.event_results.lock().unwrap();
            results.clone()
        };
        for event in &events {
            new_tree
                .entry(event.parent)
                .or_default()
                .push(event.clone());
        }
        *EVENT_TREE.lock().unwrap() = Some(new_tree);
    }
}

fn update_watch_data() {
    let now = Instant::now();
    {
        let last_update = LAST_STAT_UPDATE.lock().unwrap();
        let should_update = match &*last_update {
            None => true,
            Some(t) => now.duration_since(*t).as_nanos() > 100_000_000,
        };
        if !should_update {
            return;
        }
    }
    *LAST_STAT_UPDATE.lock().unwrap() = Some(now);

    let metrics = PerformanceMetrics::get();
    let names = metrics.get_watch_names();
    let mut new_watch_data = Vec::new();

    for name in &names {
        if let Some(record) = metrics.get_watch_records(name) {
            if record.is_empty() {
                new_watch_data.push((name.clone(), WatchStats { avg: 0.0, std: 0.0 }));
                continue;
            }

            let mut sum: i64 = 0;
            for &(_time, value) in &record {
                sum += value;
            }
            let avg_us = (sum / record.len() as i64) as f32 / 1000.0;
            let mut variance: f32 = 0.0;
            for &(_time, value) in &record {
                let us = value as f32 / 1000.0;
                variance += (avg_us - us) * (avg_us - us);
            }
            variance /= record.len() as f32;
            let std = variance.sqrt();
            new_watch_data.push((name.clone(), WatchStats { avg: avg_us, std }));
        } else {
            new_watch_data.push((name.clone(), WatchStats { avg: 0.0, std: 0.0 }));
        }
    }

    *WATCH_DATA.lock().unwrap() = new_watch_data;
}

fn render_watch_data() {
    let watch_data = WATCH_DATA.lock().unwrap();
    for (name, data) in watch_data.iter() {
        let _text1 = name.clone();
        let _text2 = format!("avg = {:.1}us, std = {:.1}us", data.avg, data.std);
        // ImGui.text(text1);
        // ImGui.text(text2);
    }
}

fn render_event_table() {
    let threshold = *FILTER_SHORT_THRESHOLD.lock().unwrap();
    // ImGui.setNextItemWidth(ImGui.getContentRegionAvail().x / 5.f);
    // ImGui.sliderFloat("Filter short events", filterShortThreshold, 0.0f, 4.0f);

    // if (ImGui.beginTable("event-table", 3, ImGuiTableFlags.ScrollY))
    {
        // ImGui.tableSetupColumn("Event", ...);
        // ImGui.tableSetupColumn("Time", ...);
        // ImGui.tableSetupColumn("Thread", ...);
        // ImGui.tableHeadersRow();

        // ImGui.tableNextRow();
        // ImGui.tableNextColumn();

        render_event_tree(0, threshold);

        // ImGui.endTable();
    }
}

fn render_event_tree(group_id: i32, threshold: f32) {
    let event_tree = EVENT_TREE.lock().unwrap();
    if let Some(ref tree) = *event_tree {
        if !tree.contains_key(&group_id) {
            return;
        }

        if let Some(group) = tree.get(&group_id) {
            let sort_by_duration = *SORT_BY_DURATION.lock().unwrap();
            let mut events: Vec<_> = group.iter().collect();
            if sort_by_duration {
                events.sort_unstable_by(|a, b| b.duration.cmp(&a.duration));
            }
            for event in &events {
                let duration_ms = event.duration as f64 / 1_000_000.0;
                if (duration_ms as f32) < threshold {
                    continue;
                }

                let leaf = !tree.contains_key(&event.id);
                let _flags = if leaf {
                    // ImGuiTreeNodeFlags.Leaf | ImGuiTreeNodeFlags.NoTreePushOnOpen | ImGuiTreeNodeFlags.Bullet
                    0
                } else {
                    0
                };
                // boolean open = ImGui.treeNodeEx(event.id(), flags, event.name());
                // ImGui.tableNextColumn();
                let _time_text = format!("{:9.2}ms", duration_ms);
                // ImGui.text(time_text);
                // ImGui.tableNextColumn();
                let _thread_text = event.thread.to_string();
                // ImGui.text(thread_text);
                // ImGui.tableNextColumn();
                // if (!leaf && open) {
                //     renderEventTree(event.id());
                //     ImGui.treePop();
                // }
            }
        }
    }
}

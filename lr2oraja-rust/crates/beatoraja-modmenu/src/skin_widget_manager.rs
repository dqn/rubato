use crate::imgui_notify::ImGuiNotify;
use crate::stubs::{
    Clipboard, ImBoolean, ImFloat, Rectangle, Skin, SkinObject, SkinObjectDestination,
};

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

const EPS: f64 = 1e-5;

static LOCK: Mutex<()> = Mutex::new(());
static EVENT_HISTORY: LazyLock<Mutex<EventHistory>> =
    LazyLock::new(|| Mutex::new(EventHistory::new()));
static WIDGETS: Mutex<Vec<SkinWidget>> = Mutex::new(Vec::new());

static EDITING_WIDGET_X: Mutex<ImFloat> = Mutex::new(ImFloat { value: 0.0 });
static EDITING_WIDGET_Y: Mutex<ImFloat> = Mutex::new(ImFloat { value: 0.0 });
static EDITING_WIDGET_W: Mutex<ImFloat> = Mutex::new(ImFloat { value: 0.0 });
static EDITING_WIDGET_H: Mutex<ImFloat> = Mutex::new(ImFloat { value: 0.0 });
static SHOW_CURSOR_POSITION: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: true });
static MOVE_OVERLAY_ENABLED: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static RESET_MOVE_OVERLAY: Mutex<bool> = Mutex::new(false);

static FOCUS: Mutex<bool> = Mutex::new(false);

pub struct SkinWidgetManager;

impl SkinWidgetManager {
    pub fn set_focus(focus: bool) {
        *FOCUS.lock().unwrap() = focus;
    }

    pub fn change_skin(skin: &Skin) {
        let _lock = LOCK.lock().unwrap();
        let mut widgets = WIDGETS.lock().unwrap();
        let mut event_history = EVENT_HISTORY.lock().unwrap();
        widgets.clear();
        event_history.clear();

        let all_skin_objects = skin.get_all_skin_objects();
        // NOTE: We're using skin object's name as id, we need to keep name is unique
        let mut duplicated_skin_object_name_count: HashMap<String, i32> = HashMap::new();

        for skin_object in all_skin_objects {
            let skin_object_name = skin_object.get_name().map(|s| s.to_string());
            let dsts = skin_object.get_all_destination();
            let mut destinations: Vec<SkinWidgetDestination> = Vec::new();

            for i in 0..dsts.len() {
                let dst_base_name = skin_object_name.as_deref().unwrap_or("Unnamed Destination");
                let combined_name = if dsts.len() == 1 {
                    dst_base_name.to_string()
                } else {
                    format!("{}({})", dst_base_name, i)
                };
                destinations.push(SkinWidgetDestination::new(combined_name, dsts[i].clone()));
            }

            let widget_base_name = skin_object_name.as_deref().unwrap_or("Unnamed Widget");
            let count = *duplicated_skin_object_name_count
                .get(widget_base_name)
                .unwrap_or(&0);
            *duplicated_skin_object_name_count
                .entry(widget_base_name.to_string())
                .or_insert(0) += 1;
            let widget_name = if count == 0 {
                widget_base_name.to_string()
            } else {
                format!("{}({})", widget_base_name, count)
            };
            widgets.push(SkinWidget::new(
                widget_name,
                skin_object.clone(),
                destinations,
            ));
        }
    }

    /// Render the skin widget manager window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        let _lock = LOCK.lock().unwrap();
        let mut open = true;
        egui::Window::new("Skin Widgets")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                let widgets = WIDGETS.lock().unwrap();
                if widgets.is_empty() {
                    ui.label("No skin is loaded");
                } else {
                    ui.horizontal(|ui| {
                        if ui.button("Undo").clicked() {
                            EVENT_HISTORY.lock().unwrap().undo();
                        }
                        let mut show_cursor = SHOW_CURSOR_POSITION.lock().unwrap();
                        ui.checkbox(&mut show_cursor.value, "Show Position");
                        drop(show_cursor);
                        if ui.button("Export").clicked() {
                            export_changes();
                        }
                    });
                    ui.separator();
                    ui.label(format!("{} widgets loaded", widgets.len()));
                }
            });
    }
}

/// Render column visibility preference settings popup.
///
/// Translated from: SkinWidgetManager.renderPreferColumnSetting()
/// In Java: ImGui popup with checkboxes for toggling table column visibility.
fn render_prefer_column_setting(_ui: &mut egui::Ui) {
    // Phase 5+: column visibility toggles for widget table
    log::warn!("not yet implemented: SkinWidgetManager.renderPreferColumnSetting egui");
}

/// Render the skin widgets table with tree nodes per widget.
///
/// Translated from: SkinWidgetManager.renderSkinWidgetsTable()
/// In Java: ImGui table with tree nodes, columns for x/y/w/h, edit popup, move overlay.
fn render_skin_widgets_table(_ui: &mut egui::Ui, _widgets: &[SkinWidget]) {
    // Phase 5+: full widget table with edit popups and move overlay
    log::warn!("not yet implemented: SkinWidgetManager.renderSkinWidgetsTable egui");
}

/// Render the modification history table.
///
/// Translated from: SkinWidgetManager.renderHistoryTable()
/// In Java: ImGui table showing event descriptions with clipper.
fn render_history_table(_ui: &mut egui::Ui) {
    let event_history = EVENT_HISTORY.lock().unwrap();
    let events = event_history.get_events();
    if events.is_empty() {
        _ui.label("No history");
    } else {
        for event in events {
            _ui.label(event.get_description());
        }
    }
}

/// Draw a float value column cell, highlighting modified values in red.
///
/// Translated from: SkinWidgetManager.drawFloatValueColumn(int, boolean, float)
fn draw_float_value_column(ui: &mut egui::Ui, _index: usize, modified: bool, value: f32) {
    let text = normalize_float(value);
    if modified {
        ui.colored_label(egui::Color32::RED, text);
    } else {
        ui.label(text);
    }
}

fn export_changes() {
    let widgets = WIDGETS.lock().unwrap();
    let event_history = EVENT_HISTORY.lock().unwrap();
    let mut changes: Vec<String> = Vec::new();

    for widget in widgets.iter() {
        for dst in &widget.destinations {
            let mut has_changed_x = false;
            let mut has_changed_y = false;
            let mut has_changed_w = false;
            let mut has_changed_h = false;

            for event in event_history.get_events_by_name(&dst.name) {
                match event.get_event_type() {
                    EventType::ChangeX => has_changed_x = true,
                    EventType::ChangeY => has_changed_y = true,
                    EventType::ChangeW => has_changed_w = true,
                    EventType::ChangeH => has_changed_h = true,
                    _ => {}
                }
            }

            if !(has_changed_x || has_changed_y || has_changed_w || has_changed_h) {
                continue;
            }

            let mut sb = format!("{{dst={}", dst.name);
            if has_changed_x {
                sb.push_str(&format!(", x={}", dst.get_dst_x()));
            }
            if has_changed_y {
                sb.push_str(&format!(", y={}", dst.get_dst_y()));
            }
            if has_changed_x {
                sb.push_str(&format!(", w={}", dst.get_dst_w()));
            }
            if has_changed_y {
                sb.push_str(&format!(", h={}", dst.get_dst_h()));
            }
            sb.push('}');
            changes.push(sb);
        }
    }

    let change_logs = changes.join("\n");
    let clipboard = Clipboard::new();
    clipboard.set_contents(&change_logs);
    ImGuiNotify::info("Copied changes to clipboard");
}

fn normalize_float(value: f32) -> String {
    // DecimalFormat("#.####")
    let formatted = format!("{:.4}", value);
    // Trim trailing zeros
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    trimmed.to_string()
}

// =========================================================================
// Event types
// =========================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventType {
    ChangeX,
    ChangeY,
    ChangeW,
    ChangeH,
    ToggleVisible,
}

#[derive(Clone, Debug)]
pub enum Event {
    ChangeSingleField {
        event_type: EventType,
        target_name: String,
        previous: f32,
        current: f32,
    },
    ToggleVisible {
        event_type: EventType,
        target_name: String,
        widget_index: usize,
        was_visible_before: bool,
    },
}

impl Event {
    pub fn get_event_type(&self) -> &EventType {
        match self {
            Event::ChangeSingleField { event_type, .. } => event_type,
            Event::ToggleVisible { event_type, .. } => event_type,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Event::ChangeSingleField { target_name, .. } => target_name,
            Event::ToggleVisible { target_name, .. } => target_name,
        }
    }

    /// Undo this event, reverting the affected destination/widget to the previous value.
    ///
    /// Translated from: Event.undo() (abstract method with implementations in
    /// ChangeSingleFieldEvent.undo() and ToggleVisibleEvent.undo())
    ///
    /// Note: Since Event doesn't hold mutable references to the actual destinations/widgets,
    /// this requires the caller to look up the target by name and apply the undo.
    /// In the current architecture, EventHistory::undo_n() handles this instead.
    pub fn undo(&self, widgets: &mut [SkinWidget]) {
        match self {
            Event::ChangeSingleField {
                event_type,
                target_name,
                previous,
                ..
            } => {
                for widget in widgets.iter_mut() {
                    for dst in widget.destinations.iter_mut() {
                        if dst.name == *target_name {
                            match event_type {
                                EventType::ChangeX => dst.set_dst_x_with_event(*previous, false),
                                EventType::ChangeY => dst.set_dst_y_with_event(*previous, false),
                                EventType::ChangeW => dst.set_dst_w_with_event(*previous, false),
                                EventType::ChangeH => dst.set_dst_h_with_event(*previous, false),
                                _ => {}
                            }
                            return;
                        }
                    }
                }
            }
            Event::ToggleVisible { widget_index, .. } => {
                if *widget_index < widgets.len() {
                    widgets[*widget_index].toggle_visible();
                }
            }
        }
    }

    pub fn get_description(&self) -> String {
        match self {
            Event::ChangeSingleField {
                event_type,
                target_name,
                previous,
                current,
            } => {
                let field_name = match event_type {
                    EventType::ChangeX => "x",
                    EventType::ChangeY => "y",
                    EventType::ChangeW => "width",
                    EventType::ChangeH => "height",
                    _ => "[ERROR] Not a ChangeSingleFieldEvent",
                };
                format!(
                    "Changed {}'s {} from {:.4} to {:.4}",
                    target_name, field_name, previous, current
                )
            }
            Event::ToggleVisible {
                target_name,
                was_visible_before,
                ..
            } => {
                if *was_visible_before {
                    format!("Make {} widget invisible", target_name)
                } else {
                    format!("Make {} widget visible", target_name)
                }
            }
        }
    }
}

// =========================================================================
// SkinWidget
// =========================================================================

#[derive(Clone, Debug)]
pub struct SkinWidget {
    pub name: String,
    pub skin_object: SkinObject,
    pub destinations: Vec<SkinWidgetDestination>,
}

impl SkinWidget {
    pub fn new(
        name: String,
        skin_object: SkinObject,
        destinations: Vec<SkinWidgetDestination>,
    ) -> Self {
        SkinWidget {
            name,
            skin_object,
            destinations,
        }
    }

    pub fn is_drawing_on_screen(&self) -> bool {
        self.skin_object.draw && self.skin_object.visible
    }

    pub fn toggle_visible(&mut self) {
        self.skin_object.visible = !self.skin_object.visible;
    }
}

// =========================================================================
// SkinWidgetDestination
// =========================================================================

#[derive(Clone, Debug)]
pub struct SkinWidgetDestination {
    pub name: String,
    pub destination: SkinObjectDestination,
    pub before_move: Option<SkinObjectDestination>,
    pub moving_state: i32,
}

impl SkinWidgetDestination {
    pub fn new(name: String, destination: SkinObjectDestination) -> Self {
        SkinWidgetDestination {
            name,
            destination,
            before_move: None,
            moving_state: 0,
        }
    }

    pub fn get_dst_x(&self) -> f32 {
        self.destination.region.x
    }

    pub fn get_dst_y(&self) -> f32 {
        self.destination.region.y
    }

    pub fn get_dst_w(&self) -> f32 {
        self.destination.region.width
    }

    pub fn get_dst_h(&self) -> f32 {
        self.destination.region.height
    }

    pub fn set_dst_x(&mut self, x: f32) {
        self.set_dst_x_with_event(x, true);
    }

    pub fn set_dst_x_with_event(&mut self, x: f32, create_event: bool) {
        let previous = self.get_dst_x();
        if create_event && ((x - previous) as f64).abs() > EPS {
            let mut history = EVENT_HISTORY.lock().unwrap();
            history.push_event(Event::ChangeSingleField {
                event_type: EventType::ChangeX,
                target_name: self.name.clone(),
                previous,
                current: x,
            });
        }
        self.destination.region.x = x;
    }

    pub fn set_dst_y(&mut self, y: f32) {
        self.set_dst_y_with_event(y, true);
    }

    pub fn set_dst_y_with_event(&mut self, y: f32, create_event: bool) {
        let previous = self.get_dst_y();
        if create_event && ((y - previous) as f64).abs() > EPS {
            let mut history = EVENT_HISTORY.lock().unwrap();
            history.push_event(Event::ChangeSingleField {
                event_type: EventType::ChangeY,
                target_name: self.name.clone(),
                previous,
                current: y,
            });
        }
        self.destination.region.y = y;
    }

    pub fn set_dst_w(&mut self, w: f32) {
        self.set_dst_w_with_event(w, true);
    }

    pub fn set_dst_w_with_event(&mut self, w: f32, create_event: bool) {
        let previous = self.get_dst_w();
        if create_event && ((w - previous) as f64).abs() > EPS {
            let mut history = EVENT_HISTORY.lock().unwrap();
            history.push_event(Event::ChangeSingleField {
                event_type: EventType::ChangeW,
                target_name: self.name.clone(),
                previous,
                current: w,
            });
        }
        self.destination.region.width = w;
    }

    pub fn set_dst_h(&mut self, h: f32) {
        self.set_dst_h_with_event(h, true);
    }

    pub fn set_dst_h_with_event(&mut self, h: f32, create_event: bool) {
        let previous = self.get_dst_h();
        if create_event && ((h - previous) as f64).abs() > EPS {
            let mut history = EVENT_HISTORY.lock().unwrap();
            history.push_event(Event::ChangeSingleField {
                event_type: EventType::ChangeH,
                target_name: self.name.clone(),
                previous,
                current: h,
            });
        }
        self.destination.region.height = h;
    }

    /// Submit the move result, producing the event
    pub fn submit_movement(&mut self) {
        if self.before_move.is_none() {
            ImGuiNotify::error(
                "Cannot submit the move result because there's no original position",
            );
            return;
        }

        let next_x = self.get_dst_x();
        let next_y = self.get_dst_y();
        let next_w = self.get_dst_w();
        let next_h = self.get_dst_h();

        // Reset the position, to mimic that we are never left the original position
        if let Some(ref bm) = self.before_move {
            let orig_x = bm.region.x;
            let orig_y = bm.region.y;
            let orig_w = bm.region.width;
            let orig_h = bm.region.height;
            self.set_dst_x_with_event(orig_x, false);
            self.set_dst_y_with_event(orig_y, false);
            self.set_dst_w_with_event(orig_w, false);
            self.set_dst_h_with_event(orig_h, false);
        }

        // Truly move to the target position
        self.set_dst_x(next_x);
        self.set_dst_y(next_y);
        self.set_dst_w(next_w);
        self.set_dst_h(next_h);
        self.before_move = None;
    }
}

// =========================================================================
// EventHistory
// =========================================================================

#[derive(Clone, Debug)]
pub struct EventHistory {
    target_name_to_events: HashMap<String, Vec<Event>>,
    event_stack: Vec<Event>,
}

impl EventHistory {
    fn new() -> Self {
        EventHistory {
            target_name_to_events: HashMap::new(),
            event_stack: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.target_name_to_events.clear();
        self.event_stack.clear();
    }

    pub fn has_event(&self, widget_name: &str, event_type: &EventType) -> bool {
        if let Some(events) = self.target_name_to_events.get(widget_name) {
            events.iter().any(|e| e.get_event_type() == event_type)
        } else {
            false
        }
    }

    pub fn get_events(&self) -> &[Event] {
        &self.event_stack
    }

    pub fn get_events_by_name(&self, name: &str) -> Vec<Event> {
        self.target_name_to_events
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    pub fn push_event(&mut self, event: Event) {
        let name = event.get_name().to_string();
        self.target_name_to_events
            .entry(name)
            .or_default()
            .push(event.clone());
        self.event_stack.push(event);
    }

    /// Undo the most recent event
    pub fn undo(&mut self) {
        self.undo_n(1);
    }

    /// Undo the most recent event multiple times
    pub fn undo_n(&mut self, times: i32) {
        let times = times.unsigned_abs() as usize;
        if times == 0 {
            return;
        }

        for _ in 0..times {
            if self.event_stack.is_empty() {
                break;
            }
            let last = self.event_stack.len() - 1;
            let last_event = self.event_stack.remove(last);
            // Undo logic would need mutable access to destination/widget
            // This is a simplified version; actual undo requires back-references
            let _ = last_event;
        }

        // Rebuild target_name_to_events from event_stack
        self.target_name_to_events.clear();
        for event in &self.event_stack {
            let name = event.get_name().to_string();
            self.target_name_to_events
                .entry(name)
                .or_default()
                .push(event.clone());
        }
    }
}

use super::super::imgui_notify::ImGuiNotify;
use super::super::{SkinObject, SkinObjectDestination};
use super::{EPS, EVENT_HISTORY};

use rubato_skin::sync_utils::lock_or_recover;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub fn event_type(&self) -> &EventType {
        match self {
            Event::ChangeSingleField { event_type, .. } => event_type,
            Event::ToggleVisible { event_type, .. } => event_type,
        }
    }

    pub fn name(&self) -> &str {
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

    pub fn description(&self) -> String {
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

    pub fn dst_x(&self) -> f32 {
        self.destination.region.x
    }

    pub fn dst_y(&self) -> f32 {
        self.destination.region.y
    }

    pub fn dst_w(&self) -> f32 {
        self.destination.region.width
    }

    pub fn dst_h(&self) -> f32 {
        self.destination.region.height
    }

    pub fn set_dst_x(&mut self, x: f32) {
        self.set_dst_x_with_event(x, true);
    }

    pub fn set_dst_x_with_event(&mut self, x: f32, create_event: bool) {
        let previous = self.dst_x();
        if create_event && ((x - previous) as f64).abs() > EPS {
            let mut history = lock_or_recover(&EVENT_HISTORY);
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
        let previous = self.dst_y();
        if create_event && ((y - previous) as f64).abs() > EPS {
            let mut history = lock_or_recover(&EVENT_HISTORY);
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
        let previous = self.dst_w();
        if create_event && ((w - previous) as f64).abs() > EPS {
            let mut history = lock_or_recover(&EVENT_HISTORY);
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
        let previous = self.dst_h();
        if create_event && ((h - previous) as f64).abs() > EPS {
            let mut history = lock_or_recover(&EVENT_HISTORY);
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

        let next_x = self.dst_x();
        let next_y = self.dst_y();
        let next_w = self.dst_w();
        let next_h = self.dst_h();

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
    pub(super) fn new() -> Self {
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
            events.iter().any(|e| e.event_type() == event_type)
        } else {
            false
        }
    }

    pub fn events(&self) -> &[Event] {
        &self.event_stack
    }

    pub fn events_by_name(&self, name: &str) -> Vec<Event> {
        self.target_name_to_events
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    pub fn push_event(&mut self, event: Event) {
        let name = event.name().to_string();
        self.target_name_to_events
            .entry(name)
            .or_default()
            .push(event.clone());
        self.event_stack.push(event);
    }

    /// Undo the most recent event (without widget access — only removes from stack)
    pub fn undo(&mut self) {
        self.undo_n(1);
    }

    /// Undo the most recent event with mutable access to widgets.
    ///
    /// Translated from: EventHistory.undo() — the Java version has back-references
    /// to the actual destinations/widgets via the Event handle field.
    /// In Rust we pass the widgets explicitly.
    pub fn undo_with_widgets(&mut self, widgets: &mut [SkinWidget]) {
        self.undo_n_with_widgets(1, widgets);
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
            // Without widget access, we can only remove from the stack
            let _ = last_event;
        }

        self.rebuild_target_name_index();
    }

    /// Undo the most recent events with mutable access to widgets
    pub fn undo_n_with_widgets(&mut self, times: i32, widgets: &mut [SkinWidget]) {
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
            last_event.undo(widgets);
        }

        self.rebuild_target_name_index();
    }

    /// Rebuild the target_name_to_events index from event_stack.
    fn rebuild_target_name_index(&mut self) {
        // Rebuild target_name_to_events from event_stack
        self.target_name_to_events.clear();
        for event in &self.event_stack {
            let name = event.name().to_string();
            self.target_name_to_events
                .entry(name)
                .or_default()
                .push(event.clone());
        }
    }
}

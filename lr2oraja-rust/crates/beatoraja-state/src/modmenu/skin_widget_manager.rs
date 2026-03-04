use super::imgui_notify::ImGuiNotify;
use super::imgui_renderer;
use super::stubs::{
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

// =========================================================================
// WidgetTableColumn — column configuration for the widget table
// =========================================================================

/// Getter function type: extracts a float value from a SkinWidgetDestination.
type ColumnGetter = fn(&SkinWidgetDestination) -> f32;

/// Represents one column in the widget table.
///
/// Translated from: SkinWidgetManager.WidgetTableColumn (Java record)
pub struct WidgetTableColumn {
    pub name: &'static str,
    pub show: bool,
    pub persistent: bool,
    pub getter: Option<ColumnGetter>,
    pub change_event_type: Option<EventType>,
}

static WIDGET_TABLE_COLUMNS: LazyLock<Mutex<Vec<WidgetTableColumn>>> = LazyLock::new(|| {
    Mutex::new(vec![
        WidgetTableColumn {
            name: "ID",
            show: true,
            persistent: true,
            getter: None,
            change_event_type: None,
        },
        WidgetTableColumn {
            name: "x",
            show: true,
            persistent: false,
            getter: Some(SkinWidgetDestination::get_dst_x),
            change_event_type: Some(EventType::ChangeX),
        },
        WidgetTableColumn {
            name: "y",
            show: true,
            persistent: false,
            getter: Some(SkinWidgetDestination::get_dst_y),
            change_event_type: Some(EventType::ChangeY),
        },
        WidgetTableColumn {
            name: "w",
            show: true,
            persistent: false,
            getter: Some(SkinWidgetDestination::get_dst_w),
            change_event_type: Some(EventType::ChangeW),
        },
        WidgetTableColumn {
            name: "h",
            show: true,
            persistent: false,
            getter: Some(SkinWidgetDestination::get_dst_h),
            change_event_type: Some(EventType::ChangeH),
        },
        WidgetTableColumn {
            name: "Operation",
            show: true,
            persistent: true,
            getter: None,
            change_event_type: None,
        },
    ])
});

pub struct SkinWidgetManager;

impl SkinWidgetManager {
    pub fn get_focus() -> bool {
        beatoraja_types::skin_widget_focus::get_focus()
    }

    pub fn set_focus(focus: bool) {
        beatoraja_types::skin_widget_focus::set_focus(focus);
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
    ///
    /// Translated from: SkinWidgetManager.show(ImBoolean)
    /// In Java: ImGui window with tab bar (SkinWidgets + History), column settings,
    /// cursor position overlay.
    pub fn show_ui(ctx: &egui::Context) {
        let _lock = LOCK.lock().unwrap();
        let mut open = true;
        egui::Window::new("Skin Widgets")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                let mut widgets = WIDGETS.lock().unwrap();
                if widgets.is_empty() {
                    ui.label("No skin is loaded");
                } else {
                    // Use a simple approach: selectable labels acting as tabs
                    // egui doesn't have built-in tab bars, so we use a stateful tab index
                    let tab_id = ui.id().with("skin_widgets_tab");
                    let mut tab_index =
                        ui.memory(|mem| mem.data.get_temp::<usize>(tab_id).unwrap_or(0));

                    ui.horizontal(|ui| {
                        if ui.selectable_label(tab_index == 0, "SkinWidgets").clicked() {
                            tab_index = 0;
                        }
                        if ui.selectable_label(tab_index == 1, "History").clicked() {
                            tab_index = 1;
                        }
                    });
                    ui.memory_mut(|mem| mem.data.insert_temp(tab_id, tab_index));

                    ui.separator();

                    if tab_index == 0 {
                        // SkinWidgets tab
                        ui.horizontal(|ui| {
                            if ui.button("Undo").clicked() {
                                let mut event_history = EVENT_HISTORY.lock().unwrap();
                                event_history.undo_with_widgets(&mut widgets);
                            }
                            render_prefer_column_setting(ui);
                            let mut show_cursor = SHOW_CURSOR_POSITION.lock().unwrap();
                            ui.checkbox(&mut show_cursor.value, "Show Position");
                            drop(show_cursor);
                            if ui.button("Export").clicked() {
                                export_changes();
                            }
                        });

                        render_skin_widgets_table(ui, &mut widgets);
                    } else {
                        // History tab
                        render_history_table(ui);
                    }

                    // Overlay cursor position
                    let show_cursor = SHOW_CURSOR_POSITION.lock().unwrap();
                    if show_cursor.value {
                        let window_height = imgui_renderer::window_height() as f32;
                        if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                            // Java: Gdx.input.getX() / (windowHeight - Gdx.input.getY())
                            let skin_y = window_height - pos.y;
                            ui.label(format!("({:.0}, {:.0})", pos.x, skin_y));
                        }
                    }
                }
            });
    }
}

/// Render column visibility preference settings popup.
///
/// Translated from: SkinWidgetManager.renderPreferColumnSetting()
/// In Java: ImGui popup with checkboxes for toggling table column visibility.
fn render_prefer_column_setting(ui: &mut egui::Ui) {
    let popup_id = ui.make_persistent_id("PreferColumnSetting");
    let response = ui.button("Columns");
    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    egui::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            let mut columns = WIDGET_TABLE_COLUMNS.lock().unwrap();
            for column in columns.iter_mut() {
                if column.persistent {
                    continue;
                }
                ui.checkbox(&mut column.show, column.name);
            }
        },
    );
}

/// Render the skin widgets table with tree nodes per widget.
///
/// Translated from: SkinWidgetManager.renderSkinWidgetsTable()
/// In Java: ImGui table with tree nodes, columns for x/y/w/h, edit popup, move overlay.
fn render_skin_widgets_table(ui: &mut egui::Ui, widgets: &mut [SkinWidget]) {
    // NOTE: This will create a snapshot for us, which can kinda prevent us step into race condition
    let columns = WIDGET_TABLE_COLUMNS.lock().unwrap();
    let showing_columns: Vec<(usize, &WidgetTableColumn)> = columns
        .iter()
        .enumerate()
        .filter(|(_, col)| col.show)
        .collect();
    let col_size = showing_columns.len();
    if col_size == 0 {
        return;
    }
    drop(columns);

    let columns = WIDGET_TABLE_COLUMNS.lock().unwrap();
    let showing_columns: Vec<(usize, &WidgetTableColumn)> = columns
        .iter()
        .enumerate()
        .filter(|(_, col)| col.show)
        .collect();

    // Table header
    egui::ScrollArea::vertical()
        .max_height(ui.text_style_height(&egui::TextStyle::Body) * 20.0)
        .show(ui, |ui| {
            egui::Grid::new("skin_widgets_table")
                .striped(true)
                .num_columns(col_size)
                .show(ui, |ui| {
                    // Header row
                    for (_, column) in &showing_columns {
                        ui.strong(column.name);
                    }
                    ui.end_row();

                    // Widget rows
                    for (widget_idx, widget) in widgets.iter_mut().enumerate() {
                        let is_widget_drawing = widget.is_drawing_on_screen();

                        // ID column: collapsing header
                        let header_id = ui.make_persistent_id(format!("widget_{}", widget_idx));
                        let mut is_open =
                            ui.memory(|mem| mem.data.get_temp::<bool>(header_id).unwrap_or(false));

                        // First column (ID): tree node
                        let label = if is_widget_drawing {
                            egui::RichText::new(&widget.name)
                        } else {
                            egui::RichText::new(&widget.name)
                                .color(egui::Color32::from_rgb(128, 128, 128))
                        };
                        if ui.selectable_label(is_open, label).clicked() {
                            is_open = !is_open;
                        }
                        ui.memory_mut(|mem| mem.data.insert_temp(header_id, is_open));

                        // Middle columns: "--" placeholder for the widget row
                        for i in 1..col_size.saturating_sub(1) {
                            let _ = i;
                            ui.weak("--");
                        }

                        // Last column (Operation): Toggle button
                        if col_size >= 2 {
                            let event_history = EVENT_HISTORY.lock().unwrap();
                            let was_visible = widget.skin_object.visible;
                            drop(event_history);
                            if ui.button("Toggle").clicked() {
                                let mut event_history = EVENT_HISTORY.lock().unwrap();
                                event_history.push_event(Event::ToggleVisible {
                                    event_type: EventType::ToggleVisible,
                                    target_name: widget.name.clone(),
                                    widget_index: widget_idx,
                                    was_visible_before: was_visible,
                                });
                                widget.toggle_visible();
                            }
                        }
                        ui.end_row();

                        // Destination sub-rows (when tree node is open)
                        if is_open {
                            for dst in widget.destinations.iter_mut() {
                                let dst_id = ui.make_persistent_id(format!("dst_{}", dst.name));

                                // First column: destination name
                                let dst_label = if is_widget_drawing {
                                    egui::RichText::new(&dst.name)
                                } else {
                                    egui::RichText::new(&dst.name)
                                        .color(egui::Color32::from_rgb(128, 128, 128))
                                };
                                ui.label(dst_label);

                                // Middle columns: float values
                                let event_history = EVENT_HISTORY.lock().unwrap();
                                let columns_ref = WIDGET_TABLE_COLUMNS.lock().unwrap();
                                let showing_mid: Vec<&WidgetTableColumn> =
                                    columns_ref.iter().filter(|col| col.show).collect();
                                // Columns from index 1 to col_size-2 (exclusive of first and last)
                                for (i, column) in showing_mid
                                    .iter()
                                    .enumerate()
                                    .take(showing_mid.len().saturating_sub(1))
                                    .skip(1)
                                {
                                    if let (Some(getter), Some(evt_type)) =
                                        (column.getter, &column.change_event_type)
                                    {
                                        let modified = event_history.has_event(&dst.name, evt_type);
                                        let value = getter(dst);
                                        draw_float_value_column(ui, i, modified, value);
                                    } else {
                                        ui.label("--");
                                    }
                                }
                                drop(columns_ref);
                                drop(event_history);

                                // Last column: Edit button
                                if col_size >= 2 {
                                    let edit_popup_id =
                                        ui.make_persistent_id(format!("edit_popup_{}", dst.name));
                                    let edit_response = ui.button("Edit");
                                    if edit_response.clicked() {
                                        *EDITING_WIDGET_X.lock().unwrap() = ImFloat {
                                            value: dst.get_dst_x(),
                                        };
                                        *EDITING_WIDGET_Y.lock().unwrap() = ImFloat {
                                            value: dst.get_dst_y(),
                                        };
                                        *EDITING_WIDGET_W.lock().unwrap() = ImFloat {
                                            value: dst.get_dst_w(),
                                        };
                                        *EDITING_WIDGET_H.lock().unwrap() = ImFloat {
                                            value: dst.get_dst_h(),
                                        };
                                        *RESET_MOVE_OVERLAY.lock().unwrap() = true;
                                        ui.memory_mut(|mem| mem.toggle_popup(edit_popup_id));
                                    }

                                    let popup_open =
                                        ui.memory(|mem| mem.is_popup_open(edit_popup_id));

                                    if popup_open {
                                        egui::Area::new(edit_popup_id)
                                            .order(egui::Order::Foreground)
                                            .fixed_pos(edit_response.rect.left_bottom())
                                            .show(ui.ctx(), |ui| {
                                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                                    render_edit_popup(ui, dst, dst_id);
                                                });
                                            });
                                    } else {
                                        // If user clicked the empty space while moving widgets,
                                        // the whole popup would be closed too.
                                        // So we have to catch the "escaping" widget here.
                                        if dst.moving_state == 2 {
                                            dst.moving_state = 0;
                                            dst.submit_movement();
                                        }
                                    }
                                }

                                ui.end_row();
                            }
                        }
                    }
                });
        });
}

/// Render the edit popup for a single destination.
///
/// Translated from the inner part of renderSkinWidgetsTable() where
/// ImGui.beginPopup("Edit Skin Widget") is used.
fn render_edit_popup(ui: &mut egui::Ui, dst: &mut SkinWidgetDestination, _dst_id: egui::Id) {
    ui.label("Edit Skin Widget");
    ui.separator();

    let mut x = EDITING_WIDGET_X.lock().unwrap();
    ui.horizontal(|ui| {
        ui.label("x");
        ui.add(egui::DragValue::new(&mut x.value).speed(1.0));
    });
    drop(x);

    let mut y = EDITING_WIDGET_Y.lock().unwrap();
    ui.horizontal(|ui| {
        ui.label("y");
        ui.add(egui::DragValue::new(&mut y.value).speed(1.0));
    });
    drop(y);

    let mut w = EDITING_WIDGET_W.lock().unwrap();
    ui.horizontal(|ui| {
        ui.label("w");
        ui.add(egui::DragValue::new(&mut w.value).speed(1.0));
    });
    drop(w);

    let mut h = EDITING_WIDGET_H.lock().unwrap();
    ui.horizontal(|ui| {
        ui.label("h");
        ui.add(egui::DragValue::new(&mut h.value).speed(1.0));
    });
    drop(h);

    if ui.button("Submit").clicked() {
        dst.set_dst_x(EDITING_WIDGET_X.lock().unwrap().value);
        dst.set_dst_y(EDITING_WIDGET_Y.lock().unwrap().value);
        dst.set_dst_w(EDITING_WIDGET_W.lock().unwrap().value);
        dst.set_dst_h(EDITING_WIDGET_H.lock().unwrap().value);
    }

    // Move overlay checkbox
    let mut move_enabled = MOVE_OVERLAY_ENABLED.lock().unwrap();
    let old_move = move_enabled.value;
    ui.checkbox(&mut move_enabled.value, "Move");
    let just_enabled = move_enabled.value && !old_move;
    let reset = *RESET_MOVE_OVERLAY.lock().unwrap();

    if just_enabled || reset {
        *RESET_MOVE_OVERLAY.lock().unwrap() = false;
        // Position would be set via ImGui.setNextWindowPos/Size in Java
        // In egui, the overlay window position is set when creating the Area below
    }

    if move_enabled.value {
        if dst.moving_state == 0 {
            let cloned_region = Rectangle {
                x: dst.get_dst_x(),
                y: dst.get_dst_y(),
                width: dst.get_dst_w(),
                height: dst.get_dst_h(),
            };
            dst.before_move = Some(SkinObjectDestination {
                time: 0,
                region: cloned_region,
                color: None,
                angle: 0.0,
                alpha: 0.0,
            });
            dst.moving_state = 1;
        }

        drop(move_enabled);
        render_move_overlay(ui, dst);
    } else {
        dst.moving_state = 0;
    }
}

/// Render the move overlay window for drag-moving a widget destination.
///
/// Translated from the move overlay section in renderSkinWidgetsTable().
/// In Java: a borderless, styled ImGui window that displays x/y/w/h and allows
/// drag-moving the widget position.
fn render_move_overlay(ui: &mut egui::Ui, dst: &mut SkinWidgetDestination) {
    let window_height = imgui_renderer::window_height() as f32;
    let w = dst.get_dst_w();
    let h = dst.get_dst_h();
    let x = dst.get_dst_x();
    let y = window_height - dst.get_dst_y() - h;

    let move_enabled = MOVE_OVERLAY_ENABLED.lock().unwrap();

    egui::Window::new("widget-overlay-popup")
        .fixed_pos(egui::pos2(x, y))
        .fixed_size(egui::vec2(w.max(100.0), h.max(40.0)))
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .frame(
            egui::Frame::window(&egui::Style::default())
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 102))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(51, 102, 255, 255),
                )),
        )
        .show(ui.ctx(), |ui| {
            ui.label(format!("x = {:.1} y = {:.1}", x, dst.get_dst_y()));
            ui.label(format!("w = {:.1} h = {:.1}", w, h));

            // NOTE: This approach is actually moving the "REAL" widget in-time
            dst.set_dst_x_with_event(x, false);
            dst.set_dst_y_with_event(dst.get_dst_y(), false);
            dst.set_dst_w_with_event(w, false);
            dst.set_dst_h_with_event(h, false);
        });

    // Focus state machine: 0 -> 1 -> 2 -> submit
    // In egui we can't easily detect window focus, so we use a simplified approach:
    // The move overlay stays active until the user unchecks "Move"
    if dst.moving_state == 1 {
        dst.moving_state = 2;
    }

    if !move_enabled.value && dst.moving_state == 2 {
        dst.moving_state = 0;
        dst.submit_movement();
    }
}

/// Render the modification history table.
///
/// Translated from: SkinWidgetManager.renderHistoryTable()
/// In Java: ImGui table showing event descriptions with clipper.
fn render_history_table(ui: &mut egui::Ui) {
    let event_history = EVENT_HISTORY.lock().unwrap();
    let events = event_history.get_events();
    if events.is_empty() {
        ui.label("No history");
    } else {
        egui::ScrollArea::vertical()
            .max_height(ui.text_style_height(&egui::TextStyle::Body) * 20.0)
            .show(ui, |ui| {
                egui::Grid::new("history_table")
                    .striped(true)
                    .num_columns(1)
                    .show(ui, |ui| {
                        ui.strong("Description");
                        ui.end_row();
                        for (i, event) in events.iter().enumerate() {
                            ui.push_id(i, |ui| {
                                ui.label(event.get_description());
                            });
                            ui.end_row();
                        }
                    });
            });
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
            let name = event.get_name().to_string();
            self.target_name_to_events
                .entry(name)
                .or_default()
                .push(event.clone());
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dst(name: &str, x: f32, y: f32, w: f32, h: f32) -> SkinWidgetDestination {
        SkinWidgetDestination::new(
            name.to_string(),
            SkinObjectDestination {
                time: 0,
                region: Rectangle {
                    x,
                    y,
                    width: w,
                    height: h,
                },
                color: None,
                angle: 0.0,
                alpha: 0.0,
            },
        )
    }

    fn make_widget(name: &str, dsts: Vec<SkinWidgetDestination>) -> SkinWidget {
        SkinWidget::new(
            name.to_string(),
            SkinObject {
                name: Some(name.to_string()),
                draw: true,
                visible: true,
                destinations: vec![],
            },
            dsts,
        )
    }

    // ---- normalize_float tests ----

    #[test]
    fn test_normalize_float_integer_value() {
        assert_eq!(normalize_float(10.0), "10");
    }

    #[test]
    fn test_normalize_float_trailing_zeros() {
        assert_eq!(normalize_float(1.5), "1.5");
    }

    #[test]
    fn test_normalize_float_four_decimal_places() {
        assert_eq!(normalize_float(1.23456), "1.2346");
    }

    #[test]
    fn test_normalize_float_zero() {
        assert_eq!(normalize_float(0.0), "0");
    }

    #[test]
    fn test_normalize_float_negative() {
        assert_eq!(normalize_float(-3.14), "-3.14");
    }

    // ---- WidgetTableColumn static tests ----

    #[test]
    fn test_widget_table_columns_count() {
        let columns = WIDGET_TABLE_COLUMNS.lock().unwrap();
        assert_eq!(columns.len(), 6);
    }

    #[test]
    fn test_widget_table_columns_first_is_id_persistent() {
        let columns = WIDGET_TABLE_COLUMNS.lock().unwrap();
        assert_eq!(columns[0].name, "ID");
        assert!(columns[0].persistent);
        assert!(columns[0].getter.is_none());
    }

    #[test]
    fn test_widget_table_columns_last_is_operation_persistent() {
        let columns = WIDGET_TABLE_COLUMNS.lock().unwrap();
        let last = &columns[columns.len() - 1];
        assert_eq!(last.name, "Operation");
        assert!(last.persistent);
        assert!(last.getter.is_none());
    }

    #[test]
    fn test_widget_table_columns_xywh_have_getters() {
        let columns = WIDGET_TABLE_COLUMNS.lock().unwrap();
        // columns[1..5] are x, y, w, h
        for i in 1..5 {
            assert!(!columns[i].persistent);
            assert!(columns[i].getter.is_some());
            assert!(columns[i].change_event_type.is_some());
        }
    }

    #[test]
    fn test_widget_table_column_getters() {
        let columns = WIDGET_TABLE_COLUMNS.lock().unwrap();
        let dst = make_dst("test", 10.0, 20.0, 30.0, 40.0);

        let x_getter = columns[1].getter.unwrap();
        assert!((x_getter(&dst) - 10.0).abs() < f32::EPSILON);

        let y_getter = columns[2].getter.unwrap();
        assert!((y_getter(&dst) - 20.0).abs() < f32::EPSILON);

        let w_getter = columns[3].getter.unwrap();
        assert!((w_getter(&dst) - 30.0).abs() < f32::EPSILON);

        let h_getter = columns[4].getter.unwrap();
        assert!((h_getter(&dst) - 40.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_widget_table_showing_columns_filter() {
        let mut columns = WIDGET_TABLE_COLUMNS.lock().unwrap();
        // Hide x and y columns
        columns[1].show = false;
        columns[2].show = false;
        let showing: Vec<&WidgetTableColumn> = columns.iter().filter(|col| col.show).collect();
        // Should be: ID, w, h, Operation = 4
        assert_eq!(showing.len(), 4);
        assert_eq!(showing[0].name, "ID");
        assert_eq!(showing[1].name, "w");
        assert_eq!(showing[2].name, "h");
        assert_eq!(showing[3].name, "Operation");

        // Restore for other tests
        columns[1].show = true;
        columns[2].show = true;
    }

    // ---- EventHistory tests ----

    #[test]
    fn test_event_history_push_and_has_event() {
        let mut history = EventHistory::new();
        assert!(!history.has_event("dst1", &EventType::ChangeX));

        history.push_event(Event::ChangeSingleField {
            event_type: EventType::ChangeX,
            target_name: "dst1".to_string(),
            previous: 0.0,
            current: 10.0,
        });

        assert!(history.has_event("dst1", &EventType::ChangeX));
        assert!(!history.has_event("dst1", &EventType::ChangeY));
        assert!(!history.has_event("other", &EventType::ChangeX));
    }

    #[test]
    fn test_event_history_undo_with_widgets() {
        let mut history = EventHistory::new();
        let mut widgets = vec![make_widget(
            "w1",
            vec![make_dst("dst1", 0.0, 0.0, 0.0, 0.0)],
        )];

        // Push a change event
        history.push_event(Event::ChangeSingleField {
            event_type: EventType::ChangeX,
            target_name: "dst1".to_string(),
            previous: 0.0,
            current: 10.0,
        });
        // Simulate that x was actually changed to 10
        widgets[0].destinations[0].destination.region.x = 10.0;
        assert!((widgets[0].destinations[0].get_dst_x() - 10.0).abs() < f32::EPSILON);

        // Undo
        history.undo_n_with_widgets(1, &mut widgets);

        // x should be reverted to 0
        assert!((widgets[0].destinations[0].get_dst_x() - 0.0).abs() < f32::EPSILON);
        assert!(history.get_events().is_empty());
    }

    #[test]
    fn test_event_history_undo_toggle_visible() {
        let mut history = EventHistory::new();
        let mut widgets = vec![make_widget("w1", vec![])];
        assert!(widgets[0].skin_object.visible);

        // Push toggle event, toggling visible to false
        history.push_event(Event::ToggleVisible {
            event_type: EventType::ToggleVisible,
            target_name: "w1".to_string(),
            widget_index: 0,
            was_visible_before: true,
        });
        widgets[0].toggle_visible(); // now false

        assert!(!widgets[0].skin_object.visible);

        // Undo
        history.undo_n_with_widgets(1, &mut widgets);
        assert!(widgets[0].skin_object.visible);
    }

    #[test]
    fn test_event_history_clear() {
        let mut history = EventHistory::new();
        history.push_event(Event::ChangeSingleField {
            event_type: EventType::ChangeX,
            target_name: "dst1".to_string(),
            previous: 0.0,
            current: 5.0,
        });
        assert!(!history.get_events().is_empty());

        history.clear();
        assert!(history.get_events().is_empty());
        assert!(!history.has_event("dst1", &EventType::ChangeX));
    }

    #[test]
    fn test_event_history_get_events_by_name() {
        let mut history = EventHistory::new();
        history.push_event(Event::ChangeSingleField {
            event_type: EventType::ChangeX,
            target_name: "dst1".to_string(),
            previous: 0.0,
            current: 10.0,
        });
        history.push_event(Event::ChangeSingleField {
            event_type: EventType::ChangeY,
            target_name: "dst1".to_string(),
            previous: 0.0,
            current: 20.0,
        });
        history.push_event(Event::ChangeSingleField {
            event_type: EventType::ChangeX,
            target_name: "dst2".to_string(),
            previous: 0.0,
            current: 5.0,
        });

        let dst1_events = history.get_events_by_name("dst1");
        assert_eq!(dst1_events.len(), 2);

        let dst2_events = history.get_events_by_name("dst2");
        assert_eq!(dst2_events.len(), 1);

        let none_events = history.get_events_by_name("nonexistent");
        assert!(none_events.is_empty());
    }

    // ---- SkinWidget tests ----

    #[test]
    fn test_skin_widget_is_drawing_on_screen() {
        let mut widget = make_widget("test", vec![]);
        assert!(widget.is_drawing_on_screen());

        widget.skin_object.visible = false;
        assert!(!widget.is_drawing_on_screen());

        widget.skin_object.visible = true;
        widget.skin_object.draw = false;
        assert!(!widget.is_drawing_on_screen());
    }

    #[test]
    fn test_skin_widget_toggle_visible() {
        let mut widget = make_widget("test", vec![]);
        assert!(widget.skin_object.visible);

        widget.toggle_visible();
        assert!(!widget.skin_object.visible);

        widget.toggle_visible();
        assert!(widget.skin_object.visible);
    }

    // ---- Event description tests ----

    #[test]
    fn test_event_change_x_description() {
        let event = Event::ChangeSingleField {
            event_type: EventType::ChangeX,
            target_name: "dst1".to_string(),
            previous: 0.0,
            current: 10.0,
        };
        assert_eq!(
            event.get_description(),
            "Changed dst1's x from 0.0000 to 10.0000"
        );
    }

    #[test]
    fn test_event_toggle_visible_description() {
        let event = Event::ToggleVisible {
            event_type: EventType::ToggleVisible,
            target_name: "widget1".to_string(),
            widget_index: 0,
            was_visible_before: true,
        };
        assert_eq!(event.get_description(), "Make widget1 widget invisible");

        let event2 = Event::ToggleVisible {
            event_type: EventType::ToggleVisible,
            target_name: "widget1".to_string(),
            widget_index: 0,
            was_visible_before: false,
        };
        assert_eq!(event2.get_description(), "Make widget1 widget visible");
    }

    // ---- SkinWidgetDestination tests ----

    #[test]
    fn test_destination_getters() {
        let dst = make_dst("test", 1.0, 2.0, 3.0, 4.0);
        assert!((dst.get_dst_x() - 1.0).abs() < f32::EPSILON);
        assert!((dst.get_dst_y() - 2.0).abs() < f32::EPSILON);
        assert!((dst.get_dst_w() - 3.0).abs() < f32::EPSILON);
        assert!((dst.get_dst_h() - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_destination_moving_state_initial() {
        let dst = make_dst("test", 0.0, 0.0, 0.0, 0.0);
        assert_eq!(dst.moving_state, 0);
        assert!(dst.before_move.is_none());
    }
}

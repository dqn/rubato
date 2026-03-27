use super::super::imgui_notify::ImGuiNotify;
use super::super::imgui_renderer;
use super::super::{Clipboard, ImFloat, Rectangle, SkinObjectDestination};
use super::{
    EDITING_WIDGET_H, EDITING_WIDGET_W, EDITING_WIDGET_X, EDITING_WIDGET_Y, EVENT_HISTORY, Event,
    EventType, MOVE_OVERLAY_ENABLED, RESET_MOVE_OVERLAY, SkinWidget, SkinWidgetDestination,
    WIDGET_TABLE_COLUMNS, WIDGETS, WidgetTableColumn,
};
use rubato_types::sync_utils::lock_or_recover;

pub(super) fn render_prefer_column_setting(ui: &mut egui::Ui) {
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
            let mut columns = lock_or_recover(&WIDGET_TABLE_COLUMNS);
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
pub(super) fn render_skin_widgets_table(ui: &mut egui::Ui, widgets: &mut [SkinWidget]) {
    // NOTE: This will create a snapshot for us, which can kinda prevent us step into race condition
    let columns = lock_or_recover(&WIDGET_TABLE_COLUMNS);
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

    let columns = lock_or_recover(&WIDGET_TABLE_COLUMNS);
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
                            let event_history = lock_or_recover(&EVENT_HISTORY);
                            let was_visible = widget.skin_object.visible;
                            drop(event_history);
                            if ui.button("Toggle").clicked() {
                                let mut event_history = lock_or_recover(&EVENT_HISTORY);
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
                                let event_history = lock_or_recover(&EVENT_HISTORY);
                                let columns_ref = lock_or_recover(&WIDGET_TABLE_COLUMNS);
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
                                        *lock_or_recover(&EDITING_WIDGET_X) =
                                            ImFloat { value: dst.dst_x() };
                                        *lock_or_recover(&EDITING_WIDGET_Y) =
                                            ImFloat { value: dst.dst_y() };
                                        *lock_or_recover(&EDITING_WIDGET_W) =
                                            ImFloat { value: dst.dst_w() };
                                        *lock_or_recover(&EDITING_WIDGET_H) =
                                            ImFloat { value: dst.dst_h() };
                                        *lock_or_recover(&RESET_MOVE_OVERLAY) = true;
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
pub(super) fn render_edit_popup(
    ui: &mut egui::Ui,
    dst: &mut SkinWidgetDestination,
    _dst_id: egui::Id,
) {
    ui.label("Edit Skin Widget");
    ui.separator();

    let mut x = lock_or_recover(&EDITING_WIDGET_X);
    ui.horizontal(|ui| {
        ui.label("x");
        ui.add(egui::DragValue::new(&mut x.value).speed(1.0));
    });
    drop(x);

    let mut y = lock_or_recover(&EDITING_WIDGET_Y);
    ui.horizontal(|ui| {
        ui.label("y");
        ui.add(egui::DragValue::new(&mut y.value).speed(1.0));
    });
    drop(y);

    let mut w = lock_or_recover(&EDITING_WIDGET_W);
    ui.horizontal(|ui| {
        ui.label("w");
        ui.add(egui::DragValue::new(&mut w.value).speed(1.0));
    });
    drop(w);

    let mut h = lock_or_recover(&EDITING_WIDGET_H);
    ui.horizontal(|ui| {
        ui.label("h");
        ui.add(egui::DragValue::new(&mut h.value).speed(1.0));
    });
    drop(h);

    if ui.button("Submit").clicked() {
        dst.set_dst_x(lock_or_recover(&EDITING_WIDGET_X).value);
        dst.set_dst_y(lock_or_recover(&EDITING_WIDGET_Y).value);
        dst.set_dst_w(lock_or_recover(&EDITING_WIDGET_W).value);
        dst.set_dst_h(lock_or_recover(&EDITING_WIDGET_H).value);
    }

    // Move overlay checkbox
    let mut move_enabled = lock_or_recover(&MOVE_OVERLAY_ENABLED);
    let old_move = move_enabled.value;
    ui.checkbox(&mut move_enabled.value, "Move");
    let just_enabled = move_enabled.value && !old_move;
    let reset = *lock_or_recover(&RESET_MOVE_OVERLAY);

    if just_enabled || reset {
        *lock_or_recover(&RESET_MOVE_OVERLAY) = false;
        // Position would be set via ImGui.setNextWindowPos/Size in Java
        // In egui, the overlay window position is set when creating the Area below
    }

    if move_enabled.value {
        if dst.moving_state == 0 {
            let cloned_region = Rectangle {
                x: dst.dst_x(),
                y: dst.dst_y(),
                width: dst.dst_w(),
                height: dst.dst_h(),
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
pub(super) fn render_move_overlay(ui: &mut egui::Ui, dst: &mut SkinWidgetDestination) {
    let window_height = imgui_renderer::window_height() as f32;
    let w = dst.dst_w();
    let h = dst.dst_h();
    let x = dst.dst_x();
    let y = window_height - dst.dst_y() - h;

    let move_enabled = lock_or_recover(&MOVE_OVERLAY_ENABLED);

    let resp = egui::Window::new("widget-overlay-popup")
        .default_pos(egui::pos2(x, y))
        .default_size(egui::vec2(w.max(100.0), h.max(40.0)))
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
            ui.label(format!("x = {:.1} y = {:.1}", dst.dst_x(), dst.dst_y()));
            ui.label(format!("w = {:.1} h = {:.1}", w, h));
        });

    // Read back actual window position after drag and convert screen-space to skin-space.
    if let Some(inner) = resp {
        let new_pos = inner.response.rect.min;
        let new_skin_x = new_pos.x;
        let new_skin_y = window_height - new_pos.y - h;
        dst.set_dst_x_with_event(new_skin_x, false);
        dst.set_dst_y_with_event(new_skin_y, false);
        dst.set_dst_w_with_event(w, false);
        dst.set_dst_h_with_event(h, false);
    }

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
pub(super) fn render_history_table(ui: &mut egui::Ui) {
    let event_history = lock_or_recover(&EVENT_HISTORY);
    let events = event_history.events();
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
                                ui.label(event.description());
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
pub(super) fn draw_float_value_column(
    ui: &mut egui::Ui,
    _index: usize,
    modified: bool,
    value: f32,
) {
    let text = normalize_float(value);
    if modified {
        ui.colored_label(egui::Color32::RED, text);
    } else {
        ui.label(text);
    }
}

pub(super) fn export_changes() {
    let widgets = lock_or_recover(&WIDGETS);
    let event_history = lock_or_recover(&EVENT_HISTORY);
    let mut changes: Vec<String> = Vec::new();

    for widget in widgets.iter() {
        for dst in &widget.destinations {
            let mut has_changed_x = false;
            let mut has_changed_y = false;
            let mut has_changed_w = false;
            let mut has_changed_h = false;

            for event in event_history.events_by_name(&dst.name) {
                match event.event_type() {
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
                sb.push_str(&format!(", x={}", dst.dst_x()));
            }
            if has_changed_y {
                sb.push_str(&format!(", y={}", dst.dst_y()));
            }
            if has_changed_x {
                sb.push_str(&format!(", w={}", dst.dst_w()));
            }
            if has_changed_y {
                sb.push_str(&format!(", h={}", dst.dst_h()));
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

pub(super) fn normalize_float(value: f32) -> String {
    // DecimalFormat("#.####")
    let formatted = format!("{:.4}", value);
    // Trim trailing zeros
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    trimmed.to_string()
}

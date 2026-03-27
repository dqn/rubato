use super::imgui_renderer;
use super::{ImBoolean, ImFloat, Skin};
#[cfg(test)]
use super::{Rectangle, SkinObject, SkinObjectDestination};

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use rubato_types::sync_utils::lock_or_recover;

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
            getter: Some(SkinWidgetDestination::dst_x),
            change_event_type: Some(EventType::ChangeX),
        },
        WidgetTableColumn {
            name: "y",
            show: true,
            persistent: false,
            getter: Some(SkinWidgetDestination::dst_y),
            change_event_type: Some(EventType::ChangeY),
        },
        WidgetTableColumn {
            name: "w",
            show: true,
            persistent: false,
            getter: Some(SkinWidgetDestination::dst_w),
            change_event_type: Some(EventType::ChangeW),
        },
        WidgetTableColumn {
            name: "h",
            show: true,
            persistent: false,
            getter: Some(SkinWidgetDestination::dst_h),
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
    pub fn focus() -> bool {
        rubato_types::skin_widget_focus::focus()
    }

    pub fn set_focus(focus: bool) {
        rubato_types::skin_widget_focus::set_focus(focus);
    }

    pub fn change_skin(skin: &Skin) {
        let _lock = lock_or_recover(&LOCK);
        let mut widgets = lock_or_recover(&WIDGETS);
        let mut event_history = lock_or_recover(&EVENT_HISTORY);
        widgets.clear();
        event_history.clear();

        let all_skin_objects = skin.all_skin_objects();
        // NOTE: We're using skin object's name as id, we need to keep name is unique
        let mut duplicated_skin_object_name_count: HashMap<String, i32> = HashMap::new();

        for skin_object in all_skin_objects {
            let skin_object_name = skin_object.name().map(|s| s.to_string());
            let dsts = skin_object.all_destination();
            let mut destinations: Vec<SkinWidgetDestination> = Vec::new();

            for (i, dst) in dsts.iter().enumerate() {
                let dst_base_name = skin_object_name.as_deref().unwrap_or("Unnamed Destination");
                let combined_name = if dsts.len() == 1 {
                    dst_base_name.to_string()
                } else {
                    format!("{}({})", dst_base_name, i)
                };
                destinations.push(SkinWidgetDestination::new(combined_name, dst.clone()));
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
        let _lock = lock_or_recover(&LOCK);
        let mut open = true;
        egui::Window::new("Skin Widgets")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                let mut widgets = lock_or_recover(&WIDGETS);
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
                                let mut event_history = lock_or_recover(&EVENT_HISTORY);
                                event_history.undo_with_widgets(&mut widgets);
                            }
                            render_prefer_column_setting(ui);
                            let mut show_cursor = lock_or_recover(&SHOW_CURSOR_POSITION);
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
                    let show_cursor = lock_or_recover(&SHOW_CURSOR_POSITION);
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

mod render_fns;
mod widget_types;

use render_fns::*;
pub use widget_types::*;

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
        assert_eq!(normalize_float(-2.5), "-2.5");
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
        assert!((widgets[0].destinations[0].dst_x() - 10.0).abs() < f32::EPSILON);

        // Undo
        history.undo_n_with_widgets(1, &mut widgets);

        // x should be reverted to 0
        assert!((widgets[0].destinations[0].dst_x() - 0.0).abs() < f32::EPSILON);
        assert!(history.events().is_empty());
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
        assert!(!history.events().is_empty());

        history.clear();
        assert!(history.events().is_empty());
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

        let dst1_events = history.events_by_name("dst1");
        assert_eq!(dst1_events.len(), 2);

        let dst2_events = history.events_by_name("dst2");
        assert_eq!(dst2_events.len(), 1);

        let none_events = history.events_by_name("nonexistent");
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
            event.description(),
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
        assert_eq!(event.description(), "Make widget1 widget invisible");

        let event2 = Event::ToggleVisible {
            event_type: EventType::ToggleVisible,
            target_name: "widget1".to_string(),
            widget_index: 0,
            was_visible_before: false,
        };
        assert_eq!(event2.description(), "Make widget1 widget visible");
    }

    // ---- SkinWidgetDestination tests ----

    #[test]
    fn test_destination_getters() {
        let dst = make_dst("test", 1.0, 2.0, 3.0, 4.0);
        assert!((dst.dst_x() - 1.0).abs() < f32::EPSILON);
        assert!((dst.dst_y() - 2.0).abs() < f32::EPSILON);
        assert!((dst.dst_w() - 3.0).abs() < f32::EPSILON);
        assert!((dst.dst_h() - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_destination_moving_state_initial() {
        let dst = make_dst("test", 0.0, 0.0, 0.0, 0.0);
        assert_eq!(dst.moving_state, 0);
        assert!(dst.before_move.is_none());
    }
}

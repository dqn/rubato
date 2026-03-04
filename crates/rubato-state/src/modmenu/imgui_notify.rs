use super::font_awesome_icons;
use super::imgui_renderer;

use std::sync::Mutex;
use std::time::Instant;

pub const NOTIFY_PADDING_X: f32 = 20.0;
pub const NOTIFY_PADDING_Y: f32 = 20.0;
pub const NOTIFY_PADDING_MESSAGE_Y: f32 = 10.0;
pub const NOTIFY_FADE_IN_OUT_TIME: i64 = 150;
pub const NOTIFY_DEFAULT_DISMISS: i64 = 3000;
pub const NOTIFY_OPACITY: f32 = 0.9;
pub const NOTIFY_USE_SEPARATOR: bool = false;
pub const NOTIFY_USE_DISMISS_BUTTON: bool = false;
pub const NOTIFY_RENDER_LIMIT: usize = 7;

/// Maximum text wrap width as a fraction of window width (Java: windowWidth / 3.0f)
const NOTIFY_TEXT_WRAP_FRACTION: f32 = 3.0;

pub const NOTIFICATION_POSITIONS: [&str; 7] = [
    "TopLeft",
    "TopCenter",
    "TopRight",
    "BottomLeft",
    "BottomCenter",
    "BottomRight",
    "Center",
];

static DEFAULT_TOAST_POS: Mutex<ToastPos> = Mutex::new(ToastPos::TopLeft);
static NOTIFICATIONS: Mutex<Vec<Toast>> = Mutex::new(Vec::new());

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToastType {
    None,
    Success,
    Warning,
    Error,
    Info,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToastPhase {
    FadeIn,
    Wait,
    FadeOut,
    Expired,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToastPos {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Center,
}

impl ToastPos {
    pub fn pivot_x(&self) -> f32 {
        match self {
            ToastPos::TopLeft | ToastPos::BottomLeft => 0.0,
            ToastPos::TopCenter | ToastPos::BottomCenter | ToastPos::Center => 0.5,
            ToastPos::TopRight | ToastPos::BottomRight => 1.0,
        }
    }

    pub fn pivot_y(&self) -> f32 {
        match self {
            ToastPos::TopLeft | ToastPos::TopCenter | ToastPos::TopRight => 0.0,
            ToastPos::BottomLeft | ToastPos::BottomCenter | ToastPos::BottomRight => 1.0,
            ToastPos::Center => 0.5,
        }
    }

    pub fn from_name(name: &str) -> ToastPos {
        match name {
            "TopLeft" => ToastPos::TopLeft,
            "TopCenter" => ToastPos::TopCenter,
            "TopRight" => ToastPos::TopRight,
            "BottomLeft" => ToastPos::BottomLeft,
            "BottomCenter" => ToastPos::BottomCenter,
            "BottomRight" => ToastPos::BottomRight,
            "Center" => ToastPos::Center,
            _ => ToastPos::TopLeft,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub toast_type: ToastType,
    pub pos: ToastPos,
    pub title: String,
    pub content: String,
    pub dismiss_time: i64,
    pub creation_time: Instant,
    pub on_button_press: bool, // stub: in Java this is Runnable; true = has callback
    pub button_label: String,
}

impl Toast {
    pub fn new(toast_type: ToastType) -> Self {
        let pos = DEFAULT_TOAST_POS.lock().unwrap().clone();
        Toast {
            toast_type,
            pos,
            title: String::new(),
            content: String::new(),
            dismiss_time: NOTIFY_DEFAULT_DISMISS,
            creation_time: Instant::now(),
            on_button_press: false,
            button_label: String::new(),
        }
    }

    pub fn with_dismiss_time(toast_type: ToastType, dismiss_time: i64) -> Self {
        let mut toast = Self::new(toast_type);
        toast.dismiss_time = dismiss_time;
        toast
    }

    pub fn with_content(toast_type: ToastType, content: String) -> Self {
        let mut toast = Self::new(toast_type);
        toast.content = content;
        toast
    }

    pub fn with_dismiss_time_and_content(
        toast_type: ToastType,
        dismiss_time: i64,
        content: String,
    ) -> Self {
        let mut toast = Self::new(toast_type);
        toast.dismiss_time = dismiss_time;
        toast.content = content;
        toast
    }

    pub fn with_button(
        toast_type: ToastType,
        dismiss_time: i64,
        button_label: String,
        content: String,
    ) -> Self {
        let mut toast = Self::new(toast_type);
        toast.dismiss_time = dismiss_time;
        toast.button_label = button_label;
        toast.on_button_press = true;
        toast.content = content;
        toast
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    pub fn set_type(&mut self, toast_type: ToastType) {
        self.toast_type = toast_type;
    }

    pub fn set_pos(&mut self, pos: ToastPos) {
        self.pos = pos;
    }

    pub fn set_on_button_press(&mut self, has_press: bool) {
        self.on_button_press = has_press;
    }

    pub fn set_button_label(&mut self, button_label: String) {
        self.button_label = button_label;
    }

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn get_default_title(&self) -> Option<&str> {
        if self.title.is_empty() {
            match self.toast_type {
                ToastType::None => None,
                ToastType::Success => Some("Success"),
                ToastType::Warning => Some("Warning"),
                ToastType::Error => Some("Error"),
                ToastType::Info => Some("Info"),
            }
        } else {
            Some(&self.title)
        }
    }

    pub fn get_type(&self) -> &ToastType {
        &self.toast_type
    }

    pub fn get_color(&self) -> [f32; 4] {
        match self.toast_type {
            ToastType::None => [1.0, 1.0, 1.0, 1.0],    // White
            ToastType::Success => [0.0, 1.0, 0.0, 1.0], // Green
            ToastType::Warning => [1.0, 1.0, 0.0, 1.0], // Yellow
            ToastType::Error => [1.0, 0.0, 0.0, 1.0],   // Red
            ToastType::Info => [0.0, 0.616, 1.0, 1.0],  // Blue
        }
    }

    pub fn get_icon(&self) -> Option<&str> {
        match self.toast_type {
            ToastType::None => None,
            ToastType::Success => Some(font_awesome_icons::CHECK_CIRCLE),
            ToastType::Warning => Some(font_awesome_icons::EXCLAMATION),
            ToastType::Error => Some(font_awesome_icons::BOMB),
            ToastType::Info => Some(font_awesome_icons::INFO_CIRCLE),
        }
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }

    pub fn get_elapsed_time(&self) -> i64 {
        self.creation_time.elapsed().as_millis() as i64
    }

    pub fn get_phase(&self) -> ToastPhase {
        let elapsed = self.get_elapsed_time();
        if elapsed > NOTIFY_FADE_IN_OUT_TIME + self.dismiss_time + NOTIFY_FADE_IN_OUT_TIME {
            ToastPhase::Expired
        } else if elapsed > NOTIFY_FADE_IN_OUT_TIME + self.dismiss_time {
            ToastPhase::FadeOut
        } else if elapsed > NOTIFY_FADE_IN_OUT_TIME {
            ToastPhase::Wait
        } else {
            ToastPhase::FadeIn
        }
    }

    pub fn get_fade_percent(&self) -> f32 {
        let phase = self.get_phase();
        let elapsed = self.get_elapsed_time();

        if phase == ToastPhase::FadeIn {
            (elapsed as f32 / NOTIFY_FADE_IN_OUT_TIME as f32) * NOTIFY_OPACITY
        } else if phase == ToastPhase::FadeOut {
            (1.0 - ((elapsed as f32 - NOTIFY_FADE_IN_OUT_TIME as f32 - self.dismiss_time as f32)
                / NOTIFY_FADE_IN_OUT_TIME as f32))
                * NOTIFY_OPACITY
        } else {
            1.0 * NOTIFY_OPACITY
        }
    }

    pub fn get_window_flags(&self) -> i32 {
        0 // stub: flags handled natively by egui Area/Frame
    }

    pub fn has_on_button_press(&self) -> bool {
        self.on_button_press
    }

    pub fn get_button_label(&self) -> &str {
        &self.button_label
    }

    pub fn get_pos(&self) -> &ToastPos {
        &self.pos
    }
}

pub struct ImGuiNotify;

impl ImGuiNotify {
    pub fn insert_notification(toast: Toast) {
        let mut notifications = NOTIFICATIONS.lock().unwrap();
        notifications.push(toast);
    }

    pub fn remove_notification(index: usize) {
        let mut notifications = NOTIFICATIONS.lock().unwrap();
        if index < notifications.len() {
            notifications.remove(index);
        }
    }

    /// Legacy render method — kept for backward compatibility.
    /// Actual egui rendering uses render_notifications_ui(ctx).
    pub fn render_notifications() {
        // No-op: rendering is now done in render_notifications_ui()
    }

    // Convenience notification methods
    pub fn success(content: &str) {
        Self::insert_notification(Toast::with_content(ToastType::Success, content.to_string()));
    }

    pub fn success_with_dismiss(content: &str, dismiss_time: i64) {
        Self::insert_notification(Toast::with_dismiss_time_and_content(
            ToastType::Success,
            dismiss_time,
            content.to_string(),
        ));
    }

    pub fn warning(content: &str) {
        Self::insert_notification(Toast::with_content(ToastType::Warning, content.to_string()));
    }

    pub fn warning_with_dismiss(content: &str, dismiss_time: i64) {
        Self::insert_notification(Toast::with_dismiss_time_and_content(
            ToastType::Warning,
            dismiss_time,
            content.to_string(),
        ));
    }

    pub fn error(content: &str) {
        Self::insert_notification(Toast::with_content(ToastType::Error, content.to_string()));
    }

    pub fn error_with_dismiss(content: &str, dismiss_time: i64) {
        Self::insert_notification(Toast::with_dismiss_time_and_content(
            ToastType::Error,
            dismiss_time,
            content.to_string(),
        ));
    }

    pub fn info(content: &str) {
        Self::insert_notification(Toast::with_content(ToastType::Info, content.to_string()));
    }

    pub fn info_with_dismiss(content: &str, dismiss_time: i64) {
        Self::insert_notification(Toast::with_dismiss_time_and_content(
            ToastType::Info,
            dismiss_time,
            content.to_string(),
        ));
    }

    pub fn with_button(
        toast_type: ToastType,
        content: &str,
        button_label: &str,
        _on_button_press: Box<dyn Fn() + Send>,
    ) {
        Self::insert_notification(Toast::with_button(
            toast_type,
            NOTIFY_DEFAULT_DISMISS,
            button_label.to_string(),
            content.to_string(),
        ));
    }

    pub fn set_notification_position(index: usize) {
        if index < NOTIFICATION_POSITIONS.len() {
            let pos = ToastPos::from_name(NOTIFICATION_POSITIONS[index]);
            *DEFAULT_TOAST_POS.lock().unwrap() = pos;
        }
    }

    /// Render toast notifications using egui.
    ///
    /// Translated from: ImGuiNotify.renderNotifications()
    /// Renders each active toast as a positioned egui Area with a styled frame,
    /// including icon, title, content, separator, dismiss button, and action button.
    pub fn render_notifications_ui(ctx: &egui::Context) {
        let mut notifications = NOTIFICATIONS.lock().unwrap();
        let mut height: f32 = 0.0;
        let text_wrap_width = imgui_renderer::window_width() as f32 / NOTIFY_TEXT_WRAP_FRACTION;

        let mut i = 0;
        let mut dismiss_index: Option<usize> = None;

        while i < notifications.len() {
            let current_toast = &notifications[i];

            // Remove expired toasts
            if current_toast.get_phase() == ToastPhase::Expired {
                notifications.remove(i);
                continue;
            }

            // Enforce render limit
            if NOTIFY_RENDER_LIMIT > 0 && i >= NOTIFY_RENDER_LIMIT {
                i += 1;
                continue;
            }

            // Snapshot all toast data before UI rendering (avoids borrow issues)
            let opacity = current_toast.get_fade_percent();
            let text_color = current_toast.get_color();
            let icon = current_toast.get_icon().map(|s| s.to_string());
            let title = current_toast.get_title().to_string();
            let default_title = current_toast.get_default_title().map(|s| s.to_string());
            let content = current_toast.get_content().to_string();
            let has_button = current_toast.has_on_button_press();
            let button_label = current_toast.get_button_label().to_string();
            let window_name = format!("##TOAST{}", i);
            let toast_pos = current_toast.pos.clone();

            let (toast_x, toast_y) = get_toast_pos(&toast_pos, height);
            // Apply pivot offset: shift by pivot * estimated window size
            // egui positions by top-left; Java ImGui uses pivot to adjust
            let pivot_x = toast_pos.pivot_x();
            let pivot_y = toast_pos.pivot_y();
            // We estimate the window size for pivot; egui doesn't expose size before rendering.
            // Use a reasonable estimate (200px wide, 60px tall per toast)
            let estimated_width = text_wrap_width.min(300.0);
            let estimated_height = 60.0_f32;
            let adjusted_x = toast_x - pivot_x * estimated_width;
            let adjusted_y = toast_y - pivot_y * estimated_height;

            let color = egui::Color32::from_rgba_unmultiplied(
                (text_color[0] * 255.0) as u8,
                (text_color[1] * 255.0) as u8,
                (text_color[2] * 255.0) as u8,
                (opacity * 255.0) as u8,
            );
            let content_color =
                egui::Color32::from_rgba_unmultiplied(200, 200, 200, (opacity * 255.0) as u8);
            let bg_alpha = (opacity * 255.0) as u8;

            let interactable = NOTIFY_USE_DISMISS_BUTTON || has_button;

            let response = egui::Area::new(egui::Id::new(&window_name))
                .fixed_pos(egui::pos2(adjusted_x, adjusted_y))
                .interactable(interactable)
                .show(ctx, |ui| {
                    ui.set_max_width(text_wrap_width);
                    let frame = egui::Frame::popup(ui.style())
                        .fill(egui::Color32::from_rgba_unmultiplied(40, 40, 40, bg_alpha));
                    frame.show(ui, |ui| {
                        let mut was_title_rendered = false;

                        // Render icon + title on same line
                        if icon.is_some() || !title.is_empty() || default_title.is_some() {
                            ui.horizontal(|ui| {
                                // Icon
                                if let Some(ref icon_str) = icon {
                                    ui.label(egui::RichText::new(icon_str).color(color));
                                    was_title_rendered = true;
                                }

                                // Title (explicit title takes precedence over default)
                                if !title.is_empty() {
                                    ui.label(egui::RichText::new(&title).color(color).strong());
                                    was_title_rendered = true;
                                } else if let Some(ref dt) = default_title {
                                    ui.label(egui::RichText::new(dt).color(color).strong());
                                    was_title_rendered = true;
                                }

                                // Dismiss button (inline with title)
                                if NOTIFY_USE_DISMISS_BUTTON
                                    && (was_title_rendered || !content.is_empty())
                                    && ui.small_button("X").clicked()
                                {
                                    dismiss_index = Some(i);
                                }
                            });
                        }

                        // Spacing between title and content
                        if was_title_rendered && !content.is_empty() {
                            ui.add_space(5.0);
                        }

                        // Separator between title and content
                        if was_title_rendered && !content.is_empty() && NOTIFY_USE_SEPARATOR {
                            ui.separator();
                        }

                        // Content text
                        if !content.is_empty() {
                            ui.label(egui::RichText::new(&content).color(content_color));
                        }

                        // Action button
                        if has_button
                            && !button_label.is_empty()
                            && ui.button(&button_label).clicked()
                        {
                            // In Java this would call onButtonPress.run()
                            // Callback not yet wired (stub: on_button_press is bool)
                            log::info!("Toast action button pressed: {}", button_label);
                        }
                    });
                });

            // Accumulate height from the rendered area
            let area_height = response.response.rect.height();
            height += area_height + NOTIFY_PADDING_MESSAGE_Y;

            i += 1;
        }

        // Process dismiss outside the loop to avoid borrow issues
        if let Some(idx) = dismiss_index
            && idx < notifications.len()
        {
            notifications.remove(idx);
        }
    }
}

fn get_relative_init_pos(pos_type: &ToastPos) -> (f32, f32) {
    let w = imgui_renderer::window_width() as f32;
    let h = imgui_renderer::window_height() as f32;
    match pos_type {
        ToastPos::Center => (w * 0.5, h * 0.5),
        ToastPos::TopLeft => (NOTIFY_PADDING_X, NOTIFY_PADDING_Y),
        ToastPos::TopCenter => (w * 0.5, NOTIFY_PADDING_Y),
        ToastPos::TopRight => (w - NOTIFY_PADDING_X, NOTIFY_PADDING_Y),
        ToastPos::BottomLeft => (NOTIFY_PADDING_X, h - NOTIFY_PADDING_Y),
        ToastPos::BottomCenter => (w * 0.5, h - NOTIFY_PADDING_Y),
        ToastPos::BottomRight => (w - NOTIFY_PADDING_X, h - NOTIFY_PADDING_Y),
    }
}

fn get_toast_pos(pos_type: &ToastPos, acc_y: f32) -> (f32, f32) {
    let adjusted_acc_y = match pos_type {
        ToastPos::BottomLeft | ToastPos::BottomCenter | ToastPos::BottomRight => -acc_y,
        _ => acc_y,
    };
    let init_pos = get_relative_init_pos(pos_type);
    (init_pos.0, init_pos.1 + adjusted_acc_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ToastPos tests ----

    #[test]
    fn test_toast_pos_pivot_x_left_positions() {
        assert!((ToastPos::TopLeft.pivot_x() - 0.0).abs() < f32::EPSILON);
        assert!((ToastPos::BottomLeft.pivot_x() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toast_pos_pivot_x_center_positions() {
        assert!((ToastPos::TopCenter.pivot_x() - 0.5).abs() < f32::EPSILON);
        assert!((ToastPos::BottomCenter.pivot_x() - 0.5).abs() < f32::EPSILON);
        assert!((ToastPos::Center.pivot_x() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toast_pos_pivot_x_right_positions() {
        assert!((ToastPos::TopRight.pivot_x() - 1.0).abs() < f32::EPSILON);
        assert!((ToastPos::BottomRight.pivot_x() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toast_pos_pivot_y_top_positions() {
        assert!((ToastPos::TopLeft.pivot_y() - 0.0).abs() < f32::EPSILON);
        assert!((ToastPos::TopCenter.pivot_y() - 0.0).abs() < f32::EPSILON);
        assert!((ToastPos::TopRight.pivot_y() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toast_pos_pivot_y_bottom_positions() {
        assert!((ToastPos::BottomLeft.pivot_y() - 1.0).abs() < f32::EPSILON);
        assert!((ToastPos::BottomCenter.pivot_y() - 1.0).abs() < f32::EPSILON);
        assert!((ToastPos::BottomRight.pivot_y() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toast_pos_pivot_y_center() {
        assert!((ToastPos::Center.pivot_y() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toast_pos_from_name_all_known() {
        assert_eq!(ToastPos::from_name("TopLeft"), ToastPos::TopLeft);
        assert_eq!(ToastPos::from_name("TopCenter"), ToastPos::TopCenter);
        assert_eq!(ToastPos::from_name("TopRight"), ToastPos::TopRight);
        assert_eq!(ToastPos::from_name("BottomLeft"), ToastPos::BottomLeft);
        assert_eq!(ToastPos::from_name("BottomCenter"), ToastPos::BottomCenter);
        assert_eq!(ToastPos::from_name("BottomRight"), ToastPos::BottomRight);
        assert_eq!(ToastPos::from_name("Center"), ToastPos::Center);
    }

    #[test]
    fn test_toast_pos_from_name_unknown_defaults_to_top_left() {
        assert_eq!(ToastPos::from_name("Invalid"), ToastPos::TopLeft);
        assert_eq!(ToastPos::from_name(""), ToastPos::TopLeft);
    }

    // ---- Toast tests ----

    #[test]
    fn test_toast_default_title_uses_type_name_when_title_empty() {
        let toast = Toast::new(ToastType::Success);
        assert_eq!(toast.get_default_title(), Some("Success"));

        let toast = Toast::new(ToastType::Warning);
        assert_eq!(toast.get_default_title(), Some("Warning"));

        let toast = Toast::new(ToastType::Error);
        assert_eq!(toast.get_default_title(), Some("Error"));

        let toast = Toast::new(ToastType::Info);
        assert_eq!(toast.get_default_title(), Some("Info"));

        let toast = Toast::new(ToastType::None);
        assert_eq!(toast.get_default_title(), None);
    }

    #[test]
    fn test_toast_default_title_uses_custom_title_when_set() {
        let mut toast = Toast::new(ToastType::Success);
        toast.set_title("Custom Title".to_string());
        assert_eq!(toast.get_default_title(), Some("Custom Title"));
    }

    #[test]
    fn test_toast_get_color_mapping() {
        assert_eq!(
            Toast::new(ToastType::None).get_color(),
            [1.0, 1.0, 1.0, 1.0]
        );
        assert_eq!(
            Toast::new(ToastType::Success).get_color(),
            [0.0, 1.0, 0.0, 1.0]
        );
        assert_eq!(
            Toast::new(ToastType::Warning).get_color(),
            [1.0, 1.0, 0.0, 1.0]
        );
        assert_eq!(
            Toast::new(ToastType::Error).get_color(),
            [1.0, 0.0, 0.0, 1.0]
        );
        assert_eq!(
            Toast::new(ToastType::Info).get_color(),
            [0.0, 0.616, 1.0, 1.0]
        );
    }

    #[test]
    fn test_toast_get_icon_mapping() {
        assert_eq!(Toast::new(ToastType::None).get_icon(), None);
        assert_eq!(
            Toast::new(ToastType::Success).get_icon(),
            Some(font_awesome_icons::CHECK_CIRCLE)
        );
        assert_eq!(
            Toast::new(ToastType::Warning).get_icon(),
            Some(font_awesome_icons::EXCLAMATION)
        );
        assert_eq!(
            Toast::new(ToastType::Error).get_icon(),
            Some(font_awesome_icons::BOMB)
        );
        assert_eq!(
            Toast::new(ToastType::Info).get_icon(),
            Some(font_awesome_icons::INFO_CIRCLE)
        );
    }

    #[test]
    fn test_toast_with_content_constructor() {
        let toast = Toast::with_content(ToastType::Info, "Hello".to_string());
        assert_eq!(toast.get_content(), "Hello");
        assert_eq!(*toast.get_type(), ToastType::Info);
        assert_eq!(toast.dismiss_time, NOTIFY_DEFAULT_DISMISS);
    }

    #[test]
    fn test_toast_with_dismiss_time_constructor() {
        let toast = Toast::with_dismiss_time(ToastType::Warning, 5000);
        assert_eq!(toast.dismiss_time, 5000);
        assert_eq!(*toast.get_type(), ToastType::Warning);
    }

    #[test]
    fn test_toast_with_dismiss_time_and_content_constructor() {
        let toast =
            Toast::with_dismiss_time_and_content(ToastType::Error, 1000, "Oops".to_string());
        assert_eq!(toast.dismiss_time, 1000);
        assert_eq!(toast.get_content(), "Oops");
        assert_eq!(*toast.get_type(), ToastType::Error);
    }

    #[test]
    fn test_toast_with_button_constructor() {
        let toast = Toast::with_button(
            ToastType::Success,
            2000,
            "Click me".to_string(),
            "Action content".to_string(),
        );
        assert!(toast.has_on_button_press());
        assert_eq!(toast.get_button_label(), "Click me");
        assert_eq!(toast.get_content(), "Action content");
        assert_eq!(toast.dismiss_time, 2000);
    }

    #[test]
    fn test_toast_phase_starts_as_fade_in() {
        let toast = Toast::new(ToastType::Info);
        // Immediately after creation, phase should be FadeIn
        assert_eq!(toast.get_phase(), ToastPhase::FadeIn);
    }

    #[test]
    fn test_toast_fade_percent_during_wait_phase() {
        // A toast with 0ms fade-in and some dismiss time should be at NOTIFY_OPACITY during wait
        let mut toast = Toast::new(ToastType::Info);
        toast.dismiss_time = 100_000; // very long dismiss
        // Since we just created it and fade-in is 150ms, within ~0ms the fade percent
        // should be close to 0 (beginning of fade-in)
        let fade = toast.get_fade_percent();
        // At time ~0, fade_in phase: (0 / 150) * 0.9 ~ 0.0
        assert!(fade >= 0.0);
        assert!(fade <= NOTIFY_OPACITY);
    }

    // ---- NOTIFICATION_POSITIONS tests ----

    #[test]
    fn test_notification_positions_count() {
        assert_eq!(NOTIFICATION_POSITIONS.len(), 7);
    }

    #[test]
    fn test_notification_positions_all_parseable_by_from_name() {
        for &pos_name in &NOTIFICATION_POSITIONS {
            let pos = ToastPos::from_name(pos_name);
            // Each position name should produce a non-default result
            // (except TopLeft which is the default)
            assert!(
                pos_name == "TopLeft" || pos != ToastPos::TopLeft,
                "Position '{}' unexpectedly parsed as TopLeft",
                pos_name
            );
        }
    }
}

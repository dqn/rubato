use crate::font_awesome_icons;
use crate::imgui_renderer;

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

// ImGuiWindowFlags combination
pub const NOTIFY_DEFAULT_TOAST_FLAGS: i32 = 0; // stub: AlwaysAutoResize | NoDecoration | NoNav | NoBringToFrontOnFocus | NoFocusOnAppearing

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
        NOTIFY_DEFAULT_TOAST_FLAGS
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

    pub fn render_notifications() {
        let mut notifications = NOTIFICATIONS.lock().unwrap();
        let mut height: f32 = 0.0;

        let mut i = 0;
        while i < notifications.len() {
            let current_toast = &notifications[i];

            if current_toast.get_phase() == ToastPhase::Expired {
                notifications.remove(i);
                continue;
            }

            if NOTIFY_RENDER_LIMIT > 0 && i >= NOTIFY_RENDER_LIMIT {
                i += 1;
                continue;
            }

            let _icon = current_toast.get_icon();
            let _title = current_toast.get_title().to_string();
            let _content = current_toast.get_content().to_string();
            let _default_title = current_toast.get_default_title().map(|s| s.to_string());
            let _opacity = current_toast.get_fade_percent();

            let mut text_color = current_toast.get_color();
            text_color[3] = _opacity;

            let _window_name = format!("##TOAST{}", i);

            // ImGui.setNextWindowBgAlpha(opacity);

            let (toast_x, toast_y) = get_toast_pos(&current_toast.pos, height);
            let _ = (toast_x, toast_y);
            // ImGui.setNextWindowPos(toastPos.x, toastPos.y, ...);

            let mut _window_flags = current_toast.get_window_flags();
            if !NOTIFY_USE_DISMISS_BUTTON && !current_toast.has_on_button_press() {
                // window_flags |= ImGuiWindowFlags.NoInputs;
            }

            // ImGui.begin(windowName, windowFlags);

            // Render title, icon, content, dismiss button, action button
            // ... (all ImGui rendering calls stubbed)

            height += 0.0 /* ImGui.getWindowHeight() */ + NOTIFY_PADDING_MESSAGE_Y;

            // ImGui.end();

            i += 1;
        }
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
    pub fn render_notifications_ui(ctx: &egui::Context) {
        let mut notifications = NOTIFICATIONS.lock().unwrap();
        let mut height: f32 = 0.0;

        let mut i = 0;
        while i < notifications.len() {
            let current_toast = &notifications[i];

            if current_toast.get_phase() == ToastPhase::Expired {
                notifications.remove(i);
                continue;
            }

            if NOTIFY_RENDER_LIMIT > 0 && i >= NOTIFY_RENDER_LIMIT {
                i += 1;
                continue;
            }

            let opacity = current_toast.get_fade_percent();
            let text_color = current_toast.get_color();
            let title = current_toast
                .get_default_title()
                .unwrap_or("Notification")
                .to_string();
            let content = current_toast.get_content().to_string();
            let window_name = format!("##TOAST{}", i);

            let (toast_x, toast_y) = get_toast_pos(&current_toast.pos, height);

            egui::Area::new(egui::Id::new(&window_name))
                .fixed_pos(egui::pos2(toast_x, toast_y))
                .show(ctx, |ui| {
                    let frame = egui::Frame::popup(ui.style()).fill(
                        egui::Color32::from_rgba_unmultiplied(40, 40, 40, (opacity * 255.0) as u8),
                    );
                    frame.show(ui, |ui| {
                        let color = egui::Color32::from_rgba_unmultiplied(
                            (text_color[0] * 255.0) as u8,
                            (text_color[1] * 255.0) as u8,
                            (text_color[2] * 255.0) as u8,
                            (opacity * 255.0) as u8,
                        );
                        if let Some(icon) = current_toast.get_icon() {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(icon).color(color));
                                ui.label(egui::RichText::new(&title).color(color).strong());
                            });
                        } else {
                            ui.label(egui::RichText::new(&title).color(color).strong());
                        }
                        if !content.is_empty() {
                            ui.label(egui::RichText::new(&content).color(
                                egui::Color32::from_rgba_unmultiplied(
                                    200,
                                    200,
                                    200,
                                    (opacity * 255.0) as u8,
                                ),
                            ));
                        }
                    });

                    // Approximate toast height
                    height += 50.0 + NOTIFY_PADDING_MESSAGE_Y;
                });

            i += 1;
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

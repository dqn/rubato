use std::sync::Mutex;

use rubato_types::sync_utils::lock_or_recover;

use super::*;

/// Shared egui state for the search text field.
/// Written by MusicSelector (game thread), read by egui render (render thread).
struct SearchFieldEguiState {
    /// Bounds from skin (screen coordinates, Y-up origin).
    bounds: Option<Rectangle>,
    /// Whether the text field is focused / visible.
    has_focus: bool,
    /// Current search text being edited.
    text: String,
    /// Placeholder text shown when empty.
    message_text: String,
    /// Set to true by egui when Enter is pressed; consumed by game thread.
    enter_pressed: bool,
    /// Set to true by egui when Escape/click-outside occurs; consumed by game thread.
    escape_pressed: bool,
    /// Window height for Y-up to Y-down coordinate conversion.
    window_height: f32,
}

static SEARCH_EGUI_STATE: Mutex<SearchFieldEguiState> = Mutex::new(SearchFieldEguiState {
    bounds: None,
    has_focus: false,
    text: String::new(),
    message_text: String::new(),
    enter_pressed: false,
    escape_pressed: false,
    window_height: 720.0,
});

/// Search text field for song search
/// Translates: bms.player.beatoraja.select.SearchTextField
///
/// In Java this extends com.badlogic.gdx.scenes.scene2d.Stage and creates
/// a LibGDX TextField with font generation. In Rust, the text field is
/// rendered via egui overlay using shared static state.
pub struct SearchTextField {
    pub search_bounds: Option<Rectangle>,
    pub text: String,
    pub message_text: String,
    pub has_focus: bool,
}

impl SearchTextField {
    pub fn new(_selector: &dyn std::any::Any, _resolution: &Resolution) -> Self {
        Self {
            search_bounds: None,
            text: String::new(),
            message_text: "search song".to_string(),
            has_focus: false,
        }
    }

    pub fn unfocus(&mut self) {
        self.text.clear();
        self.message_text = "search song".to_string();
        self.has_focus = false;
    }

    pub fn dispose(&mut self) {
        // Clear shared egui state when disposed
        let mut state = lock_or_recover(&SEARCH_EGUI_STATE);
        state.has_focus = false;
        state.text.clear();
    }

    pub fn search_bounds(&self) -> Option<&Rectangle> {
        self.search_bounds.as_ref()
    }

    /// Sync local state to the shared egui state (called before egui frame).
    pub fn sync_to_egui(&self) {
        let mut state = lock_or_recover(&SEARCH_EGUI_STATE);
        state.bounds = self.search_bounds;
        state.has_focus = self.has_focus;
        state.text.clone_from(&self.text);
        state.message_text.clone_from(&self.message_text);
        state.window_height = crate::state::modmenu::imgui_renderer::window_height().max(1) as f32;
    }

    /// Sync shared egui state back to local state (called after egui frame).
    /// Returns true if Enter was pressed (search should be submitted).
    pub fn sync_from_egui(&mut self) -> SearchFieldAction {
        let mut state = lock_or_recover(&SEARCH_EGUI_STATE);
        self.text.clone_from(&state.text);
        if state.enter_pressed {
            state.enter_pressed = false;
            SearchFieldAction::Submit
        } else if state.escape_pressed {
            state.escape_pressed = false;
            SearchFieldAction::Unfocus
        } else {
            SearchFieldAction::None
        }
    }

    /// Render the search text field using egui.
    /// Called from the egui frame (render thread) via static dispatch.
    pub fn render_egui(ctx: &egui::Context) {
        let mut state = lock_or_recover(&SEARCH_EGUI_STATE);
        if !state.has_focus || state.bounds.is_none() {
            return;
        }

        let bounds = state.bounds.unwrap();
        let window_height = state.window_height;

        // Convert Y-up skin coordinates to egui Y-down screen coordinates.
        // Skin: y is distance from bottom edge. egui: y is distance from top edge.
        let egui_x = bounds.x;
        let egui_y = window_height - bounds.y - bounds.height;
        let egui_w = bounds.width;
        let egui_h = bounds.height;

        let id = egui::Id::new("search_text_field");
        egui::Area::new(id)
            .fixed_pos(egui::pos2(egui_x, egui_y))
            .show(ctx, |ui| {
                let frame = egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                    .inner_margin(2.0);
                frame.show(ui, |ui| {
                    let font_size = (egui_h * 0.8).max(12.0);
                    let hint = state.message_text.clone();
                    // Java: search.setMaxLength(50)
                    let text_edit = egui::TextEdit::singleline(&mut state.text)
                        .desired_width(egui_w - 4.0)
                        .hint_text(hint)
                        .char_limit(50)
                        .font(egui::FontId::proportional(font_size))
                        .text_color(egui::Color32::WHITE);

                    let response = ui.add(text_edit);

                    // Request focus on the text field so keyboard input is captured
                    if !response.has_focus() {
                        response.request_focus();
                    }

                    // Check for Enter key
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        state.enter_pressed = true;
                    }

                    // Check for Escape key
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        state.escape_pressed = true;
                    }
                });
            });
    }
}

/// Actions returned from egui sync
pub enum SearchFieldAction {
    None,
    Submit,
    Unfocus,
}

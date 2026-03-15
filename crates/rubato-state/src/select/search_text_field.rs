use super::*;

/// Search text field for song search
/// Translates: bms.player.beatoraja.select.SearchTextField
///
/// In Java this extends com.badlogic.gdx.scenes.scene2d.Stage and creates
/// a LibGDX TextField with font generation. In Rust, the text rendering
/// and input handling will be provided by egui when the skin menu overlay
/// is migrated to egui. This struct holds the search state (text, focus,
/// bounds) used by `MusicSelector` for filtering.
pub struct SearchTextField {
    pub search_bounds: Option<Rectangle>,
    pub text: String,
    pub message_text: String,
    pub has_focus: bool,
}

impl SearchTextField {
    pub fn new(_selector: &dyn std::any::Any, _resolution: &Resolution) -> Self {
        // In Java: creates Stage, FreeTypeFontGenerator, TextField, etc.
        // Rendering will be handled by egui in the skin menu overlay.
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
        // In Java: disposes generator and font
    }

    pub fn search_bounds(&self) -> Option<&Rectangle> {
        self.search_bounds.as_ref()
    }
}

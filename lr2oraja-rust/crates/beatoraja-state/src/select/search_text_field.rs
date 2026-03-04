use super::stubs::*;

/// Search text field for song search
/// Translates: bms.player.beatoraja.select.SearchTextField
///
/// In Java this extends com.badlogic.gdx.scenes.scene2d.Stage and creates
/// a LibGDX TextField with font generation. In Rust, the rendering/input
/// handling is stubbed since it requires a GUI framework.
pub struct SearchTextField {
    pub search_bounds: Option<Rectangle>,
    pub text: String,
    pub message_text: String,
    pub has_focus: bool,
}

impl SearchTextField {
    pub fn new(_selector: &dyn std::any::Any, _resolution: &Resolution) -> Self {
        // In Java: creates Stage, FreeTypeFontGenerator, TextField, etc.
        // All rendering is stubbed here.
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

    pub fn get_search_bounds(&self) -> Option<&Rectangle> {
        self.search_bounds.as_ref()
    }
}

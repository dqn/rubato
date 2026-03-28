use super::super::bar::bar::Bar;
use super::super::bar_manager::BarManager;
use super::super::music_select_key_property::MusicSelectKeyProperty;
use crate::select::*;

/// Bar area data for rendering
pub(super) struct BarArea {
    pub sd: Option<usize>, // index into currentsongs
    pub x: f32,
    pub y: f32,
    pub value: i32,
    pub text: usize,
}

impl BarArea {
    pub(super) fn new() -> Self {
        Self {
            sd: None,
            x: 0.0,
            y: 0.0,
            value: -1,
            text: 0,
        }
    }
}

/// Context for BarRenderer::prepare()
/// Provides the data from MusicSelectSkin and BarManager needed for bar layout.
pub struct PrepareContext<'a> {
    pub center_bar: i32,
    pub currentsongs: &'a [Bar],
    pub selectedindex: usize,
}

/// Context for BarRenderer::render()
/// Provides the data from MusicSelector needed for bar drawing.
pub struct RenderContext<'a> {
    pub center_bar: i32,
    pub currentsongs: &'a [Bar],
    pub rival: bool,
    pub state: &'a dyn MainState,
    pub lnmode: i32,
}

/// Context for BarRenderer::input()
/// Provides the data from MusicSelector needed for scroll input handling.
pub struct BarInputContext<'a> {
    pub input: &'a mut BMSPlayerInputProcessor,
    pub property: &'a MusicSelectKeyProperty,
    pub manager: &'a mut BarManager,
    /// Callback to play SCRATCH sound
    pub play_scratch: &'a mut dyn FnMut(),
    /// Callback to stop SCRATCH sound
    pub stop_scratch: &'a mut dyn FnMut(),
}

/// Context for BarRenderer::mouse_pressed()
/// Provides the data from MusicSelector needed for click detection.
pub struct MousePressedContext<'a> {
    pub clickable_bar: &'a [i32],
    pub center_bar: i32,
    pub currentsongs: &'a [Bar],
    pub selectedindex: usize,
    pub state: &'a dyn MainState,
    pub timer_now_time: i64,
}

/// Result of mouse_pressed indicating what action to take
pub enum MousePressedAction {
    /// No bar was clicked
    None,
    /// A bar was selected (left click) — index into currentsongs
    Select(usize),
    /// Close the current directory (right click)
    Close,
}

use std::sync::Arc;

use super::selectable_bar::SelectableBarData;
use crate::state::select::SongData;

// Bar appearance ID constants
pub const STYLE_SONG: i32 = 0;
pub const STYLE_FOLDER: i32 = 1;
pub const STYLE_TABLE: i32 = 2;
pub const STYLE_COURSE: i32 = 3;
pub const STYLE_MISSING: i32 = 4;
pub const STYLE_SPECIAL: i32 = 5;
pub const STYLE_SEARCH: i32 = 6;

pub const STYLE_TEXT_PLAIN: i32 = 0;
pub const STYLE_TEXT_NEW: i32 = 1;
pub const STYLE_TEXT_MISSING: i32 = 8;

/// Function type for FunctionBar callbacks (Arc for cheap cloning)
/// In Java: BiConsumer<MusicSelector, FunctionBar>
/// Accepts &mut MusicSelector so callbacks can access selector state at execution time.
pub type FunctionBarCallback =
    Arc<dyn Fn(&mut crate::state::select::music_selector::MusicSelector) + Send + Sync>;

/// Bar that executes a function when selected
/// Translates: bms.player.beatoraja.select.bar.FunctionBar
#[derive(Clone)]
pub struct FunctionBar {
    pub selectable: SelectableBarData,
    pub function: Option<FunctionBarCallback>,
    pub title: String,
    pub subtitle: Option<String>,
    pub display_bar_type: i32,
    pub display_text_type: i32,
    pub song: Option<SongData>,
    pub level: Option<i32>,
    pub lamp: i32,
    pub lamps: Vec<i32>,
}

impl FunctionBar {
    pub fn new(title: String, display_bar_type: i32) -> Self {
        Self::new_with_text_type(title, display_bar_type, 0)
    }

    pub fn new_with_text_type(
        title: String,
        display_bar_type: i32,
        display_text_type: i32,
    ) -> Self {
        Self {
            selectable: SelectableBarData::default(),
            function: None,
            title,
            subtitle: None,
            display_bar_type,
            display_text_type,
            song: None,
            level: None,
            lamp: 0,
            lamps: Vec::new(),
        }
    }

    pub fn set_function(&mut self, f: FunctionBarCallback) {
        self.function = Some(f);
    }

    pub fn set_song_data(&mut self, song: SongData) {
        self.song = Some(song);
    }

    pub fn set_subtitle(&mut self, subtitle: String) {
        self.subtitle = Some(subtitle);
    }

    pub fn set_level(&mut self, level: i32) {
        self.level = Some(level);
    }

    pub fn accept(&self, selector: &mut crate::state::select::music_selector::MusicSelector) {
        if let Some(ref f) = self.function {
            f(selector);
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    pub fn level(&self) -> Option<i32> {
        self.level
    }

    pub fn lamp(&self, _is_player: bool) -> i32 {
        self.lamp
    }

    pub fn lamps(&self) -> &[i32] {
        &self.lamps
    }

    pub fn display_bar_type(&self) -> i32 {
        self.display_bar_type
    }

    pub fn display_text_type(&self) -> i32 {
        self.display_text_type
    }
}

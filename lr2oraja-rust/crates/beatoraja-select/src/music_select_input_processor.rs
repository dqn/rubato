use crate::bar::bar::Bar;
use crate::bar::directory_bar::DirectoryBarData;
use crate::bar::function_bar::FunctionBar;
use crate::bar::song_bar::SongBar;
use crate::music_select_command::MusicSelectCommand;
use crate::music_select_key_property::{MusicSelectKey, MusicSelectKeyProperty};
use crate::stubs::*;

/// Music select input processor
/// Translates: bms.player.beatoraja.select.MusicSelectInputProcessor
pub struct MusicSelectInputProcessor {
    /// Bar movement counter
    pub duration: i64,
    /// Bar movement direction
    pub angle: i32,

    pub durationlow: i32,
    pub durationhigh: i32,

    /// Analog scroll buffer
    pub analog_scroll_buffer: i32,
    pub analog_ticks_per_scroll: i32,

    pub is_option_key_pressed: bool,
    pub is_option_key_released: bool,

    // Duration change counter for notes display timing
    pub time_change_duration: i64,
    pub count_change_duration: i32,
}

impl MusicSelectInputProcessor {
    pub fn new(durationlow: i32, durationhigh: i32, analog_ticks_per_scroll: i32) -> Self {
        Self {
            duration: 0,
            angle: 0,
            durationlow,
            durationhigh,
            analog_scroll_buffer: 0,
            analog_ticks_per_scroll,
            is_option_key_pressed: false,
            is_option_key_released: false,
            time_change_duration: 0,
            count_change_duration: 0,
        }
    }

    /// Process input
    /// In Java: this method accesses MusicSelector, MainController, BMSPlayerInputProcessor,
    /// PlayerResource, PlayerConfig, BarRenderer, BarManager, etc.
    /// Since those are all tightly coupled, this is stubbed with todo!()
    pub fn input(&mut self) {
        log::warn!(
            "not yet implemented: MusicSelectInputProcessor.input - requires MusicSelector context"
        );
    }
}

use crate::property::boolean_property_factory;
use crate::property::float_property_factory;
use crate::property::integer_property_factory;
use crate::property::string_property_factory;
use crate::stubs::MainState;

/// Main state accessor for Lua
///
/// Translated from MainStateAccessor.java (319 lines)
/// Provides Lua functions to access game state values from MainState.
/// Exports functions: option, number, float_number, text, offset, timer,
/// timer_off_value, time, set_timer, event_exec, event_index,
/// rate, exscore, rate_best, exscore_best, rate_rival, exscore_rival,
/// volume_sys, set_volume_sys, volume_key, set_volume_key,
/// volume_bg, set_volume_bg, judge, gauge, gauge_type,
/// audio_play, audio_loop, audio_stop
///
/// Timer off value constant (Long.MIN_VALUE in Java)
pub const TIMER_OFF_VALUE: i64 = i64::MIN;

pub struct MainStateAccessor {
    // Would hold reference to MainState
    // In Rust, this will be passed as parameter instead of held as reference
}

impl MainStateAccessor {
    pub fn new(_state: &dyn MainState) -> Self {
        Self {}
    }

    /// Export all accessor functions to a Lua table
    /// In actual implementation, this would use mlua to create Lua functions
    pub fn export(&self, _table: &()) {
        // Generic functions (ID-based access)
        // table.set("option", option function)
        //   - Gets OPTION_* boolean by ID using BooleanPropertyFactory
        // table.set("number", number function)
        //   - Gets NUMBER_* integer by ID using IntegerPropertyFactory
        // table.set("float_number", float_number function)
        //   - Gets SLIDER_*/BARGRAPH_* float by ID using FloatPropertyFactory
        // table.set("text", text function)
        //   - Gets STRING_* text by ID using StringPropertyFactory
        // table.set("offset", offset function)
        //   - Gets OFFSET_* values by ID, returns table {x, y, w, h, r, a}
        // table.set("timer", timer function)
        //   - Gets timer value (micro sec) by ID
        // table.set("timer_off_value", TIMER_OFF_VALUE)
        //   - Timer OFF constant (i64::MIN)
        // table.set("time", time function)
        //   - Gets current time (micro sec)
        // table.set("set_timer", set_timer function)
        //   - Sets timer value by ID (only writable timers)
        // table.set("event_exec", event_exec function)
        //   - Executes event by ID (0, 1, or 2 args) (only runnable events)
        // table.set("event_index", event_index function)
        //   - Gets event/button index by ID

        // Concrete value accessors
        // table.set("rate", rate function)
        //   - Returns state.getScoreDataProperty().getNowRate()
        // table.set("exscore", exscore function)
        //   - Returns state.getScoreDataProperty().getNowEXScore()
        // table.set("rate_best", rate_best function)
        //   - Returns state.getScoreDataProperty().getNowBestScoreRate()
        // table.set("exscore_best", exscore_best function)
        //   - Returns state.getScoreDataProperty().getBestScore()
        // table.set("rate_rival", rate_rival function)
        //   - Returns state.getScoreDataProperty().getRivalScoreRate()
        // table.set("exscore_rival", exscore_rival function)
        //   - Returns state.getScoreDataProperty().getRivalScore()

        // Volume accessors
        // table.set("volume_sys", volume_sys function)
        //   - Returns state.main.getConfig().getAudioConfig().getSystemvolume()
        // table.set("set_volume_sys", set_volume_sys function)
        //   - Sets system volume
        // table.set("volume_key", volume_key function)
        //   - Returns state.main.getConfig().getAudioConfig().getKeyvolume()
        // table.set("set_volume_key", set_volume_key function)
        //   - Sets key volume
        // table.set("volume_bg", volume_bg function)
        //   - Returns state.main.getConfig().getAudioConfig().getBgvolume()
        // table.set("set_volume_bg", set_volume_bg function)
        //   - Sets bg volume

        // Game state
        // table.set("judge", judge function)
        //   - Returns judge count for given judge type (early + late)
        // table.set("gauge", gauge function)
        //   - Returns gauge value (BMSPlayer only, 0 otherwise)
        // table.set("gauge_type", gauge_type function)
        //   - Returns gauge type (BMSPlayer only, 0 otherwise)

        // Audio
        // table.set("audio_play", audio_play function)
        //   - Plays audio file at path with volume (clamped 0-2, default 1)
        // table.set("audio_loop", audio_loop function)
        //   - Loops audio file at path with volume
        // table.set("audio_stop", audio_stop function)
        //   - Stops audio at path

        todo!("mlua integration: export main state accessor functions")
    }
}

/// option function - Gets OPTION_* boolean by ID
/// NOTE: Creates BooleanProperty on every call (inefficient, deprecated)
pub fn option_fn(state: &dyn MainState, id: i32) -> bool {
    if let Some(prop) = boolean_property_factory::get_boolean_property(id) {
        prop.get(state)
    } else {
        false
    }
}

/// number function - Gets NUMBER_* integer by ID
/// NOTE: Creates IntegerProperty on every call (inefficient, deprecated)
pub fn number_fn(state: &dyn MainState, id: i32) -> i32 {
    if let Some(prop) = integer_property_factory::get_integer_property_by_id(id) {
        prop.get(state)
    } else {
        0
    }
}

/// float_number function - Gets SLIDER_*/BARGRAPH_* float by ID
/// NOTE: Creates FloatProperty on every call (inefficient, deprecated)
pub fn float_number_fn(state: &dyn MainState, id: i32) -> f32 {
    if let Some(prop) = float_property_factory::get_rate_property_by_id(id) {
        prop.get(state)
    } else {
        0.0
    }
}

/// text function - Gets STRING_* text by ID
/// NOTE: Creates StringProperty on every call (inefficient, deprecated)
pub fn text_fn(state: &dyn MainState, id: i32) -> String {
    if let Some(prop) = string_property_factory::get_string_property_by_id(id) {
        prop.get(state)
    } else {
        String::new()
    }
}

/// event_index function - Gets event/button index by ID
/// NOTE: Creates IntegerProperty on every call (inefficient, deprecated)
pub fn event_index_fn(state: &dyn MainState, id: i32) -> i32 {
    if let Some(prop) = integer_property_factory::get_image_index_property_by_id(id) {
        prop.get(state)
    } else {
        0
    }
}

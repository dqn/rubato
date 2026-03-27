use crate::core::bms_player_mode::Mode as AutoplayMode;
use crate::play::lane_renderer::LaneRenderer;
use bms::model::mode::Mode;

mod types;
pub use types::*;

pub struct ControlInputProcessor {
    hschanged: Vec<bool>,
    startpressedtime: i64,
    selectpressedtime: i64,
    startpressed: bool,
    selectpressed: bool,
    start_and_select_pressed: bool,
    cursorpressed: bool,
    lanecovertiming: i64,
    lane_cover_start_timing: i64,
    exitpressedtime: i64,
    exit_press_duration: i64,
    pub enable_control: bool,
    pub enable_cursor: bool,
    is_change_lift: bool,
    cover_change_margin_low: f32,
    cover_change_margin_high: f32,
    cover_speed_switch_duration: i64,
    hispeed_auto_adjust: bool,
    keybinds: Vec<i32>,
}

impl ControlInputProcessor {
    pub fn new(mode: Mode) -> Self {
        let keybinds = match mode {
            Mode::BEAT_5K | Mode::BEAT_10K => {
                vec![-1, 1, -1, 1, -1, 2, -2, -1, 1, -1, 1, -1, 2, -2]
            }
            Mode::POPN_5K | Mode::POPN_9K => vec![-1, 1, -1, 1, -1, 1, -1, 2, -2],
            Mode::BEAT_7K | Mode::BEAT_14K => vec![
                -1, 1, -1, 1, -1, 1, -1, 2, -2, -1, 1, -1, 1, -1, 1, -1, 2, -2,
            ],
            Mode::KEYBOARD_24K | Mode::KEYBOARD_24K_DOUBLE => vec![
                -1, 1, -1, 1, -1, -1, 1, -1, 1, -1, 1, -1, -1, 1, -1, 1, -1, -1, 1, -1, 1, -1, 1,
                -1, -2, 2, -1, 1, -1, 1, -1, -1, 1, -1, 1, -1, 1, -1, -1, 1, -1, 1, -1, -1, 1, -1,
                1, -1, 1, -1, -2, 2,
            ],
        };

        let keystate_size = 256; // BMSPlayerInputProcessor.KEYSTATE_SIZE
        let hschanged = vec![true; keystate_size];

        ControlInputProcessor {
            hschanged,
            startpressedtime: 0,
            selectpressedtime: 0,
            startpressed: false,
            selectpressed: false,
            start_and_select_pressed: false,
            cursorpressed: false,
            lanecovertiming: 0,
            lane_cover_start_timing: i64::MIN,
            exitpressedtime: 0,
            exit_press_duration: 1000,
            enable_control: true,
            enable_cursor: true,
            is_change_lift: true,
            cover_change_margin_low: 0.001,
            cover_change_margin_high: 0.01,
            cover_speed_switch_duration: 500,
            hispeed_auto_adjust: false,
            keybinds,
        }
    }

    /// Configure from PlayConfig values (called during construction with player context).
    pub fn configure(
        &mut self,
        cover_change_margin_low: f32,
        cover_change_margin_high: f32,
        cover_speed_switch_duration: i64,
        hispeed_auto_adjust: bool,
        exit_press_duration: i64,
    ) {
        self.cover_change_margin_low = cover_change_margin_low;
        self.cover_change_margin_high = cover_change_margin_high;
        self.cover_speed_switch_duration = cover_speed_switch_duration;
        self.hispeed_auto_adjust = hispeed_auto_adjust;
        self.exit_press_duration = exit_press_duration;
    }

    /// Returns whether control input (speed changes etc.) is enabled.
    pub fn is_enable_control(&self) -> bool {
        self.enable_control
    }

    /// Main input processing method.
    ///
    /// Translated from: ControlInputProcessor.input()
    ///
    /// Handles:
    /// - Cursor UP/DOWN for lane cover adjustment
    /// - Mouse wheel for lane cover adjustment
    /// - START button: hi-speed changes + double-press to toggle lane cover
    /// - SELECT button: duration changes + double-press to toggle hidden
    /// - START+SELECT: toggle lift/hidden change mode
    /// - START+SELECT held: quick retry (stop play)
    /// - ESC: stop play
    /// - NUM1-4: play speed changes (autoplay/replay only)
    pub fn input(&mut self, ctx: &mut ControlInputContext) -> ControlInputResult {
        let mut result = ControlInputResult::default();

        let is_play_or_practice =
            ctx.autoplay_mode == AutoplayMode::Play || ctx.autoplay_mode == AutoplayMode::Practice;

        // Control input processing
        if self.enable_control {
            // Cursor UP/DOWN for lane cover
            if self.enable_cursor {
                if ctx.control_key_up {
                    if !self.cursorpressed {
                        self.set_cover_value(-0.01, ctx.lanerender);
                        self.cursorpressed = true;
                    }
                } else if ctx.control_key_down {
                    if !self.cursorpressed {
                        self.set_cover_value(0.01, ctx.lanerender);
                        self.cursorpressed = true;
                    }
                } else {
                    self.cursorpressed = false;
                }
            }

            // Mouse wheel for lane cover
            if ctx.scroll != 0 {
                self.set_cover_value(-ctx.scroll as f32 * 0.005, ctx.lanerender);
                result.reset_scroll = true;
            }

            // START button processing
            if ctx.start_pressed || (ctx.window_hold && ctx.is_timer_play_on && !ctx.is_note_end) {
                if is_play_or_practice && self.startpressed {
                    // change hi speed by START + Keys
                    self.process_start(
                        ctx.key_states,
                        ctx.lanerender,
                        ctx.is_analog,
                        ctx.analog_diff_and_reset,
                        ctx.now_millis,
                    );
                } else if is_play_or_practice && !self.startpressed {
                    self.hschanged.fill(true);
                }
                // show-hide lane cover by double-press START
                if !self.startpressed {
                    let stime = ctx.now_millis;
                    if stime < self.startpressedtime + 500 {
                        let enabled = ctx.lanerender.is_enable_lanecover();
                        ctx.lanerender.enable_lanecover = !enabled;
                        self.startpressedtime = 0;
                    } else {
                        self.startpressedtime = stime;
                    }
                }
                self.startpressed = true;
            } else {
                self.startpressed = false;
            }

            // SELECT button processing
            if ctx.select_pressed {
                if is_play_or_practice && self.selectpressed {
                    // change duration by SELECT + Keys
                    self.process_select(
                        ctx.key_states,
                        ctx.lanerender,
                        ctx.is_analog,
                        ctx.analog_diff_and_reset,
                        ctx.now_millis,
                    );
                } else if is_play_or_practice && !self.selectpressed {
                    self.hschanged.fill(true);
                }
                // show-hide hidden by double-press SELECT
                if !self.selectpressed {
                    let stime = ctx.now_millis;
                    if stime < self.selectpressedtime + 500 {
                        let enabled = ctx.lanerender.is_enable_hidden();
                        ctx.lanerender.enable_hidden = !enabled;
                        self.selectpressedtime = 0;
                    } else {
                        self.selectpressedtime = stime;
                    }
                }
                self.selectpressed = true;
            } else {
                self.selectpressed = false;
            }

            // START+SELECT: toggle lift/hidden change mode
            if ctx.start_pressed && ctx.select_pressed {
                if !self.start_and_select_pressed {
                    self.is_change_lift = !self.is_change_lift;
                }
                self.start_and_select_pressed = true;
            } else {
                self.start_and_select_pressed = false;
            }
        }

        // Exit detection: START+SELECT held for exitPressDuration, or noteEnd + start/select
        let now = ctx.now_millis;
        if (ctx.start_pressed
            && ctx.select_pressed
            && now - self.exitpressedtime > self.exit_press_duration)
            || (ctx.is_note_end && (ctx.start_pressed || ctx.select_pressed))
        {
            result.clear_start = true;
            result.clear_select = true;
            result.stop_play = true;
        } else if !(ctx.start_pressed && ctx.select_pressed) {
            self.exitpressedtime = now;
        }

        // ESC to stop play
        if ctx.control_key_escape_pressed {
            result.stop_play = true;
        }

        // Play speed change (autoplay or replay only)
        if ctx.autoplay_mode == AutoplayMode::Autoplay || ctx.autoplay_mode == AutoplayMode::Replay
        {
            if ctx.control_key_num1 {
                result.play_speed = Some(25);
            } else if ctx.control_key_num2 {
                result.play_speed = Some(50);
            } else if ctx.control_key_num3 {
                result.play_speed = Some(200);
            } else if ctx.control_key_num4 {
                result.play_speed = Some(300);
            } else {
                result.play_speed = Some(100);
            }
        }

        result
    }

    /// Process START + key combinations for hi-speed / lane cover changes.
    ///
    /// Translated from the Java processStart lambda.
    fn process_start(
        &mut self,
        key_states: &[bool],
        lanerender: &mut LaneRenderer,
        is_analog: &[bool],
        analog_diff_and_reset: &mut dyn FnMut(usize, i32) -> i32,
        now_millis: i64,
    ) {
        for i in 0..self.keybinds.len() {
            let keystate = if i < key_states.len() {
                key_states[i]
            } else {
                false
            };
            match self.keybinds[i] {
                -1 => {
                    if keystate && !self.hschanged[i] {
                        lanerender.change_hispeed(false);
                    }
                }
                1 => {
                    if keystate && !self.hschanged[i] {
                        lanerender.change_hispeed(true);
                    }
                }
                2 => self.change_cover_value(
                    &ScratchInputContext {
                        key: i,
                        up: true,
                        key_states,
                        is_analog,
                        now_millis,
                    },
                    lanerender,
                    analog_diff_and_reset,
                ),
                -2 => self.change_cover_value(
                    &ScratchInputContext {
                        key: i,
                        up: false,
                        key_states,
                        is_analog,
                        now_millis,
                    },
                    lanerender,
                    analog_diff_and_reset,
                ),
                _ => {}
            }
            self.hschanged[i] = keystate;
        }
    }

    /// Process SELECT + key combinations for duration changes.
    ///
    /// Translated from the Java processSelect lambda.
    fn process_select(
        &mut self,
        key_states: &[bool],
        lanerender: &mut LaneRenderer,
        is_analog: &[bool],
        analog_diff_and_reset: &mut dyn FnMut(usize, i32) -> i32,
        now_millis: i64,
    ) {
        for i in 0..self.keybinds.len() {
            let keystate = if i < key_states.len() {
                key_states[i]
            } else {
                false
            };
            match self.keybinds[i] {
                -1 => {
                    if keystate && !self.hschanged[i] {
                        let dur = lanerender.duration();
                        lanerender.set_duration(dur - 1);
                    }
                }
                1 => {
                    if keystate && !self.hschanged[i] {
                        let dur = lanerender.duration();
                        lanerender.set_duration(dur + 1);
                    }
                }
                2 => self.change_duration(
                    &ScratchInputContext {
                        key: i,
                        up: true,
                        key_states,
                        is_analog,
                        now_millis,
                    },
                    lanerender,
                    analog_diff_and_reset,
                ),
                -2 => self.change_duration(
                    &ScratchInputContext {
                        key: i,
                        up: false,
                        key_states,
                        is_analog,
                        now_millis,
                    },
                    lanerender,
                    analog_diff_and_reset,
                ),
                _ => {}
            }
            self.hschanged[i] = keystate;
        }
    }

    /// Change lane cover/lift/hidden value based on current state.
    ///
    /// Translated from: Java setCoverValue(float)
    ///
    /// Rules:
    /// - Lane cover: if lanecover is on, or both lift and hidden are off
    /// - Hidden: if hidden is on (and lanecover is off)
    /// - Lift: if lift is on and isChangeLift (and lanecover is off)
    /// - When both lift and hidden are on with lanecover off, START+SELECT toggles which is adjusted
    fn set_cover_value(&mut self, value: f32, lanerender: &mut LaneRenderer) {
        if lanerender.is_enable_lanecover()
            || (!lanerender.is_enable_lift() && !lanerender.is_enable_hidden())
        {
            let lc = lanerender.lanecover();
            lanerender.set_lanecover(lc + value);
        } else if lanerender.is_enable_hidden() {
            let hc = lanerender.hidden_cover();
            lanerender.set_hidden_cover(hc - value);
        } else if lanerender.is_enable_lift() && self.is_change_lift {
            let lr = lanerender.lift_region();
            lanerender.set_lift_region(lr - value);
        }

        if self.hispeed_auto_adjust && lanerender.now_bpm() > 0.0 {
            let bpm = lanerender.now_bpm();
            lanerender.reset_hispeed(bpm);
        }
    }

    /// Change cover value by scratch input (START + Scratch).
    ///
    /// Translated from: Java changeCoverValue(int key, boolean up)
    fn change_cover_value(
        &mut self,
        scratch: &ScratchInputContext,
        lanerender: &mut LaneRenderer,
        analog_diff_and_reset: &mut dyn FnMut(usize, i32) -> i32,
    ) {
        let key_is_analog = if scratch.key < scratch.is_analog.len() {
            scratch.is_analog[scratch.key]
        } else {
            false
        };

        if key_is_analog {
            // analog input
            let d_ticks = analog_diff_and_reset(scratch.key, 200) * if scratch.up { 1 } else { -1 };
            if d_ticks != 0 {
                self.set_cover_value(d_ticks as f32 * self.cover_change_margin_low, lanerender);
            }
        } else {
            // non-analog (digital) input
            let keystate = if scratch.key < scratch.key_states.len() {
                scratch.key_states[scratch.key]
            } else {
                false
            };
            if keystate {
                let l = scratch.now_millis;
                if self.lane_cover_start_timing == i64::MIN {
                    self.lane_cover_start_timing = l;
                }
                if l - self.lanecovertiming > 50 {
                    let margin =
                        if l - self.lane_cover_start_timing > self.cover_speed_switch_duration {
                            self.cover_change_margin_high
                        } else {
                            self.cover_change_margin_low
                        };
                    let sign = if scratch.up { 1.0 } else { -1.0 };
                    self.set_cover_value(sign * margin, lanerender);
                    self.lanecovertiming = l;
                }
            } else if self.lane_cover_start_timing != i64::MIN {
                self.lane_cover_start_timing = i64::MIN;
            }
        }
    }

    /// Change duration by scratch input (SELECT + Scratch).
    ///
    /// Translated from: Java changeDuration(int key, boolean up)
    fn change_duration(
        &mut self,
        scratch: &ScratchInputContext,
        lanerender: &mut LaneRenderer,
        analog_diff_and_reset: &mut dyn FnMut(usize, i32) -> i32,
    ) {
        let key_is_analog = if scratch.key < scratch.is_analog.len() {
            scratch.is_analog[scratch.key]
        } else {
            false
        };

        if key_is_analog {
            // analog input
            let d_ticks = analog_diff_and_reset(scratch.key, 200) * if scratch.up { 1 } else { -1 };
            if d_ticks != 0 {
                let dur = lanerender.duration();
                lanerender.set_duration(dur + d_ticks);
            }
        } else {
            // non-analog (digital) input
            let keystate = if scratch.key < scratch.key_states.len() {
                scratch.key_states[scratch.key]
            } else {
                false
            };
            if keystate {
                let l = scratch.now_millis;
                if l - self.lanecovertiming > 50 {
                    let dur = lanerender.duration();
                    lanerender.set_duration(dur + if scratch.up { 1 } else { -1 });
                    self.lanecovertiming = l;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

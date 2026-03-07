use crate::lane_renderer::LaneRenderer;
use bms_model::mode::Mode;
use rubato_core::bms_player_mode::Mode as AutoplayMode;

/// Bundles the external input state needed by ControlInputProcessor,
/// avoiding the need for the processor to hold references to the parent player.
/// Modeled after KeyInputProccessor's InputContext pattern.
pub struct ControlInputContext<'a> {
    /// Mutable reference to the lane renderer for lane cover / hispeed / duration changes.
    pub lanerender: &'a mut LaneRenderer,
    /// Whether the START button is currently pressed (from BMSPlayerInputProcessor).
    pub start_pressed: bool,
    /// Whether the SELECT button is currently pressed (from BMSPlayerInputProcessor).
    pub select_pressed: bool,
    /// Control key states: UP, DOWN, ESCAPE, NUM1-4
    pub control_key_up: bool,
    pub control_key_down: bool,
    pub control_key_escape_pressed: bool,
    pub control_key_num1: bool,
    pub control_key_num2: bool,
    pub control_key_num3: bool,
    pub control_key_num4: bool,
    /// Key states array (indexed by key ID) from BMSPlayerInputProcessor.
    pub key_states: &'a [bool],
    /// Mouse scroll value (from BMSPlayerInputProcessor.getScroll()).
    pub scroll: i32,
    /// Analog input queries — closures that read from BMSPlayerInputProcessor.
    /// Returns true if key `i` is analog input.
    pub is_analog: &'a [bool],
    /// Analog diff and reset function.
    /// Takes (key_index, ms_tolerance) -> diff_ticks.
    pub analog_diff_and_reset: &'a mut dyn FnMut(usize, i32) -> i32,
    /// Whether TIMER_PLAY is on (from timer manager).
    pub is_timer_play_on: bool,
    /// Whether all notes have been passed (from BMSPlayer.isNoteEnd()).
    pub is_note_end: bool,
    /// Whether windowHold is enabled (from PlayerConfig).
    pub window_hold: bool,
    /// The autoplay mode (Play, Practice, Autoplay, Replay).
    pub autoplay_mode: AutoplayMode,
    /// Current time in milliseconds (System.currentTimeMillis() equivalent).
    pub now_millis: i64,
}

/// Actions produced by ControlInputProcessor.input() that need to be
/// applied by the caller (BMSPlayer).
#[derive(Debug, Default)]
pub struct ControlInputResult {
    /// Whether play should be stopped (START+SELECT held or ESC pressed or note end + start/select).
    pub stop_play: bool,
    /// Play speed to set (only for autoplay/replay modes). None means no change.
    pub play_speed: Option<i32>,
    /// Whether to clear start_pressed on the input processor.
    pub clear_start: bool,
    /// Whether to clear select_pressed on the input processor.
    pub clear_select: bool,
    /// Whether to reset scroll on the input processor.
    pub reset_scroll: bool,
}

/// Control input processor for BMSPlayer.
///
/// Handles START+SELECT quick retry, lane cover controls, hispeed changes,
/// duration changes, and play speed controls.
///
/// Translated from: bms.player.beatoraja.play.ControlInputProcessor
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
                    i,
                    true,
                    key_states,
                    lanerender,
                    is_analog,
                    analog_diff_and_reset,
                    now_millis,
                ),
                -2 => self.change_cover_value(
                    i,
                    false,
                    key_states,
                    lanerender,
                    is_analog,
                    analog_diff_and_reset,
                    now_millis,
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
                    i,
                    true,
                    key_states,
                    lanerender,
                    is_analog,
                    analog_diff_and_reset,
                    now_millis,
                ),
                -2 => self.change_duration(
                    i,
                    false,
                    key_states,
                    lanerender,
                    is_analog,
                    analog_diff_and_reset,
                    now_millis,
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
    #[allow(clippy::too_many_arguments)]
    fn change_cover_value(
        &mut self,
        key: usize,
        up: bool,
        key_states: &[bool],
        lanerender: &mut LaneRenderer,
        is_analog: &[bool],
        analog_diff_and_reset: &mut dyn FnMut(usize, i32) -> i32,
        now_millis: i64,
    ) {
        let key_is_analog = if key < is_analog.len() {
            is_analog[key]
        } else {
            false
        };

        if key_is_analog {
            // analog input
            let d_ticks = analog_diff_and_reset(key, 200) * if up { 1 } else { -1 };
            if d_ticks != 0 {
                self.set_cover_value(d_ticks as f32 * self.cover_change_margin_low, lanerender);
            }
        } else {
            // non-analog (digital) input
            let keystate = if key < key_states.len() {
                key_states[key]
            } else {
                false
            };
            if keystate {
                let l = now_millis;
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
                    let sign = if up { 1.0 } else { -1.0 };
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
    #[allow(clippy::too_many_arguments)]
    fn change_duration(
        &mut self,
        key: usize,
        up: bool,
        key_states: &[bool],
        lanerender: &mut LaneRenderer,
        is_analog: &[bool],
        analog_diff_and_reset: &mut dyn FnMut(usize, i32) -> i32,
        now_millis: i64,
    ) {
        let key_is_analog = if key < is_analog.len() {
            is_analog[key]
        } else {
            false
        };

        if key_is_analog {
            // analog input
            let d_ticks = analog_diff_and_reset(key, 200) * if up { 1 } else { -1 };
            if d_ticks != 0 {
                let dur = lanerender.duration();
                lanerender.set_duration(dur + d_ticks);
            }
        } else {
            // non-analog (digital) input
            let keystate = if key < key_states.len() {
                key_states[key]
            } else {
                false
            };
            if keystate {
                let l = now_millis;
                if l - self.lanecovertiming > 50 {
                    let dur = lanerender.duration();
                    lanerender.set_duration(dur + if up { 1 } else { -1 });
                    self.lanecovertiming = l;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::bms_model::BMSModel;

    /// Helper: create a default LaneRenderer from a minimal BMSModel.
    fn make_lanerender() -> LaneRenderer {
        let model = BMSModel::new();
        LaneRenderer::new(&model)
    }

    /// Helper: create a no-op analog_diff_and_reset closure.
    fn noop_analog() -> Box<dyn FnMut(usize, i32) -> i32> {
        Box::new(|_key, _ms| 0)
    }

    /// Helper: build a ControlInputContext with common defaults.
    fn make_context<'a>(
        lanerender: &'a mut LaneRenderer,
        analog_fn: &'a mut dyn FnMut(usize, i32) -> i32,
    ) -> ControlInputContext<'a> {
        ControlInputContext {
            lanerender,
            start_pressed: false,
            select_pressed: false,
            control_key_up: false,
            control_key_down: false,
            control_key_escape_pressed: false,
            control_key_num1: false,
            control_key_num2: false,
            control_key_num3: false,
            control_key_num4: false,
            key_states: &[],
            scroll: 0,
            is_analog: &[],
            analog_diff_and_reset: analog_fn,
            is_timer_play_on: false,
            is_note_end: false,
            window_hold: false,
            autoplay_mode: AutoplayMode::Play,
            now_millis: 0,
        }
    }

    // ---------------------------------------------------------------
    // Constructor tests
    // ---------------------------------------------------------------

    #[test]
    fn new_initializes_keybinds_for_beat_7k() {
        let proc = ControlInputProcessor::new(Mode::BEAT_7K);
        assert_eq!(
            proc.keybinds,
            vec![
                -1, 1, -1, 1, -1, 1, -1, 2, -2, -1, 1, -1, 1, -1, 1, -1, 2, -2
            ]
        );
    }

    #[test]
    fn new_initializes_keybinds_for_beat_5k() {
        let proc = ControlInputProcessor::new(Mode::BEAT_5K);
        assert_eq!(
            proc.keybinds,
            vec![-1, 1, -1, 1, -1, 2, -2, -1, 1, -1, 1, -1, 2, -2]
        );
    }

    #[test]
    fn new_initializes_defaults() {
        let proc = ControlInputProcessor::new(Mode::BEAT_7K);
        assert!(proc.enable_control);
        assert!(proc.enable_cursor);
        assert!(proc.is_change_lift);
        assert!(!proc.startpressed);
        assert!(!proc.selectpressed);
        assert!(!proc.start_and_select_pressed);
        assert_eq!(proc.exit_press_duration, 1000);
    }

    // ---------------------------------------------------------------
    // input() — no-op when no buttons pressed
    // ---------------------------------------------------------------

    #[test]
    fn input_no_buttons_returns_default_result() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();
        let mut ctx = make_context(&mut lr, &mut *analog);
        let result = proc.input(&mut ctx);
        assert!(!result.stop_play);
        assert!(result.play_speed.is_none());
        assert!(!result.clear_start);
        assert!(!result.clear_select);
        assert!(!result.reset_scroll);
    }

    // ---------------------------------------------------------------
    // Cursor UP/DOWN for lane cover
    // ---------------------------------------------------------------

    #[test]
    fn cursor_up_decreases_lanecover() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        let mut analog = noop_analog();
        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.control_key_up = true;
        proc.input(&mut ctx);

        // set_cover_value(-0.01) => lanecover 0.5 + (-0.01) = 0.49
        assert!((ctx.lanerender.lanecover() - 0.49).abs() < 0.001);
    }

    #[test]
    fn cursor_down_increases_lanecover() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        let mut analog = noop_analog();
        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.control_key_down = true;
        proc.input(&mut ctx);

        // set_cover_value(0.01) => lanecover 0.5 + 0.01 = 0.51
        assert!((ctx.lanerender.lanecover() - 0.51).abs() < 0.001);
    }

    #[test]
    fn cursor_not_retriggered_while_held() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        let mut analog = noop_analog();

        // First press
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.control_key_up = true;
            proc.input(&mut ctx);
        }
        // Second press (still held)
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.control_key_up = true;
            proc.input(&mut ctx);
        }
        // Should only change once
        assert!((lr.lanecover() - 0.49).abs() < 0.001);
    }

    #[test]
    fn cursor_disabled_when_enable_cursor_false() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        proc.enable_cursor = false;
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        let mut analog = noop_analog();
        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.control_key_up = true;
        proc.input(&mut ctx);

        // Should not change
        assert!((ctx.lanerender.lanecover() - 0.5).abs() < 0.001);
    }

    // ---------------------------------------------------------------
    // Mouse wheel for lane cover
    // ---------------------------------------------------------------

    #[test]
    fn scroll_adjusts_lanecover() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        let mut analog = noop_analog();
        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.scroll = 2; // scroll up
        let result = proc.input(&mut ctx);

        // set_cover_value(-2 * 0.005) = -0.01 => 0.5 + (-0.01) = 0.49
        assert!((ctx.lanerender.lanecover() - 0.49).abs() < 0.001);
        assert!(result.reset_scroll);
    }

    // ---------------------------------------------------------------
    // START double-press toggles lane cover
    // ---------------------------------------------------------------

    #[test]
    fn start_double_press_toggles_lanecover() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.enable_lanecover = false;

        let mut analog = noop_analog();

        // First press at t=1000 (use large timestamps to avoid startpressedtime=0 collision)
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.now_millis = 1000;
            proc.input(&mut ctx);
        }
        // Release
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = false;
            ctx.now_millis = 1050;
            proc.input(&mut ctx);
        }
        // Second press at t=1200 (within 500ms of first)
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.now_millis = 1200;
            proc.input(&mut ctx);
        }

        assert!(lr.is_enable_lanecover());
    }

    #[test]
    fn start_single_press_does_not_toggle_lanecover() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.enable_lanecover = false;

        let mut analog = noop_analog();

        // First press at t=1000
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.now_millis = 1000;
            proc.input(&mut ctx);
        }
        // Release
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = false;
            ctx.now_millis = 1050;
            proc.input(&mut ctx);
        }
        // Second press too late (>500ms after first)
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.now_millis = 1600;
            proc.input(&mut ctx);
        }

        assert!(!lr.is_enable_lanecover());
    }

    // ---------------------------------------------------------------
    // SELECT double-press toggles hidden
    // ---------------------------------------------------------------

    #[test]
    fn select_double_press_toggles_hidden() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.enable_hidden = false;

        let mut analog = noop_analog();

        // First press at t=1000
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.select_pressed = true;
            ctx.now_millis = 1000;
            proc.input(&mut ctx);
        }
        // Release
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.select_pressed = false;
            ctx.now_millis = 1050;
            proc.input(&mut ctx);
        }
        // Second press within 500ms
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.select_pressed = true;
            ctx.now_millis = 1200;
            proc.input(&mut ctx);
        }

        assert!(lr.is_enable_hidden());
    }

    // ---------------------------------------------------------------
    // START+SELECT toggles is_change_lift
    // ---------------------------------------------------------------

    #[test]
    fn start_and_select_toggles_change_lift() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        assert!(proc.is_change_lift);

        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        // Press both START+SELECT
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.select_pressed = true;
            ctx.now_millis = 100;
            proc.input(&mut ctx);
        }

        assert!(!proc.is_change_lift);
    }

    #[test]
    fn start_and_select_held_does_not_retoggle() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        // Press both
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.select_pressed = true;
            ctx.now_millis = 100;
            proc.input(&mut ctx);
        }
        assert!(!proc.is_change_lift);

        // Still held
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.select_pressed = true;
            ctx.now_millis = 200;
            proc.input(&mut ctx);
        }
        // Should still be false (not toggled back)
        assert!(!proc.is_change_lift);
    }

    // ---------------------------------------------------------------
    // START+SELECT held for exitPressDuration => stop play
    // ---------------------------------------------------------------

    #[test]
    fn start_select_held_beyond_duration_stops_play() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        proc.exit_press_duration = 100;

        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        // Frame 1: press both at t=0 (sets exitpressedtime implicitly)
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.select_pressed = true;
            ctx.now_millis = 0;
            let result = proc.input(&mut ctx);
            // Not yet exceeded duration (0 - 0 = 0, not > 100)
            assert!(!result.stop_play);
        }

        // Frame 2: still held at t=200 (200 - 0 > 100)
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.select_pressed = true;
            ctx.now_millis = 200;
            let result = proc.input(&mut ctx);
            assert!(result.stop_play);
            assert!(result.clear_start);
            assert!(result.clear_select);
        }
    }

    #[test]
    fn note_end_with_start_stops_play() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.is_note_end = true;
        ctx.start_pressed = true;
        ctx.now_millis = 100;
        let result = proc.input(&mut ctx);

        assert!(result.stop_play);
    }

    #[test]
    fn note_end_with_select_stops_play() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.is_note_end = true;
        ctx.select_pressed = true;
        ctx.now_millis = 100;
        let result = proc.input(&mut ctx);

        assert!(result.stop_play);
    }

    // ---------------------------------------------------------------
    // ESC stops play
    // ---------------------------------------------------------------

    #[test]
    fn escape_stops_play() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.control_key_escape_pressed = true;
        let result = proc.input(&mut ctx);

        assert!(result.stop_play);
    }

    // ---------------------------------------------------------------
    // Play speed changes (autoplay/replay only)
    // ---------------------------------------------------------------

    #[test]
    fn play_speed_num1_in_autoplay() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.autoplay_mode = AutoplayMode::Autoplay;
        ctx.control_key_num1 = true;
        let result = proc.input(&mut ctx);

        assert_eq!(result.play_speed, Some(25));
    }

    #[test]
    fn play_speed_num2_in_replay() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.autoplay_mode = AutoplayMode::Replay;
        ctx.control_key_num2 = true;
        let result = proc.input(&mut ctx);

        assert_eq!(result.play_speed, Some(50));
    }

    #[test]
    fn play_speed_num3_returns_200() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.autoplay_mode = AutoplayMode::Autoplay;
        ctx.control_key_num3 = true;
        let result = proc.input(&mut ctx);

        assert_eq!(result.play_speed, Some(200));
    }

    #[test]
    fn play_speed_num4_returns_300() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.autoplay_mode = AutoplayMode::Autoplay;
        ctx.control_key_num4 = true;
        let result = proc.input(&mut ctx);

        assert_eq!(result.play_speed, Some(300));
    }

    #[test]
    fn play_speed_default_100_in_autoplay() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.autoplay_mode = AutoplayMode::Autoplay;
        let result = proc.input(&mut ctx);

        assert_eq!(result.play_speed, Some(100));
    }

    #[test]
    fn play_speed_none_in_play_mode() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let mut analog = noop_analog();

        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.autoplay_mode = AutoplayMode::Play;
        ctx.control_key_num1 = true;
        let result = proc.input(&mut ctx);

        assert!(result.play_speed.is_none());
    }

    // ---------------------------------------------------------------
    // set_cover_value — lane cover / hidden / lift routing
    // ---------------------------------------------------------------

    #[test]
    fn set_cover_value_adjusts_lanecover_when_enabled() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        proc.set_cover_value(0.1, &mut lr);
        assert!((lr.lanecover() - 0.6).abs() < 0.001);
    }

    #[test]
    fn set_cover_value_adjusts_lanecover_when_lift_and_hidden_both_off() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = false;
        lr.enable_hidden = false;
        // lift is off by default

        proc.set_cover_value(0.1, &mut lr);
        assert!((lr.lanecover() - 0.6).abs() < 0.001);
    }

    #[test]
    fn set_cover_value_adjusts_hidden_when_hidden_on_lanecover_off() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.enable_lanecover = false;
        lr.enable_hidden = true;
        lr.set_hidden_cover(0.5);

        // value=0.1 => hidden = 0.5 - 0.1 = 0.4
        proc.set_cover_value(0.1, &mut lr);
        assert!((lr.hidden_cover() - 0.4).abs() < 0.001);
    }

    #[test]
    fn set_cover_value_adjusts_lift_when_lift_on_and_change_lift_true() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        proc.is_change_lift = true;
        let mut lr = make_lanerender();
        lr.enable_lanecover = false;
        lr.enable_hidden = false;
        lr.enable_lift = true;
        lr.set_lift_region(0.5);

        // value=0.1 => lift = 0.5 - 0.1 = 0.4
        proc.set_cover_value(0.1, &mut lr);
        assert!((lr.lift_region() - 0.4).abs() < 0.001);
    }

    #[test]
    fn set_cover_value_does_not_adjust_lift_when_change_lift_false() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        proc.is_change_lift = false;
        let mut lr = make_lanerender();
        lr.enable_lanecover = false;
        lr.enable_hidden = false;
        lr.enable_lift = true;
        lr.set_lift_region(0.5);

        proc.set_cover_value(0.1, &mut lr);
        assert!((lr.lift_region() - 0.5).abs() < 0.001);
    }

    // ---------------------------------------------------------------
    // START + keys: hispeed change
    // ---------------------------------------------------------------

    #[test]
    fn start_plus_key_changes_hispeed() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let initial_hispeed = lr.hispeed();

        let mut analog = noop_analog();

        // For BEAT_7K keybinds: [-1, 1, -1, 1, -1, 1, -1, 2, -2, ...]
        // key index 1 = keybind 1 = increase hispeed
        // Need 3 frames:
        // Frame 1: START just pressed, no keys -> hschanged.fill(true), startpressed = true
        // Frame 2: START held, key[1] NOT pressed -> process_start runs, hschanged[1] = false
        // Frame 3: START held, key[1] pressed -> process_start: hschanged[1]=false, keystate=true => trigger

        // Frame 1: START pressed (first press)
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.autoplay_mode = AutoplayMode::Play;
            ctx.now_millis = 1000;
            proc.input(&mut ctx);
        }
        // Frame 2: START held, key[1] NOT pressed (clears hschanged for that key)
        {
            let key_states = vec![false; 18];
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.key_states = &key_states;
            ctx.autoplay_mode = AutoplayMode::Play;
            ctx.now_millis = 1050;
            proc.input(&mut ctx);
        }
        // Frame 3: START held, key[1] NOW pressed
        {
            let key_states = vec![
                false, true, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false, false, false,
            ];
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = true;
            ctx.key_states = &key_states;
            ctx.autoplay_mode = AutoplayMode::Play;
            ctx.now_millis = 1100;
            proc.input(&mut ctx);
        }

        assert!(lr.hispeed() > initial_hispeed);
    }

    // ---------------------------------------------------------------
    // SELECT + keys: duration change
    // ---------------------------------------------------------------

    #[test]
    fn select_plus_key_changes_duration() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let initial_duration = lr.duration();

        let mut analog = noop_analog();

        // Frame 1: SELECT pressed (first press) -> hschanged.fill(true), selectpressed = true
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.select_pressed = true;
            ctx.autoplay_mode = AutoplayMode::Play;
            ctx.now_millis = 1000;
            proc.input(&mut ctx);
        }
        // Frame 2: SELECT held, key[1] NOT pressed -> process_select runs, hschanged[1] = false
        {
            let key_states = vec![false; 18];
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.select_pressed = true;
            ctx.key_states = &key_states;
            ctx.autoplay_mode = AutoplayMode::Play;
            ctx.now_millis = 1050;
            proc.input(&mut ctx);
        }
        // Frame 3: SELECT held, key[1] pressed -> hschanged[1]=false, keystate=true => trigger
        {
            let key_states = vec![
                false, true, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false, false, false,
            ];
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.select_pressed = true;
            ctx.key_states = &key_states;
            ctx.autoplay_mode = AutoplayMode::Play;
            ctx.now_millis = 1100;
            proc.input(&mut ctx);
        }

        assert_eq!(lr.duration(), initial_duration + 1);
    }

    // ---------------------------------------------------------------
    // Window hold: START logic triggered even when not pressing START
    // ---------------------------------------------------------------

    #[test]
    fn window_hold_triggers_start_logic() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.enable_lanecover = false;

        let mut analog = noop_analog();

        // First "press" via window hold at t=1000
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = false;
            ctx.window_hold = true;
            ctx.is_timer_play_on = true;
            ctx.is_note_end = false;
            ctx.now_millis = 1000;
            proc.input(&mut ctx);
        }
        // Release window hold conditions
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = false;
            ctx.window_hold = false;
            ctx.now_millis = 1050;
            proc.input(&mut ctx);
        }
        // Second "press" via window hold at t=1200 (within 500ms)
        {
            let mut ctx = make_context(&mut lr, &mut *analog);
            ctx.start_pressed = false;
            ctx.window_hold = true;
            ctx.is_timer_play_on = true;
            ctx.is_note_end = false;
            ctx.now_millis = 1200;
            proc.input(&mut ctx);
        }

        // Double-press should have toggled lane cover
        assert!(lr.is_enable_lanecover());
    }

    // ---------------------------------------------------------------
    // enable_control = false disables all control input
    // ---------------------------------------------------------------

    #[test]
    fn enable_control_false_disables_cursor_and_start_select() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        proc.enable_control = false;

        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        let mut analog = noop_analog();
        let mut ctx = make_context(&mut lr, &mut *analog);
        ctx.control_key_up = true;
        ctx.start_pressed = true;
        ctx.select_pressed = true;
        proc.input(&mut ctx);

        // Lane cover should not change
        assert!((ctx.lanerender.lanecover() - 0.5).abs() < 0.001);
    }

    // ---------------------------------------------------------------
    // configure() method
    // ---------------------------------------------------------------

    #[test]
    fn configure_sets_fields() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        proc.configure(0.002, 0.02, 600, true, 2000);
        assert!((proc.cover_change_margin_low - 0.002).abs() < f32::EPSILON);
        assert!((proc.cover_change_margin_high - 0.02).abs() < f32::EPSILON);
        assert_eq!(proc.cover_speed_switch_duration, 600);
        assert!(proc.hispeed_auto_adjust);
        assert_eq!(proc.exit_press_duration, 2000);
    }

    // ---------------------------------------------------------------
    // change_cover_value — digital scratch for lane cover
    // ---------------------------------------------------------------

    #[test]
    fn change_cover_value_digital_adjusts_lanecover() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        // For BEAT_7K: keybind index 7 = 2 (scratch up), index 8 = -2 (scratch down)
        // We simulate the digital scratch key being pressed
        let mut key_states = vec![false; 18];
        key_states[7] = true; // scratch up key
        let is_analog = vec![false; 18];
        let mut analog_fn = |_key: usize, _ms: i32| -> i32 { 0 };

        // Set lanecovertiming far in the past so the >50ms check passes
        proc.lanecovertiming = 0;
        proc.change_cover_value(
            7,
            true,
            &key_states,
            &mut lr,
            &is_analog,
            &mut analog_fn,
            1000,
        );

        // Should have adjusted lanecover by cover_change_margin_low (0.001)
        assert!((lr.lanecover() - 0.501).abs() < 0.001);
    }

    // ---------------------------------------------------------------
    // change_cover_value — analog scratch for lane cover
    // ---------------------------------------------------------------

    #[test]
    fn change_cover_value_analog_adjusts_lanecover() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        let key_states = vec![false; 18];
        let mut is_analog = vec![false; 18];
        is_analog[7] = true; // analog input
        let mut analog_fn = |_key: usize, _ms: i32| -> i32 { 5 }; // 5 ticks

        proc.change_cover_value(
            7,
            true,
            &key_states,
            &mut lr,
            &is_analog,
            &mut analog_fn,
            100,
        );

        // d_ticks = 5 * 1 (up) = 5, setCoverValue(5 * 0.001 = 0.005)
        // lanecover = 0.5 + 0.005 = 0.505
        assert!((lr.lanecover() - 0.505).abs() < 0.001);
    }

    // ---------------------------------------------------------------
    // change_duration — digital scratch
    // ---------------------------------------------------------------

    #[test]
    fn change_duration_digital_adjusts_duration() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let initial = lr.duration();

        let mut key_states = vec![false; 18];
        key_states[7] = true;
        let is_analog = vec![false; 18];
        let mut analog_fn = |_key: usize, _ms: i32| -> i32 { 0 };

        proc.lanecovertiming = 0;
        proc.change_duration(
            7,
            true,
            &key_states,
            &mut lr,
            &is_analog,
            &mut analog_fn,
            1000,
        );

        assert_eq!(lr.duration(), initial + 1);
    }

    // ---------------------------------------------------------------
    // change_duration — analog scratch
    // ---------------------------------------------------------------

    #[test]
    fn change_duration_analog_adjusts_duration() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        let initial = lr.duration();

        let key_states = vec![false; 18];
        let mut is_analog = vec![false; 18];
        is_analog[7] = true;
        let mut analog_fn = |_key: usize, _ms: i32| -> i32 { 3 }; // 3 ticks

        proc.change_duration(
            7,
            true,
            &key_states,
            &mut lr,
            &is_analog,
            &mut analog_fn,
            100,
        );

        // d_ticks = 3 * 1 (up) = 3
        assert_eq!(lr.duration(), initial + 3);
    }

    // ---------------------------------------------------------------
    // Digital scratch timing — high margin after long press
    // ---------------------------------------------------------------

    #[test]
    fn change_cover_value_digital_uses_high_margin_after_switch_duration() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        proc.cover_speed_switch_duration = 100;
        proc.cover_change_margin_low = 0.001;
        proc.cover_change_margin_high = 0.01;
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        let mut key_states = vec![false; 18];
        key_states[7] = true;
        let is_analog = vec![false; 18];
        let mut analog_fn = |_key: usize, _ms: i32| -> i32 { 0 };

        // First call at t=1000 (starts timing).
        // Set lanecovertiming far enough in the past so the >50ms check passes.
        proc.lanecovertiming = 0;
        proc.lane_cover_start_timing = i64::MIN;
        proc.change_cover_value(
            7,
            true,
            &key_states,
            &mut lr,
            &is_analog,
            &mut analog_fn,
            1000,
        );
        let after_first = lr.lanecover();

        // Second call at t=1200 (1200 - 1000 > 100 => high margin; 1200 - lanecovertiming > 50)
        proc.change_cover_value(
            7,
            true,
            &key_states,
            &mut lr,
            &is_analog,
            &mut analog_fn,
            1200,
        );
        let after_second = lr.lanecover();

        let first_delta = after_first - 0.5;
        let second_delta = after_second - after_first;

        // First delta should be low margin (0.001), second should be high margin (0.01)
        assert!(
            (first_delta - 0.001).abs() < 0.0001,
            "first delta should be ~0.001, got {}",
            first_delta
        );
        assert!(
            (second_delta - 0.01).abs() < 0.001,
            "second delta should be ~0.01, got {}",
            second_delta
        );
    }

    // ---------------------------------------------------------------
    // Lane cover start timing reset on key release
    // ---------------------------------------------------------------

    #[test]
    fn change_cover_value_digital_resets_timing_on_release() {
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        let mut lr = make_lanerender();
        lr.set_lanecover(0.5);
        lr.enable_lanecover = true;

        let is_analog = vec![false; 18];
        let mut analog_fn = |_key: usize, _ms: i32| -> i32 { 0 };

        // Press key
        let mut key_states = vec![false; 18];
        key_states[7] = true;
        proc.lanecovertiming = 0;
        proc.lane_cover_start_timing = i64::MIN;
        proc.change_cover_value(
            7,
            true,
            &key_states,
            &mut lr,
            &is_analog,
            &mut analog_fn,
            1000,
        );
        assert_ne!(proc.lane_cover_start_timing, i64::MIN);

        // Release key
        key_states[7] = false;
        proc.change_cover_value(
            7,
            true,
            &key_states,
            &mut lr,
            &is_analog,
            &mut analog_fn,
            1200,
        );
        assert_eq!(proc.lane_cover_start_timing, i64::MIN);
    }

    // ---------------------------------------------------------------
    // Lift-related set_cover_value integration with hidden
    // ---------------------------------------------------------------

    #[test]
    fn set_cover_value_hidden_on_lift_on_change_lift_true_adjusts_hidden() {
        // When both hidden and lift are on (lanecover off), hidden takes priority
        // in the Java code because `isEnableHidden()` is checked first
        let mut proc = ControlInputProcessor::new(Mode::BEAT_7K);
        proc.is_change_lift = true;
        let mut lr = make_lanerender();
        lr.enable_lanecover = false;
        lr.enable_hidden = true;
        lr.enable_lift = true;
        lr.set_hidden_cover(0.5);
        lr.set_lift_region(0.5);

        proc.set_cover_value(0.1, &mut lr);

        // Hidden gets adjusted (priority), lift stays
        assert!((lr.hidden_cover() - 0.4).abs() < 0.001);
        assert!((lr.lift_region() - 0.5).abs() < 0.001);
    }

    // ---------------------------------------------------------------
    // LaneRenderer enable_lift setter (needed for tests above)
    // ---------------------------------------------------------------

    // Note: set_enable_lift is tested via set_cover_value tests above.
    // If it doesn't exist, the tests will fail at compile time.
}

//! BMControllerInputProcessor - dedicated controller input processing
//!
//! Translated from: bms.player.beatoraja.input.BMControllerInputProcessor

use crate::bms_player_input_device::{BMSPlayerInputDevice, DeviceType};
use crate::stubs::{Controller, ControllerConfig};

/// BMKeys constants
pub struct BMKeys;

impl BMKeys {
    pub const BUTTON_1: i32 = 0;
    pub const BUTTON_2: i32 = 1;
    pub const BUTTON_3: i32 = 2;
    pub const BUTTON_4: i32 = 3;
    pub const BUTTON_5: i32 = 4;
    pub const BUTTON_6: i32 = 5;
    pub const BUTTON_7: i32 = 6;
    pub const BUTTON_8: i32 = 7;
    pub const BUTTON_9: i32 = 8;
    pub const BUTTON_10: i32 = 9;
    pub const BUTTON_11: i32 = 10;
    pub const BUTTON_12: i32 = 11;
    pub const BUTTON_13: i32 = 12;
    pub const BUTTON_14: i32 = 13;
    pub const BUTTON_15: i32 = 14;
    pub const BUTTON_16: i32 = 15;
    pub const BUTTON_17: i32 = 16;
    pub const BUTTON_18: i32 = 17;
    pub const BUTTON_19: i32 = 18;
    pub const BUTTON_20: i32 = 19;
    pub const BUTTON_21: i32 = 20;
    pub const BUTTON_22: i32 = 21;
    pub const BUTTON_23: i32 = 22;
    pub const BUTTON_24: i32 = 23;
    pub const BUTTON_25: i32 = 24;
    pub const BUTTON_26: i32 = 25;
    pub const BUTTON_27: i32 = 26;
    pub const BUTTON_28: i32 = 27;
    pub const BUTTON_29: i32 = 28;
    pub const BUTTON_30: i32 = 29;
    pub const BUTTON_31: i32 = 30;
    pub const BUTTON_32: i32 = 31;
    pub const AXIS1_PLUS: i32 = 32;
    pub const AXIS1_MINUS: i32 = 33;
    pub const AXIS2_PLUS: i32 = 34;
    pub const AXIS2_MINUS: i32 = 35;
    pub const AXIS3_PLUS: i32 = 36;
    pub const AXIS3_MINUS: i32 = 37;
    pub const AXIS4_PLUS: i32 = 38;
    pub const AXIS4_MINUS: i32 = 39;
    pub const AXIS5_PLUS: i32 = 40;
    pub const AXIS5_MINUS: i32 = 41;
    pub const AXIS6_PLUS: i32 = 42;
    pub const AXIS6_MINUS: i32 = 43;
    pub const AXIS7_PLUS: i32 = 44;
    pub const AXIS7_MINUS: i32 = 45;
    pub const AXIS8_PLUS: i32 = 46;
    pub const AXIS8_MINUS: i32 = 47;

    pub const MAXID: usize = 48;

    /// Text corresponding to controller keycodes
    const BMCODE: [&'static str; 48] = [
        "BUTTON 1",
        "BUTTON 2",
        "BUTTON 3",
        "BUTTON 4",
        "BUTTON 5",
        "BUTTON 6",
        "BUTTON 7",
        "BUTTON 8",
        "BUTTON 9",
        "BUTTON 10",
        "BUTTON 11",
        "BUTTON 12",
        "BUTTON 13",
        "BUTTON 14",
        "BUTTON 15",
        "BUTTON 16",
        "BUTTON 17",
        "BUTTON 18",
        "BUTTON 19",
        "BUTTON 20",
        "BUTTON 21",
        "BUTTON 22",
        "BUTTON 23",
        "BUTTON 24",
        "BUTTON 25",
        "BUTTON 26",
        "BUTTON 27",
        "BUTTON 28",
        "BUTTON 29",
        "BUTTON 30",
        "BUTTON 31",
        "BUTTON 32",
        "UP (AXIS 1 +)",
        "DOWN (AXIS 1 -)",
        "RIGHT (AXIS 2 +)",
        "LEFT (AXIS 2 -)",
        "AXIS 3 +",
        "AXIS 3 -",
        "AXIS 4 +",
        "AXIS 4 -",
        "AXIS 5 +",
        "AXIS 5 -",
        "AXIS 6 +",
        "AXIS 6 -",
        "AXIS 7 +",
        "AXIS 7 -",
        "AXIS 8 +",
        "AXIS 8 -",
    ];

    pub fn to_string(keycode: i32) -> &'static str {
        if keycode >= 0 && (keycode as usize) < Self::BMCODE.len() {
            Self::BMCODE[keycode as usize]
        } else {
            "Unknown"
        }
    }
}

const AXIS_LENGTH: usize = 8;

/// tick: minimum scratch movement
/// INFINITAS, DAO, YuanCon -> 0.00787
/// arcin board -> 0.00784
const TICK_MAX_SIZE: f32 = 0.009;

trait AnalogScratchAlgorithm: Send {
    fn analog_scratch_input(&mut self, current_scratch_x: f32, plus: bool) -> bool;
}

struct AnalogScratchAlgorithmVersion1 {
    /// Analog scratch threshold
    analog_scratch_threshold: i32,
    /// Scratch stop counter
    counter: i64,
    /// Analog scratch position (-1<->0<->1)
    old_scratch_x: f32,
    /// Analog scratch input flag
    scratch_active: bool,
    /// Analog scratch right rotation flag
    right_move_scratching: bool,
}

impl AnalogScratchAlgorithmVersion1 {
    fn new(analog_scratch_threshold: i32) -> Self {
        Self {
            analog_scratch_threshold,
            counter: 1,
            old_scratch_x: 10.0,
            scratch_active: false,
            right_move_scratching: false,
        }
    }
}

impl AnalogScratchAlgorithm for AnalogScratchAlgorithmVersion1 {
    fn analog_scratch_input(&mut self, current_scratch_x: f32, plus: bool) -> bool {
        if self.old_scratch_x > 1.0 {
            self.old_scratch_x = current_scratch_x;
            self.scratch_active = false;
            return false;
        }

        if self.old_scratch_x != current_scratch_x {
            // Analog scratch position movement occurred
            let mut now_right = false;
            if self.old_scratch_x < current_scratch_x {
                now_right = true;
                if (current_scratch_x - self.old_scratch_x)
                    > (1.0 - current_scratch_x + self.old_scratch_x)
                {
                    now_right = false;
                }
            } else if self.old_scratch_x > current_scratch_x {
                now_right = false;
                if (self.old_scratch_x - current_scratch_x)
                    > ((current_scratch_x + 1.0) - self.old_scratch_x)
                {
                    now_right = true;
                }
            }

            if self.scratch_active && (self.right_move_scratching != now_right) {
                // Left rotation -> right rotation
                self.right_move_scratching = now_right;
            } else if !self.scratch_active {
                // No movement -> rotation
                self.scratch_active = true;
                self.right_move_scratching = now_right;
            }

            self.counter = 0;
            self.old_scratch_x = current_scratch_x;
        }

        // counter > Threshold ... Stop Scratching.
        if self.counter > self.analog_scratch_threshold as i64 && self.scratch_active {
            self.scratch_active = false;
            self.counter = 0;
        }

        if self.counter == i64::MAX {
            self.counter = 0;
        }

        self.counter += 1;

        if plus {
            self.scratch_active && self.right_move_scratching
        } else {
            self.scratch_active && !self.right_move_scratching
        }
    }
}

struct AnalogScratchAlgorithmVersion2 {
    /// Analog scratch threshold
    analog_scratch_threshold: i32,
    /// Scratch stop counter
    counter: i64,
    /// (Mode 2) scratch movement count within threshold (2 -> scratch)
    analog_scratch_tick_counter: i32,
    /// Analog scratch position (-1<->0<->1)
    old_scratch_x: f32,
    /// Analog scratch input flag
    scratch_active: bool,
    /// Analog scratch right rotation flag
    right_move_scratching: bool,
}

impl AnalogScratchAlgorithmVersion2 {
    fn new(analog_scratch_threshold: i32) -> Self {
        Self {
            analog_scratch_threshold,
            counter: 1,
            analog_scratch_tick_counter: 0,
            old_scratch_x: 10.0,
            scratch_active: false,
            right_move_scratching: false,
        }
    }
}

impl AnalogScratchAlgorithm for AnalogScratchAlgorithmVersion2 {
    fn analog_scratch_input(&mut self, current_scratch_x: f32, plus: bool) -> bool {
        if self.old_scratch_x > 1.0 {
            self.old_scratch_x = current_scratch_x;
            self.scratch_active = false;
            return false;
        }

        if self.old_scratch_x != current_scratch_x {
            // Analog scratch position movement occurred
            let ticks = compute_analog_diff(self.old_scratch_x, current_scratch_x);
            let now_right = ticks >= 0;

            if self.scratch_active && (self.right_move_scratching != now_right) {
                // Left rotation -> right rotation
                self.right_move_scratching = now_right;
                self.scratch_active = false;
                self.analog_scratch_tick_counter = 0;
            } else if !self.scratch_active {
                // No movement -> rotation
                if self.analog_scratch_tick_counter == 0
                    || self.counter <= self.analog_scratch_threshold as i64
                {
                    self.analog_scratch_tick_counter += ticks.unsigned_abs() as i32;
                }
                // scratchActive=true
                if self.analog_scratch_tick_counter >= 2 {
                    self.scratch_active = true;
                    self.right_move_scratching = now_right;
                }
            }

            self.counter = 0;
            self.old_scratch_x = current_scratch_x;
        }

        // counter > 2*Threshold ... Stop Scratching.
        if self.counter > (self.analog_scratch_threshold as i64) * 2 {
            self.scratch_active = false;
            self.analog_scratch_tick_counter = 0;
            self.counter = 0;
        }

        self.counter += 1;

        if plus {
            self.scratch_active && self.right_move_scratching
        } else {
            self.scratch_active && !self.right_move_scratching
        }
    }
}

/// Dedicated controller input processing
pub struct BMControllerInputProcessor {
    pub(crate) controller: Controller,
    /// Device name
    name: String,
    /// Controller enabled
    enabled: bool,
    /// Button key assign
    buttons: Vec<i32>,
    /// Start key assign
    start: i32,
    /// Select key assign
    select: i32,
    /// Each AXIS value (-1.0 - 1.0)
    axis: [f32; AXIS_LENGTH],
    /// Each button state
    buttonstate: [bool; BMKeys::MAXID],
    /// Whether each button state has changed
    buttonchanged: [bool; BMKeys::MAXID],
    /// Each button state change time (us)
    buttontime: [i64; BMKeys::MAXID],
    /// Button state change re-acceptance time (ms)
    duration: i32,
    /// Last pressed button
    last_pressed_button: i32,
    /// JKOC_HACK (UP/DOWN false reaction prevention)
    jkoc: bool,
    /// Analog scratch algorithm (None = do not use analog scratch)
    analog_scratch_algorithm: Option<Vec<Box<dyn AnalogScratchAlgorithm>>>,
}

/// Callback interface for BMSPlayerInputProcessor methods called from BMControllerInputProcessor
pub trait BMControllerCallback {
    fn key_changed_from_controller(
        &mut self,
        device_index: usize,
        microtime: i64,
        key: usize,
        pressed: bool,
    );
    fn start_changed(&mut self, pressed: bool);
    fn set_select_pressed(&mut self, pressed: bool);
    fn set_analog_state(&mut self, key: usize, is_analog: bool, value: f32);
}

impl BMControllerInputProcessor {
    pub fn new(name: String, controller: Controller, controller_config: &ControllerConfig) -> Self {
        let mut proc = Self {
            controller,
            name,
            enabled: false,
            buttons: vec![
                BMKeys::BUTTON_4,
                BMKeys::BUTTON_7,
                BMKeys::BUTTON_3,
                BMKeys::BUTTON_8,
                BMKeys::BUTTON_2,
                BMKeys::BUTTON_5,
                BMKeys::AXIS2_PLUS,
                BMKeys::AXIS1_PLUS,
                BMKeys::AXIS2_MINUS,
            ],
            start: BMKeys::BUTTON_9,
            select: BMKeys::BUTTON_10,
            axis: [0.0; AXIS_LENGTH],
            buttonstate: [false; BMKeys::MAXID],
            buttonchanged: [false; BMKeys::MAXID],
            buttontime: [0; BMKeys::MAXID],
            duration: 16,
            last_pressed_button: -1,
            jkoc: false,
            analog_scratch_algorithm: None,
        };
        proc.set_config(controller_config);
        proc
    }

    pub fn set_config(&mut self, controller_config: &ControllerConfig) {
        self.buttons = controller_config.get_key_assign().to_vec();
        self.start = controller_config.get_start();
        self.select = controller_config.get_select();
        self.duration = controller_config.get_duration();
        self.jkoc = controller_config.get_jkoc();

        if controller_config.is_analog_scratch() {
            let mut analog_scratch_algorithm: Vec<Box<dyn AnalogScratchAlgorithm>> =
                Vec::with_capacity(AXIS_LENGTH);
            let analog_scratch_threshold = controller_config.get_analog_scratch_threshold();
            for _i in 0..AXIS_LENGTH {
                match controller_config.get_analog_scratch_mode() {
                    ControllerConfig::ANALOG_SCRATCH_VER_1 => {
                        analog_scratch_algorithm.push(Box::new(
                            AnalogScratchAlgorithmVersion1::new(analog_scratch_threshold),
                        ));
                    }
                    ControllerConfig::ANALOG_SCRATCH_VER_2 => {
                        analog_scratch_algorithm.push(Box::new(
                            AnalogScratchAlgorithmVersion2::new(analog_scratch_threshold),
                        ));
                    }
                    _ => {
                        // default: push a v1
                        analog_scratch_algorithm.push(Box::new(
                            AnalogScratchAlgorithmVersion1::new(analog_scratch_threshold),
                        ));
                    }
                }
            }
            self.analog_scratch_algorithm = Some(analog_scratch_algorithm);
        } else {
            self.analog_scratch_algorithm = None;
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn poll(
        &mut self,
        microtime: i64,
        callback: &mut dyn BMControllerCallback,
        device_index: usize,
    ) {
        if !self.enabled {
            return;
        }

        // AXIS update
        for i in 0..AXIS_LENGTH {
            self.axis[i] = self.controller.get_axis(i as i32);
        }

        for button in 0..self.buttonstate.len() {
            if microtime >= self.buttontime[button] + (self.duration as i64) * 1000 {
                let prev = self.buttonstate[button];
                if button as i32 <= BMKeys::BUTTON_32 {
                    self.buttonstate[button] = self.controller.get_button(button as i32);
                } else if self.jkoc {
                    if button as i32 == BMKeys::AXIS1_PLUS {
                        self.buttonstate[button] = (self.axis[0] > 0.9) || (self.axis[3] > 0.9);
                    } else if button as i32 == BMKeys::AXIS1_MINUS {
                        self.buttonstate[button] = (self.axis[0] < -0.9) || (self.axis[3] < -0.9);
                    } else {
                        self.buttonstate[button] = false;
                    }
                } else {
                    let axis_index = ((button as i32) - BMKeys::AXIS1_PLUS) / 2;
                    let plus = ((button as i32) - BMKeys::AXIS1_PLUS) % 2 == 0;
                    self.buttonstate[button] = self.scratch_input(axis_index, plus);
                }

                let changed = prev != self.buttonstate[button];
                self.buttonchanged[button] = changed;
                if changed {
                    self.buttontime[button] = microtime;
                }

                if !prev && self.buttonstate[button] {
                    self.set_last_pressed_button(button as i32);
                }
            }
        }

        for i in 0..self.buttons.len() {
            let button = self.buttons[i];
            if button >= 0
                && (button as usize) < BMKeys::MAXID
                && self.buttonchanged[button as usize]
            {
                callback.key_changed_from_controller(
                    device_index,
                    microtime,
                    i,
                    self.buttonstate[button as usize],
                );
                self.buttonchanged[button as usize] = false;
            }
        }

        if self.start >= 0
            && (self.start as usize) < BMKeys::MAXID
            && self.buttonchanged[self.start as usize]
        {
            callback.start_changed(self.buttonstate[self.start as usize]);
            self.buttonchanged[self.start as usize] = false;
        }
        if self.select >= 0
            && (self.select as usize) < BMKeys::MAXID
            && self.buttonchanged[self.select as usize]
        {
            callback.set_select_pressed(self.buttonstate[self.select as usize]);
            self.buttonchanged[self.select as usize] = false;
        }

        let is_analog = !self.jkoc && self.analog_scratch_algorithm.is_some();
        for i in 0..self.buttons.len() {
            let button = self.buttons[i];
            if button < 0 || (button as usize) >= BMKeys::MAXID {
                continue;
            }
            if is_analog && button >= BMKeys::AXIS1_PLUS {
                let analog_value = self.get_analog_value(button);
                callback.set_analog_state(i, true, analog_value);
            } else {
                callback.set_analog_state(i, false, 0.0);
            }
        }
    }

    fn get_analog_value(&self, button: i32) -> f32 {
        // assume isAnalog(button) == true.
        let axis_index = ((button - BMKeys::AXIS1_PLUS) / 2) as usize;
        let plus = (button - BMKeys::AXIS1_PLUS) % 2 == 0;
        let value = self.controller.get_axis(axis_index as i32);
        if plus { value } else { -value }
    }

    fn scratch_input(&mut self, axis_index: i32, plus: bool) -> bool {
        if let Some(ref mut analog_scratch_algorithm) = self.analog_scratch_algorithm {
            // Analog scratch
            let idx = axis_index as usize;
            if idx < analog_scratch_algorithm.len() {
                analog_scratch_algorithm[idx].analog_scratch_input(self.axis[idx], plus)
            } else {
                false
            }
        } else {
            // Do not use analog scratch
            let idx = axis_index as usize;
            if plus {
                self.axis[idx] > 0.9
            } else {
                self.axis[idx] < -0.9
            }
        }
    }

    pub fn get_last_pressed_button(&self) -> i32 {
        self.last_pressed_button
    }

    pub fn set_last_pressed_button(&mut self, last_pressed_button: i32) {
        self.last_pressed_button = last_pressed_button;
    }

    pub fn set_enable(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl BMSPlayerInputDevice for BMControllerInputProcessor {
    fn device_type(&self) -> DeviceType {
        DeviceType::BmController
    }

    fn clear(&mut self) {
        self.buttonchanged.fill(false);
        self.buttontime.fill(i64::MIN);
        self.last_pressed_button = -1;
    }
}

pub fn compute_analog_diff(old_value: f32, new_value: f32) -> i32 {
    let mut analog_diff = new_value - old_value;
    if analog_diff > 1.0 {
        analog_diff -= 2.0 + TICK_MAX_SIZE / 2.0;
    } else if analog_diff < -1.0 {
        analog_diff += 2.0 + TICK_MAX_SIZE / 2.0;
    }
    analog_diff /= TICK_MAX_SIZE;
    if analog_diff > 0.0 {
        analog_diff.ceil() as i32
    } else {
        analog_diff.floor() as i32
    }
}

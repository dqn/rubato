//! MouseScratchInput - mouse-as-scratch input processing
//!
//! Translated from: bms.player.beatoraja.input.MouseScratchInput

use crate::keyboard_input_processor::KeyboardCallback;
use crate::stubs::{GdxGraphics, GdxInput, KeyboardConfig, MouseScratchConfig};

const _MOUSESCRATCH_RIGHT: usize = 0;
const _MOUSESCRATCH_LEFT: usize = 1;
const _MOUSESCRATCH_UP: usize = 2;
const _MOUSESCRATCH_DOWN: usize = 3;

/// Mouse-as-scratch input
pub struct MouseScratchInput {
    keys: Vec<i32>,
    control: Vec<i32>,

    mouse_scratch_state: [bool; 4],
    mouse_scratch_changed: [bool; 4],

    mouse_to_analog: Option<MouseToAnalog>,
    /// Mouse scratch algorithm (None = do not use mouse scratch)
    mouse_scratch_algorithm: [Option<Box<dyn MouseScratchAlgorithm>>; 2],
    /// Last pressed mouse scratch
    pub last_mouse_scratch: i32,
    /// Minimum key input interval (Java: stored for potential future use)
    _duration: i32,
    /// Use mouse scratch
    mouse_scratch_enabled: bool,
    /// Scratch stop threshold (ms) (Java: stored for potential future use)
    _mouse_scratch_time_threshold: i32,
    /// Scratch distance (Java: stored for potential future use)
    _mouse_scratch_distance: i32,
}

impl MouseScratchInput {
    pub fn new(config: &KeyboardConfig) -> Self {
        let mut input = Self {
            keys: vec![],
            control: vec![],
            mouse_scratch_state: [false; 4],
            mouse_scratch_changed: [false; 4],
            mouse_to_analog: None,
            mouse_scratch_algorithm: [None, None],
            last_mouse_scratch: -1,
            _duration: 0,
            mouse_scratch_enabled: false,
            _mouse_scratch_time_threshold: 150,
            _mouse_scratch_distance: 150,
        };
        input.set_config(config);
        input
    }

    pub fn poll(&mut self, microtime: i64, callback: &mut dyn KeyboardCallback) {
        let presstime = microtime / 1000;
        // MOUSE update
        if self.mouse_scratch_enabled {
            if let Some(ref mut mta) = self.mouse_to_analog {
                mta.update();
            }
            // Read current positions before mutably borrowing algorithms
            let positions: [i32; 2] = if let Some(ref mta) = self.mouse_to_analog {
                [mta.distance_moved(true), mta.distance_moved(false)]
            } else {
                [0, 0]
            };
            for (i, position) in positions
                .iter()
                .enumerate()
                .take(self.mouse_scratch_algorithm.len())
            {
                if let Some(ref mut alg) = self.mouse_scratch_algorithm[i] {
                    alg.update(presstime, *position);
                }
            }

            for mouse_input in 0..self.mouse_scratch_state.len() {
                let prev = self.mouse_scratch_state[mouse_input];
                if let Some(ref alg) = self.mouse_scratch_algorithm[mouse_input / 2] {
                    self.mouse_scratch_state[mouse_input] =
                        alg.is_scratch_active(mouse_input % 2 == 0);
                }
                if prev != self.mouse_scratch_state[mouse_input] {
                    self.mouse_scratch_changed[mouse_input] = true;
                    if !prev {
                        self.last_mouse_scratch = mouse_input as i32;
                    }
                }
            }

            for (i, &axis) in self.keys.iter().enumerate() {
                if axis >= 0 && self.mouse_scratch_changed[axis as usize] {
                    callback.key_changed_from_keyboard(
                        microtime,
                        i,
                        self.mouse_scratch_state[axis as usize],
                    );
                    self.mouse_scratch_changed[axis as usize] = false;
                }
            }

            if !self.control.is_empty()
                && self.control[0] >= 0
                && self.mouse_scratch_changed[self.control[0] as usize]
            {
                callback.start_changed(self.mouse_scratch_state[self.control[0] as usize]);
                self.mouse_scratch_changed[self.control[0] as usize] = false;
            }

            if self.control.len() > 1
                && self.control[1] >= 0
                && self.mouse_scratch_changed[self.control[1] as usize]
            {
                callback.set_select_pressed(self.mouse_scratch_state[self.control[1] as usize]);
                self.mouse_scratch_changed[self.control[1] as usize] = false;
            }

            for (i, &key) in self.keys.iter().enumerate() {
                if key >= 0 {
                    let value = self.mouse_analog_value(key);
                    callback.set_analog_state(i, true, value);
                }
            }
        }
    }

    pub fn set_config(&mut self, config: &KeyboardConfig) {
        let msconfig = &config.mouse_scratch_config;
        self.keys = msconfig.keys.to_vec();
        self._duration = config.duration;
        self.control = vec![msconfig.start, msconfig.select];

        self.mouse_scratch_enabled = msconfig.mouse_scratch_enabled;
        self._mouse_scratch_time_threshold = msconfig.mouse_scratch_time_threshold;
        self._mouse_scratch_distance = msconfig.mouse_scratch_distance;
        if self.mouse_scratch_enabled {
            let mouse_to_analog = MouseToAnalog::new(msconfig.mouse_scratch_distance);
            for (i, alg_slot) in self.mouse_scratch_algorithm.iter_mut().enumerate() {
                let x_axis = i == 0;
                match msconfig.mouse_scratch_mode {
                    MouseScratchConfig::MOUSE_SCRATCH_VER_1 => {
                        *alg_slot = Some(Box::new(MouseScratchAlgorithmVersion1::new(
                            msconfig.mouse_scratch_time_threshold,
                            &mouse_to_analog,
                            x_axis,
                        )));
                    }
                    MouseScratchConfig::MOUSE_SCRATCH_VER_2 => {
                        *alg_slot = Some(Box::new(MouseScratchAlgorithmVersion2::new(
                            msconfig.mouse_scratch_time_threshold,
                            &mouse_to_analog,
                            x_axis,
                        )));
                    }
                    _ => {}
                }
            }
            self.mouse_to_analog = Some(mouse_to_analog);
        } else {
            self.mouse_to_analog = None;
            for alg in &mut self.mouse_scratch_algorithm {
                *alg = None;
            }
        }
    }

    pub fn clear(&mut self) {
        //Arrays.fill(keytime, -duration);
        for alg in self.mouse_scratch_algorithm.iter_mut().flatten() {
            alg.reset();
        }
        self.last_mouse_scratch = -1;
    }

    fn mouse_analog_value(&self, mouse_input: i32) -> f32 {
        let plus = mouse_input % 2 == 0;
        let x_axis = mouse_input < 2;
        let value = if let Some(ref mta) = self.mouse_to_analog {
            mta.analog_value(x_axis)
        } else {
            0.0
        };
        if plus { value } else { -value }
    }

    pub fn last_mouse_scratch(&self) -> i32 {
        self.last_mouse_scratch
    }
}

/// MouseToAnalog
pub struct MouseToAnalog {
    scratch_distance: i32,
    tick_length: i32,
    domain: i32,

    total_x_distance_moved: i32,
    total_y_distance_moved: i32,
}

impl MouseToAnalog {
    pub const TICKS_FOR_SCRATCH: i32 = 2;

    pub fn new(scratch_distance: i32) -> Self {
        let tick_length = 1.max(scratch_distance / Self::TICKS_FOR_SCRATCH);
        let domain = 256 * tick_length;
        Self {
            scratch_distance,
            tick_length,
            domain,
            total_x_distance_moved: 0,
            total_y_distance_moved: 0,
        }
    }

    pub fn update(&mut self) {
        let x_distance_moved = GdxInput::get_x() - GdxGraphics::get_width() / 2;
        let y_distance_moved = GdxInput::get_y() - GdxGraphics::get_height() / 2;
        GdxInput::set_cursor_position(GdxGraphics::get_width() / 2, GdxGraphics::get_height() / 2);

        self.total_x_distance_moved =
            ((self.total_x_distance_moved + x_distance_moved) % self.domain + self.domain)
                % self.domain;
        self.total_y_distance_moved =
            ((self.total_y_distance_moved + y_distance_moved) % self.domain + self.domain)
                % self.domain;
    }

    pub fn scratch_distance(&self) -> i32 {
        self.scratch_distance
    }

    pub fn compute_distance_diff(&self, v1: i32, v2: i32) -> i32 {
        let v = v2 - v1;
        if v >= self.domain / 2 {
            return v - self.domain;
        }
        if v < -self.domain / 2 {
            return v + self.domain;
        }
        v
    }

    pub fn distance_moved(&self, x_axis: bool) -> i32 {
        if x_axis {
            self.total_x_distance_moved
        } else {
            self.total_y_distance_moved
        }
    }

    pub fn analog_value(&self, x_axis: bool) -> f32 {
        (self.distance_moved(x_axis) % 256) as f32 / 128.0 - 1.0
    }
}

/// MouseScratchAlgorithm trait
trait MouseScratchAlgorithm: Send {
    fn is_scratch_active(&self, positive: bool) -> bool;
    fn update(&mut self, presstime: i64, curr_position: i32);
    fn reset(&mut self);
}

/// Base helper for time diff calculation
fn get_time_diff(lastpresstime: &mut i64, presstime: i64) -> i64 {
    if *lastpresstime < 0 {
        *lastpresstime = presstime;
        return 0;
    }
    let dtime = presstime - *lastpresstime;
    *lastpresstime = presstime;
    dtime
}

/// MouseScratchAlgorithmVersion1
struct MouseScratchAlgorithmVersion1 {
    scratch_duration: i32,
    _x_axis: bool,

    // We store a copy of scratch_distance/tick_length/domain from MouseToAnalog
    // for computing distance diff without borrowing MouseToAnalog
    _mta_tick_length: i32,
    mta_domain: i32,

    prev_position: i32,
    remaining_time: i32,

    current_scratch: i32,
    lastpresstime: i64,
}

impl MouseScratchAlgorithmVersion1 {
    fn new(scratch_duration: i32, mouse_to_analog: &MouseToAnalog, x_axis: bool) -> Self {
        let prev_position = mouse_to_analog.distance_moved(x_axis);
        Self {
            scratch_duration,
            _x_axis: x_axis,
            _mta_tick_length: mouse_to_analog.tick_length,
            mta_domain: mouse_to_analog.domain,
            prev_position,
            remaining_time: 0,
            current_scratch: 0,
            lastpresstime: -1,
        }
    }

    fn compute_distance_diff_local(&self, v1: i32, v2: i32) -> i32 {
        let v = v2 - v1;
        if v >= self.mta_domain / 2 {
            return v - self.mta_domain;
        }
        if v < -self.mta_domain / 2 {
            return v + self.mta_domain;
        }
        v
    }
}

impl MouseScratchAlgorithm for MouseScratchAlgorithmVersion1 {
    fn is_scratch_active(&self, positive: bool) -> bool {
        if positive {
            self.current_scratch > 0
        } else {
            self.current_scratch < 0
        }
    }

    fn update(&mut self, presstime: i64, curr_position: i32) {
        let dtime = get_time_diff(&mut self.lastpresstime, presstime);

        let d_ticks = self.compute_distance_diff_local(self.prev_position, curr_position);
        self.prev_position = curr_position;

        if d_ticks > 0 {
            self.remaining_time = self.scratch_duration;
            self.current_scratch = 1;
        } else if d_ticks < 0 {
            self.remaining_time = self.scratch_duration;
            self.current_scratch = -1;
        } else if self.remaining_time > 0 {
            self.remaining_time -= dtime as i32;
        } else {
            self.current_scratch = 0;
        }
    }

    fn reset(&mut self) {
        self.lastpresstime = -1;
    }
}

/// MouseScratchAlgorithmVersion2
struct MouseScratchAlgorithmVersion2 {
    scratch_duration: i32,
    scratch_distance: i32,
    scratch_reverse_distance: i32,
    _x_axis: bool,

    _mta_tick_length: i32,
    mta_domain: i32,

    current_scratch: i32,

    prev_position: i32,

    positive_no_movement_time: i32,
    negative_no_movement_time: i32,

    positive_distance: i32,
    negative_distance: i32,

    lastpresstime: i64,
}

impl MouseScratchAlgorithmVersion2 {
    fn new(scratch_duration: i32, mouse_to_analog: &MouseToAnalog, x_axis: bool) -> Self {
        let scratch_distance = mouse_to_analog.scratch_distance();
        let scratch_reverse_distance = scratch_distance / 3;
        let prev_position = mouse_to_analog.distance_moved(x_axis);
        Self {
            scratch_duration,
            scratch_distance,
            scratch_reverse_distance,
            _x_axis: x_axis,
            _mta_tick_length: mouse_to_analog.tick_length,
            mta_domain: mouse_to_analog.domain,
            current_scratch: 0,
            prev_position,
            positive_no_movement_time: 0,
            negative_no_movement_time: 0,
            positive_distance: 0,
            negative_distance: 0,
            lastpresstime: -1,
        }
    }

    fn compute_distance_diff_local(&self, v1: i32, v2: i32) -> i32 {
        let v = v2 - v1;
        if v >= self.mta_domain / 2 {
            return v - self.mta_domain;
        }
        if v < -self.mta_domain / 2 {
            return v + self.mta_domain;
        }
        v
    }
}

impl MouseScratchAlgorithm for MouseScratchAlgorithmVersion2 {
    fn is_scratch_active(&self, positive: bool) -> bool {
        if positive {
            self.current_scratch > 0
        } else {
            self.current_scratch < 0
        }
    }

    fn update(&mut self, presstime: i64, curr_position: i32) {
        let dtime = get_time_diff(&mut self.lastpresstime, presstime);
        let distance_diff = self.compute_distance_diff_local(self.prev_position, curr_position);
        self.prev_position = curr_position;

        if self.positive_distance == 0 {
            self.positive_no_movement_time = 0;
        }
        if self.negative_distance == 0 {
            self.negative_no_movement_time = 0;
        }
        self.positive_distance = 0.max(self.positive_distance + distance_diff);
        self.negative_distance = 0.max(self.negative_distance - distance_diff);
        self.positive_no_movement_time += dtime as i32;
        self.negative_no_movement_time += dtime as i32;

        if self.positive_distance > 0 {
            if self.current_scratch == -1 && self.positive_distance >= self.scratch_reverse_distance
            {
                self.current_scratch = 0;
                self.negative_distance = 0;
                self.negative_no_movement_time = 0;
            }
            if self.positive_distance > self.scratch_distance {
                self.current_scratch = 1;
                self.positive_no_movement_time = 0;
                self.positive_distance = self.scratch_distance;
            }
        }
        if self.negative_distance > 0 {
            if self.current_scratch == 1 && self.negative_distance >= self.scratch_reverse_distance
            {
                self.current_scratch = 0;
                self.positive_distance = 0;
                self.positive_no_movement_time = 0;
            }
            if self.negative_distance > self.scratch_distance {
                self.current_scratch = -1;
                self.negative_no_movement_time = 0;
                self.negative_distance = self.scratch_distance;
            }
        }
        if self.positive_no_movement_time >= self.scratch_duration {
            self.positive_distance = 0;
            if self.current_scratch == 1 {
                self.current_scratch = 0;
            }
        }
        if self.negative_no_movement_time >= self.scratch_duration {
            self.negative_distance = 0;
            if self.current_scratch == -1 {
                self.current_scratch = 0;
            }
        }
    }

    fn reset(&mut self) {
        self.lastpresstime = -1;
    }
}

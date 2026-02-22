#![allow(dead_code)]

use crate::{Controller, ControllerListener};

/// Corresponds to bms.player.beatoraja.controller.Lwjgl3Controller
///
/// Individual controller with axis/button state.
/// Stores current axis and button state, polls GLFW joystick state each frame,
/// and fires events on state changes.
pub struct Lwjgl3Controller {
    /// Per-controller listeners
    pub listeners: Vec<Box<dyn ControllerListener>>,
    /// GLFW joystick index
    pub index: i32,
    /// Current axis state
    pub axis_state: Vec<f32>,
    /// Current button state
    pub button_state: Vec<bool>,
    /// Controller name
    pub name: String,
}

impl Lwjgl3Controller {
    /// Corresponds to Lwjgl3Controller(Lwjgl3ControllerManager, int)
    ///
    /// Creates a new controller for the given GLFW joystick index.
    /// Queries GLFW for the number of axes and buttons to initialize state arrays.
    pub fn new(_manager_index: usize, index: i32) -> Self {
        // In Java:
        //   this.axisState = new float[GLFW.glfwGetJoystickAxes(index).limit()];
        //   this.buttonState = new boolean[GLFW.glfwGetJoystickButtons(index).limit()];
        //   this.name = GLFW.glfwGetJoystickName(index);
        // All GLFW calls are stubbed — actual integration uses gilrs.
        log::warn!(
            "not yet implemented: GLFW/gilrs integration: query joystick axes/buttons/name for index {}",
            index
        );
        Self::new_with_state(index, 0, 0, format!("Controller {}", index))
    }

    /// Creates a controller with pre-initialized state (for testing or manual construction).
    pub fn new_with_state(index: i32, num_axes: usize, num_buttons: usize, name: String) -> Self {
        Lwjgl3Controller {
            listeners: Vec::new(),
            index,
            axis_state: vec![0.0; num_axes],
            button_state: vec![false; num_buttons],
            name,
        }
    }

    /// Corresponds to Lwjgl3Controller.pollState()
    ///
    /// Polls the GLFW joystick state and fires events on changes.
    /// Returns (disconnected, axis_changes, button_changes) for the manager to process.
    ///
    /// The returned axis_changes are (axis_code, new_value) tuples.
    /// The returned button_changes are (button_code, pressed) tuples.
    pub fn poll_state(&mut self) -> PollResult {
        // In Java:
        //   if(!GLFW.glfwJoystickPresent(index)) {
        //       manager.disconnected(this);
        //       return;
        //   }
        //   FloatBuffer axes = GLFW.glfwGetJoystickAxes(index);
        //   if(axes == null) { manager.disconnected(this); return; }
        //   ByteBuffer buttons = GLFW.glfwGetJoystickButtons(index);
        //   if(buttons == null) { manager.disconnected(this); return; }

        // GLFW calls stubbed — actual integration uses gilrs.
        // When integrated, the logic is:
        //   1. Check if joystick is still present; if not, return disconnected.
        //   2. Read new axis values; if null, return disconnected.
        //   3. Read new button values; if null, return disconnected.
        //   4. Compare with stored state, fire local listener events on changes.
        //   5. Return changes for manager-level listener dispatch.
        log::warn!(
            "not yet implemented: GLFW/gilrs integration: poll joystick state for index {}",
            self.index
        );
        PollResult::Disconnected
    }

    /// Processes axis state changes and fires local listener events.
    /// Called with new axis values read from the gamepad API.
    #[allow(clippy::needless_range_loop)]
    pub fn process_axis_changes(&mut self, new_axes: &[f32]) -> Vec<(i32, f32)> {
        let mut changes = Vec::new();

        // for(int i = 0; i < axes.limit(); i++) {
        //     if(axisState[i] != axes.get(i)) {
        //         for(ControllerListener listener: listeners) {
        //             if (listener.axisMoved(this, i, axes.get(i))) break;
        //         }
        //         manager.axisChanged(this, i, axes.get(i));
        //     }
        //     axisState[i] = axes.get(i);
        // }
        for i in 0..new_axes.len().min(self.axis_state.len()) {
            if (self.axis_state[i] - new_axes[i]).abs() > f32::EPSILON {
                // Fire local listeners
                for listener in &mut self.listeners {
                    if listener.axis_moved(0, i as i32, new_axes[i]) {
                        break;
                    }
                }
                changes.push((i as i32, new_axes[i]));
            }
            self.axis_state[i] = new_axes[i];
        }

        changes
    }

    /// Processes button state changes and fires local listener events.
    /// Called with new button values read from the gamepad API.
    #[allow(clippy::needless_range_loop)]
    pub fn process_button_changes(&mut self, new_buttons: &[bool]) -> Vec<(i32, bool)> {
        let mut changes = Vec::new();

        // for(int i = 0; i < buttons.limit(); i++) {
        //     if(buttonState[i] != (buttons.get(i) == GLFW.GLFW_PRESS)) {
        //         for(ControllerListener listener: listeners) {
        //             if(buttons.get(i) == GLFW.GLFW_PRESS) {
        //                 if (listener.buttonDown(this, i)) break;
        //             } else {
        //                 if (listener.buttonUp(this, i)) break;
        //             }
        //         }
        //         manager.buttonChanged(this, i, buttons.get(i) == GLFW.GLFW_PRESS);
        //     }
        //     buttonState[i] = buttons.get(i) == GLFW.GLFW_PRESS;
        // }
        for i in 0..new_buttons.len().min(self.button_state.len()) {
            if self.button_state[i] != new_buttons[i] {
                // Fire local listeners
                for listener in &mut self.listeners {
                    if new_buttons[i] {
                        if listener.button_down(0, i as i32) {
                            break;
                        }
                    } else if listener.button_up(0, i as i32) {
                        break;
                    }
                }
                changes.push((i as i32, new_buttons[i]));
            }
            self.button_state[i] = new_buttons[i];
        }

        changes
    }

    /// Adds a listener for this controller's events.
    pub fn add_listener(&mut self, listener: Box<dyn ControllerListener>) {
        self.listeners.push(listener);
    }

    /// Removes a listener by index.
    pub fn remove_listener(&mut self, index: usize) {
        if index < self.listeners.len() {
            self.listeners.remove(index);
        }
    }
}

/// Result of polling a controller's state.
pub enum PollResult {
    /// Controller is still connected; contains axis and button changes.
    Connected {
        axis_changes: Vec<(i32, f32)>,
        button_changes: Vec<(i32, bool)>,
    },
    /// Controller has been disconnected.
    Disconnected,
}

impl Controller for Lwjgl3Controller {
    /// Corresponds to Lwjgl3Controller.getButton(int)
    fn get_button(&self, button_code: i32) -> bool {
        // if(buttonCode < 0 || buttonCode >= buttonState.length) {
        //     return false;
        // }
        // return buttonState[buttonCode];
        if button_code < 0 || button_code as usize >= self.button_state.len() {
            return false;
        }
        self.button_state[button_code as usize]
    }

    /// Corresponds to Lwjgl3Controller.getAxis(int)
    fn get_axis(&self, axis_code: i32) -> f32 {
        // if(axisCode < 0 || axisCode >= axisState.length) {
        //     return 0;
        // }
        // return axisState[axisCode];
        if axis_code < 0 || axis_code as usize >= self.axis_state.len() {
            return 0.0;
        }
        self.axis_state[axis_code as usize]
    }

    /// Corresponds to Lwjgl3Controller.getName()
    fn get_name(&self) -> &str {
        &self.name
    }

    /// Corresponds to Lwjgl3Controller.getUniqueId()
    fn get_unique_id(&self) -> Option<String> {
        // return null;
        None
    }

    /// Corresponds to Lwjgl3Controller.getMinButtonIndex()
    fn get_min_button_index(&self) -> i32 {
        0
    }

    /// Corresponds to Lwjgl3Controller.getMaxButtonIndex()
    fn get_max_button_index(&self) -> i32 {
        0
    }

    /// Corresponds to Lwjgl3Controller.getAxisCount()
    fn get_axis_count(&self) -> i32 {
        0
    }

    /// Corresponds to Lwjgl3Controller.isConnected()
    fn is_connected(&self) -> bool {
        false
    }

    /// Corresponds to Lwjgl3Controller.canVibrate()
    fn can_vibrate(&self) -> bool {
        false
    }

    /// Corresponds to Lwjgl3Controller.isVibrating()
    fn is_vibrating(&self) -> bool {
        false
    }

    /// Corresponds to Lwjgl3Controller.startVibration(int, float)
    fn start_vibration(&mut self, _duration: i32, _strength: f32) {
        // empty in Java
    }

    /// Corresponds to Lwjgl3Controller.cancelVibration()
    fn cancel_vibration(&mut self) {
        // empty in Java
    }

    /// Corresponds to Lwjgl3Controller.supportsPlayerIndex()
    fn supports_player_index(&self) -> bool {
        false
    }

    /// Corresponds to Lwjgl3Controller.getPlayerIndex()
    fn get_player_index(&self) -> i32 {
        0
    }

    /// Corresponds to Lwjgl3Controller.setPlayerIndex(int)
    fn set_player_index(&mut self, _index: i32) {
        // empty in Java
    }

    // get_mapping() and get_power_level() use default trait implementations (return None)
}

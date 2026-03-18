use super::{Controller, ControllerListener};

/// Button mapping: sequential index → gilrs::Button.
/// Matches the GLFW button order used by beatoraja config files.
pub const BUTTON_MAP: &[gilrs::Button] = &[
    gilrs::Button::South,         // 0  - A / Cross
    gilrs::Button::East,          // 1  - B / Circle
    gilrs::Button::West,          // 2  - X / Square
    gilrs::Button::North,         // 3  - Y / Triangle
    gilrs::Button::LeftTrigger,   // 4  - LB
    gilrs::Button::RightTrigger,  // 5  - RB
    gilrs::Button::LeftTrigger2,  // 6  - LT
    gilrs::Button::RightTrigger2, // 7  - RT
    gilrs::Button::Select,        // 8  - Back / Share
    gilrs::Button::Start,         // 9  - Start / Options
    gilrs::Button::LeftThumb,     // 10 - L3
    gilrs::Button::RightThumb,    // 11 - R3
    gilrs::Button::DPadUp,        // 12
    gilrs::Button::DPadDown,      // 13
    gilrs::Button::DPadLeft,      // 14
    gilrs::Button::DPadRight,     // 15
    gilrs::Button::Mode,          // 16 - Guide
    gilrs::Button::C,             // 17
    gilrs::Button::Z,             // 18
];

/// Axis mapping: sequential index → gilrs::Axis.
pub const AXIS_MAP: &[gilrs::Axis] = &[
    gilrs::Axis::LeftStickX,  // 0
    gilrs::Axis::LeftStickY,  // 1
    gilrs::Axis::RightStickX, // 2
    gilrs::Axis::RightStickY, // 3
    gilrs::Axis::LeftZ,       // 4 (LT analog)
    gilrs::Axis::RightZ,      // 5 (RT analog)
    gilrs::Axis::DPadX,       // 6
    gilrs::Axis::DPadY,       // 7
];

/// Reads current button states from a gilrs gamepad as sequential booleans.
pub fn read_button_state(gamepad: &gilrs::Gamepad) -> Vec<bool> {
    BUTTON_MAP
        .iter()
        .map(|&btn| gamepad.is_pressed(btn))
        .collect()
}

/// Reads current axis values from a gilrs gamepad as sequential floats.
pub fn read_axis_state(gamepad: &gilrs::Gamepad) -> Vec<f32> {
    AXIS_MAP.iter().map(|&ax| gamepad.value(ax)).collect()
}

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
    /// gilrs gamepad identifier
    pub gamepad_id: Option<gilrs::GamepadId>,
    /// Whether the controller is currently connected
    pub connected: bool,
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
        // Fallback for non-gilrs construction.
        Self::new_with_state(index, 0, 0, format!("Controller {}", index))
    }

    /// Creates a controller backed by a gilrs gamepad.
    pub fn new_from_gilrs(index: i32, gamepad: &gilrs::Gamepad) -> Self {
        let name = gamepad.name().to_string();
        let num_buttons = BUTTON_MAP.len();
        let num_axes = AXIS_MAP.len();
        let gamepad_id = Some(gamepad.id());

        log::info!(
            "Controller connected: index={}, name={}, buttons={}, axes={}",
            index,
            name,
            num_buttons,
            num_axes,
        );

        Lwjgl3Controller {
            listeners: Vec::new(),
            index,
            axis_state: vec![0.0; num_axes],
            button_state: vec![false; num_buttons],
            name,
            gamepad_id,
            connected: true,
        }
    }

    /// Creates a controller with pre-initialized state (for testing or manual construction).
    pub fn new_with_state(index: i32, num_axes: usize, num_buttons: usize, name: String) -> Self {
        Lwjgl3Controller {
            listeners: Vec::new(),
            index,
            axis_state: vec![0.0; num_axes],
            button_state: vec![false; num_buttons],
            name,
            gamepad_id: None,
            connected: false,
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
        // State is now updated externally via update_from_gamepad().
        // This method is kept for backward compatibility.
        if self.connected {
            PollResult::Connected {
                axis_changes: Vec::new(),
                button_changes: Vec::new(),
            }
        } else {
            PollResult::Disconnected
        }
    }

    /// Updates this controller's state from a gilrs gamepad.
    /// Returns axis and button changes for the manager to dispatch.
    pub fn update_from_gamepad(&mut self, gamepad: &gilrs::Gamepad) -> PollResult {
        if !gamepad.is_connected() {
            self.connected = false;
            return PollResult::Disconnected;
        }

        let new_axes = read_axis_state(gamepad);
        let new_buttons = read_button_state(gamepad);

        let axis_changes = self.process_axis_changes(&new_axes);
        let button_changes = self.process_button_changes(&new_buttons);

        PollResult::Connected {
            axis_changes,
            button_changes,
        }
    }

    /// Processes axis state changes and fires local listener events.
    /// Called with new axis values read from the gamepad API.
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
        for (i, (state, &new_val)) in self.axis_state.iter_mut().zip(new_axes.iter()).enumerate() {
            if (*state - new_val).abs() > f32::EPSILON {
                // Fire local listeners
                for listener in &mut self.listeners {
                    if listener.axis_moved(self.index as usize, i as i32, new_val) {
                        break;
                    }
                }
                changes.push((i as i32, new_val));
            }
            *state = new_val;
        }

        changes
    }

    /// Processes button state changes and fires local listener events.
    /// Called with new button values read from the gamepad API.
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
        for (i, (state, &new_val)) in self
            .button_state
            .iter_mut()
            .zip(new_buttons.iter())
            .enumerate()
        {
            if *state != new_val {
                // Fire local listeners
                for listener in &mut self.listeners {
                    if new_val {
                        if listener.button_down(self.index as usize, i as i32) {
                            break;
                        }
                    } else if listener.button_up(self.index as usize, i as i32) {
                        break;
                    }
                }
                changes.push((i as i32, new_val));
            }
            *state = new_val;
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
    fn name(&self) -> &str {
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
        self.button_state.len() as i32
    }

    /// Corresponds to Lwjgl3Controller.getAxisCount()
    fn get_axis_count(&self) -> i32 {
        self.axis_state.len() as i32
    }

    /// Corresponds to Lwjgl3Controller.isConnected()
    fn is_connected(&self) -> bool {
        self.connected
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Test listener that records all events for verification.
    struct RecordingListener {
        events: Arc<Mutex<Vec<ListenerEvent>>>,
        /// If true, consume (return true) to stop propagation.
        consume: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum ListenerEvent {
        AxisMoved {
            controller: usize,
            axis: i32,
            value: f32,
        },
        ButtonDown {
            controller: usize,
            button: i32,
        },
        ButtonUp {
            controller: usize,
            button: i32,
        },
    }

    impl RecordingListener {
        fn new(events: Arc<Mutex<Vec<ListenerEvent>>>, consume: bool) -> Self {
            Self { events, consume }
        }
    }

    impl ControllerListener for RecordingListener {
        fn connected(&mut self, _controller_index: usize) {}
        fn disconnected(&mut self, _controller_index: usize) {}

        fn axis_moved(&mut self, controller_index: usize, axis_code: i32, value: f32) -> bool {
            self.events
                .lock()
                .expect("mutex poisoned")
                .push(ListenerEvent::AxisMoved {
                    controller: controller_index,
                    axis: axis_code,
                    value,
                });
            self.consume
        }

        fn button_down(&mut self, controller_index: usize, button_code: i32) -> bool {
            self.events
                .lock()
                .expect("mutex poisoned")
                .push(ListenerEvent::ButtonDown {
                    controller: controller_index,
                    button: button_code,
                });
            self.consume
        }

        fn button_up(&mut self, controller_index: usize, button_code: i32) -> bool {
            self.events
                .lock()
                .expect("mutex poisoned")
                .push(ListenerEvent::ButtonUp {
                    controller: controller_index,
                    button: button_code,
                });
            self.consume
        }
    }

    #[test]
    fn new_with_state_initializes_zeroed_arrays() {
        let ctrl = Lwjgl3Controller::new_with_state(0, 4, 8, "Test Pad".to_string());

        assert_eq!(ctrl.index, 0);
        assert_eq!(ctrl.name, "Test Pad");
        assert_eq!(ctrl.axis_state, vec![0.0; 4]);
        assert_eq!(ctrl.button_state, vec![false; 8]);
        assert!(!ctrl.connected);
        assert!(ctrl.gamepad_id.is_none());
        assert!(ctrl.listeners.is_empty());
    }

    #[test]
    fn controller_trait_button_bounds_check() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 2, 4, "Pad".to_string());
        ctrl.button_state[1] = true;
        ctrl.button_state[3] = true;

        // Valid indices
        assert!(!ctrl.get_button(0));
        assert!(ctrl.get_button(1));
        assert!(!ctrl.get_button(2));
        assert!(ctrl.get_button(3));

        // Out-of-bounds returns false (matches Java behavior)
        assert!(!ctrl.get_button(-1));
        assert!(!ctrl.get_button(4));
        assert!(!ctrl.get_button(100));
    }

    #[test]
    fn controller_trait_axis_bounds_check() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 3, 0, "Pad".to_string());
        ctrl.axis_state[0] = -0.75;
        ctrl.axis_state[1] = 0.0;
        ctrl.axis_state[2] = 1.0;

        // Valid indices
        assert!((ctrl.get_axis(0) - (-0.75)).abs() < f32::EPSILON);
        assert!((ctrl.get_axis(1) - 0.0).abs() < f32::EPSILON);
        assert!((ctrl.get_axis(2) - 1.0).abs() < f32::EPSILON);

        // Out-of-bounds returns 0.0 (matches Java behavior)
        assert!((ctrl.get_axis(-1) - 0.0).abs() < f32::EPSILON);
        assert!((ctrl.get_axis(3) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn controller_trait_metadata_properties() {
        let ctrl = Lwjgl3Controller::new_with_state(5, 8, 19, "Xbox Controller".to_string());

        assert_eq!(ctrl.name(), "Xbox Controller");
        assert_eq!(ctrl.get_unique_id(), None);
        assert_eq!(ctrl.get_min_button_index(), 0);
        assert_eq!(ctrl.get_max_button_index(), 19);
        assert_eq!(ctrl.get_axis_count(), 8);
        assert!(!ctrl.is_connected());
        assert!(!ctrl.can_vibrate());
        assert!(!ctrl.is_vibrating());
        assert!(!ctrl.supports_player_index());
        assert_eq!(ctrl.get_player_index(), 0);
        assert_eq!(ctrl.get_mapping(), None);
        assert_eq!(ctrl.get_power_level(), None);
    }

    #[test]
    fn process_axis_changes_detects_diff_and_updates_state() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 4, 0, "Pad".to_string());

        // First update: all zeros → some movement
        let new_axes = vec![0.5, 0.0, -0.3, 0.0];
        let changes = ctrl.process_axis_changes(&new_axes);

        // Only axes 0 and 2 changed from 0.0
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], (0, 0.5));
        assert_eq!(changes[1], (2, -0.3));
        // State is updated
        assert_eq!(ctrl.axis_state, vec![0.5, 0.0, -0.3, 0.0]);

        // Second update: same values → no changes
        let changes = ctrl.process_axis_changes(&new_axes);
        assert!(changes.is_empty());

        // Third update: partial change
        let new_axes = vec![0.5, 1.0, -0.3, 0.0];
        let changes = ctrl.process_axis_changes(&new_axes);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0], (1, 1.0));
    }

    #[test]
    fn process_button_changes_detects_press_and_release() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 0, 4, "Pad".to_string());

        // Press buttons 0 and 2
        let new_buttons = vec![true, false, true, false];
        let changes = ctrl.process_button_changes(&new_buttons);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], (0, true));
        assert_eq!(changes[1], (2, true));
        assert_eq!(ctrl.button_state, vec![true, false, true, false]);

        // Release button 0, press button 1
        let new_buttons = vec![false, true, true, false];
        let changes = ctrl.process_button_changes(&new_buttons);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], (0, false)); // released
        assert_eq!(changes[1], (1, true)); // pressed
    }

    #[test]
    fn process_axis_changes_handles_length_mismatch() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 4, 0, "Pad".to_string());

        // Fewer axes than controller state — only processes up to min length
        let new_axes = vec![0.5, 0.3];
        let changes = ctrl.process_axis_changes(&new_axes);
        assert_eq!(changes.len(), 2);
        // Indices 2 and 3 remain at 0.0
        assert!((ctrl.axis_state[2] - 0.0).abs() < f32::EPSILON);
        assert!((ctrl.axis_state[3] - 0.0).abs() < f32::EPSILON);

        // More axes than controller state — only processes up to controller's length
        let mut ctrl2 = Lwjgl3Controller::new_with_state(0, 2, 0, "Pad".to_string());
        let new_axes = vec![0.1, 0.2, 0.3, 0.4];
        let changes = ctrl2.process_axis_changes(&new_axes);
        assert_eq!(changes.len(), 2);
        assert_eq!(ctrl2.axis_state.len(), 2);
    }

    #[test]
    fn listener_fires_on_axis_change() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 2, 0, "Pad".to_string());
        let events = Arc::new(Mutex::new(Vec::new()));
        ctrl.add_listener(Box::new(RecordingListener::new(events.clone(), false)));

        let new_axes = vec![0.7, 0.0];
        ctrl.process_axis_changes(&new_axes);

        let recorded = events.lock().expect("mutex poisoned");
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            ListenerEvent::AxisMoved {
                controller: 0,
                axis: 0,
                value: 0.7,
            }
        );
    }

    #[test]
    fn listener_fires_on_button_press_and_release() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 0, 2, "Pad".to_string());
        let events = Arc::new(Mutex::new(Vec::new()));
        ctrl.add_listener(Box::new(RecordingListener::new(events.clone(), false)));

        // Press button 1
        ctrl.process_button_changes(&[false, true]);
        // Release button 1
        ctrl.process_button_changes(&[false, false]);

        let recorded = events.lock().expect("mutex poisoned");
        assert_eq!(recorded.len(), 2);
        assert_eq!(
            recorded[0],
            ListenerEvent::ButtonDown {
                controller: 0,
                button: 1,
            }
        );
        assert_eq!(
            recorded[1],
            ListenerEvent::ButtonUp {
                controller: 0,
                button: 1,
            }
        );
    }

    #[test]
    fn consuming_listener_stops_propagation() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 2, 2, "Pad".to_string());

        let events_first = Arc::new(Mutex::new(Vec::new()));
        let events_second = Arc::new(Mutex::new(Vec::new()));

        // First listener consumes, second should NOT receive events
        ctrl.add_listener(Box::new(RecordingListener::new(events_first.clone(), true)));
        ctrl.add_listener(Box::new(RecordingListener::new(
            events_second.clone(),
            false,
        )));

        // Axis change
        ctrl.process_axis_changes(&[1.0, 0.0]);
        assert_eq!(events_first.lock().expect("mutex poisoned").len(), 1);
        assert_eq!(events_second.lock().expect("mutex poisoned").len(), 0);

        // Button press
        ctrl.process_button_changes(&[true, false]);
        assert_eq!(events_first.lock().expect("mutex poisoned").len(), 2); // +1 button_down
        assert_eq!(events_second.lock().expect("mutex poisoned").len(), 0); // still 0

        // Button release
        ctrl.process_button_changes(&[false, false]);
        assert_eq!(events_first.lock().expect("mutex poisoned").len(), 3); // +1 button_up
        assert_eq!(events_second.lock().expect("mutex poisoned").len(), 0); // still 0
    }

    #[test]
    fn poll_state_reflects_connected_flag() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 2, 2, "Pad".to_string());

        // Not connected -> Disconnected
        ctrl.connected = false;
        assert!(matches!(ctrl.poll_state(), PollResult::Disconnected));

        // Connected -> Connected with empty changes
        ctrl.connected = true;
        let result = ctrl.poll_state();
        assert!(
            matches!(
                &result,
                PollResult::Connected {
                    axis_changes,
                    button_changes,
                } if axis_changes.is_empty() && button_changes.is_empty()
            ),
            "expected Connected with empty changes, got Disconnected"
        );
    }

    #[test]
    fn disconnect_during_poll_does_not_panic() {
        // A controller disconnect should be handled gracefully, not crash.
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 4, 4, "Pad".to_string());
        ctrl.connected = true;

        // Simulate disconnect by flipping the flag mid-session
        ctrl.connected = false;
        let result = ctrl.poll_state();
        assert!(matches!(result, PollResult::Disconnected));

        // Verify the controller reports disconnected state through the trait
        assert!(!ctrl.is_connected());

        // Operations on a disconnected controller should still work safely
        assert!(!ctrl.get_button(0));
        assert!((ctrl.get_axis(0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn remove_listener_by_index() {
        let mut ctrl = Lwjgl3Controller::new_with_state(0, 0, 2, "Pad".to_string());

        let events_a = Arc::new(Mutex::new(Vec::new()));
        let events_b = Arc::new(Mutex::new(Vec::new()));
        ctrl.add_listener(Box::new(RecordingListener::new(events_a.clone(), false)));
        ctrl.add_listener(Box::new(RecordingListener::new(events_b.clone(), false)));
        assert_eq!(ctrl.listeners.len(), 2);

        // Remove first listener
        ctrl.remove_listener(0);
        assert_eq!(ctrl.listeners.len(), 1);

        // Fire event — only second listener (now at index 0) receives it
        ctrl.process_button_changes(&[true, false]);
        assert!(events_a.lock().expect("mutex poisoned").is_empty());
        assert_eq!(events_b.lock().expect("mutex poisoned").len(), 1);

        // Remove out-of-bounds index — no panic
        ctrl.remove_listener(99);
        assert_eq!(ctrl.listeners.len(), 1);
    }

    #[test]
    fn local_listener_receives_controller_own_index_for_axis() {
        // Controller with index=3 should pass 3 to local listeners, not 0
        let mut ctrl = Lwjgl3Controller::new_with_state(3, 2, 0, "Pad".to_string());
        let events = Arc::new(Mutex::new(Vec::new()));
        ctrl.add_listener(Box::new(RecordingListener::new(events.clone(), false)));

        ctrl.process_axis_changes(&[0.5, 0.0]);

        let recorded = events.lock().expect("mutex poisoned");
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            ListenerEvent::AxisMoved {
                controller: 3,
                axis: 0,
                value: 0.5,
            }
        );
    }

    #[test]
    fn local_listener_receives_controller_own_index_for_buttons() {
        // Controller with index=5 should pass 5 to local listeners, not 0
        let mut ctrl = Lwjgl3Controller::new_with_state(5, 0, 2, "Pad".to_string());
        let events = Arc::new(Mutex::new(Vec::new()));
        ctrl.add_listener(Box::new(RecordingListener::new(events.clone(), false)));

        ctrl.process_button_changes(&[true, false]);

        let recorded = events.lock().expect("mutex poisoned");
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            ListenerEvent::ButtonDown {
                controller: 5,
                button: 0,
            }
        );
    }

    #[test]
    fn button_map_and_axis_map_sizes() {
        // BUTTON_MAP must cover the standard 19 gamepad buttons
        assert_eq!(BUTTON_MAP.len(), 19);
        // AXIS_MAP must cover the standard 8 axes
        assert_eq!(AXIS_MAP.len(), 8);

        // Verify first and last entries are correct
        assert_eq!(BUTTON_MAP[0], gilrs::Button::South);
        assert_eq!(BUTTON_MAP[18], gilrs::Button::Z);
        assert_eq!(AXIS_MAP[0], gilrs::Axis::LeftStickX);
        assert_eq!(AXIS_MAP[7], gilrs::Axis::DPadY);
    }
}

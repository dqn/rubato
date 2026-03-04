#![allow(dead_code)]

use super::lwjgl3_controller::{Lwjgl3Controller, PollResult};
use super::{ControllerListener, ControllerManager};

/// Corresponds to bms.player.beatoraja.controller.Lwjgl3ControllerManager
///
/// Controller manager that polls gamepads via gilrs.
/// Maintains a list of controllers, polls joystick state each frame,
/// and manages controller connect/disconnect events.
pub struct Lwjgl3ControllerManager {
    /// All currently connected controllers.
    pub controllers: Vec<Lwjgl3Controller>,
    /// Global listeners for controller events.
    pub listeners: Vec<Box<dyn ControllerListener>>,
    /// Whether the window is focused.
    pub focused: bool,
    /// gilrs gamepad context.
    gilrs: Option<gilrs::Gilrs>,
    /// Next index to assign to a new controller.
    next_index: i32,
}

impl Lwjgl3ControllerManager {
    /// Corresponds to Lwjgl3ControllerManager()
    ///
    /// Creates a new controller manager backed by gilrs.
    pub fn new() -> Self {
        let gilrs = match gilrs::Gilrs::new() {
            Ok(g) => {
                log::info!("gilrs initialized successfully");
                Some(g)
            }
            Err(e) => {
                log::error!("Failed to initialize gilrs: {}", e);
                None
            }
        };

        let mut manager = Lwjgl3ControllerManager {
            controllers: Vec::new(),
            listeners: Vec::new(),
            focused: true,
            gilrs,
            next_index: 0,
        };

        // Initial poll to discover already-connected gamepads
        manager.poll_state();
        manager
    }

    /// Corresponds to Lwjgl3ControllerManager.pollState()
    ///
    /// Processes gilrs events and polls all connected controllers.
    pub fn poll_state(&mut self) {
        // Take gilrs out of self to avoid borrow conflicts (Option::take + put-back pattern).
        let Some(mut gilrs) = self.gilrs.take() else {
            return;
        };

        // Phase 1: Process pending gilrs events (updates internal gamepad state)
        while let Some(gilrs::Event { id, event, .. }) = gilrs.next_event() {
            match event {
                gilrs::EventType::Connected => {
                    log::info!("Gamepad connected event: {:?}", id);
                }
                gilrs::EventType::Disconnected => {
                    log::info!("Gamepad disconnected event: {:?}", id);
                }
                _ => {}
            }
        }

        // Phase 2: Discover newly connected gamepads
        let new_gamepads: Vec<_> = gilrs
            .gamepads()
            .filter(|(_, gp)| gp.is_connected())
            .filter(|(id, _)| !self.controllers.iter().any(|c| c.gamepad_id == Some(*id)))
            .map(|(id, _)| id)
            .collect();

        let mut new_controllers = Vec::new();
        for gid in new_gamepads {
            let gamepad = gilrs.gamepad(gid);
            let index = self.next_index;
            self.next_index += 1;
            new_controllers.push(Lwjgl3Controller::new_from_gilrs(index, &gamepad));
        }

        // Phase 3: Connect new controllers (self is freely available)
        for controller in new_controllers {
            self.connected(controller);
        }

        // Phase 4: Poll each connected controller, collecting changes
        struct ControllerChanges {
            idx: usize,
            axis_changes: Vec<(i32, f32)>,
            button_changes: Vec<(i32, bool)>,
        }

        let mut all_changes: Vec<ControllerChanges> = Vec::new();
        let mut disconnected_indices: Vec<usize> = Vec::new();

        for idx in 0..self.controllers.len() {
            if let Some(gid) = self.controllers[idx].gamepad_id {
                let gamepad = gilrs.gamepad(gid);
                match self.controllers[idx].update_from_gamepad(&gamepad) {
                    PollResult::Disconnected => {
                        disconnected_indices.push(idx);
                    }
                    PollResult::Connected {
                        axis_changes,
                        button_changes,
                    } => {
                        if !axis_changes.is_empty() || !button_changes.is_empty() {
                            all_changes.push(ControllerChanges {
                                idx,
                                axis_changes,
                                button_changes,
                            });
                        }
                    }
                }
            }
        }

        // Put gilrs back before firing events (which borrow self mutably)
        self.gilrs = Some(gilrs);

        // Phase 5: Fire manager-level events for changes
        for changes in all_changes {
            for (axis_code, value) in changes.axis_changes {
                self.axis_changed(changes.idx, axis_code, value);
            }
            for (button_code, pressed) in changes.button_changes {
                self.button_changed(changes.idx, button_code, pressed);
            }
        }

        // Handle disconnections (reverse order to preserve indices)
        for idx in disconnected_indices.into_iter().rev() {
            self.disconnected(idx);
        }
    }

    /// Corresponds to Lwjgl3ControllerManager.connected(Lwjgl3Controller)
    ///
    /// Called when a new controller is connected.
    pub fn connected(&mut self, controller: Lwjgl3Controller) {
        // controllers.add(controller);
        // for(ControllerListener listener: listeners) {
        //     listener.connected(controller);
        // }
        self.controllers.push(controller);
        let controller_index = self.controllers.len() - 1;
        for listener in &mut self.listeners {
            listener.connected(controller_index);
        }
    }

    /// Corresponds to Lwjgl3ControllerManager.disconnected(Lwjgl3Controller)
    ///
    /// Called when a controller is disconnected.
    pub fn disconnected(&mut self, controller_index: usize) {
        // controllers.removeValue(controller, true);
        // for(ControllerListener listener: listeners) {
        //     listener.disconnected(controller);
        // }
        if controller_index < self.controllers.len() {
            self.controllers.remove(controller_index);
            for listener in &mut self.listeners {
                listener.disconnected(controller_index);
            }
        }
    }

    /// Corresponds to Lwjgl3ControllerManager.axisChanged(Lwjgl3Controller, int, float)
    ///
    /// Called when a controller's axis value changes.
    pub fn axis_changed(&mut self, controller_index: usize, axis_code: i32, value: f32) {
        // for(ControllerListener listener: listeners) {
        //     if (listener.axisMoved(controller, axisCode, value)) break;
        // }
        for listener in &mut self.listeners {
            if listener.axis_moved(controller_index, axis_code, value) {
                break;
            }
        }
    }

    /// Corresponds to Lwjgl3ControllerManager.buttonChanged(Lwjgl3Controller, int, boolean)
    ///
    /// Called when a controller's button state changes.
    pub fn button_changed(&mut self, controller_index: usize, button_code: i32, value: bool) {
        // for(ControllerListener listener: listeners) {
        //     if(value) {
        //         if (listener.buttonDown(controller, buttonCode)) break;
        //     } else {
        //         if (listener.buttonUp(controller, buttonCode)) break;
        //     }
        // }
        for listener in &mut self.listeners {
            if value {
                if listener.button_down(controller_index, button_code) {
                    break;
                }
            } else if listener.button_up(controller_index, button_code) {
                break;
            }
        }
    }

    /// Corresponds to Lwjgl3ControllerManager.setUnfocused(long, boolean)
    ///
    /// GLFW window focus callback.
    pub fn set_unfocused(&mut self, _win: i64, is_focused: bool) {
        self.focused = is_focused;
    }
}

impl Default for Lwjgl3ControllerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ControllerManager for Lwjgl3ControllerManager {
    /// Corresponds to Lwjgl3ControllerManager.getControllers()
    fn get_controllers(&mut self) -> &[Lwjgl3Controller] {
        self.poll_state();
        &self.controllers
    }

    /// Corresponds to Lwjgl3ControllerManager.getCurrentController()
    fn get_current_controller(&self) -> Option<usize> {
        // return null;
        None
    }

    /// Corresponds to Lwjgl3ControllerManager.addListener(ControllerListener)
    fn add_listener(&mut self, listener: Box<dyn ControllerListener>) {
        self.listeners.push(listener);
    }

    /// Corresponds to Lwjgl3ControllerManager.removeListener(ControllerListener)
    fn remove_listener(&mut self, index: usize) {
        if index < self.listeners.len() {
            self.listeners.remove(index);
        }
    }

    /// Corresponds to Lwjgl3ControllerManager.clearListeners()
    fn clear_listeners(&mut self) {
        self.listeners.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Events recorded by the test listener.
    #[derive(Debug, Clone, PartialEq)]
    enum ManagerEvent {
        Connected(usize),
        Disconnected(usize),
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

    struct RecordingListener {
        events: Arc<Mutex<Vec<ManagerEvent>>>,
        consume: bool,
    }

    impl RecordingListener {
        fn new(events: Arc<Mutex<Vec<ManagerEvent>>>, consume: bool) -> Self {
            Self { events, consume }
        }
    }

    impl ControllerListener for RecordingListener {
        fn connected(&mut self, controller_index: usize) {
            self.events
                .lock()
                .unwrap()
                .push(ManagerEvent::Connected(controller_index));
        }

        fn disconnected(&mut self, controller_index: usize) {
            self.events
                .lock()
                .unwrap()
                .push(ManagerEvent::Disconnected(controller_index));
        }

        fn axis_moved(&mut self, controller_index: usize, axis_code: i32, value: f32) -> bool {
            self.events.lock().unwrap().push(ManagerEvent::AxisMoved {
                controller: controller_index,
                axis: axis_code,
                value,
            });
            self.consume
        }

        fn button_down(&mut self, controller_index: usize, button_code: i32) -> bool {
            self.events.lock().unwrap().push(ManagerEvent::ButtonDown {
                controller: controller_index,
                button: button_code,
            });
            self.consume
        }

        fn button_up(&mut self, controller_index: usize, button_code: i32) -> bool {
            self.events.lock().unwrap().push(ManagerEvent::ButtonUp {
                controller: controller_index,
                button: button_code,
            });
            self.consume
        }
    }

    /// Constructs a manager without gilrs (no hardware needed).
    fn manager_without_gilrs() -> Lwjgl3ControllerManager {
        Lwjgl3ControllerManager {
            controllers: Vec::new(),
            listeners: Vec::new(),
            focused: true,
            gilrs: None,
            next_index: 0,
        }
    }

    #[test]
    fn connected_adds_controller_and_fires_listener() {
        let mut mgr = manager_without_gilrs();
        let events = Arc::new(Mutex::new(Vec::new()));
        mgr.listeners
            .push(Box::new(RecordingListener::new(events.clone(), false)));

        let ctrl = Lwjgl3Controller::new_with_state(0, 2, 4, "Pad A".to_string());
        mgr.connected(ctrl);

        assert_eq!(mgr.controllers.len(), 1);
        assert_eq!(mgr.controllers[0].name, "Pad A");
        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], ManagerEvent::Connected(0));
    }

    #[test]
    fn disconnected_removes_controller_and_fires_listener() {
        let mut mgr = manager_without_gilrs();
        let events = Arc::new(Mutex::new(Vec::new()));
        mgr.listeners
            .push(Box::new(RecordingListener::new(events.clone(), false)));

        mgr.connected(Lwjgl3Controller::new_with_state(
            0,
            2,
            4,
            "Pad A".to_string(),
        ));
        mgr.connected(Lwjgl3Controller::new_with_state(
            1,
            2,
            4,
            "Pad B".to_string(),
        ));
        assert_eq!(mgr.controllers.len(), 2);

        // Disconnect first controller
        mgr.disconnected(0);
        assert_eq!(mgr.controllers.len(), 1);
        assert_eq!(mgr.controllers[0].name, "Pad B");

        let recorded = events.lock().unwrap();
        // 2 Connected + 1 Disconnected
        assert_eq!(recorded.len(), 3);
        assert_eq!(recorded[2], ManagerEvent::Disconnected(0));
    }

    #[test]
    fn disconnected_out_of_bounds_is_noop() {
        let mut mgr = manager_without_gilrs();
        // Should not panic
        mgr.disconnected(0);
        mgr.disconnected(99);
        assert!(mgr.controllers.is_empty());
    }

    #[test]
    fn button_changed_dispatches_down_and_up_events() {
        let mut mgr = manager_without_gilrs();
        let events = Arc::new(Mutex::new(Vec::new()));
        mgr.listeners
            .push(Box::new(RecordingListener::new(events.clone(), false)));

        // button press
        mgr.button_changed(0, 5, true);
        // button release
        mgr.button_changed(0, 5, false);

        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert_eq!(
            recorded[0],
            ManagerEvent::ButtonDown {
                controller: 0,
                button: 5,
            }
        );
        assert_eq!(
            recorded[1],
            ManagerEvent::ButtonUp {
                controller: 0,
                button: 5,
            }
        );
    }

    #[test]
    fn axis_changed_dispatches_event() {
        let mut mgr = manager_without_gilrs();
        let events = Arc::new(Mutex::new(Vec::new()));
        mgr.listeners
            .push(Box::new(RecordingListener::new(events.clone(), false)));

        mgr.axis_changed(1, 3, 0.85);

        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            ManagerEvent::AxisMoved {
                controller: 1,
                axis: 3,
                value: 0.85,
            }
        );
    }

    #[test]
    fn consuming_listener_stops_manager_propagation() {
        let mut mgr = manager_without_gilrs();
        let events_first = Arc::new(Mutex::new(Vec::new()));
        let events_second = Arc::new(Mutex::new(Vec::new()));

        mgr.listeners
            .push(Box::new(RecordingListener::new(events_first.clone(), true)));
        mgr.listeners.push(Box::new(RecordingListener::new(
            events_second.clone(),
            false,
        )));

        mgr.button_changed(0, 0, true);
        mgr.axis_changed(0, 0, 0.5);

        assert_eq!(events_first.lock().unwrap().len(), 2);
        assert_eq!(events_second.lock().unwrap().len(), 0);
    }

    #[test]
    fn listener_management_add_remove_clear() {
        let mut mgr = manager_without_gilrs();
        let events = Arc::new(Mutex::new(Vec::new()));

        mgr.add_listener(Box::new(RecordingListener::new(events.clone(), false)));
        mgr.add_listener(Box::new(RecordingListener::new(events.clone(), false)));
        assert_eq!(mgr.listeners.len(), 2);

        mgr.remove_listener(0);
        assert_eq!(mgr.listeners.len(), 1);

        // Remove out-of-bounds — no panic
        mgr.remove_listener(99);
        assert_eq!(mgr.listeners.len(), 1);

        mgr.clear_listeners();
        assert!(mgr.listeners.is_empty());
    }

    #[test]
    fn set_unfocused_toggles_focused_flag() {
        let mut mgr = manager_without_gilrs();
        assert!(mgr.focused);

        mgr.set_unfocused(0, false);
        assert!(!mgr.focused);

        mgr.set_unfocused(0, true);
        assert!(mgr.focused);
    }

    #[test]
    fn get_current_controller_returns_none() {
        let mgr = manager_without_gilrs();
        assert_eq!(mgr.get_current_controller(), None);
    }
}

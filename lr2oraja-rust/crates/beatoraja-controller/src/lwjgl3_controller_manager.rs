#![allow(dead_code)]

use crate::lwjgl3_controller::Lwjgl3Controller;
use crate::{ControllerListener, ControllerManager};

/// GLFW joystick constants
/// Corresponds to GLFW.GLFW_JOYSTICK_1
const GLFW_JOYSTICK_1: i32 = 0;
/// Corresponds to GLFW.GLFW_JOYSTICK_LAST
const GLFW_JOYSTICK_LAST: i32 = 15;

/// Corresponds to bms.player.beatoraja.controller.Lwjgl3ControllerManager
///
/// Controller manager that polls GLFW joysticks.
/// Maintains a list of controllers, polls joystick state each frame,
/// and manages controller connect/disconnect events.
pub struct Lwjgl3ControllerManager {
    /// All currently connected controllers.
    pub controllers: Vec<Lwjgl3Controller>,
    /// Joystick indices that failed to initialize and are blacklisted.
    pub blacklisted_controllers: Vec<i32>,
    /// Global listeners for controller events.
    pub listeners: Vec<Box<dyn ControllerListener>>,
    /// Whether the window is focused.
    pub focused: bool,
}

impl Lwjgl3ControllerManager {
    /// Corresponds to Lwjgl3ControllerManager()
    ///
    /// Creates a new controller manager.
    /// In Java, the constructor sets up a GLFW window focus callback and starts polling.
    pub fn new() -> Self {
        // In Java:
        //   GLFW.glfwSetWindowFocusCallback(windowHandle, this::setUnfocused);
        //   pollState();
        //   Gdx.app.postRunnable(...); // recurring poll
        // GLFW window handle and callback setup is deferred to gilrs integration.

        let mut manager = Lwjgl3ControllerManager {
            controllers: Vec::new(),
            blacklisted_controllers: Vec::new(),
            listeners: Vec::new(),
            focused: true,
        };

        // Initial poll
        manager.poll_state();
        manager
    }

    /// Corresponds to Lwjgl3ControllerManager.pollState()
    ///
    /// Scans for new joysticks and polls all connected controllers.
    pub fn poll_state(&mut self) {
        // for(int i = GLFW.GLFW_JOYSTICK_1; i < GLFW.GLFW_JOYSTICK_LAST; i++) {
        //     if (blacklistedControllers.contains(i, true)) { continue; }
        //     if(GLFW.glfwJoystickPresent(i)) {
        //         boolean alreadyUsed = false;
        //         for(int j = 0; j < controllers.size; j++) {
        //             if(((Lwjgl3Controller)controllers.get(j)).index == i) {
        //                 alreadyUsed = true;
        //                 break;
        //             }
        //         }
        //         if(!alreadyUsed) {
        //             try {
        //                 Lwjgl3Controller controller = new Lwjgl3Controller(this, i);
        //                 connected(controller);
        //             } catch (Exception e) {
        //                 blacklistedControllers.add(i);
        //             }
        //         }
        //     }
        // }
        for i in GLFW_JOYSTICK_1..GLFW_JOYSTICK_LAST {
            if self.blacklisted_controllers.contains(&i) {
                continue;
            }

            // GLFW.glfwJoystickPresent(i) — stubbed
            let joystick_present = false; // todo!("GLFW/gilrs integration: check joystick present")

            if joystick_present {
                let already_used = self.controllers.iter().any(|c| c.index == i);

                if !already_used {
                    // In Java: new Lwjgl3Controller(this, i) can throw,
                    // which causes the index to be blacklisted.
                    // Lwjgl3Controller::new() currently uses todo!(),
                    // so we mark this as pending integration.
                    // todo!("GLFW/gilrs integration: create controller for index {}", i);
                    log::debug!(
                        "GLFW/gilrs integration pending: would create controller for index {}",
                        i
                    );
                }
            }
        }

        // polledControllers.addAll(controllers);
        // for(Controller controller: polledControllers) {
        //     ((Lwjgl3Controller)controller).pollState();
        // }
        // polledControllers.clear();
        //
        // We need to collect indices first to avoid borrow issues,
        // then poll each controller and process manager-level events.
        let controller_indices: Vec<usize> = (0..self.controllers.len()).collect();
        for idx in controller_indices {
            // In the real implementation, poll_state() would be called on each controller.
            // The controller's poll_state returns axis/button changes and disconnect status.
            // For now, this is a no-op since poll_state() uses todo!().
            // When integrated:
            //   match self.controllers[idx].poll_state() {
            //       PollResult::Disconnected => { self.disconnected(idx); }
            //       PollResult::Connected { axis_changes, button_changes } => {
            //           for (axis_code, value) in axis_changes {
            //               self.axis_changed(idx, axis_code, value);
            //           }
            //           for (button_code, pressed) in button_changes {
            //               self.button_changed(idx, button_code, pressed);
            //           }
            //       }
            //   }
            let _ = idx;
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

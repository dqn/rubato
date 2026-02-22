#![allow(dead_code)]

pub mod lwjgl3_controller;
pub mod lwjgl3_controller_manager;
pub mod lwjgl3_controllers;

/// Corresponds to com.badlogic.gdx.controllers.ControllerListener
///
/// Listener interface for controller events. Methods return `true` to consume the event
/// (preventing further listeners from receiving it), or `false` to pass it along.
pub trait ControllerListener {
    /// Called when a controller is connected.
    fn connected(&mut self, controller_index: usize);

    /// Called when a controller is disconnected.
    fn disconnected(&mut self, controller_index: usize);

    /// Called when an axis is moved. Returns `true` if the event was consumed.
    fn axis_moved(&mut self, controller_index: usize, axis_code: i32, value: f32) -> bool;

    /// Called when a button is pressed. Returns `true` if the event was consumed.
    fn button_down(&mut self, controller_index: usize, button_code: i32) -> bool;

    /// Called when a button is released. Returns `true` if the event was consumed.
    fn button_up(&mut self, controller_index: usize, button_code: i32) -> bool;
}

/// Corresponds to com.badlogic.gdx.controllers.Controller
pub trait Controller {
    /// Returns the button state for the given button code.
    fn get_button(&self, button_code: i32) -> bool;

    /// Returns the axis value for the given axis code.
    fn get_axis(&self, axis_code: i32) -> f32;

    /// Returns the name of the controller.
    fn get_name(&self) -> &str;

    /// Returns the unique ID of the controller, if available.
    fn get_unique_id(&self) -> Option<String>;

    /// Returns the minimum button index.
    fn get_min_button_index(&self) -> i32;

    /// Returns the maximum button index.
    fn get_max_button_index(&self) -> i32;

    /// Returns the axis count.
    fn get_axis_count(&self) -> i32;

    /// Returns whether the controller is connected.
    fn is_connected(&self) -> bool;

    /// Returns whether the controller supports vibration.
    fn can_vibrate(&self) -> bool;

    /// Returns whether the controller is currently vibrating.
    fn is_vibrating(&self) -> bool;

    /// Starts vibration with the given duration (ms) and strength.
    fn start_vibration(&mut self, duration: i32, strength: f32);

    /// Cancels any ongoing vibration.
    fn cancel_vibration(&mut self);

    /// Returns whether the controller supports player index.
    fn supports_player_index(&self) -> bool;

    /// Returns the player index.
    fn get_player_index(&self) -> i32;

    /// Sets the player index.
    fn set_player_index(&mut self, index: i32);

    /// Returns the controller mapping, if available.
    ///
    /// Translated from: Controller.getMapping()
    fn get_mapping(&self) -> Option<()> {
        None
    }

    /// Returns the controller power level, if available.
    ///
    /// Translated from: Controller.getPowerLevel()
    fn get_power_level(&self) -> Option<()> {
        None
    }
}

/// Corresponds to com.badlogic.gdx.controllers.ControllerManager
pub trait ControllerManager {
    /// Returns all connected controllers.
    fn get_controllers(&mut self) -> &[lwjgl3_controller::Lwjgl3Controller];

    /// Returns the current controller, if any.
    fn get_current_controller(&self) -> Option<usize>;

    /// Adds a listener for controller events.
    fn add_listener(&mut self, listener: Box<dyn ControllerListener>);

    /// Removes a listener by index.
    fn remove_listener(&mut self, index: usize);

    /// Removes all listeners.
    fn clear_listeners(&mut self);
}

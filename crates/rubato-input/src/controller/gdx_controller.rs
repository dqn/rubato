/// Simple controller data struct (com.badlogic.gdx.controllers.Controller)
///
/// A lightweight data holder for controller state, used by BMControllerInputProcessor.
/// This is distinct from the `Controller` trait which defines the full controller interface.
pub struct GdxController {
    name: String,
    pub axis_state: Vec<f32>,
    pub button_state: Vec<bool>,
}

impl GdxController {
    pub fn new(name: String) -> Self {
        Self {
            name,
            axis_state: Vec::new(),
            button_state: Vec::new(),
        }
    }

    /// Creates a controller with pre-initialized state arrays.
    pub fn with_state(name: String, num_buttons: usize, num_axes: usize) -> Self {
        Self {
            name,
            axis_state: vec![0.0; num_axes],
            button_state: vec![false; num_buttons],
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn button(&self, button: i32) -> bool {
        if button < 0 || button as usize >= self.button_state.len() {
            return false;
        }
        self.button_state[button as usize]
    }

    pub fn axis(&self, axis: i32) -> f32 {
        if axis < 0 || axis as usize >= self.axis_state.len() {
            return 0.0;
        }
        self.axis_state[axis as usize]
    }
}

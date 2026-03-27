mod constants;
mod gdx_key_name;
mod mutators;
#[cfg(test)]
mod tests;

use rubato_types::play_mode_config::{
    ControllerConfig, KeyboardConfig, MidiConfig, MidiInput, MouseScratchConfig,
};

use crate::core::main_controller::MainController;
use crate::core::app_context::GameContext;
use crate::core::main_state::{MainState, MainStateData, MainStateType, StateTransition};
use crate::core::timer_manager::TimerManager;

use constants::{KEYS, KEYSA, MODE};
use gdx_key_name::gdx_key_name;

// Key configuration screen.
// Translated from Java: KeyConfiguration extends MainState
//
// This is heavily dependent on libGDX UI (SpriteBatch, BitmapFont, ShapeDrawer, etc.)
// and input processing (BMSPlayerInputProcessor, BMControllerInputProcessor, MidiInputProcessor).
// Most rendering and input methods are stubbed pending Phase 5+ graphics integration.

pub struct KeyConfiguration {
    state_data: MainStateData,
    cursorpos: usize,
    _scrollpos: usize,
    keyinput: bool,
    mode: usize,
    _deletepressed: bool,
    // References to input processors and config are Phase 5+ types
    // egui rendering deferred to Phase 9 launcher
}

impl KeyConfiguration {
    pub fn new(_main: &MainController) -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            cursorpos: 0,
            _scrollpos: 0,
            keyinput: false,
            mode: 0,
            _deletepressed: false,
        }
    }

    pub fn create(&mut self) {
        // egui: requires wgpu skin loading + input capture integration
    }

    pub fn render(&mut self) {
        // egui: requires wgpu SpriteBatch + ShapeDrawer rendering integration
    }

    pub fn set_key_assign_mode(&mut self, index: usize) {
        self.cursorpos = index;
        self.keyinput = true;
    }

    /// Get the display name of the key bound to the given game key index.
    ///
    /// `keyboard_keys`: key code array from KeyboardConfig.keys
    ///
    /// Java: KeyConfiguration.getKeyAssign(int index, BMSPlayerInputProcessor)
    pub fn key_assign(&self, index: usize, keyboard_keys: &[i32]) -> String {
        if index >= KEYSA[self.mode].len() {
            return "!!!".to_string();
        }
        let key_index = KEYSA[self.mode][index];
        if key_index < 0 {
            // START (-1) / SELECT (-2) — shown by KEYS label
            return "---".to_string();
        }
        let keycode = keyboard_keys.get(key_index as usize).copied().unwrap_or(-1);
        if keycode < 0 {
            return "---".to_string();
        }
        gdx_key_name(keycode).to_string()
    }

    pub fn mode(&self) -> usize {
        self.mode
    }

    pub fn mode_name(&self) -> &str {
        MODE[self.mode]
    }

    pub fn keys(&self) -> &[&str] {
        KEYS[self.mode]
    }

    pub fn keysa(&self) -> &[i32] {
        KEYSA[self.mode]
    }

    pub fn dispose_resources(&mut self) {
        // Java disposes BitmapFont (LibGDX GPU texture). In Rust, font resources
        // (GlyphAtlas/SpriteBatch) are owned by the render pipeline and dropped automatically.
    }

    // -- Getters --

    /// Returns the keyboard key assigned at the given index.
    /// Positive index: keys[index]. -1: start. -2: select. Other: 0.
    ///
    /// Java: KeyConfiguration.getKeyboardKeyAssign(int index)
    pub fn keyboard_key_assign(kb: &KeyboardConfig, index: i32) -> i32 {
        if index >= 0 {
            kb.keys.get(index as usize).copied().unwrap_or(0)
        } else if index == -1 {
            kb.start
        } else if index == -2 {
            kb.select
        } else {
            0
        }
    }

    /// Returns the controller key assigned at the given device and index.
    /// Positive index: keys[index]. -1: start. -2: select. Other: 0.
    ///
    /// Java: KeyConfiguration.getControllerKeyAssign(int device, int index)
    pub fn controller_key_assign(
        controllers: &[ControllerConfig],
        device: usize,
        index: i32,
    ) -> i32 {
        let cc = match controllers.get(device) {
            Some(c) => c,
            None => return 0,
        };
        if index >= 0 {
            cc.keys.get(index as usize).copied().unwrap_or(0)
        } else if index == -1 {
            cc.start
        } else if index == -2 {
            cc.select
        } else {
            0
        }
    }

    /// Returns the MIDI input assigned at the given index.
    /// Positive index: keys[index]. -1: start. -2: select. Other: default.
    ///
    /// Java: KeyConfiguration.getMidiKeyAssign(int index)
    pub fn midi_key_assign(midi: &MidiConfig, index: i32) -> MidiInput {
        if index >= 0 {
            midi.keys
                .get(index as usize)
                .copied()
                .flatten()
                .unwrap_or_default()
        } else if index == -1 {
            midi.start.unwrap_or_default()
        } else if index == -2 {
            midi.select.unwrap_or_default()
        } else {
            MidiInput::default()
        }
    }

    /// Returns the mouse scratch key string at the given index, or `default` if none.
    ///
    /// Java: KeyConfiguration.getMouseScratchKeyString(int index, String defaultKeyString)
    pub fn mouse_scratch_key_string(
        msc: &MouseScratchConfig,
        index: i32,
        default: Option<&str>,
    ) -> Option<String> {
        let result = if index >= 0 {
            msc.key_string(index as usize)
        } else if index == -1 {
            msc.start_string()
        } else if index == -2 {
            msc.select_string()
        } else {
            None
        };
        result
            .map(|s| s.to_string())
            .or_else(|| default.map(|s| s.to_string()))
    }
}

impl MainState for KeyConfiguration {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Config)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {
        // Initialize key configuration state.
        // In Java, this loads a skin, creates BitmapFont, ShapeDrawer, and sets up
        // BMSPlayerInputProcessor / BMControllerInputProcessor / MidiInputProcessor.
        // In Rust, the key configuration UI is rendered via wgpu SpriteBatch
        // (requires RenderPipeline + GpuTextureManager from beatoraja-render).
        log::info!(
            "KeyConfiguration::create — initialized for mode {:?}",
            self.mode
        );
    }

    fn render(&mut self) {
        // Render key configuration screen.
        // In Java, renders via SpriteBatch + ShapeDrawer: mode/key labels,
        // current key assignments, controller axis values, MIDI input display.
        // In Rust, requires wgpu render pass with SpriteBatch::flush_to_gpu().
        // The data model (modes, key assignments) is fully available via
        // self.player_config and self.key_config.
    }

    fn input(&mut self) {
        // Process key assignment input.
        // In Java, listens for keyboard/controller/MIDI input and assigns
        // the pressed key to the currently selected slot.
        // In Rust, input is available via BMSPlayerInputProcessor from beatoraja-input.
        // Key assignment: when a key is pressed, store its keycode in the current
        // PlayModeConfig's keyboard/controller/midi config for the active lane.
    }

    fn dispose(&mut self) {
        self.dispose_resources();
        // Call default trait dispose for skin cleanup
        let data = self.main_state_data_mut();
        if let Some(ref mut skin) = data.skin {
            skin.dispose_skin();
        }
        data.skin = None;
    }

    fn render_with_game_context(&mut self, _ctx: &mut GameContext) -> Option<StateTransition> {
        self.render();
        Some(StateTransition::Continue)
    }

    fn input_with_game_context(&mut self, _ctx: &mut GameContext) -> Option<()> {
        self.input();
        Some(())
    }
}

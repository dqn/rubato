//! BMSPlayerInputProcessor - main input manager
//!
//! Translated from: bms.player.beatoraja.input.BMSPlayerInputProcessor

use rubato_types::monotonic_clock::monotonic_micros;

use crate::controller::lwjgl3_controller_manager::Lwjgl3ControllerManager;

use crate::bm_controller_input_processor::{
    BMControllerCallback, BMControllerInputProcessor, compute_analog_diff,
};
use crate::bms_player_input_device::{BMSPlayerInputDevice, DeviceType};
use crate::controller::gdx_controller::GdxController;
use crate::key_command::KeyCommand;
use crate::key_input_log::KeyInputLog;
use crate::keyboard_input_processor::{
    ControlKeys, KeyBoardInputProcesseor, KeyboardCallback, MASK_CTRL, MASK_SHIFT,
};
use crate::midi_input_processor::MidiInputProcessor;
use rubato_types::config::Config;
use rubato_types::play_mode_config::{
    ControllerConfig, KeyboardConfig, MidiConfig, PlayModeConfig,
};
use rubato_types::player_config::PlayerConfig;

pub const KEYSTATE_SIZE: usize = 256;

/// Key logger
struct KeyLogger {
    keylog: Vec<KeyInputLog>,
}

impl KeyLogger {
    pub const INITIAL_LOG_COUNT: usize = 10000;

    pub fn new() -> Self {
        Self {
            keylog: Vec::with_capacity(Self::INITIAL_LOG_COUNT),
        }
    }

    /// Add key input log
    pub fn add(&mut self, presstime: i64, keycode: i32, pressed: bool) {
        self.keylog
            .push(KeyInputLog::with_data(presstime, keycode, pressed));
    }

    /// Clear key log
    pub fn clear(&mut self) {
        self.keylog.clear();
    }

    pub fn to_array(&self) -> &[KeyInputLog] {
        &self.keylog
    }
}

/// Main input manager
///
/// Manages keyboard, controller, and MIDI input
pub struct BMSPlayerInputProcessor {
    enable: bool,

    kbinput: KeyBoardInputProcesseor,

    bminput: Vec<BMControllerInputProcessor>,

    midiinput: MidiInputProcessor,

    keylog: KeyLogger,

    /// Each key ON/OFF state
    /// Sized to fit all mode inputs
    keystate: [bool; KEYSTATE_SIZE],
    /// Each key last update time
    time: [i64; KEYSTATE_SIZE],

    /// Analog scroll for song select bar and lane cover
    analog_scroll: bool,
    /// Analog state for song select bar scrolling
    /// (analog state of each key)
    is_analog: [bool; KEYSTATE_SIZE],
    last_analog_value: [f32; KEYSTATE_SIZE],
    current_analog_value: [f32; KEYSTATE_SIZE],
    analog_last_reset_time: [i64; KEYSTATE_SIZE],

    last_key_device: Option<DeviceType>,

    starttime: i64,
    micro_margin_time: i64,

    pub mousex: i32,
    pub mousey: i32,
    pub mousebutton: i32,
    pub mousepressed: bool,
    pub mousedragged: bool,
    pub mouse_moved: bool,

    pub scroll_x: f32,
    pub scroll_y: f32,

    start_pressed: bool,
    pub select_pressed: bool,

    device_type: DeviceType,

    controller_manager: Lwjgl3ControllerManager,
}

impl BMSPlayerInputProcessor {
    pub fn new(config: &Config, _player: &PlayerConfig) -> Self {
        Self::create(config, true)
    }

    /// Create an input processor without opening MIDI ports.
    /// Used by wrapper processors (DecideMainControllerRef, ResultMainController, etc.)
    /// that mirror state from the real processor and never poll devices directly.
    pub fn new_without_midi(config: &Config) -> Self {
        Self::create(config, false)
    }

    fn create(config: &Config, open_midi: bool) -> Self {
        let resolution = config.display.resolution;
        let default_kb_config = KeyboardConfig::default();
        let kbinput = KeyBoardInputProcesseor::new(&default_kb_config, resolution);
        // Gdx.input.setInputProcessor(kbinput);

        // Controllers.preferredManager = "bms.player.beatoraja.controller.Lwjgl3ControllerManager";
        let controller_manager = Lwjgl3ControllerManager::new();

        // In Java: for (Controller c : Controllers.getControllers()) { bminput.add(new BMControllerInputProcessor(c, ...)); }
        // Limitation: bminput is populated once from the startup controller snapshot.
        // Controller hotplug/disconnect after construction is not supported;
        // a restart is required. This matches Java's behavior where controllers
        // are enumerated once at BMSPlayerInputProcessor construction.
        let default_controller_config = ControllerConfig::default();
        let mut bminput: Vec<BMControllerInputProcessor> = Vec::new();
        for ctrl in &controller_manager.controllers {
            // Device name uniqueness (Java: デバイス名のユニーク化)
            let mut index = 1;
            let mut name = ctrl.name.clone();
            for bm in &bminput {
                if bm.name() == name {
                    index += 1;
                    name = format!("{}-{}", ctrl.name, index);
                }
            }
            let controller = GdxController::with_state(
                name.clone(),
                ctrl.button_state.len(),
                ctrl.axis_state.len(),
            );
            bminput.push(BMControllerInputProcessor::new(
                name,
                controller,
                &default_controller_config,
            ));
        }

        let mut midiinput = MidiInputProcessor::new();
        if open_midi {
            midiinput.open();
        }
        let midi_config = MidiConfig::default();
        midiinput.set_config(&midi_config);

        let analog_scroll = config.select.analog_scroll;

        Self {
            enable: true,
            kbinput,
            bminput,
            midiinput,
            keylog: KeyLogger::new(),
            keystate: [false; KEYSTATE_SIZE],
            time: [i64::MIN; KEYSTATE_SIZE],
            analog_scroll,
            is_analog: [false; KEYSTATE_SIZE],
            last_analog_value: [0.0; KEYSTATE_SIZE],
            current_analog_value: [0.0; KEYSTATE_SIZE],
            analog_last_reset_time: [0; KEYSTATE_SIZE],
            last_key_device: None,
            starttime: 0,
            micro_margin_time: 0,
            mousex: 0,
            mousey: 0,
            mousebutton: 0,
            mousepressed: false,
            mousedragged: false,
            mouse_moved: false,
            scroll_x: 0.0,
            scroll_y: 0.0,
            start_pressed: false,
            select_pressed: false,
            device_type: DeviceType::Keyboard,
            controller_manager,
        }
    }

    pub fn set_keyboard_config(&mut self, config: &KeyboardConfig) {
        self.kbinput.set_config(config);
    }

    pub fn set_controller_config(&mut self, configs: &mut [ControllerConfig]) {
        let mut b = vec![false; configs.len()];
        for controller in self.bminput.iter_mut() {
            controller.enabled = false;
            for (i, config) in configs.iter_mut().enumerate() {
                if b[i] {
                    continue;
                }
                if config.name().is_none() || config.name().is_some_and(|n| n.is_empty()) {
                    config.name = controller.name().to_string();
                }
                if controller.name() == config.name().unwrap_or("") {
                    controller.set_config(config);
                    controller.enabled = true;
                    b[i] = true;
                    break;
                }
            }
        }
    }

    pub fn set_midi_config(&mut self, config: &MidiConfig) {
        self.midiinput.set_config(config);
    }

    pub fn set_start_time(&mut self, starttime: i64) {
        self.starttime = starttime;
        if starttime != 0 {
            self.reset_all_key_changed_time();
            self.keylog.clear();
            self.kbinput.clear();
            for bm in self.bminput.iter_mut() {
                bm.clear();
            }
        }
        self.midiinput.starttime = starttime;
    }

    pub fn set_key_log_margin_time(&mut self, milli_margin_time: i64) {
        self.micro_margin_time = milli_margin_time * 1000;
    }

    pub fn start_time(&self) -> i64 {
        self.starttime
    }

    /// Returns the key state for the specified key ID
    pub fn key_state(&self, id: i32) -> bool {
        if id >= 0 && (id as usize) < self.keystate.len() {
            self.keystate[id as usize]
        } else {
            false
        }
    }

    /// Sets the key state for the specified key ID
    pub fn set_key_state(&mut self, id: i32, pressed: bool, time: i64) {
        if id >= 0 && (id as usize) < self.keystate.len() {
            self.keystate[id as usize] = pressed;
            self.time[id as usize] = time;
        }
    }

    /// Returns the key state change time for the specified key ID
    pub fn key_changed_time(&self, id: i32) -> i64 {
        if id >= 0 && (id as usize) < self.time.len() {
            self.time[id as usize]
        } else {
            i64::MIN
        }
    }

    /// Resets the key state change time for the specified key ID
    pub fn reset_key_changed_time(&mut self, id: i32) -> bool {
        if id >= 0 && (id as usize) < self.time.len() {
            let result = self.time[id as usize] != i64::MIN;
            self.time[id as usize] = i64::MIN;
            result
        } else {
            false
        }
    }

    /// Reset all key states
    pub fn reset_all_key_state(&mut self) {
        self.keystate.fill(false);
        self.time.fill(i64::MIN);
    }

    /// Reset all key state change times
    pub fn reset_all_key_changed_time(&mut self) {
        self.time.fill(i64::MIN);
    }

    pub fn last_key_changed_device(&self) -> Option<DeviceType> {
        self.last_key_device
    }

    pub fn number_of_device(&self) -> usize {
        self.bminput.len() + 1
    }

    fn clear_live_game_input_state(&mut self) {
        self.reset_all_key_state();
        self.kbinput.clear();
        for bm in self.bminput.iter_mut() {
            bm.clear();
        }
        self.midiinput.clear();
    }

    pub fn set_play_config(&mut self, playconfig: &mut PlayModeConfig) {
        // Changing the gameplay mapping must drop any pressed state carried
        // over from the previous state, otherwise PLAY can start with held
        // beams / judgments from stale inputs.
        self.clear_live_game_input_state();

        // KB, controller, Midi exclusive button processing
        let mut kbkeys = playconfig.keyboard.keys.to_vec();
        let mut exclusive = vec![false; kbkeys.len()];

        let kbcount = Self::set_play_config0(&mut kbkeys, &mut exclusive);
        // Write back mutated keys so exclusive deduplication propagates
        // (Java arrays are passed by reference; Rust clones need explicit write-back)
        playconfig.keyboard.keys = kbkeys;

        let mut cokeys: Vec<Vec<i32>> = Vec::new();
        let mut cocount = 0;
        for controller in &playconfig.controller {
            cokeys.push(controller.keys.to_vec());
        }
        for item in &mut cokeys {
            cocount += Self::set_play_config0(item, &mut exclusive);
        }
        // Write back mutated controller keys
        for (i, item) in cokeys.into_iter().enumerate() {
            playconfig.controller[i].keys = item;
        }

        let mut midi_keys_mut = playconfig.midi.keys.to_vec();
        let mut micount = 0;
        for (key, excl) in midi_keys_mut.iter_mut().zip(exclusive.iter_mut()) {
            if *excl {
                *key = None;
            } else if key.is_some() {
                *excl = true;
                micount += 1;
            }
        }
        // Write back mutated MIDI keys
        playconfig.midi.keys = midi_keys_mut;

        // Set key configs for each device
        self.kbinput.set_config(&playconfig.keyboard);
        let controllers = &mut playconfig.controller;
        self.set_controller_config(controllers);
        self.midiinput.set_config(&playconfig.midi);

        if kbcount >= cocount && kbcount >= micount {
            self.device_type = DeviceType::Keyboard;
        } else if cocount >= kbcount && cocount >= micount {
            self.device_type = DeviceType::BmController;
        } else {
            self.device_type = DeviceType::Midi;
        }
    }

    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }

    fn set_play_config0(keys: &mut [i32], exclusive: &mut [bool]) -> i32 {
        let mut count = 0;
        for (key, excl) in keys.iter_mut().zip(exclusive.iter_mut()) {
            if *excl {
                *key = -1;
            } else if *key != -1 {
                *excl = true;
                count += 1;
            }
        }
        count
    }

    pub fn set_enable(&mut self, enable: bool) {
        self.enable = enable;
        if !enable {
            self.clear_live_game_input_state();
        }
    }

    pub fn control_key_state(&self, key: ControlKeys) -> bool {
        self.kbinput.key_state(key.keycode())
    }

    /// Returns true if either Alt key is currently held.
    ///
    /// Translated from: input.getKeyState(Input.Keys.ALT_LEFT) || input.getKeyState(Input.Keys.ALT_RIGHT)
    pub fn is_alt_held(&self) -> bool {
        use crate::gdx_compat::GdxInput;
        use crate::keys::Keys;
        GdxInput::is_key_pressed(Keys::ALT_LEFT) || GdxInput::is_key_pressed(Keys::ALT_RIGHT)
    }

    pub fn is_control_key_pressed(&mut self, key: ControlKeys) -> bool {
        self.kbinput.is_key_pressed(key.keycode())
    }

    pub fn is_control_key_pressed_with_modifiers(
        &mut self,
        key: ControlKeys,
        held_modifiers: i32,
        not_held_modifiers: &[i32],
    ) -> bool {
        self.kbinput.is_key_pressed_with_modifiers(
            key.keycode(),
            held_modifiers,
            not_held_modifiers,
        )
    }

    fn key_changed_internal(
        &mut self,
        device: DeviceType,
        presstime: i64,
        i: usize,
        pressed: bool,
    ) {
        if !self.enable {
            return;
        }
        if self.keystate[i] != pressed {
            self.keystate[i] = pressed;
            self.time[i] = presstime;
            self.last_key_device = Some(device);
            if self.starttime != 0 {
                self.keylog
                    .add(presstime - self.micro_margin_time, i as i32, pressed);
            }
        }
    }

    fn set_analog_state_internal(&mut self, i: usize, _is_analog: bool, _analog_value: f32) {
        if !self.enable {
            return;
        }
        if self.analog_scroll {
            self.is_analog[i] = _is_analog;
            self.current_analog_value[i] = _analog_value;
        } else {
            self.is_analog[i] = false;
            self.current_analog_value[i] = 0.0;
        }
    }

    /// Injects analog state into the runtime snapshot.
    ///
    /// Used by cross-crate state wrappers that need to mirror live controller state
    /// without polling device backends directly.
    pub fn set_analog_state(&mut self, i: usize, is_analog: bool, analog_value: f32) {
        self.set_analog_state_internal(i, is_analog, analog_value);
    }

    pub fn reset_analog_input(&mut self, i: usize) {
        self.last_analog_value[i] = self.current_analog_value[i];
        self.analog_last_reset_time[i] = monotonic_micros();
    }

    pub fn time_since_last_analog_reset(&self, i: usize) -> i64 {
        (monotonic_micros() - self.analog_last_reset_time[i]) / 1000
    }

    pub fn analog_diff(&self, i: usize) -> i32 {
        compute_analog_diff(self.last_analog_value[i], self.current_analog_value[i])
    }

    pub fn is_analog_input(&self, i: usize) -> bool {
        self.is_analog[i]
    }

    pub fn analog_diff_and_reset(&mut self, i: usize, ms_tolerance: i32) -> i32 {
        let mut d_ticks = 0;
        if self.time_since_last_analog_reset(i) <= ms_tolerance as i64 {
            d_ticks = 0.max(self.analog_diff(i));
        }
        self.reset_analog_input(i);
        d_ticks
    }

    pub fn key_input_log(&self) -> &[KeyInputLog] {
        self.keylog.to_array()
    }

    /// Sets the start button pressed state.
    ///
    /// Translated from: BMSPlayerInputProcessor.startChanged(boolean)
    pub fn start_changed(&mut self, pressed: bool) {
        self.start_pressed = pressed;
    }

    pub fn start_pressed(&self) -> bool {
        self.start_pressed
    }

    pub fn is_activated(&mut self, key: KeyCommand) -> bool {
        let mask_ctrl = MASK_CTRL;
        let mask_ctrl_shift = MASK_CTRL | MASK_SHIFT;

        match key {
            KeyCommand::ShowFps => self.is_control_key_pressed(ControlKeys::F1),
            KeyCommand::UpdateFolder => self.is_control_key_pressed(ControlKeys::F2),
            KeyCommand::OpenExplorer => self.is_control_key_pressed_with_modifiers(
                ControlKeys::F3,
                0,
                &[mask_ctrl, mask_ctrl_shift],
            ),
            KeyCommand::CopySongMd5Hash => self.is_control_key_pressed_with_modifiers(
                ControlKeys::F3,
                mask_ctrl,
                &[mask_ctrl_shift],
            ),
            KeyCommand::CopySongSha256Hash => {
                self.is_control_key_pressed_with_modifiers(ControlKeys::F3, mask_ctrl_shift, &[])
            }
            KeyCommand::SwitchScreenMode => self.is_control_key_pressed(ControlKeys::F4),
            KeyCommand::SaveScreenshot => self.is_control_key_pressed(ControlKeys::F6),
            KeyCommand::AddFavoriteSong => self.is_control_key_pressed(ControlKeys::F8),
            KeyCommand::AddFavoriteChart => self.is_control_key_pressed(ControlKeys::F9),
            KeyCommand::AutoplayFolder => self.is_control_key_pressed(ControlKeys::F10),
            KeyCommand::OpenIr => self.is_control_key_pressed(ControlKeys::F11),
            KeyCommand::OpenSkinConfiguration => self.is_control_key_pressed(ControlKeys::F12),
            KeyCommand::ToggleModMenu => {
                self.is_control_key_pressed(ControlKeys::F5)
                    || self.is_control_key_pressed(ControlKeys::Insert)
            }
            KeyCommand::CopyHighlightedMenuText => {
                self.is_control_key_pressed_with_modifiers(ControlKeys::KeyC, mask_ctrl, &[])
            }
        }
    }

    pub fn is_select_pressed(&self) -> bool {
        self.select_pressed
    }
    pub fn keyboard_input_processor(&self) -> &KeyBoardInputProcesseor {
        &self.kbinput
    }

    pub fn keyboard_input_processor_mut(&mut self) -> &mut KeyBoardInputProcesseor {
        &mut self.kbinput
    }

    pub fn bm_input_processor(&self) -> &[BMControllerInputProcessor] {
        &self.bminput
    }

    pub fn midi_input_processor(&self) -> &MidiInputProcessor {
        &self.midiinput
    }

    pub fn is_mouse_pressed(&self) -> bool {
        self.mousepressed
    }

    pub fn consume_mouse_pressed(&mut self) {
        self.mousepressed = false;
    }

    pub fn is_mouse_dragged(&self) -> bool {
        self.mousedragged
    }

    pub fn consume_mouse_dragged(&mut self) {
        self.mousedragged = false;
    }

    pub fn is_mouse_moved(&self) -> bool {
        self.mouse_moved
    }
    pub fn scroll(&self) -> i32 {
        -(self.scroll_y as i32)
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
    }

    pub fn poll(&mut self) {
        let now = (rubato_types::monotonic_clock::monotonic_micros() - self.starttime).max(0);

        // Poll keyboard
        // We need to use a temporary struct to act as callback since
        // kbinput.poll needs &mut self for the callback methods
        let mut kb_events = KbEvents::default();
        self.kbinput.poll(now, &mut kb_events);
        // Apply keyboard events
        for event in &kb_events.key_events {
            self.key_changed_internal(
                DeviceType::Keyboard,
                event.microtime,
                event.key,
                event.pressed,
            );
        }
        for event in &kb_events.analog_events {
            self.set_analog_state_internal(event.key, event.is_analog, event.value);
        }
        if let Some(pressed) = kb_events.start_changed {
            self.start_pressed = pressed;
        }
        if let Some(pressed) = kb_events.select_changed {
            self.select_pressed = pressed;
        }
        if let Some(moved) = kb_events.mouse_moved {
            self.mouse_moved = moved;
        }
        if let Some(x) = kb_events.mouse_x {
            self.mousex = x;
        }
        if let Some(y) = kb_events.mouse_y {
            self.mousey = y;
        }
        if let Some(button) = kb_events.mouse_button {
            self.mousebutton = button;
        }
        if let Some(pressed) = kb_events.mouse_pressed {
            self.mousepressed = pressed;
        }
        if let Some(dragged) = kb_events.mouse_dragged {
            self.mousedragged = dragged;
        }
        self.scroll_x += kb_events.scroll_x;
        self.scroll_y += kb_events.scroll_y;

        // Read mouse button, drag, and scroll from SharedKeyState (winit events).
        // In Java, mouse events arrive via InputProcessor callbacks (touchDown,
        // touchDragged, scrolled). In Rust, winit events are written to
        // SharedKeyState and polled here.
        {
            use crate::gdx_compat::{GdxGraphics, GdxInput};
            use crate::winit_input_bridge::MOUSE_BUTTON_LEFT;
            let left_pressed = GdxInput::is_button_pressed(MOUSE_BUTTON_LEFT);
            if left_pressed && !self.mousepressed {
                self.mousepressed = true;
                self.mousebutton = MOUSE_BUTTON_LEFT;
                // Apply the same resolution transform as touch_down()
                let gw = GdxGraphics::get_width();
                let gh = GdxGraphics::get_height();
                let res = self.kbinput.resolution();
                if gw > 0 && gh > 0 {
                    self.mousex = GdxInput::get_x() * res.width() / gw;
                    self.mousey = res.height() - GdxInput::get_y() * res.height() / gh;
                }
            } else if !left_pressed && self.mousepressed {
                self.mousepressed = false;
            }
            if GdxInput::drain_mouse_dragged() {
                self.mousedragged = true;
                let gw = GdxGraphics::get_width();
                let gh = GdxGraphics::get_height();
                let res = self.kbinput.resolution();
                if gw > 0 && gh > 0 {
                    self.mousex = GdxInput::get_x() * res.width() / gw;
                    self.mousey = res.height() - GdxInput::get_y() * res.height() / gh;
                }
            }
            let (sdx, sdy) = GdxInput::drain_scroll();
            self.scroll_x += sdx;
            self.scroll_y += sdy;
        }

        // Update controller state from manager
        self.controller_manager.poll_state();
        for (idx, bm) in self.bminput.iter_mut().enumerate() {
            if idx < self.controller_manager.controllers.len() {
                let mgr_ctrl = &self.controller_manager.controllers[idx];
                bm.controller.axis_state.clone_from(&mgr_ctrl.axis_state);
                bm.controller
                    .button_state
                    .clone_from(&mgr_ctrl.button_state);
            }
        }

        // Poll controllers
        for idx in 0..self.bminput.len() {
            let mut ctrl_events = CtrlEvents::default();
            // We need to use unsafe to split the borrow since poll needs &mut self
            // but we also need &mut self for callback. Instead, collect events.
            self.bminput[idx].poll(now, &mut ctrl_events, idx);
            for event in &ctrl_events.key_events {
                self.key_changed_internal(
                    DeviceType::BmController,
                    event.microtime,
                    event.key,
                    event.pressed,
                );
            }
            for event in &ctrl_events.analog_events {
                self.set_analog_state_internal(event.key, event.is_analog, event.value);
            }
            if let Some(pressed) = ctrl_events.start_changed {
                self.start_pressed = pressed;
            }
            if let Some(pressed) = ctrl_events.select_changed {
                self.select_pressed = pressed;
            }
        }

        // Poll MIDI
        let mut midi_events = MidiEvents::default();
        self.midiinput.poll(&mut midi_events);
        for event in &midi_events.key_events {
            self.key_changed_internal(DeviceType::Midi, event.microtime, event.key, event.pressed);
        }
        for event in &midi_events.analog_events {
            self.set_analog_state_internal(event.key, event.is_analog, event.value);
        }
        if let Some(pressed) = midi_events.start_changed {
            self.start_pressed = pressed;
        }
        if let Some(pressed) = midi_events.select_changed {
            self.select_pressed = pressed;
        }
    }

    pub fn dispose(&mut self) {
        self.midiinput.close();
    }

    pub fn sync_runtime_state_from(&mut self, source: &Self) {
        self.enable = source.enable;
        self.kbinput.sync_runtime_state_from(&source.kbinput);
        self.keystate = source.keystate;
        self.time = source.time;
        self.is_analog = source.is_analog;
        self.last_analog_value = source.last_analog_value;
        self.current_analog_value = source.current_analog_value;
        self.analog_last_reset_time = source.analog_last_reset_time;
        self.last_key_device = source.last_key_device;
        self.mousex = source.mousex;
        self.mousey = source.mousey;
        self.mousebutton = source.mousebutton;
        self.mousepressed = source.mousepressed;
        self.mousedragged = source.mousedragged;
        self.mouse_moved = source.mouse_moved;
        self.scroll_x = source.scroll_x;
        self.scroll_y = source.scroll_y;
        self.start_pressed = source.start_pressed;
        self.select_pressed = source.select_pressed;
        self.device_type = source.device_type;
    }
}

/// Temporary struct to collect keyboard callback events
#[derive(Default)]
struct KbEvents {
    key_events: Vec<KeyEvent>,
    analog_events: Vec<AnalogEvent>,
    start_changed: Option<bool>,
    select_changed: Option<bool>,
    mouse_moved: Option<bool>,
    mouse_x: Option<i32>,
    mouse_y: Option<i32>,
    mouse_button: Option<i32>,
    mouse_pressed: Option<bool>,
    mouse_dragged: Option<bool>,
    scroll_x: f32,
    scroll_y: f32,
}

struct KeyEvent {
    microtime: i64,
    key: usize,
    pressed: bool,
}

struct AnalogEvent {
    key: usize,
    is_analog: bool,
    value: f32,
}

impl KeyboardCallback for KbEvents {
    fn key_changed_from_keyboard(&mut self, microtime: i64, key: usize, pressed: bool) {
        self.key_events.push(KeyEvent {
            microtime,
            key,
            pressed,
        });
    }

    fn start_changed(&mut self, pressed: bool) {
        self.start_changed = Some(pressed);
    }

    fn set_select_pressed(&mut self, pressed: bool) {
        self.select_changed = Some(pressed);
    }

    fn set_analog_state(&mut self, key: usize, is_analog: bool, value: f32) {
        self.analog_events.push(AnalogEvent {
            key,
            is_analog,
            value,
        });
    }

    fn set_mouse_moved(&mut self, moved: bool) {
        self.mouse_moved = Some(moved);
    }

    fn set_mouse_x(&mut self, x: i32) {
        self.mouse_x = Some(x);
    }

    fn set_mouse_y(&mut self, y: i32) {
        self.mouse_y = Some(y);
    }

    fn set_mouse_button(&mut self, button: i32) {
        self.mouse_button = Some(button);
    }

    fn set_mouse_pressed(&mut self, pressed: bool) {
        self.mouse_pressed = Some(pressed);
    }

    fn set_mouse_dragged(&mut self, dragged: bool) {
        self.mouse_dragged = Some(dragged);
    }

    fn add_scroll_x(&mut self, amount: f32) {
        self.scroll_x += amount;
    }

    fn add_scroll_y(&mut self, amount: f32) {
        self.scroll_y += amount;
    }
}

/// Temporary struct to collect controller callback events
#[derive(Default)]
struct CtrlEvents {
    key_events: Vec<KeyEvent>,
    analog_events: Vec<AnalogEvent>,
    start_changed: Option<bool>,
    select_changed: Option<bool>,
}

impl BMControllerCallback for CtrlEvents {
    fn key_changed_from_controller(
        &mut self,
        _device_index: usize,
        microtime: i64,
        key: usize,
        pressed: bool,
    ) {
        self.key_events.push(KeyEvent {
            microtime,
            key,
            pressed,
        });
    }

    fn start_changed(&mut self, pressed: bool) {
        self.start_changed = Some(pressed);
    }

    fn set_select_pressed(&mut self, pressed: bool) {
        self.select_changed = Some(pressed);
    }

    fn set_analog_state(&mut self, key: usize, is_analog: bool, value: f32) {
        self.analog_events.push(AnalogEvent {
            key,
            is_analog,
            value,
        });
    }
}

#[derive(Default)]
struct MidiEvents {
    key_events: Vec<KeyEvent>,
    analog_events: Vec<AnalogEvent>,
    start_changed: Option<bool>,
    select_changed: Option<bool>,
}

impl crate::midi_input_processor::MidiCallback for MidiEvents {
    fn key_changed_from_midi(&mut self, microtime: i64, key: usize, pressed: bool) {
        self.key_events.push(KeyEvent {
            microtime,
            key,
            pressed,
        });
    }

    fn start_changed(&mut self, pressed: bool) {
        self.start_changed = Some(pressed);
    }

    fn set_select_pressed(&mut self, pressed: bool) {
        self.select_changed = Some(pressed);
    }

    fn set_analog_state(&mut self, key: usize, is_analog: bool, value: f32) {
        self.analog_events.push(AnalogEvent {
            key,
            is_analog,
            value,
        });
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::keys::Keys;
    use crate::winit_input_bridge::SharedKeyState;
    use rubato_types::config::Config;
    use rubato_types::player_config::PlayerConfig;

    fn make_input_processor() -> BMSPlayerInputProcessor {
        let config = Config::default();
        let player = PlayerConfig::default();
        BMSPlayerInputProcessor::new(&config, &player)
    }

    #[test]
    fn test_initial_key_states_all_false() {
        let proc = make_input_processor();
        for i in 0..KEYSTATE_SIZE as i32 {
            assert!(!proc.key_state(i));
        }
    }

    #[test]
    fn test_set_and_get_key_state() {
        let mut proc = make_input_processor();
        proc.set_key_state(0, true, 1000);
        assert!(proc.key_state(0));
        assert_eq!(proc.key_changed_time(0), 1000);

        proc.set_key_state(0, false, 2000);
        assert!(!proc.key_state(0));
        assert_eq!(proc.key_changed_time(0), 2000);
    }

    #[test]
    fn test_reset_key_changed_time() {
        let mut proc = make_input_processor();
        proc.set_key_state(5, true, 1000);
        assert!(proc.reset_key_changed_time(5));
        assert_eq!(proc.key_changed_time(5), i64::MIN);
        // Second reset should return false since already reset
        assert!(!proc.reset_key_changed_time(5));
    }

    #[test]
    fn test_reset_all_key_state() {
        let mut proc = make_input_processor();
        proc.set_key_state(0, true, 1000);
        proc.set_key_state(1, true, 2000);
        proc.reset_all_key_state();
        assert!(!proc.key_state(0));
        assert!(!proc.key_state(1));
        assert_eq!(proc.key_changed_time(0), i64::MIN);
    }

    #[test]
    fn test_set_start_time_resets_state() {
        let mut proc = make_input_processor();
        proc.set_key_state(0, true, 1000);
        proc.set_start_time(5000);
        // After setStartTime(nonzero), times should be reset
        assert_eq!(proc.key_changed_time(0), i64::MIN);
        assert_eq!(proc.start_time(), 5000);
    }

    #[test]
    fn test_key_log_margin_time() {
        let mut proc = make_input_processor();
        proc.set_key_log_margin_time(10);
        // micro_margin_time should be 10 * 1000 = 10000
        // This is internal; just verify no panic
    }

    #[test]
    fn test_mouse_state() {
        let proc = make_input_processor();
        assert_eq!(proc.mousex, 0);
        assert_eq!(proc.mousey, 0);
        assert!(!proc.is_mouse_pressed());
        assert!(!proc.is_mouse_dragged());
        assert!(!proc.is_mouse_moved());
        assert_eq!(proc.scroll(), 0);
    }

    #[test]
    fn test_start_and_select_pressed() {
        let mut proc = make_input_processor();
        assert!(!proc.start_pressed());
        assert!(!proc.is_select_pressed());

        proc.start_changed(true);
        assert!(proc.start_pressed());

        proc.select_pressed = true;
        assert!(proc.is_select_pressed());
    }

    #[test]
    fn test_enable_disable() {
        let mut proc = make_input_processor();
        proc.set_key_state(0, true, 1000);
        proc.set_enable(false);
        // Disable should reset all state
        assert!(!proc.key_state(0));
    }

    #[test]
    fn test_get_key_state_out_of_range() {
        let proc = make_input_processor();
        assert!(!proc.key_state(-1));
        assert!(!proc.key_state(256));
        assert!(!proc.key_state(1000));
    }

    #[test]
    fn test_number_of_device() {
        let proc = make_input_processor();
        // 1 (keyboard) + 0 controllers = 1
        assert_eq!(proc.number_of_device(), 1);
    }

    #[test]
    fn test_device_type_default() {
        let proc = make_input_processor();
        assert_eq!(proc.device_type(), DeviceType::Keyboard);
    }

    #[test]
    fn test_poll_with_shared_key_state() {
        // Set up the shared key state with RAII guard for cleanup
        let shared_state = SharedKeyState::new();
        let _guard = crate::gdx_compat::set_shared_key_state_guarded(shared_state.clone());

        // Use a config with duration=0 to avoid timing issues in tests
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut proc = BMSPlayerInputProcessor::new(&config, &player);
        // Override keyboard config with zero duration and explicit start/select keys
        let mut kb_config = KeyboardConfig::default();
        kb_config.duration = 0;
        kb_config.start = Keys::Q;
        kb_config.select = Keys::W;
        proc.set_keyboard_config(&kb_config);

        // Press the Z key (default keys[0] = Keys::Z = 54)
        shared_state.set_key_pressed(Keys::Z, true);

        // Poll should detect the key press
        proc.poll();

        // The first key in default config maps to key index 0
        // keystate[0] should now be true
        assert!(
            proc.key_state(0),
            "key Z press should be detected as key index 0"
        );

        // Also test start key: default start key = Keys::Q = 45
        shared_state.set_key_pressed(Keys::Q, true);
        proc.poll();
        assert!(proc.start_pressed(), "start key (Q) should be detected");

        // Also test select key: default select key = Keys::W = 51
        shared_state.set_key_pressed(Keys::W, true);
        proc.poll();
        assert!(
            proc.is_select_pressed(),
            "select key (W) should be detected"
        );

        // Release Z key
        shared_state.set_key_pressed(Keys::Z, false);
        proc.poll();
        assert!(!proc.key_state(0), "key Z release should be detected");
    }

    #[test]
    fn test_scroll_state() {
        let mut proc = make_input_processor();
        assert_eq!(proc.scroll_x, 0.0);
        assert_eq!(proc.scroll_y, 0.0);

        proc.reset_scroll();
        assert_eq!(proc.scroll_x, 0.0);
        assert_eq!(proc.scroll_y, 0.0);
    }

    #[test]
    fn test_key_input_log_empty_initially() {
        let proc = make_input_processor();
        let log = proc.key_input_log();
        assert!(log.is_empty());
    }

    /// Verify the device name uniqueness algorithm matches Java behavior.
    /// Java: index starts at 1, increments for each existing duplicate, appends "-{index}".
    #[test]
    fn test_device_name_uniqueness_algorithm() {
        let raw_names = ["Pad A", "Pad A", "Pad B", "Pad A"];
        let mut unique_names: Vec<String> = Vec::new();
        for raw in &raw_names {
            let mut index = 1;
            let mut name = raw.to_string();
            for existing in &unique_names {
                if *existing == name {
                    index += 1;
                    name = format!("{}-{}", raw, index);
                }
            }
            unique_names.push(name);
        }
        assert_eq!(unique_names, vec!["Pad A", "Pad A-2", "Pad B", "Pad A-3"]);
    }

    #[test]
    fn test_device_name_uniqueness_no_duplicates() {
        let raw_names = ["Pad A", "Pad B", "Pad C"];
        let mut unique_names: Vec<String> = Vec::new();
        for raw in &raw_names {
            let mut index = 1;
            let mut name = raw.to_string();
            for existing in &unique_names {
                if *existing == name {
                    index += 1;
                    name = format!("{}-{}", raw, index);
                }
            }
            unique_names.push(name);
        }
        assert_eq!(unique_names, vec!["Pad A", "Pad B", "Pad C"]);
    }

    // -- Finding 1: exclusive key processing write-back --

    #[test]
    fn test_set_play_config_exclusive_keys_written_back_to_keyboard() {
        use rubato_types::play_mode_config::PlayModeConfig;

        let mut proc = make_input_processor();
        let mut playconfig = PlayModeConfig::default();

        // Set keyboard key[0] to some value
        playconfig.keyboard.keys[0] = 42;
        // Set controller key[0] to the same value (duplicate)
        if !playconfig.controller.is_empty() {
            playconfig.controller[0].keys[0] = 42;
        }

        proc.set_play_config(&mut playconfig);

        // Keyboard processes first, so it keeps key[0] = 42.
        // Controller key[0] should be set to -1 because it was exclusive-deduped.
        assert_eq!(
            playconfig.keyboard.keys[0], 42,
            "keyboard key[0] should be preserved (processed first)"
        );
        if !playconfig.controller.is_empty() {
            assert_eq!(
                playconfig.controller[0].keys[0], -1,
                "controller key[0] should be deduped to -1"
            );
        }
    }

    #[test]
    fn test_set_play_config_clears_stale_game_key_state() {
        use rubato_types::play_mode_config::PlayModeConfig;

        let mut proc = make_input_processor();
        let mut playconfig = PlayModeConfig::default();

        proc.set_key_state(0, true, 123_456);
        proc.set_key_state(1, true, 234_567);

        proc.set_play_config(&mut playconfig);

        assert!(
            !proc.key_state(0),
            "play config install must clear stale pressed state for key 0"
        );
        assert!(
            !proc.key_state(1),
            "play config install must clear stale pressed state for key 1"
        );
        assert_eq!(
            proc.key_changed_time(0),
            i64::MIN,
            "play config install must clear stale change times for key 0"
        );
        assert_eq!(
            proc.key_changed_time(1),
            i64::MIN,
            "play config install must clear stale change times for key 1"
        );
    }

    #[test]
    fn test_set_play_config_exclusive_keys_written_back_to_midi() {
        use rubato_types::play_mode_config::PlayModeConfig;

        let mut proc = make_input_processor();
        let mut playconfig = PlayModeConfig::default();

        // Set keyboard key[0] to some non-negative value
        playconfig.keyboard.keys[0] = 42;
        // Set midi key[0] to Some
        if !playconfig.midi.keys.is_empty() {
            playconfig.midi.keys[0] = Some(rubato_types::play_mode_config::MidiInput::default());
        }

        proc.set_play_config(&mut playconfig);

        // Keyboard processes first, taking exclusive ownership of slot 0.
        // MIDI key[0] should be set to None because keyboard claimed slot 0.
        if !playconfig.midi.keys.is_empty() {
            assert!(
                playconfig.midi.keys[0].is_none(),
                "midi key[0] should be deduped to None"
            );
        }
    }

    /// Regression: MIDI exclusive dedup must not count None (unbound) slots as
    /// claimed bindings. Counting None slots inflates micount, which can cause
    /// device_type to be incorrectly set to Midi when no MIDI keys are bound.
    #[test]
    fn test_midi_dedup_does_not_count_none_slots() {
        use rubato_types::play_mode_config::PlayModeConfig;

        let mut proc = make_input_processor();
        let mut playconfig = PlayModeConfig::default();

        // Clear all keyboard bindings so kbcount = 0
        for k in playconfig.keyboard.keys.iter_mut() {
            *k = -1;
        }
        // Clear all controller bindings so cocount = 0
        for controller in playconfig.controller.iter_mut() {
            for k in controller.keys.iter_mut() {
                *k = -1;
            }
        }
        // Clear all MIDI bindings to None so micount should be 0
        for k in playconfig.midi.keys.iter_mut() {
            *k = None;
        }

        proc.set_play_config(&mut playconfig);

        // With all counts at 0, kbcount (0) >= cocount (0) && kbcount (0) >= micount (0)
        // should select Keyboard. Before the fix, micount was inflated by None slots,
        // causing Midi to be selected instead.
        assert_eq!(
            proc.device_type(),
            DeviceType::Keyboard,
            "device_type should be Keyboard when all bindings are empty, not Midi"
        );
    }

    /// Regression: poll() must clamp `now` to >= 0 when starttime is in the future.
    /// A negative `now` would flow into key_changed_internal and store negative
    /// press times, corrupting judge timing.
    #[test]
    fn test_poll_clamps_now_to_non_negative_when_starttime_is_future() {
        let shared_state = SharedKeyState::new();
        let _guard = crate::gdx_compat::set_shared_key_state_guarded(shared_state.clone());

        let config = Config::default();
        let player = PlayerConfig::default();
        let mut proc = BMSPlayerInputProcessor::new(&config, &player);
        let mut kb_config = KeyboardConfig::default();
        kb_config.duration = 0;
        proc.set_keyboard_config(&kb_config);

        // Set starttime far in the future so monotonic_micros() - starttime < 0
        proc.set_start_time(i64::MAX);

        // Press a key and poll
        shared_state.set_key_pressed(Keys::Z, true);
        proc.poll();

        // If the key was detected, its changed time must be >= 0 (clamped)
        if proc.key_state(0) {
            assert!(
                proc.key_changed_time(0) >= 0,
                "key press time must not be negative when starttime is in the future, got {}",
                proc.key_changed_time(0),
            );
        }
        // Even if no key detected (duration gate), verify the internal time
        // array was never written with a negative value by checking all slots
        for i in 0..KEYSTATE_SIZE as i32 {
            let t = proc.key_changed_time(i);
            assert!(
                t == i64::MIN || t >= 0,
                "time[{}] must be either i64::MIN (unset) or >= 0, got {}",
                i,
                t,
            );
        }
    }

    /// Regression: sync_runtime_state_from must copy the enable field.
    /// If the source is disabled, the destination must also become disabled,
    /// otherwise key_changed_internal would still accept input on the copy.
    #[test]
    fn test_sync_runtime_state_from_copies_enable_field() {
        let mut dst = make_input_processor();
        let mut src = make_input_processor();

        // Destination starts enabled (default)
        assert!(dst.enable);

        // Disable source
        src.set_enable(false);
        assert!(!src.enable);

        // Sync should propagate the disabled state
        dst.sync_runtime_state_from(&src);
        assert!(
            !dst.enable,
            "sync_runtime_state_from must copy enable=false from source"
        );

        // Re-enable source and sync again
        src.set_enable(true);
        dst.sync_runtime_state_from(&src);
        assert!(
            dst.enable,
            "sync_runtime_state_from must copy enable=true from source"
        );
    }

    #[test]
    fn test_set_play_config0_exclusive_dedup() {
        // Verify the core deduplication logic
        let mut keys = vec![10, 20, 30];
        let mut exclusive = vec![false, true, false];

        BMSPlayerInputProcessor::set_play_config0(&mut keys, &mut exclusive);

        // key[0]: exclusive[0] was false, key != -1 -> keep, mark exclusive
        assert_eq!(keys[0], 10);
        assert!(exclusive[0]);

        // key[1]: exclusive[1] was true -> key set to -1
        assert_eq!(keys[1], -1);
        assert!(exclusive[1]);

        // key[2]: exclusive[2] was false, key != -1 -> keep, mark exclusive
        assert_eq!(keys[2], 30);
        assert!(exclusive[2]);
    }

    // -- KeyLogger pool removal regression --

    #[test]
    fn test_keylogger_beyond_initial_capacity() {
        // Regression: the old pool-based KeyLogger stopped reusing pooled objects
        // after INITIAL_LOG_COUNT adds, silently degrading to fresh allocations.
        // After removing the pool, all adds are uniform Vec::push.
        let mut logger = KeyLogger::new();
        let count = KeyLogger::INITIAL_LOG_COUNT + 100;
        for i in 0..count {
            logger.add(i as i64, (i % 256) as i32, i % 2 == 0);
        }
        let logs = logger.to_array();
        assert_eq!(logs.len(), count);
        // Verify first and last entries are correct
        assert_eq!(logs[0].time(), 0);
        assert_eq!(logs[0].keycode(), 0);
        assert!(logs[0].is_pressed());
        let last = &logs[count - 1];
        assert_eq!(last.time(), (count - 1) as i64);
        assert_eq!(last.keycode(), ((count - 1) % 256) as i32);
    }

    #[test]
    fn test_keylogger_clear_resets_completely() {
        let mut logger = KeyLogger::new();
        for i in 0..100 {
            logger.add(i, i as i32, true);
        }
        assert_eq!(logger.to_array().len(), 100);
        logger.clear();
        assert!(logger.to_array().is_empty());
        // After clear, adding again should work correctly
        logger.add(999, 42, false);
        assert_eq!(logger.to_array().len(), 1);
        assert_eq!(logger.to_array()[0].time(), 999);
    }
}

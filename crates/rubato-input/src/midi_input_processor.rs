//! MidiInputProcessor - MIDI device input processing
//!
//! Translated from: bms.player.beatoraja.input.MidiInputProcessor
//! Uses midir for real MIDI device enumeration and input.

use crate::bms_player_input_device::{BMSPlayerInputDevice, DeviceType};
use crate::stubs::{MidiConfig, MidiInput, MidiInputType};
use midir::{MidiInput as MidirInput, MidiInputConnection};
use std::sync::mpsc;

const MAX_KEYS: usize = 128;

/// Callback interface for BMSPlayerInputProcessor methods called from MidiInputProcessor
pub trait MidiCallback {
    fn key_changed_from_midi(&mut self, microtime: i64, key: usize, pressed: bool);
    fn start_changed(&mut self, pressed: bool);
    fn set_select_pressed(&mut self, pressed: bool);
    fn set_analog_state(&mut self, key: usize, is_analog: bool, value: f32);
}

/// Key handler type: stores a closure-like mapping from MIDI note -> game key
struct KeyHandler {
    key_index: usize,
    handler_type: KeyHandlerType,
}

enum KeyHandlerType {
    GameKey,
    Start,
    Select,
}

/// MIDI input processor
pub struct MidiInputProcessor {
    // milliseconds
    starttime: i64,

    pitch: i32,

    last_pressed_key_available: bool,
    last_pressed_key: MidiInput,

    // pitch value: -8192 ~ 8191
    pitch_threshold: i32,

    // MIDI note number -> game key number
    // NOTE: this approach does not allow multiple key assignments to one MIDI key
    key_map: Vec<Option<KeyHandler>>,

    pitch_bend_up: Option<KeyHandler>,
    pitch_bend_down: Option<KeyHandler>,

    // Active MIDI input connections (midir auto-disconnects on drop)
    connections: Vec<MidiInputConnection<()>>,

    // Receives (command, data1, data2) from MIDI callback threads
    receiver: Option<mpsc::Receiver<(i32, i32, i32)>>,
}

impl Default for MidiInputProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiInputProcessor {
    pub fn new() -> Self {
        let mut proc = Self {
            starttime: 0,
            pitch: 0,
            last_pressed_key_available: false,
            last_pressed_key: MidiInput::default(),
            pitch_threshold: 8192 / 32,
            key_map: Vec::new(),
            pitch_bend_up: None,
            pitch_bend_down: None,
            connections: Vec::new(),
            receiver: None,
        };
        proc.clear_handlers();
        proc
    }

    pub fn open(&mut self) {
        // Close any existing connections first
        self.close();

        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);

        // Enumerate MIDI input ports
        let midi_in = match MidirInput::new("beatoraja-enum") {
            Ok(m) => m,
            Err(e) => {
                log::warn!("Failed to create MIDI input for enumeration: {}", e);
                return;
            }
        };

        let ports = midi_in.ports();
        log::info!("Found {} MIDI input port(s)", ports.len());

        for port in &ports {
            let port_name = midi_in
                .port_name(port)
                .unwrap_or_else(|_| "unknown".to_string());

            // Each MidirInput::connect() consumes the instance, so create a new one per port
            let midi_in_for_port = match MidirInput::new(&format!("beatoraja-{}", port_name)) {
                Ok(m) => m,
                Err(e) => {
                    log::warn!(
                        "Failed to create MIDI input for port '{}': {}",
                        port_name,
                        e
                    );
                    continue;
                }
            };

            let tx = sender.clone();
            match midi_in_for_port.connect(
                port,
                &port_name,
                move |_timestamp_us, message, _data| {
                    if message.is_empty() {
                        return;
                    }
                    let command = (message[0] & 0xF0) as i32;
                    let data1 = if message.len() > 1 {
                        message[1] as i32
                    } else {
                        0
                    };
                    let data2 = if message.len() > 2 {
                        message[2] as i32
                    } else {
                        0
                    };
                    if let Err(e) = tx.send((command, data1, data2)) {
                        log::warn!(
                            "MIDI event dropped (channel disconnected): command=0x{:02X}, data1={}, data2={} - {}",
                            e.0.0, e.0.1, e.0.2, e
                        );
                    }
                },
                (),
            ) {
                Ok(conn) => {
                    log::info!("Connected to MIDI input port: {}", port_name);
                    self.connections.push(conn);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to connect to MIDI input port '{}': {}",
                        port_name,
                        e
                    );
                }
            }
        }
    }

    pub fn close(&mut self) {
        // Dropping MidiInputConnection auto-disconnects
        self.connections.clear();
        self.receiver = None;
    }

    /// Poll pending MIDI messages from the channel and dispatch them.
    /// Must be called from the main thread each frame.
    pub fn poll(&mut self, callback: &mut dyn MidiCallback) {
        // Temporarily take the receiver to avoid borrow conflict with &mut self
        let receiver = match self.receiver.take() {
            Some(r) => r,
            None => return,
        };

        // Drain all pending messages (non-blocking)
        while let Ok((command, data1, data2)) = receiver.try_recv() {
            self.on_short_message(command, data1, data2, callback);
        }

        // Put the receiver back
        self.receiver = Some(receiver);
    }

    #[allow(clippy::needless_range_loop)]
    pub fn set_config(&mut self, config: &MidiConfig) {
        self.clear_impl();
        self.clear_handlers();

        let keys = config.get_keys();
        for i in 0..keys.len() {
            if let Some(input) = &keys[i] {
                self.set_handler(
                    input,
                    KeyHandler {
                        key_index: i,
                        handler_type: KeyHandlerType::GameKey,
                    },
                );
            }
        }

        if let Some(start) = config.get_start() {
            self.set_handler(
                start,
                KeyHandler {
                    key_index: 0,
                    handler_type: KeyHandlerType::Start,
                },
            );
        }
        if let Some(select) = config.get_select() {
            self.set_handler(
                select,
                KeyHandler {
                    key_index: 0,
                    handler_type: KeyHandlerType::Select,
                },
            );
        }
    }

    pub fn set_start_time(&mut self, starttime: i64) {
        self.starttime = starttime;
    }

    fn clear_impl(&mut self) {
        self.last_pressed_key_available = false;
    }

    pub fn clear_handlers(&mut self) {
        self.key_map = (0..MAX_KEYS).map(|_| None).collect();
        self.pitch_bend_up = None;
        self.pitch_bend_down = None;
    }

    fn set_handler(&mut self, input: &MidiInput, handler: KeyHandler) {
        match input.input_type {
            MidiInputType::NOTE => {
                if input.value >= 0
                    && (input.value as usize) < MAX_KEYS
                    && self.key_map[input.value as usize].is_none()
                {
                    self.key_map[input.value as usize] = Some(handler);
                }
            }
            MidiInputType::PITCH_BEND => {
                if input.value > 0 && self.pitch_bend_up.is_none() {
                    self.pitch_bend_up = Some(handler);
                } else if input.value < 0 && self.pitch_bend_down.is_none() {
                    self.pitch_bend_down = Some(handler);
                }
            }
            MidiInputType::CONTROL_CHANGE => {
                // no-op
            }
        }
    }

    pub fn note_off(&self, num: usize, callback: &mut dyn MidiCallback) {
        if let Some(handler) = &self.key_map[num] {
            Self::dispatch_handler(handler, false, self.current_time(), callback);
        }
    }

    pub fn note_on(&mut self, num: usize, callback: &mut dyn MidiCallback) {
        self.last_pressed_key_available = true;
        self.last_pressed_key.input_type = MidiInputType::NOTE;
        self.last_pressed_key.value = num as i32;
        if let Some(handler) = &self.key_map[num] {
            Self::dispatch_handler(handler, true, self.current_time(), callback);
        }
    }

    fn on_pitch_bend_up(&mut self, pressed: bool, callback: &mut dyn MidiCallback) {
        if pressed {
            self.last_pressed_key_available = true;
            self.last_pressed_key.input_type = MidiInputType::PITCH_BEND;
            self.last_pressed_key.value = 1;
        }
        if let Some(handler) = &self.pitch_bend_up {
            Self::dispatch_handler(handler, pressed, self.current_time(), callback);
        }
    }

    fn on_pitch_bend_down(&mut self, pressed: bool, callback: &mut dyn MidiCallback) {
        if pressed {
            self.last_pressed_key_available = true;
            self.last_pressed_key.input_type = MidiInputType::PITCH_BEND;
            self.last_pressed_key.value = -1;
        }
        if let Some(handler) = &self.pitch_bend_down {
            Self::dispatch_handler(handler, pressed, self.current_time(), callback);
        }
    }

    fn dispatch_handler(
        handler: &KeyHandler,
        pressed: bool,
        current_time: i64,
        callback: &mut dyn MidiCallback,
    ) {
        match handler.handler_type {
            KeyHandlerType::GameKey => {
                callback.key_changed_from_midi(current_time, handler.key_index, pressed);
                callback.set_analog_state(handler.key_index, false, 0.0);
            }
            KeyHandlerType::Start => {
                callback.start_changed(pressed);
            }
            KeyHandlerType::Select => {
                callback.set_select_pressed(pressed);
            }
        }
    }

    fn current_time(&self) -> i64 {
        // System.nanoTime() / 1000 - starttime
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as i64;
        nanos / 1000 - self.starttime
    }

    pub fn has_last_pressed_key(&self) -> bool {
        self.last_pressed_key_available
    }

    pub fn get_last_pressed_key(&self) -> Option<MidiInput> {
        if self.last_pressed_key_available {
            Some(self.last_pressed_key.copy_from())
        } else {
            None
        }
    }

    pub fn clear_last_pressed_key(&mut self) {
        self.last_pressed_key_available = false;
    }

    /// Process a MIDI short message (note on, note off, pitch bend)
    /// This replaces the Java MidiReceiver.send() method.
    pub fn on_short_message(
        &mut self,
        command: i32,
        data1: i32,
        data2: i32,
        callback: &mut dyn MidiCallback,
    ) {
        // ShortMessage constants
        const NOTE_OFF: i32 = 0x80;
        const NOTE_ON: i32 = 0x90;
        const PITCH_BEND: i32 = 0xE0;

        match command {
            NOTE_OFF => {
                self.note_off(data1 as usize, callback);
            }
            NOTE_ON => {
                if data2 == 0 {
                    self.note_off(data1 as usize, callback);
                } else {
                    self.note_on(data1 as usize, callback);
                }
            }
            PITCH_BEND => {
                let new_pitch = ((data1 & 0x7f) | ((data2 & 0x7f) << 7)) as i16 as i32 - 0x2000;
                if new_pitch > self.pitch_threshold {
                    if self.pitch < -self.pitch_threshold {
                        self.on_pitch_bend_down(false, callback);
                    }
                    if self.pitch <= self.pitch_threshold {
                        self.on_pitch_bend_up(true, callback);
                    }
                } else if new_pitch < -self.pitch_threshold {
                    if self.pitch > self.pitch_threshold {
                        self.on_pitch_bend_up(false, callback);
                    }
                    if self.pitch >= -self.pitch_threshold {
                        self.on_pitch_bend_down(true, callback);
                    }
                } else {
                    if self.pitch > self.pitch_threshold {
                        self.on_pitch_bend_up(false, callback);
                    }
                    if self.pitch < -self.pitch_threshold {
                        self.on_pitch_bend_down(false, callback);
                    }
                }
                self.pitch = new_pitch;
            }
            _ => {}
        }
    }
}

impl BMSPlayerInputDevice for MidiInputProcessor {
    fn device_type(&self) -> DeviceType {
        DeviceType::Midi
    }

    fn clear(&mut self) {
        self.clear_impl();
    }
}

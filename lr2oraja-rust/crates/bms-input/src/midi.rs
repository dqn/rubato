use std::sync::mpsc;

use anyhow::Result;

use bms_config::play_mode_config::{MidiConfig, MidiInputType};

use crate::device::InputEvent;

/// Special keycode for Start button from MIDI.
pub const MIDI_START_KEYCODE: i32 = -1;
/// Special keycode for Select button from MIDI.
pub const MIDI_SELECT_KEYCODE: i32 = -2;

const MAX_KEYS: usize = 128;
/// Pitch bend threshold (8192 / 32 = 256).
const PITCH_THRESHOLD: i32 = 256;

/// Raw MIDI event received from a device callback.
struct MidiEvent {
    status: u8,
    data1: u8,
    data2: u8,
}

/// Handler target for a MIDI note or control mapping.
#[derive(Debug, Clone, Copy)]
enum KeyTarget {
    /// A game lane key (index into the keys array).
    Lane(i32),
    /// The Start button.
    Start,
    /// The Select button.
    Select,
}

impl KeyTarget {
    fn keycode(self) -> i32 {
        match self {
            KeyTarget::Lane(k) => k,
            KeyTarget::Start => MIDI_START_KEYCODE,
            KeyTarget::Select => MIDI_SELECT_KEYCODE,
        }
    }
}

/// Mapping for a pitch bend direction.
#[derive(Debug, Clone, Copy)]
struct PitchBendMapping {
    target: KeyTarget,
}

/// Mapping for a MIDI control change.
#[derive(Debug, Clone, Copy)]
struct CcMapping {
    cc_number: i32,
    target: KeyTarget,
}

/// MIDI input processor.
///
/// Ported from Java `MidiInputProcessor.java`.
pub struct MidiInput {
    /// Active MIDI connections (kept alive to receive events).
    #[allow(dead_code)] // Kept alive to receive MIDI events via callbacks
    connections: Vec<midir::MidiInputConnection<()>>,
    /// Channel receiver for MIDI events from callbacks.
    event_rx: mpsc::Receiver<MidiEvent>,
    /// Channel sender cloned into callbacks.
    #[allow(dead_code)] // Cloned into MIDI callbacks; kept alive for channel
    event_tx: mpsc::Sender<MidiEvent>,
    /// MIDI note -> key target mapping (128 entries, None = unmapped).
    note_key_map: [Option<KeyTarget>; MAX_KEYS],
    /// Pitch bend up handler.
    pitch_bend_up: Option<PitchBendMapping>,
    /// Pitch bend down handler.
    pitch_bend_down: Option<PitchBendMapping>,
    /// Control change mappings.
    cc_keys: Vec<CcMapping>,
    /// Current pitch bend value (for hysteresis).
    pitch: i32,
    /// Last pressed key info for config UI.
    last_pressed_key: Option<LastPressedKey>,
}

/// Information about the last pressed MIDI key (for config UI).
#[derive(Debug, Clone, Copy)]
pub struct LastPressedKey {
    pub type_: MidiInputType,
    pub value: i32,
}

impl MidiInput {
    /// Open all available MIDI input ports and begin receiving events.
    pub fn open() -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel();
        let mut connections = Vec::new();

        let midi_in = midir::MidiInput::new("bms-input")?;
        let ports = midi_in.ports();

        for port in &ports {
            // Need a fresh MidiInput instance per connection
            let midi_in_for_port = midir::MidiInput::new("bms-input")?;
            let tx = event_tx.clone();
            let port_name = midi_in_for_port
                .port_name(port)
                .unwrap_or_else(|_| "unknown".to_string());

            match midi_in_for_port.connect(
                port,
                &port_name,
                move |_timestamp, message, _| {
                    if message.len() >= 2 {
                        let event = MidiEvent {
                            status: message[0],
                            data1: message[1],
                            data2: if message.len() >= 3 { message[2] } else { 0 },
                        };
                        let _ = tx.send(event);
                    }
                },
                (),
            ) {
                Ok(conn) => connections.push(conn),
                Err(_) => { /* Device unavailable, skip */ }
            }
        }

        Ok(Self {
            connections,
            event_rx,
            event_tx,
            note_key_map: [None; MAX_KEYS],
            pitch_bend_up: None,
            pitch_bend_down: None,
            cc_keys: Vec::new(),
            pitch: 0,
            last_pressed_key: None,
        })
    }

    /// Create a MidiInput without opening any ports (for testing).
    #[cfg(test)]
    fn new_disconnected() -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        Self {
            connections: Vec::new(),
            event_rx,
            event_tx,
            note_key_map: [None; MAX_KEYS],
            pitch_bend_up: None,
            pitch_bend_down: None,
            cc_keys: Vec::new(),
            pitch: 0,
            last_pressed_key: None,
        }
    }

    /// Poll for pending MIDI events and convert them to InputEvents.
    pub fn poll(&mut self, now_us: i64) -> Vec<InputEvent> {
        let mut events = Vec::new();

        while let Ok(midi_event) = self.event_rx.try_recv() {
            self.process_event(&midi_event, now_us, &mut events);
        }

        events
    }

    /// Process a single MIDI event and append resulting InputEvents.
    ///
    /// Faithfully ports the Java `MidiReceiver.send()` logic including
    /// pitch bend hysteresis.
    fn process_event(&mut self, event: &MidiEvent, now_us: i64, events: &mut Vec<InputEvent>) {
        let status = event.status & 0xF0;
        match status {
            // NOTE_ON: 0x90..0x9F
            0x90 => {
                let note = (event.data1 & 0x7F) as usize;
                let velocity = event.data2 & 0x7F;
                if velocity > 0 {
                    self.note_on(note, now_us, events);
                } else {
                    // Velocity 0 = note off
                    self.note_off(note, now_us, events);
                }
            }
            // NOTE_OFF: 0x80..0x8F
            0x80 => {
                let note = (event.data1 & 0x7F) as usize;
                self.note_off(note, now_us, events);
            }
            // PITCH_BEND: 0xE0..0xEF
            0xE0 => {
                let value = ((event.data2 as i32 & 0x7F) << 7) | (event.data1 as i32 & 0x7F);
                let new_pitch = value - 0x2000; // Center at 0
                self.handle_pitch_bend(new_pitch, now_us, events);
            }
            // CONTROL_CHANGE: 0xB0..0xBF
            0xB0 => {
                let cc_number = event.data1 as i32;
                let cc_value = event.data2;
                let pressed = cc_value > 63;

                // Record last pressed key
                if pressed {
                    self.last_pressed_key = Some(LastPressedKey {
                        type_: MidiInputType::ControlChange,
                        value: cc_number,
                    });
                }

                for mapping in &self.cc_keys {
                    if mapping.cc_number == cc_number {
                        events.push(InputEvent::KeyChanged {
                            keycode: mapping.target.keycode(),
                            pressed,
                            time_us: now_us,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn note_on(&mut self, note: usize, now_us: i64, events: &mut Vec<InputEvent>) {
        self.last_pressed_key = Some(LastPressedKey {
            type_: MidiInputType::Note,
            value: note as i32,
        });

        if note < MAX_KEYS
            && let Some(target) = self.note_key_map[note]
        {
            events.push(InputEvent::KeyChanged {
                keycode: target.keycode(),
                pressed: true,
                time_us: now_us,
            });
        }
    }

    fn note_off(&mut self, note: usize, now_us: i64, events: &mut Vec<InputEvent>) {
        if note < MAX_KEYS
            && let Some(target) = self.note_key_map[note]
        {
            events.push(InputEvent::KeyChanged {
                keycode: target.keycode(),
                pressed: false,
                time_us: now_us,
            });
        }
    }

    /// Handle pitch bend with hysteresis (faithful port from Java).
    ///
    /// The Java implementation uses a state-based approach:
    /// - When pitch crosses above +threshold: release down (if was below -threshold), press up
    /// - When pitch crosses below -threshold: release up (if was above +threshold), press down
    /// - When pitch returns to center: release whichever direction was active
    fn handle_pitch_bend(&mut self, new_pitch: i32, now_us: i64, events: &mut Vec<InputEvent>) {
        if new_pitch > PITCH_THRESHOLD {
            if self.pitch < -PITCH_THRESHOLD {
                self.emit_pitch_bend_down(false, now_us, events);
            }
            if self.pitch <= PITCH_THRESHOLD {
                self.emit_pitch_bend_up(true, now_us, events);
            }
        } else if new_pitch < -PITCH_THRESHOLD {
            if self.pitch > PITCH_THRESHOLD {
                self.emit_pitch_bend_up(false, now_us, events);
            }
            if self.pitch >= -PITCH_THRESHOLD {
                self.emit_pitch_bend_down(true, now_us, events);
            }
        } else {
            if self.pitch > PITCH_THRESHOLD {
                self.emit_pitch_bend_up(false, now_us, events);
            }
            if self.pitch < -PITCH_THRESHOLD {
                self.emit_pitch_bend_down(false, now_us, events);
            }
        }
        self.pitch = new_pitch;
    }

    fn emit_pitch_bend_up(&mut self, pressed: bool, now_us: i64, events: &mut Vec<InputEvent>) {
        if pressed {
            self.last_pressed_key = Some(LastPressedKey {
                type_: MidiInputType::PitchBend,
                value: 1,
            });
        }
        if let Some(mapping) = &self.pitch_bend_up {
            events.push(InputEvent::KeyChanged {
                keycode: mapping.target.keycode(),
                pressed,
                time_us: now_us,
            });
        }
    }

    fn emit_pitch_bend_down(&mut self, pressed: bool, now_us: i64, events: &mut Vec<InputEvent>) {
        if pressed {
            self.last_pressed_key = Some(LastPressedKey {
                type_: MidiInputType::PitchBend,
                value: -1,
            });
        }
        if let Some(mapping) = &self.pitch_bend_down {
            events.push(InputEvent::KeyChanged {
                keycode: mapping.target.keycode(),
                pressed,
                time_us: now_us,
            });
        }
    }

    /// Apply MIDI configuration (key mappings).
    pub fn set_config(&mut self, config: &MidiConfig) {
        self.note_key_map = [None; MAX_KEYS];
        self.pitch_bend_up = None;
        self.pitch_bend_down = None;
        self.cc_keys.clear();
        self.pitch = 0;
        self.last_pressed_key = None;

        for (i, input) in config.keys.iter().enumerate() {
            if let Some(input) = input {
                self.set_handler(input.type_, input.value, KeyTarget::Lane(i as i32));
            }
        }

        if let Some(ref start) = config.start {
            self.set_handler(start.type_, start.value, KeyTarget::Start);
        }

        if let Some(ref select) = config.select {
            self.set_handler(select.type_, select.value, KeyTarget::Select);
        }
    }

    /// Register a handler for a MIDI input mapping.
    fn set_handler(&mut self, type_: MidiInputType, value: i32, target: KeyTarget) {
        match type_ {
            MidiInputType::Note => {
                let idx = value as usize;
                if idx < MAX_KEYS && self.note_key_map[idx].is_none() {
                    self.note_key_map[idx] = Some(target);
                }
            }
            MidiInputType::PitchBend => {
                if value > 0 && self.pitch_bend_up.is_none() {
                    self.pitch_bend_up = Some(PitchBendMapping { target });
                } else if value < 0 && self.pitch_bend_down.is_none() {
                    self.pitch_bend_down = Some(PitchBendMapping { target });
                }
            }
            MidiInputType::ControlChange => {
                self.cc_keys.push(CcMapping {
                    cc_number: value,
                    target,
                });
            }
        }
    }

    /// Get the last pressed MIDI key info (for config UI).
    pub fn get_last_pressed_key(&self) -> Option<&LastPressedKey> {
        self.last_pressed_key.as_ref()
    }

    /// Clear the last pressed key state.
    pub fn clear_last_pressed_key(&mut self) {
        self.last_pressed_key = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_config::play_mode_config::MidiInput as MidiInputConfig;

    /// Helper to create a MidiInput with a test channel for injecting events.
    fn setup_test() -> (MidiInput, mpsc::Sender<MidiEvent>) {
        let midi = MidiInput::new_disconnected();
        let tx = midi.event_tx.clone();
        (midi, tx)
    }

    #[test]
    fn test_note_on_with_velocity() {
        let (mut midi, tx) = setup_test();
        // Map MIDI note 60 to lane 0
        midi.note_key_map[60] = Some(KeyTarget::Lane(0));

        // NOTE_ON channel 0, note 60, velocity 100
        tx.send(MidiEvent {
            status: 0x90,
            data1: 60,
            data2: 100,
        })
        .unwrap();

        let events = midi.poll(1000);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 0,
                pressed: true,
                time_us: 1000,
            }
        );

        // Last pressed key should be recorded
        let lpk = midi.get_last_pressed_key().unwrap();
        assert_eq!(lpk.type_, MidiInputType::Note);
        assert_eq!(lpk.value, 60);
    }

    #[test]
    fn test_note_on_velocity_zero_is_note_off() {
        let (mut midi, tx) = setup_test();
        midi.note_key_map[60] = Some(KeyTarget::Lane(0));

        // NOTE_ON with velocity 0 = note off
        tx.send(MidiEvent {
            status: 0x90,
            data1: 60,
            data2: 0,
        })
        .unwrap();

        let events = midi.poll(2000);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 0,
                pressed: false,
                time_us: 2000,
            }
        );
    }

    #[test]
    fn test_note_off() {
        let (mut midi, tx) = setup_test();
        midi.note_key_map[60] = Some(KeyTarget::Lane(0));

        // NOTE_OFF channel 0, note 60
        tx.send(MidiEvent {
            status: 0x80,
            data1: 60,
            data2: 64,
        })
        .unwrap();

        let events = midi.poll(3000);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 0,
                pressed: false,
                time_us: 3000,
            }
        );
    }

    #[test]
    fn test_pitch_bend_14bit_decoding() {
        let (mut midi, tx) = setup_test();
        midi.pitch_bend_up = Some(PitchBendMapping {
            target: KeyTarget::Lane(7),
        });

        // Pitch bend: data1 = LSB, data2 = MSB
        // value = (MSB << 7) | LSB = (0x60 << 7) | 0x00 = 12288
        // centered = 12288 - 8192 = 4096 (well above threshold 256)
        tx.send(MidiEvent {
            status: 0xE0,
            data1: 0x00,
            data2: 0x60,
        })
        .unwrap();

        let events = midi.poll(4000);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 7,
                pressed: true,
                time_us: 4000,
            }
        );
    }

    #[test]
    fn test_pitch_bend_threshold_hysteresis() {
        let (mut midi, _tx) = setup_test();
        midi.pitch_bend_up = Some(PitchBendMapping {
            target: KeyTarget::Lane(7),
        });
        midi.pitch_bend_down = Some(PitchBendMapping {
            target: KeyTarget::Lane(8),
        });

        // Start at center (0)
        assert_eq!(midi.pitch, 0);

        // Move above threshold -> up pressed
        let mut events = Vec::new();
        midi.handle_pitch_bend(300, 1000, &mut events);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 7,
                pressed: true,
                time_us: 1000,
            }
        );
        assert_eq!(midi.pitch, 300);

        // Stay above threshold -> no new events
        events.clear();
        midi.handle_pitch_bend(500, 2000, &mut events);
        assert_eq!(events.len(), 0);

        // Return to center -> up released
        events.clear();
        midi.handle_pitch_bend(0, 3000, &mut events);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 7,
                pressed: false,
                time_us: 3000,
            }
        );

        // Move below negative threshold -> down pressed
        events.clear();
        midi.handle_pitch_bend(-300, 4000, &mut events);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 8,
                pressed: true,
                time_us: 4000,
            }
        );

        // Jump from negative to positive -> down released, then up pressed
        events.clear();
        midi.handle_pitch_bend(400, 5000, &mut events);
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 8,
                pressed: false,
                time_us: 5000,
            }
        );
        assert_eq!(
            events[1],
            InputEvent::KeyChanged {
                keycode: 7,
                pressed: true,
                time_us: 5000,
            }
        );
    }

    #[test]
    fn test_pitch_bend_below_threshold_no_events() {
        let (mut midi, _tx) = setup_test();
        midi.pitch_bend_up = Some(PitchBendMapping {
            target: KeyTarget::Lane(7),
        });
        midi.pitch_bend_down = Some(PitchBendMapping {
            target: KeyTarget::Lane(8),
        });

        // Values within threshold -> no events
        let mut events = Vec::new();
        midi.handle_pitch_bend(100, 1000, &mut events);
        assert_eq!(events.len(), 0);

        midi.handle_pitch_bend(-100, 2000, &mut events);
        assert_eq!(events.len(), 0);

        midi.handle_pitch_bend(0, 3000, &mut events);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_control_change_threshold() {
        let (mut midi, tx) = setup_test();
        midi.cc_keys.push(CcMapping {
            cc_number: 64,
            target: KeyTarget::Lane(3),
        });

        // CC 64, value 127 (> 63 = pressed)
        tx.send(MidiEvent {
            status: 0xB0,
            data1: 64,
            data2: 127,
        })
        .unwrap();

        let events = midi.poll(5000);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 3,
                pressed: true,
                time_us: 5000,
            }
        );

        // CC 64, value 0 (<= 63 = released)
        tx.send(MidiEvent {
            status: 0xB0,
            data1: 64,
            data2: 0,
        })
        .unwrap();

        let events = midi.poll(6000);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 3,
                pressed: false,
                time_us: 6000,
            }
        );
    }

    #[test]
    fn test_control_change_boundary() {
        let (mut midi, tx) = setup_test();
        midi.cc_keys.push(CcMapping {
            cc_number: 1,
            target: KeyTarget::Lane(0),
        });

        // CC value 63 -> not pressed (threshold is > 63)
        tx.send(MidiEvent {
            status: 0xB0,
            data1: 1,
            data2: 63,
        })
        .unwrap();

        let events = midi.poll(1000);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 0,
                pressed: false,
                time_us: 1000,
            }
        );

        // CC value 64 -> pressed
        tx.send(MidiEvent {
            status: 0xB0,
            data1: 1,
            data2: 64,
        })
        .unwrap();

        let events = midi.poll(2000);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 0,
                pressed: true,
                time_us: 2000,
            }
        );
    }

    #[test]
    fn test_set_config_note_mapping() {
        let (mut midi, tx) = setup_test();

        let config = MidiConfig {
            keys: vec![
                Some(MidiInputConfig {
                    type_: MidiInputType::Note,
                    value: 60,
                }),
                Some(MidiInputConfig {
                    type_: MidiInputType::Note,
                    value: 62,
                }),
                None,
            ],
            start: Some(MidiInputConfig {
                type_: MidiInputType::Note,
                value: 47,
            }),
            select: Some(MidiInputConfig {
                type_: MidiInputType::Note,
                value: 48,
            }),
        };
        midi.set_config(&config);

        // Note 60 -> lane 0
        tx.send(MidiEvent {
            status: 0x90,
            data1: 60,
            data2: 100,
        })
        .unwrap();
        // Note 62 -> lane 1
        tx.send(MidiEvent {
            status: 0x90,
            data1: 62,
            data2: 100,
        })
        .unwrap();
        // Note 47 -> start
        tx.send(MidiEvent {
            status: 0x90,
            data1: 47,
            data2: 100,
        })
        .unwrap();
        // Note 48 -> select
        tx.send(MidiEvent {
            status: 0x90,
            data1: 48,
            data2: 100,
        })
        .unwrap();

        let events = midi.poll(1000);
        assert_eq!(events.len(), 4);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 0,
                pressed: true,
                time_us: 1000,
            }
        );
        assert_eq!(
            events[1],
            InputEvent::KeyChanged {
                keycode: 1,
                pressed: true,
                time_us: 1000,
            }
        );
        assert_eq!(
            events[2],
            InputEvent::KeyChanged {
                keycode: MIDI_START_KEYCODE,
                pressed: true,
                time_us: 1000,
            }
        );
        assert_eq!(
            events[3],
            InputEvent::KeyChanged {
                keycode: MIDI_SELECT_KEYCODE,
                pressed: true,
                time_us: 1000,
            }
        );
    }

    #[test]
    fn test_set_config_pitch_bend_mapping() {
        let (mut midi, _tx) = setup_test();

        let config = MidiConfig {
            keys: vec![
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(MidiInputConfig {
                    type_: MidiInputType::PitchBend,
                    value: 1, // positive direction
                }),
                Some(MidiInputConfig {
                    type_: MidiInputType::PitchBend,
                    value: -1, // negative direction
                }),
            ],
            start: None,
            select: None,
        };
        midi.set_config(&config);

        // Pitch up -> lane 7
        let mut events = Vec::new();
        midi.handle_pitch_bend(300, 1000, &mut events);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 7,
                pressed: true,
                time_us: 1000,
            }
        );

        // Return to center
        events.clear();
        midi.handle_pitch_bend(0, 2000, &mut events);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 7,
                pressed: false,
                time_us: 2000,
            }
        );

        // Pitch down -> lane 8
        events.clear();
        midi.handle_pitch_bend(-300, 3000, &mut events);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 8,
                pressed: true,
                time_us: 3000,
            }
        );
    }

    #[test]
    fn test_set_config_cc_mapping() {
        let (mut midi, tx) = setup_test();

        let config = MidiConfig {
            keys: vec![Some(MidiInputConfig {
                type_: MidiInputType::ControlChange,
                value: 64,
            })],
            start: None,
            select: None,
        };
        midi.set_config(&config);

        // CC 64, value 127 -> lane 0 pressed
        tx.send(MidiEvent {
            status: 0xB0,
            data1: 64,
            data2: 127,
        })
        .unwrap();

        let events = midi.poll(1000);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 0,
                pressed: true,
                time_us: 1000,
            }
        );
    }

    #[test]
    fn test_unmapped_note_produces_no_events() {
        let (mut midi, tx) = setup_test();
        // No mappings configured

        tx.send(MidiEvent {
            status: 0x90,
            data1: 60,
            data2: 100,
        })
        .unwrap();

        let events = midi.poll(1000);
        assert_eq!(events.len(), 0);

        // But last_pressed_key should still be recorded
        let lpk = midi.get_last_pressed_key().unwrap();
        assert_eq!(lpk.type_, MidiInputType::Note);
        assert_eq!(lpk.value, 60);
    }

    #[test]
    fn test_clear_last_pressed_key() {
        let (mut midi, tx) = setup_test();

        tx.send(MidiEvent {
            status: 0x90,
            data1: 60,
            data2: 100,
        })
        .unwrap();
        midi.poll(1000);

        assert!(midi.get_last_pressed_key().is_some());
        midi.clear_last_pressed_key();
        assert!(midi.get_last_pressed_key().is_none());
    }

    #[test]
    fn test_note_on_different_channels() {
        let (mut midi, tx) = setup_test();
        midi.note_key_map[60] = Some(KeyTarget::Lane(0));

        // Channel 0: 0x90
        tx.send(MidiEvent {
            status: 0x90,
            data1: 60,
            data2: 100,
        })
        .unwrap();
        // Channel 5: 0x95
        tx.send(MidiEvent {
            status: 0x95,
            data1: 60,
            data2: 100,
        })
        .unwrap();

        let events = midi.poll(1000);
        // Both channels should produce events (Java masks to command only)
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| matches!(
            e,
            InputEvent::KeyChanged {
                keycode: 0,
                pressed: true,
                ..
            }
        )));
    }

    #[test]
    fn test_set_config_duplicate_note_first_wins() {
        let (mut midi, tx) = setup_test();

        // Two keys mapped to the same MIDI note
        let config = MidiConfig {
            keys: vec![
                Some(MidiInputConfig {
                    type_: MidiInputType::Note,
                    value: 60,
                }),
                Some(MidiInputConfig {
                    type_: MidiInputType::Note,
                    value: 60, // Duplicate
                }),
            ],
            start: None,
            select: None,
        };
        midi.set_config(&config);

        tx.send(MidiEvent {
            status: 0x90,
            data1: 60,
            data2: 100,
        })
        .unwrap();

        let events = midi.poll(1000);
        // First mapping wins (lane 0)
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            InputEvent::KeyChanged {
                keycode: 0,
                pressed: true,
                time_us: 1000,
            }
        );
    }

    #[test]
    fn test_last_pressed_key_records_cc() {
        let (mut midi, tx) = setup_test();

        tx.send(MidiEvent {
            status: 0xB0,
            data1: 11,
            data2: 127,
        })
        .unwrap();
        midi.poll(1000);

        let lpk = midi.get_last_pressed_key().unwrap();
        assert_eq!(lpk.type_, MidiInputType::ControlChange);
        assert_eq!(lpk.value, 11);
    }

    #[test]
    fn test_last_pressed_key_records_pitch_bend_up() {
        let (mut midi, _tx) = setup_test();
        midi.pitch_bend_up = Some(PitchBendMapping {
            target: KeyTarget::Lane(7),
        });

        let mut events = Vec::new();
        midi.handle_pitch_bend(300, 1000, &mut events);

        let lpk = midi.get_last_pressed_key().unwrap();
        assert_eq!(lpk.type_, MidiInputType::PitchBend);
        assert_eq!(lpk.value, 1);
    }

    #[test]
    fn test_last_pressed_key_records_pitch_bend_down() {
        let (mut midi, _tx) = setup_test();
        midi.pitch_bend_down = Some(PitchBendMapping {
            target: KeyTarget::Lane(8),
        });

        let mut events = Vec::new();
        midi.handle_pitch_bend(-300, 1000, &mut events);

        let lpk = midi.get_last_pressed_key().unwrap();
        assert_eq!(lpk.type_, MidiInputType::PitchBend);
        assert_eq!(lpk.value, -1);
    }
}

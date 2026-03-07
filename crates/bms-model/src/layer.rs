use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    pub event: Event,
    pub sequence: Vec<Vec<Sequence>>,
}

impl Layer {
    pub fn new(event: Event, sequence: Vec<Vec<Sequence>>) -> Self {
        Layer { event, sequence }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Event {
    pub event_type: EventType,
    pub interval: i32,
}

impl Event {
    pub fn new(event_type: EventType, interval: i32) -> Self {
        Event {
            event_type,
            interval,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Always,
    Play,
    Miss,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Sequence {
    pub time: i64,
    pub id: i32,
}

impl Sequence {
    pub const END: i32 = i32::MIN;

    pub fn new_end(time: i64) -> Self {
        Sequence {
            time,
            id: Self::END,
        }
    }

    pub fn new(time: i64, id: i32) -> Self {
        Sequence { time, id }
    }

    pub fn is_end(&self) -> bool {
        self.id == Self::END
    }
}

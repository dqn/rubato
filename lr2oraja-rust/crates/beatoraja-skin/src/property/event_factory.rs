use super::event::Event;
use crate::stubs::MainState;

/// Returns an Event for the given event ID.
/// If the ID matches a built-in EventType, returns that event.
/// Otherwise, returns a generic event that delegates to `state.execute_event()`.
pub fn get_event_by_id(event_id: i32) -> Option<Box<dyn Event>> {
    for et in EVENT_TYPES.iter() {
        if et.id == event_id {
            return Some(Box::new(StubEvent { event_id: et.id }));
        }
    }

    // For unknown IDs, create a generic event that delegates to state.executeEvent
    Some(Box::new(StubEvent { event_id }))
}

/// Returns an Event for the given event name.
pub fn get_event_by_name(event_name: &str) -> Option<Box<dyn Event>> {
    for et in EVENT_TYPES.iter() {
        if et.name == event_name {
            return Some(Box::new(StubEvent { event_id: et.id }));
        }
    }
    None
}

/// Creates a zero-arg event.
pub fn create_zero_arg_event(event_id: i32) -> Box<dyn Event> {
    Box::new(StubEvent { event_id })
}

/// Creates a one-arg event.
pub fn create_one_arg_event(event_id: i32) -> Box<dyn Event> {
    Box::new(StubEvent { event_id })
}

/// Creates a two-arg event.
pub fn create_two_arg_event(event_id: i32) -> Box<dyn Event> {
    Box::new(StubEvent { event_id })
}

struct EventTypeEntry {
    id: i32,
    name: &'static str,
}

static EVENT_TYPES: &[EventTypeEntry] = &[
    EventTypeEntry {
        id: 11,
        name: "mode",
    },
    EventTypeEntry {
        id: 12,
        name: "sort",
    },
    EventTypeEntry {
        id: 312,
        name: "songbar_sort",
    },
    EventTypeEntry {
        id: 13,
        name: "keyconfig",
    },
    EventTypeEntry {
        id: 14,
        name: "skinconfig",
    },
    EventTypeEntry {
        id: 15,
        name: "play",
    },
    EventTypeEntry {
        id: 16,
        name: "autoplay",
    },
    EventTypeEntry {
        id: 315,
        name: "practice",
    },
    EventTypeEntry {
        id: 17,
        name: "open_document",
    },
    EventTypeEntry {
        id: 40,
        name: "gauge1p",
    },
    EventTypeEntry {
        id: 42,
        name: "option1p",
    },
    EventTypeEntry {
        id: 43,
        name: "option2p",
    },
    EventTypeEntry {
        id: 54,
        name: "optiondp",
    },
    EventTypeEntry {
        id: 55,
        name: "hsfix",
    },
    EventTypeEntry {
        id: 57,
        name: "hispeed1p",
    },
    EventTypeEntry {
        id: 59,
        name: "duration1p",
    },
    EventTypeEntry {
        id: 342,
        name: "hispeedautoadjust",
    },
    EventTypeEntry {
        id: 19,
        name: "replay1",
    },
    EventTypeEntry {
        id: 316,
        name: "replay2",
    },
    EventTypeEntry {
        id: 317,
        name: "replay3",
    },
    EventTypeEntry {
        id: 318,
        name: "replay4",
    },
    EventTypeEntry {
        id: 210,
        name: "open_ir",
    },
    EventTypeEntry {
        id: 211,
        name: "update_folder",
    },
    EventTypeEntry {
        id: 212,
        name: "open_with_explorer",
    },
    EventTypeEntry {
        id: 213,
        name: "open_download_site",
    },
    EventTypeEntry {
        id: 72,
        name: "bga",
    },
    EventTypeEntry {
        id: 73,
        name: "bgaexpand",
    },
    EventTypeEntry {
        id: 74,
        name: "notesdisplaytiming",
    },
    EventTypeEntry {
        id: 75,
        name: "notesdisplaytimingautoadjust",
    },
    EventTypeEntry {
        id: 77,
        name: "target",
    },
    EventTypeEntry {
        id: 78,
        name: "gaugeautoshift",
    },
    EventTypeEntry {
        id: 341,
        name: "bottomshiftablegauge",
    },
    EventTypeEntry {
        id: 79,
        name: "rival",
    },
    EventTypeEntry {
        id: 90,
        name: "favorite_chart",
    },
    EventTypeEntry {
        id: 89,
        name: "favorite_song",
    },
    EventTypeEntry {
        id: 101,
        name: "keyassign1",
    },
    EventTypeEntry {
        id: 102,
        name: "keyassign2",
    },
    EventTypeEntry {
        id: 103,
        name: "keyassign3",
    },
    EventTypeEntry {
        id: 104,
        name: "keyassign4",
    },
    EventTypeEntry {
        id: 105,
        name: "keyassign5",
    },
    EventTypeEntry {
        id: 106,
        name: "keyassign6",
    },
    EventTypeEntry {
        id: 107,
        name: "keyassign7",
    },
    EventTypeEntry {
        id: 108,
        name: "keyassign8",
    },
    EventTypeEntry {
        id: 109,
        name: "keyassign9",
    },
    EventTypeEntry {
        id: 110,
        name: "keyassign10",
    },
    EventTypeEntry {
        id: 111,
        name: "keyassign11",
    },
    EventTypeEntry {
        id: 112,
        name: "keyassign12",
    },
    EventTypeEntry {
        id: 113,
        name: "keyassign13",
    },
    EventTypeEntry {
        id: 114,
        name: "keyassign14",
    },
    EventTypeEntry {
        id: 115,
        name: "keyassign15",
    },
    EventTypeEntry {
        id: 116,
        name: "keyassign16",
    },
    EventTypeEntry {
        id: 117,
        name: "keyassign17",
    },
    EventTypeEntry {
        id: 118,
        name: "keyassign18",
    },
    EventTypeEntry {
        id: 119,
        name: "keyassign19",
    },
    EventTypeEntry {
        id: 120,
        name: "keyassign20",
    },
    EventTypeEntry {
        id: 121,
        name: "keyassign21",
    },
    EventTypeEntry {
        id: 122,
        name: "keyassign22",
    },
    EventTypeEntry {
        id: 123,
        name: "keyassign23",
    },
    EventTypeEntry {
        id: 124,
        name: "keyassign24",
    },
    EventTypeEntry {
        id: 125,
        name: "keyassign25",
    },
    EventTypeEntry {
        id: 126,
        name: "keyassign26",
    },
    EventTypeEntry {
        id: 127,
        name: "keyassign27",
    },
    EventTypeEntry {
        id: 128,
        name: "keyassign28",
    },
    EventTypeEntry {
        id: 129,
        name: "keyassign29",
    },
    EventTypeEntry {
        id: 130,
        name: "keyassign30",
    },
    EventTypeEntry {
        id: 131,
        name: "keyassign31",
    },
    EventTypeEntry {
        id: 132,
        name: "keyassign32",
    },
    EventTypeEntry {
        id: 133,
        name: "keyassign33",
    },
    EventTypeEntry {
        id: 134,
        name: "keyassign34",
    },
    EventTypeEntry {
        id: 135,
        name: "keyassign35",
    },
    EventTypeEntry {
        id: 136,
        name: "keyassign36",
    },
    EventTypeEntry {
        id: 137,
        name: "keyassign37",
    },
    EventTypeEntry {
        id: 138,
        name: "keyassign38",
    },
    EventTypeEntry {
        id: 139,
        name: "keyassign39",
    },
    EventTypeEntry {
        id: 150,
        name: "keyassign40",
    },
    EventTypeEntry {
        id: 151,
        name: "keyassign41",
    },
    EventTypeEntry {
        id: 152,
        name: "keyassign42",
    },
    EventTypeEntry {
        id: 153,
        name: "keyassign43",
    },
    EventTypeEntry {
        id: 154,
        name: "keyassign44",
    },
    EventTypeEntry {
        id: 155,
        name: "keyassign45",
    },
    EventTypeEntry {
        id: 156,
        name: "keyassign46",
    },
    EventTypeEntry {
        id: 157,
        name: "keyassign47",
    },
    EventTypeEntry {
        id: 158,
        name: "keyassign48",
    },
    EventTypeEntry {
        id: 159,
        name: "keyassign49",
    },
    EventTypeEntry {
        id: 160,
        name: "keyassign50",
    },
    EventTypeEntry {
        id: 161,
        name: "keyassign51",
    },
    EventTypeEntry {
        id: 162,
        name: "keyassign52",
    },
    EventTypeEntry {
        id: 163,
        name: "keyassign53",
    },
    EventTypeEntry {
        id: 164,
        name: "keyassign54",
    },
    EventTypeEntry {
        id: 308,
        name: "lnmode",
    },
    EventTypeEntry {
        id: 321,
        name: "autosavereplay1",
    },
    EventTypeEntry {
        id: 322,
        name: "autosavereplay2",
    },
    EventTypeEntry {
        id: 323,
        name: "autosavereplay3",
    },
    EventTypeEntry {
        id: 324,
        name: "autosavereplay4",
    },
    EventTypeEntry {
        id: 330,
        name: "lanecover",
    },
    EventTypeEntry {
        id: 331,
        name: "lift",
    },
    EventTypeEntry {
        id: 332,
        name: "hidden",
    },
    EventTypeEntry {
        id: 340,
        name: "judgealgorithm",
    },
    EventTypeEntry {
        id: 343,
        name: "guidese",
    },
    EventTypeEntry {
        id: 344,
        name: "chartreplicationmode",
    },
    EventTypeEntry {
        id: 350,
        name: "extranotedepth",
    },
    EventTypeEntry {
        id: 351,
        name: "minemode",
    },
    EventTypeEntry {
        id: 352,
        name: "scrollmode",
    },
    EventTypeEntry {
        id: 353,
        name: "longnotemode",
    },
    EventTypeEntry {
        id: 360,
        name: "seventonine_pattern",
    },
    EventTypeEntry {
        id: 361,
        name: "seventonine_type",
    },
    // OPTION_CONSTANT ID from SkinProperty
    EventTypeEntry {
        id: 1046,
        name: "constant",
    },
];

/// Stub Event that will be replaced when Phase 7+ is available.
struct StubEvent {
    event_id: i32,
}

impl Event for StubEvent {
    fn exec(&self, _state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        log::warn!(
            "not yet implemented: EventFactory event_id={} requires MainState subtypes (MusicSelector, BMSPlayer, etc.)",
            self.event_id
        );
    }

    fn get_event_id(&self) -> i32 {
        self.event_id
    }
}

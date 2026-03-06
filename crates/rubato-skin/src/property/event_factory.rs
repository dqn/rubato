use super::event::Event;
use crate::skin_property;
use crate::stubs::MainState;

use rubato_core::bms_player_mode::BMSPlayerMode;
use rubato_play::judge_algorithm::DEFAULT_ALGORITHM;
use rubato_play::target_property::TargetProperty;
use rubato_types::event_id::EventId;
use rubato_types::main_state_type::MainStateType;
use rubato_types::play_config;

// ============================================================
// Public factory API
// ============================================================

/// Returns an Event for the given event ID.
/// If the ID matches a built-in EventType, returns that event.
/// Otherwise, returns a generic event that delegates to `state.execute_event()`.
pub fn event_by_id(event_id: i32) -> Option<Box<dyn Event>> {
    let eid = EventId::new(event_id);
    for et in EVENT_TYPES.iter() {
        if et.id == eid {
            return Some((et.create_event)());
        }
    }

    // For unknown IDs, create a generic event that delegates to state.executeEvent
    Some(Box::new(DelegateEvent { event_id: eid }))
}

/// Returns an Event for the given event name.
pub fn event_by_name(event_name: &str) -> Option<Box<dyn Event>> {
    for et in EVENT_TYPES.iter() {
        if et.name == event_name {
            return Some((et.create_event)());
        }
    }
    None
}

/// Creates a zero-arg event that delegates to `state.execute_event()`.
pub fn create_zero_arg_event(event_id: i32) -> Box<dyn Event> {
    Box::new(DelegateEvent {
        event_id: EventId::new(event_id),
    })
}

/// Creates a one-arg event that delegates to `state.execute_event()`.
pub fn create_one_arg_event(event_id: i32) -> Box<dyn Event> {
    Box::new(DelegateEvent {
        event_id: EventId::new(event_id),
    })
}

/// Creates a two-arg event that delegates to `state.execute_event()`.
pub fn create_two_arg_event(event_id: i32) -> Box<dyn Event> {
    Box::new(DelegateEvent {
        event_id: EventId::new(event_id),
    })
}

// ============================================================
// Event type registry
// ============================================================

struct EventTypeEntry {
    id: EventId,
    name: &'static str,
    create_event: fn() -> Box<dyn Event>,
}

static EVENT_TYPES: &[EventTypeEntry] = &[
    // --- MusicSelector navigation / mode events ---
    EventTypeEntry {
        id: EventId(11),
        name: "mode",
        create_event: || Box::new(ModeEvent),
    },
    EventTypeEntry {
        id: EventId(12),
        name: "sort",
        create_event: || Box::new(SortEvent),
    },
    EventTypeEntry {
        id: EventId(312),
        name: "songbar_sort",
        create_event: || Box::new(SongbarSortEvent),
    },
    EventTypeEntry {
        id: EventId(13),
        name: "keyconfig",
        create_event: || Box::new(StateChangeEvent(MainStateType::Config)),
    },
    EventTypeEntry {
        id: EventId(14),
        name: "skinconfig",
        create_event: || Box::new(StateChangeEvent(MainStateType::SkinConfig)),
    },
    EventTypeEntry {
        id: EventId(15),
        name: "play",
        create_event: || Box::new(SelectSongEvent(BMSPlayerMode::PLAY)),
    },
    EventTypeEntry {
        id: EventId(16),
        name: "autoplay",
        create_event: || Box::new(SelectSongEvent(BMSPlayerMode::AUTOPLAY)),
    },
    EventTypeEntry {
        id: EventId(315),
        name: "practice",
        create_event: || Box::new(SelectSongEvent(BMSPlayerMode::PRACTICE)),
    },
    // --- OS interaction events (Desktop.open / Desktop.browse) ---
    EventTypeEntry {
        id: EventId(17),
        name: "open_document",
        create_event: || {
            Box::new(DelegateEvent {
                event_id: EventId(17),
            })
        },
    },
    // --- PlayerConfig cycler events ---
    EventTypeEntry {
        id: EventId(40),
        name: "gauge1p",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(40),
                get: |c| c.gauge,
                set: |c, v| c.gauge = v,
                count: 6,
                music_selector_only: true,
            })
        },
    },
    EventTypeEntry {
        id: EventId(42),
        name: "option1p",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(42),
                get: |c| c.random,
                set: |c, v| c.random = v,
                count: 10,
                music_selector_only: false,
            })
        },
    },
    EventTypeEntry {
        id: EventId(43),
        name: "option2p",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(43),
                get: |c| c.random2,
                set: |c, v| c.random2 = v,
                count: 10,
                music_selector_only: true,
            })
        },
    },
    EventTypeEntry {
        id: EventId(54),
        name: "optiondp",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(54),
                get: |c| c.doubleoption,
                set: |c, v| c.doubleoption = v,
                count: 4,
                music_selector_only: true,
            })
        },
    },
    // --- PlayConfig events (hispeed, duration, etc.) ---
    EventTypeEntry {
        id: EventId(55),
        name: "hsfix",
        create_event: || {
            Box::new(PlayConfigCycleEvent {
                event_id: EventId(55),
                get: |pc| pc.fixhispeed,
                set: |pc, v| pc.fixhispeed = v,
                count: 5,
            })
        },
    },
    EventTypeEntry {
        id: EventId(57),
        name: "hispeed1p",
        create_event: || Box::new(HispeedEvent),
    },
    EventTypeEntry {
        id: EventId(59),
        name: "duration1p",
        create_event: || Box::new(DurationEvent),
    },
    EventTypeEntry {
        id: EventId(342),
        name: "hispeedautoadjust",
        create_event: || Box::new(HispeedAutoAdjustEvent),
    },
    // --- Replay events ---
    EventTypeEntry {
        id: EventId(19),
        name: "replay1",
        create_event: || Box::new(ReplayEvent(0)),
    },
    EventTypeEntry {
        id: EventId(316),
        name: "replay2",
        create_event: || Box::new(ReplayEvent(1)),
    },
    EventTypeEntry {
        id: EventId(317),
        name: "replay3",
        create_event: || Box::new(ReplayEvent(2)),
    },
    EventTypeEntry {
        id: EventId(318),
        name: "replay4",
        create_event: || Box::new(ReplayEvent(3)),
    },
    // --- OS interaction events ---
    EventTypeEntry {
        id: EventId(210),
        name: "open_ir",
        create_event: || {
            Box::new(DelegateEvent {
                event_id: EventId(210),
            })
        },
    },
    EventTypeEntry {
        id: EventId(211),
        name: "update_folder",
        create_event: || {
            Box::new(DelegateEvent {
                event_id: EventId(211),
            })
        },
    },
    EventTypeEntry {
        id: EventId(212),
        name: "open_with_explorer",
        create_event: || {
            Box::new(DelegateEvent {
                event_id: EventId(212),
            })
        },
    },
    EventTypeEntry {
        id: EventId(213),
        name: "open_download_site",
        create_event: || {
            Box::new(DelegateEvent {
                event_id: EventId(213),
            })
        },
    },
    // --- Config cycler events (bga, bgaexpand) ---
    EventTypeEntry {
        id: EventId(72),
        name: "bga",
        create_event: || {
            Box::new(ConfigCycleEvent {
                event_id: EventId(72),
                get: |c| c.bga,
                set: |c, v| c.bga = v,
                count: 3,
            })
        },
    },
    EventTypeEntry {
        id: EventId(73),
        name: "bgaexpand",
        create_event: || {
            Box::new(ConfigCycleEvent {
                event_id: EventId(73),
                get: |c| c.bga_expand,
                set: |c, v| c.bga_expand = v,
                count: 3,
            })
        },
    },
    // --- Notes display timing ---
    EventTypeEntry {
        id: EventId(74),
        name: "notesdisplaytiming",
        create_event: || Box::new(NotesDisplayTimingEvent),
    },
    EventTypeEntry {
        id: EventId(75),
        name: "notesdisplaytimingautoadjust",
        create_event: || Box::new(NotesDisplayTimingAutoAdjustEvent),
    },
    // --- Target ---
    EventTypeEntry {
        id: EventId(77),
        name: "target",
        create_event: || Box::new(TargetEvent),
    },
    // --- More PlayerConfig cyclers ---
    EventTypeEntry {
        id: EventId(78),
        name: "gaugeautoshift",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(78),
                get: |c| c.gauge_auto_shift,
                set: |c, v| c.gauge_auto_shift = v,
                count: 5,
                music_selector_only: true,
            })
        },
    },
    EventTypeEntry {
        id: EventId(341),
        name: "bottomshiftablegauge",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(341),
                get: |c| c.bottom_shiftable_gauge,
                set: |c, v| c.bottom_shiftable_gauge = v,
                count: 3,
                music_selector_only: true,
            })
        },
    },
    // --- Rival ---
    EventTypeEntry {
        id: EventId(79),
        name: "rival",
        create_event: || {
            // Rival selection requires RivalDataAccessor which is not yet available
            // → delegate to state.execute_event for now
            Box::new(DelegateEvent {
                event_id: EventId(79),
            })
        },
    },
    // --- Favorite events ---
    EventTypeEntry {
        id: EventId(90),
        name: "favorite_chart",
        create_event: || {
            // Favorite chart requires SongDatabase.setSongDatas, BarManager.updateBar,
            // and ImGuiNotify which cross crate boundaries → delegate
            Box::new(DelegateEvent {
                event_id: EventId(90),
            })
        },
    },
    EventTypeEntry {
        id: EventId(89),
        name: "favorite_song",
        create_event: || {
            // Favorite song similarly requires cross-crate access → delegate
            Box::new(DelegateEvent {
                event_id: EventId(89),
            })
        },
    },
    // --- Key assign events (54 entries: keyassign1..keyassign54) ---
    // In Java, changeKeyAssign is a no-op (the body only casts to KeyConfiguration
    // and does nothing). We preserve this behavior.
    EventTypeEntry {
        id: EventId(101),
        name: "keyassign1",
        create_event: || Box::new(KeyAssignEvent(0)),
    },
    EventTypeEntry {
        id: EventId(102),
        name: "keyassign2",
        create_event: || Box::new(KeyAssignEvent(1)),
    },
    EventTypeEntry {
        id: EventId(103),
        name: "keyassign3",
        create_event: || Box::new(KeyAssignEvent(2)),
    },
    EventTypeEntry {
        id: EventId(104),
        name: "keyassign4",
        create_event: || Box::new(KeyAssignEvent(3)),
    },
    EventTypeEntry {
        id: EventId(105),
        name: "keyassign5",
        create_event: || Box::new(KeyAssignEvent(4)),
    },
    EventTypeEntry {
        id: EventId(106),
        name: "keyassign6",
        create_event: || Box::new(KeyAssignEvent(5)),
    },
    EventTypeEntry {
        id: EventId(107),
        name: "keyassign7",
        create_event: || Box::new(KeyAssignEvent(6)),
    },
    EventTypeEntry {
        id: EventId(108),
        name: "keyassign8",
        create_event: || Box::new(KeyAssignEvent(7)),
    },
    EventTypeEntry {
        id: EventId(109),
        name: "keyassign9",
        create_event: || Box::new(KeyAssignEvent(8)),
    },
    EventTypeEntry {
        id: EventId(110),
        name: "keyassign10",
        create_event: || Box::new(KeyAssignEvent(9)),
    },
    EventTypeEntry {
        id: EventId(111),
        name: "keyassign11",
        create_event: || Box::new(KeyAssignEvent(10)),
    },
    EventTypeEntry {
        id: EventId(112),
        name: "keyassign12",
        create_event: || Box::new(KeyAssignEvent(11)),
    },
    EventTypeEntry {
        id: EventId(113),
        name: "keyassign13",
        create_event: || Box::new(KeyAssignEvent(12)),
    },
    EventTypeEntry {
        id: EventId(114),
        name: "keyassign14",
        create_event: || Box::new(KeyAssignEvent(13)),
    },
    EventTypeEntry {
        id: EventId(115),
        name: "keyassign15",
        create_event: || Box::new(KeyAssignEvent(14)),
    },
    EventTypeEntry {
        id: EventId(116),
        name: "keyassign16",
        create_event: || Box::new(KeyAssignEvent(15)),
    },
    EventTypeEntry {
        id: EventId(117),
        name: "keyassign17",
        create_event: || Box::new(KeyAssignEvent(16)),
    },
    EventTypeEntry {
        id: EventId(118),
        name: "keyassign18",
        create_event: || Box::new(KeyAssignEvent(17)),
    },
    EventTypeEntry {
        id: EventId(119),
        name: "keyassign19",
        create_event: || Box::new(KeyAssignEvent(18)),
    },
    EventTypeEntry {
        id: EventId(120),
        name: "keyassign20",
        create_event: || Box::new(KeyAssignEvent(19)),
    },
    EventTypeEntry {
        id: EventId(121),
        name: "keyassign21",
        create_event: || Box::new(KeyAssignEvent(20)),
    },
    EventTypeEntry {
        id: EventId(122),
        name: "keyassign22",
        create_event: || Box::new(KeyAssignEvent(21)),
    },
    EventTypeEntry {
        id: EventId(123),
        name: "keyassign23",
        create_event: || Box::new(KeyAssignEvent(22)),
    },
    EventTypeEntry {
        id: EventId(124),
        name: "keyassign24",
        create_event: || Box::new(KeyAssignEvent(23)),
    },
    EventTypeEntry {
        id: EventId(125),
        name: "keyassign25",
        create_event: || Box::new(KeyAssignEvent(24)),
    },
    EventTypeEntry {
        id: EventId(126),
        name: "keyassign26",
        create_event: || Box::new(KeyAssignEvent(25)),
    },
    EventTypeEntry {
        id: EventId(127),
        name: "keyassign27",
        create_event: || Box::new(KeyAssignEvent(26)),
    },
    EventTypeEntry {
        id: EventId(128),
        name: "keyassign28",
        create_event: || Box::new(KeyAssignEvent(27)),
    },
    EventTypeEntry {
        id: EventId(129),
        name: "keyassign29",
        create_event: || Box::new(KeyAssignEvent(28)),
    },
    EventTypeEntry {
        id: EventId(130),
        name: "keyassign30",
        create_event: || Box::new(KeyAssignEvent(29)),
    },
    EventTypeEntry {
        id: EventId(131),
        name: "keyassign31",
        create_event: || Box::new(KeyAssignEvent(30)),
    },
    EventTypeEntry {
        id: EventId(132),
        name: "keyassign32",
        create_event: || Box::new(KeyAssignEvent(31)),
    },
    EventTypeEntry {
        id: EventId(133),
        name: "keyassign33",
        create_event: || Box::new(KeyAssignEvent(32)),
    },
    EventTypeEntry {
        id: EventId(134),
        name: "keyassign34",
        create_event: || Box::new(KeyAssignEvent(33)),
    },
    EventTypeEntry {
        id: EventId(135),
        name: "keyassign35",
        create_event: || Box::new(KeyAssignEvent(34)),
    },
    EventTypeEntry {
        id: EventId(136),
        name: "keyassign36",
        create_event: || Box::new(KeyAssignEvent(35)),
    },
    EventTypeEntry {
        id: EventId(137),
        name: "keyassign37",
        create_event: || Box::new(KeyAssignEvent(36)),
    },
    EventTypeEntry {
        id: EventId(138),
        name: "keyassign38",
        create_event: || Box::new(KeyAssignEvent(37)),
    },
    EventTypeEntry {
        id: EventId(139),
        name: "keyassign39",
        create_event: || Box::new(KeyAssignEvent(38)),
    },
    EventTypeEntry {
        id: EventId(150),
        name: "keyassign40",
        create_event: || Box::new(KeyAssignEvent(39)),
    },
    EventTypeEntry {
        id: EventId(151),
        name: "keyassign41",
        create_event: || Box::new(KeyAssignEvent(40)),
    },
    EventTypeEntry {
        id: EventId(152),
        name: "keyassign42",
        create_event: || Box::new(KeyAssignEvent(41)),
    },
    EventTypeEntry {
        id: EventId(153),
        name: "keyassign43",
        create_event: || Box::new(KeyAssignEvent(42)),
    },
    EventTypeEntry {
        id: EventId(154),
        name: "keyassign44",
        create_event: || Box::new(KeyAssignEvent(43)),
    },
    EventTypeEntry {
        id: EventId(155),
        name: "keyassign45",
        create_event: || Box::new(KeyAssignEvent(44)),
    },
    EventTypeEntry {
        id: EventId(156),
        name: "keyassign46",
        create_event: || Box::new(KeyAssignEvent(45)),
    },
    EventTypeEntry {
        id: EventId(157),
        name: "keyassign47",
        create_event: || Box::new(KeyAssignEvent(46)),
    },
    EventTypeEntry {
        id: EventId(158),
        name: "keyassign48",
        create_event: || Box::new(KeyAssignEvent(47)),
    },
    EventTypeEntry {
        id: EventId(159),
        name: "keyassign49",
        create_event: || Box::new(KeyAssignEvent(48)),
    },
    EventTypeEntry {
        id: EventId(160),
        name: "keyassign50",
        create_event: || Box::new(KeyAssignEvent(49)),
    },
    EventTypeEntry {
        id: EventId(161),
        name: "keyassign51",
        create_event: || Box::new(KeyAssignEvent(50)),
    },
    EventTypeEntry {
        id: EventId(162),
        name: "keyassign52",
        create_event: || Box::new(KeyAssignEvent(51)),
    },
    EventTypeEntry {
        id: EventId(163),
        name: "keyassign53",
        create_event: || Box::new(KeyAssignEvent(52)),
    },
    EventTypeEntry {
        id: EventId(164),
        name: "keyassign54",
        create_event: || Box::new(KeyAssignEvent(53)),
    },
    // --- LN mode (disabled in this fork) ---
    EventTypeEntry {
        id: EventId(308),
        name: "lnmode",
        create_event: || Box::new(LnModeEvent),
    },
    // --- Auto save replay ---
    EventTypeEntry {
        id: EventId(321),
        name: "autosavereplay1",
        create_event: || {
            Box::new(AutoSaveReplayEvent {
                index: 0,
                event_id: EventId(321),
            })
        },
    },
    EventTypeEntry {
        id: EventId(322),
        name: "autosavereplay2",
        create_event: || {
            Box::new(AutoSaveReplayEvent {
                index: 1,
                event_id: EventId(322),
            })
        },
    },
    EventTypeEntry {
        id: EventId(323),
        name: "autosavereplay3",
        create_event: || {
            Box::new(AutoSaveReplayEvent {
                index: 2,
                event_id: EventId(323),
            })
        },
    },
    EventTypeEntry {
        id: EventId(324),
        name: "autosavereplay4",
        create_event: || {
            Box::new(AutoSaveReplayEvent {
                index: 3,
                event_id: EventId(324),
            })
        },
    },
    // --- PlayConfig toggle events ---
    EventTypeEntry {
        id: EventId(330),
        name: "lanecover",
        create_event: || {
            Box::new(PlayConfigToggleEvent {
                event_id: EventId(330),
                get: |pc| pc.enablelanecover,
                set: |pc, v| pc.enablelanecover = v,
            })
        },
    },
    EventTypeEntry {
        id: EventId(331),
        name: "lift",
        create_event: || {
            Box::new(PlayConfigToggleEvent {
                event_id: EventId(331),
                get: |pc| pc.enablelift,
                set: |pc, v| pc.enablelift = v,
            })
        },
    },
    EventTypeEntry {
        id: EventId(332),
        name: "hidden",
        create_event: || {
            Box::new(PlayConfigToggleEvent {
                event_id: EventId(332),
                get: |pc| pc.enablehidden,
                set: |pc, v| pc.enablehidden = v,
            })
        },
    },
    // --- Judge algorithm ---
    EventTypeEntry {
        id: EventId(340),
        name: "judgealgorithm",
        create_event: || Box::new(JudgeAlgorithmEvent),
    },
    // --- Guide SE ---
    EventTypeEntry {
        id: EventId(343),
        name: "guidese",
        create_event: || Box::new(GuideSeEvent),
    },
    // --- Chart replication mode ---
    EventTypeEntry {
        id: EventId(344),
        name: "chartreplicationmode",
        create_event: || Box::new(ChartReplicationModeEvent),
    },
    // --- More PlayerConfig cyclers ---
    EventTypeEntry {
        id: EventId(350),
        name: "extranotedepth",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(350),
                get: |c| c.extranote_depth,
                set: |c, v| c.extranote_depth = v,
                count: 4,
                music_selector_only: true,
            })
        },
    },
    EventTypeEntry {
        id: EventId(351),
        name: "minemode",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(351),
                get: |c| c.mine_mode,
                set: |c, v| c.mine_mode = v,
                count: 5,
                music_selector_only: true,
            })
        },
    },
    EventTypeEntry {
        id: EventId(352),
        name: "scrollmode",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(352),
                get: |c| c.scroll_mode,
                set: |c, v| c.scroll_mode = v,
                count: 3,
                music_selector_only: true,
            })
        },
    },
    EventTypeEntry {
        id: EventId(353),
        name: "longnotemode",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(353),
                get: |c| c.longnote_mode,
                set: |c, v| c.longnote_mode = v,
                count: 6,
                music_selector_only: true,
            })
        },
    },
    EventTypeEntry {
        id: EventId(360),
        name: "seventonine_pattern",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(360),
                get: |c| c.seven_to_nine_pattern,
                set: |c, v| c.seven_to_nine_pattern = v,
                count: 7,
                music_selector_only: true,
            })
        },
    },
    EventTypeEntry {
        id: EventId(361),
        name: "seventonine_type",
        create_event: || {
            Box::new(PlayerConfigCycleEvent {
                event_id: EventId(361),
                get: |c| c.seven_to_nine_type,
                set: |c, v| c.seven_to_nine_type = v,
                count: 3,
                music_selector_only: true,
            })
        },
    },
    // OPTION_CONSTANT ID from SkinProperty (400)
    EventTypeEntry {
        id: EventId(skin_property::OPTION_CONSTANT),
        name: "constant",
        create_event: || {
            Box::new(PlayConfigToggleEvent {
                event_id: EventId(skin_property::OPTION_CONSTANT),
                get: |pc| pc.enable_constant,
                set: |pc, v| pc.enable_constant = v,
            })
        },
    },
];

// ============================================================
// Delegate Event: forwards to state.execute_event()
// Used for events that require types not available in beatoraja-skin
// (e.g., SongDatabase, BarManager, Desktop, IRConnection, etc.)
// ============================================================

struct DelegateEvent {
    event_id: EventId,
}

impl Event for DelegateEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, arg2: i32) {
        state.execute_event(self.event_id.as_i32(), arg1, arg2);
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// State change events (keyconfig, skinconfig)
// ============================================================

struct StateChangeEvent(MainStateType);

impl Event for StateChangeEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if state.is_music_selector() {
            state.change_state(self.0);
        }
    }

    fn get_event_id(&self) -> EventId {
        match self.0 {
            MainStateType::Config => EventId(13),
            MainStateType::SkinConfig => EventId(14),
            _ => EventId::UNDEFINED,
        }
    }
}

// ============================================================
// Select song events (play, autoplay, practice)
// ============================================================

struct SelectSongEvent(BMSPlayerMode);

impl Event for SelectSongEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if state.is_music_selector() {
            state.select_song(self.0.clone());
        }
    }

    fn get_event_id(&self) -> EventId {
        match &self.0 {
            m if *m == BMSPlayerMode::PLAY => EventId(15),
            m if *m == BMSPlayerMode::AUTOPLAY => EventId(16),
            m if *m == BMSPlayerMode::PRACTICE => EventId(315),
            _ => EventId::UNDEFINED,
        }
    }
}

// ============================================================
// Replay events
// ============================================================

struct ReplayEvent(i32);

impl Event for ReplayEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if state.is_music_selector()
            && let Some(mode) = BMSPlayerMode::replay_mode(self.0)
        {
            state.select_song(mode.clone());
        }
        // MusicResult/CourseResult replay saving is handled by execute_event delegation
        // because those types need cross-crate access
        if !state.is_music_selector() {
            state.execute_event(self.get_event_id().as_i32(), 0, 0);
        }
    }

    fn get_event_id(&self) -> EventId {
        match self.0 {
            0 => EventId(19),
            1 => EventId(316),
            2 => EventId(317),
            3 => EventId(318),
            _ => EventId::UNDEFINED,
        }
    }
}

// ============================================================
// Mode event: cycle through MODE filter array
// ============================================================

/// Mode filter array (same as MusicSelector.MODE in Java)
static MODE_FILTER: [Option<bms_model::mode::Mode>; 8] = [
    None,
    Some(bms_model::mode::Mode::BEAT_7K),
    Some(bms_model::mode::Mode::BEAT_14K),
    Some(bms_model::mode::Mode::POPN_9K),
    Some(bms_model::mode::Mode::BEAT_5K),
    Some(bms_model::mode::Mode::BEAT_10K),
    Some(bms_model::mode::Mode::KEYBOARD_24K),
    Some(bms_model::mode::Mode::KEYBOARD_24K_DOUBLE),
];

struct ModeEvent;

impl Event for ModeEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current_mode = config.mode.clone();
        let mut mode_idx = 0;
        for (i, m) in MODE_FILTER.iter().enumerate() {
            if *m == current_mode {
                mode_idx = i;
                break;
            }
        }
        let len = MODE_FILTER.len();
        let next_idx = if arg1 >= 0 {
            (mode_idx + 1) % len
        } else {
            (mode_idx + len - 1) % len
        };
        config.mode = MODE_FILTER[next_idx].clone();
        state.update_bar_after_change();
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(11)
    }
}

// ============================================================
// Sort event: cycle through default sorters
// ============================================================

struct SortEvent;

impl Event for SortEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let len = rubato_types::bar_sorter::BarSorter::DEFAULT_SORTER.len() as i32;
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current = config.sort;
        let next = if arg1 >= 0 {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        config.sort = next;
        config.sortid = Some(
            rubato_types::bar_sorter::BarSorter::DEFAULT_SORTER[next as usize]
                .name()
                .to_string(),
        );
        state.update_bar_after_change();
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(12)
    }
}

// ============================================================
// Songbar sort event: cycle through ALL sorters by sortid
// ============================================================

struct SongbarSortEvent;

impl Event for SongbarSortEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let all = &rubato_types::bar_sorter::BarSorter::ALL_SORTER;
        let len = all.len();
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current_sortid = config.sortid.clone().unwrap_or_default();
        let mut found_idx = None;
        for (i, s) in all.iter().enumerate() {
            if s.name() == current_sortid {
                found_idx = Some(i);
                break;
            }
        }
        if let Some(idx) = found_idx {
            let next_idx = if arg1 >= 0 {
                (idx + 1) % len
            } else {
                (idx + len - 1) % len
            };
            config.sortid = Some(all[next_idx].name().to_string());
            state.update_bar_after_change();
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(312)
    }
}

// ============================================================
// PlayerConfig cycle event (generic)
// Cycles a PlayerConfig integer field through [0..count)
// ============================================================

struct PlayerConfigCycleEvent {
    event_id: EventId,
    get: fn(&rubato_types::player_config::PlayerConfig) -> i32,
    set: fn(&mut rubato_types::player_config::PlayerConfig, i32),
    count: i32,
    music_selector_only: bool,
}

impl Event for PlayerConfigCycleEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if self.music_selector_only && !state.is_music_selector() {
            return;
        }
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current = (self.get)(config);
        let next = if arg1 >= 0 {
            (current + 1) % self.count
        } else {
            (current + self.count - 1) % self.count
        };
        (self.set)(config, next);
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// PlayConfig cycle event (generic)
// Cycles a PlayConfig integer field through [0..count)
// Only available for MusicSelector (needs getSelectedBarPlayConfig)
// ============================================================

struct PlayConfigCycleEvent {
    event_id: EventId,
    get: fn(&play_config::PlayConfig) -> i32,
    set: fn(&mut play_config::PlayConfig, i32),
    count: i32,
}

impl Event for PlayConfigCycleEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.get_selected_play_config_mut() else {
            return;
        };
        let current = (self.get)(pc);
        let next = if arg1 >= 0 {
            (current + 1) % self.count
        } else {
            (current + self.count - 1) % self.count
        };
        (self.set)(pc, next);
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// PlayConfig toggle event (generic)
// Toggles a PlayConfig boolean field
// ============================================================

struct PlayConfigToggleEvent {
    event_id: EventId,
    get: fn(&play_config::PlayConfig) -> bool,
    set: fn(&mut play_config::PlayConfig, bool),
}

impl Event for PlayConfigToggleEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.get_selected_play_config_mut() else {
            return;
        };
        let current = (self.get)(pc);
        (self.set)(pc, !current);
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// Config cycle event (for bga, bgaexpand)
// Cycles a Config integer field through [0..count)
// ============================================================

struct ConfigCycleEvent {
    event_id: EventId,
    get: fn(&rubato_types::config::Config) -> i32,
    set: fn(&mut rubato_types::config::Config, i32),
    count: i32,
}

impl Event for ConfigCycleEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(config) = state.get_config_mut() else {
            return;
        };
        let current = (self.get)(config);
        let next = if arg1 >= 0 {
            (current + 1) % self.count
        } else {
            (current + self.count - 1) % self.count
        };
        (self.set)(config, next);
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// Hispeed event
// ============================================================

struct HispeedEvent;

impl Event for HispeedEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.get_selected_play_config_mut() else {
            return;
        };
        let margin = pc.hispeedmargin;
        let delta = if arg1 >= 0 { margin } else { -margin };
        let new_hispeed =
            (pc.hispeed + delta).clamp(play_config::HISPEED_MIN, play_config::HISPEED_MAX);
        if (new_hispeed - pc.hispeed).abs() > f32::EPSILON {
            pc.hispeed = new_hispeed;
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(57)
    }
}

// ============================================================
// Duration event
// ============================================================

struct DurationEvent;

impl Event for DurationEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.get_selected_play_config_mut() else {
            return;
        };
        let inc = if arg2 > 0 { arg2 } else { 1 };
        let delta = if arg1 >= 0 { inc } else { -inc };
        let new_duration =
            (pc.duration + delta).clamp(play_config::DURATION_MIN, play_config::DURATION_MAX);
        if new_duration != pc.duration {
            pc.duration = new_duration;
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(59)
    }
}

// ============================================================
// Hispeed auto-adjust toggle
// ============================================================

struct HispeedAutoAdjustEvent;

impl Event for HispeedAutoAdjustEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.get_selected_play_config_mut() else {
            return;
        };
        pc.hispeedautoadjust = !pc.hispeedautoadjust;
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(342)
    }
}

// ============================================================
// Notes display timing event
// ============================================================

struct NotesDisplayTimingEvent;

impl Event for NotesDisplayTimingEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let max = rubato_types::player_config::JUDGETIMING_MAX;
        let min = rubato_types::player_config::JUDGETIMING_MIN;
        let inc = if arg1 >= 0 {
            if config.judgetiming < max { 1 } else { 0 }
        } else if config.judgetiming > min {
            -1
        } else {
            0
        };
        if inc != 0 {
            config.judgetiming += inc;
            if state.is_music_selector() {
                state.play_option_change_sound();
            }
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(74)
    }
}

// ============================================================
// Notes display timing auto-adjust toggle
// ============================================================

struct NotesDisplayTimingAutoAdjustEvent;

impl Event for NotesDisplayTimingAutoAdjustEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        let Some(config) = state.player_config_mut() else {
            return;
        };
        config.notes_display_timing_auto_adjust = !config.notes_display_timing_auto_adjust;
        if state.is_music_selector() {
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(75)
    }
}

// ============================================================
// Target event: cycle through target IDs
// ============================================================

struct TargetEvent;

impl Event for TargetEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let targets = {
            let targets = TargetProperty::targets();
            if targets.is_empty() {
                config.targetlist.clone()
            } else {
                targets
            }
        };
        if targets.is_empty() {
            return;
        }
        let mut index = 0;
        for (i, t) in targets.iter().enumerate() {
            if *t == config.targetid {
                index = i;
                break;
            }
        }
        let len = targets.len();
        let next = if arg1 >= 0 {
            (index + 1) % len
        } else {
            (index + len - 1) % len
        };
        config.targetid = targets[next].clone();
        state.play_option_change_sound();
        if state.is_music_selector() {
            state.update_bar_after_change();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(77)
    }
}

// ============================================================
// Key assign event (no-op, matches Java behavior)
// ============================================================

struct KeyAssignEvent(i32);

impl Event for KeyAssignEvent {
    fn exec(&self, _state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        // In Java, changeKeyAssign only checks `state instanceof KeyConfiguration`
        // and does nothing inside the body. Preserved as no-op.
    }

    fn get_event_id(&self) -> EventId {
        // keyassign1..39 = 101..139, keyassign40..54 = 150..164
        if self.0 < 39 {
            EventId(101 + self.0)
        } else {
            EventId(150 + (self.0 - 39))
        }
    }
}

// ============================================================
// LN mode event (disabled in this fork)
// ============================================================

struct LnModeEvent;

impl Event for LnModeEvent {
    fn exec(&self, _state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        // LN mode switching is disabled in this fork (endless dream).
        // Java code has the logic commented out with `return;` at the top.
    }

    fn get_event_id(&self) -> EventId {
        EventId(308)
    }
}

// ============================================================
// Auto save replay event
// ============================================================

struct AutoSaveReplayEvent {
    index: usize,
    event_id: EventId,
}

impl Event for AutoSaveReplayEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        // ReplayAutoSaveConstraint::values().len() = 11
        let length = 11;
        let Some(config) = state.player_config_mut() else {
            return;
        };
        if self.index >= config.autosavereplay.len() {
            return;
        }
        let current = config.autosavereplay[self.index];
        let next = if arg1 >= 0 {
            (current + 1) % length
        } else {
            (current + length - 1) % length
        };
        config.autosavereplay[self.index] = next;
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// Judge algorithm event
// ============================================================

struct JudgeAlgorithmEvent;

impl Event for JudgeAlgorithmEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let algorithms = DEFAULT_ALGORITHM;
        let alg_len = algorithms.len();
        let Some(pc) = state.get_selected_play_config_mut() else {
            return;
        };
        let jt = pc.judgetype.clone();
        for (i, alg) in algorithms.iter().enumerate() {
            if jt == alg.name() {
                let next = if arg1 >= 0 {
                    (i + 1) % alg_len
                } else {
                    (i + alg_len - 1) % alg_len
                };
                pc.judgetype = algorithms[next].name().to_string();
                // Need to play sound after releasing borrow on pc
                break;
            }
        }
        // Check if judgetype actually changed
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(340)
    }
}

// ============================================================
// Guide SE toggle
// ============================================================

struct GuideSeEvent;

impl Event for GuideSeEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(config) = state.player_config_mut() else {
            return;
        };
        config.is_guide_se = !config.is_guide_se;
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(343)
    }
}

// ============================================================
// Chart replication mode event
// ============================================================

struct ChartReplicationModeEvent;

impl Event for ChartReplicationModeEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        // ChartReplicationMode.values() = [NONE, RIVALCHART, RIVALOPTION]
        let values = ["NONE", "RIVALCHART", "RIVALOPTION"];
        let len = values.len();
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current_id = config.sortid.clone().unwrap_or_default();
        let mut found = false;
        for (i, name) in values.iter().enumerate() {
            if *name == current_id {
                let next = if arg1 >= 0 {
                    (i + 1) % len
                } else {
                    (i + len - 1) % len
                };
                config.sortid = Some(values[next].to_string());
                found = true;
                break;
            }
        }
        if found {
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(344)
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::{MainController, PlayerResource, SkinOffset, TextureRegion, Timer};

    /// Test implementation of MainState that provides mutable config access
    struct TestMainState {
        timer: Timer,
        main: MainController,
        resource: PlayerResource,
        player_config: rubato_types::player_config::PlayerConfig,
        config: rubato_types::config::Config,
        play_config: rubato_types::play_config::PlayConfig,
        is_selector: bool,
        option_change_played: bool,
        bar_updated: bool,
        executed_events: Vec<(i32, i32, i32)>,
        changed_state: Option<MainStateType>,
        selected_song: Option<BMSPlayerMode>,
    }

    impl TestMainState {
        fn new() -> Self {
            Self {
                timer: Timer::default(),
                main: MainController { debug: false },
                resource: PlayerResource,
                player_config: rubato_types::player_config::PlayerConfig::default(),
                config: rubato_types::config::Config::default(),
                play_config: rubato_types::play_config::PlayConfig::default(),
                is_selector: true,
                option_change_played: false,
                bar_updated: false,
                executed_events: Vec::new(),
                changed_state: None,
                selected_song: None,
            }
        }
    }

    impl MainState for TestMainState {
        fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
            &self.timer
        }
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
        fn get_main(&self) -> &MainController {
            &self.main
        }
        fn get_image(&self, _id: i32) -> Option<TextureRegion> {
            None
        }
        fn get_resource(&self) -> &PlayerResource {
            &self.resource
        }

        fn is_music_selector(&self) -> bool {
            self.is_selector
        }

        fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
            Some(&mut self.player_config)
        }

        fn get_player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
            Some(&self.player_config)
        }

        fn get_config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
            Some(&mut self.config)
        }

        fn get_config_ref(&self) -> Option<&rubato_types::config::Config> {
            Some(&self.config)
        }

        fn get_selected_play_config_mut(
            &mut self,
        ) -> Option<&mut rubato_types::play_config::PlayConfig> {
            Some(&mut self.play_config)
        }

        fn get_selected_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
            Some(&self.play_config)
        }

        fn play_option_change_sound(&mut self) {
            self.option_change_played = true;
        }

        fn update_bar_after_change(&mut self) {
            self.bar_updated = true;
        }

        fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
            self.executed_events.push((id, arg1, arg2));
        }

        fn change_state(&mut self, state_type: MainStateType) {
            self.changed_state = Some(state_type);
        }

        fn select_song(&mut self, mode: BMSPlayerMode) {
            self.selected_song = Some(mode);
        }
    }

    #[test]
    fn test_get_event_by_id_known() {
        let event = event_by_id(11).unwrap();
        assert_eq!(event.get_event_id(), EventId(11));
    }

    #[test]
    fn test_get_event_by_id_unknown() {
        let event = event_by_id(9999).unwrap();
        assert_eq!(event.get_event_id(), EventId(9999));
    }

    #[test]
    fn test_get_event_by_name() {
        let event = event_by_name("mode").unwrap();
        assert_eq!(event.get_event_id(), EventId(11));
    }

    #[test]
    fn test_get_event_by_name_unknown() {
        assert!(event_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_gauge_cycle_forward() {
        let mut state = TestMainState::new();
        state.player_config.gauge = 3;
        let event = event_by_id(40).unwrap(); // gauge1p
        event.exec(&mut state, 1, 0); // forward
        assert_eq!(state.player_config.gauge, 4);
        assert!(state.option_change_played);
    }

    #[test]
    fn test_gauge_cycle_backward() {
        let mut state = TestMainState::new();
        state.player_config.gauge = 0;
        let event = event_by_id(40).unwrap(); // gauge1p
        event.exec(&mut state, -1, 0); // backward wraps to 5
        assert_eq!(state.player_config.gauge, 5);
    }

    #[test]
    fn test_option1p_cycle() {
        let mut state = TestMainState::new();
        state.player_config.random = 9;
        let event = event_by_id(42).unwrap(); // option1p
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.random, 0); // wraps
    }

    #[test]
    fn test_option1p_cycle_not_music_selector() {
        let mut state = TestMainState::new();
        state.is_selector = false;
        state.player_config.random = 9;
        let event = event_by_id(42).unwrap(); // option1p
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.random, 0);
    }

    #[test]
    fn test_option2p_cycle() {
        let mut state = TestMainState::new();
        state.player_config.random2 = 5;
        let event = event_by_id(43).unwrap(); // option2p
        event.exec(&mut state, -1, 0);
        assert_eq!(state.player_config.random2, 4);
    }

    #[test]
    fn test_hsfix_cycle() {
        let mut state = TestMainState::new();
        state.play_config.fixhispeed = 3;
        let event = event_by_id(55).unwrap(); // hsfix
        event.exec(&mut state, 1, 0);
        assert_eq!(state.play_config.fixhispeed, 4);
    }

    #[test]
    fn test_hispeed_forward() {
        let mut state = TestMainState::new();
        state.play_config.hispeed = 1.0;
        state.play_config.hispeedmargin = 0.25;
        let event = event_by_id(57).unwrap(); // hispeed1p
        event.exec(&mut state, 1, 0);
        assert!((state.play_config.hispeed - 1.25).abs() < 0.001);
        assert!(state.option_change_played);
    }

    #[test]
    fn test_hispeed_backward() {
        let mut state = TestMainState::new();
        state.play_config.hispeed = 1.0;
        state.play_config.hispeedmargin = 0.25;
        let event = event_by_id(57).unwrap();
        event.exec(&mut state, -1, 0);
        assert!((state.play_config.hispeed - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_hispeed_clamp_max() {
        let mut state = TestMainState::new();
        state.play_config.hispeed = play_config::HISPEED_MAX;
        state.play_config.hispeedmargin = 0.25;
        let event = event_by_id(57).unwrap();
        event.exec(&mut state, 1, 0);
        assert!((state.play_config.hispeed - play_config::HISPEED_MAX).abs() < 0.001);
        // No sound since value didn't change
        assert!(!state.option_change_played);
    }

    #[test]
    fn test_duration_forward() {
        let mut state = TestMainState::new();
        state.play_config.duration = 500;
        let event = event_by_id(59).unwrap(); // duration1p
        event.exec(&mut state, 1, 0);
        assert_eq!(state.play_config.duration, 501);
    }

    #[test]
    fn test_duration_with_arg2() {
        let mut state = TestMainState::new();
        state.play_config.duration = 500;
        let event = event_by_id(59).unwrap();
        event.exec(&mut state, 1, 10); // increment by 10
        assert_eq!(state.play_config.duration, 510);
    }

    #[test]
    fn test_hispeed_auto_adjust_toggle() {
        let mut state = TestMainState::new();
        assert!(!state.play_config.hispeedautoadjust);
        let event = event_by_id(342).unwrap();
        event.exec(&mut state, 0, 0);
        assert!(state.play_config.hispeedautoadjust);
        event.exec(&mut state, 0, 0);
        assert!(!state.play_config.hispeedautoadjust);
    }

    #[test]
    fn test_lanecover_toggle() {
        let mut state = TestMainState::new();
        assert!(state.play_config.enablelanecover); // default is true
        let event = event_by_id(330).unwrap();
        event.exec(&mut state, 0, 0);
        assert!(!state.play_config.enablelanecover);
    }

    #[test]
    fn test_lift_toggle() {
        let mut state = TestMainState::new();
        assert!(!state.play_config.enablelift);
        let event = event_by_id(331).unwrap();
        event.exec(&mut state, 0, 0);
        assert!(state.play_config.enablelift);
    }

    #[test]
    fn test_hidden_toggle() {
        let mut state = TestMainState::new();
        assert!(!state.play_config.enablehidden);
        let event = event_by_id(332).unwrap();
        event.exec(&mut state, 0, 0);
        assert!(state.play_config.enablehidden);
    }

    #[test]
    fn test_constant_toggle() {
        let mut state = TestMainState::new();
        assert!(!state.play_config.enable_constant);
        let event = event_by_id(skin_property::OPTION_CONSTANT).unwrap();
        assert_eq!(
            event.get_event_id(),
            EventId(skin_property::OPTION_CONSTANT)
        );
        event.exec(&mut state, 0, 0);
        assert!(state.play_config.enable_constant);
    }

    #[test]
    fn test_bga_cycle() {
        let mut state = TestMainState::new();
        state.config.bga = 2;
        let event = event_by_id(72).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.config.bga, 0); // wraps from 2
    }

    #[test]
    fn test_bgaexpand_cycle() {
        let mut state = TestMainState::new();
        state.config.bga_expand = 0;
        let event = event_by_id(73).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.config.bga_expand, 1);
    }

    #[test]
    fn test_notes_display_timing_forward() {
        let mut state = TestMainState::new();
        state.player_config.judgetiming = 0;
        let event = event_by_id(74).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.judgetiming, 1);
    }

    #[test]
    fn test_notes_display_timing_backward() {
        let mut state = TestMainState::new();
        state.player_config.judgetiming = 0;
        let event = event_by_id(74).unwrap();
        event.exec(&mut state, -1, 0);
        assert_eq!(state.player_config.judgetiming, -1);
    }

    #[test]
    fn test_notes_display_timing_clamp_max() {
        let mut state = TestMainState::new();
        state.player_config.judgetiming = rubato_types::player_config::JUDGETIMING_MAX;
        let event = event_by_id(74).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(
            state.player_config.judgetiming,
            rubato_types::player_config::JUDGETIMING_MAX
        );
    }

    #[test]
    fn test_notes_display_timing_auto_adjust() {
        let mut state = TestMainState::new();
        assert!(!state.player_config.notes_display_timing_auto_adjust);
        let event = event_by_id(75).unwrap();
        event.exec(&mut state, 0, 0);
        assert!(state.player_config.notes_display_timing_auto_adjust);
    }

    #[test]
    fn test_guide_se_toggle() {
        let mut state = TestMainState::new();
        assert!(!state.player_config.is_guide_se);
        let event = event_by_id(343).unwrap();
        event.exec(&mut state, 0, 0);
        assert!(state.player_config.is_guide_se);
    }

    #[test]
    fn test_lnmode_noop() {
        let mut state = TestMainState::new();
        state.player_config.lnmode = 0;
        let event = event_by_id(308).unwrap();
        event.exec(&mut state, 1, 0);
        // LN mode is disabled; value should not change
        assert_eq!(state.player_config.lnmode, 0);
    }

    #[test]
    fn test_autosavereplay_cycle() {
        let mut state = TestMainState::new();
        state.player_config.autosavereplay = vec![5, 0, 0, 0];
        let event = event_by_id(321).unwrap(); // autosavereplay1 (index 0)
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.autosavereplay[0], 6);
    }

    #[test]
    fn test_autosavereplay_wrap() {
        let mut state = TestMainState::new();
        state.player_config.autosavereplay = vec![10, 0, 0, 0];
        let event = event_by_id(321).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.autosavereplay[0], 0); // wraps at 11
    }

    #[test]
    fn test_state_change_keyconfig() {
        let mut state = TestMainState::new();
        let event = event_by_id(13).unwrap(); // keyconfig
        event.exec(&mut state, 0, 0);
        assert_eq!(state.changed_state, Some(MainStateType::Config));
    }

    #[test]
    fn test_state_change_skinconfig() {
        let mut state = TestMainState::new();
        let event = event_by_id(14).unwrap(); // skinconfig
        event.exec(&mut state, 0, 0);
        assert_eq!(state.changed_state, Some(MainStateType::SkinConfig));
    }

    #[test]
    fn test_play_select_song() {
        let mut state = TestMainState::new();
        let event = event_by_id(15).unwrap(); // play
        event.exec(&mut state, 0, 0);
        assert_eq!(state.selected_song, Some(BMSPlayerMode::PLAY));
    }

    #[test]
    fn test_autoplay_select_song() {
        let mut state = TestMainState::new();
        let event = event_by_id(16).unwrap(); // autoplay
        event.exec(&mut state, 0, 0);
        assert_eq!(state.selected_song, Some(BMSPlayerMode::AUTOPLAY));
    }

    #[test]
    fn test_practice_select_song() {
        let mut state = TestMainState::new();
        let event = event_by_id(315).unwrap(); // practice
        event.exec(&mut state, 0, 0);
        assert_eq!(state.selected_song, Some(BMSPlayerMode::PRACTICE));
    }

    #[test]
    fn test_replay_select_song() {
        let mut state = TestMainState::new();
        let event = event_by_id(19).unwrap(); // replay1
        event.exec(&mut state, 0, 0);
        assert_eq!(state.selected_song, Some(BMSPlayerMode::REPLAY_1));
    }

    #[test]
    fn test_mode_cycle_forward() {
        let mut state = TestMainState::new();
        state.player_config.mode = None; // index 0
        let event = event_by_id(11).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(
            state.player_config.mode,
            Some(bms_model::mode::Mode::BEAT_7K)
        );
        assert!(state.bar_updated);
        assert!(state.option_change_played);
    }

    #[test]
    fn test_mode_cycle_backward() {
        let mut state = TestMainState::new();
        state.player_config.mode = None; // index 0
        let event = event_by_id(11).unwrap();
        event.exec(&mut state, -1, 0);
        assert_eq!(
            state.player_config.mode,
            Some(bms_model::mode::Mode::KEYBOARD_24K_DOUBLE)
        );
    }

    #[test]
    fn test_sort_cycle() {
        let mut state = TestMainState::new();
        state.player_config.sort = 0;
        let event = event_by_id(12).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.sort, 1);
        assert!(state.bar_updated);
        assert!(state.option_change_played);
    }

    #[test]
    fn test_key_assign_noop() {
        let mut state = TestMainState::new();
        let event = event_by_id(101).unwrap(); // keyassign1
        event.exec(&mut state, 0, 0);
        // Should not modify anything
        assert!(!state.option_change_played);
    }

    #[test]
    fn test_key_assign_event_ids() {
        // Verify the ID mapping: 101-139 for indices 0-38, 150-164 for indices 39-53
        let event = event_by_name("keyassign1").unwrap();
        assert_eq!(event.get_event_id(), EventId(101));
        let event = event_by_name("keyassign39").unwrap();
        assert_eq!(event.get_event_id(), EventId(139));
        let event = event_by_name("keyassign40").unwrap();
        assert_eq!(event.get_event_id(), EventId(150));
        let event = event_by_name("keyassign54").unwrap();
        assert_eq!(event.get_event_id(), EventId(164));
    }

    #[test]
    fn test_delegate_event_for_unknown_id() {
        let mut state = TestMainState::new();
        let event = event_by_id(9999).unwrap();
        event.exec(&mut state, 1, 2);
        assert_eq!(state.executed_events, vec![(9999, 1, 2)]);
    }

    #[test]
    fn test_not_music_selector_skips() {
        let mut state = TestMainState::new();
        state.is_selector = false;
        let event = event_by_id(40).unwrap(); // gauge1p
        event.exec(&mut state, 1, 0);
        // Should not modify config since not MusicSelector
        assert_eq!(state.player_config.gauge, 0);
        assert!(!state.option_change_played);
    }

    #[test]
    fn test_target_cycle_not_music_selector() {
        let mut state = TestMainState::new();
        state.is_selector = false;
        state.player_config.targetid = "RATE_MAX-".to_string();
        let event = event_by_id(77).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.targetid, "MAX");
    }

    #[test]
    fn test_notes_display_timing_works_for_any_state() {
        let mut state = TestMainState::new();
        state.is_selector = false;
        state.player_config.judgetiming = 0;
        let event = event_by_id(74).unwrap();
        event.exec(&mut state, 1, 0);
        // notesdisplaytiming works for any state, not just MusicSelector
        assert_eq!(state.player_config.judgetiming, 1);
        // But sound is only played for MusicSelector
        assert!(!state.option_change_played);
    }

    #[test]
    fn test_extranotedepth_cycle() {
        let mut state = TestMainState::new();
        state.player_config.extranote_depth = 3;
        let event = event_by_id(350).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.extranote_depth, 0); // wraps at 4
    }

    #[test]
    fn test_minemode_cycle() {
        let mut state = TestMainState::new();
        state.player_config.mine_mode = 4;
        let event = event_by_id(351).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.mine_mode, 0); // wraps at 5
    }

    #[test]
    fn test_scrollmode_cycle() {
        let mut state = TestMainState::new();
        state.player_config.scroll_mode = 2;
        let event = event_by_id(352).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.scroll_mode, 0); // wraps at 3
    }

    #[test]
    fn test_longnotemode_cycle() {
        let mut state = TestMainState::new();
        state.player_config.longnote_mode = 5;
        let event = event_by_id(353).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.longnote_mode, 0); // wraps at 6
    }

    #[test]
    fn test_seventonine_pattern_cycle() {
        let mut state = TestMainState::new();
        state.player_config.seven_to_nine_pattern = 6;
        let event = event_by_id(360).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.seven_to_nine_pattern, 0); // wraps at 7
    }

    #[test]
    fn test_seventonine_type_cycle() {
        let mut state = TestMainState::new();
        state.player_config.seven_to_nine_type = 2;
        let event = event_by_id(361).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.seven_to_nine_type, 0); // wraps at 3
    }

    #[test]
    fn test_judge_algorithm_cycle() {
        let mut state = TestMainState::new();
        state.play_config.judgetype = "Combo".to_string();
        let event = event_by_id(340).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.play_config.judgetype, "Duration");
    }

    #[test]
    fn test_judge_algorithm_cycle_backward() {
        let mut state = TestMainState::new();
        state.play_config.judgetype = "Combo".to_string();
        let event = event_by_id(340).unwrap();
        event.exec(&mut state, -1, 0);
        assert_eq!(state.play_config.judgetype, "Lowest");
    }

    #[test]
    fn test_optiondp_cycle() {
        let mut state = TestMainState::new();
        state.player_config.doubleoption = 3;
        let event = event_by_id(54).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.doubleoption, 0); // wraps at 4
    }

    #[test]
    fn test_gaugeautoshift_cycle() {
        let mut state = TestMainState::new();
        state.player_config.gauge_auto_shift = 4;
        let event = event_by_id(78).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.gauge_auto_shift, 0); // wraps at 5
    }

    #[test]
    fn test_bottomshiftablegauge_cycle() {
        let mut state = TestMainState::new();
        state.player_config.bottom_shiftable_gauge = 2;
        let event = event_by_id(341).unwrap();
        event.exec(&mut state, 1, 0);
        assert_eq!(state.player_config.bottom_shiftable_gauge, 0); // wraps at 3
    }

    #[test]
    fn test_all_event_types_have_matching_ids() {
        // Verify every EVENT_TYPES entry creates an event with the correct ID
        for et in EVENT_TYPES.iter() {
            let event = (et.create_event)();
            assert_eq!(
                event.get_event_id(),
                et.id,
                "Event '{}' has mismatched ID: expected {}, got {}",
                et.name,
                et.id,
                event.get_event_id()
            );
        }
    }

    #[test]
    fn test_create_helper_functions() {
        let e = create_zero_arg_event(42);
        assert_eq!(e.get_event_id(), EventId(42));
        let e = create_one_arg_event(43);
        assert_eq!(e.get_event_id(), EventId(43));
        let e = create_two_arg_event(44);
        assert_eq!(e.get_event_id(), EventId(44));
    }
}

use super::event::Event;
use crate::skin_property;

use rubato_core::bms_player_mode::BMSPlayerMode;
use rubato_types::event_id::EventId;
use rubato_types::main_state_type::MainStateType;

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
                get: |c| c.play_settings.gauge,
                set: |c, v| c.play_settings.gauge = v,
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
                get: |c| c.play_settings.random,
                set: |c, v| c.play_settings.random = v,
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
                get: |c| c.play_settings.random2,
                set: |c, v| c.play_settings.random2 = v,
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
                get: |c| c.play_settings.doubleoption,
                set: |c, v| c.play_settings.doubleoption = v,
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
                get: |c| c.render.bga,
                set: |c, v| c.render.bga = v,
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
                get: |c| c.render.bga_expand,
                set: |c, v| c.render.bga_expand = v,
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
                get: |c| c.play_settings.gauge_auto_shift,
                set: |c, v| c.play_settings.gauge_auto_shift = v,
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
                get: |c| c.play_settings.bottom_shiftable_gauge,
                set: |c, v| c.play_settings.bottom_shiftable_gauge = v,
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
                get: |c| c.display_settings.extranote_depth,
                set: |c, v| c.display_settings.extranote_depth = v,
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
                get: |c| c.play_settings.mine_mode,
                set: |c, v| c.play_settings.mine_mode = v,
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
                get: |c| c.display_settings.scroll_mode,
                set: |c, v| c.display_settings.scroll_mode = v,
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
                get: |c| c.note_modifier_settings.longnote_mode,
                set: |c, v| c.note_modifier_settings.longnote_mode = v,
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
                get: |c| c.note_modifier_settings.seven_to_nine_pattern,
                set: |c, v| c.note_modifier_settings.seven_to_nine_pattern = v,
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
                get: |c| c.note_modifier_settings.seven_to_nine_type,
                set: |c, v| c.note_modifier_settings.seven_to_nine_type = v,
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

mod event_impls;
use event_impls::*;

#[cfg(test)]
use crate::stubs::MainState;
#[cfg(test)]
use rubato_types::play_config;

#[cfg(test)]
mod tests;

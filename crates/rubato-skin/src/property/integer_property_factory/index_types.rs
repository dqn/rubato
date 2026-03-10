use rubato_types::value_id::ValueId;

use super::value_type_data::IndexTypeEntry;

pub(super) static INDEX_TYPES: &[IndexTypeEntry] = &[
    IndexTypeEntry {
        id: ValueId(303),
        name: "showjudgearea",
    },
    IndexTypeEntry {
        id: ValueId(305),
        name: "markprocessednote",
    },
    IndexTypeEntry {
        id: ValueId(306),
        name: "bpmguide",
    },
    IndexTypeEntry {
        id: ValueId(301),
        name: "customjudge",
    },
    IndexTypeEntry {
        id: ValueId(308),
        name: "lnmode",
    },
    IndexTypeEntry {
        id: ValueId(75),
        name: "notesdisplaytimingautoadjust",
    },
    IndexTypeEntry {
        id: ValueId(78),
        name: "gaugeautoshift",
    },
    IndexTypeEntry {
        id: ValueId(341),
        name: "bottomshiftablegauge",
    },
    IndexTypeEntry {
        id: ValueId(72),
        name: "bga",
    },
    IndexTypeEntry {
        id: ValueId(11),
        name: "mode",
    },
    IndexTypeEntry {
        id: ValueId(12),
        name: "sort",
    },
    IndexTypeEntry {
        id: ValueId(40),
        name: "gaugetype_1p",
    },
    IndexTypeEntry {
        id: ValueId(42),
        name: "option_1p",
    },
    IndexTypeEntry {
        id: ValueId(43),
        name: "option_2p",
    },
    IndexTypeEntry {
        id: ValueId(54),
        name: "option_dp",
    },
    IndexTypeEntry {
        id: ValueId(55),
        name: "hsfix",
    },
    IndexTypeEntry {
        id: ValueId(61),
        name: "option_target1_1p",
    },
    IndexTypeEntry {
        id: ValueId(62),
        name: "option_target1_2p",
    },
    IndexTypeEntry {
        id: ValueId(63),
        name: "option_target1_dp",
    },
    IndexTypeEntry {
        id: ValueId(342),
        name: "hispeedautoadjust",
    },
    IndexTypeEntry {
        id: ValueId(89),
        name: "favorite_song",
    },
    IndexTypeEntry {
        id: ValueId(90),
        name: "favorite_chart",
    },
    IndexTypeEntry {
        id: ValueId(321),
        name: "autosave_replay1",
    },
    IndexTypeEntry {
        id: ValueId(322),
        name: "autosave_replay2",
    },
    IndexTypeEntry {
        id: ValueId(323),
        name: "autosave_replay3",
    },
    IndexTypeEntry {
        id: ValueId(324),
        name: "autosave_replay4",
    },
    IndexTypeEntry {
        id: ValueId(330),
        name: "lanecover",
    },
    IndexTypeEntry {
        id: ValueId(331),
        name: "lift",
    },
    IndexTypeEntry {
        id: ValueId(332),
        name: "hidden",
    },
    IndexTypeEntry {
        id: ValueId(340),
        name: "judgealgorithm",
    },
    IndexTypeEntry {
        id: ValueId(343),
        name: "guidese",
    },
    IndexTypeEntry {
        id: ValueId(350),
        name: "extranotedepth",
    },
    IndexTypeEntry {
        id: ValueId(351),
        name: "minemode",
    },
    IndexTypeEntry {
        id: ValueId(352),
        name: "scrollmode",
    },
    IndexTypeEntry {
        id: ValueId(353),
        name: "longnotemode",
    },
    IndexTypeEntry {
        id: ValueId(360),
        name: "seventonine_pattern",
    },
    IndexTypeEntry {
        id: ValueId(361),
        name: "seventonine_type",
    },
    IndexTypeEntry {
        id: ValueId(370),
        name: "cleartype",
    },
    IndexTypeEntry {
        id: ValueId(371),
        name: "cleartype_target",
    },
    IndexTypeEntry {
        id: ValueId(390),
        name: "cleartype_ranking1",
    },
    IndexTypeEntry {
        id: ValueId(391),
        name: "cleartype_ranking2",
    },
    IndexTypeEntry {
        id: ValueId(392),
        name: "cleartype_ranking3",
    },
    IndexTypeEntry {
        id: ValueId(393),
        name: "cleartype_ranking4",
    },
    IndexTypeEntry {
        id: ValueId(394),
        name: "cleartype_ranking5",
    },
    IndexTypeEntry {
        id: ValueId(395),
        name: "cleartype_ranking6",
    },
    IndexTypeEntry {
        id: ValueId(396),
        name: "cleartype_ranking7",
    },
    IndexTypeEntry {
        id: ValueId(397),
        name: "cleartype_ranking8",
    },
    IndexTypeEntry {
        id: ValueId(398),
        name: "cleartype_ranking9",
    },
    IndexTypeEntry {
        id: ValueId(399),
        name: "cleartype_ranking10",
    },
    IndexTypeEntry {
        id: ValueId(400),
        name: "constant",
    },
    IndexTypeEntry {
        id: ValueId(450),
        name: "pattern_1p_1",
    },
    IndexTypeEntry {
        id: ValueId(451),
        name: "pattern_1p_2",
    },
    IndexTypeEntry {
        id: ValueId(452),
        name: "pattern_1p_3",
    },
    IndexTypeEntry {
        id: ValueId(453),
        name: "pattern_1p_4",
    },
    IndexTypeEntry {
        id: ValueId(454),
        name: "pattern_1p_5",
    },
    IndexTypeEntry {
        id: ValueId(455),
        name: "pattern_1p_6",
    },
    IndexTypeEntry {
        id: ValueId(456),
        name: "pattern_1p_7",
    },
    IndexTypeEntry {
        id: ValueId(457),
        name: "pattern_1p_8",
    },
    IndexTypeEntry {
        id: ValueId(458),
        name: "pattern_1p_9",
    },
    IndexTypeEntry {
        id: ValueId(459),
        name: "pattern_1p_SCR",
    },
    IndexTypeEntry {
        id: ValueId(460),
        name: "pattern_2p_1",
    },
    IndexTypeEntry {
        id: ValueId(461),
        name: "pattern_2p_2",
    },
    IndexTypeEntry {
        id: ValueId(462),
        name: "pattern_2p_3",
    },
    IndexTypeEntry {
        id: ValueId(463),
        name: "pattern_2p_4",
    },
    IndexTypeEntry {
        id: ValueId(464),
        name: "pattern_2p_5",
    },
    IndexTypeEntry {
        id: ValueId(465),
        name: "pattern_2p_6",
    },
    IndexTypeEntry {
        id: ValueId(466),
        name: "pattern_2p_7",
    },
    IndexTypeEntry {
        id: ValueId(469),
        name: "pattern_2p_SCR",
    },
    // Old spec assist options
    IndexTypeEntry {
        id: ValueId(1046),
        name: "assist_constant",
    },
    IndexTypeEntry {
        id: ValueId(1047),
        name: "assist_legacy",
    },
    IndexTypeEntry {
        id: ValueId(1048),
        name: "assist_nomine",
    },
];

use rubato_types::value_id::ValueId;

use super::value_type_data::ValueTypeEntry;

pub(super) static VALUE_TYPES: &[ValueTypeEntry] = &[
    ValueTypeEntry {
        id: ValueId(12),
        name: "notesdisplaytiming",
    },
    ValueTypeEntry {
        id: ValueId(17),
        name: "playtime_total_hour",
    },
    ValueTypeEntry {
        id: ValueId(18),
        name: "playtime_total_minute",
    },
    ValueTypeEntry {
        id: ValueId(19),
        name: "playtime_totla_saecond",
    },
    ValueTypeEntry {
        id: ValueId(20),
        name: "current_fps",
    },
    ValueTypeEntry {
        id: ValueId(21),
        name: "currenttime_year",
    },
    ValueTypeEntry {
        id: ValueId(22),
        name: "currenttime_month",
    },
    ValueTypeEntry {
        id: ValueId(23),
        name: "currenttime_day",
    },
    ValueTypeEntry {
        id: ValueId(24),
        name: "currenttime_hour",
    },
    ValueTypeEntry {
        id: ValueId(25),
        name: "currenttime_minute",
    },
    ValueTypeEntry {
        id: ValueId(26),
        name: "currenttime_saecond",
    },
    ValueTypeEntry {
        id: ValueId(27),
        name: "boottime_hour",
    },
    ValueTypeEntry {
        id: ValueId(28),
        name: "boottime_minute",
    },
    ValueTypeEntry {
        id: ValueId(29),
        name: "boottime_second",
    },
    ValueTypeEntry {
        id: ValueId(30),
        name: "player_playcount",
    },
    ValueTypeEntry {
        id: ValueId(31),
        name: "player_clearcount",
    },
    ValueTypeEntry {
        id: ValueId(32),
        name: "player_failcount",
    },
    ValueTypeEntry {
        id: ValueId(33),
        name: "player_perfect",
    },
    ValueTypeEntry {
        id: ValueId(34),
        name: "player_great",
    },
    ValueTypeEntry {
        id: ValueId(35),
        name: "player_good",
    },
    ValueTypeEntry {
        id: ValueId(36),
        name: "player_bad",
    },
    ValueTypeEntry {
        id: ValueId(37),
        name: "player_poor",
    },
    ValueTypeEntry {
        id: ValueId(333),
        name: "player_notes",
    },
    ValueTypeEntry {
        id: ValueId(57),
        name: "volume_system",
    },
    ValueTypeEntry {
        id: ValueId(58),
        name: "volume_key",
    },
    ValueTypeEntry {
        id: ValueId(59),
        name: "volume_background",
    },
    ValueTypeEntry {
        id: ValueId(77),
        name: "playcount",
    },
    ValueTypeEntry {
        id: ValueId(78),
        name: "clearcount",
    },
    ValueTypeEntry {
        id: ValueId(79),
        name: "failcount",
    },
    ValueTypeEntry {
        id: ValueId(90),
        name: "maxbpm",
    },
    ValueTypeEntry {
        id: ValueId(91),
        name: "minbpm",
    },
    ValueTypeEntry {
        id: ValueId(92),
        name: "mainbpm",
    },
    ValueTypeEntry {
        id: ValueId(160),
        name: "nowbpm",
    },
    ValueTypeEntry {
        id: ValueId(161),
        name: "playtime_minute",
    },
    ValueTypeEntry {
        id: ValueId(162),
        name: "playtime_second",
    },
    ValueTypeEntry {
        id: ValueId(163),
        name: "timeleft_minute",
    },
    ValueTypeEntry {
        id: ValueId(164),
        name: "timeleft_second",
    },
    ValueTypeEntry {
        id: ValueId(165),
        name: "loading_progress",
    },
    ValueTypeEntry {
        id: ValueId(179),
        name: "ir_rank",
    },
    ValueTypeEntry {
        id: ValueId(182),
        name: "ir_prevrank",
    },
    ValueTypeEntry {
        id: ValueId(202),
        name: "ir_player_noplay",
    },
    ValueTypeEntry {
        id: ValueId(210),
        name: "ir_player_failed",
    },
    ValueTypeEntry {
        id: ValueId(204),
        name: "ir_player_assist",
    },
    ValueTypeEntry {
        id: ValueId(206),
        name: "ir_player_lightassist",
    },
    ValueTypeEntry {
        id: ValueId(212),
        name: "ir_player_easy",
    },
    ValueTypeEntry {
        id: ValueId(214),
        name: "ir_player_normal",
    },
    ValueTypeEntry {
        id: ValueId(216),
        name: "ir_player_hard",
    },
    ValueTypeEntry {
        id: ValueId(208),
        name: "ir_player_exhard",
    },
    ValueTypeEntry {
        id: ValueId(218),
        name: "ir_player_fullcombo",
    },
    ValueTypeEntry {
        id: ValueId(222),
        name: "ir_player_perfect",
    },
    ValueTypeEntry {
        id: ValueId(224),
        name: "ir_player_max",
    },
    ValueTypeEntry {
        id: ValueId(220),
        name: "ir_update_waiting",
    },
    ValueTypeEntry {
        id: ValueId(226),
        name: "ir_totalclear",
    },
    ValueTypeEntry {
        id: ValueId(227),
        name: "ir_totalclearrate",
    },
    ValueTypeEntry {
        id: ValueId(241),
        name: "ir_totalclearrate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(228),
        name: "ir_totalfullcombo",
    },
    ValueTypeEntry {
        id: ValueId(229),
        name: "ir_totalfullcomborate",
    },
    ValueTypeEntry {
        id: ValueId(242),
        name: "ir_totalfullcomborate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(203),
        name: "ir_player_noplay_rate",
    },
    ValueTypeEntry {
        id: ValueId(230),
        name: "ir_player_noplay_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(211),
        name: "ir_player_failed_rate",
    },
    ValueTypeEntry {
        id: ValueId(234),
        name: "ir_player_failed_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(205),
        name: "ir_player_assist_rate",
    },
    ValueTypeEntry {
        id: ValueId(231),
        name: "ir_player_assist_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(207),
        name: "ir_player_lightassist_rate",
    },
    ValueTypeEntry {
        id: ValueId(232),
        name: "ir_player_lightassist_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(213),
        name: "ir_player_easy_rate",
    },
    ValueTypeEntry {
        id: ValueId(235),
        name: "ir_player_easy_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(215),
        name: "ir_player_normal_rate",
    },
    ValueTypeEntry {
        id: ValueId(236),
        name: "ir_player_normal_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(217),
        name: "ir_player_hard_rate",
    },
    ValueTypeEntry {
        id: ValueId(237),
        name: "ir_player_hard_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(209),
        name: "ir_player_exhard_rate",
    },
    ValueTypeEntry {
        id: ValueId(233),
        name: "ir_player_exhard_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(219),
        name: "ir_player_fullcombo_rate",
    },
    ValueTypeEntry {
        id: ValueId(238),
        name: "ir_player_fullcombo_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(223),
        name: "ir_player_perfect_rate",
    },
    ValueTypeEntry {
        id: ValueId(239),
        name: "ir_player_perfect_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(225),
        name: "ir_player_max_rate",
    },
    ValueTypeEntry {
        id: ValueId(240),
        name: "ir_player_max_rate_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(312),
        name: "duration",
    },
    ValueTypeEntry {
        id: ValueId(313),
        name: "duration_green",
    },
    ValueTypeEntry {
        id: ValueId(320),
        name: "folder_noplay",
    },
    ValueTypeEntry {
        id: ValueId(321),
        name: "folder_failed",
    },
    ValueTypeEntry {
        id: ValueId(322),
        name: "folder_assist",
    },
    ValueTypeEntry {
        id: ValueId(323),
        name: "folder_lightassist",
    },
    ValueTypeEntry {
        id: ValueId(324),
        name: "folder_easy",
    },
    ValueTypeEntry {
        id: ValueId(325),
        name: "folder_normal",
    },
    ValueTypeEntry {
        id: ValueId(326),
        name: "folder_hard",
    },
    ValueTypeEntry {
        id: ValueId(327),
        name: "folder_exhard",
    },
    ValueTypeEntry {
        id: ValueId(328),
        name: "folder_fullcombo",
    },
    ValueTypeEntry {
        id: ValueId(329),
        name: "folder_prefect",
    },
    ValueTypeEntry {
        id: ValueId(330),
        name: "folder_max",
    },
    ValueTypeEntry {
        id: ValueId(350),
        name: "chart_totalnote_n",
    },
    ValueTypeEntry {
        id: ValueId(351),
        name: "chart_totalnote_ln",
    },
    ValueTypeEntry {
        id: ValueId(352),
        name: "chart_totalnote_s",
    },
    ValueTypeEntry {
        id: ValueId(353),
        name: "chart_totalnote_ls",
    },
    ValueTypeEntry {
        id: ValueId(364),
        name: "chart_averagedensity",
    },
    ValueTypeEntry {
        id: ValueId(365),
        name: "chart_averagedensity_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(362),
        name: "chart_enddensity",
    },
    ValueTypeEntry {
        id: ValueId(363),
        name: "chart_enddensity_peak",
    },
    ValueTypeEntry {
        id: ValueId(360),
        name: "chart_peakdensity",
    },
    ValueTypeEntry {
        id: ValueId(361),
        name: "chart_peakdensity_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(368),
        name: "chart_totalgauge",
    },
    ValueTypeEntry {
        id: ValueId(372),
        name: "duration_average",
    },
    ValueTypeEntry {
        id: ValueId(373),
        name: "duration_average_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(374),
        name: "timing_average",
    },
    ValueTypeEntry {
        id: ValueId(375),
        name: "timing_average_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(376),
        name: "timing_stddev",
    },
    ValueTypeEntry {
        id: ValueId(377),
        name: "timing_atddev_afterdot",
    },
    ValueTypeEntry {
        id: ValueId(380),
        name: "ranking_exscore1",
    },
    ValueTypeEntry {
        id: ValueId(381),
        name: "ranking_exscore2",
    },
    ValueTypeEntry {
        id: ValueId(382),
        name: "ranking_exscore3",
    },
    ValueTypeEntry {
        id: ValueId(383),
        name: "ranking_exscore4",
    },
    ValueTypeEntry {
        id: ValueId(384),
        name: "ranking_exscore5",
    },
    ValueTypeEntry {
        id: ValueId(385),
        name: "ranking_exscore6",
    },
    ValueTypeEntry {
        id: ValueId(386),
        name: "ranking_exscore7",
    },
    ValueTypeEntry {
        id: ValueId(387),
        name: "ranking_exscore8",
    },
    ValueTypeEntry {
        id: ValueId(388),
        name: "ranking_exscore9",
    },
    ValueTypeEntry {
        id: ValueId(389),
        name: "ranking_exscore10",
    },
    ValueTypeEntry {
        id: ValueId(390),
        name: "ranking_index1",
    },
    ValueTypeEntry {
        id: ValueId(391),
        name: "ranking_index2",
    },
    ValueTypeEntry {
        id: ValueId(392),
        name: "ranking_index3",
    },
    ValueTypeEntry {
        id: ValueId(393),
        name: "ranking_index4",
    },
    ValueTypeEntry {
        id: ValueId(394),
        name: "ranking_index5",
    },
    ValueTypeEntry {
        id: ValueId(395),
        name: "ranking_index6",
    },
    ValueTypeEntry {
        id: ValueId(396),
        name: "ranking_index7",
    },
    ValueTypeEntry {
        id: ValueId(397),
        name: "ranking_index8",
    },
    ValueTypeEntry {
        id: ValueId(398),
        name: "ranking_index9",
    },
    ValueTypeEntry {
        id: ValueId(399),
        name: "ranking_index10",
    },
    ValueTypeEntry {
        id: ValueId(400),
        name: "judgerank",
    },
    ValueTypeEntry {
        id: ValueId(525),
        name: "judge_duration1",
    },
    ValueTypeEntry {
        id: ValueId(526),
        name: "judge_duration2",
    },
    ValueTypeEntry {
        id: ValueId(527),
        name: "judge_duration3",
    },
    ValueTypeEntry {
        id: ValueId(1163),
        name: "chartlength_minute",
    },
    ValueTypeEntry {
        id: ValueId(1164),
        name: "chartlength_second",
    },
];

//! Timer ID constants for skin property system.

use rubato_types::timer_id::TimerId;

pub const TIMER_STARTINPUT: TimerId = TimerId(1);

pub const TIMER_FADEOUT: TimerId = TimerId(2);

pub const TIMER_FAILED: TimerId = TimerId(3);

pub const TIMER_SONGBAR_MOVE: TimerId = TimerId(10);
pub const TIMER_SONGBAR_CHANGE: TimerId = TimerId(11);

pub const TIMER_SONGBAR_MOVE_UP: TimerId = TimerId(12);
pub const TIMER_SONGBAR_MOVE_DOWN: TimerId = TimerId(13);

pub const TIMER_SONGBAR_STOP: TimerId = TimerId(14);

pub const TIMER_README_BEGIN: TimerId = TimerId(15);
pub const TIMER_README_END: TimerId = TimerId(16);

pub const TIMER_PANEL1_ON: TimerId = TimerId(21);

pub const TIMER_PANEL2_ON: TimerId = TimerId(22);

pub const TIMER_PANEL3_ON: TimerId = TimerId(23);

pub const TIMER_PANEL4_ON: TimerId = TimerId(24);

pub const TIMER_PANEL5_ON: TimerId = TimerId(25);

pub const TIMER_PANEL6_ON: TimerId = TimerId(26);

pub const TIMER_PANEL1_OFF: TimerId = TimerId(31);

pub const TIMER_PANEL2_OFF: TimerId = TimerId(32);

pub const TIMER_PANEL3_OFF: TimerId = TimerId(33);

pub const TIMER_PANEL4_OFF: TimerId = TimerId(34);

pub const TIMER_PANEL5_OFF: TimerId = TimerId(35);

pub const TIMER_PANEL6_OFF: TimerId = TimerId(36);

pub const TIMER_READY: TimerId = TimerId(40);

pub const TIMER_PLAY: TimerId = TimerId(41);

pub const TIMER_GAUGE_INCLEASE_1P: TimerId = TimerId(42);
pub const TIMER_GAUGE_INCLEASE_2P: TimerId = TimerId(43);

pub const TIMER_GAUGE_MAX_1P: TimerId = TimerId(44);
pub const TIMER_GAUGE_MAX_2P: TimerId = TimerId(45);

pub const TIMER_JUDGE_1P: TimerId = TimerId(46);
pub const TIMER_JUDGE_2P: TimerId = TimerId(47);
pub const TIMER_JUDGE_3P: TimerId = TimerId(247);

pub const TIMER_COMBO_1P: TimerId = TimerId(446);
pub const TIMER_COMBO_2P: TimerId = TimerId(447);
pub const TIMER_COMBO_3P: TimerId = TimerId(448);

pub const TIMER_FULLCOMBO_1P: TimerId = TimerId(48);

pub const TIMER_SCORE_A: TimerId = TimerId(348);
pub const TIMER_SCORE_AA: TimerId = TimerId(349);
pub const TIMER_SCORE_AAA: TimerId = TimerId(350);
pub const TIMER_SCORE_BEST: TimerId = TimerId(351);
pub const TIMER_SCORE_TARGET: TimerId = TimerId(352);

pub const TIMER_FULLCOMBO_2P: TimerId = TimerId(49);

pub const TIMER_BOMB_1P_SCRATCH: TimerId = TimerId(50);
pub const TIMER_BOMB_1P_KEY1: TimerId = TimerId(51);

pub const TIMER_HCN_ACTIVE_1P_SCRATCH: TimerId = TimerId(250);
pub const TIMER_HCN_ACTIVE_1P_KEY1: TimerId = TimerId(251);

pub const TIMER_BOMB_1P_KEY2: TimerId = TimerId(52);
pub const TIMER_BOMB_1P_KEY3: TimerId = TimerId(53);
pub const TIMER_BOMB_1P_KEY4: TimerId = TimerId(54);
pub const TIMER_BOMB_1P_KEY5: TimerId = TimerId(55);
pub const TIMER_BOMB_1P_KEY6: TimerId = TimerId(56);
pub const TIMER_BOMB_1P_KEY7: TimerId = TimerId(57);
pub const TIMER_BOMB_1P_KEY8: TimerId = TimerId(58);
pub const TIMER_BOMB_1P_KEY9: TimerId = TimerId(59);

pub const TIMER_BOMB_2P_SCRATCH: TimerId = TimerId(60);
pub const TIMER_BOMB_2P_KEY1: TimerId = TimerId(61);
pub const TIMER_BOMB_2P_KEY2: TimerId = TimerId(62);
pub const TIMER_BOMB_2P_KEY3: TimerId = TimerId(63);
pub const TIMER_BOMB_2P_KEY4: TimerId = TimerId(64);
pub const TIMER_BOMB_2P_KEY5: TimerId = TimerId(65);
pub const TIMER_BOMB_2P_KEY6: TimerId = TimerId(66);
pub const TIMER_BOMB_2P_KEY7: TimerId = TimerId(67);
pub const TIMER_BOMB_2P_KEY8: TimerId = TimerId(68);
pub const TIMER_BOMB_2P_KEY9: TimerId = TimerId(69);

pub const TIMER_HOLD_1P_SCRATCH: TimerId = TimerId(70);
pub const TIMER_HOLD_1P_KEY1: TimerId = TimerId(71);

pub const TIMER_HCN_DAMAGE_1P_SCRATCH: TimerId = TimerId(270);
pub const TIMER_HCN_DAMAGE_1P_KEY1: TimerId = TimerId(271);

pub const TIMER_HOLD_2P_SCRATCH: TimerId = TimerId(80);
pub const TIMER_HOLD_2P_KEY1: TimerId = TimerId(81);

pub const TIMER_KEYON_1P_SCRATCH: TimerId = TimerId(100);
pub const TIMER_KEYON_1P_KEY1: TimerId = TimerId(101);
pub const TIMER_KEYON_1P_KEY2: TimerId = TimerId(102);
pub const TIMER_KEYON_1P_KEY3: TimerId = TimerId(103);
pub const TIMER_KEYON_1P_KEY4: TimerId = TimerId(104);
pub const TIMER_KEYON_1P_KEY5: TimerId = TimerId(105);
pub const TIMER_KEYON_1P_KEY6: TimerId = TimerId(106);
pub const TIMER_KEYON_1P_KEY7: TimerId = TimerId(107);
pub const TIMER_KEYON_1P_KEY8: TimerId = TimerId(108);
pub const TIMER_KEYON_1P_KEY9: TimerId = TimerId(109);

pub const TIMER_KEYON_2P_SCRATCH: TimerId = TimerId(110);
pub const TIMER_KEYON_2P_KEY1: TimerId = TimerId(111);
pub const TIMER_KEYON_2P_KEY2: TimerId = TimerId(112);
pub const TIMER_KEYON_2P_KEY3: TimerId = TimerId(113);
pub const TIMER_KEYON_2P_KEY4: TimerId = TimerId(114);
pub const TIMER_KEYON_2P_KEY5: TimerId = TimerId(115);
pub const TIMER_KEYON_2P_KEY6: TimerId = TimerId(116);
pub const TIMER_KEYON_2P_KEY7: TimerId = TimerId(117);
pub const TIMER_KEYON_2P_KEY8: TimerId = TimerId(118);
pub const TIMER_KEYON_2P_KEY9: TimerId = TimerId(119);

pub const TIMER_KEYOFF_1P_SCRATCH: TimerId = TimerId(120);
pub const TIMER_KEYOFF_1P_KEY1: TimerId = TimerId(121);
pub const TIMER_KEYOFF_1P_KEY2: TimerId = TimerId(122);
pub const TIMER_KEYOFF_1P_KEY3: TimerId = TimerId(123);
pub const TIMER_KEYOFF_1P_KEY4: TimerId = TimerId(124);
pub const TIMER_KEYOFF_1P_KEY5: TimerId = TimerId(125);
pub const TIMER_KEYOFF_1P_KEY6: TimerId = TimerId(126);
pub const TIMER_KEYOFF_1P_KEY7: TimerId = TimerId(127);
pub const TIMER_KEYOFF_1P_KEY8: TimerId = TimerId(128);
pub const TIMER_KEYOFF_1P_KEY9: TimerId = TimerId(129);

pub const TIMER_KEYOFF_2P_SCRATCH: TimerId = TimerId(130);
pub const TIMER_KEYOFF_2P_KEY1: TimerId = TimerId(131);
pub const TIMER_KEYOFF_2P_KEY2: TimerId = TimerId(132);
pub const TIMER_KEYOFF_2P_KEY3: TimerId = TimerId(133);
pub const TIMER_KEYOFF_2P_KEY4: TimerId = TimerId(134);
pub const TIMER_KEYOFF_2P_KEY5: TimerId = TimerId(135);
pub const TIMER_KEYOFF_2P_KEY6: TimerId = TimerId(136);
pub const TIMER_KEYOFF_2P_KEY7: TimerId = TimerId(137);
pub const TIMER_KEYOFF_2P_KEY8: TimerId = TimerId(138);
pub const TIMER_KEYOFF_2P_KEY9: TimerId = TimerId(139);

pub const TIMER_RHYTHM: TimerId = TimerId(140);

pub const TIMER_ENDOFNOTE_1P: TimerId = TimerId(143);
pub const TIMER_ENDOFNOTE_2P: TimerId = TimerId(144);

pub const TIMER_RESULTGRAPH_BEGIN: TimerId = TimerId(150);
pub const TIMER_RESULTGRAPH_END: TimerId = TimerId(151);

pub const TIMER_RESULT_UPDATESCORE: TimerId = TimerId(152);

pub const TIMER_IR_CONNECT_BEGIN: TimerId = TimerId(172);
pub const TIMER_IR_CONNECT_SUCCESS: TimerId = TimerId(173);
pub const TIMER_IR_CONNECT_FAIL: TimerId = TimerId(174);

pub const TIMER_PM_CHARA_1P_NEUTRAL: TimerId = TimerId(900);
pub const TIMER_PM_CHARA_1P_FEVER: TimerId = TimerId(901);
pub const TIMER_PM_CHARA_1P_GREAT: TimerId = TimerId(902);
pub const TIMER_PM_CHARA_1P_GOOD: TimerId = TimerId(903);
pub const TIMER_PM_CHARA_1P_BAD: TimerId = TimerId(904);
pub const TIMER_PM_CHARA_2P_NEUTRAL: TimerId = TimerId(905);
pub const TIMER_PM_CHARA_2P_GREAT: TimerId = TimerId(906);
pub const TIMER_PM_CHARA_2P_BAD: TimerId = TimerId(907);

pub const TIMER_MUSIC_END: TimerId = TimerId(908);

pub const TIMER_PM_CHARA_DANCE: TimerId = TimerId(909);

pub const TIMER_BOMB_1P_KEY10: TimerId = TimerId(1010);
pub const TIMER_BOMB_1P_KEY99: TimerId = TimerId(1099);

pub const TIMER_BOMB_2P_KEY10: TimerId = TimerId(1110);
pub const TIMER_BOMB_2P_KEY99: TimerId = TimerId(1199);

pub const TIMER_HOLD_1P_KEY10: TimerId = TimerId(1210);
pub const TIMER_HOLD_1P_KEY99: TimerId = TimerId(1299);

pub const TIMER_HOLD_2P_KEY10: TimerId = TimerId(1310);
pub const TIMER_HOLD_2P_KEY99: TimerId = TimerId(1399);

pub const TIMER_KEYON_1P_KEY10: TimerId = TimerId(1410);
pub const TIMER_KEYON_1P_KEY99: TimerId = TimerId(1499);

pub const TIMER_KEYON_2P_KEY10: TimerId = TimerId(1510);
pub const TIMER_KEYON_2P_KEY99: TimerId = TimerId(1599);

pub const TIMER_KEYOFF_1P_KEY10: TimerId = TimerId(1610);
pub const TIMER_KEYOFF_1P_KEY99: TimerId = TimerId(1699);

pub const TIMER_KEYOFF_2P_KEY10: TimerId = TimerId(1710);
pub const TIMER_KEYOFF_2P_KEY99: TimerId = TimerId(1799);

pub const TIMER_HCN_ACTIVE_1P_KEY10: TimerId = TimerId(1810);
pub const TIMER_HCN_ACTIVE_2P_KEY10: TimerId = TimerId(1910);

pub const TIMER_HCN_DAMAGE_1P_KEY10: TimerId = TimerId(2010);
pub const TIMER_HCN_DAMAGE_2P_KEY10: TimerId = TimerId(2110);

pub const TIMER_MAX: TimerId = TimerId(2999);

pub const TIMER_CUSTOM_BEGIN: TimerId = TimerId(10000);
pub const TIMER_CUSTOM_END: TimerId = TimerId(19999);

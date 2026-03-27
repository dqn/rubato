//! Value/Float/Rate constants for skin property system.

use rubato_types::value_id::ValueId;

pub const RATE_MUSICSELECT_POSITION: i32 = 1;

pub const RATE_LANECOVER: i32 = 4;

pub const RATE_LANECOVER2: i32 = 5;

pub const RATE_MUSIC_PROGRESS: i32 = 6;

pub const RATE_SKINSELECT_POSITION: i32 = 7;

pub const RATE_RANKING_POSITION: i32 = 8;

pub const RATE_MASTERVOLUME: i32 = 17;

pub const RATE_KEYVOLUME: i32 = 18;

pub const RATE_BGMVOLUME: i32 = 19;

pub const RATE_MUSIC_PROGRESS_BAR: i32 = 101;

pub const RATE_LOAD_PROGRESS: i32 = 102;

pub const RATE_LEVEL: i32 = 103;

pub const RATE_LEVEL_BEGINNER: i32 = 105;
pub const RATE_LEVEL_NORMAL: i32 = 106;
pub const RATE_LEVEL_HYPER: i32 = 107;
pub const RATE_LEVEL_ANOTHER: i32 = 108;
pub const RATE_LEVEL_INSANE: i32 = 109;

pub const RATE_SCORE: i32 = 110;

pub const RATE_SCORE_FINAL: i32 = 111;

pub const RATE_BESTSCORE_NOW: i32 = 112;

pub const RATE_BESTSCORE: i32 = 113;

pub const RATE_TARGETSCORE_NOW: i32 = 114;

pub const RATE_TARGETSCORE: i32 = 115;

pub const RATE_PGREAT: i32 = 140;

pub const RATE_GREAT: i32 = 141;

pub const RATE_GOOD: i32 = 142;

pub const RATE_BAD: i32 = 143;

pub const RATE_POOR: i32 = 144;

pub const RATE_MAXCOMBO: i32 = 145;

pub const RATE_EXSCORE: i32 = 147;

pub const FLOAT_SCORE_RATE: i32 = 1102;

pub const FLOAT_TOTAL_RATE: i32 = 1115;

pub const FLOAT_SCORE_RATE2: i32 = 155;

pub const FLOAT_DURATION_AVERAGE: i32 = 372;

pub const FLOAT_TIMING_AVERAGE: i32 = 374;

pub const FLOAT_TIMIGN_STDDEV: i32 = 376;

pub const FLOAT_PERFECT_RATE: i32 = 85;

pub const FLOAT_GREAT_RATE: i32 = 86;

pub const FLOAT_GOOD_RATE: i32 = 87;

pub const FLOAT_BAD_RATE: i32 = 88;

pub const FLOAT_POOR_RATE: i32 = 89;

pub const FLOAT_RIVAL_PERFECT_RATE: i32 = 285;

pub const FLOAT_RIVAL_GREAT_RATE: i32 = 286;

pub const FLOAT_RIVAL_GOOD_RATE: i32 = 287;

pub const FLOAT_RIVAL_BAD_RATE: i32 = 288;

pub const FLOAT_RIVAL_POOR_RATE: i32 = 289;

pub const FLOAT_BEST_RATE: i32 = 183;

pub const FLOAT_RIVAL_RATE: i32 = 122;

pub const FLOAT_TARGET_RATE: i32 = 135;
pub const FLOAT_TARGET_RATE2: i32 = 157;

pub const FLOAT_HISPEED: i32 = 310;

pub const FLOAT_GROOVEGAUGE_1P: i32 = 1107;

pub const FLOAT_CHART_AVERAGEDENSITY: i32 = 367;
pub const FLOAT_CHART_ENDDENSITY: i32 = 362;
pub const FLOAT_CHART_PEAKDENSITY: i32 = 360;
pub const FLOAT_CHART_TOTALGAUGE: i32 = 368;

pub const FLOAT_LOADING_PROGRESS: i32 = 165;

pub const FLOAT_IR_TOTALCLEARRATE: i32 = 227;
pub const FLOAT_IR_TOTALFULLCOMBORATE: i32 = 229;

pub const FLOAT_IR_PLAYER_NOPLAY_RATE: i32 = 203;
pub const FLOAT_IR_PLAYER_FAILED_RATE: i32 = 211;
pub const FLOAT_IR_PLAYER_ASSIST_RATE: i32 = 205;
pub const FLOAT_IR_PLAYER_LIGHTASSIST_RATE: i32 = 207;
pub const FLOAT_IR_PLAYER_EASY_RATE: i32 = 213;
pub const FLOAT_IR_PLAYER_NORMAL_RATE: i32 = 215;
pub const FLOAT_IR_PLAYER_HARD_RATE: i32 = 217;
pub const FLOAT_IR_PLAYER_EXHARD_RATE: i32 = 209;
pub const FLOAT_IR_PLAYER_FULLCOMBO_RATE: i32 = 219;
pub const FLOAT_IR_PLAYER_PERFECT_RATE: i32 = 223;
pub const FLOAT_IR_PLAYER_MAX_RATE: i32 = 225;

pub const VALUE_JUDGE_1P_DURATION: ValueId = ValueId(525);

pub const VALUE_JUDGE_2P_DURATION: ValueId = ValueId(526);

pub const VALUE_JUDGE_3P_DURATION: ValueId = ValueId(527);

pub const VALUE_JUDGE_1P_SCRATCH: ValueId = ValueId(500);
pub const VALUE_JUDGE_1P_KEY1: ValueId = ValueId(501);
pub const VALUE_JUDGE_1P_KEY9: ValueId = ValueId(509);

pub const VALUE_JUDGE_2P_SCRATCH: ValueId = ValueId(510);
pub const VALUE_JUDGE_2P_KEY1: ValueId = ValueId(511);
pub const VALUE_JUDGE_2P_KEY9: ValueId = ValueId(519);

pub const VALUE_JUDGE_1P: ValueId = ValueId(520);
pub const VALUE_JUDGE_2P: ValueId = ValueId(521);
pub const VALUE_JUDGE_3P: ValueId = ValueId(522);

pub const VALUE_JUDGE_1P_KEY10: ValueId = ValueId(1510);
pub const VALUE_JUDGE_1P_KEY99: ValueId = ValueId(1599);

pub const VALUE_JUDGE_2P_KEY10: ValueId = ValueId(1610);
pub const VALUE_JUDGE_2P_KEY99: ValueId = ValueId(1699);

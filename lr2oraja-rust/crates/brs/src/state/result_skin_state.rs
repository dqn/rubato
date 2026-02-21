// Result-specific skin state synchronization.
//
// Updates SharedGameState with score, judge counts, rank, and update flags
// for the Result screen.

use bms_rule::ScoreData;
use bms_skin::property_id::{
    FLOAT_BEST_RATE, FLOAT_SCORE_RATE2, NUMBER_BAD, NUMBER_BAD_PLUS_POOR_PLUS_MISS,
    NUMBER_BEST_RATE, NUMBER_BEST_RATE_AFTERDOT, NUMBER_CLEAR, NUMBER_COMBOBREAK,
    NUMBER_DIFF_EXSCORE, NUMBER_DIFF_HIGHSCORE, NUMBER_DIFF_HIGHSCORE2, NUMBER_DIFF_MAXCOMBO,
    NUMBER_DIFF_MISSCOUNT, NUMBER_EARLY_BAD, NUMBER_EARLY_GOOD, NUMBER_EARLY_GREAT,
    NUMBER_EARLY_MISS, NUMBER_EARLY_PERFECT, NUMBER_EARLY_POOR, NUMBER_GOOD, NUMBER_GREAT,
    NUMBER_HIGHSCORE2, NUMBER_LATE_BAD, NUMBER_LATE_GOOD, NUMBER_LATE_GREAT, NUMBER_LATE_MISS,
    NUMBER_LATE_PERFECT, NUMBER_LATE_POOR, NUMBER_MAXCOMBO2, NUMBER_MAXCOMBO3, NUMBER_MISS,
    NUMBER_MISSCOUNT2, NUMBER_PERFECT, NUMBER_POOR, NUMBER_POOR_PLUS_MISS, NUMBER_SCORE_RATE,
    NUMBER_SCORE_RATE_AFTERDOT, NUMBER_SCORE2, NUMBER_SCORE3, NUMBER_TARGET_CLEAR,
    NUMBER_TARGET_MAXCOMBO, NUMBER_TARGET_MISSCOUNT, NUMBER_TOTALEARLY, NUMBER_TOTALLATE,
    NUMBER_TOTALNOTES2, OPTION_1PWIN, OPTION_2PWIN, OPTION_A, OPTION_AA, OPTION_AAA, OPTION_B,
    OPTION_BAD_EXIST, OPTION_BEST_A_1P, OPTION_BEST_AA_1P, OPTION_BEST_AAA_1P, OPTION_BEST_B_1P,
    OPTION_BEST_C_1P, OPTION_BEST_D_1P, OPTION_BEST_E_1P, OPTION_BEST_F_1P, OPTION_C, OPTION_D,
    OPTION_DRAW, OPTION_DRAW_MAXCOMBO, OPTION_DRAW_MISSCOUNT, OPTION_DRAW_SCORE,
    OPTION_DRAW_SCORERANK, OPTION_DRAW_TARGET, OPTION_E, OPTION_F, OPTION_GOOD_EXIST,
    OPTION_GREAT_EXIST, OPTION_MISS_EXIST, OPTION_PERFECT_EXIST, OPTION_POOR_EXIST,
    OPTION_RESULT_A_1P, OPTION_RESULT_AA_1P, OPTION_RESULT_AAA_1P, OPTION_RESULT_B_1P,
    OPTION_RESULT_C_1P, OPTION_RESULT_CLEAR, OPTION_RESULT_D_1P, OPTION_RESULT_E_1P,
    OPTION_RESULT_F_1P, OPTION_RESULT_FAIL, OPTION_UPDATE_MAXCOMBO, OPTION_UPDATE_MISSCOUNT,
    OPTION_UPDATE_SCORE, OPTION_UPDATE_SCORERANK, OPTION_UPDATE_TARGET,
};

use crate::game_state::SharedGameState;

/// Synchronize result-specific state into SharedGameState for skin rendering.
///
/// `target_exscore`: optional rival/target EX score for comparison flags.
pub fn sync_result_state(
    state: &mut SharedGameState,
    score: &ScoreData,
    oldscore: &ScoreData,
    maxcombo: i32,
    target_exscore: Option<i32>,
) {
    // Score values
    let ex = score.exscore();
    let old_ex = oldscore.exscore();
    state.integers.insert(NUMBER_SCORE2, ex);
    state.integers.insert(NUMBER_SCORE3, ex);
    state.integers.insert(NUMBER_MAXCOMBO2, maxcombo);
    state.integers.insert(NUMBER_MAXCOMBO3, maxcombo);
    state.integers.insert(NUMBER_TOTALNOTES2, score.notes);
    state.integers.insert(NUMBER_HIGHSCORE2, old_ex);
    state.integers.insert(NUMBER_MISSCOUNT2, score.minbp);

    // Clear type IDs (Java: NUMBER_CLEAR / NUMBER_TARGET_CLEAR)
    state.integers.insert(NUMBER_CLEAR, score.clear.id() as i32);
    state
        .integers
        .insert(NUMBER_TARGET_CLEAR, oldscore.clear.id() as i32);

    // Old score target properties (Java: NUMBER_TARGET_MAXCOMBO, NUMBER_TARGET_MISSCOUNT)
    if oldscore.maxcombo > 0 {
        state
            .integers
            .insert(NUMBER_TARGET_MAXCOMBO, oldscore.maxcombo);
    }
    if oldscore.notes > 0 {
        state
            .integers
            .insert(NUMBER_TARGET_MISSCOUNT, oldscore.minbp);
    }

    // Score diffs
    state.integers.insert(NUMBER_DIFF_EXSCORE, ex - old_ex);
    state.integers.insert(NUMBER_DIFF_HIGHSCORE, ex - old_ex);
    state.integers.insert(NUMBER_DIFF_HIGHSCORE2, ex - old_ex);
    state
        .integers
        .insert(NUMBER_DIFF_MAXCOMBO, maxcombo - oldscore.maxcombo);
    // Java: NUMBER_DIFF_MISSCOUNT = newScore.minbp - oldScore.minbp
    if oldscore.notes > 0 {
        state
            .integers
            .insert(NUMBER_DIFF_MISSCOUNT, score.minbp - oldscore.minbp);
    }

    // Judge counts
    state
        .integers
        .insert(NUMBER_PERFECT, score.judge_count(bms_rule::JUDGE_PG));
    state
        .integers
        .insert(NUMBER_GREAT, score.judge_count(bms_rule::JUDGE_GR));
    state
        .integers
        .insert(NUMBER_GOOD, score.judge_count(bms_rule::JUDGE_GD));
    state
        .integers
        .insert(NUMBER_BAD, score.judge_count(bms_rule::JUDGE_BD));
    state
        .integers
        .insert(NUMBER_POOR, score.judge_count(bms_rule::JUDGE_PR));
    state
        .integers
        .insert(NUMBER_MISS, score.judge_count(bms_rule::JUDGE_MS));

    // Score rate as integer (Java: NUMBER_SCORE_RATE / NUMBER_SCORE_RATE_AFTERDOT)
    let max_ex = score.notes * 2;
    if max_ex > 0 {
        let rate_100 = ex as f64 * 100.0 / max_ex as f64;
        state.integers.insert(NUMBER_SCORE_RATE, rate_100 as i32);
        state.integers.insert(
            NUMBER_SCORE_RATE_AFTERDOT,
            ((rate_100 * 100.0) as i32) % 100,
        );
        state.floats.insert(FLOAT_SCORE_RATE2, rate_100 as f32);
    }

    // Best rate (old score rate)
    if oldscore.notes > 0 {
        let old_max = oldscore.notes * 2;
        let best_rate_100 = old_ex as f64 * 100.0 / old_max as f64;
        state
            .integers
            .insert(NUMBER_BEST_RATE, best_rate_100 as i32);
        state.integers.insert(
            NUMBER_BEST_RATE_AFTERDOT,
            ((best_rate_100 * 100.0) as i32) % 100,
        );
        state.floats.insert(FLOAT_BEST_RATE, best_rate_100 as f32);
    }

    // Early/Late judge count split (Java: NUMBER_EARLY_PERFECT through NUMBER_LATE_MISS)
    state.integers.insert(
        NUMBER_EARLY_PERFECT,
        score.judge_count_early(bms_rule::JUDGE_PG),
    );
    state.integers.insert(
        NUMBER_LATE_PERFECT,
        score.judge_count_late(bms_rule::JUDGE_PG),
    );
    state.integers.insert(
        NUMBER_EARLY_GREAT,
        score.judge_count_early(bms_rule::JUDGE_GR),
    );
    state.integers.insert(
        NUMBER_LATE_GREAT,
        score.judge_count_late(bms_rule::JUDGE_GR),
    );
    state.integers.insert(
        NUMBER_EARLY_GOOD,
        score.judge_count_early(bms_rule::JUDGE_GD),
    );
    state
        .integers
        .insert(NUMBER_LATE_GOOD, score.judge_count_late(bms_rule::JUDGE_GD));
    state.integers.insert(
        NUMBER_EARLY_BAD,
        score.judge_count_early(bms_rule::JUDGE_BD),
    );
    state
        .integers
        .insert(NUMBER_LATE_BAD, score.judge_count_late(bms_rule::JUDGE_BD));
    state.integers.insert(
        NUMBER_EARLY_POOR,
        score.judge_count_early(bms_rule::JUDGE_PR),
    );
    state
        .integers
        .insert(NUMBER_LATE_POOR, score.judge_count_late(bms_rule::JUDGE_PR));
    state.integers.insert(
        NUMBER_EARLY_MISS,
        score.judge_count_early(bms_rule::JUDGE_MS),
    );
    state
        .integers
        .insert(NUMBER_LATE_MISS, score.judge_count_late(bms_rule::JUDGE_MS));

    // Aggregate early/late counts (Java: NUMBER_TOTALEARLY/TOTALLATE — sum indices 1-5)
    let total_early = score.judge_count_early(bms_rule::JUDGE_GR)
        + score.judge_count_early(bms_rule::JUDGE_GD)
        + score.judge_count_early(bms_rule::JUDGE_BD)
        + score.judge_count_early(bms_rule::JUDGE_PR)
        + score.judge_count_early(bms_rule::JUDGE_MS);
    let total_late = score.judge_count_late(bms_rule::JUDGE_GR)
        + score.judge_count_late(bms_rule::JUDGE_GD)
        + score.judge_count_late(bms_rule::JUDGE_BD)
        + score.judge_count_late(bms_rule::JUDGE_PR)
        + score.judge_count_late(bms_rule::JUDGE_MS);
    state.integers.insert(NUMBER_TOTALEARLY, total_early);
    state.integers.insert(NUMBER_TOTALLATE, total_late);

    // Combo break (Java: bad + poor count)
    let combo_break = score.judge_count(bms_rule::JUDGE_BD) + score.judge_count(bms_rule::JUDGE_PR);
    state.integers.insert(NUMBER_COMBOBREAK, combo_break);

    // Poor + Miss (Java: poor + miss count)
    let poor_plus_miss =
        score.judge_count(bms_rule::JUDGE_PR) + score.judge_count(bms_rule::JUDGE_MS);
    state.integers.insert(NUMBER_POOR_PLUS_MISS, poor_plus_miss);

    // Bad + Poor + Miss
    let bad_poor_miss = score.judge_count(bms_rule::JUDGE_BD)
        + score.judge_count(bms_rule::JUDGE_PR)
        + score.judge_count(bms_rule::JUDGE_MS);
    state
        .integers
        .insert(NUMBER_BAD_PLUS_POOR_PLUS_MISS, bad_poor_miss);

    // Clear/Fail flags
    let cleared =
        score.clear != bms_rule::ClearType::Failed && score.clear != bms_rule::ClearType::NoPlay;
    state.booleans.insert(OPTION_RESULT_CLEAR, cleared);
    state.booleans.insert(OPTION_RESULT_FAIL, !cleared);

    // Rank flags (based on score rate)
    let rate = if max_ex > 0 {
        ex as f64 / max_ex as f64
    } else {
        0.0
    };
    sync_rank_flags(state, rate);

    // Score rate as float
    state
        .floats
        .insert(bms_skin::property_id::FLOAT_SCORE_RATE, rate as f32 * 100.0);

    // Update flags (comparing with old score)
    let score_updated = ex > old_ex;
    let combo_updated = maxcombo > oldscore.maxcombo;
    let miss_updated = oldscore.notes > 0 && score.minbp < oldscore.minbp;
    state.booleans.insert(OPTION_UPDATE_SCORE, score_updated);
    state.booleans.insert(OPTION_UPDATE_MAXCOMBO, combo_updated);
    state.booleans.insert(OPTION_UPDATE_MISSCOUNT, miss_updated);

    // Rank update check
    let old_rate = if oldscore.notes > 0 {
        old_ex as f64 / (oldscore.notes * 2) as f64
    } else {
        0.0
    };
    state.booleans.insert(
        OPTION_UPDATE_SCORERANK,
        rank_index(rate) > rank_index(old_rate),
    );

    // Draw flags (equal comparisons)
    state.booleans.insert(OPTION_DRAW_SCORE, ex == old_ex);
    state
        .booleans
        .insert(OPTION_DRAW_MAXCOMBO, maxcombo == oldscore.maxcombo);
    state.booleans.insert(
        OPTION_DRAW_MISSCOUNT,
        oldscore.notes > 0 && score.minbp == oldscore.minbp,
    );
    state.booleans.insert(
        OPTION_DRAW_SCORERANK,
        rank_index(rate) == rank_index(old_rate),
    );

    // Target/rival comparison (Java: MusicResult rivalScore)
    if let Some(target) = target_exscore {
        state.booleans.insert(OPTION_UPDATE_TARGET, ex > target);
        state.booleans.insert(OPTION_DRAW_TARGET, ex == target);
        state.booleans.insert(OPTION_1PWIN, ex > target);
        state.booleans.insert(OPTION_2PWIN, ex < target);
        state.booleans.insert(OPTION_DRAW, ex == target);
    }

    // Judge existence flags (any count > 0)
    state.booleans.insert(
        OPTION_PERFECT_EXIST,
        score.judge_count(bms_rule::JUDGE_PG) > 0,
    );
    state.booleans.insert(
        OPTION_GREAT_EXIST,
        score.judge_count(bms_rule::JUDGE_GR) > 0,
    );
    state
        .booleans
        .insert(OPTION_GOOD_EXIST, score.judge_count(bms_rule::JUDGE_GD) > 0);
    state
        .booleans
        .insert(OPTION_BAD_EXIST, score.judge_count(bms_rule::JUDGE_BD) > 0);
    state
        .booleans
        .insert(OPTION_POOR_EXIST, score.judge_count(bms_rule::JUDGE_PR) > 0);
    state
        .booleans
        .insert(OPTION_MISS_EXIST, score.judge_count(bms_rule::JUDGE_MS) > 0);

    // Best rank flags (based on old score rate)
    sync_best_rank_flags(state, old_rate);

    // Overall rank flags (OPTION_AAA-F: >=threshold style)
    sync_overall_rank_flags(state, rate);
}

fn sync_rank_flags(state: &mut SharedGameState, rate: f64) {
    // Clear all rank flags
    let ranks = [
        OPTION_RESULT_AAA_1P,
        OPTION_RESULT_AA_1P,
        OPTION_RESULT_A_1P,
        OPTION_RESULT_B_1P,
        OPTION_RESULT_C_1P,
        OPTION_RESULT_D_1P,
        OPTION_RESULT_E_1P,
        OPTION_RESULT_F_1P,
    ];
    for &r in &ranks {
        state.booleans.insert(r, false);
    }

    // Set the appropriate rank (beatoraja thresholds)
    let rank_id = match rate {
        r if r >= 8.0 / 9.0 => OPTION_RESULT_AAA_1P,
        r if r >= 7.0 / 9.0 => OPTION_RESULT_AA_1P,
        r if r >= 6.0 / 9.0 => OPTION_RESULT_A_1P,
        r if r >= 5.0 / 9.0 => OPTION_RESULT_B_1P,
        r if r >= 4.0 / 9.0 => OPTION_RESULT_C_1P,
        r if r >= 3.0 / 9.0 => OPTION_RESULT_D_1P,
        r if r >= 2.0 / 9.0 => OPTION_RESULT_E_1P,
        _ => OPTION_RESULT_F_1P,
    };
    state.booleans.insert(rank_id, true);
}

fn rank_index(rate: f64) -> i32 {
    match rate {
        r if r >= 8.0 / 9.0 => 7,
        r if r >= 7.0 / 9.0 => 6,
        r if r >= 6.0 / 9.0 => 5,
        r if r >= 5.0 / 9.0 => 4,
        r if r >= 4.0 / 9.0 => 3,
        r if r >= 3.0 / 9.0 => 2,
        r if r >= 2.0 / 9.0 => 1,
        _ => 0,
    }
}

/// Set best rank flags (OPTION_BEST_AAA_1P through OPTION_BEST_F_1P).
fn sync_best_rank_flags(state: &mut SharedGameState, old_rate: f64) {
    let ranks = [
        OPTION_BEST_AAA_1P,
        OPTION_BEST_AA_1P,
        OPTION_BEST_A_1P,
        OPTION_BEST_B_1P,
        OPTION_BEST_C_1P,
        OPTION_BEST_D_1P,
        OPTION_BEST_E_1P,
        OPTION_BEST_F_1P,
    ];
    for &r in &ranks {
        state.booleans.insert(r, false);
    }

    let rank_id = match old_rate {
        r if r >= 8.0 / 9.0 => OPTION_BEST_AAA_1P,
        r if r >= 7.0 / 9.0 => OPTION_BEST_AA_1P,
        r if r >= 6.0 / 9.0 => OPTION_BEST_A_1P,
        r if r >= 5.0 / 9.0 => OPTION_BEST_B_1P,
        r if r >= 4.0 / 9.0 => OPTION_BEST_C_1P,
        r if r >= 3.0 / 9.0 => OPTION_BEST_D_1P,
        r if r >= 2.0 / 9.0 => OPTION_BEST_E_1P,
        _ => OPTION_BEST_F_1P,
    };
    state.booleans.insert(rank_id, true);
}

/// Set overall rank flags (OPTION_AAA through OPTION_F).
///
/// Java: these use >=threshold style (multiple can be true simultaneously).
/// e.g., if rank is AAA, then AAA/AA/A/B/C/D/E/F are all true.
fn sync_overall_rank_flags(state: &mut SharedGameState, rate: f64) {
    let thresholds = [
        (OPTION_AAA, 8.0 / 9.0),
        (OPTION_AA, 7.0 / 9.0),
        (OPTION_A, 6.0 / 9.0),
        (OPTION_B, 5.0 / 9.0),
        (OPTION_C, 4.0 / 9.0),
        (OPTION_D, 3.0 / 9.0),
        (OPTION_E, 2.0 / 9.0),
        (OPTION_F, 0.0),
    ];
    for &(id, threshold) in &thresholds {
        state.booleans.insert(id, rate >= threshold);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_score(exscore_half: i32, notes: i32) -> ScoreData {
        let mut s = ScoreData::default();
        s.epg = exscore_half; // epg contributes 2 to exscore
        s.notes = notes;
        s
    }

    #[test]
    fn test_sync_result_score_values() {
        let mut state = SharedGameState::default();
        let score = make_score(10, 20);
        let oldscore = make_score(5, 20);
        sync_result_state(&mut state, &score, &oldscore, 15, None);

        assert_eq!(*state.integers.get(&NUMBER_SCORE2).unwrap(), 20); // 10 * 2
        assert_eq!(*state.integers.get(&NUMBER_MAXCOMBO2).unwrap(), 15);
        assert_eq!(*state.integers.get(&NUMBER_HIGHSCORE2).unwrap(), 10); // 5 * 2
    }

    #[test]
    fn test_sync_result_clear_flag() {
        let mut state = SharedGameState::default();
        let mut score = ScoreData::default();
        score.clear = bms_rule::ClearType::Normal;
        score.notes = 10;
        sync_result_state(&mut state, &score, &ScoreData::default(), 0, None);
        assert!(*state.booleans.get(&OPTION_RESULT_CLEAR).unwrap());
        assert!(!*state.booleans.get(&OPTION_RESULT_FAIL).unwrap());
    }

    #[test]
    fn test_sync_result_fail_flag() {
        let mut state = SharedGameState::default();
        let mut score = ScoreData::default();
        score.clear = bms_rule::ClearType::Failed;
        score.notes = 10;
        sync_result_state(&mut state, &score, &ScoreData::default(), 0, None);
        assert!(!*state.booleans.get(&OPTION_RESULT_CLEAR).unwrap());
        assert!(*state.booleans.get(&OPTION_RESULT_FAIL).unwrap());
    }

    #[test]
    fn test_rank_aaa() {
        let mut state = SharedGameState::default();
        sync_rank_flags(&mut state, 0.95);
        assert!(*state.booleans.get(&OPTION_RESULT_AAA_1P).unwrap());
        assert!(!*state.booleans.get(&OPTION_RESULT_AA_1P).unwrap());
    }

    #[test]
    fn test_rank_f() {
        let mut state = SharedGameState::default();
        sync_rank_flags(&mut state, 0.1);
        assert!(*state.booleans.get(&OPTION_RESULT_F_1P).unwrap());
        assert!(!*state.booleans.get(&OPTION_RESULT_AAA_1P).unwrap());
    }

    #[test]
    fn test_update_flags() {
        let mut state = SharedGameState::default();
        let mut score = make_score(15, 20);
        score.minbp = 3;
        let mut oldscore = make_score(10, 20);
        oldscore.minbp = 5;
        sync_result_state(&mut state, &score, &oldscore, 18, None);
        assert!(*state.booleans.get(&OPTION_UPDATE_SCORE).unwrap());
        assert!(*state.booleans.get(&OPTION_UPDATE_MISSCOUNT).unwrap());
    }

    #[test]
    fn test_defaults() {
        let mut state = SharedGameState::default();
        sync_result_state(
            &mut state,
            &ScoreData::default(),
            &ScoreData::default(),
            0,
            None,
        );
        assert!(state.integers.contains_key(&NUMBER_SCORE2));
        assert!(state.booleans.contains_key(&OPTION_RESULT_CLEAR));
    }
}

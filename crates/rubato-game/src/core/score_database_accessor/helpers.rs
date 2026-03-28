use rubato_types::player_data::PlayerData;
use rubato_types::score_data::ScoreData;

pub(super) fn row_to_score_data(row: &rusqlite::Row) -> ScoreData {
    use rubato_types::score_data::{JudgeCounts, PlayOption, TimingStats};
    ScoreData {
        sha256: row.get::<_, String>("sha256").unwrap_or_default(),
        mode: row.get("mode").unwrap_or(0),
        clear: row.get("clear").unwrap_or(0),
        judge_counts: JudgeCounts {
            epg: row.get("epg").unwrap_or(0),
            lpg: row.get("lpg").unwrap_or(0),
            egr: row.get("egr").unwrap_or(0),
            lgr: row.get("lgr").unwrap_or(0),
            egd: row.get("egd").unwrap_or(0),
            lgd: row.get("lgd").unwrap_or(0),
            ebd: row.get("ebd").unwrap_or(0),
            lbd: row.get("lbd").unwrap_or(0),
            epr: row.get("epr").unwrap_or(0),
            lpr: row.get("lpr").unwrap_or(0),
            ems: row.get("ems").unwrap_or(0),
            lms: row.get("lms").unwrap_or(0),
        },
        notes: row.get("notes").unwrap_or(0),
        maxcombo: row.get("combo").unwrap_or(0),
        minbp: row.get("minbp").unwrap_or(i32::MAX),
        timing_stats: TimingStats {
            avgjudge: {
                let raw: i64 = row.get("avgjudge").unwrap_or(i64::MAX);
                if raw == i32::MAX as i64 {
                    i64::MAX
                } else {
                    raw
                }
            },
            ..Default::default()
        },
        playcount: row.get("playcount").unwrap_or(0),
        clearcount: row.get("clearcount").unwrap_or(0),
        trophy: row.get::<_, String>("trophy").unwrap_or_default(),
        ghost: row.get::<_, String>("ghost").unwrap_or_default(),
        play_option: PlayOption {
            option: row.get("option").unwrap_or(0),
            seed: row.get("seed").unwrap_or(-1),
            random: row.get("random").unwrap_or(0),
            ..Default::default()
        },
        date: row.get("date").unwrap_or(0),
        state: row.get("state").unwrap_or(0),
        scorehash: row.get::<_, String>("scorehash").unwrap_or_default(),
        ..Default::default()
    }
}

pub(super) fn row_to_player_data(row: &rusqlite::Row) -> PlayerData {
    PlayerData {
        date: row.get("date").unwrap_or(0),
        playcount: row.get("playcount").unwrap_or(0),
        clear: row.get("clear").unwrap_or(0),
        epg: row.get("epg").unwrap_or(0),
        lpg: row.get("lpg").unwrap_or(0),
        egr: row.get("egr").unwrap_or(0),
        lgr: row.get("lgr").unwrap_or(0),
        egd: row.get("egd").unwrap_or(0),
        lgd: row.get("lgd").unwrap_or(0),
        ebd: row.get("ebd").unwrap_or(0),
        lbd: row.get("lbd").unwrap_or(0),
        epr: row.get("epr").unwrap_or(0),
        lpr: row.get("lpr").unwrap_or(0),
        ems: row.get("ems").unwrap_or(0),
        lms: row.get("lms").unwrap_or(0),
        playtime: row.get("playtime").unwrap_or(0),
        maxcombo: row.get("maxcombo").unwrap_or(0),
    }
}

pub(super) fn score_data_to_value(score: &ScoreData, col_name: &str) -> rusqlite::types::Value {
    match col_name {
        "sha256" => rusqlite::types::Value::Text(score.sha256.clone()),
        "mode" => rusqlite::types::Value::Integer(score.mode as i64),
        "clear" => rusqlite::types::Value::Integer(score.clear as i64),
        "epg" => rusqlite::types::Value::Integer(score.judge_counts.epg as i64),
        "lpg" => rusqlite::types::Value::Integer(score.judge_counts.lpg as i64),
        "egr" => rusqlite::types::Value::Integer(score.judge_counts.egr as i64),
        "lgr" => rusqlite::types::Value::Integer(score.judge_counts.lgr as i64),
        "egd" => rusqlite::types::Value::Integer(score.judge_counts.egd as i64),
        "lgd" => rusqlite::types::Value::Integer(score.judge_counts.lgd as i64),
        "ebd" => rusqlite::types::Value::Integer(score.judge_counts.ebd as i64),
        "lbd" => rusqlite::types::Value::Integer(score.judge_counts.lbd as i64),
        "epr" => rusqlite::types::Value::Integer(score.judge_counts.epr as i64),
        "lpr" => rusqlite::types::Value::Integer(score.judge_counts.lpr as i64),
        "ems" => rusqlite::types::Value::Integer(score.judge_counts.ems as i64),
        "lms" => rusqlite::types::Value::Integer(score.judge_counts.lms as i64),
        "notes" => rusqlite::types::Value::Integer(score.notes as i64),
        "combo" => rusqlite::types::Value::Integer(score.maxcombo as i64),
        "minbp" => rusqlite::types::Value::Integer(score.minbp as i64),
        // Normalize sentinel: write i32::MAX (not i64::MAX) for Java DB compatibility.
        "avgjudge" => rusqlite::types::Value::Integer(if score.timing_stats.avgjudge == i64::MAX {
            i32::MAX as i64
        } else {
            score.timing_stats.avgjudge
        }),
        "playcount" => rusqlite::types::Value::Integer(score.playcount as i64),
        "clearcount" => rusqlite::types::Value::Integer(score.clearcount as i64),
        "trophy" => rusqlite::types::Value::Text(score.trophy.clone()),
        "ghost" => rusqlite::types::Value::Text(score.ghost.clone()),
        "option" => rusqlite::types::Value::Integer(score.play_option.option as i64),
        "seed" => rusqlite::types::Value::Integer(score.play_option.seed),
        "random" => rusqlite::types::Value::Integer(score.play_option.random as i64),
        "date" => rusqlite::types::Value::Integer(score.date),
        "state" => rusqlite::types::Value::Integer(score.state as i64),
        "scorehash" => rusqlite::types::Value::Text(score.scorehash.clone()),
        _ => rusqlite::types::Value::Null,
    }
}

/// Calculate today's local midnight as a unix timestamp.
///
/// Handles DST transitions safely:
/// - Ambiguous time (clocks fall back): picks the earlier of the two.
/// - Non-existent time (clocks spring forward): falls back to the current local time's
///   start-of-day in UTC.
pub(super) fn local_midnight_timestamp() -> i64 {
    let naive_midnight = chrono::Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("valid time");
    naive_midnight
        .and_local_timezone(chrono::Local)
        .earliest()
        .unwrap_or_else(|| {
            // DST spring forward: local midnight doesn't exist, fall back to UTC interpretation
            naive_midnight.and_utc().with_timezone(&chrono::Local)
        })
        .timestamp()
}

pub(super) fn player_data_to_value(pd: &PlayerData, col_name: &str) -> rusqlite::types::Value {
    match col_name {
        "date" => rusqlite::types::Value::Integer(pd.date),
        "playcount" => rusqlite::types::Value::Integer(pd.playcount),
        "clear" => rusqlite::types::Value::Integer(pd.clear),
        "epg" => rusqlite::types::Value::Integer(pd.epg),
        "lpg" => rusqlite::types::Value::Integer(pd.lpg),
        "egr" => rusqlite::types::Value::Integer(pd.egr),
        "lgr" => rusqlite::types::Value::Integer(pd.lgr),
        "egd" => rusqlite::types::Value::Integer(pd.egd),
        "lgd" => rusqlite::types::Value::Integer(pd.lgd),
        "ebd" => rusqlite::types::Value::Integer(pd.ebd),
        "lbd" => rusqlite::types::Value::Integer(pd.lbd),
        "epr" => rusqlite::types::Value::Integer(pd.epr),
        "lpr" => rusqlite::types::Value::Integer(pd.lpr),
        "ems" => rusqlite::types::Value::Integer(pd.ems),
        "lms" => rusqlite::types::Value::Integer(pd.lms),
        "playtime" => rusqlite::types::Value::Integer(pd.playtime),
        "maxcombo" => rusqlite::types::Value::Integer(pd.maxcombo),
        _ => rusqlite::types::Value::Null,
    }
}

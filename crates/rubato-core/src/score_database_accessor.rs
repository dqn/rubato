use std::collections::HashMap;

use rusqlite::Connection;

use crate::player_data::PlayerData;
use crate::player_information::PlayerInformation;
use crate::score_data::ScoreData;
use crate::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};
use crate::validatable::Validatable;

// Re-export SongData stub from stubs module for use by other accessors
pub use crate::stubs::SongData;

pub trait ScoreDataCollector {
    fn collect(&mut self, song: &SongData, score: Option<&ScoreData>);
}

const LOAD_CHUNK_SIZE: usize = 1000;

/// Score database accessor.
/// Translated from Java: ScoreDatabaseAccessor extends SQLiteDatabaseAccessor
pub struct ScoreDatabaseAccessor {
    conn: Connection,
    base: SQLiteDatabaseAccessor,
}

impl ScoreDatabaseAccessor {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA shared_cache = ON")?;
        conn.pragma_update(None, "synchronous", "OFF")?;
        conn.pragma_update(None, "cache_size", 2000)?;

        let tables = vec![
            Table::new(
                "info",
                vec![
                    Column::with_pk("id", "TEXT", 1, 1),
                    Column::with_pk("name", "TEXT", 1, 0),
                    Column::new("rank", "TEXT"),
                ],
            ),
            Table::new(
                "player",
                vec![
                    Column::with_pk("date", "INTEGER", 0, 1),
                    Column::new("playcount", "INTEGER"),
                    Column::new("clear", "INTEGER"),
                    Column::new("epg", "INTEGER"),
                    Column::new("lpg", "INTEGER"),
                    Column::new("egr", "INTEGER"),
                    Column::new("lgr", "INTEGER"),
                    Column::new("egd", "INTEGER"),
                    Column::new("lgd", "INTEGER"),
                    Column::new("ebd", "INTEGER"),
                    Column::new("lbd", "INTEGER"),
                    Column::new("epr", "INTEGER"),
                    Column::new("lpr", "INTEGER"),
                    Column::new("ems", "INTEGER"),
                    Column::new("lms", "INTEGER"),
                    Column::new("playtime", "INTEGER"),
                    Column::new("maxcombo", "INTEGER"),
                ],
            ),
            Table::new(
                "score",
                vec![
                    Column::with_pk("sha256", "TEXT", 1, 1),
                    Column::with_pk("mode", "INTEGER", 0, 1),
                    Column::new("clear", "INTEGER"),
                    Column::new("epg", "INTEGER"),
                    Column::new("lpg", "INTEGER"),
                    Column::new("egr", "INTEGER"),
                    Column::new("lgr", "INTEGER"),
                    Column::new("egd", "INTEGER"),
                    Column::new("lgd", "INTEGER"),
                    Column::new("ebd", "INTEGER"),
                    Column::new("lbd", "INTEGER"),
                    Column::new("epr", "INTEGER"),
                    Column::new("lpr", "INTEGER"),
                    Column::new("ems", "INTEGER"),
                    Column::new("lms", "INTEGER"),
                    Column::new("notes", "INTEGER"),
                    Column::new("combo", "INTEGER"),
                    Column::new("minbp", "INTEGER"),
                    Column::with_default("avgjudge", "INTEGER", 1, 0, &i32::MAX.to_string()),
                    Column::new("playcount", "INTEGER"),
                    Column::new("clearcount", "INTEGER"),
                    Column::new("trophy", "TEXT"),
                    Column::new("ghost", "TEXT"),
                    Column::new("option", "INTEGER"),
                    Column::new("seed", "INTEGER"),
                    Column::new("random", "INTEGER"),
                    Column::new("date", "INTEGER"),
                    Column::new("state", "INTEGER"),
                    Column::new("scorehash", "TEXT"),
                ],
            ),
        ];

        let base = SQLiteDatabaseAccessor::new(tables);

        Ok(Self { conn, base })
    }

    pub fn create_table(&self) {
        if let Err(e) = self.base.validate(&self.conn) {
            log::error!("Exception during score database initialization: {}", e);
            return;
        }
        if self.player_datas(1).is_empty() {
            let pd = PlayerData::default();
            if let Err(e) = self
                .base
                .insert_with_values(&self.conn, "player", &|col_name| {
                    player_data_to_value(&pd, col_name)
                })
            {
                log::error!("Exception during score database initialization: {}", e);
            }
        }
    }

    pub fn information(&self) -> Option<PlayerInformation> {
        match self
            .conn
            .prepare("SELECT * FROM info")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| {
                    Ok(PlayerInformation {
                        id: row.get(0).ok(),
                        name: row.get(1).ok(),
                        rank: row.get(2).ok(),
                    })
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
            }) {
            Ok(info) => {
                if !info.is_empty() {
                    Some(info.into_iter().next().expect("iterator has next"))
                } else {
                    None
                }
            }
            Err(e) => {
                log::error!("Exception getting score: {}", e);
                None
            }
        }
    }

    pub fn set_information(&self, info: &PlayerInformation) {
        if let Err(e) = (|| -> anyhow::Result<()> {
            self.conn.execute("DELETE FROM info", [])?;
            self.base
                .insert_with_values(&self.conn, "info", &|col_name| match col_name {
                    "id" => rusqlite::types::Value::Text(info.id.clone().unwrap_or_default()),
                    "name" => rusqlite::types::Value::Text(info.name.clone().unwrap_or_default()),
                    "rank" => rusqlite::types::Value::Text(info.rank.clone().unwrap_or_default()),
                    _ => rusqlite::types::Value::Null,
                })?;
            Ok(())
        })() {
            log::error!("Exception setting information: {}", e);
        }
    }

    pub fn score_data(&self, hash: &str, mode: i32) -> Option<ScoreData> {
        match self
            .conn
            .prepare("SELECT * FROM score WHERE sha256 = ?1 AND mode = ?2")
            .and_then(|mut stmt| {
                stmt.query_map(rusqlite::params![hash, mode], |row| {
                    Ok(row_to_score_data(row))
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
            }) {
            Ok(scores) => {
                let scores: Vec<ScoreData> = scores
                    .into_iter()
                    .filter_map(|mut s| if s.validate() { Some(s) } else { None })
                    .collect();
                if scores.is_empty() {
                    return None;
                }
                let mut best: Option<ScoreData> = None;
                for s in scores {
                    if best.is_none() || s.clear > best.as_ref().expect("best is Some").clear {
                        best = Some(s);
                    }
                }
                best
            }
            Err(e) => {
                log::error!("Exception getting score: {}", e);
                None
            }
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub fn score_datas_for_songs(
        &self,
        collector: &mut dyn ScoreDataCollector,
        songs: &[SongData],
        lnmode: i32,
    ) {
        let mut str_buf = String::with_capacity(songs.len() * 68);
        self.get_score_datas_inner(collector, songs, lnmode, &mut str_buf, true);
        str_buf.clear();
        self.get_score_datas_inner(collector, songs, 0, &mut str_buf, false);
    }

    #[allow(clippy::needless_range_loop)]
    fn get_score_datas_inner(
        &self,
        collector: &mut dyn ScoreDataCollector,
        songs: &[SongData],
        mode: i32,
        _str_buf: &mut String,
        hasln: bool,
    ) {
        let result: Result<(), anyhow::Error> = (|| {
            let song_length = songs.len();
            let chunk_length = song_length.div_ceil(LOAD_CHUNK_SIZE);
            let mut scores: Vec<ScoreData> = Vec::new();

            for i in 0..chunk_length {
                let chunk_start = i * LOAD_CHUNK_SIZE;
                let chunk_end = std::cmp::min(song_length, (i + 1) * LOAD_CHUNK_SIZE);
                let mut chunk_hashes: Vec<String> = Vec::new();
                for j in chunk_start..chunk_end {
                    let song = &songs[j];
                    let has_uln = !song.file.sha256.is_empty();
                    if (hasln && has_uln) || (!hasln && !has_uln) {
                        chunk_hashes.push(song.file.sha256.clone());
                    }
                }

                if !chunk_hashes.is_empty() {
                    let placeholders: Vec<String> = chunk_hashes
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("?{}", i + 1))
                        .collect();
                    let sql = format!(
                        "SELECT * FROM score WHERE sha256 IN ({}) AND mode = ?{}",
                        placeholders.join(","),
                        chunk_hashes.len() + 1
                    );
                    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = chunk_hashes
                        .iter()
                        .map(|h| Box::new(h.clone()) as Box<dyn rusqlite::types::ToSql>)
                        .collect();
                    params.push(Box::new(mode));
                    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                        params.iter().map(|p| p.as_ref()).collect();
                    let mut stmt = self.conn.prepare(&sql)?;
                    let sub_scores: Vec<ScoreData> = stmt
                        .query_map(param_refs.as_slice(), |row| Ok(row_to_score_data(row)))?
                        .filter_map(|r| r.ok())
                        .filter_map(|mut s| if s.validate() { Some(s) } else { None })
                        .collect();
                    scores.extend(sub_scores);
                }
            }

            for song in songs {
                let has_uln = !song.file.sha256.is_empty();
                if (hasln && has_uln) || (!hasln && !has_uln) {
                    let sha = song.file.sha256.as_str();
                    let mut found = false;
                    for score in &scores {
                        if sha == score.sha256 {
                            collector.collect(song, Some(score));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        collector.collect(song, None);
                    }
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            log::error!("Exception getting scores: {}", e);
        }
    }

    pub fn score_datas(&self, sql: &str) -> Option<Vec<ScoreData>> {
        match self
            .conn
            .prepare(&format!("SELECT * FROM score WHERE {}", sql))
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok(row_to_score_data(row)))
                    .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
            }) {
            Ok(scores) => Some(
                scores
                    .into_iter()
                    .filter_map(|mut s| if s.validate() { Some(s) } else { None })
                    .collect(),
            ),
            Err(e) => {
                log::error!("Exception getting scores: {}", e);
                None
            }
        }
    }

    pub fn set_score_data(&self, score: &ScoreData) {
        self.set_score_data_batch(&[score]);
    }

    pub fn set_score_data_batch(&self, scores: &[&ScoreData]) {
        let result: anyhow::Result<()> = (|| {
            let tx = self.conn.unchecked_transaction()?;
            for score in scores {
                self.base
                    .insert_with_values(&self.conn, "score", &|col_name| {
                        score_data_to_value(score, col_name)
                    })?;
            }
            tx.commit()?;
            Ok(())
        })();
        if let Err(e) = result {
            log::error!("Exception updating score: {}", e);
        }
    }

    pub fn set_score_data_map(&self, map: &HashMap<String, HashMap<String, String>>) {
        // Whitelist valid score column names to prevent SQL injection
        const VALID_SCORE_COLUMNS: &[&str] = &[
            "sha256",
            "player",
            "mode",
            "clear",
            "date",
            "playcount",
            "clearcount",
            "epg",
            "lpg",
            "egr",
            "lgr",
            "egd",
            "lgd",
            "ebd",
            "lbd",
            "epr",
            "lpr",
            "ems",
            "lms",
            "maxcombo",
            "notes",
            "passnotes",
            "minbp",
            "avgjudge",
            "totalDuration",
            "avg",
            "totalAvg",
            "stddev",
            "option",
            "seed",
            "random",
            "judge",
            "gauge",
            "state",
            "scorehash",
            "combo",
            "trophy",
        ];

        let result: anyhow::Result<()> = (|| {
            let tx = self.conn.unchecked_transaction()?;
            for (hash, values) in map {
                let mut set_parts: Vec<String> = Vec::new();
                let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
                let mut idx = 1;

                for (key, val) in values {
                    if !VALID_SCORE_COLUMNS.contains(&key.as_str()) {
                        log::warn!("Invalid column name for score update: {}", key);
                        continue;
                    }
                    set_parts.push(format!("[{}] = ?{}", key, idx));
                    params.push(Box::new(val.clone()));
                    idx += 1;
                }
                if !set_parts.is_empty() {
                    let sql = format!(
                        "UPDATE score SET {} WHERE sha256 = ?{}",
                        set_parts.join(", "),
                        idx
                    );
                    params.push(Box::new(hash.clone()));
                    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                        params.iter().map(|p| p.as_ref()).collect();
                    self.conn.execute(&sql, param_refs.as_slice())?;
                }
            }
            tx.commit()?;
            Ok(())
        })();
        if let Err(e) = result {
            log::error!("Exception updating score: {}", e);
        }
    }

    pub fn delete_score_data(&self, sha256: &str, mode: i32) {
        if let Err(e) = self.conn.execute(
            "DELETE FROM score WHERE sha256 = ? and mode = ?",
            rusqlite::params![sha256, mode],
        ) {
            log::error!("Exception deleting score: {}", e);
        }
    }

    pub fn player_data(&self) -> Option<PlayerData> {
        let pds = self.player_datas(1);
        if !pds.is_empty() {
            Some(pds.into_iter().next().expect("iterator has next"))
        } else {
            None
        }
    }

    pub fn player_datas(&self, count: i32) -> Vec<PlayerData> {
        let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = if count > 0 {
            (
                "SELECT * FROM player ORDER BY date DESC LIMIT ?1",
                vec![Box::new(count) as Box<dyn rusqlite::types::ToSql>],
            )
        } else {
            ("SELECT * FROM player ORDER BY date DESC", vec![])
        };
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        match self.conn.prepare(sql).and_then(|mut stmt| {
            stmt.query_map(param_refs.as_slice(), |row| Ok(row_to_player_data(row)))
                .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        }) {
            Ok(pds) => pds,
            Err(e) => {
                log::error!("Exception getting player data: {}", e);
                Vec::new()
            }
        }
    }

    pub fn set_player_data(&self, pd: &PlayerData) {
        let result: anyhow::Result<()> = (|| {
            let tx = self.conn.unchecked_transaction()?;

            // Calculate today's local midnight unixtime
            // Java uses Calendar.getInstance(TimeZone.getDefault()) for local timezone
            let unixtime = local_midnight_timestamp();

            let mut pd_copy = *pd;
            pd_copy.date = unixtime;

            self.base
                .insert_with_values(&self.conn, "player", &|col_name| {
                    player_data_to_value(&pd_copy, col_name)
                })?;
            tx.commit()?;
            Ok(())
        })();
        if let Err(e) = result {
            log::error!("Exception updating score: {}", e);
        }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

impl rubato_types::score_database_access::ScoreDatabaseAccess for ScoreDatabaseAccessor {
    fn create_table(&self) {
        ScoreDatabaseAccessor::create_table(self);
    }

    fn score_data(&self, sha256: &str, mode: i32) -> Option<ScoreData> {
        ScoreDatabaseAccessor::score_data(self, sha256, mode)
    }

    fn set_score_data_slice(&self, scores: &[ScoreData]) {
        let refs: Vec<&ScoreData> = scores.iter().collect();
        self.set_score_data_batch(&refs);
    }
}

fn row_to_score_data(row: &rusqlite::Row) -> ScoreData {
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
            avgjudge: row.get("avgjudge").unwrap_or(i64::MAX),
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

fn row_to_player_data(row: &rusqlite::Row) -> PlayerData {
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

fn score_data_to_value(score: &ScoreData, col_name: &str) -> rusqlite::types::Value {
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
        "avgjudge" => rusqlite::types::Value::Integer(score.timing_stats.avgjudge),
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
fn local_midnight_timestamp() -> i64 {
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

fn player_data_to_value(pd: &PlayerData, col_name: &str) -> rusqlite::types::Value {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_midnight_timestamp_does_not_panic() {
        // This would panic before the fix if called during a DST transition
        // because and_local_timezone().unwrap() fails on Ambiguous/None results.
        let ts = local_midnight_timestamp();
        assert!(ts > 0, "timestamp should be positive");
    }

    #[test]
    fn local_midnight_timestamp_is_start_of_day() {
        let ts = local_midnight_timestamp();
        let now_ts = chrono::Local::now().timestamp();
        // Midnight should be at most 24 hours before now (86400 seconds)
        assert!(
            now_ts - ts < 86400,
            "midnight timestamp should be within the last 24 hours"
        );
        assert!(ts <= now_ts, "midnight should not be in the future");
    }

    #[test]
    fn set_player_data_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_score.db");
        let accessor = ScoreDatabaseAccessor::new(db_path.to_str().unwrap()).unwrap();
        accessor.create_table();

        let pd = PlayerData {
            playcount: 10,
            clear: 5,
            playtime: 3600,
            ..Default::default()
        };

        // This should not panic even during DST transitions
        accessor.set_player_data(&pd);

        // Verify the data was written
        let loaded = accessor.player_data();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.playcount, 10);
        assert_eq!(loaded.clear, 5);
        assert!(loaded.date > 0, "date should be set to local midnight");
    }

    // --- score_data_to_value tests ---

    #[test]
    fn test_score_data_to_value_basic() {
        use rubato_types::score_data::{JudgeCounts, PlayOption, TimingStats};
        let sd = ScoreData {
            sha256: "abc123def456".to_string(),
            mode: 7,
            clear: 5,
            judge_counts: JudgeCounts {
                epg: 100,
                lpg: 90,
                egr: 80,
                lgr: 70,
                egd: 10,
                lgd: 9,
                ebd: 3,
                lbd: 2,
                epr: 1,
                lpr: 0,
                ems: 4,
                lms: 5,
            },
            notes: 500,
            maxcombo: 300,
            minbp: 15,
            timing_stats: TimingStats {
                avgjudge: 42,
                ..Default::default()
            },
            playcount: 10,
            clearcount: 7,
            trophy: "g".to_string(),
            ghost: "ghost_data".to_string(),
            play_option: PlayOption {
                option: 2,
                seed: 12345,
                random: 1,
                ..Default::default()
            },
            date: 1700000000,
            state: 3,
            scorehash: "hashvalue".to_string(),
            ..Default::default()
        };

        assert_eq!(
            score_data_to_value(&sd, "sha256"),
            rusqlite::types::Value::Text("abc123def456".to_string())
        );
        assert_eq!(
            score_data_to_value(&sd, "mode"),
            rusqlite::types::Value::Integer(7)
        );
        assert_eq!(
            score_data_to_value(&sd, "clear"),
            rusqlite::types::Value::Integer(5)
        );
        assert_eq!(
            score_data_to_value(&sd, "epg"),
            rusqlite::types::Value::Integer(100)
        );
        assert_eq!(
            score_data_to_value(&sd, "lpg"),
            rusqlite::types::Value::Integer(90)
        );
        assert_eq!(
            score_data_to_value(&sd, "egr"),
            rusqlite::types::Value::Integer(80)
        );
        assert_eq!(
            score_data_to_value(&sd, "lgr"),
            rusqlite::types::Value::Integer(70)
        );
        assert_eq!(
            score_data_to_value(&sd, "egd"),
            rusqlite::types::Value::Integer(10)
        );
        assert_eq!(
            score_data_to_value(&sd, "lgd"),
            rusqlite::types::Value::Integer(9)
        );
        assert_eq!(
            score_data_to_value(&sd, "ebd"),
            rusqlite::types::Value::Integer(3)
        );
        assert_eq!(
            score_data_to_value(&sd, "lbd"),
            rusqlite::types::Value::Integer(2)
        );
        assert_eq!(
            score_data_to_value(&sd, "epr"),
            rusqlite::types::Value::Integer(1)
        );
        assert_eq!(
            score_data_to_value(&sd, "lpr"),
            rusqlite::types::Value::Integer(0)
        );
        assert_eq!(
            score_data_to_value(&sd, "ems"),
            rusqlite::types::Value::Integer(4)
        );
        assert_eq!(
            score_data_to_value(&sd, "lms"),
            rusqlite::types::Value::Integer(5)
        );
        assert_eq!(
            score_data_to_value(&sd, "notes"),
            rusqlite::types::Value::Integer(500)
        );
        // "combo" maps to maxcombo
        assert_eq!(
            score_data_to_value(&sd, "combo"),
            rusqlite::types::Value::Integer(300)
        );
        assert_eq!(
            score_data_to_value(&sd, "minbp"),
            rusqlite::types::Value::Integer(15)
        );
        assert_eq!(
            score_data_to_value(&sd, "avgjudge"),
            rusqlite::types::Value::Integer(42)
        );
        assert_eq!(
            score_data_to_value(&sd, "playcount"),
            rusqlite::types::Value::Integer(10)
        );
        assert_eq!(
            score_data_to_value(&sd, "clearcount"),
            rusqlite::types::Value::Integer(7)
        );
        assert_eq!(
            score_data_to_value(&sd, "trophy"),
            rusqlite::types::Value::Text("g".to_string())
        );
        assert_eq!(
            score_data_to_value(&sd, "ghost"),
            rusqlite::types::Value::Text("ghost_data".to_string())
        );
        assert_eq!(
            score_data_to_value(&sd, "option"),
            rusqlite::types::Value::Integer(2)
        );
        assert_eq!(
            score_data_to_value(&sd, "seed"),
            rusqlite::types::Value::Integer(12345)
        );
        assert_eq!(
            score_data_to_value(&sd, "random"),
            rusqlite::types::Value::Integer(1)
        );
        assert_eq!(
            score_data_to_value(&sd, "date"),
            rusqlite::types::Value::Integer(1700000000)
        );
        assert_eq!(
            score_data_to_value(&sd, "state"),
            rusqlite::types::Value::Integer(3)
        );
        assert_eq!(
            score_data_to_value(&sd, "scorehash"),
            rusqlite::types::Value::Text("hashvalue".to_string())
        );
    }

    #[test]
    fn test_score_data_to_value_default_fields() {
        let sd = ScoreData::default();

        assert_eq!(
            score_data_to_value(&sd, "sha256"),
            rusqlite::types::Value::Text(String::new())
        );
        assert_eq!(
            score_data_to_value(&sd, "mode"),
            rusqlite::types::Value::Integer(0)
        );
        assert_eq!(
            score_data_to_value(&sd, "clear"),
            rusqlite::types::Value::Integer(0)
        );
        assert_eq!(
            score_data_to_value(&sd, "minbp"),
            rusqlite::types::Value::Integer(i32::MAX as i64)
        );
        assert_eq!(
            score_data_to_value(&sd, "avgjudge"),
            rusqlite::types::Value::Integer(i64::MAX)
        );
        assert_eq!(
            score_data_to_value(&sd, "seed"),
            rusqlite::types::Value::Integer(-1)
        );
        assert_eq!(
            score_data_to_value(&sd, "trophy"),
            rusqlite::types::Value::Text(String::new())
        );
        assert_eq!(
            score_data_to_value(&sd, "ghost"),
            rusqlite::types::Value::Text(String::new())
        );
        assert_eq!(
            score_data_to_value(&sd, "scorehash"),
            rusqlite::types::Value::Text(String::new())
        );
    }

    #[test]
    fn test_score_data_to_value_unknown_column_returns_null() {
        let sd = ScoreData::default();

        assert_eq!(
            score_data_to_value(&sd, "nonexistent"),
            rusqlite::types::Value::Null
        );
        assert_eq!(score_data_to_value(&sd, ""), rusqlite::types::Value::Null);
    }

    // --- player_data_to_value tests ---

    #[test]
    fn test_player_data_to_value_basic() {
        let pd = PlayerData {
            date: 1700000000,
            playcount: 50,
            clear: 30,
            epg: 100,
            lpg: 90,
            egr: 80,
            lgr: 70,
            egd: 10,
            lgd: 9,
            ebd: 3,
            lbd: 2,
            epr: 1,
            lpr: 0,
            ems: 4,
            lms: 5,
            playtime: 7200,
            maxcombo: 500,
        };

        assert_eq!(
            player_data_to_value(&pd, "date"),
            rusqlite::types::Value::Integer(1700000000)
        );
        assert_eq!(
            player_data_to_value(&pd, "playcount"),
            rusqlite::types::Value::Integer(50)
        );
        assert_eq!(
            player_data_to_value(&pd, "clear"),
            rusqlite::types::Value::Integer(30)
        );
        assert_eq!(
            player_data_to_value(&pd, "epg"),
            rusqlite::types::Value::Integer(100)
        );
        assert_eq!(
            player_data_to_value(&pd, "lpg"),
            rusqlite::types::Value::Integer(90)
        );
        assert_eq!(
            player_data_to_value(&pd, "egr"),
            rusqlite::types::Value::Integer(80)
        );
        assert_eq!(
            player_data_to_value(&pd, "lgr"),
            rusqlite::types::Value::Integer(70)
        );
        assert_eq!(
            player_data_to_value(&pd, "egd"),
            rusqlite::types::Value::Integer(10)
        );
        assert_eq!(
            player_data_to_value(&pd, "lgd"),
            rusqlite::types::Value::Integer(9)
        );
        assert_eq!(
            player_data_to_value(&pd, "ebd"),
            rusqlite::types::Value::Integer(3)
        );
        assert_eq!(
            player_data_to_value(&pd, "lbd"),
            rusqlite::types::Value::Integer(2)
        );
        assert_eq!(
            player_data_to_value(&pd, "epr"),
            rusqlite::types::Value::Integer(1)
        );
        assert_eq!(
            player_data_to_value(&pd, "lpr"),
            rusqlite::types::Value::Integer(0)
        );
        assert_eq!(
            player_data_to_value(&pd, "ems"),
            rusqlite::types::Value::Integer(4)
        );
        assert_eq!(
            player_data_to_value(&pd, "lms"),
            rusqlite::types::Value::Integer(5)
        );
        assert_eq!(
            player_data_to_value(&pd, "playtime"),
            rusqlite::types::Value::Integer(7200)
        );
        assert_eq!(
            player_data_to_value(&pd, "maxcombo"),
            rusqlite::types::Value::Integer(500)
        );
    }

    #[test]
    fn test_player_data_to_value_unknown_column_returns_null() {
        let pd = PlayerData::default();

        assert_eq!(
            player_data_to_value(&pd, "nonexistent"),
            rusqlite::types::Value::Null
        );
        assert_eq!(player_data_to_value(&pd, ""), rusqlite::types::Value::Null);
    }

    // --- Roundtrip / integration tests using :memory: DB ---

    /// Helper: create an in-memory ScoreDatabaseAccessor with tables initialized.
    fn memory_accessor() -> ScoreDatabaseAccessor {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA shared_cache = ON").unwrap();
        conn.pragma_update(None, "synchronous", "OFF").unwrap();
        conn.pragma_update(None, "cache_size", 2000).unwrap();

        let tables = vec![
            Table::new(
                "info",
                vec![
                    Column::with_pk("id", "TEXT", 1, 1),
                    Column::with_pk("name", "TEXT", 1, 0),
                    Column::new("rank", "TEXT"),
                ],
            ),
            Table::new(
                "player",
                vec![
                    Column::with_pk("date", "INTEGER", 0, 1),
                    Column::new("playcount", "INTEGER"),
                    Column::new("clear", "INTEGER"),
                    Column::new("epg", "INTEGER"),
                    Column::new("lpg", "INTEGER"),
                    Column::new("egr", "INTEGER"),
                    Column::new("lgr", "INTEGER"),
                    Column::new("egd", "INTEGER"),
                    Column::new("lgd", "INTEGER"),
                    Column::new("ebd", "INTEGER"),
                    Column::new("lbd", "INTEGER"),
                    Column::new("epr", "INTEGER"),
                    Column::new("lpr", "INTEGER"),
                    Column::new("ems", "INTEGER"),
                    Column::new("lms", "INTEGER"),
                    Column::new("playtime", "INTEGER"),
                    Column::new("maxcombo", "INTEGER"),
                ],
            ),
            Table::new(
                "score",
                vec![
                    Column::with_pk("sha256", "TEXT", 1, 1),
                    Column::with_pk("mode", "INTEGER", 0, 1),
                    Column::new("clear", "INTEGER"),
                    Column::new("epg", "INTEGER"),
                    Column::new("lpg", "INTEGER"),
                    Column::new("egr", "INTEGER"),
                    Column::new("lgr", "INTEGER"),
                    Column::new("egd", "INTEGER"),
                    Column::new("lgd", "INTEGER"),
                    Column::new("ebd", "INTEGER"),
                    Column::new("lbd", "INTEGER"),
                    Column::new("epr", "INTEGER"),
                    Column::new("lpr", "INTEGER"),
                    Column::new("ems", "INTEGER"),
                    Column::new("lms", "INTEGER"),
                    Column::new("notes", "INTEGER"),
                    Column::new("combo", "INTEGER"),
                    Column::new("minbp", "INTEGER"),
                    Column::with_default("avgjudge", "INTEGER", 1, 0, &i32::MAX.to_string()),
                    Column::new("playcount", "INTEGER"),
                    Column::new("clearcount", "INTEGER"),
                    Column::new("trophy", "TEXT"),
                    Column::new("ghost", "TEXT"),
                    Column::new("option", "INTEGER"),
                    Column::new("seed", "INTEGER"),
                    Column::new("random", "INTEGER"),
                    Column::new("date", "INTEGER"),
                    Column::new("state", "INTEGER"),
                    Column::new("scorehash", "TEXT"),
                ],
            ),
        ];

        let base = SQLiteDatabaseAccessor::new(tables);
        let accessor = ScoreDatabaseAccessor { conn, base };
        accessor.base.validate(&accessor.conn).unwrap();
        accessor
    }

    /// Build a valid ScoreData (passes validate()) with given sha256, mode, clear.
    fn make_score(sha256: &str, mode: i32, clear: i32) -> ScoreData {
        let mut sd = ScoreData::default();
        sd.sha256 = sha256.to_string();
        sd.mode = mode;
        sd.clear = clear;
        sd.notes = 100;
        sd.passnotes = 100;
        sd.judge_counts.epg = 50;
        sd.judge_counts.lpg = 30;
        sd.judge_counts.egr = 10;
        sd.judge_counts.lgr = 5;
        sd.judge_counts.egd = 2;
        sd.judge_counts.lgd = 1;
        sd.judge_counts.ebd = 1;
        sd.judge_counts.lbd = 0;
        sd.judge_counts.epr = 1;
        sd.judge_counts.lpr = 0;
        sd.judge_counts.ems = 0;
        sd.judge_counts.lms = 0;
        sd.maxcombo = 80;
        sd.minbp = 5;
        sd.timing_stats.avgjudge = 10;
        sd.playcount = 3;
        sd.clearcount = 2;
        sd.trophy = "g".to_string();
        sd.ghost = String::new();
        sd.play_option.option = 0;
        sd.play_option.seed = 42;
        sd.play_option.random = 0;
        sd.date = 1700000000;
        sd.state = 0;
        sd.scorehash = "hash1".to_string();
        sd
    }

    #[test]
    fn test_score_data_roundtrip_via_memory_db() {
        let accessor = memory_accessor();

        let sd = make_score("abc123", 0, 5);
        accessor.set_score_data(&sd);

        let loaded = accessor.score_data("abc123", 0);
        assert!(loaded.is_some(), "score should be retrievable after insert");
        let loaded = loaded.unwrap();

        assert_eq!(loaded.sha256, "abc123");
        assert_eq!(loaded.mode, 0);
        assert_eq!(loaded.clear, 5);
        assert_eq!(loaded.judge_counts.epg, 50);
        assert_eq!(loaded.judge_counts.lpg, 30);
        assert_eq!(loaded.judge_counts.egr, 10);
        assert_eq!(loaded.judge_counts.lgr, 5);
        assert_eq!(loaded.judge_counts.egd, 2);
        assert_eq!(loaded.judge_counts.lgd, 1);
        assert_eq!(loaded.judge_counts.ebd, 1);
        assert_eq!(loaded.judge_counts.lbd, 0);
        assert_eq!(loaded.judge_counts.epr, 1);
        assert_eq!(loaded.judge_counts.lpr, 0);
        assert_eq!(loaded.judge_counts.ems, 0);
        assert_eq!(loaded.judge_counts.lms, 0);
        assert_eq!(loaded.notes, 100);
        assert_eq!(loaded.maxcombo, 80);
        assert_eq!(loaded.minbp, 5);
        assert_eq!(loaded.timing_stats.avgjudge, 10);
        assert_eq!(loaded.playcount, 3);
        assert_eq!(loaded.clearcount, 2);
        assert_eq!(loaded.trophy, "g");
        assert_eq!(loaded.play_option.option, 0);
        assert_eq!(loaded.play_option.seed, 42);
        assert_eq!(loaded.play_option.random, 0);
        assert_eq!(loaded.date, 1700000000);
        assert_eq!(loaded.state, 0);
        assert_eq!(loaded.scorehash, "hash1");
    }

    #[test]
    fn test_set_and_get_score_data() {
        let accessor = memory_accessor();

        // Write two scores with different sha256
        let sd1 = make_score("hash_aaa", 0, 3);
        let sd2 = make_score("hash_bbb", 0, 7);
        accessor.set_score_data(&sd1);
        accessor.set_score_data(&sd2);

        // Retrieve each independently
        let loaded1 = accessor.score_data("hash_aaa", 0).unwrap();
        assert_eq!(loaded1.sha256, "hash_aaa");
        assert_eq!(loaded1.clear, 3);

        let loaded2 = accessor.score_data("hash_bbb", 0).unwrap();
        assert_eq!(loaded2.sha256, "hash_bbb");
        assert_eq!(loaded2.clear, 7);

        // Non-existent hash returns None
        assert!(accessor.score_data("hash_zzz", 0).is_none());

        // Wrong mode returns None
        assert!(accessor.score_data("hash_aaa", 99).is_none());
    }

    #[test]
    fn test_get_score_data_picks_best_clear() {
        let accessor = memory_accessor();

        // Insert a score, then overwrite with a higher clear via INSERT OR REPLACE.
        // Since (sha256, mode) is the primary key, second insert replaces the first.
        let sd_low = make_score("hash_best", 0, 2);
        accessor.set_score_data(&sd_low);

        let mut sd_high = make_score("hash_best", 0, 8);
        sd_high.judge_counts.epg = 70;
        // Different mode so both exist
        sd_high.mode = 1;
        accessor.set_score_data(&sd_high);

        // mode=0 returns the original clear=2 score
        let loaded0 = accessor.score_data("hash_best", 0).unwrap();
        assert_eq!(loaded0.clear, 2);

        // mode=1 returns the clear=8 score
        let loaded1 = accessor.score_data("hash_best", 1).unwrap();
        assert_eq!(loaded1.clear, 8);
        assert_eq!(loaded1.judge_counts.epg, 70);
    }

    #[test]
    fn test_get_score_datas_for_songs_empty() {
        let accessor = memory_accessor();

        struct TestCollector {
            calls: Vec<(String, Option<i32>)>,
        }
        impl ScoreDataCollector for TestCollector {
            fn collect(&mut self, song: &SongData, score: Option<&ScoreData>) {
                self.calls
                    .push((song.file.sha256.clone(), score.map(|s| s.clear)));
            }
        }

        let mut collector = TestCollector { calls: vec![] };
        let songs: Vec<SongData> = vec![];
        accessor.score_datas_for_songs(&mut collector, &songs, 0);

        assert!(
            collector.calls.is_empty(),
            "empty songs list should produce no collector calls"
        );
    }

    #[test]
    fn test_delete_score_data() {
        let accessor = memory_accessor();

        let sd = make_score("hash_del", 0, 5);
        accessor.set_score_data(&sd);
        assert!(accessor.score_data("hash_del", 0).is_some());

        accessor.delete_score_data("hash_del", 0);
        assert!(
            accessor.score_data("hash_del", 0).is_none(),
            "score should be deleted"
        );
    }

    #[test]
    fn test_set_score_data_batch() {
        let accessor = memory_accessor();

        let sd1 = make_score("batch_1", 0, 3);
        let sd2 = make_score("batch_2", 0, 6);
        let sd3 = make_score("batch_3", 0, 9);
        accessor.set_score_data_batch(&[&sd1, &sd2, &sd3]);

        assert_eq!(accessor.score_data("batch_1", 0).unwrap().clear, 3);
        assert_eq!(accessor.score_data("batch_2", 0).unwrap().clear, 6);
        assert_eq!(accessor.score_data("batch_3", 0).unwrap().clear, 9);
    }

    #[test]
    fn test_get_score_datas_sql_filter() {
        let accessor = memory_accessor();

        let sd1 = make_score("sql_a", 0, 3);
        let mut sd2 = make_score("sql_b", 0, 8);
        sd2.playcount = 20;
        accessor.set_score_data(&sd1);
        accessor.set_score_data(&sd2);

        let results = accessor.score_datas("playcount >= 20").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].sha256, "sql_b");
    }
}

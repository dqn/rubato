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
        if self.get_player_datas(1).is_empty() {
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

    pub fn get_information(&self) -> Option<PlayerInformation> {
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
                    Some(info.into_iter().next().unwrap())
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

    pub fn get_score_data(&self, hash: &str, mode: i32) -> Option<ScoreData> {
        match self
            .conn
            .prepare(&format!(
                "SELECT * FROM score WHERE sha256 = '{}' AND mode = {}",
                hash, mode
            ))
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok(row_to_score_data(row)))
                    .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
            }) {
            Ok(scores) => {
                let scores: Vec<ScoreData> = scores
                    .into_iter()
                    .filter(|s| s.clone().validate())
                    .collect();
                if scores.is_empty() {
                    return None;
                }
                let mut best: Option<ScoreData> = None;
                for s in scores {
                    if best.is_none() || s.clear > best.as_ref().unwrap().clear {
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
    pub fn get_score_datas_for_songs(
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
        str_buf: &mut String,
        hasln: bool,
    ) {
        let result: Result<(), anyhow::Error> = (|| {
            let song_length = songs.len();
            let chunk_length = song_length.div_ceil(LOAD_CHUNK_SIZE);
            let mut scores: Vec<ScoreData> = Vec::new();

            for i in 0..chunk_length {
                let chunk_start = i * LOAD_CHUNK_SIZE;
                let chunk_end = std::cmp::min(song_length, (i + 1) * LOAD_CHUNK_SIZE);
                for j in chunk_start..chunk_end {
                    let song = &songs[j];
                    let has_uln = !song.sha256.is_empty(); // Simplified; real check needs hasUndefinedLongNote
                    if (hasln && has_uln) || (!hasln && !has_uln) {
                        if !str_buf.is_empty() {
                            str_buf.push(',');
                        }
                        str_buf.push('\'');
                        str_buf.push_str(&song.sha256);
                        str_buf.push('\'');
                    }
                }

                if !str_buf.is_empty() {
                    let sql = format!(
                        "SELECT * FROM score WHERE sha256 IN ({}) AND mode = {}",
                        str_buf, mode
                    );
                    let mut stmt = self.conn.prepare(&sql)?;
                    let sub_scores: Vec<ScoreData> = stmt
                        .query_map([], |row| Ok(row_to_score_data(row)))?
                        .filter_map(|r| r.ok())
                        .filter(|s| s.clone().validate())
                        .collect();
                    str_buf.clear();
                    scores.extend(sub_scores);
                }
            }

            for song in songs {
                let has_uln = !song.sha256.is_empty();
                if (hasln && has_uln) || (!hasln && !has_uln) {
                    let sha = song.sha256.as_str();
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

    pub fn get_score_datas(&self, sql: &str) -> Option<Vec<ScoreData>> {
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
                    .filter(|s| s.clone().validate())
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
        let result: anyhow::Result<()> = (|| {
            let tx = self.conn.unchecked_transaction()?;
            for (hash, values) in map {
                let mut vs = String::new();
                for (key, val) in values {
                    vs.push_str(&format!("{} = {},", key, val));
                }
                if !vs.is_empty() {
                    vs.truncate(vs.len() - 1);
                    vs.push(' ');
                    self.conn.execute(
                        &format!("UPDATE score SET {} WHERE sha256 = '{}'", vs, hash),
                        [],
                    )?;
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

    pub fn get_player_data(&self) -> Option<PlayerData> {
        let pds = self.get_player_datas(1);
        if !pds.is_empty() {
            Some(pds.into_iter().next().unwrap())
        } else {
            None
        }
    }

    pub fn get_player_datas(&self, count: i32) -> Vec<PlayerData> {
        let sql = if count > 0 {
            format!("SELECT * FROM player ORDER BY date DESC limit {}", count)
        } else {
            "SELECT * FROM player ORDER BY date DESC".to_string()
        };

        match self.conn.prepare(&sql).and_then(|mut stmt| {
            stmt.query_map([], |row| Ok(row_to_player_data(row)))
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

            // Calculate today's midnight unixtime
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            // Truncate to midnight (UTC)
            let unixtime = (now / 86400) * 86400;

            let mut pd_copy = pd.clone();
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

    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }
}

impl beatoraja_types::score_database_access::ScoreDatabaseAccess for ScoreDatabaseAccessor {
    fn create_table(&self) {
        ScoreDatabaseAccessor::create_table(self);
    }

    fn get_score_data(&self, sha256: &str, mode: i32) -> Option<ScoreData> {
        ScoreDatabaseAccessor::get_score_data(self, sha256, mode)
    }

    fn set_score_data_slice(&self, scores: &[ScoreData]) {
        let refs: Vec<&ScoreData> = scores.iter().collect();
        self.set_score_data_batch(&refs);
    }
}

#[allow(clippy::field_reassign_with_default)]
fn row_to_score_data(row: &rusqlite::Row) -> ScoreData {
    let mut sd = ScoreData::default();
    sd.sha256 = row.get::<_, String>("sha256").unwrap_or_default();
    sd.mode = row.get("mode").unwrap_or(0);
    sd.clear = row.get("clear").unwrap_or(0);
    sd.epg = row.get("epg").unwrap_or(0);
    sd.lpg = row.get("lpg").unwrap_or(0);
    sd.egr = row.get("egr").unwrap_or(0);
    sd.lgr = row.get("lgr").unwrap_or(0);
    sd.egd = row.get("egd").unwrap_or(0);
    sd.lgd = row.get("lgd").unwrap_or(0);
    sd.ebd = row.get("ebd").unwrap_or(0);
    sd.lbd = row.get("lbd").unwrap_or(0);
    sd.epr = row.get("epr").unwrap_or(0);
    sd.lpr = row.get("lpr").unwrap_or(0);
    sd.ems = row.get("ems").unwrap_or(0);
    sd.lms = row.get("lms").unwrap_or(0);
    sd.notes = row.get("notes").unwrap_or(0);
    sd.combo = row.get("combo").unwrap_or(0);
    sd.minbp = row.get("minbp").unwrap_or(i32::MAX);
    sd.avgjudge = row.get("avgjudge").unwrap_or(i64::MAX);
    sd.playcount = row.get("playcount").unwrap_or(0);
    sd.clearcount = row.get("clearcount").unwrap_or(0);
    sd.trophy = row.get::<_, String>("trophy").unwrap_or_default();
    sd.ghost = row.get::<_, String>("ghost").unwrap_or_default();
    sd.option = row.get("option").unwrap_or(0);
    sd.seed = row.get("seed").unwrap_or(-1);
    sd.random = row.get("random").unwrap_or(0);
    sd.date = row.get("date").unwrap_or(0);
    sd.state = row.get("state").unwrap_or(0);
    sd.scorehash = row.get::<_, String>("scorehash").unwrap_or_default();
    sd
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
        "epg" => rusqlite::types::Value::Integer(score.epg as i64),
        "lpg" => rusqlite::types::Value::Integer(score.lpg as i64),
        "egr" => rusqlite::types::Value::Integer(score.egr as i64),
        "lgr" => rusqlite::types::Value::Integer(score.lgr as i64),
        "egd" => rusqlite::types::Value::Integer(score.egd as i64),
        "lgd" => rusqlite::types::Value::Integer(score.lgd as i64),
        "ebd" => rusqlite::types::Value::Integer(score.ebd as i64),
        "lbd" => rusqlite::types::Value::Integer(score.lbd as i64),
        "epr" => rusqlite::types::Value::Integer(score.epr as i64),
        "lpr" => rusqlite::types::Value::Integer(score.lpr as i64),
        "ems" => rusqlite::types::Value::Integer(score.ems as i64),
        "lms" => rusqlite::types::Value::Integer(score.lms as i64),
        "notes" => rusqlite::types::Value::Integer(score.notes as i64),
        "combo" => rusqlite::types::Value::Integer(score.combo as i64),
        "minbp" => rusqlite::types::Value::Integer(score.minbp as i64),
        "avgjudge" => rusqlite::types::Value::Integer(score.avgjudge),
        "playcount" => rusqlite::types::Value::Integer(score.playcount as i64),
        "clearcount" => rusqlite::types::Value::Integer(score.clearcount as i64),
        "trophy" => rusqlite::types::Value::Text(score.trophy.clone()),
        "ghost" => rusqlite::types::Value::Text(score.ghost.clone()),
        "option" => rusqlite::types::Value::Integer(score.option as i64),
        "seed" => rusqlite::types::Value::Integer(score.seed),
        "random" => rusqlite::types::Value::Integer(score.random as i64),
        "date" => rusqlite::types::Value::Integer(score.date),
        "state" => rusqlite::types::Value::Integer(score.state as i64),
        "scorehash" => rusqlite::types::Value::Text(score.scorehash.clone()),
        _ => rusqlite::types::Value::Null,
    }
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

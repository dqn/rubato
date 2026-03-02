use std::sync::Mutex;

use beatoraja_core::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};
use beatoraja_core::validatable::remove_invalid_elements_vec;
use beatoraja_types::song_information_db::SongInformationDb;
use bms_model::bms_model::BMSModel;
use rusqlite::Connection;

use crate::song_data::SongData;
use crate::song_information::SongInformation;

const LOAD_CHUNK_SIZE: usize = 1000;

/// Song information database accessor
pub struct SongInformationAccessor {
    base: SQLiteDatabaseAccessor,
    conn: Mutex<Connection>,
}

impl SongInformationAccessor {
    pub fn new(filepath: &str) -> anyhow::Result<Self> {
        let base = SQLiteDatabaseAccessor::new(vec![Table::new(
            "information",
            vec![
                Column::with_pk("sha256", "TEXT", 1, 1),
                Column::new("n", "INTEGER"),
                Column::new("ln", "INTEGER"),
                Column::new("s", "INTEGER"),
                Column::new("ls", "INTEGER"),
                Column::new("total", "REAL"),
                Column::new("density", "REAL"),
                Column::new("peakdensity", "REAL"),
                Column::new("enddensity", "REAL"),
                Column::new("mainbpm", "REAL"),
                Column::new("distribution", "TEXT"),
                Column::new("speedchange", "TEXT"),
                Column::new("lanenotes", "TEXT"),
            ],
        )]);

        let conn = Connection::open(filepath)?;
        conn.execute_batch("PRAGMA shared_cache = ON; PRAGMA synchronous = OFF;")?;
        base.validate(&conn)?;

        Ok(Self {
            base,
            conn: Mutex::new(conn),
        })
    }

    pub fn get_informations(&self, sql: &str) -> Vec<SongInformation> {
        let query = format!("SELECT * FROM information WHERE {}", sql);
        match self.query_informations(&query, &[]) {
            Ok(infos) => remove_invalid_elements_vec(infos),
            Err(e) => {
                log::error!("Error querying informations: {}", e);
                Vec::new()
            }
        }
    }

    pub fn get_information(&self, sha256: &str) -> Option<SongInformation> {
        let query = "SELECT * FROM information WHERE sha256 = ?1";
        match self.query_informations(query, &[sha256]) {
            Ok(mut infos) => {
                let infos = remove_invalid_elements_vec(std::mem::take(&mut infos));
                infos.into_iter().next()
            }
            Err(e) => {
                log::error!("Error querying information: {}", e);
                None
            }
        }
    }

    pub fn get_information_for_songs(&self, songs: &mut [SongData]) {
        let song_length = songs.len();
        let chunk_length = song_length.div_ceil(LOAD_CHUNK_SIZE);
        let mut infos: Vec<SongInformation> = Vec::new();

        for i in 0..chunk_length {
            let chunk_start = i * LOAD_CHUNK_SIZE;
            let chunk_end = song_length.min((i + 1) * LOAD_CHUNK_SIZE);

            for song in songs.iter().take(chunk_end).skip(chunk_start) {
                let sha256 = song.sha256.clone();
                if sha256.is_empty() {
                    continue;
                }
                let query = "SELECT * FROM information WHERE sha256 = ?1";
                match self.query_informations(query, &[sha256.as_str()]) {
                    Ok(sub_infos) => {
                        let valid = remove_invalid_elements_vec(sub_infos);
                        infos.extend(valid);
                    }
                    Err(e) => {
                        log::error!("Error querying information for songs: {}", e);
                    }
                }
            }
        }

        for song in songs.iter_mut() {
            for info in &infos {
                if info.sha256 == song.sha256 {
                    song.info = Some(info.clone());
                    break;
                }
            }
        }
    }

    pub fn start_update(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("BEGIN TRANSACTION")?;
        Ok(())
    }

    pub fn update(&self, model: &BMSModel) {
        let info = SongInformation::from_model(model);
        if let Err(e) = self.insert_information(&info) {
            log::error!("Error inserting information: {}", e);
        }
    }

    pub fn end_update(&self) {
        let conn = self.conn.lock().unwrap();
        if let Err(e) = conn.execute_batch("COMMIT") {
            log::error!("Error committing update: {}", e);
        }
    }

    fn query_informations(
        &self,
        sql: &str,
        params: &[&str],
    ) -> anyhow::Result<Vec<SongInformation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let param_values: Vec<&dyn rusqlite::types::ToSql> = params
            .iter()
            .map(|p| p as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(param_values.as_slice(), |row| {
            let mut info = SongInformation::new();
            info.sha256 = row.get::<_, String>(0).unwrap_or_default();
            info.n = row.get::<_, i32>(1).unwrap_or(0);
            info.ln = row.get::<_, i32>(2).unwrap_or(0);
            info.s = row.get::<_, i32>(3).unwrap_or(0);
            info.ls = row.get::<_, i32>(4).unwrap_or(0);
            info.total = row.get::<_, f64>(5).unwrap_or(0.0);
            info.density = row.get::<_, f64>(6).unwrap_or(0.0);
            info.peakdensity = row.get::<_, f64>(7).unwrap_or(0.0);
            info.enddensity = row.get::<_, f64>(8).unwrap_or(0.0);
            info.mainbpm = row.get::<_, f64>(9).unwrap_or(0.0);
            let distribution: String = row.get::<_, String>(10).unwrap_or_default();
            let speedchange: String = row.get::<_, String>(11).unwrap_or_default();
            let lanenotes: String = row.get::<_, String>(12).unwrap_or_default();
            info.distribution = distribution;
            info.speedchange = speedchange;
            info.lanenotes = lanenotes;
            Ok(info)
        })?;
        let mut result = Vec::new();
        for mut info in rows.flatten() {
            // Parse encoded fields
            let distribution = info.distribution.clone();
            if !distribution.is_empty() {
                info.set_distribution(distribution);
            }
            let speedchange = info.speedchange.clone();
            if !speedchange.is_empty() {
                info.set_speedchange(speedchange);
            }
            let lanenotes = info.lanenotes.clone();
            if !lanenotes.is_empty() {
                info.set_lanenotes(lanenotes);
            }
            result.push(info);
        }
        Ok(result)
    }

    fn insert_information(&self, info: &SongInformation) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        self.base.insert_with_values(
            &conn,
            "information",
            &|name: &str| -> rusqlite::types::Value {
                match name {
                    "sha256" => rusqlite::types::Value::Text(info.sha256.clone()),
                    "n" => rusqlite::types::Value::Integer(info.n as i64),
                    "ln" => rusqlite::types::Value::Integer(info.ln as i64),
                    "s" => rusqlite::types::Value::Integer(info.s as i64),
                    "ls" => rusqlite::types::Value::Integer(info.ls as i64),
                    "total" => rusqlite::types::Value::Real(info.total),
                    "density" => rusqlite::types::Value::Real(info.density),
                    "peakdensity" => rusqlite::types::Value::Real(info.peakdensity),
                    "enddensity" => rusqlite::types::Value::Real(info.enddensity),
                    "mainbpm" => rusqlite::types::Value::Real(info.mainbpm),
                    "distribution" => rusqlite::types::Value::Text(info.distribution.clone()),
                    "speedchange" => rusqlite::types::Value::Text(info.speedchange.clone()),
                    "lanenotes" => rusqlite::types::Value::Text(info.lanenotes.clone()),
                    _ => rusqlite::types::Value::Null,
                }
            },
        )
    }
}

impl SongInformationDb for SongInformationAccessor {
    fn get_informations(&self, sql: &str) -> Vec<SongInformation> {
        self.get_informations(sql)
    }

    fn get_information(&self, sha256: &str) -> Option<SongInformation> {
        self.get_information(sha256)
    }

    fn get_information_for_songs(&self, songs: &mut [SongData]) {
        self.get_information_for_songs(songs)
    }

    fn start_update(&self) -> anyhow::Result<()> {
        self.start_update()
    }

    fn update(&self, model: &BMSModel) {
        self.update(model)
    }

    fn end_update(&self) {
        self.end_update()
    }
}

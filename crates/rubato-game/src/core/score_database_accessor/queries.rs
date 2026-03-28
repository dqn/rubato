use std::collections::HashMap;

use rubato_types::player_data::PlayerData;
use rubato_types::player_information::PlayerInformation;
use rubato_types::score_data::ScoreData;
use rubato_types::validatable::Validatable;

use super::helpers::{row_to_player_data, row_to_score_data};
use super::{LOAD_CHUNK_SIZE, ScoreDataCollector, ScoreDatabaseAccessor, SongData};

impl ScoreDatabaseAccessor {
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
                    if best.as_ref().is_none_or(|b| s.clear > b.clear) {
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

    fn get_score_datas_inner(
        &self,
        collector: &mut dyn ScoreDataCollector,
        songs: &[SongData],
        mode: i32,
        _str_buf: &mut String,
        hasln: bool,
    ) {
        let result: Result<(), anyhow::Error> = (|| {
            let mut scores: Vec<ScoreData> = Vec::new();

            for chunk in songs.chunks(LOAD_CHUNK_SIZE) {
                let mut chunk_hashes: Vec<String> = Vec::new();
                for song in chunk {
                    let has_uln = song.chart.has_undefined_long_note();
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

            let score_map: HashMap<&str, &ScoreData> =
                scores.iter().map(|s| (s.sha256.as_str(), s)).collect();

            for song in songs {
                let has_uln = song.chart.has_undefined_long_note();
                if (hasln && has_uln) || (!hasln && !has_uln) {
                    let sha = song.file.sha256.as_str();
                    collector.collect(song, score_map.get(sha).copied());
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            log::error!("Exception getting scores: {}", e);
        }
    }

    // SQL injection note: `sql` is a raw WHERE clause from local folder config JSON files.
    // This is a local-only desktop app -- the user who can write those files already has full
    // local access. Parameterization would require a significant refactor of the folder filter
    // system. Same pattern as the Java original (beatoraja).
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
                "SELECT * FROM player ORDER BY date DESC, rowid DESC LIMIT ?1",
                vec![Box::new(count) as Box<dyn rusqlite::types::ToSql>],
            )
        } else {
            (
                "SELECT * FROM player ORDER BY date DESC, rowid DESC",
                vec![],
            )
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
}

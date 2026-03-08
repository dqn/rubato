use std::collections::HashMap;

use crate::stubs::{ScoreData, ScoreDatabaseAccessor, SongDatabaseAccessor};

/// Score data importer.
/// Translated from Java: ScoreDataImporter
pub struct ScoreDataImporter {
    scoredb: ScoreDatabaseAccessor,
}

impl ScoreDataImporter {
    pub fn new(scoredb: ScoreDatabaseAccessor) -> Self {
        Self { scoredb }
    }

    pub fn import_from_lr2_score_database(&self, path: &str, songdb: &dyn SongDatabaseAccessor) {
        let clears: [i32; 7] = [0, 1, 4, 5, 6, 8, 9];
        self.scoredb.create_table();

        match Self::read_lr2_scores(path) {
            Ok(scores) => {
                let mut result: Vec<ScoreData> = Vec::new();
                for score in &scores {
                    let md5 = score
                        .get("hash")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let song = songdb.song_datas_by_hashes(std::slice::from_ref(&md5));
                    if !song.is_empty() {
                        let clear_idx =
                            score.get("clear").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                        let mut sd = ScoreData::default();
                        sd.judge_counts.epg =
                            score.get("perfect").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.judge_counts.egr =
                            score.get("great").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.judge_counts.egd =
                            score.get("good").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.judge_counts.ebd =
                            score.get("bad").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.judge_counts.epr =
                            score.get("poor").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.minbp = score.get("minbp").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.clear = if clear_idx < clears.len() {
                            clears[clear_idx]
                        } else {
                            0
                        };
                        sd.playcount =
                            score.get("playcount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.clearcount = score
                            .get("clearcount")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32;
                        sd.sha256 = song[0].file.sha256.clone();
                        sd.notes = song[0].chart.notes;
                        result.push(sd);
                    }
                }

                self.import_scores(&result, "LR2");
            }
            Err(e) => {
                log::error!("Score import exception: {}", e);
            }
        }
    }

    pub fn import_scores(&self, scores: &[ScoreData], scorehash: &str) {
        let mut result: Vec<ScoreData> = Vec::new();

        for score in scores {
            let mut oldsd = self.scoredb.score_data(&score.sha256, score.mode);
            if oldsd.is_none() {
                oldsd = Some(ScoreData {
                    playcount: score.playcount,
                    clearcount: score.clearcount,
                    sha256: score.sha256.clone(),
                    mode: score.mode,
                    notes: score.notes,
                    ..Default::default()
                });
            }
            if let Some(ref mut old) = oldsd {
                old.scorehash = scorehash.to_string();
                if old.update(score, true) {
                    result.push(old.clone());
                }
            }
        }

        let score_refs: Vec<&ScoreData> = result.iter().collect();
        self.scoredb.set_score_data_batch(&score_refs);
        log::info!("Score import complete - imported count: {}", result.len());
    }

    fn read_lr2_scores(path: &str) -> anyhow::Result<Vec<HashMap<String, serde_json::Value>>> {
        let conn = rusqlite::Connection::open(path)?;
        let mut stmt = conn.prepare("SELECT * FROM score")?;
        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).expect("column name").to_string())
            .collect();
        let rows = stmt.query_map([], |row| {
            let mut map = HashMap::new();
            for (i, name) in column_names.iter().enumerate() {
                let value: rusqlite::types::Value = row.get(i)?;
                let json_value = match value {
                    rusqlite::types::Value::Null => serde_json::Value::Null,
                    rusqlite::types::Value::Integer(n) => serde_json::Value::Number(n.into()),
                    rusqlite::types::Value::Real(f) => serde_json::json!(f),
                    rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
                    rusqlite::types::Value::Blob(b) => {
                        serde_json::Value::String(format!("{:?}", b))
                    }
                };
                map.insert(name.clone(), json_value);
            }
            Ok(map)
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
}

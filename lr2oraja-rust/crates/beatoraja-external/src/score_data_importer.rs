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

    pub fn import_from_lr2_score_database(&self, path: &str, songdb: &SongDatabaseAccessor) {
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
                    let song = songdb.get_song_datas(&[&md5]);
                    if !song.is_empty() {
                        let mut sd = ScoreData::default();
                        sd.epg =
                            score.get("perfect").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.egr =
                            score.get("great").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.egd =
                            score.get("good").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.ebd =
                            score.get("bad").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.epr =
                            score.get("poor").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.minbp =
                            score.get("minbp").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let clear_idx =
                            score.get("clear").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                        if clear_idx < clears.len() {
                            sd.clear = clears[clear_idx];
                        }
                        sd.playcount =
                            score.get("playcount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        sd.clearcount =
                            score
                                .get("clearcount")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i32;
                        sd.sha256 = song[0].get_sha256().to_string();
                        sd.notes = song[0].get_notes();
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
            let mut oldsd = self
                .scoredb
                .get_score_data(score.get_sha256(), score.get_mode());
            if oldsd.is_none() {
                let mut new_sd = ScoreData::default();
                new_sd.playcount = score.get_playcount();
                new_sd.clearcount = score.get_clearcount();
                new_sd.sha256 = score.get_sha256().to_string();
                new_sd.mode = score.get_mode();
                new_sd.notes = score.get_notes();
                oldsd = Some(new_sd);
            }
            if let Some(ref mut old) = oldsd {
                old.scorehash = scorehash.to_string();
                if old.update(score, true) {
                    result.push(old.clone());
                }
            }
        }

        self.scoredb.set_score_data(&result);
        log::info!("Score import complete - imported count: {}", result.len());
    }

    fn read_lr2_scores(_path: &str) -> anyhow::Result<Vec<HashMap<String, serde_json::Value>>> {
        // In Java this uses JDBC to connect to SQLite and reads the "score" table.
        // In Rust, use rusqlite to connect to the LR2 score database.
        todo!("LR2 score database reading via rusqlite")
    }
}

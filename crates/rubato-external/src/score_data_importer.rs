use std::collections::HashMap;

use crate::{ScoreData, ScoreDatabaseAccessor, SongDatabaseAccessor};

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
        if let Err(e) = self.scoredb.create_table() {
            log::error!("Failed to create score table: {e}");
        }

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
                        let clear_idx = score
                            .get("clear")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0)
                            .max(0) as usize;
                        let mut sd = ScoreData::default();
                        sd.judge_counts.epg = Self::clamp_nonneg_i64_to_i32(
                            score.get("perfect").and_then(|v| v.as_i64()).unwrap_or(0),
                        );
                        sd.judge_counts.egr = Self::clamp_nonneg_i64_to_i32(
                            score.get("great").and_then(|v| v.as_i64()).unwrap_or(0),
                        );
                        sd.judge_counts.egd = Self::clamp_nonneg_i64_to_i32(
                            score.get("good").and_then(|v| v.as_i64()).unwrap_or(0),
                        );
                        sd.judge_counts.ebd = Self::clamp_nonneg_i64_to_i32(
                            score.get("bad").and_then(|v| v.as_i64()).unwrap_or(0),
                        );
                        sd.judge_counts.epr = Self::clamp_nonneg_i64_to_i32(
                            score.get("poor").and_then(|v| v.as_i64()).unwrap_or(0),
                        );
                        sd.minbp = Self::clamp_nonneg_i64_to_i32(
                            score.get("minbp").and_then(|v| v.as_i64()).unwrap_or(0),
                        );
                        sd.clear = if clear_idx < clears.len() {
                            clears[clear_idx]
                        } else {
                            0
                        };
                        sd.playcount = Self::clamp_nonneg_i64_to_i32(
                            score.get("playcount").and_then(|v| v.as_i64()).unwrap_or(0),
                        );
                        sd.clearcount = Self::clamp_nonneg_i64_to_i32(
                            score
                                .get("clearcount")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0),
                        );
                        sd.sha256 = song[0].file.sha256.clone();
                        sd.notes = song[0].chart.notes;
                        // LR2 had no LN mode concept. For songs with undefined LN,
                        // import the score under all LN modes (0/1/2) so it is visible
                        // regardless of the user's current lnmode setting.
                        if song[0].chart.has_undefined_long_note() {
                            for lnmode in 0..3 {
                                let mut sd_ln = sd.clone();
                                sd_ln.mode = lnmode;
                                result.push(sd_ln);
                            }
                        } else {
                            result.push(sd);
                        }
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
            let existing = self.scoredb.score_data(&score.sha256, score.mode);
            let is_new = existing.is_none();
            let mut old = existing.unwrap_or_else(|| ScoreData {
                playcount: score.playcount,
                clearcount: score.clearcount,
                sha256: score.sha256.clone(),
                mode: score.mode,
                notes: score.notes,
                ..Default::default()
            });
            old.scorehash = scorehash.to_string();
            if is_new {
                old.update(score, true);
                result.push(old);
            } else {
                // Always accumulate imported play/clear counts for existing scores
                old.playcount = old.playcount.max(score.playcount);
                old.clearcount = old.clearcount.max(score.clearcount);
                if old.update(score, true) {
                    result.push(old);
                } else if score.playcount > 0 || score.clearcount > 0 {
                    // Even if score metrics didn't improve, persist updated counters
                    result.push(old);
                }
            }
        }

        let score_refs: Vec<&ScoreData> = result.iter().collect();
        self.scoredb.set_score_data_batch(&score_refs);
        log::info!("Score import complete - imported count: {}", result.len());
    }

    /// Clamp an i64 value from external data to i32 range, preventing silent wrapping.
    fn clamp_nonneg_i64_to_i32(val: i64) -> i32 {
        val.clamp(0, i32::MAX as i64) as i32
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_nonneg_i64_to_i32_prevents_wrapping_on_overflow() {
        // Regression: values exceeding i32::MAX were cast with `as i32`,
        // silently wrapping to negative numbers.
        let overflow_val: i64 = i32::MAX as i64 + 1;
        let result = ScoreDataImporter::clamp_nonneg_i64_to_i32(overflow_val);
        assert_eq!(
            result,
            i32::MAX,
            "i64 value {} should clamp to i32::MAX ({}), not wrap to {}",
            overflow_val,
            i32::MAX,
            overflow_val as i32
        );
    }

    #[test]
    fn clamp_nonneg_i64_to_i32_preserves_normal_values() {
        assert_eq!(ScoreDataImporter::clamp_nonneg_i64_to_i32(0), 0);
        assert_eq!(ScoreDataImporter::clamp_nonneg_i64_to_i32(100), 100);
        assert_eq!(
            ScoreDataImporter::clamp_nonneg_i64_to_i32(i32::MAX as i64),
            i32::MAX
        );
    }

    #[test]
    fn clamp_nonneg_i64_to_i32_clamps_negative_to_zero() {
        assert_eq!(ScoreDataImporter::clamp_nonneg_i64_to_i32(-1), 0);
        assert_eq!(ScoreDataImporter::clamp_nonneg_i64_to_i32(i64::MIN), 0);
    }

    #[test]
    fn clamp_nonneg_i64_to_i32_clamps_large_positive() {
        assert_eq!(
            ScoreDataImporter::clamp_nonneg_i64_to_i32(i64::MAX),
            i32::MAX
        );
        assert_eq!(
            ScoreDataImporter::clamp_nonneg_i64_to_i32(5_000_000_000),
            i32::MAX
        );
    }
}

use std::collections::HashMap;

use rubato_types::player_data::PlayerData;
use rubato_types::player_information::PlayerInformation;
use rubato_types::score_data::ScoreData;

use super::ScoreDatabaseAccessor;
use super::helpers::{local_midnight_timestamp, player_data_to_value, score_data_to_value};

impl ScoreDatabaseAccessor {
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
        // Whitelist must match the actual score table columns defined in
        // ScoreDatabaseAccessor::new(). Phantom columns that don't exist in
        // the schema would cause silent UPDATE failures (no rows matched).
        const VALID_SCORE_COLUMNS: &[&str] = &[
            "sha256",
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
            "notes",
            "combo",
            "minbp",
            "avgjudge",
            "ghost",
            "option",
            "seed",
            "random",
            "state",
            "scorehash",
            // Java parity: trophy string may accumulate duplicate characters across repeated plays.
            // Consider deduplicating before write if string growth becomes an issue.
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
                    debug_assert!(
                        !key.contains('[') && !key.contains(']'),
                        "bracket in whitelisted column name would break SQL escaping: {}",
                        key
                    );
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
}

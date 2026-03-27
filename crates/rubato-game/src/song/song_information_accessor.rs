use std::sync::Mutex;

use crate::core::validatable::remove_invalid_elements_vec;
use bms::model::bms_model::BMSModel;
use rubato_db::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};
use rubato_types::song_information_db::SongInformationDb;
use rubato_types::sync_utils::lock_or_recover;
use rusqlite::Connection;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

use crate::song::song_data::SongData;
use crate::song::song_information::SongInformation;

/// SQLite authorizer callback that only allows read-only operations.
/// Used to guard queries that interpolate untrusted SQL.
fn read_only_authorizer(ctx: AuthContext<'_>) -> Authorization {
    match ctx.action {
        AuthAction::Select
        | AuthAction::Read { .. }
        | AuthAction::Function { .. }
        | AuthAction::Recursive => Authorization::Allow,
        _ => Authorization::Deny,
    }
}

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
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA shared_cache = ON; PRAGMA synchronous = NORMAL;",
        )?;
        base.validate(&conn)?;

        Ok(Self {
            base,
            conn: Mutex::new(conn),
        })
    }

    /// # Safety contract
    /// `sql` is injected directly into a WHERE clause. All current callers pass hardcoded
    /// string literals. A `read_only_authorizer` prevents writes/DDL but does not prevent
    /// expensive sub-selects. Do not pass user-supplied input without parameterization.
    pub fn informations(&self, sql: &str) -> Vec<SongInformation> {
        let query = format!("SELECT * FROM information WHERE {}", sql);
        // Hold the lock for the entire authorizer lifecycle to prevent
        // another thread from seeing the read-only authorizer during
        // concurrent write operations.
        let conn = lock_or_recover(&self.conn);
        conn.authorizer(Some(read_only_authorizer));
        let result = match Self::query_informations_on_conn(&conn, &query, &[]) {
            Ok(infos) => remove_invalid_elements_vec(infos),
            Err(e) => {
                log::error!("Error querying informations: {}", e);
                Vec::new()
            }
        };
        conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        result
    }

    pub fn information(&self, sha256: &str) -> Option<SongInformation> {
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

    /// Performance note: issues one SQL query per song (O(N) round trips) and
    /// O(N*M) matching. Java parity. For large libraries, consider batching with
    /// `WHERE sha256 IN (...)` and a HashMap-based join.
    pub fn information_for_songs(&self, songs: &mut [SongData]) {
        let mut infos: Vec<SongInformation> = Vec::new();

        for chunk in songs.chunks(LOAD_CHUNK_SIZE) {
            for song in chunk {
                let sha256 = song.file.sha256.clone();
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
                if info.sha256 == song.file.sha256 {
                    song.info = Some(info.clone());
                    break;
                }
            }
        }
    }

    pub fn start_update(&self) -> anyhow::Result<()> {
        let conn = lock_or_recover(&self.conn);
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
        let conn = lock_or_recover(&self.conn);
        if let Err(e) = conn.execute_batch("COMMIT") {
            log::error!("Error committing update: {}", e);
        }
    }

    fn query_informations(
        &self,
        sql: &str,
        params: &[&str],
    ) -> anyhow::Result<Vec<SongInformation>> {
        let conn = lock_or_recover(&self.conn);
        Self::query_informations_on_conn(&conn, sql, params)
    }

    fn query_informations_on_conn(
        conn: &Connection,
        sql: &str,
        params: &[&str],
    ) -> anyhow::Result<Vec<SongInformation>> {
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
        let conn = lock_or_recover(&self.conn);
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
    fn informations(&self, sql: &str) -> Vec<SongInformation> {
        self.informations(sql)
    }

    fn information(&self, sha256: &str) -> Option<SongInformation> {
        self.information(sha256)
    }

    fn information_for_songs(&self, songs: &mut [SongData]) {
        self.information_for_songs(songs)
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

#[cfg(test)]
mod tests {
    use super::*;

    // 64-char hex string to pass SongInformation::validate() sha256 length check
    const TEST_SHA256: &str = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";

    /// Helper: create a SongInformationAccessor with a test row.
    fn setup_info_accessor() -> (SongInformationAccessor, tempfile::TempDir) {
        let tmpdir = tempfile::tempdir().unwrap();
        let db_path = tmpdir.path().join("info.db");
        let accessor = SongInformationAccessor::new(&db_path.to_string_lossy()).unwrap();
        // Insert a test row
        let conn = lock_or_recover(&accessor.conn);
        conn.execute(
            &format!(
                "INSERT INTO information (sha256, n, ln, s, ls, total, density, peakdensity, enddensity, mainbpm, distribution, speedchange, lanenotes) \
                 VALUES ('{}', 100, 10, 5, 2, 200.0, 5.0, 10.0, 3.0, 150.0, '', '', '')",
                TEST_SHA256
            ),
            [],
        ).unwrap();
        drop(conn);
        (accessor, tmpdir)
    }

    /// Legitimate WHERE clauses must still work through the read-only authorizer.
    #[test]
    fn informations_allows_read_queries() {
        let (accessor, _tmpdir) = setup_info_accessor();

        let results = accessor.informations("n = 100");
        assert_eq!(results.len(), 1, "n = 100 should match one info");

        let results = accessor.informations("1=1");
        assert_eq!(results.len(), 1, "1=1 should match all infos");

        let results = accessor.informations("n = 999");
        assert!(results.is_empty(), "n = 999 should match nothing");
    }

    /// Verify the authorizer is properly removed after the query so
    /// subsequent operations work correctly.
    #[test]
    fn informations_authorizer_cleanup() {
        let (accessor, _tmpdir) = setup_info_accessor();

        // First call installs and removes the authorizer
        let results = accessor.informations("1=1");
        assert_eq!(results.len(), 1);

        // Second call should also work (authorizer was properly removed)
        let results = accessor.informations("n = 100");
        assert_eq!(
            results.len(),
            1,
            "second query should work after authorizer cleanup"
        );
    }

    /// After informations() returns, the authorizer must be fully cleared so
    /// that write operations on the same connection succeed immediately.
    /// This is a regression test for a TOCTOU race where the authorizer was
    /// set and cleared in separate lock/unlock cycles.
    #[test]
    fn informations_authorizer_not_leaked_to_subsequent_writes() {
        let (accessor, _tmpdir) = setup_info_accessor();

        // Run a read-only query through informations()
        let results = accessor.informations("1=1");
        assert_eq!(results.len(), 1);

        // Immediately after, a write operation on the same connection must succeed.
        // If the authorizer leaked, this INSERT would be denied.
        let sha2 = "b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3";
        let conn = lock_or_recover(&accessor.conn);
        let result = conn.execute(
            &format!(
                "INSERT INTO information (sha256, n, ln, s, ls, total, density, peakdensity, enddensity, mainbpm, distribution, speedchange, lanenotes) \
                 VALUES ('{}', 50, 5, 2, 1, 100.0, 2.0, 4.0, 1.0, 130.0, '', '', '')",
                sha2
            ),
            [],
        );
        assert!(
            result.is_ok(),
            "INSERT after informations() must succeed; authorizer was not cleared: {:?}",
            result.err()
        );
    }

    /// Verify that interleaving informations() with insert_information()
    /// from another thread does not cause the authorizer to block writes.
    /// Regression test for TOCTOU where the authorizer was set/cleared
    /// in separate lock cycles, allowing concurrent threads to see it.
    #[test]
    fn informations_does_not_block_concurrent_writes() {
        let (accessor, _tmpdir) = setup_info_accessor();
        let accessor = std::sync::Arc::new(accessor);

        let accessor_clone = std::sync::Arc::clone(&accessor);
        let write_handle = std::thread::spawn(move || {
            // Attempt many writes while the other thread is calling informations()
            for i in 0..50 {
                let sha = format!("{:064x}", 0xc0ffee_u64 + i);
                let info = SongInformation {
                    sha256: sha,
                    n: i as i32,
                    ..SongInformation::new()
                };
                // This should never fail due to the authorizer being set
                if let Err(e) = accessor_clone.insert_information(&info) {
                    panic!("insert_information failed on iteration {}: {}", i, e);
                }
            }
        });

        // Run many reads concurrently
        for _ in 0..50 {
            let _ = accessor.informations("1=1");
        }

        write_handle.join().expect("writer thread panicked");
    }

    /// Verify that busy_timeout is set on the connection so that concurrent
    /// writers retry instead of immediately failing with SQLITE_BUSY.
    #[test]
    fn connection_has_busy_timeout() {
        let (accessor, _tmpdir) = setup_info_accessor();
        let conn = lock_or_recover(&accessor.conn);
        let timeout: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        assert!(
            timeout >= 5000,
            "busy_timeout should be at least 5000ms, got {}",
            timeout
        );
    }

    /// Verify that WAL journal mode and synchronous = NORMAL are set,
    /// matching the safety level of SQLiteSongDatabaseAccessor.
    #[test]
    fn connection_has_wal_and_synchronous_normal() {
        let (accessor, _tmpdir) = setup_info_accessor();
        let conn = lock_or_recover(&accessor.conn);

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            journal_mode, "wal",
            "journal_mode should be WAL for crash safety, got {}",
            journal_mode
        );

        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        // synchronous = NORMAL is 1
        assert_eq!(
            synchronous, 1,
            "synchronous should be NORMAL (1), got {}",
            synchronous
        );
    }

    /// The read-only authorizer blocks destructive operations when set on the
    /// information connection. This tests the authorizer directly.
    #[test]
    fn informations_authorizer_blocks_destructive_ops() {
        let (accessor, _tmpdir) = setup_info_accessor();

        let conn = lock_or_recover(&accessor.conn);
        conn.authorizer(Some(read_only_authorizer));

        // SELECT should succeed
        let count: i64 = conn
            .query_row("SELECT count(*) FROM information", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "SELECT should work with read-only authorizer");

        // INSERT should be blocked
        let result = conn.execute(
            "INSERT INTO information (sha256) VALUES ('evil_sha256')",
            [],
        );
        assert!(result.is_err(), "INSERT should be denied by authorizer");

        // DELETE should be blocked
        let result = conn.execute("DELETE FROM information", []);
        assert!(result.is_err(), "DELETE should be denied by authorizer");

        // DROP TABLE should be blocked
        let result = conn.execute_batch("DROP TABLE information");
        assert!(result.is_err(), "DROP TABLE should be denied by authorizer");

        conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);

        // Verify data is intact
        let count: i64 = conn
            .query_row("SELECT count(*) FROM information", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "data should be intact after blocked operations");
    }
}

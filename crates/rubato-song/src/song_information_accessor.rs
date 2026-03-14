use std::sync::Mutex;

use bms_model::bms_model::BMSModel;
use rubato_core::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};
use rubato_core::validatable::remove_invalid_elements_vec;
use rubato_types::song_information_db::SongInformationDb;
use rusqlite::Connection;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

use crate::song_data::SongData;
use crate::song_information::SongInformation;

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
        conn.execute_batch("PRAGMA shared_cache = ON; PRAGMA synchronous = OFF;")?;
        base.validate(&conn)?;

        Ok(Self {
            base,
            conn: Mutex::new(conn),
        })
    }

    pub fn informations(&self, sql: &str) -> Vec<SongInformation> {
        let query = format!("SELECT * FROM information WHERE {}", sql);
        // Guard untrusted SQL with read-only authorizer
        let conn = self.conn.lock().expect("conn lock poisoned");
        conn.authorizer(Some(read_only_authorizer));
        drop(conn);
        let result = match self.query_informations(&query, &[]) {
            Ok(infos) => remove_invalid_elements_vec(infos),
            Err(e) => {
                log::error!("Error querying informations: {}", e);
                Vec::new()
            }
        };
        let conn = self.conn.lock().expect("conn lock poisoned");
        conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        drop(conn);
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
        let conn = self.conn.lock().expect("conn lock poisoned");
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
        let conn = self.conn.lock().expect("conn lock poisoned");
        if let Err(e) = conn.execute_batch("COMMIT") {
            log::error!("Error committing update: {}", e);
        }
    }

    fn query_informations(
        &self,
        sql: &str,
        params: &[&str],
    ) -> anyhow::Result<Vec<SongInformation>> {
        let conn = self.conn.lock().expect("conn lock poisoned");
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
        let conn = self.conn.lock().expect("conn lock poisoned");
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
        let conn = accessor.conn.lock().expect("conn lock poisoned");
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

    /// The read-only authorizer blocks destructive operations when set on the
    /// information connection. This tests the authorizer directly.
    #[test]
    fn informations_authorizer_blocks_destructive_ops() {
        let (accessor, _tmpdir) = setup_info_accessor();

        let conn = accessor.conn.lock().expect("conn lock poisoned");
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

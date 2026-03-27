// Shared test helpers for beatoraja-core integration tests.

use std::path::Path;

use rubato_game::core::score_database_accessor::ScoreDatabaseAccessor;

/// Create a ScoreDatabaseAccessor backed by a real SQLite file inside `dir`.
/// Schema tables are created automatically via `create_table()`.
pub fn open_score_db(dir: &Path) -> ScoreDatabaseAccessor {
    let db_path = dir.join("score.db");
    let accessor =
        ScoreDatabaseAccessor::new(db_path.to_str().expect("temp path should be valid UTF-8"))
            .expect("ScoreDatabaseAccessor::new should succeed on a fresh file");
    accessor.create_table().expect("create table");
    accessor
}

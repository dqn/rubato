// SQL injection tests for ScoreDatabaseAccessor.
//
// These tests demonstrate that format!-based SQL construction in
// score_database_accessor.rs is vulnerable to injection.  The tests are
// "red-only": they expose the bug but do NOT fix the SQL.
//
// Vulnerable call sites:
//   - get_score_datas(sql) at line 287: raw SQL fragment in WHERE clause
//   - set_score_data_map(map) at lines 330-343: hash interpolated via format!

mod helpers;

use std::collections::HashMap;

use beatoraja_core::score_data::ScoreData;

/// Build a minimal ScoreData that passes `Validatable::validate()`.
///
/// Key requirements from validate():
///   - notes > 0
///   - playcount >= clearcount (both default to 0, OK)
///   - passnotes <= notes  (passnotes defaults to 0, OK)
///   - minbp >= 0  (default is i32::MAX, OK)
///   - avgjudge >= 0  (default is i64::MAX, OK)
fn make_score(sha256: &str, mode: i32, clear: i32) -> ScoreData {
    ScoreData {
        sha256: sha256.to_string(),
        mode,
        clear,
        notes: 100,
        ..Default::default()
    }
}

// -----------------------------------------------------------------------
// 47a — get_score_datas("1=1") returns ALL rows (WHERE clause injection)
// -----------------------------------------------------------------------

#[test]
fn get_score_datas_sql_injection_returns_all_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db = helpers::open_score_db(dir.path());

    // Insert two rows with distinct hashes.
    let score_a = make_score("aaaa", 0, 5);
    let score_b = make_score("bbbb", 0, 7);
    db.set_score_data(&score_a);
    db.set_score_data(&score_b);

    // Legitimate use: filter by a specific hash.
    let legit = db.get_score_datas("sha256 = 'aaaa'").unwrap();
    assert_eq!(
        legit.len(),
        1,
        "legitimate query should return exactly 1 row"
    );

    // Injection: "1=1" makes the WHERE clause always true, returning every row.
    let injected = db
        .get_score_datas("1=1")
        .expect("get_score_datas should succeed with injected SQL");
    assert_eq!(
        injected.len(),
        2,
        "SQL injection via '1=1' should bypass the intended filter and return all rows"
    );
}

// -----------------------------------------------------------------------
// 47b — set_score_data_map with injected hash modifies wrong rows
// -----------------------------------------------------------------------

#[test]
fn set_score_data_map_injection_modifies_wrong_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db = helpers::open_score_db(dir.path());

    // Insert a victim row.
    let victim = make_score("victim_hash", 0, 3);
    db.set_score_data(&victim);

    // The attacker hash is crafted so that the generated SQL:
    //   UPDATE score SET clear = 9 WHERE sha256 = 'x' OR sha256 = 'victim_hash' --'
    // evaluates to an always-matching condition for the victim row.
    let injected_hash = "x' OR sha256 = 'victim_hash' --";

    let mut values: HashMap<String, String> = HashMap::new();
    values.insert("clear".to_string(), "9".to_string());

    let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
    map.insert(injected_hash.to_string(), values);

    // This should only affect the row with sha256 = injected_hash (which doesn't
    // exist), but due to SQL injection it will update the victim row.
    db.set_score_data_map(&map);

    let restored = db
        .get_score_data("victim_hash", 0)
        .expect("victim row should still exist");

    // If the injection worked, the victim's clear was changed from 3 to 9.
    assert_eq!(
        restored.clear, 9,
        "SQL injection in set_score_data_map should have overwritten the victim row's clear value"
    );
}

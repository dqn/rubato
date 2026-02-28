//! Smoke tests for bms-table crate: zero-coverage API surface.
//!
//! These tests verify that core data types can be constructed and accessed
//! without panicking.

use bms_table::bms_table_element::BmsTableElement;
use bms_table::course::{Course, Trophy};
use bms_table::difficulty_table::DifficultyTable;
use bms_table::difficulty_table_element::DifficultyTableElement;

// ---------------------------------------------------------------------------
// DifficultyTable
// ---------------------------------------------------------------------------

#[test]
fn difficulty_table_default() {
    let dt = DifficultyTable::default();

    assert!(dt.get_elements().is_empty());
    assert!(dt.get_level_description().is_empty());
    assert!(dt.get_course().is_empty());
    assert!(dt.table.get_name().is_none());
    assert!(dt.table.get_id().is_none());
    assert!(dt.table.get_tag().is_none());
    assert!(dt.table.get_data_url().is_empty());
    assert!(dt.table.get_models().is_empty());
    assert!(!dt.table.is_editable());
    assert!(dt.table.is_auto_update());
    assert_eq!(dt.table.get_lastupdate(), 0);
    assert_eq!(dt.table.get_access_count(), 0);
}

#[test]
fn difficulty_table_with_source_url_and_elements() {
    let mut dt = DifficultyTable::new_with_source_url("https://example.com/table.html");

    assert_eq!(dt.table.get_source_url(), "https://example.com/table.html");

    // Add an element
    let mut elem = DifficultyTableElement::new_with_params(
        "12",
        "Test Song",
        42,
        "https://example.com/dl",
        "https://example.com/diff",
        "hard chart",
        "abc123",
        "",
    );
    elem.set_state(1);
    elem.set_evaluation(5);

    dt.table.add_element(elem);

    let elements = dt.get_elements();
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].get_level(), "12");
    assert_eq!(elements[0].element.get_title(), Some("Test Song"));
    assert_eq!(elements[0].get_bmsid(), 42);
    assert_eq!(elements[0].get_state(), 1);
    assert_eq!(elements[0].get_evaluation(), 5);
    assert_eq!(elements[0].get_comment(), "hard chart");
}

// ---------------------------------------------------------------------------
// BmsTableElement
// ---------------------------------------------------------------------------

#[test]
fn bms_table_element_hash_fields() {
    let mut elem = BmsTableElement::new();
    elem.set_md5("d41d8cd98f00b204e9800998ecf8427e");
    elem.set_sha256("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    elem.set_title("Test Title");
    elem.set_artist("Test Artist");
    elem.set_mode("beat-7k");

    assert_eq!(elem.get_md5(), Some("d41d8cd98f00b204e9800998ecf8427e"));
    assert_eq!(
        elem.get_sha256(),
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );
    assert_eq!(elem.get_title(), Some("Test Title"));
    assert_eq!(elem.get_artist(), Some("Test Artist"));
    assert_eq!(elem.get_mode(), Some("beat-7k"));
    // Parent hash is None by default
    assert!(elem.get_parent_hash().is_none());
}

#[test]
fn bms_table_element_parent_hash_roundtrip() {
    let mut elem = BmsTableElement::new();

    // Set multiple parent hashes
    let hashes = vec!["hash1".to_string(), "hash2".to_string()];
    elem.set_parent_hash(Some(&hashes));
    assert_eq!(
        elem.get_parent_hash(),
        Some(vec!["hash1".to_string(), "hash2".to_string()])
    );

    // Clear parent hashes
    elem.set_parent_hash(None);
    assert!(elem.get_parent_hash().is_none());
}

// ---------------------------------------------------------------------------
// Course and Trophy
// ---------------------------------------------------------------------------

#[test]
fn course_construction() {
    let mut course = Course::new();
    // Default name is Japanese "新規段位"
    assert!(!course.get_name().is_empty());

    course.set_name("Dan Course A");
    assert_eq!(course.get_name(), "Dan Course A");

    course.set_style("7KEYS");
    assert_eq!(course.get_style(), "7KEYS");

    course.set_constraint(vec!["GAUGE_LR2".to_string()]);
    assert_eq!(course.get_constraint(), &["GAUGE_LR2".to_string()]);

    // Charts
    let mut chart = BmsTableElement::new();
    chart.set_md5("abc123");
    course.set_charts(vec![chart]);
    assert_eq!(course.get_charts().len(), 1);
}

#[test]
fn trophy_construction() {
    let mut trophy = Trophy::new();
    // Default name is Japanese "新規トロフィー"
    assert!(!trophy.get_name().is_empty());

    trophy.set_name("Gold Trophy");
    trophy.set_style("gold");
    trophy.set_scorerate(90.0);
    trophy.set_missrate(5.0);

    assert_eq!(trophy.get_name(), "Gold Trophy");
    assert_eq!(trophy.get_style(), "gold");
    assert!((trophy.get_scorerate() - 90.0).abs() < f64::EPSILON);
    assert!((trophy.get_missrate() - 5.0).abs() < f64::EPSILON);
}

// Integration test: CourseData parse -> validate pipeline
//
// Tests CourseData creation, constraint checking, validation,
// and serialization round-trip as an end-to-end pipeline.

use beatoraja_types::course_data::{CourseData, CourseDataConstraint, TrophyData};
use beatoraja_types::song_data::SongData;
use beatoraja_types::validatable::Validatable;

/// Helper: create a valid SongData with the given title and md5 hash.
fn make_song(title: &str, md5: &str) -> SongData {
    let mut song = SongData::new();
    song.title = title.to_string();
    song.md5 = md5.to_string();
    song
}

// ---------------------------------------------------------------------------
// CourseData creation and constraint checking
// ---------------------------------------------------------------------------

#[test]
fn course_data_with_class_constraint_is_class_course() {
    let mut cd = CourseData::default();
    cd.set_name("Dan Course".to_string());
    cd.set_song(vec![
        make_song("Stage 1", "aaa111"),
        make_song("Stage 2", "bbb222"),
        make_song("Stage 3", "ccc333"),
        make_song("Stage 4", "ddd444"),
    ]);
    cd.set_constraint(vec![
        CourseDataConstraint::Class,
        CourseDataConstraint::NoSpeed,
        CourseDataConstraint::Gauge7Keys,
    ]);

    assert!(
        cd.is_class_course(),
        "Course with Class constraint should be a class course"
    );
}

#[test]
fn course_data_with_mirror_constraint_is_class_course() {
    let mut cd = CourseData::default();
    cd.set_name("Mirror Dan".to_string());
    cd.set_song(vec![make_song("Stage 1", "aaa111")]);
    cd.set_constraint(vec![CourseDataConstraint::Mirror]);

    assert!(
        cd.is_class_course(),
        "Course with Mirror constraint should be a class course"
    );
}

#[test]
fn course_data_with_random_constraint_is_class_course() {
    let mut cd = CourseData::default();
    cd.set_name("Random Dan".to_string());
    cd.set_song(vec![make_song("Stage 1", "aaa111")]);
    cd.set_constraint(vec![CourseDataConstraint::Random]);

    assert!(
        cd.is_class_course(),
        "Course with Random constraint should be a class course"
    );
}

#[test]
fn course_data_without_class_mirror_random_is_not_class_course() {
    let mut cd = CourseData::default();
    cd.set_name("Casual Course".to_string());
    cd.set_song(vec![make_song("Stage 1", "aaa111")]);
    cd.set_constraint(vec![
        CourseDataConstraint::NoSpeed,
        CourseDataConstraint::NoGood,
        CourseDataConstraint::Ln,
    ]);

    assert!(
        !cd.is_class_course(),
        "Course without Class/Mirror/Random should not be a class course"
    );
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn validate_empty_songs_returns_false() {
    let mut cd = CourseData::default();
    cd.set_name("Empty Course".to_string());
    // No songs
    assert!(
        !cd.validate(),
        "Course with no songs should fail validation"
    );
}

#[test]
fn validate_assigns_default_name_when_missing() {
    let mut cd = CourseData::default();
    // name is None by default
    cd.set_song(vec![make_song("Stage 1", "aaa111")]);
    assert!(cd.validate());
    assert_eq!(
        cd.get_name(),
        "No Course Title",
        "Validation should assign default name when none is set"
    );
}

#[test]
fn validate_assigns_default_name_when_empty() {
    let mut cd = CourseData::default();
    cd.set_name(String::new());
    cd.set_song(vec![make_song("Stage 1", "aaa111")]);
    assert!(cd.validate());
    assert_eq!(cd.get_name(), "No Course Title");
}

#[test]
fn validate_assigns_default_titles_to_untitled_songs() {
    let mut cd = CourseData::default();
    cd.set_name("My Course".to_string());

    // Create songs with empty titles but valid hashes
    let mut song1 = SongData::new();
    song1.md5 = "hash1".to_string();
    // title is empty

    let mut song2 = SongData::new();
    song2.md5 = "hash2".to_string();
    // title is empty

    cd.set_song(vec![song1, song2]);
    assert!(cd.validate());

    // Songs should have been assigned default titles
    assert_eq!(cd.get_song()[0].title, "course 1");
    assert_eq!(cd.get_song()[1].title, "course 2");
}

#[test]
fn validate_deduplicates_constraints_by_type() {
    let mut cd = CourseData::default();
    cd.set_name("Dedup Course".to_string());
    cd.set_song(vec![make_song("Stage 1", "aaa111")]);

    // Add multiple constraints of the same type (type 0: Class, Mirror, Random)
    cd.set_constraint(vec![
        CourseDataConstraint::Class,
        CourseDataConstraint::Mirror,     // same type as Class (type 0)
        CourseDataConstraint::NoSpeed,    // type 1
        CourseDataConstraint::NoGood,     // type 2
        CourseDataConstraint::NoGreat,    // same type as NoGood (type 2)
        CourseDataConstraint::Gauge7Keys, // type 3
        CourseDataConstraint::Ln,         // type 4
        CourseDataConstraint::Cn,         // same type as Ln (type 4)
    ]);

    assert!(cd.validate());

    // After validation, only one constraint per type should remain
    let constraints = cd.get_constraint();
    assert_eq!(
        constraints.len(),
        5,
        "Should have exactly 5 constraints (one per type)"
    );

    // First of each type wins
    assert_eq!(constraints[0], CourseDataConstraint::Class); // type 0
    assert_eq!(constraints[1], CourseDataConstraint::NoSpeed); // type 1
    assert_eq!(constraints[2], CourseDataConstraint::NoGood); // type 2
    assert_eq!(constraints[3], CourseDataConstraint::Gauge7Keys); // type 3
    assert_eq!(constraints[4], CourseDataConstraint::Ln); // type 4
}

#[test]
fn validate_removes_invalid_trophies() {
    let mut cd = CourseData::default();
    cd.set_name("Trophy Course".to_string());
    cd.set_song(vec![make_song("Stage 1", "aaa111")]);

    // Mix valid and invalid trophies
    cd.set_trophy(vec![
        TrophyData::new("Gold".to_string(), 5.0, 90.0), // valid
        TrophyData::new("Bad".to_string(), 0.0, 90.0),  // invalid: missrate <= 0
        TrophyData::new("Silver".to_string(), 10.0, 80.0), // valid
        TrophyData::new("Overflow".to_string(), 5.0, 100.0), // invalid: scorerate >= 100
    ]);

    assert!(cd.validate());

    let trophies = cd.get_trophy();
    assert_eq!(trophies.len(), 2, "Should keep only 2 valid trophies");
    assert_eq!(trophies[0].get_name(), "Gold");
    assert_eq!(trophies[1].get_name(), "Silver");
}

#[test]
fn validate_fails_for_song_without_hash() {
    let mut cd = CourseData::default();
    cd.set_name("Invalid Song Course".to_string());

    // Song with title but no md5/sha256
    let mut invalid_song = SongData::new();
    invalid_song.title = "No Hash".to_string();
    // md5 and sha256 are both empty

    cd.set_song(vec![invalid_song]);
    assert!(
        !cd.validate(),
        "Course with unhashable song should fail validation"
    );
}

// ---------------------------------------------------------------------------
// Serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn course_data_full_serde_roundtrip() {
    let mut cd = CourseData::default();
    cd.set_name("Full Course".to_string());
    cd.set_release(false);
    cd.set_song(vec![
        make_song("Stage 1", "hash_a"),
        make_song("Stage 2", "hash_b"),
        make_song("Stage 3", "hash_c"),
    ]);
    cd.set_constraint(vec![
        CourseDataConstraint::Class,
        CourseDataConstraint::NoSpeed,
        CourseDataConstraint::Gauge7Keys,
        CourseDataConstraint::Ln,
    ]);
    cd.set_trophy(vec![
        TrophyData::new("Gold".to_string(), 3.0, 95.0),
        TrophyData::new("Silver".to_string(), 10.0, 85.0),
    ]);

    let json = serde_json::to_string_pretty(&cd).expect("Serialization should succeed");
    let restored: CourseData = serde_json::from_str(&json).expect("Deserialization should succeed");

    // Verify name
    assert_eq!(restored.get_name(), "Full Course");

    // Verify release
    assert!(!restored.release);

    // Verify songs
    assert_eq!(restored.get_song().len(), 3);
    assert_eq!(restored.get_song()[0].title, "Stage 1");
    assert_eq!(restored.get_song()[0].md5, "hash_a");
    assert_eq!(restored.get_song()[1].title, "Stage 2");
    assert_eq!(restored.get_song()[2].title, "Stage 3");

    // Verify constraints
    assert_eq!(restored.get_constraint().len(), 4);
    assert_eq!(restored.get_constraint()[0], CourseDataConstraint::Class);
    assert_eq!(restored.get_constraint()[1], CourseDataConstraint::NoSpeed);
    assert_eq!(
        restored.get_constraint()[2],
        CourseDataConstraint::Gauge7Keys
    );
    assert_eq!(restored.get_constraint()[3], CourseDataConstraint::Ln);

    // Verify trophies
    assert_eq!(restored.get_trophy().len(), 2);
    assert_eq!(restored.get_trophy()[0].get_name(), "Gold");
    assert_eq!(restored.get_trophy()[0].get_missrate(), 3.0);
    assert_eq!(restored.get_trophy()[0].get_scorerate(), 95.0);
    assert_eq!(restored.get_trophy()[1].get_name(), "Silver");
}

#[test]
fn course_data_empty_serde_roundtrip() {
    let cd = CourseData::default();

    let json = serde_json::to_string(&cd).unwrap();
    let restored: CourseData = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.get_name(), "");
    assert!(restored.get_song().is_empty());
    assert!(restored.get_constraint().is_empty());
    assert!(restored.get_trophy().is_empty());
    assert!(restored.release);
}

// ---------------------------------------------------------------------------
// CourseDataConstraint enumeration
// ---------------------------------------------------------------------------

#[test]
fn all_constraint_names_roundtrip_via_get_value() {
    for constraint in CourseDataConstraint::values() {
        let name = constraint.name_str();
        let resolved = CourseDataConstraint::get_value(name);
        assert_eq!(
            resolved,
            Some(*constraint),
            "Constraint {:?} with name '{}' should round-trip through get_value",
            constraint,
            name
        );
    }
}

#[test]
fn constraint_types_are_valid_range() {
    for constraint in CourseDataConstraint::values() {
        let ct = constraint.constraint_type();
        assert!(
            (0..5).contains(&ct),
            "Constraint {:?} has type {} which is outside valid range 0..5",
            constraint,
            ct
        );
    }
}

// ---------------------------------------------------------------------------
// TrophyData validation
// ---------------------------------------------------------------------------

#[test]
fn trophy_data_serde_roundtrip() {
    let trophy = TrophyData::new("Platinum".to_string(), 2.5, 97.5);

    let json = serde_json::to_string(&trophy).unwrap();
    let restored: TrophyData = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.get_name(), "Platinum");
    assert_eq!(restored.get_missrate(), 2.5);
    assert_eq!(restored.get_scorerate(), 97.5);
}

#[test]
fn trophy_data_validate_edge_cases() {
    // Barely valid
    let mut trophy = TrophyData::new("Edge".to_string(), 0.001, 99.999);
    assert!(trophy.validate(), "Barely valid trophy should pass");

    // Exactly zero missrate is invalid
    let mut trophy = TrophyData::new("Zero Miss".to_string(), 0.0, 50.0);
    assert!(!trophy.validate(), "Zero missrate should fail");

    // Negative missrate
    let mut trophy = TrophyData::new("Neg Miss".to_string(), -1.0, 50.0);
    assert!(!trophy.validate(), "Negative missrate should fail");

    // Exactly 100 scorerate is invalid
    let mut trophy = TrophyData::new("Full Score".to_string(), 5.0, 100.0);
    assert!(!trophy.validate(), "Scorerate of exactly 100 should fail");

    // Scorerate over 100
    let mut trophy = TrophyData::new("Over Score".to_string(), 5.0, 101.0);
    assert!(!trophy.validate(), "Scorerate over 100 should fail");
}

// ---------------------------------------------------------------------------
// End-to-end pipeline: create -> validate -> serialize -> deserialize -> verify
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_course_pipeline() {
    // Step 1: Create a course with realistic data
    let mut cd = CourseData::default();
    cd.set_name("10th Dan".to_string());
    cd.set_release(true);
    cd.set_song(vec![
        make_song("FREEDOM DiVE", "abc123def456"),
        make_song("Blue Zenith", "789012abc345"),
        make_song("Yomi yori", "def678ghi901"),
        make_song("XTREME", "jkl234mno567"),
    ]);
    cd.set_constraint(vec![
        CourseDataConstraint::Class,
        CourseDataConstraint::NoSpeed,
        CourseDataConstraint::Gauge7Keys,
    ]);
    cd.set_trophy(vec![
        TrophyData::new("Clear".to_string(), 50.0, 10.0),
        TrophyData::new("Hard Clear".to_string(), 20.0, 50.0),
        TrophyData::new("Full Combo".to_string(), 1.0, 90.0),
    ]);

    // Step 2: Validate
    assert!(cd.validate(), "Well-formed course should pass validation");
    assert!(cd.is_class_course(), "Class constraint means class course");

    // Step 3: Serialize
    let json = serde_json::to_string_pretty(&cd).unwrap();

    // Step 4: Deserialize
    let restored: CourseData = serde_json::from_str(&json).unwrap();

    // Step 5: Verify restored data
    assert_eq!(restored.get_name(), "10th Dan");
    assert!(restored.release);
    assert_eq!(restored.get_song().len(), 4);
    assert_eq!(restored.get_song()[0].title, "FREEDOM DiVE");
    assert_eq!(restored.get_song()[3].md5, "jkl234mno567");
    assert!(restored.is_class_course());

    // Constraints should have been deduplicated to one per type
    assert_eq!(restored.get_constraint().len(), 3);

    // Trophies should all be valid
    assert_eq!(restored.get_trophy().len(), 3);
    assert_eq!(restored.get_trophy()[2].get_name(), "Full Combo");
}

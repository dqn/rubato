// Golden master tests: compare Rust CourseData parse + validate against Java fixture export.

use std::path::Path;

use golden_master::course_data_fixtures::{CourseDataFixture, CourseDataTestCase};
use rubato_types::course_data::{CourseData, CourseDataConstraint};
use rubato_types::validatable::Validatable;

fn fixtures_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .leak()
}

fn test_bms_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-bms")
        .leak()
}

fn load_fixture() -> CourseDataFixture {
    let path = fixtures_dir().join("course_data.json");
    assert!(
        path.exists(),
        "Course data fixture not found: {}. Run `just golden-master-course-data-gen` first.",
        path.display()
    );
    let content = std::fs::read_to_string(&path).expect("Failed to read fixture");
    serde_json::from_str(&content).expect("Failed to parse fixture")
}

fn find_test_case<'a>(fixture: &'a CourseDataFixture, source_file: &str) -> &'a CourseDataTestCase {
    fixture
        .test_cases
        .iter()
        .find(|tc| tc.source_file == source_file)
        .unwrap_or_else(|| panic!("Test case not found for {source_file}"))
}

fn compare_course_data(
    rust_valid: bool,
    rust: &CourseData,
    java: &CourseDataTestCase,
) -> Vec<String> {
    let mut diffs = Vec::new();

    if rust_valid != java.valid {
        diffs.push(format!("valid: rust={} java={}", rust_valid, java.valid));
    }

    // Only compare fields if validation passed in both
    if !rust_valid && !java.valid {
        return diffs;
    }

    // name is Option<String> in Rust, String in fixture
    let rust_name = rust.name();
    if rust_name != java.name {
        diffs.push(format!("name: rust={:?} java={:?}", rust_name, java.name));
    }

    if rust.release != java.release {
        diffs.push(format!(
            "release: rust={} java={}",
            rust.release, java.release
        ));
    }

    if rust.is_class_course() != java.is_class_course {
        diffs.push(format!(
            "is_class_course: rust={} java={}",
            rust.is_class_course(),
            java.is_class_course
        ));
    }

    // Compare songs
    if rust.hash.len() != java.hash.len() {
        diffs.push(format!(
            "hash.len: rust={} java={}",
            rust.hash.len(),
            java.hash.len()
        ));
    } else {
        for (i, (rs, js)) in rust.hash.iter().zip(java.hash.iter()).enumerate() {
            if rs.file.sha256 != js.sha256 {
                diffs.push(format!(
                    "hash[{}].sha256: rust={:?} java={:?}",
                    i, rs.file.sha256, js.sha256
                ));
            }
            if rs.file.md5 != js.md5 {
                diffs.push(format!(
                    "hash[{}].md5: rust={:?} java={:?}",
                    i, rs.file.md5, js.md5
                ));
            }
            if rs.metadata.title != js.title {
                diffs.push(format!(
                    "hash[{}].title: rust={:?} java={:?}",
                    i, rs.metadata.title, js.title
                ));
            }
        }
    }

    // Compare constraints
    let rust_constraint_names: Vec<String> =
        rust.constraint.iter().map(constraint_to_string).collect();
    if rust_constraint_names.len() != java.constraint.len() {
        diffs.push(format!(
            "constraint.len: rust={} java={}",
            rust_constraint_names.len(),
            java.constraint.len()
        ));
    } else {
        for (i, (rc, jc)) in rust_constraint_names
            .iter()
            .zip(java.constraint.iter())
            .enumerate()
        {
            if rc != jc {
                diffs.push(format!("constraint[{}]: rust={:?} java={:?}", i, rc, jc));
            }
        }
    }

    // Compare trophies
    if rust.trophy.len() != java.trophy.len() {
        diffs.push(format!(
            "trophy.len: rust={} java={}",
            rust.trophy.len(),
            java.trophy.len()
        ));
    } else {
        for (i, (rt, jt)) in rust.trophy.iter().zip(java.trophy.iter()).enumerate() {
            // TrophyData.name is Option<String>
            let rt_name = rt.name();
            if rt_name != jt.name {
                diffs.push(format!(
                    "trophy[{}].name: rust={:?} java={:?}",
                    i, rt_name, jt.name
                ));
            }
            if (rt.missrate - jt.missrate).abs() > 0.001 {
                diffs.push(format!(
                    "trophy[{}].missrate: rust={} java={}",
                    i, rt.missrate, jt.missrate
                ));
            }
            if (rt.scorerate - jt.scorerate).abs() > 0.001 {
                diffs.push(format!(
                    "trophy[{}].scorerate: rust={} java={}",
                    i, rt.scorerate, jt.scorerate
                ));
            }
        }
    }

    diffs
}

fn constraint_to_string(c: &CourseDataConstraint) -> String {
    // Match the Java CourseDataConstraint.name field using name_str()
    c.name_str().to_string()
}

fn run_course_data_test(source_file: &str) {
    let fixture = load_fixture();
    let test_case = find_test_case(&fixture, source_file);

    let course_path = test_bms_dir().join("courses").join(source_file);
    assert!(
        course_path.exists(),
        "Course file not found: {}",
        course_path.display()
    );

    let content = std::fs::read_to_string(&course_path).expect("Failed to read course file");
    let mut course: CourseData =
        serde_json::from_str(&content).expect("Failed to parse course JSON");
    let valid = course.validate();

    let diffs = compare_course_data(valid, &course, test_case);
    if !diffs.is_empty() {
        panic!(
            "CourseData mismatch for {} ({} differences):\n{}",
            source_file,
            diffs.len(),
            diffs
                .iter()
                .map(|d| format!("  - {d}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[test]
fn course_data_basic() {
    run_course_data_test("basic_course.json");
}

#[test]
fn course_data_class() {
    run_course_data_test("class_course.json");
}

#[test]
fn course_data_complex() {
    run_course_data_test("complex_course.json");
}

#[test]
fn course_data_edge_case() {
    run_course_data_test("edge_case_course.json");
}

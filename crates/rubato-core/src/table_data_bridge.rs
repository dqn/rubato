//! Bridge between bms-table crate types and beatoraja-core TableData.
//!
//! Translates: TableDataAccessor.toSongData (Java)
//! Converts BmsTableElement / DifficultyTableElement → SongData,
//! and DifficultyTable → TableData.

use bms_model::mode::Mode;
use bms_table::bms_table_element::BmsTableElement;
use bms_table::course::{Course, Trophy};
use bms_table::difficulty_table::DifficultyTable;
use bms_table::difficulty_table_element::DifficultyTableElement;

use crate::stubs::SongData;
use crate::table_data::{TableData, TableFolder};
use rubato_types::course_data::{CourseData, CourseDataConstraint, TrophyData};

/// Convert a BmsTableElement to SongData.
///
/// Translated from Java: TableDataAccessor.toSongData(BMSTableElement, Mode)
pub fn bms_table_element_to_song_data(
    te: &BmsTableElement,
    default_mode: Option<&Mode>,
) -> SongData {
    let mut song = SongData::new();

    if let Some(md5) = te.md5() {
        song.md5 = md5.to_lowercase();
    }
    if let Some(sha256) = te.sha256() {
        song.sha256 = sha256.to_lowercase();
    }
    if let Some(title) = te.title() {
        song.title = title.to_string();
    }
    if let Some(artist) = te.artist() {
        song.set_artist(artist.to_string());
    }

    // Resolve mode: element mode takes precedence, then default_mode
    let element_mode = te.mode().and_then(Mode::from_hint);
    let mode_id = element_mode
        .as_ref()
        .map(|m| m.id())
        .or_else(|| default_mode.map(|m| m.id()))
        .unwrap_or(0);
    song.mode = mode_id;

    if let Some(url) = te.url() {
        song.set_url(url.to_string());
    }
    if let Some(ipfs) = te.ipfs() {
        song.ipfs = Some(ipfs.to_string());
    }
    if let Some(parent_hash) = te.parent_hash() {
        song.org_md5 = Some(parent_hash);
    }

    song
}

/// Convert a DifficultyTableElement to SongData.
///
/// Extends bms_table_element_to_song_data with DifficultyTableElement-specific fields
/// (appendurl, appendipfs).
pub fn difficulty_table_element_to_song_data(
    dte: &DifficultyTableElement,
    default_mode: Option<&Mode>,
) -> SongData {
    let mut song = bms_table_element_to_song_data(&dte.element, default_mode);

    if let Some(append_url) = dte.append_url() {
        song.set_appendurl(append_url.to_string());
    }
    if let Some(append_ipfs) = dte.append_ipfs() {
        song.appendipfs = Some(append_ipfs.to_string());
    }

    song
}

/// Convert a bms-table Course to a beatoraja-types CourseData.
fn course_to_course_data(course: &Course, default_mode: Option<&Mode>) -> CourseData {
    let mut cd = CourseData::default();
    cd.set_name(course.name().to_string());

    let songs: Vec<SongData> = course
        .charts()
        .iter()
        .map(|chart| bms_table_element_to_song_data(chart, default_mode))
        .collect();
    cd.hash = songs;

    let constraints: Vec<CourseDataConstraint> = course
        .constraint()
        .iter()
        .filter_map(|c| CourseDataConstraint::value(c))
        .collect();
    cd.constraint = constraints;

    if !course.get_trophy().is_empty() {
        let trophies: Vec<TrophyData> = course
            .get_trophy()
            .iter()
            .map(trophy_to_trophy_data)
            .collect();
        cd.trophy = trophies;
    }

    cd
}

/// Convert a bms-table Trophy to a beatoraja-types TrophyData.
fn trophy_to_trophy_data(trophy: &Trophy) -> TrophyData {
    let mut td = TrophyData::default();
    td.set_name(trophy.name().to_string());
    td.missrate = trophy.get_missrate() as f32;
    td.scorerate = trophy.scorerate() as f32;
    td
}

/// Convert a DifficultyTable to a TableData.
///
/// Translated from Java: DifficultyTableAccessor.read()
/// Creates TableData with folders (one per level) and courses.
pub fn difficulty_table_to_table_data(dt: &DifficultyTable, url: &str) -> TableData {
    let default_mode = dt.table.mode().and_then(Mode::from_hint);

    let tag = dt
        .table
        .get_tag()
        .unwrap_or_else(|| dt.table.id().unwrap_or("").to_string());

    let folders: Vec<TableFolder> = dt
        .level_description()
        .iter()
        .map(|lv| {
            let folder_name = format!("{}{}", tag, lv);
            let songs: Vec<SongData> = dt
                .elements()
                .iter()
                .filter(|dte| dte.get_level() == lv)
                .map(|dte| difficulty_table_element_to_song_data(dte, default_mode.as_ref()))
                .collect();
            TableFolder {
                name: Some(folder_name),
                songs,
            }
        })
        .collect();

    let courses: Vec<CourseData> = dt
        .course()
        .iter()
        .flat_map(|course_list| course_list.iter())
        .map(|g| course_to_course_data(g, default_mode.as_ref()))
        .collect();

    TableData {
        url: url.to_string(),
        name: dt.table.name().unwrap_or("").to_string(),
        tag,
        folder: folders,
        course: courses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // bms_table_element_to_song_data tests
    // ========================================

    #[test]
    fn test_bms_table_element_to_song_data_basic() {
        let mut te = BmsTableElement::new();
        te.set_md5("ABC123DEF456");
        te.set_sha256("DEADBEEF1234");
        te.set_title("Test Song");
        te.set_artist("Test Artist");
        te.set_url("https://example.com/download");
        te.set_ipfs("QmTestHash");

        let song = bms_table_element_to_song_data(&te, None);

        // MD5 and SHA256 should be lowercased
        assert_eq!(song.md5, "abc123def456");
        assert_eq!(song.sha256, "deadbeef1234");
        assert_eq!(song.title, "Test Song");
        assert_eq!(song.artist, "Test Artist");
        assert_eq!(song.url(), "https://example.com/download");
        assert_eq!(song.get_ipfs_str(), "QmTestHash");
        assert_eq!(song.mode, 0); // no mode set
    }

    #[test]
    fn test_bms_table_element_to_song_data_with_default_mode() {
        let te = BmsTableElement::new();

        let song = bms_table_element_to_song_data(&te, Some(&Mode::BEAT_7K));

        assert_eq!(song.mode, 7);
    }

    #[test]
    fn test_bms_table_element_to_song_data_element_mode_overrides_default() {
        let mut te = BmsTableElement::new();
        te.set_mode("beat-5k");

        let song = bms_table_element_to_song_data(&te, Some(&Mode::BEAT_7K));

        assert_eq!(song.mode, 5); // element mode wins
    }

    #[test]
    fn test_bms_table_element_to_song_data_empty() {
        let te = BmsTableElement::new();
        let song = bms_table_element_to_song_data(&te, None);

        assert_eq!(song.md5, "");
        assert_eq!(song.sha256, "");
        assert_eq!(song.title, "");
        assert_eq!(song.artist, "");
        assert_eq!(song.url(), "");
        assert_eq!(song.mode, 0);
    }

    #[test]
    fn test_bms_table_element_to_song_data_parent_hash() {
        let mut te = BmsTableElement::new();
        te.set_parent_hash(Some(&["hash1".to_string(), "hash2".to_string()]));

        let song = bms_table_element_to_song_data(&te, None);

        assert_eq!(song.org_md5_vec(), &["hash1", "hash2"]);
    }

    // ========================================
    // difficulty_table_element_to_song_data tests
    // ========================================

    #[test]
    fn test_difficulty_table_element_to_song_data_with_append_fields() {
        let mut dte = DifficultyTableElement::new();
        dte.element.set_md5("abc123");
        dte.element.set_title("DTE Song");
        dte.set_append_url("https://example.com/diff");
        dte.set_append_ipfs("QmDiffHash");

        let song = difficulty_table_element_to_song_data(&dte, None);

        assert_eq!(song.md5, "abc123");
        assert_eq!(song.title, "DTE Song");
        assert_eq!(song.appendurl(), "https://example.com/diff");
        assert_eq!(song.append_ipfs_str(), "QmDiffHash");
    }

    #[test]
    fn test_difficulty_table_element_to_song_data_without_append() {
        let mut dte = DifficultyTableElement::new();
        dte.element.set_md5("def456");

        let song = difficulty_table_element_to_song_data(&dte, None);

        assert_eq!(song.md5, "def456");
        assert_eq!(song.appendurl(), "");
        assert_eq!(song.append_ipfs_str(), "");
    }

    // ========================================
    // difficulty_table_to_table_data tests
    // ========================================

    #[test]
    fn test_difficulty_table_to_table_data_basic() {
        let mut dt = DifficultyTable::new();
        dt.table.set_name("Normal Table");
        dt.table.set_id("N");
        dt.set_level_description(&["1".to_string(), "2".to_string()]);

        // Add elements at level "1"
        let mut dte1 = DifficultyTableElement::new();
        dte1.element.set_md5("hash_a");
        dte1.element.set_title("Song A");
        dte1.set_level(Some("1"));
        dt.table.add_element(dte1);

        // Add elements at level "2"
        let mut dte2 = DifficultyTableElement::new();
        dte2.element.set_md5("hash_b");
        dte2.element.set_title("Song B");
        dte2.set_level(Some("2"));
        dt.table.add_element(dte2);

        let td = difficulty_table_to_table_data(&dt, "https://example.com/table");

        assert_eq!(td.name(), "Normal Table");
        assert_eq!(td.get_url(), "https://example.com/table");
        assert_eq!(td.tag, "N");
        assert_eq!(td.get_folder().len(), 2);

        // Level "1" folder
        assert_eq!(td.get_folder()[0].name(), "N1");
        assert_eq!(td.get_folder()[0].get_song().len(), 1);
        assert_eq!(td.get_folder()[0].get_song()[0].md5, "hash_a");

        // Level "2" folder
        assert_eq!(td.get_folder()[1].name(), "N2");
        assert_eq!(td.get_folder()[1].get_song().len(), 1);
        assert_eq!(td.get_folder()[1].get_song()[0].md5, "hash_b");
    }

    #[test]
    fn test_difficulty_table_to_table_data_with_courses() {
        let mut dt = DifficultyTable::new();
        dt.table.set_name("Course Table");
        dt.table.set_id("CT");
        dt.set_level_description(&["1".to_string()]);

        // Add a course
        let mut chart1 = BmsTableElement::new();
        chart1.set_md5("course_hash_1");
        let mut chart2 = BmsTableElement::new();
        chart2.set_md5("course_hash_2");

        let mut course = Course::new();
        course.set_name("Dan 1st");
        course.charts = vec![chart1, chart2];
        course.constraint = vec!["grade_mirror".to_string(), "gauge_lr2".to_string()];

        let mut trophy = Trophy::new();
        trophy.set_name("Gold");
        trophy.missrate = 5.0;
        trophy.scorerate = 90.0;
        course.trophy = vec![trophy];

        dt.course = vec![vec![course]];

        let td = difficulty_table_to_table_data(&dt, "https://example.com/course");

        assert_eq!(td.get_course().len(), 1);
        let cd = &td.get_course()[0];
        assert_eq!(cd.name(), "Dan 1st");
        assert_eq!(cd.hash.len(), 2);
        assert_eq!(cd.hash[0].md5, "course_hash_1");
        assert_eq!(cd.hash[1].md5, "course_hash_2");

        assert_eq!(cd.constraint.len(), 2);
        assert_eq!(cd.constraint[0], CourseDataConstraint::Mirror);
        assert_eq!(cd.constraint[1], CourseDataConstraint::GaugeLr2);

        assert_eq!(cd.trophy.len(), 1);
        assert_eq!(cd.trophy[0].name(), "Gold");
        assert_eq!(cd.trophy[0].missrate, 5.0);
        assert_eq!(cd.trophy[0].scorerate, 90.0);
    }

    #[test]
    fn test_difficulty_table_to_table_data_with_mode() {
        let mut dt = DifficultyTable::new();
        dt.table.set_name("DP Table");
        dt.table.set_id("DP");
        dt.table.set_mode("beat-14k");
        dt.set_level_description(&["1".to_string()]);

        let mut dte = DifficultyTableElement::new();
        dte.element.set_md5("dp_hash");
        dte.set_level(Some("1"));
        dt.table.add_element(dte);

        let td = difficulty_table_to_table_data(&dt, "https://example.com/dp");

        assert_eq!(td.get_folder()[0].get_song()[0].mode, 14);
    }

    #[test]
    fn test_difficulty_table_to_table_data_empty() {
        let mut dt = DifficultyTable::new();
        dt.table.set_name("Empty Table");
        dt.table.set_id("E");

        let td = difficulty_table_to_table_data(&dt, "https://example.com/empty");

        assert_eq!(td.name(), "Empty Table");
        assert!(td.get_folder().is_empty());
        assert!(td.get_course().is_empty());
    }

    #[test]
    fn test_difficulty_table_to_table_data_tag_fallback_to_id() {
        let mut dt = DifficultyTable::new();
        dt.table.set_name("Test");
        dt.table.set_id("TID");
        // No explicit tag set — should fallback to id

        let td = difficulty_table_to_table_data(&dt, "https://example.com");

        // BmsTable.get_tag() returns id when no tag is set
        assert_eq!(td.tag, "TID");
    }

    #[test]
    fn test_difficulty_table_to_table_data_multiple_elements_same_level() {
        let mut dt = DifficultyTable::new();
        dt.table.set_name("Multi");
        dt.table.set_id("M");
        dt.set_level_description(&["A".to_string()]);

        for i in 0..3 {
            let mut dte = DifficultyTableElement::new();
            dte.element.set_md5(&format!("hash_{}", i));
            dte.element.set_title(&format!("Song {}", i));
            dte.set_level(Some("A"));
            dt.table.add_element(dte);
        }

        let td = difficulty_table_to_table_data(&dt, "url");

        assert_eq!(td.get_folder().len(), 1);
        assert_eq!(td.get_folder()[0].get_song().len(), 3);
    }

    #[test]
    fn test_difficulty_table_to_table_data_multiple_course_lists() {
        let mut dt = DifficultyTable::new();
        dt.table.set_name("Multi Course");
        dt.table.set_id("MC");
        dt.set_level_description(&[]);

        // Two course lists with one course each
        let mut course1 = Course::new();
        course1.set_name("Course 1");
        let mut course2 = Course::new();
        course2.set_name("Course 2");

        dt.course = vec![vec![course1], vec![course2]];

        let td = difficulty_table_to_table_data(&dt, "url");

        // flat_map merges all course lists
        assert_eq!(td.get_course().len(), 2);
        assert_eq!(td.get_course()[0].name(), "Course 1");
        assert_eq!(td.get_course()[1].name(), "Course 2");
    }

    #[test]
    fn test_md5_lowercased() {
        let mut te = BmsTableElement::new();
        te.set_md5("ABCDEF0123456789");

        let song = bms_table_element_to_song_data(&te, None);

        assert_eq!(song.md5, "abcdef0123456789");
    }

    #[test]
    fn test_sha256_lowercased() {
        let mut te = BmsTableElement::new();
        te.set_sha256("ABCDEF0123456789ABCDEF0123456789");

        let song = bms_table_element_to_song_data(&te, None);

        assert_eq!(song.sha256, "abcdef0123456789abcdef0123456789");
    }

    #[test]
    fn test_invalid_element_mode_falls_back_to_default() {
        let mut te = BmsTableElement::new();
        te.set_mode("invalid-mode");

        let song = bms_table_element_to_song_data(&te, Some(&Mode::BEAT_7K));

        // invalid mode string -> Mode::from_hint returns None -> falls back to default
        assert_eq!(song.mode, 7);
    }

    #[test]
    fn test_invalid_element_mode_no_default() {
        let mut te = BmsTableElement::new();
        te.set_mode("invalid-mode");

        let song = bms_table_element_to_song_data(&te, None);

        assert_eq!(song.mode, 0);
    }

    #[test]
    fn test_course_unknown_constraint_filtered() {
        let mut course = Course::new();
        course.set_name("Test Course");
        course.constraint = vec![
            "grade_mirror".to_string(),
            "unknown_constraint".to_string(),
            "gauge_lr2".to_string(),
        ];

        let cd = course_to_course_data(&course, None);

        // "unknown_constraint" should be filtered out
        assert_eq!(cd.constraint.len(), 2);
        assert_eq!(cd.constraint[0], CourseDataConstraint::Mirror);
        assert_eq!(cd.constraint[1], CourseDataConstraint::GaugeLr2);
    }
}

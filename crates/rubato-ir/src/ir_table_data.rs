use rubato_core::table_data::TableData;
use rubato_types::validatable::Validatable;

use crate::ir_chart_data::IRChartData;
use crate::ir_course_data::IRCourseData;

/// IR table data
///
/// Translated from: IRTableData.java
#[derive(Clone, Debug)]
pub struct IRTableData {
    /// Table name
    pub name: String,
    /// Table folder data
    pub folders: Vec<IRTableFolder>,
    /// Course data
    pub courses: Vec<IRCourseData>,
}

impl IRTableData {
    pub fn new(name: String, folders: Vec<IRTableFolder>, courses: Vec<IRCourseData>) -> Self {
        Self {
            name,
            folders,
            courses,
        }
    }

    /// Convert IRTableData back to TableData.
    /// Translated from: Java BarManager.java (lines 134-190)
    pub fn to_table_data(&self) -> Option<TableData> {
        use rubato_core::table_data::TableFolder;

        let folder: Vec<TableFolder> = self
            .folders
            .iter()
            .map(|f| {
                let songs: Vec<rubato_types::song_data::SongData> =
                    f.charts.iter().map(|c| c.to_song_data()).collect();
                TableFolder {
                    name: Some(f.name.clone()),
                    songs,
                }
            })
            .collect();

        let course: Vec<rubato_core::course_data::CourseData> =
            self.courses.iter().map(|c| c.to_course_data()).collect();

        let mut td = TableData {
            name: self.name.clone(),
            folder,
            course,
            ..Default::default()
        };

        if td.validate() { Some(td) } else { None }
    }

    pub fn from_table_data(table: &TableData) -> Self {
        let mut folders = Vec::with_capacity(table.folder.len());
        for tf in &table.folder {
            let mut charts = Vec::with_capacity(tf.songs.len());
            for song in &tf.songs {
                // TableFolder songs are rubato_core::stubs::SongData
                charts.push(create_ir_chart_data_from_core_song(song));
            }
            folders.push(IRTableFolder::new(
                tf.name.clone().unwrap_or_default(),
                charts,
            ));
        }

        let mut courses = Vec::with_capacity(table.course.len());
        for course in &table.course {
            courses.push(IRCourseData::new(course));
        }

        Self {
            name: table.name().to_string(),
            folders,
            courses,
        }
    }
}

/// Create IRChartData from rubato_core::stubs::SongData
fn create_ir_chart_data_from_core_song(song: &rubato_core::stubs::SongData) -> IRChartData {
    IRChartData {
        md5: song.md5.clone(),
        sha256: song.sha256.clone(),
        title: song.title.clone(),
        subtitle: String::new(),
        genre: String::new(),
        artist: String::new(),
        subartist: String::new(),
        url: song.url.clone().unwrap_or_default(),
        appendurl: String::new(),
        level: 0,
        total: 0,
        mode: None,
        lntype: 0,
        judge: 0,
        minbpm: 0,
        maxbpm: 0,
        notes: 0,
        has_undefined_ln: false,
        has_ln: false,
        has_cn: false,
        has_hcn: false,
        has_mine: false,
        has_random: false,
        has_stop: false,
        values: std::collections::HashMap::new(),
    }
}

/// Table folder data
///
/// Translated from: IRTableData.IRTableFolder (inner class)
#[derive(Clone, Debug)]
pub struct IRTableFolder {
    /// Folder name
    pub name: String,
    /// Chart data
    pub charts: Vec<IRChartData>,
}

impl IRTableFolder {
    pub fn new(name: String, charts: Vec<IRChartData>) -> Self {
        Self { name, charts }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_course_data::IRTrophyData;
    use rubato_core::course_data::CourseDataConstraint;

    fn make_chart(sha256: &str, title: &str) -> IRChartData {
        IRChartData {
            sha256: sha256.to_string(),
            title: title.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_to_table_data_with_folders() {
        let ir = IRTableData::new(
            "Test Table".to_string(),
            vec![IRTableFolder::new(
                "Level 1".to_string(),
                vec![make_chart("abc123", "Song A")],
            )],
            vec![],
        );
        let td = ir.to_table_data();
        assert!(td.is_some());
        let td = td.unwrap();
        assert_eq!(td.name(), "Test Table");
        assert_eq!(td.folder.len(), 1);
        assert_eq!(td.folder[0].name(), "Level 1");
        assert_eq!(td.folder[0].songs.len(), 1);
        assert_eq!(td.folder[0].songs[0].sha256, "abc123");
        assert_eq!(td.folder[0].songs[0].title, "Song A");
    }

    #[test]
    fn test_to_table_data_with_courses() {
        let ir = IRTableData::new(
            "Course Table".to_string(),
            vec![],
            vec![IRCourseData {
                name: "Dan Course".to_string(),
                charts: vec![make_chart("def456", "Course Song")],
                constraint: vec![CourseDataConstraint::NoSpeed],
                trophy: vec![IRTrophyData {
                    name: "Gold".to_string(),
                    scorerate: 90.0,
                    smissrate: 5.0,
                }],
                lntype: -1,
            }],
        );
        let td = ir.to_table_data();
        assert!(td.is_some());
        let td = td.unwrap();
        assert_eq!(td.name(), "Course Table");
        assert_eq!(td.course.len(), 1);
        assert_eq!(td.course[0].name(), "Dan Course");
        assert!(td.course[0].release);
        assert_eq!(td.course[0].trophy.len(), 1);
        assert_eq!(td.course[0].trophy[0].name, Some("Gold".to_string()));
    }

    #[test]
    fn test_to_table_data_empty_returns_none() {
        let ir = IRTableData::new("".to_string(), vec![], vec![]);
        // Empty name fails validation
        assert!(ir.to_table_data().is_none());
    }

    #[test]
    fn test_roundtrip_from_table_data_to_table_data() {
        let ir_original = IRTableData::new(
            "Roundtrip".to_string(),
            vec![IRTableFolder::new(
                "Folder".to_string(),
                vec![make_chart("sha", "Title")],
            )],
            vec![],
        );
        let td = ir_original.to_table_data().unwrap();
        let ir_back = IRTableData::from_table_data(&td);
        assert_eq!(ir_back.name, "Roundtrip");
        assert_eq!(ir_back.folders.len(), 1);
        assert_eq!(ir_back.folders[0].charts[0].sha256, "sha");
    }
}

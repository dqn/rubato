use rubato_core::course_data::{CourseData, CourseDataConstraint, TrophyData};

use crate::ir_chart_data::IRChartData;

/// IR course data
///
/// Translated from: IRCourseData.java
#[derive(Clone, Debug)]
pub struct IRCourseData {
    /// Course name
    pub name: String,
    /// Chart data
    pub charts: Vec<IRChartData>,
    /// Course constraints
    pub constraint: Vec<CourseDataConstraint>,
    /// Trophy data
    pub trophy: Vec<IRTrophyData>,
    /// LN TYPE (-1: unspecified, 0: LN, 1: CN, 2: HCN)
    pub lntype: i32,
}

impl IRCourseData {
    /// Convert IRCourseData back to CourseData.
    /// Translated from: Java BarManager.java inline mapping (lines 157-186)
    pub fn to_course_data(&self) -> CourseData {
        let songs: Vec<rubato_types::song_data::SongData> =
            self.charts.iter().map(|c| c.to_song_data()).collect();
        let trophy: Vec<TrophyData> = self
            .trophy
            .iter()
            .map(|t| TrophyData {
                name: Some(t.name.clone()),
                scorerate: t.scorerate,
                missrate: t.smissrate,
            })
            .collect();
        let mut cd = CourseData::default();
        cd.set_name(self.name.clone());
        cd.hash = songs;
        cd.constraint = self.constraint.clone();
        cd.trophy = trophy;
        cd.release = true;
        cd
    }

    pub fn new(course: &CourseData) -> Self {
        Self::new_with_lntype(course, -1)
    }

    pub fn new_with_lntype(course: &CourseData, lntype: i32) -> Self {
        let songs = &course.hash;
        let mut charts = Vec::with_capacity(songs.len());
        for song in songs {
            // CourseData uses rubato_core::stubs::SongData which is different from our stubs::SongData
            // We need to create IRChartData from the available song data
            // Since CourseData::SongData is a different type, we create a minimal IRChartData
            charts.push(create_ir_chart_data_from_core_song(song));
        }

        let constraints = course.constraint.to_vec();

        let mut trophy = Vec::with_capacity(course.trophy.len());
        for t in &course.trophy {
            trophy.push(IRTrophyData::new(t));
        }

        Self {
            name: course.name().to_string(),
            charts,
            constraint: constraints,
            trophy,
            lntype,
        }
    }
}

/// Create IRChartData from rubato_core::stubs::SongData
fn create_ir_chart_data_from_core_song(song: &rubato_core::stubs::SongData) -> IRChartData {
    IRChartData {
        md5: song.file.md5.clone(),
        sha256: song.file.sha256.clone(),
        title: song.metadata.title.clone(),
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

/// IR trophy data
///
/// Translated from: IRCourseData.IRTrophyData (inner class)
#[derive(Clone, Debug)]
pub struct IRTrophyData {
    /// Trophy name
    pub name: String,
    /// Trophy score rate condition
    pub scorerate: f32,
    /// Trophy miss rate condition
    pub smissrate: f32,
}

impl IRTrophyData {
    pub fn new(trophy: &TrophyData) -> Self {
        Self {
            name: trophy.name.clone().unwrap_or_default(),
            scorerate: trophy.scorerate,
            smissrate: trophy.missrate,
        }
    }
}

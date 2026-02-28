use super::selectable_bar::SelectableBarData;
use crate::stubs::*;

/// Random course selection bar
/// Translates: bms.player.beatoraja.select.bar.RandomCourseBar
#[derive(Clone)]
pub struct RandomCourseBar {
    pub selectable: SelectableBarData,
    pub course: RandomCourseData,
}

impl RandomCourseBar {
    pub fn new(course: RandomCourseData) -> Self {
        Self {
            selectable: SelectableBarData::default(),
            course,
        }
    }

    pub fn get_course_data(&self) -> &RandomCourseData {
        &self.course
    }

    pub fn get_title(&self) -> String {
        self.course.get_name().to_string()
    }

    pub fn get_song_datas(&self) -> Vec<SongData> {
        self.course.get_song_datas()
    }

    pub fn exists_all_songs(&self) -> bool {
        if self.course.get_stage().is_empty() {
            return false;
        }
        // In Java: checks each stage for null
        // Here we check the stage vec
        true
    }

    pub fn get_lamp(&self, _is_player: bool) -> i32 {
        0
    }
}

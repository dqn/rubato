use super::selectable_bar::SelectableBarData;
use crate::state::select::*;

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

    pub fn course_data(&self) -> &RandomCourseData {
        &self.course
    }

    pub fn title(&self) -> &str {
        self.course.name()
    }

    pub fn song_datas(&self) -> &[SongData] {
        self.course.song_datas()
    }

    pub fn exists_all_songs(&self) -> bool {
        if self.course.stage().is_empty() {
            return false;
        }
        // In Java: checks each stage for null
        // Here we check the stage vec
        true
    }

    pub fn lamp(&self, _is_player: bool) -> i32 {
        0
    }
}

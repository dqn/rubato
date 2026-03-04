// CourseResultSkin.java -> course_result_skin.rs
// Mechanical line-by-line translation.

use super::stubs::{Skin, SkinHeader};

/// Course result skin
pub struct CourseResultSkin {
    pub skin: Skin,
    ranktime: i32,
}

impl CourseResultSkin {
    pub fn new(header: SkinHeader) -> Self {
        Self {
            skin: Skin::new(header),
            ranktime: 0,
        }
    }

    pub fn get_rank_time(&self) -> i32 {
        self.ranktime
    }

    pub fn set_rank_time(&mut self, ranktime: i32) {
        self.ranktime = ranktime;
    }

    pub fn get_input(&self) -> i32 {
        self.skin.get_input()
    }

    pub fn get_scene(&self) -> i32 {
        self.skin.get_scene()
    }

    pub fn get_fadeout(&self) -> i32 {
        self.skin.get_fadeout()
    }
}

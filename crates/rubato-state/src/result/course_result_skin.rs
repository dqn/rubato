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

    pub fn from_loaded_skin(skin: &Skin) -> Self {
        let mut timing_skin = Skin::new(skin.header.clone());
        timing_skin.set_input(skin.input());
        timing_skin.set_scene(skin.scene());
        timing_skin.set_fadeout(skin.fadeout());
        Self {
            skin: timing_skin,
            ranktime: 0,
        }
    }

    pub fn rank_time(&self) -> i32 {
        self.ranktime
    }

    pub fn set_rank_time(&mut self, ranktime: i32) {
        self.ranktime = ranktime;
    }

    pub fn input(&self) -> i32 {
        self.skin.input()
    }

    pub fn scene(&self) -> i32 {
        self.skin.scene()
    }

    pub fn fadeout(&self) -> i32 {
        self.skin.fadeout()
    }
}

// CourseResultSkin.java -> course_result_skin.rs
// Mechanical line-by-line translation.

use super::stubs::{Skin, SkinHeader};

/// Course result skin
pub struct CourseResultSkin {
    pub skin: Skin,
    pub ranktime: i32,
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
        timing_skin.input = skin.input();
        timing_skin.scene = skin.scene();
        timing_skin.fadeout = skin.fadeout();
        Self {
            skin: timing_skin,
            ranktime: 0,
        }
    }

    pub fn rank_time(&self) -> i32 {
        self.ranktime
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

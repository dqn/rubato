// MusicResultSkin.java -> music_result_skin.rs
// Mechanical line-by-line translation.

use super::stubs::{Skin, SkinHeader};

/// Music result skin
pub struct MusicResultSkin {
    pub skin: Skin,
    pub ranktime: i32,
}

impl MusicResultSkin {
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

// Unified result skin data (replaces MusicResultSkin and CourseResultSkin).
// Both were structurally identical; this single type serves both music and course result screens.

use super::{Skin, SkinHeader};

/// Result skin timing metadata shared by music and course result screens.
pub struct ResultSkinData {
    pub skin: Skin,
    pub ranktime: i32,
}

impl ResultSkinData {
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
            ranktime: skin.ranktime(),
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

    /// Create a ResultSkinData with explicit timing values (for tests).
    #[cfg(test)]
    pub fn new_with_timings(ranktime: i32, input: i32, scene: i32, fadeout: i32) -> Self {
        let mut skin = Skin::new(SkinHeader::default());
        skin.input = input;
        skin.scene = scene;
        skin.fadeout = fadeout;
        Self { skin, ranktime }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_loaded_skin_copies_ranktime() {
        let header = SkinHeader::default();
        let mut skin = Skin::new(header);
        skin.ranktime = 750;

        let result_data = ResultSkinData::from_loaded_skin(&skin);
        assert_eq!(result_data.ranktime, 750);
    }

    #[test]
    fn from_loaded_skin_copies_zero_ranktime() {
        let header = SkinHeader::default();
        let skin = Skin::new(header);

        let result_data = ResultSkinData::from_loaded_skin(&skin);
        assert_eq!(result_data.ranktime, 0);
    }

    #[test]
    fn from_loaded_skin_copies_negative_ranktime() {
        let header = SkinHeader::default();
        let mut skin = Skin::new(header);
        skin.ranktime = -300;

        let result_data = ResultSkinData::from_loaded_skin(&skin);
        assert_eq!(result_data.ranktime, -300);
    }
}

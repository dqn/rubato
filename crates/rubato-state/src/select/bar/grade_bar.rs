use super::selectable_bar::SelectableBarData;
use crate::select::stubs::*;

/// Course selection bar
/// Translates: bms.player.beatoraja.select.bar.GradeBar
#[derive(Clone)]
pub struct GradeBar {
    pub selectable: SelectableBarData,
    /// Course data
    pub course: CourseData,
    /// Mirror score
    pub mscore: Option<ScoreData>,
    /// Random score
    pub rscore: Option<ScoreData>,
}

impl GradeBar {
    pub fn new(course: CourseData) -> Self {
        Self {
            selectable: SelectableBarData::default(),
            course,
            mscore: None,
            rscore: None,
        }
    }

    pub fn get_course_data(&self) -> &CourseData {
        &self.course
    }

    pub fn get_song_datas(&self) -> &[SongData] {
        self.course.get_song()
    }

    pub fn get_title(&self) -> String {
        self.course.get_name().to_string()
    }

    pub fn exists_all_songs(&self) -> bool {
        for song in self.course.get_song() {
            if song.get_path().is_none() {
                return false;
            }
        }
        true
    }

    pub fn get_mirror_score(&self) -> Option<&ScoreData> {
        self.mscore.as_ref()
    }

    pub fn set_mirror_score(&mut self, score: Option<ScoreData>) {
        self.mscore = score;
    }

    pub fn get_random_score(&self) -> Option<&ScoreData> {
        self.rscore.as_ref()
    }

    pub fn set_random_score(&mut self, score: Option<ScoreData>) {
        self.rscore = score;
    }

    pub fn get_trophy(&self) -> Option<&TrophyData> {
        let scores = [
            self.selectable.bar_data.get_score(),
            self.mscore.as_ref(),
            self.rscore.as_ref(),
        ];

        let trophies = self.course.get_trophy();
        for i in (0..trophies.len()).rev() {
            for score in &scores {
                if let Some(score) = score
                    && Self::qualified(score, &trophies[i])
                {
                    return Some(&trophies[i]);
                }
            }
        }
        None
    }

    fn qualified(score: &ScoreData, trophy: &TrophyData) -> bool {
        score.get_notes() != 0
            && trophy.get_missrate() >= score.get_minbp() as f32 * 100.0 / score.get_notes() as f32
            && trophy.get_scorerate()
                <= score.get_exscore() as f32 * 100.0 / (score.get_notes() as f32 * 2.0)
    }

    pub fn get_lamp(&self, is_player: bool) -> i32 {
        if !is_player {
            return self
                .selectable
                .bar_data
                .get_rival_score()
                .map(|s| s.get_clear())
                .unwrap_or(0);
        }

        let mut result = 0;
        if let Some(score) = self.selectable.bar_data.get_score()
            && score.get_clear() > result
        {
            result = score.get_clear();
        }
        if let Some(score) = self.get_mirror_score()
            && score.get_clear() > result
        {
            result = score.get_clear();
        }
        if let Some(score) = self.get_random_score()
            && score.get_clear() > result
        {
            result = score.get_clear();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score_with_clear(clear: i32) -> ScoreData {
        let mut sd = ScoreData::default();
        sd.clear = clear;
        sd
    }

    #[test]
    fn test_get_lamp_player_returns_max_of_all_scores() {
        let mut bar = GradeBar::new(CourseData::default());

        // No scores: returns 0
        assert_eq!(bar.get_lamp(true), 0);

        // Player score only
        bar.selectable.bar_data.set_score(Some(score_with_clear(3)));
        assert_eq!(bar.get_lamp(true), 3);

        // Mirror score is higher
        bar.set_mirror_score(Some(score_with_clear(5)));
        assert_eq!(bar.get_lamp(true), 5);

        // Random score is highest
        bar.set_random_score(Some(score_with_clear(7)));
        assert_eq!(bar.get_lamp(true), 7);

        // Player score is highest
        bar.selectable.bar_data.set_score(Some(score_with_clear(9)));
        assert_eq!(bar.get_lamp(true), 9);
    }

    #[test]
    fn test_get_lamp_rival_returns_rival_clear() {
        let mut bar = GradeBar::new(CourseData::default());

        // Set player scores (should be ignored for rival)
        bar.selectable.bar_data.set_score(Some(score_with_clear(9)));
        bar.set_mirror_score(Some(score_with_clear(8)));
        bar.set_random_score(Some(score_with_clear(7)));

        // Set rival score
        bar.selectable
            .bar_data
            .set_rival_score(Some(score_with_clear(4)));

        assert_eq!(bar.get_lamp(false), 4);
    }

    #[test]
    fn test_get_lamp_rival_with_no_rival_score_returns_zero() {
        let mut bar = GradeBar::new(CourseData::default());

        // Set player scores but no rival score
        bar.selectable.bar_data.set_score(Some(score_with_clear(9)));
        bar.set_mirror_score(Some(score_with_clear(8)));

        assert_eq!(bar.get_lamp(false), 0);
    }
}

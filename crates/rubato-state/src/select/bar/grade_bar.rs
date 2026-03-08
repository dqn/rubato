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

    pub fn course_data(&self) -> &CourseData {
        &self.course
    }

    pub fn song_datas(&self) -> &[SongData] {
        &self.course.hash
    }

    pub fn title(&self) -> &str {
        self.course.name()
    }

    pub fn exists_all_songs(&self) -> bool {
        for song in &self.course.hash {
            if song.file.path().is_none() {
                return false;
            }
        }
        true
    }

    pub fn mirror_score(&self) -> Option<&ScoreData> {
        self.mscore.as_ref()
    }

    pub fn random_score(&self) -> Option<&ScoreData> {
        self.rscore.as_ref()
    }

    pub fn trophy(&self) -> Option<&TrophyData> {
        let scores = [
            self.selectable.bar_data.score(),
            self.mscore.as_ref(),
            self.rscore.as_ref(),
        ];

        let trophies = &self.course.trophy;
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
        score.notes != 0
            && trophy.missrate >= score.minbp as f32 * 100.0 / score.notes as f32
            && trophy.scorerate <= score.exscore() as f32 * 100.0 / (score.notes as f32 * 2.0)
    }

    pub fn lamp(&self, is_player: bool) -> i32 {
        if !is_player {
            return self
                .selectable
                .bar_data
                .rival_score()
                .map(|s| s.clear)
                .unwrap_or(0);
        }

        let mut result = 0;
        if let Some(score) = self.selectable.bar_data.score()
            && score.clear > result
        {
            result = score.clear;
        }
        if let Some(score) = self.mirror_score()
            && score.clear > result
        {
            result = score.clear;
        }
        if let Some(score) = self.random_score()
            && score.clear > result
        {
            result = score.clear;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score_with_clear(clear: i32) -> ScoreData {
        ScoreData {
            clear,
            ..Default::default()
        }
    }

    #[test]
    fn test_get_lamp_player_returns_max_of_all_scores() {
        let mut bar = GradeBar::new(CourseData::default());

        // No scores: returns 0
        assert_eq!(bar.lamp(true), 0);

        // Player score only
        bar.selectable.bar_data.score = Some(score_with_clear(3));
        assert_eq!(bar.lamp(true), 3);

        // Mirror score is higher
        bar.mscore = Some(score_with_clear(5));
        assert_eq!(bar.lamp(true), 5);

        // Random score is highest
        bar.rscore = Some(score_with_clear(7));
        assert_eq!(bar.lamp(true), 7);

        // Player score is highest
        bar.selectable.bar_data.score = Some(score_with_clear(9));
        assert_eq!(bar.lamp(true), 9);
    }

    #[test]
    fn test_get_lamp_rival_returns_rival_clear() {
        let mut bar = GradeBar::new(CourseData::default());

        // Set player scores (should be ignored for rival)
        bar.selectable.bar_data.score = Some(score_with_clear(9));
        bar.mscore = Some(score_with_clear(8));
        bar.rscore = Some(score_with_clear(7));

        // Set rival score
        bar.selectable.bar_data.rscore = Some(score_with_clear(4));

        assert_eq!(bar.lamp(false), 4);
    }

    #[test]
    fn test_get_lamp_rival_with_no_rival_score_returns_zero() {
        let mut bar = GradeBar::new(CourseData::default());

        // Set player scores but no rival score
        bar.selectable.bar_data.score = Some(score_with_clear(9));
        bar.mscore = Some(score_with_clear(8));

        assert_eq!(bar.lamp(false), 0);
    }
}

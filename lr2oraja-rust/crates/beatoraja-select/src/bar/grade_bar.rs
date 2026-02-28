use super::selectable_bar::SelectableBarData;
use crate::stubs::*;

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

    pub fn get_lamp(&self, _is_player: bool) -> i32 {
        // TODO: rival score
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

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::core::course_data::{CourseData, CourseDataConstraint, TrophyData};
use crate::core::random_stage_data::RandomStageData;
use rubato_types::SongData;

/// Random course data - selects songs by SQL query results
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RandomCourseData {
    pub name: Option<String>,
    pub stage: Vec<RandomStageData>,
    pub constraint: Vec<CourseDataConstraint>,
    pub rconstraint: Vec<RandomCourseDataConstraint>,
    pub trophy: Vec<TrophyData>,
    pub song_datas: Vec<SongData>,
}

impl RandomCourseData {
    pub const EMPTY: &'static [RandomCourseData] = &[];

    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    pub fn stage(&self) -> &[RandomStageData] {
        &self.stage
    }

    pub fn song_datas(&self) -> &[SongData] {
        &self.song_datas
    }

    pub fn is_release(&self) -> bool {
        false
    }

    pub fn create_course_data(&self) -> CourseData {
        let now = chrono::Local::now();
        let date_str = now.format("%Y%m%d_%H%M%S").to_string();
        let name = format!("{} {}", self.name.as_deref().unwrap_or(""), date_str);
        CourseData {
            name: Some(name),
            hash: self.song_datas.clone(),
            constraint: self.constraint.clone(),
            trophy: self.trophy.clone(),
            release: false,
        }
    }

    /// Run the full lottery: query DB for each stage's SQL, then pick random songs.
    ///
    /// Java: RandomCourseData.lotterySongDatas(MainController)
    pub fn lottery_song_datas(
        &mut self,
        songdb: &dyn rubato_types::song_database_accessor::SongDatabaseAccessor,
        score_db_path: &str,
        scorelog_db_path: &str,
        info_db_path: Option<&str>,
    ) {
        let is_distinct = self
            .rconstraint
            .contains(&RandomCourseDataConstraint::Distinct);
        let stage_count = self.stage.len();
        let mut results: Vec<Option<SongData>> = vec![None; stage_count];
        let mut lots: Vec<SongData> = Vec::new();

        for (i, stage) in self.stage.iter().enumerate() {
            let sql_opt = stage.sql.as_deref().filter(|s| !s.is_empty());
            if sql_opt.is_none() && i > 0 {
                Self::lottery_song_data(&mut results, i, &lots, is_distinct);
                continue;
            }
            let sql = sql_opt.unwrap_or("1");
            lots = songdb.song_datas_by_sql(sql, score_db_path, scorelog_db_path, info_db_path);
            Self::lottery_song_data(&mut results, i, &lots, is_distinct);
        }

        self.song_datas = results.into_iter().flatten().collect();
    }

    /// Lottery song datas from provided lots arrays.
    /// This is a simplified version - the actual DB query is handled externally.
    #[allow(clippy::needless_range_loop)]
    pub fn lottery_song_data(
        song_datas: &mut [Option<SongData>],
        index: usize,
        lots: &[SongData],
        is_distinct: bool,
    ) {
        if lots.is_empty() {
            return;
        }
        let mut rng = rand::thread_rng();
        if !is_distinct {
            song_datas[index] = Some(lots[rng.gen_range(0..lots.len())].clone());
            return;
        }

        // Lottery song, re-lottery if duplicated with previous stages. Allow duplicates if no options left.
        let mut temp_lots: Vec<&SongData> = lots.iter().collect();
        while !temp_lots.is_empty() {
            let ri = rng.gen_range(0..temp_lots.len());
            let candidate = temp_lots[ri].clone();
            let mut is_duplicate = false;
            for j in 0..index {
                match &song_datas[j] {
                    None => continue,
                    Some(prev) => {
                        if candidate.file.sha256 == prev.file.sha256 {
                            temp_lots.remove(ri);
                            is_duplicate = true;
                            break;
                        }
                    }
                }
            }
            if !is_duplicate {
                song_datas[index] = Some(candidate);
                return;
            }
        }
        song_datas[index] = Some(lots[rng.gen_range(0..lots.len())].clone());
    }
}

/// Random course data constraint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RandomCourseDataConstraint {
    Distinct,
}

impl RandomCourseDataConstraint {
    pub fn name_str(&self) -> &str {
        match self {
            RandomCourseDataConstraint::Distinct => "distinct",
        }
    }

    pub fn constraint_type(&self) -> i32 {
        match self {
            RandomCourseDataConstraint::Distinct => 0,
        }
    }

    pub fn value(name: &str) -> Option<RandomCourseDataConstraint> {
        match name {
            "distinct" => Some(RandomCourseDataConstraint::Distinct),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_song(sha256: &str) -> SongData {
        let mut s = SongData::default();
        s.file.sha256 = sha256.to_string();
        s
    }

    #[test]
    fn distinct_skips_empty_stages_and_still_deduplicates() {
        // Stage 0: empty (no candidates), stage 1: picks "A", stage 2: should not pick "A" again
        let lots = vec![make_song("A")];
        let mut song_datas: Vec<Option<SongData>> = vec![None; 3];

        // Stage 0: no lots -> stays None
        // Stage 1: picks from lots
        RandomCourseData::lottery_song_data(&mut song_datas, 1, &lots, true);
        assert_eq!(song_datas[1].as_ref().unwrap().file.sha256, "A");

        // Stage 2: lots has only "A", stage 0 is None but stage 1 has "A"
        // With the fix, it should skip None and detect "A" as duplicate,
        // exhaust temp_lots, then fall back to "A" (only option).
        let lots2 = vec![make_song("A"), make_song("B")];
        RandomCourseData::lottery_song_data(&mut song_datas, 2, &lots2, true);
        // If duplicate check works, it should pick "B"
        assert_eq!(song_datas[2].as_ref().unwrap().file.sha256, "B");
    }
}

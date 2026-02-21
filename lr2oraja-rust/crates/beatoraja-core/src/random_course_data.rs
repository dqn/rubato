use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::course_data::{CourseData, CourseDataConstraint, TrophyData};
use crate::random_stage_data::RandomStageData;
use crate::stubs::SongData;

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
                    None => break,
                    Some(prev) => {
                        if candidate.sha256 == prev.sha256 {
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

    pub fn get_value(name: &str) -> Option<RandomCourseDataConstraint> {
        match name {
            "distinct" => Some(RandomCourseDataConstraint::Distinct),
            _ => None,
        }
    }
}

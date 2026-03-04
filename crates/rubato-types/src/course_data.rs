use serde::{Deserialize, Serialize};

use crate::stubs::SongData;
use crate::validatable::{Validatable, remove_invalid_elements_vec};

/// Course data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CourseData {
    pub name: Option<String>,
    pub hash: Vec<SongData>,
    pub constraint: Vec<CourseDataConstraint>,
    pub trophy: Vec<TrophyData>,
    pub release: bool,
}

impl Default for CourseData {
    fn default() -> Self {
        Self {
            name: None,
            hash: Vec::new(),
            constraint: Vec::new(),
            trophy: Vec::new(),
            release: true,
        }
    }
}

impl CourseData {
    pub fn get_name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    pub fn get_constraint(&self) -> &[CourseDataConstraint] {
        &self.constraint
    }

    pub fn get_song(&self) -> &[SongData] {
        &self.hash
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn set_song(&mut self, hash: Vec<SongData>) {
        self.hash = hash;
    }

    pub fn get_trophy(&self) -> &[TrophyData] {
        &self.trophy
    }

    pub fn set_trophy(&mut self, trophy: Vec<TrophyData>) {
        self.trophy = trophy;
    }

    pub fn set_constraint(&mut self, constraint: Vec<CourseDataConstraint>) {
        self.constraint = constraint;
    }

    pub fn set_release(&mut self, release: bool) {
        self.release = release;
    }

    pub fn set_course_song_models(&mut self, _models: &[SongData]) {
        // Java compat stub
    }

    pub fn is_class_course(&self) -> bool {
        for con in &self.constraint {
            if *con == CourseDataConstraint::Class
                || *con == CourseDataConstraint::Mirror
                || *con == CourseDataConstraint::Random
            {
                return true;
            }
        }
        false
    }

    pub fn shrink(&mut self) {
        for song in &mut self.hash {
            song.shrink();
        }
    }
}

impl Validatable for CourseData {
    fn validate(&mut self) -> bool {
        if self.hash.is_empty() {
            return false;
        }
        if self.name.as_ref().is_none_or(|n| n.is_empty()) {
            self.name = Some("No Course Title".to_string());
        }
        for i in 0..self.hash.len() {
            if self.hash[i].title.is_empty() {
                self.hash[i].title = format!("course {}", i + 1);
            }
            if !self.hash[i].validate() {
                return false;
            }
        }

        // Deduplicate constraints by type
        let mut cdc: [Option<CourseDataConstraint>; 5] = [None, None, None, None, None];
        for c in &self.constraint {
            let t = c.constraint_type() as usize;
            if t < 5 && cdc[t].is_none() {
                cdc[t] = Some(*c);
            }
        }
        self.constraint = cdc.iter().filter_map(|c| *c).collect();

        self.trophy = remove_invalid_elements_vec(std::mem::take(&mut self.trophy));
        true
    }
}

/// Course data constraint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CourseDataConstraint {
    #[serde(alias = "grade")]
    Class,
    #[serde(alias = "grade_mirror")]
    Mirror,
    #[serde(alias = "grade_random")]
    Random,
    #[serde(alias = "no_speed")]
    NoSpeed,
    #[serde(alias = "no_good")]
    NoGood,
    #[serde(alias = "no_great")]
    NoGreat,
    #[serde(alias = "gauge_lr2")]
    GaugeLr2,
    #[serde(alias = "gauge_5k")]
    Gauge5Keys,
    #[serde(alias = "gauge_7k")]
    Gauge7Keys,
    #[serde(alias = "gauge_9k")]
    Gauge9Keys,
    #[serde(alias = "gauge_24k")]
    Gauge24Keys,
    #[serde(alias = "ln")]
    Ln,
    #[serde(alias = "cn")]
    Cn,
    #[serde(alias = "hcn")]
    Hcn,
}

impl CourseDataConstraint {
    pub fn name_str(&self) -> &str {
        match self {
            CourseDataConstraint::Class => "grade",
            CourseDataConstraint::Mirror => "grade_mirror",
            CourseDataConstraint::Random => "grade_random",
            CourseDataConstraint::NoSpeed => "no_speed",
            CourseDataConstraint::NoGood => "no_good",
            CourseDataConstraint::NoGreat => "no_great",
            CourseDataConstraint::GaugeLr2 => "gauge_lr2",
            CourseDataConstraint::Gauge5Keys => "gauge_5k",
            CourseDataConstraint::Gauge7Keys => "gauge_7k",
            CourseDataConstraint::Gauge9Keys => "gauge_9k",
            CourseDataConstraint::Gauge24Keys => "gauge_24k",
            CourseDataConstraint::Ln => "ln",
            CourseDataConstraint::Cn => "cn",
            CourseDataConstraint::Hcn => "hcn",
        }
    }

    pub fn constraint_type(&self) -> i32 {
        match self {
            CourseDataConstraint::Class
            | CourseDataConstraint::Mirror
            | CourseDataConstraint::Random => 0,
            CourseDataConstraint::NoSpeed => 1,
            CourseDataConstraint::NoGood | CourseDataConstraint::NoGreat => 2,
            CourseDataConstraint::GaugeLr2
            | CourseDataConstraint::Gauge5Keys
            | CourseDataConstraint::Gauge7Keys
            | CourseDataConstraint::Gauge9Keys
            | CourseDataConstraint::Gauge24Keys => 3,
            CourseDataConstraint::Ln | CourseDataConstraint::Cn | CourseDataConstraint::Hcn => 4,
        }
    }

    pub fn values() -> &'static [CourseDataConstraint] {
        &[
            CourseDataConstraint::Class,
            CourseDataConstraint::Mirror,
            CourseDataConstraint::Random,
            CourseDataConstraint::NoSpeed,
            CourseDataConstraint::NoGood,
            CourseDataConstraint::NoGreat,
            CourseDataConstraint::GaugeLr2,
            CourseDataConstraint::Gauge5Keys,
            CourseDataConstraint::Gauge7Keys,
            CourseDataConstraint::Gauge9Keys,
            CourseDataConstraint::Gauge24Keys,
            CourseDataConstraint::Ln,
            CourseDataConstraint::Cn,
            CourseDataConstraint::Hcn,
        ]
    }

    pub fn get_value(name: &str) -> Option<CourseDataConstraint> {
        for constraint in CourseDataConstraint::values() {
            if constraint.name_str() == name {
                return Some(*constraint);
            }
        }
        None
    }
}

/// Course data trophy condition
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrophyData {
    pub name: Option<String>,
    pub missrate: f32,
    pub scorerate: f32,
}

impl TrophyData {
    pub fn new(name: String, missrate: f32, scorerate: f32) -> Self {
        Self {
            name: Some(name),
            missrate,
            scorerate,
        }
    }

    pub fn get_name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn get_missrate(&self) -> f32 {
        self.missrate
    }

    pub fn set_missrate(&mut self, v: f32) {
        self.missrate = v;
    }

    pub fn get_scorerate(&self) -> f32 {
        self.scorerate
    }

    pub fn set_scorerate(&mut self, v: f32) {
        self.scorerate = v;
    }
}

impl Validatable for TrophyData {
    fn validate(&mut self) -> bool {
        self.name.as_ref().is_some_and(|n| !n.is_empty())
            && self.missrate > 0.0
            && self.scorerate < 100.0
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    // -- CourseData tests --

    #[test]
    fn test_course_data_default() {
        let cd = CourseData::default();
        assert!(cd.name.is_none());
        assert!(cd.hash.is_empty());
        assert!(cd.constraint.is_empty());
        assert!(cd.trophy.is_empty());
        assert!(cd.release);
    }

    #[test]
    fn test_course_data_name_accessor() {
        let mut cd = CourseData::default();
        assert_eq!(cd.get_name(), "");

        cd.set_name("My Course".to_string());
        assert_eq!(cd.get_name(), "My Course");
    }

    #[test]
    fn test_course_data_song_accessor() {
        let mut cd = CourseData::default();
        assert!(cd.get_song().is_empty());

        let mut song = SongData::new();
        song.title = "Song 1".to_string();
        cd.set_song(vec![song]);
        assert_eq!(cd.get_song().len(), 1);
        assert_eq!(cd.get_song()[0].title, "Song 1");
    }

    #[test]
    fn test_course_data_trophy_accessor() {
        let mut cd = CourseData::default();
        assert!(cd.get_trophy().is_empty());

        let trophy = TrophyData::new("Gold".to_string(), 5.0, 90.0);
        cd.set_trophy(vec![trophy]);
        assert_eq!(cd.get_trophy().len(), 1);
        assert_eq!(cd.get_trophy()[0].get_name(), "Gold");
    }

    #[test]
    fn test_course_data_constraint_accessor() {
        let mut cd = CourseData::default();
        assert!(cd.get_constraint().is_empty());

        cd.set_constraint(vec![
            CourseDataConstraint::Class,
            CourseDataConstraint::NoSpeed,
        ]);
        assert_eq!(cd.get_constraint().len(), 2);
    }

    #[test]
    fn test_course_data_release() {
        let mut cd = CourseData::default();
        assert!(cd.release);

        cd.set_release(false);
        assert!(!cd.release);
    }

    #[test]
    fn test_course_data_is_class_course() {
        let mut cd = CourseData::default();
        assert!(!cd.is_class_course());

        cd.constraint = vec![CourseDataConstraint::Class];
        assert!(cd.is_class_course());

        cd.constraint = vec![CourseDataConstraint::Mirror];
        assert!(cd.is_class_course());

        cd.constraint = vec![CourseDataConstraint::Random];
        assert!(cd.is_class_course());

        cd.constraint = vec![CourseDataConstraint::NoSpeed];
        assert!(!cd.is_class_course());
    }

    #[test]
    fn test_course_data_serde_round_trip() {
        let mut cd = CourseData::default();
        cd.set_name("Test Course".to_string());
        cd.set_release(false);
        cd.set_constraint(vec![CourseDataConstraint::Class, CourseDataConstraint::Ln]);

        let json = serde_json::to_string(&cd).unwrap();
        let deserialized: CourseData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.get_name(), "Test Course");
        assert!(!deserialized.release);
        assert_eq!(deserialized.constraint.len(), 2);
        assert_eq!(deserialized.constraint[0], CourseDataConstraint::Class);
        assert_eq!(deserialized.constraint[1], CourseDataConstraint::Ln);
    }

    // -- CourseDataConstraint tests --

    #[test]
    fn test_constraint_values_count() {
        assert_eq!(CourseDataConstraint::values().len(), 14);
    }

    #[test]
    fn test_constraint_name_str() {
        assert_eq!(CourseDataConstraint::Class.name_str(), "grade");
        assert_eq!(CourseDataConstraint::Mirror.name_str(), "grade_mirror");
        assert_eq!(CourseDataConstraint::NoSpeed.name_str(), "no_speed");
        assert_eq!(CourseDataConstraint::GaugeLr2.name_str(), "gauge_lr2");
        assert_eq!(CourseDataConstraint::Ln.name_str(), "ln");
        assert_eq!(CourseDataConstraint::Hcn.name_str(), "hcn");
    }

    #[test]
    fn test_constraint_type() {
        assert_eq!(CourseDataConstraint::Class.constraint_type(), 0);
        assert_eq!(CourseDataConstraint::Mirror.constraint_type(), 0);
        assert_eq!(CourseDataConstraint::Random.constraint_type(), 0);
        assert_eq!(CourseDataConstraint::NoSpeed.constraint_type(), 1);
        assert_eq!(CourseDataConstraint::NoGood.constraint_type(), 2);
        assert_eq!(CourseDataConstraint::NoGreat.constraint_type(), 2);
        assert_eq!(CourseDataConstraint::GaugeLr2.constraint_type(), 3);
        assert_eq!(CourseDataConstraint::Gauge7Keys.constraint_type(), 3);
        assert_eq!(CourseDataConstraint::Ln.constraint_type(), 4);
        assert_eq!(CourseDataConstraint::Cn.constraint_type(), 4);
    }

    #[test]
    fn test_constraint_get_value() {
        assert_eq!(
            CourseDataConstraint::get_value("grade"),
            Some(CourseDataConstraint::Class)
        );
        assert_eq!(
            CourseDataConstraint::get_value("no_speed"),
            Some(CourseDataConstraint::NoSpeed)
        );
        assert_eq!(
            CourseDataConstraint::get_value("gauge_7k"),
            Some(CourseDataConstraint::Gauge7Keys)
        );
        assert_eq!(CourseDataConstraint::get_value("nonexistent"), None);
    }

    // -- TrophyData tests --

    #[test]
    fn test_trophy_data_construction() {
        let trophy = TrophyData::new("Silver".to_string(), 10.0, 80.0);
        assert_eq!(trophy.get_name(), "Silver");
        assert_eq!(trophy.get_missrate(), 10.0);
        assert_eq!(trophy.get_scorerate(), 80.0);
    }

    #[test]
    fn test_trophy_data_default() {
        let trophy = TrophyData::default();
        assert_eq!(trophy.get_name(), "");
        assert_eq!(trophy.get_missrate(), 0.0);
        assert_eq!(trophy.get_scorerate(), 0.0);
    }

    #[test]
    fn test_trophy_data_setters() {
        let mut trophy = TrophyData::default();
        trophy.set_name("Gold".to_string());
        trophy.set_missrate(5.0);
        trophy.set_scorerate(95.0);

        assert_eq!(trophy.get_name(), "Gold");
        assert_eq!(trophy.get_missrate(), 5.0);
        assert_eq!(trophy.get_scorerate(), 95.0);
    }

    #[test]
    fn test_trophy_data_validate() {
        // Valid case: has name, missrate > 0, scorerate < 100
        let mut valid = TrophyData::new("Test".to_string(), 5.0, 90.0);
        assert!(valid.validate());

        // Invalid: no name
        let mut no_name = TrophyData::default();
        no_name.missrate = 5.0;
        no_name.scorerate = 90.0;
        assert!(!no_name.validate());

        // Invalid: missrate <= 0
        let mut zero_miss = TrophyData::new("Test".to_string(), 0.0, 90.0);
        assert!(!zero_miss.validate());

        // Invalid: scorerate >= 100
        let mut high_score = TrophyData::new("Test".to_string(), 5.0, 100.0);
        assert!(!high_score.validate());
    }

    #[test]
    fn test_trophy_data_serde_round_trip() {
        let trophy = TrophyData::new("Diamond".to_string(), 3.5, 95.0);
        let json = serde_json::to_string(&trophy).unwrap();
        let deserialized: TrophyData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.get_name(), "Diamond");
        assert_eq!(deserialized.get_missrate(), 3.5);
        assert_eq!(deserialized.get_scorerate(), 95.0);
    }
}

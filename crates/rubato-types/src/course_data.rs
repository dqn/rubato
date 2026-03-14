use serde::{Deserialize, Serialize};

use crate::song_data::SongData;
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
    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
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
        for (i, hash) in self.hash.iter_mut().enumerate() {
            if hash.metadata.title.is_empty() {
                hash.metadata.title = format!("course {}", i + 1);
            }
            if !hash.validate() {
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
    #[serde(rename = "grade")]
    Class,
    #[serde(rename = "grade_mirror")]
    Mirror,
    #[serde(rename = "grade_random")]
    Random,
    #[serde(rename = "no_speed")]
    NoSpeed,
    #[serde(rename = "no_good")]
    NoGood,
    #[serde(rename = "no_great")]
    NoGreat,
    #[serde(rename = "gauge_lr2")]
    GaugeLr2,
    #[serde(rename = "gauge_5k")]
    Gauge5Keys,
    #[serde(rename = "gauge_7k")]
    Gauge7Keys,
    #[serde(rename = "gauge_9k")]
    Gauge9Keys,
    #[serde(rename = "gauge_24k")]
    Gauge24Keys,
    #[serde(rename = "ln")]
    Ln,
    #[serde(rename = "cn")]
    Cn,
    #[serde(rename = "hcn")]
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

    pub fn value(name: &str) -> Option<CourseDataConstraint> {
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

    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
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
mod tests {
    use super::*;

    #[test]
    fn test_course_data_trophy_accessor() {
        let mut cd = CourseData::default();
        assert!(cd.trophy.is_empty());

        let trophy = TrophyData::new("Gold".to_string(), 5.0, 90.0);
        cd.trophy = vec![trophy];
        assert_eq!(cd.trophy.len(), 1);
        assert_eq!(cd.trophy[0].name(), "Gold");
    }

    #[test]
    fn test_course_data_constraint_accessor() {
        let mut cd = CourseData::default();
        assert!(cd.constraint.is_empty());

        cd.constraint = vec![CourseDataConstraint::Class, CourseDataConstraint::NoSpeed];
        assert_eq!(cd.constraint.len(), 2);
    }

    #[test]
    fn test_course_data_release() {
        let mut cd = CourseData::default();
        assert!(cd.release);

        cd.release = false;
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
        cd.release = false;
        cd.constraint = vec![CourseDataConstraint::Class, CourseDataConstraint::Ln];

        let json = serde_json::to_string(&cd).unwrap();
        let deserialized: CourseData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name(), "Test Course");
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
            CourseDataConstraint::value("grade"),
            Some(CourseDataConstraint::Class)
        );
        assert_eq!(
            CourseDataConstraint::value("no_speed"),
            Some(CourseDataConstraint::NoSpeed)
        );
        assert_eq!(
            CourseDataConstraint::value("gauge_7k"),
            Some(CourseDataConstraint::Gauge7Keys)
        );
        assert_eq!(CourseDataConstraint::value("nonexistent"), None);
    }

    // -- TrophyData tests --

    #[test]
    fn test_trophy_data_construction() {
        let trophy = TrophyData::new("Silver".to_string(), 10.0, 80.0);
        assert_eq!(trophy.name(), "Silver");
        assert_eq!(trophy.missrate, 10.0);
        assert_eq!(trophy.scorerate, 80.0);
    }

    #[test]
    fn test_trophy_data_default() {
        let trophy = TrophyData::default();
        assert_eq!(trophy.name(), "");
        assert_eq!(trophy.missrate, 0.0);
        assert_eq!(trophy.scorerate, 0.0);
    }

    #[test]
    fn test_trophy_data_setters() {
        let mut trophy = TrophyData::default();
        trophy.set_name("Gold".to_string());
        trophy.missrate = 5.0;
        trophy.scorerate = 95.0;

        assert_eq!(trophy.name(), "Gold");
        assert_eq!(trophy.missrate, 5.0);
        assert_eq!(trophy.scorerate, 95.0);
    }

    #[test]
    fn test_trophy_data_validate() {
        // Valid case: has name, missrate > 0, scorerate < 100
        let mut valid = TrophyData::new("Test".to_string(), 5.0, 90.0);
        assert!(valid.validate());

        // Invalid: no name
        let mut no_name = TrophyData {
            missrate: 5.0,
            scorerate: 90.0,
            ..TrophyData::default()
        };
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

        assert_eq!(deserialized.name(), "Diamond");
        assert_eq!(deserialized.missrate, 3.5);
        assert_eq!(deserialized.scorerate, 95.0);
    }

    // -- CourseDataConstraint serde rename tests --

    #[test]
    fn test_constraint_serializes_as_lowercase_java_name() {
        // With #[serde(rename = "...")], serialization must produce the Java name,
        // not PascalCase.
        let json = serde_json::to_string(&CourseDataConstraint::Class).unwrap();
        assert_eq!(json, r#""grade""#);

        let json = serde_json::to_string(&CourseDataConstraint::Mirror).unwrap();
        assert_eq!(json, r#""grade_mirror""#);

        let json = serde_json::to_string(&CourseDataConstraint::Gauge5Keys).unwrap();
        assert_eq!(json, r#""gauge_5k""#);

        let json = serde_json::to_string(&CourseDataConstraint::Hcn).unwrap();
        assert_eq!(json, r#""hcn""#);
    }

    #[test]
    fn test_constraint_deserializes_from_lowercase_java_name() {
        let c: CourseDataConstraint = serde_json::from_str(r#""grade""#).unwrap();
        assert_eq!(c, CourseDataConstraint::Class);

        let c: CourseDataConstraint = serde_json::from_str(r#""no_speed""#).unwrap();
        assert_eq!(c, CourseDataConstraint::NoSpeed);

        let c: CourseDataConstraint = serde_json::from_str(r#""gauge_24k""#).unwrap();
        assert_eq!(c, CourseDataConstraint::Gauge24Keys);

        let c: CourseDataConstraint = serde_json::from_str(r#""ln""#).unwrap();
        assert_eq!(c, CourseDataConstraint::Ln);
    }

    #[test]
    fn test_constraint_all_variants_serde_roundtrip() {
        for constraint in CourseDataConstraint::values() {
            let json = serde_json::to_string(constraint).unwrap();
            let restored: CourseDataConstraint = serde_json::from_str(&json).unwrap();
            assert_eq!(
                restored, *constraint,
                "Constraint {:?} should round-trip through serde with Java name",
                constraint
            );
            // Verify the serialized form matches name_str()
            let expected_json = format!("\"{}\"", constraint.name_str());
            assert_eq!(
                json,
                expected_json,
                "Constraint {:?} should serialize as {:?}",
                constraint,
                constraint.name_str()
            );
        }
    }
}

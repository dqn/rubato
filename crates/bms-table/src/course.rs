use std::fmt;

use crate::bms_table_element::BmsTableElement;

/// Style (key mode) for a course/dan.
///
/// Represents the play key configuration. Values come from external JSON
/// table headers (e.g., `"7KEYS"`, `"14KEYS"`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CourseStyle {
    /// No style specified (empty string in JSON).
    #[default]
    None,
    /// 5-key single play.
    Keys5,
    /// 7-key single play.
    Keys7,
    /// 9-key pop'n style.
    Keys9,
    /// 10-key double play (5+5).
    Keys10,
    /// 14-key double play (7+7).
    Keys14,
    /// 24-key keyboard.
    Keys24,
    /// 24-key keyboard double.
    Keys24Double,
    /// Unknown style string not in the known set.
    Other(String),
}

impl CourseStyle {
    /// Parse a style string into a `CourseStyle`.
    pub fn from_str_value(s: &str) -> Self {
        match s {
            "" => CourseStyle::None,
            "5KEYS" => CourseStyle::Keys5,
            "7KEYS" => CourseStyle::Keys7,
            "9KEYS" => CourseStyle::Keys9,
            "10KEYS" => CourseStyle::Keys10,
            "14KEYS" => CourseStyle::Keys14,
            "24KEYS" => CourseStyle::Keys24,
            "24KEYS DOUBLE" => CourseStyle::Keys24Double,
            other => CourseStyle::Other(other.to_string()),
        }
    }

    /// Convert back to the canonical string representation.
    pub fn as_str(&self) -> &str {
        match self {
            CourseStyle::None => "",
            CourseStyle::Keys5 => "5KEYS",
            CourseStyle::Keys7 => "7KEYS",
            CourseStyle::Keys9 => "9KEYS",
            CourseStyle::Keys10 => "10KEYS",
            CourseStyle::Keys14 => "14KEYS",
            CourseStyle::Keys24 => "24KEYS",
            CourseStyle::Keys24Double => "24KEYS DOUBLE",
            CourseStyle::Other(s) => s,
        }
    }
}

impl fmt::Display for CourseStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Style (appearance) for a trophy.
///
/// Values come from external JSON table headers (e.g., `"gold"`, `"silver"`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TrophyStyle {
    /// No style specified (empty string in JSON).
    #[default]
    None,
    Gold,
    Silver,
    Bronze,
    /// Unknown style string not in the known set.
    Other(String),
}

impl TrophyStyle {
    /// Parse a style string into a `TrophyStyle`.
    pub fn from_str_value(s: &str) -> Self {
        match s {
            "" => TrophyStyle::None,
            "gold" => TrophyStyle::Gold,
            "silver" => TrophyStyle::Silver,
            "bronze" => TrophyStyle::Bronze,
            other => TrophyStyle::Other(other.to_string()),
        }
    }

    /// Convert back to the canonical string representation.
    pub fn as_str(&self) -> &str {
        match self {
            TrophyStyle::None => "",
            TrophyStyle::Gold => "gold",
            TrophyStyle::Silver => "silver",
            TrophyStyle::Bronze => "bronze",
            TrophyStyle::Other(s) => s,
        }
    }
}

impl fmt::Display for TrophyStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Course {
    name: String,
    pub charts: Vec<BmsTableElement>,
    style: CourseStyle,
    pub constraint: Vec<String>,
    pub trophy: Vec<Trophy>,
}

impl Course {
    pub fn new() -> Self {
        Self {
            name: "\u{65b0}\u{898f}\u{6bb5}\u{4f4d}".to_string(),
            charts: Vec::new(),
            style: CourseStyle::None,
            constraint: Vec::new(),
            trophy: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn charts(&self) -> &[BmsTableElement] {
        &self.charts
    }

    pub fn get_style(&self) -> &CourseStyle {
        &self.style
    }

    pub fn set_style(&mut self, style: &str) {
        self.style = CourseStyle::from_str_value(style);
    }

    pub fn constraint(&self) -> &[String] {
        &self.constraint
    }
    pub fn get_trophy(&self) -> &[Trophy] {
        &self.trophy
    }
}

impl Default for Course {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Trophy {
    name: String,
    style: TrophyStyle,
    pub scorerate: f64,
    pub missrate: f64,
}

impl Trophy {
    pub fn new() -> Self {
        Self {
            name: "\u{65b0}\u{898f}\u{30c8}\u{30ed}\u{30d5}\u{30a3}\u{30fc}".to_string(),
            style: TrophyStyle::None,
            scorerate: 0.0,
            missrate: 100.0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn style(&self) -> &TrophyStyle {
        &self.style
    }

    pub fn set_style(&mut self, style: &str) {
        self.style = TrophyStyle::from_str_value(style);
    }

    pub fn scorerate(&self) -> f64 {
        self.scorerate
    }
    pub fn get_missrate(&self) -> f64 {
        self.missrate
    }
}

impl Default for Trophy {
    fn default() -> Self {
        Self::new()
    }
}

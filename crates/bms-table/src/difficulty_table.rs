use serde_json::Value;

use crate::bms_table::BmsTable;
use crate::course::Course;
use crate::difficulty_table_element::DifficultyTableElement;

pub const LEVEL_ORDER: &str = "level_order";

#[derive(Debug, Clone)]
pub struct DifficultyTable {
    pub table: BmsTable<DifficultyTableElement>,
    pub course: Vec<Vec<Course>>,
}

impl DifficultyTable {
    pub fn new() -> Self {
        Self {
            table: BmsTable::new(),
            course: Vec::new(),
        }
    }

    pub fn new_with_source_url(source_url: &str) -> Self {
        let mut dt = Self::new();
        dt.table.source_url = source_url.to_string();
        dt
    }

    pub fn elements(&self) -> Vec<DifficultyTableElement> {
        self.table.models.clone()
    }

    #[allow(dead_code)]
    fn index_of(&self, level: &str) -> i32 {
        let desc = self.level_description();
        desc.iter()
            .position(|d| d == level)
            .map_or(-1, |i| i as i32)
    }

    pub fn level_description(&self) -> Vec<String> {
        if let Some(l) = self.table.values.get(LEVEL_ORDER)
            && let Some(arr) = l.as_array()
        {
            return arr.iter().map(value_to_string).collect();
        }
        Vec::new()
    }

    pub fn set_level_description(&mut self, level_description: &[String]) {
        let arr: Vec<Value> = level_description
            .iter()
            .map(|s| Value::String(s.clone()))
            .collect();
        self.table
            .values
            .insert(LEVEL_ORDER.to_string(), Value::Array(arr));
    }

    pub fn course(&self) -> &Vec<Vec<Course>> {
        &self.course
    }
}

impl Default for DifficultyTable {
    fn default() -> Self {
        Self::new()
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_table_default_is_empty() {
        let dt = DifficultyTable::default();
        assert!(dt.elements().is_empty());
        assert!(dt.level_description().is_empty());
        assert!(dt.course().is_empty());
        assert!(dt.table.name().is_none());
    }

    #[test]
    fn difficulty_table_with_source_url() {
        let dt = DifficultyTable::new_with_source_url("https://example.com/table.html");
        assert!(dt.elements().is_empty());
    }

    #[test]
    fn set_and_get_level_description() {
        let mut dt = DifficultyTable::new();
        let levels = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        dt.set_level_description(&levels);
        assert_eq!(dt.level_description(), levels);
    }

    #[test]
    fn set_and_get_course() {
        let mut dt = DifficultyTable::new();
        assert!(dt.course().is_empty());

        let courses: Vec<Vec<Course>> = vec![vec![], vec![]];
        dt.course = courses.clone();
        assert_eq!(dt.course().len(), 2);
    }

    #[test]
    fn bms_table_element_default_fields() {
        let elem = DifficultyTableElement::new();
        assert_eq!(elem.level, "");
        assert_eq!(elem.state, 0);
        assert_eq!(elem.eval, 0);
        assert_eq!(elem.comment(), "");
        assert_eq!(elem.information(), "");
        assert_eq!(elem.proposer(), "");
        assert_eq!(elem.bmsid(), 0);
    }

    #[test]
    fn bms_table_element_set_values_roundtrip() {
        let mut elem = DifficultyTableElement::new();
        elem.set_level(Some("12"));
        elem.set_comment("test comment");
        elem.set_information("test info");
        elem.set_proposer("tester");
        elem.set_bmsid(42);

        assert_eq!(elem.level, "12");
        assert_eq!(elem.comment(), "test comment");
        assert_eq!(elem.information(), "test info");
        assert_eq!(elem.proposer(), "tester");
        assert_eq!(elem.bmsid(), 42);

        // get_values() returns a HashMap with all fields
        let values = elem.values();
        assert_eq!(values.get("level").unwrap().as_str().unwrap(), "12");
        assert_eq!(
            values.get("comment").unwrap().as_str().unwrap(),
            "test comment"
        );
    }

    #[test]
    fn value_to_string_handles_types() {
        assert_eq!(value_to_string(&Value::String("hello".into())), "hello");
        assert_eq!(value_to_string(&Value::Null), "");
        assert_eq!(value_to_string(&Value::from(42)), "42");
    }
}

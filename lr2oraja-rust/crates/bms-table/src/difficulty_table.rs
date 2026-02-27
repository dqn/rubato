use serde_json::Value;

use crate::bms_table::BmsTable;
use crate::course::Course;
use crate::difficulty_table_element::DifficultyTableElement;

pub const LEVEL_ORDER: &str = "level_order";

#[derive(Debug, Clone)]
pub struct DifficultyTable {
    pub table: BmsTable<DifficultyTableElement>,
    course: Vec<Vec<Course>>,
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
        dt.table.set_source_url(source_url);
        dt
    }

    pub fn get_elements(&self) -> Vec<DifficultyTableElement> {
        self.table.get_models().clone()
    }

    #[allow(dead_code, clippy::needless_range_loop)]
    fn index_of(&self, level: &str) -> i32 {
        let desc = self.get_level_description();
        for i in 0..desc.len() {
            if desc[i] == level {
                return i as i32;
            }
        }
        -1
    }

    #[allow(clippy::needless_range_loop)]
    pub fn get_level_description(&self) -> Vec<String> {
        if let Some(l) = self.table.get_values().get(LEVEL_ORDER)
            && let Some(arr) = l.as_array()
        {
            let mut levels: Vec<String> = Vec::with_capacity(arr.len());
            for i in 0..arr.len() {
                levels.push(value_to_string(&arr[i]));
            }
            return levels;
        }
        Vec::new()
    }

    pub fn set_level_description(&mut self, level_description: &[String]) {
        let arr: Vec<Value> = level_description
            .iter()
            .map(|s| Value::String(s.clone()))
            .collect();
        self.table
            .get_values_mut()
            .insert(LEVEL_ORDER.to_string(), Value::Array(arr));
    }

    pub fn get_course(&self) -> &Vec<Vec<Course>> {
        &self.course
    }

    pub fn set_course(&mut self, course: Vec<Vec<Course>>) {
        self.course = course;
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
        assert!(dt.get_elements().is_empty());
        assert!(dt.get_level_description().is_empty());
        assert!(dt.get_course().is_empty());
        assert!(dt.table.get_name().is_none());
    }

    #[test]
    fn difficulty_table_with_source_url() {
        let dt = DifficultyTable::new_with_source_url("https://example.com/table.html");
        assert!(dt.get_elements().is_empty());
    }

    #[test]
    fn set_and_get_level_description() {
        let mut dt = DifficultyTable::new();
        let levels = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        dt.set_level_description(&levels);
        assert_eq!(dt.get_level_description(), levels);
    }

    #[test]
    fn set_and_get_course() {
        let mut dt = DifficultyTable::new();
        assert!(dt.get_course().is_empty());

        let courses: Vec<Vec<Course>> = vec![vec![], vec![]];
        dt.set_course(courses.clone());
        assert_eq!(dt.get_course().len(), 2);
    }

    #[test]
    fn bms_table_element_default_fields() {
        let elem = DifficultyTableElement::new();
        assert_eq!(elem.get_level(), "");
        assert_eq!(elem.get_state(), 0);
        assert_eq!(elem.get_evaluation(), 0);
        assert_eq!(elem.get_comment(), "");
        assert_eq!(elem.get_information(), "");
        assert_eq!(elem.get_proposer(), "");
        assert_eq!(elem.get_bmsid(), 0);
    }

    #[test]
    fn bms_table_element_set_values_roundtrip() {
        let mut elem = DifficultyTableElement::new();
        elem.set_level(Some("12"));
        elem.set_comment("test comment");
        elem.set_information("test info");
        elem.set_proposer("tester");
        elem.set_bmsid(42);

        assert_eq!(elem.get_level(), "12");
        assert_eq!(elem.get_comment(), "test comment");
        assert_eq!(elem.get_information(), "test info");
        assert_eq!(elem.get_proposer(), "tester");
        assert_eq!(elem.get_bmsid(), 42);

        // get_values() returns a HashMap with all fields
        let values = elem.get_values();
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

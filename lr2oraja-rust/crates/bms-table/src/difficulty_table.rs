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

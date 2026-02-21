use std::collections::HashMap;

use crate::bms_table_manager_listener::BmsTableManagerListener;
use crate::difficulty_table::DifficultyTable;
use crate::difficulty_table_element::DifficultyTableElement;

pub struct BmsTableManager {
    table_list: Vec<DifficultyTable>,
    listeners: Vec<Box<dyn BmsTableManagerListener>>,
    user_list: HashMap<String, Vec<DifficultyTableElement>>,
    memo_map: HashMap<String, String>,
}

impl BmsTableManager {
    pub fn new() -> Self {
        Self {
            table_list: Vec::new(),
            listeners: Vec::new(),
            user_list: HashMap::new(),
            memo_map: HashMap::new(),
        }
    }

    pub fn add_listener(&mut self, l: Box<dyn BmsTableManagerListener>) {
        self.listeners.push(l);
    }

    pub fn fire_model_changed(&mut self) {
        for i in 0..self.listeners.len() {
            self.listeners[i].model_changed();
        }
    }

    pub fn add_bms_table(&mut self, dt: DifficultyTable) {
        self.table_list.push(dt);
        self.fire_model_changed();
    }

    pub fn remove_bms_table(&mut self, index: usize) {
        if index < self.table_list.len() {
            self.table_list.remove(index);
            self.fire_model_changed();
        }
    }

    pub fn get_bms_tables(&self) -> Vec<&DifficultyTable> {
        self.table_list.iter().collect()
    }

    pub fn get_table_list(&self) -> &Vec<DifficultyTable> {
        &self.table_list
    }

    pub fn get_table_list_mut(&mut self) -> &mut Vec<DifficultyTable> {
        &mut self.table_list
    }

    pub fn get_user_list(&self) -> &HashMap<String, Vec<DifficultyTableElement>> {
        &self.user_list
    }

    pub fn set_user_list(&mut self, user_list: HashMap<String, Vec<DifficultyTableElement>>) {
        self.user_list = user_list;
    }

    pub fn get_memo_map(&self) -> &HashMap<String, String> {
        &self.memo_map
    }

    pub fn set_memo_map(&mut self, memo_map: HashMap<String, String>) {
        self.memo_map = memo_map;
    }

    pub fn get_user_difficulty_table_elements(
        &mut self,
        name: &str,
    ) -> &mut Vec<DifficultyTableElement> {
        if !self.user_list.contains_key(name) {
            self.user_list.insert(name.to_string(), Vec::new());
        }
        self.user_list.get_mut(name).unwrap()
    }

    pub fn set_table_list(&mut self, table_list: Vec<DifficultyTable>) {
        self.table_list = table_list;
    }

    pub fn clear_all_table_elements(&mut self) {
        for i in 0..self.table_list.len() {
            self.table_list[i].table.remove_all_elements();
        }
    }
}

impl Default for BmsTableManager {
    fn default() -> Self {
        Self::new()
    }
}

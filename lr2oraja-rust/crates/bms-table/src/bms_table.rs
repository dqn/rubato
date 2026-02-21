use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub const NAME: &str = "name";
pub const SYMBOL: &str = "symbol";
pub const TAG: &str = "tag";
pub const DATA_URL: &str = "data_url";
pub const ATTR: &str = "attr";
pub const MODE: &str = "mode";

#[derive(Debug, Clone)]
pub struct BmsTable<T> {
    values: HashMap<String, Value>,
    source_url: String,
    head_url: String,
    data_url: Vec<String>,
    auto_update: bool,
    merge_configurations: HashMap<String, HashMap<String, String>>,
    lastupdate: i64,
    models: Vec<T>,
    editable: bool,
    access_count: i32,
}

impl<T: Clone> BmsTable<T> {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            source_url: String::new(),
            head_url: String::new(),
            data_url: Vec::new(),
            auto_update: true,
            merge_configurations: HashMap::new(),
            lastupdate: 0,
            models: Vec::new(),
            editable: false,
            access_count: 0,
        }
    }

    pub fn get_name(&self) -> Option<&str> {
        self.values.get(NAME).and_then(|v| v.as_str())
    }

    pub fn set_name(&mut self, name: &str) {
        self.values
            .insert(NAME.to_string(), Value::String(name.to_string()));
    }

    pub fn get_id(&self) -> Option<&str> {
        self.values.get(SYMBOL).and_then(|v| v.as_str())
    }

    pub fn set_id(&mut self, id: &str) {
        self.values
            .insert(SYMBOL.to_string(), Value::String(id.to_string()));
    }

    pub fn get_data_url(&self) -> &[String] {
        &self.data_url
    }

    pub fn set_data_url(&mut self, datas: Vec<String>) {
        self.data_url = datas;
    }

    pub fn get_merge_configurations(&self) -> &HashMap<String, HashMap<String, String>> {
        &self.merge_configurations
    }

    pub fn set_merge_configurations(
        &mut self,
        merge_configurations: HashMap<String, HashMap<String, String>>,
    ) {
        self.merge_configurations = merge_configurations;
    }

    pub fn get_tag(&self) -> Option<String> {
        if self.values.contains_key(TAG) {
            return self
                .values
                .get(TAG)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        self.get_id().map(|s| s.to_string())
    }

    pub fn set_tag(&mut self, tag: &str) {
        self.values
            .insert(TAG.to_string(), Value::String(tag.to_string()));
    }

    pub fn get_source_url(&self) -> &str {
        &self.source_url
    }

    pub fn set_source_url(&mut self, source_url: &str) {
        self.source_url = source_url.to_string();
    }

    pub fn get_models(&self) -> &Vec<T> {
        &self.models
    }

    pub fn get_models_mut(&mut self) -> &mut Vec<T> {
        &mut self.models
    }

    pub fn set_models(&mut self, models: Vec<T>) {
        self.models.clear();
        for m in models {
            self.models.push(m);
        }
    }

    pub fn add_element(&mut self, dte: T) {
        self.models.push(dte);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        self.lastupdate = now;
    }

    pub fn remove_element_at(&mut self, index: usize) {
        if index < self.models.len() {
            self.models.remove(index);
        }
    }

    pub fn remove_all_elements(&mut self) {
        self.models.clear();
    }

    pub fn is_editable(&self) -> bool {
        self.editable
    }

    pub fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }

    pub fn get_attrmap(&self) -> HashMap<String, String> {
        if let Some(v) = self.values.get(ATTR)
            && let Some(obj) = v.as_object()
        {
            let mut map = HashMap::new();
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    map.insert(k.clone(), s.to_string());
                }
            }
            return map;
        }
        HashMap::new()
    }

    pub fn set_attrmap(&mut self, attrmap: HashMap<String, String>) {
        let obj: serde_json::Map<String, Value> = attrmap
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        self.values.insert(ATTR.to_string(), Value::Object(obj));
    }

    pub fn get_head_url(&self) -> &str {
        &self.head_url
    }

    pub fn set_head_url(&mut self, head_url: &str) {
        self.head_url = head_url.to_string();
    }

    pub fn get_lastupdate(&self) -> i64 {
        self.lastupdate
    }

    pub fn get_access_count(&self) -> i32 {
        self.access_count
    }

    pub fn set_access_count(&mut self, access_count: i32) {
        self.access_count = access_count;
    }

    pub fn is_auto_update(&self) -> bool {
        self.auto_update
    }

    pub fn set_auto_update(&mut self, autoupdate: bool) {
        self.auto_update = autoupdate;
    }

    pub fn get_values(&self) -> &HashMap<String, Value> {
        &self.values
    }

    pub fn get_values_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.values
    }

    pub fn set_values(&mut self, values: &HashMap<String, Value>) {
        self.values.clear();
        for (k, v) in values {
            self.values.insert(k.clone(), v.clone());
        }
    }

    pub fn get_mode(&self) -> Option<&str> {
        self.values.get(MODE).and_then(|v| v.as_str())
    }

    pub fn set_mode(&mut self, mode: &str) {
        self.values
            .insert(MODE.to_string(), Value::String(mode.to_string()));
    }
}

impl<T: Clone> Default for BmsTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

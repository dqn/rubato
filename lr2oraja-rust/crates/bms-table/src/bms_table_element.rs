use std::collections::HashMap;

use serde_json::Value;

pub const TITLE: &str = "title";
pub const ARTIST: &str = "artist";
pub const MD5: &str = "md5";
pub const SHA256: &str = "sha256";
pub const MODE: &str = "mode";

#[derive(Debug, Clone)]
pub struct BmsTableElement {
    pub values: HashMap<String, Value>,
}

impl BmsTableElement {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn get_title(&self) -> Option<&str> {
        self.values.get(TITLE).and_then(|v| v.as_str())
    }

    pub fn set_title(&mut self, title: &str) {
        self.values
            .insert(TITLE.to_string(), Value::String(title.to_string()));
    }

    pub fn get_url(&self) -> Option<&str> {
        self.values.get("url").and_then(|v| v.as_str())
    }

    pub fn set_url(&mut self, url1: &str) {
        self.values
            .insert("url".to_string(), Value::String(url1.to_string()));
    }

    pub fn get_ipfs(&self) -> Option<&str> {
        self.values.get("ipfs").and_then(|v| v.as_str())
    }

    pub fn set_ipfs(&mut self, ipfs1: &str) {
        self.values
            .insert("ipfs".to_string(), Value::String(ipfs1.to_string()));
    }

    pub fn get_artist(&self) -> Option<&str> {
        self.values.get(ARTIST).and_then(|v| v.as_str())
    }

    pub fn set_artist(&mut self, url1name: &str) {
        self.values
            .insert(ARTIST.to_string(), Value::String(url1name.to_string()));
    }

    pub fn get_md5(&self) -> Option<&str> {
        self.values.get(MD5).and_then(|v| v.as_str())
    }

    pub fn set_md5(&mut self, hash: &str) {
        self.values
            .insert(MD5.to_string(), Value::String(hash.to_string()));
    }

    pub fn get_sha256(&self) -> Option<&str> {
        self.values.get(SHA256).and_then(|v| v.as_str())
    }

    pub fn set_sha256(&mut self, hash: &str) {
        self.values
            .insert(SHA256.to_string(), Value::String(hash.to_string()));
    }

    pub fn get_mode(&self) -> Option<&str> {
        self.values.get(MODE).and_then(|v| v.as_str())
    }

    pub fn set_mode(&mut self, mode: &str) {
        self.values
            .insert(MODE.to_string(), Value::String(mode.to_string()));
    }

    #[allow(clippy::vec_init_then_push)]
    pub fn get_parent_hash(&self) -> Option<Vec<String>> {
        let o = self.values.get("org_md5")?;
        if let Some(s) = o.as_str() {
            let mut result: Vec<String> = Vec::new();
            result.push(s.to_string());
            return Some(result);
        }
        if let Some(arr) = o.as_array() {
            let result: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            return Some(result);
        }
        None
    }

    #[allow(clippy::redundant_guards)]
    pub fn set_parent_hash(&mut self, hashes: Option<&[String]>) {
        match hashes {
            None => {
                self.values.remove("org_md5");
            }
            Some(h) if h.is_empty() => {
                self.values.remove("org_md5");
            }
            Some(h) => {
                let arr: Vec<Value> = h.iter().map(|s| Value::String(s.clone())).collect();
                self.values.insert("org_md5".to_string(), Value::Array(arr));
            }
        }
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
}

impl Default for BmsTableElement {
    fn default() -> Self {
        Self::new()
    }
}

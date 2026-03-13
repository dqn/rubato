use std::collections::HashMap;

use serde_json::Value;

use crate::bms_table_element::BmsTableElement;

pub const STATE_NEW: i32 = 1;
pub const STATE_UPDATE: i32 = 2;
pub const STATE_VOTE: i32 = 3;
pub const STATE_RECOMMEND: i32 = 4;
pub const STATE_DELETE: i32 = 5;
pub const STATE_REVIVE: i32 = 6;

/// Parameters for constructing a DifficultyTableElement.
pub struct DifficultyTableElementParams<'a> {
    pub did: &'a str,
    pub title: &'a str,
    pub bmsid: i32,
    pub url1: &'a str,
    pub url2: &'a str,
    pub comment: &'a str,
    pub hash: &'a str,
    pub ipfs: &'a str,
}

#[derive(Debug, Clone)]
pub struct DifficultyTableElement {
    pub element: BmsTableElement,
    pub state: i32,
    pub eval: i32,
    pub level: String,
    diffname: String,
    comment: String,
    info: String,
    proposer: String,
}

impl DifficultyTableElement {
    pub fn new() -> Self {
        Self {
            element: BmsTableElement::new(),
            state: 0,
            eval: 0,
            level: String::new(),
            diffname: String::new(),
            comment: String::new(),
            info: String::new(),
            proposer: String::new(),
        }
    }

    pub fn new_with_params(params: &DifficultyTableElementParams<'_>) -> Self {
        let mut dte = Self::new();
        dte.set_level(Some(params.did));
        dte.element.set_title(params.title);
        dte.set_bmsid(params.bmsid);
        dte.element.set_url(params.url1);
        dte.set_append_url(params.url2);
        dte.set_comment(params.comment);
        dte.element.set_md5(params.hash);
        dte.element.set_ipfs(params.ipfs);
        dte
    }

    pub fn set_level(&mut self, did: Option<&str>) {
        match did {
            None => {
                self.level = String::new();
            }
            Some(d) => {
                self.level = d.to_string();
            }
        }
    }

    pub fn package_url(&self) -> Option<&str> {
        self.element.values.get("url_pack").and_then(|v| v.as_str())
    }

    pub fn set_package_url(&mut self, url1sub: &str) {
        self.element
            .values
            .insert("url_pack".to_string(), Value::String(url1sub.to_string()));
    }

    pub fn package_name(&self) -> Option<&str> {
        self.element
            .values
            .get("name_pack")
            .and_then(|v| v.as_str())
    }

    pub fn set_package_name(&mut self, url1subname: &str) {
        self.element.values.insert(
            "name_pack".to_string(),
            Value::String(url1subname.to_string()),
        );
    }

    pub fn append_url(&self) -> Option<&str> {
        self.element.values.get("url_diff").and_then(|v| v.as_str())
    }

    pub fn set_append_url(&mut self, url2: &str) {
        self.element
            .values
            .insert("url_diff".to_string(), Value::String(url2.to_string()));
    }

    pub fn append_ipfs(&self) -> Option<&str> {
        self.element
            .values
            .get("ipfs_diff")
            .and_then(|v| v.as_str())
    }

    pub fn set_append_ipfs(&mut self, ipfs2: &str) {
        self.element
            .values
            .insert("ipfs_diff".to_string(), Value::String(ipfs2.to_string()));
    }

    pub fn append_artist(&self) -> &str {
        &self.diffname
    }

    pub fn set_append_artist(&mut self, url2name: &str) {
        self.diffname = url2name.to_string();
    }

    pub fn comment(&self) -> &str {
        &self.comment
    }

    pub fn set_comment(&mut self, comment1: &str) {
        self.comment = comment1.to_string();
    }

    pub fn information(&self) -> &str {
        &self.info
    }

    pub fn set_information(&mut self, comment2: &str) {
        self.info = comment2.to_string();
    }

    pub fn proposer(&self) -> &str {
        &self.proposer
    }

    pub fn set_proposer(&mut self, proposer: &str) {
        self.proposer = proposer.to_string();
    }

    pub fn bmsid(&self) -> i32 {
        let mut result: i32 = 0;
        if let Some(v) = self.element.values.get("lr2_bmsid") {
            let s = v.to_string();
            let s = s.trim_matches('"');
            if let Ok(n) = s.parse::<i32>() {
                result = n;
            }
        }
        result
    }

    pub fn set_bmsid(&mut self, bmsid: i32) {
        self.element.values.insert(
            "lr2_bmsid".to_string(),
            Value::Number(serde_json::Number::from(bmsid)),
        );
    }

    pub fn set_values(&mut self, values: &HashMap<String, Value>) {
        self.element.set_values(values);
        self.state = values.get("state").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        self.eval = values.get("eval").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        let level = values.get("level");
        self.set_level(level.map(value_to_string).as_deref().or(Some("")));
        let diffname = values.get("name_diff");
        self.set_append_artist(&diffname.map(value_to_string).unwrap_or_default());
        let comment = values.get("comment");
        self.set_comment(&comment.map(value_to_string).unwrap_or_default());
        let info = values.get("tag");
        self.set_information(&info.map(value_to_string).unwrap_or_default());
        let proposer = values.get("proposer");
        self.set_proposer(&proposer.map(value_to_string).unwrap_or_default());
    }

    pub fn values(&self) -> HashMap<String, Value> {
        let mut result = self.element.values.clone();
        result.insert("level".to_string(), Value::String(self.level.clone()));
        result.insert(
            "eval".to_string(),
            Value::Number(serde_json::Number::from(self.eval)),
        );
        result.insert(
            "state".to_string(),
            Value::Number(serde_json::Number::from(self.state)),
        );
        result.insert(
            "name_diff".to_string(),
            Value::String(self.diffname.clone()),
        );
        result.insert("comment".to_string(), Value::String(self.comment.clone()));
        result.insert("tag".to_string(), Value::String(self.info.clone()));
        if !self.proposer.is_empty() {
            result.insert("proposer".to_string(), Value::String(self.proposer.clone()));
        } else {
            result.remove("proposer");
        }
        result
    }
}

impl Default for DifficultyTableElement {
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
    fn set_values_reads_state_from_hashmap() {
        let mut dte = DifficultyTableElement::new();
        let mut values = HashMap::new();
        values.insert("state".to_string(), Value::Number(3.into()));
        dte.set_values(&values);
        assert_eq!(dte.state, 3);
    }

    #[test]
    fn set_values_reads_eval_from_hashmap() {
        let mut dte = DifficultyTableElement::new();
        let mut values = HashMap::new();
        values.insert("eval".to_string(), Value::Number(5.into()));
        dte.set_values(&values);
        assert_eq!(dte.eval, 5);
    }

    #[test]
    fn set_values_defaults_state_and_eval_to_zero_when_missing() {
        let mut dte = DifficultyTableElement::new();
        dte.state = 99;
        dte.eval = 99;
        let values = HashMap::new();
        dte.set_values(&values);
        assert_eq!(dte.state, 0);
        assert_eq!(dte.eval, 0);
    }

    #[test]
    fn set_values_reads_level_and_other_fields() {
        let mut dte = DifficultyTableElement::new();
        let mut values = HashMap::new();
        values.insert("level".to_string(), Value::String("★12".to_string()));
        values.insert("name_diff".to_string(), Value::String("artist".to_string()));
        values.insert(
            "comment".to_string(),
            Value::String("good chart".to_string()),
        );
        values.insert("tag".to_string(), Value::String("info tag".to_string()));
        values.insert("proposer".to_string(), Value::String("someone".to_string()));
        values.insert("state".to_string(), Value::Number(2.into()));
        values.insert("eval".to_string(), Value::Number(4.into()));
        dte.set_values(&values);
        assert_eq!(dte.level, "★12");
        assert_eq!(dte.append_artist(), "artist");
        assert_eq!(dte.comment(), "good chart");
        assert_eq!(dte.information(), "info tag");
        assert_eq!(dte.proposer(), "someone");
        assert_eq!(dte.state, 2);
        assert_eq!(dte.eval, 4);
    }

    #[test]
    fn values_roundtrip_preserves_state_and_eval() {
        let mut dte = DifficultyTableElement::new();
        let mut input = HashMap::new();
        input.insert("state".to_string(), Value::Number(STATE_VOTE.into()));
        input.insert("eval".to_string(), Value::Number(7.into()));
        dte.set_values(&input);
        let output = dte.values();
        assert_eq!(
            output.get("state").and_then(|v| v.as_i64()),
            Some(STATE_VOTE as i64)
        );
        assert_eq!(output.get("eval").and_then(|v| v.as_i64()), Some(7));
    }
}

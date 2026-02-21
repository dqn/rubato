use std::collections::HashMap;

use serde_json::Value;

use crate::bms_table_element::BmsTableElement;

pub const STATE_NEW: i32 = 1;
pub const STATE_UPDATE: i32 = 2;
pub const STATE_VOTE: i32 = 3;
pub const STATE_RECOMMEND: i32 = 4;
pub const STATE_DELETE: i32 = 5;
pub const STATE_REVIVE: i32 = 6;

#[derive(Debug, Clone)]
pub struct DifficultyTableElement {
    pub element: BmsTableElement,
    state: i32,
    eval: i32,
    level: String,
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

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_params(
        did: &str,
        title: &str,
        bmsid: i32,
        url1: &str,
        url2: &str,
        comment: &str,
        hash: &str,
        ipfs: &str,
    ) -> Self {
        let mut dte = Self::new();
        dte.set_level(Some(did));
        dte.element.set_title(title);
        dte.set_bmsid(bmsid);
        dte.element.set_url(url1);
        dte.set_append_url(url2);
        dte.set_comment(comment);
        dte.element.set_md5(hash);
        dte.element.set_ipfs(ipfs);
        dte
    }

    pub fn get_state(&self) -> i32 {
        self.state
    }

    pub fn set_state(&mut self, id: i32) {
        self.state = id;
    }

    pub fn get_level(&self) -> &str {
        &self.level
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

    pub fn get_evaluation(&self) -> i32 {
        self.eval
    }

    pub fn set_evaluation(&mut self, eval: i32) {
        self.eval = eval;
    }

    pub fn get_package_url(&self) -> Option<&str> {
        self.element
            .get_values()
            .get("url_pack")
            .and_then(|v| v.as_str())
    }

    pub fn set_package_url(&mut self, url1sub: &str) {
        self.element
            .get_values_mut()
            .insert("url_pack".to_string(), Value::String(url1sub.to_string()));
    }

    pub fn get_package_name(&self) -> Option<&str> {
        self.element
            .get_values()
            .get("name_pack")
            .and_then(|v| v.as_str())
    }

    pub fn set_package_name(&mut self, url1subname: &str) {
        self.element.get_values_mut().insert(
            "name_pack".to_string(),
            Value::String(url1subname.to_string()),
        );
    }

    pub fn get_append_url(&self) -> Option<&str> {
        self.element
            .get_values()
            .get("url_diff")
            .and_then(|v| v.as_str())
    }

    pub fn set_append_url(&mut self, url2: &str) {
        self.element
            .get_values_mut()
            .insert("url_diff".to_string(), Value::String(url2.to_string()));
    }

    pub fn get_append_ipfs(&self) -> Option<&str> {
        self.element
            .get_values()
            .get("ipfs_diff")
            .and_then(|v| v.as_str())
    }

    pub fn set_append_ipfs(&mut self, ipfs2: &str) {
        self.element
            .get_values_mut()
            .insert("ipfs_diff".to_string(), Value::String(ipfs2.to_string()));
    }

    pub fn get_append_artist(&self) -> &str {
        &self.diffname
    }

    pub fn set_append_artist(&mut self, url2name: &str) {
        self.diffname = url2name.to_string();
    }

    pub fn get_comment(&self) -> &str {
        &self.comment
    }

    pub fn set_comment(&mut self, comment1: &str) {
        self.comment = comment1.to_string();
    }

    pub fn get_information(&self) -> &str {
        &self.info
    }

    pub fn set_information(&mut self, comment2: &str) {
        self.info = comment2.to_string();
    }

    pub fn get_proposer(&self) -> &str {
        &self.proposer
    }

    pub fn set_proposer(&mut self, proposer: &str) {
        self.proposer = proposer.to_string();
    }

    pub fn get_bmsid(&self) -> i32 {
        let mut result: i32 = 0;
        if let Some(v) = self.element.get_values().get("lr2_bmsid") {
            let s = v.to_string();
            let s = s.trim_matches('"');
            if let Ok(n) = s.parse::<i32>() {
                result = n;
            }
        }
        result
    }

    pub fn set_bmsid(&mut self, bmsid: i32) {
        self.element.get_values_mut().insert(
            "lr2_bmsid".to_string(),
            Value::Number(serde_json::Number::from(bmsid)),
        );
    }

    pub fn set_values(&mut self, values: &HashMap<String, Value>) {
        self.element.set_values(values);
        let statevalue: i32 = 0;
        self.state = statevalue;

        let evalvalue: i32 = 0;
        self.eval = evalvalue;

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

    pub fn get_values(&self) -> HashMap<String, Value> {
        let mut result = self.element.get_values().clone();
        result.insert(
            "level".to_string(),
            Value::String(self.get_level().to_string()),
        );
        result.insert(
            "eval".to_string(),
            Value::Number(serde_json::Number::from(self.get_evaluation())),
        );
        result.insert(
            "state".to_string(),
            Value::Number(serde_json::Number::from(self.get_state())),
        );
        result.insert(
            "name_diff".to_string(),
            Value::String(self.get_append_artist().to_string()),
        );
        result.insert(
            "comment".to_string(),
            Value::String(self.get_comment().to_string()),
        );
        result.insert(
            "tag".to_string(),
            Value::String(self.get_information().to_string()),
        );
        if !self.get_proposer().is_empty() {
            result.insert(
                "proposer".to_string(),
                Value::String(self.get_proposer().to_string()),
            );
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

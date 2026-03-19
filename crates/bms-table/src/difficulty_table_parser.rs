use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use url::Url;

use anyhow::bail;

use crate::bms_table_element::BmsTableElement;
use crate::course::{Course, Trophy};
use crate::difficulty_table::DifficultyTable;
use crate::difficulty_table_element::DifficultyTableElement;

struct HtmlRegexes {
    br: Regex,
    anchor: Regex,
    avg_judge: Regex,
}

fn html_regexes() -> &'static HtmlRegexes {
    static REGEXES: OnceLock<HtmlRegexes> = OnceLock::new();
    REGEXES.get_or_init(|| HtmlRegexes {
        br: Regex::new(r"(?i)<br\s*/?>").expect("valid br regex"),
        anchor: Regex::new(r#"(?i)<a\s[^>]*>|</a>"#).expect("valid anchor regex"),
        avg_judge: Regex::new(r"Avg:.*JUDGE:[A-Z]+\s*").expect("valid avg_judge regex"),
    })
}

/// Maximum allowed HTTP response size (64 MB).
/// Prevents memory exhaustion from malicious or misconfigured servers.
const MAX_RESPONSE_SIZE: u64 = 64 * 1024 * 1024;

pub struct DifficultyTableParser {
    data: HashMap<String, Vec<String>>,
}

impl DifficultyTableParser {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    fn http_client() -> Result<reqwest::blocking::Client> {
        Ok(reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?)
    }

    pub fn contains_header(&mut self, urlname: &str) -> bool {
        self.get_meta_tag(urlname, "bmstable").is_some()
    }

    pub fn alternate_bms_table_url(&mut self, urlname: &str) -> Option<String> {
        self.get_meta_tag(urlname, "bmstable-alt")
    }

    // NOTE: Uses reqwest::blocking::Client with 30s timeout. Callers must ensure this runs
    // on a background thread, not the main/render thread, to avoid UI freezes.
    fn read_all_lines(&self, urlname: &str) -> Option<Vec<String>> {
        match Self::http_client().and_then(|c| Ok(c.get(urlname).send()?.error_for_status()?)) {
            Ok(response) => {
                if let Some(content_length) = response.content_length() {
                    if content_length > MAX_RESPONSE_SIZE {
                        log::error!(
                            "\u{96e3}\u{6613}\u{5ea6}\u{8868}\u{30b5}\u{30a4}\u{30c8}\u{89e3}\u{6790}\u{4e2d}\u{306e}\u{4f8b}\u{5916}:response too large ({} bytes)",
                            content_length
                        );
                        return None;
                    }
                }
                match response.bytes() {
                    Ok(bytes) => {
                        if bytes.len() as u64 > MAX_RESPONSE_SIZE {
                            log::error!(
                                "\u{96e3}\u{6613}\u{5ea6}\u{8868}\u{30b5}\u{30a4}\u{30c8}\u{89e3}\u{6790}\u{4e2d}\u{306e}\u{4f8b}\u{5916}:response too large ({} bytes)",
                                bytes.len()
                            );
                            return None;
                        }
                        let text = Self::decode_bytes_with_charset(&bytes);
                        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
                        Some(lines)
                    }
                    Err(e) => {
                        log::error!(
                            "\u{96e3}\u{6613}\u{5ea6}\u{8868}\u{30b5}\u{30a4}\u{30c8}\u{89e3}\u{6790}\u{4e2d}\u{306e}\u{4f8b}\u{5916}:{}",
                            e
                        );
                        None
                    }
                }
            }
            Err(e) => {
                log::error!(
                    "\u{96e3}\u{6613}\u{5ea6}\u{8868}\u{30b5}\u{30a4}\u{30c8}\u{89e3}\u{6790}\u{4e2d}\u{306e}\u{4f8b}\u{5916}:{}",
                    e
                );
                None
            }
        }
    }

    /// Detect charset from `<meta>` tags in raw bytes and decode accordingly.
    /// Falls back to Shift_JIS for non-UTF-8 content (common for Japanese BMS tables).
    fn decode_bytes_with_charset(bytes: &[u8]) -> String {
        // Try UTF-8 first; if it works cleanly, check for an explicit charset override.
        let tentative = String::from_utf8_lossy(bytes);

        // Look for <meta http-equiv="content-type" content="...charset=XXX">
        // or <meta charset="XXX"> in the first few KB.
        let mut end = tentative.len().min(4096);
        // Walk backward to find a char boundary (MSRV-safe alternative to floor_char_boundary)
        while end > 0 && !tentative.is_char_boundary(end) {
            end -= 1;
        }
        let probe = &tentative[..end];
        let probe_lower = probe.to_ascii_lowercase();
        let charset = if let Some(idx) = probe_lower.find("charset=") {
            let rest = &probe[idx + 8..];
            // Skip optional leading quote (charset="..." or charset='...')
            let rest = rest
                .strip_prefix('"')
                .or_else(|| rest.strip_prefix('\''))
                .unwrap_or(rest);
            let end = rest
                .find(|c: char| c == '"' || c == '\'' || c == ';' || c == '>' || c.is_whitespace())
                .unwrap_or(rest.len());
            Some(rest[..end].trim().to_lowercase())
        } else {
            None
        };

        match charset.as_deref() {
            Some("utf-8" | "utf8") | None => {
                // If UTF-8 decoded cleanly (no replacement chars), use it.
                if !tentative.contains('\u{FFFD}') {
                    return tentative.into_owned();
                }
                // Contains replacement chars and no explicit charset; try Shift_JIS.
                let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
                decoded.into_owned()
            }
            Some("shift_jis" | "shift-jis" | "sjis" | "x-sjis" | "ms932" | "windows-31j") => {
                let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
                decoded.into_owned()
            }
            Some("euc-jp" | "euc_jp" | "eucjp") => {
                let (decoded, _, _) = encoding_rs::EUC_JP.decode(bytes);
                decoded.into_owned()
            }
            Some("iso-2022-jp") => {
                let (decoded, _, _) = encoding_rs::ISO_2022_JP.decode(bytes);
                decoded.into_owned()
            }
            Some(other) => {
                // Try to look up the encoding by label.
                if let Some(encoding) = encoding_rs::Encoding::for_label(other.as_bytes()) {
                    let (decoded, _, _) = encoding.decode(bytes);
                    decoded.into_owned()
                } else {
                    tentative.into_owned()
                }
            }
        }
    }

    fn get_meta_tag(&mut self, urlname: &str, name: &str) -> Option<String> {
        if !self.data.contains_key(urlname)
            && let Some(lines) = self.read_all_lines(urlname)
        {
            self.data.insert(urlname.to_string(), lines);
        }
        let lines = self.data.get(urlname)?;
        let search = format!("<meta name=\"{}\"", name);
        for line in lines {
            if line.to_lowercase().contains(&search) {
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() > 3 {
                    return Some(parts[3].to_string());
                }
            }
        }
        None
    }

    pub fn decode(&mut self, b: bool, diff: &mut DifficultyTable) -> Result<()> {
        let urlname = diff.table.source_url.clone();
        let mut tableurl: Option<String> = None;
        let mut _enc: Option<String> = None;
        if urlname.is_empty() {
            tableurl = Some(diff.table.head_url.clone());
        } else {
            if !self.data.contains_key(&urlname)
                && let Some(lines) = self.read_all_lines(&urlname)
            {
                self.data.insert(urlname.clone(), lines);
            }
            let lines = self
                .data
                .get(&urlname)
                .ok_or_else(|| anyhow::anyhow!("Failed to read URL"))?
                .clone();
            for line in &lines {
                if line
                    .to_lowercase()
                    .contains("<meta http-equiv=\"content-type\"")
                {
                    // Java parity: fragile double-quote splitting for meta-tag attribute extraction.
                    // May miss tables with single-quoted or otherwise-formatted HTML meta tags.
                    let parts: Vec<&str> = line.split('"').collect();
                    if parts.len() > 3 {
                        let str_val = parts[3];
                        if let Some(idx) = str_val.find("charset=") {
                            _enc = Some(str_val[idx + 8..].to_string());
                        }
                    }
                }
                if line.to_lowercase().contains("<meta name=\"bmstable\"") {
                    let parts: Vec<&str> = line.split('"').collect();
                    if parts.len() > 3 {
                        tableurl = Some(parts[3].to_string());
                    }
                }
            }
        }
        if let Some(ref tu) = tableurl {
            let abs_url = self.get_absolute_url(&urlname, tu);
            self.decode_json_table(diff, &abs_url, b)?;
            diff.table.source_url = urlname.clone();
        } else if let Some(ref enc) = _enc {
            let _enc_upper = enc.to_uppercase();
            // encoding normalization (unused in current code path)
        }
        Ok(())
    }

    fn get_absolute_url(&self, source: &str, path: &str) -> String {
        // If path is already an absolute URL, return as-is.
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_string();
        }
        // Use proper URL joining to handle root-relative (/path), parent-relative
        // (../path), and current-relative (./path or bare path) links correctly.
        if let Ok(base) = Url::parse(source)
            && let Ok(resolved) = base.join(path)
        {
            return resolved.to_string();
        }
        // Fallback: simple concatenation with directory base.
        let urldir = if let Some(idx) = source.rfind('/') {
            &source[..idx + 1]
        } else {
            source
        };
        let p = path.strip_prefix("./").unwrap_or(path);
        format!("{}{}", urldir, p)
    }

    pub fn decode_json_table(
        &self,
        dt: &mut DifficultyTable,
        jsonheader_url: &str,
        save_elements: bool,
    ) -> Result<()> {
        self.decode_json_table_header_from_url(dt, jsonheader_url)?;
        let urls = dt.table.data_url.clone();
        if save_elements {
            dt.table.remove_all_elements();
            let mut elements: Vec<DifficultyTableElement> = Vec::new();
            let mut levels: Vec<String> = Vec::new();
            for url in &urls {
                let conf = dt
                    .table
                    .merge_configurations
                    .get(url)
                    .cloned()
                    .unwrap_or_default();
                let mut table = DifficultyTable::new();

                let source_url = dt.table.source_url.clone();
                let head_url = dt.table.head_url.clone();
                let base_url = if source_url.is_empty() {
                    head_url.clone()
                } else {
                    self.get_absolute_url(&source_url, &head_url)
                };
                let data_url = self.get_absolute_url(&base_url, url);

                self.decode_json_table_data_from_url(&mut table, &data_url)?;
                for l in &table.level_description() {
                    levels.push(l.clone());
                }
                for dte in table.elements() {
                    let level_conf = conf.get(dte.level.as_str());
                    if level_conf.is_none_or(|v| !v.is_empty()) {
                        let contains = elements.iter().any(|e| {
                            e.element.md5() == dte.element.md5() && e.element.md5().is_some()
                        });
                        if !contains {
                            let mut dte = dte.clone();
                            if let Some(new_level) = conf.get(dte.level.as_str()) {
                                dte.set_level(Some(new_level));
                            }
                            elements.push(dte);
                        }
                    }
                }
            }
            if dt.level_description().is_empty() {
                dt.set_level_description(&levels);
            }
            dt.table.set_models(elements);
        }
        Ok(())
    }

    pub fn decode_json_table_header_from_file(
        &self,
        dt: &mut DifficultyTable,
        jsonheader: &Path,
    ) -> Result<()> {
        let content = fs::read_to_string(jsonheader)?;
        let result: HashMap<String, Value> = serde_json::from_str(&content)?;
        self.decode_json_table_header_internal(dt, &result)?;
        Ok(())
    }

    pub fn decode_json_table_header_from_url(
        &self,
        dt: &mut DifficultyTable,
        jsonheader_url: &str,
    ) -> Result<()> {
        let response = Self::http_client()?
            .get(jsonheader_url)
            .send()?
            .error_for_status()?;
        if let Some(content_length) = response.content_length() {
            if content_length > MAX_RESPONSE_SIZE {
                bail!("Response too large: {} bytes", content_length);
            }
        }
        let bytes = response.bytes()?;
        if bytes.len() as u64 > MAX_RESPONSE_SIZE {
            bail!("Response too large: {} bytes", bytes.len());
        }
        let text = Self::decode_bytes_with_charset(&bytes);
        let result: HashMap<String, Value> = serde_json::from_str(&text)?;
        self.decode_json_table_header_internal(dt, &result)?;
        dt.table.head_url = jsonheader_url.to_string();
        Ok(())
    }

    fn decode_json_table_header_internal(
        &self,
        dt: &mut DifficultyTable,
        result: &HashMap<String, Value>,
    ) -> Result<()> {
        dt.table.set_values(result);
        let dataurl = result.get("data_url");
        if let Some(du) = dataurl {
            if let Some(s) = du.as_str() {
                dt.table.data_url = vec![s.to_string()];
            }
            if let Some(arr) = du.as_array() {
                let urls: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                dt.table.data_url = urls;
            }
        }
        let mut mergerule: HashMap<String, HashMap<String, String>> = HashMap::new();
        let merge: Vec<HashMap<String, String>> = if let Some(dr) = result.get("data_rule") {
            if let Some(arr) = dr.as_array() {
                arr.iter()
                    .filter_map(|v| {
                        if let Some(obj) = v.as_object() {
                            let map: HashMap<String, String> = obj
                                .iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect();
                            Some(map)
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        // Java parity: zip truncates to the shorter side. When merge has fewer
        // entries than data_urls, extra URLs get no merge config (empty map default).
        let data_urls = dt.table.data_url.clone();
        for (url, m) in data_urls.iter().zip(merge.iter()) {
            mergerule.insert(url.clone(), m.clone());
        }
        dt.table.merge_configurations = mergerule;
        let mut courses: Vec<Vec<Course>> = Vec::new();
        if let Some(course_val) = result.get("course") {
            if let Some(course_arr) = course_val.as_array()
                && !course_arr.is_empty()
            {
                if course_arr[0].is_array() {
                    // List<List<Map>>
                    for course_list_val in course_arr {
                        if let Some(course_list) = course_list_val.as_array() {
                            let mut l: Vec<Course> = Vec::new();
                            for grade in course_list {
                                if let Some(grade_obj) = grade.as_object() {
                                    let gr = parse_course(grade_obj)?;
                                    l.push(gr);
                                }
                            }
                            courses.push(l);
                        }
                    }
                } else if course_arr[0].is_object() {
                    // List<Map>
                    let mut l: Vec<Course> = Vec::new();
                    for grade in course_arr {
                        if let Some(grade_obj) = grade.as_object() {
                            let gr = parse_course(grade_obj)?;
                            l.push(gr);
                        }
                    }
                    courses.push(l);
                }
            }
        } else if let Some(grade_val) = result.get("grade")
            && let Some(grade_arr) = grade_val.as_array()
        {
            let mut l: Vec<Course> = Vec::new();
            for grade in grade_arr {
                if let Some(grade_obj) = grade.as_object() {
                    let mut gr = Course::new();
                    if let Some(name) = grade_obj.get("name").and_then(|v| v.as_str()) {
                        gr.set_name(name);
                    }
                    let mut charts: Vec<BmsTableElement> = Vec::new();
                    if let Some(md5_arr) = grade_obj.get("md5").and_then(|v| v.as_array()) {
                        for md5 in md5_arr {
                            if let Some(md5_str) = md5.as_str() {
                                let mut dte = DifficultyTableElement::new();
                                dte.element.set_md5(md5_str);
                                charts.push(dte.element);
                            }
                        }
                    }
                    gr.charts = charts;
                    if let Some(style) = grade_obj.get("style").and_then(|v| v.as_str()) {
                        gr.set_style(style);
                    }
                    gr.constraint = vec!["grade_mirror".to_string(), "gauge_lr2".to_string()];
                    l.push(gr);
                }
            }
            courses.push(l);
        }
        dt.course = courses;
        if result.get("name").is_none() || result.get("symbol").is_none() {
            return Err(anyhow::anyhow!(
                "\u{30d8}\u{30c3}\u{30c0}\u{90e8}\u{306e}\u{60c5}\u{5831}\u{304c}\u{4e0d}\u{8db3}\u{3057}\u{3066}\u{3044}\u{307e}\u{3059}"
            ));
        }
        Ok(())
    }

    pub fn decode_json_table_data_from_file(
        &self,
        dt: &mut DifficultyTable,
        jsondata: &Path,
    ) -> Result<()> {
        let content = fs::read_to_string(jsondata)?;
        let result: Vec<HashMap<String, Value>> = serde_json::from_str(&content)?;
        self.decode_json_table_data_internal(dt, &result, true);
        Ok(())
    }

    pub fn decode_json_table_data_from_url(
        &self,
        dt: &mut DifficultyTable,
        jsondata_url: &str,
    ) -> Result<()> {
        log::info!(
            "\u{96e3}\u{6613}\u{5ea6}\u{8868}\u{30c7}\u{30fc}\u{30bf}\u{8aad}\u{307f}\u{8fbc}\u{307f} - {}",
            jsondata_url
        );
        let response = Self::http_client()?
            .get(jsondata_url)
            .send()?
            .error_for_status()?;
        if let Some(content_length) = response.content_length() {
            if content_length > MAX_RESPONSE_SIZE {
                bail!("Response too large: {} bytes", content_length);
            }
        }
        let bytes = response.bytes()?;
        if bytes.len() as u64 > MAX_RESPONSE_SIZE {
            bail!("Response too large: {} bytes", bytes.len());
        }
        let text = Self::decode_bytes_with_charset(&bytes);
        let result: Vec<HashMap<String, Value>> = serde_json::from_str(&text)?;
        self.decode_json_table_data_internal(dt, &result, false);
        Ok(())
    }

    #[allow(clippy::needless_range_loop)]
    fn decode_json_table_data_internal(
        &self,
        dt: &mut DifficultyTable,
        result: &[HashMap<String, Value>],
        accept: bool,
    ) {
        dt.table.remove_all_elements();
        let mut levelorder: Vec<String> = Vec::new();
        for m in result {
            let has_level = m.get("level").is_some();
            let md5_ok = m
                .get("md5")
                .and_then(|v| v.as_str())
                .map(|s| s.len() > 24)
                .unwrap_or(false);
            let sha256_ok = m
                .get("sha256")
                .and_then(|v| v.as_str())
                .map(|s| s.len() > 24)
                .unwrap_or(false);

            if accept || (has_level && (md5_ok || sha256_ok)) {
                let mut dte = DifficultyTableElement::new();
                dte.set_values(m);
                if dte.element.mode().is_none()
                    && let Some(mode) = dt.table.mode()
                {
                    dte.element.set_mode(mode);
                }

                let level = m.get("level").map(value_to_string).unwrap_or_default();
                if !levelorder.contains(&level) {
                    levelorder.push(level);
                }
                dt.table.add_element(dte);
            } else {
                let title = m.get("title").map(value_to_string).unwrap_or_default();
                let level = m.get("level").map(value_to_string).unwrap_or_default();
                let md5 = m.get("md5").map(value_to_string).unwrap_or_default();
                log::info!(
                    "{}\u{306e}\u{8b5c}\u{9762}\u{5b9a}\u{7fa9}\u{306b}\u{4e0d}\u{5099}\u{304c}\u{3042}\u{308a}\u{307e}\u{3059} - level:{}  md5:{}",
                    title,
                    level,
                    md5
                );
            }
        }

        if dt.level_description().is_empty() {
            dt.set_level_description(&levelorder);
        }
    }

    pub fn encode_json_table_header(&self, dt: &DifficultyTable, jsonheader: &Path) {
        let result = (|| -> Result<()> {
            let mut header: HashMap<String, Value> = HashMap::new();
            header.insert(
                "name".to_string(),
                Value::String(dt.table.name().unwrap_or_default().to_string()),
            );
            header.insert(
                "symbol".to_string(),
                Value::String(dt.table.id().unwrap_or_default().to_string()),
            );
            header.insert(
                "tag".to_string(),
                Value::String(dt.table.tag().unwrap_or_default()),
            );
            let levels: Vec<Value> = dt
                .level_description()
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect();
            header.insert("level_order".to_string(), Value::Array(levels));
            if dt.table.data_url.len() > 1 {
                let arr: Vec<Value> = dt
                    .table
                    .data_url
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect();
                header.insert("data_url".to_string(), Value::Array(arr));
            } else if dt.table.data_url.len() == 1 {
                header.insert(
                    "data_url".to_string(),
                    Value::String(dt.table.data_url[0].clone()),
                );
            }
            let attrmap = dt.table.attrmap();
            if !attrmap.is_empty() {
                let obj: serde_json::Map<String, Value> = attrmap
                    .into_iter()
                    .map(|(k, v)| (k, Value::String(v)))
                    .collect();
                header.insert("attr".to_string(), Value::Object(obj));
            }

            let course = dt.course();
            if !course.is_empty() {
                let mut all_groups: Vec<Value> = Vec::new();
                for group in course {
                    let mut grade: Vec<Value> = Vec::new();
                    for g in group {
                        let mut m: serde_json::Map<String, Value> = serde_json::Map::new();
                        m.insert("name".to_string(), Value::String(g.name().to_string()));
                        m.insert("style".to_string(), Value::String(g.style.clone()));
                        if !g.constraint.is_empty() {
                            let arr: Vec<Value> = g
                                .constraint
                                .iter()
                                .map(|s| Value::String(s.clone()))
                                .collect();
                            m.insert("constraint".to_string(), Value::Array(arr));
                        }
                        if !g.trophy.is_empty() {
                            let arr: Vec<Value> = g
                                .trophy
                                .iter()
                                .map(|t| {
                                    let mut tm: serde_json::Map<String, Value> =
                                        serde_json::Map::new();
                                    tm.insert(
                                        "name".to_string(),
                                        Value::String(t.name().to_string()),
                                    );
                                    tm.insert(
                                        "style".to_string(),
                                        Value::String(t.style().to_string()),
                                    );
                                    tm.insert(
                                        "scorerate".to_string(),
                                        Value::Number(
                                            serde_json::Number::from_f64(t.scorerate)
                                                .unwrap_or_else(|| {
                                                    serde_json::Number::from_f64(0.0)
                                                        .expect("0.0 is a valid f64")
                                                }),
                                        ),
                                    );
                                    tm.insert(
                                        "missrate".to_string(),
                                        Value::Number(
                                            serde_json::Number::from_f64(t.missrate)
                                                .unwrap_or_else(|| {
                                                    serde_json::Number::from_f64(100.0)
                                                        .expect("100.0 is a valid f64")
                                                }),
                                        ),
                                    );
                                    Value::Object(tm)
                                })
                                .collect();
                            m.insert("trophy".to_string(), Value::Array(arr));
                        }
                        if !g.charts.is_empty() {
                            let arr: Vec<Value> = g
                                .charts
                                .iter()
                                .map(|c| {
                                    Value::Object(
                                        c.values
                                            .iter()
                                            .map(|(k, v)| (k.clone(), v.clone()))
                                            .collect(),
                                    )
                                })
                                .collect();
                            m.insert("charts".to_string(), Value::Array(arr));
                        }
                        grade.push(Value::Object(m));
                    }
                    all_groups.push(Value::Array(grade));
                }
                // If only one group, serialize as flat array for backwards compat
                if all_groups.len() == 1 {
                    header.insert(
                        "course".to_string(),
                        all_groups
                            .into_iter()
                            .next()
                            .expect("invariant: len == 1 checked above"),
                    );
                } else {
                    header.insert("course".to_string(), Value::Array(all_groups));
                }
            }

            let json = serde_json::to_string(&header)?;
            fs::write(jsonheader, json)?;
            Ok(())
        })();
        if let Err(e) = result {
            log::error!(
                "\u{96e3}\u{6613}\u{5ea6}\u{8868}\u{306e}\u{4fdd}\u{5b58}\u{4e2d}\u{306e}\u{4f8b}\u{5916}:{}",
                e
            );
        }
    }

    pub fn encode_json_table_data(
        &self,
        dt: &mut DifficultyTable,
        jsonheader: &Path,
        jsondata: &Path,
    ) {
        let result = (|| -> Result<()> {
            let data_filename = jsondata
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            dt.table.data_url = vec![data_filename];
            self.encode_json_table_header(dt, jsonheader);
            let mut datas: Vec<HashMap<String, Value>> = Vec::new();
            for te in dt.elements() {
                datas.push(te.values());
            }
            let json = serde_json::to_string_pretty(&datas)?;
            fs::write(jsondata, json)?;
            Ok(())
        })();
        if let Err(e) = result {
            log::error!(
                "\u{96e3}\u{6613}\u{5ea6}\u{8868}\u{306e}\u{4fdd}\u{5b58}\u{4e2d}\u{306e}\u{4f8b}\u{5916}:{}",
                e
            );
        }
    }

    #[allow(dead_code, clippy::needless_range_loop)]
    fn parse_difficulty_table(
        &self,
        dt: &mut DifficultyTable,
        mark: &str,
        reader: Box<dyn BufRead>,
        save_element: bool,
    ) -> Result<()> {
        let mut diff = false;
        let mut state: i32 = -1;
        let mut result: Vec<DifficultyTableElement> = Vec::new();
        let mut dte: Option<DifficultyTableElement> = None;
        dt.table.remove_all_elements();
        let first = Regex::new(&format!(
            r#"\s*\[\s*\d+,\s*"{}.*"\s*,.*"#,
            regex::escape(mark)
        ))?;
        let br = BufReader::new(reader);
        for line_result in br.lines() {
            let line = line_result?;
            if line.contains("var mname = [") {
                diff = true;
            }
            if line.contains("</script>") {
                diff = false;
            }
            if diff && state == -1 && first.is_match(&line) {
                let mut new_dte = DifficultyTableElement::new();
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() > 1 {
                    let did = &parts[1][mark.len()..];
                    new_dte.set_level(Some(did));
                }
                dte = Some(new_dte);
                state = 0;
            }

            if state >= 0 {
                match state {
                    0 => {
                        state += 1;
                    }
                    1 => {
                        let parts: Vec<&str> = line.split('"').collect();
                        if parts.len() > 1
                            && let Some(ref mut d) = dte
                        {
                            d.element.set_title(parts[1]);
                        }
                        state += 1;
                    }
                    2 => {
                        let parts: Vec<&str> = line.split('"').collect();
                        if parts.len() > 1 {
                            let cleaned = parts[1].replace(' ', "");
                            if let Ok(bmsid) = cleaned.parse::<i32>()
                                && let Some(ref mut d) = dte
                            {
                                d.set_bmsid(bmsid);
                            }
                        }
                        state += 1;
                    }
                    3 => {
                        let parts: Vec<&str> = line.split('"').collect();
                        if parts.len() > 1 {
                            let split: Vec<&str> = parts[1].split('\'').collect();
                            if split.len() > 2
                                && let Some(ref mut d) = dte
                            {
                                d.element.set_url(split[1]);
                            }
                            let regexes = html_regexes();
                            let split_br: Vec<&str> = regexes.br.split(parts[1]).collect();
                            let artist = regexes.anchor.replace_all(split_br[0], "");
                            if let Some(ref mut d) = dte {
                                d.element.set_artist(&artist);
                            }
                            if split_br.len() > 1 {
                                let split2: Vec<&str> = split_br[1].split('\'').collect();
                                if split2.len() > 2
                                    && let Some(ref mut d) = dte
                                {
                                    d.set_package_url(split2[1]);
                                }
                                let pkg_name = regexes.anchor.replace_all(split_br[1], "");
                                if let Some(ref mut d) = dte {
                                    d.set_package_name(&pkg_name);
                                }
                            }
                        }
                        state += 1;
                    }
                    4 => {
                        let parts: Vec<&str> = line.split('"').collect();
                        if parts.len() > 1 {
                            let split3: Vec<&str> = parts[1].split('\'').collect();
                            if split3.len() > 2
                                && let Some(ref mut d) = dte
                            {
                                d.set_append_url(split3[1]);
                            }
                            let append_artist = html_regexes().anchor.replace_all(parts[1], "");
                            if let Some(ref mut d) = dte {
                                d.set_append_artist(&append_artist);
                            }
                        }
                        state += 1;
                    }
                    5 => {
                        let parts: Vec<&str> = line.split('"').collect();
                        if parts.len() > 1 {
                            let comment = html_regexes().avg_judge.replace_all(parts[1], "");
                            if let Some(ref mut d) = dte {
                                d.set_comment(&comment);
                            }
                        }
                        if let Some(d) = dte.take() {
                            result.push(d);
                        }
                        dte = None;
                        state = -1;
                    }
                    _ => {}
                }
            }
        }
        if save_element {
            for elem in &result {
                dt.table.add_element(elem.clone());
            }
        }
        if dt.level_description().is_empty() {
            let mut l: Vec<String> = Vec::new();
            for elem in &result {
                let level = elem.level.as_str().to_string();
                if !l.contains(&level) {
                    l.push(level);
                }
            }
            dt.set_level_description(&l);
        }
        Ok(())
    }
}

impl Default for DifficultyTableParser {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_course(grade_obj: &serde_json::Map<String, Value>) -> Result<Course> {
    let mut gr = Course::new();
    if let Some(name) = grade_obj.get("name").and_then(|v| v.as_str()) {
        gr.set_name(name);
    }
    let mut charts: Vec<BmsTableElement> = Vec::new();
    if let Some(charts_val) = grade_obj.get("charts") {
        if let Some(charts_arr) = charts_val.as_array() {
            for chart in charts_arr {
                if let Some(chart_obj) = chart.as_object() {
                    let mut dte = DifficultyTableElement::new();
                    let map: HashMap<String, Value> = chart_obj
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    dte.element.set_values(&map);
                    charts.push(dte.element);
                }
            }
        }
    } else if let Some(md5_val) = grade_obj.get("md5")
        && let Some(md5_arr) = md5_val.as_array()
    {
        for md5 in md5_arr {
            if let Some(md5_str) = md5.as_str() {
                let mut dte = DifficultyTableElement::new();
                dte.element.set_md5(md5_str);
                charts.push(dte.element);
            }
        }
    }
    gr.charts = charts;
    if let Some(style) = grade_obj.get("style").and_then(|v| v.as_str()) {
        gr.set_style(style);
    }
    if let Some(constraint_arr) = grade_obj.get("constraint").and_then(|v| v.as_array()) {
        let constraint: Vec<String> = constraint_arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        gr.constraint = constraint;
    }
    if let Some(trophy_val) = grade_obj.get("trophy")
        && let Some(trophy_arr) = trophy_val.as_array()
    {
        let mut trophy_list: Vec<Trophy> = Vec::new();
        for tr in trophy_arr {
            if let Some(tr_obj) = tr.as_object() {
                let mut t = Trophy::new();
                if let Some(name) = tr_obj.get("name").and_then(|v| v.as_str()) {
                    t.set_name(name);
                }
                if let Some(missrate) = tr_obj.get("missrate").and_then(|v| v.as_f64()) {
                    t.missrate = missrate;
                }
                if let Some(scorerate) = tr_obj.get("scorerate").and_then(|v| v.as_f64()) {
                    t.scorerate = scorerate;
                }
                if let Some(style) = tr_obj.get("style").and_then(|v| v.as_str()) {
                    t.set_style(style);
                }
                trophy_list.push(t);
            }
        }
        gr.trophy = trophy_list;
    }
    Ok(gr)
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_decode_header_from_file_basic() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{"name":"Test Table","symbol":"T","data_url":"data.json"}}"#
        )
        .unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        parser
            .decode_json_table_header_from_file(&mut dt, tmp.path())
            .unwrap();

        assert_eq!(dt.table.name().unwrap(), "Test Table");
        assert_eq!(dt.table.id().unwrap(), "T");
        assert_eq!(dt.table.data_url, &["data.json".to_string()]);
    }

    #[test]
    fn test_decode_header_data_url_as_array() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{"name":"Test","symbol":"T","data_url":["data1.json","data2.json"]}}"#
        )
        .unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        parser
            .decode_json_table_header_from_file(&mut dt, tmp.path())
            .unwrap();

        assert_eq!(
            dt.table.data_url,
            &["data1.json".to_string(), "data2.json".to_string()]
        );
    }

    #[test]
    fn test_decode_data_from_file_with_hashes() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"[
                {{"md5":"abc123","title":"Song 1","level":"5"}},
                {{"sha256":"def456","title":"Song 2","level":"10"}}
            ]"#
        )
        .unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        parser
            .decode_json_table_data_from_file(&mut dt, tmp.path())
            .unwrap();

        let elements = dt.elements();
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].element.title().unwrap(), "Song 1");
        assert_eq!(elements[0].element.md5().unwrap(), "abc123");
        assert_eq!(elements[0].level.as_str(), "5");
        assert_eq!(elements[1].element.title().unwrap(), "Song 2");
        assert_eq!(elements[1].element.sha256().unwrap(), "def456");
        assert_eq!(elements[1].level.as_str(), "10");
    }

    #[test]
    fn test_decode_data_from_file_filters_no_hash() {
        // decode_json_table_data_from_file calls internal with accept=true,
        // so all entries are accepted regardless of hash presence.
        // The URL-based path uses accept=false which filters entries.
        // To test filtering, call decode_json_table_data_internal directly.
        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();

        let long_hash = "a".repeat(25);
        let short_hash = "b".repeat(10);
        let data: Vec<HashMap<String, Value>> = vec![
            // valid: has level + long md5
            HashMap::from([
                ("level".to_string(), Value::String("1".to_string())),
                ("md5".to_string(), Value::String(long_hash.clone())),
                ("title".to_string(), Value::String("Good Song".to_string())),
            ]),
            // filtered: has level but md5 too short and no sha256
            HashMap::from([
                ("level".to_string(), Value::String("2".to_string())),
                ("md5".to_string(), Value::String(short_hash)),
                ("title".to_string(), Value::String("Short Hash".to_string())),
            ]),
            // filtered: no level
            HashMap::from([
                ("md5".to_string(), Value::String(long_hash.clone())),
                ("title".to_string(), Value::String("No Level".to_string())),
            ]),
            // filtered: no hash at all
            HashMap::from([
                ("level".to_string(), Value::String("3".to_string())),
                ("title".to_string(), Value::String("No Hash".to_string())),
            ]),
            // valid: has level + long sha256
            HashMap::from([
                ("level".to_string(), Value::String("4".to_string())),
                ("sha256".to_string(), Value::String(long_hash)),
                ("title".to_string(), Value::String("SHA Song".to_string())),
            ]),
        ];

        // accept=false triggers filtering by level + hash length > 24
        parser.decode_json_table_data_internal(&mut dt, &data, false);

        let elements = dt.elements();
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].element.title().unwrap(), "Good Song");
        assert_eq!(elements[1].element.title().unwrap(), "SHA Song");
    }

    #[test]
    fn test_encode_decode_header_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let header_path = dir.path().join("header.json");

        // Build a DifficultyTable with known data
        let mut dt = DifficultyTable::new();
        dt.table.set_name("Roundtrip Table");
        dt.table.set_id("RT");
        dt.table.set_tag("roundtrip-tag");
        dt.table.data_url = vec!["data.json".to_string()];
        dt.set_level_description(&["1".to_string(), "2".to_string(), "3".to_string()]);

        let parser = DifficultyTableParser::new();
        parser.encode_json_table_header(&dt, &header_path);

        // Decode the written file into a fresh DifficultyTable
        let mut dt2 = DifficultyTable::new();
        parser
            .decode_json_table_header_from_file(&mut dt2, &header_path)
            .unwrap();

        assert_eq!(dt2.table.name().unwrap(), "Roundtrip Table");
        assert_eq!(dt2.table.id().unwrap(), "RT");
        assert_eq!(dt2.table.data_url, &["data.json".to_string()]);
        assert_eq!(
            dt2.level_description(),
            vec!["1".to_string(), "2".to_string(), "3".to_string()]
        );
    }

    #[test]
    fn test_decode_header_missing_name() {
        let mut tmp = NamedTempFile::new().unwrap();
        // Has symbol but no name - should return error
        write!(tmp, r#"{{"symbol":"T","data_url":"data.json"}}"#).unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        let result = parser.decode_json_table_header_from_file(&mut dt, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_header_missing_symbol() {
        let mut tmp = NamedTempFile::new().unwrap();
        // Has name but no symbol - should return error
        write!(tmp, r#"{{"name":"Test","data_url":"data.json"}}"#).unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        let result = parser.decode_json_table_header_from_file(&mut dt, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_absolute_url_relative() {
        let parser = DifficultyTableParser::new();

        // Relative path gets prepended with the directory portion of source
        let result = parser.get_absolute_url("https://example.com/tables/index.html", "data.json");
        assert_eq!(result, "https://example.com/tables/data.json");

        // Path starting with "./" has the prefix stripped
        let result =
            parser.get_absolute_url("https://example.com/tables/index.html", "./data.json");
        assert_eq!(result, "https://example.com/tables/data.json");

        // Absolute HTTP URL is returned as-is
        let result = parser.get_absolute_url(
            "https://example.com/tables/index.html",
            "https://other.com/data.json",
        );
        assert_eq!(result, "https://other.com/data.json");

        // Path already starting with the source directory is returned as-is
        let result = parser.get_absolute_url(
            "https://example.com/tables/index.html",
            "https://example.com/tables/data.json",
        );
        assert_eq!(result, "https://example.com/tables/data.json");
    }

    #[test]
    fn test_get_absolute_url_no_slash_in_source() {
        let parser = DifficultyTableParser::new();

        // When source has no slash, the entire source is used as "directory"
        let result = parser.get_absolute_url("source", "data.json");
        assert_eq!(result, "sourcedata.json");
    }

    #[test]
    fn test_decode_header_with_course() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{
                "name": "Course Table",
                "symbol": "CT",
                "data_url": "data.json",
                "course": [
                    {{
                        "name": "Course A",
                        "charts": [
                            {{"md5": "hash1", "title": "Chart 1"}},
                            {{"md5": "hash2", "title": "Chart 2"}}
                        ],
                        "style": "7KEYS",
                        "constraint": ["grade_mirror", "gauge_lr2"],
                        "trophy": [
                            {{"name": "Gold", "missrate": 5.0, "scorerate": 90.0}}
                        ]
                    }},
                    {{
                        "name": "Course B",
                        "charts": [
                            {{"sha256": "hash3", "title": "Chart 3"}}
                        ],
                        "style": "14KEYS"
                    }}
                ]
            }}"#
        )
        .expect("unwrap");

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        parser
            .decode_json_table_header_from_file(&mut dt, tmp.path())
            .expect("JSON decode");

        let courses = dt.course();
        assert_eq!(courses.len(), 1); // single list wrapping both courses
        assert_eq!(courses[0].len(), 2);

        let course_a = &courses[0][0];
        assert_eq!(course_a.name(), "Course A");
        assert_eq!(course_a.style, "7KEYS");
        assert_eq!(course_a.charts().len(), 2);
        assert_eq!(course_a.charts()[0].md5().expect("md5"), "hash1");
        assert_eq!(course_a.charts()[1].md5().expect("md5"), "hash2");
        assert_eq!(course_a.constraint(), &["grade_mirror", "gauge_lr2"]);
        assert_eq!(course_a.trophy.len(), 1);
        assert_eq!(course_a.trophy[0].name(), "Gold");
        assert_eq!(course_a.trophy[0].missrate, 5.0);
        assert_eq!(course_a.trophy[0].scorerate, 90.0);

        let course_b = &courses[0][1];
        assert_eq!(course_b.name(), "Course B");
        assert_eq!(course_b.style, "14KEYS");
        assert_eq!(course_b.charts().len(), 1);
        assert_eq!(course_b.charts()[0].sha256().expect("sha256"), "hash3");
    }

    #[test]
    fn test_decode_header_with_nested_course() {
        // Test the List<List<Map>> course format (array of arrays)
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{
                "name": "Nested Course",
                "symbol": "NC",
                "data_url": "data.json",
                "course": [
                    [
                        {{"name": "Grade 1", "charts": [{{"md5": "h1"}}]}},
                        {{"name": "Grade 2", "charts": [{{"md5": "h2"}}]}}
                    ],
                    [
                        {{"name": "Grade 3", "charts": [{{"md5": "h3"}}]}}
                    ]
                ]
            }}"#
        )
        .unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        parser
            .decode_json_table_header_from_file(&mut dt, tmp.path())
            .unwrap();

        let courses = dt.course();
        assert_eq!(courses.len(), 2);
        assert_eq!(courses[0].len(), 2);
        assert_eq!(courses[0][0].name(), "Grade 1");
        assert_eq!(courses[0][1].name(), "Grade 2");
        assert_eq!(courses[1].len(), 1);
        assert_eq!(courses[1][0].name(), "Grade 3");
    }

    #[test]
    fn test_decode_header_with_grade_fallback() {
        // Test the "grade" field fallback (older format)
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{
                "name": "Grade Table",
                "symbol": "GT",
                "data_url": "data.json",
                "grade": [
                    {{
                        "name": "Dan 1",
                        "md5": ["md5_a", "md5_b"],
                        "style": "7KEYS"
                    }}
                ]
            }}"#
        )
        .expect("unwrap");

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        parser
            .decode_json_table_header_from_file(&mut dt, tmp.path())
            .expect("JSON decode");

        let courses = dt.course();
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].len(), 1);
        let dan = &courses[0][0];
        assert_eq!(dan.name(), "Dan 1");
        assert_eq!(dan.style, "7KEYS");
        assert_eq!(dan.charts().len(), 2);
        assert_eq!(dan.charts()[0].md5().expect("md5"), "md5_a");
        assert_eq!(dan.charts()[1].md5().expect("md5"), "md5_b");
        // grade format always adds these constraints
        assert_eq!(dan.constraint(), &["grade_mirror", "gauge_lr2"]);
    }

    #[test]
    fn test_encode_decode_data_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let header_path = dir.path().join("header.json");
        let data_path = dir.path().join("data.json");

        let mut dt = DifficultyTable::new();
        dt.table.set_name("Data Table");
        dt.table.set_id("DT");

        // Add elements
        let mut dte1 = DifficultyTableElement::new();
        dte1.element.set_md5("hash_aaa");
        dte1.element.set_title("Song A");
        dte1.set_level(Some("5"));
        dt.table.add_element(dte1);

        let mut dte2 = DifficultyTableElement::new();
        dte2.element.set_sha256("hash_bbb");
        dte2.element.set_title("Song B");
        dte2.set_level(Some("10"));
        dt.table.add_element(dte2);

        let parser = DifficultyTableParser::new();
        parser.encode_json_table_data(&mut dt, &header_path, &data_path);

        // Decode data back
        let mut dt2 = DifficultyTable::new();
        parser
            .decode_json_table_data_from_file(&mut dt2, &data_path)
            .unwrap();

        let elements = dt2.elements();
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].element.title().unwrap(), "Song A");
        assert_eq!(elements[0].element.md5().unwrap(), "hash_aaa");
        assert_eq!(elements[0].level.as_str(), "5");
        assert_eq!(elements[1].element.title().unwrap(), "Song B");
        assert_eq!(elements[1].element.sha256().unwrap(), "hash_bbb");
        assert_eq!(elements[1].level.as_str(), "10");
    }

    #[test]
    fn test_decode_data_from_file_sets_level_description() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"[
                {{"level":"3","md5":"abc","title":"A"}},
                {{"level":"5","md5":"def","title":"B"}},
                {{"level":"3","md5":"ghi","title":"C"}}
            ]"#
        )
        .unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        parser
            .decode_json_table_data_from_file(&mut dt, tmp.path())
            .unwrap();

        // Level descriptions are populated from unique levels in order of appearance
        let levels = dt.level_description();
        assert_eq!(levels, vec!["3".to_string(), "5".to_string()]);
    }

    #[test]
    fn test_decode_data_from_file_empty_array() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "[]").unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        parser
            .decode_json_table_data_from_file(&mut dt, tmp.path())
            .unwrap();

        assert!(dt.elements().is_empty());
        assert!(dt.level_description().is_empty());
    }

    #[test]
    fn test_encode_header_multiple_data_urls() {
        let dir = tempfile::tempdir().unwrap();
        let header_path = dir.path().join("header.json");

        let mut dt = DifficultyTable::new();
        dt.table.set_name("Multi URL");
        dt.table.set_id("MU");
        dt.table.data_url = vec!["a.json".to_string(), "b.json".to_string()];

        let parser = DifficultyTableParser::new();
        parser.encode_json_table_header(&dt, &header_path);

        // Decode and verify data_url is an array
        let mut dt2 = DifficultyTable::new();
        parser
            .decode_json_table_header_from_file(&mut dt2, &header_path)
            .unwrap();
        assert_eq!(
            dt2.table.data_url,
            &["a.json".to_string(), "b.json".to_string()]
        );
    }

    #[test]
    fn test_decode_header_with_data_rule() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{
                "name": "Rule Table",
                "symbol": "RT",
                "data_url": ["main.json", "extra.json"],
                "data_rule": [
                    {{"1": "A", "2": "B"}},
                    {{"3": "C"}}
                ]
            }}"#
        )
        .unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        parser
            .decode_json_table_header_from_file(&mut dt, tmp.path())
            .unwrap();

        let configs = &dt.table.merge_configurations;
        assert_eq!(configs.len(), 2);
        assert_eq!(configs["main.json"]["1"], "A");
        assert_eq!(configs["main.json"]["2"], "B");
        assert_eq!(configs["extra.json"]["3"], "C");
    }

    #[test]
    fn test_decode_header_invalid_json() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "not valid json").unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        let result = parser.decode_json_table_header_from_file(&mut dt, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_data_invalid_json() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "not valid json").unwrap();

        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        let result = parser.decode_json_table_data_from_file(&mut dt, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_header_nonexistent_file() {
        let parser = DifficultyTableParser::new();
        let mut dt = DifficultyTable::new();
        let result =
            parser.decode_json_table_header_from_file(&mut dt, Path::new("/nonexistent/file.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_absolute_url_already_absolute() {
        let parser = DifficultyTableParser::new();
        let result =
            parser.get_absolute_url("http://example.com/table/", "https://other.com/data.json");
        assert_eq!(result, "https://other.com/data.json");
    }

    #[test]
    fn test_get_absolute_url_relative_path() {
        let parser = DifficultyTableParser::new();
        let result = parser.get_absolute_url("http://example.com/table/index.html", "data.json");
        assert_eq!(result, "http://example.com/table/data.json");
    }

    #[test]
    fn test_get_absolute_url_dot_relative() {
        let parser = DifficultyTableParser::new();
        let result = parser.get_absolute_url("http://example.com/table/index.html", "./data.json");
        assert_eq!(result, "http://example.com/table/data.json");
    }

    #[test]
    fn test_get_absolute_url_root_relative() {
        let parser = DifficultyTableParser::new();
        let result =
            parser.get_absolute_url("http://example.com/table/index.html", "/other/data.json");
        assert_eq!(result, "http://example.com/other/data.json");
    }

    #[test]
    fn test_get_absolute_url_parent_relative() {
        let parser = DifficultyTableParser::new();
        let result =
            parser.get_absolute_url("http://example.com/table/sub/index.html", "../data.json");
        assert_eq!(result, "http://example.com/table/data.json");
    }

    #[test]
    fn test_decode_bytes_with_charset_utf8() {
        let input = b"<html><head><meta charset=\"utf-8\"></head><body>Hello</body></html>";
        let result = DifficultyTableParser::decode_bytes_with_charset(input);
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_decode_bytes_with_charset_shift_jis() {
        // Create Shift_JIS encoded content with charset declaration
        let html =
            "<html><head><meta charset=\"Shift_JIS\"></head><body>\u{97f3}\u{697d}</body></html>";
        let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(html);
        let result = DifficultyTableParser::decode_bytes_with_charset(&encoded);
        assert!(
            result.contains("\u{97f3}\u{697d}"),
            "Should decode Shift_JIS content correctly: {}",
            result
        );
    }

    #[test]
    fn test_decode_bytes_with_charset_content_type_meta() {
        let html = "<html><head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=Shift_JIS\"></head></html>";
        let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(html);
        let result = DifficultyTableParser::decode_bytes_with_charset(&encoded);
        assert!(result.contains("Content-Type"));
    }

    #[test]
    fn test_decode_bytes_with_charset_unicode_before_charset_tag() {
        // Regression: to_lowercase() on certain Unicode characters (e.g. uppercase Eszett
        // U+1E9E -> "ss") changes byte length, making the index from the lowered string
        // invalid for the original. to_ascii_lowercase() preserves byte positions for
        // non-ASCII characters.
        //
        // U+1E9E (uppercase Eszett) is 3 bytes in UTF-8: [0xE1, 0xBA, 0x9E]
        // to_lowercase() maps it to "ss" (2 bytes), shrinking the string by 1 byte.
        // If we used that shrunken index on the original string, slicing would be off by 1,
        // causing the parsed charset to be "=" instead of "shift_jis".
        //
        // Build a Shift_JIS page where the UTF-8 lossy decode puts U+1E9E before charset=.
        // The raw bytes use 0xE1 0xBA 0x9E (UTF-8 for U+1E9E) which Shift_JIS interprets
        // differently, but from_utf8_lossy on the mixed input still surfaces U+1E9E in the
        // probe string when the non-Shift_JIS preamble is valid UTF-8.
        let preamble = b"<html><head><title>";
        let eszett_utf8: &[u8] = "\u{1E9E}".as_bytes(); // 3 bytes: [0xE1, 0xBA, 0x9E]
        let mid = b"test</title><meta charset=\"Shift_JIS\"></head><body>";
        // Shift_JIS bytes for Japanese text (U+97F3 U+697D = "music")
        let body_sjis: &[u8] = &[0x89, 0xB9, 0x8A, 0x79]; // "ongaku" in Shift_JIS
        let tail = b"</body></html>";

        let mut bytes = Vec::new();
        bytes.extend_from_slice(preamble);
        bytes.extend_from_slice(eszett_utf8);
        bytes.extend_from_slice(mid);
        bytes.extend_from_slice(body_sjis);
        bytes.extend_from_slice(tail);

        let result = DifficultyTableParser::decode_bytes_with_charset(&bytes);
        // Correct behavior: detect charset=Shift_JIS and decode body as Japanese text.
        // With the bug, charset is misparsed as "=" which falls through to the
        // encoding_rs::Encoding::for_label fallback (returns None), then falls back to
        // UTF-8 lossy, corrupting the Shift_JIS body bytes.
        assert!(
            result.contains("\u{97F3}") || result.contains("\u{697D}"),
            "Should decode Shift_JIS body correctly when charset is properly parsed, got: {}",
            result
        );
    }
}

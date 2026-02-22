use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::Result;
use regex::Regex;
use serde_json::Value;

use crate::bms_table_element::BmsTableElement;
use crate::course::{Course, Trophy};
use crate::difficulty_table::DifficultyTable;
use crate::difficulty_table_element::DifficultyTableElement;

pub struct DifficultyTableParser {
    data: HashMap<String, Vec<String>>,
}

impl DifficultyTableParser {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn contains_header(&mut self, urlname: &str) -> bool {
        self.get_meta_tag(urlname, "bmstable").is_some()
    }

    pub fn get_alternate_bms_table_url(&mut self, urlname: &str) -> Option<String> {
        self.get_meta_tag(urlname, "bmstable-alt")
    }

    fn read_all_lines(&self, urlname: &str) -> Option<Vec<String>> {
        match reqwest::blocking::get(urlname) {
            Ok(response) => match response.text() {
                Ok(text) => {
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
            },
            Err(e) => {
                log::error!(
                    "\u{96e3}\u{6613}\u{5ea6}\u{8868}\u{30b5}\u{30a4}\u{30c8}\u{89e3}\u{6790}\u{4e2d}\u{306e}\u{4f8b}\u{5916}:{}",
                    e
                );
                None
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

    #[allow(clippy::unnecessary_get_then_check)]
    pub fn decode(&mut self, b: bool, diff: &mut DifficultyTable) -> Result<()> {
        let urlname = diff.table.get_source_url().to_string();
        let mut tableurl: Option<String> = None;
        let mut _enc: Option<String> = None;
        if urlname.is_empty() {
            tableurl = Some(diff.table.get_head_url().to_string());
        } else {
            if !self.data.contains_key(&urlname)
                && let Some(lines) = self.read_all_lines(&urlname)
            {
                self.data.insert(urlname.clone(), lines);
            }
            if self.data.get(&urlname).is_none() {
                return Err(anyhow::anyhow!("Failed to read URL"));
            }
            let lines = self.data.get(&urlname).unwrap().clone();
            for line in &lines {
                if line
                    .to_lowercase()
                    .contains("<meta http-equiv=\"content-type\"")
                {
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
            diff.table.set_source_url(&urlname);
        } else if let Some(ref enc) = _enc {
            let _enc_upper = enc.to_uppercase();
            // encoding normalization (unused in current code path)
        }
        Ok(())
    }

    #[allow(clippy::manual_strip)]
    fn get_absolute_url(&self, source: &str, path: &str) -> String {
        let urldir = if let Some(idx) = source.rfind('/') {
            &source[..idx + 1]
        } else {
            source
        };
        if !path.starts_with("http") && !path.starts_with(urldir) {
            let p = if path.starts_with("./") {
                &path[2..]
            } else {
                path
            };
            return format!("{}{}", urldir, p);
        }
        path.to_string()
    }

    pub fn decode_json_table(
        &self,
        dt: &mut DifficultyTable,
        jsonheader_url: &str,
        save_elements: bool,
    ) -> Result<()> {
        self.decode_json_table_header_from_url(dt, jsonheader_url)?;
        let urls = dt.table.get_data_url().to_vec();
        if save_elements {
            dt.table.remove_all_elements();
            let mut elements: Vec<DifficultyTableElement> = Vec::new();
            let mut levels: Vec<String> = Vec::new();
            for url in &urls {
                let conf = dt
                    .table
                    .get_merge_configurations()
                    .get(url)
                    .cloned()
                    .unwrap_or_default();
                let mut table = DifficultyTable::new();

                let source_url = dt.table.get_source_url().to_string();
                let head_url = dt.table.get_head_url().to_string();
                let base_url = if source_url.is_empty() {
                    head_url.clone()
                } else {
                    self.get_absolute_url(&source_url, &head_url)
                };
                let data_url = self.get_absolute_url(&base_url, url);

                self.decode_json_table_data_from_url(&mut table, &data_url)?;
                for l in &table.get_level_description() {
                    levels.push(l.clone());
                }
                for dte in table.get_elements() {
                    let level_conf = conf.get(dte.get_level());
                    if level_conf.is_none() || !level_conf.unwrap().is_empty() {
                        let contains = false;
                        if !contains {
                            let mut dte = dte.clone();
                            if let Some(new_level) = conf.get(dte.get_level()) {
                                dte.set_level(Some(new_level));
                            }
                            elements.push(dte);
                        }
                    }
                }
            }
            if dt.get_level_description().is_empty() {
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
        let response = reqwest::blocking::get(jsonheader_url)?;
        let text = response.text()?;
        let result: HashMap<String, Value> = serde_json::from_str(&text)?;
        self.decode_json_table_header_internal(dt, &result)?;
        dt.table.set_head_url(jsonheader_url);
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
                dt.table.set_data_url(vec![s.to_string()]);
            }
            if let Some(arr) = du.as_array() {
                let urls: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                dt.table.set_data_url(urls);
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
        let data_urls = dt.table.get_data_url().to_vec();
        for i in 0..data_urls.len() {
            if i == merge.len() {
                break;
            }
            mergerule.insert(data_urls[i].clone(), merge[i].clone());
        }
        dt.table.set_merge_configurations(mergerule);
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
                    gr.set_charts(charts);
                    if let Some(style) = grade_obj.get("style").and_then(|v| v.as_str()) {
                        gr.set_style(style);
                    }
                    gr.set_constraint(vec!["grade_mirror".to_string(), "gauge_lr2".to_string()]);
                    l.push(gr);
                }
            }
            courses.push(l);
        }
        dt.set_course(courses);
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
        let response = reqwest::blocking::get(jsondata_url)?;
        let text = response.text()?;
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
                if dte.element.get_mode().is_none()
                    && let Some(mode) = dt.table.get_mode()
                {
                    dte.element.set_mode(mode);
                }

                let level = m.get("level").map(value_to_string).unwrap_or_default();
                let mut b = true;
                for j in 0..levelorder.len() {
                    if levelorder[j] == level {
                        b = false;
                    }
                }
                if b {
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

        if dt.get_level_description().is_empty() {
            dt.set_level_description(&levelorder);
        }
    }

    pub fn encode_json_table_header(&self, dt: &DifficultyTable, jsonheader: &Path) {
        let result = (|| -> Result<()> {
            let mut header: HashMap<String, Value> = HashMap::new();
            header.insert(
                "name".to_string(),
                Value::String(dt.table.get_name().unwrap_or_default().to_string()),
            );
            header.insert(
                "symbol".to_string(),
                Value::String(dt.table.get_id().unwrap_or_default().to_string()),
            );
            header.insert(
                "tag".to_string(),
                Value::String(dt.table.get_tag().unwrap_or_default()),
            );
            let levels: Vec<Value> = dt
                .get_level_description()
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect();
            header.insert("level_order".to_string(), Value::Array(levels));
            let data_urls = dt.table.get_data_url();
            if data_urls.len() > 1 {
                let arr: Vec<Value> = data_urls.iter().map(|s| Value::String(s.clone())).collect();
                header.insert("data_url".to_string(), Value::Array(arr));
            } else if data_urls.len() == 1 {
                header.insert("data_url".to_string(), Value::String(data_urls[0].clone()));
            }
            let attrmap = dt.table.get_attrmap();
            if !attrmap.is_empty() {
                let obj: serde_json::Map<String, Value> = attrmap
                    .into_iter()
                    .map(|(k, v)| (k, Value::String(v)))
                    .collect();
                header.insert("attr".to_string(), Value::Object(obj));
            }

            // Java: "TODO 後でcourseの仕様に合わせる" — incomplete in Java too (only name/style)
            let course = dt.get_course();
            if !course.is_empty() {
                let mut grade: Vec<Value> = Vec::new();
                for g in &course[0] {
                    let mut m: serde_json::Map<String, Value> = serde_json::Map::new();
                    m.insert("name".to_string(), Value::String(g.get_name().to_string()));
                    m.insert(
                        "style".to_string(),
                        Value::String(g.get_style().to_string()),
                    );
                    grade.push(Value::Object(m));
                }
                header.insert("course".to_string(), Value::Array(grade));
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
            dt.table.set_data_url(vec![data_filename]);
            self.encode_json_table_header(dt, jsonheader);
            let mut datas: Vec<HashMap<String, Value>> = Vec::new();
            for te in &dt.get_elements() {
                datas.push(te.get_values());
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

    #[allow(
        dead_code,
        clippy::needless_range_loop,
        clippy::regex_creation_in_loops
    )]
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
                            let br_re = Regex::new(r"(?i)<br\s*/?>").unwrap();
                            let split_br: Vec<&str> = br_re.split(parts[1]).collect();
                            let a_re = Regex::new(r"(?i)<a\s+href=.+'>|</a>").unwrap();
                            let artist = a_re.replace_all(split_br[0], "");
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
                                let pkg_name = a_re.replace_all(split_br[1], "");
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
                            let a_re = Regex::new(r"(?i)<a\s+href=.+'>|</a>").unwrap();
                            let append_artist = a_re.replace_all(parts[1], "");
                            if let Some(ref mut d) = dte {
                                d.set_append_artist(&append_artist);
                            }
                        }
                        state += 1;
                    }
                    5 => {
                        let parts: Vec<&str> = line.split('"').collect();
                        if parts.len() > 1 {
                            let avg_re = Regex::new(r"Avg:.*JUDGE:[A-Z]+\s*").unwrap();
                            let comment = avg_re.replace_all(parts[1], "");
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
            for i in 0..result.len() {
                dt.table.add_element(result[i].clone());
            }
        }
        if dt.get_level_description().is_empty() {
            let mut l: Vec<String> = Vec::new();
            for i in 0..result.len() {
                let mut b = true;
                for j in 0..l.len() {
                    if l[j] == result[i].get_level() {
                        b = false;
                    }
                }
                if b {
                    l.push(result[i].get_level().to_string());
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
    gr.set_charts(charts);
    if let Some(style) = grade_obj.get("style").and_then(|v| v.as_str()) {
        gr.set_style(style);
    }
    if let Some(constraint_arr) = grade_obj.get("constraint").and_then(|v| v.as_array()) {
        let constraint: Vec<String> = constraint_arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        gr.set_constraint(constraint);
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
                    t.set_missrate(missrate);
                }
                if let Some(scorerate) = tr_obj.get("scorerate").and_then(|v| v.as_f64()) {
                    t.set_scorerate(scorerate);
                }
                if let Some(style) = tr_obj.get("style").and_then(|v| v.as_str()) {
                    t.set_style(style);
                }
                trophy_list.push(t);
            }
        }
        gr.set_trophy(trophy_list);
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

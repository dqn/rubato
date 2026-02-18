//! Difficulty table JSON parser.
//!
//! Port of Java `DifficultyTableParser.java` from jbmstable-parser.
//! Parses JSON-based BMS difficulty table headers and chart data.
//! HTTP fetching is handled separately in the `brs` crate.

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use serde_json::Value;
use url::Url;

use crate::CourseData;
use crate::course_data::{CourseDataConstraint, CourseSongData, TrophyData};
use crate::table_data::{TableData, TableFolder};

/// Parsed header from a difficulty table JSON header file.
#[derive(Debug, Clone)]
pub struct DifficultyTableHeader {
    pub name: String,
    pub symbol: String,
    pub tag: String,
    pub level_order: Vec<String>,
    pub data_url: Vec<String>,
    pub data_rule: Vec<HashMap<String, String>>,
    pub courses: Vec<Vec<ParsedCourse>>,
    pub trophies: Vec<TrophyData>,
}

/// A parsed course definition from the header.
#[derive(Debug, Clone)]
pub struct ParsedCourse {
    pub name: String,
    pub songs: Vec<CourseSongData>,
    pub constraint: Vec<CourseDataConstraint>,
    pub trophy: Vec<TrophyData>,
}

/// A parsed chart entry from the data JSON.
#[derive(Debug, Clone)]
pub struct ParsedChart {
    pub level: String,
    pub md5: String,
    pub sha256: String,
    pub title: String,
    pub artist: String,
    pub url: String,
}

/// Parse a JSON header string into a `DifficultyTableHeader`.
///
/// Handles:
/// - `data_url` as both a single string and an array of strings
/// - `course` as `Vec<Vec<CourseObj>>` or `Vec<CourseObj>` (wrapped in a single vec)
/// - `grade` as legacy format (adds `GradeMirror` + `GaugeLr2` constraints automatically)
/// - `tag` defaults to `symbol` if absent
pub fn parse_json_header(json_str: &str) -> Result<DifficultyTableHeader> {
    let root: Value = serde_json::from_str(json_str)?;
    let obj = root
        .as_object()
        .ok_or_else(|| anyhow!("header JSON must be an object"))?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing required field: name"))?
        .to_string();

    let symbol = obj
        .get("symbol")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing required field: symbol"))?
        .to_string();

    let tag = obj
        .get("tag")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| symbol.clone());

    // level_order: array of strings (convert non-strings via to_string)
    let level_order = obj
        .get("level_order")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|v| match v.as_str() {
                    Some(s) => s.to_string(),
                    None => v.to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    // data_url: String or Vec<String>
    let data_url = match obj.get("data_url") {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    };

    // data_rule: Vec<Map<String, String>>, one per data_url
    let data_rule = match obj.get("data_rule") {
        Some(Value::Array(arr)) => arr
            .iter()
            .take(data_url.len())
            .filter_map(|v| {
                v.as_object().map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect::<HashMap<String, String>>()
                })
            })
            .collect(),
        _ => Vec::new(),
    };

    // Parse courses from "course" or legacy "grade"
    let courses = if let Some(course_val) = obj.get("course") {
        parse_courses(course_val)?
    } else if let Some(grade_val) = obj.get("grade") {
        parse_grade_legacy(grade_val)?
    } else {
        Vec::new()
    };

    // Parse top-level trophies
    let trophies = obj
        .get("trophy")
        .and_then(|v| v.as_array())
        .map(|arr| parse_trophy_array(arr))
        .unwrap_or_default();

    Ok(DifficultyTableHeader {
        name,
        symbol,
        tag,
        level_order,
        data_url,
        data_rule,
        courses,
        trophies,
    })
}

/// Parse a JSON data string (chart list) into a vec of `ParsedChart`.
///
/// Only includes entries where:
/// - `level` is present and non-empty
/// - At least one of `md5` or `sha256` has length > 24
pub fn parse_json_data(json_str: &str) -> Result<Vec<ParsedChart>> {
    let root: Value = serde_json::from_str(json_str)?;
    let arr = root
        .as_array()
        .ok_or_else(|| anyhow!("data JSON must be an array"))?;

    let mut charts = Vec::new();
    for item in arr {
        let level = item
            .get("level")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .unwrap_or_default();

        if level.is_empty() {
            continue;
        }

        let md5 = item
            .get("md5")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let sha256 = item
            .get("sha256")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if md5.len() <= 24 && sha256.len() <= 24 {
            continue;
        }

        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let artist = item
            .get("artist")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let url = item
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        charts.push(ParsedChart {
            level,
            md5,
            sha256,
            title,
            artist,
            url,
        });
    }

    Ok(charts)
}

/// Extract the bmstable (or bmstable-alt) URL from HTML content.
///
/// Scans line-by-line for `<meta name="bmstable" content="...">`.
/// Falls back to `bmstable-alt` if `bmstable` is not found.
pub fn extract_bmstable_url(html: &str) -> Option<String> {
    let result = extract_meta_content(html, "bmstable");
    if result.is_some() {
        return result;
    }
    extract_meta_content(html, "bmstable-alt")
}

/// Resolve a potentially relative URL against a source URL.
///
/// - If `relative` starts with "http" -> return as-is.
/// - If `relative` starts with "./" -> strip "./" prefix.
/// - Prepend the directory portion of `source_url` (everything before last '/').
pub fn resolve_url(source_url: &str, relative: &str) -> String {
    if let Ok(base) = Url::parse(source_url)
        && let Ok(joined) = base.join(relative)
    {
        return joined.to_string();
    }

    let path = if let Some(stripped) = relative.strip_prefix("./") {
        stripped
    } else {
        relative
    };

    let dir = match source_url.rfind('/') {
        Some(idx) => &source_url[..=idx],
        None => source_url,
    };

    format!("{dir}{path}")
}

/// Convert parsed header and chart data into a `TableData`.
///
/// - Creates one `TableFolder` per level in `level_order`.
/// - Folder name = `"{tag}{level}"`.
/// - Groups charts by level into their respective folders.
/// - Converts courses from `ParsedCourse` -> `CourseData`.
pub fn to_table_data(
    header: &DifficultyTableHeader,
    charts: &[ParsedChart],
    source_url: &str,
) -> TableData {
    let mut folders: Vec<TableFolder> = header
        .level_order
        .iter()
        .map(|level| TableFolder {
            name: format!("{}{}", header.tag, level),
            songs: Vec::new(),
        })
        .collect();

    // Group charts by level
    for chart in charts {
        if let Some(folder) = folders
            .iter_mut()
            .enumerate()
            .find(|(i, _)| header.level_order.get(*i) == Some(&chart.level))
            .map(|(_, f)| f)
        {
            folder.songs.push(CourseSongData {
                sha256: chart.sha256.clone(),
                md5: chart.md5.clone(),
                title: chart.title.clone(),
            });
        }
    }

    // Convert courses
    let mut all_courses = Vec::new();
    for course_group in &header.courses {
        for parsed in course_group {
            all_courses.push(CourseData {
                name: parsed.name.clone(),
                hash: parsed.songs.clone(),
                constraint: parsed.constraint.clone(),
                trophy: parsed.trophy.clone(),
                release: true,
            });
        }
    }

    TableData {
        url: source_url.to_string(),
        name: header.name.clone(),
        tag: header.tag.clone(),
        folder: folders,
        course: all_courses,
    }
}

/// Apply a data_rule mapping to a list of charts.
///
/// For each chart:
/// - If the chart's level is not in the rule -> keep as-is
/// - If the rule maps to a non-empty string -> remap the level
/// - If the rule maps to an empty string -> exclude the chart
pub fn apply_data_rule(charts: &[ParsedChart], rule: &HashMap<String, String>) -> Vec<ParsedChart> {
    charts
        .iter()
        .filter_map(|chart| match rule.get(&chart.level) {
            Some(mapped) if mapped.is_empty() => None,
            Some(mapped) => Some(ParsedChart {
                level: mapped.clone(),
                ..chart.clone()
            }),
            None => Some(chart.clone()),
        })
        .collect()
}

// --- Internal helpers ---

/// Extract content attribute value from a meta tag with the given name.
fn extract_meta_content(html: &str, meta_name: &str) -> Option<String> {
    let name_lower = meta_name.to_lowercase();

    for line in html.lines() {
        let lower = line.to_lowercase();
        if !lower.contains("<meta") {
            continue;
        }

        // Check for name="bmstable" or name='bmstable'
        let has_name = lower.contains(&format!("name=\"{name_lower}\""))
            || lower.contains(&format!("name='{name_lower}'"));
        if !has_name {
            continue;
        }

        // Extract content value using the original line (preserving case)
        if let Some(content) = extract_content_value(line) {
            return Some(content);
        }
    }
    None
}

/// Extract the value of a `content="..."` or `content='...'` attribute from a tag string.
fn extract_content_value(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let content_prefix = "content=";
    let idx = lower.find(content_prefix)?;
    let after = &line[idx + content_prefix.len()..];

    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let inner = &after[1..];
    let end = inner.find(quote)?;
    Some(inner[..end].to_string())
}

/// Parse `course` field: either `Vec<Vec<CourseObj>>` or `Vec<CourseObj>`.
fn parse_courses(val: &Value) -> Result<Vec<Vec<ParsedCourse>>> {
    let arr = val
        .as_array()
        .ok_or_else(|| anyhow!("course must be an array"))?;

    if arr.is_empty() {
        return Ok(Vec::new());
    }

    // Detect nesting: if first element is an array, it's Vec<Vec<CourseObj>>
    if arr[0].is_array() {
        let mut result = Vec::new();
        for group in arr {
            let group_arr = group
                .as_array()
                .ok_or_else(|| anyhow!("course group must be an array"))?;
            let mut courses = Vec::new();
            for course_obj in group_arr {
                courses.push(parse_single_course(course_obj)?);
            }
            result.push(courses);
        }
        Ok(result)
    } else {
        // Vec<CourseObj> — wrap in single vec
        let mut courses = Vec::new();
        for course_obj in arr {
            courses.push(parse_single_course(course_obj)?);
        }
        Ok(vec![courses])
    }
}

/// Parse `grade` legacy field: adds `GradeMirror` + `GaugeLr2` constraints automatically.
fn parse_grade_legacy(val: &Value) -> Result<Vec<Vec<ParsedCourse>>> {
    let arr = val
        .as_array()
        .ok_or_else(|| anyhow!("grade must be an array"))?;

    let mut courses = Vec::new();
    for grade_obj in arr {
        let obj = grade_obj
            .as_object()
            .ok_or_else(|| anyhow!("grade entry must be an object"))?;

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        // Grade uses "md5" array only
        let songs = parse_md5_array(obj.get("md5"));

        courses.push(ParsedCourse {
            name,
            songs,
            constraint: vec![
                CourseDataConstraint::GradeMirror,
                CourseDataConstraint::GaugeLr2,
            ],
            trophy: Vec::new(),
        });
    }

    Ok(vec![courses])
}

/// Parse a single course object from JSON.
fn parse_single_course(val: &Value) -> Result<ParsedCourse> {
    let obj = val
        .as_object()
        .ok_or_else(|| anyhow!("course entry must be an object"))?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    // Charts: either "charts" (array of objects with md5/sha256) or "md5" (array of hash strings)
    let songs = if let Some(charts_val) = obj.get("charts") {
        parse_charts_array(charts_val)?
    } else {
        parse_md5_array(obj.get("md5"))
    };

    // Constraints: array of strings
    let constraint = obj
        .get("constraint")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(parse_constraint)
                .collect()
        })
        .unwrap_or_default();

    // Trophy
    let trophy = obj
        .get("trophy")
        .and_then(|v| v.as_array())
        .map(|arr| parse_trophy_array(arr))
        .unwrap_or_default();

    Ok(ParsedCourse {
        name,
        songs,
        constraint,
        trophy,
    })
}

/// Parse an array of chart objects into `CourseSongData`.
fn parse_charts_array(val: &Value) -> Result<Vec<CourseSongData>> {
    let arr = val
        .as_array()
        .ok_or_else(|| anyhow!("charts must be an array"))?;

    let mut songs = Vec::new();
    for chart in arr {
        let md5 = chart
            .get("md5")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let sha256 = chart
            .get("sha256")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let title = chart
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        songs.push(CourseSongData { sha256, md5, title });
    }
    Ok(songs)
}

/// Parse an md5 array (list of hash strings) into `CourseSongData`.
fn parse_md5_array(val: Option<&Value>) -> Vec<CourseSongData> {
    val.and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|md5| CourseSongData {
                    sha256: String::new(),
                    md5: md5.to_string(),
                    title: String::new(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse a constraint string to a `CourseDataConstraint`.
fn parse_constraint(s: &str) -> Option<CourseDataConstraint> {
    // Use serde deserialization to match the renamed variants
    let json_str = format!("\"{s}\"");
    serde_json::from_str::<CourseDataConstraint>(&json_str).ok()
}

/// Parse a trophy array from JSON values.
fn parse_trophy_array(arr: &[Value]) -> Vec<TrophyData> {
    arr.iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            let name = obj.get("name")?.as_str()?.to_string();
            let missrate = obj.get("missrate").and_then(|v| v.as_f64())? as f32;
            let scorerate = obj.get("scorerate").and_then(|v| v.as_f64())? as f32;
            Some(TrophyData {
                name,
                missrate,
                scorerate,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_json_header tests ----

    #[test]
    fn header_basic_name_symbol_tag() {
        let json = r#"{
            "name": "Insane Table",
            "symbol": "★",
            "tag": "insane",
            "level_order": ["1", "2", "3"]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.name, "Insane Table");
        assert_eq!(header.symbol, "★");
        assert_eq!(header.tag, "insane");
        assert_eq!(header.level_order, vec!["1", "2", "3"]);
    }

    #[test]
    fn header_tag_defaults_to_symbol() {
        let json = r#"{
            "name": "Table",
            "symbol": "△"
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.tag, "△");
    }

    #[test]
    fn header_data_url_as_string() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "data_url": "data.json"
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.data_url, vec!["data.json"]);
    }

    #[test]
    fn header_data_url_as_array() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "data_url": ["data1.json", "data2.json"]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.data_url, vec!["data1.json", "data2.json"]);
    }

    #[test]
    fn header_missing_name_returns_error() {
        let json = r#"{"symbol": "S"}"#;
        assert!(parse_json_header(json).is_err());
    }

    #[test]
    fn header_missing_symbol_returns_error() {
        let json = r#"{"name": "N"}"#;
        assert!(parse_json_header(json).is_err());
    }

    #[test]
    fn header_course_as_nested_array() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "course": [[
                {
                    "name": "Dan 1",
                    "md5": ["aabbccddaabbccddaabbccddaabbccdd"],
                    "constraint": ["grade_mirror", "gauge_lr2"]
                }
            ]]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.courses.len(), 1);
        assert_eq!(header.courses[0].len(), 1);
        assert_eq!(header.courses[0][0].name, "Dan 1");
        assert_eq!(header.courses[0][0].songs.len(), 1);
        assert_eq!(
            header.courses[0][0].songs[0].md5,
            "aabbccddaabbccddaabbccddaabbccdd"
        );
        assert_eq!(header.courses[0][0].constraint.len(), 2);
        assert_eq!(
            header.courses[0][0].constraint[0],
            CourseDataConstraint::GradeMirror
        );
        assert_eq!(
            header.courses[0][0].constraint[1],
            CourseDataConstraint::GaugeLr2
        );
    }

    #[test]
    fn header_course_as_flat_array() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "course": [
                {
                    "name": "Course A",
                    "md5": ["aaaa"],
                    "constraint": ["no_speed"]
                }
            ]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.courses.len(), 1);
        assert_eq!(header.courses[0].len(), 1);
        assert_eq!(header.courses[0][0].name, "Course A");
    }

    #[test]
    fn header_course_with_charts_objects() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "course": [
                {
                    "name": "Course B",
                    "charts": [
                        {"md5": "hash1", "sha256": "sha1", "title": "Song 1"},
                        {"md5": "hash2", "title": "Song 2"}
                    ],
                    "constraint": ["ln"]
                }
            ]
        }"#;
        let header = parse_json_header(json).unwrap();
        let course = &header.courses[0][0];
        assert_eq!(course.songs.len(), 2);
        assert_eq!(course.songs[0].md5, "hash1");
        assert_eq!(course.songs[0].sha256, "sha1");
        assert_eq!(course.songs[1].md5, "hash2");
        assert_eq!(course.songs[1].sha256, "");
    }

    #[test]
    fn header_grade_legacy_format() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "grade": [
                {
                    "name": "7th Dan",
                    "md5": ["md5hash1", "md5hash2"]
                }
            ]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.courses.len(), 1);
        assert_eq!(header.courses[0].len(), 1);
        let grade = &header.courses[0][0];
        assert_eq!(grade.name, "7th Dan");
        assert_eq!(grade.songs.len(), 2);
        // Legacy grade auto-adds GradeMirror + GaugeLr2
        assert_eq!(grade.constraint.len(), 2);
        assert_eq!(grade.constraint[0], CourseDataConstraint::GradeMirror);
        assert_eq!(grade.constraint[1], CourseDataConstraint::GaugeLr2);
    }

    #[test]
    fn header_course_with_trophy() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "course": [
                {
                    "name": "C",
                    "md5": ["h1"],
                    "constraint": [],
                    "trophy": [
                        {"name": "Gold", "missrate": 5.0, "scorerate": 90.0}
                    ]
                }
            ]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.courses[0][0].trophy.len(), 1);
        assert_eq!(header.courses[0][0].trophy[0].name, "Gold");
    }

    #[test]
    fn header_top_level_trophies() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "trophy": [
                {"name": "Silver", "missrate": 10.0, "scorerate": 80.0}
            ]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.trophies.len(), 1);
        assert_eq!(header.trophies[0].name, "Silver");
        assert!((header.trophies[0].missrate - 10.0).abs() < f32::EPSILON);
        assert!((header.trophies[0].scorerate - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn header_level_order_numeric_values() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "level_order": [1, 2, 3]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.level_order, vec!["1", "2", "3"]);
    }

    // ---- parse_json_data tests ----

    #[test]
    fn data_normal_parsing() {
        let json = r#"[
            {
                "level": "10",
                "md5": "0123456789abcdef0123456789abcdef",
                "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                "title": "Test Song",
                "artist": "Test Artist",
                "url": "http://example.com/dl"
            }
        ]"#;
        let charts = parse_json_data(json).unwrap();
        assert_eq!(charts.len(), 1);
        assert_eq!(charts[0].level, "10");
        assert_eq!(charts[0].md5, "0123456789abcdef0123456789abcdef");
        assert_eq!(charts[0].title, "Test Song");
        assert_eq!(charts[0].artist, "Test Artist");
        assert_eq!(charts[0].url, "http://example.com/dl");
    }

    #[test]
    fn data_filters_short_hashes() {
        let json = r#"[
            {
                "level": "1",
                "md5": "short",
                "sha256": "alsoshort",
                "title": "Should be filtered"
            }
        ]"#;
        let charts = parse_json_data(json).unwrap();
        assert_eq!(charts.len(), 0);
    }

    #[test]
    fn data_accepts_if_md5_long_enough() {
        let json = r#"[
            {
                "level": "1",
                "md5": "0123456789abcdef0123456789a",
                "title": "Has long md5"
            }
        ]"#;
        let charts = parse_json_data(json).unwrap();
        assert_eq!(charts.len(), 1);
    }

    #[test]
    fn data_accepts_if_sha256_long_enough() {
        let json = r#"[
            {
                "level": "5",
                "sha256": "0123456789abcdef0123456789a",
                "title": "Has long sha256"
            }
        ]"#;
        let charts = parse_json_data(json).unwrap();
        assert_eq!(charts.len(), 1);
    }

    #[test]
    fn data_filters_missing_level() {
        let json = r#"[
            {
                "md5": "0123456789abcdef0123456789abcdef",
                "title": "No level"
            }
        ]"#;
        let charts = parse_json_data(json).unwrap();
        assert_eq!(charts.len(), 0);
    }

    #[test]
    fn data_filters_empty_level() {
        let json = r#"[
            {
                "level": "",
                "md5": "0123456789abcdef0123456789abcdef",
                "title": "Empty level"
            }
        ]"#;
        let charts = parse_json_data(json).unwrap();
        assert_eq!(charts.len(), 0);
    }

    #[test]
    fn data_numeric_level_converted_to_string() {
        let json = r#"[
            {
                "level": 7,
                "md5": "0123456789abcdef0123456789abcdef",
                "title": "Numeric level"
            }
        ]"#;
        let charts = parse_json_data(json).unwrap();
        assert_eq!(charts.len(), 1);
        assert_eq!(charts[0].level, "7");
    }

    #[test]
    fn data_missing_optional_fields_default_empty() {
        let json = r#"[
            {
                "level": "1",
                "md5": "0123456789abcdef0123456789abcdef"
            }
        ]"#;
        let charts = parse_json_data(json).unwrap();
        assert_eq!(charts.len(), 1);
        assert_eq!(charts[0].title, "");
        assert_eq!(charts[0].artist, "");
        assert_eq!(charts[0].url, "");
    }

    // ---- extract_bmstable_url tests ----

    #[test]
    fn extract_normal_bmstable() {
        let html = r#"<html>
<head>
<meta name="bmstable" content="header.json">
</head>
</html>"#;
        assert_eq!(extract_bmstable_url(html), Some("header.json".to_string()));
    }

    #[test]
    fn extract_case_insensitive_meta() {
        let html = r#"<META NAME="bmstable" CONTENT="header.json">"#;
        assert_eq!(extract_bmstable_url(html), Some("header.json".to_string()));
    }

    #[test]
    fn extract_single_quotes() {
        let html = r#"<meta name='bmstable' content='table/header.json'>"#;
        assert_eq!(
            extract_bmstable_url(html),
            Some("table/header.json".to_string())
        );
    }

    #[test]
    fn extract_bmstable_alt() {
        let html = r#"<meta name="bmstable-alt" content="alt_header.json">"#;
        assert_eq!(
            extract_bmstable_url(html),
            Some("alt_header.json".to_string())
        );
    }

    #[test]
    fn extract_bmstable_preferred_over_alt() {
        let html = r#"
<meta name="bmstable" content="primary.json">
<meta name="bmstable-alt" content="alt.json">
"#;
        assert_eq!(extract_bmstable_url(html), Some("primary.json".to_string()));
    }

    #[test]
    fn extract_no_match() {
        let html = r#"<html><head><title>No table</title></head></html>"#;
        assert_eq!(extract_bmstable_url(html), None);
    }

    #[test]
    fn extract_unrelated_meta() {
        let html = r#"<meta name="description" content="not a table">"#;
        assert_eq!(extract_bmstable_url(html), None);
    }

    // ---- resolve_url tests ----

    #[test]
    fn resolve_absolute_url() {
        assert_eq!(
            resolve_url(
                "http://example.com/table/",
                "https://cdn.example.com/data.json"
            ),
            "https://cdn.example.com/data.json"
        );
    }

    #[test]
    fn resolve_relative_with_dot_slash() {
        assert_eq!(
            resolve_url("http://example.com/table/header.json", "./data.json"),
            "http://example.com/table/data.json"
        );
    }

    #[test]
    fn resolve_relative_without_dot_slash() {
        assert_eq!(
            resolve_url("http://example.com/table/header.json", "data.json"),
            "http://example.com/table/data.json"
        );
    }

    #[test]
    fn resolve_relative_with_subdirectory() {
        assert_eq!(
            resolve_url("http://example.com/table/header.json", "sub/data.json"),
            "http://example.com/table/sub/data.json"
        );
    }

    #[test]
    fn resolve_http_prefix_treated_as_absolute() {
        assert_eq!(
            resolve_url("http://example.com/", "http://other.com/data.json"),
            "http://other.com/data.json"
        );
    }

    #[test]
    fn resolve_root_relative_url() {
        assert_eq!(
            resolve_url("https://example.com/table/header.json", "/data.json"),
            "https://example.com/data.json"
        );
    }

    #[test]
    fn resolve_parent_relative_url() {
        assert_eq!(
            resolve_url("https://example.com/table/sub/header.json", "../data.json"),
            "https://example.com/table/data.json"
        );
    }

    #[test]
    fn resolve_scheme_relative_url() {
        assert_eq!(
            resolve_url(
                "https://example.com/table/header.json",
                "//cdn.example.com/data.json"
            ),
            "https://cdn.example.com/data.json"
        );
    }

    // ---- to_table_data tests ----

    #[test]
    fn to_table_data_level_grouping() {
        let header = DifficultyTableHeader {
            name: "Test Table".to_string(),
            symbol: "★".to_string(),
            tag: "★".to_string(),
            level_order: vec!["1".to_string(), "2".to_string()],
            data_url: vec!["data.json".to_string()],
            data_rule: Vec::new(),
            courses: Vec::new(),
            trophies: Vec::new(),
        };

        let charts = vec![
            ParsedChart {
                level: "1".to_string(),
                md5: "md5a".to_string(),
                sha256: "sha256a".to_string(),
                title: "Song A".to_string(),
                artist: String::new(),
                url: String::new(),
            },
            ParsedChart {
                level: "2".to_string(),
                md5: "md5b".to_string(),
                sha256: "sha256b".to_string(),
                title: "Song B".to_string(),
                artist: String::new(),
                url: String::new(),
            },
            ParsedChart {
                level: "1".to_string(),
                md5: "md5c".to_string(),
                sha256: "sha256c".to_string(),
                title: "Song C".to_string(),
                artist: String::new(),
                url: String::new(),
            },
        ];

        let td = to_table_data(&header, &charts, "http://example.com/table");
        assert_eq!(td.name, "Test Table");
        assert_eq!(td.url, "http://example.com/table");
        assert_eq!(td.tag, "★");
        assert_eq!(td.folder.len(), 2);
        assert_eq!(td.folder[0].name, "★1");
        assert_eq!(td.folder[0].songs.len(), 2);
        assert_eq!(td.folder[0].songs[0].title, "Song A");
        assert_eq!(td.folder[0].songs[1].title, "Song C");
        assert_eq!(td.folder[1].name, "★2");
        assert_eq!(td.folder[1].songs.len(), 1);
        assert_eq!(td.folder[1].songs[0].title, "Song B");
    }

    #[test]
    fn to_table_data_course_conversion() {
        let header = DifficultyTableHeader {
            name: "T".to_string(),
            symbol: "S".to_string(),
            tag: "S".to_string(),
            level_order: Vec::new(),
            data_url: Vec::new(),
            data_rule: Vec::new(),
            courses: vec![vec![ParsedCourse {
                name: "Dan 1".to_string(),
                songs: vec![CourseSongData {
                    sha256: String::new(),
                    md5: "hashA".to_string(),
                    title: String::new(),
                }],
                constraint: vec![
                    CourseDataConstraint::GradeMirror,
                    CourseDataConstraint::GaugeLr2,
                ],
                trophy: vec![TrophyData {
                    name: "Gold".to_string(),
                    missrate: 3.0,
                    scorerate: 85.0,
                }],
            }]],
            trophies: Vec::new(),
        };

        let td = to_table_data(&header, &[], "http://example.com");
        assert_eq!(td.course.len(), 1);
        assert_eq!(td.course[0].name, "Dan 1");
        assert_eq!(td.course[0].hash.len(), 1);
        assert_eq!(td.course[0].hash[0].md5, "hashA");
        assert_eq!(td.course[0].constraint.len(), 2);
        assert_eq!(td.course[0].trophy.len(), 1);
        assert!(td.course[0].release);
    }

    #[test]
    fn to_table_data_empty_charts() {
        let header = DifficultyTableHeader {
            name: "Empty".to_string(),
            symbol: "E".to_string(),
            tag: "E".to_string(),
            level_order: vec!["1".to_string()],
            data_url: Vec::new(),
            data_rule: Vec::new(),
            courses: Vec::new(),
            trophies: Vec::new(),
        };

        let td = to_table_data(&header, &[], "http://example.com");
        assert_eq!(td.folder.len(), 1);
        assert_eq!(td.folder[0].name, "E1");
        assert_eq!(td.folder[0].songs.len(), 0);
    }

    #[test]
    fn to_table_data_charts_with_unknown_level_ignored() {
        let header = DifficultyTableHeader {
            name: "T".to_string(),
            symbol: "S".to_string(),
            tag: "S".to_string(),
            level_order: vec!["1".to_string()],
            data_url: Vec::new(),
            data_rule: Vec::new(),
            courses: Vec::new(),
            trophies: Vec::new(),
        };

        let charts = vec![ParsedChart {
            level: "999".to_string(),
            md5: "md5x".to_string(),
            sha256: String::new(),
            title: "Unknown level".to_string(),
            artist: String::new(),
            url: String::new(),
        }];

        let td = to_table_data(&header, &charts, "http://example.com");
        assert_eq!(td.folder.len(), 1);
        assert_eq!(td.folder[0].songs.len(), 0);
    }

    // ---- data_rule parsing tests ----

    #[test]
    fn header_data_rule_parsed() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "data_url": ["src1.json", "src2.json"],
            "data_rule": [
                {"Beginner": "1", "Normal": "2"},
                {"Easy": "1", "Hard": ""}
            ]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.data_rule.len(), 2);
        assert_eq!(header.data_rule[0].get("Beginner").unwrap(), "1");
        assert_eq!(header.data_rule[0].get("Normal").unwrap(), "2");
        assert_eq!(header.data_rule[1].get("Easy").unwrap(), "1");
        assert_eq!(header.data_rule[1].get("Hard").unwrap(), "");
    }

    #[test]
    fn header_data_rule_absent_defaults_empty() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "data_url": ["data.json"]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert!(header.data_rule.is_empty());
    }

    #[test]
    fn header_data_rule_shorter_than_data_url() {
        let json = r#"{
            "name": "T",
            "symbol": "S",
            "data_url": ["src1.json", "src2.json", "src3.json"],
            "data_rule": [{"A": "1"}]
        }"#;
        let header = parse_json_header(json).unwrap();
        assert_eq!(header.data_rule.len(), 1);
    }

    // ---- apply_data_rule tests ----

    fn make_chart(level: &str, md5: &str) -> ParsedChart {
        ParsedChart {
            level: level.to_string(),
            md5: md5.to_string(),
            sha256: String::new(),
            title: String::new(),
            artist: String::new(),
            url: String::new(),
        }
    }

    #[test]
    fn apply_data_rule_remaps_levels() {
        let charts = vec![make_chart("Beginner", "a"), make_chart("Normal", "b")];
        let rule = HashMap::from([
            ("Beginner".to_string(), "1".to_string()),
            ("Normal".to_string(), "2".to_string()),
        ]);
        let result = apply_data_rule(&charts, &rule);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].level, "1");
        assert_eq!(result[1].level, "2");
    }

    #[test]
    fn apply_data_rule_excludes_empty_mapping() {
        let charts = vec![
            make_chart("Easy", "a"),
            make_chart("Hard", "b"),
            make_chart("Normal", "c"),
        ];
        let rule = HashMap::from([
            ("Easy".to_string(), "1".to_string()),
            ("Hard".to_string(), String::new()),
        ]);
        let result = apply_data_rule(&charts, &rule);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].level, "1");
        assert_eq!(result[0].md5, "a");
        // "Normal" not in rule -> kept as-is
        assert_eq!(result[1].level, "Normal");
        assert_eq!(result[1].md5, "c");
    }

    #[test]
    fn apply_data_rule_empty_rule_no_changes() {
        let charts = vec![make_chart("5", "x"), make_chart("10", "y")];
        let rule = HashMap::new();
        let result = apply_data_rule(&charts, &rule);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].level, "5");
        assert_eq!(result[1].level, "10");
    }
}

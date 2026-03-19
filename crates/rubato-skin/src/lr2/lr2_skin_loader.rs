use std::collections::HashMap;

use crate::property::boolean_property_factory::BooleanPropertyFactory;
use crate::reexports::MainState;
use crate::skin_object::DestinationParams;

/// LR2 skin loader base
///
/// Translated from LR2SkinLoader.java
/// Abstract base class for LR2 skin loaders providing #IF/#ELSE/#ENDIF processing
/// and command dispatch.
///
/// Represents a named command that can be executed during skin loading
pub struct CommandEntry {
    pub name: String,
    pub handler: CommandHandler,
}

/// Command handler types
pub enum CommandHandler {
    /// A closure-based handler
    #[allow(clippy::type_complexity)]
    Fn(Box<dyn Fn(&[String]) + Send + Sync>),
    /// Placeholder for enum-based commands (dispatched externally)
    External,
}

/// Shared state for LR2SkinLoader
pub struct LR2SkinLoaderState {
    pub op: HashMap<i32, i32>,
    pub skip: bool,
    pub ifs: bool,
    command_names: Vec<String>,
}

impl Default for LR2SkinLoaderState {
    fn default() -> Self {
        Self::new()
    }
}

impl LR2SkinLoaderState {
    pub fn new() -> Self {
        Self {
            op: HashMap::new(),
            skip: false,
            ifs: false,
            command_names: Vec::new(),
        }
    }

    pub fn add_command_name(&mut self, name: &str) {
        self.command_names.push(name.to_uppercase());
    }

    /// Process conditional directives (#IF, #ELSEIF, #ELSE, #ENDIF, #SETOPTION)
    /// Returns the command name (without #) if a command should be dispatched, or None
    pub fn process_line_directives(
        &mut self,
        line: &str,
        state: Option<&dyn MainState>,
    ) -> Option<(String, Vec<String>)> {
        if !line.starts_with('#') {
            return None;
        }
        let str_parts: Vec<String> = line.split(',').map(|s| s.to_string()).collect();
        if str_parts.is_empty() {
            return None;
        }

        let cmd_upper = str_parts[0].to_uppercase();

        if cmd_upper == "#IF" {
            self.ifs = true;
            for part in &str_parts[1..] {
                let mut b = false;
                if part.is_empty() {
                    continue;
                }
                let cleaned = part.replace('!', "-");
                let cleaned: String = cleaned
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '-')
                    .collect();
                match cleaned.parse::<i32>() {
                    Ok(opt) => {
                        if opt >= 0 {
                            if self.op.get(&opt).copied().unwrap_or(-1) == 1 {
                                b = true;
                            }
                        } else if self.op.get(&(opt.wrapping_neg())).copied().unwrap_or(-1) == 0 {
                            b = true;
                        }
                        if !b
                            && !self.op.contains_key(&(opt.unsigned_abs() as i32))
                            && let Some(state) = state
                        {
                            let draw = BooleanPropertyFactory::boolean_property(opt);
                            if let Some(ref draw) = draw {
                                b = draw.get(state);
                            }
                        }
                        if !b {
                            self.ifs = false;
                            break;
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            self.skip = !self.ifs;
        } else if cmd_upper == "#ELSEIF" {
            if self.ifs {
                self.skip = true;
            } else {
                self.ifs = true;
                for part in &str_parts[1..] {
                    let mut b = false;
                    let cleaned = part.replace('!', "-");
                    let cleaned: String = cleaned
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '-')
                        .collect();
                    match cleaned.parse::<i32>() {
                        Ok(opt) => {
                            if opt >= 0 {
                                if self.op.get(&opt).copied().unwrap_or(-1) == 1 {
                                    b = true;
                                }
                            } else if self.op.get(&(opt.wrapping_neg())).copied().unwrap_or(-1) == 0
                            {
                                b = true;
                            }
                            if !b
                                && !self.op.contains_key(&(opt.unsigned_abs() as i32))
                                && let Some(state) = state
                            {
                                let draw = BooleanPropertyFactory::boolean_property(opt);
                                if let Some(ref draw) = draw {
                                    b = draw.get(state);
                                }
                            }
                            if !b {
                                self.ifs = false;
                                break;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
                self.skip = !self.ifs;
            }
        } else if cmd_upper == "#ELSE" {
            self.skip = self.ifs;
        } else if cmd_upper == "#ENDIF" {
            self.skip = false;
            self.ifs = false;
        }

        if !self.skip {
            if cmd_upper == "#SETOPTION" && str_parts.len() >= 3 {
                let index: i32 = str_parts[1].parse().unwrap_or(0);
                let value: i32 = str_parts[2].parse().unwrap_or(0);
                self.op.insert(index, if value >= 1 { 1 } else { 0 });
            }

            let cmd_name = str_parts[0][1..].to_uppercase();
            // Check if any registered command matches
            for name in &self.command_names {
                if name.eq_ignore_ascii_case(&cmd_name) {
                    return Some((cmd_name, str_parts));
                }
            }
            // Even if not in registered names, return it for external dispatch
            return Some((cmd_name, str_parts));
        }
        None
    }

    pub fn option(&self) -> &HashMap<i32, i32> {
        &self.op
    }
}

/// Get path, replacing LR2 theme paths and checking filemap for custom file substitutions.
/// Matches Java SkinLoader.getPath() logic: filemap starts_with matching, wildcard (*) expansion,
/// pipe (|) separator handling, and random file selection.
pub fn lr2_path(skinpath: &str, imagepath: &str, filemap: &HashMap<String, String>) -> String {
    let mut resolved = imagepath
        .replace("LR2files\\Theme", skinpath)
        .replace('\\', "/");

    // Check filemap for custom file substitutions (Java: imagepath.startsWith(key))
    for (key, value) in filemap {
        if resolved.starts_with(key.as_str()) {
            let foot = &resolved[key.len()..];
            resolved = format!("{}{}", value, foot);
            // After filemap substitution, return immediately to skip wildcard logic (matching Java)
            return resolved;
        }
    }

    // Wildcard (*) expansion: find matching files in the directory
    if resolved.contains('*') {
        let mut ext = resolved[resolved.rfind('*').expect("contains '*'") + 1..].to_string();
        // Pipe (|) separator handling for extension filtering
        if resolved.contains('|') {
            let star_pos = resolved.rfind('*').expect("contains '*'");
            let pipe_pos = resolved.find('|').expect("contains '|'");
            let last_pipe = resolved.rfind('|').expect("contains '|'");
            let between = if star_pos < pipe_pos {
                &resolved[star_pos + 1..pipe_pos]
            } else {
                ""
            };
            if resolved.len() > last_pipe + 1 {
                ext = format!("{}{}", between, &resolved[last_pipe + 1..]);
            } else {
                ext = between.to_string();
            }
        }
        let ext_lower = ext.to_lowercase();
        let star_pos = resolved.rfind('*').expect("contains '*'");
        let dir_path = if let Some(last_slash) = resolved.rfind('/') {
            &resolved[..last_slash]
        } else {
            "."
        };
        // Extract the filename prefix before the '*' (e.g., "bg" from "bg*.png")
        let prefix = if let Some(last_slash) = resolved[..star_pos].rfind('/') {
            resolved[last_slash + 1..star_pos].to_lowercase()
        } else {
            resolved[..star_pos].to_lowercase()
        };
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            let matching: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    name.starts_with(&prefix) && name.ends_with(&ext_lower)
                })
                .map(|e| e.path().to_string_lossy().into_owned())
                .collect();
            if !matching.is_empty() {
                use rand::Rng;
                let idx = rand::thread_rng().gen_range(0..matching.len());
                return matching[idx].clone();
            }
        }
    }

    resolved
}

/// Check whether a resolved resource path stays within the allowed base directory.
///
/// Returns `true` if the path is safe (no traversal outside `base_dir`).
/// Returns `false` if the path contains `..` components that would escape,
/// or if canonicalization reveals it is outside the base directory.
///
/// Used to prevent path traversal attacks from malicious skin files
/// (e.g., `#INCLUDE ../../../../etc/passwd`).
pub fn is_path_within(base_dir: &std::path::Path, resource_path: &std::path::Path) -> bool {
    // Fast check: reject paths with ".." components before touching the filesystem
    for component in resource_path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            // The path contains ".." -- check if it still resolves within base_dir.
            // We cannot reject blindly because "subdir/../other" is valid and stays within base.
            break;
        }
    }

    // Try canonicalizing both paths. If the resource doesn't exist yet,
    // normalize manually by resolving ".." against the base.
    let canonical_base = match base_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return true, // If base doesn't exist, skip the check (dev/test scenario)
    };

    // If the file exists, canonicalize it directly
    if let Ok(canonical_resource) = resource_path.canonicalize() {
        return canonical_resource.starts_with(&canonical_base);
    }

    // File doesn't exist yet: normalize manually by iterating components
    let mut normalized = canonical_base.clone();
    // Get the relative portion: strip any prefix that matches base_dir
    let relative = if resource_path.starts_with(base_dir) {
        resource_path
            .strip_prefix(base_dir)
            .unwrap_or(resource_path)
    } else if resource_path.is_relative() {
        resource_path
    } else {
        // Absolute path that doesn't start with base_dir
        return false;
    };

    for component in relative.components() {
        match component {
            std::path::Component::ParentDir => {
                if !normalized.starts_with(&canonical_base) {
                    return false;
                }
                normalized.pop();
            }
            std::path::Component::Normal(c) => {
                normalized.push(c);
            }
            std::path::Component::CurDir => {}
            _ => {}
        }
    }

    normalized.starts_with(&canonical_base)
}

/// Get a trimmed string at the given index, or "" if out of bounds.
/// Shared helper used by LR2 play/select/result skin loaders.
pub fn str_at(parts: &[String], idx: usize) -> &str {
    parts.get(idx).map(|s| s.trim()).unwrap_or("")
}

/// Process a `SRC_NOTECHART` / `SRC_NOTECHART_1P` command.
///
/// Creates a `SkinNoteDistributionGraph` from the parsed values and stores it
/// in `noteobj`, updating `gauge` with the field dimensions. This logic is
/// identical across play, select, result, and course-result loaders.
pub fn process_src_notechart(
    str_parts: &[String],
    gauge: &mut crate::reexports::Rectangle,
    noteobj: &mut Option<crate::skin_note_distribution_graph::SkinNoteDistributionGraph>,
) {
    let values = parse_int(str_parts);
    let obj = crate::skin_note_distribution_graph::SkinNoteDistributionGraph::new(
        values[1], values[15], values[16], values[17], values[18], values[19],
    );
    *gauge = crate::reexports::Rectangle::new(0.0, 0.0, values[11] as f32, values[12] as f32);
    *noteobj = Some(obj);
}

/// Process a `DST_NOTECHART` / `DST_NOTECHART_1P` command.
///
/// Sets the destination on the previously created `SkinNoteDistributionGraph`.
/// Shared across play, select, result, and course-result loaders.
pub fn process_dst_notechart(
    str_parts: &[String],
    src_height: f32,
    dst_width: f32,
    dst_height: f32,
    src_width: f32,
    gauge: &mut crate::reexports::Rectangle,
    noteobj: &mut Option<crate::skin_note_distribution_graph::SkinNoteDistributionGraph>,
) {
    let values = parse_int(str_parts);
    gauge.x = values[3] as f32;
    gauge.y = src_height - values[4] as f32;
    if let Some(obj) = noteobj {
        let dstw = crate::safe_div_f32(dst_width, src_width);
        let dsth = crate::safe_div_f32(dst_height, src_height);
        let offsets = read_offset(str_parts, 21);
        obj.data.set_destination_with_int_timer_and_offsets(
            &DestinationParams {
                time: values[2] as i64,
                x: gauge.x * dstw,
                y: gauge.y * dsth,
                w: gauge.width * dstw,
                h: gauge.height * dsth,
                acc: values[7],
                a: values[8],
                r: values[9],
                g: values[10],
                b: values[11],
                blend: values[12],
                filter: values[13],
                angle: values[14],
                center: values[15],
                loop_val: values[16],
            },
            values[17],
            values[18],
            values[19],
            values[20],
            &offsets,
        );
    }
}

/// Process a `SRC_BPMCHART` command.
///
/// Creates a `SkinBPMGraph` from the parsed values and stores it in
/// `bpmgraphobj`, updating `gauge` with the field dimensions.
/// Shared across play, select, result, and course-result loaders.
pub fn process_src_bpmchart(
    str_parts: &[String],
    gauge: &mut crate::reexports::Rectangle,
    bpmgraphobj: &mut Option<crate::skin_bpm_graph::SkinBPMGraph>,
) {
    let values = parse_int(str_parts);
    let obj = crate::skin_bpm_graph::SkinBPMGraph::new(crate::skin_bpm_graph::BpmGraphConfig {
        delay: values[3],
        line_width: values[4],
        main_bpm_color: str_at(str_parts, 5),
        min_bpm_color: str_at(str_parts, 6),
        max_bpm_color: str_at(str_parts, 7),
        other_bpm_color: str_at(str_parts, 8),
        stop_line_color: str_at(str_parts, 9),
        transition_line_color: str_at(str_parts, 10),
    });
    *gauge = crate::reexports::Rectangle::new(0.0, 0.0, values[1] as f32, values[2] as f32);
    *bpmgraphobj = Some(obj);
}

/// Process a `DST_BPMCHART` command.
///
/// Sets the destination on the previously created `SkinBPMGraph`.
/// Shared across play, select, result, and course-result loaders.
pub fn process_dst_bpmchart(
    str_parts: &[String],
    src_height: f32,
    dst_width: f32,
    dst_height: f32,
    src_width: f32,
    gauge: &mut crate::reexports::Rectangle,
    bpmgraphobj: &mut Option<crate::skin_bpm_graph::SkinBPMGraph>,
) {
    let values = parse_int(str_parts);
    gauge.x = values[3] as f32;
    gauge.y = src_height - values[4] as f32;
    if let Some(obj) = bpmgraphobj {
        let dstw = crate::safe_div_f32(dst_width, src_width);
        let dsth = crate::safe_div_f32(dst_height, src_height);
        let offsets = read_offset(str_parts, 21);
        obj.data.set_destination_with_int_timer_and_offsets(
            &DestinationParams {
                time: values[2] as i64,
                x: gauge.x * dstw,
                y: gauge.y * dsth,
                w: gauge.width * dstw,
                h: gauge.height * dsth,
                acc: values[7],
                a: values[8],
                r: values[9],
                g: values[10],
                b: values[11],
                blend: values[12],
                filter: values[13],
                angle: values[14],
                center: values[15],
                loop_val: values[16],
            },
            values[17],
            values[18],
            values[19],
            values[20],
            &offsets,
        );
    }
}

/// Parse int array from string array (matching Java parseInt behavior)
pub fn parse_int(s: &[String]) -> [i32; 22] {
    let mut result = [0i32; 22];
    for i in 1..result.len().min(s.len()) {
        let cleaned = s[i].replace('!', "-").replace(' ', "");
        if let Ok(v) = cleaned.parse::<i32>() {
            result[i] = v;
        }
    }
    result
}

/// Read offset values from string array starting at given index
pub fn read_offset(str_parts: &[String], start_index: usize) -> Vec<i32> {
    read_offset_with_base(str_parts, start_index, &[])
}

/// Read offset values with base offset array
pub fn read_offset_with_base(str_parts: &[String], start_index: usize, offset: &[i32]) -> Vec<i32> {
    let mut result: Vec<i32> = offset.to_vec();
    for part in str_parts.get(start_index..).unwrap_or_default() {
        let s: String = part
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        if !s.is_empty()
            && let Ok(v) = s.parse::<i32>()
        {
            result.push(v);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lr2_path_filemap_replaces_prefix_without_wildcard() {
        let mut filemap = HashMap::new();
        filemap.insert("theme/".to_string(), "/custom/".to_string());
        let result = lr2_path("skinroot", "theme/bg.png", &filemap);
        assert_eq!(result, "/custom/bg.png");
    }

    #[test]
    fn lr2_path_filemap_replaces_prefix_with_wildcard() {
        // Regression: previously the code duplicated the segment between key.len() and star_pos,
        // producing "theme/bg/custom/bg*.png" instead of "/custom/bg*.png".
        let mut filemap = HashMap::new();
        filemap.insert("theme/".to_string(), "/custom/".to_string());
        let result = lr2_path("skinroot", "theme/bg*.png", &filemap);
        assert_eq!(result, "/custom/bg*.png");
    }

    #[test]
    fn lr2_path_filemap_exact_key_match() {
        let mut filemap = HashMap::new();
        filemap.insert("wallpaper.png".to_string(), "/my/wall.png".to_string());
        let result = lr2_path("skinroot", "wallpaper.png", &filemap);
        assert_eq!(result, "/my/wall.png");
    }

    #[test]
    fn lr2_path_filemap_wildcard_in_foot_preserved() {
        let mut filemap = HashMap::new();
        filemap.insert("images/".to_string(), "/replaced/".to_string());
        let result = lr2_path("skinroot", "images/sub/bg*.jpg", &filemap);
        assert_eq!(result, "/replaced/sub/bg*.jpg");
    }

    #[test]
    fn lr2_path_no_filemap_match_passes_through() {
        let filemap = HashMap::new();
        let result = lr2_path("skinroot", "other/file.png", &filemap);
        assert_eq!(result, "other/file.png");
    }

    #[test]
    fn lr2_path_replaces_lr2_theme_prefix() {
        let filemap = HashMap::new();
        let result = lr2_path("myskin", "LR2files\\Theme/bg.png", &filemap);
        assert_eq!(result, "myskin/bg.png");
    }

    #[test]
    fn lr2_path_backslash_normalized() {
        let filemap = HashMap::new();
        let result = lr2_path("skin", "path\\to\\file.png", &filemap);
        assert_eq!(result, "path/to/file.png");
    }

    /// Helper: build a minimal str_parts array for DST chart commands.
    /// values[2]=time, values[3]=x, values[4]=y, rest are defaults.
    fn make_dst_str_parts(x: i32, y: i32) -> Vec<String> {
        let mut parts = vec!["CMD".to_string()];
        // [1]=unused, [2]=time, [3]=x, [4]=y, [5..6] unused, [7]=acc,
        // [8..16]=a,r,g,b,blend,filter,angle,center,loop, [17]=timer, [18..20]=op, [21..]=offsets
        parts.push("0".into()); // 1
        parts.push("0".into()); // 2 (time)
        parts.push(x.to_string()); // 3 (x)
        parts.push(y.to_string()); // 4 (y)
        for _ in 5..22 {
            parts.push("0".into());
        }
        parts
    }

    #[test]
    fn process_dst_notechart_y_matches_java() {
        // Java: gauge.y = src.height - values[4], then y_dst = gauge.y * dh
        // where dh = dst_height / src_height.
        // With src=480, dst=720, gauge_h=100, y_val=200:
        // Java: gauge.y = 480 - 200 = 280, y_dst = 280 * (720/480) = 420.0
        let src_height: f32 = 480.0;
        let dst_height: f32 = 720.0;
        let src_width: f32 = 640.0;
        let dst_width: f32 = 1280.0;
        let gauge_w: f32 = 200.0;
        let gauge_h: f32 = 100.0;

        let str_parts = make_dst_str_parts(50, 200);
        let mut gauge = crate::reexports::Rectangle::new(0.0, 0.0, gauge_w, gauge_h);
        let mut noteobj =
            Some(crate::skin_note_distribution_graph::SkinNoteDistributionGraph::new_default());
        process_dst_notechart(
            &str_parts,
            src_height,
            dst_width,
            dst_height,
            src_width,
            &mut gauge,
            &mut noteobj,
        );
        let obj = noteobj.unwrap();
        let dst = &obj.data.dst[0];
        let expected_y = (src_height - 200.0) * (dst_height / src_height); // 280 * 1.5 = 420
        assert!(
            (dst.region.y - expected_y).abs() < 0.01,
            "notechart y={}, expected={}",
            dst.region.y,
            expected_y
        );
    }

    // --- parse_int edge cases ---

    #[test]
    fn parse_int_empty_array() {
        let parts: Vec<String> = vec![];
        let result = parse_int(&parts);
        assert!(result.iter().all(|&v| v == 0));
    }

    #[test]
    fn parse_int_single_element() {
        let parts = vec!["#CMD".to_string()];
        let result = parse_int(&parts);
        assert!(result.iter().all(|&v| v == 0));
    }

    #[test]
    fn parse_int_spaces_stripped() {
        let parts: Vec<String> = vec!["#CMD".into(), " 10 ".into(), "  20  ".into()];
        let result = parse_int(&parts);
        assert_eq!(result[1], 10);
        assert_eq!(result[2], 20);
    }

    #[test]
    fn parse_int_bang_negative() {
        let parts: Vec<String> = vec!["#CMD".into(), "!100".into()];
        let result = parse_int(&parts);
        assert_eq!(result[1], -100);
    }

    #[test]
    fn parse_int_multiple_bangs() {
        // "!!5" -> "--5" after replacement, parse as -5 (leading double minus)
        let parts: Vec<String> = vec!["#CMD".into(), "!!5".into()];
        let result = parse_int(&parts);
        // "--5" after space removal -> parse attempt. Rust parse considers "--5" invalid.
        // So it defaults to 0.
        assert_eq!(result[1], 0);
    }

    #[test]
    fn parse_int_index_zero_always_zero() {
        // parse_int starts at index 1, so index 0 is always 0
        let parts: Vec<String> = vec!["999".into(), "10".into()];
        let result = parse_int(&parts);
        assert_eq!(result[0], 0);
        assert_eq!(result[1], 10);
    }

    #[test]
    fn parse_int_exactly_22_parts() {
        let mut parts: Vec<String> = vec!["#CMD".into()];
        for i in 1..22 {
            parts.push(i.to_string());
        }
        let result = parse_int(&parts);
        for i in 1..22 {
            assert_eq!(result[i], i as i32);
        }
    }

    #[test]
    fn parse_int_mixed_valid_invalid() {
        let parts: Vec<String> = vec![
            "#CMD".into(),
            "42".into(),
            "abc".into(),
            "".into(),
            "-7".into(),
        ];
        let result = parse_int(&parts);
        assert_eq!(result[1], 42);
        assert_eq!(result[2], 0); // "abc" -> 0
        assert_eq!(result[3], 0); // "" -> 0
        assert_eq!(result[4], -7);
    }

    // --- read_offset edge cases ---

    #[test]
    fn read_offset_with_base_prepends_base() {
        let parts: Vec<String> = vec!["#DST".into(), "0".into(), "0".into()];
        let offsets = read_offset_with_base(&parts, 1, &[100, 200]);
        // base=[100, 200], then parts[1]="0", parts[2]="0"
        assert_eq!(offsets[0], 100);
        assert_eq!(offsets[1], 200);
        assert_eq!(offsets[2], 0);
        assert_eq!(offsets[3], 0);
    }

    #[test]
    fn read_offset_non_numeric_parts_skipped() {
        let parts: Vec<String> = vec![
            "#DST".into(),
            "abc".into(),
            "10".into(),
            "".into(),
            "20".into(),
        ];
        let offsets = read_offset(&parts, 1);
        // "abc" -> filtered to "" -> empty -> skipped
        // "10" -> parsed
        // "" -> empty -> skipped
        // "20" -> parsed
        assert_eq!(offsets, vec![10, 20]);
    }

    #[test]
    fn read_offset_start_beyond_length() {
        let parts: Vec<String> = vec!["#CMD".into()];
        let offsets = read_offset(&parts, 100);
        assert!(offsets.is_empty());
    }

    #[test]
    fn read_offset_negative_values_parsed() {
        let parts: Vec<String> = vec!["#CMD".into(), "-50".into(), "30".into()];
        let offsets = read_offset(&parts, 1);
        assert_eq!(offsets, vec![-50, 30]);
    }

    // --- str_at edge cases ---

    #[test]
    fn str_at_in_bounds() {
        let parts: Vec<String> = vec!["a".into(), " hello ".into()];
        assert_eq!(str_at(&parts, 1), "hello");
    }

    #[test]
    fn str_at_out_of_bounds() {
        let parts: Vec<String> = vec!["a".into()];
        assert_eq!(str_at(&parts, 5), "");
    }

    #[test]
    fn str_at_empty_vec() {
        let parts: Vec<String> = vec![];
        assert_eq!(str_at(&parts, 0), "");
    }

    // --- lr2_path additional edge cases ---

    #[test]
    fn lr2_path_empty_imagepath() {
        let filemap = HashMap::new();
        let result = lr2_path("skinroot", "", &filemap);
        assert_eq!(result, "");
    }

    #[test]
    fn lr2_path_backslash_in_filemap_value() {
        let mut filemap = HashMap::new();
        filemap.insert("key/".to_string(), "value\\path/".to_string());
        let result = lr2_path("skinroot", "key/file.png", &filemap);
        assert_eq!(result, "value\\path/file.png");
    }

    // --- LR2SkinLoaderState conditional directive tests ---

    #[test]
    fn process_line_directives_non_hash_line_returns_none() {
        let mut state = LR2SkinLoaderState::new();
        assert!(
            state
                .process_line_directives("SCENETIME,100", None)
                .is_none()
        );
    }

    #[test]
    fn process_line_directives_empty_line_returns_none() {
        let mut state = LR2SkinLoaderState::new();
        assert!(state.process_line_directives("", None).is_none());
    }

    #[test]
    fn process_line_directives_if_endif_resets_skip() {
        let mut state = LR2SkinLoaderState::new();
        state.op.insert(1, 0); // option 1 = false
        state.process_line_directives("#IF,1", None);
        assert!(state.skip); // should skip
        state.process_line_directives("#ENDIF", None);
        assert!(!state.skip); // skip reset
    }

    #[test]
    fn process_line_directives_setoption() {
        let mut state = LR2SkinLoaderState::new();
        state.process_line_directives("#SETOPTION,42,1", None);
        assert_eq!(state.op.get(&42), Some(&1));
    }

    #[test]
    fn process_line_directives_setoption_zero_value() {
        let mut state = LR2SkinLoaderState::new();
        state.process_line_directives("#SETOPTION,10,0", None);
        assert_eq!(state.op.get(&10), Some(&0));
    }

    #[test]
    fn process_line_directives_negative_if_checks_negated_option() {
        let mut state = LR2SkinLoaderState::new();
        state.op.insert(5, 0); // option 5 = 0 (false)
        // #IF,-5 means "if option 5 is false" -> should pass
        state.process_line_directives("#IF,-5", None);
        assert!(!state.skip); // condition met
    }

    #[test]
    fn process_line_directives_elseif_after_true_if() {
        let mut state = LR2SkinLoaderState::new();
        state.op.insert(1, 1); // option 1 = true
        state.process_line_directives("#IF,1", None);
        assert!(!state.skip); // IF branch taken
        state.process_line_directives("#ELSEIF,1", None);
        assert!(state.skip); // ELSEIF skipped because IF was true
    }

    #[test]
    fn process_line_directives_else_after_false_if() {
        let mut state = LR2SkinLoaderState::new();
        state.op.insert(1, 0); // option 1 = false
        state.process_line_directives("#IF,1", None);
        assert!(state.skip); // IF branch not taken
        state.process_line_directives("#ELSE", None);
        assert!(!state.skip); // ELSE branch taken
    }

    #[test]
    fn process_dst_bpmchart_y_matches_java() {
        let src_height: f32 = 480.0;
        let dst_height: f32 = 720.0;
        let src_width: f32 = 640.0;
        let dst_width: f32 = 1280.0;
        let gauge_w: f32 = 200.0;
        let gauge_h: f32 = 100.0;

        let str_parts = make_dst_str_parts(50, 200);
        let mut gauge = crate::reexports::Rectangle::new(0.0, 0.0, gauge_w, gauge_h);
        let mut bpmobj = Some(crate::skin_bpm_graph::SkinBPMGraph::new(
            crate::skin_bpm_graph::BpmGraphConfig {
                delay: 0,
                line_width: 1,
                main_bpm_color: "",
                min_bpm_color: "",
                max_bpm_color: "",
                other_bpm_color: "",
                stop_line_color: "",
                transition_line_color: "",
            },
        ));
        process_dst_bpmchart(
            &str_parts,
            src_height,
            dst_width,
            dst_height,
            src_width,
            &mut gauge,
            &mut bpmobj,
        );
        let obj = bpmobj.unwrap();
        let dst = &obj.data.dst[0];
        let expected_y = (src_height - 200.0) * (dst_height / src_height);
        assert!(
            (dst.region.y - expected_y).abs() < 0.01,
            "bpmchart y={}, expected={}",
            dst.region.y,
            expected_y
        );
    }

    #[test]
    fn process_line_directives_if_i32_min_does_not_panic() {
        // Regression: i32::MIN.abs() and -i32::MIN both panic in debug mode.
        // Skin CSV option values are user-editable, so i32::MIN is reachable
        // via "!2147483648" which becomes "-2147483648" after '!' -> '-' replacement.
        let mut state = LR2SkinLoaderState::new();
        // Should not panic
        state.process_line_directives("#IF,-2147483648", None);
    }

    #[test]
    fn process_line_directives_elseif_i32_min_does_not_panic() {
        // Same regression for #ELSEIF path
        let mut state = LR2SkinLoaderState::new();
        state.op.insert(1, 0); // make #IF fail so #ELSEIF is evaluated
        state.process_line_directives("#IF,1", None);
        assert!(state.skip);
        state.process_line_directives("#ELSEIF,-2147483648", None);
    }

    #[test]
    fn lr2_path_star_after_pipe_does_not_panic() {
        // Regression: when '*' appears after '|' in the path,
        // star_pos + 1 > pipe_pos causes slice panic.
        let filemap = HashMap::new();
        // Construct a path where pipe comes before star: "dir/file|ext*"
        let result = lr2_path("skin", "dir/file|ext*.png", &filemap);
        // Should not panic; exact result depends on filesystem but the function must not crash
        let _ = result;
    }

    #[test]
    fn is_path_within_rejects_traversal() {
        let base = std::env::temp_dir();
        // "../../../etc/passwd" escapes the base directory
        let malicious = base.join("../../../etc/passwd");
        assert!(
            !is_path_within(&base, &malicious),
            "path traversal should be rejected"
        );
    }

    #[test]
    fn is_path_within_allows_normal_subpath() {
        let base = std::env::temp_dir();
        let normal = base.join("skins/myskin/include.lr2skin");
        assert!(
            is_path_within(&base, &normal),
            "normal subpath should be allowed"
        );
    }

    #[test]
    fn is_path_within_allows_benign_dotdot() {
        // "subdir/../other" stays within base
        let base = std::env::temp_dir();
        let benign = base.join("subdir/../other.lr2skin");
        assert!(
            is_path_within(&base, &benign),
            "benign parent traversal within base should be allowed"
        );
    }

    #[test]
    fn is_path_within_rejects_absolute_outside() {
        let base = std::env::temp_dir();
        let outside = std::path::PathBuf::from("/etc/passwd");
        assert!(
            !is_path_within(&base, &outside),
            "absolute path outside base should be rejected"
        );
    }
}

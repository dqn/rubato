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
                        } else if self.op.get(&(-opt)).copied().unwrap_or(-1) == 0 {
                            b = true;
                        }
                        if !b
                            && !self.op.contains_key(&opt.abs())
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
                            } else if self.op.get(&(-opt)).copied().unwrap_or(-1) == 0 {
                                b = true;
                            }
                            if !b
                                && !self.op.contains_key(&opt.abs())
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
            if resolved.len() > last_pipe + 1 {
                ext = format!(
                    "{}{}",
                    &resolved[star_pos + 1..pipe_pos],
                    &resolved[last_pipe + 1..]
                );
            } else {
                ext = resolved[star_pos + 1..pipe_pos].to_string();
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
        obj.data.set_destination_with_int_timer_ops(
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
        obj.data.set_destination_with_int_timer_ops(
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
}

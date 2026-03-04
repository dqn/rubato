use std::collections::HashMap;

use crate::property::boolean_property_factory::BooleanPropertyFactory;
use crate::stubs::MainState;

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
            for i in 1..str_parts.len() {
                let mut b = false;
                if str_parts[i].is_empty() {
                    continue;
                }
                let cleaned = str_parts[i].replace('!', "-");
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
                            let draw = BooleanPropertyFactory::get_boolean_property(opt);
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
                for i in 1..str_parts.len() {
                    let mut b = false;
                    let cleaned = str_parts[i].replace('!', "-");
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
                                let draw = BooleanPropertyFactory::get_boolean_property(opt);
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

    pub fn get_option(&self) -> &HashMap<i32, i32> {
        &self.op
    }
}

/// Get path, replacing LR2 theme paths and checking filemap for custom file substitutions.
/// Matches Java SkinLoader.getPath() logic: filemap starts_with matching, wildcard (*) expansion,
/// pipe (|) separator handling, and random file selection.
pub fn get_lr2_path(skinpath: &str, imagepath: &str, filemap: &HashMap<String, String>) -> String {
    let mut resolved = imagepath
        .replace("LR2files\\Theme", skinpath)
        .replace('\\', "/");

    // Check filemap for custom file substitutions (Java: imagepath.startsWith(key))
    for (key, value) in filemap {
        if resolved.starts_with(key.as_str()) {
            let foot = &resolved[key.len()..];
            if let Some(star_pos) = resolved.rfind('*') {
                resolved = format!("{}{}{}", &resolved[..star_pos], value, foot);
            } else {
                resolved = format!("{}{}", value, foot);
            }
            // After filemap substitution, clear resolved to skip wildcard logic (matching Java)
            return resolved;
        }
    }

    // Wildcard (*) expansion: find matching files in the directory
    if resolved.contains('*') {
        let mut ext = resolved[resolved.rfind('*').unwrap() + 1..].to_string();
        // Pipe (|) separator handling for extension filtering
        if resolved.contains('|') {
            let star_pos = resolved.rfind('*').unwrap();
            let pipe_pos = resolved.find('|').unwrap();
            let last_pipe = resolved.rfind('|').unwrap();
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
        if let Some(last_slash) = resolved.rfind('/') {
            let dir_path = &resolved[..last_slash];
            if let Ok(entries) = std::fs::read_dir(dir_path) {
                let matching: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .to_string_lossy()
                            .to_lowercase()
                            .ends_with(&ext_lower)
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
    }

    resolved
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
    for i in start_index..str_parts.len() {
        let s: String = str_parts[i]
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

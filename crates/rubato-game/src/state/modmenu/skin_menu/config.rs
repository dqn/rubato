// Skin configuration management: loading, saving, scanning, and settings access.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::super::{
    CustomFile, CustomOffset, CustomOption, JSONSkinLoader, LR2SkinHeaderLoader, LuaSkinLoader,
    SkinConfig, SkinFilePath, SkinHeader, SkinOffset, SkinOption, SkinProperty, SkinType,
    TYPE_LR2SKIN, Validatable,
};
use super::header_converters::{skin_header_from_json_data, skin_header_from_lr2_data};
use super::{
    AVAILABLE_FILES, CURRENT_SKIN, CURRENT_SKIN_TYPE, DIRTY_CONFIG, OffsetValue, READY, SET_FILES,
    SET_OFFSETS, SET_OPTIONS, SKIN_MENU_STATE,
};
use rubato_types::main_controller_access::MainControllerCommand;
use rubato_types::sync_utils::lock_or_recover;

pub(super) fn refresh() {
    *lock_or_recover(&SET_OPTIONS) = None;
    *lock_or_recover(&AVAILABLE_FILES) = None;
    *lock_or_recover(&SET_FILES) = None;
    *lock_or_recover(&SET_OFFSETS) = None;

    // observedState = main.getCurrentState();
    // SkinHeader currentSceneSkin = observedState.getSkin().header;
    // currentSkinType = currentSceneSkin.getSkinType();
    // currentSkin = null;
    // switchCurrentSceneSkin(currentSceneSkin);
    // skins = loadAllSkins(currentSkinType);
    *lock_or_recover(&READY) = true;
}

#[allow(dead_code)]
pub(super) fn load_all_skins(skin_type: &SkinType) -> Vec<SkinHeader> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let skins_dir = PathBuf::from("skin");
    scan_skins(&skins_dir, &mut paths);

    let mut skins: Vec<SkinHeader> = Vec::new();
    let current_skin = lock_or_recover(&CURRENT_SKIN);

    for path in &paths {
        let path_string = path.to_string_lossy().to_lowercase();
        let mut header: Option<SkinHeader> = None;

        if let Some(ref cs) = *current_skin
            && cs.path().is_some_and(|p| path == p)
        {
            header = Some(cs.clone());
        }

        if header.is_none() {
            if path_string.ends_with(".json") {
                let mut loader = JSONSkinLoader::new();
                header = loader.load_header(path).map(skin_header_from_json_data);
            } else if path_string.ends_with(".luaskin") {
                let mut loader = LuaSkinLoader::new();
                let _ = loader.load_header(path);
                // header stays None -- lua skin loader not yet fully implemented
            } else if path_string.ends_with(".lr2skin") {
                let main_present = lock_or_recover(&SKIN_MENU_STATE).main.is_some();
                if main_present {
                    let mut loader = LR2SkinHeaderLoader::new("");
                    match loader.load_skin(path, None).map(skin_header_from_lr2_data) {
                        Ok(mut h) => {
                            // 7/14key skin can also be used for 5/10key
                            if *skin_type == SkinType::Play5Keys
                                && h.skin_type().is_some_and(|st| *st == SkinType::Play7Keys)
                                && h.toast_type() == TYPE_LR2SKIN
                                && let Ok(mut h2) =
                                    loader.load_skin(path, None).map(skin_header_from_lr2_data)
                            {
                                h2.set_skin_type(SkinType::Play5Keys);
                                if !h2.name().unwrap_or("").to_lowercase().contains("7key") {
                                    let new_name = format!("{} (7KEYS) ", h2.name().unwrap_or(""));
                                    h2.set_name(new_name);
                                }
                                h = h2;
                            }
                            if *skin_type == SkinType::Play10Keys
                                && h.skin_type().is_some_and(|st| *st == SkinType::Play14Keys)
                                && h.toast_type() == TYPE_LR2SKIN
                                && let Ok(mut h2) =
                                    loader.load_skin(path, None).map(skin_header_from_lr2_data)
                            {
                                h2.set_skin_type(SkinType::Play10Keys);
                                if !h2.name().unwrap_or("").to_lowercase().contains("14key") {
                                    let new_name = format!("{} (14KEYS) ", h2.name().unwrap_or(""));
                                    h2.set_name(new_name);
                                }
                                h = h2;
                            }
                            header = Some(h);
                        }
                        Err(_e) => {
                            // e.printStackTrace()
                        }
                    }
                }
            }
        }

        if let Some(h) = header
            && h.skin_type().is_some_and(|st| st == skin_type)
        {
            skins.push(h);
        }
    }

    skins
}

#[allow(dead_code)]
fn scan_skins(path: &Path, paths: &mut Vec<PathBuf>) {
    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                scan_skins(&entry.path(), paths);
            }
        }
    } else {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if name.ends_with(".lr2skin") || name.ends_with(".luaskin") || name.ends_with(".json") {
            paths.push(path.to_path_buf());
        }
    }
}

pub(super) fn matches_skin_file_pattern_case_insensitive(filename: &str, pattern: &str) -> bool {
    let normalized_filename = filename.to_ascii_lowercase();
    let normalized_pattern = pattern.to_ascii_lowercase();

    if !normalized_pattern.contains('*') {
        return normalized_filename == normalized_pattern;
    }

    let parts: Vec<&str> = normalized_pattern
        .split('*')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return true;
    }

    let mut search_start = 0usize;
    for (index, part) in parts.iter().enumerate() {
        if index == 0 && !normalized_pattern.starts_with('*') {
            if !normalized_filename[search_start..].starts_with(part) {
                return false;
            }
            search_start += part.len();
            continue;
        }

        let Some(relative_pos) = normalized_filename[search_start..].find(part) else {
            return false;
        };
        search_start += relative_pos + part.len();
    }

    if !normalized_pattern.ends_with('*')
        && let Some(last_part) = parts.last()
    {
        return normalized_filename.ends_with(last_part);
    }

    true
}

#[cfg(test)]
pub(super) fn parse_custom_file(file: &CustomFile) -> Option<Vec<String>> {
    parse_custom_file_with_skin_path(file, None)
}

fn parse_custom_file_with_skin_path(
    file: &CustomFile,
    skin_path: Option<&Path>,
) -> Option<Vec<String>> {
    let mut file_selection: Vec<String> = Vec::new();

    let last_slash = file.path.rfind('/');
    let last_slash_idx = last_slash.unwrap_or(0);
    let name = if last_slash.is_some() {
        &file.path[last_slash_idx + 1..]
    } else {
        &file.path
    };

    let name = if file.path.contains('|') {
        let pipe_idx = file.path.rfind('|').expect("contains '|'");
        if file.path.len() > pipe_idx + 1 {
            let first_pipe = file.path.find('|').expect("contains '|'");
            let start = if last_slash.is_some() {
                last_slash_idx + 1
            } else {
                0
            };
            format!(
                "{}{}",
                &file.path[start..first_pipe],
                &file.path[pipe_idx + 1..]
            )
        } else {
            let first_pipe = file.path.find('|').expect("contains '|'");
            let start = if last_slash.is_some() {
                last_slash_idx + 1
            } else {
                0
            };
            file.path[start..first_pipe].to_string()
        }
    } else {
        name.to_string()
    };

    let raw_dir = if last_slash.is_some() {
        PathBuf::from(&file.path[..last_slash_idx])
    } else {
        PathBuf::from(".")
    };
    // Resolve relative custom-file paths from the skin directory,
    // not from the process working directory.
    let dirpath = if raw_dir.is_relative() {
        if let Some(skin_dir) = skin_path.and_then(|p| p.parent()) {
            skin_dir.join(&raw_dir)
        } else {
            raw_dir
        }
    } else {
        raw_dir
    };

    if !dirpath.exists() {
        return None;
    }

    // In Java: Files.newDirectoryStream(dirpath, "{name.lower(),name.upper()}")
    if let Ok(entries) = fs::read_dir(&dirpath) {
        for entry in entries.flatten() {
            let entry_name = entry.file_name().to_string_lossy().to_string();
            if matches_skin_file_pattern_case_insensitive(&entry_name, &name) {
                file_selection.push(entry_name);
            }
        }
    }
    file_selection.push("Random".to_string());

    Some(file_selection)
}

pub(super) fn load_saved_skin_settings(header: &SkinHeader) {
    let skin_path = header
        .path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let menu_state = lock_or_recover(&SKIN_MENU_STATE);

    let Some(ref pc) = menu_state.player_config else {
        return;
    };

    let mut saved_properties: Option<&SkinProperty> = None;

    let skin_type_id = header.skin_type().map(|st| st.id()).unwrap_or(0) as usize;
    if skin_type_id < pc.skin.len()
        && let Some(ref live_config) = pc.skin[skin_type_id]
        && live_config.path().is_some_and(|p| p == skin_path)
    {
        saved_properties = live_config.properties();
    }

    if saved_properties.is_none() {
        for saved_config in &pc.skin_history {
            if saved_config.path().is_some_and(|p| p == skin_path) {
                saved_properties = saved_config.properties();
                break;
            }
        }
    }

    if let Some(props) = saved_properties {
        let mut options = lock_or_recover(&SET_OPTIONS);
        let opt_map = options.get_or_insert_with(HashMap::new);
        for option in props.option.iter().flatten() {
            if let Some(ref name) = option.name {
                opt_map.insert(name.clone(), option.value);
            }
        }

        let mut files = lock_or_recover(&SET_FILES);
        let file_map = files.get_or_insert_with(HashMap::new);
        for file in props.file.iter().flatten() {
            if let (Some(name), Some(path)) = (&file.name, &file.path) {
                file_map.insert(name.clone(), path.clone());
            }
        }

        let mut offsets = lock_or_recover(&SET_OFFSETS);
        let offset_map = offsets.get_or_insert_with(HashMap::new);
        for offset in props.offset.iter().flatten() {
            if let Some(ref name) = offset.name {
                offset_map.insert(
                    name.clone(),
                    OffsetValue::new(offset.x, offset.y, offset.w, offset.h, offset.r, offset.a),
                );
            }
        }
    }
}

pub(super) fn get_option_setting(option: &CustomOption) -> i32 {
    let options = lock_or_recover(&SET_OPTIONS);
    if let Some(ref map) = *options
        && let Some(&value) = map.get(&option.name)
    {
        return value;
    }
    option.default_option()
}

pub(super) fn get_file_setting(file: &CustomFile) -> Option<String> {
    let files = lock_or_recover(&SET_FILES);
    if let Some(ref map) = *files
        && let Some(path) = map.get(&file.name)
    {
        return Some(path.clone());
    }
    file.def.clone()
}

pub(super) fn get_offset_setting(offset: &CustomOffset) -> OffsetValue {
    let mut offsets = lock_or_recover(&SET_OFFSETS);
    let map = offsets.get_or_insert_with(HashMap::new);
    *map.entry(offset.name.clone())
        .or_insert_with(|| OffsetValue::new(0, 0, 0, 0, 0, 0))
}

pub(super) fn complete_property(header: &SkinHeader) -> SkinProperty {
    // default out all unset properties and collect everything into the property object
    let mut options: Vec<Option<SkinOption>> = Vec::new();
    let mut files: Vec<Option<SkinFilePath>> = Vec::new();
    let mut offsets: Vec<Option<SkinOffset>> = Vec::new();

    for option in header.custom_options() {
        let value = get_option_setting(option);
        let mut opt_map = lock_or_recover(&SET_OPTIONS);
        let map = opt_map.get_or_insert_with(HashMap::new);
        map.insert(option.name.clone(), value);
        options.push(Some(SkinOption {
            name: Some(option.name.clone()),
            value,
        }));
    }

    for file in header.custom_files() {
        let file_selection =
            parse_custom_file_with_skin_path(file, header.path().map(|p| p.as_path()))
                .unwrap_or_else(|| vec!["Random".to_string()]);

        {
            let mut available = lock_or_recover(&AVAILABLE_FILES);
            let map = available.get_or_insert_with(HashMap::new);
            map.insert(file.name.clone(), file_selection.clone());
        }

        let mut selection = {
            let files_map = lock_or_recover(&SET_FILES);
            files_map.as_ref().and_then(|m| m.get(&file.name).cloned())
        };

        if selection.is_none()
            && let Some(ref def) = file.def
        {
            for filename in &file_selection {
                if filename.eq_ignore_ascii_case(def) {
                    selection = Some(filename.clone());
                    break;
                }
                if let Some(point) = filename.rfind('.')
                    && filename[..point].eq_ignore_ascii_case(def)
                {
                    selection = Some(filename.clone());
                    break;
                }
            }
        }

        // fileSelection[0] always present due to inserted 'Random'
        if selection.is_none() {
            selection = file_selection.first().cloned();
        }

        let sel = selection.unwrap_or_default();
        {
            let mut files_map = lock_or_recover(&SET_FILES);
            let map = files_map.get_or_insert_with(HashMap::new);
            map.insert(file.name.clone(), sel.clone());
        }

        files.push(Some(SkinFilePath {
            name: Some(file.name.clone()),
            path: Some(sel),
        }));
    }

    for offset in header.custom_offsets() {
        let value = get_offset_setting(offset);
        offsets.push(Some(SkinOffset {
            name: Some(offset.name.clone()),
            x: value.x,
            y: value.y,
            w: value.w,
            h: value.h,
            r: value.r,
            a: value.a,
        }));
    }

    SkinProperty {
        option: options,
        file: files,
        offset: offsets,
    }
}

pub(super) fn dirty(flag: bool) {
    if flag {
        *lock_or_recover(&DIRTY_CONFIG) = true;
    }
}

pub(super) fn save_current_config(next_skin: &SkinHeader) {
    *lock_or_recover(&DIRTY_CONFIG) = false;

    let current_skin = lock_or_recover(&CURRENT_SKIN);
    if current_skin.is_none() {
        return;
    }
    let cs = current_skin.as_ref().expect("current_skin is Some");

    let skin_path = cs
        .path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let property = complete_property(cs);
    let config = SkinConfig {
        path: Some(skin_path.clone()),
        properties: Some(property),
    };

    // Determine command to push while holding SKIN_MENU_STATE, then release
    // before actually pushing (push_command re-acquires the lock for the queue).
    let command = {
        let mut menu_state = lock_or_recover(&SKIN_MENU_STATE);
        let Some(ref mut pc) = menu_state.player_config else {
            return;
        };

        let current_type = lock_or_recover(&CURRENT_SKIN_TYPE);
        if let Some(ref st) = *current_type
            && next_skin.name() == cs.name()
        {
            let id = st.id() as usize;
            if id < pc.skin.len() {
                pc.skin[id] = Some(config.clone());
            }
            MainControllerCommand::UpdateSkinConfig(id, Some(Box::new(config)))
        } else if let Some(entry) = pc
            .skin_history
            .iter_mut()
            .find(|h| h.path().is_some_and(|p| p == skin_path))
        {
            *entry = config.clone();
            MainControllerCommand::UpdateSkinHistory(skin_path, Box::new(config))
        } else {
            // this skin hasn't been in the config history before, add it
            pc.skin_history.push(config.clone());
            MainControllerCommand::UpdateSkinHistory(skin_path, Box::new(config))
        }
    };
    drop(current_skin);

    push_command(command);
}

/// Push a command to MainController via the command queue in SKIN_MENU_STATE.
fn push_command(command: MainControllerCommand) {
    let state = lock_or_recover(&SKIN_MENU_STATE);
    if let Some(ref q) = state.command_queue {
        q.push(command);
    }
}

pub(super) fn reset_current_skin_config() {
    *lock_or_recover(&SET_OPTIONS) = Some(HashMap::new());
    *lock_or_recover(&AVAILABLE_FILES) = Some(HashMap::new());
    *lock_or_recover(&SET_FILES) = Some(HashMap::new());
    *lock_or_recover(&SET_OFFSETS) = Some(HashMap::new());
}

pub(super) fn switch_current_scene_skin(header: SkinHeader) {
    {
        let current = lock_or_recover(&CURRENT_SKIN);
        if current.is_some() {
            drop(current);
            save_current_config(&header);
        }
    }

    reset_current_skin_config();
    load_saved_skin_settings(&header);

    *lock_or_recover(&CURRENT_SKIN) = Some(header.clone());
    let _property = complete_property(&header);

    let skin_path = header
        .path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut config = SkinConfig {
        path: Some(skin_path),
        properties: Some(_property),
    };
    config.validate();

    // MainState scene = main.getCurrentState();
    // Skin skin = SkinLoader.load(scene, currentSkinType, config);
    // if (skin == null) { ... fallback ... }
    // playerConfig.getSkin()[currentSkinType.getId()] = config;
    // scene.setSkin(skin);
    // skin.prepare(scene);
    // if (scene instanceof MusicSelector) { ((MusicSelector)scene).getBarRender().updateBarText(); }
}

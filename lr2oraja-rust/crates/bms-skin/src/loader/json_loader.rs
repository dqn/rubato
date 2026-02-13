// JSON skin loader.
//
// Loads beatoraja-format JSON skin files and converts them into the
// Skin data model.
//
// The loading pipeline:
// 1. Read JSON file (UTF-8, with Shift_JIS fallback)
// 2. Pre-process: resolve conditional branches and file includes
// 3. Deserialize into JsonSkinData
// 4. Convert to SkinHeader (for skin selection UI)
// 5. Convert to Skin (full skin with all objects)

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use bms_config::resolution::Resolution;
use bms_config::skin_type::SkinType;

use crate::custom_event::{CustomEventDef, CustomTimerDef};
use crate::image_handle::ImageHandle;
use crate::loader::json_skin::{FlexId, JsonAnimation, JsonDestination, JsonSkinData};
use crate::property_id::{
    BooleanId, EventId, OFFSET_ALL, OFFSET_JUDGE_1P, OFFSET_JUDGEDETAIL_1P, OFFSET_NOTES_1P,
    TimerId,
};
use crate::skin::Skin;
use crate::skin_header::{
    CustomCategory, CustomCategoryItem, CustomFile, CustomOffset, CustomOption, SkinFormat,
    SkinHeader,
};
use crate::skin_object::{Color, Destination, Rect, SkinObjectBase};
use crate::skin_object_type::SkinObjectType;
use crate::skin_text::{FontType, TextShadow};
use crate::skin_visualizer::parse_color;
use crate::stretch_type::StretchType;

/// All Resolution variants for dimension-based lookup.
const ALL_RESOLUTIONS: [Resolution; 15] = [
    Resolution::Sd,
    Resolution::Svga,
    Resolution::Xga,
    Resolution::Hd,
    Resolution::Quadvga,
    Resolution::Fwxga,
    Resolution::Sxgaplus,
    Resolution::Hdplus,
    Resolution::Uxga,
    Resolution::Wsxgaplus,
    Resolution::Fullhd,
    Resolution::Wuxga,
    Resolution::Qxga,
    Resolution::Wqhd,
    Resolution::Ultrahd,
];

// ---------------------------------------------------------------------------
// Conditional processing
// ---------------------------------------------------------------------------

/// Tests whether an option condition is satisfied.
///
/// Condition format (matching Java's JsonSkinSerializer.testOption):
/// - `901` → option 901 is enabled
/// - `-901` → option 901 is NOT enabled
/// - `[901, 911]` → 901 AND 911 enabled
/// - `[[901, 902], 911]` → (901 OR 902) AND 911
pub fn test_option(condition: &Value, enabled: &HashSet<i32>) -> bool {
    match condition {
        Value::Null => true,
        Value::Number(n) => {
            let op = n.as_i64().unwrap_or(0) as i32;
            test_option_number(op, enabled)
        }
        Value::Array(arr) => {
            for item in arr {
                let ok = match item {
                    Value::Number(n) => {
                        let op = n.as_i64().unwrap_or(0) as i32;
                        test_option_number(op, enabled)
                    }
                    Value::Array(sub) => {
                        // OR group: at least one must be enabled
                        sub.iter().any(|v| {
                            if let Value::Number(n) = v {
                                let op = n.as_i64().unwrap_or(0) as i32;
                                test_option_number(op, enabled)
                            } else {
                                false
                            }
                        })
                    }
                    _ => false,
                };
                if !ok {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

fn test_option_number(op: i32, enabled: &HashSet<i32>) -> bool {
    if op >= 0 {
        enabled.contains(&op)
    } else {
        !enabled.contains(&(-op))
    }
}

/// Pre-processes a JSON Value to resolve conditional branches.
///
/// For array elements with `{"if": condition, "value": obj}` or
/// `{"if": condition, "values": [objs]}`, evaluates the condition
/// and includes/excludes the items accordingly.
///
/// For objects with `{"include": "path"}`, loads the referenced file.
/// (File includes are NOT implemented in this phase — they return null.)
pub fn resolve_conditionals(value: Value, enabled: &HashSet<i32>) -> Value {
    resolve_conditionals_with_context(value, enabled, false)
}

fn resolve_conditionals_with_context(
    value: Value,
    enabled: &HashSet<i32>,
    in_object_field: bool,
) -> Value {
    match value {
        Value::Array(arr) => {
            // ObjectSerializer behavior in Java:
            // when an object field is encoded as a conditional branch array
            // (`[{if, value}, ...]`), only the first matched branch is used.
            if in_object_field && is_object_conditional_branch_array(&arr) {
                for item in &arr {
                    if let Value::Object(obj) = item {
                        let condition = obj.get("if").unwrap_or(&Value::Null);
                        if test_option(condition, enabled)
                            && let Some(val) = obj.get("value")
                        {
                            return resolve_conditionals_with_context(val.clone(), enabled, false);
                        }
                    }
                }
                return Value::Null;
            }

            let mut result = Vec::new();
            for item in arr {
                if let Value::Object(ref obj) = item {
                    if obj.contains_key("if")
                        && (obj.contains_key("value") || obj.contains_key("values"))
                    {
                        // Conditional branch
                        let condition = obj.get("if").unwrap_or(&Value::Null);
                        if test_option(condition, enabled) {
                            if let Some(val) = obj.get("value") {
                                result.push(resolve_conditionals_with_context(
                                    val.clone(),
                                    enabled,
                                    false,
                                ));
                            }
                            if let Some(Value::Array(vals)) = obj.get("values") {
                                for v in vals {
                                    result.push(resolve_conditionals_with_context(
                                        v.clone(),
                                        enabled,
                                        false,
                                    ));
                                }
                            }
                        }
                        continue;
                    }
                    if obj.contains_key("include") {
                        // File include — deferred to Phase 10 integration
                        continue;
                    }
                }
                result.push(resolve_conditionals_with_context(item, enabled, false));
            }
            Value::Array(result)
        }
        Value::Object(mut map) => {
            // Check if this object itself is a conditional branch
            if map.contains_key("if") && map.contains_key("value") {
                let condition = map.get("if").unwrap_or(&Value::Null);
                if test_option(condition, enabled)
                    && let Some(val) = map.remove("value")
                {
                    return resolve_conditionals_with_context(val, enabled, false);
                }
                return Value::Null;
            }
            // Recurse into object fields
            let resolved: serde_json::Map<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, resolve_conditionals_with_context(v, enabled, true)))
                .collect();
            Value::Object(resolved)
        }
        other => other,
    }
}

fn is_object_conditional_branch_array(arr: &[Value]) -> bool {
    !arr.is_empty()
        && arr.iter().all(|item| {
            let Value::Object(obj) = item else {
                return false;
            };
            obj.contains_key("if") && obj.contains_key("value") && !obj.contains_key("values")
        })
}

// ---------------------------------------------------------------------------
// Header loading
// ---------------------------------------------------------------------------

/// Pre-processes lenient JSON (as used by beatoraja skins) into strict JSON.
///
/// Handles:
/// - Missing commas between objects/arrays: `}  {` → `}, {`
/// - Trailing commas before `}` or `]`: `, }` → `}`
pub fn preprocess_json(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escape_next = false;
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        let c = chars[i];
        if escape_next {
            escape_next = false;
            result.push(c);
            continue;
        }
        if c == '\\' && in_string {
            escape_next = true;
            result.push(c);
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            result.push(c);
            continue;
        }
        if in_string {
            result.push(c);
            continue;
        }

        // Remove trailing commas: skip comma if next non-whitespace is } or ]
        if c == ',' {
            let next_nonws = chars[i + 1..].iter().find(|ch| !ch.is_ascii_whitespace());
            if matches!(next_nonws, Some('}') | Some(']')) {
                continue; // skip trailing comma
            }
        }

        // Insert missing commas: after } or ] if next non-whitespace is { or [ or " or digit/minus
        if c == '}' || c == ']' {
            result.push(c);
            let next_nonws = chars[i + 1..].iter().find(|ch| !ch.is_ascii_whitespace());
            if matches!(
                next_nonws,
                Some('{') | Some('[') | Some('"') | Some('0'..='9') | Some('-')
            ) {
                result.push(',');
            }
            continue;
        }

        result.push(c);
    }

    result
}

/// Loads only the skin header from a JSON skin file.
///
/// This is used for the skin selection UI — it reads metadata and
/// custom options without loading the full skin.
pub fn load_header(json_str: &str) -> Result<SkinHeader> {
    let preprocessed = preprocess_json(json_str);
    let data: JsonSkinData =
        serde_json::from_str(&preprocessed).context("Failed to parse JSON skin")?;
    build_header(&data, None)
}

/// Builds a SkinHeader from parsed JSON skin data.
pub fn build_header(data: &JsonSkinData, path: Option<&Path>) -> Result<SkinHeader> {
    if data.skin_type == -1 {
        anyhow::bail!("Skin type not specified (type = -1)");
    }

    let skin_type = SkinType::from_id(data.skin_type);

    // Build custom options
    let mut options = Vec::new();
    for prop in &data.property {
        let mut op_ids = Vec::new();
        let mut op_names = Vec::new();
        for item in &prop.item {
            op_ids.push(item.op);
            op_names.push(item.name.clone().unwrap_or_default());
        }
        let mut opt = CustomOption::new(prop.name.clone().unwrap_or_default(), op_ids, op_names);
        if let Some(def) = &prop.def {
            opt.default_label = Some(def.clone());
        }
        options.push(opt);
    }

    // Build custom files
    let parent_dir = path
        .and_then(|p| p.parent())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut files = Vec::new();
    for fp in &data.filepath {
        let full_path = if parent_dir.is_empty() {
            fp.path.clone().unwrap_or_default()
        } else {
            format!("{}/{}", parent_dir, fp.path.as_deref().unwrap_or(""))
        };
        files.push(CustomFile::new(
            fp.name.clone().unwrap_or_default(),
            full_path,
            fp.def.clone(),
        ));
    }

    // Build custom offsets
    let mut offsets = Vec::new();
    for off in &data.offset {
        offsets.push(CustomOffset::new(
            off.name.clone().unwrap_or_default(),
            off.id,
            off.x,
            off.y,
            off.w,
            off.h,
            off.r,
            off.a,
        ));
    }

    // Add standard play-mode offsets
    if is_play_type(skin_type) {
        offsets.push(CustomOffset::new(
            "All offset(%)".to_string(),
            OFFSET_ALL,
            true,
            true,
            true,
            true,
            false,
            false,
        ));
        offsets.push(CustomOffset::new(
            "Notes offset".to_string(),
            OFFSET_NOTES_1P,
            false,
            false,
            false,
            true,
            false,
            false,
        ));
        offsets.push(CustomOffset::new(
            "Judge offset".to_string(),
            OFFSET_JUDGE_1P,
            true,
            true,
            true,
            true,
            false,
            true,
        ));
        offsets.push(CustomOffset::new(
            "Judge Detail offset".to_string(),
            OFFSET_JUDGEDETAIL_1P,
            true,
            true,
            true,
            true,
            false,
            true,
        ));
    }

    // Build categories
    let mut categories = Vec::new();
    for cat in &data.category {
        let mut items = Vec::new();
        for cat_item_name in &cat.item {
            // Find matching option
            for (i, prop) in data.property.iter().enumerate() {
                if prop.category.as_deref() == Some(cat_item_name) {
                    items.push(CustomCategoryItem::Option(i));
                }
            }
            // Find matching file
            for (i, fp) in data.filepath.iter().enumerate() {
                if fp.category.as_deref() == Some(cat_item_name) {
                    items.push(CustomCategoryItem::File(i));
                }
            }
            // Find matching offset
            for (i, off) in data.offset.iter().enumerate() {
                if off.category.as_deref() == Some(cat_item_name) {
                    items.push(CustomCategoryItem::Offset(i));
                }
            }
        }
        categories.push(CustomCategory {
            name: cat.name.clone().unwrap_or_default(),
            items,
        });
    }

    // Detect source resolution from skin dimensions
    let source_resolution = ALL_RESOLUTIONS
        .iter()
        .find(|r| r.width() == data.w && r.height() == data.h)
        .copied();

    Ok(SkinHeader {
        format: SkinFormat::Beatoraja,
        path: path.map(|p| p.to_path_buf()),
        skin_type,
        name: data.name.clone().unwrap_or_default(),
        author: data.author.clone().unwrap_or_default(),
        options,
        files,
        offsets,
        categories,
        resolution: source_resolution.unwrap_or(Resolution::Hd),
        source_resolution,
        destination_resolution: None,
    })
}

/// Returns true if the skin type is a play screen type.
fn is_play_type(skin_type: Option<SkinType>) -> bool {
    matches!(
        skin_type,
        Some(
            SkinType::Play5Keys
                | SkinType::Play7Keys
                | SkinType::Play9Keys
                | SkinType::Play10Keys
                | SkinType::Play14Keys
                | SkinType::Play24Keys
                | SkinType::Play24KeysDouble
        )
    )
}

// ---------------------------------------------------------------------------
// Full skin loading
// ---------------------------------------------------------------------------

/// Loads a full Skin from a JSON skin string.
///
/// `enabled_options`: set of enabled option IDs (from user's skin config).
/// `dest_resolution`: the display resolution to scale to.
pub fn load_skin(
    json_str: &str,
    enabled_options: &HashSet<i32>,
    dest_resolution: Resolution,
    path: Option<&Path>,
) -> Result<Skin> {
    let data = parse_skin_data(json_str, enabled_options)?;
    let source_images = infer_existing_source_images(&data, path);
    load_skin_with_images(
        json_str,
        enabled_options,
        dest_resolution,
        path,
        &source_images,
    )
}

/// Loads a full Skin from a JSON skin string with pre-loaded source images.
///
/// `source_images` maps source ID strings (from the `source` array) to
/// `ImageHandle` values. When an image/slider/graph references a source ID,
/// the corresponding handle is used to populate `SkinImage.sources`,
/// `SkinSlider.source_images`, or `SkinGraph.source_images`.
pub fn load_skin_with_images(
    json_str: &str,
    enabled_options: &HashSet<i32>,
    dest_resolution: Resolution,
    path: Option<&Path>,
    source_images: &HashMap<String, ImageHandle>,
) -> Result<Skin> {
    let data = parse_skin_data(json_str, enabled_options)?;

    // Build header
    let mut header = build_header(&data, path)?;
    header.destination_resolution = Some(dest_resolution);

    // Create skin
    let mut skin = Skin::new(header);
    skin.fadeout = data.fadeout;
    skin.input = data.input;
    if data.scene > 0 {
        skin.scene = data.scene;
    }

    // Build options map
    for opt in &data.property {
        for item in &opt.item {
            let is_enabled = enabled_options.contains(&item.op);
            skin.options.insert(item.op, if is_enabled { 1 } else { 0 });
        }
    }

    // Process destinations → create skin objects
    for dst in &data.destination {
        if let Some(obj) = build_skin_object(&data, dst, path, source_images) {
            skin.add(obj);
        }
    }

    // Custom events
    for evt in &data.custom_events {
        let condition = evt
            .condition
            .as_ref()
            .and_then(|p| p.as_id().map(BooleanId));
        if let Some(action_ref) = &evt.action
            && let Some(action_id) = action_ref.as_id()
        {
            skin.custom_events.push(CustomEventDef::new(
                EventId(action_id),
                condition,
                evt.min_interval,
            ));
        }
    }

    // Custom timers
    for timer in &data.custom_timers {
        let timer_func = timer.timer.as_ref().and_then(|p| p.as_id().map(TimerId));
        if let Some(func) = timer_func {
            skin.custom_timers
                .push(CustomTimerDef::active(TimerId(timer.id), func));
        } else {
            skin.custom_timers
                .push(CustomTimerDef::passive(TimerId(timer.id)));
        }
    }

    // Collect state-specific configs
    let skin_type = skin.header.skin_type;
    if is_play_type(skin_type) {
        let mut config = collect_play_config(&skin.objects).unwrap_or_default();
        config.playstart = data.playstart;
        config.loadstart = data.close;
        config.loadend = data.loadend;
        config.finish_margin = data.finishmargin;
        config.judge_timer = data.judgetimer;
        skin.play_config = Some(config);
    }
    if skin_type == Some(SkinType::MusicSelect) {
        skin.select_config = collect_select_config(&skin.objects);
    }
    if skin_type == Some(SkinType::Result) {
        skin.result_config = collect_result_config(&skin.objects);
    }
    if skin_type == Some(SkinType::CourseResult) {
        skin.course_result_config = collect_course_result_config(&skin.objects);
    }

    Ok(skin)
}

fn parse_skin_data(json_str: &str, enabled_options: &HashSet<i32>) -> Result<JsonSkinData> {
    // Pre-process lenient JSON and resolve conditionals.
    let preprocessed = preprocess_json(json_str);
    let raw: Value = serde_json::from_str(&preprocessed).context("Failed to parse JSON")?;
    let resolved = resolve_conditionals(raw, enabled_options);
    serde_json::from_value(resolved).context("Failed to deserialize resolved JSON")
}

fn infer_existing_source_images(
    data: &JsonSkinData,
    skin_path: Option<&Path>,
) -> HashMap<String, ImageHandle> {
    let mut source_images = HashMap::new();
    let mut next_handle = 1u32;

    for source in &data.source {
        let id = source.id.as_str();
        if id.is_empty() || source_images.contains_key(id) {
            continue;
        }
        let Some(source_path) = source.path.as_deref() else {
            continue;
        };
        if source_path_exists(source_path, skin_path) {
            source_images.insert(id.to_string(), ImageHandle(next_handle));
            next_handle += 1;
        }
    }

    source_images
}

fn source_path_exists(source_path: &str, skin_path: Option<&Path>) -> bool {
    let source = Path::new(source_path);
    let has_wildcard =
        source_path.contains('*') || source_path.contains('?') || source_path.contains('[');

    if has_wildcard {
        // Keep parity with Java JSON/Lua loaders:
        // directory-segment wildcards like ".../*/main.png" are not resolved
        // during source loading and therefore remain missing.
        let segments: Vec<&str> = source_path.split('/').collect();
        if segments.len() > 1
            && segments[..segments.len() - 1]
                .iter()
                .any(|seg| seg.contains('*') || seg.contains('?') || seg.contains('['))
        {
            return false;
        }

        let pattern = if source.is_absolute() {
            source.to_path_buf()
        } else if let Some(base_dir) = skin_path.and_then(Path::parent) {
            base_dir.join(source)
        } else {
            source.to_path_buf()
        };

        let Ok(paths) = glob::glob(pattern.to_string_lossy().as_ref()) else {
            return false;
        };
        return paths.filter_map(Result::ok).any(|p| p.exists());
    }

    if source.is_absolute() {
        return source.exists();
    }

    if let Some(base_dir) = skin_path.and_then(Path::parent) {
        let joined = base_dir.join(source);
        if joined.exists() {
            return true;
        }
    }

    source.exists()
}

// ---------------------------------------------------------------------------
// State-specific config collection
// ---------------------------------------------------------------------------

/// Scans skin objects and extracts play-specific objects into PlaySkinConfig.
fn collect_play_config(objects: &[SkinObjectType]) -> Option<crate::play_skin::PlaySkinConfig> {
    let mut config = crate::play_skin::PlaySkinConfig::default();
    let mut found = false;

    for obj in objects {
        match obj {
            SkinObjectType::Note(n) => {
                config.note = Some(n.clone());
                found = true;
            }
            SkinObjectType::Judge(j) => {
                config.judges.push(*j.clone());
                found = true;
            }
            SkinObjectType::Bga(b) => {
                config.bga = Some(b.clone());
                found = true;
            }
            SkinObjectType::Hidden(h) => {
                config.hidden_cover = Some(h.clone());
                found = true;
            }
            SkinObjectType::LiftCover(l) => {
                config.lift_cover = Some(l.clone());
                found = true;
            }
            _ => {}
        }
    }

    if found { Some(config) } else { None }
}

/// Scans skin objects and extracts select-specific objects into MusicSelectSkinConfig.
fn collect_select_config(
    objects: &[SkinObjectType],
) -> Option<crate::music_select_skin::MusicSelectSkinConfig> {
    let mut config = crate::music_select_skin::MusicSelectSkinConfig::default();
    let mut found = false;

    for obj in objects {
        match obj {
            SkinObjectType::Bar(bar) => {
                config.bar = Some(bar.clone());
                found = true;
            }
            SkinObjectType::DistributionGraph(g) => {
                config.distribution_graph = Some(g.clone());
                found = true;
            }
            _ => {}
        }
    }

    if found { Some(config) } else { None }
}

/// Scans skin objects and extracts result-specific objects into ResultSkinConfig.
fn collect_result_config(
    objects: &[SkinObjectType],
) -> Option<crate::result_skin::ResultSkinConfig> {
    let mut config = crate::result_skin::ResultSkinConfig::default();
    let mut found = false;

    for obj in objects {
        match obj {
            SkinObjectType::GaugeGraph(g) => {
                config.gauge_graph = Some(g.clone());
                found = true;
            }
            SkinObjectType::NoteDistributionGraph(g) => {
                config.note_graph = Some(g.clone());
                found = true;
            }
            SkinObjectType::BpmGraph(g) => {
                config.bpm_graph = Some(g.clone());
                found = true;
            }
            SkinObjectType::TimingDistributionGraph(g) => {
                config.timing_graph = Some(g.clone());
                found = true;
            }
            _ => {}
        }
    }

    if found { Some(config) } else { None }
}

/// Scans skin objects and extracts course-result-specific objects.
fn collect_course_result_config(
    objects: &[SkinObjectType],
) -> Option<crate::result_skin::CourseResultSkinConfig> {
    let mut config = crate::result_skin::CourseResultSkinConfig::default();
    let mut found = false;

    for obj in objects {
        match obj {
            SkinObjectType::GaugeGraph(g) => {
                config.gauge_graph = Some(g.clone());
                found = true;
            }
            SkinObjectType::NoteDistributionGraph(g) => {
                config.note_graph = Some(g.clone());
                found = true;
            }
            _ => {}
        }
    }

    if found { Some(config) } else { None }
}

// ---------------------------------------------------------------------------
// Helper functions for sub-object resolution
// ---------------------------------------------------------------------------

/// Resolves an image FlexId to an ImageHandle integer.
///
/// Searches `data.image` for a matching ID, then looks up its `src` in
/// `source_images` to get the handle value.
fn resolve_image_ref(
    data: &JsonSkinData,
    flex_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<i32> {
    let img_def = data.image.iter().find(|i| i.id == *flex_id)?;
    let handle = source_images.get(img_def.src.as_str())?;
    Some(handle.0 as i32)
}

/// Resolves a sub-destination reference to a SkinImage.
///
/// Finds the image definition matching the sub-destination ID, builds a
/// SkinImage with its source handle, and applies the sub-destination.
fn resolve_sub_image(
    data: &JsonSkinData,
    sub_dst: &JsonDestination,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<crate::skin_image::SkinImage> {
    let img_def = data.image.iter().find(|i| i.id == sub_dst.id)?;
    let handle = source_images.get(img_def.src.as_str())?;
    let timer = img_def.timer.as_ref().and_then(|t| t.as_id());
    let mut img = crate::skin_image::SkinImage::from_frames(vec![*handle], timer, img_def.cycle);
    apply_destination(&mut img.base, sub_dst);
    Some(img)
}

/// Resolves a sub-destination reference to a SkinNumber.
///
/// Finds the value definition matching the sub-destination ID, builds a
/// SkinNumber with the same logic as try_build_number(), and applies the
/// sub-destination.
fn resolve_sub_number(
    data: &JsonSkinData,
    sub_dst: &JsonDestination,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<crate::skin_number::SkinNumber> {
    let val_def = data.value.iter().find(|v| v.id == sub_dst.id)?;
    let ref_id = if let Some(ref val) = val_def.value {
        val.as_id().unwrap_or(val_def.ref_id)
    } else {
        val_def.ref_id
    };

    let timer = val_def.timer.as_ref().and_then(|t| t.as_id());

    // Resolve source image and split into grid
    let grid = source_images
        .get(val_def.src.as_str())
        .map(|&handle| {
            split_grid(
                handle,
                val_def.x,
                val_def.y,
                val_def.w,
                val_def.h,
                val_def.divx,
                val_def.divy,
            )
        })
        .unwrap_or_default();

    let (digit_sources, has_minus, zeropadding_override) =
        build_number_source_set(&grid, timer, val_def.cycle);

    let zeropadding = zeropadding_override.unwrap_or(val_def.zeropadding);

    let mut num = crate::skin_number::SkinNumber {
        base: SkinObjectBase::default(),
        ref_id: Some(crate::property_id::IntegerId(ref_id)),
        keta: val_def.digit,
        zero_padding: crate::skin_number::ZeroPadding::from_i32(zeropadding),
        align: crate::skin_number::NumberAlign::from_i32(val_def.align),
        space: val_def.space,
        digit_sources,
        has_minus_images: has_minus,
        ..Default::default()
    };
    apply_destination(&mut num.base, sub_dst);
    if let Some(offsets) = &val_def.offset {
        num.digit_offsets = offsets
            .iter()
            .map(|o| crate::skin_object::SkinOffset {
                x: o.x as f32,
                y: o.y as f32,
                w: o.w as f32,
                h: o.h as f32,
                ..Default::default()
            })
            .collect();
    }
    Some(num)
}

/// Resolves a sub-destination reference to a SkinText.
///
/// Finds the text definition matching the sub-destination ID, builds a
/// SkinText with the same logic as try_build_text(), and applies the
/// sub-destination.
fn resolve_sub_text(
    data: &JsonSkinData,
    sub_dst: &JsonDestination,
    skin_path: Option<&Path>,
) -> Option<crate::skin_text::SkinText> {
    let text_def = data.text.iter().find(|t| t.id == sub_dst.id)?;
    let ref_id = if let Some(ref val) = text_def.value {
        val.as_id().unwrap_or(text_def.ref_id)
    } else {
        text_def.ref_id
    };
    let outline_color = if text_def.outline_width > 0.0 {
        Some(parse_color(&text_def.outline_color))
    } else {
        None
    };
    let shadow = if text_def.shadow_offset_x != 0.0 || text_def.shadow_offset_y != 0.0 {
        Some(crate::skin_text::TextShadow {
            color: parse_color(&text_def.shadow_color),
            offset_x: text_def.shadow_offset_x,
            offset_y: text_def.shadow_offset_y,
            smoothness: text_def.shadow_smoothness,
        })
    } else {
        None
    };
    let font_type = resolve_font_type(data, &text_def.font, skin_path);
    let mut text = crate::skin_text::SkinText {
        base: SkinObjectBase::default(),
        ref_id: Some(crate::property_id::StringId(ref_id)),
        constant_text: text_def.constant_text.clone(),
        font_size: text_def.size as f32,
        align: crate::skin_text::TextAlign::from_i32(text_def.align),
        wrapping: text_def.wrapping,
        overflow: crate::skin_text::TextOverflow::from_i32(text_def.overflow),
        outline_color,
        outline_width: text_def.outline_width,
        shadow,
        font_type,
        ..Default::default()
    };
    apply_destination(&mut text.base, sub_dst);
    Some(text)
}

/// Resolves a bar image from imageset or image.
///
/// First searches `data.imageset` for the sub-destination ID. If found,
/// resolves all images in the set. Falls back to `resolve_sub_image()`.
fn resolve_bar_image(
    data: &JsonSkinData,
    sub_dst: &JsonDestination,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<crate::skin_image::SkinImage> {
    // Try imageset first
    if let Some(set_def) = data.imageset.iter().find(|s| s.id == sub_dst.id) {
        let ref_id = if let Some(ref val) = set_def.value {
            val.as_id().unwrap_or(set_def.ref_id)
        } else {
            set_def.ref_id
        };
        let mut sources = Vec::new();
        for image_ref in &set_def.images {
            let Some(image_def) = data.image.iter().find(|img| img.id == *image_ref) else {
                continue;
            };
            let Some(&handle) = source_images.get(image_def.src.as_str()) else {
                continue;
            };
            let timer = image_def.timer.as_ref().and_then(|t| t.as_id());
            sources.push(crate::skin_image::SkinImageSource::Frames {
                images: vec![handle],
                timer,
                cycle: image_def.cycle,
            });
        }
        if sources.is_empty() {
            return None;
        }
        let mut img = if ref_id != 0 {
            crate::skin_image::SkinImage::with_ref(sources, crate::property_id::IntegerId(ref_id))
        } else {
            crate::skin_image::SkinImage {
                sources,
                ..Default::default()
            }
        };
        apply_destination(&mut img.base, sub_dst);
        return Some(img);
    }
    // Fall back to single image
    resolve_sub_image(data, sub_dst, source_images)
}

// ---------------------------------------------------------------------------
// Number / Float source set building
// ---------------------------------------------------------------------------

use crate::image_handle::ImageRegion;
use crate::skin_source::{SkinSourceSet, build_number_source_set, split_grid};

/// Builds a `SkinSourceSet` for a SkinFloat from grid images.
///
/// Matches Java `JsonSkinObjectLoader.java:175-382`:
/// - %26: 13 positive + 13 negative (signed, separate images)
/// - %24: 12 positive + 12 negative (unsigned, separate images)
/// - %22: 10 digits + back-zero sharing + decimal point (unsigned, separate +/-)
/// - %12: 12 per state (shared positive/negative)
/// - %11: 10 digits + back-zero sharing + decimal point (shared)
/// - fallback: treat as 12 per state
///
/// Returns `(source_set, has_minus)`.
fn build_float_source_set(
    images: &[ImageRegion],
    timer: Option<i32>,
    cycle: i32,
    divx: i32,
    divy: i32,
) -> (SkinSourceSet, bool) {
    let len = images.len();
    if len == 0 {
        return (SkinSourceSet::new(vec![], timer, cycle), false);
    }

    if len.is_multiple_of(26) {
        // 13 positive + 13 negative per state
        let states = len / 26;
        let mut positive = Vec::with_capacity(states);
        for j in 0..states {
            let row: Vec<ImageRegion> = (0..13).map(|i| images[j * 26 + i]).collect();
            positive.push(row);
        }
        (SkinSourceSet::new(positive, timer, cycle), true)
    } else if len.is_multiple_of(24) {
        // 12 positive + 12 negative per state
        let states = len / 24;
        let mut positive = Vec::with_capacity(states);
        for j in 0..states {
            let row: Vec<ImageRegion> = (0..12).map(|i| images[j * 24 + i]).collect();
            positive.push(row);
        }
        (SkinSourceSet::new(positive, timer, cycle), true)
    } else if len.is_multiple_of(22) {
        // 10 digits + back-zero sharing + decimal point, separate +/- images
        let states = len / 22;
        let mut positive = Vec::with_capacity(states);
        for j in 0..states {
            let mut row = Vec::with_capacity(12);
            for i in 0..10 {
                row.push(images[j * 22 + i]);
            }
            row.push(images[j * 22]); // index 10: back-zero shared with digit 0
            row.push(images[j * 22 + 10]); // index 11: decimal point
            positive.push(row);
        }
        (SkinSourceSet::new(positive, timer, cycle), true)
    } else if len.is_multiple_of(12) {
        // 12 per state, shared positive/negative
        let states = len / 12;
        let mut rows = Vec::with_capacity(states);
        for j in 0..states {
            let row: Vec<ImageRegion> = (0..12).map(|i| images[j * 12 + i]).collect();
            rows.push(row);
        }
        (SkinSourceSet::new(rows, timer, cycle), false)
    } else if len.is_multiple_of(11) {
        // 10 digits + back-zero sharing + decimal point, shared
        let states = len / 11;
        let mut rows = Vec::with_capacity(states);
        for j in 0..states {
            let mut row = Vec::with_capacity(12);
            for i in 0..10 {
                row.push(images[j * 11 + i]);
            }
            row.push(images[j * 11]); // index 10: back-zero shared with digit 0
            row.push(images[j * 11 + 10]); // index 11: decimal point
            rows.push(row);
        }
        (SkinSourceSet::new(rows, timer, cycle), false)
    } else {
        // Fallback: treat as 12 per state
        let d = 12;
        let total = (divx * divy) as usize;
        let states = total / d;
        if states == 0 {
            return (SkinSourceSet::new(vec![], timer, cycle), false);
        }
        let mut rows = Vec::with_capacity(states);
        for j in 0..states {
            let row: Vec<ImageRegion> = (0..d)
                .map(|i| {
                    let idx = j * d + i;
                    if idx < images.len() {
                        images[idx]
                    } else {
                        ImageRegion::default()
                    }
                })
                .collect();
            rows.push(row);
        }
        (SkinSourceSet::new(rows, timer, cycle), false)
    }
}

// ---------------------------------------------------------------------------
// Object building
// ---------------------------------------------------------------------------

/// Builds a SkinObjectType from a JSON destination and the skin data.
///
/// Matches destination IDs against image/value/text/slider/graph definitions.
/// Returns None if no matching object definition is found.
fn build_skin_object(
    data: &JsonSkinData,
    dst: &JsonDestination,
    skin_path: Option<&Path>,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<SkinObjectType> {
    let dst_id = &dst.id;

    // Check for negative numeric ID → reference image
    if let Ok(id) = dst_id.as_str().parse::<i32>()
        && id < 0
    {
        let mut img = crate::skin_image::SkinImage::from_reference(-id);
        apply_destination(&mut img.base, dst);
        return Some(img.into());
    }

    // Skin-type specific objects must be resolved before plain images.
    if let Some(obj) = try_build_song_list(data, dst, dst_id, source_images, skin_path) {
        return Some(obj);
    }
    if let Some(obj) = try_build_note(data, dst, dst_id, source_images) {
        return Some(obj);
    }
    if let Some(obj) = try_build_judge(data, dst, dst_id, source_images) {
        return Some(obj);
    }
    if let Some(obj) = try_build_gauge(data, dst, dst_id) {
        return Some(obj);
    }
    if let Some(obj) = try_build_bga(data, dst, dst_id) {
        return Some(obj);
    }
    if let Some(obj) = try_build_hidden_cover(data, dst, dst_id) {
        return Some(obj);
    }
    if let Some(obj) = try_build_lift_cover(data, dst, dst_id) {
        return Some(obj);
    }
    if let Some(obj) = try_build_gauge_graph(data, dst, dst_id) {
        return Some(obj);
    }
    if let Some(obj) = try_build_judge_graph(data, dst, dst_id) {
        return Some(obj);
    }
    if let Some(obj) = try_build_float(data, dst, dst_id, source_images) {
        return Some(obj);
    }

    // Try matching against each object type
    if let Some(obj) = try_build_image(data, dst, dst_id, source_images) {
        return Some(obj);
    }
    if let Some(obj) = try_build_image_set(data, dst, dst_id, source_images) {
        return Some(obj);
    }
    if let Some(obj) = try_build_number(data, dst, dst_id, source_images) {
        return Some(obj);
    }
    if let Some(obj) = try_build_text(data, dst, dst_id, skin_path) {
        return Some(obj);
    }
    if let Some(obj) = try_build_slider(data, dst, dst_id, source_images) {
        return Some(obj);
    }
    if let Some(obj) = try_build_graph(data, dst, dst_id, source_images) {
        return Some(obj);
    }
    if let Some(obj) = try_build_bpm_graph(data, dst, dst_id) {
        return Some(obj);
    }
    if let Some(obj) = try_build_hit_error_visualizer(data, dst, dst_id) {
        return Some(obj);
    }
    if let Some(obj) = try_build_timing_visualizer(data, dst, dst_id) {
        return Some(obj);
    }
    if let Some(obj) = try_build_timing_distribution(data, dst, dst_id) {
        return Some(obj);
    }

    None
}

fn try_build_song_list(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
    skin_path: Option<&Path>,
) -> Option<SkinObjectType> {
    use crate::skin_bar::*;

    let song_list = data.songlist.as_ref()?;
    if song_list.id != *dst_id {
        return None;
    }

    let mut bar = SkinBar {
        position: song_list.center,
        ..Default::default()
    };
    apply_destination(&mut bar.base, dst);

    // Bar images (on/off) via imageset or image
    for (i, on_dst) in song_list.liston.iter().enumerate() {
        if i >= BAR_COUNT {
            break;
        }
        bar.bar_image_on[i] = resolve_bar_image(data, on_dst, source_images);
    }
    for (i, off_dst) in song_list.listoff.iter().enumerate() {
        if i >= BAR_COUNT {
            break;
        }
        bar.bar_image_off[i] = resolve_bar_image(data, off_dst, source_images);
    }

    // Lamps
    for (i, lamp_dst) in song_list.lamp.iter().enumerate() {
        if i >= BAR_LAMP_COUNT {
            break;
        }
        bar.lamp[i] = resolve_sub_image(data, lamp_dst, source_images);
    }
    for (i, lamp_dst) in song_list.playerlamp.iter().enumerate() {
        if i >= BAR_LAMP_COUNT {
            break;
        }
        bar.my_lamp[i] = resolve_sub_image(data, lamp_dst, source_images);
    }
    for (i, lamp_dst) in song_list.rivallamp.iter().enumerate() {
        if i >= BAR_LAMP_COUNT {
            break;
        }
        bar.rival_lamp[i] = resolve_sub_image(data, lamp_dst, source_images);
    }

    // Trophies
    for (i, trophy_dst) in song_list.trophy.iter().enumerate() {
        if i >= BAR_TROPHY_COUNT {
            break;
        }
        bar.trophy[i] = resolve_sub_image(data, trophy_dst, source_images);
    }

    // Labels
    for (i, label_dst) in song_list.label.iter().enumerate() {
        if i >= BAR_LABEL_COUNT {
            break;
        }
        bar.label[i] = resolve_sub_image(data, label_dst, source_images);
    }

    // Texts
    for (i, text_dst) in song_list.text.iter().enumerate() {
        if i >= BAR_TEXT_COUNT {
            break;
        }
        bar.text[i] = resolve_sub_text(data, text_dst, skin_path);
    }

    // Levels (number displays)
    for (i, level_dst) in song_list.level.iter().enumerate() {
        if i >= BAR_LEVEL_COUNT {
            break;
        }
        bar.bar_level[i] = resolve_sub_number(data, level_dst, source_images);
    }

    Some(bar.into())
}

fn try_build_note(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<SkinObjectType> {
    use crate::skin_note::*;

    let note = data.note.as_ref()?;
    if note.id != *dst_id {
        return None;
    }

    let lane_count = note.dst.len();
    let mut skin_note = SkinNote::default();
    apply_destination(&mut skin_note.base, dst);

    // Build per-lane configurations
    for i in 0..lane_count {
        let mut lane = SkinLane::default();

        // Normal note
        if let Some(id) = note.note.get(i) {
            lane.note = resolve_image_ref(data, id, source_images);
        }

        // LN textures
        if let Some(id) = note.lnend.get(i) {
            lane.longnote[LN_END] = resolve_image_ref(data, id, source_images);
        }
        if let Some(id) = note.lnstart.get(i) {
            lane.longnote[LN_START] = resolve_image_ref(data, id, source_images);
        }
        // LN body: if lnbody_active is present, it's the active body
        if !note.lnbody_active.is_empty() {
            if let Some(id) = note.lnbody_active.get(i) {
                lane.longnote[LN_BODY_ACTIVE] = resolve_image_ref(data, id, source_images);
            }
            if let Some(id) = note.lnbody.get(i) {
                lane.longnote[LN_BODY_INACTIVE] = resolve_image_ref(data, id, source_images);
            }
        } else {
            if let Some(id) = note.lnbody.get(i) {
                lane.longnote[LN_BODY_ACTIVE] = resolve_image_ref(data, id, source_images);
            }
            if let Some(id) = note.lnactive.get(i) {
                lane.longnote[LN_BODY_INACTIVE] = resolve_image_ref(data, id, source_images);
            }
        }

        // HCN textures
        if let Some(id) = note.hcnend.get(i) {
            lane.longnote[HCN_END] = resolve_image_ref(data, id, source_images);
        }
        if let Some(id) = note.hcnstart.get(i) {
            lane.longnote[HCN_START] = resolve_image_ref(data, id, source_images);
        }
        if !note.hcnbody_active.is_empty() {
            if let Some(id) = note.hcnbody_active.get(i) {
                lane.longnote[HCN_BODY_ACTIVE] = resolve_image_ref(data, id, source_images);
            }
            if let Some(id) = note.hcnbody.get(i) {
                lane.longnote[HCN_BODY_INACTIVE] = resolve_image_ref(data, id, source_images);
            }
        } else {
            if let Some(id) = note.hcnbody.get(i) {
                lane.longnote[HCN_BODY_ACTIVE] = resolve_image_ref(data, id, source_images);
            }
            if let Some(id) = note.hcnactive.get(i) {
                lane.longnote[HCN_BODY_INACTIVE] = resolve_image_ref(data, id, source_images);
            }
        }
        // HCN reactive / damage
        if !note.hcnbody_reactive.is_empty() {
            if let Some(id) = note.hcnbody_reactive.get(i) {
                lane.longnote[HCN_BODY_REACTIVE] = resolve_image_ref(data, id, source_images);
            }
        } else if let Some(id) = note.hcnreactive.get(i) {
            lane.longnote[HCN_BODY_REACTIVE] = resolve_image_ref(data, id, source_images);
        }
        if !note.hcnbody_miss.is_empty() {
            if let Some(id) = note.hcnbody_miss.get(i) {
                lane.longnote[HCN_BODY_DAMAGE] = resolve_image_ref(data, id, source_images);
            }
        } else if let Some(id) = note.hcndamage.get(i) {
            lane.longnote[HCN_BODY_DAMAGE] = resolve_image_ref(data, id, source_images);
        }

        // Mine, hidden, processed
        if let Some(id) = note.mine.get(i) {
            lane.mine_note = resolve_image_ref(data, id, source_images);
        }
        if let Some(id) = note.hidden.get(i) {
            lane.hidden_note = resolve_image_ref(data, id, source_images);
        }
        if let Some(id) = note.processed.get(i) {
            lane.processed_note = resolve_image_ref(data, id, source_images);
        }

        // Scale
        if let Some(&s) = note.size.get(i) {
            lane.scale = s;
        }

        // Secondary destination offset
        if let Some(d2) = note.dst2 {
            lane.dst_note2 = d2;
        }

        skin_note.lanes.push(lane);
    }

    // Line images from group/bpm/stop/time
    if let Some(group) = note.group.first() {
        skin_note.line_image = resolve_image_ref(data, &group.id, source_images);
    }
    if let Some(bpm) = note.bpm.first() {
        skin_note.bpm_line_image = resolve_image_ref(data, &bpm.id, source_images);
    }
    if let Some(stop) = note.stop.first() {
        skin_note.stop_line_image = resolve_image_ref(data, &stop.id, source_images);
    }
    if let Some(time) = note.time.first() {
        skin_note.time_line_image = resolve_image_ref(data, &time.id, source_images);
    }

    if skin_note.base.destinations.is_empty() {
        skin_note.base.add_destination(Destination {
            time: 0,
            region: Rect::new(0.0, 0.0, 0.0, 0.0),
            color: Color::white(),
            angle: 0,
            acc: 0,
        });
    }
    Some(skin_note.into())
}

fn try_build_judge(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<SkinObjectType> {
    use crate::skin_judge::JUDGE_COUNT;

    let judge_def = data.judge.iter().find(|j| j.id == *dst_id)?;

    let mut judge = crate::skin_judge::SkinJudge {
        player: judge_def.index,
        shift: judge_def.shift,
        ..Default::default()
    };
    apply_destination(&mut judge.base, dst);

    // Populate judge images (up to JUDGE_COUNT=7)
    for (i, img_dst) in judge_def.images.iter().enumerate() {
        if i >= JUDGE_COUNT {
            break;
        }
        judge.judge_images[i] = resolve_sub_image(data, img_dst, source_images);
    }

    // Populate judge combo numbers (up to JUDGE_COUNT=7)
    for (i, num_dst) in judge_def.numbers.iter().enumerate() {
        if i >= JUDGE_COUNT {
            break;
        }
        if let Some(mut num) = resolve_sub_number(data, num_dst, source_images) {
            num.relative = true;
            judge.judge_counts[i] = Some(num);
        }
    }

    Some(judge.into())
}

fn try_build_gauge(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let gauge = data.gauge.as_ref()?;
    if gauge.id != *dst_id {
        return None;
    }

    let mut skin_gauge = crate::skin_gauge::SkinGauge::new(gauge.parts);
    apply_destination(&mut skin_gauge.base, dst);
    Some(skin_gauge.into())
}

fn try_build_bga(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let bga = data.bga.as_ref()?;
    if bga.id != *dst_id {
        return None;
    }

    let mut skin_bga = crate::skin_bga::SkinBga::default();
    apply_destination(&mut skin_bga.base, dst);
    Some(skin_bga.into())
}

fn try_build_hidden_cover(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let hidden = data.hidden_cover.iter().find(|h| h.id == *dst_id)?;

    let mut skin_hidden = crate::skin_hidden::SkinHidden {
        disapear_line: hidden.disapear_line as f32,
        link_lift: hidden.is_disapear_line_link_lift,
        timer: hidden.timer.as_ref().and_then(|t| t.as_id()),
        cycle: hidden.cycle,
        ..Default::default()
    };
    apply_destination(&mut skin_hidden.base, dst);
    Some(skin_hidden.into())
}

fn try_build_lift_cover(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let lift = data.lift_cover.iter().find(|l| l.id == *dst_id)?;

    let mut skin_lift = crate::skin_hidden::SkinLiftCover {
        disapear_line: lift.disapear_line as f32,
        link_lift: lift.is_disapear_line_link_lift,
        timer: lift.timer.as_ref().and_then(|t| t.as_id()),
        cycle: lift.cycle,
        ..Default::default()
    };
    apply_destination(&mut skin_lift.base, dst);
    Some(skin_lift.into())
}

fn try_build_gauge_graph(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let _gauge_graph = data.gaugegraph.iter().find(|g| g.id == *dst_id)?;
    let mut graph = crate::skin_distribution_graph::SkinDistributionGraph::default();
    apply_destination(&mut graph.base, dst);
    Some(graph.into())
}

fn try_build_judge_graph(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let graph_def = data.judgegraph.iter().find(|g| g.id == *dst_id)?;
    let mut graph = crate::skin_visualizer::SkinNoteDistributionGraph::new(
        graph_def.graph_type,
        graph_def.delay,
    );
    graph.back_tex_off = graph_def.back_tex_off != 0;
    graph.order_reverse = graph_def.order_reverse != 0;
    graph.no_gap = graph_def.no_gap != 0;
    graph.no_gap_x = graph_def.no_gap_x != 0;
    apply_destination(&mut graph.base, dst);
    Some(graph.into())
}

fn try_build_float(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<SkinObjectType> {
    let float_def = data.floatvalue.iter().find(|f| f.id == *dst_id)?;
    let ref_id = if let Some(value) = &float_def.value {
        value.as_id().unwrap_or(float_def.ref_id)
    } else {
        float_def.ref_id
    };

    let timer = float_def.timer.as_ref().and_then(|t| t.as_id());

    // Resolve source image and split into grid
    let grid = source_images
        .get(float_def.src.as_str())
        .map(|&handle| {
            split_grid(
                handle,
                float_def.x,
                float_def.y,
                float_def.w,
                float_def.h,
                float_def.divx,
                float_def.divy,
            )
        })
        .unwrap_or_default();

    let (digit_sources, _has_minus) = build_float_source_set(
        &grid,
        timer,
        float_def.cycle,
        float_def.divx,
        float_def.divy,
    );

    let mut float_obj = crate::skin_float::SkinFloat {
        ref_id: Some(crate::property_id::FloatId(ref_id)),
        iketa: float_def.iketa,
        fketa: float_def.fketa,
        sign_visible: float_def.is_sign_visible,
        gain: float_def.gain,
        zero_padding: float_def.zeropadding,
        align: float_def.align,
        digit_sources,
        ..Default::default()
    };
    apply_destination(&mut float_obj.base, dst);
    Some(float_obj.into())
}

fn try_build_image(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<SkinObjectType> {
    let img_def = data.image.iter().find(|i| i.id == *dst_id)?;

    let mut skin_img = crate::skin_image::SkinImage::default();
    apply_destination(&mut skin_img.base, dst);

    // Resolve source image handle
    if let Some(&handle) = source_images.get(img_def.src.as_str()) {
        let timer = img_def.timer.as_ref().and_then(|t| t.as_id());
        skin_img.sources = vec![crate::skin_image::SkinImageSource::Frames {
            images: vec![handle],
            timer,
            cycle: img_def.cycle,
        }];
    }

    // Record click event
    if let Some(act) = &img_def.act
        && let Some(id) = act.as_id()
    {
        skin_img.base.click_event = Some(EventId(id));
        skin_img.base.click_event_type = img_def.click;
    }

    Some(skin_img.into())
}

fn try_build_image_set(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<SkinObjectType> {
    let set_def = data.imageset.iter().find(|i| i.id == *dst_id)?;

    let mut skin_img = crate::skin_image::SkinImage::default();
    apply_destination(&mut skin_img.base, dst);

    // Set the ref selector
    let ref_id = if let Some(ref val) = set_def.value {
        val.as_id().unwrap_or(set_def.ref_id)
    } else {
        set_def.ref_id
    };
    if ref_id != 0 {
        skin_img.ref_id = Some(crate::property_id::IntegerId(ref_id));
    }

    // Resolve sources from referenced image IDs.
    for image_ref in &set_def.images {
        let Some(image_def) = data.image.iter().find(|img| img.id == *image_ref) else {
            continue;
        };
        let Some(&handle) = source_images.get(image_def.src.as_str()) else {
            continue;
        };
        let timer = image_def.timer.as_ref().and_then(|t| t.as_id());
        skin_img
            .sources
            .push(crate::skin_image::SkinImageSource::Frames {
                images: vec![handle],
                timer,
                cycle: image_def.cycle,
            });
    }

    if let Some(act) = &set_def.act
        && let Some(id) = act.as_id()
    {
        skin_img.base.click_event = Some(EventId(id));
        skin_img.base.click_event_type = set_def.click;
    }

    Some(skin_img.into())
}

fn try_build_number(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<SkinObjectType> {
    let val_def = data.value.iter().find(|v| v.id == *dst_id)?;

    let ref_id = if let Some(ref val) = val_def.value {
        val.as_id().unwrap_or(val_def.ref_id)
    } else {
        val_def.ref_id
    };

    let timer = val_def.timer.as_ref().and_then(|t| t.as_id());

    // Resolve source image and split into grid
    let grid = source_images
        .get(val_def.src.as_str())
        .map(|&handle| {
            split_grid(
                handle,
                val_def.x,
                val_def.y,
                val_def.w,
                val_def.h,
                val_def.divx,
                val_def.divy,
            )
        })
        .unwrap_or_default();

    let (digit_sources, has_minus, zeropadding_override) =
        build_number_source_set(&grid, timer, val_def.cycle);

    let zeropadding = zeropadding_override.unwrap_or(val_def.zeropadding);

    let mut num = crate::skin_number::SkinNumber {
        base: SkinObjectBase::default(),
        ref_id: Some(crate::property_id::IntegerId(ref_id)),
        keta: val_def.digit,
        zero_padding: crate::skin_number::ZeroPadding::from_i32(zeropadding),
        align: crate::skin_number::NumberAlign::from_i32(val_def.align),
        space: val_def.space,
        digit_sources,
        has_minus_images: has_minus,
        ..Default::default()
    };
    apply_destination(&mut num.base, dst);

    // Record per-digit offsets
    if let Some(offsets) = &val_def.offset {
        num.digit_offsets = offsets
            .iter()
            .map(|o| crate::skin_object::SkinOffset {
                x: o.x as f32,
                y: o.y as f32,
                w: o.w as f32,
                h: o.h as f32,
                ..Default::default()
            })
            .collect();
    }

    Some(num.into())
}

fn try_build_text(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    skin_path: Option<&Path>,
) -> Option<SkinObjectType> {
    let text_def = data.text.iter().find(|t| t.id == *dst_id)?;

    let ref_id = if let Some(ref val) = text_def.value {
        val.as_id().unwrap_or(text_def.ref_id)
    } else {
        text_def.ref_id
    };

    let outline_color = if text_def.outline_width > 0.0 {
        Some(parse_color(&text_def.outline_color))
    } else {
        None
    };

    let shadow = if text_def.shadow_offset_x != 0.0 || text_def.shadow_offset_y != 0.0 {
        Some(TextShadow {
            color: parse_color(&text_def.shadow_color),
            offset_x: text_def.shadow_offset_x,
            offset_y: text_def.shadow_offset_y,
            smoothness: text_def.shadow_smoothness,
        })
    } else {
        None
    };

    // Resolve font type from font ID
    let font_type = resolve_font_type(data, &text_def.font, skin_path);

    let mut text = crate::skin_text::SkinText {
        base: SkinObjectBase::default(),
        ref_id: Some(crate::property_id::StringId(ref_id)),
        constant_text: text_def.constant_text.clone(),
        font_size: text_def.size as f32,
        align: crate::skin_text::TextAlign::from_i32(text_def.align),
        wrapping: text_def.wrapping,
        overflow: crate::skin_text::TextOverflow::from_i32(text_def.overflow),
        outline_color,
        outline_width: text_def.outline_width,
        shadow,
        font_type,
        ..Default::default()
    };
    apply_destination(&mut text.base, dst);

    Some(text.into())
}

/// Resolves a font ID reference to a FontType.
///
/// Looks up the font definition in the skin data, then determines the type
/// based on the file extension and font_type field.
fn resolve_font_type(data: &JsonSkinData, font_id: &FlexId, skin_path: Option<&Path>) -> FontType {
    let font_def = match data.font.iter().find(|f| f.id == *font_id) {
        Some(f) => f,
        None => return FontType::Default,
    };

    let raw_path = match &font_def.path {
        Some(p) if !p.is_empty() => p.clone(),
        _ => return FontType::Default,
    };

    // Resolve relative path against skin directory
    let full_path = if let Some(sp) = skin_path
        && let Some(parent) = sp.parent()
    {
        let candidate = parent.join(&raw_path);
        candidate.to_string_lossy().to_string()
    } else {
        raw_path.clone()
    };

    // Determine font type by extension
    let lower = raw_path.to_lowercase();
    if lower.ends_with(".fnt") {
        FontType::Bitmap {
            path: full_path,
            bitmap_type: font_def.font_type,
        }
    } else {
        FontType::Ttf(full_path)
    }
}

fn try_build_slider(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<SkinObjectType> {
    let sl_def = data.slider.iter().find(|s| s.id == *dst_id)?;

    let value_id = if let Some(ref val) = sl_def.value {
        val.as_id().unwrap_or(sl_def.slider_type)
    } else {
        sl_def.slider_type
    };

    let mut slider = crate::skin_slider::SkinSlider {
        base: SkinObjectBase::default(),
        direction: crate::skin_slider::SliderDirection::from_i32(sl_def.angle),
        range: sl_def.range,
        ref_id: Some(crate::property_id::FloatId(value_id)),
        changeable: sl_def.changeable,
        ..Default::default()
    };
    apply_destination(&mut slider.base, dst);

    // Resolve source image handle
    if let Some(&handle) = source_images.get(sl_def.src.as_str()) {
        slider.source_images = vec![handle];
    }

    Some(slider.into())
}

fn try_build_graph(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
    source_images: &HashMap<String, ImageHandle>,
) -> Option<SkinObjectType> {
    let gr_def = data.graph.iter().find(|g| g.id == *dst_id)?;

    let value_id = if let Some(ref val) = gr_def.value {
        val.as_id().unwrap_or(gr_def.graph_type)
    } else {
        gr_def.graph_type
    };

    let direction = crate::skin_graph::GraphDirection::from_i32(gr_def.angle);

    let mut graph = crate::skin_graph::SkinGraph {
        base: SkinObjectBase::default(),
        direction,
        ref_id: Some(crate::property_id::FloatId(value_id)),
        ..Default::default()
    };
    apply_destination(&mut graph.base, dst);

    // Resolve source image handle
    if let Some(&handle) = source_images.get(gr_def.src.as_str()) {
        graph.source_images = vec![handle];
    }

    Some(graph.into())
}

fn try_build_bpm_graph(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let bg_def = data.bpmgraph.iter().find(|b| b.id == *dst_id)?;

    let mut bpm_graph = crate::skin_bpm_graph::SkinBpmGraph {
        base: SkinObjectBase::default(),
        delay: bg_def.delay,
        line_width: bg_def.line_width,
        colors: crate::skin_bpm_graph::BpmGraphColors {
            main_bpm: parse_color(&bg_def.main_bpm_color),
            min_bpm: parse_color(&bg_def.min_bpm_color),
            max_bpm: parse_color(&bg_def.max_bpm_color),
            other_bpm: parse_color(&bg_def.other_bpm_color),
            stop: parse_color(&bg_def.stop_line_color),
            transition: parse_color(&bg_def.transition_line_color),
        },
    };
    apply_destination(&mut bpm_graph.base, dst);

    Some(bpm_graph.into())
}

fn try_build_hit_error_visualizer(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let hev_def = data.hiterrorvisualizer.iter().find(|h| h.id == *dst_id)?;

    let mut vis = crate::skin_visualizer::SkinHitErrorVisualizer {
        base: SkinObjectBase::default(),
        width: hev_def.width,
        judge_width_millis: hev_def.judge_width_millis,
        line_width: hev_def.line_width,
        hiterror_mode: hev_def.hiterror_mode != 0,
        color_mode: hev_def.color_mode != 0,
        ema_mode: crate::skin_visualizer::EmaMode::from_i32(hev_def.ema_mode),
        ema_alpha: hev_def.alpha,
        window_length: hev_def.window_length,
        draw_decay: hev_def.draw_decay != 0,
        line_color: parse_color(&hev_def.line_color),
        center_color: parse_color(&hev_def.center_color),
        ema_color: parse_color(&hev_def.ema_color),
        judge_colors: [
            parse_color(&hev_def.pg_color),
            parse_color(&hev_def.gr_color),
            parse_color(&hev_def.gd_color),
            parse_color(&hev_def.bd_color),
            parse_color(&hev_def.pr_color),
        ],
    };
    apply_destination(&mut vis.base, dst);

    Some(vis.into())
}

fn try_build_timing_visualizer(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let tv_def = data.timingvisualizer.iter().find(|t| t.id == *dst_id)?;

    let mut vis = crate::skin_visualizer::SkinTimingVisualizer {
        base: SkinObjectBase::default(),
        width: tv_def.width,
        judge_width_millis: tv_def.judge_width_millis,
        line_width: tv_def.line_width,
        draw_decay: tv_def.draw_decay != 0,
        line_color: parse_color(&tv_def.line_color),
        center_color: parse_color(&tv_def.center_color),
        judge_colors: [
            parse_color(&tv_def.pg_color),
            parse_color(&tv_def.gr_color),
            parse_color(&tv_def.gd_color),
            parse_color(&tv_def.bd_color),
            parse_color(&tv_def.pr_color),
        ],
    };
    apply_destination(&mut vis.base, dst);

    Some(vis.into())
}

fn try_build_timing_distribution(
    data: &JsonSkinData,
    dst: &JsonDestination,
    dst_id: &FlexId,
) -> Option<SkinObjectType> {
    let td_def = data
        .timingdistributiongraph
        .iter()
        .find(|t| t.id == *dst_id)?;

    let mut graph = crate::skin_visualizer::SkinTimingDistributionGraph {
        base: SkinObjectBase::default(),
        graph_width: td_def.width,
        line_width: td_def.line_width,
        draw_average: td_def.draw_average != 0,
        draw_dev: td_def.draw_dev != 0,
        graph_color: parse_color(&td_def.graph_color),
        average_color: parse_color(&td_def.average_color),
        dev_color: parse_color(&td_def.dev_color),
        judge_colors: [
            parse_color(&td_def.pg_color),
            parse_color(&td_def.gr_color),
            parse_color(&td_def.gd_color),
            parse_color(&td_def.bd_color),
            parse_color(&td_def.pr_color),
        ],
    };
    apply_destination(&mut graph.base, dst);

    Some(graph.into())
}

// ---------------------------------------------------------------------------
// Destination processing
// ---------------------------------------------------------------------------

/// Applies a JSON destination to a SkinObjectBase.
///
/// Fills animation keyframes with inheritance (MIN_VALUE → inherit from
/// previous frame or use defaults), sets timer, blend, offsets, etc.
fn apply_destination(base: &mut SkinObjectBase, dst: &JsonDestination) {
    // Set base properties
    base.blend = dst.blend;
    base.filter = dst.filter;
    base.set_center(dst.center);
    base.name = Some(dst.id.as_str().to_string());

    // Timer
    if let Some(ref timer) = dst.timer
        && let Some(id) = timer.as_id()
    {
        base.timer = Some(TimerId(id));
    }

    // Loop
    base.loop_time = dst.loop_time;

    // Draw conditions — when draw is present (even as a Lua function/script),
    // op is completely ignored.  This matches Java behavior where
    // `if (dst.draw != null)` prevents op from being used as option_conditions.
    if let Some(ref draw) = dst.draw {
        if let Some(id) = draw.as_id() {
            base.draw_conditions.push(BooleanId(id));
        } else {
            // Lua draw function — mark so test harness can treat as hidden.
            base.has_script_draw = true;
        }
    } else if !dst.op.is_empty() {
        base.option_conditions = dst.op.clone();
    }

    // Stretch
    if dst.stretch >= 0 {
        base.stretch = StretchType::from_id(dst.stretch).unwrap_or_default();
    }

    // Mouse rect
    if let Some(ref mr) = dst.mouse_rect {
        base.mouse_rect = Some(Rect::new(
            mr.x as f32,
            mr.y as f32,
            mr.w as f32,
            mr.h as f32,
        ));
    }

    // Animation keyframes
    let mut prev: Option<ResolvedAnimation> = None;
    for anim in &dst.dst {
        let resolved = resolve_animation(anim, prev.as_ref());
        base.add_destination(Destination {
            time: resolved.time as i64,
            region: Rect::new(
                resolved.x as f32,
                resolved.y as f32,
                resolved.w as f32,
                resolved.h as f32,
            ),
            color: Color::from_rgba_u8(
                resolved.r as u8,
                resolved.g as u8,
                resolved.b as u8,
                resolved.a as u8,
            ),
            angle: resolved.angle,
            acc: resolved.acc,
        });
        prev = Some(resolved);
    }

    // Offsets
    let mut offset_ids: Vec<i32> = dst.offsets.clone();
    offset_ids.push(dst.offset);
    base.set_offset_ids(&offset_ids);
}

/// Resolves animation keyframe values, inheriting from the previous frame
/// or using defaults for the first frame.
///
/// Matches Java's `setDestination()` fill logic exactly.
fn resolve_animation(anim: &JsonAnimation, prev: Option<&ResolvedAnimation>) -> ResolvedAnimation {
    match prev {
        None => ResolvedAnimation {
            time: if anim.time == i32::MIN { 0 } else { anim.time },
            x: if anim.x == i32::MIN { 0 } else { anim.x },
            y: if anim.y == i32::MIN { 0 } else { anim.y },
            w: if anim.w == i32::MIN { 0 } else { anim.w },
            h: if anim.h == i32::MIN { 0 } else { anim.h },
            acc: if anim.acc == i32::MIN { 0 } else { anim.acc },
            a: if anim.a == i32::MIN { 255 } else { anim.a },
            r: if anim.r == i32::MIN { 255 } else { anim.r },
            g: if anim.g == i32::MIN { 255 } else { anim.g },
            b: if anim.b == i32::MIN { 255 } else { anim.b },
            angle: if anim.angle == i32::MIN {
                0
            } else {
                anim.angle
            },
        },
        Some(prev_resolved) => ResolvedAnimation {
            time: if anim.time == i32::MIN {
                prev_resolved.time
            } else {
                anim.time
            },
            x: if anim.x == i32::MIN {
                prev_resolved.x
            } else {
                anim.x
            },
            y: if anim.y == i32::MIN {
                prev_resolved.y
            } else {
                anim.y
            },
            w: if anim.w == i32::MIN {
                prev_resolved.w
            } else {
                anim.w
            },
            h: if anim.h == i32::MIN {
                prev_resolved.h
            } else {
                anim.h
            },
            acc: if anim.acc == i32::MIN {
                prev_resolved.acc
            } else {
                anim.acc
            },
            a: if anim.a == i32::MIN {
                prev_resolved.a
            } else {
                anim.a
            },
            r: if anim.r == i32::MIN {
                prev_resolved.r
            } else {
                anim.r
            },
            g: if anim.g == i32::MIN {
                prev_resolved.g
            } else {
                anim.g
            },
            b: if anim.b == i32::MIN {
                prev_resolved.b
            } else {
                anim.b
            },
            angle: if anim.angle == i32::MIN {
                prev_resolved.angle
            } else {
                anim.angle
            },
        },
    }
}
#[derive(Clone, Copy)]
struct ResolvedAnimation {
    time: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    acc: i32,
    a: i32,
    r: i32,
    g: i32,
    b: i32,
    angle: i32,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::json_skin::PropertyRef;

    // -- Option testing --

    #[test]
    fn test_option_single_enabled() {
        let enabled = HashSet::from([901]);
        assert!(test_option(&serde_json::json!(901), &enabled));
        assert!(!test_option(&serde_json::json!(902), &enabled));
    }

    #[test]
    fn test_option_negation() {
        let enabled = HashSet::from([901]);
        assert!(!test_option(&serde_json::json!(-901), &enabled));
        assert!(test_option(&serde_json::json!(-902), &enabled));
    }

    #[test]
    fn test_option_and() {
        let enabled = HashSet::from([901, 911]);
        assert!(test_option(&serde_json::json!([901, 911]), &enabled));
        assert!(!test_option(&serde_json::json!([901, 912]), &enabled));
    }

    #[test]
    fn test_option_or_and() {
        let enabled = HashSet::from([902, 911]);
        // (901 OR 902) AND 911
        assert!(test_option(&serde_json::json!([[901, 902], 911]), &enabled));
        // (903 OR 904) AND 911
        assert!(!test_option(
            &serde_json::json!([[903, 904], 911]),
            &enabled
        ));
    }

    #[test]
    fn test_option_null() {
        let enabled = HashSet::new();
        assert!(test_option(&Value::Null, &enabled));
    }

    // -- Conditional resolution --

    #[test]
    fn test_resolve_conditional_array() {
        let enabled = HashSet::from([901]);
        let json = serde_json::json!([
            {"if": 901, "value": {"id": "a"}},
            {"if": 902, "value": {"id": "b"}},
            {"id": "c"}
        ]);
        let resolved = resolve_conditionals(json, &enabled);
        let arr = resolved.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], "a");
        assert_eq!(arr[1]["id"], "c");
    }

    #[test]
    fn test_resolve_conditional_values() {
        let enabled = HashSet::from([901]);
        let json = serde_json::json!([
            {"if": 901, "values": [{"id": "a"}, {"id": "b"}]}
        ]);
        let resolved = resolve_conditionals(json, &enabled);
        let arr = resolved.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_resolve_conditional_not_matched() {
        let enabled = HashSet::new();
        let json = serde_json::json!([
            {"if": 901, "value": {"id": "a"}}
        ]);
        let resolved = resolve_conditionals(json, &enabled);
        let arr = resolved.as_array().unwrap();
        assert!(arr.is_empty());
    }

    #[test]
    fn test_resolve_object_conditional_branch_first_match() {
        let enabled = HashSet::from([901, 902]);
        let json = serde_json::json!({
            "obj": [
                {"if": 901, "value": {"id": "a"}},
                {"if": 902, "value": {"id": "b"}}
            ]
        });
        let resolved = resolve_conditionals(json, &enabled);
        assert!(resolved["obj"].is_object());
        assert_eq!(resolved["obj"]["id"], "a");
    }

    #[test]
    fn test_resolve_object_conditional_branch_no_match() {
        let enabled = HashSet::new();
        let json = serde_json::json!({
            "obj": [
                {"if": 901, "value": {"id": "a"}},
                {"if": 902, "value": {"id": "b"}}
            ]
        });
        let resolved = resolve_conditionals(json, &enabled);
        assert!(resolved["obj"].is_null());
    }

    // -- Header loading --

    #[test]
    fn test_load_header_minimal() {
        let json = r#"{
            "type": 6,
            "name": "Test Skin",
            "author": "Test Author",
            "w": 1280,
            "h": 720
        }"#;
        let header = load_header(json).unwrap();
        assert_eq!(header.name, "Test Skin");
        assert_eq!(header.author, "Test Author");
        assert_eq!(header.format, SkinFormat::Beatoraja);
    }

    #[test]
    fn test_load_header_with_options() {
        let json = r#"{
            "type": 0,
            "name": "Play Skin",
            "property": [
                {
                    "name": "BGA Size",
                    "item": [
                        {"name": "Normal", "op": 900},
                        {"name": "Extend", "op": 901}
                    ],
                    "def": "Normal"
                }
            ]
        }"#;
        let header = load_header(json).unwrap();
        assert_eq!(header.options.len(), 1);
        assert_eq!(header.options[0].option_ids, vec![900, 901]);
        assert_eq!(header.options[0].default_label, Some("Normal".to_string()));
    }

    #[test]
    fn test_load_header_play_offsets() {
        let json = r#"{"type": 0, "name": "Play7K"}"#;
        let header = load_header(json).unwrap();
        // Play skins get 4 standard offsets added
        assert_eq!(header.offsets.len(), 4);
        assert_eq!(header.offsets[0].name, "All offset(%)");
        assert_eq!(header.offsets[1].name, "Notes offset");
    }

    #[test]
    fn test_load_header_invalid_type() {
        let json = r#"{"name": "No Type"}"#;
        assert!(load_header(json).is_err());
    }

    // -- Full skin loading --

    #[test]
    fn test_load_skin_minimal() {
        let json = r#"{
            "type": 6,
            "name": "Decide",
            "w": 1280,
            "h": 720,
            "fadeout": 500,
            "scene": 3000,
            "destination": [
                {"id": -100, "dst": [{"x": 0, "y": 0, "w": 1280, "h": 720}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.fadeout, 500);
        assert_eq!(skin.scene, 3000);
        assert_eq!(skin.object_count(), 1);
    }

    #[test]
    fn test_load_skin_with_text() {
        let json = r#"{
            "type": 6,
            "name": "Test",
            "text": [
                {"id": "title", "font": 0, "size": 30, "ref": 12}
            ],
            "destination": [
                {"id": "title", "dst": [{"x": 100, "y": 200, "w": 18, "h": 18}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        match &skin.objects[0] {
            SkinObjectType::Text(t) => {
                assert_eq!(t.font_size, 30.0);
                assert_eq!(t.ref_id.unwrap().0, 12);
            }
            _ => panic!("Expected Text object"),
        }
    }

    #[test]
    fn test_load_skin_with_conditionals() {
        let json = r#"{
            "type": 6,
            "name": "Test",
            "image": [
                {"id": "img_a", "src": 0},
                {"id": "img_b", "src": 0}
            ],
            "destination": [
                {"if": 901, "value": {"id": "img_a", "dst": [{"x": 0, "y": 0, "w": 100, "h": 100}]}},
                {"id": "img_b", "dst": [{"x": 0, "y": 0, "w": 200, "h": 200}]}
            ]
        }"#;

        // Without option 901 enabled: only img_b
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);

        // With option 901 enabled: both objects
        let skin = load_skin(json, &HashSet::from([901]), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 2);
    }

    #[test]
    fn test_load_skin_custom_events() {
        let json = r#"{
            "type": 6,
            "name": "Test",
            "customEvents": [
                {"id": 1000, "action": 100, "condition": -50, "minInterval": 200}
            ],
            "customTimers": [
                {"id": 10000, "timer": 41},
                {"id": 10001}
            ],
            "destination": []
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.custom_events.len(), 1);
        assert_eq!(skin.custom_events[0].id, EventId(100));
        assert_eq!(skin.custom_events[0].condition, Some(BooleanId(-50)));
        assert_eq!(skin.custom_events[0].min_interval, 200);
        assert_eq!(skin.custom_timers.len(), 2);
        assert!(!skin.custom_timers[0].is_passive());
        assert!(skin.custom_timers[1].is_passive());
    }

    // -- Animation resolution --

    #[test]
    fn test_animation_first_frame_defaults() {
        let anim = JsonAnimation {
            time: i32::MIN,
            x: 100,
            y: i32::MIN,
            w: 200,
            h: i32::MIN,
            acc: i32::MIN,
            a: i32::MIN,
            r: i32::MIN,
            g: i32::MIN,
            b: i32::MIN,
            angle: i32::MIN,
        };
        let resolved = resolve_animation(&anim, None);
        assert_eq!(resolved.time, 0);
        assert_eq!(resolved.x, 100);
        assert_eq!(resolved.y, 0);
        assert_eq!(resolved.w, 200);
        assert_eq!(resolved.h, 0);
        assert_eq!(resolved.a, 255);
        assert_eq!(resolved.r, 255);
        assert_eq!(resolved.angle, 0);
    }

    #[test]
    fn test_animation_inheritance() {
        let prev = JsonAnimation {
            time: 0,
            x: 100,
            y: 200,
            w: 300,
            h: 400,
            acc: 0,
            a: 255,
            r: 255,
            g: 255,
            b: 255,
            angle: 45,
        };
        let prev_resolved = resolve_animation(&prev, None);
        let anim = JsonAnimation {
            time: 1000,
            x: i32::MIN, // inherit 100
            y: 500,      // override
            w: i32::MIN, // inherit 300
            h: i32::MIN, // inherit 400
            acc: i32::MIN,
            a: 128,
            r: i32::MIN,
            g: i32::MIN,
            b: i32::MIN,
            angle: i32::MIN, // inherit 45
        };
        let resolved = resolve_animation(&anim, Some(&prev_resolved));
        assert_eq!(resolved.time, 1000);
        assert_eq!(resolved.x, 100);
        assert_eq!(resolved.y, 500);
        assert_eq!(resolved.w, 300);
        assert_eq!(resolved.a, 128);
        assert_eq!(resolved.angle, 45);
    }

    // -- Destination processing --

    #[test]
    fn test_apply_destination_basic() {
        let dst = JsonDestination {
            id: FlexId::from("test"),
            blend: 2,
            filter: 1,
            center: 5,
            loop_time: 1000,
            stretch: 1,
            dst: vec![JsonAnimation {
                time: 0,
                x: 10,
                y: 20,
                w: 100,
                h: 50,
                acc: 0,
                a: 200,
                r: 255,
                g: 128,
                b: 64,
                angle: 30,
            }],
            ..Default::default()
        };
        let mut base = SkinObjectBase::default();
        apply_destination(&mut base, &dst);

        assert_eq!(base.blend, 2);
        assert_eq!(base.filter, 1);
        assert_eq!(base.center, 5);
        assert_eq!(base.loop_time, 1000);
        assert_eq!(base.stretch, StretchType::KeepAspectRatioFitInner);
        assert_eq!(base.destinations.len(), 1);
        assert_eq!(base.destinations[0].time, 0);
        assert!((base.destinations[0].region.x - 10.0).abs() < 0.001);
        assert_eq!(base.destinations[0].angle, 30);
    }

    #[test]
    fn test_apply_destination_with_timer_and_draw() {
        let dst = JsonDestination {
            id: FlexId::from("test"),
            timer: Some(PropertyRef::Id(42)),
            draw: Some(PropertyRef::Id(100)),
            dst: vec![JsonAnimation::default()],
            ..Default::default()
        };
        let mut base = SkinObjectBase::default();
        apply_destination(&mut base, &dst);

        assert_eq!(base.timer, Some(TimerId(42)));
        assert_eq!(base.draw_conditions, vec![BooleanId(100)]);
    }

    #[test]
    fn test_apply_destination_offsets() {
        let dst = JsonDestination {
            id: FlexId::from("test"),
            offset: 10,
            offsets: vec![20, 30],
            dst: vec![JsonAnimation::default()],
            ..Default::default()
        };
        let mut base = SkinObjectBase::default();
        apply_destination(&mut base, &dst);

        assert_eq!(base.offset_ids, vec![20, 30, 10]);
    }

    // -- Font resolution --

    #[test]
    fn test_font_resolution_ttf() {
        let json = r#"{
            "type": 6,
            "name": "Test",
            "font": [{"id": "0", "path": "fonts/myfont.ttf", "type": 0}],
            "text": [{"id": "title", "font": 0, "size": 24, "ref": 12}],
            "destination": [
                {"id": "title", "dst": [{"x": 0, "y": 0, "w": 200, "h": 30}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        match &skin.objects[0] {
            SkinObjectType::Text(t) => {
                assert!(matches!(t.font_type, FontType::Ttf(_)));
                if let FontType::Ttf(path) = &t.font_type {
                    assert_eq!(path, "fonts/myfont.ttf");
                }
            }
            _ => panic!("Expected Text object"),
        }
    }

    #[test]
    fn test_font_resolution_bitmap() {
        let json = r#"{
            "type": 6,
            "name": "Test",
            "font": [{"id": "0", "path": "fonts/bitmap.fnt", "type": 1}],
            "text": [{"id": "title", "font": 0, "size": 24, "ref": 12}],
            "destination": [
                {"id": "title", "dst": [{"x": 0, "y": 0, "w": 200, "h": 30}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        match &skin.objects[0] {
            SkinObjectType::Text(t) => {
                if let FontType::Bitmap { path, bitmap_type } = &t.font_type {
                    assert_eq!(path, "fonts/bitmap.fnt");
                    assert_eq!(*bitmap_type, 1);
                } else {
                    panic!("Expected Bitmap font type");
                }
            }
            _ => panic!("Expected Text object"),
        }
    }

    #[test]
    fn test_font_resolution_default_when_missing() {
        let json = r#"{
            "type": 6,
            "name": "Test",
            "text": [{"id": "title", "font": 99, "size": 24, "ref": 12}],
            "destination": [
                {"id": "title", "dst": [{"x": 0, "y": 0, "w": 200, "h": 30}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        match &skin.objects[0] {
            SkinObjectType::Text(t) => {
                assert!(matches!(t.font_type, FontType::Default));
            }
            _ => panic!("Expected Text object"),
        }
    }

    // -- Real skin loading (ECFN) --

    #[test]
    fn test_load_ecfn_select_skin_no_crash() {
        let skin_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("skins")
            .join("ECFN")
            .join("select")
            .join("select.json");

        if !skin_path.exists() {
            eprintln!("ECFN skin not found, skipping: {}", skin_path.display());
            return;
        }

        let json_str = std::fs::read_to_string(&skin_path).unwrap();

        // load_skin (no images) must not crash even with missing source images
        let skin = load_skin(&json_str, &HashSet::new(), Resolution::Hd, Some(&skin_path)).unwrap();

        assert_eq!(skin.header.name, "beatoraja_default");
        assert!(skin.object_count() > 0);

        // load_skin_with_images with empty map also must not crash
        let skin2 = load_skin_with_images(
            &json_str,
            &HashSet::new(),
            Resolution::Hd,
            Some(&skin_path),
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(skin.object_count(), skin2.object_count());
    }

    #[test]
    fn test_load_skin_with_images_missing_sources_graceful() {
        let json = r#"{
            "type": 6,
            "name": "Test Missing Sources",
            "source": [
                {"id": 0, "path": "nonexistent.png"},
                {"id": 1, "path": "also_missing.png"}
            ],
            "image": [
                {"id": "img_a", "src": 0},
                {"id": "img_b", "src": 1}
            ],
            "slider": [
                {"id": "sl", "src": 0, "angle": 1, "range": 50, "type": 17}
            ],
            "graph": [
                {"id": "gr", "src": 1, "angle": 1, "type": 100}
            ],
            "destination": [
                {"id": "img_a", "dst": [{"x": 0, "y": 0, "w": 100, "h": 100}]},
                {"id": "img_b", "dst": [{"x": 0, "y": 0, "w": 100, "h": 100}]},
                {"id": "sl", "dst": [{"x": 0, "y": 0, "w": 10, "h": 10}]},
                {"id": "gr", "dst": [{"x": 0, "y": 0, "w": 100, "h": 10}]}
            ]
        }"#;

        // Empty source_images map — all sources are "missing"
        let skin =
            load_skin_with_images(json, &HashSet::new(), Resolution::Hd, None, &HashMap::new())
                .unwrap();

        // All 4 objects should still be created (just with empty source images)
        assert_eq!(skin.object_count(), 4);

        // Image objects should have empty sources
        match &skin.objects[0] {
            SkinObjectType::Image(img) => assert!(img.sources.is_empty()),
            _ => panic!("Expected Image"),
        }

        // Slider should have empty source_images
        match &skin.objects[2] {
            SkinObjectType::Slider(sl) => assert!(sl.source_images.is_empty()),
            _ => panic!("Expected Slider"),
        }

        // Graph should have empty source_images
        match &skin.objects[3] {
            SkinObjectType::Graph(gr) => assert!(gr.source_images.is_empty()),
            _ => panic!("Expected Graph"),
        }
    }

    // -- JSON pre-processing --

    #[test]
    fn test_preprocess_trailing_comma() {
        let input = r#"{"a": [1, 2, 3, ], "b": {"x": 1, }}"#;
        let output = preprocess_json(input);
        assert!(serde_json::from_str::<Value>(&output).is_ok());
    }

    #[test]
    fn test_preprocess_missing_comma() {
        let input = r#"[{"id": "a"} {"id": "b"}]"#;
        let output = preprocess_json(input);
        let parsed: Vec<Value> = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_preprocess_both_issues() {
        let input = r#"{"items": [{"x": 1,} {"y": 2}]}"#;
        let output = preprocess_json(input);
        assert!(serde_json::from_str::<Value>(&output).is_ok());
    }

    #[test]
    fn test_preprocess_preserves_strings() {
        let input = r#"{"text": "hello} {world", "x": 1}"#;
        let output = preprocess_json(input);
        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["text"], "hello} {world");
    }

    #[test]
    fn test_preprocess_valid_json_unchanged() {
        let input = r#"{"a": [1, 2], "b": {"c": 3}}"#;
        let output = preprocess_json(input);
        assert_eq!(
            serde_json::from_str::<Value>(&output).unwrap(),
            serde_json::from_str::<Value>(input).unwrap()
        );
    }

    // -- State-specific config collection --

    #[test]
    fn test_collect_play_config_with_note() {
        let json = r#"{
            "type": 0,
            "name": "Play7K",
            "note": {"id": "note1", "dst": [{"x": 0, "y": 0, "w": 100, "h": 400}]},
            "destination": [
                {"id": "note1", "dst": [{"x": 0, "y": 0, "w": 100, "h": 400}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert!(skin.play_config.is_some());
        let config = skin.play_config.unwrap();
        assert!(config.note.is_some());
    }

    #[test]
    fn test_collect_play_config_with_judge() {
        let json = r#"{
            "type": 0,
            "name": "Play7K",
            "judge": [{"id": "judge1", "index": 0, "shift": true}],
            "destination": [
                {"id": "judge1", "dst": [{"x": 0, "y": 0, "w": 200, "h": 50}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert!(skin.play_config.is_some());
        let config = skin.play_config.unwrap();
        assert_eq!(config.judges.len(), 1);
        assert_eq!(config.judges[0].player, 0);
        assert!(config.judges[0].shift);
    }

    #[test]
    fn test_collect_play_config_with_bga() {
        let json = r#"{
            "type": 0,
            "name": "Play7K",
            "bga": {"id": "bga1"},
            "destination": [
                {"id": "bga1", "dst": [{"x": 0, "y": 0, "w": 256, "h": 256}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert!(skin.play_config.is_some());
        assert!(skin.play_config.unwrap().bga.is_some());
    }

    #[test]
    fn test_collect_select_config_with_bar() {
        let json = r#"{
            "type": 5,
            "name": "Select",
            "songlist": {"id": "bar1", "center": 5},
            "destination": [
                {"id": "bar1", "dst": [{"x": 0, "y": 0, "w": 800, "h": 40}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert!(skin.select_config.is_some());
        let config = skin.select_config.unwrap();
        assert!(config.bar.is_some());
        assert_eq!(config.bar.unwrap().position, 5);
    }

    #[test]
    fn test_no_config_for_decide() {
        let json = r#"{
            "type": 6,
            "name": "Decide",
            "image": [{"id": "bg", "src": 0}],
            "destination": [
                {"id": "bg", "dst": [{"x": 0, "y": 0, "w": 1280, "h": 720}]}
            ]
        }"#;
        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert!(skin.play_config.is_none());
        assert!(skin.select_config.is_none());
        assert!(skin.result_config.is_none());
        assert!(skin.course_result_config.is_none());
    }

    // -- Note lane population (19-A6) --

    #[test]
    fn test_note_lanes_populated() {
        let mut images = HashMap::new();
        images.insert("0".to_string(), ImageHandle(1));

        let json = r#"{
            "type": 0,
            "name": "Play7K",
            "source": [{"id": 0, "path": "notes.png"}],
            "image": [
                {"id": "n1", "src": 0},
                {"id": "n2", "src": 0},
                {"id": "ls1", "src": 0},
                {"id": "le1", "src": 0},
                {"id": "m1", "src": 0},
                {"id": "grp", "src": 0}
            ],
            "note": {
                "id": "note1",
                "note": ["n1", "n2"],
                "lnstart": ["ls1"],
                "lnend": ["le1"],
                "mine": ["m1"],
                "size": [1.5, 2.0],
                "dst2": 10,
                "dst": [
                    {"x": 0, "y": 0, "w": 50, "h": 400},
                    {"x": 50, "y": 0, "w": 50, "h": 400}
                ],
                "group": [{"id": "grp", "dst": [{"x": 0, "y": 0, "w": 100, "h": 2}]}]
            },
            "destination": [
                {"id": "note1", "dst": [{"x": 0, "y": 0, "w": 100, "h": 400}]}
            ]
        }"#;

        let skin =
            load_skin_with_images(json, &HashSet::new(), Resolution::Hd, None, &images).unwrap();
        assert!(skin.play_config.is_some());
        let config = skin.play_config.unwrap();
        let note = config.note.unwrap();

        assert_eq!(note.lanes.len(), 2);
        // Lane 0: note, lnstart, lnend, mine populated
        assert_eq!(note.lanes[0].note, Some(1));
        assert_eq!(note.lanes[0].longnote[crate::skin_note::LN_START], Some(1));
        assert_eq!(note.lanes[0].longnote[crate::skin_note::LN_END], Some(1));
        assert_eq!(note.lanes[0].mine_note, Some(1));
        assert!((note.lanes[0].scale - 1.5).abs() < f32::EPSILON);
        assert_eq!(note.lanes[0].dst_note2, 10);

        // Lane 1: only note populated
        assert_eq!(note.lanes[1].note, Some(1));
        assert!(note.lanes[1].longnote[crate::skin_note::LN_START].is_none());
        assert!((note.lanes[1].scale - 2.0).abs() < f32::EPSILON);

        // Line image
        assert_eq!(note.line_image, Some(1));
    }

    #[test]
    fn test_note_ln_body_active_branch() {
        let mut images = HashMap::new();
        images.insert("0".to_string(), ImageHandle(10));

        // With lnbody_active: active→LN_BODY_ACTIVE, body→LN_BODY_INACTIVE
        let json = r#"{
            "type": 0,
            "name": "Play7K",
            "source": [{"id": 0, "path": "notes.png"}],
            "image": [
                {"id": "body", "src": 0},
                {"id": "active", "src": 0}
            ],
            "note": {
                "id": "note1",
                "lnbody": ["body"],
                "lnbodyActive": ["active"],
                "dst": [{"x": 0, "y": 0, "w": 50, "h": 400}]
            },
            "destination": [
                {"id": "note1", "dst": [{"x": 0, "y": 0, "w": 50, "h": 400}]}
            ]
        }"#;

        let skin =
            load_skin_with_images(json, &HashSet::new(), Resolution::Hd, None, &images).unwrap();
        let note = skin.play_config.unwrap().note.unwrap();
        assert_eq!(note.lanes.len(), 1);
        assert_eq!(
            note.lanes[0].longnote[crate::skin_note::LN_BODY_ACTIVE],
            Some(10)
        );
        assert_eq!(
            note.lanes[0].longnote[crate::skin_note::LN_BODY_INACTIVE],
            Some(10)
        );

        // Without lnbody_active: body→LN_BODY_ACTIVE, lnactive→LN_BODY_INACTIVE
        let json2 = r#"{
            "type": 0,
            "name": "Play7K",
            "source": [{"id": 0, "path": "notes.png"}],
            "image": [
                {"id": "body", "src": 0},
                {"id": "lnact", "src": 0}
            ],
            "note": {
                "id": "note1",
                "lnbody": ["body"],
                "lnactive": ["lnact"],
                "dst": [{"x": 0, "y": 0, "w": 50, "h": 400}]
            },
            "destination": [
                {"id": "note1", "dst": [{"x": 0, "y": 0, "w": 50, "h": 400}]}
            ]
        }"#;

        let skin2 =
            load_skin_with_images(json2, &HashSet::new(), Resolution::Hd, None, &images).unwrap();
        let note2 = skin2.play_config.unwrap().note.unwrap();
        assert_eq!(
            note2.lanes[0].longnote[crate::skin_note::LN_BODY_ACTIVE],
            Some(10)
        );
        assert_eq!(
            note2.lanes[0].longnote[crate::skin_note::LN_BODY_INACTIVE],
            Some(10)
        );
    }

    // -- Judge population (19-A6) --

    #[test]
    fn test_judge_images_and_numbers() {
        let mut images = HashMap::new();
        images.insert("0".to_string(), ImageHandle(5));

        let json = r#"{
            "type": 0,
            "name": "Play7K",
            "source": [{"id": 0, "path": "judge.png"}],
            "image": [
                {"id": "j_pg", "src": 0},
                {"id": "j_gr", "src": 0}
            ],
            "value": [
                {"id": "cnt_pg", "src": 0, "digit": 4, "ref": 150},
                {"id": "cnt_gr", "src": 0, "digit": 4, "ref": 151}
            ],
            "judge": [{
                "id": "judge1",
                "index": 0,
                "shift": true,
                "images": [
                    {"id": "j_pg", "dst": [{"x": 0, "y": 0, "w": 200, "h": 50}]},
                    {"id": "j_gr", "dst": [{"x": 0, "y": 50, "w": 200, "h": 50}]}
                ],
                "numbers": [
                    {"id": "cnt_pg", "dst": [{"x": 200, "y": 0, "w": 20, "h": 30}]},
                    {"id": "cnt_gr", "dst": [{"x": 200, "y": 50, "w": 20, "h": 30}]}
                ]
            }],
            "destination": [
                {"id": "judge1", "dst": [{"x": 0, "y": 0, "w": 400, "h": 100}]}
            ]
        }"#;

        let skin =
            load_skin_with_images(json, &HashSet::new(), Resolution::Hd, None, &images).unwrap();
        let config = skin.play_config.unwrap();
        assert_eq!(config.judges.len(), 1);

        let judge = &config.judges[0];
        assert!(judge.judge_images[0].is_some()); // PG image
        assert!(judge.judge_images[1].is_some()); // GR image
        assert!(judge.judge_images[2].is_none()); // GD not set

        assert!(judge.judge_counts[0].is_some()); // PG count
        assert!(judge.judge_counts[1].is_some()); // GR count
        assert!(judge.judge_counts[2].is_none()); // GD not set

        // Numbers should have relative=true
        let pg_num = judge.judge_counts[0].as_ref().unwrap();
        assert!(pg_num.relative);
        assert_eq!(pg_num.keta, 4);
        assert_eq!(pg_num.ref_id, Some(crate::property_id::IntegerId(150)));
    }

    // -- Song list sub-objects (19-B4) --

    #[test]
    fn test_song_list_sub_objects() {
        let mut images = HashMap::new();
        images.insert("0".to_string(), ImageHandle(3));

        let json = r#"{
            "type": 5,
            "name": "Select",
            "source": [{"id": 0, "path": "select.png"}],
            "image": [
                {"id": "lamp_img", "src": 0},
                {"id": "label_img", "src": 0},
                {"id": "trophy_img", "src": 0}
            ],
            "value": [
                {"id": "lv_num", "src": 0, "digit": 3, "ref": 300}
            ],
            "text": [
                {"id": "title_text", "font": 0, "size": 20, "ref": 10}
            ],
            "songlist": {
                "id": "bar1",
                "center": 5,
                "lamp": [
                    {"id": "lamp_img", "dst": [{"x": 0, "y": 0, "w": 10, "h": 10}]}
                ],
                "label": [
                    {"id": "label_img", "dst": [{"x": 20, "y": 0, "w": 10, "h": 10}]}
                ],
                "trophy": [
                    {"id": "trophy_img", "dst": [{"x": 40, "y": 0, "w": 10, "h": 10}]}
                ],
                "text": [
                    {"id": "title_text", "dst": [{"x": 60, "y": 0, "w": 200, "h": 20}]}
                ],
                "level": [
                    {"id": "lv_num", "dst": [{"x": 260, "y": 0, "w": 50, "h": 20}]}
                ]
            },
            "destination": [
                {"id": "bar1", "dst": [{"x": 0, "y": 0, "w": 800, "h": 40}]}
            ]
        }"#;

        let skin =
            load_skin_with_images(json, &HashSet::new(), Resolution::Hd, None, &images).unwrap();
        assert!(skin.select_config.is_some());
        let config = skin.select_config.unwrap();
        let bar = config.bar.unwrap();

        assert_eq!(bar.position, 5);
        assert!(bar.lamp[0].is_some());
        assert!(bar.lamp[1].is_none());
        assert!(bar.label[0].is_some());
        assert!(bar.trophy[0].is_some());
        assert!(bar.text[0].is_some());
        let text = bar.text[0].as_ref().unwrap();
        assert_eq!(text.ref_id, Some(crate::property_id::StringId(10)));
        assert!(bar.bar_level[0].is_some());
        let level = bar.bar_level[0].as_ref().unwrap();
        assert_eq!(level.keta, 3);
        assert_eq!(level.ref_id, Some(crate::property_id::IntegerId(300)));
    }

    // -- Play config timing (19-E2) --

    #[test]
    fn test_play_config_timing() {
        let json = r#"{
            "type": 0,
            "name": "Play7K",
            "close": 1500,
            "loadend": 2000,
            "playstart": 1000,
            "judgetimer": 2,
            "finishmargin": 500,
            "destination": []
        }"#;

        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert!(skin.play_config.is_some());
        let config = skin.play_config.unwrap();
        assert_eq!(config.playstart, 1000);
        assert_eq!(config.loadstart, 1500);
        assert_eq!(config.loadend, 2000);
        assert_eq!(config.judge_timer, 2);
        assert_eq!(config.finish_margin, 500);
    }

    #[test]
    fn test_play_config_timing_defaults() {
        let json = r#"{
            "type": 0,
            "name": "Play7K",
            "destination": []
        }"#;

        let skin = load_skin(json, &HashSet::new(), Resolution::Hd, None).unwrap();
        assert!(skin.play_config.is_some());
        let config = skin.play_config.unwrap();
        assert_eq!(config.playstart, 0);
        assert_eq!(config.loadstart, 0);
        assert_eq!(config.loadend, 0);
        assert_eq!(config.judge_timer, 1); // default judgetimer is 1
        assert_eq!(config.finish_margin, 0);
    }
}

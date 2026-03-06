// Mechanical translation of JSONSkinLoader.java
// Main JSON skin loader

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use log::{error, warn};

use crate::json::json_course_result_skin_object_loader::JsonCourseResultSkinObjectLoader;
use crate::json::json_decide_skin_object_loader::JsonDecideSkinObjectLoader;
use crate::json::json_key_configuration_skin_object_loader::JsonKeyConfigurationSkinObjectLoader;
use crate::json::json_play_skin_object_loader::JsonPlaySkinObjectLoader;
use crate::json::json_result_skin_object_loader::JsonResultSkinObjectLoader;
use crate::json::json_select_skin_object_loader::JsonSelectSkinObjectLoader;
use crate::json::json_skin;
use crate::json::json_skin_configuration_skin_object_loader::JsonSkinConfigurationSkinObjectLoader;
use crate::json::json_skin_serializer::JsonSkinSerializer;
use crate::stubs::*;

/// Parse a JSON string into a `json_skin::Skin` with Gson-compatible leniency.
///
/// Java Gson silently accepts trailing commas and coerces numbers to strings.
/// serde_json is strict, so we preprocess the input to match Gson behavior:
/// 1. Strip trailing commas before `]` and `}` (syntax level)
/// 2. Coerce numeric values to strings for `id`/`src`/`font` keys (value level)
fn parse_skin_json(content: &str) -> Result<json_skin::Skin, serde_json::Error> {
    let cleaned = fix_lenient_json(content);
    let mut value: serde_json::Value = serde_json::from_str(&cleaned)?;
    coerce_json_numbers_to_strings(&mut value);
    serde_json::from_value(value)
}

/// Apply Gson-compatible leniency fixes to a JSON string:
/// 1. Strip UTF-8 BOM prefix
/// 2. Strip `//` line comments and `/* */` block comments (string-aware)
/// 3. Strip trailing commas before `]` and `}`
/// 4. Insert missing commas between `}` and `{` (array element separators)
///
/// All transformations are string-literal-aware: braces/commas inside `"..."`
/// are never modified.
fn fix_lenient_json(json: &str) -> String {
    // 1. Strip UTF-8 BOM
    let json = json.strip_prefix('\u{FEFF}').unwrap_or(json);

    // 2. Strip comments (string-aware state machine)
    let stripped = strip_comments(json);

    // 3-4. Fix trailing commas and missing commas (string-aware state machine)
    fix_commas_string_aware(&stripped)
}

/// String-aware comma fixer: removes trailing commas and inserts missing commas
/// between adjacent objects, without touching content inside string literals.
fn fix_commas_string_aware(json: &str) -> String {
    let bytes = json.as_bytes();
    let len = bytes.len();
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            out.push(bytes[i]);
            if bytes[i] == b'\\' {
                i += 1;
                if i < len {
                    out.push(bytes[i]);
                }
            } else if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Outside string
        match bytes[i] {
            b'"' => {
                in_string = true;
                out.push(b'"');
                i += 1;
            }
            b',' => {
                // Check if this is a trailing comma (comma followed by whitespace then ] or })
                let mut j = i + 1;
                while j < len && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < len && (bytes[j] == b'}' || bytes[j] == b']') {
                    // Trailing comma - skip it
                    i += 1;
                } else {
                    out.push(b',');
                    i += 1;
                }
            }
            b'}' => {
                out.push(b'}');
                // Check if next non-whitespace is '{' - insert missing comma
                let mut j = i + 1;
                while j < len && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < len && bytes[j] == b'{' {
                    out.push(b',');
                }
                i += 1;
            }
            _ => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }

    // SAFETY: input is valid UTF-8 and we only inserted ASCII bytes
    String::from_utf8(out).unwrap_or_else(|_| json.to_string())
}

/// Strip `//` line comments and `/* */` block comments from JSON text,
/// preserving comment-like sequences inside string literals.
fn strip_comments(json: &str) -> String {
    let bytes = json.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            let ch = bytes[i];
            out.push(ch as char);
            if ch == b'\\' {
                // Escaped character: copy next byte verbatim
                i += 1;
                if i < len {
                    out.push(bytes[i] as char);
                }
            } else if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Outside string
        if bytes[i] == b'"' {
            in_string = true;
            out.push('"');
            i += 1;
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            // Line comment: skip to end of line
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            // Keep the newline to preserve line structure
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Block comment: skip to closing */
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2; // skip */
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }

    out
}

/// Recursively walk a JSON value tree and convert numeric values to strings
/// for object keys known to be `Option<String>` in the Rust model.
fn coerce_json_numbers_to_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if matches!(key.as_str(), "id" | "src" | "font") && val.is_number() {
                    *val = serde_json::Value::String(
                        val.as_i64()
                            .map(|n| n.to_string())
                            .or_else(|| val.as_f64().map(|n| n.to_string()))
                            .unwrap_or_default(),
                    );
                }
                coerce_json_numbers_to_strings(val);
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                coerce_json_numbers_to_strings(val);
            }
        }
        _ => {}
    }
}

/// Corresponds to JSONSkinLoader.SourceData
#[derive(Clone, Debug)]
pub struct SourceData {
    pub path: String,
    pub loaded: bool,
    pub data: Option<SourceDataType>,
}

#[derive(Clone, Debug)]
pub enum SourceDataType {
    Texture(Texture),
    Movie(SkinSourceMovie),
}

/// Stub for SkinSourceMovie
#[derive(Clone, Debug, Default)]
pub struct SkinSourceMovie {
    pub path: String,
}

impl SourceData {
    pub fn new(path: String) -> Self {
        Self {
            path,
            loaded: false,
            data: None,
        }
    }
}

/// Corresponds to JSONSkinLoader
pub struct JSONSkinLoader {
    pub dstr: Resolution,
    pub usecim: bool,
    pub bga_expand: i32,

    pub sk: Option<json_skin::Skin>,

    pub source_map: HashMap<String, SourceData>,
    pub bitmap_source_map: HashMap<String, ()>, // SkinTextBitmap.SkinTextBitmapSource stubbed

    pub filemap: HashMap<String, String>,

    pub serializer: Option<JsonSkinSerializer>,
}

impl Default for JSONSkinLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl JSONSkinLoader {
    /// Constructor for header loading
    pub fn new() -> Self {
        Self {
            dstr: Resolution {
                width: 1920.0,
                height: 1080.0,
            },
            usecim: false,
            bga_expand: -1,
            sk: None,
            source_map: HashMap::new(),
            bitmap_source_map: HashMap::new(),
            filemap: HashMap::new(),
            serializer: None,
        }
    }

    /// Constructor for skin body loading
    pub fn with_config(config: &rubato_core::config::Config) -> Self {
        Self {
            dstr: Resolution {
                width: config.window_width as f32,
                height: config.window_height as f32,
            },
            usecim: false,
            bga_expand: config.bga_expand,
            sk: None,
            source_map: HashMap::new(),
            bitmap_source_map: HashMap::new(),
            filemap: HashMap::new(),
            serializer: None,
        }
    }

    pub fn load_skin(
        &mut self,
        p: &Path,
        skin_type: &crate::skin_type::SkinType,
        _property: &SkinConfigProperty,
    ) -> Option<SkinData> {
        self.load(p, skin_type, _property)
    }

    pub fn load_header(&mut self, p: &Path) -> Option<SkinHeaderData> {
        self.serializer = Some(JsonSkinSerializer::new());

        // Try reading as UTF-8 first, then Shift_JIS
        let content = match std::fs::read_to_string(p) {
            Ok(c) => c,
            Err(_) => match std::fs::read(p) {
                Ok(bytes) => {
                    warn!("Error parsing json, retrying with Shift JIS: {:?}", p);
                    let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                    decoded.into_owned()
                }
                Err(_) => {
                    error!("JSON skin file not found: {:?}", p);
                    return None;
                }
            },
        };

        match parse_skin_json(&content) {
            Ok(sk) => {
                self.sk = Some(sk.clone());
                self.load_json_skin_header(&sk, p)
            }
            Err(_e) => {
                // Try Shift_JIS
                match std::fs::read(p) {
                    Ok(bytes) => {
                        warn!("Error parsing json, retrying with Shift JIS: {:?}", p);
                        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                        match parse_skin_json(&decoded) {
                            Ok(sk) => {
                                self.sk = Some(sk.clone());
                                self.load_json_skin_header(&sk, p)
                            }
                            Err(e2) => {
                                error!("Failed to parse JSON skin: {:?} - {}", p, e2);
                                None
                            }
                        }
                    }
                    Err(_) => {
                        error!("JSON skin file not found: {:?}", p);
                        None
                    }
                }
            }
        }
    }

    /// Public accessor for LuaSkinLoader to call loadJsonSkinHeader
    pub fn load_header_from_skin(&self, sk: &json_skin::Skin, p: &Path) -> Option<SkinHeaderData> {
        self.load_json_skin_header(sk, p)
    }

    fn load_json_skin_header(&self, sk: &json_skin::Skin, p: &Path) -> Option<SkinHeaderData> {
        if sk.skin_type == -1 {
            return None;
        }

        let source_resolution = if sk.w > 0 && sk.h > 0 {
            Some(Resolution {
                width: sk.w as f32,
                height: sk.h as f32,
            })
        } else {
            None
        };

        let mut header = SkinHeaderData::new();
        header.skin_type = sk.skin_type;
        header.name = sk.name.clone().unwrap_or_default();
        header.author = sk.author.clone().unwrap_or_default();
        header.path = p.to_path_buf();
        header.header_type = HEADER_TYPE_BEATORJASKIN;
        header.source_resolution = source_resolution;

        // Process categories
        let mut category_items: Vec<Vec<Option<CustomItemData>>> =
            Vec::with_capacity(sk.category.len());
        for category in &sk.category {
            category_items.push(vec![None; category.item.len()]);
        }

        // Process properties -> options
        let mut options: Vec<CustomOptionData> = Vec::with_capacity(sk.property.len());
        for pr in sk.property.iter() {
            let mut op: Vec<i32> = Vec::with_capacity(pr.item.len());
            let mut names: Vec<String> = Vec::with_capacity(pr.item.len());
            for item in &pr.item {
                op.push(item.op);
                names.push(item.name.clone().unwrap_or_default());
            }
            let option = CustomOptionData {
                name: pr.name.clone().unwrap_or_default(),
                option: op,
                names,
                def: pr.def.clone(),
                selected_option: 0,
            };

            // Associate with categories
            for (cat_idx, category) in sk.category.iter().enumerate() {
                for (item_idx, item) in category.item.iter().enumerate() {
                    if let Some(ref pr_category) = pr.category
                        && item == pr_category
                        && let Some(items) = category_items.get_mut(cat_idx)
                        && let Some(slot) = items.get_mut(item_idx)
                    {
                        *slot = Some(CustomItemData::Option(option.clone()));
                    }
                }
            }

            options.push(option);
        }
        header.custom_options = options;

        // Process filepaths -> files
        let mut files: Vec<CustomFileData> = Vec::with_capacity(sk.filepath.len());
        for pr in &sk.filepath {
            // Keep filepath as-is without prepending parent directory
            let file = CustomFileData {
                name: pr.name.clone().unwrap_or_default(),
                path: pr.path.clone().unwrap_or_default(),
                def: pr.def.clone(),
                selected_filename: None,
            };

            for (cat_idx, category) in sk.category.iter().enumerate() {
                for (item_idx, item) in category.item.iter().enumerate() {
                    if let Some(ref pr_category) = pr.category
                        && item == pr_category
                        && let Some(items) = category_items.get_mut(cat_idx)
                        && let Some(slot) = items.get_mut(item_idx)
                    {
                        *slot = Some(CustomItemData::File(file.clone()));
                    }
                }
            }

            files.push(file);
        }
        header.custom_files = files;

        // Process offsets
        // Only PLAY_* types get the default offsets
        let offset_length_addition = match header.skin_type {
            0 | 1 | 2 | 3 | 4 | 12 | 13 | 14 | 16 | 17 | 18 => 4, // PLAY_* types
            _ => 0,
        };

        let mut offsets: Vec<CustomOffsetData> =
            Vec::with_capacity(sk.offset.len() + offset_length_addition);
        for pr in &sk.offset {
            let offset = CustomOffsetData {
                name: pr.name.clone().unwrap_or_default(),
                id: pr.id,
                x: pr.x,
                y: pr.y,
                w: pr.w,
                h: pr.h,
                r: pr.r,
                a: pr.a,
            };

            for (cat_idx, category) in sk.category.iter().enumerate() {
                for (item_idx, item) in category.item.iter().enumerate() {
                    if let Some(ref pr_category) = pr.category
                        && item == pr_category
                        && let Some(items) = category_items.get_mut(cat_idx)
                        && let Some(slot) = items.get_mut(item_idx)
                    {
                        *slot = Some(CustomItemData::Offset(offset.clone()));
                    }
                }
            }

            offsets.push(offset);
        }

        // Add play-specific offsets
        if offset_length_addition > 0 {
            offsets.push(CustomOffsetData {
                name: "All offset(%)".to_string(),
                id: OFFSET_ALL,
                x: true,
                y: true,
                w: true,
                h: true,
                r: false,
                a: false,
            });
            offsets.push(CustomOffsetData {
                name: "Notes offset".to_string(),
                id: OFFSET_NOTES_1P,
                x: false,
                y: false,
                w: false,
                h: true,
                r: false,
                a: false,
            });
            offsets.push(CustomOffsetData {
                name: "Judge offset".to_string(),
                id: OFFSET_JUDGE_1P,
                x: true,
                y: true,
                w: true,
                h: true,
                r: false,
                a: true,
            });
            offsets.push(CustomOffsetData {
                name: "Judge Detail offset".to_string(),
                id: OFFSET_JUDGEDETAIL_1P,
                x: true,
                y: true,
                w: true,
                h: true,
                r: false,
                a: true,
            });
        }
        header.custom_offsets = offsets;

        // Process categories
        let mut categories: Vec<CustomCategoryData> = Vec::new();
        for (i, pr) in sk.category.iter().enumerate() {
            let mut items_vec: Vec<CustomItemData> = Vec::new();
            if let Some(items) = category_items.get(i) {
                for item_data in items.iter().flatten() {
                    items_vec.push(item_data.clone());
                }
            }
            categories.push(CustomCategoryData {
                name: pr.name.clone().unwrap_or_default(),
                items: items_vec,
            });
        }
        header.custom_categories = categories;

        Some(header)
    }

    pub fn load(
        &mut self,
        p: &Path,
        skin_type: &crate::skin_type::SkinType,
        _property: &SkinConfigProperty,
    ) -> Option<SkinData> {
        self.serializer = Some(JsonSkinSerializer::new());

        let header = self.load_header(p)?;

        // Read and parse JSON
        let content = match std::fs::read_to_string(p) {
            Ok(c) => c,
            Err(_) => match std::fs::read(p) {
                Ok(bytes) => {
                    warn!("Error parsing json, retrying with Shift JIS: {:?}", p);
                    let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                    decoded.into_owned()
                }
                Err(_) => {
                    error!("JSON skin file not found: {:?}", p);
                    return None;
                }
            },
        };

        let sk = match parse_skin_json(&content) {
            Ok(s) => s,
            Err(_) => match std::fs::read(p) {
                Ok(bytes) => {
                    let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                    match parse_skin_json(&decoded) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Failed to parse JSON skin: {}", e);
                            return None;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read JSON skin: {}", e);
                    return None;
                }
            },
        };
        self.sk = Some(sk.clone());

        self.load_json_skin(&header, &sk, skin_type, _property, p)
    }

    fn get_enabled_options(&self, header: &SkinHeaderData) -> HashSet<i32> {
        let mut enabled = HashSet::new();
        for option in &header.custom_options {
            enabled.insert(option.selected_option);
        }
        enabled
    }

    pub(crate) fn load_json_skin(
        &mut self,
        header: &SkinHeaderData,
        sk: &json_skin::Skin,
        skin_type: &crate::skin_type::SkinType,
        _property: &SkinConfigProperty,
        p: &Path,
    ) -> Option<SkinData> {
        // Determine source resolution
        let _src = Resolution {
            width: sk.w as f32,
            height: sk.h as f32,
        };

        self.source_map.clear();
        self.bitmap_source_map.clear();

        // Populate source map
        for source in &sk.source {
            if let Some(ref id) = source.id {
                self.source_map.insert(
                    id.clone(),
                    SourceData::new(source.path.clone().unwrap_or_default()),
                );
            }
        }

        let mut skin = Self::get_skin_for_type(skin_type, header);
        skin.fadeout = sk.fadeout;
        skin.input = sk.input;
        skin.scene = sk.scene;

        // Process destinations
        for dst in &sk.destination {
            // Try to parse dst.id as negative integer for SkinImage(-id)
            let mut obj: Option<SkinObjectData> = None;
            if let Some(ref id) = dst.id
                && let Ok(id_num) = id.parse::<i32>()
                && id_num < 0
            {
                obj = Some(SkinObjectData::new_image_by_id(-id_num));
            }

            if obj.is_none() {
                // Delegate to screen-specific object loader
                obj = self.load_skin_object_for_type(skin_type, &skin, sk, dst, p);
            }

            if let Some(mut o) = obj {
                o.name = dst.id.clone();
                self.set_destination(&mut skin, &mut o, dst);
                skin.objects.push(o);
            }
        }

        // Process skinSelect
        if let Some(ref skin_select) = sk.skin_select {
            skin.custom_offset_style = skin_select.custom_offset_style;
            skin.default_skin_type = skin_select.default_category;
            skin.sample_bms = skin_select.custom_bms.clone();
            if skin_select.custom_property_count > 0 {
                skin.custom_property_count = skin_select.custom_property_count;
            } else {
                let mut count = 0;
                for image in &sk.image {
                    if let Some(act) = image.act
                        && is_skin_customize_button(act)
                    {
                        let index = get_skin_customize_index(act);
                        if count <= index {
                            count = index + 1;
                        }
                    }
                }
                for image_set in &sk.imageset {
                    if let Some(act) = image_set.act
                        && is_skin_customize_button(act)
                    {
                        let index = get_skin_customize_index(act);
                        if count <= index {
                            count = index + 1;
                        }
                    }
                }
                skin.custom_property_count = count;
            }
        }

        // Process custom events
        for event in &sk.custom_events {
            skin.custom_events.push(CustomEventData {
                id: event.id,
                action: event.action,
                condition: event.condition,
                min_interval: event.min_interval,
            });
        }

        // Process custom timers
        for timer in &sk.custom_timers {
            skin.custom_timers.push(CustomTimerData {
                id: timer.id,
                timer: timer.timer,
            });
        }

        Some(skin)
    }

    /// Dispatches `get_skin()` to the appropriate screen-specific object loader.
    /// Corresponds to Java: `objectLoader.getSkin(header)`.
    fn get_skin_for_type(
        skin_type: &crate::skin_type::SkinType,
        header: &SkinHeaderData,
    ) -> SkinData {
        use crate::json::json_skin_object_loader::JsonSkinObjectLoader;
        use crate::skin_type::SkinType;

        match skin_type {
            SkinType::Play7Keys
            | SkinType::Play5Keys
            | SkinType::Play14Keys
            | SkinType::Play10Keys
            | SkinType::Play9Keys
            | SkinType::Play24Keys
            | SkinType::Play24KeysDouble => JsonPlaySkinObjectLoader.get_skin(header),
            SkinType::MusicSelect => JsonSelectSkinObjectLoader.get_skin(header),
            SkinType::Decide => JsonDecideSkinObjectLoader.get_skin(header),
            SkinType::Result => JsonResultSkinObjectLoader.get_skin(header),
            SkinType::CourseResult => JsonCourseResultSkinObjectLoader.get_skin(header),
            SkinType::SkinSelect => JsonSkinConfigurationSkinObjectLoader.get_skin(header),
            _ => JsonKeyConfigurationSkinObjectLoader.get_skin(header),
        }
    }

    fn load_skin_object_for_type(
        &mut self,
        skin_type: &crate::skin_type::SkinType,
        skin: &SkinData,
        sk: &json_skin::Skin,
        dst: &json_skin::Destination,
        p: &Path,
    ) -> Option<SkinObjectData> {
        use crate::json::json_skin_object_loader::JsonSkinObjectLoader;
        use crate::skin_type::SkinType;

        match skin_type {
            // Java: PLAY_5KEYS, PLAY_7KEYS, PLAY_9KEYS, PLAY_10KEYS, PLAY_14KEYS,
            //       PLAY_24KEYS, PLAY_24KEYS_DOUBLE
            SkinType::Play7Keys
            | SkinType::Play5Keys
            | SkinType::Play14Keys
            | SkinType::Play10Keys
            | SkinType::Play9Keys
            | SkinType::Play24Keys
            | SkinType::Play24KeysDouble => {
                let loader_impl =
                    crate::json::json_play_skin_object_loader::JsonPlaySkinObjectLoader;
                loader_impl.load_skin_object(self, skin, sk, dst, p)
            }
            // Java: MUSIC_SELECT
            SkinType::MusicSelect => {
                let loader_impl =
                    crate::json::json_select_skin_object_loader::JsonSelectSkinObjectLoader;
                loader_impl.load_skin_object(self, skin, sk, dst, p)
            }
            // Java: DECIDE
            SkinType::Decide => {
                let loader_impl =
                    crate::json::json_decide_skin_object_loader::JsonDecideSkinObjectLoader;
                loader_impl.load_skin_object(self, skin, sk, dst, p)
            }
            // Java: RESULT
            SkinType::Result => {
                let loader_impl =
                    crate::json::json_result_skin_object_loader::JsonResultSkinObjectLoader;
                loader_impl.load_skin_object(self, skin, sk, dst, p)
            }
            // Java: COURSE_RESULT
            SkinType::CourseResult => {
                let loader_impl = crate::json::json_course_result_skin_object_loader::JsonCourseResultSkinObjectLoader;
                loader_impl.load_skin_object(self, skin, sk, dst, p)
            }
            // Java: SKIN_SELECT
            SkinType::SkinSelect => {
                let loader_impl = crate::json::json_skin_configuration_skin_object_loader::JsonSkinConfigurationSkinObjectLoader;
                loader_impl.load_skin_object(self, skin, sk, dst, p)
            }
            // Java: KEY_CONFIG + default
            _ => {
                let loader_impl = crate::json::json_key_configuration_skin_object_loader::JsonKeyConfigurationSkinObjectLoader;
                loader_impl.load_skin_object(self, skin, sk, dst, p)
            }
        }
    }

    pub(crate) fn set_destination(
        &self,
        _skin: &mut SkinData,
        obj: &mut SkinObjectData,
        dst: &json_skin::Destination,
    ) {
        let mut prev: Option<json_skin::Animation> = None;
        for a_orig in &dst.dst {
            let mut a = a_orig.clone();
            if let Some(ref p) = prev {
                a.time = if a.time == i32::MIN { p.time } else { a.time };
                a.x = if a.x == i32::MIN { p.x } else { a.x };
                a.y = if a.y == i32::MIN { p.y } else { a.y };
                a.w = if a.w == i32::MIN { p.w } else { a.w };
                a.h = if a.h == i32::MIN { p.h } else { a.h };
                a.acc = if a.acc == i32::MIN { p.acc } else { a.acc };
                a.angle = if a.angle == i32::MIN {
                    p.angle
                } else {
                    a.angle
                };
                a.a = if a.a == i32::MIN { p.a } else { a.a };
                a.r = if a.r == i32::MIN { p.r } else { a.r };
                a.g = if a.g == i32::MIN { p.g } else { a.g };
                a.b = if a.b == i32::MIN { p.b } else { a.b };
            } else {
                a.time = if a.time == i32::MIN { 0 } else { a.time };
                a.x = if a.x == i32::MIN { 0 } else { a.x };
                a.y = if a.y == i32::MIN { 0 } else { a.y };
                a.w = if a.w == i32::MIN { 0 } else { a.w };
                a.h = if a.h == i32::MIN { 0 } else { a.h };
                a.acc = if a.acc == i32::MIN { 0 } else { a.acc };
                a.angle = if a.angle == i32::MIN { 0 } else { a.angle };
                a.a = if a.a == i32::MIN { 255 } else { a.a };
                a.r = if a.r == i32::MIN { 255 } else { a.r };
                a.g = if a.g == i32::MIN { 255 } else { a.g };
                a.b = if a.b == i32::MIN { 255 } else { a.b };
            }

            obj.destinations.push(DestinationData {
                time: a.time,
                x: a.x,
                y: a.y,
                w: a.w,
                h: a.h,
                acc: a.acc,
                a: a.a,
                r: a.r,
                g: a.g,
                b: a.b,
                blend: dst.blend,
                filter: dst.filter,
                angle: a.angle,
                center: dst.center,
                loop_val: dst.loop_val,
                timer: dst.timer,
                op: dst.op.clone(),
                draw: dst.draw,
            });

            if let Some(ref mouse_rect) = dst.mouse_rect {
                obj.mouse_rect = Some(RectData {
                    x: mouse_rect.x,
                    y: mouse_rect.y,
                    w: mouse_rect.w,
                    h: mouse_rect.h,
                });
            }

            prev = Some(a);
        }

        // Set offsets
        let mut offsets: Vec<i32> = Vec::with_capacity(dst.offsets.len() + 1);
        for o in &dst.offsets {
            offsets.push(*o);
        }
        offsets.push(dst.offset);
        obj.offset_ids = offsets;

        if dst.stretch >= 0 {
            obj.stretch = dst.stretch;
        }
    }

    pub fn get_source(&mut self, srcid: &str, p: &Path) -> Option<SourceDataType> {
        // Check if already loaded
        if let Some(data) = self.source_map.get(srcid) {
            if data.loaded {
                return data.data.clone();
            }
        } else {
            return None;
        }

        // Extract path before mutable borrow
        let data_path = self.source_map.get(srcid).unwrap().path.clone();
        let parent = p
            .parent()
            .map(|pp| pp.to_string_lossy().to_string())
            .unwrap_or_default();
        let image_path = format!("{}/{}", parent, data_path);
        let image_file = get_path_with_filemap(&image_path, &self.filemap);

        let mut result_data: Option<SourceDataType> = None;

        if std::path::Path::new(&image_file).exists() {
            let lower = image_file.to_lowercase();
            let is_movie = MOV_EXTENSIONS.iter().any(|ext| lower.ends_with(ext));

            if is_movie {
                result_data = Some(SourceDataType::Movie(SkinSourceMovie { path: image_file }));
            } else {
                result_data = Some(SourceDataType::Texture(Texture::new(&image_file)));
            }
        }

        // Now do the mutable borrow
        if let Some(data) = self.source_map.get_mut(srcid) {
            data.data = result_data.clone();
            data.loaded = true;
        }

        result_data
    }

    fn get_path(&self, path: &str) -> String {
        get_path_with_filemap(path, &self.filemap)
    }

    /// Get texture for a path, using usecim setting.
    /// Corresponds to Java JSONSkinLoader.getTexture(String path) which delegates to
    /// SkinLoader.getTexture(path, usecim).
    pub fn get_texture(&self, path: &str) -> Option<Texture> {
        if std::path::Path::new(path).exists() {
            Some(Texture::new(path))
        } else {
            None
        }
    }
}

pub(crate) fn get_path_with_filemap(path: &str, filemap: &HashMap<String, String>) -> String {
    for (key, value) in filemap {
        if path.contains(key.as_str()) {
            return path.replace(key.as_str(), value.as_str());
        }
    }
    path.to_string()
}

// SkinProperty constants
const OFFSET_ALL: i32 = 900;
const OFFSET_NOTES_1P: i32 = 901;
const OFFSET_JUDGE_1P: i32 = 902;
const OFFSET_JUDGEDETAIL_1P: i32 = 903;

const HEADER_TYPE_BEATORJASKIN: i32 = crate::skin_header::TYPE_BEATORJASKIN;

const MOV_EXTENSIONS: &[&str] = &[".mpg", ".mpeg", ".avi", ".wmv", ".mp4", ".m4v", ".webm"];

fn is_skin_customize_button(event_id: i32) -> bool {
    crate::skin_property_mapper::is_skin_customize_button(event_id)
}

fn get_skin_customize_index(event_id: i32) -> i32 {
    crate::skin_property_mapper::get_skin_customize_index(event_id)
}

// Data types for skin loading results (replacing actual skin objects for now)

#[derive(Clone, Debug, Default)]
pub struct SkinHeaderData {
    pub skin_type: i32,
    pub name: String,
    pub author: String,
    pub path: PathBuf,
    pub header_type: i32,
    pub custom_options: Vec<CustomOptionData>,
    pub custom_files: Vec<CustomFileData>,
    pub custom_offsets: Vec<CustomOffsetData>,
    pub custom_categories: Vec<CustomCategoryData>,
    pub source_resolution: Option<Resolution>,
    pub destination_resolution: Option<Resolution>,
}

impl SkinHeaderData {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct CustomOptionData {
    pub name: String,
    pub option: Vec<i32>,
    pub names: Vec<String>,
    pub def: Option<String>,
    pub selected_option: i32,
}

#[derive(Clone, Debug, Default)]
pub struct CustomFileData {
    pub name: String,
    pub path: String,
    pub def: Option<String>,
    pub selected_filename: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct CustomOffsetData {
    pub name: String,
    pub id: i32,
    pub x: bool,
    pub y: bool,
    pub w: bool,
    pub h: bool,
    pub r: bool,
    pub a: bool,
}

#[derive(Clone, Debug)]
pub enum CustomItemData {
    Option(CustomOptionData),
    File(CustomFileData),
    Offset(CustomOffsetData),
}

#[derive(Clone, Debug, Default)]
pub struct CustomCategoryData {
    pub name: String,
    pub items: Vec<CustomItemData>,
}

#[derive(Clone, Debug, Default)]
pub struct SkinConfigProperty;

#[derive(Clone, Debug, Default)]
pub struct SkinData {
    /// Which skin type this data represents (Play, Result, Select, etc.).
    /// Corresponds to Java's PlaySkin / MusicResultSkin / etc. subclass identity.
    pub skin_type: Option<crate::skin_type::SkinType>,
    /// Header information used to construct this skin.
    pub header: Option<SkinHeaderData>,
    pub fadeout: i32,
    pub input: i32,
    pub scene: i32,
    pub objects: Vec<SkinObjectData>,
    pub custom_events: Vec<CustomEventData>,
    pub custom_timers: Vec<CustomTimerData>,
    pub custom_offset_style: i32,
    pub default_skin_type: i32,
    pub sample_bms: Option<Vec<String>>,
    pub custom_property_count: i32,
}

impl SkinData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a SkinData initialized from a header, matching Java's `new XxxSkin(header)`.
    ///
    /// The Java `Skin(SkinHeader)` constructor stores the header and computes
    /// resolution scaling (dw, dh). In Rust, resolution scaling is deferred to
    /// `convert_skin_data`, but the header and skin type are stored here.
    pub fn from_header(header: &SkinHeaderData, skin_type: crate::skin_type::SkinType) -> Self {
        Self {
            skin_type: Some(skin_type),
            header: Some(header.clone()),
            ..Self::default()
        }
    }
}

/// Discriminant for the type of skin object represented by SkinObjectData.
/// Each variant captures the parameters that the rendering pipeline needs.
#[derive(Clone, Debug, Default)]
pub enum SkinObjectType {
    /// Default/unknown
    #[default]
    Unknown,
    /// Negative-ID SkinImage (e.g. SkinImage(-id))
    ImageById(i32),
    /// SkinImage from texture source
    Image {
        src: Option<String>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        divx: i32,
        divy: i32,
        timer: Option<i32>,
        cycle: i32,
        len: i32,
        ref_id: i32,
        act: Option<i32>,
        click: i32,
        is_movie: bool,
    },
    /// SkinImage from image set
    ImageSet {
        images: Vec<String>,
        ref_id: i32,
        value: Option<i32>,
        act: Option<i32>,
        click: i32,
    },
    /// ImageSet with each image resolved to its full Image definition.
    /// Used for bar sub-objects where images must be resolved at conversion time
    /// (bar rendering uses MinimalSkinMainState which can't resolve SkinSourceReference).
    ResolvedImageSet {
        images: Vec<ResolvedImageEntry>,
        ref_id: i32,
    },
    /// SkinNumber
    Number {
        src: Option<String>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        divx: i32,
        divy: i32,
        timer: Option<i32>,
        cycle: i32,
        digit: i32,
        padding: i32,
        zeropadding: i32,
        space: i32,
        ref_id: i32,
        value: Option<i32>,
        align: i32,
        offsets: Option<Vec<SkinNumberOffset>>,
    },
    /// SkinFloat
    Float {
        src: Option<String>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        divx: i32,
        divy: i32,
        timer: Option<i32>,
        cycle: i32,
        iketa: i32,
        fketa: i32,
        is_signvisible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        ref_id: i32,
        value: Option<i32>,
        gain: f32,
        offsets: Option<Vec<SkinNumberOffset>>,
    },
    /// SkinText
    Text {
        font: Option<String>,
        size: i32,
        align: i32,
        ref_id: i32,
        value: Option<i32>,
        constant_text: Option<String>,
        wrapping: bool,
        overflow: i32,
        outline_color: String,
        outline_width: f32,
        shadow_color: String,
        shadow_offset_x: f32,
        shadow_offset_y: f32,
        shadow_smoothness: f32,
    },
    /// SkinSlider
    Slider {
        src: Option<String>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        divx: i32,
        divy: i32,
        timer: Option<i32>,
        cycle: i32,
        angle: i32,
        range: i32,
        slider_type: i32,
        changeable: bool,
        value: Option<i32>,
        event: Option<i32>,
        is_ref_num: bool,
        min: i32,
        max: i32,
    },
    /// SkinGraph
    Graph {
        src: Option<String>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        divx: i32,
        divy: i32,
        timer: Option<i32>,
        cycle: i32,
        angle: i32,
        graph_type: i32,
        value: Option<i32>,
        is_ref_num: bool,
        min: i32,
        max: i32,
    },
    /// SkinDistributionGraph (graph with type < 0)
    DistributionGraph {
        src: Option<String>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        divx: i32,
        divy: i32,
        timer: Option<i32>,
        cycle: i32,
        graph_type: i32,
    },
    /// SkinGaugeGraphObject
    GaugeGraph {
        color: Option<Vec<String>>,
        assist_clear_bg_color: String,
        assist_and_easy_fail_bg_color: String,
        groove_fail_bg_color: String,
        groove_clear_and_hard_bg_color: String,
        ex_hard_bg_color: String,
        hazard_bg_color: String,
        assist_clear_line_color: String,
        assist_and_easy_fail_line_color: String,
        groove_fail_line_color: String,
        groove_clear_and_hard_line_color: String,
        ex_hard_line_color: String,
        hazard_line_color: String,
        borderline_color: String,
        border_color: String,
    },
    /// SkinNoteDistributionGraph
    JudgeGraph {
        graph_type: i32,
        delay: i32,
        back_tex_off: i32,
        order_reverse: i32,
        no_gap: i32,
        no_gap_x: i32,
    },
    /// SkinBPMGraph
    BpmGraph {
        delay: i32,
        line_width: i32,
        main_bpm_color: String,
        min_bpm_color: String,
        max_bpm_color: String,
        other_bpm_color: String,
        stop_line_color: String,
        transition_line_color: String,
    },
    /// SkinHitErrorVisualizer
    HitErrorVisualizer {
        width: i32,
        judge_width_millis: i32,
        line_width: i32,
        color_mode: i32,
        hiterror_mode: i32,
        ema_mode: i32,
        line_color: String,
        center_color: String,
        pg_color: String,
        gr_color: String,
        gd_color: String,
        bd_color: String,
        pr_color: String,
        ema_color: String,
        alpha: f32,
        window_length: i32,
        transparent: i32,
        draw_decay: i32,
    },
    /// SkinTimingVisualizer
    TimingVisualizer {
        width: i32,
        judge_width_millis: i32,
        line_width: i32,
        line_color: String,
        center_color: String,
        pg_color: String,
        gr_color: String,
        gd_color: String,
        bd_color: String,
        pr_color: String,
        transparent: i32,
        draw_decay: i32,
    },
    /// SkinTimingDistributionGraph
    TimingDistributionGraph {
        width: i32,
        line_width: i32,
        graph_color: String,
        average_color: String,
        dev_color: String,
        pg_color: String,
        gr_color: String,
        gd_color: String,
        bd_color: String,
        pr_color: String,
        draw_average: i32,
        draw_dev: i32,
    },
    /// SkinGauge
    Gauge {
        nodes: Vec<String>,
        parts: i32,
        gauge_type: i32,
        range: i32,
        cycle: i32,
        starttime: i32,
        endtime: i32,
    },
    /// SkinNote (play skin only)
    Note,
    /// SkinHidden (hidden cover, play skin only)
    HiddenCover {
        src: Option<String>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        divx: i32,
        divy: i32,
        timer: Option<i32>,
        cycle: i32,
        disapear_line: i32,
        is_disapear_line_link_lift: bool,
    },
    /// SkinHidden (lift cover, play skin only)
    LiftCover {
        src: Option<String>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        divx: i32,
        divy: i32,
        timer: Option<i32>,
        cycle: i32,
        disapear_line: i32,
        is_disapear_line_link_lift: bool,
    },
    /// SkinBGA (play skin only)
    Bga { bga_expand: i32 },
    /// SkinJudge (play skin only)
    Judge { index: i32, shift: bool },
    /// PMchara (play skin only)
    PmChara {
        src: Option<String>,
        color: i32,
        chara_type: i32,
        side: i32,
    },
    /// SkinBar (select skin only)
    SongList {
        center: i32,
        clickable: Vec<i32>,
        /// Resolved bar sub-objects (images, text, levels, lamps, etc.)
        /// Populated by JsonSelectSkinObjectLoader, consumed by skin_data_converter.
        bar_data: Option<Box<SongListBarData>>,
    },
    /// Search text region (select skin only)
    SearchTextRegion { x: f32, y: f32, w: f32, h: f32 },
}

/// A single resolved image from an ImageSet (carries the full src/region info).
#[derive(Clone, Debug, Default)]
pub struct ResolvedImageEntry {
    pub src: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub divx: i32,
    pub divy: i32,
}

/// Resolved bar sub-object data for JSON skin SongList.
/// Each vec has one entry per bar slot. The converter extracts these into SelectBarData.
#[derive(Clone, Debug, Default)]
pub struct SongListBarData {
    pub listoff: Vec<Option<SkinObjectData>>,
    pub liston: Vec<Option<SkinObjectData>>,
    pub text: Vec<Option<SkinObjectData>>,
    pub level: Vec<Option<SkinObjectData>>,
    pub lamp: Vec<Option<SkinObjectData>>,
    pub playerlamp: Vec<Option<SkinObjectData>>,
    pub rivallamp: Vec<Option<SkinObjectData>>,
    pub trophy: Vec<Option<SkinObjectData>>,
    pub label: Vec<Option<SkinObjectData>>,
}

/// Offset data for SkinNumber/SkinFloat per-digit offsets
#[derive(Clone, Debug, Default)]
pub struct SkinNumberOffset {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Debug, Default)]
pub struct SkinObjectData {
    pub name: Option<String>,
    pub object_type: SkinObjectType,
    pub destinations: Vec<DestinationData>,
    pub offset_ids: Vec<i32>,
    pub stretch: i32,
    pub mouse_rect: Option<RectData>,
}

impl SkinObjectData {
    pub fn new_image_by_id(id: i32) -> Self {
        Self {
            name: Some(format!("{}", -id)),
            object_type: SkinObjectType::ImageById(id),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DestinationData {
    pub time: i32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub acc: i32,
    pub a: i32,
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub blend: i32,
    pub filter: i32,
    pub angle: i32,
    pub center: i32,
    pub loop_val: i32,
    pub timer: Option<i32>,
    pub op: Vec<i32>,
    pub draw: Option<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct RectData {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Debug, Default)]
pub struct CustomEventData {
    pub id: i32,
    pub action: Option<i32>,
    pub condition: Option<i32>,
    pub min_interval: i32,
}

#[derive(Clone, Debug, Default)]
pub struct CustomTimerData {
    pub id: i32,
    pub timer: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid skin JSON that parses successfully via parse_skin_json.
    const MINIMAL_SKIN_JSON: &str = r#"{"type":5,"name":"test","w":1920,"h":1080}"#;

    /// Skin JSON with a line comment — valid in Gson but rejected by serde_json.
    const SKIN_WITH_COMMENT: &str = r#"{
        // This is a line comment
        "type": 5,
        "name": "test"
    }"#;

    /// Skin JSON with `}{` (close-open) inside a string value, which fix_lenient_json corrupts.
    const SKIN_WITH_BRACES_IN_STRING: &str = r#"{"type":5,"name":"a}{b"}"#;

    #[test]
    fn test_fix_lenient_json_trailing_comma() {
        let input = r#"[1, 2, 3,]"#;
        let fixed = fix_lenient_json(input);
        assert_eq!(fixed, "[1, 2, 3]");
    }

    #[test]
    fn test_fix_lenient_json_missing_comma() {
        let input = "[\n\t{\"a\":1}\n\t{\"b\":2}\n]";
        let fixed = fix_lenient_json(input);
        let parsed: serde_json::Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_parse_skin_json_numeric_id_and_src() {
        let input =
            r#"{"type":5,"source":[{"id":0,"path":"a.png"}],"image":[{"id":"bg","src":1}]}"#;
        let skin = parse_skin_json(input).unwrap();
        assert_eq!(skin.source[0].id, Some("0".to_string()));
        assert_eq!(skin.image[0].src, Some("1".to_string()));
    }

    #[test]
    fn test_parse_default_select_skin() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("skin/default/select.json");
        if !path.exists() {
            // Skip if skin file not present in CI
            return;
        }
        let content = std::fs::read_to_string(&path).unwrap();
        let skin = parse_skin_json(&content);
        assert!(
            skin.is_ok(),
            "Failed to parse select.json: {:?}",
            skin.err()
        );
        let skin = skin.unwrap();
        assert_eq!(skin.skin_type, 5);
        assert_eq!(skin.name, Some("beatoraja default".to_string()));
        assert!(!skin.source.is_empty());
        assert!(!skin.image.is_empty());
    }

    #[test]
    fn test_parse_default_play24_skin() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("skin/default/play24.json");
        if !path.exists() {
            return;
        }
        let content = std::fs::read_to_string(&path).unwrap();
        let skin = parse_skin_json(&content).unwrap();
        assert_eq!(skin.skin_type, 16);
        assert!(!skin.destination.is_empty());
        assert!(!skin.text.is_empty());
    }

    // ---- Phase 45b: verify MINIMAL_SKIN_JSON round-trips ----

    #[test]
    fn test_minimal_skin_json_parses() {
        let skin = parse_skin_json(MINIMAL_SKIN_JSON).unwrap();
        assert_eq!(skin.skin_type, 5);
        assert_eq!(skin.name, Some("test".to_string()));
        assert_eq!(skin.w, 1920);
        assert_eq!(skin.h, 1080);
    }

    // ---- Line comments are now stripped by fix_lenient_json ----

    #[test]
    fn test_line_comment_stripped() {
        let skin = parse_skin_json(SKIN_WITH_COMMENT).unwrap();
        assert_eq!(skin.skin_type, 5);
        assert_eq!(skin.name, Some("test".to_string()));
    }

    // ---- Block comments are now stripped by fix_lenient_json ----

    #[test]
    fn test_block_comment_stripped() {
        let input = r#"{ /* block comment */ "type": 5, "name": "test" }"#;
        let skin = parse_skin_json(input).unwrap();
        assert_eq!(skin.skin_type, 5);
        assert_eq!(skin.name, Some("test".to_string()));
    }

    // ---- BOM handling ----

    #[test]
    fn test_bom_prefix_stripped() {
        let input = format!("\u{FEFF}{}", MINIMAL_SKIN_JSON);
        let skin = parse_skin_json(&input).unwrap();
        assert_eq!(skin.skin_type, 5);
        assert_eq!(skin.name, Some("test".to_string()));
    }

    // ---- Comment stripping: string-safety ----

    #[test]
    fn test_comments_inside_string_not_stripped() {
        // `//` and `/* */` inside a JSON string value must be preserved
        let input = r#"{"type":5,"name":"a // b /* c */ d","w":1920,"h":1080}"#;
        let skin = parse_skin_json(input).unwrap();
        assert_eq!(skin.name, Some("a // b /* c */ d".to_string()));
    }

    #[test]
    fn test_nested_block_comment_edge_case() {
        // Nested `/*` inside a block comment: the first `*/` ends the comment
        // (same behavior as Java/Gson)
        let input = r#"{ /* outer /* inner */ "type": 5, "name": "test" }"#;
        let skin = parse_skin_json(input).unwrap();
        assert_eq!(skin.skin_type, 5);
        assert_eq!(skin.name, Some("test".to_string()));
    }

    // ---- Phase 48c fix: fix_lenient_json preserves `}{` inside string literals ----
    // The string-aware state machine skips braces inside quoted strings.

    #[test]
    fn test_fix_lenient_json_preserves_braces_in_strings() {
        let input = r#"{"path":"a}{b"}"#;
        let fixed = fix_lenient_json(input);
        assert_eq!(
            fixed, input,
            "fix_lenient_json must not modify braces inside string literals"
        );
    }

    // ---- Phase 48d: M3 — numeric `path` not coerced to string ----
    // The coercion whitelist is `id | src | font`; `path` is not included.
    // Gson coerces ALL numeric values when the target field is String, but
    // our coercion only handles the whitelisted keys.

    #[test]
    fn test_numeric_path_not_coerced() {
        let input = r#"{"type":5,"source":[{"id":"0","path":42}]}"#;
        let cleaned = fix_lenient_json(input);
        let mut value: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        coerce_json_numbers_to_strings(&mut value);

        let path_val = &value["source"][0]["path"];
        assert!(
            path_val.is_number(),
            "path should remain numeric (not in coercion whitelist), got: {}",
            path_val
        );
        assert_eq!(path_val.as_i64(), Some(42));
    }

    // ---- get_skin_for_type dispatch tests ----

    #[test]
    fn test_get_skin_for_type_play7keys() {
        use crate::skin_type::SkinType;
        let header = SkinHeaderData {
            skin_type: SkinType::Play7Keys.id(),
            ..Default::default()
        };
        let skin = JSONSkinLoader::get_skin_for_type(&SkinType::Play7Keys, &header);
        assert_eq!(skin.skin_type, Some(SkinType::Play7Keys));
        assert!(skin.header.is_some());
    }

    #[test]
    fn test_get_skin_for_type_music_select() {
        use crate::skin_type::SkinType;
        let header = SkinHeaderData {
            skin_type: SkinType::MusicSelect.id(),
            ..Default::default()
        };
        let skin = JSONSkinLoader::get_skin_for_type(&SkinType::MusicSelect, &header);
        assert_eq!(skin.skin_type, Some(SkinType::MusicSelect));
    }

    #[test]
    fn test_get_skin_for_type_decide() {
        use crate::skin_type::SkinType;
        let header = SkinHeaderData::default();
        let skin = JSONSkinLoader::get_skin_for_type(&SkinType::Decide, &header);
        assert_eq!(skin.skin_type, Some(SkinType::Decide));
    }

    #[test]
    fn test_get_skin_for_type_result() {
        use crate::skin_type::SkinType;
        let header = SkinHeaderData::default();
        let skin = JSONSkinLoader::get_skin_for_type(&SkinType::Result, &header);
        assert_eq!(skin.skin_type, Some(SkinType::Result));
    }

    #[test]
    fn test_get_skin_for_type_course_result() {
        use crate::skin_type::SkinType;
        let header = SkinHeaderData::default();
        let skin = JSONSkinLoader::get_skin_for_type(&SkinType::CourseResult, &header);
        assert_eq!(skin.skin_type, Some(SkinType::CourseResult));
    }

    #[test]
    fn test_get_skin_for_type_skin_select() {
        use crate::skin_type::SkinType;
        let header = SkinHeaderData::default();
        let skin = JSONSkinLoader::get_skin_for_type(&SkinType::SkinSelect, &header);
        assert_eq!(skin.skin_type, Some(SkinType::SkinSelect));
    }

    #[test]
    fn test_get_skin_for_type_key_config_default() {
        use crate::skin_type::SkinType;
        let header = SkinHeaderData::default();
        // SoundSet falls through to KeyConfig default branch
        let skin = JSONSkinLoader::get_skin_for_type(&SkinType::SoundSet, &header);
        assert_eq!(skin.skin_type, Some(SkinType::KeyConfig));
    }

    #[test]
    fn test_skin_data_from_header_stores_header() {
        use crate::skin_type::SkinType;
        let header = SkinHeaderData {
            skin_type: SkinType::Play7Keys.id(),
            name: "My Skin".to_string(),
            author: "Author".to_string(),
            ..Default::default()
        };
        let skin = SkinData::from_header(&header, SkinType::Play7Keys);
        assert_eq!(skin.skin_type, Some(SkinType::Play7Keys));
        let stored = skin.header.unwrap();
        assert_eq!(stored.name, "My Skin");
        assert_eq!(stored.author, "Author");
    }

    #[test]
    fn test_skin_data_new_has_none_skin_type() {
        let skin = SkinData::new();
        assert_eq!(skin.skin_type, None);
        assert!(skin.header.is_none());
    }

    #[test]
    fn test_is_skin_customize_button_in_range() {
        // BUTTON_SKIN_CUSTOMIZE1 = 220, BUTTON_SKIN_CUSTOMIZE10 = 229
        // Range is [220, 229) — 220..228 are customize buttons
        assert!(is_skin_customize_button(220)); // CUSTOMIZE1
        assert!(is_skin_customize_button(224)); // CUSTOMIZE5
        assert!(is_skin_customize_button(228)); // CUSTOMIZE9
    }

    #[test]
    fn test_is_skin_customize_button_out_of_range() {
        assert!(!is_skin_customize_button(219)); // below range
        assert!(!is_skin_customize_button(229)); // CUSTOMIZE10 is the exclusive upper bound
        assert!(!is_skin_customize_button(230)); // above range
        assert!(!is_skin_customize_button(0));
        assert!(!is_skin_customize_button(-1));
    }

    #[test]
    fn test_get_skin_customize_index() {
        // Index is relative to BUTTON_SKIN_CUSTOMIZE1 (220)
        assert_eq!(get_skin_customize_index(220), 0);
        assert_eq!(get_skin_customize_index(221), 1);
        assert_eq!(get_skin_customize_index(228), 8);
    }
}

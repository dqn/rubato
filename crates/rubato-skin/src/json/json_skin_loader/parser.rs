use std::path::PathBuf;

use crate::json::json_skin;
use crate::stubs::Resolution;

pub(crate) fn parse_skin_json(content: &str) -> Result<json_skin::Skin, serde_json::Error> {
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
pub(super) fn fix_lenient_json(json: &str) -> String {
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
pub(super) fn coerce_json_numbers_to_strings(value: &mut serde_json::Value) {
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
    pub caps: rubato_types::offset_capabilities::OffsetCapabilities,
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
#[derive(Clone, Copy, Debug, Default)]
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

// LR2 CSV skin loader.
//
// Loads LR2-format CSV skin files (.lr2skin) and converts them into
// the Skin data model. Handles:
// - SRC_*/DST_* command pairs for object creation and placement
// - #IF/#ELSEIF/#ELSE/#ENDIF conditional blocks
// - Bottom-left Y origin → top-left origin coordinate transform
// - STARTINPUT, SCENETIME, FADEOUT, STRETCH global properties
//
// State-specific commands (SRC_NOTE, SRC_JUDGE, etc.) are not implemented
// in this phase — they require types defined in later phases.
//
// Ported from LR2SkinCSVLoader.java and LR2SkinLoader.java.

use std::collections::HashMap;

use anyhow::Result;

use bms_config::resolution::Resolution;
use bms_config::skin_type::SkinType;

use crate::image_handle::ImageHandle;
use crate::loader::lr2_play_loader::{self, Lr2PlayState};
use crate::loader::lr2_result_loader::{self, Lr2ResultState};
use crate::loader::lr2_select_loader::{self, Lr2SelectState};
use crate::property_id::{BooleanId, FloatId, IntegerId, StringId, TimerId};
use crate::skin::Skin;
use crate::skin_gauge::{GaugePart, GaugePartType, SkinGauge};
use crate::skin_graph::{GraphDirection, SkinGraph};
use crate::skin_header::SkinHeader;
use crate::skin_image::SkinImage;
use crate::skin_number::{NumberAlign, SkinNumber, ZeroPadding};
use crate::skin_object::{Color, Destination, Rect, SkinObjectBase};
use crate::skin_slider::{SkinSlider, SliderDirection};
use crate::skin_source::{build_number_source_set, split_grid};
use crate::skin_text::{SkinText, TextAlign};
use crate::stretch_type::StretchType;

// ---------------------------------------------------------------------------
// CSV parsing utilities (pub for use by lr2_header_loader)
// ---------------------------------------------------------------------------

/// Parses a CSV field as i32. Returns 0 on failure.
/// Handles LR2 convention: '!' → '-' for negative numbers, strips spaces.
pub fn parse_field(fields: &[&str], index: usize) -> i32 {
    fields
        .get(index)
        .map(|s| {
            let cleaned: String = s
                .replace('!', "-")
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '-')
                .collect();
            cleaned.parse::<i32>().unwrap_or(0)
        })
        .unwrap_or(0)
}

/// Parses CSV fields into a 22-element i32 array (index 0 is unused).
/// Matches Java's `parseInt(String[])` method.
fn parse_int(fields: &[&str]) -> [i32; 22] {
    let mut result = [0i32; 22];
    for (i, item) in result
        .iter_mut()
        .enumerate()
        .take(22.min(fields.len()))
        .skip(1)
    {
        *item = parse_field(fields, i);
    }
    result
}

/// Reads offset IDs from CSV fields starting at the given index.
/// Matches Java's `readOffset(String[], int)` method.
pub(crate) fn read_offset(fields: &[&str], start: usize) -> Vec<i32> {
    let mut result = Vec::new();
    for field in fields.iter().skip(start) {
        let cleaned: String = field
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        if !cleaned.is_empty()
            && let Ok(v) = cleaned.parse::<i32>()
        {
            result.push(v);
        }
    }
    result
}

/// Processes #IF / #ELSEIF / #ELSE / #ENDIF conditional commands.
///
/// Returns true if the command was a conditional directive (caller should skip
/// further processing of this line).
///
/// Matches Java's `LR2SkinLoader.processLine()` conditional logic.
pub fn process_conditional(
    cmd: &str,
    fields: &[&str],
    options: &HashMap<i32, i32>,
    skip: &mut bool,
    found_true: &mut bool,
) -> bool {
    match cmd {
        "IF" => {
            *found_true = true;
            for field in fields.iter().skip(1) {
                if field.is_empty() {
                    continue;
                }
                let cleaned: String = field
                    .replace('!', "-")
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '-')
                    .collect();
                if let Ok(opt) = cleaned.parse::<i32>() {
                    let ok = if opt >= 0 {
                        options.get(&opt).copied() == Some(1)
                    } else {
                        options.get(&(-opt)).copied() == Some(0)
                    };
                    if !ok {
                        *found_true = false;
                        break;
                    }
                }
            }
            *skip = !*found_true;
            true
        }
        "ELSEIF" => {
            if *found_true {
                *skip = true;
            } else {
                *found_true = true;
                for field in fields.iter().skip(1) {
                    let cleaned: String = field
                        .replace('!', "-")
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '-')
                        .collect();
                    if let Ok(opt) = cleaned.parse::<i32>() {
                        let ok = if opt >= 0 {
                            options.get(&opt).copied() == Some(1)
                        } else {
                            options.get(&(-opt)).copied() == Some(0)
                        };
                        if !ok {
                            *found_true = false;
                            break;
                        }
                    }
                }
                *skip = !*found_true;
            }
            true
        }
        "ELSE" => {
            *skip = *found_true;
            true
        }
        "ENDIF" => {
            *skip = false;
            *found_true = false;
            true
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Loader state
// ---------------------------------------------------------------------------

/// Current object slot being built (set by SRC, used by DST).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ObjectSlot {
    Image,
    Number,
    Text,
    Slider,
    Graph,
    Button,
    Gauge,
}

/// Loader state shared between the main loader and state-specific sub-loaders.
pub struct Lr2CsvState {
    /// Current object indices in skin.objects
    current: HashMap<ObjectSlot, usize>,
    /// Source resolution height (for Y-flip)
    pub(crate) srch: f32,
    /// Source resolution width
    pub(crate) srcw: f32,
    /// Destination resolution width
    pub(crate) dstw: f32,
    /// Destination resolution height
    pub(crate) dsth: f32,
    /// Current stretch mode (-1 = unset)
    pub(crate) stretch: i32,
    /// Condition state
    skip: bool,
    found_true: bool,
    /// Option map (id -> 0/1)
    options: HashMap<i32, i32>,
    /// SRC_GROOVEGAUGE add_x (node spacing X).
    groovex: i32,
    /// SRC_GROOVEGAUGE add_y (node spacing Y).
    groovey: i32,
}

impl Lr2CsvState {
    /// Creates a new Lr2CsvState for the given resolutions.
    pub fn new(src: Resolution, dst: Resolution, options: &HashMap<i32, i32>) -> Self {
        Self {
            current: HashMap::new(),
            srch: src.height() as f32,
            srcw: src.width() as f32,
            dstw: dst.width() as f32,
            dsth: dst.height() as f32,
            stretch: -1,
            skip: false,
            found_true: false,
            options: options.clone(),
            groovex: 0,
            groovey: 0,
        }
    }

    /// Applies a DST command to a specific object index.
    ///
    /// Used by state-specific loaders to position their objects.
    pub fn apply_dst_to(&self, idx: usize, fields: &[&str], skin: &mut Skin) {
        let base: &mut SkinObjectBase = skin.objects[idx].base_mut();
        self.apply_dst_to_base(base, fields, &[]);
    }

    /// Applies a DST command directly to a `SkinObjectBase`.
    ///
    /// Used by state-specific loaders (e.g., judge, combo) to position sub-objects.
    /// `extra_offsets` are prepended to the field-based offsets, matching Java's
    /// `readOffset(str, 21, defaultOffsets)` pattern.
    pub fn apply_dst_to_base(
        &self,
        base: &mut SkinObjectBase,
        fields: &[&str],
        extra_offsets: &[i32],
    ) {
        let values = parse_int(fields);
        let (mut x, mut y, mut w, mut h) = (values[3], values[4], values[5], values[6]);

        if w < 0 {
            x += w;
            w = -w;
        }
        if h < 0 {
            y += h;
            h = -h;
        }

        let y_flipped = self.srch - (y + h) as f32;

        let time = values[2] as i64;
        let color = Color::from_rgba_u8(
            values[9] as u8,
            values[10] as u8,
            values[11] as u8,
            values[8] as u8,
        );

        base.add_destination(Destination {
            time,
            region: Rect::new(x as f32, y_flipped, w as f32, h as f32),
            color,
            angle: values[14],
            acc: values[7],
        });

        if base.destinations.len() == 1 {
            base.blend = values[12];
            base.filter = values[13];
            base.set_center(values[15]);
            base.loop_time = values[16];

            let timer_id = values[17];
            if timer_id != 0 {
                base.timer = Some(TimerId(timer_id));
            }

            for &op_val in &[values[18], values[19], values[20]] {
                if op_val != 0 {
                    base.draw_conditions.push(BooleanId(op_val));
                }
            }

            let mut offset_ids: Vec<i32> = extra_offsets.to_vec();
            offset_ids.extend(read_offset(fields, 21));
            base.set_offset_ids(&offset_ids);

            if self.stretch >= 0 {
                base.stretch = StretchType::from_id(self.stretch).unwrap_or_default();
            }
        }
    }
}

/// Public wrapper around parse_int for use by sub-loaders.
pub fn parse_int_pub(fields: &[&str]) -> [i32; 22] {
    parse_int(fields)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Loads a full Skin from LR2 CSV content.
///
/// `content` should already be decoded from MS932 to UTF-8.
/// `header` should be loaded via `lr2_header_loader::load_lr2_header()`.
/// `enabled_options` maps option IDs to their enabled state (0/1).
/// `dest_resolution` is the target display resolution.
/// `skin_type` is the skin type (Play, Select, Result, etc.) for state-specific dispatch.
pub fn load_lr2_skin(
    content: &str,
    mut header: SkinHeader,
    enabled_options: &HashMap<i32, i32>,
    dest_resolution: Resolution,
    skin_type: Option<SkinType>,
) -> Result<Skin> {
    header.destination_resolution = Some(dest_resolution);

    let src = header.source_resolution.unwrap_or(header.resolution);

    let mut skin = Skin::new(header);

    // Copy enabled options into the skin
    for (&id, &val) in enabled_options {
        skin.options.insert(id, val);
    }

    let mut state = Lr2CsvState::new(src, dest_resolution, enabled_options);

    // State-specific sub-states
    let mut play_state = Lr2PlayState::default();
    let mut select_state = Lr2SelectState::default();
    let mut result_state = Lr2ResultState::default();

    for line in content.lines() {
        if !line.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = line.split(',').collect();
        if fields.is_empty() {
            continue;
        }

        let cmd = fields[0].trim_start_matches('#').to_uppercase();

        // Handle conditionals
        if process_conditional(
            &cmd,
            &fields,
            &state.options,
            &mut state.skip,
            &mut state.found_true,
        ) {
            continue;
        }

        if state.skip {
            continue;
        }

        // Handle SETOPTION
        if cmd == "SETOPTION" && fields.len() >= 3 {
            let index = parse_field(&fields, 1);
            let value = if parse_field(&fields, 2) >= 1 { 1 } else { 0 };
            state.options.insert(index, value);
            continue;
        }

        process_command(
            &cmd,
            &fields,
            &mut skin,
            &mut state,
            skin_type,
            &mut StateContext {
                play: &mut play_state,
                select: &mut select_state,
                result: &mut result_state,
            },
        );
    }

    // Remove objects with no destinations (matching Java behavior)
    skin.objects.retain(|obj| obj.base().is_valid());

    // Collect state-specific configs
    match skin_type {
        Some(SkinType::Play7Keys)
        | Some(SkinType::Play5Keys)
        | Some(SkinType::Play9Keys)
        | Some(SkinType::Play10Keys)
        | Some(SkinType::Play14Keys)
        | Some(SkinType::Play24Keys)
        | Some(SkinType::Play24KeysDouble)
        | Some(SkinType::Play7KeysBattle)
        | Some(SkinType::Play5KeysBattle)
        | Some(SkinType::Play9KeysBattle)
        | Some(SkinType::Play24KeysBattle) => {
            skin.play_config = lr2_play_loader::collect_play_config(&skin, &play_state);
        }
        Some(SkinType::MusicSelect) => {
            skin.select_config = lr2_select_loader::collect_select_config(&skin, &select_state);
        }
        Some(SkinType::Result) => {
            skin.result_config = lr2_result_loader::collect_result_config(&skin, &result_state);
        }
        Some(SkinType::CourseResult) => {
            skin.course_result_config =
                lr2_result_loader::collect_course_result_config(&skin, &result_state);
        }
        _ => {}
    }

    Ok(skin)
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Groups state-specific loaders to reduce parameter count.
struct StateContext<'a> {
    play: &'a mut Lr2PlayState,
    select: &'a mut Lr2SelectState,
    result: &'a mut Lr2ResultState,
}

fn process_command(
    cmd: &str,
    fields: &[&str],
    skin: &mut Skin,
    state: &mut Lr2CsvState,
    skin_type: Option<SkinType>,
    ctx: &mut StateContext,
) {
    match cmd {
        // Global properties
        "STARTINPUT" => {
            skin.input = parse_field(fields, 1);
            skin.rank_time = parse_field(fields, 2);
        }
        "SCENETIME" => skin.scene = parse_field(fields, 1),
        "FADEOUT" => skin.fadeout = parse_field(fields, 1),
        "STRETCH" => state.stretch = parse_field(fields, 1),

        // SRC commands
        "SRC_IMAGE" => src_image(fields, skin, state),
        "SRC_NUMBER" => src_number(fields, skin, state),
        "SRC_TEXT" => src_text(fields, skin, state),
        "SRC_SLIDER" | "SRC_SLIDER_REFNUMBER" => src_slider(cmd, fields, skin, state),
        "SRC_BARGRAPH" | "SRC_BARGRAPH_REFNUMBER" => src_bargraph(cmd, fields, skin, state),
        "SRC_BUTTON" => src_button(fields, skin, state),
        "SRC_GROOVEGAUGE" => src_groovegauge(fields, skin, state),
        "SRC_GROOVEGAUGE_EX" => src_groovegauge_ex(fields, skin, state),

        // DST commands
        "DST_IMAGE" => apply_dst(ObjectSlot::Image, fields, skin, state),
        "DST_NUMBER" => apply_dst(ObjectSlot::Number, fields, skin, state),
        "DST_TEXT" => apply_dst(ObjectSlot::Text, fields, skin, state),
        "DST_SLIDER" => apply_dst(ObjectSlot::Slider, fields, skin, state),
        "DST_BARGRAPH" => apply_dst_bargraph(fields, skin, state),
        "DST_BUTTON" => apply_dst(ObjectSlot::Button, fields, skin, state),
        "DST_GROOVEGAUGE" => dst_groovegauge(fields, skin, state),

        // State-specific dispatch
        _ => {
            let handled = match skin_type {
                Some(SkinType::Result) => {
                    lr2_result_loader::process_result_command(cmd, fields, skin, state, ctx.result)
                }
                Some(SkinType::CourseResult) => lr2_result_loader::process_course_result_command(
                    cmd, fields, skin, state, ctx.result,
                ),
                Some(SkinType::MusicSelect) => {
                    lr2_select_loader::process_select_command(cmd, fields, skin, state, ctx.select)
                }
                Some(
                    SkinType::Play7Keys
                    | SkinType::Play5Keys
                    | SkinType::Play9Keys
                    | SkinType::Play10Keys
                    | SkinType::Play14Keys
                    | SkinType::Play24Keys
                    | SkinType::Play24KeysDouble
                    | SkinType::Play7KeysBattle
                    | SkinType::Play5KeysBattle
                    | SkinType::Play9KeysBattle
                    | SkinType::Play24KeysBattle,
                ) => lr2_play_loader::process_play_command(cmd, fields, skin, state, ctx.play),
                _ => false,
            };
            let _ = handled; // Suppress unused warning
        }
    }
}

// ---------------------------------------------------------------------------
// SRC handlers — create skin objects
// ---------------------------------------------------------------------------

fn src_image(fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let gr = parse_field(fields, 2);
    let img = if gr >= 100 {
        // Reference image (no actual drawing)
        SkinImage::from_reference(gr)
    } else {
        let values = parse_int(fields);
        // Store timer and cycle from the SRC definition
        SkinImage::from_frames(Vec::new(), nonzero_timer(values[10]), values[9])
    };
    let idx = skin.objects.len();
    skin.add(img.into());
    state.current.insert(ObjectSlot::Image, idx);
}

fn src_number(fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let values = parse_int(fields);
    let gr = values[2];
    let divx = values[7].max(1);
    let divy = values[8].max(1);

    if divx * divy < 10 {
        state.current.remove(&ObjectSlot::Number);
        return;
    }

    // Build grid from image handle
    let handle = ImageHandle(gr as u32);
    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];
    let (digit_sources, minus_digit_sources, zeropadding_override) =
        build_number_source_set(&grid, timer, cycle);

    // Detect 24-frame (12 positive + 12 negative digits) vs 10/11-digit layout
    let total_frames = divx * divy;
    let (keta, zero_padding) = if total_frames % 24 == 0 {
        // 24-frame format
        (
            values[13] + 1,
            ZeroPadding::from_i32(if fields.len() > 14 && !fields[14].is_empty() {
                values[14]
            } else {
                2
            }),
        )
    } else {
        (
            values[13],
            ZeroPadding::from_i32(zeropadding_override.unwrap_or(0)),
        )
    };

    let num = SkinNumber {
        base: SkinObjectBase::default(),
        ref_id: Some(IntegerId(values[11])),
        keta,
        zero_padding,
        align: NumberAlign::from_i32(values[12]),
        space: values[15],
        digit_sources,
        minus_digit_sources,
        image_timer: timer,
        image_cycle: cycle,
        ..Default::default()
    };

    let idx = skin.objects.len();
    skin.add(num.into());
    state.current.insert(ObjectSlot::Number, idx);
}

fn src_text(fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let values = parse_int(fields);

    let text = SkinText {
        base: SkinObjectBase::default(),
        ref_id: Some(StringId(values[3])),
        align: TextAlign::from_i32(values[4]),
        editable: values[5] != 0,
        ..Default::default()
    };

    let idx = skin.objects.len();
    skin.add(text.into());
    state.current.insert(ObjectSlot::Text, idx);
}

fn src_slider(cmd: &str, fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let values = parse_int(fields);
    let direction = SliderDirection::from_i32(values[11]);

    // Scale range based on direction
    let range_scale = if matches!(direction, SliderDirection::Right | SliderDirection::Left) {
        state.dstw / state.srcw
    } else {
        state.dsth / state.srch
    };
    let range = (values[12] as f32 * range_scale) as i32;

    let (range_min, range_max) = if cmd == "SRC_SLIDER_REFNUMBER" {
        (Some(values[15]), Some(values[16]))
    } else {
        (None, None)
    };

    let changeable = if cmd == "SRC_SLIDER_REFNUMBER" {
        true // REFNUMBER sliders are always writable
    } else {
        values[14] == 0
    };

    let slider = SkinSlider {
        base: SkinObjectBase::default(),
        direction,
        range,
        ref_id: Some(FloatId(values[13])),
        changeable,
        range_min,
        range_max,
        ..Default::default()
    };

    let idx = skin.objects.len();
    skin.add(slider.into());
    state.current.insert(ObjectSlot::Slider, idx);
}

fn src_bargraph(cmd: &str, fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let values = parse_int(fields);
    let gr = values[2];

    let graph = if cmd == "SRC_BARGRAPH_REFNUMBER" {
        SkinGraph {
            base: SkinObjectBase::default(),
            ref_id: Some(FloatId(values[11])),
            direction: GraphDirection::from_i32(values[12]),
            source_image_id: if gr >= 100 { Some(gr) } else { None },
            range_min: Some(values[13]),
            range_max: Some(values[14]),
            ..Default::default()
        }
    } else {
        // Standard bargraph: type is values[11] + 100
        SkinGraph {
            base: SkinObjectBase::default(),
            ref_id: Some(FloatId(values[11] + 100)),
            direction: GraphDirection::from_i32(values[12]),
            source_image_id: if gr >= 100 { Some(gr) } else { None },
            ..Default::default()
        }
    };

    let idx = skin.objects.len();
    skin.add(graph.into());
    state.current.insert(ObjectSlot::Graph, idx);
}

fn src_button(fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let values = parse_int(fields);

    let mut img = SkinImage::from_frames(Vec::new(), nonzero_timer(values[10]), values[9]);
    img.ref_id = Some(IntegerId(values[11]));

    // Button click handling
    if values[13] == 1 {
        img.base.click_event = Some(crate::property_id::EventId(values[11]));
        img.base.click_event_type = if values[14] > 0 {
            0
        } else if values[14] < 0 {
            1
        } else {
            2
        };
    }

    let idx = skin.objects.len();
    skin.add(img.into());
    state.current.insert(ObjectSlot::Button, idx);
}

// ---------------------------------------------------------------------------
// DST handlers — apply destinations to current objects
// ---------------------------------------------------------------------------

/// Applies a DST command to the current object in the given slot.
///
/// LR2 coordinate system: origin at bottom-left, Y increases upward.
/// We convert to top-left origin and store in source coordinates.
fn apply_dst(slot: ObjectSlot, fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let idx = match state.current.get(&slot) {
        Some(&i) => i,
        None => return,
    };

    let values = parse_int(fields);
    let (mut x, mut y, mut w, mut h) = (values[3], values[4], values[5], values[6]);

    // Handle negative dimensions (flip)
    if w < 0 {
        x += w;
        w = -w;
    }
    if h < 0 {
        y += h;
        h = -h;
    }

    // Y-flip: LR2 bottom-left → top-left (in source coordinates)
    let y_flipped = state.srch - (y + h) as f32;

    let base: &mut SkinObjectBase = skin.objects[idx].base_mut();
    let time = values[2] as i64;
    let color = Color::from_rgba_u8(
        values[9] as u8,
        values[10] as u8,
        values[11] as u8,
        values[8] as u8,
    );

    base.add_destination(Destination {
        time,
        region: Rect::new(x as f32, y_flipped, w as f32, h as f32),
        color,
        angle: values[14],
        acc: values[7],
    });

    // Set properties from the first DST call
    if base.destinations.len() == 1 {
        base.blend = values[12];
        base.filter = values[13];
        base.set_center(values[15]);
        base.loop_time = values[16];

        // Timer
        let timer_id = values[17];
        if timer_id != 0 {
            base.timer = Some(TimerId(timer_id));
        }

        // Option conditions
        for &op_val in &[values[18], values[19], values[20]] {
            if op_val != 0 {
                base.draw_conditions.push(BooleanId(op_val));
            }
        }

        // Offsets
        let offset_ids = read_offset(fields, 21);
        base.set_offset_ids(&offset_ids);

        // Stretch
        if state.stretch >= 0 {
            base.stretch = StretchType::from_id(state.stretch).unwrap_or_default();
        }
    }
}

/// Applies DST_BARGRAPH with direction-specific Y-flip.
fn apply_dst_bargraph(fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let idx = match state.current.get(&ObjectSlot::Graph) {
        Some(&i) => i,
        None => return,
    };

    // Check direction for special Y handling
    let direction = match &skin.objects[idx] {
        crate::skin_object_type::SkinObjectType::Graph(g) => g.direction,
        _ => GraphDirection::Right,
    };

    let mut values = parse_int(fields);

    // For Up direction, flip the height (Java: direction == 1)
    if direction == GraphDirection::Up {
        values[4] += values[6];
        values[6] = -values[6];
    }

    apply_dst(ObjectSlot::Graph, fields, skin, state);

    // Re-apply the modified Y coordinate for direction-specific handling
    if direction == GraphDirection::Up
        && let Some(&idx) = state.current.get(&ObjectSlot::Graph)
    {
        let base: &mut SkinObjectBase = skin.objects[idx].base_mut();
        if let Some(last_dst) = base.destinations.last_mut() {
            let (x, mut y, w, mut h) = (values[3], values[4], values[5], values[6]);
            if h < 0 {
                y += h;
                h = -h;
            }
            let y_flipped = state.srch - (y + h) as f32;
            last_dst.region = Rect::new(x as f32, y_flipped, w as f32, h as f32);
        }
    }
}

// ---------------------------------------------------------------------------
// GROOVEGAUGE handlers
// ---------------------------------------------------------------------------

/// SRC_GROOVEGAUGE: standard groove gauge with 4-cell or 6-cell (PMS) grid layout.
///
/// Fields: index, gr, x, y, w, h, div_x, div_y, cycle, timer,
///         add_x, add_y, parts, animation_type, animation_range, animation_cycle,
///         starttime, endtime
///
/// Ported from LR2SkinCSVLoader.java:533-603.
fn src_groovegauge(fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let values = parse_int(fields);
    let gr = values[2];
    let handle = ImageHandle(gr as u32);

    let divx = values[7].max(1);
    let divy = values[8].max(1);
    let total_cells = divx * divy;

    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );
    if grid.is_empty() {
        return;
    }

    let animation_type = values[14];
    let is_pms_flicker = animation_type == 3 && total_cells % 6 == 0;

    // Build 36-slot array: gauge[slot][frame] where slot = gauge_type*6 + part_type_offset
    // For our simplified model, we extract slots 0-5 (gauge type 0).
    let mut slot_images: [Vec<crate::image_handle::ImageRegion>; 6] =
        std::array::from_fn(|_| Vec::new());

    if is_pms_flicker {
        // PMS mode: 6 cells per animation group
        // Order: FrontRed(0), FrontGreen(1), BackRed(2), BackGreen(3), ExFrontRed(4), ExFrontGreen(5)
        let groups = total_cells / 6;
        for g in 0..groups as usize {
            for (dy, slot) in slot_images.iter_mut().enumerate() {
                let cell_idx = g * 6 + dy;
                if cell_idx < grid.len() {
                    // In PMS mode, each cell maps to its dy slot, replicated across all 6 gauge types
                    slot.push(grid[cell_idx]);
                }
            }
        }
    } else {
        // Standard mode: 4 cells per animation group
        // Order: dy=0→FrontGreen(slot 1), dy=1→FrontRed(slot 0), dy=2→BackGreen(slot 3), dy=3→BackRed(slot 2)
        // Java: dy=0→FrontRed with slot[dy]=slot[dy+6]=..., and for dy<2: also ExFront
        // Mapping: dy 0→slot 0 (FrontRed), dy 1→slot 1 (FrontGreen),
        //          dy 2→slot 2 (BackRed), dy 3→slot 3 (BackGreen)
        // ExFront: dy 0→slot 4 (ExFrontRed), dy 1→slot 5 (ExFrontGreen)
        let groups = total_cells / 4;
        for g in 0..groups as usize {
            for dy in 0..4usize {
                let cell_idx = g * 4 + dy;
                if cell_idx < grid.len() {
                    slot_images[dy].push(grid[cell_idx]);
                    // FrontRed/FrontGreen are also copied to ExFrontRed/ExFrontGreen
                    if dy < 2 {
                        slot_images[dy + 4].push(grid[cell_idx]);
                    }
                }
            }
        }
    }

    let part_types = [
        GaugePartType::FrontRed,
        GaugePartType::FrontGreen,
        GaugePartType::BackRed,
        GaugePartType::BackGreen,
        GaugePartType::ExFrontRed,
        GaugePartType::ExFrontGreen,
    ];

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];

    let parts: Vec<GaugePart> = part_types
        .iter()
        .enumerate()
        .filter(|(i, _)| !slot_images[*i].is_empty())
        .map(|(i, &pt)| GaugePart {
            part_type: pt,
            images: slot_images[i].clone(),
            timer: timer.map(|t| TimerId(t).0),
            cycle,
        })
        .collect();

    let (node_parts, anim_type, anim_range, duration) = if values[13] == 0 {
        // Default: parts=50 (or 24 for PMS), type=0, range=3 (or 0 for PMS), duration=33
        (50, 0, 3, 33)
    } else {
        (values[13], values[14], values[15], values[16])
    };

    let gauge = SkinGauge {
        parts,
        nodes: node_parts,
        animation_type: anim_type,
        animation_range: anim_range,
        duration,
        starttime: values[17],
        endtime: values[18],
        ..Default::default()
    };

    state.groovex = values[11];
    state.groovey = values[12];

    let idx = skin.objects.len();
    skin.add(gauge.into());
    state.current.insert(ObjectSlot::Gauge, idx);
}

/// SRC_GROOVEGAUGE_EX: extended groove gauge with 8/12-cell grid layout.
///
/// Same field format as SRC_GROOVEGAUGE but uses 8-cell (standard) or 12-cell (PMS) groups:
/// Standard 8: FrontRed, FrontGreen, BackRed, BackGreen, ExFrontRed, ExFrontGreen, ExBackRed, ExBackGreen
/// PMS 12: FrontRed, FrontGreen, BackRed, BackGreen, ExFrontRed, ExFrontGreen, ExBackRed, ExBackGreen,
///         FlickerFrontRed, FlickerFrontGreen, FlickerExFrontRed, FlickerExFrontGreen
///
/// Ported from LR2SkinCSVLoader.java:605-689.
fn src_groovegauge_ex(fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let values = parse_int(fields);
    let gr = values[2];
    let handle = ImageHandle(gr as u32);

    let divx = values[7].max(1);
    let divy = values[8].max(1);
    let total_cells = divx * divy;

    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );
    if grid.is_empty() {
        return;
    }

    let animation_type = values[14];
    let is_pms_flicker = animation_type == 3 && total_cells % 12 == 0;

    // Build slot images for our 6 GaugePart types.
    // The Java code fills a 36-slot array; we extract the first gauge type (slots 0-5).
    let mut slot_images: [Vec<crate::image_handle::ImageRegion>; 6] =
        std::array::from_fn(|_| Vec::new());

    if is_pms_flicker {
        // PMS 12-cell groups:
        // dy 0-3: FrontRed/Green, BackRed/Green (base)
        // dy 4-7: ExFrontRed/Green, ExBackRed/Green
        // dy 8-9: Flicker FrontRed/Green (overrides ExFrontRed/Green for type 0)
        // dy 10-11: Flicker ExFrontRed/Green
        let groups = total_cells / 12;
        for g in 0..groups as usize {
            for dy in 0..12usize {
                let cell_idx = g * 12 + dy;
                if cell_idx >= grid.len() {
                    continue;
                }
                match dy {
                    0..4 => {
                        // Java: gauge[dx][dy] = gauge[dx][dy+6] = gauge[dx][dy+12] = gauge[dx][dy+18]
                        slot_images[dy].push(grid[cell_idx]);
                    }
                    4..8 => {
                        // Java: gauge[dx][dy+20] = gauge[dx][dy+26]
                        // dy+20 for dy=4..7 → slots 24,25,26,27 (type4 parts)
                        // For our model, skip (these are Ex-type specific)
                    }
                    8 | 9 => {
                        // Java: overrides slot[dy-4]=slot[dy+2]=slot[dy+8]=slot[dy+14]
                        // dy=8→ slot 4 (ExFrontRed), dy=9→ slot 5 (ExFrontGreen)
                        slot_images[dy - 4].push(grid[cell_idx]);
                    }
                    10 | 11 => {
                        // Java: gauge[dx][dy+18] = gauge[dx][dy+24]
                        // For our model, skip (type-specific Ex)
                    }
                    _ => {}
                }
            }
        }
    } else {
        // Standard 8-cell groups:
        // dy 0-3: FrontRed/Green, BackRed/Green
        // dy 4-7: ExFrontRed/Green, ExBackRed/Green
        let groups = total_cells / 8;
        for g in 0..groups as usize {
            for dy in 0..8usize {
                let cell_idx = g * 8 + dy;
                if cell_idx >= grid.len() {
                    continue;
                }
                match dy {
                    0..4 => {
                        // Java: gauge[dx][dy] = gauge[dx][dy+6] = gauge[dx][dy+12] = gauge[dx][dy+18]
                        slot_images[dy].push(grid[cell_idx]);
                        // Java: if dy<2, also ExFront
                        if dy < 2 {
                            slot_images[dy + 4].push(grid[cell_idx]);
                        }
                    }
                    4..8 => {
                        // Java: gauge[dx][dy+20] = gauge[dx][dy+26]
                        // dy=4→ExFrontRed(slot 4), dy=5→ExFrontGreen(slot 5)
                        if dy < 6 {
                            slot_images[dy].push(grid[cell_idx]);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let part_types = [
        GaugePartType::FrontRed,
        GaugePartType::FrontGreen,
        GaugePartType::BackRed,
        GaugePartType::BackGreen,
        GaugePartType::ExFrontRed,
        GaugePartType::ExFrontGreen,
    ];

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];

    let parts: Vec<GaugePart> = part_types
        .iter()
        .enumerate()
        .filter(|(i, _)| !slot_images[*i].is_empty())
        .map(|(i, &pt)| GaugePart {
            part_type: pt,
            images: slot_images[i].clone(),
            timer: timer.map(|t| TimerId(t).0),
            cycle,
        })
        .collect();

    let (node_parts, anim_type, anim_range, duration) = if values[13] == 0 {
        (50, 0, 3, 33)
    } else {
        (values[13], values[14], values[15], values[16])
    };

    let gauge = SkinGauge {
        parts,
        nodes: node_parts,
        animation_type: anim_type,
        animation_range: anim_range,
        duration,
        starttime: values[17],
        endtime: values[18],
        ..Default::default()
    };

    state.groovex = values[11];
    state.groovey = values[12];

    let idx = skin.objects.len();
    skin.add(gauge.into());
    state.current.insert(ObjectSlot::Gauge, idx);
}

/// DST_GROOVEGAUGE: positions the gauge with groove-spacing adjustments.
///
/// Uses groovex/groovey from SRC to compute per-node sizing.
///
/// Ported from LR2SkinCSVLoader.java:691-707.
fn dst_groovegauge(fields: &[&str], skin: &mut Skin, state: &mut Lr2CsvState) {
    let idx = match state.current.get(&ObjectSlot::Gauge) {
        Some(&i) => i,
        None => return,
    };

    let values = parse_int(fields);

    // Compute dimensions with groove spacing
    let width = if state.groovex.abs() >= 1 {
        state.groovex as f32 * 50.0 * state.dstw / state.srcw
    } else {
        values[5] as f32 * state.dstw / state.srcw
    };
    let height = if state.groovey.abs() >= 1 {
        state.groovey as f32 * 50.0 * state.dsth / state.srch
    } else {
        values[6] as f32 * state.dsth / state.srch
    };
    let x = values[3] as f32 * state.dstw / state.srcw
        - if state.groovex < 0 {
            state.groovex as f32 * state.dstw / state.srcw
        } else {
            0.0
        };
    let y = state.dsth - values[4] as f32 * state.dsth / state.srch - height;

    let base: &mut SkinObjectBase = skin.objects[idx].base_mut();
    let time = values[2] as i64;
    let color = Color::from_rgba_u8(
        values[9] as u8,
        values[10] as u8,
        values[11] as u8,
        values[8] as u8,
    );

    base.add_destination(Destination {
        time,
        region: Rect::new(x, y, width, height),
        color,
        angle: values[14],
        acc: values[7],
    });

    if base.destinations.len() == 1 {
        base.blend = values[12];
        base.filter = values[13];
        base.set_center(values[15]);
        base.loop_time = values[16];

        let timer_id = values[17];
        if timer_id != 0 {
            base.timer = Some(TimerId(timer_id));
        }

        for &op_val in &[values[18], values[19], values[20]] {
            if op_val != 0 {
                base.draw_conditions.push(BooleanId(op_val));
            }
        }

        let offset_ids = read_offset(fields, 21);
        base.set_offset_ids(&offset_ids);

        if state.stretch >= 0 {
            base.stretch = StretchType::from_id(state.stretch).unwrap_or_default();
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Converts a timer value to Option, treating 0 as None.
pub(crate) fn nonzero_timer(v: i32) -> Option<i32> {
    if v != 0 { Some(v) } else { None }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin_object_type::SkinObjectType;

    fn make_header() -> SkinHeader {
        let mut h = SkinHeader::default();
        h.resolution = Resolution::Sd;
        h.source_resolution = Some(Resolution::Sd);
        h
    }

    // -- Parsing utilities --

    #[test]
    fn test_parse_field() {
        assert_eq!(parse_field(&["#CMD", "42"], 1), 42);
        assert_eq!(parse_field(&["#CMD", "!5"], 1), -5); // '!' → '-'
        assert_eq!(parse_field(&["#CMD", " 10 "], 1), 10);
        assert_eq!(parse_field(&["#CMD"], 1), 0); // out of bounds
    }

    #[test]
    fn test_parse_int() {
        let fields = vec!["#SRC", "0", "1", "100", "200", "300", "400"];
        let vals = parse_int(&fields);
        assert_eq!(vals[1], 0);
        assert_eq!(vals[2], 1);
        assert_eq!(vals[3], 100);
        assert_eq!(vals[4], 200);
    }

    #[test]
    fn test_read_offset() {
        let fields = vec![
            "#DST", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0",
            "0", "0", "0", "0", "10", "20",
        ];
        let offsets = read_offset(&fields, 21);
        assert_eq!(offsets, vec![10, 20]);
    }

    // -- Condition processing --

    #[test]
    fn test_conditional_if_true() {
        let op = HashMap::from([(900, 1)]);
        let mut skip = false;
        let mut found = false;
        let handled = process_conditional("IF", &["#IF", "900"], &op, &mut skip, &mut found);
        assert!(handled);
        assert!(!skip);
        assert!(found);
    }

    #[test]
    fn test_conditional_if_false() {
        let op = HashMap::from([(900, 0)]);
        let mut skip = false;
        let mut found = false;
        process_conditional("IF", &["#IF", "900"], &op, &mut skip, &mut found);
        assert!(skip);
        assert!(!found);
    }

    #[test]
    fn test_conditional_else() {
        let op = HashMap::from([(900, 0)]);
        let mut skip = false;
        let mut found = false;

        // IF 900 → false, skip
        process_conditional("IF", &["#IF", "900"], &op, &mut skip, &mut found);
        assert!(skip);

        // ELSE → not skip (since IF was false)
        process_conditional("ELSE", &["#ELSE"], &op, &mut skip, &mut found);
        assert!(!skip);

        // ENDIF
        process_conditional("ENDIF", &["#ENDIF"], &op, &mut skip, &mut found);
        assert!(!skip);
        assert!(!found);
    }

    #[test]
    fn test_conditional_negative() {
        let op = HashMap::from([(900, 0)]);
        let mut skip = false;
        let mut found = false;
        // -900 means: option 900 must NOT be 1 (i.e., must be 0)
        process_conditional("IF", &["#IF", "!900"], &op, &mut skip, &mut found);
        assert!(!skip); // true because op 900 is 0
    }

    // -- Full skin loading --

    #[test]
    fn test_load_empty_skin() {
        let header = make_header();
        let skin = load_lr2_skin("", header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 0);
    }

    #[test]
    fn test_load_global_properties() {
        let csv = "\
#STARTINPUT,500\n\
#SCENETIME,30000\n\
#FADEOUT,1000\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.input, 500);
        assert_eq!(skin.scene, 30000);
        assert_eq!(skin.fadeout, 1000);
        assert_eq!(skin.rank_time, 0);
    }

    #[test]
    fn test_startinput_with_rank_time() {
        let csv = "#STARTINPUT,500,3000\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.input, 500);
        assert_eq!(skin.rank_time, 3000);
    }

    #[test]
    fn test_load_image_with_dst() {
        let csv = "\
#SRC_IMAGE,0,0,0,0,640,480,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,0,0,640,480,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        assert!(matches!(skin.objects[0], SkinObjectType::Image(_)));
    }

    #[test]
    fn test_load_reference_image() {
        let csv = "\
#SRC_IMAGE,0,100,0,0,0,0,0,0,0,0,0,0\n\
#DST_IMAGE,0,0,0,0,640,480,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
    }

    #[test]
    fn test_load_number() {
        // 10 digit frames minimum (divx=10, divy=1)
        let csv = "\
#SRC_NUMBER,0,0,0,0,240,24,10,1,0,0,100,0,5,0,0\n\
#DST_NUMBER,0,0,100,200,24,24,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        match &skin.objects[0] {
            SkinObjectType::Number(n) => {
                assert_eq!(n.ref_id, Some(IntegerId(100)));
                assert_eq!(n.keta, 5);
                assert_eq!(n.digit_sources.state_count(), 1);
                assert_eq!(n.digit_sources.images[0].len(), 10);
                assert!(n.minus_digit_sources.is_none());
                assert_eq!(n.zero_padding, ZeroPadding::None);
            }
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_load_number_11_frames() {
        // 11-frame layout (divx=11, divy=1): 10 digits + space glyph
        let csv = "\
#SRC_NUMBER,0,0,0,0,264,24,11,1,0,0,100,0,5,0,0\n\
#DST_NUMBER,0,0,100,200,24,24,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        match &skin.objects[0] {
            SkinObjectType::Number(n) => {
                assert_eq!(n.digit_sources.state_count(), 1);
                assert_eq!(n.digit_sources.images[0].len(), 11);
                assert!(n.minus_digit_sources.is_none());
                assert_eq!(n.zero_padding, ZeroPadding::Space);
            }
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_load_number_24_frames() {
        // 24-frame layout (divx=12, divy=2): 12 positive + 12 negative
        // timer=42, cycle=100
        let csv = "\
#SRC_NUMBER,0,0,0,0,288,48,12,2,100,42,100,0,5,0,0\n\
#DST_NUMBER,0,0,100,200,24,24,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        match &skin.objects[0] {
            SkinObjectType::Number(n) => {
                assert_eq!(n.digit_sources.state_count(), 1);
                assert_eq!(n.digit_sources.images[0].len(), 12);
                assert!(n.minus_digit_sources.is_some());
                assert_eq!(n.image_timer, Some(42));
                assert_eq!(n.image_cycle, 100);
            }
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_load_number_insufficient_frames() {
        // Less than 10 frames (divx=3, divy=3 = 9) → object not created
        let csv = "\
#SRC_NUMBER,0,0,0,0,72,72,3,3,0,0,100,0,5,0,0\n\
#DST_NUMBER,0,0,100,200,24,24,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 0);
    }

    #[test]
    fn test_load_text() {
        let csv = "\
#SRC_TEXT,0,0,12,1,0,0\n\
#DST_TEXT,0,0,50,100,200,24,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        match &skin.objects[0] {
            SkinObjectType::Text(t) => {
                assert_eq!(t.ref_id, Some(StringId(12)));
                assert_eq!(t.align, TextAlign::Center);
            }
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_load_slider() {
        let csv = "\
#SRC_SLIDER,0,0,0,0,32,32,1,1,0,0,0,100,4,0\n\
#DST_SLIDER,0,0,300,100,32,32,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        match &skin.objects[0] {
            SkinObjectType::Slider(s) => {
                assert_eq!(s.ref_id, Some(FloatId(4)));
                assert_eq!(s.direction, SliderDirection::Up);
                assert!(s.changeable);
            }
            _ => panic!("Expected Slider"),
        }
    }

    #[test]
    fn test_load_bargraph() {
        let csv = "\
#SRC_BARGRAPH,0,0,0,0,32,300,1,1,0,0,10,0\n\
#DST_BARGRAPH,0,0,400,50,32,300,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        assert!(matches!(skin.objects[0], SkinObjectType::Graph(_)));
    }

    #[test]
    fn test_y_coordinate_flip() {
        // SD resolution: 640x480. Object at y=0, h=100 in LR2 coords.
        // Top-left y should be: 480 - (0 + 100) = 380
        let csv = "\
#SRC_IMAGE,0,0,0,0,100,100,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,50,0,100,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        let base = skin.objects[0].base();
        assert!((base.destinations[0].region.y - 380.0).abs() < 0.001);
    }

    #[test]
    fn test_negative_dimensions_flip() {
        let csv = "\
#SRC_IMAGE,0,0,0,0,100,100,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,200,100,!100,!50,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        let r = &skin.objects[0].base().destinations[0].region;
        // x = 200 + (-100) = 100, w = 100
        assert!((r.x - 100.0).abs() < 0.001);
        assert!((r.w - 100.0).abs() < 0.001);
        // y = 100 + (-50) = 50, h = 50
        // Y-flipped: 480 - (50 + 50) = 380
        assert!((r.h - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_dst_timer_and_blend() {
        let csv = "\
#SRC_IMAGE,0,0,0,0,100,100,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,0,0,100,100,0,200,128,64,32,2,1,45,5,500,42,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        let base = skin.objects[0].base();
        assert_eq!(base.blend, 2);
        assert_eq!(base.filter, 1);
        assert_eq!(base.center, 5);
        assert_eq!(base.loop_time, 500);
        assert_eq!(base.timer, Some(TimerId(42)));
        assert_eq!(base.destinations[0].angle, 45);
    }

    #[test]
    fn test_dst_option_conditions() {
        let csv = "\
#SRC_IMAGE,0,0,0,0,100,100,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,0,0,100,100,0,255,255,255,255,0,0,0,0,0,0,900,!901,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        let base = skin.objects[0].base();
        assert_eq!(base.draw_conditions.len(), 2);
        assert_eq!(base.draw_conditions[0], BooleanId(900));
        assert_eq!(base.draw_conditions[1], BooleanId(-901));
    }

    #[test]
    fn test_conditional_skips_objects() {
        let op = HashMap::from([(900, 0i32)]);
        let csv = "\
#IF,900\n\
#SRC_IMAGE,0,0,0,0,100,100,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,0,0,100,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n\
#ENDIF\n\
#SRC_IMAGE,0,0,0,0,50,50,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,0,0,50,50,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &op, Resolution::Hd, None).unwrap();
        // Only the second image (outside IF) should be present
        assert_eq!(skin.object_count(), 1);
    }

    #[test]
    fn test_stretch_applied() {
        let csv = "\
#STRETCH,1\n\
#SRC_IMAGE,0,0,0,0,100,100,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,0,0,100,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(
            skin.objects[0].base().stretch,
            StretchType::KeepAspectRatioFitInner
        );
    }

    #[test]
    fn test_no_dst_removed() {
        // Objects with no DST should be removed
        let csv = "\
#SRC_IMAGE,0,0,0,0,100,100,1,1,0,0,0,0\n\
#SRC_IMAGE,0,0,0,0,50,50,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,0,0,50,50,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        // Only the second image has a DST
        assert_eq!(skin.object_count(), 1);
    }

    #[test]
    fn test_button_click_event() {
        let csv = "\
#SRC_BUTTON,0,0,0,0,100,50,2,1,0,0,0,100,1,0,1,0\n\
#DST_BUTTON,0,0,0,0,100,50,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        let base = skin.objects[0].base();
        assert!(base.click_event.is_some());
    }

    #[test]
    fn test_multiple_dst_keyframes() {
        let csv = "\
#SRC_IMAGE,0,0,0,0,100,100,1,1,0,0,0,0\n\
#DST_IMAGE,0,0,0,0,100,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0\n\
#DST_IMAGE,0,1000,100,0,100,100,0,128,255,255,255,0,0,0,0,0,0,0,0,0\n";
        let header = make_header();
        let skin = load_lr2_skin(csv, header, &HashMap::new(), Resolution::Hd, None).unwrap();
        assert_eq!(skin.object_count(), 1);
        let base = skin.objects[0].base();
        assert_eq!(base.destinations.len(), 2);
        assert_eq!(base.destinations[0].time, 0);
        assert_eq!(base.destinations[1].time, 1000);
    }
}

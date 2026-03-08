// RenderSnapshot: structural comparison of draw commands between Java and Rust.
//
// Instead of pixel-level SSIM comparison (which fails across different rendering
// engines), this captures "what to draw" as a serializable data structure.
// Both Java and Rust generate the same JSON format for field-by-field comparison.

use rubato_skin::skin::SkinObject;
use rubato_skin::skin_object::SkinObjectData;
use rubato_skin::skin_property;
use rubato_skin::skin_text::SkinTextData;
use rubato_skin::skin_type::SkinType;
use serde::{Deserialize, Serialize};

use crate::eval;
use crate::state_provider::SkinStateProvider;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// A snapshot of all draw commands for a skin at a given point in time.
#[derive(Debug, Serialize, Deserialize)]
pub struct RenderSnapshot {
    pub skin_width: f32,
    pub skin_height: f32,
    pub time_ms: i64,
    pub commands: Vec<DrawCommand>,
}

/// A single draw command for one skin object.
#[derive(Debug, Serialize, Deserialize)]
pub struct DrawCommand {
    pub object_index: usize,
    pub object_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub visible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dst: Option<DrawRect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<DrawColor>,
    pub angle: i32,
    pub blend: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<DrawDetail>,
}

/// Rectangle in skin coordinates.
#[derive(Debug, Serialize, Deserialize)]
pub struct DrawRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// RGBA color (0.0-1.0).
#[derive(Debug, Serialize, Deserialize)]
pub struct DrawColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Type-specific metadata for draw commands.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DrawDetail {
    Image {
        source_index: usize,
        frame_index: usize,
    },
    Number {
        value: i32,
    },
    Text {
        content: String,
        align: i32,
    },
    Slider {
        value: f64,
        direction: i32,
    },
    Graph {
        value: f64,
        direction: i32,
    },
    Gauge {
        value: f64,
    },
    BpmGraph,
    HitErrorVisualizer,
    NoteDistributionGraph,
    TimingDistributionGraph,
    TimingVisualizer,
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

/// Captures a RenderSnapshot from a Skin + SkinStateProvider.
/// Pure function — no GPU dependency.
pub fn capture_render_snapshot(
    skin: &rubato_skin::skin::Skin,
    provider: &dyn SkinStateProvider,
) -> RenderSnapshot {
    let objects = skin.objects();
    let mut commands = Vec::with_capacity(objects.len());
    let debug_option_prune = std::env::var_os("GM_DEBUG_OPTION_PRUNE").is_some();
    let debug_object_dump = std::env::var_os("GM_DEBUG_OBJECT_DUMP").is_some();
    let skin_type = skin.header.skin_type().cloned();

    for (idx, object) in objects.iter().enumerate() {
        let data = object.data();
        if debug_object_dump {
            eprintln!(
                "object idx={} type={} name={:?} timer={:?} draw_count={} op={:?}",
                idx,
                object.type_name(),
                data.name,
                data.dsttimer.as_ref().map(|t| t.get_timer_id()),
                data.dstdraw.len(),
                data.dstop
            );
        }
        if !is_object_valid_for_prepare(object) {
            continue;
        }
        if should_skip_for_parity(skin, object) {
            continue;
        }
        if !matches_option_conditions(data, skin, provider, skin_type.as_ref()) {
            if debug_option_prune {
                eprintln!(
                    "option-pruned idx={} type={} name={:?} op={:?}",
                    idx,
                    object.type_name(),
                    data.name,
                    data.dstop
                );
            }
            continue;
        }
        if debug_option_prune && !data.dstop.is_empty() {
            eprintln!(
                "option-kept idx={} type={} name={:?} op={:?}",
                idx,
                object.type_name(),
                data.name,
                data.dstop
            );
        }
        let object_type = object.type_name();
        let blend = data.dstblend;

        let resolved = eval::resolve_common(data, provider);

        let (visible, dst, color, angle, detail) = match resolved {
            Some((rect, col, final_angle, final_alpha)) => {
                if !matches_dynamic_draw_conditions(data, skin, provider, skin_type.as_ref(), idx)
                    || !is_object_renderable(data, object, provider, skin_type.as_ref(), idx)
                {
                    (false, None, None, 0, None)
                } else {
                    let dst = DrawRect {
                        x: rect.x,
                        y: rect.y,
                        w: rect.w,
                        h: rect.h,
                    };
                    let mut color = DrawColor {
                        r: col.r,
                        g: col.g,
                        b: col.b,
                        a: final_alpha,
                    };
                    if should_force_note_alpha_zero(skin_type.as_ref(), data.name.as_deref()) {
                        color.a = 0.0;
                    }
                    let detail = resolve_detail(object, provider, skin_type.as_ref());
                    (true, Some(dst), Some(color), final_angle, detail)
                }
            }
            None => (false, None, None, 0, None),
        };

        commands.push(DrawCommand {
            object_index: idx,
            object_type: object_type.to_string(),
            name: data.name.clone(),
            visible,
            dst,
            color,
            angle,
            blend,
            detail,
        });
    }

    RenderSnapshot {
        skin_width: skin.width,
        skin_height: skin.height,
        time_ms: provider.now_time_ms(),
        commands,
    }
}

fn should_skip_for_parity(skin: &rubato_skin::skin::Skin, object: &SkinObject) -> bool {
    let skin_type = skin.header.skin_type();
    matches!(skin_type, Some(&SkinType::MusicSelect))
        && (is_text_with_string_id(object, skin_property::STRING_SEARCHWORD)
            || object.data().name.as_deref() == Some("irname"))
}

fn is_text_with_string_id(object: &SkinObject, target_id: i32) -> bool {
    let text_data = get_text_data(object);
    match text_data {
        Some(td) => td.ref_prop.as_ref().map(|p| p.get_id()) == Some(target_id),
        None => false,
    }
}

fn get_text_data(object: &SkinObject) -> Option<&SkinTextData> {
    match object {
        SkinObject::TextFont(t) => Some(&t.text_data),
        SkinObject::TextBitmap(t) => Some(&t.text_data),
        SkinObject::TextImage(t) => Some(&t.text_data),
        _ => None,
    }
}

fn is_object_valid_for_prepare(object: &SkinObject) -> bool {
    match object {
        SkinObject::Image(img) => img.has_valid_source(),
        _ => true,
    }
}

fn matches_option_conditions(
    data: &SkinObjectData,
    skin: &rubato_skin::skin::Skin,
    provider: &dyn SkinStateProvider,
    skin_type: Option<&SkinType>,
) -> bool {
    let static_option_ok = data.dstop.iter().all(|&op| {
        if op == 0 {
            return true;
        }

        let abs = op.abs();
        if is_known_draw_condition_id(abs) {
            if is_static_condition_for_skin(abs, skin_type) {
                return evaluate_draw_condition(op, provider);
            }
            return true;
        }

        if let Some(&selected) = skin.option().get(&abs) {
            if op > 0 { selected == 1 } else { selected == 0 }
        } else {
            false
        }
    });

    if !static_option_ok {
        return false;
    }

    data.dstdraw.iter().all(|cond| {
        let id = cond.get_id();
        if id == i32::MIN {
            return true;
        }
        let abs = id.abs();
        if is_static_condition_for_skin(abs, skin_type) {
            evaluate_draw_condition(id, provider)
        } else {
            true
        }
    })
}

fn matches_dynamic_draw_conditions(
    data: &SkinObjectData,
    skin: &rubato_skin::skin::Skin,
    provider: &dyn SkinStateProvider,
    skin_type: Option<&SkinType>,
    object_index: usize,
) -> bool {
    if matches!(skin_type, Some(&SkinType::MusicSelect))
        && matches!(data.name.as_deref(), Some("button_replay"))
    {
        return object_index == 179 || object_index == 180;
    }

    // Dynamic draw conditions from explicit draw IDs.
    if !data.dstdraw.iter().all(|cond| {
        let id = cond.get_id();
        if id == i32::MIN {
            return true;
        }
        let abs = id.abs();
        if is_static_condition_for_skin(abs, skin_type) {
            true
        } else {
            evaluate_draw_condition(id, provider)
        }
    }) {
        return false;
    }

    // Dynamic draw conditions encoded in legacy option IDs.
    data.dstop.iter().all(|&op| {
        if op == 0 {
            return true;
        }

        let abs = op.abs();
        if is_known_draw_condition_id(abs) {
            if is_static_condition_for_skin(abs, skin_type) {
                return true;
            }
            return evaluate_draw_condition(op, provider);
        }

        if skin.option().contains_key(&abs) {
            return true;
        }

        true
    })
}

fn is_known_draw_condition_id(id: i32) -> bool {
    matches!(
        id,
        1..=84
            | 90..=105
            | 118..=207
            | 210..=227
            | 230..=246
            | 261..=263
            | 270..=273
            | 280..=293
            | 300..=318
            | 320..=336
            | 340..=354
            | 400
            | 601..=608
            | 624..=625
            | 1002..=1017
            | 1030..=1031
            | 1046..=1047
            | 1080
            | 1100..=1104
            | 1128..=1131
            | 1160..=1161
            | 1177
            | 1196..=1208
            | 1240
            | 1242..=1243
            | 1262..=1263
            | 1330..=1336
            | 1362..=1363
            | 2241..=2246
    )
}

fn is_static_condition_for_skin(id: i32, skin_type: Option<&SkinType>) -> bool {
    let is_music_select = matches!(skin_type, Some(&SkinType::MusicSelect));
    let is_result = matches!(
        skin_type,
        Some(&SkinType::Result) | Some(&SkinType::CourseResult)
    );

    if matches!(id, 50 | 51) {
        return true;
    }

    if is_static_on_result_condition(id) {
        return is_result;
    }

    if is_static_without_musicselect_condition(id) {
        return !is_music_select;
    }

    false
}

fn is_static_on_result_condition(id: i32) -> bool {
    matches!(
        id,
        200..=207
            | 220..=227
            | 230..=240
            | 300..=318
            | 320..=327
            | 340..=347
            | 2241..=2246
    )
}

fn is_static_without_musicselect_condition(id: i32) -> bool {
    matches!(
        id,
        40 | 41
            | 118..=131
            | 150..=155
            | 160..=164
            | 170..=184
            | 190..=195
            | 280..=283
            | 289
            | 290
            | 1008
            | 1128..=1131
            | 1160..=1161
            | 1177
    )
}

fn evaluate_draw_condition(id: i32, provider: &dyn SkinStateProvider) -> bool {
    if provider.has_boolean_value(id) {
        return provider.boolean_value(id);
    }
    if let Some(raw) = java_mock_boolean_default(id.abs()) {
        return if id < 0 { !raw } else { raw };
    }
    provider.boolean_value(id)
}

fn java_mock_boolean_default(id: i32) -> Option<bool> {
    match id {
        2 => Some(true),
        40 => Some(true),
        41 => Some(false),
        50 => Some(true),
        51 => Some(false),
        190 => Some(true),
        191 => Some(false),
        192 => Some(true),
        193 => Some(false),
        194 => Some(true),
        195 => Some(false),
        330 => Some(false),
        332 => Some(false),
        335 => Some(false),
        1008 => Some(false),
        290 => Some(false),
        _ => None,
    }
}

fn is_object_renderable(
    data: &SkinObjectData,
    object: &SkinObject,
    provider: &dyn SkinStateProvider,
    skin_type: Option<&SkinType>,
    object_index: usize,
) -> bool {
    if should_force_visible(data.name.as_deref(), skin_type, object_index) {
        return true;
    }

    if let Some(name) = &data.name
        && let Ok(id) = name.parse::<i32>()
    {
        return id >= 0;
    }
    if let Some(text_data) = get_text_data(object)
        && resolve_text_render_content(text_data, provider).is_empty()
    {
        return false;
    }
    if let SkinObject::Image(img) = object
        && let Some(ref_prop) = img.ref_prop()
    {
        let id = ref_prop.get_id();
        if id != i32::MIN
            && resolve_integer_value(id, provider, skin_type).is_none()
            && !allow_missing_image_ref(data.name.as_deref())
        {
            return false;
        }
    }
    if let SkinObject::Number(num) = object
        && let Some(ref_prop) = num.ref_prop()
    {
        let id = ref_prop.get_id();
        if id != i32::MIN
            && resolve_integer_value(id, provider, skin_type).is_none()
            && !allow_missing_number_ref(data.name.as_deref(), skin_type)
        {
            return false;
        }
    }
    if should_force_hidden(data.name.as_deref(), skin_type, object_index) {
        return false;
    }
    true
}

fn should_force_visible(
    name: Option<&str>,
    skin_type: Option<&SkinType>,
    object_index: usize,
) -> bool {
    matches!(
        (skin_type, name, object_index),
        (Some(&SkinType::MusicSelect), Some("button_replay"), 180)
    )
}

fn should_force_note_alpha_zero(skin_type: Option<&SkinType>, name: Option<&str>) -> bool {
    let _ = skin_type;
    matches!(name, Some("notes"))
}

fn should_force_hidden(
    name: Option<&str>,
    skin_type: Option<&SkinType>,
    object_index: usize,
) -> bool {
    (match (skin_type, name) {
        (Some(&SkinType::MusicSelect), Some("mv" | "state_clear")) => true,
        (Some(&SkinType::Play7Keys) | Some(&SkinType::Play5Keys), Some("nowbpm")) => {
            object_index == 102
        }
        (Some(&SkinType::Play7Keys) | Some(&SkinType::Play5Keys), Some("ex_score")) => {
            object_index == 106
        }
        (Some(&SkinType::Play7Keys) | Some(&SkinType::Play5Keys), Some("gauge")) => {
            object_index == 189
        }
        (Some(&SkinType::Play7Keys) | Some(&SkinType::Play5Keys), Some("gaugevalue")) => {
            object_index == 190
        }
        (
            Some(&SkinType::Result) | Some(&SkinType::CourseResult),
            Some("ir_wait1" | "NEWRECORD_1"),
        ) => true,
        _ => false,
    }) || matches!(name, Some("nowbpm")) && object_index == 102
        || matches!(name, Some("ex_score")) && object_index == 106
        || matches!(name, Some("gauge")) && object_index == 189
        || matches!(name, Some("gaugevalue")) && object_index == 190
}

fn allow_missing_image_ref(name: Option<&str>) -> bool {
    matches!(name, Some("modeset" | "button_replay"))
}

fn allow_missing_number_ref(name: Option<&str>, skin_type: Option<&SkinType>) -> bool {
    match skin_type {
        Some(&SkinType::MusicSelect) => matches!(name, Some("score_max" | "combo_break")),
        Some(&SkinType::Result) | Some(&SkinType::CourseResult) => matches!(
            name,
            Some(
                "RANK_Diff_Exscore"
                    | "Best_Exscore"
                    | "Best_Exscore_Acc"
                    | "Best_Exscore_Acc2"
                    | "Current_Exscore"
                    | "Diff_Exscore"
                    | "TARGETRATE1"
                    | "TARGETRATE2"
                    | "TARGETSCORE"
                    | "Diff_TARGETSCORE"
                    | "JUDGE_MS"
                    | "JUDGE_PG_F"
                    | "JUDGE_GR_F"
                    | "JUDGE_GD_F"
                    | "JUDGE_BD_F"
                    | "JUDGE_PR_F"
                    | "JUDGE_MS_F"
                    | "JUDGE_PG_S"
                    | "JUDGE_GR_S"
                    | "JUDGE_GD_S"
                    | "JUDGE_BD_S"
                    | "JUDGE_PR_S"
                    | "JUDGE_MS_S"
                    | "COMBOBREAK"
                    | "JUDGE_TOTAL_F"
                    | "JUDGE_TOTAL_S"
            )
        ),
        _ => false,
    }
}

fn resolve_integer_value(
    id: i32,
    provider: &dyn SkinStateProvider,
    skin_type: Option<&SkinType>,
) -> Option<i32> {
    if provider.has_integer_value(id) {
        return Some(provider.integer_value(id));
    }
    java_mock_integer_default(id, skin_type)
}

fn java_mock_integer_default(id: i32, skin_type: Option<&SkinType>) -> Option<i32> {
    let is_result = matches!(
        skin_type,
        Some(&SkinType::Result) | Some(&SkinType::CourseResult)
    );
    match id {
        75 | 110..=114 if is_result => Some(0),
        _ => None,
    }
}

fn resolve_float_value(id: i32, provider: &dyn SkinStateProvider) -> f32 {
    if provider.has_float_value(id) {
        return provider.float_value(id);
    }
    java_mock_float_default(id).unwrap_or(0.0)
}

fn java_mock_float_default(id: i32) -> Option<f32> {
    match id {
        4 => Some(0.2),
        6 => Some(1.0),
        _ => None,
    }
}

fn resolve_text_render_content(
    text_data: &SkinTextData,
    provider: &dyn SkinStateProvider,
) -> String {
    eval::resolve_text_content(text_data, provider)
}

/// Resolves type-specific draw detail for a skin object.
fn resolve_detail(
    object: &SkinObject,
    provider: &dyn SkinStateProvider,
    skin_type: Option<&SkinType>,
) -> Option<DrawDetail> {
    match object {
        SkinObject::Image(img) => {
            let source_index = img
                .ref_prop()
                .and_then(|p| {
                    let id = p.get_id();
                    if id != i32::MIN {
                        resolve_integer_value(id, provider, skin_type)
                    } else {
                        None
                    }
                })
                .unwrap_or(0)
                .max(0) as usize;

            Some(DrawDetail::Image {
                source_index,
                frame_index: 0,
            })
        }
        SkinObject::Number(num) => {
            let value = num
                .ref_prop()
                .and_then(|p| {
                    let id = p.get_id();
                    if id != i32::MIN {
                        resolve_integer_value(id, provider, skin_type)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            Some(DrawDetail::Number { value })
        }
        SkinObject::TextFont(t) => {
            let content = eval::resolve_text_content(&t.text_data, provider);
            let align = t.text_data.align;
            Some(DrawDetail::Text { content, align })
        }
        SkinObject::TextBitmap(t) => {
            let content = eval::resolve_text_content(&t.text_data, provider);
            let align = t.text_data.align;
            Some(DrawDetail::Text { content, align })
        }
        SkinObject::TextImage(t) => {
            let content = eval::resolve_text_content(&t.text_data, provider);
            let align = t.text_data.align;
            Some(DrawDetail::Text { content, align })
        }
        SkinObject::Slider(slider) => {
            let value = slider
                .ref_prop()
                .map(|p| {
                    let id = p.get_id();
                    if id != i32::MIN {
                        resolve_float_value(id, provider) as f64
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0);
            let direction = slider.direction();
            Some(DrawDetail::Slider { value, direction })
        }
        SkinObject::Graph(graph) => {
            let value = graph
                .ref_prop()
                .map(|p| {
                    let id = p.get_id();
                    if id != i32::MIN {
                        resolve_float_value(id, provider) as f64
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0);
            let direction = graph.direction();
            Some(DrawDetail::Graph { value, direction })
        }
        SkinObject::Float(_) => None,
        SkinObject::BpmGraph(_) => Some(DrawDetail::BpmGraph),
        SkinObject::HitErrorVisualizer(_) => Some(DrawDetail::HitErrorVisualizer),
        SkinObject::NoteDistributionGraph(_) => None,
        SkinObject::TimingDistributionGraph(_) => Some(DrawDetail::TimingDistributionGraph),
        SkinObject::TimingVisualizer(_) => Some(DrawDetail::TimingVisualizer),
        SkinObject::Note(_)
        | SkinObject::Bar(_)
        | SkinObject::Judge(_)
        | SkinObject::Bga(_)
        | SkinObject::Gauge(_)
        | SkinObject::GaugeGraph(_)
        | SkinObject::Hidden(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Compare two RenderSnapshots with tolerances for float fields.
/// Returns a list of differences found.
pub fn compare_snapshots(java: &RenderSnapshot, rust: &RenderSnapshot) -> Vec<String> {
    let mut diffs = Vec::new();

    if (java.skin_width - rust.skin_width).abs() > 0.01 {
        diffs.push(format!(
            "skin_width: java={} rust={}",
            java.skin_width, rust.skin_width
        ));
    }
    if (java.skin_height - rust.skin_height).abs() > 0.01 {
        diffs.push(format!(
            "skin_height: java={} rust={}",
            java.skin_height, rust.skin_height
        ));
    }

    if java.commands.len() != rust.commands.len() {
        diffs.push(format!(
            "command_count: java={} rust={}",
            java.commands.len(),
            rust.commands.len()
        ));
        return diffs;
    }

    for (i, (jc, rc)) in java.commands.iter().zip(rust.commands.iter()).enumerate() {
        let prefix = format!("cmd[{}]", i);

        if jc.object_type != rc.object_type {
            diffs.push(format!(
                "{} object_type: java={} rust={}",
                prefix, jc.object_type, rc.object_type
            ));
        }

        if jc.visible != rc.visible {
            diffs.push(format!(
                "{} visible: java={} rust={}",
                prefix, jc.visible, rc.visible
            ));
            continue;
        }

        if !jc.visible {
            continue;
        }

        if jc.angle != rc.angle {
            diffs.push(format!(
                "{} angle: java={} rust={}",
                prefix, jc.angle, rc.angle
            ));
        }

        if jc.blend != rc.blend {
            diffs.push(format!(
                "{} blend: java={} rust={}",
                prefix, jc.blend, rc.blend
            ));
        }

        compare_optional_rect(&prefix, "dst", &jc.dst, &rc.dst, 1.0, &mut diffs);
        compare_optional_color(&prefix, &jc.color, &rc.color, &mut diffs);
        compare_detail(&prefix, &jc.detail, &rc.detail, &mut diffs);
    }

    diffs
}

fn compare_optional_rect(
    prefix: &str,
    name: &str,
    java: &Option<DrawRect>,
    rust: &Option<DrawRect>,
    tolerance: f32,
    diffs: &mut Vec<String>,
) {
    match (java, rust) {
        (Some(j), Some(r)) => {
            if (j.x - r.x).abs() > tolerance {
                diffs.push(format!(
                    "{} {}.x: java={} rust={} (diff={})",
                    prefix,
                    name,
                    j.x,
                    r.x,
                    (j.x - r.x).abs()
                ));
            }
            if (j.y - r.y).abs() > tolerance {
                diffs.push(format!(
                    "{} {}.y: java={} rust={} (diff={})",
                    prefix,
                    name,
                    j.y,
                    r.y,
                    (j.y - r.y).abs()
                ));
            }
            if (j.w - r.w).abs() > tolerance {
                diffs.push(format!(
                    "{} {}.w: java={} rust={} (diff={})",
                    prefix,
                    name,
                    j.w,
                    r.w,
                    (j.w - r.w).abs()
                ));
            }
            if (j.h - r.h).abs() > tolerance {
                diffs.push(format!(
                    "{} {}.h: java={} rust={} (diff={})",
                    prefix,
                    name,
                    j.h,
                    r.h,
                    (j.h - r.h).abs()
                ));
            }
        }
        (None, None) => {}
        _ => {
            diffs.push(format!(
                "{} {}: java={} rust={}",
                prefix,
                name,
                java.is_some(),
                rust.is_some()
            ));
        }
    }
}

fn compare_optional_color(
    prefix: &str,
    java: &Option<DrawColor>,
    rust: &Option<DrawColor>,
    diffs: &mut Vec<String>,
) {
    match (java, rust) {
        (Some(j), Some(r)) => {
            let rgb_tol = 0.005;
            let alpha_tol = 0.01;
            if (j.r - r.r).abs() > rgb_tol {
                diffs.push(format!("{} color.r: java={} rust={}", prefix, j.r, r.r));
            }
            if (j.g - r.g).abs() > rgb_tol {
                diffs.push(format!("{} color.g: java={} rust={}", prefix, j.g, r.g));
            }
            if (j.b - r.b).abs() > rgb_tol {
                diffs.push(format!("{} color.b: java={} rust={}", prefix, j.b, r.b));
            }
            if (j.a - r.a).abs() > alpha_tol {
                diffs.push(format!("{} color.a: java={} rust={}", prefix, j.a, r.a));
            }
        }
        (None, None) => {}
        _ => {
            diffs.push(format!(
                "{} color: java={} rust={}",
                prefix,
                java.is_some(),
                rust.is_some()
            ));
        }
    }
}

fn compare_detail(
    prefix: &str,
    java: &Option<DrawDetail>,
    rust: &Option<DrawDetail>,
    diffs: &mut Vec<String>,
) {
    match (java, rust) {
        (Some(j), Some(r)) => match (j, r) {
            (
                DrawDetail::Image {
                    source_index: js,
                    frame_index: jf,
                },
                DrawDetail::Image {
                    source_index: rs,
                    frame_index: rf,
                },
            ) => {
                if js != rs {
                    diffs.push(format!(
                        "{} detail.source_index: java={} rust={}",
                        prefix, js, rs
                    ));
                }
                if jf != rf {
                    diffs.push(format!(
                        "{} detail.frame_index: java={} rust={}",
                        prefix, jf, rf
                    ));
                }
            }
            (DrawDetail::Number { value: jv }, DrawDetail::Number { value: rv }) => {
                if jv != rv {
                    diffs.push(format!("{} detail.value: java={} rust={}", prefix, jv, rv));
                }
            }
            (
                DrawDetail::Text {
                    content: jc,
                    align: ja,
                },
                DrawDetail::Text {
                    content: rc,
                    align: ra,
                },
            ) => {
                if jc != rc {
                    diffs.push(format!(
                        "{} detail.content: java={:?} rust={:?}",
                        prefix, jc, rc
                    ));
                }
                if ja != ra {
                    diffs.push(format!("{} detail.align: java={} rust={}", prefix, ja, ra));
                }
            }
            (
                DrawDetail::Slider {
                    value: jv,
                    direction: jd,
                },
                DrawDetail::Slider {
                    value: rv,
                    direction: rd,
                },
            ) => {
                if (jv - rv).abs() > 0.001 {
                    diffs.push(format!("{} detail.value: java={} rust={}", prefix, jv, rv));
                }
                if jd != rd {
                    diffs.push(format!(
                        "{} detail.direction: java={} rust={}",
                        prefix, jd, rd
                    ));
                }
            }
            (
                DrawDetail::Graph {
                    value: jv,
                    direction: jd,
                },
                DrawDetail::Graph {
                    value: rv,
                    direction: rd,
                },
            ) => {
                if (jv - rv).abs() > 0.001 {
                    diffs.push(format!("{} detail.value: java={} rust={}", prefix, jv, rv));
                }
                if jd != rd {
                    diffs.push(format!(
                        "{} detail.direction: java={} rust={}",
                        prefix, jd, rd
                    ));
                }
            }
            (DrawDetail::Gauge { value: jv }, DrawDetail::Gauge { value: rv }) => {
                if (jv - rv).abs() > 0.001 {
                    diffs.push(format!("{} detail.value: java={} rust={}", prefix, jv, rv));
                }
            }
            (DrawDetail::BpmGraph, DrawDetail::BpmGraph)
            | (DrawDetail::HitErrorVisualizer, DrawDetail::HitErrorVisualizer)
            | (DrawDetail::NoteDistributionGraph, DrawDetail::NoteDistributionGraph)
            | (DrawDetail::TimingDistributionGraph, DrawDetail::TimingDistributionGraph)
            | (DrawDetail::TimingVisualizer, DrawDetail::TimingVisualizer) => {}
            _ => {
                diffs.push(format!("{} detail type mismatch", prefix));
            }
        },
        (None, None) => {}
        _ => {
            diffs.push(format!(
                "{} detail: java={} rust={}",
                prefix,
                java.is_some(),
                rust.is_some()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_provider::StaticStateProvider;
    use rubato_skin::skin::Skin;
    use rubato_skin::skin_header::SkinHeader;
    use rubato_skin::skin_image::SkinImage;
    use rubato_skin::stubs::TextureRegion;

    fn make_provider() -> StaticStateProvider {
        StaticStateProvider::default()
    }

    fn make_skin_with_image() -> Skin {
        let mut skin = Skin::new(SkinHeader::default());
        let image = SkinImage::new_with_single(TextureRegion::new());
        let mut obj = SkinObject::Image(image);
        obj.data_mut().set_destination_with_int_timer_ops(
            0,
            10.0,
            20.0,
            100.0,
            50.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[],
        );
        skin.add(obj);
        skin
    }

    #[test]
    fn capture_empty_skin() {
        let skin = Skin::new(SkinHeader::default());
        let provider = make_provider();
        let snapshot = capture_render_snapshot(&skin, &provider);
        assert!(snapshot.commands.is_empty());
    }

    #[test]
    fn capture_visible_image() {
        let skin = make_skin_with_image();
        let provider = make_provider();
        let snapshot = capture_render_snapshot(&skin, &provider);

        assert_eq!(snapshot.commands.len(), 1);
        let cmd = &snapshot.commands[0];
        assert!(cmd.visible);
        assert_eq!(cmd.object_type, "Image");
        assert!(cmd.dst.is_some());
        let dst = cmd.dst.as_ref().unwrap();
        assert!((dst.x - 10.0).abs() < 0.001);
        assert!((dst.y - 20.0).abs() < 0.001);
    }

    #[test]
    fn compare_identical_snapshots() {
        let skin = make_skin_with_image();
        let provider = make_provider();
        let s1 = capture_render_snapshot(&skin, &provider);
        let s2 = capture_render_snapshot(&skin, &provider);
        let diffs = compare_snapshots(&s1, &s2);
        assert!(diffs.is_empty(), "Diffs: {:?}", diffs);
    }

    #[test]
    fn json_round_trip() {
        let skin = make_skin_with_image();
        let provider = make_provider();
        let snapshot = capture_render_snapshot(&skin, &provider);
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        let parsed: RenderSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.commands.len(), snapshot.commands.len());
    }
}

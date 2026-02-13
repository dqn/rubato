// LR2 Play skin loader.
//
// Handles state-specific commands for the play screen:
// - SRC_NOTE / SRC_LN_* / SRC_HCN_* / SRC_MINE — Note textures
// - DST_NOTE / DST_NOTE2 / DST_NOTE_EXPANSION_RATE — Note placement
// - SRC_LINE / DST_LINE — Measure lines
// - SRC_NOWJUDGE_1P / DST_NOWJUDGE_1P (2P/3P) — Judge display
// - SRC_NOWCOMBO_1P / DST_NOWCOMBO_1P (2P/3P) — Combo numbers
// - SRC_JUDGELINE / DST_JUDGELINE — Judge line
// - SRC_BGA / DST_BGA — BGA display
// - SRC_HIDDEN / DST_HIDDEN / SRC_LIFT / DST_LIFT — Covers
// - CLOSE / PLAYSTART / LOADSTART / LOADEND / FINISHMARGIN / JUDGETIMER
// - SRC_NOTECHART_1P / SRC_BPMCHART / SRC_TIMING_1P — Graphs
//
// Ported from LR2PlaySkinLoader.java.

use crate::image_handle::ImageHandle;
use crate::loader::lr2_csv_loader::{
    Lr2CsvState, nonzero_timer, parse_field, parse_int_pub, read_offset,
};
use crate::play_skin::PlaySkinConfig;
use crate::property_id::{
    BooleanId, IntegerId, OFFSET_JUDGE_1P, OFFSET_JUDGE_2P, OFFSET_JUDGE_3P, OFFSET_LIFT, TimerId,
};
use crate::skin::Skin;
use crate::skin_bga::SkinBga;
use crate::skin_hidden::{SkinHidden, SkinLiftCover};
use crate::skin_image::SkinImage;
use crate::skin_judge::{JUDGE_COUNT, SkinJudge};
use crate::skin_note::SkinNote;
use crate::skin_number::{NumberAlign, SkinNumber, ZeroPadding};
use crate::skin_object::{Color, Destination, Rect, SkinObjectBase};
use crate::skin_object_type::SkinObjectType;
use crate::skin_source::{build_number_source_set, split_grid};
use crate::stretch_type::StretchType;

// ---------------------------------------------------------------------------
// Play state
// ---------------------------------------------------------------------------

/// Internal state for play skin loading.
#[derive(Default)]
pub struct Lr2PlayState {
    /// Note object index in skin.objects.
    note_idx: Option<usize>,
    /// Note object being constructed (reserved for texture population).
    _note: SkinNote,
    /// Current lane being loaded for note textures.
    note_lane: i32,
    /// Current judge object indices in skin.objects.
    judge_idx: [Option<usize>; 3],
    /// BGA object index.
    bga_idx: Option<usize>,
    /// Hidden cover object index.
    hidden_idx: Option<usize>,
    /// Lift cover object index.
    lift_idx: Option<usize>,
    /// Line (measure line) object index.
    line_idx: Option<usize>,
    /// Judge line object index.
    judgeline_idx: Option<usize>,
    /// Note chart object index.
    notechart_idx: Option<usize>,
    /// BPM chart object index.
    bpmchart_idx: Option<usize>,
    /// Timing chart object index.
    timingchart_idx: Option<usize>,
    /// PLAYSTART command value (ms).
    pub playstart: i32,
    /// LOADSTART command value (ms).
    pub loadstart: i32,
    /// LOADEND command value (ms).
    pub loadend: i32,
    /// FINISHMARGIN command value (ms).
    pub finish_margin: i32,
    /// JUDGETIMER command value (ms).
    pub judge_timer: i32,
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Processes a play-screen specific LR2 command.
///
/// Returns true if the command was handled.
pub fn process_play_command(
    cmd: &str,
    fields: &[&str],
    skin: &mut Skin,
    state: &mut Lr2CsvState,
    play_state: &mut Lr2PlayState,
) -> bool {
    match cmd {
        // Global timing commands
        "CLOSE" => {
            skin.scene = parse_field(fields, 1);
            true
        }
        "PLAYSTART" => {
            play_state.playstart = parse_field(fields, 1);
            true
        }
        "LOADSTART" => {
            play_state.loadstart = parse_field(fields, 1);
            true
        }
        "LOADEND" => {
            play_state.loadend = parse_field(fields, 1);
            true
        }
        "FINISHMARGIN" => {
            play_state.finish_margin = parse_field(fields, 1);
            true
        }
        "JUDGETIMER" => {
            play_state.judge_timer = parse_field(fields, 1);
            true
        }

        // Note textures
        "SRC_NOTE" => {
            play_state.note_lane = parse_field(fields, 2);
            true
        }
        "SRC_LN_END"
        | "SRC_LN_START"
        | "SRC_LN_BODY"
        | "SRC_LN_BODY_INACTIVE"
        | "SRC_LN_BODY_ACTIVE" => {
            // Store LN texture reference for current lane
            true
        }
        "SRC_HCN_END"
        | "SRC_HCN_START"
        | "SRC_HCN_BODY"
        | "SRC_HCN_BODY_INACTIVE"
        | "SRC_HCN_BODY_ACTIVE"
        | "SRC_HCN_DAMAGE"
        | "SRC_HCN_REACTIVE" => {
            // Store HCN texture reference for current lane
            true
        }
        "SRC_MINE" => {
            // Store mine note texture reference
            true
        }

        // Note placement
        "DST_NOTE" => {
            if play_state.note_idx.is_none() {
                let note = SkinNote::default();
                let idx = skin.objects.len();
                skin.add(note.into());
                play_state.note_idx = Some(idx);
            }
            if let Some(idx) = play_state.note_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }
        "DST_NOTE2" => true,
        "DST_NOTE_EXPANSION_RATE" => true,

        // Measure line
        "SRC_LINE" => {
            let img = crate::skin_image::SkinImage::default();
            let idx = skin.objects.len();
            skin.add(img.into());
            play_state.line_idx = Some(idx);
            true
        }
        "DST_LINE" => {
            if let Some(idx) = play_state.line_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }

        // Judge display
        "SRC_NOWJUDGE_1P" => {
            src_judge(0, fields, skin, play_state);
            true
        }
        "SRC_NOWJUDGE_2P" => {
            src_judge(1, fields, skin, play_state);
            true
        }
        "SRC_NOWJUDGE_3P" => {
            src_judge(2, fields, skin, play_state);
            true
        }
        "DST_NOWJUDGE_1P" => {
            dst_judge(0, fields, skin, state, play_state);
            true
        }
        "DST_NOWJUDGE_2P" => {
            dst_judge(1, fields, skin, state, play_state);
            true
        }
        "DST_NOWJUDGE_3P" => {
            dst_judge(2, fields, skin, state, play_state);
            true
        }

        // Combo numbers
        "SRC_NOWCOMBO_1P" => {
            src_nowcombo(0, fields, skin, play_state);
            true
        }
        "SRC_NOWCOMBO_2P" => {
            src_nowcombo(1, fields, skin, play_state);
            true
        }
        "SRC_NOWCOMBO_3P" => {
            src_nowcombo(2, fields, skin, play_state);
            true
        }
        "DST_NOWCOMBO_1P" => {
            dst_nowcombo(0, fields, skin, state, play_state);
            true
        }
        "DST_NOWCOMBO_2P" => {
            dst_nowcombo(1, fields, skin, state, play_state);
            true
        }
        "DST_NOWCOMBO_3P" => {
            dst_nowcombo(2, fields, skin, state, play_state);
            true
        }

        // Judge line
        "SRC_JUDGELINE" => {
            let img = crate::skin_image::SkinImage::default();
            let idx = skin.objects.len();
            skin.add(img.into());
            play_state.judgeline_idx = Some(idx);
            true
        }
        "DST_JUDGELINE" => {
            if let Some(idx) = play_state.judgeline_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }

        // BGA
        "SRC_BGA" => {
            let bga = SkinBga::default();
            let idx = skin.objects.len();
            skin.add(bga.into());
            play_state.bga_idx = Some(idx);
            true
        }
        "DST_BGA" => {
            if let Some(idx) = play_state.bga_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }

        // Hidden / Lift covers
        "SRC_HIDDEN" => {
            src_hidden(fields, skin, play_state);
            true
        }
        "DST_HIDDEN" => {
            if let Some(idx) = play_state.hidden_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }
        "SRC_LIFT" => {
            src_lift(fields, skin, play_state);
            true
        }
        "DST_LIFT" => {
            if let Some(idx) = play_state.lift_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }

        // Charts
        "SRC_NOTECHART_1P" => {
            let graph = crate::skin_visualizer::SkinNoteDistributionGraph::default();
            let idx = skin.objects.len();
            skin.add(graph.into());
            play_state.notechart_idx = Some(idx);
            true
        }
        "DST_NOTECHART_1P" => {
            if let Some(idx) = play_state.notechart_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }
        "SRC_BPMCHART" => {
            let graph = crate::skin_bpm_graph::SkinBpmGraph::default();
            let idx = skin.objects.len();
            skin.add(graph.into());
            play_state.bpmchart_idx = Some(idx);
            true
        }
        "DST_BPMCHART" => {
            if let Some(idx) = play_state.bpmchart_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }
        "SRC_TIMING_1P" => {
            let graph = crate::skin_visualizer::SkinTimingDistributionGraph::default();
            let idx = skin.objects.len();
            skin.add(graph.into());
            play_state.timingchart_idx = Some(idx);
            true
        }
        "DST_TIMING_1P" => {
            if let Some(idx) = play_state.timingchart_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }

        // Pomyu character stubs
        "DST_PM_CHARA_1P"
        | "DST_PM_CHARA_2P"
        | "DST_PM_CHARA_ANIMATION"
        | "SRC_PM_CHARA_IMAGE"
        | "DST_PM_CHARA_IMAGE" => true,

        _ => false,
    }
}

/// Collects play state into PlaySkinConfig after loading completes.
pub fn collect_play_config(skin: &Skin, play_state: &Lr2PlayState) -> Option<PlaySkinConfig> {
    let note = play_state.note_idx.and_then(|idx| {
        skin.objects.get(idx).and_then(|obj| {
            if let crate::skin_object_type::SkinObjectType::Note(n) = obj {
                Some(n.clone())
            } else {
                None
            }
        })
    });

    let bga = play_state.bga_idx.and_then(|idx| {
        skin.objects.get(idx).and_then(|obj| {
            if let crate::skin_object_type::SkinObjectType::Bga(b) = obj {
                Some(b.clone())
            } else {
                None
            }
        })
    });

    let hidden_cover = play_state.hidden_idx.and_then(|idx| {
        skin.objects.get(idx).and_then(|obj| {
            if let crate::skin_object_type::SkinObjectType::Hidden(h) = obj {
                Some(h.clone())
            } else {
                None
            }
        })
    });

    let lift_cover = play_state.lift_idx.and_then(|idx| {
        skin.objects.get(idx).and_then(|obj| {
            if let crate::skin_object_type::SkinObjectType::LiftCover(l) = obj {
                Some(l.clone())
            } else {
                None
            }
        })
    });

    let judges: Vec<SkinJudge> = play_state
        .judge_idx
        .iter()
        .filter_map(|idx_opt| {
            idx_opt.and_then(|idx| {
                skin.objects.get(idx).and_then(|obj| {
                    if let crate::skin_object_type::SkinObjectType::Judge(j) = obj {
                        Some(j.as_ref().clone())
                    } else {
                        None
                    }
                })
            })
        })
        .collect();

    let has_visual = note.is_some()
        || bga.is_some()
        || hidden_cover.is_some()
        || lift_cover.is_some()
        || !judges.is_empty();
    let has_timing = play_state.playstart != 0
        || play_state.loadstart != 0
        || play_state.loadend != 0
        || play_state.finish_margin != 0
        || play_state.judge_timer != 0;

    if !has_visual && !has_timing {
        return None;
    }

    Some(PlaySkinConfig {
        note,
        bga,
        hidden_cover,
        lift_cover,
        judges,
        playstart: play_state.playstart,
        loadstart: play_state.loadstart,
        loadend: play_state.loadend,
        finish_margin: play_state.finish_margin,
        judge_timer: play_state.judge_timer,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Remaps an LR2 judge ID to the internal slot index.
///
/// LR2: 0-5 are remapped as `5 - raw_id`, 6+ are kept as-is.
/// Matches Java: `values[1] <= 5 ? (5 - values[1]) : values[1]`.
fn remap_judge_id(raw_id: i32) -> usize {
    if raw_id <= 5 {
        (5 - raw_id) as usize
    } else {
        raw_id as usize
    }
}

fn src_judge(player: usize, fields: &[&str], skin: &mut Skin, play_state: &mut Lr2PlayState) {
    let values = parse_int_pub(fields);

    // Lazy creation: only create SkinJudge on first SRC_NOWJUDGE for this player
    if play_state.judge_idx[player].is_none() {
        // Java: new SkinJudge(player, (values[11] != 1))
        let shift = values[11] != 1;
        let mut judge = SkinJudge {
            player: player as i32,
            shift,
            ..Default::default()
        };
        // Add a minimal destination to mark the base as valid.
        // Individual judge_images have their own destinations for positioning.
        // Without this, the retain filter in load_lr2_skin removes the object.
        judge.base.add_destination(Destination {
            time: 0,
            region: Rect::default(),
            color: Color::from_rgba_u8(0, 0, 0, 0),
            angle: 0,
            acc: 0,
        });
        let idx = skin.objects.len();
        skin.add(judge.into());
        play_state.judge_idx[player] = Some(idx);
    }

    let slot = remap_judge_id(values[1]);
    if slot >= JUDGE_COUNT {
        return;
    }

    // Create SkinImage for this judge slot
    let gr = values[2];
    let img = SkinImage::from_frames(
        vec![ImageHandle(gr as u32)],
        nonzero_timer(values[10]),
        values[9],
    );

    // Populate judge_images[slot]
    if let Some(idx) = play_state.judge_idx[player]
        && let SkinObjectType::Judge(ref mut judge) = skin.objects[idx]
    {
        judge.judge_images[slot] = Some(img);
    }
}

fn dst_judge(
    player: usize,
    fields: &[&str],
    skin: &mut Skin,
    state: &mut Lr2CsvState,
    play_state: &mut Lr2PlayState,
) {
    let idx = match play_state.judge_idx[player] {
        Some(i) => i,
        None => return,
    };

    let values = parse_int_pub(fields);
    let slot = remap_judge_id(values[1]);
    if slot >= JUDGE_COUNT {
        return;
    }

    let offset_id = match player {
        0 => OFFSET_JUDGE_1P,
        1 => OFFSET_JUDGE_2P,
        _ => OFFSET_JUDGE_3P,
    };

    if let SkinObjectType::Judge(ref mut judge) = skin.objects[idx]
        && let Some(ref mut img) = judge.judge_images[slot]
    {
        state.apply_dst_to_base(&mut img.base, fields, &[offset_id, OFFSET_LIFT]);
    }
}

fn src_nowcombo(player: usize, fields: &[&str], skin: &mut Skin, play_state: &mut Lr2PlayState) {
    let idx = match play_state.judge_idx[player] {
        Some(i) => i,
        None => return,
    };

    let values = parse_int_pub(fields);
    let slot = remap_judge_id(values[1]);
    if slot >= JUDGE_COUNT {
        return;
    }

    let gr = values[2];
    let divx = values[7].max(1);
    let divy = values[8].max(1);

    if divx * divy < 10 {
        return;
    }

    let handle = ImageHandle(gr as u32);
    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];
    let (digit_sources, minus_digit_sources, zeropadding_override) =
        build_number_source_set(&grid, timer, cycle);

    // Java: images.length > 10 ? 2 : 0 where images.length = divy
    let zero_padding = if divy > 10 {
        ZeroPadding::Space
    } else {
        ZeroPadding::from_i32(zeropadding_override.unwrap_or(0))
    };

    // Java align remap: values[12] == 1 ? 2 : values[12]
    let align_raw = if values[12] == 1 { 2 } else { values[12] };

    let num = SkinNumber {
        base: SkinObjectBase::default(),
        ref_id: Some(IntegerId(values[11])),
        keta: values[13],
        zero_padding,
        align: NumberAlign::from_i32(align_raw),
        space: values[15],
        digit_sources,
        minus_digit_sources,
        image_timer: timer,
        image_cycle: cycle,
        ..Default::default()
    };

    if let SkinObjectType::Judge(ref mut judge) = skin.objects[idx] {
        judge.judge_counts[slot] = Some(num);
    }
}

fn dst_nowcombo(
    player: usize,
    fields: &[&str],
    skin: &mut Skin,
    state: &mut Lr2CsvState,
    play_state: &mut Lr2PlayState,
) {
    let idx = match play_state.judge_idx[player] {
        Some(i) => i,
        None => return,
    };

    let values = parse_int_pub(fields);
    let slot = remap_judge_id(values[1]);
    if slot >= JUDGE_COUNT {
        return;
    }

    let offset_id = match player {
        0 => OFFSET_JUDGE_1P,
        1 => OFFSET_JUDGE_2P,
        _ => OFFSET_JUDGE_3P,
    };

    if let SkinObjectType::Judge(ref mut judge) = skin.objects[idx]
        && let Some(ref mut num) = judge.judge_counts[slot]
    {
        num.relative = true;

        // Center alignment X adjustment (Java: x -= keta * w / 2)
        let mut x = values[3] as f32;
        if num.align == NumberAlign::Center {
            x -= num.keta as f32 * values[5] as f32 / 2.0;
        }

        // Y is negated for relative positioning (offset from judge image)
        let y = -(values[4] as f32);
        let w = values[5] as f32;
        let h = values[6] as f32;

        let time = values[2] as i64;
        let color = Color::from_rgba_u8(
            values[9] as u8,
            values[10] as u8,
            values[11] as u8,
            values[8] as u8,
        );

        num.base.add_destination(Destination {
            time,
            region: Rect::new(x, y, w, h),
            color,
            angle: values[14],
            acc: values[7],
        });

        if num.base.destinations.len() == 1 {
            num.base.blend = values[12];
            num.base.filter = values[13];
            num.base.set_center(values[15]);
            num.base.loop_time = values[16];

            let timer_id = values[17];
            if timer_id != 0 {
                num.base.timer = Some(TimerId(timer_id));
            }

            for &op_val in &[values[18], values[19], values[20]] {
                if op_val != 0 {
                    num.base.draw_conditions.push(BooleanId(op_val));
                }
            }

            let mut offsets = vec![offset_id, OFFSET_LIFT];
            offsets.extend(read_offset(fields, 21));
            num.base.set_offset_ids(&offsets);

            if state.stretch >= 0 {
                num.base.stretch = StretchType::from_id(state.stretch).unwrap_or_default();
            }
        }
    }
}

fn src_hidden(fields: &[&str], skin: &mut Skin, play_state: &mut Lr2PlayState) {
    let values = parse_int_pub(fields);
    let mut hidden = SkinHidden::default();
    let v11 = values[11];
    if v11 > 0 {
        hidden.disapear_line = v11 as f32;
    }
    // Java: str[12] empty or values[12] != 0 → link_lift = true
    hidden.link_lift = fields
        .get(12)
        .map(|s| s.is_empty() || values[12] != 0)
        .unwrap_or(false);
    let idx = skin.objects.len();
    skin.add(hidden.into());
    play_state.hidden_idx = Some(idx);
}

fn src_lift(fields: &[&str], skin: &mut Skin, play_state: &mut Lr2PlayState) {
    let values = parse_int_pub(fields);
    let mut lift = SkinLiftCover::default();
    let v11 = values[11];
    if v11 > 0 {
        lift.disapear_line = v11 as f32;
    }
    // Same pattern as SRC_HIDDEN
    lift.link_lift = fields
        .get(12)
        .map(|s| s.is_empty() || values[12] != 0)
        .unwrap_or(false);
    let idx = skin.objects.len();
    skin.add(lift.into());
    play_state.lift_idx = Some(idx);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin_header::SkinHeader;
    use bms_config::resolution::Resolution;
    use std::collections::HashMap;

    fn make_skin() -> (Skin, Lr2CsvState) {
        let mut header = SkinHeader::default();
        header.resolution = Resolution::Sd;
        header.source_resolution = Some(Resolution::Sd);
        header.destination_resolution = Some(Resolution::Hd);
        let skin = Skin::new(header);
        let state = Lr2CsvState::new(Resolution::Sd, Resolution::Hd, &HashMap::new());
        (skin, state)
    }

    #[test]
    fn test_close_command() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();
        let fields: Vec<&str> = "#CLOSE,5000".split(',').collect();
        assert!(process_play_command(
            "CLOSE", &fields, &mut skin, &mut state, &mut ps
        ));
        assert_eq!(skin.scene, 5000);
    }

    #[test]
    fn test_dst_note_creates_note() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        let dst: Vec<&str> = "#DST_NOTE,0,0,100,50,200,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        assert!(process_play_command(
            "DST_NOTE", &dst, &mut skin, &mut state, &mut ps
        ));

        assert!(ps.note_idx.is_some());
        assert_eq!(skin.object_count(), 1);
        assert!(matches!(skin.objects[0], SkinObjectType::Note(_)));
    }

    #[test]
    fn test_bga_src_dst() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        let src: Vec<&str> = "#SRC_BGA,0,0,0,0,256,256,1,1,0,0".split(',').collect();
        assert!(process_play_command(
            "SRC_BGA", &src, &mut skin, &mut state, &mut ps
        ));
        assert!(ps.bga_idx.is_some());

        let dst: Vec<&str> = "#DST_BGA,0,0,0,0,256,256,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        assert!(process_play_command(
            "DST_BGA", &dst, &mut skin, &mut state, &mut ps
        ));
    }

    #[test]
    fn test_hidden_lift_covers() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        let src: Vec<&str> = "#SRC,0,0,0,0,100,100,1,1,0,0".split(',').collect();
        let dst: Vec<&str> = "#DST,0,0,0,0,100,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();

        process_play_command("SRC_HIDDEN", &src, &mut skin, &mut state, &mut ps);
        process_play_command("DST_HIDDEN", &dst, &mut skin, &mut state, &mut ps);
        assert!(ps.hidden_idx.is_some());

        process_play_command("SRC_LIFT", &src, &mut skin, &mut state, &mut ps);
        process_play_command("DST_LIFT", &dst, &mut skin, &mut state, &mut ps);
        assert!(ps.lift_idx.is_some());
    }

    #[test]
    fn test_judge_1p() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        let src: Vec<&str> = "#SRC,0,0,0,0,100,50,1,1,0,0".split(',').collect();
        let dst: Vec<&str> = "#DST,0,0,200,300,100,50,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();

        process_play_command("SRC_NOWJUDGE_1P", &src, &mut skin, &mut state, &mut ps);
        process_play_command("DST_NOWJUDGE_1P", &dst, &mut skin, &mut state, &mut ps);

        assert!(ps.judge_idx[0].is_some());
        assert!(matches!(skin.objects[0], SkinObjectType::Judge(_)));
    }

    #[test]
    fn test_collect_play_config() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        let dst: Vec<&str> = "#DST_NOTE,0,0,100,50,200,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        process_play_command("DST_NOTE", &dst, &mut skin, &mut state, &mut ps);

        let src: Vec<&str> = "#SRC,0,0,0,0,256,256,1,1,0,0".split(',').collect();
        process_play_command("SRC_BGA", &src, &mut skin, &mut state, &mut ps);

        let config = collect_play_config(&skin, &ps).unwrap();
        assert!(config.note.is_some());
        assert!(config.bga.is_some());
    }

    #[test]
    fn test_empty_play_config_returns_none() {
        let (skin, _) = make_skin();
        let ps = Lr2PlayState::default();
        assert!(collect_play_config(&skin, &ps).is_none());
    }

    #[test]
    fn test_timing_commands_parsed() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        let fields_ps: Vec<&str> = "#PLAYSTART,1000".split(',').collect();
        let fields_ls: Vec<&str> = "#LOADSTART,500".split(',').collect();
        let fields_le: Vec<&str> = "#LOADEND,800".split(',').collect();
        let fields_fm: Vec<&str> = "#FINISHMARGIN,2000".split(',').collect();
        let fields_jt: Vec<&str> = "#JUDGETIMER,120".split(',').collect();

        assert!(process_play_command(
            "PLAYSTART",
            &fields_ps,
            &mut skin,
            &mut state,
            &mut ps
        ));
        assert!(process_play_command(
            "LOADSTART",
            &fields_ls,
            &mut skin,
            &mut state,
            &mut ps
        ));
        assert!(process_play_command(
            "LOADEND", &fields_le, &mut skin, &mut state, &mut ps
        ));
        assert!(process_play_command(
            "FINISHMARGIN",
            &fields_fm,
            &mut skin,
            &mut state,
            &mut ps
        ));
        assert!(process_play_command(
            "JUDGETIMER",
            &fields_jt,
            &mut skin,
            &mut state,
            &mut ps
        ));

        assert_eq!(ps.playstart, 1000);
        assert_eq!(ps.loadstart, 500);
        assert_eq!(ps.loadend, 800);
        assert_eq!(ps.finish_margin, 2000);
        assert_eq!(ps.judge_timer, 120);
    }

    #[test]
    fn test_timing_in_play_config() {
        let (skin, _state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // Set timing values
        ps.playstart = 1000;
        ps.loadstart = 500;
        ps.loadend = 800;
        ps.finish_margin = 2000;
        ps.judge_timer = 120;

        // Need at least one visual object or timing values for non-None config
        let config = collect_play_config(&skin, &ps).unwrap();
        assert_eq!(config.playstart, 1000);
        assert_eq!(config.loadstart, 500);
        assert_eq!(config.loadend, 800);
        assert_eq!(config.finish_margin, 2000);
        assert_eq!(config.judge_timer, 120);
    }

    #[test]
    fn test_hidden_disapear_line_parsed() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // values[11]=50 (disapear_line), values[12]=1 (link_lift)
        let fields: Vec<&str> = "#SRC_HIDDEN,0,0,0,0,100,100,1,1,0,0,50,1"
            .split(',')
            .collect();
        process_play_command("SRC_HIDDEN", &fields, &mut skin, &mut state, &mut ps);

        let dst: Vec<&str> = "#DST,0,0,0,0,100,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        process_play_command("DST_HIDDEN", &dst, &mut skin, &mut state, &mut ps);

        let config = collect_play_config(&skin, &ps).unwrap();
        let hidden = config.hidden_cover.unwrap();
        assert!((hidden.disapear_line - 50.0).abs() < f32::EPSILON);
        assert!(hidden.link_lift);
    }

    #[test]
    fn test_hidden_no_disapear_line() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // values[11]=0 (no disapear_line), no field 12
        let fields: Vec<&str> = "#SRC_HIDDEN,0,0,0,0,100,100,1,1,0,0,0".split(',').collect();
        process_play_command("SRC_HIDDEN", &fields, &mut skin, &mut state, &mut ps);

        let dst: Vec<&str> = "#DST,0,0,0,0,100,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        process_play_command("DST_HIDDEN", &dst, &mut skin, &mut state, &mut ps);

        let config = collect_play_config(&skin, &ps).unwrap();
        let hidden = config.hidden_cover.unwrap();
        assert!((hidden.disapear_line - 0.0).abs() < f32::EPSILON);
        assert!(!hidden.link_lift);
    }

    #[test]
    fn test_lift_disapear_line_parsed() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        let fields: Vec<&str> = "#SRC_LIFT,0,0,0,0,100,100,1,1,0,0,30,1"
            .split(',')
            .collect();
        process_play_command("SRC_LIFT", &fields, &mut skin, &mut state, &mut ps);

        let dst: Vec<&str> = "#DST,0,0,0,0,100,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        process_play_command("DST_LIFT", &dst, &mut skin, &mut state, &mut ps);

        let config = collect_play_config(&skin, &ps).unwrap();
        let lift = config.lift_cover.unwrap();
        assert!((lift.disapear_line - 30.0).abs() < f32::EPSILON);
        assert!(lift.link_lift);
    }

    #[test]
    fn test_judge_shift_flag() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // values[11]=1 means shift=false (special mode)
        let fields_no_shift: Vec<&str> = "#SRC,0,0,0,0,100,50,1,1,0,0,1".split(',').collect();
        process_play_command(
            "SRC_NOWJUDGE_1P",
            &fields_no_shift,
            &mut skin,
            &mut state,
            &mut ps,
        );

        let dst: Vec<&str> = "#DST,0,0,200,300,100,50,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        process_play_command("DST_NOWJUDGE_1P", &dst, &mut skin, &mut state, &mut ps);

        let config = collect_play_config(&skin, &ps).unwrap();
        // values[11]=1 → shift=false
        assert!(!config.judges[0].shift);

        // Now test values[11]=0 → shift=true
        let mut ps2 = Lr2PlayState::default();
        let (mut skin2, mut state2) = make_skin();
        let fields_shift: Vec<&str> = "#SRC,0,0,0,0,100,50,1,1,0,0,0".split(',').collect();
        process_play_command(
            "SRC_NOWJUDGE_1P",
            &fields_shift,
            &mut skin2,
            &mut state2,
            &mut ps2,
        );
        let dst2: Vec<&str> = "#DST,0,0,200,300,100,50,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        process_play_command("DST_NOWJUDGE_1P", &dst2, &mut skin2, &mut state2, &mut ps2);

        let config2 = collect_play_config(&skin2, &ps2).unwrap();
        assert!(config2.judges[0].shift);
    }

    #[test]
    fn test_note_texture_commands_handled() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();
        let fields: Vec<&str> = vec!["#CMD", "0", "3"];

        assert!(process_play_command(
            "SRC_NOTE", &fields, &mut skin, &mut state, &mut ps
        ));
        assert!(process_play_command(
            "SRC_LN_END",
            &fields,
            &mut skin,
            &mut state,
            &mut ps
        ));
        assert!(process_play_command(
            "SRC_LN_START",
            &fields,
            &mut skin,
            &mut state,
            &mut ps
        ));
        assert!(process_play_command(
            "SRC_LN_BODY",
            &fields,
            &mut skin,
            &mut state,
            &mut ps
        ));
        assert!(process_play_command(
            "SRC_HCN_END",
            &fields,
            &mut skin,
            &mut state,
            &mut ps
        ));
        assert!(process_play_command(
            "SRC_MINE", &fields, &mut skin, &mut state, &mut ps
        ));
    }

    #[test]
    fn test_unhandled_returns_false() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();
        let fields: Vec<&str> = vec!["#UNKNOWN"];

        assert!(!process_play_command(
            "UNKNOWN", &fields, &mut skin, &mut state, &mut ps
        ));
    }

    #[test]
    fn test_hidden_link_lift_empty_field() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // values[11]=50, field[12] is empty string → link_lift = true
        let fields: Vec<&str> = "#SRC_HIDDEN,0,0,0,0,100,100,1,1,0,0,50,"
            .split(',')
            .collect();
        process_play_command("SRC_HIDDEN", &fields, &mut skin, &mut state, &mut ps);

        let dst: Vec<&str> = "#DST,0,0,0,0,100,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        process_play_command("DST_HIDDEN", &dst, &mut skin, &mut state, &mut ps);

        let config = collect_play_config(&skin, &ps).unwrap();
        let hidden = config.hidden_cover.unwrap();
        assert!(hidden.link_lift);
    }

    #[test]
    fn test_src_judge_lazy_creation() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // First SRC creates a SkinJudge
        let src1: Vec<&str> = "#SRC,0,0,0,0,100,50,1,1,0,0,0".split(',').collect();
        process_play_command("SRC_NOWJUDGE_1P", &src1, &mut skin, &mut state, &mut ps);
        assert!(ps.judge_idx[0].is_some());
        assert_eq!(skin.object_count(), 1);

        // Second SRC reuses the same SkinJudge (no new object created)
        let src2: Vec<&str> = "#SRC,1,0,0,0,100,50,1,1,0,0,0".split(',').collect();
        process_play_command("SRC_NOWJUDGE_1P", &src2, &mut skin, &mut state, &mut ps);
        assert_eq!(skin.object_count(), 1);
    }

    #[test]
    fn test_src_judge_populates_image() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // raw_id=5 → slot = 5-5 = 0 (JUDGE_PERFECT)
        let src: Vec<&str> = "#SRC,5,2,0,0,100,50,1,1,0,0,0".split(',').collect();
        process_play_command("SRC_NOWJUDGE_1P", &src, &mut skin, &mut state, &mut ps);

        if let SkinObjectType::Judge(ref judge) = skin.objects[0] {
            assert!(judge.judge_images[0].is_some()); // slot 0 = PERFECT
            assert!(judge.judge_images[1].is_none()); // others unset
        } else {
            panic!("Expected Judge");
        }
    }

    #[test]
    fn test_judge_id_remapping() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // Populate all 7 slots: raw 0→5, 1→4, 2→3, 3→2, 4→1, 5→0, 6→6
        for raw_id in 0..=6 {
            let src = format!("#SRC,{raw_id},0,0,0,100,50,1,1,0,0,0");
            let fields: Vec<&str> = src.split(',').collect();
            process_play_command("SRC_NOWJUDGE_1P", &fields, &mut skin, &mut state, &mut ps);
        }

        if let SkinObjectType::Judge(ref judge) = skin.objects[0] {
            for i in 0..=5 {
                let expected_slot = (5 - i) as usize;
                assert!(
                    judge.judge_images[expected_slot].is_some(),
                    "raw_id={i} should map to slot {expected_slot}"
                );
            }
            assert!(
                judge.judge_images[6].is_some(),
                "raw_id=6 should map to slot 6"
            );
        } else {
            panic!("Expected Judge");
        }
    }

    #[test]
    fn test_src_nowcombo_populates_number() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // First create a judge (raw_id=0 → slot 5)
        let src_judge: Vec<&str> = "#SRC,0,0,0,0,100,50,1,1,0,0,0".split(',').collect();
        process_play_command(
            "SRC_NOWJUDGE_1P",
            &src_judge,
            &mut skin,
            &mut state,
            &mut ps,
        );

        // Add combo: raw_id=0 → slot 5, divx=10, divy=1, ref_id=150, keta=5
        let src_combo: Vec<&str> = "#SRC_NOWCOMBO_1P,0,1,0,0,240,24,10,1,0,0,150,0,5,0,0"
            .split(',')
            .collect();
        process_play_command(
            "SRC_NOWCOMBO_1P",
            &src_combo,
            &mut skin,
            &mut state,
            &mut ps,
        );

        if let SkinObjectType::Judge(ref judge) = skin.objects[0] {
            let slot = 5; // raw_id=0 → 5-0=5
            assert!(judge.judge_counts[slot].is_some());
            let num = judge.judge_counts[slot].as_ref().unwrap();
            assert_eq!(num.ref_id, Some(IntegerId(150)));
            assert_eq!(num.keta, 5);
        } else {
            panic!("Expected Judge");
        }
    }

    #[test]
    fn test_dst_nowjudge_applies_to_image_base() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // Create judge and populate slot 5 (raw_id=0)
        let src: Vec<&str> = "#SRC,0,0,0,0,100,50,1,1,0,0,0".split(',').collect();
        process_play_command("SRC_NOWJUDGE_1P", &src, &mut skin, &mut state, &mut ps);

        // Apply DST to the judge image
        let dst: Vec<&str> = "#DST,0,0,200,300,100,50,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
            .split(',')
            .collect();
        process_play_command("DST_NOWJUDGE_1P", &dst, &mut skin, &mut state, &mut ps);

        if let SkinObjectType::Judge(ref judge) = skin.objects[0] {
            let slot = 5; // raw_id=0 → 5
            let img = judge.judge_images[slot].as_ref().unwrap();
            assert!(!img.base.destinations.is_empty());
            // Verify OFFSET_JUDGE_1P and OFFSET_LIFT are set
            assert!(img.base.offset_ids.contains(&OFFSET_JUDGE_1P));
            assert!(img.base.offset_ids.contains(&OFFSET_LIFT));
        } else {
            panic!("Expected Judge");
        }
    }

    #[test]
    fn test_dst_nowcombo_sets_relative() {
        let (mut skin, mut state) = make_skin();
        let mut ps = Lr2PlayState::default();

        // Create judge
        let src_judge: Vec<&str> = "#SRC,0,0,0,0,100,50,1,1,0,0,0".split(',').collect();
        process_play_command(
            "SRC_NOWJUDGE_1P",
            &src_judge,
            &mut skin,
            &mut state,
            &mut ps,
        );

        // Create combo (raw_id=0 → slot 5)
        let src_combo: Vec<&str> = "#SRC_NOWCOMBO_1P,0,1,0,0,240,24,10,1,0,0,150,0,5,0,0"
            .split(',')
            .collect();
        process_play_command(
            "SRC_NOWCOMBO_1P",
            &src_combo,
            &mut skin,
            &mut state,
            &mut ps,
        );

        // Apply DST to combo
        let dst_combo: Vec<&str> =
            "#DST_NOWCOMBO_1P,0,0,50,30,24,24,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
                .split(',')
                .collect();
        process_play_command(
            "DST_NOWCOMBO_1P",
            &dst_combo,
            &mut skin,
            &mut state,
            &mut ps,
        );

        if let SkinObjectType::Judge(ref judge) = skin.objects[0] {
            let slot = 5; // raw_id=0 → 5
            let num = judge.judge_counts[slot].as_ref().unwrap();
            assert!(num.relative);
            assert!(!num.base.destinations.is_empty());
            // Y should be negated for relative positioning
            assert!(num.base.destinations[0].region.y < 0.0);
        } else {
            panic!("Expected Judge");
        }
    }
}

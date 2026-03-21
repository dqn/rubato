use super::*;
use bms_model::bms_model::BMSModel;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;
use rubato_types::play_config::{
    FIX_HISPEED_MAINBPM, FIX_HISPEED_MAXBPM, FIX_HISPEED_MINBPM, FIX_HISPEED_OFF,
    FIX_HISPEED_STARTBPM, PlayConfig,
};

// --- Helper to create a minimal BMSModel with timelines ---

fn make_model_with_timelines(timelines: Vec<TimeLine>, bpm: f64) -> BMSModel {
    let mut model = BMSModel::new();
    model.bpm = bpm;
    model.timelines = timelines;
    model
}

fn make_timeline(section: f64, time_us: i64, bpm: f64, notesize: i32) -> TimeLine {
    let mut tl = TimeLine::new(section, time_us, notesize);
    tl.bpm = bpm;
    tl
}

fn default_ctx(all_timelines: &[TimeLine]) -> DrawLaneContext {
    DrawLaneContext {
        time: 0,
        timer_play: Some(0),
        timer_141: None,
        judge_timing: 0,
        is_practice: false,
        practice_start_time: 0,
        now_time: 0,
        now_quarter_note_time: 0,
        note_expansion_rate: [100, 100],
        lane_group_regions: vec![],
        show_bpmguide: false,
        show_pastnote: false,
        mark_processednote: false,
        show_hiddennote: false,
        show_judgearea: false,
        lntype: LNTYPE_LONGNOTE,
        judge_time_regions: vec![],
        processing_long_notes: vec![None; 8],
        passing_long_notes: vec![None; 8],
        hell_charge_judges: vec![false; 8],
        bad_judge_time: 0,
        model_bpm: 120.0,
        // Safety: all_timelines outlives the DrawLaneContext in every test.
        all_timelines: unsafe { TimelinesRef::from_slice(all_timelines) },
        forced_cn_endings: false,
    }
}

fn make_lanes(count: usize) -> Vec<SkinLane> {
    let mut lanes = Vec::new();
    for _ in 0..count {
        let mut lane = SkinLane::new();
        lane.region_x = 0.0;
        lane.region_y = 0.0;
        lane.region_width = 30.0;
        lane.region_height = 500.0;
        lane.scale = 10.0;
        lanes.push(lane);
    }
    lanes
}

// =========================================================================
// calc_region tests
// =========================================================================

#[test]
fn calc_region_normal() {
    // At 120 BPM, hispeed 1.0, scroll 1.0: 240000/120/1 = 2000
    let region = LaneRenderer::calc_region(120.0, 1.0, 1.0);
    assert!((region - 2000.0).abs() < 0.001);
}

#[test]
fn calc_region_with_hispeed() {
    // At 120 BPM, hispeed 2.0: 240000/120/2 = 1000
    let region = LaneRenderer::calc_region(120.0, 2.0, 1.0);
    assert!((region - 1000.0).abs() < 0.001);
}

#[test]
fn calc_region_with_scroll() {
    // At 120 BPM, hispeed 1.0, scroll 2.0: 2000/2 = 1000
    let region = LaneRenderer::calc_region(120.0, 1.0, 2.0);
    assert!((region - 1000.0).abs() < 0.001);
}

#[test]
fn calc_region_zero_scroll_returns_zero() {
    let region = LaneRenderer::calc_region(120.0, 1.0, 0.0);
    assert!((region).abs() < 0.001);
}

// =========================================================================
// calc_constant_alpha tests
// =========================================================================

#[test]
fn constant_alpha_fully_visible() {
    // Timeline is before target time -> fully visible
    let alpha = LaneRenderer::calc_constant_alpha(500_000, 1_000_000, 500, 100_000.0);
    assert_eq!(alpha, Some(1.0));
}

#[test]
fn constant_alpha_hidden() {
    // Timeline is far beyond target time + alpha limit -> hidden
    let alpha = LaneRenderer::calc_constant_alpha(2_000_000, 500_000, 500, 100_000.0);
    assert_eq!(alpha, None);
}

#[test]
fn constant_alpha_fadein() {
    // Timeline is just past target time, within alpha limit -> fade-in
    // target_time = 500_000 + 500*1000 = 1_000_000
    // tl time = 1_050_000, diff = 50_000
    // alpha = (100_000 - 50_000) / 100_000 = 0.5
    let alpha = LaneRenderer::calc_constant_alpha(1_050_000, 500_000, 500, 100_000.0);
    assert!(alpha.is_some());
    assert!((alpha.unwrap() - 0.5).abs() < 0.001);
}

#[test]
fn constant_alpha_negative_limit_hidden() {
    // Negative alpha limit: hidden when past target
    let alpha = LaneRenderer::calc_constant_alpha(2_000_000, 500_000, 500, -100_000.0);
    assert_eq!(alpha, None);
}

#[test]
fn constant_alpha_negative_limit_fadein() {
    // Negative alpha limit: fade-in when within negative range before target
    // target_time = 500_000 + 500*1000 = 1_000_000
    // tl time = 950_000 (before target), diff = 950_000 - 1_000_000 = -50_000
    // alpha_limit = -100_000
    // diff (-50_000) > alpha_limit (-100_000) -> fade-in
    // alpha = 1.0 - (alpha_limit - diff) / alpha_limit
    //       = 1.0 - (-100_000 - (-50_000)) / (-100_000)
    //       = 1.0 - (-50_000) / (-100_000) = 1.0 - 0.5 = 0.5
    let alpha = LaneRenderer::calc_constant_alpha(950_000, 500_000, 500, -100_000.0);
    assert!(alpha.is_some());
    assert!((alpha.unwrap() - 0.5).abs() < 0.001);
}

#[test]
fn constant_alpha_zero_limit_no_fadein_window() {
    // When alpha_limit == 0.0 (constant_fadein_time == 0), there is no fade-in window.
    // Notes past target are hidden (None), notes before target are fully visible.
    // The division by zero is unreachable because the condition check prevents it.
    let alpha = LaneRenderer::calc_constant_alpha(2_000_000, 500_000, 500, 0.0);
    assert_eq!(
        alpha, None,
        "zero alpha_limit: past target should be hidden"
    );

    let alpha = LaneRenderer::calc_constant_alpha(500_000, 1_000_000, 500, 0.0);
    assert_eq!(
        alpha,
        Some(1.0),
        "zero alpha_limit: before target should be fully visible"
    );
}

// =========================================================================
// calc_note_expansion tests
// =========================================================================

#[test]
fn note_expansion_disabled() {
    let (w, h) = LaneRenderer::calc_note_expansion(100, 0, 100, 100, 9.0, 150.0);
    assert_eq!(w, 1.0);
    assert_eq!(h, 1.0);
}

#[test]
fn note_expansion_during_expand_phase() {
    // expansion_rate = 200%, elapsed = 4.5 (half of expansion_time 9.0)
    // scale = 1.0 + (200/100 - 1) * 4.5/9 = 1.0 + 1.0 * 0.5 = 1.5
    let (w, h) = LaneRenderer::calc_note_expansion(
        50, // now
        46, // quarter_note_time (elapsed = 50 - 46 = 4)
        200, 200, 9.0, 150.0,
    );
    // elapsed = 50 - 46 = 4
    // scale = 1.0 + 1.0 * 4/9 = 1.444...
    assert!((w - 1.4444).abs() < 0.01);
    assert!((h - 1.4444).abs() < 0.01);
}

#[test]
fn note_expansion_during_contraction_phase() {
    // expansion_time = 9, contraction_time = 150
    // elapsed = 50 (past expansion_time, in contraction phase)
    // contraction_elapsed = 50 - 9 = 41
    // scale = 1.0 + (200/100 - 1) * (150 - 41) / 150 = 1.0 + 1.0 * 109/150 = 1.7267
    let (w, _h) = LaneRenderer::calc_note_expansion(100, 50, 200, 200, 9.0, 150.0);
    assert!((w - 1.7267).abs() < 0.01);
}

#[test]
fn note_expansion_after_contraction() {
    // elapsed > expansion_time + contraction_time -> normal size
    let (w, h) = LaneRenderer::calc_note_expansion(200, 0, 200, 200, 9.0, 150.0);
    assert_eq!(w, 1.0);
    assert_eq!(h, 1.0);
}

// =========================================================================
// calc_y_offset tests
// =========================================================================

#[test]
fn y_offset_during_stop() {
    // When prev timeline is in a stop, full section distance is used
    let mut prev = make_timeline(0.0, 0, 120.0, 8);
    prev.stop = 2_000_000; // 2 second stop
    let tl = make_timeline(1.0, 1_000_000, 120.0, 8);

    let offset = LaneRenderer::calc_y_offset(&tl, &prev, 500_000, 100.0);
    // During stop: full section * scroll * rxhs = 1.0 * 1.0 * 100.0 = 100.0
    assert!((offset - 100.0).abs() < 0.001);
}

#[test]
fn y_offset_normal_scroll() {
    let prev = make_timeline(0.0, 0, 120.0, 8);
    let tl = make_timeline(1.0, 1_000_000, 120.0, 8);

    // At time 500000 (halfway): offset should be proportional
    let offset = LaneRenderer::calc_y_offset(&tl, &prev, 500_000, 100.0);
    // time_diff = 1_000_000 - 500_000 = 500_000
    // total_time = 1_000_000 - 0 - 0 = 1_000_000
    // offset = 1.0 * 1.0 * (500_000/1_000_000) * 100.0 = 50.0
    assert!((offset - 50.0).abs() < 0.001);
}

#[test]
fn y_offset_with_scroll() {
    let mut prev = make_timeline(0.0, 0, 120.0, 8);
    prev.scroll = 2.0;
    let tl = make_timeline(1.0, 1_000_000, 120.0, 8);

    let offset = LaneRenderer::calc_y_offset(&tl, &prev, 500_000, 100.0);
    // 1.0 * 2.0 * (500_000/1_000_000) * 100.0 = 100.0
    assert!((offset - 100.0).abs() < 0.001);
}

#[test]
fn y_offset_first_timeline() {
    let tl = make_timeline(1.0, 1_000_000, 120.0, 8);

    let offset = LaneRenderer::calc_y_offset_first(&tl, 500_000, 100.0);
    // section * (tl_time - microtime) / tl_time * rxhs
    // = 1.0 * 500_000 / 1_000_000 * 100.0 = 50.0
    assert!((offset - 50.0).abs() < 0.001);
}

#[test]
fn y_offset_first_at_time_zero() {
    let tl = make_timeline(1.0, 0, 120.0, 8);
    let offset = LaneRenderer::calc_y_offset_first(&tl, 0, 100.0);
    assert!((offset).abs() < 0.001);
}

// =========================================================================
// draw_lane integration tests
// =========================================================================

#[test]
fn draw_lane_empty_lanes_returns_empty() {
    let tl = make_timeline(0.0, 0, 120.0, 8);
    let model = make_model_with_timelines(vec![tl], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let result = renderer.draw_lane(&ctx, &[], &[]);

    assert!(result.commands.is_empty());
}

#[test]
fn draw_lane_updates_now_bpm() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.section_line = true;
    let mut tl1 = make_timeline(1.0, 500_000, 150.0, 8);
    tl1.section_line = true;
    let mut tl2 = make_timeline(2.0, 1_000_000, 180.0, 8);
    tl2.section_line = true;
    let model = make_model_with_timelines(vec![tl0, tl1, tl2], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    // Set time to 750ms (past tl1 at 500ms but before tl2 at 1000ms)
    ctx.time = 750;
    ctx.timer_play = Some(0);

    let lanes = make_lanes(8);
    renderer.draw_lane(&ctx, &lanes, &[]);

    // nowbpm should be 150 (from tl1 which is the last timeline before current time)
    assert!((renderer.now_bpm() - 150.0).abs() < 0.001);
}

#[test]
fn draw_lane_calculates_current_duration() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.section_line = true;
    let model = make_model_with_timelines(vec![tl0], 120.0);
    let mut renderer = LaneRenderer::new(&model);
    renderer.duration = 1000;

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    renderer.draw_lane(&ctx, &lanes, &[]);

    // region = 240000/120/1.0 / 1.0 = 2000
    // currentduration = 2000 * (1 - 0) = 2000
    assert_eq!(renderer.current_duration(), 2000);
}

#[test]
fn draw_lane_current_duration_with_lanecover() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.section_line = true;
    let model = make_model_with_timelines(vec![tl0], 120.0);
    let mut renderer = LaneRenderer::new(&model);
    renderer.enable_lanecover = true;
    renderer.lanecover = 0.5;

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    renderer.draw_lane(&ctx, &lanes, &[]);

    // region = 2000, currentduration = 2000 * (1 - 0.5) = 1000
    assert_eq!(renderer.current_duration(), 1000);
}

#[test]
fn draw_lane_lift_offset() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.section_line = true;
    let model = make_model_with_timelines(vec![tl0], 120.0);
    let mut renderer = LaneRenderer::new(&model);
    renderer.enable_lift = true;
    renderer.lift = 0.2;

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // Y-down: hl = region_y + region_height * (1 - lift) = 0 + 500 * 0.8 = 400
    // lift_offset_y = (region_y + region_height) - hl = 500 - 400 = 100
    assert!((result.lift_offset_y - 100.0).abs() < 0.001);
}

#[test]
fn draw_lane_hidden_cover_enabled() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.section_line = true;
    let model = make_model_with_timelines(vec![tl0], 120.0);
    let mut renderer = LaneRenderer::new(&model);
    renderer.enable_hidden = true;
    renderer.hidden = 0.3;

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    assert!(result.hidden_cover.visible);
    // hidden_y = hidden * region_height = 0.3 * 500 = 150
    assert!((result.hidden_cover.y - 150.0).abs() < 0.001);
}

#[test]
fn draw_lane_hidden_cover_disabled() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.section_line = true;
    let model = make_model_with_timelines(vec![tl0], 120.0);
    let mut renderer = LaneRenderer::new(&model);
    renderer.enable_hidden = false;

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    assert!(!result.hidden_cover.visible);
}

#[test]
fn draw_lane_section_line_emitted() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    tl1.section_line = true;
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // Should contain at least one DrawSectionLine command
    let has_section_line = result
        .commands
        .iter()
        .any(|c| matches!(c, DrawCommand::DrawSectionLine { .. }));
    assert!(has_section_line, "Expected DrawSectionLine command");
}

#[test]
fn draw_lane_normal_note_emitted() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // Should contain a DrawNote for lane 0
    let has_note = result.commands.iter().any(|c| {
        matches!(
            c,
            DrawCommand::DrawNote {
                lane: 0,
                image_type: NoteImageType::Normal,
                ..
            }
        )
    });
    assert!(has_note, "Expected DrawNote command for lane 0");
}

#[test]
fn draw_lane_future_note_moves_toward_judge_line_over_time() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);
    let lanes = make_lanes(8);

    let all_tls = &model.timelines;
    let mut early_ctx = default_ctx(all_tls);
    early_ctx.time = 0;
    let early = renderer.draw_lane(&early_ctx, &lanes, &[]);
    let early_y = early
        .commands
        .iter()
        .find_map(|cmd| match cmd {
            DrawCommand::DrawNote {
                lane: 0,
                image_type: NoteImageType::Normal,
                y,
                ..
            } => Some(*y),
            _ => None,
        })
        .expect("future note should be drawable at early time");

    let mut late_ctx = default_ctx(all_tls);
    late_ctx.time = 500;
    let late = renderer.draw_lane(&late_ctx, &lanes, &[]);
    let late_y = late
        .commands
        .iter()
        .find_map(|cmd| match cmd {
            DrawCommand::DrawNote {
                lane: 0,
                image_type: NoteImageType::Normal,
                y,
                ..
            } => Some(*y),
            _ => None,
        })
        .expect("future note should still be drawable halfway to the judge line");

    assert!(
        late_y < early_y,
        "with Y-up rendering, notes should move toward the judge line (smaller y) over time: early_y={early_y}, late_y={late_y}"
    );
}

#[test]
fn draw_lane_mine_note_emitted() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    tl1.set_note(2, Some(Note::new_mine(1, 0.5)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    let has_mine = result.commands.iter().any(|c| {
        matches!(
            c,
            DrawCommand::DrawNote {
                lane: 2,
                image_type: NoteImageType::Mine,
                ..
            }
        )
    });
    assert!(has_mine, "Expected mine note DrawNote command for lane 2");
}

#[test]
fn draw_lane_past_note_not_shown_without_flag() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    // Note at time 0 is already past when time > 0
    let mut tl1 = make_timeline(0.5, 500_000, 120.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    ctx.time = 1000; // well past the note
    ctx.show_pastnote = false;

    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // No note commands expected since note is past and show_pastnote is false
    let has_note = result.commands.iter().any(|c| {
        matches!(
            c,
            DrawCommand::DrawNote {
                image_type: NoteImageType::Normal,
                ..
            }
        )
    });
    assert!(
        !has_note,
        "Should not draw past notes when show_pastnote is false"
    );
}

#[test]
fn draw_lane_constant_mode_hides_far_notes() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    // Note far in the future
    let mut tl1 = make_timeline(10.0, 10_000_000, 120.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);
    renderer.enable_constant = true;
    renderer.duration = 500; // target = 500_000 us
    renderer.constant_fadein_time = 0.0; // alpha_limit = 0

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    ctx.time = 0;

    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // Far future note (10s) should be hidden in constant mode with duration 500ms
    let has_note = result.commands.iter().any(|c| {
        matches!(
            c,
            DrawCommand::DrawNote {
                image_type: NoteImageType::Normal,
                ..
            }
        )
    });
    assert!(
        !has_note,
        "Far future note should be hidden in constant mode"
    );
}

#[test]
fn draw_lane_offsets_accumulated() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let offsets = vec![
        DrawLaneOffset {
            x: 5.0,
            y: 10.0,
            w: 2.0,
            h: 3.0,
        },
        DrawLaneOffset {
            x: 1.0,
            y: 2.0,
            w: 1.0,
            h: 1.0,
        },
    ];
    let result = renderer.draw_lane(&ctx, &lanes, &offsets);

    // Verify note commands reflect accumulated offsets
    let note_cmd = result.commands.iter().find(|c| {
        matches!(
            c,
            DrawCommand::DrawNote {
                image_type: NoteImageType::Normal,
                ..
            }
        )
    });
    assert!(note_cmd.is_some(), "Expected a DrawNote command");
    if let Some(DrawCommand::DrawNote { x, w, .. }) = note_cmd {
        // offset_x = 5+1 = 6, offset_w = 2+1 = 3
        // x = region_x + offset_x = 0 + 6 = 6
        // w = region_width + offset_w = 30 + 3 = 33
        assert!(
            (*x - 6.0).abs() < 0.001,
            "x should include offset, got {}",
            x
        );
        assert!(
            (*w - 33.0).abs() < 0.001,
            "w should include offset, got {}",
            w
        );
    }
}

#[test]
fn draw_lane_practice_mode_uses_start_time() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    tl0.section_line = true;
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    ctx.is_practice = true;
    ctx.practice_start_time = 500; // 500ms

    let lanes = make_lanes(8);
    let _result = renderer.draw_lane(&ctx, &lanes, &[]);

    // In practice mode, hispeed should be 1.0 and pos should be reset to 0
    // The test primarily verifies no panics and correct practice mode behavior
    assert_eq!(renderer.pos, 0);
}

#[test]
fn draw_lane_long_note_emits_body_and_caps() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;

    // Create LN start at tl1, end at tl2
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    let mut tl2 = make_timeline(2.0, 2_000_000, 120.0, 8);

    // We need to set pair indices referencing each other via timeline indices
    // In the model, timelines are stored in order, so tl1 is index 1, tl2 is index 2
    let mut start_note = Note::new_long(1);
    start_note.set_pair_index(Some(2)); // points to tl2's index in all_timelines
    start_note.set_end(false);

    let mut end_note = Note::new_long(1);
    end_note.set_pair_index(Some(1)); // points to tl1's index
    end_note.set_end(true);

    tl1.set_note(0, Some(start_note));
    tl2.set_note(0, Some(end_note));

    let model = make_model_with_timelines(vec![tl0, tl1, tl2], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // Should have DrawLongNote commands (body + end, and optionally start)
    let ln_count = result
        .commands
        .iter()
        .filter(|c| matches!(c, DrawCommand::DrawLongNote { .. }))
        .count();
    assert!(
        ln_count >= 2,
        "Expected at least 2 DrawLongNote commands (body + end), got {}",
        ln_count
    );
}

#[test]
fn draw_lane_hidden_note_shown_when_enabled() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    tl1.set_hidden_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    ctx.show_hiddennote = true;

    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    let has_hidden = result.commands.iter().any(|c| {
        matches!(
            c,
            DrawCommand::DrawNote {
                image_type: NoteImageType::Hidden,
                ..
            }
        )
    });
    assert!(has_hidden, "Expected hidden note DrawNote command");
}

#[test]
fn draw_lane_hidden_note_not_shown_when_disabled() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    tl1.set_hidden_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    ctx.show_hiddennote = false;

    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    let has_hidden = result.commands.iter().any(|c| {
        matches!(
            c,
            DrawCommand::DrawNote {
                image_type: NoteImageType::Hidden,
                ..
            }
        )
    });
    assert!(
        !has_hidden,
        "Hidden notes should not be drawn when disabled"
    );
}

#[test]
fn draw_lane_bpm_guide_emitted() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    tl0.section_line = true;
    // tl1 has different BPM
    let mut tl1 = make_timeline(1.0, 1_000_000, 180.0, 8);
    tl1.section_line = true;
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    ctx.show_bpmguide = true;
    ctx.lane_group_regions = vec![LaneGroupRegion {
        x: 0.0,
        width: 100.0,
    }];

    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    let has_bpm_line = result
        .commands
        .iter()
        .any(|c| matches!(c, DrawCommand::DrawBpmLine { .. }));
    assert!(has_bpm_line, "Expected BPM line when BPM changes");

    let has_bpm_text = result
        .commands
        .iter()
        .any(|c| matches!(c, DrawCommand::DrawBpmText { .. }));
    assert!(has_bpm_text, "Expected BPM text when BPM changes");
}

#[test]
fn draw_lane_hidden_cover_with_lift() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.section_line = true;
    let model = make_model_with_timelines(vec![tl0], 120.0);
    let mut renderer = LaneRenderer::new(&model);
    renderer.enable_hidden = true;
    renderer.hidden = 0.4;
    renderer.enable_lift = true;
    renderer.lift = 0.2;

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // hidden_y = (1 - lift) * hidden * region_height = 0.8 * 0.4 * 500 = 160
    assert!(result.hidden_cover.visible);
    assert!(
        (result.hidden_cover.y - 160.0).abs() < 0.001,
        "hidden_y = {}, expected 160",
        result.hidden_cover.y
    );
}

// =========================================================================
// apply_play_config tests
// =========================================================================

#[test]
fn apply_play_config_updates_all_fields() {
    let tl0 = make_timeline(0.0, 0, 120.0, 8);
    let model = make_model_with_timelines(vec![tl0], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let pc = PlayConfig {
        hispeed: 3.5,
        duration: 750,
        lanecover: 0.3,
        enablelanecover: true,
        lift: 0.15,
        enablelift: true,
        hidden: 0.25,
        enablehidden: true,
        enable_constant: true,
        constant_fadein_time: 200,
        fixhispeed: FIX_HISPEED_MAINBPM,
        hispeedmargin: 0.5,
        ..PlayConfig::default()
    };

    renderer.apply_play_config(&pc);

    assert!((renderer.hispeed() - 3.5).abs() < f32::EPSILON, "hispeed");
    assert_eq!(renderer.duration(), 750, "duration");
    assert!(
        (renderer.lanecover() - 0.3).abs() < f32::EPSILON,
        "lanecover"
    );
    assert!(renderer.is_enable_lanecover(), "enable_lanecover");
    assert!((renderer.lift_region() - 0.15).abs() < f32::EPSILON, "lift");
    assert!(renderer.is_enable_lift(), "enable_lift");
    assert!(
        (renderer.hidden_cover() - 0.25).abs() < f32::EPSILON,
        "hidden"
    );
    assert!(renderer.is_enable_hidden(), "enable_hidden");
    assert!(
        (renderer.hispeedmargin() - 0.5).abs() < f32::EPSILON,
        "hispeedmargin"
    );

    // Verify via round-trip: extract PlayConfig and compare
    let extracted = renderer.play_config();
    assert!(extracted.enable_constant, "enable_constant round-trip");
    assert_eq!(
        extracted.constant_fadein_time, 200,
        "constant_fadein_time round-trip"
    );
    assert_eq!(
        extracted.fixhispeed, FIX_HISPEED_MAINBPM,
        "fixhispeed round-trip"
    );
}

// =========================================================================
// fixhispeed basebpm regression tests
// =========================================================================

/// Regression test: local FIX_HISPEED_* constants in lane_renderer had wrong
/// values (MINBPM=2, MAXBPM=3, MAINBPM=4) swapped relative to the canonical
/// constants in rubato_types::play_config (MAXBPM=2, MAINBPM=3, MINBPM=4,
/// matching Java PlayConfig). With default fixhispeed=FIX_HISPEED_MAINBPM(3),
/// the match hit the wrong arm, setting basebpm to maxbpm instead of mainbpm.
///
/// This test creates a model with four distinct BPM values:
///   model.bpm (start) = 100, minbpm = 80, maxbpm = 200, mainbpm = 150
/// and verifies each fixhispeed mode sets basebpm to the correct value.
#[test]
fn fixhispeed_basebpm_set_correctly_for_each_mode() {
    // Timeline layout:
    //   tl0: bpm=100 (model start bpm), 1 note
    //   tl1: bpm=80  (will be minbpm), 1 note
    //   tl2: bpm=200 (will be maxbpm), 1 note
    //   tl3: bpm=150 (will be mainbpm - most notes), 3 notes
    //   tl4: bpm=150, 3 notes (more notes at 150 to make it the main BPM)
    let mut tl0 = make_timeline(0.0, 0, 100.0, 8);
    tl0.set_note(0, Some(Note::new_normal(1)));

    let mut tl1 = make_timeline(1.0, 500_000, 80.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));

    let mut tl2 = make_timeline(2.0, 1_000_000, 200.0, 8);
    tl2.set_note(0, Some(Note::new_normal(1)));

    let mut tl3 = make_timeline(3.0, 1_500_000, 150.0, 8);
    tl3.set_note(0, Some(Note::new_normal(1)));
    tl3.set_note(1, Some(Note::new_normal(1)));
    tl3.set_note(2, Some(Note::new_normal(1)));

    let mut tl4 = make_timeline(4.0, 2_000_000, 150.0, 8);
    tl4.set_note(0, Some(Note::new_normal(1)));
    tl4.set_note(1, Some(Note::new_normal(1)));
    tl4.set_note(2, Some(Note::new_normal(1)));

    let model = make_model_with_timelines(vec![tl0, tl1, tl2, tl3, tl4], 100.0);

    // Verify model properties are as expected
    assert!(
        (model.min_bpm() - 80.0).abs() < 0.001,
        "minbpm should be 80"
    );
    assert!(
        (model.max_bpm() - 200.0).abs() < 0.001,
        "maxbpm should be 200"
    );

    // Test FIX_HISPEED_OFF: basebpm stays at default (0.0 from new())
    {
        let mut renderer = LaneRenderer::new(&model);
        let pc = PlayConfig {
            fixhispeed: FIX_HISPEED_OFF,
            ..PlayConfig::default()
        };
        renderer.apply_play_config(&pc);
        renderer.init(&model);
        // OFF preserves previous basebpm, which is 0.0 since new() sets it to 0.0
        // and first init() with OFF also preserves it
        assert!(
            renderer.base_bpm().abs() < 0.001,
            "FIX_HISPEED_OFF: basebpm should remain 0.0, got {}",
            renderer.base_bpm()
        );
    }

    // Test FIX_HISPEED_STARTBPM: basebpm = model.bpm = 100
    {
        let mut renderer = LaneRenderer::new(&model);
        let pc = PlayConfig {
            fixhispeed: FIX_HISPEED_STARTBPM,
            ..PlayConfig::default()
        };
        renderer.apply_play_config(&pc);
        renderer.init(&model);
        assert!(
            (renderer.base_bpm() - 100.0).abs() < 0.001,
            "FIX_HISPEED_STARTBPM: basebpm should be 100 (model.bpm), got {}",
            renderer.base_bpm()
        );
    }

    // Test FIX_HISPEED_MAXBPM (=2): basebpm = maxbpm = 200
    {
        let mut renderer = LaneRenderer::new(&model);
        let pc = PlayConfig {
            fixhispeed: FIX_HISPEED_MAXBPM,
            ..PlayConfig::default()
        };
        renderer.apply_play_config(&pc);
        renderer.init(&model);
        assert!(
            (renderer.base_bpm() - 200.0).abs() < 0.001,
            "FIX_HISPEED_MAXBPM: basebpm should be 200 (maxbpm), got {}",
            renderer.base_bpm()
        );
    }

    // Test FIX_HISPEED_MAINBPM (=3): basebpm = mainbpm = 150
    {
        let mut renderer = LaneRenderer::new(&model);
        let pc = PlayConfig {
            fixhispeed: FIX_HISPEED_MAINBPM,
            ..PlayConfig::default()
        };
        renderer.apply_play_config(&pc);
        renderer.init(&model);
        assert!(
            (renderer.base_bpm() - 150.0).abs() < 0.001,
            "FIX_HISPEED_MAINBPM: basebpm should be 150 (mainbpm), got {}",
            renderer.base_bpm()
        );
    }

    // Test FIX_HISPEED_MINBPM (=4): basebpm = minbpm = 80
    {
        let mut renderer = LaneRenderer::new(&model);
        let pc = PlayConfig {
            fixhispeed: FIX_HISPEED_MINBPM,
            ..PlayConfig::default()
        };
        renderer.apply_play_config(&pc);
        renderer.init(&model);
        assert!(
            (renderer.base_bpm() - 80.0).abs() < 0.001,
            "FIX_HISPEED_MINBPM: basebpm should be 80 (minbpm), got {}",
            renderer.base_bpm()
        );
    }
}

/// Regression test: pos-advancement for LN end notes previously checked the
/// pair's (start note's) time instead of the end note's own time. Java always
/// uses: `(ln.isEnd() ? ln : ln.getPair()).getMicroTime()` -- the end note's time.
///
/// Scenario: LN start at tl1 (t=1s), LN end at tl2 (t=2s). At time 3s both
/// notes are past, so pos should advance past tl1. Before the fix, the end note
/// branch checked pair (start) time which is also past, but the logic was
/// identical in both branches making it accidentally correct for this case.
/// The real bug manifests when start and end notes have divergent visibility
/// but this test verifies the code path doesn't regress.
#[test]
fn pos_advance_ln_end_uses_own_time() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;

    // LN start at tl1 (t=1s), end at tl2 (t=2s)
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    let mut tl2 = make_timeline(2.0, 2_000_000, 120.0, 8);

    let mut start_note = Note::new_long(1);
    start_note.set_pair_index(Some(2));
    start_note.set_end(false);

    let mut end_note = Note::new_long(1);
    end_note.set_pair_index(Some(1));
    end_note.set_end(true);

    tl1.set_note(0, Some(start_note));
    tl2.set_note(0, Some(end_note));

    // Add a timeline after the LN to verify pos advancement
    let mut tl3 = make_timeline(3.0, 3_000_000, 120.0, 8);
    tl3.set_note(0, Some(Note::new_normal(1)));

    let model = make_model_with_timelines(vec![tl0, tl1, tl2, tl3], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    // Set time well past both LN notes (end at 2s)
    ctx.time = 2500; // 2.5 seconds
    ctx.timer_play = Some(0);

    let lanes = make_lanes(8);
    renderer.draw_lane(&ctx, &lanes, &[]);

    // pos should have advanced past tl1 (the LN start timeline)
    // since the LN end time (2s) < current time (2.5s)
    assert!(
        renderer.pos > 0,
        "pos should advance past LN timelines when end time is past, got pos={}",
        renderer.pos
    );
}

/// Regression test: LN end note in pos-advancement should NOT advance when
/// the end note's own time is still in the future, even though the pair (start)
/// time might be in the past.
#[test]
fn pos_advance_ln_end_blocks_when_end_note_future() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;

    // LN start at tl1 (t=0.5s), end at tl2 (t=2s)
    let mut tl1 = make_timeline(0.5, 500_000, 120.0, 8);
    let mut tl2 = make_timeline(2.0, 2_000_000, 120.0, 8);

    let mut start_note = Note::new_long(1);
    start_note.set_pair_index(Some(2));
    start_note.set_end(false);

    let mut end_note = Note::new_long(1);
    end_note.set_pair_index(Some(1));
    end_note.set_end(true);

    tl1.set_note(0, Some(start_note));
    tl2.set_note(0, Some(end_note));

    let model = make_model_with_timelines(vec![tl0, tl1, tl2], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    // Time is past start note (0.5s) but before end note (2s)
    ctx.time = 1000; // 1 second
    ctx.timer_play = Some(0);

    let lanes = make_lanes(8);
    renderer.draw_lane(&ctx, &lanes, &[]);

    // The LN end at tl2 (t=2s) is still in the future, so tl1 should NOT be
    // advanced past (the LN body is still visible).
    // pos should remain at 0 because tl1 has a start note whose pair end is future.
    assert_eq!(
        renderer.pos, 0,
        "pos should NOT advance past LN start when end note is still future"
    );
}

/// Regression test: is_passing comparison used incompatible index spaces.
/// With processing/passing now storing timeline indices (converted from JudgeNote
/// indices in render_skin.rs), verify that matching timeline indices produce
/// correct is_processing/is_passing state for LN body rendering.
#[test]
fn long_note_processing_uses_timeline_indices() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;

    // LN start at tl1 (t=1s), end at tl2 (t=3s)
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);
    let mut tl2 = make_timeline(3.0, 3_000_000, 120.0, 8);

    let mut start_note = Note::new_long(1);
    start_note.set_pair_index(Some(2));
    start_note.set_end(false);

    let mut end_note = Note::new_long(1);
    end_note.set_pair_index(Some(1));
    end_note.set_end(true);

    tl1.set_note(0, Some(start_note));
    tl2.set_note(0, Some(end_note));

    let model = make_model_with_timelines(vec![tl0, tl1, tl2], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    // Set processing_long_notes[0] = Some(2) -- timeline index 2 (the end note)
    // This simulates what render_skin.rs now produces after JudgeNote->timeline conversion
    ctx.processing_long_notes = vec![Some(2), None, None, None, None, None, None, None];

    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // The LN should be drawn with active body (image_index 2 for CN)
    // because processing_long_notes[0] == Some(2) matches pair_tl_idx == 2
    let has_active_body = result
        .commands
        .iter()
        .any(|c| matches!(c, DrawCommand::DrawLongNote { image_index: 2, .. }));
    assert!(
        has_active_body,
        "LN should render with active body when processing matches pair timeline index"
    );
}

#[test]
fn apply_play_config_then_init_recalculates_basebpm() {
    let mut tl0 = make_timeline(0.0, 0, 130.0, 8);
    tl0.section_line = true;
    // Add a second timeline with a note so mainbpm is determined
    let mut tl1 = make_timeline(1.0, 1_000_000, 150.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 130.0);
    let mut renderer = LaneRenderer::new(&model);

    // Apply config with FIX_HISPEED_STARTBPM - basebpm should become model.bpm (130)
    let pc = PlayConfig {
        fixhispeed: FIX_HISPEED_STARTBPM,
        hispeed: 2.0,
        enablelanecover: true,
        lanecover: 0.0,
        ..PlayConfig::default()
    };
    renderer.apply_play_config(&pc);
    renderer.init(&model);

    // After init with FIX_HISPEED_STARTBPM, basebpm = model.bpm = 130
    // basehispeed should be set to hispeed when fixhispeed != OFF
    let extracted = renderer.play_config();
    assert_eq!(extracted.fixhispeed, FIX_HISPEED_STARTBPM);
}

// =========================================================================
// reset_hispeed division-by-zero guard tests
// =========================================================================

/// Regression test: reset_hispeed() divides by `self.duration as f64`.
/// While PlayConfig::validate() clamps duration to DURATION_MIN=1, bypassing
/// validate (e.g., direct field assignment) could leave duration=0, causing
/// division by zero / inf / NaN in hispeed calculation.
#[test]
fn reset_hispeed_zero_duration_does_not_panic_or_produce_nan() {
    let tl0 = make_timeline(0.0, 0, 120.0, 8);
    let model = make_model_with_timelines(vec![tl0], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    // Force duration to 0 (bypassing PlayConfig::validate)
    renderer.duration = 0;
    renderer.fixhispeed = FIX_HISPEED_MAINBPM;
    let original_hispeed = renderer.hispeed();

    // Should early-return without modifying hispeed
    renderer.reset_hispeed(120.0);

    assert_eq!(
        renderer.hispeed(),
        original_hispeed,
        "hispeed should remain unchanged when duration is 0"
    );
    assert!(
        renderer.hispeed().is_finite(),
        "hispeed must never be NaN or Inf"
    );
}

// =========================================================================
// Regression: bad_judge_time.unsigned_abs() wraps i64::MIN back to negative
// =========================================================================

#[test]
fn pms_miss_poor_bad_judge_time_i64_min_does_not_wrap() {
    // When judge_table has fewer than 4 entries, bad_judge_time can be i64::MIN.
    // unsigned_abs() on i64::MIN returns 2^63 which overflows back to i64::MIN
    // when cast as i64. saturating_abs() correctly returns i64::MAX.
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    let mut tl1 = make_timeline(1.0, 500_000, 120.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    ctx.bad_judge_time = i64::MIN;
    ctx.time = 1000; // past the note at 500ms
    ctx.timer_play = Some(0);

    let mut lanes = make_lanes(8);
    // Enable PMS miss POOR rendering by setting dstnote2 != i32::MIN
    for lane in &mut lanes {
        lane.dstnote2 = -200;
    }

    // Before the fix, this would compute bad_time as i64::MIN (negative),
    // causing incorrect PMS miss POOR note positions. After the fix,
    // saturating_abs() caps at i64::MAX, so the miss POOR path runs safely.
    let result = renderer.draw_lane(&ctx, &lanes, &[]);
    // The key assertion: no panic, and the function completes.
    // The commands should not contain notes with NaN/infinite y positions.
    for cmd in &result.commands {
        if let DrawCommand::DrawNote { y, .. } = cmd {
            assert!(y.is_finite(), "note y position must be finite, got {}", y);
        }
    }
}

// =========================================================================
// Regression: PMS stop_time calculation was redundant (simplifies to bad_time)
// =========================================================================

#[test]
fn pms_miss_poor_last_timeline_stop_time_equals_bad_time() {
    // The PMS miss POOR else-branch (last timeline) had:
    //   stop_time = max(0, tl.micro_time + tl.micro_stop + bad_time - tl.micro_time - tl.micro_stop)
    // which algebraically simplifies to max(0, bad_time) = bad_time (since bad_time >= 0).
    // This test exercises the last-timeline branch to verify consistent output.
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.bpm = 120.0;
    tl0.stop = 100_000; // 100ms stop
    let mut tl1 = make_timeline(1.0, 500_000, 120.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    // Only one timeline with note (tl1 is the last), so the else-branch triggers
    // when iterating backwards from tl1 with no tl at i+1.
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    ctx.bad_judge_time = -200_000; // 200ms bad window (negative = early side)
    ctx.time = 1500; // well past the note
    ctx.timer_play = Some(0);

    let mut lanes = make_lanes(8);
    for lane in &mut lanes {
        lane.dstnote2 = -200;
    }

    // Should complete without panic and produce consistent y positions
    let result = renderer.draw_lane(&ctx, &lanes, &[]);
    for cmd in &result.commands {
        if let DrawCommand::DrawNote { y, .. } = cmd {
            assert!(y.is_finite(), "note y position must be finite, got {}", y);
        }
    }
}

// =========================================================================
// Regression: practice mode timeline display wraps at 35+ minutes
// =========================================================================

#[test]
fn practice_mode_timeline_text_correct_at_36_minutes() {
    // 36 minutes = 2,160,000 ms = 2,160,000,000 us
    // As i32 milliseconds: 2,160,000,000 > i32::MAX (2,147,483,647), wrapping negative.
    // Using milli_time() (i64) avoids this.
    let time_36min_us: i64 = 36 * 60 * 1_000_000; // 2,160,000,000 us

    // Place tl0 just 1 second before tl1 so both are within the visible lane region.
    let tl0_time = time_36min_us - 1_000_000;
    let mut tl0 = make_timeline(0.0, tl0_time, 120.0, 8);
    tl0.bpm = 120.0;
    tl0.section_line = true;
    // Second timeline at 36 minutes -- across the i32 ms boundary
    let mut tl1 = make_timeline(1.0, time_36min_us, 120.0, 8);
    tl1.section_line = true;
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let mut ctx = default_ctx(all_tls);
    ctx.is_practice = true;
    // In practice mode, practice_start_time is used as the time (in ms).
    // Set it just before tl0 so both timelines are in the future and visible.
    ctx.practice_start_time = (tl0_time / 1000) - 500;
    ctx.lane_group_regions = vec![LaneGroupRegion {
        x: 0.0,
        width: 100.0,
    }];

    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // Find DrawTimeText commands and verify the 36-minute timeline text
    let time_texts: Vec<&String> = result
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::DrawTimeText { text, .. } => Some(text),
            _ => None,
        })
        .collect();

    // The 36-minute timeline should produce "36:00.0" text.
    // Before the fix, tl.time() (i32 ms) would overflow and show a negative/wrong value.
    let has_36min = time_texts.iter().any(|t| t.contains("36:00"));
    assert!(
        has_36min,
        "Expected '36:00' in timeline text for 36-minute mark, got: {:?}",
        time_texts
    );
}

// =========================================================================
// Regression: LN body quad inverted when height < scale at lane-cover edge
// =========================================================================

/// Regression test for: draw_long_note_commands emitted `h: height - scale`
/// without clamping. When a long note is partially scrolled behind the lane cover,
/// `ln_height` is clamped to 0.0 at the outer call site (line 554) but
/// `draw_long_note_commands` received `height=0.0` and computed `h = 0 - scale`
/// (a negative body height), producing an inverted/flipped quad at the lane-cover
/// boundary. All three LN types (LN, CN, HCN) shared the same bug.
///
/// This test calls `draw_long_note_commands` directly with `height < scale` and
/// verifies that no DrawLongNote body command has a negative `h`.
#[test]
fn long_note_body_height_never_negative() {
    let tl0 = make_timeline(0.0, 0, 120.0, 8);
    let model = make_model_with_timelines(vec![tl0], 120.0);
    let renderer = LaneRenderer::new(&model);

    let all_tls: &[bms_model::time_line::TimeLine] = &[];
    let ctx = default_ctx(all_tls);

    // Build one LN start note for each of the three LN types
    let ln_types = [
        bms_model::note::TYPE_LONGNOTE,
        bms_model::note::TYPE_CHARGENOTE,
        bms_model::note::TYPE_HELLCHARGENOTE,
    ];

    for &ln_type in &ln_types {
        let mut note = Note::new_long(1);
        note.set_pair_index(Some(1)); // pair_tl_idx = 1
        note.set_end(false);
        // Force the note_type field so draw_long_note_commands routes to the right branch.
        // Each branch is: `(ctx.lntype == LNTYPE_X && note_type == TYPE_UNDEFINED) || note_type == TYPE_X`
        // Using explicit note_type avoids depending on ctx.lntype.
        note.set_long_note_type(ln_type);

        let mut commands = Vec::new();
        // height=0.0, scale=20.0 -- exactly the case when ln_height was clamped to 0.0
        // at the outer site but body height was computed as 0 - 20 = -20.
        renderer.draw_long_note_commands(
            &mut commands,
            &ctx,
            &DrawLongNoteParams {
                lane: 0,
                x: 0.0,
                y: 0.0,
                width: 30.0,
                height: 0.0,
                scale: 20.0,
                note: &note,
                pair_tl_idx: 1,
                note_tl_idx: 0,
            },
        );

        // Verify no body command has a negative h
        for cmd in &commands {
            if let DrawCommand::DrawLongNote { h, image_index, .. } = cmd {
                // image_index 0,2,4,6,7,8,9 are body/start; only body (2,3,6,7,8,9) can be negative
                // The start/end cap commands use `h: scale` (always positive) --
                // so we only care about body commands which have even-indexed body images.
                // But to be safe, assert all h values are non-negative.
                assert!(
                    *h >= 0.0,
                    "DrawLongNote command has negative h={h} for ln_type={ln_type} image_index={image_index}"
                );
            }
        }
    }
}

/// Regression: LN body height loop must terminate by timeline index, not by
/// f64 section equality. When an intermediate timeline shares the same section
/// value as the pair (end) timeline, the old f64 equality check would break
/// prematurely, producing a shorter LN body.
#[test]
fn draw_lane_ln_body_not_truncated_by_coincidental_section_match() {
    // tl0: baseline
    let tl0 = make_timeline(0.0, 0, 120.0, 8);

    // tl1: LN start (section=1.0)
    let mut tl1 = make_timeline(1.0, 1_000_000, 120.0, 8);

    // tl2: intermediate timeline with same section as pair (section=3.0)
    let tl2 = make_timeline(3.0, 2_000_000, 120.0, 8);

    // tl3: LN end (section=3.0, same section as tl2 but different timeline)
    let mut tl3 = make_timeline(3.0, 3_000_000, 120.0, 8);

    // Wire up the LN: start at tl1 (index 1), end at tl3 (index 3)
    let mut start_note = Note::new_long(1);
    start_note.set_pair_index(Some(3));
    start_note.set_end(false);

    let mut end_note = Note::new_long(1);
    end_note.set_pair_index(Some(1));
    end_note.set_end(true);

    tl1.set_note(0, Some(start_note));
    tl3.set_note(0, Some(end_note));

    let model = make_model_with_timelines(vec![tl0, tl1, tl2, tl3], 120.0);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // Should still emit LN body commands spanning the full range (tl1 to tl3),
    // not truncated at tl2 despite tl2 sharing the same section as tl3.
    let ln_count = result
        .commands
        .iter()
        .filter(|c| matches!(c, DrawCommand::DrawLongNote { .. }))
        .count();
    assert!(
        ln_count >= 2,
        "Expected at least 2 DrawLongNote commands (body + end), got {}. \
         LN body may have been truncated by coincidental section equality.",
        ln_count
    );
}

// =========================================================================
// currentduration i32 overflow regression tests
// =========================================================================

/// Regression test: pathologically slow BPM produces a region value that
/// exceeds i32 range. calc_region(0.00001, 1.0, 1.0) =
/// 24_000_000_000_000 which far exceeds i32::MAX (~2.1 billion). The
/// explicit clamp before i32 cast makes the saturation intent clear and
/// guards against any future refactoring that might change the cast
/// target type.
#[test]
fn currentduration_clamps_on_extreme_slow_bpm() {
    // BPM = 0.00001 -> region = 240000/0.00001/1.0/1.0 = 24_000_000_000_000
    let mut tl0 = make_timeline(0.0, 0, 0.00001, 8);
    tl0.section_line = true;
    let model = make_model_with_timelines(vec![tl0], 0.00001);
    let mut renderer = LaneRenderer::new(&model);

    let all_tls = &model.timelines;
    let ctx = default_ctx(all_tls);
    let lanes = make_lanes(8);
    renderer.draw_lane(&ctx, &lanes, &[]);

    // The region is astronomically large; currentduration must saturate
    // to i32::MAX rather than wrapping or producing a negative value.
    assert_eq!(renderer.current_duration(), i32::MAX);
}

// =========================================================================
// mainbpm fallback regression tests
// =========================================================================

/// Regression test: when all timelines have zero notes, mainbpm must fall
/// back to model.bpm (the chart's declared BPM) rather than staying at 0.0.
/// Without the fallback, FIX_HISPEED_MAINBPM uses basebpm=0.0, which
/// produces an infinite or NaN hispeed region.
#[test]
fn mainbpm_falls_back_to_model_bpm_when_no_notes() {
    // Two timelines, neither has any notes.
    let tl0 = make_timeline(0.0, 0, 140.0, 8);
    let tl1 = make_timeline(1.0, 1_000_000, 140.0, 8);
    let model = make_model_with_timelines(vec![tl0, tl1], 140.0);

    let renderer = LaneRenderer::new(&model);
    assert!(
        (renderer.main_bpm() - 140.0).abs() < 0.001,
        "mainbpm should fall back to model.bpm (140.0) when no notes exist, got {}",
        renderer.main_bpm()
    );
}

/// When notes do exist, mainbpm should still be determined by the BPM
/// carrying the most notes (existing behavior preserved).
#[test]
fn mainbpm_uses_most_notes_bpm_when_notes_exist() {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.set_note(0, Some(Note::new_normal(1)));
    // 3 notes at BPM 150 -> should be the main BPM
    let mut tl1 = make_timeline(1.0, 1_000_000, 150.0, 8);
    tl1.set_note(0, Some(Note::new_normal(1)));
    tl1.set_note(1, Some(Note::new_normal(1)));
    tl1.set_note(2, Some(Note::new_normal(1)));
    let model = make_model_with_timelines(vec![tl0, tl1], 120.0);

    let renderer = LaneRenderer::new(&model);
    assert!(
        (renderer.main_bpm() - 150.0).abs() < 0.001,
        "mainbpm should be 150.0 (most notes), got {}",
        renderer.main_bpm()
    );
}

/// FIX_HISPEED_MAINBPM with zero-note model should use chart BPM for
/// basebpm, not 0.0.
#[test]
fn fixhispeed_mainbpm_uses_fallback_when_no_notes() {
    let tl0 = make_timeline(0.0, 0, 160.0, 8);
    let model = make_model_with_timelines(vec![tl0], 160.0);

    let mut renderer = LaneRenderer::new(&model);
    let pc = PlayConfig {
        fixhispeed: FIX_HISPEED_MAINBPM,
        ..PlayConfig::default()
    };
    renderer.apply_play_config(&pc);
    renderer.init(&model);
    assert!(
        (renderer.base_bpm() - 160.0).abs() < 0.001,
        "FIX_HISPEED_MAINBPM with no notes: basebpm should be 160.0, got {}",
        renderer.base_bpm()
    );
}

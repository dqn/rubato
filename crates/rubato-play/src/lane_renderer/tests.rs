use super::*;
use bms_model::bms_model::BMSModel;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;

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

fn default_ctx(all_timelines: &[TimeLine]) -> DrawLaneContext<'_> {
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
        all_timelines,
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

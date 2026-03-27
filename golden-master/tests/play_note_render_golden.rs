/// Golden test for play screen note rendering via LaneRenderer::draw_lane().
///
/// Verifies:
/// - Notes at different times produce DrawNote commands with correct Y positions
/// - Y-up coordinate system: future notes have larger y (further from judge line),
///   notes closer to the judge line have smaller y (closer to region_y)
/// - Long notes produce DrawLongNote commands with correct body/cap positions
/// - Section lines are emitted at correct y offsets
/// - DrawCommand output is stable (golden fixture regression)
use bms::model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use bms::model::note::Note;
use bms::model::time_line::TimeLine;
use rubato_game::play::lane_renderer::{
    DrawCommand, DrawLaneContext, LaneRenderer, NoteImageType, TimelinesRef,
};
use rubato_game::play::skin::note::SkinLane;
use serde::{Deserialize, Serialize};

const FIXTURE_PATH: &str = "golden-master/fixtures/play_note_draw_commands.json";

// ---------------------------------------------------------------------------
// Serializable draw command snapshot for golden comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct NoteDrawSnapshot {
    commands: Vec<SnapshotCommand>,
    lift_offset_y: f32,
    lanecover_offset_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
enum SnapshotCommand {
    SetColor {
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    },
    SetBlend {
        blend: i32,
    },
    SetType {
        typ: i32,
    },
    DrawNote {
        lane: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        image_type: String,
    },
    DrawLongNote {
        lane: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        image_index: usize,
    },
    DrawSectionLine {
        y_offset: i32,
    },
    DrawBpmLine {
        y_offset: i32,
        bpm: f64,
    },
    Other {
        description: String,
    },
}

fn convert_command(cmd: &DrawCommand) -> SnapshotCommand {
    match cmd {
        DrawCommand::SetColor { r, g, b, a } => SnapshotCommand::SetColor {
            r: *r,
            g: *g,
            b: *b,
            a: *a,
        },
        DrawCommand::SetBlend(blend) => SnapshotCommand::SetBlend { blend: *blend },
        DrawCommand::SetType(typ) => SnapshotCommand::SetType { typ: *typ },
        DrawCommand::DrawNote {
            lane,
            x,
            y,
            w,
            h,
            image_type,
        } => SnapshotCommand::DrawNote {
            lane: *lane,
            x: *x,
            y: *y,
            w: *w,
            h: *h,
            image_type: format!("{:?}", image_type),
        },
        DrawCommand::DrawLongNote {
            lane,
            x,
            y,
            w,
            h,
            image_index,
        } => SnapshotCommand::DrawLongNote {
            lane: *lane,
            x: *x,
            y: *y,
            w: *w,
            h: *h,
            image_index: *image_index,
        },
        DrawCommand::DrawSectionLine { y_offset } => SnapshotCommand::DrawSectionLine {
            y_offset: *y_offset,
        },
        DrawCommand::DrawBpmLine { y_offset, bpm } => SnapshotCommand::DrawBpmLine {
            y_offset: *y_offset,
            bpm: *bpm,
        },
        other => SnapshotCommand::Other {
            description: format!("{:?}", other),
        },
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_model(timelines: Vec<TimeLine>, bpm: f64) -> BMSModel {
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

/// Make 8-lane skin lanes with tall region to fit all notes in the visible area.
/// BPM 120, hispeed 1.0 => region = 240000/120 = 2000ms visible window.
/// Lane height 1000px => 0.5px per ms.
fn make_lanes_7k() -> Vec<SkinLane> {
    (0..8)
        .map(|i| {
            let mut lane = SkinLane::new();
            lane.region_x = 100.0 + i as f32 * 40.0;
            lane.region_y = 0.0;
            lane.region_width = 38.0;
            lane.region_height = 1000.0; // judge line at y=1000
            lane.scale = 8.0;
            lane
        })
        .collect()
}

fn make_ctx(all_timelines: &[TimeLine]) -> DrawLaneContext {
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
        judge_time_regions: vec![vec![[0i64; 2]; 5]; 8],
        processing_long_notes: vec![None; 8],
        passing_long_notes: vec![None; 8],
        hell_charge_judges: vec![false; 8],
        bad_judge_time: 0,
        model_bpm: 120.0,
        // Safety: all_timelines outlives the DrawLaneContext in all test uses.
        all_timelines: unsafe { TimelinesRef::from_slice(all_timelines) },
        forced_cn_endings: false,
    }
}

fn capture_snapshot(
    model: &BMSModel,
    lanes: &[SkinLane],
    ctx: &DrawLaneContext,
) -> NoteDrawSnapshot {
    let mut renderer = LaneRenderer::new(model);
    let result = renderer.draw_lane(ctx, lanes, &[]);
    NoteDrawSnapshot {
        commands: result.commands.iter().map(convert_command).collect(),
        lift_offset_y: result.lift_offset_y,
        lanecover_offset_y: (result.lanecover_offset_y * 100.0).round() / 100.0,
    }
}

// ---------------------------------------------------------------------------
// Test model
// ---------------------------------------------------------------------------

/// Build a 7K BMS model with notes close together so they all fit in the visible area.
/// BPM 120, region = 2000ms. All notes within 0-1600ms.
///
/// - tl0: section 0, time 0us, BPM 120, section line (base)
/// - tl1: section 0.25, time 500000us (500ms), section line
/// - tl2: section 0.50, time 1000000us (1000ms), normal note on lane 0 + section line
/// - tl3: section 0.75, time 1500000us (1500ms), normal note on lane 3
/// - tl4: section 0.875, time 1750000us (1750ms), mine note on lane 5
/// - tl5: section 0.50, time 1000000us -- reused as LN pair start (separate model below)
fn build_test_model() -> BMSModel {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.section_line = true;

    let mut tl1 = make_timeline(0.25, 500_000, 120.0, 8);
    tl1.section_line = true;

    let mut tl2 = make_timeline(0.50, 1_000_000, 120.0, 8);
    tl2.set_note(0, Some(Note::new_normal(1)));
    tl2.section_line = true;

    let mut tl3 = make_timeline(0.75, 1_500_000, 120.0, 8);
    tl3.set_note(3, Some(Note::new_normal(2)));

    let mut tl4 = make_timeline(0.875, 1_750_000, 120.0, 8);
    tl4.set_note(5, Some(Note::new_mine(1, 0.5)));

    make_model(vec![tl0, tl1, tl2, tl3, tl4], 120.0)
}

/// Build a model specifically for LN testing.
fn build_ln_model() -> BMSModel {
    let mut tl0 = make_timeline(0.0, 0, 120.0, 8);
    tl0.section_line = true;

    let mut tl1 = make_timeline(0.25, 500_000, 120.0, 8);
    let mut ln_start = Note::new_long(1);
    ln_start.set_pair_index(Some(2)); // points to tl2
    ln_start.set_end(false);
    tl1.set_note(1, Some(ln_start));

    let mut tl2 = make_timeline(0.50, 1_000_000, 120.0, 8);
    let mut ln_end = Note::new_long(1);
    ln_end.set_pair_index(Some(1)); // points to tl1
    ln_end.set_end(true);
    tl2.set_note(1, Some(ln_end));

    make_model(vec![tl0, tl1, tl2], 120.0)
}

// ---------------------------------------------------------------------------
// Golden fixture test
// ---------------------------------------------------------------------------

#[test]
fn play_note_draw_commands_golden() {
    let model = build_test_model();
    let lanes = make_lanes_7k();
    let ctx = make_ctx(&model.timelines);
    let snapshot = capture_snapshot(&model, &lanes, &ctx);

    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(FIXTURE_PATH);

    if fixture_path.exists() {
        // Compare against golden fixture
        let fixture_json = std::fs::read_to_string(&fixture_path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture_path.display(), e));
        let expected: NoteDrawSnapshot = serde_json::from_str(&fixture_json)
            .unwrap_or_else(|e| panic!("Failed to parse fixture: {}", e));

        assert_eq!(
            snapshot.commands.len(),
            expected.commands.len(),
            "Command count mismatch: got {}, expected {}.\nGot: {}\nExpected: {}",
            snapshot.commands.len(),
            expected.commands.len(),
            serde_json::to_string_pretty(&snapshot.commands).unwrap(),
            serde_json::to_string_pretty(&expected.commands).unwrap(),
        );

        for (i, (got, exp)) in snapshot
            .commands
            .iter()
            .zip(expected.commands.iter())
            .enumerate()
        {
            assert_eq!(
                got, exp,
                "Command [{}] mismatch.\nGot:      {:?}\nExpected: {:?}",
                i, got, exp
            );
        }

        assert!(
            (snapshot.lift_offset_y - expected.lift_offset_y).abs() < 0.01,
            "lift_offset_y mismatch: got {}, expected {}",
            snapshot.lift_offset_y,
            expected.lift_offset_y
        );
    } else {
        // Generate golden fixture
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        std::fs::write(&fixture_path, &json).unwrap_or_else(|e| {
            panic!("Failed to write fixture {}: {}", fixture_path.display(), e)
        });
        eprintln!(
            "Generated golden fixture at {} ({} commands)",
            fixture_path.display(),
            snapshot.commands.len()
        );
    }
}

// ---------------------------------------------------------------------------
// Y-coordinate direction verification
// ---------------------------------------------------------------------------

#[test]
fn note_y_increases_toward_judge_line() {
    // Y-up: notes further in time have larger y (further from judge line at region_y).
    // The judge line sits at region_y (bottom of the lane region in screen space),
    // and notes scroll upward (increasing y) as they approach from the future.
    let model = build_test_model();
    let lanes = make_lanes_7k();
    let ctx = make_ctx(&model.timelines);

    let mut renderer = LaneRenderer::new(&model);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    // Collect DrawNote y-positions
    let note_ys: Vec<(usize, f32)> = result
        .commands
        .iter()
        .filter_map(|cmd| {
            if let DrawCommand::DrawNote {
                lane,
                y,
                image_type: NoteImageType::Normal,
                ..
            } = cmd
            {
                Some((*lane, *y))
            } else {
                None
            }
        })
        .collect();

    assert!(
        note_ys.len() >= 2,
        "Expected at least 2 normal notes, got {}",
        note_ys.len()
    );

    // Lane 0 note at 1000ms, lane 3 note at 1500ms.
    // At time=0, lane 0 is closer to the judge line. In Y-up: closer note has SMALLER y.
    let lane0_note = note_ys.iter().find(|(lane, _)| *lane == 0);
    let lane3_note = note_ys.iter().find(|(lane, _)| *lane == 3);

    if let (Some((_, y_close)), Some((_, y_far))) = (lane0_note, lane3_note) {
        assert!(
            y_close < y_far,
            "Y-up violation: closer note (lane 0, 1000ms) y={} should be < further note (lane 3, 1500ms) y={}",
            y_close,
            y_far
        );

        // Both should be within the lane area [0, 1000]
        assert!(
            *y_close >= 0.0 && *y_close <= 1000.0,
            "Lane 0 note y={} out of lane bounds [0, 1000]",
            y_close
        );
        assert!(
            *y_far >= 0.0 && *y_far <= 1000.0,
            "Lane 3 note y={} out of lane bounds [0, 1000]",
            y_far
        );
    } else {
        panic!("Missing expected notes. Found: {:?}", note_ys);
    }
}

#[test]
fn mine_note_rendered_with_correct_type() {
    let model = build_test_model();
    let lanes = make_lanes_7k();
    let ctx = make_ctx(&model.timelines);

    let mut renderer = LaneRenderer::new(&model);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    let has_mine = result.commands.iter().any(|cmd| {
        matches!(
            cmd,
            DrawCommand::DrawNote {
                lane: 5,
                image_type: NoteImageType::Mine,
                ..
            }
        )
    });
    assert!(has_mine, "Expected mine note on lane 5");
}

#[test]
fn long_note_produces_body_and_caps() {
    let model = build_ln_model();
    let lanes = make_lanes_7k();
    let ctx = make_ctx(&model.timelines);

    let mut renderer = LaneRenderer::new(&model);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    let ln_commands: Vec<_> = result
        .commands
        .iter()
        .filter(|cmd| matches!(cmd, DrawCommand::DrawLongNote { lane: 1, .. }))
        .collect();

    assert!(
        ln_commands.len() >= 2,
        "Expected at least 2 DrawLongNote commands for lane 1 LN, got {}.\nAll commands: {:?}",
        ln_commands.len(),
        result.commands
    );

    // Verify LN body spans a vertical range
    let ln_ys: Vec<f32> = ln_commands
        .iter()
        .filter_map(|cmd| {
            if let DrawCommand::DrawLongNote { y, .. } = cmd {
                Some(*y)
            } else {
                None
            }
        })
        .collect();

    let min_y = ln_ys.iter().cloned().reduce(f32::min).unwrap();
    let max_y = ln_ys.iter().cloned().reduce(f32::max).unwrap();
    assert!(
        max_y > min_y,
        "Long note should span a vertical range, but min_y={} max_y={}",
        min_y,
        max_y
    );
}

#[test]
fn section_lines_emitted_for_visible_sections() {
    let model = build_test_model();
    let lanes = make_lanes_7k();
    let ctx = make_ctx(&model.timelines);

    let mut renderer = LaneRenderer::new(&model);
    let result = renderer.draw_lane(&ctx, &lanes, &[]);

    let section_lines: Vec<i32> = result
        .commands
        .iter()
        .filter_map(|cmd| {
            if let DrawCommand::DrawSectionLine { y_offset } = cmd {
                Some(*y_offset)
            } else {
                None
            }
        })
        .collect();

    // tl0 (section 0) at time 0 is at the judge line (y_offset near 0 or at judge line).
    // tl1 (section 0.25) at 500ms is above (smaller y_offset).
    // tl2 (section 0.50) at 1000ms is further above.
    assert!(
        !section_lines.is_empty(),
        "Expected section lines. All commands: {:?}",
        result.commands
    );
}

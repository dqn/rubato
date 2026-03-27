use super::constants::{DPRANDOM, GAUGE, GRAPHTYPESTR, RANDOM};
use super::*;
use bms::model::time_line::TimeLine;

fn make_test_model(mode: &Mode, times: &[i32]) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(*mode);
    let mut timelines = Vec::new();
    for &t in times {
        let mut tl = TimeLine::new(t.into(), 0, mode.key());
        tl.bpm = 120.0;
        timelines.push(tl);
    }
    model.timelines = timelines;
    model.total = 300.0;
    model.judgerank = 100;
    model
}

#[test]
fn test_gauge_creates_with_startgauge() {
    let mut practice = PracticeConfiguration::new();
    practice.property.gaugecategory = Some(GaugeProperty::SevenKeys);
    practice.property.gaugetype = 2; // NORMAL
    practice.property.startgauge = 50;

    let model = make_test_model(&Mode::BEAT_7K, &[0, 5000]);

    let gauge = practice.gauge(&model);

    assert!(gauge.is_some());
    let gauge = gauge.unwrap();
    assert!((gauge.value() - 50.0).abs() < f64::EPSILON as f32);
}

// --- process_input tests ---

/// Helper to make a model with real micro-times for practice tests.
fn make_timed_model(mode: &Mode, time_millis: &[i32]) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(*mode);
    let mut timelines = Vec::new();
    for &t_ms in time_millis {
        let micro = t_ms as i64 * 1000;
        let tl = TimeLine::new(120.0, micro, mode.key());
        timelines.push(tl);
    }
    model.timelines = timelines;
    model.total = 300.0;
    model.judgerank = 100;
    model
}

#[test]
fn process_input_down_advances_cursor() {
    let mut practice = PracticeConfiguration::new();
    let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
    practice.create(&model);
    assert_eq!(practice.cursor_pos(), 0);

    practice.process_input(false, true, false, false, 1000);
    assert_eq!(practice.cursor_pos(), 1);
}

#[test]
fn process_input_up_wraps_cursor() {
    let mut practice = PracticeConfiguration::new();
    let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
    practice.create(&model);
    assert_eq!(practice.cursor_pos(), 0);

    // UP from 0 should go to element 9 (skipping invisible 10, 11 in SP)
    practice.process_input(true, false, false, false, 1000);
    assert_eq!(practice.cursor_pos(), 9);
}

#[test]
fn process_input_right_increments_value() {
    let mut practice = PracticeConfiguration::new();
    // Need timeline times large enough so starttime + 2000 <= last_time
    let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
    practice.create(&model);

    let start_before = practice.practice_property().starttime;
    // Right held = increment. presscount starts at 0, so first press triggers immediately.
    practice.process_input(false, false, false, true, 1000);
    let start_after = practice.practice_property().starttime;

    // cursor at 0 = STARTTIME, right should increment by 100
    assert_eq!(start_after, start_before + 100);
}

#[test]
fn process_input_left_decrements_value() {
    let mut practice = PracticeConfiguration::new();
    let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
    practice.create(&model);

    // First set starttime to something > 0 so we can decrement
    practice.practice_property_mut().starttime = 500;

    practice.process_input(false, false, true, false, 1000);
    assert_eq!(practice.practice_property().starttime, 400);
}

#[test]
fn process_input_resets_presscount_when_no_lr() {
    let mut practice = PracticeConfiguration::new();
    let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
    practice.create(&model);

    // Trigger a press to set presscount
    practice.process_input(false, false, false, true, 1000);
    assert_ne!(practice.presscount, 0);

    // Release both → presscount resets
    practice.process_input(false, false, false, false, 1500);
    assert_eq!(practice.presscount, 0);
}

// --- draw() tests ---

#[test]
fn draw_emits_element_text_commands() {
    let practice = PracticeConfiguration::new();
    let region = (0.0, 0.0, 800.0, 600.0);
    let judge_counts = [(0, 0, 0); 6];

    let commands = practice.draw(region, &judge_counts, false);

    // Should have element text commands for visible elements (indices 0..9 in SP mode)
    let text_cmds: Vec<_> = commands
        .iter()
        .filter(|c| matches!(c, PracticeDrawCommand::DrawText { .. }))
        .collect();
    // 10 elements visible in SP (0..9) + 6 judge count lines = 16 text commands
    // (no "PRESS 1KEY" because media_loaded is false)
    assert_eq!(text_cmds.len(), 16);
}

#[test]
fn draw_emits_press_1key_when_media_loaded() {
    let practice = PracticeConfiguration::new();
    let region = (0.0, 0.0, 800.0, 600.0);
    let judge_counts = [(0, 0, 0); 6];

    let commands = practice.draw(region, &judge_counts, true);

    let press_cmd = commands.iter().find(|c| match c {
        PracticeDrawCommand::DrawText { text, .. } => text.contains("PRESS 1KEY"),
        _ => false,
    });
    assert!(press_cmd.is_some());
}

#[test]
fn draw_does_not_emit_press_1key_when_not_loaded() {
    let practice = PracticeConfiguration::new();
    let region = (0.0, 0.0, 800.0, 600.0);
    let judge_counts = [(0, 0, 0); 6];

    let commands = practice.draw(region, &judge_counts, false);

    let press_cmd = commands.iter().find(|c| match c {
        PracticeDrawCommand::DrawText { text, .. } => text.contains("PRESS 1KEY"),
        _ => false,
    });
    assert!(press_cmd.is_none());
}

#[test]
fn draw_emits_graph_command() {
    let mut practice = PracticeConfiguration::new();
    practice.property.graphtype = 1;
    practice.property.starttime = 1000;
    practice.property.endtime = 5000;
    practice.property.freq = 100;
    let region = (10.0, 20.0, 400.0, 300.0);
    let judge_counts = [(0, 0, 0); 6];

    let commands = practice.draw(region, &judge_counts, false);

    let graph_cmd = commands
        .iter()
        .find(|c| matches!(c, PracticeDrawCommand::DrawGraph { .. }));
    assert!(graph_cmd.is_some());

    if let Some(PracticeDrawCommand::DrawGraph {
        graph_type,
        region: gr,
        start_time,
        end_time,
        freq,
    }) = graph_cmd
    {
        assert_eq!(*graph_type, 1);
        assert_eq!(*start_time, 1000);
        assert_eq!(*end_time, 5000);
        assert!((freq - 1.0).abs() < f32::EPSILON);
        // Region height should be rh / 4
        assert!((gr.3 - 75.0).abs() < f32::EPSILON);
    }
}

#[test]
fn draw_cursor_position_colors_element_yellow() {
    let mut practice = PracticeConfiguration::new();
    // Move cursor to element 2
    practice.cursorpos = 2;
    let region = (0.0, 0.0, 800.0, 600.0);
    let judge_counts = [(0, 0, 0); 6];

    let commands = practice.draw(region, &judge_counts, false);

    // Element text commands: elements 0..9 visible
    // Element at index 2 (cursorpos) should be Yellow, others Cyan
    let element_texts: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            PracticeDrawCommand::DrawText { color, text, .. }
                if text.starts_with("START")
                    || text.starts_with("END")
                    || text.starts_with("GAUGE")
                    || text.starts_with("JUDGE")
                    || text.starts_with("TOTAL")
                    || text.starts_with("FREQ")
                    || text.starts_with("GRAPH")
                    || text.starts_with("OPTION") =>
            {
                Some(*color)
            }
            _ => None,
        })
        .collect();

    // Element 2 should be Yellow (cursor position)
    assert_eq!(element_texts[2], PracticeColor::Yellow);
    // Element 0 should be Cyan (not cursor)
    assert_eq!(element_texts[0], PracticeColor::Cyan);
}

#[test]
fn draw_judge_counts_are_white() {
    let practice = PracticeConfiguration::new();
    let region = (0.0, 0.0, 800.0, 600.0);
    let judge_counts = [
        (10, 3, 7),
        (5, 2, 3),
        (1, 0, 1),
        (0, 0, 0),
        (0, 0, 0),
        (0, 0, 0),
    ];

    let commands = practice.draw(region, &judge_counts, false);

    let white_texts: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            PracticeDrawCommand::DrawText { text, color, .. } if *color == PracticeColor::White => {
                Some(text.clone())
            }
            _ => None,
        })
        .collect();

    // Should have 6 judge count lines
    assert_eq!(white_texts.len(), 6);
    // First line should contain PGREAT and the counts
    assert!(white_texts[0].contains("PGREAT"));
    assert!(white_texts[0].contains("10"));
    assert!(white_texts[0].contains("3"));
    assert!(white_texts[0].contains("7"));
}

#[test]
fn test_sanitize_clamps_out_of_bounds() {
    let mut prop = PracticeProperty::new();
    prop.gaugetype = 100;
    prop.random = -3;
    prop.random2 = 15;
    prop.doubleop = 10;
    prop.graphtype = -1;
    prop.freq = 0;
    prop.sanitize();
    assert!((prop.gaugetype as usize) < GAUGE.len());
    assert!((prop.random as usize) < RANDOM.len());
    assert!((prop.random2 as usize) < RANDOM.len());
    assert!((prop.doubleop as usize) < DPRANDOM.len());
    assert!((prop.graphtype as usize) < GRAPHTYPESTR.len());
    assert_eq!(prop.freq, 50);
}

#[test]
fn test_sanitize_clamps_freq_negative() {
    let mut prop = PracticeProperty::new();
    prop.freq = -10;
    prop.sanitize();
    assert_eq!(prop.freq, 50);
}

#[test]
fn test_sanitize_clamps_freq_above_max() {
    let mut prop = PracticeProperty::new();
    prop.freq = 500;
    prop.sanitize();
    assert_eq!(prop.freq, 200);
}

#[test]
fn test_sanitize_preserves_valid_freq() {
    let mut prop = PracticeProperty::new();
    prop.freq = 75;
    prop.sanitize();
    assert_eq!(prop.freq, 75);
}

#[test]
fn test_sanitize_ensures_endtime_after_starttime_when_equal() {
    let mut prop = PracticeProperty::new();
    prop.starttime = 5000;
    prop.endtime = 5000;
    prop.sanitize();
    assert_eq!(
        prop.endtime, 6000,
        "endtime should be starttime + 1000 when equal"
    );
}

#[test]
fn test_sanitize_ensures_endtime_after_starttime_when_inverted() {
    let mut prop = PracticeProperty::new();
    prop.starttime = 8000;
    prop.endtime = 3000;
    prop.sanitize();
    assert_eq!(
        prop.endtime, 9000,
        "endtime should be starttime + 1000 when starttime > endtime"
    );
}

#[test]
fn test_sanitize_preserves_valid_endtime() {
    let mut prop = PracticeProperty::new();
    prop.starttime = 2000;
    prop.endtime = 10000;
    prop.sanitize();
    assert_eq!(
        prop.endtime, 10000,
        "endtime should be unchanged when already > starttime + 1000"
    );
}

#[test]
fn test_get_element_text_out_of_bounds_no_panic() {
    let mut pc = PracticeConfiguration::default();
    pc.property.gaugetype = 99;
    pc.property.random = -5;
    pc.property.random2 = 100;
    pc.property.doubleop = 50;
    pc.property.graphtype = -1;
    // Should not panic, should produce "?" for out-of-bounds
    let text = pc.element_text(2);
    assert!(text.contains("?"));
    let text = pc.element_text(9);
    assert!(text.contains("?"));
}

#[test]
fn test_create_restores_total_from_saved_config_with_zero_total() {
    // Simulate saved practice/<sha>.json with total=0.0 (missing field from older version)
    let sha = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";
    let dir = std::path::Path::new("practice");
    std::fs::create_dir_all(dir).ok();
    let path = dir.join(format!("{}.json", sha));
    let saved = serde_json::json!({
        "starttime": 1000,
        "endtime": 5000,
        "gaugetype": 2,
        "startgauge": 20,
        "random": 0,
        "random2": 0,
        "doubleop": 0,
        "judgerank": 100,
        "freq": 100,
        "graphtype": 0
        // total is absent -> serde default 0.0
    });
    std::fs::write(&path, serde_json::to_string(&saved).unwrap()).unwrap();

    let mut model = make_test_model(&Mode::BEAT_7K, &[0, 5000]);
    model.sha256 = sha.to_string();
    model.total = 300.0;

    let mut practice = PracticeConfiguration::new();
    practice.create(&model);

    // total should be restored from model, not left as 0.0
    assert!((practice.practice_property().total - 300.0).abs() < f64::EPSILON);

    // Cleanup
    std::fs::remove_file(&path).ok();
}

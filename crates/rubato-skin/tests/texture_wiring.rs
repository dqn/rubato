// Texture wiring invariant tests: verify that skin loader properly wires
// textures from SkinSourceData through to SkinNoteObject fields.
//
// These tests would have caught bug #2: LR2 loader loaded textures from
// SRC_NOTE but the SkinNoteObject had empty TextureRegion::new() because
// assemble_objects() did not copy the loaded textures.

use rubato_skin::lr2::lr2_play_skin_loader::{LR2PlaySkinLoaderState, SkinSourceData};
use rubato_skin::lr2::lr2_skin_csv_loader::LR2SkinLoaderAccess;
use rubato_skin::reexports::{Rectangle, Resolution, TextureRegion};
use rubato_skin::skin::Skin;
use rubato_skin::skin_note_object::SkinNoteObject;
use rubato_skin::types::skin::SkinObject;
use rubato_skin::types::skin_header::SkinHeader;

fn make_test_texture() -> TextureRegion {
    TextureRegion {
        u: 0.0,
        v: 0.0,
        u2: 1.0,
        v2: 1.0,
        region_x: 0,
        region_y: 0,
        region_width: 64,
        region_height: 16,
        texture: None, // Texture handle not needed for wiring check
    }
}

fn make_loader_with_notes() -> LR2PlaySkinLoaderState {
    let src = Resolution {
        width: 1280.0,
        height: 720.0,
    };
    let dst = Resolution {
        width: 1920.0,
        height: 1080.0,
    };
    let mut loader = LR2PlaySkinLoaderState::new(
        rubato_skin::skin_type::SkinType::Play7Keys,
        src,
        dst,
        false,
        String::new(),
    );
    loader.lanerender = true;

    // Set up lane regions (required for assemble_objects to create SkinNoteObject)
    for i in 0..8 {
        loader.laner[i] = Some(Rectangle {
            x: (i as f32) * 40.0,
            y: 0.0,
            width: 40.0,
            height: 500.0,
        });
        loader.scale[i] = 1.0;
    }

    // Wire note textures for lanes 0 and 1
    let tex = make_test_texture();
    loader.note[0] = Some(SkinSourceData {
        images: Some(vec![tex.clone()]),
        timer: 0,
        cycle: 0,
    });
    loader.note[1] = Some(SkinSourceData {
        images: Some(vec![tex.clone()]),
        timer: 0,
        cycle: 0,
    });

    // Wire mine texture for lane 0
    loader.mine[0] = Some(SkinSourceData {
        images: Some(vec![tex.clone()]),
        timer: 0,
        cycle: 0,
    });

    // Wire LN body texture for lane 0
    loader.lnbody[0] = Some(SkinSourceData {
        images: Some(vec![tex]),
        timer: 0,
        cycle: 0,
    });

    loader
}

fn make_skin() -> Skin {
    let header = SkinHeader::default();
    Skin::new(header)
}

fn find_note_object(skin: &Skin) -> Option<&SkinNoteObject> {
    skin.objects().iter().find_map(|obj| {
        if let SkinObject::Note(note) = obj {
            Some(note)
        } else {
            None
        }
    })
}

// ===========================================================================
// Test 1: assemble_objects wires note textures
// ===========================================================================

#[test]
fn lr2_loader_wires_note_textures() {
    let mut loader = make_loader_with_notes();
    let mut skin = make_skin();

    loader.assemble_objects(&mut skin);

    let note = find_note_object(&skin).expect("Skin should contain a SkinNoteObject");
    assert!(
        note.note_images[0].is_some(),
        "note texture for lane 0 should be wired after assemble_objects"
    );
    assert!(
        note.note_images[1].is_some(),
        "note texture for lane 1 should be wired after assemble_objects"
    );
    // Lane 2 was not set
    assert!(
        note.note_images[2].is_none(),
        "note texture for lane 2 should be None (not set)"
    );
}

// ===========================================================================
// Test 2: assemble_objects wires mine textures
// ===========================================================================

#[test]
fn lr2_loader_wires_mine_textures() {
    let mut loader = make_loader_with_notes();
    let mut skin = make_skin();

    loader.assemble_objects(&mut skin);

    let note = find_note_object(&skin).expect("Skin should contain a SkinNoteObject");
    assert!(
        note.mine_images[0].is_some(),
        "mine texture for lane 0 should be wired after assemble_objects"
    );
    assert!(
        note.mine_images[1].is_none(),
        "mine texture for lane 1 should be None (not set)"
    );
}

// ===========================================================================
// Test 3: assemble_objects wires LN body textures
// ===========================================================================

#[test]
fn lr2_loader_wires_ln_textures() {
    let mut loader = make_loader_with_notes();
    let mut skin = make_skin();

    loader.assemble_objects(&mut skin);

    let note = find_note_object(&skin).expect("Skin should contain a SkinNoteObject");
    // LN body is at index 2 in the ln_sources array (lnbody)
    assert!(
        note.ln_body_images[0][2].is_some(),
        "LN body texture for lane 0 should be wired at index 2"
    );
    // Other LN types for lane 0 should be None
    assert!(
        note.ln_body_images[0][0].is_none(),
        "LN start for lane 0 should be None (lnstart not set)"
    );
}

// ===========================================================================
// Test 4: SkinNoteObject draw with texture produces sprite draw
// ===========================================================================

#[test]
fn draw_note_with_texture_produces_draw_call() {
    use rubato_skin::skin_object::SkinObjectRenderer;

    let mut note = SkinNoteObject::new(8);
    note.note_images[0] = Some(make_test_texture());

    note.draw_commands = vec![rubato_types::draw_command::DrawCommand::DrawNote {
        lane: 0,
        x: 10.0,
        y: 20.0,
        w: 40.0,
        h: 5.0,
        image_type: rubato_types::draw_command::NoteImageType::Normal,
    }];

    let mut sprite = SkinObjectRenderer::new();
    note.draw(&mut sprite);

    // The sprite batch should have received a draw call
    assert!(
        !sprite.sprite.vertices().is_empty(),
        "draw() with a wired texture should produce sprite vertices"
    );
}

// ===========================================================================
// Test 5: SkinNoteObject draw without texture produces no vertices
// ===========================================================================

#[test]
fn draw_note_without_texture_produces_no_vertices() {
    use rubato_skin::skin_object::SkinObjectRenderer;

    let mut note = SkinNoteObject::new(8);
    // note_images[0] is None (no texture wired)

    note.draw_commands = vec![rubato_types::draw_command::DrawCommand::DrawNote {
        lane: 0,
        x: 10.0,
        y: 20.0,
        w: 40.0,
        h: 5.0,
        image_type: rubato_types::draw_command::NoteImageType::Normal,
    }];

    let mut sprite = SkinObjectRenderer::new();
    note.draw(&mut sprite);

    // Without a texture, no sprite should be submitted
    assert!(
        sprite.sprite.vertices().is_empty(),
        "draw() without a wired texture should produce no sprite vertices"
    );
}

// ===========================================================================
// Test 6: compute_note_draw_commands produces non-empty commands
// ===========================================================================

/// Cross-boundary integration test: exercises the full
/// compute_note_draw_commands() → draw_lane() pipeline through the
/// SkinDrawable trait. Verifies that draw_commands is non-empty after
/// the call, which is the precondition for SkinNoteObject::draw() to
/// actually render anything.
#[test]
fn compute_note_draw_commands_produces_commands() {
    use bms::model::bms_model::BMSModel;
    use bms::model::note::Note;
    use bms::model::time_line::TimeLine;
    use rubato_game::core::main_state::SkinDrawable;
    use rubato_game::play::lane_renderer::{DrawLaneContext, LaneRenderer};

    // 1. Create a model with one note
    let mut model = BMSModel::new();
    model.bpm = 120.0;
    let mode = bms::model::mode::Mode::BEAT_7K;
    model.set_mode(mode);
    let mut tl = TimeLine::new(0.0, 1_000_000, mode.key() as i32);
    tl.bpm = 120.0;
    tl.set_note(0, Some(Note::new_normal(1)));
    model.timelines.push(tl);

    // 2. Create LaneRenderer
    let mut lr = LaneRenderer::new(&model);
    lr.init(&model);

    // 3. Create Skin with a SkinNoteObject (via LR2 loader helper)
    let mut loader = make_loader_with_notes();
    let mut skin = make_skin();
    loader.assemble_objects(&mut skin);

    // Verify Note exists before prepare
    assert!(
        find_note_object(&skin).is_some(),
        "precondition: SkinNoteObject must be in skin before prepare"
    );

    // Run Skin::prepare() (as prepare_skin does with NullTimer)
    // This is the step that builds objectarray_indices and may remove objects
    skin.prepare_skin(None);

    // Verify Note survives prepare
    assert!(
        find_note_object(&skin).is_some(),
        "SkinNoteObject must survive Skin::prepare() - if this fails, \
         prepare() is removing the Note due to validate/option/draw-condition failure"
    );

    // 4. Build a DrawLaneContext with TIMER_PLAY active.
    // Safety: model.timelines outlives the DrawLaneContext (consumed synchronously below).
    let all_timelines =
        unsafe { rubato_game::play::lane_renderer::TimelinesRef::from_slice(&model.timelines) };
    let draw_ctx = DrawLaneContext {
        time: 1000,
        timer_play: Some(0), // TIMER_PLAY started at time 0
        timer_141: None,
        judge_timing: 0,
        is_practice: false,
        practice_start_time: 0,
        now_time: 1000,
        now_quarter_note_time: 0,
        note_expansion_rate: [100, 100],
        lane_group_regions: Vec::new(),
        show_bpmguide: false,
        show_pastnote: false,
        mark_processednote: false,
        show_hiddennote: false,
        show_judgearea: false,
        lntype: bms::model::bms_model::LnType::ChargeNote,
        judge_time_regions: vec![vec![[0, 0]; 5]; 8],
        processing_long_notes: vec![None; 8],
        passing_long_notes: vec![None; 8],
        hell_charge_judges: vec![false; 8],
        bad_judge_time: 0,
        model_bpm: 120.0,
        all_timelines,
        forced_cn_endings: false,
    };

    // 5. Call compute_note_draw_commands via SkinDrawable trait (closure-based API)
    skin.compute_note_draw_commands(&mut |lanes| lr.draw_lane(&draw_ctx, lanes, &[]).commands);

    // 6. Verify draw_commands is non-empty
    let note = find_note_object(&skin).expect("SkinNoteObject must still be in skin");
    assert!(
        !note.draw_commands.is_empty(),
        "draw_commands must be non-empty after compute_note_draw_commands(). \
         Got 0 commands - this means draw_lane() returned empty, likely because \
         lanes are empty or the SkinObject::Note was not found."
    );
}

// ===========================================================================
// Test 7: lanerender=false means no SkinNoteObject is created
// ===========================================================================

#[test]
fn no_note_object_when_lanerender_disabled() {
    let mut loader = make_loader_with_notes();
    loader.lanerender = false;
    let mut skin = make_skin();

    loader.assemble_objects(&mut skin);

    assert!(
        find_note_object(&skin).is_none(),
        "no SkinNoteObject should be created when lanerender is false"
    );
}

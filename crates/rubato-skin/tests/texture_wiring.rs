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

    note.draw_commands = vec![rubato_play::lane_renderer::DrawCommand::DrawNote {
        lane: 0,
        x: 10.0,
        y: 20.0,
        w: 40.0,
        h: 5.0,
        image_type: rubato_play::lane_renderer::NoteImageType::Normal,
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

    note.draw_commands = vec![rubato_play::lane_renderer::DrawCommand::DrawNote {
        lane: 0,
        x: 10.0,
        y: 20.0,
        w: 40.0,
        h: 5.0,
        image_type: rubato_play::lane_renderer::NoteImageType::Normal,
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
// Test 6: lanerender=false means no SkinNoteObject is created
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

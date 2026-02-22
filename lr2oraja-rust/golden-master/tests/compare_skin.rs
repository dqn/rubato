// Golden master tests for skin loaders.
//
// Tests JSON and LR2 skin loaders by loading test skin files and
// verifying structural properties of SkinHeaderData / SkinData / LR2SkinHeaderData.
//
// Lua loader tests are skipped: LuaSkinLoader is currently stubbed.
// ECFN skin tests are skipped: requires external skin files not in the repository.
// Full Skin snapshot tests are deferred: JSONSkinLoader returns SkinData (not Skin),
// so snapshot_from_skin() cannot be used until the full loading pipeline is connected.

use std::path::{Path, PathBuf};

use beatoraja_skin::json::json_skin_loader::{
    JSONSkinLoader, SkinConfigProperty, SkinData, SkinHeaderData,
};
use beatoraja_skin::lr2::lr2_skin_header_loader::LR2SkinHeaderLoader;
use beatoraja_types::skin_type::SkinType;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_bms_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test-bms")
}

fn load_json_header(filename: &str) -> SkinHeaderData {
    let path = test_bms_dir().join(filename);
    let mut loader = JSONSkinLoader::new();
    loader
        .load_header(&path)
        .unwrap_or_else(|| panic!("Failed to load JSON header: {}", path.display()))
}

fn load_json_skin(filename: &str) -> SkinData {
    let path = test_bms_dir().join(filename);
    let skin_type = SkinType::Decide;
    let property = SkinConfigProperty;
    let mut loader = JSONSkinLoader::new();
    loader
        .load(&path, &skin_type, &property)
        .unwrap_or_else(|| panic!("Failed to load JSON skin: {}", path.display()))
}

// ===========================================================================
// JSON Loader tests (using test_skin.json)
// ===========================================================================

#[test]
fn json_test_header() {
    let header = load_json_header("test_skin.json");

    // skin_type 6 = Decide
    assert_eq!(header.skin_type, 6, "skin_type should be Decide (6)");
    assert_eq!(header.name, "Test Skin");
    assert_eq!(header.author, "Test");

    // Resolution
    if let Some(ref res) = header.source_resolution {
        assert_eq!(res.width as i32, 1280, "source width");
        assert_eq!(res.height as i32, 720, "source height");
    } else {
        panic!("source_resolution should be Some");
    }

    // No custom options/files/offsets in this skin
    assert!(header.custom_options.is_empty(), "no custom options");
    assert!(header.custom_files.is_empty(), "no custom files");
    assert!(header.custom_offsets.is_empty(), "no custom offsets");
}

#[test]
fn json_test_load() {
    let skin = load_json_skin("test_skin.json");

    // Timing
    assert_eq!(skin.input, 400, "input");
    assert_eq!(skin.scene, 3000, "scene");
    assert_eq!(skin.fadeout, 500, "fadeout");

    // 3 destinations with negative IDs -> 3 objects
    assert_eq!(skin.objects.len(), 3, "object count");
}

#[test]
fn json_test_destinations() {
    let skin = load_json_skin("test_skin.json");

    // Object 0: id="-1", 1 destination, no timer, blend 0
    assert_eq!(skin.objects[0].destinations.len(), 1, "obj[0] dst count");
    let d0 = &skin.objects[0].destinations[0];
    assert_eq!(d0.blend, 0, "obj[0] blend");
    assert!(d0.timer.is_none(), "obj[0] no timer");
    assert_eq!(d0.x, 0);
    assert_eq!(d0.y, 0);
    assert_eq!(d0.w, 1280);
    assert_eq!(d0.h, 720);

    // Object 1: id="-2", 1 destination, timer=40
    assert_eq!(skin.objects[1].destinations.len(), 1, "obj[1] dst count");
    let d1 = &skin.objects[1].destinations[0];
    assert_eq!(d1.timer, Some(40), "obj[1] timer");
    assert_eq!(d1.x, 100);
    assert_eq!(d1.y, 100);
    assert_eq!(d1.w, 200);
    assert_eq!(d1.h, 200);

    // Object 2: id="-3", blend=2 (additive), 2 destinations
    assert_eq!(skin.objects[2].destinations.len(), 2, "obj[2] dst count");
    assert_eq!(skin.objects[2].destinations[0].blend, 2, "obj[2] blend");
    assert_eq!(skin.objects[2].destinations[0].x, 50);
    assert_eq!(skin.objects[2].destinations[1].x, 150);
    assert_eq!(skin.objects[2].destinations[1].w, 200);
}

// ===========================================================================
// JSON Options tests (using test_skin_options.json)
// ===========================================================================

#[test]
fn json_options_header() {
    let header = load_json_header("test_skin_options.json");

    // skin_type 5 = MusicSelect
    assert_eq!(header.skin_type, 5, "skin_type should be MusicSelect (5)");
    assert_eq!(header.name, "Test Options Skin");

    // Resolution
    if let Some(ref res) = header.source_resolution {
        assert_eq!(res.width as i32, 1920, "source width");
        assert_eq!(res.height as i32, 1080, "source height");
    }

    // property (custom option)
    assert_eq!(header.custom_options.len(), 1, "property count");
    assert_eq!(header.custom_options[0].name, "BG Style");
    assert_eq!(header.custom_options[0].option, vec![900, 901]);
    assert_eq!(header.custom_options[0].names, vec!["Dark", "Light"]);
    assert_eq!(
        header.custom_options[0].def.as_deref(),
        Some("Dark"),
        "default label"
    );

    // filepath
    assert_eq!(header.custom_files.len(), 1, "filepath count");
    assert_eq!(header.custom_files[0].name, "Background");
    assert_eq!(header.custom_files[0].path, "bg/*.png");
    assert_eq!(
        header.custom_files[0].def.as_deref(),
        Some("default.png"),
        "default filename"
    );

    // offset
    assert_eq!(header.custom_offsets.len(), 1, "offset count");
    assert_eq!(header.custom_offsets[0].name, "Judge Position");
    assert_eq!(header.custom_offsets[0].id, 50);
    assert!(header.custom_offsets[0].x, "editable_x");
    assert!(header.custom_offsets[0].y, "editable_y");
    assert!(!header.custom_offsets[0].w, "not editable_w");
    assert!(!header.custom_offsets[0].h, "not editable_h");
    assert!(!header.custom_offsets[0].r, "not editable_r");
    assert!(!header.custom_offsets[0].a, "not editable_a");
}

#[test]
fn json_options_load() {
    let skin = load_json_skin("test_skin_options.json");

    // Timing
    assert_eq!(skin.input, 300, "input");
    assert_eq!(skin.scene, 2000, "scene");
    assert_eq!(skin.fadeout, 400, "fadeout");

    // 2 destinations with negative IDs -> 2 objects
    // (option filtering is done at render time, not at load time)
    assert_eq!(skin.objects.len(), 2, "object count");

    // First object: unconditional (no op)
    assert!(
        skin.objects[0].destinations[0].op.is_empty(),
        "obj[0] should have no op conditions"
    );

    // Second object: conditional on option 900
    assert_eq!(
        skin.objects[1].destinations[0].op,
        vec![900],
        "obj[1] should have op=[900]"
    );
    // Destination coordinates (stored as raw JSON values, not scaled)
    assert_eq!(skin.objects[1].destinations[0].w, 960);
    assert_eq!(skin.objects[1].destinations[0].h, 540);
    assert_eq!(skin.objects[1].destinations[0].a, 128);
}

// ===========================================================================
// LR2 CSV Loader tests (header only)
// ===========================================================================

#[test]
fn lr2_csv_header() {
    let path = test_bms_dir().join("test_skin.lr2skin");
    let skinpath = path.to_string_lossy().to_string();
    let mut loader = LR2SkinHeaderLoader::new(&skinpath);
    let header = loader
        .load_skin(&path, None)
        .expect("Failed to load LR2 header");

    // #INFORMATION,0,...  -> Play7Keys
    assert_eq!(
        header.skin_type,
        Some(SkinType::Play7Keys),
        "skin_type should be Play7Keys"
    );
    assert_eq!(header.name, "TestLR2Skin");
    assert_eq!(header.author, "TestAuthor");

    // #RESOLUTION,1 -> HD (1280x720)
    if let Some(ref res) = header.resolution {
        assert_eq!(res.width as i32, 1280, "resolution width");
        assert_eq!(res.height as i32, 720, "resolution height");
    } else {
        panic!("resolution should be Some for #RESOLUTION,1");
    }

    // #CUSTOMOPTION
    // LR2 adds default play skin options (BGA Size, Ghost, etc.) for play skin types,
    // so we search by name instead of checking index
    let test_option = header
        .custom_options
        .iter()
        .find(|o| o.name == "TestOption");
    assert!(test_option.is_some(), "TestOption should exist");
    let test_option = test_option.unwrap();
    assert_eq!(test_option.option, vec![900, 901]);
    assert_eq!(test_option.contents, vec!["ON", "OFF"]);

    // #CUSTOMFILE
    assert!(
        !header.custom_files.is_empty(),
        "should have at least 1 custom file"
    );
    let bg_file = header.custom_files.iter().find(|f| f.name == "Background");
    assert!(bg_file.is_some(), "Background custom file should exist");
    let bg_file = bg_file.unwrap();
    assert_eq!(bg_file.path, "bg/*.png");
    assert_eq!(bg_file.def.as_deref(), Some("default.png"));

    // #CUSTOMOFFSET
    assert!(
        !header.custom_offsets.is_empty(),
        "should have at least 1 custom offset"
    );
    let lift_offset = header
        .custom_offsets
        .iter()
        .find(|o| o.name == "LiftOffset");
    assert!(lift_offset.is_some(), "LiftOffset should exist");
    let lift_offset = lift_offset.unwrap();
    assert_eq!(lift_offset.id, 3);
    assert!(!lift_offset.x, "not editable_x");
    assert!(lift_offset.y, "editable_y");
}

// ===========================================================================
// Lua Loader tests — SKIPPED
// ===========================================================================
// LuaSkinLoader.load_header() and load_skin() are currently stubbed (return None).
// Lua skin tests will be activated when the loader is implemented.

// ===========================================================================
// ECFN Skin tests — SKIPPED
// ===========================================================================
// ECFN skins require external skin files (skin/ECFN/) not present in the repository.
// These tests will be activated when ECFN skin files are available.

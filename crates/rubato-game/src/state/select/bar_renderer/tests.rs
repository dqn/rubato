use super::*;
use crate::core::config::Config;
use crate::state::select::bar::folder_bar::FolderBar;
use crate::state::select::bar::song_bar::SongBar;
use rubato_skin::json::json_skin_loader::SkinConfigProperty;
use rubato_skin::reexports::Timer;
use rubato_skin::skin_data_converter;
use rubato_skin::skin_text::SkinText;
use rubato_skin::skin_text::SkinTextEnum;
use rubato_skin::text::skin_text_bitmap::{SkinTextBitmap, SkinTextBitmapSource};
use rubato_types::skin_type::SkinType;

/// Create a test SkinImage with draw=true and specified region.
/// Uses a default TextureRegion (no real texture, but valid for layout tests).
/// Sets `fixr` so that `SkinObjectData::prepare_region` preserves `draw=true`
/// even when `dst` is empty (the default).
fn make_test_image(x: f32, y: f32, w: f32, h: f32) -> SkinImage {
    let mut img = SkinImage::new_with_single(TextureRegion::default());
    img.data.draw = true;
    img.data.region = Rectangle::new(x, y, w, h);
    img.data.fixr = Some(Rectangle::new(x, y, w, h));
    img
}

/// Mock MainState for testing (implements rubato_skin::reexports::MainState)
struct MockMainState {
    timer: Timer,
}

impl Default for MockMainState {
    fn default() -> Self {
        Self {
            timer: Timer::default(),
        }
    }
}

impl rubato_types::timer_access::TimerAccess for MockMainState {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }
    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for(timer_id)
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for MockMainState {}

impl MainState for MockMainState {}

fn make_song_data(sha256: &str, path: Option<&str>) -> SongData {
    let mut sd = SongData::default();
    sd.file.sha256 = sha256.to_string();
    if let Some(p) = path {
        sd.file.set_path(p.to_string());
    }
    sd
}

fn make_song_bar_bar(sha256: &str, path: Option<&str>) -> Bar {
    Bar::Song(Box::new(SongBar::new(make_song_data(sha256, path))))
}

fn ecfn_barfont_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skin/ECFN/_font/barfont.fnt")
}

fn ecfn_select_skin_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../skin/ECFN/select/select.luaskin")
}

#[test]
fn test_bar_renderer_new() {
    let renderer = BarRenderer::new(300, 100, 5);
    assert_eq!(renderer.durationlow, 300);
    assert_eq!(renderer.durationhigh, 100);
    assert_eq!(renderer.analog_ticks_per_scroll, 5);
    assert_eq!(renderer.barlength, 60);
    assert_eq!(renderer.duration, 0);
    assert_eq!(renderer.angle, 0);
    assert!(!renderer.keyinput);
    assert!(renderer.bartextupdate);
}

#[test]
fn test_bar_renderer_two_phase_prepare_render() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);

    let songs: Vec<Bar> = (0..60)
        .map(|i| make_song_bar_bar(&format!("song{}", i), Some("/path.bms")))
        .collect();

    let prep_ctx = PrepareContext {
        center_bar: 0,
        currentsongs: &songs,
        selectedindex: 0,
    };

    // Phase 1: prepare
    renderer.prepare(&bar, 1000, &prep_ctx);
    assert_eq!(renderer.time, 1000);

    // Phase 2: render
    let mut sprite = SkinObjectRenderer::new();
    let state = MockMainState::default();
    let render_ctx = RenderContext {
        center_bar: 0,
        currentsongs: &songs,
        rival: false,
        state: &state,
        lnmode: 0,
    };
    renderer.render(&mut sprite, &mut bar, &render_ctx);
}

#[test]
fn test_bar_renderer_prepare_stores_time() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let bar = SkinBar::new(0);
    let songs = vec![make_song_bar_bar("a", Some("/a.bms"))];

    let ctx = PrepareContext {
        center_bar: 0,
        currentsongs: &songs,
        selectedindex: 0,
    };

    renderer.prepare(&bar, 5000, &ctx);
    assert_eq!(renderer.time, 5000);

    renderer.prepare(&bar, 10000, &ctx);
    assert_eq!(renderer.time, 10000);
}

#[test]
fn test_bar_renderer_prepare_empty_songs() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let bar = SkinBar::new(0);
    let songs: Vec<Bar> = Vec::new();

    let ctx = PrepareContext {
        center_bar: 0,
        currentsongs: &songs,
        selectedindex: 0,
    };

    renderer.prepare(&bar, 1000, &ctx);
    assert_eq!(renderer.time, 1000);
}

#[test]
fn test_bar_renderer_prepare_bar_type_classification() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);

    // Set a bar image on index 0 with draw=true so prepare processes it
    bar.barimageon[0] = Some(make_test_image(10.0, 20.0, 100.0, 30.0));
    bar.barimageoff[0] = Some(make_test_image(10.0, 20.0, 100.0, 30.0));

    // Create a song bar (exists)
    let songs = vec![make_song_bar_bar("abc", Some("/path.bms"))];
    let ctx = PrepareContext {
        center_bar: 0,
        currentsongs: &songs,
        selectedindex: 0,
    };

    renderer.prepare(&bar, 1000, &ctx);

    // Bar area 0 should have value 0 (SongBar exists)
    assert_eq!(renderer.bararea[0].value, 0);
    assert!(renderer.bararea[0].sd.is_some());
}

#[test]
fn test_bar_renderer_prepare_folder_bar_type() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);

    bar.barimageoff[0] = Some(make_test_image(0.0, 0.0, 100.0, 30.0));

    let songs = vec![Bar::Folder(Box::new(FolderBar::new(
        None,
        "test".to_string(),
    )))];
    let ctx = PrepareContext {
        center_bar: 1, // center is 1, so bar 0 uses off image
        currentsongs: &songs,
        selectedindex: 0,
    };

    renderer.prepare(&bar, 1000, &ctx);
    // FolderBar -> value 1
    assert_eq!(renderer.bararea[0].value, 1);
}

#[test]
fn test_bar_renderer_prepare_song_bar_missing() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);

    bar.barimageoff[0] = Some(make_test_image(0.0, 0.0, 0.0, 0.0));

    // SongBar with no path = missing
    let songs = vec![make_song_bar_bar("abc", None)];
    let ctx = PrepareContext {
        center_bar: 1,
        currentsongs: &songs,
        selectedindex: 0,
    };

    renderer.prepare(&bar, 1000, &ctx);
    // Missing SongBar -> value 4
    assert_eq!(renderer.bararea[0].value, 4);
}

#[test]
fn test_bar_renderer_update_bar_text() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    // bartextupdate starts true to ensure fonts are prepared on first render pass
    assert!(renderer.bartextupdate);
    renderer.update_bar_text();
    assert!(renderer.bartextupdate);
}

#[test]
fn test_bar_renderer_render_bartextupdate_collects_chars() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    renderer.bartextupdate = true;

    let mut bar = SkinBar::new(0);

    // Create a song bar with a non-empty title
    let mut sd = SongData::default();
    sd.file.sha256 = "abc".to_string();
    sd.file.set_path("/path.bms".to_string());
    sd.metadata.title = "Test Song Title".to_string();
    let songs = vec![Bar::Song(Box::new(SongBar::new(sd)))];

    let state = MockMainState::default();
    let render_ctx = RenderContext {
        center_bar: 0,
        currentsongs: &songs,
        rival: false,
        state: &state,
        lnmode: 0,
    };

    let mut sprite = SkinObjectRenderer::new();
    renderer.render(&mut sprite, &mut bar, &render_ctx);

    // bartextupdate should be reset after render
    assert!(!renderer.bartextupdate);
    // bartextcharset should contain characters from the song title
    assert!(!renderer.bartextcharset.is_empty());
    assert!(renderer.bartextcharset.contains(&'T'));
    assert!(renderer.bartextcharset.contains(&'e'));
}

#[test]
fn test_bar_renderer_render_draws_ecfn_bitmap_bartext_quads() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);
    let font_path = ecfn_barfont_path();
    assert!(
        font_path.exists(),
        "ECFN bar bitmap font should exist: {}",
        font_path.display()
    );

    let source = SkinTextBitmapSource::new(font_path, false);
    let mut text = SkinTextBitmap::new(source, 25.0);
    text.text_data.data.region = Rectangle::new(155.0, 13.0, 580.0, 24.0);
    text.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
    bar.set_text(SkinBar::BARTEXT_SONG_NORMAL, SkinTextEnum::Bitmap(text));
    bar.barimageon[0] = Some(make_test_image(1258.0, 538.0, 730.0, 52.0));
    bar.barimageoff[0] = Some(make_test_image(1258.0, 538.0, 730.0, 52.0));

    let mut sd = SongData::default();
    sd.file.sha256 = "bitmap-bartext".to_string();
    sd.file.set_path("/path.bms".to_string());
    sd.metadata.title = "FolderSong abc".to_string();
    let songs = vec![Bar::Song(Box::new(SongBar::new(sd)))];

    let prep_ctx = PrepareContext {
        center_bar: 0,
        currentsongs: &songs,
        selectedindex: 0,
    };
    renderer.prepare(&bar, 1000, &prep_ctx);

    let state = MockMainState::default();
    let render_ctx = RenderContext {
        center_bar: 0,
        currentsongs: &songs,
        rival: false,
        state: &state,
        lnmode: 0,
    };

    let mut sprite = SkinObjectRenderer::new();
    sprite.sprite.enable_capture();
    renderer.render(&mut sprite, &mut bar, &render_ctx);

    let quads = sprite
        .sprite
        .captured_quads()
        .iter()
        .filter(|quad| quad.texture_key.is_some())
        .collect::<Vec<_>>();
    assert!(
        !quads.is_empty(),
        "bar renderer should emit textured glyph quads for bitmap bar text"
    );
}

#[test]
fn test_bar_renderer_render_draws_ecfn_loaded_songlist_bitmap_bartext_quads() {
    let path = ecfn_select_skin_path();
    assert!(
        path.exists(),
        "ECFN select skin should exist: {}",
        path.display()
    );

    let mut loader =
        rubato_skin::lua::lua_skin_loader::LuaSkinLoader::new_without_state(&Config::default());
    let header = loader
        .load_header(&path)
        .expect("ECFN select Lua skin header should load");
    let data = loader
        .load(&path, &SkinType::MusicSelect, &SkinConfigProperty)
        .expect("ECFN select Lua skin should load into SkinData");
    let mut skin = skin_data_converter::convert_skin_data(
        &header,
        data,
        &mut loader.json_loader.source_map,
        &path,
        loader.json_loader.usecim,
        &loader.json_loader.dstr,
        &loader.json_loader.filemap,
    )
    .expect("ECFN select Lua skin should convert into runtime Skin");
    let mut bar_data = skin
        .take_select_bar_data()
        .expect("ECFN select skin should expose SelectBarData");

    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);
    bar.set_text(
        SkinBar::BARTEXT_SONG_NORMAL,
        bar_data.bartext[SkinBar::BARTEXT_SONG_NORMAL]
            .take()
            .expect("ECFN select skin should provide songlist song text"),
    );
    bar.barimageon[0] = Some(make_test_image(1258.0, 538.0, 730.0, 52.0));
    bar.barimageoff[0] = Some(make_test_image(1258.0, 538.0, 730.0, 52.0));

    let mut sd = SongData::default();
    sd.file.sha256 = "ecfn-loaded-bartext".to_string();
    sd.file.set_path("/path.bms".to_string());
    sd.metadata.title = "FolderSong abc".to_string();
    let songs = vec![Bar::Song(Box::new(SongBar::new(sd)))];

    let prep_ctx = PrepareContext {
        center_bar: 0,
        currentsongs: &songs,
        selectedindex: 0,
    };
    renderer.prepare(&bar, 1000, &prep_ctx);

    let state = MockMainState::default();
    bar.prepare(1000, &state);
    let render_ctx = RenderContext {
        center_bar: 0,
        currentsongs: &songs,
        rival: false,
        state: &state,
        lnmode: 0,
    };

    let mut sprite = SkinObjectRenderer::new();
    sprite.sprite.enable_capture();
    renderer.render(&mut sprite, &mut bar, &render_ctx);

    let quads = sprite
        .sprite
        .captured_quads()
        .iter()
        .filter(|quad| quad.texture_key.is_some())
        .collect::<Vec<_>>();
    assert!(
        !quads.is_empty(),
        "bar renderer should emit textured glyph quads for ECFN-loaded bitmap bar text"
    );
}

#[test]
fn test_bar_renderer_centers_ecfn_loaded_songlist_bitmap_bartext_vertically() {
    let path = ecfn_select_skin_path();
    assert!(
        path.exists(),
        "ECFN select skin should exist: {}",
        path.display()
    );

    let mut loader =
        rubato_skin::lua::lua_skin_loader::LuaSkinLoader::new_without_state(&Config::default());
    let header = loader
        .load_header(&path)
        .expect("ECFN select Lua skin header should load");
    let data = loader
        .load(&path, &SkinType::MusicSelect, &SkinConfigProperty)
        .expect("ECFN select Lua skin should load into SkinData");
    let mut skin = skin_data_converter::convert_skin_data(
        &header,
        data,
        &mut loader.json_loader.source_map,
        &path,
        loader.json_loader.usecim,
        &loader.json_loader.dstr,
        &loader.json_loader.filemap,
    )
    .expect("ECFN select Lua skin should convert into runtime Skin");
    let mut bar_data = skin
        .take_select_bar_data()
        .expect("ECFN select skin should expose SelectBarData");

    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);
    let on_region = bar_data.barimageon[0]
        .as_ref()
        .and_then(|image| image.data.all_destination().first().map(|dst| dst.region))
        .expect("ECFN select skin should provide scaled selected bar destination");
    let off_region = bar_data.barimageoff[0]
        .as_ref()
        .and_then(|image| image.data.all_destination().first().map(|dst| dst.region))
        .expect("ECFN select skin should provide scaled unselected bar destination");
    bar.set_text(
        SkinBar::BARTEXT_SONG_NORMAL,
        bar_data.bartext[SkinBar::BARTEXT_SONG_NORMAL]
            .take()
            .expect("ECFN select skin should provide songlist song text"),
    );
    bar.barimageon[0] = Some(make_test_image(
        on_region.x,
        on_region.y,
        on_region.width,
        on_region.height,
    ));
    bar.barimageoff[0] = Some(make_test_image(
        off_region.x,
        off_region.y,
        off_region.width,
        off_region.height,
    ));

    let mut sd = SongData::default();
    sd.file.sha256 = "ecfn-loaded-bartext-center".to_string();
    sd.file.set_path("/path.bms".to_string());
    sd.metadata.title = "FolderSong abc".to_string();
    let songs = vec![Bar::Song(Box::new(SongBar::new(sd)))];

    let prep_ctx = PrepareContext {
        center_bar: 0,
        currentsongs: &songs,
        selectedindex: 0,
    };
    renderer.prepare(&bar, 1000, &prep_ctx);

    let state = MockMainState::default();
    bar.prepare(1000, &state);
    let render_ctx = RenderContext {
        center_bar: 0,
        currentsongs: &songs,
        rival: false,
        state: &state,
        lnmode: 0,
    };

    let text_region = bar
        .text(SkinBar::BARTEXT_SONG_NORMAL)
        .expect("songlist text should stay available")
        .get_text_data()
        .data
        .region;
    let expected_center_y = renderer.bararea[0].y + text_region.y + text_region.height / 2.0;

    let mut sprite = SkinObjectRenderer::new();
    sprite.sprite.enable_capture();
    renderer.render(&mut sprite, &mut bar, &render_ctx);

    let glyph_quads = sprite
        .sprite
        .captured_quads()
        .iter()
        .filter(|quad| {
            quad.texture_key
                .as_deref()
                .is_some_and(|texture| texture.starts_with("__pixmap_"))
        })
        .collect::<Vec<_>>();
    assert!(
        !glyph_quads.is_empty(),
        "bar renderer should emit bitmap glyph quads for ECFN-loaded songlist text"
    );

    let min_y = glyph_quads
        .iter()
        .map(|quad| quad.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = glyph_quads
        .iter()
        .map(|quad| quad.y + quad.h)
        .fold(f32::NEG_INFINITY, f32::max);
    let glyph_center_y = (min_y + max_y) / 2.0;

    assert!(
        (glyph_center_y - expected_center_y).abs() <= 4.0,
        "songlist bitmap text should stay vertically centered in its destination, got glyph_center_y={}, expected_center_y={}, glyph_bbox=({}, {}), text_region=({}, {}, {}, {}), bar_offset_y={}",
        glyph_center_y,
        expected_center_y,
        min_y,
        max_y,
        text_region.x,
        text_region.y,
        text_region.width,
        text_region.height,
        renderer.bararea[0].y
    );
}

#[test]
fn test_bar_renderer_prepare_angle_zero_no_nan() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);

    // Set up two adjacent bar images so the lerp path is exercised
    bar.barimageoff[0] = Some(make_test_image(10.0, 20.0, 100.0, 30.0));
    bar.barimageoff[1] = Some(make_test_image(10.0, 60.0, 100.0, 30.0));

    let songs = vec![
        make_song_bar_bar("a", Some("/a.bms")),
        make_song_bar_bar("b", Some("/b.bms")),
    ];

    // Simulate: angle=0 but duration far in the future => apply_movement=true
    // This triggers the division-by-zero path: angle_lerp = ... / self.angle as f32
    renderer.angle = 0;
    renderer.duration = i64::MAX;

    let ctx = PrepareContext {
        center_bar: 2,
        currentsongs: &songs,
        selectedindex: 0,
    };

    renderer.prepare(&bar, 1000, &ctx);

    // When angle=0, no movement should be applied. Bar positions should match
    // the skin-defined positions (the bar image region coordinates).
    // Before the fix, division by zero produced Infinity which corrupted positions.
    assert_eq!(
        renderer.bararea[0].x, 10.0,
        "bararea[0].x should match skin position"
    );
    assert_eq!(
        renderer.bararea[0].y, 20.0,
        "bararea[0].y should match skin position"
    );
    assert_eq!(
        renderer.bararea[1].x, 10.0,
        "bararea[1].x should match skin position"
    );
    assert_eq!(
        renderer.bararea[1].y, 60.0,
        "bararea[1].y should match skin position"
    );
}

#[test]
fn test_bar_renderer_mouse_pressed_no_songs() {
    let renderer = BarRenderer::new(300, 100, 5);
    let bar = SkinBar::new(0);
    let state = MockMainState::default();

    let ctx = MousePressedContext {
        clickable_bar: &[0, 1, 2],
        center_bar: 1,
        currentsongs: &[],
        selectedindex: 0,
        state: &state,
        timer_now_time: 0,
    };

    let result = renderer.mouse_pressed(&bar, 0, 100, 200, &ctx);
    assert!(matches!(result, MousePressedAction::None));
}

#[test]
fn test_bar_renderer_mouse_pressed_no_hit() {
    let renderer = BarRenderer::new(300, 100, 5);
    let bar = SkinBar::new(0);
    let songs = vec![make_song_bar_bar("abc", Some("/path.bms"))];
    let state = MockMainState::default();

    let ctx = MousePressedContext {
        clickable_bar: &[0],
        center_bar: 0,
        currentsongs: &songs,
        selectedindex: 0,
        state: &state,
        timer_now_time: 0,
    };

    // No bar images set, so bar_images returns None -> no hit
    let result = renderer.mouse_pressed(&bar, 0, 100, 200, &ctx);
    assert!(matches!(result, MousePressedAction::None));
}

/// When a bar image's `draw` flag is false at render time, `draw_bar_images`
/// must set `bararea[i].value = -1` so that subsequent draw passes (text, lamps,
/// levels, etc.) skip that slot. Without this invalidation, floating UI elements
/// appear on invisible bars. The `draw` flag is a per-frame skin property that
/// can change between prepare and render, so render-time invalidation is needed.
#[test]
fn test_draw_bar_images_invalidates_value_when_draw_is_false() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);

    // Both bar images start with draw=true so prepare assigns valid values
    bar.barimageoff[0] = Some(make_test_image(0.0, 0.0, 100.0, 30.0));
    bar.barimageoff[1] = Some(make_test_image(0.0, 40.0, 100.0, 30.0));

    let songs = vec![
        make_song_bar_bar("a", Some("/a.bms")),
        make_song_bar_bar("b", Some("/b.bms")),
    ];

    let prep_ctx = PrepareContext {
        center_bar: 2, // neither 0 nor 1, so both use off images
        currentsongs: &songs,
        selectedindex: 0,
    };
    renderer.prepare(&bar, 1000, &prep_ctx);

    // After prepare, both bars should have valid (non -1) values
    assert_ne!(
        renderer.bararea[0].value, -1,
        "bar 0 should be valid after prepare"
    );
    assert_ne!(
        renderer.bararea[1].value, -1,
        "bar 1 should be valid after prepare"
    );

    // Simulate skin draw condition flipping bar 1's draw flag to false between
    // prepare and render (this happens per-frame with skin draw conditions).
    bar.barimageoff[1].as_mut().unwrap().data.draw = false;

    let state = MockMainState::default();
    let render_ctx = RenderContext {
        center_bar: 2,
        currentsongs: &songs,
        rival: false,
        state: &state,
        lnmode: 0,
    };

    let mut sprite = SkinObjectRenderer::new();
    renderer.render(&mut sprite, &mut bar, &render_ctx);

    // Bar 0 (draw=true) should keep its value
    assert_ne!(
        renderer.bararea[0].value, -1,
        "bar 0 with draw=true should retain value"
    );
    // Bar 1 (draw=false at render time) should be invalidated to -1
    assert_eq!(
        renderer.bararea[1].value, -1,
        "bar 1 with draw=false should be invalidated to -1"
    );
}

/// Regression: when center_bar is negative, `center_bar as usize` wraps to
/// usize::MAX, corrupting the index computation. The fix uses i64 arithmetic
/// with rem_euclid to handle negative values correctly.
#[test]
fn test_bar_renderer_prepare_negative_center_bar() {
    let mut renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);

    bar.barimageoff[0] = Some(make_test_image(10.0, 20.0, 100.0, 30.0));

    let songs = vec![
        make_song_bar_bar("a", Some("/a.bms")),
        make_song_bar_bar("b", Some("/b.bms")),
        make_song_bar_bar("c", Some("/c.bms")),
    ];

    // Negative center_bar should not panic or produce wrong indices
    let ctx = PrepareContext {
        center_bar: -1,
        currentsongs: &songs,
        selectedindex: 0,
    };

    renderer.prepare(&bar, 1000, &ctx);

    // Should not panic, and bararea[0] should have a valid song reference
    assert!(renderer.bararea[0].sd.is_some());
    let idx = renderer.bararea[0].sd.unwrap();
    assert!(
        idx < songs.len(),
        "index {} should be within song list bounds",
        idx
    );
}

/// Regression: mouse_pressed with negative center_bar should not panic.
#[test]
fn test_bar_renderer_mouse_pressed_negative_center_bar() {
    let renderer = BarRenderer::new(300, 100, 5);
    let mut bar = SkinBar::new(0);
    let songs = vec![
        make_song_bar_bar("a", Some("/a.bms")),
        make_song_bar_bar("b", Some("/b.bms")),
    ];
    let state = MockMainState::default();

    // Set up a clickable bar image so the hit-test loop body runs
    bar.barimageoff[0] = Some(make_test_image(0.0, 0.0, 200.0, 200.0));

    let ctx = MousePressedContext {
        clickable_bar: &[0],
        center_bar: -1,
        currentsongs: &songs,
        selectedindex: 0,
        state: &state,
        timer_now_time: 0,
    };

    // Should not panic
    let _result = renderer.mouse_pressed(&bar, 0, 100, 100, &ctx);
}

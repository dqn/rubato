use super::*;
use crate::select::bar::folder_bar::FolderBar;
use crate::select::bar::song_bar::SongBar;
use rubato_skin::stubs::{MainController, PlayerResource, SkinOffset, Timer};

/// Create a test SkinImage with draw=true and specified region.
/// Uses a default TextureRegion (no real texture, but valid for layout tests).
fn make_test_image(x: f32, y: f32, w: f32, h: f32) -> SkinImage {
    let mut img = SkinImage::new_with_single(TextureRegion::default());
    img.data.draw = true;
    img.data.region = Rectangle::new(x, y, w, h);
    img
}

/// Mock MainState for testing (implements rubato_skin::stubs::MainState)
struct MockMainState {
    timer: Timer,
    main: MainController,
    resource: PlayerResource,
}

impl Default for MockMainState {
    fn default() -> Self {
        Self {
            timer: Timer::default(),
            main: MainController { debug: false },
            resource: PlayerResource,
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

impl MainState for MockMainState {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
        &self.timer
    }
    fn get_main(&self) -> &MainController {
        &self.main
    }
    fn get_image(&self, _id: i32) -> Option<rubato_skin::stubs::TextureRegion> {
        None
    }
    fn get_resource(&self) -> &PlayerResource {
        &self.resource
    }
}

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
    assert!(!renderer.bartextupdate);
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
        loader_finished: false,
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
    assert!(!renderer.bartextupdate);
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
        loader_finished: false,
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

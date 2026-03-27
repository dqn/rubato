// E2E gameplay lifecycle tests.
//
// Tests the full BMS load -> play -> result lifecycle with real BMS data.
// Unlike e2e_screen_transitions.rs (which tests transitions with default/empty models),
// these tests load actual BMS files and verify the complete gameplay pipeline works
// end-to-end with real chart data.
//
// Verifies:
// - BMS file loading via PlayerResource.set_bms_file() with real test fixtures
// - State transitions with loaded BMS data: MusicSelect -> Decide -> Play -> Result
// - Direct BMS launch path: create() with bmsfile -> Play -> Result
// - Lifecycle methods (create/render/dispose) work with real chart data in each state
// - PlayerResource correctly propagates BMS model to Play state via factory

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rubato_game::core::bms_player_mode::BMSPlayerMode;
use rubato_game::core::config::Config;
use rubato_game::core::main_controller::MainController;
use rubato_game::core::main_loader::MainLoader;
use rubato_game::core::main_state::MainStateType;
use rubato_game::core::player_config::PlayerConfig;
use rubato_game::core::score_database_accessor::ScoreDatabaseAccessor;
use rubato_game::song::song_information_accessor::SongInformationAccessor;
use rubato_game::state::select::bar::bar::Bar;
use rubato_game::state::select::bar::folder_bar::FolderBar;
use rubato_game::state::select::bar::song_bar::SongBar;
use rubato_game::state::select::music_selector::MusicSelector;
use rubato_game::state_factory::LauncherStateFactory;
use rubato_input::gdx_compat::set_shared_key_state;
use rubato_input::keys::Keys;
use rubato_input::winit_input_bridge::SharedKeyState;
use rubato_render::sprite_batch::CapturedDrawQuad;
use rubato_types::folder_data::FolderData;
use rubato_types::main_controller_access::MainControllerAccess;
use rubato_types::skin_config::SkinConfig;
use rubato_types::skin_type::SkinType;
use rubato_types::song_data::SongData;
use rubato_types::song_information::SongInformation;
use rubato_types::timer_id::TimerId;

fn test_bms_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-bms")
        .join("minimal_7k.bms")
}

fn make_controller_with_factory() -> MainController {
    let config = Config::default();
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(LauncherStateFactory::new().into_creator());
    mc
}

fn ecfn_player_config() -> PlayerConfig {
    let mut player = PlayerConfig::default();
    player.skin[SkinType::Play7Keys.id() as usize] =
        Some(SkinConfig::new_with_path("skin/ECFN/play/play7.luaskin"));
    player.skin[SkinType::MusicSelect.id() as usize] =
        Some(SkinConfig::new_with_path("skin/ECFN/select/select.luaskin"));
    player.skin[SkinType::Decide.id() as usize] =
        Some(SkinConfig::new_with_path("skin/ECFN/decide/decide.luaskin"));
    player.validate();
    player
}

fn default_json_select_player_config() -> PlayerConfig {
    let mut player = PlayerConfig::default();
    player.skin[SkinType::MusicSelect.id() as usize] =
        Some(SkinConfig::new_with_path("skin/default/select.json"));
    player.validate();
    player
}

fn make_song_bar(path: &Path) -> Bar {
    let mut song = SongData::default();
    song.metadata.title = "minimal_7k".to_string();
    song.chart.mode = 7;
    song.file.sha256 = "select-enter-minimal".to_string();
    song.file.set_path(path.to_string_lossy().to_string());
    Bar::Song(Box::new(SongBar::new(song)))
}

fn make_select_stats_song(path: &Path) -> SongData {
    let mut song = SongData::default();
    song.metadata.title = "select-stats".to_string();
    song.chart.mode = 7;
    song.chart.maxbpm = 180;
    song.chart.minbpm = 90;
    song.chart.level = 12;
    song.file.sha256 = "s".repeat(64);
    song.file.set_path(path.to_string_lossy().to_string());
    song.parent = "e2977170".to_string();
    song
}

fn make_select_stats_score(sha256: &str) -> rubato_game::core::score_data::ScoreData {
    let mut score = rubato_game::core::score_data::ScoreData {
        sha256: sha256.to_string(),
        ..Default::default()
    };
    score.judge_counts.epg = 100;
    score.judge_counts.lpg = 20;
    score.judge_counts.egr = 15;
    score.judge_counts.lgr = 5;
    score.notes = 400;
    score.maxcombo = 321;
    score.minbp = 7;
    score.playcount = 10;
    score.clearcount = 6;
    score
}

fn make_ecfn_title_song(path: &Path) -> SongData {
    let mut song = SongData::default();
    song.metadata.title = "ふぁんぶる！".to_string();
    song.metadata.subtitle = "[SP NORMAL]".to_string();
    song.metadata.genre = "Electronica".to_string();
    song.metadata.artist = "藤原ハガネ feat. 重音テトSV".to_string();
    song.chart.mode = 7;
    song.file.sha256 = "e".repeat(64);
    song.file.set_path(path.to_string_lossy().to_string());
    song
}

fn make_ecfn_title_song_bar(path: &Path) -> Bar {
    Bar::Song(Box::new(SongBar::new(make_ecfn_title_song(path))))
}

fn make_ecfn_songlist_song_bar(path: &Path) -> Bar {
    let mut song = SongData::default();
    song.metadata.title = "FolderSong abc".to_string();
    song.chart.mode = 7;
    song.file.sha256 = "f".repeat(64);
    song.file.set_path(path.to_string_lossy().to_string());
    Bar::Song(Box::new(SongBar::new(song)))
}

fn make_ecfn_folder_bar() -> Bar {
    let folder = FolderData {
        title: "folder name abc".to_string(),
        adddate: 0,
        ..Default::default()
    };
    Bar::Folder(Box::new(FolderBar::new(
        Some(folder),
        "folder-bar-crc".to_string(),
    )))
}

fn write_song_info_row(path: &Path, info: &SongInformation) {
    let conn = rusqlite::Connection::open(path).expect("song info db should open");
    conn.execute(
        "INSERT INTO information (
            sha256, n, ln, s, ls, total, density, peakdensity, enddensity, mainbpm,
            distribution, speedchange, lanenotes
        ) VALUES (?1, 0, 0, 0, 0, 0.0, 0.0, 0.0, 0.0, ?2, '', '', '')",
        rusqlite::params![info.sha256, info.mainbpm],
    )
    .expect("song info row should insert");
}

// ---------------------------------------------------------------------------
// Reusable MusicSelect render test harness
// ---------------------------------------------------------------------------

/// Build a MainController with ECFN Lua skin + shared MusicSelector.
/// Returns the controller and the shared selector for post-create modifications.
fn build_ecfn_select_controller(bars: Vec<Bar>) -> (MainController, Arc<Mutex<MusicSelector>>) {
    let player = ecfn_player_config();
    let mut selector = MusicSelector::new();
    selector.config = player.clone();
    if !bars.is_empty() {
        selector.manager.currentsongs = bars;
        selector.manager.selectedindex = 0;
    }
    let shared = Arc::new(Mutex::new(selector));
    let mut mc = MainController::new(None, Config::default(), player, None, false);
    mc.set_state_factory(LauncherStateFactory::new().into_creator());
    mc.set_shared_music_selector(Arc::clone(&shared));
    mc.create();
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));
    (mc, shared)
}

/// Enable capture, render the specified number of frames, and return all quads.
fn capture_render_quads(mc: &mut MainController, frames: usize) -> Vec<CapturedDrawQuad> {
    mc.sprite_batch_mut()
        .expect("sprite batch should exist")
        .enable_capture();
    for _ in 0..frames {
        mc.render();
    }
    mc.sprite_batch()
        .expect("sprite batch should exist")
        .captured_quads()
        .to_vec()
}

// ---------------------------------------------------------------------------
// Semantic capture assertion helpers
// ---------------------------------------------------------------------------

/// Filter criteria for captured draw quads.
struct QuadFilter {
    /// Texture key prefix requirement.
    /// `None` = no texture requirement.
    /// `Some("")` = must have a texture key (any).
    /// `Some("__pixmap_")` = must start with that prefix.
    texture_prefix: Option<&'static str>,
    /// Spatial bounds.
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    /// Size upper limits (0.0 = no limit).
    max_w: f32,
    max_h: f32,
}

impl QuadFilter {
    fn matches(&self, quad: &CapturedDrawQuad) -> bool {
        if let Some(prefix) = self.texture_prefix {
            match &quad.texture_key {
                Some(key) if key.starts_with(prefix) => {}
                _ => return false,
            }
        }
        quad.x >= self.min_x
            && quad.x < self.max_x
            && quad.y >= self.min_y
            && quad.y < self.max_y
            && (self.max_w == 0.0 || quad.w <= self.max_w)
            && (self.max_h == 0.0 || quad.h <= self.max_h)
    }

    fn apply<'a>(&self, quads: &'a [CapturedDrawQuad]) -> Vec<&'a CapturedDrawQuad> {
        quads.iter().filter(|q| self.matches(q)).collect()
    }

    /// Assert that at least `min_count` quads match, with diagnostic output on failure.
    fn assert_present(&self, quads: &[CapturedDrawQuad], min_count: usize, context: &str) {
        let matched: Vec<_> = self.apply(quads);
        if matched.len() >= min_count {
            return;
        }

        // Diagnostic: expanded region (1.5x from center)
        let cx = (self.min_x + self.max_x) / 2.0;
        let cy = (self.min_y + self.max_y) / 2.0;
        let hw = (self.max_x - self.min_x) * 0.75;
        let hh = (self.max_y - self.min_y) * 0.75;
        let expanded = QuadFilter {
            texture_prefix: self.texture_prefix,
            min_x: cx - hw,
            max_x: cx + hw,
            min_y: cy - hh,
            max_y: cy + hh,
            max_w: if self.max_w == 0.0 {
                0.0
            } else {
                self.max_w * 1.5
            },
            max_h: if self.max_h == 0.0 {
                0.0
            } else {
                self.max_h * 1.5
            },
        };
        let nearby: Vec<_> = expanded
            .apply(quads)
            .iter()
            .take(20)
            .map(|q| {
                format!(
                    "  ({:.0},{:.0} {:.0}x{:.0} {:?})",
                    q.x, q.y, q.w, q.h, q.texture_key
                )
            })
            .collect();

        // Diagnostic: all small textured quads
        let small_textured: Vec<_> = quads
            .iter()
            .filter(|q| {
                q.texture_key.is_some()
                    && (self.max_w == 0.0 || q.w <= self.max_w * 1.5)
                    && (self.max_h == 0.0 || q.h <= self.max_h * 1.5)
            })
            .take(30)
            .map(|q| {
                format!(
                    "  ({:.0},{:.0} {:.0}x{:.0} {:?})",
                    q.x, q.y, q.w, q.h, q.texture_key
                )
            })
            .collect();

        let matched_sample: Vec<_> = matched
            .iter()
            .take(10)
            .map(|q| {
                format!(
                    "  ({:.0},{:.0} {:.0}x{:.0} {:?})",
                    q.x, q.y, q.w, q.h, q.texture_key
                )
            })
            .collect();

        panic!(
            "{context}: expected >= {min_count} quads, got {count}\n\
             Filter: x[{min_x:.0}..{max_x:.0}] y[{min_y:.0}..{max_y:.0}] \
             max_size({max_w:.0}x{max_h:.0}) prefix={prefix:?}\n\
             Matched ({count}):\n{matched_str}\n\
             Nearby (expanded region, {nearby_count}):\n{nearby_str}\n\
             All small textured ({small_count}):\n{small_str}",
            count = matched.len(),
            min_x = self.min_x,
            max_x = self.max_x,
            min_y = self.min_y,
            max_y = self.max_y,
            max_w = self.max_w,
            max_h = self.max_h,
            prefix = self.texture_prefix,
            matched_str = matched_sample.join("\n"),
            nearby_count = nearby.len(),
            nearby_str = nearby.join("\n"),
            small_count = small_textured.len(),
            small_str = small_textured.join("\n"),
        );
    }
}

// ---------------------------------------------------------------------------
// A. Full gameplay lifecycle with BMS file: MusicSelect -> Decide -> Play -> Result
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_select_decide_play_result_with_bms() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    let mut mc = make_controller_with_factory();

    // create() initializes PlayerResource and enters MusicSelect (no bmsfile arg)
    mc.create();
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect),);

    // Load BMS file onto PlayerResource (simulates song selection)
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist after create()");
        // mode_type 0 = Play mode
        let loaded = resource.set_bms_file(&bms_path, 0, 0);
        assert!(loaded, "BMS file should load successfully");
        assert!(
            resource.bms_model().is_some(),
            "BMS model should be available after loading"
        );
    }

    // Render a frame in MusicSelect with BMS data loaded
    mc.render();
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect),);

    // Transition to Decide
    mc.change_state(MainStateType::Decide);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Decide));
    mc.render();

    // Transition to Play (factory reads BMS model from PlayerResource)
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));
    mc.render();

    // Transition to Result
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Result));
    mc.render();

    // Return to MusicSelect (normal game loop)
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect),);

    // Clean dispose
    mc.dispose();
    assert!(mc.current_state().is_none());
    assert!(mc.current_state_type().is_none());
}

#[test]
fn e2e_ecfn_select_enter_reaches_manual_play_without_stuck_beams() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    let shared_state = SharedKeyState::new();
    set_shared_key_state(shared_state.clone());

    let player = ecfn_player_config();
    let mut config = Config::default();
    config.select.skip_decide_screen = true;
    let mut selector = MusicSelector::new();
    selector.config = player.clone();
    selector.manager.currentsongs = vec![make_song_bar(&bms_path)];
    selector.manager.selectedindex = 0;

    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(LauncherStateFactory::new().into_creator());
    mc.set_shared_music_selector(Arc::new(Mutex::new(selector)));
    mc.create();

    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    shared_state.set_key_pressed(Keys::ENTER, true);
    mc.render();
    mc.render();
    shared_state.set_key_pressed(Keys::ENTER, false);

    for _ in 0..120 {
        if mc.current_state_type() == Some(MainStateType::Play) {
            break;
        }
        mc.render();
    }

    assert_eq!(
        mc.current_state_type(),
        Some(MainStateType::Play),
        "Enter from MusicSelect should eventually transition into Play"
    );
    assert_eq!(
        mc.player_resource().and_then(|r| r.play_mode()).copied(),
        Some(BMSPlayerMode::PLAY),
        "Enter start must keep manual PLAY mode"
    );

    let has_stuck_beam = mc.current_state().is_some_and(|state| {
        (100..=119).any(|id| state.main_state_data().timer.is_timer_on(TimerId::new(id)))
    });
    assert!(
        !has_stuck_beam,
        "manual play should not enter with key beam timers already on"
    );

    mc.sprite_batch_mut()
        .expect("sprite batch should exist after create")
        .enable_capture();
    for _ in 0..120 {
        mc.render();
    }
    let quads = mc
        .sprite_batch()
        .expect("sprite batch should exist after rendering")
        .captured_quads()
        .to_vec();
    let unique_textures = quads
        .iter()
        .filter_map(|quad| quad.texture_key.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        quads.len() > 100,
        "select->Enter manual play should render more than lane-only quads, got {}",
        quads.len()
    );
    assert!(
        unique_textures.len() >= 3,
        "select->Enter manual play should use multiple texture groups, got {:?}",
        unique_textures
    );
}

#[test]
fn e2e_music_select_standalone_default_json_skin_draws_runtime_numeric_value_quads() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    let song = make_select_stats_song(&bms_path);
    let score = make_select_stats_score(&song.file.sha256);
    let info = SongInformation {
        sha256: song.file.sha256.clone(),
        mainbpm: 150.0,
        ..Default::default()
    };
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let player_root = tempdir.path().join("player");
    let player_dir = player_root.join("player1");
    std::fs::create_dir_all(&player_dir).expect("player directory should be created");

    let song_db_path = tempdir.path().join("songdata.db");
    let song_db =
        rubato_game::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
            &song_db_path.to_string_lossy(),
            &[],
        )
        .expect("song db should open");
    rubato_types::song_database_accessor::SongDatabaseAccessor::set_song_datas(
        &song_db,
        &[song.clone()],
    )
    .expect("song db should store the test song");

    let score_db_path = player_dir.join("score.db");
    let score_db = ScoreDatabaseAccessor::new(
        score_db_path
            .to_str()
            .expect("score db path should be valid UTF-8"),
    )
    .expect("score db should open");
    score_db
        .create_table()
        .expect("score db schema should exist");
    score_db.set_score_data(&score);

    let info_db_path = tempdir.path().join("songinfo.db");
    let info_db = SongInformationAccessor::new(
        info_db_path
            .to_str()
            .expect("song info db path should be valid UTF-8"),
    )
    .expect("song info db should open");
    write_song_info_row(&info_db_path, &info);

    let player = default_json_select_player_config();
    let mut config = Config::default();
    config.playername = Some("player1".to_string());
    config.paths.playerpath = player_root.to_string_lossy().to_string();
    config.paths.songpath = song_db_path.to_string_lossy().to_string();
    config.paths.songinfopath = info_db_path.to_string_lossy().to_string();

    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_info_database(Box::new(info_db));
    mc.set_state_factory(LauncherStateFactory::new().into_creator());
    mc.create();

    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

    let quads = capture_render_quads(&mut mc, 120);

    let bpm_filter = QuadFilter {
        texture_prefix: None,
        min_x: 370.0,
        max_x: 470.0,
        min_y: 512.0,
        max_y: 530.5,
        max_w: 18.1,
        max_h: 18.1,
    };
    bpm_filter.assert_present(
        &quads,
        1,
        "standalone MusicSelect should draw BPM digits from runtime chart data",
    );

    let score_filter = QuadFilter {
        texture_prefix: None,
        min_x: 200.0,
        max_x: 290.0,
        min_y: 372.0,
        max_y: 390.5,
        max_w: 18.1,
        max_h: 18.1,
    };
    score_filter.assert_present(
        &quads,
        1,
        "standalone MusicSelect should draw score digits from the runtime score DB",
    );
}

#[test]
fn e2e_music_select_ecfn_skin_draws_japanese_title_bitmap_glyphs() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    let (mut mc, _shared) = build_ecfn_select_controller(vec![make_ecfn_title_song_bar(&bms_path)]);
    let quads = capture_render_quads(&mut mc, 120);

    let filter = QuadFilter {
        texture_prefix: Some(""),
        min_x: 280.0,
        max_x: 780.0,
        min_y: 450.0,
        max_y: 560.0,
        max_w: 70.0,
        max_h: 70.0,
    };
    filter.assert_present(
        &quads,
        1,
        "ECFN select skin should draw title-region bitmap font quads for Japanese song titles",
    );
}

#[test]
fn e2e_music_select_ecfn_skin_draws_songlist_song_title_bitmap_glyphs() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    let (mut mc, shared_selector) = build_ecfn_select_controller(vec![]);

    {
        let mut selector = shared_selector.lock().expect("selector lock poisoned");
        selector.manager.currentsongs = vec![make_ecfn_songlist_song_bar(&bms_path)];
        selector.manager.selectedindex = 0;
        if let Some(bar) = selector.bar_rendering.bar.as_mut() {
            bar.update_bar_text();
        }
    }

    let quads = capture_render_quads(&mut mc, 120);

    let filter = QuadFilter {
        texture_prefix: Some("__pixmap_"),
        min_x: 930.0,
        max_x: 1180.0,
        min_y: 300.0,
        max_y: 460.0,
        max_w: 60.0,
        max_h: 60.0,
    };
    filter.assert_present(
        &quads,
        5,
        "ECFN select skin should draw songlist title bitmap glyphs",
    );

    // Songlist draws glyph quads across multiple bar rows spanning a wide vertical range.
    // QuadFilter already validates that quads are within the expected spatial region;
    // asserting a single vertical center is inappropriate for multi-row rendering.
}

#[test]
fn e2e_music_select_ecfn_skin_draws_songlist_folder_title_bitmap_glyphs() {
    let (mut mc, shared_selector) = build_ecfn_select_controller(vec![]);

    {
        let mut selector = shared_selector.lock().expect("selector lock poisoned");
        selector.manager.currentsongs = vec![make_ecfn_folder_bar()];
        selector.manager.selectedindex = 0;
        if let Some(bar) = selector.bar_rendering.bar.as_mut() {
            bar.update_bar_text();
        }
    }

    let quads = capture_render_quads(&mut mc, 120);

    let filter = QuadFilter {
        texture_prefix: Some("__pixmap_"),
        min_x: 860.0,
        max_x: 1180.0,
        min_y: 300.0,
        max_y: 460.0,
        max_w: 60.0,
        max_h: 60.0,
    };
    filter.assert_present(
        &quads,
        5,
        "ECFN select skin should draw folder-bar bitmap glyphs",
    );
}

// ---------------------------------------------------------------------------
// B. Direct BMS launch: create(bmsfile) -> Play -> Result
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_direct_bms_launch_play_to_result() {
    let bms_path = test_bms_path();
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    // Use MainLoader::play() with bmsfile (production path for direct launch)
    let mut mc = MainLoader::play(
        Some(bms_path),
        None,
        false,
        Some(Config::default()),
        Some(PlayerConfig::default()),
        false,
    )
    .expect("MainLoader::play() should succeed");
    mc.set_state_factory(LauncherStateFactory::new().into_creator());

    // create() loads the BMS file and enters Play directly
    mc.create();
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

    // Verify BMS model was loaded into PlayerResource
    let has_model = mc.player_resource().and_then(|r| r.bms_model()).is_some();
    assert!(has_model, "BMS model should be loaded in PlayerResource");

    // Render multiple frames in Play state
    for _ in 0..3 {
        mc.render();
    }
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

    // Transition to Result (end of song)
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Result));
    mc.render();

    // Clean dispose
    mc.dispose();
    assert!(mc.current_state().is_none());
}

// ---------------------------------------------------------------------------
// C. BMS load -> Play with lifecycle methods (pause/resume/resize)
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_play_lifecycle_with_bms() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // Enter Play state
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

    // Exercise full lifecycle methods
    mc.render();
    mc.pause();
    mc.resume();
    mc.resize(1920, 1080);
    mc.render();
    mc.resize(1280, 720);
    mc.render();

    // State should remain Play throughout lifecycle
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));
}

// ---------------------------------------------------------------------------
// D. BMS data propagation: model reaches Play state via factory
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_bms_model_propagates_to_play_state() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS and capture expected note count
    let expected_notes;
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
        let model = resource
            .bms_model()
            .expect("model should be present after load");
        expected_notes = model.total_notes();
        assert!(
            expected_notes > 0,
            "test BMS file should have at least 1 note"
        );
    }

    // Enter Play state - factory reads model from PlayerResource
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

    // Verify the BMS model is still accessible via PlayerResource
    let actual_notes = mc
        .player_resource()
        .and_then(|r| r.bms_model())
        .map(|m| m.total_notes())
        .unwrap_or(0);
    assert_eq!(
        actual_notes, expected_notes,
        "BMS model note count should be preserved through Play state creation"
    );
}

// ---------------------------------------------------------------------------
// E. Course result path: Play -> CourseResult with BMS
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_play_to_course_result_with_bms() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // Play -> CourseResult (course mode end-of-song path)
    mc.change_state(MainStateType::Play);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Play));
    mc.render();

    mc.change_state(MainStateType::CourseResult);
    assert_eq!(mc.current_state_type(), Some(MainStateType::CourseResult),);
    mc.render();

    // Back to MusicSelect
    mc.change_state(MainStateType::MusicSelect);
    assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect),);

    mc.dispose();
    assert!(mc.current_state().is_none());
}

// ---------------------------------------------------------------------------
// F. Multiple play sessions: load BMS, play, result, re-enter play
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_multiple_play_sessions() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    for session in 0..3 {
        // Load BMS (simulates re-selecting the same song)
        {
            let resource = mc
                .player_resource_mut()
                .expect("PlayerResource should exist");
            assert!(
                resource.set_bms_file(&bms_path, 0, 0),
                "session {} BMS load failed",
                session
            );
        }

        // Play
        mc.change_state(MainStateType::Play);
        assert_eq!(
            mc.current_state_type(),
            Some(MainStateType::Play),
            "session {} Play state failed",
            session
        );
        mc.render();

        // Result
        mc.change_state(MainStateType::Result);
        assert_eq!(
            mc.current_state_type(),
            Some(MainStateType::Result),
            "session {} Result state failed",
            session
        );
        mc.render();

        // Back to MusicSelect
        mc.change_state(MainStateType::MusicSelect);
        assert_eq!(
            mc.current_state_type(),
            Some(MainStateType::MusicSelect),
            "session {} MusicSelect state failed",
            session
        );
    }

    mc.dispose();
    assert!(mc.current_state().is_none());
}

// ---------------------------------------------------------------------------
// G. Skip decide screen with BMS: MusicSelect -> Decide(skipped) -> Play -> Result
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_skip_decide_with_bms() {
    let bms_path = test_bms_path();

    let mut config = Config::default();
    config.select.skip_decide_screen = true;
    let player = PlayerConfig::default();
    let mut mc = MainController::new(None, config, player, None, false);
    mc.set_state_factory(LauncherStateFactory::new().into_creator());
    mc.create();

    // Load BMS
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // When skip_decide_screen is true, requesting Decide creates Play instead
    mc.change_state(MainStateType::Decide);
    assert_eq!(
        mc.current_state_type(),
        Some(MainStateType::Play),
        "Decide should skip to Play when skip_decide_screen is true"
    );
    mc.render();

    // Continue to Result
    mc.change_state(MainStateType::Result);
    assert_eq!(mc.current_state_type(), Some(MainStateType::Result));
    mc.render();

    mc.dispose();
    assert!(mc.current_state().is_none());
}

// ---------------------------------------------------------------------------
// H. Render multiple frames per state with BMS data
// ---------------------------------------------------------------------------

#[test]
fn e2e_gameplay_sustained_rendering_with_bms() {
    let bms_path = test_bms_path();
    let mut mc = make_controller_with_factory();
    mc.create();

    // Load BMS
    {
        let resource = mc
            .player_resource_mut()
            .expect("PlayerResource should exist");
        assert!(resource.set_bms_file(&bms_path, 0, 0));
    }

    // Render 10 frames in each state to test sustained operation
    let states = [
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
    ];

    for state_type in &states {
        mc.change_state(*state_type);
        for frame in 0..10 {
            mc.render();
            assert_eq!(
                mc.current_state_type(),
                Some(*state_type),
                "State should remain {:?} at frame {}",
                state_type,
                frame,
            );
        }
    }

    mc.dispose();
}

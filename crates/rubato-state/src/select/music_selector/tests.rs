use super::*;
use crate::select::bar::bar::Bar;
use crate::select::bar::grade_bar::GradeBar;
use crate::select::bar::selectable_bar::SelectableBarData;
use crate::select::bar::song_bar::SongBar;
use ::bms_model::bms_model::BMSModel;
use ::bms_model::note::Note;
use rubato_audio::audio_driver::AudioDriver;
use rubato_core::main_state::MainState;
use rubato_types::skin_render_context::SkinRenderContext;

struct MockAudioDriver {
    play_count: usize,
    stop_count: usize,
}

impl MockAudioDriver {
    fn new() -> Self {
        Self {
            play_count: 0,
            stop_count: 0,
        }
    }
}

impl AudioDriver for MockAudioDriver {
    fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {
        self.play_count += 1;
    }

    fn set_volume_path(&mut self, _path: &str, _volume: f32) {}

    fn is_playing_path(&self, _path: &str) -> bool {
        false
    }

    fn stop_path(&mut self, _path: &str) {
        self.stop_count += 1;
    }

    fn dispose_path(&mut self, _path: &str) {}

    fn set_model(&mut self, _model: &BMSModel) {}

    fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}

    fn abort(&mut self) {}

    fn get_progress(&self) -> f32 {
        1.0
    }

    fn play_note(&mut self, _n: &Note, _volume: f32, _pitch: i32) {}

    fn play_judge(&mut self, _judge: i32, _fast: bool) {}

    fn stop_note(&mut self, _n: Option<&Note>) {}

    fn set_volume_note(&mut self, _n: &Note, _volume: f32) {}

    fn set_global_pitch(&mut self, _pitch: f32) {}

    fn get_global_pitch(&self) -> f32 {
        1.0
    }

    fn dispose_old(&mut self) {}

    fn dispose(&mut self) {}
}

fn make_song_data(sha256: &str, path: Option<&str>) -> SongData {
    let mut sd = SongData::default();
    sd.file.sha256 = sha256.to_string();
    if let Some(p) = path {
        sd.set_path(p.to_string());
    }
    sd
}

fn make_song_bar(sha256: &str, path: Option<&str>) -> Bar {
    Bar::Song(Box::new(SongBar::new(make_song_data(sha256, path))))
}

fn set_selected_bar(selector: &mut MusicSelector, bar: Bar) {
    selector.manager.currentsongs = vec![bar];
    selector.manager.selectedindex = 0;
}

#[test]
fn test_state_type() {
    let selector = MusicSelector::new();
    assert_eq!(selector.state_type(), Some(MainStateType::MusicSelect));
}

#[test]
fn test_main_state_data_access() {
    let mut selector = MusicSelector::new();
    // Verify we can access timer through main_state_data
    let timer = &selector.main_state_data().timer;
    assert!(!timer.is_timer_on(skin_property::TIMER_STARTINPUT));

    // Verify mutable access
    selector
        .main_state_data_mut()
        .timer
        .set_timer_on(skin_property::TIMER_STARTINPUT);
    assert!(
        selector
            .main_state_data()
            .timer
            .is_timer_on(skin_property::TIMER_STARTINPUT)
    );
}

#[test]
fn test_sync_audio_ticks_preview_processor() {
    let config = Config::default();
    let mut selector = MusicSelector::with_config(config.clone());
    let mut preview = PreviewMusicProcessor::new(&config);
    preview.set_default("/bgm/default.ogg");
    preview.start(None);
    selector.preview_state.preview = Some(preview);

    let mut audio = MockAudioDriver::new();
    selector.sync_audio(&mut audio);

    assert_eq!(audio.play_count, 1);
}

#[test]
fn test_select_skin_context_uses_sort_for_image_index_12() {
    let mut selector = MusicSelector::new();
    selector.config.select_settings.sort = 5;
    selector.config.judge_settings.judgetiming = 17;
    let mut timer = TimerManager::new();
    let ctx = SelectSkinContext {
        timer: &mut timer,
        selector: &mut selector,
    };

    assert_eq!(ctx.image_index_value(12), 5);
}

#[test]
fn test_select_skin_context_uses_random_for_image_index_42() {
    let mut selector = MusicSelector::new();
    selector.config.play_settings.random = 4;
    let mut timer = TimerManager::new();
    let ctx = SelectSkinContext {
        timer: &mut timer,
        selector: &mut selector,
    };

    assert_eq!(ctx.image_index_value(42), 4);
}

#[test]
fn test_select_skin_context_uses_mode_image_index_mapping() {
    let mut selector = MusicSelector::new();
    selector.config.mode = Some(bms_model::Mode::BEAT_5K);
    let mut timer = TimerManager::new();
    let ctx = SelectSkinContext {
        timer: &mut timer,
        selector: &mut selector,
    };

    assert_eq!(ctx.image_index_value(11), 1);
}

#[test]
fn test_select_skin_context_uses_selected_song_play_config_for_image_index_330() {
    let mut selector = MusicSelector::new();
    selector.config.mode = Some(bms_model::Mode::BEAT_7K);
    selector.config.mode7.playconfig.enablelanecover = false;
    selector.config.mode5.playconfig.enablelanecover = true;
    let mut song = make_song_data("play-config", Some("/test/selected.bms"));
    song.chart.mode = bms_model::Mode::BEAT_5K.id();
    set_selected_bar(&mut selector, Bar::Song(Box::new(SongBar::new(song))));
    let mut timer = TimerManager::new();
    let ctx = SelectSkinContext {
        timer: &mut timer,
        selector: &mut selector,
    };

    assert_eq!(ctx.image_index_value(330), 1);
}

#[test]
fn test_select_skin_context_uses_selected_song_judge_algorithm_for_image_index_340() {
    let mut selector = MusicSelector::new();
    selector.config.mode = Some(bms_model::Mode::BEAT_7K);
    selector.config.mode7.playconfig.judgetype = "Combo".to_string();
    selector.config.mode5.playconfig.judgetype = "Lowest".to_string();
    let mut song = make_song_data("judge-type", Some("/test/judge.bms"));
    song.chart.mode = bms_model::Mode::BEAT_5K.id();
    set_selected_bar(&mut selector, Bar::Song(Box::new(SongBar::new(song))));
    let mut timer = TimerManager::new();
    let ctx = SelectSkinContext {
        timer: &mut timer,
        selector: &mut selector,
    };

    assert_eq!(ctx.image_index_value(340), 2);
}

#[test]
fn test_select_skin_context_uses_selected_score_clear_for_image_index_370() {
    let mut selector = MusicSelector::new();
    let mut bar = make_song_bar("clear", Some("/test/clear.bms"));
    let mut score = ScoreData::default();
    score.clear = 6;
    bar.set_score(Some(score));
    set_selected_bar(&mut selector, bar);
    let mut timer = TimerManager::new();
    let ctx = SelectSkinContext {
        timer: &mut timer,
        selector: &mut selector,
    };

    assert_eq!(ctx.image_index_value(370), 6);
}

#[test]
fn test_select_skin_context_uses_rival_clear_for_image_index_371() {
    let mut selector = MusicSelector::new();
    let mut bar = make_song_bar("rival-clear", Some("/test/rival-clear.bms"));
    let mut rival = ScoreData::default();
    rival.clear = 8;
    bar.set_rival_score(Some(rival));
    set_selected_bar(&mut selector, bar);
    let mut timer = TimerManager::new();
    let ctx = SelectSkinContext {
        timer: &mut timer,
        selector: &mut selector,
    };

    assert_eq!(ctx.image_index_value(371), 8);
}

#[test]
fn test_select_skin_context_uses_target_visual_index_for_image_index_77() {
    let mut selector = MusicSelector::new();
    selector.config.select_settings.targetid = "MAX".to_string();
    let mut timer = TimerManager::new();
    let ctx = SelectSkinContext {
        timer: &mut timer,
        selector: &mut selector,
    };

    assert_eq!(ctx.image_index_value(77), 10);
}

#[test]
fn test_create_lifecycle() {
    let mut selector = MusicSelector::new();
    // Set playedsong and playedcourse to verify they get cleared
    selector.playedsong = Some(make_song_data("abc", Some("/test/song.bms")));
    selector.play = Some(BMSPlayerMode::PLAY);

    selector.create();

    assert!(selector.play.is_none());
    assert!(!selector.preview_state.show_note_graph);
    assert!(selector.playedsong.is_none());
}

#[test]
fn test_create_clears_played_course() {
    let mut selector = MusicSelector::new();
    let course = CourseData {
        name: Some("Test Course".to_string()),
        hash: vec![make_song_data("s1", Some("/path1.bms"))],
        constraint: vec![],
        trophy: vec![],
        release: false,
    };
    selector.playedcourse = Some(course);

    selector.create();

    assert!(selector.playedcourse.is_none());
}

#[test]
fn test_prepare_starts_preview() {
    let mut selector = MusicSelector::new();
    // prepare without preview processor should not panic
    selector.prepare();
}

#[test]
fn test_shutdown_stops_preview() {
    let mut selector = MusicSelector::new();
    // shutdown without preview/search should not panic
    selector.shutdown();
}

#[test]
fn test_dispose_clears_skin_and_stage() {
    let mut selector = MusicSelector::new();
    selector.dispose();
    assert!(selector.main_state_data.skin.is_none());
    assert!(selector.main_state_data.stage.is_none());
    assert!(selector.search.is_none());
}

#[test]
fn test_set_panel_state_timers() {
    let mut selector = MusicSelector::new();

    // Set panel state to 1
    selector.set_panel_state(1);
    assert_eq!(selector.panelstate, 1);
    assert!(
        selector
            .main_state_data
            .timer
            .is_timer_on(skin_property::TIMER_PANEL1_ON)
    );

    // Set panel state to 2
    selector.set_panel_state(2);
    assert_eq!(selector.panelstate, 2);
    // Panel 1 should be off, panel 2 should be on
    assert!(
        selector
            .main_state_data
            .timer
            .is_timer_on(skin_property::TIMER_PANEL1_OFF)
    );
    assert!(
        selector
            .main_state_data
            .timer
            .is_timer_on(rubato_types::timer_id::TimerId::new(
                skin_property::TIMER_PANEL1_ON.as_i32() + 1
            ))
    );

    // Set panel state to 0
    selector.set_panel_state(0);
    assert_eq!(selector.panelstate, 0);
    assert!(
        selector
            .main_state_data
            .timer
            .is_timer_on(rubato_types::timer_id::TimerId::new(
                skin_property::TIMER_PANEL1_OFF.as_i32() + 1
            ))
    );
}

#[test]
fn test_set_panel_state_same_value_no_change() {
    let mut selector = MusicSelector::new();
    selector.set_panel_state(1);

    // Setting same value should not toggle timers
    selector.set_panel_state(1);
    assert_eq!(selector.panelstate, 1);
}

#[test]
fn test_exists_constraint_no_selection() {
    let selector = MusicSelector::new();
    assert!(!selector.exists_constraint(&CourseDataConstraint::Class));
}

#[test]
fn test_exists_constraint_with_grade_bar() {
    let mut selector = MusicSelector::new();
    let course = CourseData {
        name: Some("Test".to_string()),
        hash: vec![],
        constraint: vec![CourseDataConstraint::Class, CourseDataConstraint::Mirror],
        trophy: vec![],
        release: false,
    };
    selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
    selector.manager.selectedindex = 0;

    assert!(selector.exists_constraint(&CourseDataConstraint::Class));
    assert!(selector.exists_constraint(&CourseDataConstraint::Mirror));
    assert!(!selector.exists_constraint(&CourseDataConstraint::Random));
}

#[test]
fn test_exists_constraint_with_song_bar() {
    let mut selector = MusicSelector::new();
    selector.manager.currentsongs = vec![make_song_bar("abc", Some("/test.bms"))];
    selector.manager.selectedindex = 0;

    // SongBar has no constraints
    assert!(!selector.exists_constraint(&CourseDataConstraint::Class));
}

#[test]
fn test_select_directory_bar() {
    let mut selector = MusicSelector::new();
    let bar = Bar::Folder(Box::new(crate::select::bar::folder_bar::FolderBar::new(
        None,
        "test_crc".to_string(),
    )));
    // select on a directory bar should try to open it (and set play = None still)
    selector.select(&bar);
    // Play should not be set for directory bars
    assert!(selector.play.is_none());
}

#[test]
fn test_select_song_bar() {
    let mut selector = MusicSelector::new();
    let bar = make_song_bar("abc123", Some("/test/song.bms"));
    selector.select(&bar);
    // SongBar (non-directory) should set play mode
    assert_eq!(selector.play, Some(BMSPlayerMode::PLAY));
}

#[test]
fn test_select_song() {
    let mut selector = MusicSelector::new();
    assert!(selector.play.is_none());
    selector.select_song(BMSPlayerMode::PRACTICE);
    assert_eq!(selector.play, Some(BMSPlayerMode::PRACTICE));
}

#[test]
fn test_ranking_position_no_ir() {
    let mut selector = MusicSelector::new();

    // No IR data, default values
    assert_eq!(selector.ranking_position(), 0.0);

    // Set position with no IR data — ranking_max = 1, so 1 * 0.5 = 0
    selector.set_ranking_position(0.5);
    assert_eq!(selector.ranking.ranking_offset, 0);
}

#[test]
fn test_ranking_position_with_ir() {
    let mut selector = MusicSelector::new();

    // Use update_score to set total player count
    let mut ir = RankingData::new();
    use rubato_core::score_data::ScoreData as CoreScoreData;
    use rubato_ir::ir_score_data::IRScoreData;
    let scores: Vec<IRScoreData> = (0..10)
        .map(|i| {
            let mut sd = CoreScoreData::default();
            sd.judge_counts.epg = (i + 1) * 10; // different exscores so sorting works
            IRScoreData::new(&sd)
        })
        .collect();
    ir.update_score(&scores, None);
    selector.ranking.currentir = Some(ir);

    selector.set_ranking_position(0.5);
    assert_eq!(selector.ranking.ranking_offset, 5); // 10 * 0.5

    let pos = selector.ranking_position();
    assert!((pos - 0.5).abs() < 0.01);
}

#[test]
fn test_ranking_position_bounds() {
    let mut selector = MusicSelector::new();
    // Out of range values should not change offset
    selector.ranking.ranking_offset = 3;
    selector.set_ranking_position(-0.1);
    assert_eq!(selector.ranking.ranking_offset, 3);

    selector.set_ranking_position(1.0);
    assert_eq!(selector.ranking.ranking_offset, 3);
}

#[test]
fn test_selected_bar_moved_resets_state() {
    let mut selector = MusicSelector::new();
    selector.preview_state.show_note_graph = true;
    selector.manager.currentsongs = vec![make_song_bar("abc", Some("/test.bms"))];
    selector.manager.selectedindex = 0;

    selector.selected_bar_moved();

    assert!(!selector.preview_state.show_note_graph);
    // selectedreplay should be -1 since no replay exists
    assert_eq!(selector.selectedreplay, -1);
}

#[test]
fn test_selected_bar_moved_no_ir() {
    let mut selector = MusicSelector::new();
    // With no bars
    selector.selected_bar_moved();

    assert!(selector.ranking.currentir.is_none());
    assert_eq!(selector.ranking.current_ranking_duration, -1);
}

#[test]
fn test_render_timers() {
    let mut selector = MusicSelector::new();
    // Without a skin loaded, TIMER_STARTINPUT should NOT be set
    // (matches Java: timer.switchTimer(TIMER_STARTINPUT) is guarded by getSkin().getInput())
    selector.render();
    assert!(
        !selector
            .main_state_data
            .timer
            .is_timer_on(skin_property::TIMER_STARTINPUT)
    );
}

#[test]
fn test_render_ir_timers_no_ir() {
    let mut selector = MusicSelector::new();
    selector.render();
    // With no IR data, all IR timers should be off
    assert!(
        !selector
            .main_state_data
            .timer
            .is_timer_on(skin_property::TIMER_IR_CONNECT_BEGIN)
    );
    assert!(
        !selector
            .main_state_data
            .timer
            .is_timer_on(skin_property::TIMER_IR_CONNECT_SUCCESS)
    );
    assert!(
        !selector
            .main_state_data
            .timer
            .is_timer_on(skin_property::TIMER_IR_CONNECT_FAIL)
    );
}

#[test]
fn test_command_reset_replay_no_selection() {
    let mut selector = MusicSelector::new();
    selector.selectedreplay = 2;
    selector.execute(MusicSelectCommand::ResetReplay);
    assert_eq!(selector.selectedreplay, -1);
}

#[test]
fn test_command_reset_replay_with_selection() {
    let mut selector = MusicSelector::new();
    let mut song_bar = SongBar::new(make_song_data("abc", Some("/test.bms")));
    song_bar.selectable.exists_replay[2] = true;
    selector.manager.currentsongs = vec![Bar::Song(Box::new(song_bar))];
    selector.manager.selectedindex = 0;

    selector.execute(MusicSelectCommand::ResetReplay);
    assert_eq!(selector.selectedreplay, 2);
}

#[test]
fn test_chart_replication_mode() {
    assert_eq!(
        ChartReplicationMode::get("NONE"),
        ChartReplicationMode::None
    );
    assert_eq!(
        ChartReplicationMode::get("RIVALCHART"),
        ChartReplicationMode::RivalChart
    );
    assert_eq!(
        ChartReplicationMode::get("UNKNOWN"),
        ChartReplicationMode::None
    );

    assert_eq!(ChartReplicationMode::None.name(), "NONE");
    assert_eq!(ChartReplicationMode::ReplayChart.name(), "REPLAYCHART");
}

#[test]
fn test_mode_constants() {
    assert!(MODE[0].is_none());
    assert_eq!(MODE[1], Some(bms_model::Mode::BEAT_7K));
    assert_eq!(MODE[2], Some(bms_model::Mode::BEAT_14K));
    assert_eq!(MODE[3], Some(bms_model::Mode::POPN_9K));
    assert_eq!(MODE[4], Some(bms_model::Mode::BEAT_5K));
    assert_eq!(MODE[5], Some(bms_model::Mode::BEAT_10K));
    assert_eq!(MODE[6], Some(bms_model::Mode::KEYBOARD_24K));
    assert_eq!(MODE[7], Some(bms_model::Mode::KEYBOARD_24K_DOUBLE));
}

#[test]
fn test_dispatch_select_song_event() {
    let mut selector = MusicSelector::new();
    assert!(selector.play.is_none());

    selector.dispatch_input_events(vec![InputEvent::SelectSong(BMSPlayerMode::AUTOPLAY)]);

    assert_eq!(selector.play, Some(BMSPlayerMode::AUTOPLAY));
}

#[test]
fn test_dispatch_execute_command_event() {
    let mut selector = MusicSelector::new();
    selector.selectedreplay = 5;

    selector.dispatch_input_events(vec![InputEvent::Execute(MusicSelectCommand::ResetReplay)]);

    // ResetReplay with no selected bar sets to -1
    assert_eq!(selector.selectedreplay, -1);
}

#[test]
fn test_dispatch_bar_manager_close_event() {
    let mut selector = MusicSelector::new();
    // At root level, close should not panic
    selector.dispatch_input_events(vec![InputEvent::BarManagerClose]);
}

#[test]
fn test_dispatch_multiple_events() {
    let mut selector = MusicSelector::new();
    selector.selectedreplay = 5;

    selector.dispatch_input_events(vec![
        InputEvent::Execute(MusicSelectCommand::ResetReplay),
        InputEvent::SelectSong(BMSPlayerMode::PRACTICE),
    ]);

    assert_eq!(selector.selectedreplay, -1);
    assert_eq!(selector.play, Some(BMSPlayerMode::PRACTICE));
}

#[test]
fn test_process_input_with_context_no_musicinput() {
    let mut selector = MusicSelector::new();
    // musicinput is None by default; should return early without panic
    let config = rubato_core::config::Config::default();
    let player_config = PlayerConfig::default();
    let mut input = BMSPlayerInputProcessor::new(&config, &player_config);
    selector.process_input_with_context(&mut input);
}

#[test]
fn test_process_input_with_context_basic() {
    use crate::select::music_select_input_processor::MusicSelectInputProcessor;

    let mut selector = MusicSelector::new();
    // Install musicinput processor
    selector.musicinput = Some(MusicSelectInputProcessor::new(300, 50, 10));

    let config = rubato_core::config::Config::default();
    let player_config = PlayerConfig::default();
    let mut input = BMSPlayerInputProcessor::new(&config, &player_config);

    // process_input should not panic and should set panel state to 0
    // (since no start/select keys are pressed, falls into the else branch)
    selector.process_input_with_context(&mut input);
    assert_eq!(selector.panelstate, 0);
}

#[test]
fn test_dispatch_open_directory_event() {
    let mut selector = MusicSelector::new();
    // OpenDirectory at root with no selected bar — should not panic
    selector.dispatch_input_events(vec![InputEvent::OpenDirectory]);
}

// ============================================================
// Mock MainController for read_chart/read_course tests
// ============================================================

use rubato_types::main_controller_access::MainControllerAccess;
use rubato_types::player_resource_access::PlayerResourceAccess;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Tracks state changes and resource operations for assertions.
#[derive(Default)]
struct MockState {
    state_changes: Vec<MainStateType>,
    played_audio_paths: Vec<String>,
    cleared: bool,
    bms_file_path: Option<PathBuf>,
    bms_file_mode_type: Option<i32>,
    bms_file_mode_id: Option<i32>,
    bms_file_result: bool,
    course_files: Option<Vec<PathBuf>>,
    course_files_result: bool,
    tablename: Option<String>,
    tablelevel: Option<String>,
    rival_score: Option<Option<ScoreData>>,
    chart_option: Option<Option<rubato_types::replay_data::ReplayData>>,
    course_data: Option<CourseData>,
    course_song_data: Vec<SongData>,
    auto_play_songs: Option<Vec<PathBuf>>,
    auto_play_loop: Option<bool>,
    next_song_result: bool,
}

/// Mock PlayerResource that records operations.
struct MockPlayerResource {
    state: Arc<Mutex<MockState>>,
    course_gauge: Vec<Vec<Vec<f32>>>,
    course_replay: Vec<rubato_types::replay_data::ReplayData>,
}

impl PlayerResourceAccess for MockPlayerResource {
    fn into_any_send(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }
    fn config(&self) -> &rubato_types::config::Config {
        static CFG: std::sync::OnceLock<rubato_types::config::Config> = std::sync::OnceLock::new();
        CFG.get_or_init(rubato_types::config::Config::default)
    }
    fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
        static PC: std::sync::OnceLock<rubato_types::player_config::PlayerConfig> =
            std::sync::OnceLock::new();
        PC.get_or_init(rubato_types::player_config::PlayerConfig::default)
    }
    fn score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn rival_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn target_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn course_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn set_course_score_data(&mut self, _score: ScoreData) {}
    fn songdata(&self) -> Option<&SongData> {
        None
    }
    fn songdata_mut(&mut self) -> Option<&mut SongData> {
        None
    }
    fn set_songdata(&mut self, _data: Option<SongData>) {}
    fn replay_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        None
    }
    fn replay_data_mut(&mut self) -> Option<&mut rubato_types::replay_data::ReplayData> {
        None
    }
    fn course_replay(&self) -> &[rubato_types::replay_data::ReplayData] {
        &[]
    }
    fn add_course_replay(&mut self, _rd: rubato_types::replay_data::ReplayData) {}
    fn course_data(&self) -> Option<&CourseData> {
        None
    }
    fn course_index(&self) -> usize {
        0
    }
    fn next_course(&mut self) -> bool {
        false
    }
    fn constraint(&self) -> Vec<rubato_types::course_data::CourseDataConstraint> {
        vec![]
    }
    fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
        None
    }
    fn groove_gauge(&self) -> Option<&rubato_types::groove_gauge::GrooveGauge> {
        None
    }
    fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
        static EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
        &EMPTY
    }
    fn add_course_gauge(&mut self, _gauge: Vec<Vec<f32>>) {}
    fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
        &mut self.course_gauge
    }
    fn score_data_mut(&mut self) -> Option<&mut ScoreData> {
        None
    }
    fn course_replay_mut(&mut self) -> &mut Vec<rubato_types::replay_data::ReplayData> {
        &mut self.course_replay
    }
    fn maxcombo(&self) -> i32 {
        0
    }
    fn org_gauge_option(&self) -> i32 {
        0
    }
    fn set_org_gauge_option(&mut self, _val: i32) {}
    fn assist(&self) -> i32 {
        0
    }
    fn is_update_score(&self) -> bool {
        false
    }
    fn is_update_course_score(&self) -> bool {
        false
    }
    fn is_force_no_ir_send(&self) -> bool {
        false
    }
    fn is_freq_on(&self) -> bool {
        false
    }
    fn reverse_lookup_data(&self) -> Vec<String> {
        vec![]
    }
    fn reverse_lookup_levels(&self) -> Vec<String> {
        vec![]
    }
    fn clear(&mut self) {
        self.state.lock().expect("mutex poisoned").cleared = true;
    }
    fn set_bms_file(&mut self, path: &Path, mode_type: i32, mode_id: i32) -> bool {
        let mut s = self.state.lock().expect("mutex poisoned");
        s.bms_file_path = Some(path.to_path_buf());
        s.bms_file_mode_type = Some(mode_type);
        s.bms_file_mode_id = Some(mode_id);
        s.bms_file_result
    }
    fn set_course_bms_files(&mut self, files: &[PathBuf]) -> bool {
        let mut s = self.state.lock().expect("mutex poisoned");
        s.course_files = Some(files.to_vec());
        s.course_files_result
    }
    fn set_tablename(&mut self, name: &str) {
        self.state.lock().expect("mutex poisoned").tablename = Some(name.to_string());
    }
    fn set_tablelevel(&mut self, level: &str) {
        self.state.lock().expect("mutex poisoned").tablelevel = Some(level.to_string());
    }
    fn set_rival_score_data_option(&mut self, score: Option<ScoreData>) {
        self.state.lock().expect("mutex poisoned").rival_score = Some(score);
    }
    fn set_chart_option_data(&mut self, option: Option<rubato_types::replay_data::ReplayData>) {
        self.state.lock().expect("mutex poisoned").chart_option = Some(option);
    }
    fn set_course_data(&mut self, data: CourseData) {
        self.state.lock().expect("mutex poisoned").course_data = Some(data);
    }
    fn clear_course_data(&mut self) {
        self.state.lock().expect("mutex poisoned").course_data = None;
    }
    fn course_song_data(&self) -> Vec<SongData> {
        self.state
            .lock()
            .expect("mutex poisoned")
            .course_song_data
            .clone()
    }
    fn set_auto_play_songs(&mut self, paths: Vec<PathBuf>, loop_play: bool) {
        let mut s = self.state.lock().expect("mutex poisoned");
        s.auto_play_songs = Some(paths);
        s.auto_play_loop = Some(loop_play);
    }
    fn next_song(&mut self) -> bool {
        self.state.lock().expect("mutex poisoned").next_song_result
    }
}

/// Mock MainController that delegates resource access to MockPlayerResource.
struct MockMainController {
    state: Arc<Mutex<MockState>>,
    resource: MockPlayerResource,
}

impl MockMainController {
    fn new(state: Arc<Mutex<MockState>>) -> Self {
        let resource = MockPlayerResource {
            state: state.clone(),
            course_gauge: Vec::new(),
            course_replay: Vec::new(),
        };
        Self { state, resource }
    }
}

impl MainControllerAccess for MockMainController {
    fn config(&self) -> &rubato_types::config::Config {
        static CFG: std::sync::OnceLock<rubato_types::config::Config> = std::sync::OnceLock::new();
        CFG.get_or_init(rubato_types::config::Config::default)
    }
    fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
        static PC: std::sync::OnceLock<rubato_types::player_config::PlayerConfig> =
            std::sync::OnceLock::new();
        PC.get_or_init(rubato_types::player_config::PlayerConfig::default)
    }
    fn change_state(&mut self, state: MainStateType) {
        self.state
            .lock()
            .expect("mutex poisoned")
            .state_changes
            .push(state);
    }
    fn save_config(&self) {}
    fn exit(&self) {}
    fn save_last_recording(&self, _reason: &str) {}
    fn update_song(&mut self, _path: Option<&str>) {}
    fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        Some(&self.resource)
    }
    fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        Some(&mut self.resource)
    }
    fn play_audio_path(&mut self, path: &str, _volume: f32, _loop_play: bool) {
        self.state
            .lock()
            .unwrap()
            .played_audio_paths
            .push(path.to_string());
    }
}

fn make_selector_with_mock() -> (MusicSelector, Arc<Mutex<MockState>>) {
    let state = Arc::new(Mutex::new(MockState::default()));
    let mock = MockMainController::new(state.clone());
    let mut selector = MusicSelector::new();
    selector.set_main_controller(Box::new(mock));
    (selector, state)
}

struct ChangeStateSkin;

impl rubato_core::main_state::SkinDrawable for ChangeStateSkin {
    fn draw_all_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
    }

    fn update_custom_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
    }

    fn mouse_pressed_at(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
        ctx.change_state(MainStateType::Config);
    }

    fn mouse_dragged_at(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
    }

    fn prepare_skin(&mut self) {}
    fn dispose_skin(&mut self) {}
    fn fadeout(&self) -> i32 {
        0
    }
    fn input(&self) -> i32 {
        0
    }
    fn scene(&self) -> i32 {
        0
    }
    fn get_width(&self) -> f32 {
        0.0
    }
    fn get_height(&self) -> f32 {
        0.0
    }
    fn swap_sprite_batch(&mut self, _batch: &mut rubato_render::sprite_batch::SpriteBatch) {}
}

// ============================================================
// read_chart tests
// ============================================================

#[test]
fn test_read_chart_success_clears_resource_and_transitions() {
    let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
    if !bms_path.exists() {
        return;
    }
    let path_str = bms_path.to_string_lossy().to_string();
    let mut selector = MusicSelector::new();
    selector.play = Some(BMSPlayerMode::PLAY);

    let song = make_song_data("abc123", Some(&path_str));
    let bar = make_song_bar("abc123", Some(&path_str));

    selector.read_chart(&song, &bar);

    assert!(
        selector.player_resource.is_some(),
        "player_resource should be created"
    );
    assert_eq!(
        selector.pending_state_change,
        Some(MainStateType::Decide),
        "should transition to DECIDE on success"
    );
    assert_eq!(
        selector
            .playedsong
            .as_ref()
            .map(|sd| sd.file.sha256.as_str()),
        Some("abc123"),
        "playedsong should be set"
    );
}

#[test]
fn test_read_chart_failure_does_not_transition() {
    let mut selector = MusicSelector::new();
    selector.play = Some(BMSPlayerMode::PLAY);

    let song = make_song_data("abc123", Some("/nonexistent.bms"));
    let bar = make_song_bar("abc123", Some("/nonexistent.bms"));

    selector.read_chart(&song, &bar);

    assert!(
        selector.player_resource.is_some(),
        "player_resource should still be created"
    );
    assert_eq!(
        selector.pending_state_change, None,
        "should NOT transition on failure"
    );
    assert!(
        selector.playedsong.is_none(),
        "playedsong should NOT be set on failure"
    );
}

#[test]
fn test_read_chart_sets_rival_score_and_chart_option() {
    let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
    if !bms_path.exists() {
        return;
    }
    let path_str = bms_path.to_string_lossy().to_string();
    let mut selector = MusicSelector::new();
    selector.play = Some(BMSPlayerMode::PLAY);

    let song = make_song_data("abc123", Some(&path_str));
    let bar = make_song_bar("abc123", Some(&path_str));

    selector.read_chart(&song, &bar);

    // Verify chart was loaded successfully and state transition requested
    assert_eq!(
        selector.pending_state_change,
        Some(MainStateType::Decide),
        "should transition on success"
    );
}

#[test]
fn test_read_chart_without_main_controller_does_not_panic() {
    let mut selector = MusicSelector::new();
    selector.play = Some(BMSPlayerMode::PLAY);

    let song = make_song_data("abc123", Some("/test/song.bms"));
    let bar = make_song_bar("abc123", Some("/test/song.bms"));

    // Should not panic, just log warning
    selector.read_chart(&song, &bar);
}

// ============================================================
// read_course tests
// ============================================================

#[test]
fn test_read_course_success_transitions_to_decide() {
    let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
    if !bms_path.exists() {
        return;
    }
    let path_str = bms_path.to_string_lossy().to_string();

    let mut selector = MusicSelector::new();
    let course = CourseData {
        name: Some("Test Course".to_string()),
        hash: vec![make_song_data("s1", Some(&path_str))],
        constraint: vec![],
        trophy: vec![],
        release: false,
    };
    selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
    selector.manager.selectedindex = 0;

    selector.read_course(BMSPlayerMode::PLAY);

    assert_eq!(
        selector.pending_state_change,
        Some(MainStateType::Decide),
        "should transition to DECIDE"
    );
    assert!(
        selector.playedcourse.is_some(),
        "playedcourse should be set"
    );
}

#[test]
fn test_read_course_missing_songs_does_not_transition() {
    let mut selector = MusicSelector::new();

    // GradeBar with a song that has no path
    let course = CourseData {
        name: Some("Incomplete Course".to_string()),
        hash: vec![
            make_song_data("s1", Some("/path/song1.bms")),
            make_song_data("s2", None), // missing path
        ],
        constraint: vec![],
        trophy: vec![],
        release: false,
    };
    selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
    selector.manager.selectedindex = 0;

    selector.read_course(BMSPlayerMode::PLAY);

    assert_eq!(
        selector.pending_state_change, None,
        "should NOT transition when songs are missing"
    );
}

#[test]
fn test_read_course_class_constraint_resets_random() {
    let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
    if !bms_path.exists() {
        return;
    }
    let path_str = bms_path.to_string_lossy().to_string();

    let mut selector = MusicSelector::new();
    // Set non-zero random options
    selector.config.play_settings.random = 3;
    selector.config.play_settings.random2 = 4;
    selector.config.play_settings.doubleoption = 2;

    let course = CourseData {
        name: Some("Class Course".to_string()),
        hash: vec![make_song_data("s1", Some(&path_str))],
        constraint: vec![CourseDataConstraint::Class],
        trophy: vec![],
        release: false,
    };
    selector.manager.currentsongs = vec![Bar::Grade(Box::new(GradeBar::new(course)))];
    selector.manager.selectedindex = 0;

    selector.read_course(BMSPlayerMode::PLAY);

    assert_eq!(
        selector.config.play_settings.random, 0,
        "CLASS should reset random to 0"
    );
    assert_eq!(
        selector.config.play_settings.random2, 0,
        "CLASS should reset random2 to 0"
    );
    assert_eq!(
        selector.config.play_settings.doubleoption, 0,
        "CLASS should reset doubleoption to 0"
    );
}

// ============================================================
// _read_course tests
// ============================================================

#[test]
fn test_internal_read_course_ln_constraint() {
    let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
    if !bms_path.exists() {
        return;
    }
    let path_str = bms_path.to_string_lossy().to_string();

    let mut selector = MusicSelector::new();
    selector.config.play_settings.lnmode = 2;

    let course = CourseData {
        name: Some("LN Course".to_string()),
        hash: vec![make_song_data("s1", Some(&path_str))],
        constraint: vec![CourseDataConstraint::Ln],
        trophy: vec![],
        release: false,
    };
    let bar = Bar::Grade(Box::new(GradeBar::new(course)));

    let result = selector._read_course(&BMSPlayerMode::PLAY, &bar);

    assert!(result, "_read_course should return true on success");
    assert_eq!(
        selector.config.play_settings.lnmode, 0,
        "LN constraint should set lnmode to 0"
    );
}

#[test]
fn test_internal_read_course_autoplay_applies_constraints() {
    let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
    if !bms_path.exists() {
        return;
    }
    let path_str = bms_path.to_string_lossy().to_string();

    let mut selector = MusicSelector::new();
    selector.config.play_settings.random = 5;

    let course = CourseData {
        name: Some("Class Course".to_string()),
        hash: vec![make_song_data("s1", Some(&path_str))],
        constraint: vec![CourseDataConstraint::Class],
        trophy: vec![],
        release: false,
    };
    let bar = Bar::Grade(Box::new(GradeBar::new(course)));

    let result = selector._read_course(&BMSPlayerMode::AUTOPLAY, &bar);

    assert!(result);
    // AUTOPLAY applies CLASS constraint (same as PLAY)
    assert_eq!(
        selector.config.play_settings.random, 0,
        "AUTOPLAY should apply CLASS constraint and reset random"
    );
}

#[test]
fn test_internal_read_course_replay_skips_constraints() {
    let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/5key.bms");
    if !bms_path.exists() {
        return;
    }
    let path_str = bms_path.to_string_lossy().to_string();

    let mut selector = MusicSelector::new();
    selector.config.play_settings.random = 5;

    let course = CourseData {
        name: Some("Class Course".to_string()),
        hash: vec![make_song_data("s1", Some(&path_str))],
        constraint: vec![CourseDataConstraint::Class],
        trophy: vec![],
        release: false,
    };
    let bar = Bar::Grade(Box::new(GradeBar::new(course)));

    let result = selector._read_course(&BMSPlayerMode::REPLAY_1, &bar);

    assert!(result);
    // REPLAY should NOT apply constraints
    assert_eq!(
        selector.config.play_settings.random, 5,
        "REPLAY should not reset random"
    );
}

// ============================================================
// read_random_course tests
// ============================================================

#[test]
fn test_read_random_course_missing_songs_does_not_transition() {
    let (mut selector, state) = make_selector_with_mock();

    // RandomCourseBar with no stages — exists_all_songs returns false
    let rcd = RandomCourseData {
        name: Some("Random Course".to_string()),
        stage: vec![], // no stages = not all songs exist
        ..Default::default()
    };
    selector.manager.currentsongs = vec![Bar::RandomCourse(Box::new(
        crate::select::bar::random_course_bar::RandomCourseBar::new(rcd),
    ))];
    selector.manager.selectedindex = 0;

    selector.read_random_course(BMSPlayerMode::PLAY);

    let s = state.lock().expect("mutex poisoned");
    assert!(
        s.state_changes.is_empty(),
        "should NOT transition when random course has no stages"
    );
}

// ============================================================
// directory autoplay tests
// ============================================================

#[test]
fn test_directory_autoplay_transitions_to_decide() {
    // Use a real BMS file so next_song() succeeds
    let bms_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/test.bms");
    if !bms_path.exists() {
        // Skip if test BMS file not available
        return;
    }
    let mut selector = MusicSelector::new();
    selector.read_directory_autoplay(vec![bms_path]);

    assert!(
        selector.player_resource.is_some(),
        "player_resource should be created"
    );
    assert_eq!(
        selector.pending_state_change,
        Some(MainStateType::Decide),
        "should transition to DECIDE"
    );
}

#[test]
fn test_directory_autoplay_no_transition_when_next_song_fails() {
    // Non-existent paths cause next_song() to return false
    let mut selector = MusicSelector::new();
    selector.read_directory_autoplay(vec![PathBuf::from("/nonexistent/song_a.bms")]);

    assert!(
        selector.player_resource.is_some(),
        "player_resource should be created"
    );
    assert_eq!(
        selector.pending_state_change, None,
        "should NOT transition when next_song returns false"
    );
}

#[test]
fn test_directory_autoplay_empty_paths_does_nothing() {
    let mut selector = MusicSelector::new();
    selector.read_directory_autoplay(vec![]);

    assert!(
        selector.player_resource.is_none(),
        "player_resource should NOT be created for empty paths"
    );
    assert_eq!(
        selector.pending_state_change, None,
        "should NOT transition with empty paths"
    );
}

#[test]
fn test_directory_autoplay_path_extraction_from_container_bar() {
    // Verify path extraction logic from directory bar children
    let children = vec![
        make_song_bar("sha_a", Some("/dir/song_a.bms")),
        make_song_bar("sha_b", Some("/dir/song_b.bms")),
        make_song_bar("sha_c", None), // no path - should be filtered out
    ];
    let container = crate::select::bar::container_bar::ContainerBar::new(String::new(), children);
    let bar = Bar::Container(Box::new(container));

    assert!(bar.is_directory_bar());

    // Extract paths the same way render() does
    let paths: Vec<PathBuf> = if let Bar::Container(b) = &bar {
        b.children()
            .iter()
            .filter_map(|bar| {
                bar.as_song_bar()
                    .filter(|sb| sb.exists_song())
                    .and_then(|sb| sb.song_data().path())
                    .map(PathBuf::from)
            })
            .collect()
    } else {
        vec![]
    };

    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0], PathBuf::from("/dir/song_a.bms"));
    assert_eq!(paths[1], PathBuf::from("/dir/song_b.bms"));
}

#[test]
fn test_handle_skin_mouse_pressed_uses_selector_context() {
    let mut selector = MusicSelector::new();
    selector.main_state_data.skin = Some(Box::new(ChangeStateSkin));

    <MusicSelector as MainState>::handle_skin_mouse_pressed(&mut selector, 0, 32, 48);

    assert_eq!(selector.pending_state_change, Some(MainStateType::Config));
}
